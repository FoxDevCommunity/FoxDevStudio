//! The bytecode interpreter.
//!
//! # Fibers and frames
//! Every running program/method is a *fiber*: a value stack, a frame stack, the TRY handlers
//! installed by its frames and the bookkeeping for a pending host request. `step` runs a fiber
//! until it finishes, fails or yields a `HostRequest`; the scheduler performs the request and
//! calls `resume` (or `resume_error`) before stepping again. Several fibers can be parked at
//! once (READ EVENTS in one, a Click method running in another).
//!
//! A frame owns its local slots, its PRIVATE variables, `THIS`, its WITH stack and the argument
//! list. `LoadName` walks the privates of the current frame, then of every caller frame
//! (dynamic scoping), then the PUBLIC globals; locals are slots and never visible to callees.
//!
//! # Runtime compilation
//! `&macro`, `&cmd` lines and `EVALUATE()` compile their text against the current function's
//! slot names and run it in an *inline* frame: a frame that has no storage of its own and
//! forwards every locals / privates / WITH / argument access to its owner frame (so slot numbers
//! in the compiled code match, and a snippet may create privates in the owner). The owner's
//! locals vector is temporarily extended for the temporaries the inline code needs and cut back
//! when the inline frame returns. Compiled inline modules are cached per (kind, owner function,
//! text).
//!
//! # Errors
//! An error unwinds to the innermost TRY handler of the fiber (whichever frame installed it):
//! frames above it are popped, the value stack is cut back to the depth at `TryPush`, and
//! execution continues at the CATCH block (a finally-only handler is installed for it) or at the
//! FINALLY block with the error pending for `EndFinally`. Without a handler, `ON ERROR` text is
//! compiled as a snippet and run in a new frame; when it returns the failing frame continues at
//! its next statement. Otherwise the fiber ends with `Step::Error`. `CATCH TO var` receives the
//! error *message* (error objects arrive in phase 6); `BuiltinCtx::last_error` has the full error.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::builtins::{self, BuiltinCtx, BuiltinResult};
use crate::bytecode::*;
use crate::compiler;
use crate::data::{AreaRef, Cursor, DataSession, Move, PAGE_RECORDS, Walk, bytes_of, index_key_padded};
use crate::lexer::is_ident_char;
use crate::query::{HavingRef, PlanColumn, PlanInto, QueryPlan, QueryRun, QuerySourceState};
use crate::error::RtError;
use crate::host::{BreakReason, ClassDefOut, Host, HostRequest, JsonValue, Member, StepMode};
use crate::value::{self, CmpOp, FoxArray, FuncId, Handle, Settings, Value};

pub type FiberId = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Done { value: Value, nodefault: bool },
    Error(RtError),
    Suspend(HostRequest),
}

/// What to do with the value a suspended fiber is resumed with.
#[derive(Debug, Clone)]
enum Pending {
    Push,
    Discard,
    /// Hands the answer to the instruction that asked, which is about to run again.
    DataReply,
    /// The object's `Error` method has finished; it may handle errors again.
    EndClassError {
        obj: u32,
    },
    /// WAIT WINDOW ... TO var: push the key ("" for Null).
    WaitKey,
    /// A READ has come back: the values the user left in the fields go back where they came
    /// from, in the order the fields were put up.
    ReadGets,
    /// MENU TO has come back: the number chosen goes into the variable it named.
    MenuTo {
        name: String,
    },
    /// A file function's answer: the value, and the error number FERROR() then reports.
    FileResult,
    /// A file command's answer: an error is raised, output is printed, nothing is pushed.
    FileCommand {
        op: String,
    },
    /// A library function has returned. The answer is the return value and what the library
    /// wrote into the arguments it was given the address of, which are written back through the
    /// references the caller passed.
    DllReturn {
        refs: Vec<Value>,
    },
    FinishDoForm {
        flags: u8,
    },
    /// The host loaded a program: call `func` (None = MAIN) in the resumed module id.
    LoadedProgram {
        func: Option<String>,
        args: Vec<Value>,
        discard: bool,
        /// The program the host was asked for, for the error when it has none.
        program: String,
    },
    /// QUIT / CANCEL: the fiber ends when resumed.
    Finish,
    /// `oServer.Listen(n)` has come back: the port the host actually bound is kept on the
    /// server, so `oServer.Port` reads it, and answered to the program.
    Listening {
        server: u32,
    },
}

#[derive(Debug, Clone)]
enum FrameKind {
    /// A procedure call; the result is pushed on the caller's stack unless `discard` (DO).
    Call { discard: bool },
    /// Runtime-compiled code sharing the storage of `owner`.
    Inline { owner: usize, push_result: bool },
    /// An ON ERROR handler; when it returns the caller continues at `resume_pc`.
    OnError { resume_pc: usize, stmt_sp: usize },
}

#[derive(Debug)]
struct Frame {
    module: u32,
    func: u32,
    pc: usize,
    locals: Vec<Value>,
    privates: HashMap<String, Value>,
    this: Option<Handle>,
    with_stack: Vec<Value>,
    args: Vec<Value>,
    line: u32,
    nodefault: bool,
    stack_base: usize,
    /// Stack depth at the last `Stmt`, so ON ERROR can resume at the next statement.
    stmt_sp: usize,
    /// Slots `RELEASE` has let go of. A local lives in a numbered slot, so there is nowhere to
    /// take it from; Visual FoxPro leaves the name undefined until something writes it again,
    /// and this is what says so. Empty until a routine releases one, which almost none do.
    released: Vec<bool>,
    kind: FrameKind,
}

#[derive(Debug, Clone)]
struct TryHandler {
    frame: usize,
    catch: Option<usize>,
    finally: Option<usize>,
    sp: usize,
    with_len: usize,
    pending_len: usize,
}

/// The files of a `SET PROCEDURE TO`, and how far loading them has got.
#[derive(Debug, Default)]
struct ProcedureLoad {
    additive: bool,
    names: Vec<String>,
    next: usize,
}

#[derive(Debug, Default)]
struct Fiber {
    frames: Vec<Frame>,
    stack: Vec<Value>,
    handlers: Vec<TryHandler>,
    /// One entry per FINALLY block being executed: the error to re-raise at `EndFinally`.
    pending_rethrow: Vec<Option<RtError>>,
    pending: Option<Pending>,
    resumed: Option<Result<Value, RtError>>,
    last_error: Option<RtError>,
    nodefault: bool,
    in_error_handler: bool,
    result: Option<Value>,
    /// The answer to the data request an instruction is part-way through. Data work can take
    /// more than one round trip - open, then read a page, then read a memo - so those
    /// instructions rewind the program counter, suspend, and run again with the reply here.
    data_reply: Option<Value>,
    /// Whether that reply is a `SET CLASSLIB` the host has read rather than an index file, so
    /// the instruction that runs again knows which of the two it was waiting for.
    classlib_reply: bool,
    /// A `SET PROCEDURE TO` part way through loading its files: each one the host has to
    /// load is a round trip, and the list only changes once every file is in.
    procedure_load: Option<ProcedureLoad>,
    /// What the debugger asked this fiber to do when it was let go, and how deep its frames
    /// were then - which is what tells a step over a call from a step into it.
    stepping: Option<(StepMode, usize)>,
    /// The SELECT-SQL running in this fiber. Only one at a time: a query is a statement, and a
    /// statement does not start another before it finishes.
    query: Option<QueryRun>,
    /// A query an error has just been raised out of. Its sources have still to be put back, and
    /// putting them back can need the host, so it is done by the run loop rather than by the
    /// error path - which has nowhere to wait.
    query_unwind: Option<QueryRun>,
    /// Sources opened for a query that has not begun gathering yet.
    pending_sources: Vec<QuerySourceState>,
    /// The work area that was selected when the query's first source was opened, which is the
    /// one the statement was written in. Opening a source selects it, so by the time the run
    /// begins the current area is a source's, and INTO ARRAY has to put the program back where
    /// it stood rather than there.
    pending_area: Option<usize>,
    /// The routine a `RETURN TO` is on its way to, while the frames between here and it are
    /// being unwound. Empty text is `RETURN TO MASTER`: the program the run started in.
    return_to: Option<String>,
}

/// What a function value is: where the code is, and the frame the lambda was written in as far
/// as it can be carried. The captured values are copies, taken when the lambda was made, because
/// the frame they lived in is gone by the time the lambda runs.
#[derive(Debug, Clone)]
pub struct FuncValue {
    pub module: u32,
    pub func: u32,
    /// `THIS` where the lambda was written, so a handler written in a method keeps its object.
    pub this: Option<Handle>,
    /// One value per entry of the function's `captures`, in that order.
    pub captured: Vec<Value>,
}

enum Flow {
    Next,
    /// A `RETURN` the program wrote. It leaves a line put together by a macro as well as the
    /// routine that line belongs to, because in Visual FoxPro that line *is* part of the
    /// routine: the text goes into it before it is read.
    Return(Value),
    /// The routine ran out of statements. It only ends what is running.
    EndOfCode(Value),
    Suspend(HostRequest),
}

pub struct Vm {
    settings: Settings,
    globals: HashMap<String, Value>,
    modules: Vec<Rc<Module>>,
    /// Program modules by upper-cased name.
    programs: HashMap<String, u32>,
    fibers: HashMap<FiberId, Fiber>,
    next_fiber: FiberId,
    /// The function values lambdas have made, by id. A function value is an id into this table
    /// exactly as an object value is a handle into the host's, which is what lets one travel to
    /// the host and come back as the same function. Nothing is ever taken out: a function value
    /// lives as long as the runtime, like a compiled procedure. See docs/foxscript.md.
    functions: HashMap<u32, Rc<FuncValue>>,
    next_function: u32,
    /// The `FoxScript` namespace and everything it has made.
    natives: crate::foxscript::Natives,
    on_error: Option<String>,
    /// A record listing has started and still owes its row of field names, which goes above the
    /// first record it finds to write.
    list_heading_due: bool,
    /// (what the text is, owner module, owner func, text) -> inline module id.
    inline_cache: HashMap<(Inline, u32, u32, String), u32>,
    /// Work areas and their cursors. One set per VM, as VFP has one default data session.
    data: DataSession,
    /// The pictures LOADPICTURE() has read, which SAVEPICTURE() writes back out.
    pictures: crate::picture::Pictures,
    /// What the last low-level file function left for FERROR().
    ferror: i64,
    /// What a `SET TEXTMERGE TO MEMVAR` destination has collected. Visual FoxPro puts it in the
    /// variable when the output is closed rather than as it goes, so it is held here until then.
    textmerge_held: String,
    /// Whether a line has been begun in the destination, which is what decides if the next `\`
    /// puts a line end in front of its text. A file has one the moment it is opened; a variable
    /// being built has nothing in front of it, so its first line starts where it stands.
    textmerge_begun: bool,
    /// Whether the file destination has been written to yet, so that the first write replaces
    /// what the file held and the rest add to it - unless ADDITIVE said to keep it.
    textmerge_written: bool,
    /// Windows library functions a `DECLARE ... DLL` has made callable: what the program calls
    /// it, and the library and declaration it stands for.
    dlls: HashMap<String, (String, crate::bytecode::DllProto)>,
    /// Visual FoxPro libraries `SET LIBRARY TO` has loaded: what the host calls each one and
    /// the path it was loaded from, in the order they were loaded, which is the order
    /// `SET("LIBRARY")` lists them in.
    libraries: Vec<(u32, String)>,
    /// The functions those libraries added: the name a program calls, and which function of
    /// which library it is. A name loaded twice is the later one, as the product's is.
    library_funcs: HashMap<String, (u32, u32)>,
    /// `SET PROCEDURE TO`: the programs whose routines a bare call finds after the current
    /// program's own, in the order they are looked in - which is the order they were listed.
    procedure_files: Vec<u32>,
    /// Programs loaded only to be procedure files and never run. Measured: once a procedure
    /// file is released its routines are not found, while a program that ran with `DO` keeps
    /// its routines findable after it returns. These are the ones that go when released.
    procedure_only: HashSet<u32>,
    /// Objects whose `Error` method is running, so an error inside it is not handed back to it.
    in_class_error: HashSet<u32>,
    /// The work area the last USE opened, so the index beside it can be read next.
    last_opened: Option<usize>,
    /// The tag `INDEX ON` is gathering keys for.
    index_build: Option<IndexBuild>,
    /// The single-entry indexes `USE ... INDEX` or `SET INDEX TO` has left to read.
    idx_open: Option<IdxOpen>,
    /// What `COPY INDEXES` has left to read and write.
    copy_indexes: Option<CopyIndexes>,
    /// The `COPY TAG` on its way.
    copy_tag: Option<CopyTag>,
    /// What `COUNT`, `SUM`, `AVERAGE` and `CALCULATE` are adding up as they walk the table.
    aggregates: Vec<Agg>,
    /// The table `COPY TO`, `SORT TO` or `TOTAL ON` is gathering.
    copy: Option<CopyOut>,
    /// The file `APPEND FROM` is reading, and the records it has left to add.
    append: Option<AppendFrom>,
    /// The records `PACK` has left to write back.
    pack: Option<Pack>,
    /// The records `INSERT BLANK` has left to move down one.
    shift: Option<Shift>,
    /// The table `ALTER TABLE` is reading and writing back.
    alter: Option<Alter>,
    /// The column names that alteration worked out for itself, kept for as long as it runs.
    alter_names: Vec<String>,
    /// How deep the program is in transactions: while it is in one, every write is held.
    txn: u32,
    /// The databases that are open, and which of them is current.
    databases: Vec<Database>,
    current_db: Option<usize>,
    /// A database container on its way to or from the host, a file at a time.
    db_io: Option<DbIo>,
    /// A container has just been read, so its procedures are about to be told - with the
    /// clauses `OPEN DATABASE` was written with, which dbc_OpenData is handed.
    opened_database: Option<u16>,
    /// The clauses the `OPEN DATABASE` now reading its container was written with, kept from
    /// the command until the file has arrived and the event can be told.
    open_clauses: u16,
    /// The name a database command took off the stack before it had to wait for a file.
    db_await: Option<String>,
    /// The menus the program has defined, and which of them is up.
    menus: crate::menu::Menus,
    /// The character screen, and the windows a program has opened on it.
    screen: crate::screen::Screen,
    /// The report that is running: what it is reading, what it has read, and what it has
    /// written so far.
    report: Option<RunningReport>,
    /// The commands `ON ESCAPE`, `ON KEY LABEL` and the rest have hung off what may happen,
    /// by what they hang off. `ON("ESCAPE")` reads them back.
    handlers: HashMap<String, String>,
    /// Where the debugger has asked the program to stop, by the name of the source the line
    /// belongs to and the line number in it. A .prg's frames all answer to the program's own
    /// name; a form's method answers to `objPath.Event`, which is what its editor shows, so
    /// either name reaches the lines it owns.
    breakpoints: Vec<(String, u32)>,
    /// What `PUSH KEY` kept, for `POP KEY` to put back.
    key_stack: Vec<HashMap<String, String>>,
    /// The name a `FROM (cPath)` worked out, kept while the table is being opened so the
    /// instruction that runs again does not take it off the stack twice.
    sql_named: Option<String>,
    /// The file APPEND MEMO has read, kept while the field it goes into is fetched.
    memo_io: Option<String>,
    /// The work areas a command that says `IN` moved away from, to go back to.
    area_stack: Vec<usize>,
    /// COMARRAY(): how each COM object was last told to take an array.
    com_arrays: HashMap<u32, i64>,
    /// COMPROP(): what each COM object has been set to, by handle and setting name.
    com_props: HashMap<(u32, String), Value>,
    /// SQLSETPROP(): what each connection has been set to, by number and setting name.
    sql_props: HashMap<(i64, String), Value>,
    /// What is true of one cursor: offline, transactable. Keyed by alias and flag name.
    cursor_flags: std::collections::HashSet<(String, String)>,
    /// SETRESULTSET(): the work area of the cursor a server hands back to whoever called it,
    /// or 0 when none is marked.
    result_set: usize,
    /// Table handles a cursor replaced, waiting to be given back to the host.
    to_release: Vec<u32>,
    /// The header of that file, between asking for its records and getting them.
    source_header: Option<crate::dbf::DbfHeader>,
    /// The object `SCATTER NAME` is filling in, and the properties it has left to add.
    scatter_name: Option<ScatterName>,
}

/// A database that is open: where its container is and what it holds.
pub struct Database {
    pub path: String,
    pub objects: Vec<crate::dbc::DbObject>,
    /// Kept for the day a database can be open without being current.
    pub open: bool,
    /// The module the stored procedures were compiled into, once anything has called one.
    procedures: Option<u32>,
}

/// The code page a text file of stored procedures is read and written in, which is the one the
/// runtime works in: `CPCURRENT()` answers 1252 and the dbc procedure events are handed the same
/// number - measured, `COPY PROCEDURES TO` and `APPEND PROCEDURES FROM` both say 1252.
const TEXT_CODE_PAGE: f64 = 1252.0;

/// One argument a database event is handed.
///
/// Visual FoxPro lower-cases every name, alias and path it passes to a dbc procedure and leaves
/// the value a DBSETPROP was given exactly as the program wrote it - measured, by recording
/// what a container's procedures were called with for `CREATE TABLE MixedName` and
/// `DBSETPROP("OtherName", "Table", "Comment", "Mixed Case Value")`. So an argument says which
/// of the two kinds it is rather than being folded at the call.
#[derive(Clone, Debug)]
pub enum DbcArg {
    /// The name of something: a table, a view, a property, a file. Handed over lower-cased.
    Name(String),
    /// A value whose case is the program's own.
    Text(String),
    Flag(bool),
    Num(f64),
}

impl DbcArg {
    fn name(text: impl AsRef<str>) -> Self {
        DbcArg::Name(text.as_ref().to_string())
    }

    /// The argument as it is written into the call that runs the procedure.
    fn written(&self) -> String {
        match self {
            DbcArg::Name(text) => format!("\"{}\"", text.replace('"', "").to_ascii_lowercase()),
            DbcArg::Text(text) => format!("\"{}\"", text.replace('"', "")),
            DbcArg::Flag(true) => ".T.".to_string(),
            DbcArg::Flag(false) => ".F.".to_string(),
            DbcArg::Num(n) => format!("{n}"),
        }
    }
}

impl Database {
    /// Whether this container's events are switched on.
    ///
    /// `DBSETPROP(db, "Database", "DBCEvents", .T.)` is what turns them on, and until it has
    /// been said the product calls none of these procedures at all: a container whose
    /// procedures record every call, driven through creating, opening, closing, adding,
    /// removing, renaming, validating and packing, recorded nothing until the property was
    /// set - measured.
    pub fn events_enabled(&self) -> bool {
        self.objects
            .iter()
            .find(|o| o.kind.eq_ignore_ascii_case("Database"))
            .and_then(|o| {
                o.property
                    .lines()
                    .find_map(|line| line.split_once('=').filter(|(key, _)| key.trim().eq_ignore_ascii_case("DBCEvents")))
            })
            .is_some_and(|(_, value)| value.trim().eq_ignore_ascii_case(".T."))
    }

    /// The stored procedures the container holds, as their source.
    pub fn procedure_source(&self) -> &str {
        self.objects
            .iter()
            .find(|o| o.kind.eq_ignore_ascii_case("StoredProc"))
            .map(|o| o.code.as_str())
            .unwrap_or("")
    }

    /// Replaces them, and forgets what was compiled from the old ones.
    pub fn set_procedures(&mut self, source: String) {
        self.procedures = None;
        match self.objects.iter_mut().find(|o| o.kind.eq_ignore_ascii_case("StoredProc")) {
            Some(object) => object.code = source,
            None => {
                let id = self.objects.iter().map(|o| o.id).max().unwrap_or(0) + 1;
                let mut object = crate::dbc::DbObject::new(id, 1, "StoredProc", "StoredProceduresSource");
                object.code = source;
                self.objects.push(object);
            }
        }
    }

    /// The number the next object of the container takes.
    fn next_id(&self) -> i32 {
        self.objects.iter().map(|o| o.id).max().unwrap_or(0) + 1
    }

    /// The name a program calls it: the container's own, without the folder or the extension.
    pub fn name(&self) -> String {
        stem_of(&self.path)
    }

    /// The view of that name, when the database has one.
    pub fn view(&self, name: &str) -> Option<&crate::dbc::DbObject> {
        self.objects.iter().find(|o| o.kind == "View" && o.name.eq_ignore_ascii_case(name))
    }

    /// Where the `.dbf` of a table of that name is, when the container holds one.
    ///
    /// The container records a path of its own for each table and it is read from the container's
    /// folder rather than from the default directory: measured, `USE testdata!products` run from
    /// anywhere at all opens `<samples>\Data\PRODUCTS.DBF`, the file `testdata.dbc` names, and
    /// `DBF()` says so.
    pub fn table_file(&self, name: &str) -> Option<(String, String)> {
        let object = self
            .objects
            .iter()
            .find(|o| o.kind.eq_ignore_ascii_case("Table") && o.name.eq_ignore_ascii_case(name))?;
        Some((beside(&self.path, &crate::dbc::table_file(object)), object.name.trim().to_string()))
    }
}

/// What a `dbname!tablename` comes to when no open container holds a table of that name: the
/// part after the `!`, opened as an ordinary file.
///
/// Measured: with `testdata` open, `USE testdata!nosuch` says `nosuch.dbf` does not exist, and
/// names neither the container nor the whole of what was written.
fn plain_table(named: String) -> String {
    match named.split_once('!') {
        Some((_, table)) => table.trim().to_string(),
        None => named,
    }
}

/// A file named from beside `container`: a name of its own is read from the container's folder,
/// and one that says where it starts is left alone.
fn beside(container: &str, file: &str) -> String {
    if crate::value::is_rooted(file) {
        return file.to_string();
    }
    match container.rfind(['/', '\\']) {
        Some(cut) => format!("{}{file}", &container[..=cut]),
        None => file.to_string(),
    }
}

/// A database container on its way to or from the host: the container itself, then the memo
/// file beside it.
struct DbIo {
    /// 0 the container, 1 the memo file.
    stage: u8,
    path: String,
    dbf: Vec<u8>,
    memo: Vec<u8>,
    writing: bool,
    /// True when the container is only being attached - read and left open, but not made
    /// current - which is what a `dbname!tablename` in a command does to the database it names.
    attach: bool,
}

/// The memo file beside a database container.
pub fn with_dct(path: &str) -> String {
    with_extension(path, "dct")
}

/// The extension a named document has when the name is written without one.
///
/// Every command that opens or makes a document names a kind before it names the file, and the
/// kind says what the file is: `MODIFY DATABASE dvds` opens `dvds.dbc`, `MODIFY COMMAND go`
/// opens `go.prg`, `CREATE FORM edit` makes `edit.scx`. Measured against Visual FoxPro 9,
/// which wrote `zzz1.PRG` for `MODIFY COMMAND zzz1` and `zzz2.TXT` for `MODIFY FILE zzz2`.
fn document_extension(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "COMMAND" | "PROGRAM" => "prg",
        "FILE" => "txt",
        "FORM" | "SCREEN" => "scx",
        "CLASS" => "vcx",
        "MENU" => "mnx",
        "PROJECT" => "pjx",
        "DATABASE" => "dbc",
        "REPORT" => "frx",
        "LABEL" => "lbx",
        "QUERY" => "qpr",
        // BROWSE and the designers that work on what is already open name no kind of file
        _ => return None,
    })
}

/// A document named for a command that knows what kind it is.
///
/// A name written with an extension of its own is left as it stands - `MODIFY FILE dvds.log`
/// means that file - so only a bare name takes the kind's.
fn named_document(named: &str, kind: &str) -> String {
    match document_extension(kind) {
        Some(extension) if !has_extension(named) => with_extension(named, extension),
        _ => named.to_string(),
    }
}

/// Whether a path names its own kind, rather than a folder that happens to have a dot in it.
fn has_extension(path: &str) -> bool {
    path.rfind('.').is_some_and(|dot| !path[dot..].contains(['/', '\\']))
}

/// A file name with that extension, whatever it had.
fn with_extension(path: &str, extension: &str) -> String {
    let stem = match has_extension(path) {
        true => &path[..path.rfind('.').unwrap_or(path.len())],
        false => path,
    };
    format!("{stem}.{extension}")
}

/// A single-entry index the host sent, or the error a program should see when there was no
/// such file or what came back was not one.
fn read_idx(bytes: &[u8], path: &str) -> Result<crate::cdx::Tag, RtError> {
    if bytes.is_empty() {
        return Err(RtError::file_not_found(path));
    }
    crate::idx::read(bytes).map_err(|e| RtError::new(RtError::FILE_NOT_FOUND, format!("'{path}' is not an index: {e}")))
}

/// The file names a command listed, as the stack holds them: anything that is not text, and
/// anything blank, names no file.
fn named_files(values: &[Value]) -> Vec<String> {
    values.iter().filter_map(|v| v.as_str().ok()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// The name of a file without its folder or its extension.
pub fn stem_of(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match name.rfind('.') {
        Some(dot) => name[..dot].to_string(),
        None => name.to_string(),
    }
}

/// The word a field's type letter stands for, as a listing writes it.
/// One variable as `DISPLAY MEMORY` writes it, and a line per element when it is an array.
///
/// The layout is Visual FoxPro's, measured: the name in twelve columns, where it lives in
/// seven, then the type letter and the value from column twenty-two. An array says which
/// program it belongs to instead of a value, and its elements follow it under their subscripts.
fn memory_lines(name: &str, public: bool, value: &Value, program: &str, settings: &Settings) -> Vec<String> {
    let where_ = if public { "Pub" } else { "Priv" };
    let Value::Array(items) = value.deref() else {
        return vec![format!("{name:<12}{where_:<7}{}", memory_value(value, settings))];
    };
    let mut out = vec![format!("{name:<12}{where_:<7}A  {program}")];
    for (i, item) in items.borrow().items.iter().enumerate() {
        out.push(format!("        ({:>4})     {}", i + 1, memory_value(&item.deref(), settings)));
    }
    out
}

/// The type letter and the value beside it, as `DISPLAY MEMORY` writes them.
///
/// Text is shown in quotes, and a number twice: as it would print, and again to eight decimal
/// places in brackets, which is what shows a program the difference SET DECIMALS is hiding.
fn memory_value(value: &Value, settings: &Settings) -> String {
    let value = value.deref();
    let shown = crate::value::display(&value, settings);
    match &value {
        Value::Str(_) => format!("C  \"{shown}\""),
        Value::Number(n, ..) => format!("N  {shown:<12}({:>19.8})", n),
        _ => format!("{}  {shown}", value.vartype()),
    }
}

fn type_word(kind: char) -> &'static str {
    match kind {
        'C' => "Character",
        'N' => "Numeric",
        'F' => "Float",
        'I' => "Integer",
        'B' => "Double",
        'Y' => "Currency",
        'L' => "Logical",
        'D' => "Date",
        'T' => "DateTime",
        'M' => "Memo",
        'G' => "General",
        'P' => "Picture",
        _ => "Character",
    }
}

/// One column of a `LIST` or `DISPLAY` of records: the field it shows and how wide it is.
struct ListColumn {
    field: crate::dbf::DbfField,
    width: usize,
    /// Numbers line up on the right, and so do their headings; everything else on the left.
    right: bool,
}

impl ListColumn {
    /// Adds one cell and the space that follows it.
    fn write(&self, line: &mut String, text: &str) {
        if self.right {
            line.push_str(&format!("{text:>0$}", self.width));
        } else {
            line.push_str(&format!("{text:<0$}", self.width));
        }
        line.push(' ');
    }
}

/// How wide a listing writes one field: as wide as the value it shows, or as wide as the field's
/// own name when the name is the longer of the two.
///
/// The widths of the values were read off Visual FoxPro rather than worked out: a date is as wide
/// as SET CENTURY makes it, a datetime as wide as SET CENTURY, SET HOURS and SET SECONDS make it,
/// a logical is the three characters of `.T.`, a memo the four of `memo`, and an integer, a
/// currency and a double have fixed widths of their own that owe nothing to how they are stored.
fn list_column(field: &crate::dbf::DbfField, settings: &Settings) -> ListColumn {
    let shown = match field.kind {
        'I' => 11,
        'Y' | 'B' => 21,
        'L' => 3,
        'M' | 'G' | 'P' => 4,
        'D' => crate::value::display(&Value::Date(Some(0)), settings).len(),
        'T' => crate::value::display(&Value::DateTime(Some(0.0)), settings).len(),
        _ => field.width(true),
    };
    ListColumn {
        field: field.clone(),
        width: shown.max(field.name.chars().count()),
        right: matches!(field.kind, 'N' | 'F' | 'I' | 'Y' | 'B'),
    }
}

/// A report on its way through: the file, what was read from it, and the lines it has made.
struct RunningReport {
    path: String,
    to_file: String,
    flags: u8,
    /// The report table, between asking for it and asking for the memo beside it.
    frx: Option<Vec<u8>>,
    report: Option<crate::report::Report>,
    lines: Vec<String>,
    /// How many lines are on the page so far, for the page break.
    on_page: usize,
}

/// How many lines a printed page holds, which is what a report breaks at.
const PAGE_LINES: usize = 66;

impl RunningReport {
    fn new(path: String, to_file: String, flags: u8) -> RunningReport {
        RunningReport { path, to_file, flags, frx: None, report: None, lines: Vec::new(), on_page: 0 }
    }

    fn add(&mut self, lines: Vec<String>) {
        for line in lines {
            if self.on_page >= PAGE_LINES {
                self.page_break();
            }
            self.lines.push(line);
            self.on_page += 1;
        }
    }

    /// `EJECT`, and what happens on its own when the page is full.
    fn page_break(&mut self) {
        self.lines.push("\u{c}".to_string());
        self.on_page = 0;
    }
}

/// One stored value as the language sees it, for the rows a Browse window is given.
fn value_of_dbf(value: Option<&crate::dbf::DbfValue>) -> Value {
    match value {
        None | Some(crate::dbf::DbfValue::Null) => Value::Null,
        Some(crate::dbf::DbfValue::Text(text)) | Some(crate::dbf::DbfValue::Memo(text)) => Value::str(text.clone()),
        Some(crate::dbf::DbfValue::Number(n)) => Value::number(*n),
        Some(crate::dbf::DbfValue::Currency(c)) => Value::Currency(*c),
        Some(crate::dbf::DbfValue::Logical(b)) => Value::Logical(*b),
        Some(crate::dbf::DbfValue::Date(days)) => Value::Date(*days),
        Some(crate::dbf::DbfValue::DateTime(secs)) => Value::DateTime(*secs),
        Some(crate::dbf::DbfValue::Bytes(bytes)) => Value::str(String::from_utf8_lossy(bytes).to_string()),
    }
}

/// The menu `DEFINE PAD` and the `ON` commands mean when they name none: Visual FoxPro's
/// own bar across the top, which is the only place a menu goes here.
const SYSTEM_MENU: &str = "_MSYSMENU";

/// A name a menu command took off the stack.
fn menu_str(slot: Option<&Option<Value>>) -> String {
    slot.and_then(|v| v.as_ref())
        .and_then(|v| v.as_str().ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// A bar number a menu command took off the stack.
fn menu_num(slot: Option<&Option<Value>>) -> i32 {
    slot.and_then(|v| v.as_ref()).and_then(|v| v.as_number().ok()).unwrap_or(0.0) as i32
}

/// The value an instruction left on the stack.
fn pop_value(fb: &mut Fiber) -> Result<Value, RtError> {
    fb.stack.pop().ok_or_else(|| RtError::new(0, "Stack underflow"))
}

/// One change `ALTER TABLE` makes, as the instruction carries it.
struct Alteration {
    /// 0 add, 1 alter, 2 drop, 3 rename.
    kind: u8,
    name: String,
    field: Option<crate::dbf::DbfField>,
    rename: Option<String>,
}

/// What a piece of source compiled while the program runs is meant to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Inline {
    /// `&cName` and EVALUATE(): its value is wanted.
    Expression,
    /// A line of a program that has not been through macro substitution.
    Line,
    /// A line a macro was spliced into, which has. It is read as it now stands: substitution
    /// happens once per line, so a `&name` still standing in it is text and not another macro.
    ExpandedLine,
    /// A name to write the value already on the stack to: `STORE x TO (cName)`.
    StoreTo,
    /// A whole program: EXECSCRIPT(). It may declare procedures and classes of its own, so it
    /// is read the way a `.prg` is rather than as a method body.
    Program,
}

/// `ALTER TABLE` in progress.
struct Alter {
    /// 0 opening the table, 1 waiting for its records, 2 writing it back, 3 done.
    stage: u8,
    handle: u32,
    path: String,
    /// The handle came from a work area that already had the table open.
    borrowed: bool,
}

/// The columns and records a table has after the changes: every record is read with the old
/// columns and written with the new ones, so a column that changed type keeps what fits.
fn alter_records(
    header: &crate::dbf::DbfHeader,
    bytes: &[u8],
    ops: &[Alteration],
) -> Result<(Vec<crate::dbf::DbfField>, Vec<(Vec<Value>, Vec<Value>)>), RtError> {
    let mut fields = header.fields.clone();
    for op in ops {
        match op.kind {
            0 => {
                if let Some(field) = &op.field
                    && !fields.iter().any(|f| f.name.eq_ignore_ascii_case(&field.name))
                {
                    fields.push(field.clone());
                }
            }
            1 => {
                if let Some(field) = &op.field
                    && let Some(slot) = fields.iter_mut().find(|f| f.name.eq_ignore_ascii_case(&op.name))
                {
                    *slot = field.clone();
                }
            }
            2 => fields.retain(|f| !f.name.eq_ignore_ascii_case(&op.name)),
            _ => {
                if let Some(new) = &op.rename
                    && let Some(slot) = fields.iter_mut().find(|f| f.name.eq_ignore_ascii_case(&op.name))
                {
                    slot.name = new.clone();
                }
            }
        }
    }
    // a renamed column is found under its old name in the records that were read
    let previous: Vec<String> = fields
        .iter()
        .map(|f| {
            ops.iter()
                .find(|o| o.kind == 3 && o.rename.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&f.name)))
                .map(|o| o.name.clone())
                .unwrap_or_else(|| f.name.clone())
        })
        .collect();

    let mut rows = Vec::new();
    for chunk in bytes.chunks(header.record_len.max(1)) {
        if chunk.len() < header.record_len || chunk.first() == Some(&0x1A) {
            break;
        }
        let record = crate::dbf::decode_record(header, chunk, crate::dbf::Padding::Keep, |_| None);
        let row: Vec<Value> = previous
            .iter()
            .map(|name| match header.field_index(name) {
                Some(i) => record.values.get(i).map(crate::data::value_of).unwrap_or(Value::Null),
                None => Value::Null,
            })
            .collect();
        // a value that does not fit its new column becomes the empty one, as VFP leaves it
        let row = fields
            .iter()
            .zip(row)
            .map(|(f, v)| if fits(f.kind, &v) { v } else { crate::data::empty_of(f.kind) })
            .collect();
        rows.push((Vec::new(), row));
    }
    Ok((fields, rows))
}

/// Whether a value can be written to a column of that type.
fn fits(kind: char, value: &Value) -> bool {
    match kind {
        'C' | 'M' | 'G' | 'P' => matches!(value.deref(), Value::Str(_)),
        'N' | 'F' | 'I' | 'B' | 'Y' => matches!(value.deref(), Value::Number(..)),
        'L' => matches!(value.deref(), Value::Logical(_)),
        'D' => matches!(value.deref(), Value::Date(_)),
        'T' | '@' => matches!(value.deref(), Value::DateTime(_)),
        _ => true,
    }
}

/// `PACK` in progress: the records that are staying, where each of them came from, and how
/// far the writing has got.
struct Pack {
    /// The records are written and the index with them: nothing is left to do.
    done: bool,
    rows: std::collections::VecDeque<Vec<u8>>,
    /// (old record number, new record number) for every record that is staying.
    moved: Vec<(u32, u32)>,
    next: u64,
    total: u64,
}

/// `INSERT [BEFORE] [BLANK]` in progress on a table in a file: where the new record goes, the
/// record itself, and the records from there down that are still to be written one lower.
struct Shift {
    at: u64,
    blank: Vec<u8>,
    rows: std::collections::VecDeque<Vec<u8>>,
    /// The record number the next write goes to; 0 until the records have been read.
    next: u64,
}

/// `APPEND FROM` in progress: which file, how far it has got, and the records still to add.
struct AppendFrom {
    /// 0 opening the file, 1 waiting for its records, 2 adding them.
    stage: u8,
    handle: u32,
    rows: std::collections::VecDeque<Vec<(String, Value)>>,
    names: Vec<String>,
    except: bool,
    cond: Option<String>,
    text: u8,
}

/// The records of a text file: one line each, split into the values the format holds.
fn text_rows(text: &str, kind: u8, target: &[(String, char, usize)]) -> Vec<Vec<(String, Value)>> {
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // a fixed-width line has no separators, so it is one value the caller lays out by
        // position; a delimited one is split on commas outside quotes
        let cells: Vec<String> = if kind == 1 { fixed_width(line, target) } else { split_delimited(line) };
        rows.push(
            cells
                .into_iter()
                .enumerate()
                .filter_map(|(i, c)| target.get(i).map(|(name, kind, _)| (name.clone(), cell_value(&c, *kind))))
                .collect(),
        );
    }
    rows
}

/// One line of a fixed-width file: a cell per column, each as wide as the column it fills.
fn fixed_width(line: &str, target: &[(String, char, usize)]) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut at = 0usize;
    let mut cells = Vec::with_capacity(target.len());
    for (_, _, width) in target {
        let end = (at + width).min(chars.len());
        cells.push(chars.get(at..end).map(|c| c.iter().collect::<String>()).unwrap_or_default().trim().to_string());
        at = end;
    }
    cells
}

/// A cell of a text file as the column it fills holds it: text stays text, and everything
/// else is read the way `VAL()` and `CTOD()` read it.
fn cell_value(cell: &str, kind: char) -> Value {
    let text = cell.trim();
    match kind {
        'N' | 'F' | 'I' | 'B' | 'Y' => Value::number(text.parse::<f64>().unwrap_or(0.0)),
        'L' => Value::Logical(matches!(text.chars().next(), Some('T' | 't' | 'Y' | 'y' | '1'))),
                // a date is stored as yyyymmdd in a table, and that is what a text copy writes
        'D' if text.len() == 8 => {
            let part = |a: usize, b: usize| text[a..b].parse::<i32>().unwrap_or(0);
            Value::Date(Some(crate::value::days_from_civil(part(0, 4), part(4, 6) as u32, part(6, 8) as u32)))
        }
        'D' => Value::Date(None),
        _ => Value::str(cell),
    }
}

/// One line of a delimited file: commas between the values, quotes around the text.
fn split_delimited(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' | '\'' => quoted = !quoted,
            ',' if !quoted => cells.push(std::mem::take(&mut cell)),
            _ => cell.push(c),
        }
    }
    cells.push(cell);
    cells.into_iter().map(|c| c.trim().to_string()).collect()
}

/// A table being made from the one in the work area: what it is to hold and what it has
/// gathered so far. Each row is its sort keys and its field values.
struct CopyOut {
    path: String,
    /// 0 the records, 1 the structure alone, 2 sorted, 3 totalled.
    kind: u8,
    fields: Vec<crate::dbf::DbfField>,
    descending: u32,
    /// 0 a table, 1 fixed-width text, 2 comma-separated, 3 delimited.
    text: u8,
    quote: String,
    separator: String,
    rows: Vec<(Vec<Value>, Vec<Value>)>,
}

/// The table `COPY STRUCTURE EXTENDED` writes: one record per field of the table in hand.
///
/// The names, widths and order are Visual FoxPro 9's own, read back from a table it wrote.
/// FIELD_NULL and FIELD_NOCP are always false here, because a table in this engine carries
/// neither the null flag nor the no-translation flag - not because the field is not nullable.
fn structure_fields() -> Vec<crate::dbf::DbfField> {
    use crate::dbf::DbfField as F;
    vec![
        F::new("FIELD_NAME", 'C', 128, 0),
        F::new("FIELD_TYPE", 'C', 1, 0),
        F::new("FIELD_LEN", 'N', 3, 0),
        F::new("FIELD_DEC", 'N', 3, 0),
        F::new("FIELD_NULL", 'L', 1, 0),
        F::new("FIELD_NOCP", 'L', 1, 0),
        F::new("FIELD_DEFA", 'M', 4, 0),
        F::new("FIELD_RULE", 'M', 4, 0),
        F::new("FIELD_ERR", 'M', 4, 0),
        F::new("TABLE_RULE", 'M', 4, 0),
        F::new("TABLE_ERR", 'M', 4, 0),
        F::new("TABLE_NAME", 'C', 128, 0),
        F::new("INS_TRIG", 'M', 4, 0),
        F::new("UPD_TRIG", 'M', 4, 0),
        F::new("DEL_TRIG", 'M', 4, 0),
        F::new("TABLE_CMT", 'M', 4, 0),
        F::new("FIELD_NEXT", 'N', 4, 0),
        F::new("FIELD_STEP", 'N', 4, 0),
    ]
}

/// One record per field, in the order `structure_fields` puts the columns.
fn structure_rows(fields: &[crate::dbf::DbfField]) -> Vec<(Vec<Value>, Vec<Value>)> {
    fields
        .iter()
        .map(|f| {
            let row = vec![
                Value::str(f.name.clone()),
                Value::str(f.kind.to_string()),
                Value::number(f.length as f64),
                Value::number(f.decimals as f64),
                Value::Logical(false),
                Value::Logical(false),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::str(""),
                Value::number(f.autoinc_next as f64),
                Value::number(f.autoinc_step as f64),
            ];
            (Vec::new(), row)
        })
        .collect()
}

/// The fields a description describes: one per record of a structure-extended table.
///
/// The columns are found by name rather than by position, because the reference lets a program
/// build the description itself - "created either manually or with COPY STRUCTURE EXTENDED" -
/// and a hand-made one need not have all sixteen of the others.
fn fields_from_description(
    columns: &[crate::dbf::DbfField],
    rows: &[(Vec<Value>, Vec<Value>)],
) -> Result<Vec<crate::dbf::DbfField>, RtError> {
    let at = |name: &str| columns.iter().position(|c| c.name.eq_ignore_ascii_case(name));
    let (Some(name_at), Some(type_at)) = (at("FIELD_NAME"), at("FIELD_TYPE")) else {
        return Err(RtError::new(
            1707,
            "CREATE ... FROM: the table it reads has no FIELD_NAME and FIELD_TYPE, so it does not \
             describe a structure; COPY STRUCTURE EXTENDED writes one that does"
                .to_string(),
        ));
    };
    let (len_at, dec_at, next_at, step_at) = (at("FIELD_LEN"), at("FIELD_DEC"), at("FIELD_NEXT"), at("FIELD_STEP"));
    let text = |row: &[Value], i: usize| row.get(i).map(|v| v.deref()).and_then(|v| v.as_str().ok().map(|s| s.trim().to_string()));
    let number = |row: &[Value], i: Option<usize>| {
        i.and_then(|i| row.get(i)).map(|v| v.deref()).and_then(|v| v.as_number().ok()).unwrap_or(0.0)
    };

    let mut out = Vec::with_capacity(rows.len());
    for (_, row) in rows {
        let name = text(row, name_at).unwrap_or_default();
        let kind = text(row, type_at).unwrap_or_default().chars().next().unwrap_or('C');
        if name.is_empty() {
            continue;
        }
        out.push(crate::dbf::DbfField {
            name: name.to_ascii_uppercase(),
            kind: kind.to_ascii_uppercase(),
            length: number(row, len_at).clamp(0.0, 254.0) as u8,
            decimals: number(row, dec_at).clamp(0.0, 254.0) as u8,
            autoinc_step: number(row, step_at).clamp(0.0, 255.0) as u8,
            autoinc_next: number(row, next_at).max(0.0) as u32,
            nullable: false,
        });
    }
    if out.is_empty() {
        return Err(RtError::new(1707, "CREATE ... FROM: the table it reads describes no fields".to_string()));
    }
    Ok(out)
}

/// The columns a FIELDS clause chose, or all of them.
fn chosen_fields(cursor: &Cursor, names: &[String], except: bool) -> Vec<crate::dbf::DbfField> {
    cursor
        .header
        .fields
        .iter()
        .filter(|f| {
            if names.is_empty() {
                true
            } else if except {
                !names.iter().any(|n| n.eq_ignore_ascii_case(&f.name))
            } else {
                names.iter().any(|n| n.eq_ignore_ascii_case(&f.name))
            }
        })
        .cloned()
        .collect()
}

/// Two sort keys, in the order the values themselves sort in.
fn compare_keys(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a.deref(), b.deref()) {
        (Value::Str(x), Value::Str(y)) => x.as_ref().cmp(y.as_ref()),
        (Value::Number(x, ..), Value::Number(y, ..)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Date(x), Value::Date(y)) => x.cmp(&y),
        (Value::DateTime(x), Value::DateTime(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Logical(x), Value::Logical(y)) => x.cmp(&y),
        _ => std::cmp::Ordering::Equal,
    }
}

/// `TOTAL ON`: one row per run of records with the same key, its numbers added up. The records
/// have to be in the order of the key already, which is what the command asks of the caller.
fn total_runs(
    fields: &[crate::dbf::DbfField],
    rows: Vec<(Vec<Value>, Vec<Value>)>,
) -> Vec<(Vec<Value>, Vec<Value>)> {
    let numeric: Vec<bool> = fields.iter().map(|f| matches!(f.kind, 'N' | 'F' | 'I' | 'B' | 'Y')).collect();
    let mut out: Vec<(Vec<Value>, Vec<Value>)> = Vec::new();
    for (key, row) in rows {
        let same = out.last().is_some_and(|(k, _): &(Vec<Value>, Vec<Value>)| {
            k.iter().zip(&key).all(|(a, b)| compare_keys(a, b) == std::cmp::Ordering::Equal)
        });
        match (same, out.last_mut()) {
            (true, Some((_, totals))) => {
                for (i, value) in row.into_iter().enumerate() {
                    if numeric.get(i) == Some(&true) {
                        let sum = totals[i].as_number().unwrap_or(0.0) + value.as_number().unwrap_or(0.0);
                        totals[i] = Value::number(sum);
                    }
                }
            }
            _ => out.push((key, row)),
        }
    }
    out
}

/// A table of its own: the header for those columns, then a record each.
fn build_table(
    fields: &[crate::dbf::DbfField],
    rows: &[(Vec<Value>, Vec<Value>)],
    codepage: Option<u16>,
) -> Result<Vec<u8>, RtError> {
    let (mut out, _) = crate::dbf::write::encode_header(fields);
    out[4..8].copy_from_slice(&(rows.len() as u32).to_le_bytes());
    // a column that accepts a null puts its bit at the end of the record, in field order
    let nulls = fields.iter().filter(|f| f.nullable).count();
    for (_, row) in rows {
        out.push(b' ');
        let mut flags = vec![0u8; nulls.div_ceil(8)];
        let mut bit = 0usize;
        for (field, value) in fields.iter().zip(row) {
            out.extend_from_slice(&crate::dbf::write::encode_field(field, field.length as usize, value, codepage)?);
            if field.nullable {
                if matches!(value.deref(), Value::Null) {
                    flags[bit / 8] |= 1 << (bit % 8);
                }
                bit += 1;
            }
        }
        out.extend_from_slice(&flags);
    }
    Ok(out)
}

/// The path with an extension when it was written without one.
fn defaulted(path: &str, extension: &str) -> String {
    let file = path.trim().rsplit(['/', '\\']).next().unwrap_or(path);
    if file.contains('.') || file.is_empty() { path.trim().to_string() } else { format!("{}.{extension}", path.trim()) }
}

/// A text file of the records: fixed-width columns, one line of separated values each, or
/// one of the two interchange formats `EXPORT` writes.
fn text_file(copy: &CopyOut, settings: &Settings) -> String {
    if copy.text == 4 || copy.text == 5 {
        let rows: Vec<&[Value]> = copy.rows.iter().map(|(_, row)| row.as_slice()).collect();
        return if copy.text == 4 {
            crate::exchange::dif(&copy.fields, &rows, settings)
        } else {
            crate::exchange::sylk(&copy.fields, &rows, settings)
        };
    }
    let mut out = String::new();
    for (_, row) in &copy.rows {
        let mut line = String::new();
        for (i, (field, value)) in copy.fields.iter().zip(row).enumerate() {
            let text = match value.deref() {
                Value::Str(s) => s.to_string(),
                Value::Logical(b) => if b { "T".to_string() } else { "F".to_string() },
                Value::Number(n, ..) => crate::value::number_text(n, field.decimals, false),
                other => crate::value::display(&other, &Settings::default()),
            };
            match copy.text {
                // fixed width: every field its own width, nothing between them, and a number
                // against the right of its column as it is written in the record
                1 => {
                    let width = field.length as usize;
                    let mut cell: String = text.chars().take(width).collect();
                    let numeric = matches!(value.deref(), Value::Number(..));
                    while cell.chars().count() < width {
                        if numeric {
                            cell.insert(0, ' ');
                        } else {
                            cell.push(' ');
                        }
                    }
                    line.push_str(&cell);
                }
                _ => {
                    let (quote, separator) = if copy.text == 2 {
                        ("\"".to_string(), ",".to_string())
                    } else {
                        (copy.quote.clone(), copy.separator.clone())
                    };
                    if i > 0 {
                        line.push_str(&separator);
                    }
                    if matches!(value.deref(), Value::Str(_)) && !quote.is_empty() {
                        line.push_str(&quote);
                        line.push_str(text.trim_end());
                        line.push_str(&quote);
                    } else {
                        line.push_str(text.trim());
                    }
                }
            }
        }
        out.push_str(line.trim_end());
        out.push_str("\r\n");
    }
    out
}

/// One column of a `CALCULATE` while the records are being walked: what it is working out,
/// and enough of what it has seen to say the answer at the end.
struct Agg {
    /// 0 CNT, 1 SUM, 2 AVG, 3 MIN, 4 MAX, 5 STD, 6 VAR, 7 NPV.
    func: u8,
    /// How many records went in, which is what CNT answers and what AVG divides by.
    count: f64,
    total: f64,
    /// The sum of the squares, for the two that measure spread.
    squares: f64,
    /// The most places past the point any record showed. A total of a `N(8,2)` column is
    /// written to two places even when it lands on a whole number, as the column would be.
    decimals: u8,
    /// The smallest or largest so far, which is a value rather than a number because MIN and
    /// MAX work on dates and text as well.
    best: Option<Value>,
}

impl Agg {
    fn new(func: u8) -> Agg {
        Agg { func, count: 0.0, total: 0.0, squares: 0.0, decimals: 0, best: None }
    }

    fn add(&mut self, value: &Value, rate: Option<f64>, settings: &Settings) -> Result<(), RtError> {
        self.count += 1.0;
        self.decimals = self.decimals.max(value.width().decimals);
        match self.func {
            0 => {}
            3 | 4 => {
                let better = match &self.best {
                    None => true,
                    Some(best) => {
                        let op = if self.func == 3 { crate::value::CmpOp::Lt } else { crate::value::CmpOp::Gt };
                        crate::value::compare(value, best, op, settings)?.truthy().unwrap_or(false)
                    }
                };
                if better {
                    self.best = Some(value.deref());
                }
            }
            7 => {
                // the flow of the nth record, discounted by the rate for n periods
                let rate = rate.unwrap_or(0.0);
                let n = self.count;
                self.total += value.as_number()? / (1.0 + rate).powf(n);
            }
            _ => {
                let n = value.as_number()?;
                self.total += n;
                self.squares += n * n;
            }
        }
        Ok(())
    }

    fn result(self) -> Value {
        let mean = if self.count > 0.0 { self.total / self.count } else { 0.0 };
        // the spread of the records that were counted, which is all of them: the population
        // formula, not the sample one
        let variance = if self.count > 0.0 { self.squares / self.count - mean * mean } else { 0.0 };
        // what the records were written to is what the answer is written to
        let like_the_column = |n: f64| Value::Number(n, value::Width::of_variable(n, self.decimals));
        match self.func {
            0 => Value::number(self.count),
            2 => like_the_column(mean),
            3 | 4 => self.best.unwrap_or(Value::number(0.0)),
            5 => like_the_column(variance.max(0.0).sqrt()),
            6 => like_the_column(variance.max(0.0)),
            _ => like_the_column(self.total),
        }
    }
}

/// `SCATTER NAME` in progress: the object once the host has made it, and the properties still
/// to add to it. One property is one request, so the instruction runs once per field.
struct ScatterName {
    object: Option<Handle>,
    left: std::collections::VecDeque<(String, Value)>,
}

/// The columns of a definition whose names the program worked out are left blank when it is
/// compiled; these are the names it came to, in the order the columns were written.
fn name_blank_columns(fields: &mut [crate::dbf::DbfField], worked: Vec<String>) {
    let mut worked = worked.into_iter();
    for field in fields.iter_mut().filter(|f| f.name.is_empty()) {
        field.name = worked.next().unwrap_or_default();
    }
}

/// The field names an instruction left on the stack, in the order they were written.
fn pop_names(fb: &mut Fiber, count: u16) -> Result<Vec<String>, RtError> {
    let mut names = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let v = fb.stack.pop().ok_or_else(|| RtError::new(0, "Stack underflow"))?;
        names.push(v.as_str()?.trim().to_ascii_uppercase());
    }
    names.reverse();
    Ok(names)
}

/// The tag `INDEX ON` is building: what the command said, and a key per record as the loop
/// over the table reaches it.
struct IndexBuild {
    area: usize,
    name: String,
    key_expr: String,
    for_expr: String,
    unique: bool,
    candidate: bool,
    descending: bool,
    /// `INDEX ON ... TO file`: the single-entry index this is building, and which of the two
    /// layouts to write it in. A tag of the compound index has neither.
    to_file: Option<String>,
    compact: bool,
    keys: Vec<(Value, u32)>,
}

/// The single-entry indexes a command is opening beside a table, one read at a time.
struct IdxOpen {
    area: usize,
    /// The files still to be read, and the one whose bytes are on their way.
    left: std::collections::VecDeque<String>,
    reading: Option<String>,
    /// Where the first file this command opened landed, which is the one that controls the
    /// order when the command did not name another.
    first: Option<usize>,
    /// What the command asked to control the order afterwards, and which way it runs.
    order: Value,
    descending: Option<bool>,
}

/// `COPY INDEXES`: the single-entry indexes left to read, and the tags they have made so far.
struct CopyIndexes {
    /// The work area the indexes belong to.
    area: usize,
    left: std::collections::VecDeque<String>,
    /// The file whose bytes are on their way.
    reading: Option<String>,
    /// The compound index the tags go in, empty for the structural one beside the table.
    cdx: String,
    /// The tags gathered so far, and whether that compound index has been read and merged.
    tags: Vec<crate::cdx::Tag>,
    read_cdx: bool,
}

/// `COPY TAG`: the tag being written out and where it is going, while the compound index it
/// comes from is on its way.
struct CopyTag {
    name: String,
    of: String,
    target: String,
    /// The tag itself, once the file named by OF has been read.
    found: Option<crate::cdx::Tag>,
    read: bool,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

/// The system variables a program finds already there.
///
/// Visual FoxPro makes them public before anything runs: the tool ones name the program that
/// does a job the development environment offers, the platform ones say which machine this is,
/// and the rest are settings and counters the language reads and writes. A program that sets one
/// of the tool variables is pointing at its own program, which is why they are ordinary
/// variables rather than anything cleverer.
///
/// Every value here was read off Visual FoxPro 9 with `TYPE()` and `EVALUATE()` over the whole
/// list rather than off a reference page; the type matters as much as the number, because
/// `_CALCMEM` is a number and not the empty string a page might lead you to write. The tool
/// variables are the one place this list departs from the product: VFP names the file inside
/// its own installation folder, and this runtime has its own, so it names the program without
/// a path.
fn system_variables() -> Vec<(&'static str, Value)> {
    let text = |s: &str| Value::str(s.to_string());
    vec![
        // the character-mode screen VFP can draw on, which this runtime does not have
        ("_ASCIICOLS", Value::number(80.0)),
        ("_ASCIIROWS", Value::number(63.0)),
        // The programs the development environment runs for a job. Visual FoxPro spells each as
        // a full path into wherever it was installed; the file name is the part that means
        // anything here. The five that are empty were measured empty in the product too - there
        // is no spell checker, no shell and no expression builder shipped for them to name.
        ("_BEAUTIFY", text("beautify.app")),
        ("_BROWSER", text("browser.app")),
        ("_BUILDER", text("builder.app")),
        ("_CODESENSE", text("foxcode.app")),
        ("_CONVERTER", text("convert.app")),
        ("_COVERAGE", text("coverage.app")),
        ("_FOXCODE", text("foxcode.dbf")),
        ("_FOXDOC", text("")),
        ("_FOXGRAPH", text("")),
        ("_FOXREF", text("foxref.app")),
        ("_FOXTASK", text("foxtask.dbf")),
        ("_GALLERY", text("gallery.app")),
        ("_GENGRAPH", text("")),
        ("_GENHTML", text("genhtml.prg")),
        ("_GENMENU", text("genmenu.prg")),
        ("_GENPD", text("")),
        ("_GENSCRN", text("")),
        ("_GENXTAB", text("vfpxtab.prg")),
        ("_GETEXPR", text("")),
        ("_INCLUDE", text("")),
        ("_OBJECTBROWSER", text("objectbrowser.app")),
        ("_REPORTBUILDER", text("reportbuilder.app")),
        ("_REPORTOUTPUT", text("reportoutput.app")),
        ("_REPORTPREVIEW", text("reportpreview.app")),
        ("_SAMPLES", text("")),
        ("_SCCTEXT", text("scctext.prg")),
        ("_SHELL", text("")),
        ("_SPELLCHK", text("")),
        ("_STARTUP", text("")),
        ("_TASKLIST", text("tasklist.app")),
        ("_TASKPANE", text("taskpane.app")),
        ("_TOOLBOX", text("toolbox.app")),
        ("_WIZARD", text("wizard.app")),
        // the machine this is
        ("_DOS", Value::Logical(false)),
        ("_MAC", Value::Logical(false)),
        ("_UNIX", Value::Logical(false)),
        ("_WINDOWS", Value::Logical(true)),
        // how ? and TEXT lay text out on the page
        ("_ALIGNMENT", text("LEFT")),
        ("_BOX", Value::Logical(true)),
        ("_INDENT", Value::number(0.0)),
        ("_LMARGIN", Value::number(0.0)),
        ("_RMARGIN", Value::number(80.0)),
        ("_WRAP", Value::Logical(false)),
        // the printer, and where a report has got to on it
        ("_PADVANCE", text("FORMFEED")),
        ("_PBPAGE", Value::number(1.0)),
        ("_PCOLNO", Value::number(0.0)),
        ("_PCOPIES", Value::number(1.0)),
        ("_PDRIVER", text("")),
        ("_PDSETUP", text("")),
        ("_PECODE", text("")),
        ("_PEJECT", text("NONE")),
        ("_PEPAGE", Value::number(32767.0)),
        ("_PFORM", text("")),
        ("_PLENGTH", Value::number(66.0)),
        ("_PLINENO", Value::number(0.0)),
        ("_PLOFFSET", Value::number(0.0)),
        ("_PPITCH", text("DEFAULT")),
        ("_PQUALITY", Value::Logical(false)),
        ("_PSCODE", text("")),
        ("_PSPACING", Value::number(1.0)),
        ("_PWAIT", Value::Logical(false)),
        // what the language reads and writes as it runs
        ("_CALCMEM", Value::number(0.0)),
        ("_CALCVALUE", Value::number(0.0)),
        ("_CLIPTEXT", text("")),
        ("_CUROBJ", Value::number(0.0)),
        ("_DBLCLICK", Value::number(0.5)),
        ("_DIARYDATE", Value::Date(None)),
        ("_INCSEEK", Value::number(0.5)),
        ("_MLINE", Value::number(0.0)),
        ("_PAGENO", Value::number(1.0)),
        ("_PAGETOTAL", Value::number(0.0)),
        ("_PRETEXT", text("")),
        ("_TABS", text("")),
        ("_TALLY", Value::number(0.0)),
        ("_TEXT", Value::number(-1.0)),
        ("_THROTTLE", Value::number(0.0)),
        ("_TRIGGERLEVEL", Value::number(0.0)),
    ]
}

/// The type a system variable is fixed at, or `None` for a name that is not one.
///
/// This is what tells a system variable from an ordinary PUBLIC one: a public variable takes
/// whatever it is handed and changes type with it, while `_PAGENO = "page one"` is error 9.
/// Visual FoxPro refuses even a currency where a number belongs, so the whole VARTYPE letter
/// has to match and not merely the kind of thing it is.
fn system_var_type(upper: &str) -> Option<char> {
    thread_local! {
        static TYPES: HashMap<&'static str, char> =
            system_variables().into_iter().map(|(n, v)| (n, v.vartype())).collect();
    }
    TYPES.with(|t| t.get(upper).copied())
}

impl Vm {
    pub fn new() -> Vm {
        Vm {
            settings: Settings::default(),
            globals: system_variables().into_iter().map(|(n, v)| (n.to_string(), v)).collect(),
            modules: Vec::new(),
            programs: HashMap::new(),
            fibers: HashMap::new(),
            next_fiber: 1,
            functions: HashMap::new(),
            next_function: 1,
            natives: crate::foxscript::Natives::default(),
            on_error: None,
            list_heading_due: false,
            pictures: crate::picture::Pictures::default(),
            inline_cache: HashMap::new(),
            dlls: HashMap::new(),
            libraries: Vec::new(),
            library_funcs: HashMap::new(),
            procedure_files: Vec::new(),
            procedure_only: HashSet::new(),
            last_opened: None,
            index_build: None,
            idx_open: None,
            copy_indexes: None,
            copy_tag: None,
            aggregates: Vec::new(),
            copy: None,
            append: None,
            pack: None,
            shift: None,
            alter: None,
            alter_names: Vec::new(),
            txn: 0,
            databases: Vec::new(),
            current_db: None,
            db_io: None,
            opened_database: None,
            open_clauses: 0,
            db_await: None,
            menus: crate::menu::Menus::default(),
            screen: crate::screen::Screen::default(),
            report: None,
            handlers: HashMap::new(),
            breakpoints: Vec::new(),
            key_stack: Vec::new(),
            sql_named: None,
            memo_io: None,
            area_stack: Vec::new(),
            com_arrays: HashMap::new(),
            com_props: HashMap::new(),
            sql_props: HashMap::new(),
            cursor_flags: std::collections::HashSet::new(),
            result_set: 0,
            to_release: Vec::new(),
            source_header: None,
            scatter_name: None,
            ferror: 0,
            textmerge_held: String::new(),
            textmerge_begun: false,
            textmerge_written: false,
            in_class_error: HashSet::new(),
            data: DataSession::new(),
        }
    }

    /// The work areas, for the built-ins that report on them.
    pub fn data(&self) -> &DataSession {
        &self.data
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    /// Registers a module; Program modules are also findable by name for `DO prog`.
    pub fn load_module(&mut self, m: Module) -> u32 {
        let id = self.modules.len() as u32;
        if m.kind == ModuleKind::Program {
            self.programs.insert(m.name.to_ascii_uppercase(), id);
        }
        self.modules.push(Rc::new(m));
        id
    }

    pub fn module(&self, id: u32) -> &Module {
        &self.modules[id as usize]
    }

    /// Id of a loaded Program module by name.
    pub fn find_program(&self, name: &str) -> Option<u32> {
        self.programs.get(&name.to_ascii_uppercase()).copied()
    }

    /// A `DEFINE CLASS` by name in any loaded module, the most recently loaded first (VFP's
    /// "the last definition wins" for classes of the same name).
    pub fn find_class(&self, name: &str) -> Option<(u32, &ClassProto)> {
        self.modules.iter().enumerate().rev().find_map(|(id, m)| m.find_class(name).map(|c| (id as u32, c)))
    }

    /// The `DEFINE CLASS` declarations of a module (empty for an unknown module id).
    pub fn classes(&self, module: u32) -> &[ClassProto] {
        self.modules.get(module as usize).map(|m| m.classes.as_slice()).unwrap_or(&[])
    }

    /// The class definition `CREATEOBJECT(name)` should carry: the running module's own classes
    /// win, then every loaded module. `None` leaves the class to the host (`Custom`, `Form`, ...).
    fn class_definition(&self, current: Option<u32>, name: &str) -> Option<ClassDefOut> {
        if let Some(id) = current
            && let Some(c) = self.modules.get(id as usize).and_then(|m| m.find_class(name))
        {
            return Some(ClassDefOut::new(id, c));
        }
        self.find_class(name).map(|(id, c)| ClassDefOut::new(id, c))
    }

    /// What a class of that name is made of, without making one: the members it declares and
    /// what each starts out holding.
    ///
    /// A class the program defines stands on another class, which stands on another, until a
    /// Visual FoxPro base class is reached; each layer lays its own declarations over what it
    /// came from, so a subclass's value wins. Nothing here is native but what the base class
    /// itself declares, which is what AMEMBERS() means by the word.
    fn class_members(&self, current: Option<u32>, class: &str) -> Option<Vec<crate::host::MemberInfo>> {
        use crate::host::{MemberInfo, MemberKind};
        /// The nearest layer to declare a member is the one whose value the class starts with;
        /// a layer further down still gets to say the member is the base class's own.
        fn push(out: &mut Vec<MemberInfo>, info: MemberInfo) {
            match out.iter_mut().find(|m| m.name == info.name) {
                Some(seen) => {
                    seen.native |= info.native;
                    seen.read_only |= info.read_only;
                }
                None => out.push(info),
            }
        }
        let mut out: Vec<MemberInfo> = Vec::new();
        let mut name = class.to_string();
        // a class that stands on itself would never end, and a thousand layers is not a hierarchy
        for _ in 0..1000 {
            let Some(def) = self.class_definition(current, &name) else { break };
            for p in &def.properties {
                push(&mut out, MemberInfo::property(&p.name, p.value.to_value()));
            }
            for m in &def.members {
                let mut info = MemberInfo::property(&m.name, Value::Logical(false));
                info.kind = MemberKind::Object;
                push(&mut out, info);
            }
            for m in &def.methods {
                // a method written for a member object belongs to that object, not to this class
                if m.name.contains('.') {
                    continue;
                }
                let mut info = MemberInfo::property(&m.name, Value::Logical(false));
                info.kind = MemberKind::Method;
                info.value = None;
                push(&mut out, info);
            }
            name = def.base_class.clone();
        }
        let Some(base) = crate::base_classes::find(&name) else {
            // nothing above it is a class this runtime knows: the name means something only if
            // a definition of the program's own answered for it
            return (!out.is_empty()).then_some(out);
        };
        for (p, default) in &base.properties {
            let mut info = MemberInfo::property(p, default.to_value());
            info.native = true;
            info.read_only = base.read_only.iter().any(|(n, _)| n == p);
            push(&mut out, info);
        }
        for p in &base.computed {
            let mut info = MemberInfo::property(p, Value::Logical(false));
            info.native = true;
            info.read_only = base.read_only.iter().any(|(n, _)| n == p);
            push(&mut out, info);
        }
        for (names, kind) in [(&base.events, MemberKind::Event), (&base.methods, MemberKind::Method)] {
            for m in names {
                let mut info = MemberInfo::property(m, Value::Logical(false));
                info.kind = kind;
                info.native = true;
                info.value = None;
                push(&mut out, info);
            }
        }
        Some(out)
    }

    /// Starts a method of a class instance: `obj_path` is `""` for the class's own method and the
    /// member's name (`"image1"`) for a member's. `None` when the class declares no such method.
    pub fn start_class_method(
        &mut self,
        module: u32,
        class: &str,
        obj_path: &str,
        event: &str,
        this: u32,
        args: Vec<Value>,
    ) -> Option<FiberId> {
        let path = if obj_path.is_empty() {
            class.to_ascii_uppercase()
        } else {
            format!("{}.{}", class.to_ascii_uppercase(), obj_path.to_ascii_uppercase())
        };
        let func = self.modules.get(module as usize)?.find_method(&path, &event.to_ascii_uppercase())?;
        Some(self.start(module, func, Some(Handle(this)), args))
    }

    pub fn set_global(&mut self, name: &str, value: Value) {
        self.globals.insert(name.to_ascii_uppercase(), value.deref());
    }

    pub fn get_global(&self, name: &str) -> Option<Value> {
        self.globals.get(&name.to_ascii_uppercase()).map(Value::deref)
    }

    pub fn on_error_handler(&self) -> Option<&str> {
        self.on_error.as_deref()
    }

    /// Creates a fiber that will run `func` of `module`. Extra arguments beyond the declared
    /// parameters are dropped (host-initiated calls are lenient).
    pub fn start(&mut self, module: u32, func: u32, this: Option<Handle>, args: Vec<Value>) -> FiberId {
        let id = self.next_fiber;
        self.next_fiber += 1;
        let mut fb = Fiber::default();
        if let Err(e) = self.push_call(&mut fb, module, func, this, args, FrameKind::Call { discard: false }, false) {
            fb.resumed = Some(Err(e));
        }
        self.fibers.insert(id, fb);
        id
    }

    /// Creates a fiber for a form method (`obj_path` relative to the form, `""` for the form).
    pub fn start_method(
        &mut self,
        module: u32,
        obj_path: &str,
        event: &str,
        this: Handle,
        args: Vec<Value>,
    ) -> Option<FiberId> {
        let func = self
            .modules
            .get(module as usize)?
            .find_method(&obj_path.to_ascii_uppercase(), &event.to_ascii_uppercase())?;
        Some(self.start(module, func, Some(this), args))
    }

    pub fn has_fiber(&self, fiber: FiberId) -> bool {
        self.fibers.contains_key(&fiber)
    }

    /// Runs the fiber until it finishes, fails or needs the host. Finished/failed fibers are removed.
    pub fn step(&mut self, host: &mut dyn Host, fiber: FiberId) -> Step {
        let Some(mut fb) = self.fibers.remove(&fiber) else {
            return Step::Error(RtError::new(0, format!("Fiber {fiber} does not exist")));
        };
        if let Some(r) = fb.resumed.take() {
            let pending = fb.pending.take();
            let outcome = match r {
                Ok(v) => self.apply_resume(host, &mut fb, pending, v),
                Err(e) => Err(e),
            };
            if let Err(e) = outcome
                && let Some(step) = self.raise(host, &mut fb, e, 0)
            {
                return step;
            }
        }
        let step = self.run(host, &mut fb, 0);
        match step {
            Step::Suspend(_) => {
                self.fibers.insert(fiber, fb);
            }
            // An unhandled error stops the fiber but does not throw it away: the host may choose
            // to carry on, which in Visual FoxPro means the next line of the program that failed.
            // The fiber waits at that line until it is resumed or aborted.
            Step::Error(_) => {
                self.rest_after_error(&mut fb);
                self.fibers.insert(fiber, fb);
            }
            Step::Done { .. } => {}
        }
        step
    }

    /// Puts a fiber that has just failed at the statement after the one that failed, so resuming
    /// it carries on rather than repeating what went wrong.
    fn rest_after_error(&mut self, fb: &mut Fiber) {
        fb.pending = None;
        fb.resumed = None;
        let Some(fr) = fb.frames.last() else { return };
        let code = &self.proto(fr.module, fr.func).code;
        let next = (fr.pc..code.len()).find(|&i| matches!(code[i], Instr::Stmt(_)));
        let stmt_sp = fr.stmt_sp;
        match next {
            Some(pc) => {
                fb.frames.last_mut().expect("frame").pc = pc;
                fb.stack.truncate(stmt_sp);
            }
            // nothing left in this frame: leave it at the end, where running it returns
            None => {
                let end = code.len().saturating_sub(1);
                fb.frames.last_mut().expect("frame").pc = end;
                fb.stack.truncate(stmt_sp);
            }
        }
    }

    /// Provides the value a suspended fiber was waiting for.
    pub fn resume(&mut self, fiber: FiberId, value: Value) {
        if let Some(fb) = self.fibers.get_mut(&fiber) {
            fb.resumed = Some(Ok(value));
        }
    }

    /// Raises `err` at the suspended instruction (catchable by TRY and ON ERROR).
    pub fn resume_error(&mut self, fiber: FiberId, err: RtError) {
        if let Some(fb) = self.fibers.get_mut(&fiber) {
            fb.pending = None;
            fb.resumed = Some(Err(err));
        }
    }

    pub fn abort(&mut self, fiber: FiberId) {
        self.fibers.remove(&fiber);
    }

    pub fn abort_all(&mut self) {
        self.fibers.clear();
    }

    /// (program, line) per frame, outermost first; inline frames are folded into their owner.
    pub fn call_stack(&self, fiber: FiberId) -> Vec<(String, u32)> {
        let Some(fb) = self.fibers.get(&fiber) else { return Vec::new() };
        fb.frames
            .iter()
            .filter(|f| !matches!(f.kind, FrameKind::Inline { .. }))
            .map(|f| (self.proto(f.module, f.func).display_name.clone(), f.line))
            .collect()
    }

    /// Evaluates expression text inside the current frame of a *suspended* fiber.
    pub fn evaluate(&mut self, host: &mut dyn Host, fiber: FiberId, expr: &str) -> Result<Value, RtError> {
        self.evaluate_in(host, fiber, usize::MAX, expr)
    }

    // ---------------------------------------------------------------------------------------
    // the debugger
    // ---------------------------------------------------------------------------------------

    /// Sets or clears a breakpoint. `program` is the name a frame of that source answers to -
    /// the program's own name for every frame of a .prg, `objPath.Event` for a method of a form,
    /// which is what its editor shows - and `line` is the line in it, counting from one.
    pub fn set_breakpoint(&mut self, program: &str, line: u32, on: bool) {
        let at = self.breakpoints.iter().position(|(p, l)| *l == line && p.eq_ignore_ascii_case(program));
        match (on, at) {
            (true, None) => self.breakpoints.push((program.to_string(), line)),
            (false, Some(i)) => {
                self.breakpoints.remove(i);
            }
            _ => {}
        }
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    /// Every breakpoint that is set, in the order they were set.
    pub fn breakpoints(&self) -> &[(String, u32)] {
        &self.breakpoints
    }

    /// How far a stopped fiber runs when it is next let go. Called while the VM is off the
    /// stack, before `resume`; `Go` puts it back to running until the next breakpoint.
    pub fn set_step_mode(&mut self, fiber: FiberId, mode: StepMode) {
        if let Some(fb) = self.fibers.get_mut(&fiber) {
            fb.stepping = match mode {
                StepMode::Go => None,
                other => Some((other, fb.frames.len())),
            };
        }
    }

    /// The frames of a stopped fiber, outermost first: what each is, where it is, and the
    /// module its source came from. `call_stack` is the same walk without the source.
    pub fn frames(&self, fiber: FiberId) -> Vec<FrameInfo> {
        let Some(fb) = self.fibers.get(&fiber) else { return Vec::new() };
        fb.frames
            .iter()
            .filter(|f| !matches!(f.kind, FrameKind::Inline { .. }))
            .map(|f| FrameInfo {
                program: self.proto(f.module, f.func).display_name.clone(),
                module: self.modules[f.module as usize].name.clone(),
                line: f.line,
            })
            .collect()
    }

    /// The variables one frame of a stopped fiber can see of its own: its LOCALs by slot name,
    /// then the PRIVATEs it declared, alphabetically. Everything a caller left visible is
    /// reached through the frame it belongs to, which is the frame the developer would pick.
    ///
    /// The slots the compiler took for itself - a FOR loop's bounds, a PARAMETERS list on its
    /// way to becoming privates - are left out: their names begin with `#` because no program
    /// could have written them, and a developer has nothing to do with them.
    pub fn frame_variables(&self, fiber: FiberId, level: usize) -> Vec<Variable> {
        let Some(fb) = self.fibers.get(&fiber) else { return Vec::new() };
        let Some(idx) = frame_at_level(fb, level) else { return Vec::new() };
        let fr = &fb.frames[idx];
        let proto = self.proto(fr.module, fr.func);
        let mut out: Vec<Variable> = proto
            .locals
            .iter()
            .enumerate()
            .filter(|(_, name)| !name.starts_with('#'))
            .map(|(slot, name)| Variable {
                name: name.clone(),
                private: false,
                value: fr.locals.get(slot).map(Value::deref).unwrap_or(Value::Logical(false)),
            })
            .collect();
        let mut privates: Vec<(&String, &Value)> = fr.privates.iter().collect();
        privates.sort_by(|a, b| a.0.cmp(b.0));
        out.extend(
            privates.iter().map(|(name, value)| Variable {
                name: (*name).clone(),
                private: true,
                value: value.deref(),
            }),
        );
        out
    }

    /// Evaluates expression text in a chosen frame of a *suspended* fiber - a watch expression,
    /// or a line typed while the program is stopped. `usize::MAX` means the frame it stopped in.
    ///
    /// The frames above the chosen one are set aside for the length of the evaluation, so the
    /// expression sees exactly what a statement of that frame would: its own slots, its privates
    /// and its callers', and nothing a routine it called has since declared.
    pub fn evaluate_in(
        &mut self,
        host: &mut dyn Host,
        fiber: FiberId,
        level: usize,
        expr: &str,
    ) -> Result<Value, RtError> {
        let Some(mut fb) = self.fibers.remove(&fiber) else {
            return Err(RtError::new(0, format!("Fiber {fiber} does not exist")));
        };
        let above = match frame_at_level(&fb, level) {
            Some(idx) if idx + 1 < fb.frames.len() => fb.frames.split_off(idx + 1),
            _ => Vec::new(),
        };
        let r = self.eval_in_frame(host, &mut fb, expr);
        fb.frames.extend(above);
        self.fibers.insert(fiber, fb);
        r
    }

    /// Hands the fiber to the debugger at the statement it is on. The step it was let go with is
    /// spent, so a fiber that is continued afterwards runs on until something else stops it.
    fn stop(&mut self, fb: &mut Fiber, reason: BreakReason) -> Flow {
        fb.stepping = None;
        let (program, line) = fb
            .frames
            .last()
            .map(|f| (self.proto(f.module, f.func).display_name.clone(), f.line))
            .unwrap_or_default();
        fb.pending = Some(Pending::Discard);
        Flow::Suspend(HostRequest::Break { program, line, reason })
    }

    /// Whether the statement about to run is one the debugger asked to stop at.
    ///
    /// Never inside an inline frame. Runtime-compiled code is part of the statement that asked
    /// for it - a `&cmd` line, a filter expression, a report's PRINT WHEN - and stopping inside
    /// one would both step into something no editor shows and break `EVALUATE()`, which cannot
    /// suspend at all.
    fn stop_here(&self, fb: &Fiber, line: u32) -> Option<BreakReason> {
        if fb.frames.last().is_some_and(|f| matches!(f.kind, FrameKind::Inline { .. })) {
            return None;
        }
        if let Some((mode, depth)) = fb.stepping {
            let here = fb.frames.len();
            let arrived = match mode {
                StepMode::Into => true,
                StepMode::Over => here <= depth,
                StepMode::Out => here < depth,
                StepMode::Go => false,
            };
            if arrived {
                return Some(BreakReason::Step);
            }
        }
        if self.breakpoints.is_empty() {
            return None;
        }
        let fr = fb.frames.last()?;
        let program = &self.proto(fr.module, fr.func).display_name;
        let module = &self.modules[fr.module as usize].name;
        let hit = self
            .breakpoints
            .iter()
            .any(|(name, at)| *at == line && (name.eq_ignore_ascii_case(program) || name.eq_ignore_ascii_case(module)));
        hit.then_some(BreakReason::Breakpoint)
    }

    /// The write that puts a line of diagnostic text in the file `SET DEBUGOUT` named, if any.
    /// The first line after that command replaces what the file held; the rest add to it.
    fn debug_output(&mut self, text: &str) -> Option<HostRequest> {
        if self.settings.debugout.is_empty() {
            return None;
        }
        let path = self.settings.at(&self.settings.debugout.clone());
        let append = self.settings.debugout_started;
        self.settings.debugout_started = true;
        Some(HostRequest::FileWrite { path, text: format!("{text}\r\n"), append })
    }

    // ---------------------------------------------------------------------------------------
    // frames
    // ---------------------------------------------------------------------------------------

    fn proto(&self, module: u32, func: u32) -> &FuncProto {
        &self.modules[module as usize].funcs[func as usize]
    }

    #[allow(clippy::too_many_arguments)]
    fn push_call(
        &mut self,
        fb: &mut Fiber,
        module: u32,
        func: u32,
        this: Option<Handle>,
        mut args: Vec<Value>,
        kind: FrameKind,
        strict: bool,
    ) -> Result<(), RtError> {
        let proto = self
            .modules
            .get(module as usize)
            .and_then(|m| m.funcs.get(func as usize))
            .ok_or_else(|| RtError::procedure_not_found(&format!("{module}:{func}")))?;
        let nparams = proto.nparams as usize;
        if args.len() > nparams {
            if strict {
                return Err(RtError::too_many_args());
            }
            args.truncate(nparams);
        }
        let mut locals = vec![Value::Logical(false); proto.locals.len()];
        for (i, a) in args.iter().enumerate().take(nparams) {
            // a parameter is a variable, and a variable gives a number a width of its own
            locals[i] = value::held_in_variable(a.clone());
        }
        fb.frames.push(Frame {
            module,
            func,
            pc: 0,
            locals,
            privates: HashMap::new(),
            this,
            with_stack: Vec::new(),
            args,
            line: 0,
            nodefault: false,
            stack_base: fb.stack.len(),
            stmt_sp: fb.stack.len(),
            released: Vec::new(),
            kind,
        });
        Ok(())
    }

    /// Records a function value and answers the id that stands for it.
    fn make_function(&mut self, f: FuncValue) -> FuncId {
        let id = self.next_function;
        self.next_function += 1;
        self.functions.insert(id, Rc::new(f));
        FuncId(id)
    }

    pub fn function_value(&self, id: FuncId) -> Option<&FuncValue> {
        self.functions.get(&id.0).map(|f| f.as_ref())
    }

    /// The one way a lambda is called, whichever door it came through: the call instruction, a
    /// built-in that was handed a function, or the host waking the runtime with an event.
    ///
    /// It pushes a frame and returns. A call in this VM is not a Rust call - `run` is a loop over
    /// an explicit stack of frames - so nothing here re-enters the interpreter and nothing
    /// re-enters a wasm export. See docs/foxscript.md.
    fn push_function_call(
        &mut self,
        fb: &mut Fiber,
        id: FuncId,
        args: Vec<Value>,
        discard: bool,
    ) -> Result<(), RtError> {
        let f = self.functions.get(&id.0).cloned().ok_or_else(|| RtError::procedure_not_found("LAMBDA"))?;
        self.push_call(fb, f.module, f.func, f.this, args, FrameKind::Call { discard }, true)?;
        self.fill_captures(fb.frames.last_mut().expect("frame"), &f);
        Ok(())
    }

    /// Writes a function value's captured values into the slots its proto put them in.
    fn fill_captures(&self, frame: &mut Frame, f: &FuncValue) {
        let proto = &self.modules[f.module as usize].funcs[f.func as usize];
        for (c, v) in proto.captures.iter().zip(f.captured.iter()) {
            if let Some(slot) = frame.locals.get_mut(c.to as usize) {
                *slot = v.clone();
            }
        }
    }

    /// A fiber whose first frame is a lambda. This is how the host wakes the runtime with an
    /// event: the handler is dispatched, exactly as a Click is, and never called from inside a
    /// host request.
    pub fn start_function(&mut self, id: FuncId, args: Vec<Value>) -> Option<FiberId> {
        let f = self.functions.get(&id.0).cloned()?;
        let fiber = self.start(f.module, f.func, f.this, args);
        let vm = &*self;
        let proto = &vm.modules[f.module as usize].funcs[f.func as usize];
        let pairs: Vec<(usize, Value)> =
            proto.captures.iter().zip(f.captured.iter()).map(|(c, v)| (c.to as usize, v.clone())).collect();
        if let Some(fb) = self.fibers.get_mut(&fiber)
            && let Some(frame) = fb.frames.last_mut()
        {
            for (slot, v) in pairs {
                if let Some(cell) = frame.locals.get_mut(slot) {
                    *cell = v;
                }
            }
        }
        Some(fiber)
    }

    /// Pushes an inline frame running `funcs[0]` of `module` with the storage of the current frame.
    fn push_inline(&mut self, fb: &mut Fiber, module: u32, push_result: bool) {
        let owner = env_index(fb, fb.frames.len() - 1);
        let needed = self.proto(module, 0).locals.len();
        let this = fb.frames[owner].this;
        let line = fb.frames.last().map(|f| f.line).unwrap_or(0);
        if fb.frames[owner].locals.len() < needed {
            fb.frames[owner].locals.resize(needed, Value::Logical(false));
        }
        fb.frames.push(Frame {
            module,
            func: 0,
            pc: 0,
            locals: Vec::new(),
            privates: HashMap::new(),
            this,
            with_stack: Vec::new(),
            args: Vec::new(),
            line,
            nodefault: false,
            stack_base: fb.stack.len(),
            stmt_sp: fb.stack.len(),
            // an inline frame borrows its owner's slots, and `env_index` sends every read and
            // write there, so its own list is never looked at
            released: Vec::new(),
            kind: FrameKind::Inline { owner, push_result },
        });
    }

    /// Pops the top frame, restoring whatever it borrowed from its owner.
    fn pop_frame(&mut self, fb: &mut Fiber) -> Frame {
        let f = fb.frames.pop().expect("frame");
        match f.kind {
            FrameKind::Inline { .. } => {
                // Cut the owner's locals back to what the (new) top frame's code expects.
                if let Some(top) = fb.frames.last() {
                    let len = self.proto(top.module, top.func).locals.len();
                    let env = env_index(fb, fb.frames.len() - 1);
                    fb.frames[env].locals.truncate(len);
                }
            }
            FrameKind::OnError { .. } => fb.in_error_handler = false,
            FrameKind::Call { .. } => {}
        }
        f
    }

    /// Runs the fiber until fewer than `floor + 1` frames remain (the frame at `floor` returned),
    /// an error is unhandled inside that range, or a host request is needed.
    fn run(&mut self, host: &mut dyn Host, fb: &mut Fiber, floor: usize) -> Step {
        loop {
            if fb.frames.len() <= floor {
                return Step::Done {
                    value: fb.result.take().unwrap_or(Value::Logical(false)),
                    nodefault: fb.nodefault,
                };
            }
            // a query an error escaped puts its work areas back before the program carries on,
            // which is done here because it can need the host and the error path cannot wait
            if fb.query_unwind.is_some() {
                match self.unwind_query(fb) {
                    Ok(Flow::Suspend(req)) => return Step::Suspend(req),
                    Ok(_) => {}
                    Err(e) => {
                        if let Some(step) = self.raise(host, fb, e, floor) {
                            return step;
                        }
                    }
                }
            }
            let (module, instr) = {
                let fr = fb.frames.last_mut().expect("frame");
                let module = self.modules[fr.module as usize].clone();
                let code = &module.funcs[fr.func as usize].code;
                let instr = code.get(fr.pc).cloned().unwrap_or(Instr::EndOfCode);
                if fr.pc >= code.len() {
                    fb.stack.push(Value::Logical(true));
                }
                fr.pc += 1;
                (module, instr)
            };
            let flow = self.exec(host, fb, &module, &instr);
            match flow {
                Ok(Flow::Next) => {}
                Ok(Flow::Return(_)) | Ok(Flow::EndOfCode(_)) => {
                    // A line a macro put together belongs to the routine that reached it, not
                    // to something that routine called, so a RETURN written in one leaves the
                    // routine as well. Running out of statements only ends the line.
                    let (v, written) = match flow {
                        Ok(Flow::Return(v)) => (v, true),
                        Ok(Flow::EndOfCode(v)) => (v, false),
                        _ => unreachable!(),
                    };
                    loop {
                        let f = self.pop_frame(fb);
                        fb.stack.truncate(f.stack_base);
                        // RETURN TO keeps unwinding until the routine it named is the one left,
                        // and the routines passed through on the way get no return value
                        let unwinding = match fb.return_to.as_deref() {
                            None => false,
                            Some(target) => {
                                let reached = match target.is_empty() {
                                    true => fb.frames.len() <= floor + 1,
                                    false => fb.frames.last().is_some_and(|fr| {
                                        self.proto(fr.module, fr.func).display_name.eq_ignore_ascii_case(target)
                                    }),
                                };
                                if reached {
                                    fb.return_to = None;
                                }
                                !reached
                            }
                        };
                        let carry_on = match f.kind {
                            FrameKind::Call { discard } => {
                                if !discard && !unwinding {
                                    fb.stack.push(v.clone());
                                }
                                unwinding
                            }
                            FrameKind::Inline { push_result, .. } => {
                                if push_result && !unwinding {
                                    fb.stack.push(v.clone());
                                }
                                unwinding || (written && !push_result)
                            }
                            FrameKind::OnError { resume_pc, stmt_sp } => {
                                if let Some(caller) = fb.frames.last_mut() {
                                    caller.pc = resume_pc;
                                }
                                fb.stack.truncate(stmt_sp);
                                unwinding
                            }
                        };
                        if fb.frames.len() <= floor {
                            fb.result = Some(v.clone());
                            fb.return_to = None;
                            break;
                        }
                        if !carry_on {
                            break;
                        }
                    }
                }
                Ok(Flow::Suspend(req)) => return Step::Suspend(req),
                Err(e) => {
                    if let Some(step) = self.raise(host, fb, e, floor) {
                        return step;
                    }
                }
            }
        }
    }

    fn apply_resume(&mut self, host: &mut dyn Host, fb: &mut Fiber, pending: Option<Pending>, v: Value) -> Result<(), RtError> {
        match pending {
            None | Some(Pending::Discard) => {}
            Some(Pending::Push) => fb.stack.push(v.deref()),
            Some(Pending::WaitKey) => fb.stack.push(if v.is_null() { Value::str("") } else { v.deref() }),
            Some(Pending::Listening { server }) => {
                let port = v.as_number().unwrap_or(0.0) as u16;
                if let Some(s) = self.natives.server_mut(crate::value::Handle(server)) {
                    s.port = Some(port);
                }
                fb.stack.push(Value::number(f64::from(port)));
            }
            Some(Pending::ReadGets) => self.read_finished(fb, &v),
            Some(Pending::MenuTo { name }) => {
                let chosen = v.as_number().unwrap_or(0.0);
                let prompts = std::mem::take(&mut self.screen.prompts);
                self.screen.chosen =
                    prompts.get((chosen as usize).wrapping_sub(1)).cloned().unwrap_or_default();
                self.write_back(fb, &name, Value::number(chosen));
            }
            Some(Pending::FileResult) => {
                let (value, errno) = file_reply(&v);
                self.ferror = errno;
                fb.stack.push(value);
            }
            Some(Pending::FileCommand { op }) => {
                let (value, errno) = file_reply(&v);
                self.ferror = errno;
                if errno != 0 {
                    return Err(file_error(&op, errno));
                }
                match (op.as_str(), value.deref()) {
                    // DIR lists; TYPE shows the file: both are output, line by line
                    ("dir", Value::Array(rows)) => {
                        for row in rows.borrow().items.iter() {
                            if let Value::Array(cells) = row.deref() {
                                let cells = cells.borrow();
                                let text: Vec<String> = cells.items.iter().map(|c| value::display(c, &self.settings)).collect();
                                host.output(&text.join("  "), true);
                            }
                        }
                    }
                    ("type", Value::Str(text)) => {
                        for line in text.lines() {
                            host.output(line, true);
                        }
                    }
                    _ => {}
                }
            }
            Some(Pending::DllReturn { refs }) => {
                // with nothing by reference the answer is the return value alone
                let (value, written) = match v.deref() {
                    Value::Array(a) => {
                        let items = a.borrow().items.clone();
                        let written = match items.get(1).map(Value::deref) {
                            Some(Value::Array(w)) => w.borrow().items.clone(),
                            _ => Vec::new(),
                        };
                        (items.first().cloned().unwrap_or(Value::Null), written)
                    }
                    other => (other, Vec::new()),
                };
                for (slot, answer) in refs.into_iter().zip(written) {
                    if answer.is_null() {
                        continue;
                    }
                    if let Value::Ref(cell) = slot {
                        *cell.borrow_mut() = answer.deref();
                    }
                }
                fb.stack.push(value);
            }
            Some(Pending::FinishDoForm { flags }) => {
                let items: Vec<Value> = match v.deref() {
                    Value::Array(a) => a.borrow().items.clone(),
                    Value::Null => Vec::new(),
                    other => vec![other],
                };
                let mut it = items.into_iter();
                if flags & form_flags::WANT_OBJECT != 0 {
                    fb.stack.push(it.next().unwrap_or(Value::Null));
                }
                if flags & form_flags::WANT_RESULT != 0 {
                    fb.stack.push(it.next().unwrap_or(Value::Null));
                }
            }
            Some(Pending::LoadedProgram { func, args, discard, program }) => {
                let id = match v.deref() {
                    Value::Number(n, ..) if n >= 0.0 && (n as usize) < self.modules.len() => n as u32,
                    // measured: a program that is not there is error 1, "File 'x.prg' does not
                    // exist.", whether DO or a bare call went looking for it
                    _ => return Err(program_missing(&program)),
                };
                let fidx = match &func {
                    None => {
                        self.procedure_only.remove(&id);
                        0
                    }
                    Some(name) => {
                        self.modules[id as usize].find_func(name).ok_or_else(|| RtError::procedure_not_found(name))?
                    }
                };
                self.push_call(fb, id, fidx, None, args, FrameKind::Call { discard }, true)?;
            }
            Some(Pending::DataReply) => fb.data_reply = Some(v.deref()),
            Some(Pending::EndClassError { obj }) => {
                self.in_class_error.remove(&obj);
            }
            Some(Pending::Finish) => {
                fb.frames.clear();
                fb.handlers.clear();
                fb.result = Some(Value::Null);
            }
        }
        Ok(())
    }

    /// Handles `err`: TRY handler, ON ERROR, or `Some(Step::Error)` when unhandled.
    ///
    /// Whatever becomes of it, the host is told: an error a program swallows is otherwise the
    /// hardest kind to place, because the only thing left of it is whatever its handler chose
    /// to say.
    fn raise(&mut self, host: &mut dyn Host, fb: &mut Fiber, err: RtError, floor: usize) -> Option<Step> {
        let located = self.locate(fb, err);
        let noted = located.clone();
        let step = self.handle(host, fb, located, floor);
        host.error_raised(&noted, !matches!(step, Some(Step::Error(_))));
        step
    }

    /// Fills in the program and line an error came from, when it does not carry them already.
    fn locate(&mut self, fb: &Fiber, mut err: RtError) -> RtError {
        if let Some(top) = fb.frames.len().checked_sub(1) {
            let env = env_index(fb, top);
            if err.program.is_empty() {
                // the name of a routine is upper-cased wherever the language says it, and an
                // error saying which routine it happened in is one of those places
                err.program =
                    self.proto(fb.frames[env].module, fb.frames[env].func).display_name.to_ascii_uppercase();
            }
            if err.line == 0 {
                err.line = fb.frames[env].line;
            }
        }
        err
    }

    /// Gives the error to the nearest handler that wants it.
    fn handle(&mut self, host: &mut dyn Host, fb: &mut Fiber, err: RtError, floor: usize) -> Option<Step> {
        fb.last_error = Some(err.clone());
        // An error raised while a HAVING predicate was running leaves the query saying that no
        // record is current. A CATCH that swallows it must not leave every later field read
        // complaining, so the clause is no longer running the moment the error is handed on.
        if let Some(state) = fb.query.as_mut().and_then(|run| run.having.as_mut()) {
            state.testing = false;
        }
        // A query the error has escaped is over, however it is handled: the work areas it
        // borrowed go back to the program, which is what a TRY around a SELECT expects to find
        // afterwards. An error caught deeper than the query started was caught inside something
        // the query called - a function in the WHERE clause with a TRY of its own - and that one
        // carries on.
        if let Some(run) = fb.query.as_ref() {
            let caught_deeper = fb
                .handlers
                .last()
                .is_some_and(|h| h.frame >= floor && h.frame > run.frame);
            if !caught_deeper {
                fb.query_unwind = fb.query.take();
            }
        }
        if let Some(step) = self.class_error_method(host, fb, &err, floor) {
            return Some(step);
        }
        while let Some(h) = fb.handlers.last().cloned() {
            if h.frame < floor {
                break;
            }
            fb.handlers.pop();
            while fb.frames.len() > h.frame + 1 {
                self.pop_frame(fb);
            }
            fb.stack.truncate(h.sp);
            let env = env_index(fb, h.frame);
            fb.frames[env].with_stack.truncate(h.with_len);
            fb.pending_rethrow.truncate(h.pending_len);
            if let Some(c) = h.catch {
                fb.frames[h.frame].pc = c;
                fb.handlers.push(TryHandler {
                    frame: h.frame,
                    catch: None,
                    finally: h.finally,
                    sp: h.sp,
                    with_len: h.with_len,
                    pending_len: h.pending_len,
                });
                return None;
            }
            if let Some(f) = h.finally {
                fb.pending_rethrow.push(Some(err));
                fb.frames[h.frame].pc = f;
                return None;
            }
        }
        if floor == 0
            && !fb.in_error_handler
            && !fb.frames.is_empty()
            && let Some(text) = self.on_error.clone()
        {
            return self.run_on_error(fb, &text);
        }
        Some(Step::Error(err))
    }

    /// VFP hands an error raised inside a method to the object's own `Error` method, unless a
    /// TRY inside that same method catches it first. The method runs through the host like any
    /// other, and the failing method then carries on at its next statement.
    fn class_error_method(&mut self, host: &mut dyn Host, fb: &mut Fiber, err: &RtError, floor: usize) -> Option<Step> {
        let (frame_ix, obj) = (floor..fb.frames.len()).rev().find_map(|i| {
            let this = fb.frames[i].this?;
            (!self.in_class_error.contains(&this.0) && host.class_method(this, "Error")).then_some((i, this))
        })?;
        // a TRY inside the failing method (or deeper) is nearer than the Error method
        if fb.handlers.last().is_some_and(|h| h.frame >= floor && h.frame >= frame_ix) {
            return None;
        }

        while fb.frames.len() > frame_ix + 1 {
            self.pop_frame(fb);
        }
        let (resume_pc, stmt_sp) = {
            let fr = &fb.frames[frame_ix];
            let code = &self.proto(fr.module, fr.func).code;
            let next = (fr.pc..code.len()).find(|&i| matches!(code[i], Instr::Stmt(_))).unwrap_or(code.len().saturating_sub(2));
            (next, fr.stmt_sp)
        };
        fb.stack.truncate(stmt_sp);
        fb.frames[frame_ix].pc = resume_pc;

        self.in_class_error.insert(obj.0);
        fb.pending = Some(Pending::EndClassError { obj: obj.0 });
        Some(Step::Suspend(HostRequest::CallMethod {
            obj: obj.0,
            name: "Error".into(),
            args: vec![
                JsonValue::Num(f64::from(err.code)),
                JsonValue::Str(err.program.clone()),
                JsonValue::Num(f64::from(err.line)),
            ],
        }))
    }


    /// `RETRY`: returns to the program that called and runs its statement again.
    ///
    /// Inside an ON ERROR handler that program is the one whose statement failed, not whatever
    /// the handler itself called, so the whole handler unwinds first - which is the case RETRY
    /// exists for: fix what broke, then do it again.
    fn retry(&mut self, fb: &mut Fiber) -> Result<(), RtError> {
        let handler = fb.frames.iter().rposition(|f| matches!(f.kind, FrameKind::OnError { .. }));
        let keep = match handler {
            Some(i) => i,
            None if fb.frames.len() > 1 => fb.frames.len() - 1,
            None => return Err(RtError::syntax("RETRY has no calling program to return to")),
        };
        let mut handler_sp = None;
        while fb.frames.len() > keep {
            let f = self.pop_frame(fb);
            fb.stack.truncate(f.stack_base);
            if let FrameKind::OnError { stmt_sp, .. } = f.kind {
                handler_sp = Some(stmt_sp);
            }
        }
        if let Some(sp) = handler_sp {
            fb.stack.truncate(sp);
        }
        self.rewind_to_statement(fb);
        Ok(())
    }

    /// Puts the frame now on top back at the start of the statement it is in, for `RETRY`. The
    /// statement is found by walking back to the `Stmt` marker before the program counter, which
    /// is the same marker the line numbers in errors come from.
    fn rewind_to_statement(&mut self, fb: &mut Fiber) {
        let Some(fr) = fb.frames.last() else { return };
        let code = &self.proto(fr.module, fr.func).code;
        let start = (0..fr.pc).rev().find(|&i| matches!(code[i], Instr::Stmt(_))).unwrap_or(0);
        let stmt_sp = fr.stmt_sp;
        let fr = fb.frames.last_mut().expect("frame");
        fr.pc = start;
        fb.stack.truncate(stmt_sp);
    }
    /// Runs the ON ERROR handler text in a new frame; the failing frame resumes at its next statement.
    fn run_on_error(&mut self, fb: &mut Fiber, text: &str) -> Option<Step> {
        while fb.frames.last().is_some_and(|f| matches!(f.kind, FrameKind::Inline { .. })) {
            self.pop_frame(fb);
        }
        let module = match self.compile_inline(Inline::Line, 0, 0, text, &[]) {
            Ok(m) => m,
            Err(e) => return Some(Step::Error(e)),
        };
        let (resume_pc, stmt_sp) = {
            let fr = fb.frames.last().expect("frame");
            let code = &self.proto(fr.module, fr.func).code;
            let next = (fr.pc..code.len())
                .find(|&i| matches!(code[i], Instr::Stmt(_)))
                .unwrap_or(code.len().saturating_sub(2));
            (next, fr.stmt_sp)
        };
        fb.stack.truncate(stmt_sp);
        if let Err(e) =
            self.push_call(fb, module, 0, None, Vec::new(), FrameKind::OnError { resume_pc, stmt_sp }, false)
        {
            return Some(Step::Error(e));
        }
        fb.in_error_handler = true;
        None
    }

    /// Compiles (or fetches from the cache) inline code for the given owner function.
    fn compile_inline(
        &mut self,
        what: Inline,
        module: u32,
        func: u32,
        text: &str,
        locals: &[String],
    ) -> Result<u32, RtError> {
        let key = (what, module, func, text.to_string());
        if let Some(&id) = self.inline_cache.get(&key) {
            return Ok(id);
        }
        let result = match what {
            Inline::Expression => compiler::compile_expression(text, locals),
            Inline::Line => compiler::compile_snippet_in(text, "macro", locals),
            Inline::ExpandedLine => compiler::compile_expanded_line(text, "macro", locals),
            Inline::StoreTo => compiler::compile_store_to(text, locals),
            Inline::Program => compiler::compile_snippet(text, "execscript"),
        };
        match result.module {
            Some(m) => {
                let id = self.modules.len() as u32;
                self.modules.push(Rc::new(m));
                self.inline_cache.insert(key, id);
                Ok(id)
            }
            None => {
                let msg = result
                    .diagnostics
                    .iter()
                    .find(|d| d.is_error())
                    .map(|d| d.message.clone())
                    .unwrap_or_else(|| "Syntax error".into());
                Err(RtError::syntax(msg))
            }
        }
    }

    /// Runs `expr` in the current frame synchronously (EVALUATE()).
    fn eval_in_frame(&mut self, host: &mut dyn Host, fb: &mut Fiber, expr: &str) -> Result<Value, RtError> {
        let top = fb.frames.last().ok_or_else(|| RtError::feature_not_available("EVALUATE() outside a program"))?;
        let (m, f) = (top.module, top.func);
        let locals = self.proto(m, f).locals.clone();
        let id = self.compile_inline(Inline::Expression, m, f, expr, &locals)?;
        let floor = fb.frames.len();
        let stack_len = fb.stack.len();
        let handlers_len = fb.handlers.len();
        let pending_len = fb.pending_rethrow.len();
        self.push_inline(fb, id, false);
        let step = self.run(host, fb, floor);
        let r = match step {
            Step::Done { value, .. } => Ok(value),
            Step::Error(e) => Err(e),
            Step::Suspend(_) => Err(RtError::feature_not_available("suspending inside EVALUATE()")),
        };
        while fb.frames.len() > floor {
            self.pop_frame(fb);
        }
        fb.stack.truncate(stack_len);
        fb.handlers.truncate(handlers_len);
        fb.pending_rethrow.truncate(pending_len);
        fb.pending = None;
        r
    }

    // ---------------------------------------------------------------------------------------
    // variables
    // ---------------------------------------------------------------------------------------

    fn load_local(&self, fb: &Fiber, slot: u32) -> Result<Value, RtError> {
        let env = env_index(fb, fb.frames.len() - 1);
        let frame = &fb.frames[env];
        if frame.released.get(slot as usize).copied().unwrap_or(false) {
            let name = self.proto(frame.module, frame.func).locals.get(slot as usize).cloned().unwrap_or_default();
            return Err(RtError::variable_not_found(&name));
        }
        Ok(frame.locals.get(slot as usize).map(Value::deref).unwrap_or(Value::Logical(false)))
    }

    fn store_local(&self, fb: &mut Fiber, slot: u32, v: Value) {
        let env = env_index(fb, fb.frames.len() - 1);
        let locals = &mut fb.frames[env].locals;
        let slot = slot as usize;
        if slot >= locals.len() {
            locals.resize(slot + 1, Value::Logical(false));
        }
        write_through(&mut locals[slot], v);
        if let Some(gone) = fb.frames[env].released.get_mut(slot) {
            *gone = false;
        }
    }

    /// Location of a dynamically scoped variable visible from the top frame.
    fn find_name(&self, fb: &Fiber, upper: &str) -> Option<NameLoc> {
        if let Some(i) = private_frame(&fb.frames, upper) {
            return Some(NameLoc::Private(i));
        }
        // A private belongs to the program that declared it and to everything that program
        // called. An event handler running while the program is parked in READ EVENTS *is*
        // something the program called - Visual FoxPro has one stack, with the handler on top of
        // it - so the parked fibers are searched, nearest first, before the globals. Without this
        // `cOld = ON("ERROR")` in a program and `ON ERROR &cOld` in a form method cannot see the
        // same variable, and neither can most of the code anyone has written.
        let mut parked: Vec<&FiberId> = self.fibers.keys().collect();
        parked.sort_unstable_by(|a, b| b.cmp(a));
        for id in parked {
            if let Some(other) = self.fibers.get(id)
                && private_frame(&other.frames, upper).is_some()
            {
                return Some(NameLoc::Parked(*id));
            }
        }
        if self.globals.contains_key(upper) { Some(NameLoc::Global) } else { None }
    }

    /// The text a macro stands for, or nothing when the name is not a character variable this
    /// program can see - a name nothing declared, or one holding a number or an object. VFP
    /// leaves that text exactly as it was written rather than complaining about it, so
    /// `? "n=&n."` where n is a number prints `n=&n.` and `? &n` fails to read as a command.
    ///
    /// `&lcPath` names a LOCAL as readily as a private, and a LOCAL lives in a slot of its frame
    /// rather than under its name.
    fn macro_value(&self, fb: &Fiber, upper: &str) -> Option<String> {
        let top = env_index(fb, fb.frames.len() - 1);
        let frame = &fb.frames[top];
        let proto = self.proto(frame.module, frame.func);
        let value = match proto.locals.iter().position(|l| l.eq_ignore_ascii_case(upper)) {
            Some(slot) => frame.locals.get(slot).cloned().map(|v| v.deref()),
            None => self.load_name(fb, upper),
        }?;
        value.as_str().ok().map(|s| s.to_string())
    }

    /// Replaces every macro in a command's text with the text it stands for, as VFP does before
    /// it reads the command at all: `&name`, `&name.` where the dot says where the name ends,
    /// and `&aNames[i]` where the text is in one element of an array.
    ///
    /// This is a pass over the characters of the line and nothing else, which is why what is
    /// inside quotes goes through it too: in the product, `? "&lcScope"` prints what lcScope
    /// holds, and `? '&c'` where c holds `x' + 'y` prints `xy`, because the text is put into
    /// the line and the line is then read. A name that is not a character variable is left
    /// standing as it was written, so `? "n=&n."` with a number in n prints `n=&n.`.
    ///
    /// `ON ERROR &cOld` is the everyday case; a command this parser could not read as it stands
    /// is the other.
    fn expand_macros(&mut self, host: &mut dyn Host, fb: &mut Fiber, text: &str) -> Result<String, RtError> {
        self.expand_macros_below(host, fb, text, &mut Vec::new())
    }

    /// The pass itself. `in_hand` are the names already being stood for, further out: what a
    /// macro stands for is read for macros of its own - `a` holding `&c` and `c` holding `abc`
    /// makes `"[&a.]"` read `[abc]` - and a name that stands for itself would otherwise never
    /// come to an end, so it is left alone the second time it is met.
    fn expand_macros_below(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        text: &str,
        in_hand: &mut Vec<String>,
    ) -> Result<String, RtError> {
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let is_macro = c == '&' && chars.get(i + 1).is_some_and(|n| n.is_ascii_alphabetic() || *n == '_');
            if !is_macro {
                out.push(c);
                i += 1;
                continue;
            }
            let mut end = i + 1;
            while chars.get(end).is_some_and(|c| is_ident_char(*c)) {
                end += 1;
            }
            let name_end = end;
            // a subscript belongs to the macro: it says which element of the array holds the text
            if let Some(close) = matches!(chars.get(end), Some('[') | Some('(')).then(|| if chars[end] == '[' { ']' } else { ')' })
                && let Some(n) = chars[end..].iter().position(|&x| x == close)
            {
                end += n + 1;
            }
            let operand: String = chars[i + 1..end].iter().collect();
            let upper = operand.to_ascii_uppercase();
            let value = if end == name_end {
                if in_hand.iter().any(|n| *n == upper) { None } else { self.macro_value(fb, &upper) }
            } else {
                self.eval_in_frame(host, fb, &operand).ok().and_then(|v| v.as_str().ok().map(|s| s.to_string()))
            };
            let Some(value) = value else {
                // nothing to put there, so the text stays as the program wrote it
                out.push(c);
                i += 1;
                continue;
            };
            in_hand.push(upper);
            let value = self.expand_macros_below(host, fb, &value, in_hand)?;
            in_hand.pop();
            out.push_str(&value);
            // a dot glued to the end closes the name, and is no part of what follows
            i = if chars.get(end) == Some(&'.') { end + 1 } else { end };
        }
        Ok(out)
    }

    fn load_name(&self, fb: &Fiber, upper: &str) -> Option<Value> {
        // `FoxScript` is a name only when nothing else in scope answers to it. A program that
        // already has a variable of that name keeps working, which is the same rule that keeps
        // `LAMBDA` a variable name: the product wins wherever it already has an answer.
        let Some(place) = self.find_name(fb, upper) else {
            return (upper == "FOXSCRIPT").then(crate::foxscript::root);
        };
        match place {
            NameLoc::Private(i) => fb.frames[i].privates.get(upper).map(Value::deref),
            NameLoc::Parked(id) => {
                let other = self.fibers.get(&id)?;
                let i = private_frame(&other.frames, upper)?;
                other.frames[i].privates.get(upper).map(Value::deref)
            }
            NameLoc::Global => self.globals.get(upper).map(Value::deref),
        }
    }

    fn store_name(&mut self, fb: &mut Fiber, upper: &str, v: Value) -> Result<(), RtError> {
        if let Some(want) = system_var_type(upper)
            && v.vartype() != want
        {
            return Err(RtError::data_type_mismatch());
        }
        match self.find_name(fb, upper) {
            Some(NameLoc::Private(i)) => write_through(fb.frames[i].privates.get_mut(upper).expect("private"), v),
            Some(NameLoc::Parked(id)) => {
                if let Some(other) = self.fibers.get_mut(&id)
                    && let Some(i) = private_frame(&other.frames, upper)
                {
                    write_through(other.frames[i].privates.get_mut(upper).expect("private"), v);
                }
            }
            Some(NameLoc::Global) => write_through(self.globals.get_mut(upper).expect("global"), v),
            None => {
                let env = env_index(fb, fb.frames.len() - 1);
                fb.frames[env].privates.insert(upper.to_string(), value::held_in_variable(v.deref()));
            }
        }
        Ok(())
    }

    fn slot_mut<'a>(&'a mut self, fb: &'a mut Fiber, var: Var, module: &Module) -> Result<&'a mut Value, RtError> {
        match var {
            Var::Local(s) => {
                let env = env_index(fb, fb.frames.len() - 1);
                let locals = &mut fb.frames[env].locals;
                if (s as usize) >= locals.len() {
                    locals.resize(s as usize + 1, Value::Logical(false));
                }
                Ok(&mut locals[s as usize])
            }
            Var::Name(n) => {
                let upper = &module.names[n as usize];
                match self.find_name(fb, upper) {
                    Some(NameLoc::Private(i)) => Ok(fb.frames[i].privates.get_mut(upper).expect("private")),
                    Some(NameLoc::Global) => Ok(self.globals.get_mut(upper).expect("global")),
                    // a private of a parked fiber cannot be handed out as a place to write
                    // through: the borrow belongs to another fiber, and @ by reference is for
                    // this one. VFP passes such a variable by value too.
                    Some(NameLoc::Parked(_)) | None => Err(RtError::variable_not_found(upper)),
                }
            }
        }
    }

    fn store_var(&mut self, fb: &mut Fiber, var: Var, module: &Module, v: Value) -> Result<(), RtError> {
        match var {
            Var::Local(s) => {
                self.store_local(fb, s, v);
                Ok(())
            }
            Var::Name(n) => self.store_name(fb, &module.names[n as usize], v),
        }
    }

    // ---------------------------------------------------------------------------------------
    // calls
    // ---------------------------------------------------------------------------------------

    /// A user function by name: the current module first, then loaded programs (latest first).
    /// The request that calls a declared library function, when the name is one.
    ///
    /// A `DECLARE ... DLL` makes a name callable exactly like a procedure, so this is asked
    /// before the program's own procedures are searched and answers nothing when the name is
    /// not declared.
    fn dll_call(&mut self, fb: &mut Fiber, upper: &str, args: &[Value]) -> Result<Option<Flow>, RtError> {
        let Some((library, proto)) = self.dlls.get(upper).cloned() else { return Ok(None) };
        // the arguments are kept as they are - a `@` argument is a reference to the caller's
        // variable - so that what the library writes can be put back into it
        fb.pending = Some(Pending::DllReturn { refs: args.to_vec() });
        Ok(Some(Flow::Suspend(HostRequest::CallDll {
            library,
            function: proto.function.clone(),
            returns: proto.returns.clone(),
            params: proto.params.iter().map(|(k, _)| k.clone()).collect(),
            by_ref: proto.params.iter().map(|(_, r)| *r).collect(),
            args: args.iter().map(JsonValue::from_value).collect(),
        })))
    }

    fn find_function(&self, current: u32, upper: &str) -> Option<(u32, u32)> {
        if let Some(f) = self.modules[current as usize].find_func(upper) {
            return Some((current, f));
        }
        // measured: the procedure files come next, in the order SET PROCEDURE listed them,
        // ahead of any other program - a routine in two of them is the first file's
        for &id in &self.procedure_files {
            if let Some(f) = self.modules[id as usize].find_func(upper) {
                return Some((id, f));
            }
        }
        for (i, m) in self.modules.iter().enumerate().rev() {
            if i as u32 != current
                && m.kind == ModuleKind::Program
                && !self.procedure_only.contains(&(i as u32))
                && let Some(f) = m.find_func(upper)
            {
                return Some((i as u32, f));
            }
        }
        None
    }

    fn this_form(&self, host: &mut dyn Host, this: Handle) -> Handle {
        self.container_of(host, this, "Form").unwrap_or(this)
    }

    /// The object of that base class this one sits in, itself included: what THISFORM and
    /// THISFORMSET each walk up to find. `Parent` leads all the way to the screen, so the walk
    /// is bounded the way a container nesting is.
    fn container_of(&self, host: &mut dyn Host, this: Handle, base_class: &str) -> Option<Handle> {
        let mut h = this;
        for _ in 0..64 {
            if host.object_class(h).is_some_and(|c| c.eq_ignore_ascii_case(base_class)) {
                return Some(h);
            }
            match host.get_prop(h, "Parent") {
                Ok(Value::Object(p)) => h = p,
                _ => return None,
            }
        }
        None
    }

    /// A member of one of the VM's own objects; nothing at all when the handle is not one of
    /// theirs, which is the signal to ask the host as usual.
    fn native_member(&self, h: Handle, name: &str) -> Option<Result<Value, RtError>> {
        let native = self.natives.get(h)?;
        Some(match native.class.member(name) {
            Some(m) if m.kind == crate::host::MemberKind::Object => {
                Ok(Value::Object(crate::foxscript::handle(m.child)))
            }
            Some(m) if m.kind == crate::host::MemberKind::Property => Ok(self.natives.property(h, m.name)),
            _ => Err(RtError::unknown_member(name)),
        })
    }

    /// A method of one of the VM's own objects.
    ///
    /// The ones that only touch the VM answer here; the ones that need a machine - opening a
    /// socket, closing one - yield a request the way every other side effect does, and the VM
    /// is off the stack while the host performs it.
    fn native_call(&mut self, fb: &mut Fiber, h: Handle, name: &str, args: Vec<Value>) -> Result<Flow, RtError> {
        use crate::foxscript::{HTTP_METHODS, NativeClass};
        let Some(native) = self.natives.get(h) else { return Err(RtError::object_not_valid()) };
        let class = native.class;
        let upper = name.to_ascii_uppercase();
        if !matches!(class.member(&upper), Some(m) if m.kind == crate::host::MemberKind::Method) {
            return Err(RtError::unknown_member(name));
        }
        match (class, upper.as_str()) {
            (NativeClass::Http, "CREATESERVER") => {
                let made = self.natives.add(NativeClass::Server);
                fb.stack.push(Value::Object(made));
                Ok(Flow::Next)
            }
            // `oServer.Get(cRoute, fnHandler)`: one method per HTTP verb, all the same code.
            // The server is answered back so that registrations chain.
            (NativeClass::Server, verb) if HTTP_METHODS.contains(&verb) => {
                let path = args.first().map(Value::deref).unwrap_or(Value::Null).as_str()?.to_string();
                let Some(Value::Function(func)) = args.get(1).map(Value::deref) else {
                    return Err(RtError::new(
                        RtError::DATA_TYPE_MISMATCH,
                        format!("{verb} needs a route and a lambda to answer it with."),
                    ));
                };
                let Some(server) = self.natives.server_mut(h) else { return Err(RtError::object_not_valid()) };
                server.route(verb, &path, func.0)?;
                fb.stack.push(Value::Object(h));
                Ok(Flow::Next)
            }
            (NativeClass::Server, "LISTEN") => {
                let port = args.first().map(Value::deref).unwrap_or(Value::Null).as_number()? as u32;
                fb.pending = Some(Pending::Listening { server: h.0 });
                Ok(Flow::Suspend(HostRequest::HttpListen { server: h.0, port }))
            }
            (NativeClass::Json, verb) => {
                let value = self.json_method(verb, &args)?;
                fb.stack.push(value);
                Ok(Flow::Next)
            }
            (NativeClass::Data, "CURSORTOJSON") => {
                let alias = args.first().map(Value::deref).unwrap_or(Value::Null).as_str()?.to_string();
                let text = self.cursor_to_json(&alias)?;
                fb.stack.push(Value::str(text));
                Ok(Flow::Next)
            }
            (NativeClass::Data, "JSONTOCURSOR") => {
                let source = args.first().map(Value::deref).unwrap_or(Value::Null);
                let alias = args.get(1).map(Value::deref).unwrap_or(Value::Null).as_str()?.to_string();
                let count = self.json_to_cursor(&source, &alias)?;
                fb.stack.push(Value::number(count as f64));
                Ok(Flow::Next)
            }
            (NativeClass::Server, "CLOSE") => {
                if let Some(server) = self.natives.server_mut(h) {
                    server.port = None;
                }
                fb.pending = Some(Pending::Push);
                Ok(Flow::Suspend(HostRequest::HttpClose { server: h.0 }))
            }
            _ => Err(RtError::unknown_member(name)),
        }
    }

    /// `FoxScript.Json`: six members and no more, all of them reads.
    fn json_method(&mut self, verb: &str, args: &[Value]) -> Result<Value, RtError> {
        use crate::json;
        let first = args.first().map(Value::deref).unwrap_or(Value::Null);
        // every member but Parse and Stringify wants a JSON value, and says so when it gets
        // something else rather than quietly answering nothing
        let doc = |v: &Value| -> Result<Rc<serde_json::Value>, RtError> {
            match v {
                Value::Json(j) => Ok(j.clone()),
                _ => Err(RtError::new(crate::foxscript::FOXSCRIPT_ERROR, format!("{verb} wants a JSON value."))),
            }
        };
        match verb {
            "PARSE" => json::parse(&first.as_str()?),
            "STRINGIFY" => {
                let pretty = crate::builtins::opt_bool(args, 1, false);
                Ok(Value::str(json::stringify(&first, pretty, &self.settings)?))
            }
            "COUNT" => Ok(Value::number(json::count(doc(&first)?.as_ref()) as f64)),
            "KEYS" => Ok(json::json(json::keys(doc(&first)?.as_ref()))),
            "HAS" => {
                let name = args.get(1).map(Value::deref).unwrap_or(Value::Null).as_str()?.to_string();
                Ok(Value::Logical(json::member(doc(&first)?.as_ref(), &name).is_some()))
            }
            // the form that asks rather than insists: a name the object has not got is .NULL.
            "GET" => {
                let name = args.get(1).map(Value::deref).unwrap_or(Value::Null).as_str()?.to_string();
                Ok(json::member(doc(&first)?.as_ref(), &name).map_or(Value::Null, json::from_json))
            }
            _ => Err(RtError::unknown_member(verb)),
        }
    }

    /// `FoxScript.Data.CursorToJson()`: every record of an open cursor as JSON text.
    ///
    /// It reads the rows the VM holds. A table the host has open arrives a page at a time and
    /// cannot be walked from inside a method call, so one of those is refused by name rather
    /// than half-answered; a query result - which is what a program hands to this - is simply
    /// there. See docs/foxscript.md.
    fn cursor_to_json(&self, alias: &str) -> Result<String, RtError> {
        let cursor = self
            .data
            .find(&AreaRef::Alias(alias.to_ascii_uppercase()))
            .ok_or_else(|| RtError::alias_not_found(alias))?;
        let Some(rows) = cursor.memory_rows() else {
            return Err(RtError::new(
                crate::foxscript::FOXSCRIPT_ERROR,
                format!("{alias} is a table on disk; copy it to a cursor first."),
            ));
        };
        let fields = &cursor.header.fields;
        let deleted = self.settings.deleted;
        let out: Vec<serde_json::Value> = rows
            .iter()
            .filter(|r| !(deleted && r.deleted))
            .map(|r| crate::json::record_to_json(fields, r))
            .collect();
        Ok(serde_json::Value::Array(out).to_string())
    }

    /// `FoxScript.Data.JsonToCursor()`: a cursor of that name holding what the JSON describes.
    fn json_to_cursor(&mut self, source: &Value, alias: &str) -> Result<usize, RtError> {
        let doc = match source {
            Value::Json(j) => j.as_ref().clone(),
            other => match crate::json::parse(&other.as_str()?)? {
                Value::Json(j) => j.as_ref().clone(),
                _ => unreachable!("parse answers a JSON value"),
            },
        };
        let (fields, records) = crate::json::json_to_records(&doc)?;
        let count = records.len();
        let cursor = crate::data::Cursor::in_memory(alias.to_ascii_uppercase(), fields, records);
        self.data.install(cursor);
        Ok(count)
    }

    /// The handler a request matches, and what its named parts matched. The host asks this the
    /// moment a request arrives, so that routing stays where the routes were registered.
    pub fn route(&self, server: Handle, method: &str, path: &str) -> Option<(u32, Vec<(String, String)>)> {
        self.natives.server(server)?.handler(method, path)
    }

    /// The members `AMEMBERS()` and `GETPEM()` are to see: the VM's own objects answer for
    /// themselves, everything else is the host's to answer for.
    pub fn object_members(&self, host: &mut dyn Host, h: Handle) -> Option<Vec<crate::host::MemberInfo>> {
        match self.natives.member_info(h) {
            Some(members) => Some(members),
            None => host.members(h),
        }
    }

    fn check_object(&self, host: &mut dyn Host, v: &Value, what: &str) -> Result<Handle, RtError> {
        let h = match v.deref() {
            Value::Object(h) => h,
            _ => return Err(RtError::not_an_object(what)),
        };
        if crate::foxscript::in_range(h) {
            return if self.natives.contains(h) { Ok(h) } else { Err(RtError::object_not_valid()) };
        }
        if host.object_class(h).is_none() {
            return Err(RtError::object_not_valid());
        }
        Ok(h)
    }

    // ---------------------------------------------------------------------------------------
    // the interpreter
    // ---------------------------------------------------------------------------------------

    fn exec(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        module: &Rc<Module>,
        instr: &Instr,
    ) -> Result<Flow, RtError> {
        macro_rules! pop {
            () => {
                fb.stack.pop().ok_or_else(|| RtError::new(0, "Stack underflow"))?
            };
        }
        // for instructions that may suspend and run again: the operand has to still be there
        macro_rules! peek {
            () => {
                fb.stack.last().cloned().ok_or_else(|| RtError::new(0, "Stack underflow"))?
            };
        }
        macro_rules! binop {
            ($f:expr) => {{
                let b = pop!();
                let a = pop!();
                fb.stack.push($f(&a, &b)?);
            }};
        }
        // arithmetic also works out how wide its answer prints, and SET DECIMALS is part of that
        macro_rules! arith {
            ($f:expr) => {{
                let b = pop!();
                let a = pop!();
                fb.stack.push($f(&a, &b, &self.settings)?);
            }};
        }
        macro_rules! cmp {
            ($op:expr) => {{
                let b = pop!();
                let a = pop!();
                fb.stack.push(value::compare(&a, &b, $op, &self.settings)?);
            }};
        }
        match instr {
            Instr::Stmt(line) => {
                let fr = fb.frames.last_mut().expect("frame");
                fr.line = *line;
                fr.stmt_sp = fb.stack.len();
                if let Some(reason) = self.stop_here(fb, *line) {
                    return Ok(self.stop(fb, reason));
                }
            }
            Instr::Const(i) => fb.stack.push(match &module.consts[*i as usize] {
                // a number the program wrote down keeps the width it was written in, and stays
                // marked as written so arithmetic between two of them follows the written rules
                Constant::Num(n, chars, decimals) => {
                    Value::Number(*n, value::Width { chars: *chars, decimals: *decimals, written: true })
                }
                Constant::Money(c) => Value::Currency(*c),
                Constant::Str(s) => Value::str(s),
                Constant::Date(d) => Value::Date(*d),
                Constant::DateTime(t) => Value::DateTime(*t),
                Constant::Bool(b) => Value::Logical(*b),
                Constant::Null => Value::Null,
                // arrays only ever appear as a class property value, never as code's operand
                Constant::Array(_) => Value::Null,
            }),
            Instr::True => fb.stack.push(Value::Logical(true)),
            Instr::False | Instr::Omitted => fb.stack.push(Value::Logical(false)),
            Instr::Null => fb.stack.push(Value::Null),
            Instr::Pop => {
                pop!();
            }
            Instr::Dup => {
                let v = fb.stack.last().cloned().ok_or_else(|| RtError::new(0, "Stack underflow"))?;
                fb.stack.push(v);
            }
            Instr::LoadLocal(s) => {
                let v = self.load_local(fb, *s)?;
                fb.stack.push(v);
            }
            Instr::StoreLocal(s) => {
                let v = pop!();
                self.store_local(fb, *s, v);
            }
            Instr::LoadName(n) => {
                let name = module.names[*n as usize].clone();
                // A field of the selected table wins over a memory variable of the same name,
                // which is what `m.` exists to override; a name that is neither is an error.
                if self.data.cursor().is_some_and(|c| c.has_field(&name)) {
                    match self.read_field(fb, None, &name)? {
                        FieldRead::Value(v) => fb.stack.push(v),
                        FieldRead::Suspend(req) => return Ok(self.ask(fb, req)),
                    }
                } else if let Some(v) = self.load_name(fb, &name) {
                    fb.stack.push(v);
                } else if let Some(alias) = self.query_field(fb, &name) {
                    // inside a query a name that is nothing else is a field of one of its
                    // sources, which is how a join names the columns it did not have to qualify
                    match self.read_field(fb, Some(&alias), &name)? {
                        FieldRead::Value(v) => fb.stack.push(v),
                        FieldRead::Suspend(req) => return Ok(self.ask(fb, req)),
                    }
                } else {
                    return Err(RtError::variable_not_found(&name));
                }
            }
            Instr::StoreName(n) => {
                let v = pop!();
                self.store_name(fb, &module.names[*n as usize], v)?;
            }
            Instr::StoreByName => {
                // `STORE x TO (cName)`: the name comes off, the value stays where it is, and
                // the assignment for that name is compiled and run in this frame - so it
                // reaches locals, properties and array elements, as writing the name out would
                let name = pop!().as_str()?.trim().to_string();
                if name.is_empty() {
                    return Err(RtError::syntax("the name to store to is empty"));
                }
                let top = fb.frames.last().expect("frame");
                let (m, f) = (top.module, top.func);
                let locals = self.proto(m, f).locals.clone();
                let id = self.compile_inline(Inline::StoreTo, m, f, &name, &locals)?;
                self.push_inline(fb, id, false);
            }
            Instr::DeclareNamed { public } => {
                let name = pop!().as_str()?.trim().to_ascii_uppercase();
                let env = env_index(fb, fb.frames.len() - 1);
                if *public {
                    self.globals.entry(name).or_insert(Value::Logical(false));
                } else {
                    fb.frames[env].privates.insert(name, Value::Logical(false));
                }
            }
            Instr::DeclLocal(s) => {
                let env = env_index(fb, fb.frames.len() - 1);
                let locals = &mut fb.frames[env].locals;
                if (*s as usize) >= locals.len() {
                    locals.resize(*s as usize + 1, Value::Logical(false));
                }
                locals[*s as usize] = Value::Logical(false);
            }
            Instr::DeclPrivate(n) => {
                let env = env_index(fb, fb.frames.len() - 1);
                fb.frames[env].privates.insert(module.names[*n as usize].clone(), Value::Logical(false));
            }
            Instr::DeclPublic(n) => {
                self.globals.entry(module.names[*n as usize].clone()).or_insert(Value::Logical(false));
            }
            Instr::ParamToPrivate { arg, name } => {
                let env = env_index(fb, fb.frames.len() - 1);
                let v = fb.frames[env].locals.get(*arg as usize).cloned().unwrap_or(Value::Logical(false));
                fb.frames[env].privates.insert(module.names[*name as usize].clone(), v);
            }
            Instr::MakeArray(ndims) => {
                let mut dims = pop_n(fb, *ndims as usize)?;
                dims.truncate(*ndims as usize);
                let (rows, cols) = array_dims(&dims)?;
                fb.stack.push(Value::Array(Rc::new(RefCell::new(FoxArray::new(rows, cols)))));
            }
            Instr::Dim { target, ndims } => {
                let mut dims = Vec::with_capacity(*ndims as usize);
                for _ in 0..*ndims {
                    dims.push(pop!());
                }
                dims.reverse();
                let (rows, cols) = array_dims(&dims)?;
                let existing = match target {
                    Var::Local(s) => self.load_local(fb, *s).unwrap_or(Value::Logical(false)),
                    Var::Name(n) => self.load_name(fb, &module.names[*n as usize]).unwrap_or(Value::Logical(false)),
                };
                if let Value::Array(a) = existing {
                    a.borrow_mut().redim(rows, cols);
                } else {
                    let arr = Value::Array(Rc::new(RefCell::new(FoxArray::new(rows, cols))));
                    self.store_var(fb, *target, module, arr)?;
                }
            }
            Instr::LoadIndex(n) => {
                let subs = pop_subscripts(fb, *n)?;
                let arr = pop!();
                let v = index_array(&arr, &subs)?;
                fb.stack.push(v);
            }
            Instr::StoreIndex(n) => {
                let subs = pop_subscripts(fb, *n)?;
                let arr = pop!();
                let v = pop!();
                match arr.deref() {
                    Value::Array(a) => a.borrow_mut().set(&subs, value::held_in_variable(v.deref()))?,
                    _ => return Err(RtError::invalid_subscript()),
                }
            }
            Instr::Ref(var) => {
                let cell = self.slot_mut(fb, *var, module)?;
                if !matches!(cell, Value::Ref(_)) {
                    let old = std::mem::replace(cell, Value::Null);
                    *cell = Value::Ref(Rc::new(RefCell::new(old)));
                }
                let r = cell.clone();
                fb.stack.push(r);
            }
            Instr::RefOrMakeArray(var) => {
                // The array a function is about to fill: a name nothing has declared becomes
                // one here, which is what Visual FoxPro does for AFIELDS() and its family.
                // Where the two part company is a call that finds nothing: VFP leaves the name
                // undefined, and this leaves the empty array it made. Both answer 0, which is
                // what a program reads to know there was nothing.
                if let Var::Name(n) = var {
                    let upper = &module.names[*n as usize];
                    if self.load_name(fb, upper).is_none() {
                        let empty = Value::Array(Rc::new(RefCell::new(FoxArray::new(1, 1))));
                        self.store_name(fb, upper, empty)?;
                    }
                }
                let cell = self.slot_mut(fb, *var, module)?;
                if !matches!(cell, Value::Ref(_)) {
                    let old = std::mem::replace(cell, Value::Null);
                    *cell = Value::Ref(Rc::new(RefCell::new(old)));
                }
                let r = cell.clone();
                fb.stack.push(r);
            }
            Instr::ReleaseLocal(slot) => {
                let env = env_index(fb, fb.frames.len() - 1);
                let released = &mut fb.frames[env].released;
                let slot = *slot as usize;
                if released.len() <= slot {
                    released.resize(slot + 1, false);
                }
                released[slot] = true;
            }
            Instr::ReleaseName(n) => {
                let name = &module.names[*n as usize];
                let env = env_index(fb, fb.frames.len() - 1);
                // a LOCAL is a slot rather than an entry in a map, so letting go of one is
                // marking it: the name reads as undefined until something writes it again
                let slot = self
                    .proto(fb.frames[env].module, fb.frames[env].func)
                    .locals
                    .iter()
                    .position(|n| n == name);
                if let Some(slot) = slot {
                    let released = &mut fb.frames[env].released;
                    if released.len() <= slot {
                        released.resize(slot + 1, false);
                    }
                    released[slot] = true;
                } else if fb.frames[env].privates.remove(name).is_none() {
                    self.globals.remove(name);
                }
            }
            Instr::ReleaseAll => {
                let env = env_index(fb, fb.frames.len() - 1);
                fb.frames[env].privates.clear();
                fb.frames[env].locals.fill(Value::Logical(false));
                self.globals.clear();
            }
            Instr::Neg => {
                let a = pop!();
                fb.stack.push(value::neg(&a)?);
            }
            Instr::Not => {
                let a = pop!();
                fb.stack.push(value::not(&a)?);
            }
            Instr::Add => arith!(value::add),
            Instr::Sub => arith!(value::sub),
            Instr::Mul => arith!(value::mul),
            Instr::Div => arith!(value::div),
            Instr::Mod => arith!(value::modulo),
            Instr::Pow => arith!(value::pow),
            Instr::Contains => binop!(value::contains),
            Instr::And => binop!(value::and),
            Instr::Or => binop!(value::or),
            Instr::Eq => cmp!(CmpOp::Eq),
            Instr::ExactEq => cmp!(CmpOp::ExactEq),
            Instr::Ne => cmp!(CmpOp::Ne),
            Instr::Lt => cmp!(CmpOp::Lt),
            Instr::Le => cmp!(CmpOp::Le),
            Instr::Gt => cmp!(CmpOp::Gt),
            Instr::Ge => cmp!(CmpOp::Ge),
            Instr::Jump(t) => fb.frames.last_mut().expect("frame").pc = *t as usize,
            Instr::JumpIfFalse(t) => {
                let v = pop!();
                if !v.truthy()? {
                    fb.frames.last_mut().expect("frame").pc = *t as usize;
                }
            }
            Instr::JumpIfTrue(t) => {
                let v = pop!();
                if v.truthy()? {
                    fb.frames.last_mut().expect("frame").pc = *t as usize;
                }
            }
            Instr::JumpIfFalseKeep(t) => {
                let top = fb.stack.last().map(Value::deref).ok_or_else(|| RtError::new(0, "Stack underflow"))?;
                match top {
                    Value::Logical(false) => fb.frames.last_mut().expect("frame").pc = *t as usize,
                    Value::Logical(true) | Value::Null => {}
                    _ => return Err(RtError::type_mismatch()),
                }
            }
            Instr::JumpIfTrueKeep(t) => {
                let top = fb.stack.last().map(Value::deref).ok_or_else(|| RtError::new(0, "Stack underflow"))?;
                match top {
                    Value::Logical(true) => fb.frames.last_mut().expect("frame").pc = *t as usize,
                    Value::Logical(false) | Value::Null => {}
                    _ => return Err(RtError::type_mismatch()),
                }
            }
            Instr::ForTest(t) => {
                let step = pop!().as_number()?;
                let end = pop!().as_number()?;
                let var = pop!().as_number()?;
                if (step >= 0.0 && var > end) || (step < 0.0 && var < end) {
                    fb.frames.last_mut().expect("frame").pc = *t as usize;
                }
            }
            Instr::ForEachNext(t) => {
                let len = fb.stack.len();
                if len < 2 {
                    return Err(RtError::new(0, "Stack underflow"));
                }
                let index = fb.stack[len - 1].as_number()? as usize;
                let arr = match fb.stack[len - 2].deref() {
                    Value::Array(a) => a,
                    _ => return Err(RtError::type_mismatch()),
                };
                let item = arr.borrow().items.get(index).cloned();
                match item {
                    Some(v) => {
                        fb.stack[len - 1] = Value::number((index + 1) as f64);
                        fb.stack.push(v);
                    }
                    None => {
                        fb.stack.truncate(len - 2);
                        fb.frames.last_mut().expect("frame").pc = *t as usize;
                    }
                }
            }
            Instr::MakeLambda(func) => {
                let top = fb.frames.len() - 1;
                let env = env_index(fb, top);
                let module_id = fb.frames[top].module;
                let this = fb.frames[env].this;
                let proto = self.proto(module_id, *func);
                let captured: Vec<Value> = proto
                    .captures
                    .iter()
                    .map(|c| fb.frames[env].locals.get(c.from as usize).map(Value::deref).unwrap_or(Value::Logical(false)))
                    .collect();
                let id = self.make_function(FuncValue { module: module_id, func: *func, this, captured });
                fb.stack.push(Value::Function(id));
            }
            Instr::IndexOrCallValue(argc) => {
                let args = pop_n(fb, *argc as usize)?;
                let base = pop!().deref();
                match base {
                    Value::Function(id) => self.push_function_call(fb, id, args, false)?,
                    _ => {
                        let subs: Vec<usize> = args.iter().map(Value::as_usize).collect::<Result<_, _>>()?;
                        let v = index_array(&base, &subs)?;
                        fb.stack.push(v);
                    }
                }
            }
            Instr::IndexOrCall { name, argc } => {
                let args = pop_n(fb, *argc as usize)?;
                let upper = &module.names[*name as usize];
                if let Some(Value::Function(id)) = self.load_name(fb, upper) {
                    self.push_function_call(fb, id, args, false)?;
                    return Ok(Flow::Next);
                }
                if let Some(Value::Array(a)) = self.load_name(fb, upper) {
                    if args.is_empty() || args.len() > 2 {
                        return Err(RtError::invalid_subscript());
                    }
                    let subs: Vec<usize> = args.iter().map(Value::as_usize).collect::<Result<_, _>>()?;
                    let v = a.borrow().get(&subs)?;
                    fb.stack.push(v);
                } else {
                    let current = fb.frames.last().expect("frame").module;
                    if let Some(request) = self.dll_call(fb, upper, &args)? {
                        return Ok(request);
                    }
                    // A library `SET LIBRARY TO` loaded adds its functions to the language, so
                    // its names are looked for before the program's own procedures. Measured:
                    // only this form reaches them - `DO Hash WITH "a", 5` looks for hash.prg.
                    if let Some(&(library, f)) = self.library_funcs.get(upper) {
                        let v = host.call_library(library, f, &args)?;
                        fb.stack.push(v);
                        return Ok(Flow::Next);
                    }
                    // measured: a name that is no routine anywhere is looked for as a program
                    // of that name, which runs and answers with what it returns - and when there
                    // is none, it is error 1 as a DO of it would be
                    if let Some((m, f)) = self.find_function(current, upper) {
                        self.push_call(fb, m, f, None, args, FrameKind::Call { discard: false }, true)?;
                    } else if let Some(id) = self.find_program(upper).or_else(|| host.resolve_program(upper)) {
                        self.procedure_only.remove(&id);
                        self.push_call(fb, id, 0, None, args, FrameKind::Call { discard: false }, true)?;
                    } else {
                        fb.pending =
                            Some(Pending::LoadedProgram { func: None, args, discard: false, program: upper.to_string() });
                        return Ok(Flow::Suspend(HostRequest::LoadProgram { name: upper.to_string() }));
                    }
                }
            }
            Instr::CallBuiltin { id, argc } => {
                let args = pop_n(fb, *argc as usize)?;
                let spec = builtins::by_id(*id);
                let result = {
                    let mut ctx = Ctx { vm: self, host, fiber: fb };
                    (spec.func)(&mut ctx, args)?
                };
                match result {
                    BuiltinResult::Value(v) => fb.stack.push(v.deref()),
                    BuiltinResult::SuspendData { request, args } => {
                        // the arguments go back where they were: this instruction runs again
                        for a in args {
                            fb.stack.push(a);
                        }
                        return Ok(self.ask(fb, request));
                    }
                    BuiltinResult::SuspendFile(req) => {
                        fb.pending = Some(Pending::FileResult);
                        return Ok(Flow::Suspend(req));
                    }
                    BuiltinResult::CallFunction { function, args } => match function.deref() {
                        Value::Function(id) => self.push_function_call(fb, id, args, false)?,
                        _ => return Err(RtError::type_mismatch()),
                    },
                    BuiltinResult::RunScript { source, args } => {
                        // a frame of its own, so the script's LOCALs go when it does and what it
                        // returns lands where the function's value was going to
                        let m = self.compile_inline(Inline::Program, 0, 0, &source, &[])?;
                        self.push_call(fb, m, 0, None, args, FrameKind::Call { discard: false }, false)?;
                    }
                    BuiltinResult::Suspend(mut req) => {
                        // CREATEOBJECT() of a class the program defines carries its definition.
                        // A call that names the file to read the class out of does not: measured,
                        // `NEWOBJECT("x", "lib.vcx")` builds the library's class even where the
                        // running program has a `DEFINE CLASS x` of its own.
                        if let HostRequest::CreateObject { class, definition, module, .. } = &mut req
                            && module.is_empty()
                        {
                            let current = fb.frames.last().map(|f| f.module);
                            *definition = self.class_definition(current, class);
                        }
                        fb.pending = Some(Pending::Push);
                        return Ok(Flow::Suspend(req));
                    }
                }
            }
            Instr::DoMenu(name) => {
                let menu = module.names[*name as usize].clone();
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::DoMenu { name: menu }));
            }
            Instr::DoDynamic { argc, in_prog } => {
                let args = pop_n(fb, *argc as usize)?;
                let upper = pop!().as_str()?.to_ascii_uppercase();
                let in_prog =
                    if *in_prog { Some(pop_value(fb)?.as_str()?.trim().to_ascii_uppercase()) } else { None };
                // a menu file installs a menu, exactly as the written-out form does
                if in_prog.is_none() && [".FXM", ".MNX", ".MPR"].iter().any(|e| upper.ends_with(e)) {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(HostRequest::DoMenu { name: upper }));
                }
                return self.run_do(fb, host, upper, args, in_prog);
            }
            Instr::Do { name, argc, in_prog } => {
                let args = pop_n(fb, *argc as usize)?;
                let upper = module.names[*name as usize].clone();
                let in_prog =
                    if *in_prog { Some(pop_value(fb)?.as_str()?.trim().to_ascii_uppercase()) } else { None };
                return self.run_do(fb, host, upper, args, in_prog);
            }
            Instr::Return => {
                let v = pop!();
                return Ok(Flow::Return(v.deref()));
            }
            Instr::ReturnTo(name) => {
                // the unwinding itself is the caller's loop: this only says where to stop
                fb.return_to = Some(module.names[*name as usize].clone());
                return Ok(Flow::Return(Value::Logical(false)));
            }
            Instr::EndOfCode => {
                let v = pop!();
                return Ok(Flow::EndOfCode(v.deref()));
            }
            Instr::LoadThis => {
                let env = env_index(fb, fb.frames.len() - 1);
                let h = fb.frames[env].this.ok_or_else(|| this_error("THIS"))?;
                fb.stack.push(Value::Object(h));
            }
            Instr::LoadThisForm => {
                let env = env_index(fb, fb.frames.len() - 1);
                let h = fb.frames[env].this.ok_or_else(|| this_error("THISFORM"))?;
                let form = self.this_form(host, h);
                fb.stack.push(Value::Object(form));
            }
            Instr::LoadThisFormSet => {
                let env = env_index(fb, fb.frames.len() - 1);
                let h = fb.frames[env].this.ok_or_else(|| this_error("THISFORMSET"))?;
                let set = self.container_of(host, h, "Formset").ok_or_else(|| not_contained_in("FORMSET"))?;
                fb.stack.push(Value::Object(set));
            }
            Instr::LoadScreen => fb.stack.push(Value::Object(Handle(0))),
            Instr::GetMember(m) => {
                let obj = pop!();
                let name = &module.members[*m as usize];
                if let Some(v) = json_member(&obj, name) {
                fb.stack.push(v?);
                return Ok(Flow::Next);
                }

                let h = self.check_object(host, &obj, name)?;
                if let Some(v) = self.native_member(h, name) {
                    fb.stack.push(v?);
                    return Ok(Flow::Next);
                }
                match host.get_member(h, name)? {
                    Member::Child(c) => fb.stack.push(Value::Object(c)),
                    Member::Property => {
                        let v = host.get_prop(h, name)?;
                        fb.stack.push(v.deref());
                    }
                    Member::Method | Member::None => return Err(RtError::unknown_member(name)),
                }
            }
            Instr::SetMember(m) => {
                let obj = pop!();
                let v = pop!();
                let name = &module.members[*m as usize];
                let h = self.check_object(host, &obj, name)?;
                if self.natives.contains(h) {
                    return Err(native_write_refused(&self.natives, h, name));
                }
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::SetProp {
                    obj: h.0,
                    name: name.clone(),
                    value: JsonValue::from_value(&v),
                }));
            }
            Instr::SetMemberIndex { name, argc } => {
                let subs = pop_subscripts(fb, *argc)?;
                let obj = pop!();
                let v = pop!();
                let name = &module.members[*name as usize];
                let h = self.check_object(host, &obj, name)?;
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::SetPropIndex {
                    obj: h.0,
                    name: name.clone(),
                    index: subs.iter().map(|s| *s as u32).collect(),
                    value: JsonValue::from_value(&v),
                }));
            }
            Instr::DimMember { name, ndims } => {
                let mut dims = pop_n(fb, *ndims as usize)?;
                dims.truncate(*ndims as usize);
                let (rows, cols) = array_dims(&dims)?;
                let obj = pop!();
                let name = &module.members[*name as usize];
                let h = self.check_object(host, &obj, name)?;
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::DimProp {
                    obj: h.0,
                    name: name.clone(),
                    rows: rows as u32,
                    cols: cols as u32,
                }));
            }
            Instr::GetMemberByName => {
                // `obj.&cName`: the variable holds the member name, so it is read at run time
                let name = pop!().as_str()?.trim().to_string();
                let obj = pop!();
                if let Some(v) = json_member(&obj, &name) {
                fb.stack.push(v?);
                return Ok(Flow::Next);
                }

                let h = self.check_object(host, &obj, &name)?;
                if let Some(v) = self.native_member(h, &name) {
                    fb.stack.push(v?);
                    return Ok(Flow::Next);
                }
                match host.get_member(h, &name)? {
                    Member::Child(c) => fb.stack.push(Value::Object(c)),
                    Member::Property => {
                        let v = host.get_prop(h, &name)?;
                        fb.stack.push(v.deref());
                    }
                    Member::Method | Member::None => return Err(RtError::unknown_member(&name)),
                }
            }
            Instr::SetMemberByName => {
                // `.&cMemberCount = v`: the variable holds the member name, so it is read here
                let name = pop!().as_str()?.trim().to_string();
                let obj = pop!();
                let v = pop!();
                let h = self.check_object(host, &obj, &name)?;
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::SetProp {
                    obj: h.0,
                    name,
                    value: JsonValue::from_value(&v),
                }));
            }
            Instr::CallMethodByName(argc) => {
                let args = pop_n(fb, *argc as usize)?;
                let name = pop!().as_str()?.trim().to_string();
                let obj = pop!();
                let h = self.check_object(host, &obj, &name)?;
                if self.natives.contains(h) {
                    return self.native_call(fb, h, &name, args);
                }
                fb.pending = Some(Pending::Push);
                return Ok(Flow::Suspend(HostRequest::CallMethod {
                    obj: h.0,
                    name,
                    args: args.iter().map(JsonValue::from_value).collect(),
                }));
            }
            Instr::CallMethod { name, argc } => {
                let args = pop_n(fb, *argc as usize)?;
                let obj = pop!();
                let name = &module.members[*name as usize];
                let h = self.check_object(host, &obj, name)?;
                if self.natives.contains(h) {
                    return self.native_call(fb, h, name, args);
                }
                fb.pending = Some(Pending::Push);
                return Ok(Flow::Suspend(HostRequest::CallMethod {
                    obj: h.0,
                    name: name.clone(),
                    args: args.iter().map(JsonValue::from_value).collect(),
                }));
            }
            Instr::PushWith => {
                let v = pop!();
                let env = env_index(fb, fb.frames.len() - 1);
                fb.frames[env].with_stack.push(v.deref());
            }
            Instr::PopWith => {
                let env = env_index(fb, fb.frames.len() - 1);
                fb.frames[env].with_stack.pop();
            }
            Instr::LoadWith => {
                let env = env_index(fb, fb.frames.len() - 1);
                let v = fb.frames[env]
                    .with_stack
                    .last()
                    .cloned()
                    .ok_or_else(|| RtError::new(RtError::OUTSIDE_WITH, "Expression is not valid outside of WITH/ENDWITH."))?;
                fb.stack.push(v);
            }
            Instr::ReleaseObject => {
                let obj = pop!();
                let h = self.check_object(host, &obj, "object")?;
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::ReleaseObject { obj: h.0 }));
            }
            Instr::Macro => {
                let text = pop!().as_str()?.to_string();
                let top = fb.frames.last().expect("frame");
                let (m, f) = (top.module, top.func);
                let locals = self.proto(m, f).locals.clone();
                let id = self.compile_inline(Inline::Expression, m, f, &text, &locals)?;
                self.push_inline(fb, id, true);
            }
            Instr::ExecMacroText(c) => {
                let source = match &module.consts[*c as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let text = self.expand_macros(host, fb, &source)?;
                let top = fb.frames.last().expect("frame");
                let (m, f) = (top.module, top.func);
                let locals = self.proto(m, f).locals.clone();
                let id = self.compile_inline(Inline::ExpandedLine, m, f, &text, &locals)?;
                self.push_inline(fb, id, false);
            }
            Instr::ToText => {
                let v = pop!();
                fb.stack.push(Value::str(value::display(&v, &self.settings)));
            }
            Instr::MergeText { always } => {
                let raw = pop!().as_str()?.to_string();
                // which characters the delimiters are is what SET TEXTMERGE DELIMITERS last
                // said, so the text is split here rather than when the line was compiled
                let text = if *always || self.settings.textmerge {
                    let (open, close) = self.settings.textmerge_delimiters.clone();
                    let mut ctx = Ctx { vm: self, host, fiber: fb };
                    builtins::string::merge(&mut ctx, &raw, &open, &close)?
                } else {
                    raw
                };
                fb.stack.push(Value::str(text));
            }
            Instr::TextOut { newline, noshow } => {
                let text = pop!().as_str()?.to_string();
                // the line end goes in front of the text, so `\` ends the line before it rather
                // than its own; `\\` carries that line on
                let lead = if *newline && self.textmerge_begun { "\r\n" } else { "" };
                self.textmerge_begun = true;
                if !noshow && !self.settings.textmerge_noshow {
                    host.output(&text, *newline);
                }
                if self.settings.textmerge_to.is_empty() {
                    return Ok(Flow::Next);
                }
                if self.settings.textmerge_memvar {
                    self.textmerge_held.push_str(lead);
                    self.textmerge_held.push_str(&text);
                    return Ok(Flow::Next);
                }
                let path = self.settings.at(&self.settings.textmerge_to.clone());
                let append = self.textmerge_written;
                self.textmerge_written = true;
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::FileWrite { path, text: format!("{lead}{text}"), append }));
            }
            Instr::Print { newline, argc } => {
                let items = pop_n(fb, *argc as usize)?;
                let text: Vec<String> = items.iter().map(|v| value::printed(v, &self.settings)).collect();
                host.output(&text.join(" "), *newline);
            }
            Instr::WaitWindow(flags) => {
                let timeout = if flags & wait_flags::HAS_TIMEOUT != 0 { Some(pop!().as_number()?) } else { None };
                let text = if flags & wait_flags::HAS_TEXT != 0 {
                    value::display(&pop!(), &self.settings)
                } else {
                    String::new()
                };
                let wants_key = flags & wait_flags::WANT_KEY != 0;
                // a key already in the type-ahead buffer is the one this WAIT was waiting for,
                // so nobody is asked: the program carries on with it, and it is echoed where
                // the cursor stands, as a key a person typed would be
                if flags & wait_flags::NOWAIT == 0
                    && let Some(key) = self.screen.take_key()
                {
                    host.output(&key.to_string(), false);
                    if wants_key {
                        fb.stack.push(Value::str(key.to_string()));
                    }
                    return Ok(Flow::Next);
                }
                fb.pending = Some(if wants_key { Pending::WaitKey } else { Pending::Discard });
                return Ok(Flow::Suspend(HostRequest::WaitWindow {
                    text,
                    nowait: flags & wait_flags::NOWAIT != 0,
                    timeout,
                    clear: flags & wait_flags::CLEAR != 0,
                }));
            }
            Instr::ReadEvents => {
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::ReadEvents));
            }
            Instr::ClearEvents => {
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::ClearEvents));
            }
            Instr::Quit => {
                fb.pending = Some(Pending::Finish);
                return Ok(Flow::Suspend(HostRequest::Quit));
            }
            Instr::Cancel => {
                fb.pending = Some(Pending::Finish);
                return Ok(Flow::Suspend(HostRequest::Cancel));
            }
            Instr::DoForm { flags, argc } => {
                let args = pop_n(fb, *argc as usize)?;
                let name = pop!().as_str()?.to_string();
                fb.pending = Some(Pending::FinishDoForm { flags: *flags });
                return Ok(Flow::Suspend(HostRequest::DoForm {
                    name,
                    args: args.iter().map(JsonValue::from_value).collect(),
                    modal: None,
                    linked: flags & form_flags::LINKED != 0,
                    noshow: flags & form_flags::NOSHOW != 0,
                    want_object: flags & form_flags::WANT_OBJECT != 0,
                    want_result: flags & form_flags::WANT_RESULT != 0,
                }));
            }
            Instr::SetCmd { name, argc, to } => {
                // SET PROCEDURE comes back here once per file the host had to load
                if fb.procedure_load.is_some() && fb.data_reply.is_some() {
                    let reply = fb.data_reply.take();
                    return self.continue_procedure_load(host, fb, reply);
                }
                // SET INDEX has files to read, so it comes back here once per file with the
                // bytes of one; every other setting is answered where it stands
                if let Some(answer) = fb.data_reply.take() {
                    // SET CLASSLIB has been read by the host, which answers with the list it
                    // now holds - the list being the host's, since the libraries are.
                    if std::mem::take(&mut fb.classlib_reply) {
                        self.settings.remembered.insert("CLASSLIB".to_string(), answer.as_str()?.to_string());
                    } else {
                        self.accept_idx(&answer)?;
                    }
                } else {
                    let args = pop_n(fb, *argc as usize)?;
                    let setting = &module.names[*name as usize];
                    // SET STEP ON is not a setting but a command: it stops the program where it
                    // stands, as a breakpoint on that line would, and there is nothing to read back.
                    if setting == "STEP" {
                        if on_word(args.first()) {
                            return Ok(self.stop(fb, BreakReason::SetStep));
                        }
                        return Ok(Flow::Next);
                    }
                    // A class library is a file the host reads, which it cannot do while the VM
                    // is on the stack, so this one waits where `SET LIBRARY TO` - whose own file
                    // is loaded through the host in `set_cmd` - does not.
                    if setting == "RELEASE PROCEDURE" {
                        let names = args.iter().map(|v| v.as_str().map(|s| s.to_string())).collect::<Result<Vec<_>, _>>()?;
                        self.release_procedures(&names);
                        return Ok(Flow::Next);
                    }
                    // `SET PROCEDURE TO a, b [ADDITIVE]`: the additive flag, then the files
                    if setting == "PROCEDURE" {
                        let additive = match args.first() {
                            Some(v) => v.truthy()?,
                            None => false,
                        };
                        let mut names = Vec::new();
                        for v in args.iter().skip(1) {
                            let name = v.as_str()?.trim().to_string();
                            if !name.is_empty() {
                                names.push(name);
                            }
                        }
                        fb.procedure_load = Some(ProcedureLoad { additive, names, ..Default::default() });
                        return self.continue_procedure_load(host, fb, None);
                    }
                    if setting == "CLASSLIB" {
                        let request = self.classlib_request(&args)?;
                        fb.classlib_reply = true;
                        return Ok(self.ask(fb, request));
                    }
                    self.set_cmd(host, fb, setting, &args, *to)?;
                }
                if let Some(req) = self.next_idx_request() {
                    return Ok(self.ask(fb, req));
                }
            }
            Instr::Debug(verb) => match verb {
                DebugVerb::Suspend => return Ok(self.stop(fb, BreakReason::Suspend)),
                DebugVerb::Resume => {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(HostRequest::DebugResume));
                }
            },
            Instr::NoDefault => {
                fb.frames.last_mut().expect("frame").nodefault = true;
                fb.nodefault = true;
            }
            Instr::DoDefault(argc) => {
                pop_n(fb, *argc as usize)?;
                fb.stack.push(Value::Logical(true));
            }
            Instr::TryPush { catch, finally } => {
                let frame = fb.frames.len() - 1;
                let env = env_index(fb, frame);
                fb.handlers.push(TryHandler {
                    frame,
                    catch: (*catch != NO_TARGET).then_some(*catch as usize),
                    finally: (*finally != NO_TARGET).then_some(*finally as usize),
                    sp: fb.stack.len(),
                    with_len: fb.frames[env].with_stack.len(),
                    pending_len: fb.pending_rethrow.len(),
                });
            }
            Instr::TryPop => {
                if let Some(h) = fb.handlers.pop()
                    && h.finally.is_some()
                {
                    fb.pending_rethrow.push(None);
                }
            }
            Instr::Use { alias, named_alias, exclusive, online, in_area } => {
                // The path stays on the stack until the host answers, because a suspend runs
                // this instruction again and it has to find its operand where it left it.
                let named = fb.stack.last().ok_or_else(|| RtError::new(RtError::SYNTAX_ERROR, "USE has no table name"))?.as_str()?.trim().to_string();
                // `USE dbname!tablename` names a container as well as a table, and the container
                // has to be open before it can say where the table is. Reading it is a round trip
                // of its own, which happens before the one that opens the table.
                if self.db_io.is_some() {
                    let reply = fb.data_reply.take();
                    if let Some(request) = self.db_io_step(reply)? {
                        return Ok(self.ask(fb, request));
                    }
                } else if fb.data_reply.is_none()
                    && let Some(container) = self.dbc_to_attach(&named)
                {
                    let request = self.attach_database(container);
                    return Ok(self.ask(fb, request));
                }
                // what the container says wins over the default directory, and the file it names
                // is what everything after this works on: the alias, DBF(), and the error a
                // table that is listed but missing raises
                let held = self.database_table(&named);
                let path = held.as_ref().map(|(file, _)| file.clone()).unwrap_or_else(|| plain_table(named));
                match fb.data_reply.take() {
                    None => {
                        if path.is_empty() {
                            fb.stack.pop();
                            if *named_alias {
                                let _ = pop!();
                            }
                            if *in_area {
                                let target = area_ref(&pop!())?;
                                let here = self.data.current_area();
                                self.data.select(&target)?;
                                let flow = self.close_tables(host, fb, false);
                                self.data.select(&AreaRef::Number(here))?;
                                return flow;
                            }
                            return self.close_tables(host, fb, false);
                        }
                        // a view of the database that is open is the SELECT it stands for,
                        // run into a cursor of its own name
                        if let Some(view) = self.view_named(&path) {
                            fb.stack.pop();
                            let named = if *named_alias { Some(pop!().as_str()?.trim().to_ascii_uppercase()) } else { None };
                            let name = named
                                .or_else(|| alias.map(|a| module.names[a as usize].clone()))
                                .unwrap_or_else(|| stem_of(&path).to_ascii_uppercase());
                            let query = if view.to_ascii_uppercase().contains(" INTO ") {
                                view
                            } else {
                                format!("{view} INTO CURSOR {name}")
                            };
                            let (m, f) = fb.frames.last().map(|fr| (fr.module, fr.func)).unwrap_or((0, 0));
                            let locals = self.proto(m, f).locals.clone();
                            let id = self.compile_inline(Inline::Line, m, f, &query, &locals)?;
                            // A view arrives transactable: the product answers .T. for
                            // ISTRANSACTABLE on one that has only just been opened - measured -
                            // where a plain table has to be told with MAKETRANSACTABLE first.
                            self.cursor_flags.insert((name.to_ascii_uppercase(), "TRANSACTABLE".to_string()));
                            self.push_inline(fb, id, false);
                            return Ok(Flow::Next);
                        }
                        // only a table the open database holds is announced, and by the name the
                        // container holds it under rather than the file the program wrote
                        if let Some(held) = self.db_table_name(&path)
                            && !self.dbc_event(host, fb, "dbc_BeforeOpenTable", &[DbcArg::Name(held)])
                        {
                            fb.stack.pop();
                            if *named_alias {
                                let _ = pop!();
                            }
                            return Err(Self::dbc_refused(&self.settings.table_at(&path)));
                        }
                        let search = self.settings.table_search(&path);
                        let path = self.settings.table_at(&path);
                        return Ok(self.ask(fb, HostRequest::DataOpen { path, exclusive: *exclusive, search }));
                    }
                    Some(reply) => {
                        fb.stack.pop();
                        let named = if *named_alias { Some(pop!().as_str()?.trim().to_ascii_uppercase()) } else { None };
                        // `USE x IN 0` opens the table somewhere else and leaves the selection
                        let restore = if *in_area {
                            let target = area_ref(&pop!())?;
                            let here = self.data.current_area();
                            self.data.select(&target)?;
                            Some(here)
                        } else {
                            None
                        };
                        // A table reached through its container answers to the name the container
                        // holds it under rather than to the file's own stem. Measured: a table
                        // renamed to aVeryLongTableName still sits in shortf.dbf, and
                        // `USE mydb!aVeryLongTableName` opens it under AVERYLONGTABLENAME.
                        let alias = named
                            .or_else(|| alias.map(|a| module.names[a as usize].clone()))
                            .or_else(|| held.map(|(_, name)| name.to_ascii_uppercase()));
                        let opened = self.open_table(fb, &path, alias, reply);
                        if let Some(here) = restore {
                            self.data.select(&AreaRef::Number(here))?;
                        }
                        opened?;
                        // the After event is handed the alias the table was opened under, where
                        // the Before one is handed the table's name - measured with
                        // `USE t1 AGAIN IN 0 ALIAS second`, which says "t1" then "second"
                        if self.db_table_name(&path).is_some() {
                            let alias = self.data.cursor().map(|c| c.alias.clone()).unwrap_or_default();
                            self.dbc_event(host, fb, "dbc_AfterOpenTable", &[DbcArg::Name(alias)]);
                        }
                        // ONLINE puts an offline view back online and ADMIN opens one to be
                        // managed. Nothing here is ever offline - there is no CREATEOFFLINE to
                        // make one - so what was opened never is what those two asked for, and
                        // VFP answers that with 2009 having opened the table anyway.
                        if *online {
                            return Err(RtError::new(
                                RtError::NOT_OFFLINE_VIEW,
                                format!("Object is not an offline view: {}", stem_of(&path)),
                            ));
                        }
                    }
                }
            }
            Instr::PushArea => {
                let what = match pop!().deref() {
                    Value::Str(s) => AreaRef::Alias(s.trim().to_string()),
                    v => AreaRef::Number(v.as_number()?.max(0.0) as usize),
                };
                self.area_stack.push(self.data.current_area());
                self.data.select(&what)?;
            }
            Instr::PopArea => {
                if let Some(area) = self.area_stack.pop() {
                    self.data.select(&AreaRef::Number(area))?;
                }
            }
            Instr::SelectArea(name) => {
                // `SELECT (cName)` evaluates to either a work area number or an alias, and which
                // one it is is only known when it has been evaluated
                let what = match name {
                    Some(n) => AreaRef::Alias(module.names[*n as usize].clone()),
                    None => match pop!().deref() {
                        Value::Str(s) => AreaRef::Alias(s.trim().to_string()),
                        v => AreaRef::Number(v.as_number()?.max(0.0) as usize),
                    },
                };
                self.data.select(&what)?;
            }
            // The operand stays on the stack until the move finishes: counting past deleted
            // records can suspend, and a suspend runs the instruction again from the top.
            Instr::Go(target) => {
                let to = match target {
                    GoTarget::Top => Move::Top,
                    GoTarget::Bottom => Move::Bottom,
                    GoTarget::Record => Move::Record(peek!().as_number()?.max(0.0) as u64),
                };
                if let Some(req) = self.move_pointer(host, fb, to)? {
                    return Ok(self.ask(fb, req));
                }
                if matches!(target, GoTarget::Record) {
                    fb.stack.pop();
                }
            }
            Instr::Skip => {
                let delta = peek!().as_number()? as i64;
                if let Some(req) = self.move_pointer(host, fb, Move::Skip(delta))? {
                    return Ok(self.ask(fb, req));
                }
                fb.stack.pop();
            }
            Instr::CloseTables { all } => {
                return self.close_tables(host, fb, *all);
            }
            Instr::ReplaceFieldNamed { additive } => {
                // both operands wait on the stack, because fetching the page runs this again
                let value = peek!();
                let name = fb.stack[fb.stack.len() - 2].deref().as_str()?.trim().to_string();
                if let Some(req) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, req));
                }
                let value = match self.added_to_memo(fb, &name, value, *additive)? {
                    Added::Value(v) => v,
                    Added::Suspend(req) => return Ok(self.ask(fb, req)),
                };
                fb.stack.truncate(fb.stack.len() - 2);
                self.data.cursor_mut().ok_or_else(no_table)?.set_field(&name, &value)?;
            }
            Instr::ReplaceField { field, additive } => {
                // the operand waits on the stack: fetching the page runs this instruction again
                let value = peek!();
                if let Some(req) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, req));
                }
                let name = module.members[*field as usize].clone();
                let value = match self.added_to_memo(fb, &name, value, *additive)? {
                    Added::Value(v) => v,
                    Added::Suspend(req) => return Ok(self.ask(fb, req)),
                };
                fb.stack.pop();
                self.data.cursor_mut().ok_or_else(no_table)?.set_field(&name, &value)?;
            }
            Instr::MarkDeleted(deleted) => {
                if let Some(req) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, req));
                }
                self.data.cursor_mut().ok_or_else(no_table)?.set_deleted(*deleted)?;
            }
            Instr::Zap => {
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                cursor.zap();
                let handle = cursor.handle();
                cursor.written();
                // the header is the record count, so emptying a table is one small write
                if let Some(handle) = handle {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(HostRequest::DataWrite { handle, recno: 1.0, bytes: Vec::new(), count: Some(0.0) }));
                }
            }
            Instr::CreateTableAllowed => {
                let path = pop!().as_str()?.trim().to_string();
                let args = [DbcArg::name(self.settings.table_at(&path)), DbcArg::name(stem_of(&path))];
                let allowed = self.dbc_event(host, fb, "dbc_BeforeCreateTable", &args);
                fb.stack.push(Value::Logical(allowed));
            }
            Instr::CreateTable { index, from_array, columns } => {
                // the array sits over the path, which the USE that follows is left holding, and
                // any column names the program worked out sit under it
                let described = if *from_array { Some(crate::data::fields_from_array(&pop!())?) } else { None };
                let nulls = self.settings.nulls_by_default();
                let worked = pop_names(fb, u16::from(*columns))?;
                let path = pop!().as_str()?.trim().to_string();
                // the container was asked before this ran, and hears that it is done when the
                // table takes its place in it - the step this statement ends with
                let mut fields = match described {
                    Some(fields) => fields,
                    None => module
                        .cursors
                        .get(*index as usize)
                        .map(|(_, defs)| defs.iter().map(|d| d.field(nulls)).collect())
                        .ok_or_else(|| {
                            RtError::new(RtError::SYNTAX_ERROR, "the table definition is missing from the module")
                        })?,
                };
                name_blank_columns(&mut fields, worked);
                let (header, memo) = crate::dbf::write::encode_header(&fields);
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::DataCreate { path: self.settings.table_at(&path), header, memo }));
            }
            Instr::DeclareDll(index) => {
                let library = pop!().as_str()?.trim().to_string();
                let proto = module.dlls.get(*index as usize).cloned().ok_or_else(|| {
                    RtError::new(RtError::SYNTAX_ERROR, "the library declaration is missing from the module")
                })?;
                self.dlls.insert(proto.called.clone(), (library, proto));
            }
            Instr::FileCommand(kind) => {
                // COPY FILE and RENAME take two names, the rest one; all pop right to left
                let two = matches!(kind, 1 | 2);
                let target = if two { pop!().as_str()?.trim().to_string() } else { String::new() };
                let path = pop!().as_str()?.trim().to_string();
                let op = match kind {
                    1 => "copy",
                    2 => "rename",
                    3 => "mkdir",
                    4 => "rmdir",
                    5 => "dir",
                    _ => "type",
                };
                fb.pending = Some(Pending::FileCommand { op: op.to_string() });
                return Ok(Flow::Suspend(HostRequest::FileOp {
                    op: op.to_string(),
                    handle: 0,
                    path: self.settings.at(&path),
                    target: if two { self.settings.at(&target) } else { String::new() },
                    text: String::new(),
                    count: 0.0,
                    offset: 0.0,
                    whence: 0,
                    search: Vec::new(),
                }));
            }
            Instr::LoadTablePath => {
                let cursor = self.data.cursor().ok_or_else(no_table)?;
                fb.stack.push(Value::str(cursor.path.clone()));
            }
            Instr::Import { sheet } => {
                let base = fb.stack.len() - if *sheet { 2 } else { 1 };
                let named = fb.stack[base].clone().as_str()?.trim().to_string();
                let which = if *sheet { fb.stack[base + 1].clone().as_str()?.trim().to_string() } else { String::new() };
                let bytes = match fb.data_reply.take() {
                    Some(reply) => crate::data::bytes_of(&reply),
                    None => {
                        let path = self.settings.at(&named);
                        return Ok(self.ask(fb, HostRequest::FileReadBytes { path }));
                    }
                };
                fb.stack.truncate(base);
                let sheet_name = (!which.is_empty()).then_some(which);
                let (fields, rows) = crate::exchange::read_sheet(&bytes, sheet_name.as_deref())?;
                // the table takes the workbook's name, which is where VFP puts it
                let target = self.settings.table_at(&with_extension(&named, "dbf"));
                self.copy = Some(CopyOut {
                    path: target.clone(),
                    kind: 0,
                    fields,
                    descending: 0,
                    text: 0,
                    quote: String::new(),
                    separator: String::new(),
                    rows: rows.into_iter().map(|r| (Vec::new(), r)).collect(),
                });
                fb.stack.push(Value::str(target));
            }
            Instr::Build { what, count, recompile } => {
                let sources = pop_n(fb, *count as usize)?;
                let target = pop!();
                let kind = match &module.consts[*what as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let named = |v: &Value| -> Result<String, RtError> {
                    let text = v.as_str()?.trim().to_string();
                    Ok(if text.is_empty() { text } else { self.settings.at(&text) })
                };
                let target = named(&target)?;
                let from = sources.iter().map(named).collect::<Result<Vec<_>, _>>()?;
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::Build { what: kind, target, from, recompile: *recompile }));
            }
            Instr::Compile { what, flags } => {
                let files = pop!().as_str()?.trim().to_string();
                let kind = match &module.consts[*what as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let files = if files.is_empty() { files } else { self.settings.at(&files) };
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::Compile {
                    what: kind,
                    files,
                    all: flags & compile_flags::ALL != 0,
                    encrypt: flags & compile_flags::ENCRYPT != 0,
                    nodebug: flags & compile_flags::NODEBUG != 0,
                }));
            }
            Instr::NewDocument(kind) => {
                let kind = match &module.consts[*kind as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let named = pop!().as_str()?.trim().to_string();
                let named = named_document(&named, &kind);
                let path = if named.is_empty() { named } else { self.settings.at(&named) };
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::NewDocument { what: kind, path }));
            }
            Instr::ModifyInContainer { what, flags } => {
                let path = pop!().as_str()?.trim().to_string();
                // MODIFY DATABASE has the one event rather than a pair, and it is handed the
                // container and the two clauses that say how the designer would have opened -
                // measured with `MODIFY DATABASE NOWAIT NOEDIT`, which is the only shape of it
                // the product comes back from without a person.
                if *what == 3 {
                    // the container the designer would open: the one named, or the one that is
                    // current when the command named none
                    let named = path.trim();
                    let path = match (named.is_empty(), self.current_db) {
                        (true, Some(index)) => self.databases[index].path.clone(),
                        (true, None) => return Ok(Flow::Next),
                        _ => self.settings.at(&named_document(named, "DATABASE")),
                    };
                    if self.current_db.is_some() {
                        let args = [
                            DbcArg::name(&path),
                            DbcArg::Flag(flags & crate::ast::db_flags::NOWAIT != 0),
                            DbcArg::Flag(flags & crate::ast::db_flags::NOEDIT != 0),
                        ];
                        if !self.dbc_event(host, fb, "dbc_ModifyData", &args) {
                            return Ok(Flow::Next);
                        }
                    }
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(HostRequest::OpenDocument { path }));
                }
                let word = match what {
                    0 => "Table",
                    1 => "View",
                    _ => "Proc",
                };
                let args = if *what == 2 { Vec::new() } else { vec![DbcArg::name(stem_of(&path))] };
                if self.current_db.is_some() {
                    if !self.dbc_event(host, fb, &format!("dbc_BeforeModify{word}"), &args) {
                        return Ok(Flow::Next);
                    }
                    // the designer opens in a window of its own and the program carries on, so
                    // what follows the opening is the After event rather than the closing
                    self.dbc_event(host, fb, &format!("dbc_AfterModify{word}"), &args);
                }
                if path.is_empty() {
                    return Ok(Flow::Next);
                }
                let path = self.settings.at(&path);
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::OpenDocument { path }));
            }
            Instr::Redefined(what) => {
                let what = match &module.consts[*what as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                return Err(RtError::new(RtError::CANNOT_REDEFINE, format!("Cannot redefine {what}.")));
            }
            Instr::Unsupported(what) => {
                let what = match &module.consts[*what as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                return Err(RtError::feature_not_available(&what));
            }
            Instr::OpenDocument(kind) => {
                let named = pop!().as_str()?.to_string();
                // a cursor that lives in memory has no file to open, and asking the host for
                // one with no name is asking for the folder itself
                if named.trim().is_empty() {
                    return Ok(Flow::Next);
                }
                let kind = match &module.consts[*kind as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let path = self.settings.at(&named_document(named.trim(), &kind));
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::OpenDocument { path }));
            }
            Instr::ClearAll { tables } => {
                // CLEAR ALL lets go of everything the program made: its variables, and the tables
                self.globals.clear();
                for frame in &mut fb.frames {
                    frame.privates.clear();
                }
                if *tables {
                    return self.close_areas(fb, true);
                }
            }
            Instr::CreateCursor { index, named, columns } => {
                let nulls = self.settings.nulls_by_default();
                let (written, mut fields) = module
                    .cursors
                    .get(*index as usize)
                    .map(|(name, defs)| (name.clone(), defs.iter().map(|d| d.field(nulls)).collect::<Vec<_>>()))
                    .ok_or_else(|| {
                        RtError::new(RtError::SYNTAX_ERROR, "the cursor definition is missing from the module")
                    })?;
                let alias = if *named { pop_value(fb)?.as_str()?.trim().to_ascii_uppercase() } else { written };
                name_blank_columns(&mut fields, pop_names(fb, u16::from(*columns))?);
                return self.install_cursor(fb, alias, fields);
            }
            Instr::ReplaceFieldAt(index) => {
                let value = peek!();
                if let Some(req) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, req));
                }
                fb.stack.pop();
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                let name = cursor
                    .header
                    .fields
                    .get(*index as usize)
                    .map(|f| f.name.clone())
                    .ok_or_else(|| RtError::new(RtError::SYNTAX_ERROR, "there are more values than the table has fields"))?;
                cursor.set_field(&name, &value)?;
            }
            Instr::AppendBlank => {
                self.data.cursor_mut().ok_or_else(no_table)?.append_blank();
            }
            Instr::InsertBlank { before } => {
                if let Some(req) = self.insert_blank(fb, *before)? {
                    return Ok(self.ask(fb, req));
                }
            }
            Instr::InsertFrom { from } => {
                let row = pop!().as_number()? as usize;
                let source = pop!();
                let (more, next) = self.insert_from(host, fb, *from, &source, row)?;
                fb.stack.push(Value::Logical(more));
                fb.stack.push(Value::number(next as f64));
            }
            Instr::CreateCursorFromArray { index, named } => {
                let fields = crate::data::fields_from_array(&pop!())?;
                let written = module
                    .cursors
                    .get(*index as usize)
                    .map(|(name, _)| name.clone())
                    .ok_or_else(|| RtError::new(RtError::SYNTAX_ERROR, "the cursor name is missing from the module"))?;
                let alias = if *named { pop_value(fb)?.as_str()?.trim().to_ascii_uppercase() } else { written };
                return self.install_cursor(fb, alias, fields);
            }
            Instr::FlushRecord => {
                let reply = fb.data_reply.take();
                // a reply while a memo is out is the block its text was written at
                if let Some(cursor) = self.data.cursor_mut()
                    && cursor.awaiting_memo()
                {
                    let block = reply.as_ref().and_then(|v| v.as_number().ok()).unwrap_or(0.0) as u32;
                    cursor.memo_written(block)?;
                }
                // the record's memos go to the file beside the table first, one write each, and
                // the blocks they land at go in the record
                let next = self
                    .data
                    .cursor_mut()
                    .and_then(|c| c.handle().map(|handle| (handle, c.next_memo_write())));
                if let Some((handle, Some((_, bytes)))) = next {
                    return Ok(self.ask(fb, HostRequest::DataWriteMemo { handle, bytes }));
                }
                // the record is finished, so every tag can be brought up to date before it goes
                self.maintain_index(host, fb);
                let holding = self.txn > 0;
                if let Some(cursor) = self.data.cursor_mut()
                    && (holding || cursor.buffered())
                    && cursor.dirty()
                {
                    // the record waits here until TABLEUPDATE, or the end of the transaction
                    let touched = cursor.take_touched();
                    let appended = cursor.grew().is_some();
                    cursor.hold(&touched, appended);
                }
                if let Some(req) = self.flush_header() {
                    // the header first: the record that follows carries the count with it
                    return Ok(self.ask(fb, req));
                }
                if let Some(req) = self.flush_record()? {
                    let again = self.data.cursor().is_some_and(Cursor::index_dirty);
                    fb.pending = Some(if again { Pending::DataReply } else { Pending::Discard });
                    if again && let Some(frame) = fb.frames.last_mut() {
                        frame.pc -= 1;
                    }
                    return Ok(Flow::Suspend(req));
                }
                if let Some(req) = self.write_index(self.data.current_area()) {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(req));
                }
            }

            Instr::Unlock { record, area, all } => {
                let recno = if *record { Some(pop!().as_number()? as u64) } else { None };
                let target = area.map(|a| AreaRef::Alias(module.names[a as usize].clone()));
                if *all && target.is_none() {
                    // UNLOCK ALL lets go of every work area's locks, not just this one's
                    for n in 1..=self.data.highest_free() {
                        if let Some(cursor) = self.data.find_mut(&AreaRef::Number(n)) {
                            cursor.unlock(None);
                        }
                    }
                } else {
                    let cursor = match target {
                        Some(what) => self.data.find_mut(&what),
                        None => self.data.cursor_mut(),
                    };
                    if let Some(cursor) = cursor {
                        cursor.unlock(recno);
                    }
                }
            }
            Instr::AlterTable(ops) => {
                // the names the program worked out are above the path, in the order the changes
                // were written; they come off before the alteration takes the path itself, and
                // are kept for as long as it runs, because it reads them again as it goes
                if self.alter.is_none() {
                    let count = ops.iter().flat_map(|o| [o.name, o.extra]).filter(Option::is_none).count();
                    self.alter_names = pop_names(fb, count as u16)?;
                }
                let mut worked = self.alter_names.iter();
                let nulls = self.settings.nulls_by_default();
                let mut changes = Vec::with_capacity(ops.len());
                for op in ops {
                    let mut name_of = |written: Option<u32>| match written {
                        Some(i) => module.names.get(i as usize).cloned().unwrap_or_default(),
                        None => worked.next().map(|n| n.trim().to_ascii_uppercase()).unwrap_or_default(),
                    };
                    let name = name_of(op.name);
                    let rename = (op.kind == 3).then(|| name_of(op.extra));
                    let field = match op.kind {
                        0 | 1 => op
                            .extra
                            .and_then(|i| module.cursors.get(i as usize))
                            .and_then(|(_, defs)| defs.first())
                            .map(|d| d.field(nulls)),
                        _ => None,
                    };
                    // the definition was written without the name when the program works it out
                    let field = field.map(|f| crate::dbf::DbfField { name: name.clone(), ..f });
                    changes.push(Alteration { kind: op.kind, name, field, rename });
                }
                if let Some(req) = self.alter_table(fb, &changes)? {
                    return Ok(self.ask(fb, req));
                }
                self.alter_names.clear();
            }
            Instr::Transaction(step) => {
                // a reply here is the last write of the transaction, and says nothing more
                fb.data_reply.take();
                match step {
                    0 => self.txn += 1,
                    1 => {
                        // everything held goes out, one write at a time, and the transaction
                        // ends when there is nothing left to send
                        if let Some(req) = self.write_one_held() {
                            return Ok(self.ask(fb, req));
                        }
                        self.txn = self.txn.saturating_sub(1);
                    }
                    _ => {
                        for area in 1..=self.data.highest_free() {
                            if let Some(cursor) = self.data.find_mut(&AreaRef::Number(area)) {
                                cursor.revert(None);
                            }
                        }
                        self.txn = self.txn.saturating_sub(1);
                    }
                }
            }
            Instr::DbCommand { kind, named, target, sql, flags } => {
                if *kind == 2 {
                    return self.close_databases(host, fb, *flags);
                }
                let sql = sql.map(|c| match &module.consts[c as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                });
                if let Some(req) = self.db_command(host, fb, *kind, *named, *target, sql, *flags)? {
                    return Ok(self.ask(fb, req));
                }
                // A container that has just been read tells its own procedures so, and one
                // event follows the other: dbc_OpenData then dbc_Activate, measured.
                if let Some(open) = self.opened_database.take() {
                    self.database_opened(host, fb, open)?;
                }
            }
            Instr::ReportBegin { flags, label, to_file } => {
                if let Some(request) = self.report_begin(host, fb, *flags, *label, *to_file)? {
                    return Ok(self.ask(fb, request));
                }
            }
            Instr::ReportRow => {
                // the record has to be in hand before its fields can be worked out
                if let Some(request) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, request));
                }
                self.report_row(host, fb)?;
            }
            Instr::ReportEnd => {
                if let Some(request) = self.report_end(host, fb)? {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(request));
                }
            }
            Instr::Browse { fields, cond, flags, titled } => {
                let word = |c: Option<u32>| match c.map(|c| &module.consts[c as usize]) {
                    Some(Constant::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                let (wanted, cond) = (word(*fields), word(*cond));
                let title = if *titled { pop_value(fb)?.as_str()?.to_string() } else { String::new() };
                if let Some(request) = self.browse(host, fb, &wanted, &cond, &title, *flags)? {
                    // reading the table takes a round trip and the answer is wanted; handing
                    // the records over takes one and the answer is nothing
                    if matches!(request, HostRequest::DataRead { .. }) {
                        return Ok(self.ask(fb, request));
                    }
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(request));
                }
            }
            Instr::OnEvent { what, given } => {
                let what = match &module.consts[*what as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let command = if *given { pop_value(fb)?.as_str()?.trim().to_string() } else { String::new() };
                if command.is_empty() {
                    self.handlers.remove(&what.to_ascii_uppercase());
                } else {
                    self.handlers.insert(what.to_ascii_uppercase(), command);
                }
            }
            Instr::Keyboard { plain, clear } => {
                let keys = pop_value(fb)?.as_str()?.to_string();
                if *clear {
                    self.screen.typed.clear();
                }
                self.screen.typed.push_str(&if *plain { keys } else { crate::screen::expand_keys(&keys) });
            }
            Instr::KeyStack { push, clear } => {
                if *push {
                    self.key_stack.push(self.handlers.clone());
                } else if let Some(kept) = self.key_stack.pop() {
                    self.handlers = kept;
                }
                if *clear {
                    self.handlers.retain(|name, _| !name.starts_with("KEY"));
                }
            }
            Instr::Ask { kind, prompted, target } => {
                // the answer arrives from the host, and the instruction runs again to keep it
                if let Some(reply) = fb.data_reply.take() {
                    let text = reply.as_str().map(|s| s.to_string()).unwrap_or_default();
                    // nothing typed is a question that was not answered, and the variable is
                    // left as it was rather than turned into an empty string
                    if text.trim().is_empty() {
                        return Ok(Flow::Next);
                    }
                    let name = module.names[*target as usize].clone();
                    let value = match kind {
                        // INPUT works out what was typed; ACCEPT and GETEXPR keep it as text
                        1 => self.eval_in_frame(host, fb, &text).unwrap_or(Value::str(text)),
                        _ => Value::str(text),
                    };
                    self.store_name(fb, &name, value)?;
                    return Ok(Flow::Next);
                }
                let prompt = if *prompted { pop_value(fb)?.as_str()?.to_string() } else { String::new() };
                let title = match kind {
                    2 => "Expression",
                    _ => "Input",
                };
                return Ok(self.ask(
                    fb,
                    HostRequest::InputBox {
                        prompt,
                        title: title.to_string(),
                        default: String::new(),
                        timeout: None,
                        timeout_value: String::new(),
                    },
                ));
            }
            Instr::Memo { what, fields, pathed, named, flags } => {
                let names = match &module.consts[*fields as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let names: Vec<String> = names.split(',').filter(|s| !s.is_empty()).map(str::to_string).collect();
                let (both, noedit, nowait, all) =
                    (flags & 1 != 0, flags & 2 != 0, flags & 4 != 0, flags & 8 != 0);
                // CLOSE MEMO says only which windows to close, and what comes back is what was
                // typed in each of them
                if *what == 3 {
                    match fb.data_reply.take() {
                        Some(reply) => {
                            for window in items_of(&reply) {
                                let parts = items_of(&window);
                                let [alias, field, text] = &parts[..] else { continue };
                                let alias = alias.as_str()?.to_string();
                                let field = field.as_str()?.to_string();
                                let text = text.as_str()?.to_string();
                                self.write_memo(&alias, &field, &text)?;
                            }
                        }
                        None => {
                            let fields = if all { Vec::new() } else { names };
                            return Ok(self.ask(fb, HostRequest::CloseMemo { fields }));
                        }
                    }
                    return Ok(Flow::Next);
                }
                // a field the program named for itself is on the stack under the file, and is
                // read rather than taken, because a suspend runs this instruction again
                let name = match named {
                    true => {
                        let at = fb.stack.len() - 1 - usize::from(*pathed);
                        fb.stack.get(at).ok_or_else(|| RtError::syntax("the field name is missing"))?.deref().as_str()?.trim().to_ascii_uppercase()
                    }
                    false => names.first().cloned().unwrap_or_default(),
                };
                let (alias, field) = match name.split_once('.') {
                    Some((alias, field)) => (Some(alias.to_string()), field.to_string()),
                    None => (None, name.clone()),
                };
                // what the field holds comes first and is kept, because reading it is a round
                // trip of its own and only one answer comes back at a time: once it is in hand
                // the next answer is the file's, or the window's
                if self.memo_io.is_none() {
                    match self.read_field(fb, alias.as_deref(), &field)? {
                        FieldRead::Value(v) => self.memo_io = Some(v.as_str().unwrap_or_default().to_string()),
                        FieldRead::Suspend(req) => return Ok(self.ask(fb, req)),
                    }
                }
                let held = self.memo_io.clone().unwrap_or_default();
                match what {
                    // APPEND MEMO: the file goes after what is there, or over it
                    0 => {
                        let text = match fb.data_reply.take() {
                            Some(reply) => reply.as_str()?.to_string(),
                            None => {
                                let named = fb.stack.last().map(|v| v.deref()).unwrap_or(Value::Null);
                                let path = self.settings.at(&named.as_str()?.to_string());
                                return Ok(self.ask(fb, HostRequest::FileRead { path, search: Vec::new() }));
                            }
                        };
                        let whole = if both { text } else { held + &text };
                        self.write_memo(alias.as_deref().unwrap_or_default(), &field, &whole)?;
                        self.memo_io = None;
                        fb.stack.pop();
                    }
                    // APPEND GENERAL: the file's bytes go into the field as they stand, and
                    // naming no file empties it
                    4 => {
                        if !*pathed {
                            self.write_memo(alias.as_deref().unwrap_or_default(), &field, "")?;
                            self.memo_io = None;
                            if *named {
                                fb.stack.pop();
                            }
                            return Ok(Flow::Next);
                        }
                        let bytes = match fb.data_reply.take() {
                            // the file comes back as bytes, and a General field keeps them as
                            // they stand: nothing here reads what is inside an OLE object
                            Some(reply) => bytes_of(&reply).into_iter().map(|b| b as char).collect::<String>(),
                            None => {
                                let file = fb.stack.last().map(|v| v.deref()).unwrap_or(Value::Null);
                                let path = self.settings.at(&file.as_str()?.to_string());
                                return Ok(self.ask(fb, HostRequest::FileReadBytes { path }));
                            }
                        };
                        self.write_memo(alias.as_deref().unwrap_or_default(), &field, &bytes)?;
                        self.memo_io = None;
                        fb.stack.pop();
                        if *named {
                            fb.stack.pop();
                        }
                    }
                    // COPY MEMO: what is there goes to a file of its own
                    1 => {
                        let named = pop_value(fb)?.as_str()?.to_string();
                        let path = self.settings.at(&with_extension(&named, "txt"));
                        self.memo_io = None;
                        fb.pending = Some(Pending::Discard);
                        return Ok(Flow::Suspend(HostRequest::FileWrite { path, text: held, append: both }));
                    }
                    // MODIFY MEMO: a window on it, and what it is left with goes back in
                    _ => match fb.data_reply.take() {
                        Some(reply) => {
                            if let Ok(text) = reply.as_str() {
                                self.write_memo(alias.as_deref().unwrap_or_default(), &field, &text)?;
                            }
                            self.memo_io = None;
                        }
                        None => {
                            let of = alias.clone().unwrap_or_else(|| {
                                self.data.cursor().map(|c| c.alias.clone()).unwrap_or_default()
                            });
                            return Ok(self.ask(
                                fb,
                                HostRequest::EditMemo { alias: of, field, text: held, noedit, nowait },
                            ));
                        }
                    },
                }
            }
            Instr::Variables { save, memo, skeleton, except, additive } => {
                // the operands stay on the stack until the work is done, because asking the
                // host for the file runs the instruction again
                let base = fb.stack.len() - 1 - usize::from(*skeleton);
                let target = fb.stack[base].deref().as_str()?.to_string();
                let skel = if *skeleton { fb.stack[base + 1].deref().as_str()?.to_string() } else { String::new() };
                if *save {
                    let bytes = crate::mem::write(&self.saved_variables(fb, &skel, *except));
                    fb.stack.truncate(base);
                    if *memo {
                        let text: String = bytes.iter().map(|b| *b as char).collect();
                        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                        cursor.set_field(&target, &Value::str(text))?;
                        return Ok(Flow::Next);
                    }
                    let path = self.settings.at(&with_extension(&target, "mem"));
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(HostRequest::FileWriteBytes { path, bytes }));
                }
                let bytes = if *memo {
                    match self.read_field(fb, None, &target)? {
                        FieldRead::Value(v) => crate::data::bytes_of(&v),
                        FieldRead::Suspend(req) => return Ok(self.ask(fb, req)),
                    }
                } else {
                    match fb.data_reply.take() {
                        Some(reply) => crate::data::bytes_of(&reply),
                        None => {
                            let path = self.settings.at(&with_extension(&target, "mem"));
                            return Ok(self.ask(fb, HostRequest::FileReadBytes { path }));
                        }
                    }
                };
                fb.stack.truncate(base);
                // what is in memory goes, unless the command said to keep it
                if !*additive {
                    let env = env_index(fb, fb.frames.len() - 1);
                    fb.frames[env].privates.clear();
                    self.globals.clear();
                }
                let env = env_index(fb, fb.frames.len() - 1);
                for (name, value) in crate::mem::read(&bytes) {
                    fb.frames[env].privates.insert(name, value);
                }
            }
            Instr::Mouse { clicks, at, drags, window, style } => {
                let style = match &module.consts[*style as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let window = if *window { pop_value(fb)?.as_str()?.to_string() } else { String::new() };
                let mut points = Vec::new();
                for _ in 0..(*drags as usize + usize::from(*at)) {
                    let col = pop_value(fb)?.as_number()?;
                    let row = pop_value(fb)?.as_number()?;
                    points.push((row, col));
                }
                points.reverse();
                let at = at.then(|| points.remove(0));
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::MousePress {
                    clicks: *clicks,
                    at,
                    drag: points,
                    window,
                    style,
                }));
            }
            Instr::Run { nowait } => {
                let command = pop_value(fb)?.as_str()?.to_string();
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::RunProgram { command, nowait: *nowait }));
            }
            Instr::Diagnostic { checked, argc } => {
                let settings = self.settings.clone();
                let parts = pop_n(fb, *argc as usize)?;
                let passed = if *checked { pop_value(fb)?.truthy().unwrap_or(false) } else { true };
                // an object is not shown, only counted as one, which is what VFP writes for it
                let text: Vec<String> = parts
                    .iter()
                    .map(|v| match v.deref() {
                        Value::Object(_) => "(Object)".to_string(),
                        other => value::display(&other, &settings),
                    })
                    .collect();
                // Visual FoxPro puts a failed assertion in a dialog offering Debug, Cancel,
                // Ignore and Ignore All, and echoes it to the Debug Output either way. There is
                // no such dialog here, so what is left is the echo and carrying on, which is
                // what Ignore does.
                let said = if *checked {
                    // an assertion says nothing while it holds, and nothing at all unless
                    // SET ASSERTS is on. Without a MESSAGE it names the line and the routine.
                    if passed || !settings.asserts {
                        return Ok(Flow::Next);
                    }
                    if text.is_empty() {
                        let fr = fb.frames.last().expect("frame");
                        let name = self.proto(fr.module, fr.func).display_name.clone();
                        format!("Assertion failed on line {} of procedure {name}.", fr.line)
                    } else {
                        text.join(" ")
                    }
                } else {
                    text.join(" ")
                };
                host.output(&said, true);
                if let Some(req) = self.debug_output(&said) {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(req));
                }
            }
            Instr::Yield { events } => {
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::Settle { events: *events }));
            }
            Instr::Blank(list) => {
                let names = match &module.consts[*list as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                };
                if let Some(req) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, req));
                }
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                let wanted: Vec<crate::dbf::DbfField> = cursor
                    .header
                    .fields
                    .iter()
                    .filter(|f| names.is_empty() || names.split(',').any(|w| w.eq_ignore_ascii_case(&f.name)))
                    .cloned()
                    .collect();
                for field in wanted {
                    cursor.set_field(&field.name, &crate::data::empty_of(field.kind))?;
                }
            }
            Instr::Eject => {
                match &mut self.report {
                    Some(running) => running.page_break(),
                    // outside a report the page ends on the screen, which starts again empty
                    None => {
                        self.screen.surface().clear();
                        self.screen.cursor = (0, 0);
                    }
                }
            }
            Instr::WindowCommand { kind, given, text, flags } => {
                let text = match text.map(|c| &module.consts[c as usize]) {
                    Some(Constant::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                let mut slots: Vec<Option<Value>> = vec![None; 6];
                for bit in (0..6usize).rev() {
                    if given & (1 << bit) != 0 {
                        slots[bit] = Some(pop_value(fb)?);
                    }
                }
                if let Some(request) = self.window_command(*kind, *flags, &text, &slots) {
                    fb.pending = match request {
                        HostRequest::ReadGets { .. } => Some(Pending::ReadGets),
                        HostRequest::ChooseFrom { .. } => Some(Pending::MenuTo { name: menu_str(slots.first()) }),
                        _ => Some(Pending::Discard),
                    };
                    return Ok(Flow::Suspend(request));
                }
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::SetScreen { screen: self.screen.document() }));
            }
            Instr::AtCommand { kind, given, style, name, valid, when } => {
                let word = |c: Option<u32>| match c.map(|c| &module.consts[c as usize]) {
                    Some(Constant::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                let (style, name, valid, when) = (word(*style), word(*name), word(*valid), word(*when));
                let mut slots: Vec<Option<Value>> = vec![None; 9];
                for bit in (0..9usize).rev() {
                    if given & (1 << bit) != 0 {
                        slots[bit] = Some(pop_value(fb)?);
                    }
                }
                self.at_command(*kind, &style, &name, &valid, &when, &slots)?;
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::SetScreen { screen: self.screen.document() }));
            }
            Instr::MenuCommand { kind, given, text, flags } => {
                let text = match text.map(|c| &module.consts[c as usize]) {
                    Some(Constant::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                // they went on in order, so they come off backwards
                let mut slots: Vec<Option<Value>> = vec![None; 6];
                for bit in (0..6usize).rev() {
                    if given & (1 << bit) != 0 {
                        slots[bit] = Some(pop_value(fb)?);
                    }
                }
                if let Some(request) = self.menu_command(*kind, *flags, &text, &slots) {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(request));
                }
            }
            Instr::ShowInfo { kind, skeleton } => {
                let skel = skeleton.then(|| pop_value(fb)?.as_str().map(|s| s.trim().to_string())).transpose()?;
                self.show_info(host, fb, *kind, skel)?;
            }
            Instr::ListBegin => self.list_heading_due = self.settings.headings,
            Instr::ListRecord { fields, numbers } => {
                if let Some(req) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, req));
                }
                let settings = self.settings.clone();
                let columns = self.list_columns(fields, &module.consts)?;
                if std::mem::take(&mut self.list_heading_due) {
                    let mut line = if *numbers { "Record#  ".to_string() } else { "  ".to_string() };
                    for column in &columns {
                        column.write(&mut line, &column.field.name);
                    }
                    host.output(line.trim_end(), true);
                }
                let cursor = self.data.cursor().ok_or_else(RtError::no_table_open)?;
                let mut line =
                    if *numbers { format!("{:>7}  ", cursor.recno()) } else { "  ".to_string() };
                for column in &columns {
                    // a memo column says which of the two it is rather than what it holds,
                    // because the text would not fit a column four characters wide
                    let shown = if column.field.kind == 'M' || column.field.kind == 'G' {
                        let empty = cursor.field(&column.field.name).is_none_or(|v| v.as_str().is_ok_and(|s| s.is_empty()));
                        let word = type_word(column.field.kind);
                        if empty { word.to_ascii_lowercase() } else { word.to_string() }
                    } else {
                        let value = cursor.field(&column.field.name).unwrap_or(Value::Null);
                        crate::value::display(&value, &settings).trim().to_string()
                    };
                    column.write(&mut line, &shown);
                }
                host.output(line.trim_end(), true);
            }
            Instr::Pack => {
                if let Some(req) = self.pack(fb)? {
                    return Ok(self.ask(fb, req));
                }
            }

            // ---- records from another file
            Instr::AppendFrom { except, count, cond, text } => {
                let condition = cond.map(|c| match &module.consts[c as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                });
                if let Some(req) = self.append_from(host, fb, *except, *count, condition, *text)? {
                    return Ok(self.ask(fb, req));
                }
            }

            // ---- a table made from a table
            Instr::CopyBegin { kind, except, count, descending, text } => {
                let (quote, separator) = if *text == 3 {
                    let separator = pop!().as_str()?.to_string();
                    let quote = pop!().as_str()?.to_string();
                    (quote, separator)
                } else {
                    (String::new(), String::new())
                };
                let names = pop_names(fb, *count)?;
                let path = pop!().as_str()?.trim().to_string();
                let cursor = self.data.cursor().ok_or_else(no_table)?;
                let mut fields = chosen_fields(cursor, &names, *except);
                let mut rows = Vec::new();
                // COPY STRUCTURE EXTENDED writes what the header says, one record per field,
                // so there are no records to walk: the rows are here already
                if *kind == 4 {
                    rows = structure_rows(&fields);
                    fields = structure_fields();
                }
                self.copy = Some(CopyOut {
                    // COPY TO ARRAY has no file to name
                    path: match *kind {
                        6 => String::new(),
                        // SDF and DELIMITED are .txt, comma-separated is .csv, DIF is .dif,
                        // and a SYLK file goes by the name it was given, with no extension
                        _ => match *text {
                            1 | 3 => self.settings.at(&defaulted(&path, "txt")),
                            2 => self.settings.at(&defaulted(&path, "csv")),
                            4 => self.settings.at(&defaulted(&path, "dif")),
                            5 => self.settings.at(&path),
                            _ => self.settings.table_at(&path),
                        },
                    },
                    kind: *kind,
                    fields,
                    descending: *descending,
                    text: *text,
                    quote,
                    separator,
                    rows,
                });
            }
            Instr::CopyRow(keys) => {
                let mut key = Vec::with_capacity(*keys as usize);
                for _ in 0..*keys {
                    key.push(pop!());
                }
                key.reverse();
                if let Some(req) = self.ensure_record(fb)? {
                    for value in key {
                        fb.stack.push(value);
                    }
                    return Ok(self.ask(fb, req));
                }
                let fields: Vec<crate::dbf::DbfField> =
                    self.copy.as_ref().map(|c| c.fields.clone()).unwrap_or_default();
                let cursor = self.data.cursor().ok_or_else(no_table)?;
                let row: Vec<Value> = fields.iter().map(|f| cursor.field(&f.name).unwrap_or(Value::Null)).collect();
                if let Some(copy) = self.copy.as_mut() {
                    copy.rows.push((key, row));
                }
            }
            Instr::CopyEnd => {
                // COPY TO ARRAY writes no file: the records become an array, and how many
                // there were goes with it so the caller can leave the variable alone when
                // there were none
                if self.copy.as_ref().is_some_and(|c| c.kind == 6) {
                    let copy = self.copy.take().expect("the copy in hand");
                    let rows = copy.rows.len();
                    let cols = copy.fields.len();
                    let items: Vec<Value> = copy.rows.into_iter().flat_map(|(_, row)| row).collect();
                    let array = FoxArray { rows, cols, items };
                    fb.stack.push(Value::Array(Rc::new(RefCell::new(array))));
                    fb.stack.push(Value::number(rows as f64));
                } else if let Some(req) = self.finish_copy()? {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(req));
                }
            }

            // ---- adding up the records
            Instr::AggBegin(funcs) => {
                self.aggregates = funcs.iter().map(|f| Agg::new(*f)).collect();
            }
            Instr::AggStep(index) => {
                let value = pop!();
                // NPV was given the rate before the flow, and discounts one by the other
                let rate = if self.aggregates.get(*index as usize).is_some_and(|a| a.func == 7) {
                    Some(pop!().as_number()?)
                } else {
                    None
                };
                let exact = self.settings.exact;
                let settings = self.settings.clone();
                if let Some(agg) = self.aggregates.get_mut(*index as usize) {
                    let _ = exact;
                    agg.add(&value, rate, &settings)?;
                }
            }
            Instr::AggEnd => {
                let results: Vec<Value> = self.aggregates.drain(..).map(|a| a.result()).collect();
                let mut array = FoxArray::new(results.len().max(1), 0);
                for (slot, value) in array.items.iter_mut().zip(results) {
                    *slot = value;
                }
                fb.stack.push(Value::Array(Rc::new(RefCell::new(array))));
            }

            // ---- a record away from the table
            Instr::Scatter { to, except, count, blank } => {
                // an object is filled in a property at a time, and the field names came off the
                // stack on the first pass, so a pass that is carrying that on takes nothing
                if *to == 2 && self.scatter_name.is_some() {
                    if let Some(req) = self.scatter_object(fb, Vec::new())? {
                        return Ok(self.ask(fb, req));
                    }
                    return Ok(Flow::Next);
                }
                let names = pop_names(fb, *count)?;
                if let Some(req) = self.ensure_record(fb)? {
                    // the record has to be in hand before its fields can be taken from it
                    for name in names {
                        fb.stack.push(Value::str(name));
                    }
                    return Ok(self.ask(fb, req));
                }
                let row = self.record_fields(&names, *except, *blank)?;
                match to {
                    // a variable per field, named after it, which is what m.custno then reads
                    1 => {
                        for (name, value) in row {
                            self.store_name(fb, &name, value)?;
                        }
                    }
                    2 => {
                        if let Some(req) = self.scatter_object(fb, row)? {
                            return Ok(self.ask(fb, req));
                        }
                    }
                    _ => {
                        // an element of an array is a variable, so a number in it takes a
                        // variable's width rather than keeping the field's
                        let items: Vec<Value> = row.into_iter().map(|(_, v)| value::held_in_variable(v)).collect();
                        let mut array = FoxArray::new(items.len().max(1), 0);
                        for (slot, value) in array.items.iter_mut().zip(items) {
                            *slot = value;
                        }
                        fb.stack.push(Value::Array(Rc::new(RefCell::new(array))));
                    }
                }
            }
            Instr::Gather { to, except, count } => {
                let names = pop_names(fb, *count)?;
                // `to` 3 is REPLACE FROM ARRAY, which names the row of the array as well
                let row = if *to == 3 { Some(pop!()) } else { None };
                let source = if *to == 1 { None } else { Some(pop!()) };
                if let Some(req) = self.ensure_record(fb)? {
                    if let Some(v) = source {
                        fb.stack.push(v);
                    }
                    if let Some(v) = row {
                        fb.stack.push(v);
                    }
                    for name in names {
                        fb.stack.push(Value::str(name));
                    }
                    return Ok(self.ask(fb, req));
                }
                let row = row.map(|v| v.as_number()).transpose()?.unwrap_or(1.0).max(1.0) as usize;
                self.gather_record(host, fb, *to, source, &names, *except, row)?;
            }

            // ---- indexes
            Instr::OpenIndex => {
                let area = self.last_opened;
                if let Some(answer) = fb.data_reply.take() {
                    self.accept_index(area, &answer);
                } else if let Some(req) = self.index_request(area) {
                    return Ok(self.ask(fb, req));
                }
            }
            Instr::SetOrder { descending } => {
                if let Some(answer) = fb.data_reply.take() {
                    self.accept_index(None, &answer);
                } else if let Some(req) = self.index_request(None) {
                    return Ok(self.ask(fb, req));
                }
                let which = pop!();
                self.choose_order(&which, *descending)?;
            }
            Instr::Seek => {
                if let Some(answer) = fb.data_reply.take() {
                    self.accept_index(None, &answer);
                } else if let Some(req) = self.index_request(None) {
                    return Ok(self.ask(fb, req));
                }
                let key = pop!();
                let found = self.seek_key(&key)?;
                if let Some(cursor) = self.data.cursor_mut() {
                    cursor.set_found(found);
                }
            }
            Instr::IndexBegin => {
                if let Some(answer) = fb.data_reply.take() {
                    self.accept_index(None, &answer);
                } else if let Some(req) = self.index_request(None) {
                    return Ok(self.ask(fb, req));
                }
                let flags = pop!().as_number()? as u32;
                let for_expr = crate::cdx::stored_expr(&pop!().as_str()?);
                let key_expr = crate::cdx::stored_expr(&pop!().as_str()?);
                let mut name = pop!().as_str()?.trim().to_ascii_uppercase();
                // `INDEX ON x TO file` writes a file of its own, and the tag in it is known
                // by the name of that file
                let to_file = if flags & 8 != 0 {
                    let path = with_extension(pop!().as_str()?.trim(), "idx");
                    name = stem_of(&path).to_ascii_uppercase();
                    Some(path)
                } else {
                    None
                };
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                // building an index closes the single-entry ones already open, which is what
                // ADDITIVE is there to stop
                if flags & 32 == 0 {
                    cursor.close_idx();
                }
                self.index_build = Some(IndexBuild {
                    area: self.data.current_area(),
                    name,
                    key_expr,
                    for_expr,
                    unique: flags & 1 != 0 || self.settings.unique,
                    candidate: flags & 2 != 0,
                    descending: flags & 4 != 0,
                    to_file,
                    compact: flags & 16 != 0,
                    keys: Vec::new(),
                });
            }
            Instr::OpenIdx { count } => {
                if let Some(answer) = fb.data_reply.take() {
                    self.accept_idx(&answer)?;
                } else {
                    let paths = named_files(&pop_n(fb, *count as usize)?);
                    self.begin_open_idx(paths, false, Value::str(""), None)?;
                }
                if let Some(req) = self.next_idx_request() {
                    return Ok(self.ask(fb, req));
                }
            }
            Instr::CopyIndexes { count, all } => {
                if let Some(answer) = fb.data_reply.take() {
                    self.copy_indexes_reply(&answer)?;
                } else {
                    let cdx = pop!().as_str()?.trim().to_string();
                    let files = named_files(&pop_n(fb, *count as usize)?);
                    self.begin_copy_indexes(files, *all, cdx)?;
                }
                if let Some(req) = self.copy_indexes_step()? {
                    return Ok(self.ask(fb, req));
                }
            }
            Instr::CopyTag => {
                if let Some(answer) = fb.data_reply.take() {
                    self.copy_tag_reply(&answer)?;
                } else {
                    let target = pop!().as_str()?.trim().to_string();
                    let of = pop!().as_str()?.trim().to_string();
                    let name = pop!().as_str()?.trim().to_string();
                    // OF naming the structural index beside the table is the same as not
                    // saying which file the tag is in, and the tags in hand are that file
                    let structural = self
                        .data
                        .cursor()
                        .map(|c| with_extension(&c.path, "cdx"))
                        .is_some_and(|beside| beside.eq_ignore_ascii_case(&with_extension(&of, "cdx")));
                    let of = if structural { String::new() } else { of };
                    self.copy_tag = Some(CopyTag { name, of, target, found: None, read: false });
                }
                if let Some(req) = self.copy_tag_step()? {
                    return Ok(self.ask(fb, req));
                }
            }
            Instr::IndexKey => {
                let key = pop!();
                let recno = self.data.cursor().map_or(0, Cursor::recno) as u32;
                if let Some(build) = self.index_build.as_mut() {
                    build.keys.push((key, recno));
                }
            }
            Instr::IndexEnd => {
                if let Some(req) = self.finish_index()? {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(req));
                }
            }
            Instr::DeleteTag { all } => {
                if let Some(answer) = fb.data_reply.take() {
                    self.accept_index(None, &answer);
                } else if let Some(req) = self.index_request(None) {
                    return Ok(self.ask(fb, req));
                }
                let name = pop!();
                let area = self.data.current_area();
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                let name = name.as_str().unwrap_or_default().trim().to_string();
                cursor.drop_tags(if *all || name.eq_ignore_ascii_case("ALL") { None } else { Some(&name) });
                if let Some(req) = self.write_index(area) {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(req));
                }
            }
            Instr::SetFilter(text) => {
                // a filter is kept the way an index key is, with the spaces between its pieces gone
                let filter = match &module.consts[*text as usize] {
                    Constant::Str(s) => crate::cdx::stored_expr(s),
                    _ => String::new(),
                };
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                cursor.set_filter(filter);
            }
            Instr::SetRelation { count, additive, off } => {
                let mut pairs: Vec<crate::data::Relation> = Vec::new();
                for _ in 0..*count {
                    let into = pop!().as_str()?.trim().to_string();
                    let expr = if *off { String::new() } else { crate::cdx::stored_expr(&pop!().as_str()?) };
                    pairs.push(crate::data::Relation { expr, into });
                }
                pairs.reverse();
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                if *off {
                    // SET RELATION OFF INTO x: that one goes and the rest stay
                    let gone: Vec<String> = pairs.iter().map(|r| r.into.clone()).collect();
                    let kept: Vec<crate::data::Relation> = cursor
                        .relations()
                        .iter()
                        .filter(|r| !gone.iter().any(|g| g.eq_ignore_ascii_case(&r.into)))
                        .cloned()
                        .collect();
                    cursor.relate(None, false);
                    for r in kept {
                        cursor.relate(Some(r), true);
                    }
                } else {
                    cursor.relate(None, *additive);
                    for r in pairs {
                        let target = AreaRef::Alias(r.into.clone());
                        if self.data.find(&target).is_none() {
                            return Err(RtError::alias_not_found(&r.into));
                        }
                        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                        cursor.relate(Some(r), true);
                    }
                    // the child follows the parent from the moment the relation is made
                    self.follow_relations(host, fb)?;
                }
            }
            Instr::Reindex => {
                if let Some(answer) = fb.data_reply.take() {
                    self.accept_index(None, &answer);
                } else if let Some(req) = self.index_request(None) {
                    return Ok(self.ask(fb, req));
                }
                let area = self.data.current_area();
                if let Some(cursor) = self.data.cursor_mut() {
                    cursor.put_tag_dirty();
                }
                if let Some(req) = self.write_index(area) {
                    fb.pending = Some(Pending::Discard);
                    return Ok(Flow::Suspend(req));
                }
            }
            Instr::SqlOpen { table, alias, named } => {
                // the name is worked out here when the query said FROM (cPath); a name that is
                // taken off the stack must not be taken twice when the open has to wait
                let path = if *named {
                    match self.sql_named.take() {
                        Some(kept) => kept,
                        None => pop_value(fb)?.as_str()?.trim().to_string(),
                    }
                } else {
                    match &module.consts[*table as usize] {
                        Constant::Str(text) => text.clone(),
                        _ => String::new(),
                    }
                };
                let alias = {
                    let written = module.names[*alias as usize].clone();
                    // FROM (cPath) with nothing after it is known by what the table is called
                    if written.is_empty() { stem_of(&path).to_ascii_uppercase() } else { written }
                };
                if let Some(req) = self.sql_open(fb, &path, &alias)? {
                    if *named {
                        self.sql_named = Some(path);
                    }
                    return Ok(self.ask(fb, req));
                }
                self.sql_named = None;
            }
            Instr::SqlTouch(which) => {
                let area = fb
                    .query
                    .as_ref()
                    .and_then(|run| run.sources.get(*which as usize))
                    .map(|source| source.area)
                    .ok_or_else(RtError::no_table_open)?;
                self.data.select(&AreaRef::Number(area))?;
                // a page that has to be fetched comes back to this same instruction, which
                // selects the same source again and so hands the page to the cursor that asked
                if let Some(req) = self.ensure_record(fb)? {
                    return Ok(self.ask(fb, req));
                }
            }
            Instr::SelectSource(which) | Instr::JoinMiss(which) => {
                let area = fb
                    .query
                    .as_ref()
                    .and_then(|run| run.sources.get(*which as usize))
                    .map(|source| source.area)
                    .ok_or_else(|| RtError::no_table_open())?;
                self.data.select(&AreaRef::Number(area))?;
                if matches!(instr, Instr::JoinMiss(_)) {
                    if let Some(req) = self.move_pointer(host, fb, Move::Record(0))? {
                        return Ok(self.ask(fb, req));
                    }
                    self.data.cursor_mut().ok_or_else(no_table)?.set_outer_miss();
                }
            }
            Instr::SqlBegin(plan) => {
                let plan = module.queries.get(*plan as usize).cloned().ok_or_else(|| {
                    RtError::new(RtError::SYNTAX_ERROR, "the query plan is missing from the module")
                })?;
                let top = if plan.has_top { Some(pop!().as_number()?) } else { None };
                // `INTO CURSOR (cName)` put the name under the count
                let into = if plan.into_named { Some(pop!().as_str()?.trim().to_ascii_uppercase()) } else { None };
                self.sql_begin(fb, plan, top, into)?;
            }
            Instr::SqlRow(argc) => {
                let values = pop_n(fb, *argc as usize)?;
                self.sql_row(fb, values)?;
            }
            Instr::SqlEnd => return self.sql_end(fb),
            Instr::SqlHavingNext => {
                let settings = self.settings.clone();
                let run = fb.query.as_mut().ok_or_else(|| {
                    RtError::new(RtError::SYNTAX_ERROR, "a HAVING clause ran outside a query")
                })?;
                run.fold_for_having(&settings)?;
                let state = run.having.get_or_insert_with(Default::default);
                let more = state.at < run.rows.len();
                state.testing = more;
                if !more {
                    // what the predicate kept is the result; the rows it was asked about are done
                    let kept = std::mem::take(&mut state.kept);
                    run.rows = kept;
                }
                fb.stack.push(Value::Logical(more));
            }
            Instr::SqlHavingValue(which) => {
                let run = fb.query.as_ref().ok_or_else(|| {
                    RtError::new(RtError::SYNTAX_ERROR, "a HAVING clause ran outside a query")
                })?;
                let named = run.plan.having.get(*which as usize).cloned().ok_or_else(|| {
                    RtError::new(RtError::SYNTAX_ERROR, "the HAVING clause names a column the plan has not got")
                })?;
                // A column of a record the clause may not name - one that is neither grouped nor
                // inside an aggregate, or one that shadows a name the select list gave with AS -
                // is 1803, unless SET ENGINEBEHAVIOR 70 is in force. Under 70 it is the value the
                // group's last record holds, which is what was gathered for it.
                let which_column = match &named {
                    HavingRef::Ungrouped { name, index, alias } => {
                        let field = self.query_field(fb, name).is_some();
                        if field && self.settings.engine_behavior() > 70 {
                            return Err(RtError::about(RtError::HAVING_INVALID, name));
                        }
                        match alias {
                            Some(column) if !field => *column,
                            _ => *index,
                        }
                    }
                    HavingRef::Column(index) => *index,
                    HavingRef::Key(_) => 0,
                };
                let run = fb.query.as_ref().expect("query");
                let slot = match &named {
                    HavingRef::Key(j) => run.key_slot(*j),
                    _ => run.column_slot(which_column),
                };
                let at = run.having.as_ref().map(|h| h.at).unwrap_or(0);
                let value = run.rows.get(at).and_then(|r| r.get(slot)).cloned().unwrap_or(Value::Null);
                fb.stack.push(value);
            }
            Instr::SqlSourceField(name) => {
                // gathered for a HAVING clause: the field of that name in whichever source has
                // one, and .NULL. when the name is only what the select list called a column
                let name = module.names[*name as usize].clone();
                let Some(alias) = self.query_field(fb, &name) else {
                    fb.stack.push(Value::Null);
                    return Ok(Flow::Next);
                };
                match self.read_field(fb, Some(&alias), &name)? {
                    FieldRead::Suspend(req) => return Ok(self.ask(fb, req)),
                    FieldRead::Value(v) => fb.stack.push(v),
                }
            }
            Instr::SqlRequireGroupBy => {
                if self.settings.engine_behavior() > 70 {
                    return Err(RtError::about(RtError::GROUP_BY_MISSING, ""));
                }
            }
            Instr::SqlHavingKeep => {
                // a HAVING clause says which groups to keep, so what it comes to has to be an
                // answer to that: measured, `HAVING COUNT(*)` is 1803 rather than a count read
                // as a truth. .NULL. is not an error - it is simply not a group that is kept.
                let value = pop!();
                let keep = match value.deref() {
                    Value::Logical(b) => b,
                    Value::Null => false,
                    _ => return Err(RtError::about(RtError::HAVING_INVALID, "")),
                };
                let run = fb.query.as_mut().ok_or_else(|| {
                    RtError::new(RtError::SYNTAX_ERROR, "a HAVING clause ran outside a query")
                })?;
                let Some(at) = run.having.as_ref().map(|h| h.at) else { return Ok(Flow::Next) };
                let row = if keep { run.rows.get(at).cloned() } else { None };
                let state = run.having.as_mut().expect("having");
                state.testing = false;
                if let Some(row) = row {
                    state.kept.push(row);
                }
                state.at += 1;
            }
            Instr::InCursor(alias) => {
                let value = pop!();
                let alias = module.names[*alias as usize].clone();
                let found = self
                    .data
                    .find(&AreaRef::Alias(alias))
                    .is_some_and(|c| c.all_rows().iter().any(|r| same_value(r.values.first(), &value, &self.settings)));
                fb.stack.push(Value::Logical(found));
            }
            Instr::SetFound => {
                let found = pop!().truthy()?;
                if let Some(cursor) = self.data.cursor_mut() {
                    cursor.set_found(found);
                }
            }
            Instr::LoadField { area, field } => {
                let field = module.members[*field as usize].clone();
                let base = area.map(|a| module.names[a as usize].clone());
                // `a.b` where `a` is a bare name is settled here, because only the running
                // program knows which it is: a memory variable holding an object, or the alias
                // of an open table. A variable wins, as it does in Visual FoxPro.
                if let Some(base) = base {
                    // `m.` is the memory-variable prefix and nothing else, even when a variable
                    // called `m` is itself an object: `CATCH TO m` then `m.Message` reads a
                    // variable called Message in Visual FoxPro, not the exception's property.
                    if base == "M" {
                        let upper = field.to_ascii_uppercase();
                        let v = self.load_name(fb, &upper).ok_or_else(|| RtError::variable_not_found(&upper))?;
                        fb.stack.push(v);
                        return Ok(Flow::Next);
                    }
                    if let Some(obj) = self.load_name(fb, &base) {
                        if let Some(v) = json_member(&obj, &field) {
                        fb.stack.push(v?);
                        return Ok(Flow::Next);
                        }

                        let h = self.check_object(host, &obj, &field)?;
                        if let Some(v) = self.native_member(h, &field) {
                            fb.stack.push(v?);
                            return Ok(Flow::Next);
                        }
                        match host.get_member(h, &field)? {
                            Member::Child(c) => fb.stack.push(Value::Object(c)),
                            Member::Property => {
                                let v = host.get_prop(h, &field)?;
                                fb.stack.push(v.deref());
                            }
                            Member::Method | Member::None => return Err(RtError::unknown_member(&field)),
                        }
                        return Ok(Flow::Next);
                    }
                    if base == "M" {
                        // the `m.` prefix: a memory variable, never a field of the same name
                        let upper = field.to_ascii_uppercase();
                        let v = self.load_name(fb, &upper).ok_or_else(|| RtError::variable_not_found(&upper))?;
                        fb.stack.push(v);
                        return Ok(Flow::Next);
                    }
                    if self.data.find(&AreaRef::Alias(base.clone())).is_none() {
                        return Err(RtError::variable_not_found(&base));
                    }
                    match self.read_field(fb, Some(&base), &field)? {
                        FieldRead::Value(v) => fb.stack.push(v),
                        FieldRead::Suspend(req) => return Ok(self.ask(fb, req)),
                    }
                } else {
                    match self.read_field(fb, None, &field)? {
                        FieldRead::Value(v) => fb.stack.push(v),
                        FieldRead::Suspend(req) => return Ok(self.ask(fb, req)),
                    }
                }
            }
            Instr::Erase => {
                let path = pop!().as_str()?.trim().to_string();
                fb.pending = Some(Pending::Discard);
                return Ok(Flow::Suspend(HostRequest::FileDelete { path }));
            }
            Instr::Retry => self.retry(fb)?,
            Instr::Throw => {
                let v = pop!();
                // a bare THROW inside a CATCH raises again the error being handled; anywhere
                // else it is a user error carrying nothing, which VFP reads back as ""
                if v.is_null() {
                    if let Some(e) = fb.last_error.clone() {
                        return Err(e);
                    }
                    return Err(RtError::user_thrown(JsonValue::Str(String::new())));
                }
                return Err(RtError::user_thrown(JsonValue::from_value(&v)));
            }
            Instr::CatchObject => {
                let e = fb.last_error.clone().unwrap_or_else(|| RtError::new(0, ""));
                fb.pending = Some(Pending::Push);
                return Ok(Flow::Suspend(HostRequest::CreateException {
                    code: e.code,
                    message: e.message,
                    program: e.program,
                    line: e.line,
                    user_value: e.user_value,
                }));
            }
            Instr::EndFinally => {
                if let Some(Some(e)) = fb.pending_rethrow.pop() {
                    return Err(e);
                }
            }
            Instr::OnError(c) => {
                let text = c.map(|i| match &module.consts[i as usize] {
                    Constant::Str(s) => s.clone(),
                    _ => String::new(),
                });
                self.on_error = match text {
                    Some(t) if t.contains('&') => Some(self.expand_macros(host, fb, &t)?).filter(|t| !t.trim().is_empty()),
                    other => other,
                };
            }
            Instr::RaiseError => {
                let message = pop!();
                let what = pop!();
                return Err(match what.deref() {
                    // `ERROR n, cText` writes cText into the gap the number's own sentence has,
                    // so `ERROR 12, "x"` reads "Variable 'x' is not found." rather than "x"
                    Value::Number(n, ..) => {
                        let code = n as u32;
                        match message.deref() {
                            Value::Str(s) => RtError::about(code, &s),
                            _ => RtError::new(
                                code,
                                RtError::standard_message(code).unwrap_or_else(|| format!("Error {code}")),
                            ),
                        }
                    }
                    other => RtError::new(RtError::USER_DEFINED, value::display(&other, &self.settings)),
                });
            }
            Instr::Nop => {}
        }
        Ok(Flow::Next)
    }

    /// `DO name [IN prog]`: a procedure of the running module, an already loaded program, or
    /// one the host has to find and compile first.
    /// Loads the files of a `SET PROCEDURE TO` one at a time, asking the host for each one it
    /// does not already have.
    ///
    /// Measured: without ADDITIVE the list is emptied before anything is loaded, and each file
    /// goes on as it is found - so `SET PROCEDURE TO a, missing, b` raises error 1 and leaves
    /// `a` on the list and nothing else. With ADDITIVE a missing file leaves the list as it was.
    fn continue_procedure_load(&mut self, host: &mut dyn Host, fb: &mut Fiber, reply: Option<Value>) -> Result<Flow, RtError> {
        let mut state = fb.procedure_load.take().unwrap_or_default();
        match reply {
            None => {
                if !state.additive {
                    self.procedure_files.clear();
                    self.remember_procedures();
                }
            }
            Some(v) => match v.deref() {
                Value::Number(n, ..) if n >= 0.0 && (n as usize) < self.modules.len() => {
                    self.procedure_only.insert(n as u32);
                    self.add_procedure_file(n as u32);
                    state.next += 1;
                }
                _ => return Err(program_missing(&state.names[state.next])),
            },
        }
        while let Some(name) = state.names.get(state.next) {
            let stem = program_stem(name);
            if let Some(id) = self.find_program(&stem) {
                self.add_procedure_file(id);
            } else if let Some(id) = host.resolve_program(&stem) {
                self.procedure_only.insert(id);
                self.add_procedure_file(id);
            } else {
                let request = HostRequest::LoadProgram { name: stem };
                fb.procedure_load = Some(state);
                return Ok(self.ask(fb, request));
            }
            state.next += 1;
        }
        Ok(Flow::Next)
    }

    /// Puts a program on the end of the procedure list. Measured: one that is already on it
    /// stays where it is rather than moving to the end.
    fn add_procedure_file(&mut self, id: u32) {
        if !self.procedure_files.contains(&id) {
            self.procedure_files.push(id);
            self.remember_procedures();
        }
    }

    /// `RELEASE PROCEDURE a, b`: those files come off the list. One that is not on it is
    /// nothing to release.
    fn release_procedures(&mut self, names: &[String]) {
        let stems: Vec<String> = names.iter().map(|n| program_stem(n)).collect();
        let modules = &self.modules;
        self.procedure_files.retain(|&id| !stems.iter().any(|s| program_stem(&modules[id as usize].name) == *s));
        self.remember_procedures();
    }

    /// What `SET("PROCEDURE")` answers: every file on the list, quoted, as the compiled program
    /// it is loaded from - the full path of the `.FXP` - separated by a comma and a space.
    fn remember_procedures(&mut self) {
        let text = self
            .procedure_files
            .iter()
            .map(|&id| {
                let stem = program_stem(&self.modules[id as usize].name);
                format!("\"{}\"", self.settings.at(&format!("{stem}.FXP")).to_ascii_uppercase())
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.settings.remembered.insert("PROCEDURE".to_string(), text);
    }

    fn run_do(
        &mut self,
        fb: &mut Fiber,
        host: &mut dyn Host,
        upper: String,
        args: Vec<Value>,
        in_prog: Option<String>,
    ) -> Result<Flow, RtError> {
        match in_prog {
            None => {
                let current = fb.frames.last().expect("frame").module;
                if let Some((m, f)) = self.find_function(current, &upper) {
                    self.push_call(fb, m, f, None, args, FrameKind::Call { discard: true }, true)?;
                } else if let Some(id) = self.find_program(&upper).or_else(|| host.resolve_program(&upper)) {
                    // a program that has run keeps its routines findable, procedure file or not
                    self.procedure_only.remove(&id);
                    self.push_call(fb, id, 0, None, args, FrameKind::Call { discard: true }, true)?;
                } else {
                    fb.pending = Some(Pending::LoadedProgram { func: None, args, discard: true, program: upper.clone() });
                    return Ok(Flow::Suspend(HostRequest::LoadProgram { name: upper }));
                }
            }
            Some(prog) => match self.find_program(&prog).or_else(|| host.resolve_program(&prog)) {
                Some(id) => {
                    let f = self.modules[id as usize].find_func(&upper).ok_or_else(|| RtError::procedure_not_found(&upper))?;
                    self.push_call(fb, id, f, None, args, FrameKind::Call { discard: true }, true)?;
                }
                None => {
                    fb.pending = Some(Pending::LoadedProgram { func: Some(upper), args, discard: true, program: prog.clone() });
                    return Ok(Flow::Suspend(HostRequest::LoadProgram { name: prog }));
                }
            },
        }
        Ok(Flow::Next)
    }

    // ---- data engine ----------------------------------------------------------------------

    /// Rewinds to the instruction that is asking and suspends. It runs again when the answer
    /// arrives, finds it in `data_reply`, and carries on from there.
    fn ask(&mut self, fb: &mut Fiber, request: HostRequest) -> Flow {
        if let Some(frame) = fb.frames.last_mut() {
            frame.pc -= 1;
        }
        fb.pending = Some(Pending::DataReply);
        Flow::Suspend(request)
    }

    /// Makes sure the record the pointer is on is in hand, so it can be changed.
    ///
    /// Moving the pointer reads nothing, so the first thing to touch a record is what pays for
    /// the page it lives on - a read or, here, a write.
    fn ensure_record(&mut self, fb: &mut Fiber) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        if let Some(page) = reply {
            cursor.accept_page(bytes_of(&page));
            let recno = cursor.recno();
            cursor.seek(recno);
        }
        let recno = cursor.recno();
        if let Some(handle) = cursor.handle()
            && recno >= 1
            && recno <= cursor.count()
            && !cursor.page_holds(recno)
        {
            let first = cursor.page_start(recno);
            cursor.expect_page(first);
            return Ok(Some(HostRequest::DataRead { handle, first: first as f64, count: PAGE_RECORDS }));
        }
        Ok(None)
    }

    /// `ALTER TABLE`: the table is read, its records are given the columns the command asks
    /// for, and it is written back.
    ///
    /// A table this changes has to be read whole, so it is opened if it is not open already and
    /// let go of afterwards; a work area holding it is given the new columns, because the file
    /// under it is not the one it opened any more.
    fn alter_table(&mut self, fb: &mut Fiber, ops: &[Alteration]) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        match &mut self.alter {
            Some(state) if state.stage == 3 => {
                self.alter = None;
                return Ok(None);
            }
            None => {
                let path = fb.stack.pop().ok_or_else(|| RtError::new(0, "Stack underflow"))?;
                let path = self.settings.table_at(path.as_str()?.trim());
                self.alter = Some(Alter { stage: 0, handle: 0, path: path.clone(), borrowed: false });
                // a work area already holding the table lends its handle rather than opening it
                let open = self.data.find(&AreaRef::Alias(default_alias(&path))).and_then(Cursor::handle);
                if let Some(handle) = open {
                    let state = self.alter.as_mut().expect("alter");
                    state.handle = handle;
                    state.borrowed = true;
                    state.stage = 1;
                    let count = self.data.find(&AreaRef::Alias(default_alias(&path))).map_or(0, Cursor::count) as u32;
                    let header = self
                        .data
                        .find(&AreaRef::Alias(default_alias(&path)))
                        .map(|c| c.header.clone())
                        .ok_or_else(no_table)?;
                    self.source_header = Some(header);
                    return Ok(Some(HostRequest::DataRead { handle, first: 1.0, count: count.max(1) }));
                }
                return Ok(Some(HostRequest::DataOpen { path, exclusive: true, search: Vec::new() }));
            }
            Some(state) if state.stage == 0 => {
                let items = match reply.map(|v| v.deref()) {
                    Some(Value::Array(a)) => a.borrow().items.clone(),
                    other => other.into_iter().collect(),
                };
                let handle = items.first().and_then(|v| v.as_number().ok()).unwrap_or(-1.0);
                if handle < 0.0 {
                    let path = state.path.clone();
                    self.alter = None;
                    return Err(RtError::new(RtError::FILE_NOT_FOUND, format!("File '{path}' does not exist")));
                }
                let header = crate::dbf::read_header(&bytes_of(items.get(1).unwrap_or(&Value::Null)))
                    .map_err(|e| RtError::new(RtError::FILE_NOT_FOUND, e.message))?;
                state.handle = handle as u32;
                state.stage = 1;
                let count = header.record_count as u32;
                self.source_header = Some(header);
                return Ok(Some(HostRequest::DataRead { handle: handle as u32, first: 1.0, count: count.max(1) }));
            }
            // the records have arrived: the new table is written from them
            Some(state) if state.stage == 1 => {
                let bytes = bytes_of(&reply.unwrap_or(Value::Null));
                let header = self.source_header.take().ok_or_else(|| RtError::new(0, "no header for the table"))?;
                let (fields, rows) = alter_records(&header, &bytes, ops)?;
                let table = build_table(&fields, &rows, header.codepage)?;
                let memo = fields.iter().any(|f| matches!(f.kind, 'M' | 'G' | 'P'));
                let path = state.path.clone();
                state.stage = 2;
                // a work area holding the table now holds one of a different shape
                let alias = default_alias(&path);
                let new_header = crate::dbf::read_header(&table).map_err(|e| RtError::new(0, e.message))?;
                if let Some(cursor) = self.data.find_mut(&AreaRef::Alias(alias)) {
                    cursor.restructure(new_header);
                }
                return Ok(Some(HostRequest::DataCreate { path, header: table, memo }));
            }
            // the file is written: the handle that was opened for it is let go of
            Some(state) => {
                state.stage = 3;
                if state.borrowed {
                    self.alter = None;
                    return Ok(None);
                }
                return Ok(Some(HostRequest::DataClose { handles: vec![state.handle] }));
            }
        }
    }

    /// One record a work area is holding, on its way to the host. Answers with nothing when
    /// every area has been emptied, which is what ends a transaction.
    fn write_one_held(&mut self) -> Option<HostRequest> {
        for area in 1..=self.data.highest_free() {
            let Some(cursor) = self.data.find_mut(&AreaRef::Number(area)) else { continue };
            let Some(handle) = cursor.handle() else {
                cursor.take_held(None);
                continue;
            };
            let Some((recno, record)) = cursor.take_one_held() else { continue };
            let count = record.appended.then(|| cursor.count() as f64);
            return Some(HostRequest::DataWrite { handle, recno: recno as f64, bytes: record.bytes, count });
        }
        None
    }

    /// The columns a `LIST` or `DISPLAY` of records writes: the fields the command named, or
    /// every field of the table when it named none.
    fn list_columns(&self, fields: &u32, consts: &[Constant]) -> Result<Vec<ListColumn>, RtError> {
        let names = match &consts[*fields as usize] {
            Constant::Str(s) => s.clone(),
            _ => String::new(),
        };
        let settings = &self.settings;
        let cursor = self.data.cursor().ok_or_else(RtError::no_table_open)?;
        let wanted: Vec<&crate::dbf::DbfField> = if names.is_empty() {
            cursor.header.fields.iter().collect()
        } else {
            names
                .split(',')
                .filter_map(|n| cursor.header.fields.iter().find(|f| f.name.eq_ignore_ascii_case(n)))
                .collect()
        };
        Ok(wanted.into_iter().map(|f| list_column(f, settings)).collect())
    }

    /// `LIST` and `DISPLAY` of what the runtime holds: the table's columns, the variables, the
    /// settings, the files, what the database holds, the libraries, the programs and the objects.
    fn show_info(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        kind: u8,
        skeleton: Option<String>,
    ) -> Result<(), RtError> {
        let settings = self.settings.clone();
        match kind {
            // the structure of the table in the selected work area
            0 => {
                let cursor = self.data.cursor().ok_or_else(no_table)?;
                host.output(&format!("Structure for table: {}", cursor.path), true);
                host.output(&format!("Number of data records: {}", cursor.count()), true);
                host.output("Field  Field Name      Type       Width    Dec", true);
                for (i, field) in cursor.header.fields.iter().enumerate() {
                    host.output(
                        &format!(
                            "{:>5}  {:<15} {:<10} {:>5} {:>6}",
                            i + 1,
                            field.name,
                            type_word(field.kind),
                            field.length,
                            field.decimals
                        ),
                        true,
                    );
                }
                host.output(&format!("** Total **  {:>27}", cursor.header.record_len), true);
            }
            // the memory variables, public and private alike
            1 => {
                let mut names: Vec<(String, bool, Value)> =
                    self.globals.iter().map(|(n, v)| (n.clone(), true, v.deref())).collect();
                for frame in &fb.frames {
                    for (name, value) in &frame.privates {
                        names.push((name.clone(), false, value.deref()));
                    }
                }
                // Visual FoxPro lists them in the order they were made. The variables here are
                // kept in a map, which has no order, so they go out by name: that is a
                // difference a program could see, and putting it right means remembering when
                // each one was made rather than only what it holds.
                names.sort_by(|a, b| a.0.cmp(&b.0));
                names.dedup_by(|a, b| a.0 == b.0);
                if let Some(skel) = &skeleton {
                    let skel = skel.to_ascii_uppercase();
                    names.retain(|(name, _, _)| {
                        crate::builtins::string::matches_pattern(skel.as_bytes(), name.as_bytes())
                    });
                }
                let program = fb
                    .frames
                    .last()
                    .map(|f| self.proto(f.module, f.func).display_name.clone())
                    .unwrap_or_default();
                for (name, public, value) in &names {
                    for line in memory_lines(name, *public, value, &program, &settings) {
                        host.output(&line, true);
                    }
                }
                // a listing narrowed to a skeleton ends there; the whole of memory is followed
                // by how much of it there is, which is where the count belongs
                if skeleton.is_none() {
                    host.output(&format!("{} variables defined", names.len()), true);
                }
            }
            // the settings and the work areas
            2 => {
                host.output(&format!("Currently Selected Database: {}", self.data.cursor().map_or("", |c| c.alias.as_str())), true);
                for area in 1..=self.data.highest_free() {
                    if let Some(cursor) = self.data.find(&AreaRef::Number(area)) {
                        host.output(
                            &format!(
                                "Select area: {:>2}, Table in Use: {:<20} Alias: {:<10} Records: {}",
                                area,
                                cursor.path,
                                cursor.alias,
                                cursor.count()
                            ),
                            true,
                        );
                        if let Some(tag) = cursor.order() {
                            host.output(&format!("    Master index file: {} Tag: {}", tag.key_expr, tag.name), true);
                        }
                    }
                }
                for (name, value) in [
                    ("EXACT", settings.exact),
                    ("DELETED", settings.deleted),
                    ("NEAR", settings.near),
                    ("UNIQUE", settings.unique),
                    ("TALK", settings.talk),
                    ("SAFETY", settings.safety),
                    ("CENTURY", settings.century),
                ] {
                    host.output(&format!("{name} is {}", if value { "ON" } else { "OFF" }), true);
                }
                host.output(&format!("DECIMALS is {}", settings.decimals), true);
                host.output(&format!("MEMOWIDTH is {}", settings.memowidth), true);
            }
            // the files of the default directory: what the host lists for the mask
            3 => {
                host.output("Use DIR or ADIR() for the files of a folder.", true);
            }
            // what the database that is open holds
            4 | 5 => {
                let wanted = if kind == 4 { "Table" } else { "View" };
                if let Some(db) = self.current_database() {
                    for object in db.objects.iter().filter(|o| o.kind == wanted) {
                        host.output(&object.name, true);
                    }
                }
            }
            // the library functions a program declared
            6 => {
                let mut names: Vec<(&String, &(String, crate::bytecode::DllProto))> = self.dlls.iter().collect();
                names.sort_by(|a, b| a.0.cmp(b.0));
                for (called, (library, proto)) in names {
                    host.output(&format!("{called:<20} {library:<24} {}", proto.function), true);
                }
            }
            // the programs that are loaded
            7 => {
                let mut names: Vec<&String> = self.programs.keys().collect();
                names.sort();
                for name in names {
                    host.output(name, true);
                }
            }
            // the objects a program made, which the host holds
            8 => {
                for (name, value) in self.globals.iter() {
                    if let Value::Object(handle) = value.deref()
                        && let Some(class) = host.object_class(handle)
                    {
                        host.output(&format!("{name:<16} {class}"), true);
                    }
                }
            }
            // connections to a server, of which this runtime has none
            _ => host.output("No connections are open.", true),
        }
        Ok(())
    }

    /// `BROWSE`: the records of the work area, worked out and handed to the host to show.
    ///
    /// A cursor that lives in memory is read from what it holds; one that lives in a file is
    /// read whole first, because a window shows the table rather than one page of it.
    fn browse(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        wanted: &str,
        cond: &str,
        title: &str,
        flags: u8,
    ) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        let cursor = self.data.cursor().ok_or_else(no_table)?;
        let alias = cursor.alias.clone();
        let header = cursor.header.clone();
        // the records come through the open table rather than from the file: what the engine
        // holds is what the program has just written, which the file may not have yet
        let rows: Vec<crate::dbf::DbfRecord> = match cursor.handle() {
            None => cursor.all_rows().to_vec(),
            Some(handle) => match reply {
                None => {
                    let count = cursor.count().min(u64::from(u32::MAX)) as u32;
                    return Ok(Some(HostRequest::DataRead { handle, first: 1.0, count: count.max(1) }));
                }
                Some(reply) => {
                    let bytes = bytes_of(&reply);
                    let len = header.record_len.max(1);
                    bytes
                        .chunks(len)
                        .filter(|chunk| chunk.len() >= len)
                        .map(|chunk| crate::dbf::decode_record(&header, chunk, crate::dbf::Padding::Keep, |_| None))
                        .collect()
                }
            },
        };

        // FIELDS names the columns; without it every field of the table is shown
        let columns: Vec<crate::host::BrowseColumn> = header
            .fields
            .iter()
            .filter(|f| wanted.is_empty() || wanted.split(',').any(|w| w.eq_ignore_ascii_case(&f.name)))
            .map(|f| crate::host::BrowseColumn { name: f.name.clone(), kind: f.kind.to_string(), width: u32::from(f.length) })
            .collect();
        let places: Vec<usize> = header
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| columns.iter().any(|c| c.name == f.name))
            .map(|(i, _)| i)
            .collect();

        let deleted_kept = self.settings.deleted;
        let mut shown = Vec::new();
        for (i, record) in rows.iter().enumerate() {
            if record.deleted && !deleted_kept {
                continue;
            }
            let recno = i as u64 + 1;
            // FOR is worked out against the record, so the pointer goes to it first
            if !cond.is_empty() {
                if let Some(cursor) = self.data.cursor_mut() {
                    cursor.seek(recno);
                }
                let passes = self.eval_in_frame(host, fb, cond).map(|v| v.truthy().unwrap_or(false));
                if !passes.unwrap_or(true) {
                    continue;
                }
            }
            shown.push(crate::host::BrowseRow {
                recno,
                deleted: record.deleted,
                values: places.iter().map(|i| JsonValue::from_value(&value_of_dbf(record.values.get(*i)))).collect(),
            });
        }
        let count = rows.len() as u64;
        Ok(Some(HostRequest::Browse {
            browse: crate::host::BrowseTable {
                alias: alias.clone(),
                title: if title.is_empty() { alias } else { title.to_string() },
                columns,
                rows: shown,
                count,
                editable: flags & 2 == 0,
            },
            nowait: flags & 1 != 0,
        }))
    }

    // ----- reports -----------------------------------------------------------------------------

    /// `REPORT FORM`: reads the file, then writes the bands that print once at the top.
    ///
    /// The file comes in two pieces - the table and the memo beside it - so this runs three
    /// times: once to ask for the first, once to ask for the second, and once to start.
    fn report_begin(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        flags: u8,
        label: bool,
        to_file: bool,
    ) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        match &mut self.report {
            // nothing started yet: take what the statement named and ask for the report file
            None => {
                let file = if to_file { pop_value(fb)?.as_str()?.to_string() } else { String::new() };
                let path = pop_value(fb)?.as_str()?.to_string();
                let path = with_extension(&path, if label { "lbx" } else { "frx" });
                self.report = Some(RunningReport::new(path.clone(), file, flags));
                Ok(Some(HostRequest::FileReadBytes { path }))
            }
            Some(running) if running.report.is_none() => {
                let bytes = reply.map(|v| bytes_of(&v)).unwrap_or_default();
                if running.frx.is_none() {
                    if bytes.is_empty() {
                        let path = running.path.clone();
                        self.report = None;
                        return Err(RtError::new(1, format!("File '{path}' does not exist")));
                    }
                    running.frx = Some(bytes);
                    let memo = with_extension(&running.path, if label { "lbt" } else { "frt" });
                    return Ok(Some(HostRequest::FileReadBytes { path: memo }));
                }
                let frx = running.frx.take().unwrap_or_default();
                let memo = (!bytes.is_empty()).then_some(bytes);
                let read = crate::report::read_report(&frx, memo.as_deref())
                    .map_err(|e| RtError::new(RtError::SYNTAX_ERROR, e))?;
                running.report = Some(read);
                let flags = running.flags;
                self.globals.insert("_PAGENO".into(), Value::number(1.0));
                // the bands that print once at the top, unless PLAIN said to leave them out
                if flags & 16 == 0 {
                    self.report_band(host, fb, crate::report::Band::Title)?;
                }
                self.report_band(host, fb, crate::report::Band::PageHeader)?;
                self.report_band(host, fb, crate::report::Band::ColumnHeader)?;
                Ok(None)
            }
            Some(_) => Ok(None),
        }
    }

    /// One record of the report: the detail band, worked out against the record the pointer is
    /// on. SUMMARY was asked for when only the totals are wanted, so the detail is left out.
    fn report_row(&mut self, host: &mut dyn Host, fb: &mut Fiber) -> Result<(), RtError> {
        let summary_only = self.report.as_ref().is_some_and(|r| r.flags & 8 != 0);
        if summary_only {
            return Ok(());
        }
        for band in [crate::report::Band::DetailHeader, crate::report::Band::Detail, crate::report::Band::DetailFooter] {
            self.report_band(host, fb, band)?;
        }
        Ok(())
    }

    /// The bands that print once at the end, and then the report goes where it was told.
    fn report_end(&mut self, host: &mut dyn Host, fb: &mut Fiber) -> Result<Option<HostRequest>, RtError> {
        if self.report.is_none() {
            return Ok(None);
        }
        for band in [crate::report::Band::Summary, crate::report::Band::PageFooter] {
            self.report_band(host, fb, band)?;
        }
        let Some(running) = self.report.take() else { return Ok(None) };
        let text = running.lines.join("\n");
        // NOCONSOLE keeps it off the screen; TO FILE puts it in a file either way
        if running.flags & 4 == 0 {
            for line in &running.lines {
                host.output(line, true);
            }
        }
        if !running.to_file.is_empty() {
            let path = with_extension(&running.to_file, "txt");
            return Ok(Some(HostRequest::FileWrite { path, text, append: false }));
        }
        Ok(None)
    }

    /// One band: every item of it that its Print When lets through, worked out and laid out.
    fn report_band(&mut self, host: &mut dyn Host, fb: &mut Fiber, band: crate::report::Band) -> Result<(), RtError> {
        let Some(running) = &self.report else { return Ok(()) };
        let Some(report) = &running.report else { return Ok(()) };
        if !report.has(band) {
            return Ok(());
        }
        let items: Vec<crate::report::Item> = report.band_items(band).into_iter().cloned().collect();
        let settings = self.settings.clone();
        let mut shown = crate::report::Report {
            bands: vec![(band, report.height_of(band))],
            items: Vec::new(),
            order: String::new(),
            columns: 0,
        };
        let mut values = Vec::new();
        for item in items {
            if !item.print_when.is_empty() {
                let ok = self.eval_in_frame(host, fb, &item.print_when).map(|v| v.truthy().unwrap_or(false));
                if !ok.unwrap_or(true) {
                    continue;
                }
            }
            let text = if item.rule {
                String::new()
            } else {
                let value = self.eval_in_frame(host, fb, &item.expr).unwrap_or(Value::str(""));
                crate::builtins::string::transform(&value, Some(&item.picture), &settings).trim_end().to_string()
            };
            shown.items.push(item);
            values.push(text);
        }
        let lines = crate::report::lay_out(&shown, band, &values);
        if let Some(running) = &mut self.report {
            running.add(lines);
        }
        Ok(())
    }

    // ----- the screen --------------------------------------------------------------------------

    /// One window command. Everything it names is already off the stack.
    fn window_command(&mut self, kind: u8, flags: u8, text: &str, slots: &[Option<Value>]) -> Option<HostRequest> {
        let name = menu_str(slots.first());
        let corner = |i: usize| slots.get(1 + i).and_then(|v| v.as_ref()).and_then(|v| v.as_number().ok());
        let title = menu_str(slots.get(5));
        // `IN WINDOW <name>` left its name in `text` marked with `\u{1}` so it survives being
        // told apart from the style words - see `window_clauses` in the parser - and does not
        // itself get read as one of them.
        let mut parent = String::new();
        let mut style_words = String::new();
        for word in text.split_whitespace() {
            if let Some(name) = word.strip_prefix('\u{1}') {
                parent = name.to_string();
            } else {
                if !style_words.is_empty() {
                    style_words.push(' ');
                }
                style_words.push_str(word);
            }
        }
        let words = style_words.to_ascii_uppercase();
        let screen = &mut self.screen;
        match kind {
            // DEFINE WINDOW: FROM r1, c1 TO r2, c2, or the whole screen when it says nothing
            0 => {
                let (r1, c1) = (corner(0).unwrap_or(0.0), corner(1).unwrap_or(0.0));
                let (r2, c2) = (
                    corner(2).unwrap_or(crate::screen::SCREEN_ROWS as f64 - 1.0),
                    corner(3).unwrap_or(crate::screen::SCREEN_COLS as f64 - 1.0),
                );
                let window = screen.define(&name, r1, c1, (r2 - r1 + 1.0).max(1.0), (c2 - c1 + 1.0).max(1.0));
                window.title = title;
                window.border = !words.contains("NONE");
                window.movable = !words.contains("NOFLOAT");
                window.parent = parent;
            }
            1 => screen.activate(&name, flags & 2 == 0),
            2 => screen.deactivate(if flags & 1 != 0 || name.is_empty() { None } else { Some(&name) }),
            // SHOW and HIDE change whether it is on the screen without changing where output goes
            3 | 4 => {
                let show = kind == 3;
                match (flags & 1 != 0 || name.is_empty(), show) {
                    (true, _) => {
                        for window in &mut screen.windows {
                            window.visible = show;
                        }
                    }
                    (false, _) => {
                        if let Some(window) = screen.window_mut(&name) {
                            window.visible = show;
                        }
                    }
                }
                if show {
                    screen.activate(&name, false);
                } else {
                    screen.order.retain(|n| !n.eq_ignore_ascii_case(&name));
                }
            }
            5 => {
                if let Some(window) = screen.window_mut(&name) {
                    window.row = corner(0).unwrap_or(window.row);
                    window.col = corner(1).unwrap_or(window.col);
                }
            }
            6 => {
                if let Some(window) = screen.window_mut(&name) {
                    window.height = corner(0).unwrap_or(window.height).max(1.0);
                    window.width = corner(1).unwrap_or(window.width).max(1.0);
                    window.grid = crate::screen::Grid::new(window.height as usize, window.width as usize);
                }
            }
            // ZOOM MAX fills the screen, MIN shrinks it to its title, NORM puts it back
            7 => {
                let (rows, cols) = (crate::screen::SCREEN_ROWS as f64, crate::screen::SCREEN_COLS as f64);
                if let Some(window) = screen.window_mut(&name) {
                    let was = (window.row, window.col, window.height, window.width);
                    window.zoomed = words.contains("MAX");
                    window.minimized = words.contains("MIN");
                    if window.zoomed {
                        window.normal.get_or_insert(was);
                        window.row = 0.0;
                        window.col = 0.0;
                        window.height = rows;
                        window.width = cols;
                        window.grid = crate::screen::Grid::new(rows as usize, cols as usize);
                    } else if !window.minimized
                        && let Some((row, col, height, width)) = window.normal.take()
                    {
                        window.row = row;
                        window.col = col;
                        window.height = height;
                        window.width = width;
                        window.grid = crate::screen::Grid::new(height as usize, width as usize);
                    }
                }
            }
            8 => {
                if let Some(window) = screen.window_mut(&name) {
                    if !title.is_empty() {
                        window.title = title;
                    }
                    if let Some(row) = corner(0) {
                        window.row = row;
                    }
                    if let Some(col) = corner(1) {
                        window.col = col;
                    }
                }
            }
            9 => screen.release(if flags & 1 != 0 || name.is_empty() { None } else { Some(&name) }),
            10 => {
                let kept = screen.windows.clone();
                screen.saved_windows.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
                screen.saved_windows.push((name, kept));
            }
            11 => {
                if let Some((_, kept)) = screen.saved_windows.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)) {
                    screen.windows = kept.clone();
                    screen.order.retain(|n| screen.windows.iter().any(|w| w.name.eq_ignore_ascii_case(n)));
                }
            }
            // ACTIVATE SCREEN: output goes back to the screen behind the windows
            12 => {
                screen.last = std::mem::take(&mut screen.output);
            }
            13 => screen.saved = Some(screen.main.clone()),
            14 => {
                if let Some(kept) = screen.saved.clone() {
                    screen.main = kept;
                }
            }
            15 => {
                if flags & 1 != 0 {
                    screen.release(None);
                } else {
                    screen.surface().clear();
                    screen.cursor = (0, 0);
                    screen.gets.clear();
                    screen.prompts.clear();
                }
            }
            // READ: the fields go to the host, which lets the user into them
            16 => {
                if screen.gets.is_empty() {
                    return None;
                }
                screen.read_level += 1;
                let fields = screen
                    .gets
                    .iter()
                    .map(|g| crate::host::GetField {
                        name: g.name.clone(),
                        row: g.row,
                        col: g.col,
                        width: g.width,
                        picture: g.picture.clone(),
                        enabled: g.enabled,
                        value: JsonValue::from_value(&g.value),
                    })
                    .collect();
                return Some(HostRequest::ReadGets { fields });
            }
            // SHOW GETS draws them again, which handing the screen over already does
            17 => {}
            18 => {
                screen.gets.clear();
                screen.prompts.clear();
            }
            19 => {
                if screen.prompts.is_empty() {
                    return None;
                }
                return Some(HostRequest::ChooseFrom { prompts: screen.prompts.clone() });
            }
            _ => {}
        }
        None
    }

    /// A `READ` has come back: each value goes back where the field it was in came from, and
    /// the fields come off the screen.
    fn read_finished(&mut self, fb: &mut Fiber, reply: &Value) {
        let gets = std::mem::take(&mut self.screen.gets);
        if let Value::Array(values) = reply.deref() {
            let values = values.borrow();
            for (get, value) in gets.iter().zip(values.items.iter()) {
                self.write_back(fb, &get.name, value.deref());
            }
        }
        self.screen.read_var = gets.last().map(|g| g.name.to_ascii_uppercase()).unwrap_or_default();
        self.screen.read_level = self.screen.read_level.saturating_sub(1);
        self.screen.prompts.clear();
    }

    /// Where a field's value goes when the read is over: the field of the table it named, or
    /// the variable, exactly as Visual FoxPro reads the name.
    fn write_back(&mut self, fb: &mut Fiber, name: &str, value: Value) {
        let upper = name.trim().to_ascii_uppercase();
        let forced = upper.starts_with("M.");
        let bare = upper.strip_prefix("M.").unwrap_or(&upper).to_string();
        let field = bare.rsplit('.').next().unwrap_or(&bare).to_string();
        let is_field = !forced
            && self.load_name(fb, &bare).is_none()
            && self.data.cursor().is_some_and(|c| c.header.fields.iter().any(|f| f.name.eq_ignore_ascii_case(&field)));
        if is_field
            && let Some(cursor) = self.data.cursor_mut()
        {
            let _ = cursor.set_field(&field, &value);
            return;
        }
        // A read has no error to come back to: if the name is a system variable and what was
        // typed is the wrong type for it, the write is refused and the variable keeps what it had.
        let _ = self.store_name(fb, &bare, value);
    }

    /// One thing an `@` line draws.
    fn at_command(
        &mut self,
        kind: u8,
        style: &str,
        name: &str,
        valid: &str,
        when: &str,
        slots: &[Option<Value>],
    ) -> Result<(), RtError> {
        let number = |i: usize| slots.get(i).and_then(|v| v.as_ref()).and_then(|v| v.as_number().ok());
        let (row, col) = (number(0).unwrap_or(0.0).max(0.0) as usize, number(1).unwrap_or(0.0).max(0.0) as usize);
        let value = slots.get(2).and_then(|v| v.clone());
        let picture = menu_str(slots.get(5));
        let function = menu_str(slots.get(6));
        let amount = number(7);
        let across = number(8);
        let settings = self.settings.clone();
        let words = style.to_ascii_uppercase();
        // the corner a command gave, or the bottom right of the surface it is drawing on
        let surface_end = {
            let grid = self.screen.surface();
            (grid.rows.saturating_sub(1), grid.cols.saturating_sub(1))
        };
        let r2 = number(3).map_or(surface_end.0, |n| n.max(0.0) as usize);
        let c2 = number(4).map_or(surface_end.1, |n| n.max(0.0) as usize);
        match kind {
            // SAY: the value as PICTURE and FUNCTION have it, put where the line said
            0 => {
                let text = match &value {
                    Some(v) => {
                        let mask = if picture.is_empty() && !function.is_empty() {
                            format!("@{function}")
                        } else if function.is_empty() {
                            picture.clone()
                        } else {
                            format!("@{function} {picture}")
                        };
                        crate::builtins::string::transform(v, Some(&mask), &settings)
                    }
                    None => String::new(),
                };
                self.screen.surface().put(row, col, &text);
                self.screen.cursor = (row, col + text.chars().count());
            }
            // GET: the field is drawn where it goes and waits for a READ
            1 => {
                let shown = match &value {
                    Some(v) => crate::builtins::string::transform(v, Some(&picture), &settings),
                    None => String::new(),
                };
                self.screen.surface().put(row, col, &shown);
                self.screen.cursor = (row, col + shown.chars().count());
                self.screen.gets.push(crate::screen::Get {
                    name: name.to_string(),
                    row,
                    col,
                    width: shown.chars().count().max(1),
                    picture,
                    valid: valid.to_string(),
                    when: when.to_string(),
                    enabled: !words.contains("DISABLE"),
                    value: value.clone().unwrap_or(Value::Null),
                });
            }
            2 => self.screen.surface().fill(row, col, r2, c2, ' '),
            // TO and BOX both draw a frame; TO picks its characters from a word, BOX from a string
            3 | 4 => {
                let chars = match (&value, words.contains("DOUBLE") || words.contains("PANEL")) {
                    (Some(v), _) => v.as_str().map(|s| s.to_string()).unwrap_or_default(),
                    (None, true) if words.contains("PANEL") => "\u{2588}".repeat(6),
                    (None, true) => "\u{2550}\u{2551}\u{2554}\u{2557}\u{255d}\u{255a}".to_string(),
                    (None, false) => "\u{2500}\u{2502}\u{250c}\u{2510}\u{2518}\u{2514}".to_string(),
                };
                self.screen.surface().draw_box(row, col, r2, c2, &chars);
            }
            5 => {
                let ch = value
                    .as_ref()
                    .and_then(|v| v.as_str().ok())
                    .and_then(|s| s.chars().next())
                    .unwrap_or(' ');
                self.screen.surface().fill(row, col, r2, c2, ch);
            }
            6 => {
                let by = amount.unwrap_or(1.0) as i32;
                // `@ ... SCROLL` names one direction in a word; the SCROLL command counts rows
                // and columns, where rows go up and columns go right when the count is positive
                let (up, left) = match across {
                    Some(n) => (by, -(n as i32)),
                    None if words.contains("DOWN") => (-by, 0),
                    None if words.contains("LEFT") => (0, by),
                    None if words.contains("RIGHT") => (0, -by),
                    None => (by, 0),
                };
                let grid = self.screen.surface();
                if across.is_none() && up == 0 && left == 0 {
                    // a scroll of no rows, with no column count beside it, empties the region
                    grid.fill(row, col, r2, c2, ' ');
                } else {
                    if up != 0 {
                        grid.scroll(row, col, r2, c2, up, false);
                    }
                    if left != 0 {
                        grid.scroll(row, col, r2, c2, left, true);
                    }
                }
            }
            // MENU and PROMPT put a choice on the screen, which MENU TO then reads
            7 | 8 => {
                if let Some(text) = value.as_ref().and_then(|v| v.as_str().ok()) {
                    self.screen.surface().put(row, col, &text);
                    if kind == 8 {
                        self.screen.prompts.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The character screen, for the functions that report on it.
    pub fn screen(&self) -> &crate::screen::Screen {
        &self.screen
    }

    /// The command hung off something that may happen, for `ON()`.
    pub fn handler(&self, what: &str) -> Option<&str> {
        self.handlers.get(&what.to_ascii_uppercase()).map(String::as_str)
    }

    // ----- menus -------------------------------------------------------------------------------

    /// One menu command. Everything it names is already off the stack, in the order the
    /// instruction documents; what comes back is the request that hands the host the menu,
    /// where the command puts one up or takes one down.
    fn menu_command(&mut self, kind: u8, flags: u8, text: &str, slots: &[Option<Value>]) -> Option<HostRequest> {
        let name = menu_str(slots.first());
        let of = menu_str(slots.get(1));
        let number = menu_num(slots.get(2));
        let prompt = menu_str(slots.get(3));
        let key = menu_str(slots.get(4));
        let message = menu_str(slots.get(5));
        let popup = flags & 4 != 0;
        let menus = &mut self.menus;
        match kind {
            0 => menus.define_menu(&name),
            1 => {
                let menu = if of.is_empty() { SYSTEM_MENU.to_string() } else { of };
                menus.define_pad(&menu, &name, &prompt, &key, &message);
                if let Some(pad) = menus.menu_mut(&menu).and_then(|m| m.pads.last_mut()) {
                    pad.skip = text.to_string();
                }
            }
            2 => menus.define_popup(&name),
            3 => {
                let owner = if of.is_empty() { menus.defining_popup.clone() } else { of };
                menus.define_bar(&owner, number, &prompt, &key, &message);
                if let Some(bar) = menus.popup_mut(&owner).and_then(|p| p.bars.iter_mut().find(|b| b.number == number)) {
                    bar.skip = text.to_string();
                }
                menus.defining_popup = owner;
            }
            // ON PAD x OF y ACTIVATE POPUP z: the pad opens that popup
            4 => {
                let menu = if of.is_empty() { SYSTEM_MENU.to_string() } else { of };
                if let Some(pad) = menus.menu_mut(&menu).and_then(|m| m.pads.iter_mut().find(|p| p.name.eq_ignore_ascii_case(&name))) {
                    pad.popup = text.to_string();
                }
            }
            5 => {
                if let Some(bar) = menus.popup_mut(&of).and_then(|p| p.bars.iter_mut().find(|b| b.number == number)) {
                    bar.popup = text.to_string();
                }
            }
            6 => {
                let menu = if of.is_empty() { SYSTEM_MENU.to_string() } else { of };
                if name.is_empty() {
                    // ON SELECTION PAD ALL: every pad of that menu runs it
                    if let Some(m) = menus.menu_mut(&menu) {
                        for pad in &mut m.pads {
                            pad.command = text.to_string();
                        }
                    }
                } else if let Some(pad) = menus.menu_mut(&menu).and_then(|m| m.pads.iter_mut().find(|p| p.name.eq_ignore_ascii_case(&name))) {
                    pad.command = text.to_string();
                }
            }
            7 => {
                if let Some(bar) = menus.popup_mut(&of).and_then(|p| p.bars.iter_mut().find(|b| b.number == number)) {
                    bar.command = text.to_string();
                }
            }
            // a command for the whole popup or the whole menu, which a choice falls back to
            8 => {
                if name.is_empty() {
                    for p in &mut menus.popups {
                        p.command = text.to_string();
                    }
                } else if let Some(p) = menus.popup_mut(&name) {
                    p.command = text.to_string();
                }
            }
            9 => {
                let menu = if name.is_empty() { SYSTEM_MENU.to_string() } else { name };
                if let Some(m) = menus.menu_mut(&menu) {
                    m.command = text.to_string();
                    for pad in &mut m.pads {
                        if pad.command.is_empty() && pad.popup.is_empty() {
                            pad.command = text.to_string();
                        }
                    }
                }
            }
            // ON EXIT runs as a choice is left, which nothing here can do without making it
            10 => {}
            // ACTIVATE and SHOW put it up; DEACTIVATE and HIDE take it down
            11 | 13 => {
                if popup {
                    menus.active_popup = name.clone();
                } else {
                    menus.active_menu = name.clone();
                    menus.last_menu = name.clone();
                }
                return Some(HostRequest::SetMenu { menu: menus.document(&name, popup) });
            }
            // DEACTIVATE ends it; HIDE only takes it off the screen, so MENU() still names it
            12 => {
                if popup {
                    menus.active_popup.clear();
                } else {
                    menus.active_menu.clear();
                }
                return Some(HostRequest::SetMenu { menu: None });
            }
            14 => return Some(HostRequest::SetMenu { menu: None }),
            15 => {
                let was_up = if popup { !menus.active_popup.is_empty() } else { !menus.active_menu.is_empty() };
                menus.release(if name.is_empty() { None } else { Some(&name) }, popup);
                let still_up = if popup { !menus.active_popup.is_empty() } else { !menus.active_menu.is_empty() };
                if was_up && !still_up {
                    return Some(HostRequest::SetMenu { menu: None });
                }
            }
            16 => menus.push(),
            17 => {
                menus.pop();
                let up = menus.active_menu.clone();
                if !up.is_empty() {
                    return Some(HostRequest::SetMenu { menu: menus.document(&up, false) });
                }
            }
            // SET MARK OF: the tick beside a pad or a bar
            18 => {
                let on = slots.get(3).and_then(|v| v.as_ref()).and_then(|v| v.truthy().ok()).unwrap_or(false);
                menus.set_mark(&name, number, &of, flags, on);
            }
            // SET SKIP OF: the condition that greys it out, kept as it was written
            19 => menus.set_skip(&name, number, &of, flags, text),
            20 => menus.message = prompt,
            21 => menus.sysmenu = text.to_string(),
            // RELEASE BAR and RELEASE PAD: one item out of a menu that stays, or all of them
            22 => menus.release_bar(&of, slots.get(2).and_then(|v| v.as_ref()).map(|_| number)),
            23 => menus.release_pad(&of, if name.is_empty() { None } else { Some(&name) }),
            _ => {}
        }
        None
    }

    /// What the host says was chosen from the menu that is up, so the command it stands for
    /// can ask BAR(), PAD(), POPUP() and PROMPT() what it was.
    pub fn menu_chosen(&mut self, pad: &str, bar: i32, popup: &str, prompt: &str) {
        self.menus.chose(pad, bar, popup, prompt);
    }

    /// The menus a program has defined, for the functions that report on them.
    pub fn menus(&self) -> &crate::menu::Menus {
        &self.menus
    }

    // ----- databases ---------------------------------------------------------------------------
    //
    // A database is a container file - a table of its own, with a memo file beside it - listing
    // the tables and views that belong to it. It is read whole when it is opened and written
    // whole when something in it changes, which is two files each way and so two round trips.

    /// The database commands, and the file a command has to read or write before it can finish.
    #[allow(clippy::too_many_arguments)]
    fn db_command(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        kind: u8,
        named: bool,
        target: bool,
        sql: Option<String>,
        flags: u16,
    ) -> Result<Option<HostRequest>, RtError> {
        use crate::ast::db_flags as df;
        let flag = |bit: u16| flags & bit != 0;
        let reply = fb.data_reply.take();
        // a command that reads or writes files is part-way through when this is set
        if self.db_io.is_some() {
            return self.db_io_step(reply);
        }
        // a command that had to ask the host for a file took its name off the stack the first
        // time through; it is kept until the answer comes back
        let waiting = self.db_await.take();
        let second = if target && waiting.is_none() { Some(pop_value(fb)?.as_str()?.trim().to_string()) } else { None };
        let name = match &waiting {
            Some(name) => Some(name.clone()),
            None if named => Some(pop_value(fb)?.as_str()?.trim().to_string()),
            None => None,
        };
        match kind {
            // CREATE DATABASE: a container with nothing in it but itself
            0 => {
                let path = self.settings.at(&with_extension(&name.clone().unwrap_or_default(), "dbc"));
                let objects = vec![crate::dbc::DbObject::new(1, 0, "Database", &stem_of(&path))];
                self.databases.push(Database { path: path.clone(), objects, open: true, procedures: None });
                self.current_db = Some(self.databases.len() - 1);
                Ok(self.write_database(self.databases.len() - 1))
            }
            // OPEN DATABASE: the container is read, both files of it
            1 => {
                let path = self.settings.at(&with_extension(&name.unwrap_or_default(), "dbc"));
                if let Some(i) = self.databases.iter().position(|d| d.path.eq_ignore_ascii_case(&path)) {
                    self.databases[i].open = true;
                    // opening the one that is already current says nothing at all - measured
                    if self.current_db == Some(i) {
                        return Ok(None);
                    }
                    let was = self.current_db;
                    self.current_db = Some(i);
                    if !self.dbc_event(host, fb, "dbc_Activate", &[DbcArg::name(&path)]) {
                        self.current_db = was;
                    }
                    return Ok(None);
                }
                self.open_clauses = flags;
                self.db_io = Some(DbIo { stage: 0, path: path.clone(), dbf: Vec::new(), memo: Vec::new(), writing: false, attach: false });
                Ok(Some(HostRequest::FileReadBytes { path }))
            }
            // CLOSE DATABASES is done a step above this, where the work areas its tables hold
            // can be handed back to the host in one go
            2 => Ok(None),
            // SET DATABASE TO
            3 => {
                let wanted = match &name {
                    None => None,
                    Some(name) if name.is_empty() => None,
                    Some(name) => {
                        let want = stem_of(name);
                        let found = self.databases.iter().position(|d| stem_of(&d.path).eq_ignore_ascii_case(&want));
                        Some(found.ok_or_else(|| {
                            RtError::new(RtError::FILE_NOT_FOUND, format!("Database '{name}' is not open"))
                        })?)
                    }
                };
                if wanted == self.current_db {
                    return Ok(None);
                }
                // the one that was current is left first, and it is its own procedures that
                // hear it; a .F. from dbc_Deactivate leaves it current - measured
                if let Some(leaving) = self.current_db {
                    let path = self.databases[leaving].path.clone();
                    if !self.dbc_event_in(host, fb, leaving, "dbc_Deactivate", &[DbcArg::name(&path)]) {
                        return Ok(None);
                    }
                }
                self.current_db = wanted;
                if let Some(entering) = wanted {
                    let path = self.databases[entering].path.clone();
                    if !self.dbc_event(host, fb, "dbc_Activate", &[DbcArg::name(&path)]) {
                        self.current_db = None;
                    }
                }
                Ok(None)
            }
            // DELETE DATABASE: the container goes, and its tables stay where they are
            4 => {
                let path = self.settings.at(&with_extension(&name.unwrap_or_default(), "dbc"));
                self.databases.retain(|d| !d.path.eq_ignore_ascii_case(&path));
                self.current_db = None;
                // the answer to the delete says nothing, and the command is done when it comes
                self.db_io = Some(DbIo { stage: 1, path: path.clone(), dbf: Vec::new(), memo: Vec::new(), writing: true, attach: false });
                Ok(Some(HostRequest::FileDelete { path }))
            }
            // VALIDATE DATABASE: nothing here can be out of step, so nothing is ever wrong.
            //
            // The event is handed the four clauses the command was written with, and a fifth
            // argument - the file - only when TO FILE was one of them: measured, a program that
            // declares five parameters and validates without TO FILE is called with four.
            5 => {
                let current_name = match self.current_db {
                    Some(index) => stem_of(&self.databases[index].path),
                    None => return Ok(None),
                };
                let mut args = vec![
                    DbcArg::Flag(flag(df::RECOVER)),
                    DbcArg::Flag(flag(df::NOCONSOLE)),
                    DbcArg::Flag(flag(df::PRINTER)),
                    DbcArg::Flag(flag(df::TO_FILE)),
                ];
                if flag(df::TO_FILE) {
                    args.push(DbcArg::name(self.settings.at(&second.clone().unwrap_or_default())));
                }
                if self.dbc_event(host, fb, "dbc_BeforeValidateData", &args) {
                    // What the product writes, measured: the same four lines for a container
                    // with nothing in it and for one with tables, views and a connection, and
                    // the same whether NOCONSOLE was written or not - NOCONSOLE keeps the
                    // report off the screen and not out of the run's output.
                    let named = current_name.to_ascii_uppercase();
                    host.output(&format!("Validate Database {named}:"), true);
                    host.output("Rebuilding structural index....  Index rebuilt.", true);
                    host.output("Database container is valid.", true);
                    host.output("", true);
                    self.dbc_event(host, fb, "dbc_AfterValidateData", &args);
                }
                Ok(None)
            }
            // FREE TABLE unties a table from a database that has gone, so unlike the rest of
            // these it works with no database open. Nothing here keeps the link a table's own
            // header holds to its database, so what is left to do is take it out of one that
            // is open and still lists it.
            //
            // Kind 22 is a table taking its place in whatever database is open, which is what
            // CREATE TABLE does when the program did not say FREE. With none open it is not an
            // error - the table is simply a free one.
            8 | 22 if self.current_db.is_none() => Ok(None),
            // ADD TABLE, REMOVE TABLE, FREE TABLE, RENAME TABLE
            6 | 7 | 8 | 9 | 22 => {
                let Some(current) = self.current_db else {
                    return Err(RtError::new(RtError::FILE_NOT_FOUND, "No database is open"));
                };
                let written = name.unwrap_or_default();
                let name = stem_of(&written);
                // What each of them is told. ADD TABLE hears the file and the name the
                // container will hold it under; REMOVE TABLE hears the name and whether the
                // file was to go with it; RENAME TABLE hears both names. The add that CREATE
                // TABLE does for itself - kind 22 - is told nothing at all: measured, creating
                // a table in an open database fires the create events and no add event.
                let (before, after, args) = match kind {
                    // the step CREATE TABLE ends with: the table is in the container now, which
                    // is when the product says the creating is done
                    22 => (
                        "",
                        "dbc_AfterCreateTable",
                        vec![DbcArg::name(self.settings.table_at(&written)), DbcArg::name(&name)],
                    ),
                    6 => (
                        "dbc_BeforeAddTable",
                        "dbc_AfterAddTable",
                        vec![DbcArg::name(self.settings.table_at(&written)), DbcArg::name(&name)],
                    ),
                    7 | 8 => (
                        "dbc_BeforeRemoveTable",
                        "dbc_AfterRemoveTable",
                        vec![DbcArg::name(&name), DbcArg::Flag(flag(df::DELETE)), DbcArg::Flag(flag(df::RECYCLE))],
                    ),
                    _ => (
                        "dbc_BeforeRenameTable",
                        "dbc_AfterRenameTable",
                        vec![DbcArg::name(&name), DbcArg::name(stem_of(second.as_deref().unwrap_or_default()))],
                    ),
                };
                if !before.is_empty() && !self.dbc_event(host, fb, before, &args) {
                    return Ok(None);
                }
                match kind {
                    6 | 22 => {
                        let next = self.databases[current].next_id();
                        let db = &mut self.databases[current];
                        let already = db.objects.iter().any(|o| o.kind == "Table" && o.name.eq_ignore_ascii_case(&name));
                        // A table already in a database cannot be added to it again: the product
                        // refuses with "Cannot add this table: it belongs to database <dbc>" -
                        // measured. The implicit add that CREATE TABLE does (kind 22) is not the
                        // command and says nothing, which is what lets a program create a table
                        // in an open database and carry on.
                        if already && kind == 6 {
                            let path = db.path.clone();
                            return Err(RtError::new(
                                RtError::TABLE_IN_DATABASE,
                                format!("Cannot add this table: it belongs to database {path}."),
                            ));
                        }
                        if !already {
                            let mut object = crate::dbc::DbObject::new(next, 1, "Table", &name.to_ascii_uppercase());
                            // where the file is, so the container can answer for it afterwards
                            // however the table is renamed
                            crate::dbc::set_table_file(&mut object, &with_extension(&written, "dbf"));
                            db.objects.push(object);
                        }
                    }
                    7 | 8 => {
                        let db = &mut self.databases[current];
                        db.objects.retain(|o| !(o.kind == "Table" && o.name.eq_ignore_ascii_case(&name)));
                    }
                    _ => {
                        let to = stem_of(&second.unwrap_or_default()).to_ascii_uppercase();
                        let db = &mut self.databases[current];
                        if let Some(object) =
                            db.objects.iter_mut().find(|o| o.kind == "Table" && o.name.eq_ignore_ascii_case(&name))
                        {
                            object.name = to;
                        }
                    }
                }
                if !after.is_empty() {
                    self.dbc_event(host, fb, after, &args);
                }
                Ok(self.write_database(current))
            }
            // CREATE SQL VIEW: the SELECT it stands for is kept in the container
            10 => {
                let Some(current) = self.current_db else {
                    return Err(RtError::new(RtError::FILE_NOT_FOUND, "No database is open"));
                };
                let name = stem_of(&name.unwrap_or_default()).to_ascii_uppercase();
                if !self.dbc_event(host, fb, "dbc_BeforeCreateView", &[DbcArg::name(&name)]) {
                    return Ok(None);
                }
                let next = self.databases[current].next_id();
                let db = &mut self.databases[current];
                db.objects.retain(|o| !(o.kind == "View" && o.name.eq_ignore_ascii_case(&name)));
                let mut view = crate::dbc::DbObject::new(next, 1, "View", &name);
                view.code = sql.unwrap_or_default();
                db.objects.push(view);
                // whether the view is a remote one is on the After event alone, which is how
                // the reference has it and how it measures
                self.dbc_event(host, fb, "dbc_AfterCreateView", &[DbcArg::name(&name), DbcArg::Flag(flag(df::REMOTE))]);
                Ok(self.write_database(current))
            }
            // DELETE VIEW and DROP VIEW: the view goes, and the container is told nothing at
            // all. Measured: neither command calls dbc_BeforeDeleteView or dbc_AfterDeleteView,
            // though both procedures exist and every other event of the run fired.
            11 => {
                let Some(current) = self.current_db else { return Ok(None) };
                let name = stem_of(&name.unwrap_or_default());
                let db = &mut self.databases[current];
                db.objects.retain(|o| !(o.kind == "View" && o.name.eq_ignore_ascii_case(&name)));
                Ok(self.write_database(current))
            }
            // CREATE CONNECTION, DELETE CONNECTION, RENAME CONNECTION and MODIFY CONNECTION:
            // a named way of reaching a data source, kept in the container beside the views
            // that use it
            18 | 19 | 20 | 21 => {
                let Some(current) = self.current_db else {
                    return Err(RtError::new(RtError::FILE_NOT_FOUND, "No database is open"));
                };
                let name = name.unwrap_or_default().to_ascii_uppercase();
                let what = match kind {
                    18 => "Create",
                    19 => "Delete",
                    20 => "Rename",
                    _ => "Modify",
                };
                // What each is told, measured. A rename hears both names; deleting and
                // modifying hear the one; and creating hears nothing at all before it and five
                // afterwards - the name, the data source, the user, the password and the
                // connection string. This runtime keeps a connection as the one string it was
                // given, so the two of those five it was not told read as empty.
                let mut args = vec![DbcArg::name(&name)];
                if kind == 20 && let Some(second) = &second {
                    args.push(DbcArg::name(second));
                }
                let written = second.clone().unwrap_or_default();
                let after_args = if kind == 18 {
                    let source = if flag(df::DATASOURCE) { written.clone() } else { String::new() };
                    let string = if flag(df::DATASOURCE) { String::new() } else { written.clone() };
                    vec![
                        DbcArg::name(&name),
                        DbcArg::name(source),
                        DbcArg::name(""),
                        DbcArg::name(""),
                        DbcArg::name(string),
                    ]
                } else {
                    args.clone()
                };
                let before_args: &[DbcArg] = if kind == 18 { &[] } else { &args };
                if !self.dbc_event(host, fb, &format!("dbc_Before{what}Connection"), before_args) {
                    return Ok(None);
                }
                let db = &mut self.databases[current];
                let next = db.objects.iter().map(|o| o.id).max().unwrap_or(1) + 1;
                let is = |o: &crate::dbc::DbObject| o.kind == "Connection" && o.name.eq_ignore_ascii_case(&name);
                match kind {
                    18 => {
                        db.objects.retain(|o| !is(o));
                        let mut made = crate::dbc::DbObject::new(next, 1, "Connection", &name);
                        // what the connection is: the string a program hands to a data source
                        made.code = second.clone().unwrap_or_default();
                        db.objects.push(made);
                    }
                    19 => db.objects.retain(|o| !is(o)),
                    20 => {
                        let to = second.clone().unwrap_or_default().to_ascii_uppercase();
                        for object in db.objects.iter_mut().filter(|o| {
                            o.kind == "Connection" && o.name.eq_ignore_ascii_case(&name)
                        }) {
                            object.name = to.clone();
                        }
                    }
                    // MODIFY CONNECTION opens the designer, which this runtime has not; what it
                    // does have is the event that says the connection was changed
                    _ => {}
                }
                self.dbc_event(host, fb, &format!("dbc_After{what}Connection"), &after_args);
                Ok(self.write_database(current))
            }
            // APPEND PROCEDURES FROM: the stored procedures come from a file of FoxPro source
            14 => {
                let Some(current) = self.current_db else {
                    return Err(RtError::new(RtError::FILE_NOT_FOUND, "No database is open"));
                };
                let path = self.settings.at(&name.unwrap_or_default());
                match reply {
                    // the file has arrived: it becomes what the container carries
                    Some(text) => {
                        let source = text.as_str().unwrap_or_default().to_string();
                        // the file, the code page its text is read in, and whether OVERWRITE
                        // was written - which is the one clause the command has
                        let args = [DbcArg::name(&path), DbcArg::Num(TEXT_CODE_PAGE), DbcArg::Flag(flag(df::ADDITIVE))];
                        if self.dbc_event(host, fb, "dbc_BeforeAppendProc", &args) {
                            self.databases[current].set_procedures(source);
                            self.dbc_event(host, fb, "dbc_AfterAppendProc", &args);
                        }
                        Ok(self.write_database(current))
                    }
                    None => {
                        self.db_await = Some(path.clone());
                        Ok(Some(HostRequest::FileRead { path, search: Vec::new() }))
                    }
                }
            }
            // COPY PROCEDURES TO: the same, written back out as source
            15 => {
                let Some(current) = self.current_db else {
                    return Err(RtError::new(RtError::FILE_NOT_FOUND, "No database is open"));
                };
                let path = self.settings.at(&name.unwrap_or_default());
                let args = [DbcArg::name(&path), DbcArg::Num(TEXT_CODE_PAGE), DbcArg::Flag(flag(df::ADDITIVE))];
                if !self.dbc_event(host, fb, "dbc_BeforeCopyProc", &args) {
                    return Ok(None);
                }
                let text = self.databases[current].procedure_source().to_string();
                self.dbc_event(host, fb, "dbc_AfterCopyProc", &args);
                self.db_io = Some(DbIo { stage: 1, path: path.clone(), dbf: Vec::new(), memo: Vec::new(), writing: true, attach: false });
                Ok(Some(HostRequest::FileWrite { path, text, append: false }))
            }
            // PACK DATABASE: the container is written out again, without what was crossed off
            16 => {
                let Some(current) = self.current_db else {
                    return Err(RtError::new(RtError::FILE_NOT_FOUND, "No database is open"));
                };
                // a pack the container refuses is one of the three refusals that raise 1705
                if !self.dbc_event(host, fb, "dbc_PackData", &[]) {
                    return Err(Self::dbc_refused(&self.databases[current].path.clone()));
                }
                Ok(self.write_database(current))
            }
            // RENAME VIEW
            17 => {
                let Some(current) = self.current_db else { return Ok(None) };
                let from = stem_of(&name.unwrap_or_default());
                let to = stem_of(&second.unwrap_or_default()).to_ascii_uppercase();
                let args = [DbcArg::name(&from), DbcArg::name(&to)];
                if !self.dbc_event(host, fb, "dbc_BeforeRenameView", &args) {
                    return Ok(None);
                }
                if let Some(view) =
                    self.databases[current].objects.iter_mut().find(|o| o.kind == "View" && o.name.eq_ignore_ascii_case(&from))
                {
                    view.name = to.clone();
                }
                self.dbc_event(host, fb, "dbc_AfterRenameView", &args);
                Ok(self.write_database(current))
            }
            // DROP TABLE: the file goes, and the container is told before and after
            13 => {
                let path = name.unwrap_or_default();
                let args = [DbcArg::name(stem_of(&path)), DbcArg::Flag(flag(df::RECYCLE))];
                if !self.dbc_event(host, fb, "dbc_BeforeDropTable", &args) {
                    return Ok(None);
                }
                if let Some(current) = self.current_db {
                    let table = stem_of(&path);
                    self.databases[current].objects.retain(|o| !(o.kind == "Table" && o.name.eq_ignore_ascii_case(&table)));
                }
                self.dbc_event(host, fb, "dbc_AfterDropTable", &args);
                let path = self.settings.table_at(&path);
                self.db_io = Some(DbIo { stage: 1, path: path.clone(), dbf: Vec::new(), memo: Vec::new(), writing: true, attach: false });
                Ok(Some(HostRequest::FileDelete { path }))
            }
            // LIST DATABASE
            _ => {
                let Some(current) = self.current_db else { return Ok(None) };
                for object in &self.databases[current].objects {
                    if object.kind.eq_ignore_ascii_case("StoredProc") {
                        continue;
                    }
                    host.output(&format!("{:<10} {}", object.kind, object.name), true);
                }
                Ok(None)
            }
        }
    }

    /// The next file of a database container on its way to or from the host.
    fn db_io_step(&mut self, reply: Option<Value>) -> Result<Option<HostRequest>, RtError> {
        let Some(state) = &mut self.db_io else { return Ok(None) };
        let memo_path = with_extension(&state.path, "dct");
        if state.writing {
            match state.stage {
                0 => {
                    state.stage = 1;
                    let bytes = std::mem::take(&mut state.memo);
                    return Ok(Some(HostRequest::FileWriteBytes { path: memo_path, bytes }));
                }
                _ => {
                    self.db_io = None;
                    return Ok(None);
                }
            }
        }
        match state.stage {
            0 => {
                state.dbf = bytes_of(&reply.unwrap_or(Value::Null));
                state.stage = 1;
                Ok(Some(HostRequest::FileReadBytes { path: memo_path }))
            }
            _ => {
                let memo = bytes_of(&reply.unwrap_or(Value::Null));
                let state = self.db_io.take().expect("the container is being read");
                if state.dbf.is_empty() {
                    return Err(RtError::new(
                        RtError::FILE_NOT_FOUND,
                        format!("File '{}' does not exist", state.path),
                    ));
                }
                let objects = crate::dbc::read_dbc(&state.dbf, Some(&memo))
                    .map_err(|e| RtError::new(RtError::FILE_NOT_FOUND, e))?;
                let attach = state.attach;
                self.databases.push(Database { path: state.path, objects, open: true, procedures: None });
                // a container read only so that a `dbname!tablename` can be resolved is left
                // open without becoming the current one - measured
                if !attach {
                    self.current_db = Some(self.databases.len() - 1);
                    self.opened_database = Some(self.open_clauses);
                }
                Ok(None)
            }
        }
    }

    /// Writes a database container back: the table first and the memo file after it.
    fn write_database(&mut self, index: usize) -> Option<HostRequest> {
        let db = self.databases.get(index)?;
        let (dbf, memo) = crate::dbc::write_dbc(&db.objects);
        let path = db.path.clone();
        self.db_io = Some(DbIo { stage: 0, path: path.clone(), dbf: Vec::new(), memo, writing: true, attach: false });
        Some(HostRequest::FileWriteBytes { path, bytes: dbf })
    }

    /// The SELECT a view of that name stands for, when the database that is open has one.
    fn view_named(&self, path: &str) -> Option<String> {
        // a view is named, not a file: a path with a folder or an extension is a table
        if path.contains(['/', '\\']) || path.contains('.') {
            return None;
        }
        let db = self.current_database()?;
        db.view(path).map(|view| view.code.clone())
    }

    /// Calls the stored procedure of that name, when the current database has one.
    ///
    /// A database container can carry procedures named after the things that happen to it -
    /// `dbc_BeforeOpenTable`, `dbc_AfterAddTable` - and Visual FoxPro calls them as it does
    /// them. A `Before` one that answers .F. stops what was about to happen.
    fn dbc_event(&mut self, host: &mut dyn Host, fb: &mut Fiber, name: &str, args: &[DbcArg]) -> bool {
        let Some(index) = self.current_db else { return true };
        self.dbc_event_in(host, fb, index, name, args)
    }

    /// The same, for a database that is not the current one: leaving one is its own event, and
    /// the procedure that hears it belongs to the database being left rather than the one being
    /// entered.
    fn dbc_event_in(&mut self, host: &mut dyn Host, fb: &mut Fiber, index: usize, name: &str, args: &[DbcArg]) -> bool {
        // A container whose events have not been switched on says nothing at all, whatever
        // procedures it carries - measured.
        if !self.databases.get(index).is_some_and(Database::events_enabled) {
            return true;
        }
        let module = match self.database_procedures(index) {
            Some(module) => module,
            None => return true,
        };
        if self.modules[module as usize].find_func(&name.to_ascii_uppercase()).is_none() {
            return true;
        }
        // the call is written out and evaluated, which is what puts the arguments in place
        let written: Vec<String> = args.iter().map(DbcArg::written).collect();
        let call = format!("{name}({})", written.join(", "));
        match self.eval_in_frame(host, fb, &call) {
            Ok(value) => !matches!(value.deref(), Value::Logical(false)),
            // a procedure that will not run does not stop the command it belongs to
            Err(_) => true,
        }
    }

    /// `CLOSE DATABASES`: every container lets go, and each is told in the order the product
    /// tells it.
    ///
    /// Measured: dbc_CloseData first, then the container's own tables close - each with its
    /// pair of close events - and dbc_Deactivate last. A .F. from either of the two leaves the
    /// database open, which is the only way a program can refuse to let one go.
    fn close_databases(&mut self, host: &mut dyn Host, fb: &mut Fiber, flags: u16) -> Result<Flow, RtError> {
        let all = flags & crate::ast::db_flags::ALL != 0;
        let current = self.current_db.map(|i| self.databases[i].path.clone());
        let mut handles = Vec::new();
        let mut keep = Vec::new();
        for index in 0..self.databases.len() {
            let path = self.databases[index].path.clone();
            let closing = [DbcArg::name(&path), DbcArg::Flag(all)];
            if !self.dbc_event_in(host, fb, index, "dbc_CloseData", &closing) {
                keep.push(index);
                continue;
            }
            handles.extend(self.close_database_tables(host, fb, index)?);
            if !self.dbc_event_in(host, fb, index, "dbc_Deactivate", &[DbcArg::name(&path)]) {
                keep.push(index);
            }
        }
        let mut left = Vec::new();
        for (index, database) in std::mem::take(&mut self.databases).into_iter().enumerate() {
            if keep.contains(&index) {
                left.push(database);
            }
        }
        self.current_db = current.and_then(|was| left.iter().position(|d| d.path.eq_ignore_ascii_case(&was)));
        self.databases = left;
        Ok(self.release_handles(fb, handles))
    }

    /// The work areas holding tables of one database, closed, with the database told either
    /// side of each. Answers the host handles they were holding.
    fn close_database_tables(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        index: usize,
    ) -> Result<Vec<u32>, RtError> {
        let holding: Vec<(usize, String)> = (1..=self.data.highest_free())
            .filter_map(|area| {
                let cursor = self.data.find(&AreaRef::Number(area))?;
                self.db_table_name_in(index, &cursor.path).map(|_| (area, cursor.alias.clone()))
            })
            .collect();
        let here = self.data.current_area();
        let mut handles = Vec::new();
        for (area, alias) in holding {
            if !self.dbc_event_in(host, fb, index, "dbc_BeforeCloseTable", &[DbcArg::name(&alias)]) {
                continue;
            }
            self.data.select(&AreaRef::Number(area))?;
            handles.extend(self.data.close_current());
            self.dbc_event_in(host, fb, index, "dbc_AfterCloseTable", &[DbcArg::name(&alias)]);
        }
        if self.data.find(&AreaRef::Number(here)).is_some() {
            self.data.select(&AreaRef::Number(here))?;
        }
        Ok(handles)
    }

    /// A container has been read: it says so, and says it is now the current one.
    ///
    /// Measured: dbc_OpenData is handed the container and the three clauses OPEN DATABASE was
    /// written with, and dbc_Activate follows it. A .F. from the first raises 1705 and leaves
    /// the database unopened; a .F. from the second leaves it open but not current.
    fn database_opened(&mut self, host: &mut dyn Host, fb: &mut Fiber, flags: u16) -> Result<(), RtError> {
        use crate::ast::db_flags as df;
        let Some(index) = self.current_db else { return Ok(()) };
        let path = self.databases[index].path.clone();
        let opening = [
            DbcArg::name(&path),
            DbcArg::Flag(flags & df::EXCLUSIVE != 0),
            DbcArg::Flag(flags & df::NOUPDATE != 0),
            DbcArg::Flag(flags & df::VALIDATE != 0),
        ];
        if !self.dbc_event(host, fb, "dbc_OpenData", &opening) {
            self.current_db = None;
            self.databases.remove(index);
            return Err(Self::dbc_refused(&path));
        }
        if !self.dbc_event(host, fb, "dbc_Activate", &[DbcArg::name(&path)]) {
            self.current_db = None;
        }
        Ok(())
    }

    /// Where a table a command named actually is, when a database is the one that knows.
    ///
    /// Two shapes reach into a container, both measured against Visual FoxPro 9:
    ///
    /// `dbname!tablename` says which database to look in. The name before the `!` is a database
    /// rather than a file: if one of that name is open it is the one asked, wherever it was
    /// opened from, and otherwise `dbname.dbc` is read where the name says - which is what
    /// `dbc_to_attach` arranges before this is asked again. A container that has no table of
    /// that name answers nothing, and the part after the `!` is then opened as an ordinary file:
    /// `USE testdata!nosuch` says `nosuch.dbf` does not exist.
    ///
    /// A bare name - no folder, no extension - is looked for in the database that is current,
    /// and what the container says wins over a file of the same name sitting in the default
    /// directory: with `testdata` open, `USE products` opens the container's PRODUCTS and not
    /// the `products.dbf` next to the program. Only the current database is asked; a second one
    /// that is open but not current is not consulted. A name that carries a folder or an
    /// extension is a file and never a database's table.
    fn database_table(&self, name: &str) -> Option<(String, String)> {
        if let Some((db, table)) = name.split_once('!') {
            let stem = stem_of(db);
            let index = self.databases.iter().position(|d| d.open && d.name().eq_ignore_ascii_case(&stem))?;
            return self.databases[index].table_file(table.trim());
        }
        if name.contains(['/', '\\', '.']) {
            return None;
        }
        self.current_database()?.table_file(name)
    }

    /// The container a `dbname!tablename` has to have open before it can be resolved, when it is
    /// not open already. Reading it is what `OPEN DATABASE` does, and leaves it open but not
    /// current - measured: after `USE <path>\testdata!products` with nothing open, `ADATABASES()`
    /// counts one and `DBC()` is still empty.
    fn dbc_to_attach(&self, name: &str) -> Option<String> {
        let (db, _) = name.split_once('!')?;
        let stem = stem_of(db);
        if self.databases.iter().any(|d| d.open && d.name().eq_ignore_ascii_case(&stem)) {
            return None;
        }
        Some(self.settings.at(&with_extension(db.trim(), "dbc")))
    }

    /// Starts reading a container so that a name reaching into it can be resolved, and says what
    /// to ask the host for. It is read exactly as `OPEN DATABASE` reads one, and left open.
    fn attach_database(&mut self, path: String) -> HostRequest {
        self.db_io = Some(DbIo { stage: 0, path: path.clone(), dbf: Vec::new(), memo: Vec::new(), writing: false, attach: true });
        HostRequest::FileReadBytes { path }
    }

    /// The name the open database holds a table under, when the table is one of its own.
    ///
    /// Only a table that belongs to the database is announced: a free table, a cursor and a
    /// query's result cursor say nothing, measured. The name handed over is the one in the
    /// container - the long name - and not the file the program happened to write.
    fn db_table_name(&self, path: &str) -> Option<String> {
        self.db_table_name_in(self.current_db?, path)
    }

    /// The same, asked of one database rather than the current one.
    fn db_table_name_in(&self, index: usize, path: &str) -> Option<String> {
        let stem = stem_of(path);
        self.databases
            .get(index)?
            .objects
            .iter()
            .find(|o| o.kind.eq_ignore_ascii_case("Table") && o.name.eq_ignore_ascii_case(&stem))
            .map(|o| o.name.clone())
    }

    /// The error the product raises when a database event refuses to let something happen.
    ///
    /// Most of them refuse quietly - the command simply does nothing - but opening a table,
    /// opening the container and packing it answer 1705 naming the file, measured.
    fn dbc_refused(path: &str) -> RtError {
        RtError::new(RtError::FILE_ACCESS_DENIED, format!("File access is denied {}.", path.to_ascii_lowercase()))
    }

    /// The module a database's stored procedures were compiled into, compiling them the first
    /// time one is called.
    fn database_procedures(&mut self, index: usize) -> Option<u32> {
        if let Some(module) = self.databases.get(index).and_then(|d| d.procedures) {
            return Some(module);
        }
        let source = self.databases.get(index)?.procedure_source().to_string();
        if source.trim().is_empty() {
            return None;
        }
        let name = format!("{}_procedures", self.databases[index].name());
        let compiled = crate::compiler::compile_program(&source, &name).module?;
        let module = self.load_module(compiled);
        self.databases[index].procedures = Some(module);
        Some(module)
    }

    /// The database that is current, for the functions that report on one.
    pub fn current_database(&self) -> Option<&Database> {
        self.current_db.and_then(|i| self.databases.get(i))
    }

    /// Every database that is open, for ADATABASES().
    pub fn databases(&self) -> &[Database] {
        &self.databases
    }

    /// `PACK`: the records marked deleted go and the ones after them move up.
    ///
    /// The table is read in one go, the records that are left are written back from the top -
    /// one write per record, so this instruction runs once per record - and the last write tells
    /// the header how many there are. The index follows the records: an entry whose record moved
    /// moves with it, and one whose record went goes too.
    /// `CREATE CURSOR`, however its columns were arrived at: an empty table of that shape in a
    /// free work area, replacing a cursor of the same name as VFP replaces it.
    fn install_cursor(
        &mut self,
        fb: &mut Fiber,
        alias: String,
        fields: Vec<crate::dbf::DbfField>,
    ) -> Result<Flow, RtError> {
        if self.data.used(&AreaRef::Alias(alias.clone())) {
            self.data.select(&AreaRef::Alias(alias.clone()))?;
        } else {
            self.data.select(&AreaRef::Number(0))?;
        }
        let old = self.data.install(Cursor::in_memory(alias, fields, Vec::new()));
        Ok(self.release_handles(fb, old.into_iter().collect()))
    }

    /// One record of `INSERT INTO ... FROM ARRAY | MEMVAR | NAME`: a blank record filled from
    /// row `row` of the source. Answers whether another row follows and which it is, so the
    /// statement's loop knows whether to come round again.
    ///
    /// Only an array has rows to run out of. An array of two dimensions holds a record each,
    /// which is what Visual FoxPro puts in; one of a single dimension is a single record, and
    /// so are MEMVAR and NAME.
    fn insert_from(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        from: u8,
        source: &Value,
        row: usize,
    ) -> Result<(bool, usize), RtError> {
        let (rows, carried) = match (from, source.deref()) {
            (0, Value::Array(a)) => {
                let a = a.borrow();
                let (rows, cols) = if a.cols == 0 { (1, a.items.len()) } else { (a.rows, a.cols) };
                // this row on its own, so GATHER sees one value per field the way it does for
                // an array of a single dimension
                let start = (row - 1) * cols;
                let items = a.items.get(start..start + cols).unwrap_or_default().to_vec();
                (rows, Some(Value::Array(Rc::new(RefCell::new(FoxArray::of(items))))))
            }
            (0, _) => return Err(RtError::type_mismatch()),
            // MEMVAR reads variables and has no value to carry; NAME carries the object
            (1, _) => (1, None),
            _ => (1, Some(source.clone())),
        };
        self.data.cursor_mut().ok_or_else(no_table)?.append_blank();
        self.gather_record(host, fb, from, carried, &[], false, 1)?;
        Ok((row < rows, row + 1))
    }

    /// `INSERT [BEFORE] [BLANK]`: an empty record beside the one the pointer is on.
    ///
    /// A cursor whose rows are here simply grows one in the middle. A table in a file has to be
    /// rewritten from the insertion point down, a record per pass through here, because the
    /// host writes one record at a time; the blank goes in first and the last write carries the
    /// new count, so a reader never sees a record the header does not admit to.
    fn insert_blank(&mut self, fb: &mut Fiber, before: bool) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        if self.shift.is_none() {
            let cursor = self.data.cursor().ok_or_else(no_table)?;
            // Visual FoxPro refuses both of these: every entry of every tag points at a record
            // number, and moving the records down one would leave them all pointing at the
            // wrong record; a held record has nowhere to go while the table is being rewritten.
            if cursor.buffered() {
                return Err(RtError::new(
                    RtError::BUFFERED_TABLE,
                    "Command cannot be issued on a table with cursors in table buffering mode",
                ));
            }
            if cursor.indexed() {
                return Err(RtError::new(
                    RtError::INSERT_NOT_ALLOWED,
                    "INSERT cannot be issued when row or table buffering is enabled or when integrity constraints are in effect",
                ));
            }
            let at = cursor.insert_at(before);
            let count = cursor.count();
            let handle = cursor.handle();
            // at the end of the table there is nothing below to move, which makes the command
            // an APPEND BLANK - and that is what it is on an empty table too
            match handle {
                Some(handle) if at <= count => {
                    let blank = self.data.cursor_mut().ok_or_else(no_table)?.blank_for_insert();
                    self.shift = Some(Shift { at, blank, rows: Default::default(), next: 0 });
                    return Ok(Some(HostRequest::DataRead { handle, first: at as f64, count: (count - at + 1) as u32 }));
                }
                Some(_) => {
                    self.data.cursor_mut().ok_or_else(no_table)?.append_blank();
                    return Ok(None);
                }
                None => {
                    self.data.cursor_mut().ok_or_else(no_table)?.insert_row(at);
                    return Ok(None);
                }
            }
        }

        let cursor = self.data.cursor().ok_or_else(no_table)?;
        let record_len = cursor.header.record_len.max(1);
        let handle = cursor.handle();
        let count = cursor.count() + 1;
        let Some(state) = &mut self.shift else { return Ok(None) };
        if state.next == 0 {
            // the blank first, then the records that were at the insertion point and below it,
            // which all go one lower
            state.rows.push_back(state.blank.clone());
            let bytes = crate::data::bytes_of(&reply.unwrap_or(Value::Null));
            for chunk in bytes.chunks(record_len) {
                if chunk.len() == record_len {
                    state.rows.push_back(chunk.to_vec());
                }
            }
            state.next = state.at;
        }
        if let Some(row) = state.rows.pop_front()
            && let Some(handle) = handle
        {
            let recno = state.next as f64;
            state.next += 1;
            // the count goes with the last write, so the header grows only once the record it
            // counts is really in the file
            let last = state.rows.is_empty();
            return Ok(Some(HostRequest::DataWrite { handle, recno, bytes: row, count: last.then_some(count as f64) }));
        }

        let Shift { at, blank, .. } = self.shift.take().expect("the shift is still here");
        self.data.cursor_mut().ok_or_else(no_table)?.inserted(at, blank);
        Ok(None)
    }

    fn pack(&mut self, fb: &mut Fiber) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        let cursor = self.data.cursor().ok_or_else(no_table)?;
        let Some(handle) = cursor.handle() else {
            // a cursor whose rows are in memory has nothing on disk to compact
            return Ok(None);
        };
        let record_len = cursor.header.record_len;
        match &mut self.pack {
            // the index has been written: the command is done
            Some(state) if state.done => {
                self.pack = None;
                return Ok(None);
            }
            None => {
                let count = cursor.count() as u32;
                self.pack = Some(Pack { rows: Default::default(), moved: Vec::new(), next: 0, total: 0, done: false });
                return Ok(Some(HostRequest::DataRead { handle, first: 1.0, count: count.max(1) }));
            }
            Some(state) if state.rows.is_empty() && state.next == 0 => {
                let bytes = bytes_of(&reply.unwrap_or(Value::Null));
                for (i, chunk) in bytes.chunks(record_len.max(1)).enumerate() {
                    if chunk.len() < record_len {
                        break;
                    }
                    if chunk.first() == Some(&crate::dbf::layout::FLAG_DELETED) {
                        continue;
                    }
                    state.moved.push((i as u32 + 1, state.rows.len() as u32 + 1));
                    state.rows.push_back(chunk.to_vec());
                }
                state.total = state.rows.len() as u64;
            }
            // a record has been written; the next one follows
            Some(_) => {}
        }

        let Some(state) = &mut self.pack else { return Ok(None) };
        if let Some(row) = state.rows.pop_front() {
            state.next += 1;
            let recno = state.next as f64;
            let count = if state.rows.is_empty() { Some(state.total as f64) } else { None };
            return Ok(Some(HostRequest::DataWrite { handle, recno, bytes: row, count }));
        }

        // the table is as short as it is going to be: the cursor and its index follow
        let moved: Vec<(u32, u32)> = state.moved.drain(..).collect();
        let total = state.total;
        state.done = true;
        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        cursor.repack(total, &moved);
        let write = self.write_index(self.data.current_area());
        if write.is_none() {
            self.pack = None;
        }
        Ok(write)
    }

    // ----- records from another file -------------------------------------------------------------

    /// `APPEND FROM`: another table's records, or a text file's lines, added to the end of this
    /// one. The file is opened, read in one go, and then a record is added per pass through this
    /// instruction, because each one is a write of its own.
    fn append_from(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        except: bool,
        count: u16,
        cond: Option<String>,
        text: u8,
    ) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        // the columns of the table being appended to, which is what a text file is laid out by
        let target: Vec<(String, char, usize)> = self
            .data
            .cursor()
            .map(|c| c.header.fields.iter().map(|f| (f.name.to_ascii_uppercase(), f.kind, f.length as usize)).collect())
            .unwrap_or_default();
        match &mut self.append {
            // the file has been let go of: the command is done
            Some(state) if state.stage == 3 => {
                self.append = None;
                return Ok(None);
            }
            // the file has not been opened yet
            None => {
                let names = pop_names(fb, count)?;
                let path = fb.stack.pop().ok_or_else(|| RtError::new(0, "Stack underflow"))?;
                let named = path.as_str()?.trim().to_string();
                let search = self.settings.table_search(&named);
                let path = self.settings.table_at(&named);
                self.append = Some(AppendFrom { stage: 0, handle: 0, rows: Default::default(), names, except, cond, text });
                return Ok(Some(if text == 0 {
                    HostRequest::DataOpen { path, exclusive: false, search }
                } else {
                    HostRequest::FileRead { path, search }
                }));
            }
            Some(state) if state.stage == 0 => {
                if state.text > 0 {
                    let lines = reply.map(|v| v.as_str().unwrap_or_default().to_string()).unwrap_or_default();
                    let rows = text_rows(&lines, state.text, &target);
                    state.rows = rows.into();
                    state.stage = 2;
                } else {
                    let items = match reply.map(|v| v.deref()) {
                        Some(Value::Array(a)) => a.borrow().items.clone(),
                        other => other.into_iter().collect(),
                    };
                    let handle = items.first().and_then(|v| v.as_number().ok()).unwrap_or(-1.0);
                    if handle < 0.0 {
                        self.append = None;
                        return Err(RtError::new(RtError::FILE_NOT_FOUND, "the file to append from does not exist"));
                    }
                    let header = crate::dbf::read_header(&bytes_of(items.get(1).unwrap_or(&Value::Null)))
                        .map_err(|e| RtError::new(RtError::FILE_NOT_FOUND, e.message))?;
                    state.handle = handle as u32;
                    state.stage = 1;
                    let records = header.record_count as u32;
                    self.source_header = Some(header);
                    return Ok(Some(HostRequest::DataRead { handle: handle as u32, first: 1.0, count: records.max(1) }));
                }
            }
            // the records have arrived: they are decoded here and added one at a time
            Some(state) if state.stage == 1 => {
                let bytes = bytes_of(&reply.unwrap_or(Value::Null));
                let header = self.source_header.take().ok_or_else(|| RtError::new(0, "no header for the file"))?;
                let mut rows = Vec::new();
                for chunk in bytes.chunks(header.record_len.max(1)) {
                    if chunk.len() < header.record_len || chunk.first() == Some(&0x1A) {
                        break;
                    }
                    // a deleted record is not appended, as VFP leaves it behind too
                    if chunk.first() == Some(&crate::dbf::layout::FLAG_DELETED) {
                        continue;
                    }
                    let record = crate::dbf::decode_record(&header, chunk, crate::dbf::Padding::Keep, |_| None);
                    let row: Vec<(String, Value)> = header
                        .fields
                        .iter()
                        .zip(&record.values)
                        .map(|(f, v)| (f.name.to_ascii_uppercase(), crate::data::value_of(v)))
                        .collect();
                    rows.push(row);
                }
                state.rows = rows.into();
                state.stage = 2;
            }
            // a record has been written; the next one follows
            Some(_) => {}
        }

        // one record per pass: it is added, tested and written
        loop {
            let Some(state) = &mut self.append else { return Ok(None) };
            let Some(row) = state.rows.pop_front() else { break };
            let names = state.names.clone();
            let except = state.except;
            let cond = state.cond.clone();
            let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
            cursor.append_blank();
            for (name, value) in row {
                let wanted = if names.is_empty() {
                    true
                } else if except {
                    !names.iter().any(|n| n.eq_ignore_ascii_case(&name))
                } else {
                    names.iter().any(|n| n.eq_ignore_ascii_case(&name))
                };
                if wanted && cursor.has_field(&name) {
                    cursor.set_field(&name, &value)?;
                }
            }
            if let Some(cond) = cond {
                // the condition is written against the record, which is now in hand
                let keep = self.eval_in_frame(host, fb, &cond).map(|v| v.truthy().unwrap_or(true)).unwrap_or(true);
                if !keep {
                    if let Some(cursor) = self.data.cursor_mut() {
                        cursor.drop_appended();
                    }
                    continue;
                }
            }
            if let Some(req) = self.flush_record()? {
                return Ok(Some(req));
            }
        }

        // the source file is let go of, which is one more pass through here
        let Some(state) = &mut self.append else { return Ok(None) };
        if state.text > 0 {
            self.append = None;
            return Ok(None);
        }
        state.stage = 3;
        Ok(Some(HostRequest::DataClose { handles: vec![state.handle] }))
    }

    // ----- a table made from a table -----------------------------------------------------------

    /// Writes what `COPY TO`, `SORT TO` or `TOTAL ON` gathered: a table of its own, or a text
    /// file of one line per record.
    fn finish_copy(&mut self) -> Result<Option<HostRequest>, RtError> {
        let Some(mut copy) = self.copy.take() else { return Ok(None) };
        if copy.kind == 2 {
            // SORT: the keys decide the order, and which of them run backwards
            let descending = copy.descending;
            copy.rows.sort_by(|a, b| {
                for (i, (x, y)) in a.0.iter().zip(&b.0).enumerate() {
                    let ord = compare_keys(x, y);
                    if ord != std::cmp::Ordering::Equal {
                        return if descending & (1 << i) != 0 { ord.reverse() } else { ord };
                    }
                }
                std::cmp::Ordering::Equal
            });
        }
        if copy.kind == 3 {
            copy.rows = total_runs(&copy.fields, copy.rows);
        }
        if copy.kind == 5 {
            // CREATE ... FROM: every record read is a field of the table being made
            let fields = fields_from_description(&copy.fields, &copy.rows)?;
            let memo = fields.iter().any(|f| matches!(f.kind, 'M' | 'G' | 'P'));
            let bytes = build_table(&fields, &[], None)?;
            return Ok(Some(HostRequest::DataCreate { path: copy.path, header: bytes, memo }));
        }
        let codepage = self.data.cursor().and_then(|c| c.header.codepage);
        if copy.text > 0 {
            let text = text_file(&copy, &self.settings);
            return Ok(Some(HostRequest::FileWrite { path: copy.path, text, append: false }));
        }
        let bytes = build_table(&copy.fields, &copy.rows, codepage)?;
        let memo = copy.fields.iter().any(|f| matches!(f.kind, 'M' | 'G' | 'P'));
        Ok(Some(HostRequest::DataCreate { path: copy.path, header: bytes, memo }))
    }

    // ----- a record away from the table --------------------------------------------------------

    /// The fields SCATTER is to take, with their values: the ones named, or all of them but the
    /// ones a FIELDS ... EXCEPT clause left out. BLANK gives the shape rather than the record.
    fn record_fields(&mut self, names: &[String], except: bool, blank: bool) -> Result<Vec<(String, Value)>, RtError> {
        let cursor = self.data.cursor().ok_or_else(no_table)?;
        let wanted: Vec<String> = cursor
            .header
            .fields
            .iter()
            .map(|f| f.name.to_ascii_uppercase())
            .filter(|name| {
                if names.is_empty() {
                    true
                } else if except {
                    !names.iter().any(|n| n.eq_ignore_ascii_case(name))
                } else {
                    names.iter().any(|n| n.eq_ignore_ascii_case(name))
                }
            })
            .collect();
        let mut row = Vec::with_capacity(wanted.len());
        for name in wanted {
            let value = if blank {
                let field = cursor.header.fields.iter().find(|f| f.name.eq_ignore_ascii_case(&name));
                let kind = field.map_or('C', |f| f.kind);
                match crate::data::empty_of(kind) {
                    // a blank character field is its width in spaces, which is what a record
                    // read from the table gives too
                    Value::Str(_) => Value::str(" ".repeat(field.map_or(0, |f| f.length as usize))),
                    // and a blank numeric one still prints to the column's places
                    other => field.map_or(other.clone(), |f| crate::data::shaped_by(f, &other)),
                }
            } else {
                cursor.field(&name).unwrap_or(Value::Null)
            };
            row.push((name, value));
        }
        Ok(row)
    }

    /// `SCATTER NAME`: an object with a property per field. The object is made by the host and
    /// each property is added by a request of its own, so this instruction runs once per field
    /// and the answers come back to it one at a time.
    fn scatter_object(&mut self, fb: &mut Fiber, row: Vec<(String, Value)>) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        match &mut self.scatter_name {
            // the fields are in hand: the object to hang them on is asked for first
            None => {
                self.scatter_name = Some(ScatterName { object: None, left: row.into_iter().collect() });
                return Ok(Some(HostRequest::CreateObject {
                    class: "Empty".into(),
                    args: Vec::new(),
                    definition: None,
                    module: String::new(),
                }));
            }
            // the object has just been made
            Some(state) if state.object.is_none() => {
                let Some(Value::Object(handle)) = reply.map(|v| v.deref()) else {
                    self.scatter_name = None;
                    return Err(RtError::not_an_object("SCATTER NAME"));
                };
                state.object = Some(handle);
            }
            // a property has been added, and the next one follows
            Some(_) => {}
        }
        let Some(state) = &mut self.scatter_name else { return Ok(None) };
        let handle = state.object.expect("the object is made by now");
        if let Some((name, value)) = state.left.pop_front() {
            return Ok(Some(HostRequest::AddProperty {
                obj: handle.0,
                name,
                value: crate::host::JsonValue::from_value(&value),
            }));
        }
        self.scatter_name = None;
        fb.stack.push(Value::Object(handle));
        Ok(None)
    }

    /// `GATHER`: the values in an array, an object or a variable each, back into the record.
    fn gather_record(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        to: u8,
        source: Option<Value>,
        names: &[String],
        except: bool,
        array_row: usize,
    ) -> Result<(), RtError> {
        let row = self.record_fields(names, except, false)?;
        let mut values: Vec<(String, Value)> = Vec::with_capacity(row.len());
        for (i, (name, _)) in row.iter().enumerate() {
            let value = match (to, &source) {
                // an array fills the fields in order, and stops where it runs out. REPLACE FROM
                // ARRAY does the same a row at a time, which for an array of one dimension is
                // the whole of it and so is the same thing again.
                (0 | 3, Some(v)) => match v.deref() {
                    Value::Array(a) => {
                        let a = a.borrow();
                        let width = match a.cols {
                            0 => a.items.len(),
                            cols => cols,
                        };
                        match a.items.get((array_row - 1) * width + i) {
                            Some(item) => item.deref(),
                            None => continue,
                        }
                    }
                    _ => return Err(RtError::type_mismatch()),
                },
                (2, Some(v)) => {
                    let Value::Object(handle) = v.deref() else { return Err(RtError::not_an_object("GATHER NAME")) };
                    match host.get_prop(handle, name) {
                        Ok(value) => value,
                        // a property the object has not got leaves the field as it was
                        Err(_) => continue,
                    }
                }
                _ => match self.load_name(fb, name) {
                    Some(value) => value,
                    None => continue,
                },
            };
            values.push((name.clone(), value));
        }
        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        for (name, value) in values {
            cursor.set_field(&name, &value)?;
        }
        Ok(())
    }

    // ----- indexes ---------------------------------------------------------------------------

    /// The request that reads the compound index beside a table, when it has not been read yet.
    /// A table whose header says it has none, and a cursor that is not a file at all, are both
    /// answered by marking the index read and having nothing to ask.
    fn index_request(&mut self, area: Option<usize>) -> Option<HostRequest> {
        let cursor = match area {
            Some(n) => self.data.find_mut(&AreaRef::Number(n)),
            None => self.data.cursor_mut(),
        }?;
        if cursor.index_asked() {
            return None;
        }
        let handle = cursor.handle().filter(|_| cursor.header.has_index);
        match handle {
            Some(handle) => Some(HostRequest::DataIndex { handle }),
            None => {
                cursor.mark_index_asked();
                None
            }
        }
    }

    /// Files the index the host sent. An index that cannot be read is treated as no index: a
    /// program that opens a table should not stop because the file beside it is damaged.
    fn accept_index(&mut self, area: Option<usize>, answer: &Value) {
        let bytes = bytes_of(answer);
        let tags = if bytes.is_empty() { Vec::new() } else { crate::cdx::read_cdx(&bytes).unwrap_or_default() };
        let cursor = match area {
            Some(n) => self.data.find_mut(&AreaRef::Number(n)),
            None => self.data.cursor_mut(),
        };
        if let Some(cursor) = cursor {
            cursor.set_tags(tags);
        }
    }

    /// The request that writes a work area's index back, when a tag has changed. Only the
    /// compound index goes: a single-entry index is a file of its own.
    fn write_index(&mut self, area: usize) -> Option<HostRequest> {
        let cursor = self.data.find_mut(&AreaRef::Number(area))?;
        if !cursor.index_dirty() {
            return None;
        }
        let handle = cursor.handle()?;
        cursor.index_written();
        let bytes = if cursor.cdx_tags().is_empty() { Vec::new() } else { crate::cdx::write_cdx(cursor.cdx_tags()) };
        Some(HostRequest::DataWriteIndex { handle, bytes })
    }

    // ----- single-entry indexes --------------------------------------------------------------
    //
    // A `.idx` holds one index expression and nothing else. Opening one puts its keys in the
    // work area as a tag named after the file, in front of the compound index's tags, which is
    // the order Visual FoxPro numbers them in; from there SET ORDER, SEEK and SKIP are what
    // they always were. A file named here is read as a single-entry index whichever of the two
    // layouts it is in; a compound index is not something these commands open.
    //
    // A record written while one is open moves in it the way it moves in a tag, because the
    // maintenance walks every tag the work area holds. What does not happen yet is the file
    // being written back afterwards: it is written when the index is built or copied, and
    // REINDEX or another INDEX ON is what brings a stale one up to date.

    /// `USE ... INDEX` and `SET INDEX TO`: the files are read one at a time, and when they are
    /// all in the table takes the order the command asked for, or the first file's.
    fn begin_open_idx(
        &mut self,
        paths: Vec<String>,
        additive: bool,
        order: Value,
        descending: Option<bool>,
    ) -> Result<(), RtError> {
        let area = self.data.current_area();
        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        if !additive {
            cursor.close_idx();
        }
        self.idx_open = Some(IdxOpen { area, left: paths.into(), reading: None, first: None, order, descending });
        Ok(())
    }

    /// What `SET CLASSLIB TO` asks the host for: the files to read, and how to hold them.
    ///
    /// The arguments arrive as the parser laid them out - ADDITIVE, then the ALIAS, then the
    /// files - and each file is read from the default directory and given the `.vcx` extension
    /// a bare name carries, which is how `SET CLASSLIB TO ..\solution` names `solution.vcx`.
    fn classlib_request(&self, args: &[Value]) -> Result<HostRequest, RtError> {
        let additive = args.first().map(Value::truthy).transpose()?.unwrap_or(false);
        let alias = args.get(1).map(|v| v.as_str()).transpose()?.unwrap_or_default().trim().to_string();
        let mut files = Vec::new();
        for arg in args.iter().skip(2) {
            let name = arg.as_str()?.trim().to_string();
            if !name.is_empty() {
                files.push(self.settings.class_library_at(&name));
            }
        }
        Ok(HostRequest::LoadClassLib { files, alias, additive })
    }

    /// The bytes of the next index file to read, or nothing when they are all in and the
    /// order has been settled.
    fn next_idx_request(&mut self) -> Option<HostRequest> {
        let state = self.idx_open.as_mut()?;
        if let Some(path) = state.left.pop_front() {
            let path = with_extension(&path, "idx");
            state.reading = Some(path.clone());
            return Some(HostRequest::FileReadBytes { path });
        }
        let state = self.idx_open.take()?;
        let cursor = self.data.find_mut(&AreaRef::Number(state.area))?;
        // SET INDEX TO with nothing after it closes them and leaves the table in record order
        let Some(first) = state.first else {
            cursor.set_order(None);
            return None;
        };
        let named = state.order.as_str().map(|s| !s.trim().is_empty()).unwrap_or(true);
        // the first file the command named is the controlling one unless it said otherwise
        let which = if named { state.order } else { Value::number((first + 1) as f64) };
        let _ = self.choose_order(&which, state.descending);
        None
    }

    /// Files one of them, under the name of the file it came from.
    fn accept_idx(&mut self, answer: &Value) -> Result<(), RtError> {
        let bytes = bytes_of(answer);
        let Some(state) = self.idx_open.as_mut() else { return Ok(()) };
        let Some(path) = state.reading.take() else { return Ok(()) };
        let area = state.area;
        // a file that is not there stops the command, and nothing of it is left half open
        let mut tag = read_idx(&bytes, &path).inspect_err(|_| self.idx_open = None)?;
        tag.name = stem_of(&path).to_ascii_uppercase();
        let cursor = self.data.find_mut(&AreaRef::Number(area)).ok_or_else(no_table)?;
        let at = cursor.open_idx(path, tag);
        if let Some(state) = self.idx_open.as_mut() {
            state.first.get_or_insert(at);
        }
        Ok(())
    }

    /// `COPY INDEXES`: each file named becomes a tag of a compound index, called after the
    /// file it came from. ALL takes the ones already open instead of reading anything.
    fn begin_copy_indexes(&mut self, files: Vec<String>, all: bool, cdx: String) -> Result<(), RtError> {
        let area = self.data.current_area();
        let cursor = self.data.cursor().ok_or_else(no_table)?;
        let mut tags = Vec::new();
        if all {
            for (i, file) in cursor.idx_files().iter().enumerate() {
                let mut tag = cursor.tags()[i].clone();
                tag.name = stem_of(file).to_ascii_uppercase();
                tags.push(tag);
            }
        }
        self.copy_indexes =
            Some(CopyIndexes { area, left: files.into(), reading: None, cdx, tags, read_cdx: false });
        Ok(())
    }

    /// The next thing COPY INDEXES has to ask the host for: an index file, the compound index
    /// the tags are going into, or the write that finishes it.
    fn copy_indexes_step(&mut self) -> Result<Option<HostRequest>, RtError> {
        let Some(state) = self.copy_indexes.as_mut() else { return Ok(None) };
        if let Some(path) = state.left.pop_front() {
            let path = with_extension(&path, "idx");
            state.reading = Some(path.clone());
            return Ok(Some(HostRequest::FileReadBytes { path }));
        }
        if !state.cdx.is_empty() && !state.read_cdx {
            return Ok(Some(HostRequest::FileReadBytes { path: with_extension(&state.cdx, "cdx") }));
        }
        let state = self.copy_indexes.take().expect("the state is there");
        if state.cdx.is_empty() {
            // no TO: the tags join the structural compound index beside the table
            let cursor = self.data.find_mut(&AreaRef::Number(state.area)).ok_or_else(no_table)?;
            for tag in state.tags {
                cursor.put_tag(tag);
            }
            return Ok(self.write_index(state.area));
        }
        let bytes = crate::cdx::write_cdx(&state.tags);
        Ok(Some(HostRequest::FileWriteBytes { path: with_extension(&state.cdx, "cdx"), bytes }))
    }

    /// The bytes COPY INDEXES asked for: an index file it turns into a tag, or the compound
    /// index the tags are being added to, whose own tags stay unless a new one has their name.
    fn copy_indexes_reply(&mut self, answer: &Value) -> Result<(), RtError> {
        let bytes = bytes_of(answer);
        let Some(state) = self.copy_indexes.as_mut() else { return Ok(()) };
        if let Some(path) = state.reading.take() {
            let mut tag = read_idx(&bytes, &path).inspect_err(|_| self.copy_indexes = None)?;
            tag.name = stem_of(&path).to_ascii_uppercase();
            self.copy_indexes.as_mut().expect("the state is there").tags.push(tag);
            return Ok(());
        }
        if !state.cdx.is_empty() && !state.read_cdx {
            state.read_cdx = true;
            let mut kept = if bytes.is_empty() { Vec::new() } else { crate::cdx::read_cdx(&bytes).unwrap_or_default() };
            for tag in state.tags.drain(..) {
                match kept.iter().position(|t| t.name.eq_ignore_ascii_case(&tag.name)) {
                    Some(i) => kept[i] = tag,
                    None => kept.push(tag),
                }
            }
            state.tags = kept;
        }
        Ok(())
    }

    /// `COPY TAG`: the tag written out as a single-entry index of its own. Visual FoxPro writes
    /// the compact kind, and an ascending one even from a descending tag, so this does too.
    fn copy_tag_step(&mut self) -> Result<Option<HostRequest>, RtError> {
        let Some(state) = self.copy_tag.as_mut() else { return Ok(None) };
        if !state.of.is_empty() && !state.read {
            return Ok(Some(HostRequest::FileReadBytes { path: with_extension(&state.of, "cdx") }));
        }
        let state = self.copy_tag.take().expect("the state is there");
        let tag = match state.found {
            Some(tag) => tag,
            None => {
                let cursor = self.data.cursor().ok_or_else(no_table)?;
                cursor
                    .tags()
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(&state.name))
                    .cloned()
                    .ok_or_else(RtError::tag_not_found)?
            }
        };
        let bytes = crate::idx::write(&crate::cdx::Tag { descending: false, ..tag }, true);
        Ok(Some(HostRequest::FileWriteBytes { path: with_extension(&state.target, "idx"), bytes }))
    }

    /// The compound index `COPY TAG ... OF` named, and the tag looked up in it.
    fn copy_tag_reply(&mut self, answer: &Value) -> Result<(), RtError> {
        let bytes = bytes_of(answer);
        let Some(state) = self.copy_tag.as_mut() else { return Ok(()) };
        if state.read {
            return Ok(());
        }
        state.read = true;
        let tags = if bytes.is_empty() { Vec::new() } else { crate::cdx::read_cdx(&bytes).unwrap_or_default() };
        state.found = tags.into_iter().find(|t| t.name.eq_ignore_ascii_case(&state.name));
        if state.found.is_none() {
            self.copy_tag = None;
            return Err(RtError::tag_not_found());
        }
        Ok(())
    }

    /// `SET ORDER TO`: which tag decides what the next record is, and which way it runs.
    fn choose_order(&mut self, which: &Value, descending: Option<bool>) -> Result<(), RtError> {
        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        let chosen = match which.deref() {
            Value::Logical(false) | Value::Null => None,
            Value::Number(n, ..) => {
                let n = n as i64;
                if n <= 0 {
                    None
                } else if n as usize > cursor.tags().len() {
                    return Err(RtError::tag_not_found());
                } else {
                    Some(n as usize - 1)
                }
            }
            Value::Str(s) => {
                let name = s.trim().to_string();
                if name.is_empty() {
                    None
                } else {
                    Some(
                        cursor
                            .tag_index(&name)
                            .ok_or_else(RtError::tag_not_found)?,
                    )
                }
            }
            _ => return Err(RtError::type_mismatch()),
        };
        cursor.set_order_way(chosen, descending.filter(|_| chosen.is_some()));
        // VFP leaves the pointer on the first record of the new order
        if chosen.is_some() {
            let first = cursor.record_at(1).unwrap_or(cursor.count() + 1);
            cursor.seek(first);
        }
        Ok(())
    }

    /// `SEEK`: to the first record the controlling order holds under a key. Answers whether it
    /// was there; SET NEAR decides where the pointer lands when it was not.
    fn seek_key(&mut self, key: &Value) -> Result<bool, RtError> {
        let near = self.settings.near;
        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        crate::data::seek_in_order(cursor, key, near)
    }

    /// `INDEX ON`: the keys are gathered, so the tag can be sorted, put in the index and written.
    fn finish_index(&mut self) -> Result<Option<HostRequest>, RtError> {
        let Some(build) = self.index_build.take() else { return Ok(None) };
        // every record of a table gives a key of the same type, so the first one decides the
        // shape of the whole tag: eight bytes for a number or a date, one for a logical, and
        // for text as many characters as the longest key needs
        let sample = build.keys.first().map(|(v, _)| v.deref()).unwrap_or(Value::str(""));
        let numeric = matches!(sample, Value::Number(..) | Value::Date(_) | Value::DateTime(_));
        let key_len = if numeric {
            8
        } else if matches!(sample, Value::Logical(_)) {
            1
        } else {
            build.keys.iter().filter_map(|(v, _)| v.as_str().ok().map(|s| s.chars().count())).max().unwrap_or(1).clamp(1, 240)
        };
        let mut entries: Vec<crate::cdx::Entry> = Vec::with_capacity(build.keys.len());
        for (value, recno) in &build.keys {
            entries.push(crate::cdx::Entry { key: index_key_padded(value, numeric, key_len)?, recno: *recno });
        }
        entries.sort_by(|a, b| (&a.key, a.recno).cmp(&(&b.key, b.recno)));
        if build.unique {
            entries.dedup_by(|a, b| a.key == b.key);
        }
        let tag = crate::cdx::Tag {
            name: build.name,
            key_expr: build.key_expr,
            for_expr: build.for_expr,
            key_len,
            descending: build.descending,
            unique: build.unique,
            candidate: build.candidate,
            numeric,
            entries,
        };
        let cursor = self.data.find_mut(&AreaRef::Number(build.area)).ok_or_else(no_table)?;
        // `TO file` puts the index in a file of its own, opens it beside the table and writes
        // it; `TAG name` puts it in the compound index
        let (at, write) = match &build.to_file {
            Some(path) => {
                let bytes = crate::idx::write(&tag, build.compact);
                let at = cursor.open_idx(path.clone(), tag);
                (at, Some(HostRequest::FileWriteBytes { path: path.clone(), bytes }))
            }
            None => (cursor.put_tag(tag), None),
        };
        // the index just built is the controlling order, which is where VFP leaves it
        cursor.set_order(Some(at));
        let first = cursor.record_at(1).unwrap_or(cursor.count() + 1);
        cursor.seek(first);
        Ok(write.or_else(|| self.write_index(build.area)))
    }

    /// Puts the record that was just written back in its place in every tag of its table.
    ///
    /// The key expression is evaluated where the program stands, which is what makes a key like
    /// `UPPER(company)` mean the record in hand. A tag whose expression cannot be evaluated -
    /// one that reads a memo not fetched yet, say - is left alone rather than made wrong, and
    /// REINDEX puts it right.
    fn maintain_index(&mut self, host: &mut dyn Host, fb: &mut Fiber) {
        let Some(cursor) = self.data.cursor() else { return };
        if cursor.tags().is_empty() || !cursor.dirty() {
            return;
        }
        let area = self.data.current_area();
        let recno = cursor.recno() as u32;
        let specs: Vec<(usize, String, String, bool, usize)> = cursor
            .tags()
            .iter()
            .enumerate()
            .map(|(i, t)| (i, t.key_expr.clone(), t.for_expr.clone(), t.numeric, t.key_len))
            .collect();
        for (i, key_expr, for_expr, numeric, key_len) in specs {
            let keep = if for_expr.is_empty() {
                true
            } else {
                match self.eval_in_frame(host, fb, &for_expr) {
                    Ok(v) => v.truthy().unwrap_or(true),
                    Err(_) => continue,
                }
            };
            let key = if keep {
                match self.eval_in_frame(host, fb, &key_expr).and_then(|v| index_key_padded(&v, numeric, key_len)) {
                    Ok(key) => Some(key),
                    Err(_) => continue,
                }
            } else {
                None
            };
            if let Some(cursor) = self.data.find_mut(&AreaRef::Number(area)) {
                cursor.reindex_record(i, recno, key);
            }
        }
    }

    /// Sends the changed record of the selected work area back to the host.
    ///
    /// A cursor whose rows are in memory has nowhere to send them, and a record nobody changed
    /// costs nothing: both simply do not write.
    fn flush_record(&mut self) -> Result<Option<HostRequest>, RtError> {
        let Some(cursor) = self.data.cursor_mut() else { return Ok(None) };
        if !cursor.dirty() {
            return Ok(None);
        }
        let recno = cursor.recno();
        let count = cursor.grew().map(|c| c as f64);
        let handle = cursor.handle();
        let bytes = cursor.current_bytes();
        cursor.written();
        match (handle, bytes) {
            (Some(handle), Some(bytes)) => Ok(Some(HostRequest::DataWrite { handle, recno: recno as f64, bytes, count })),
            _ => Ok(None),
        }
    }

    /// The header back to the file, when what it says has changed.
    ///
    /// What an autoincrementing field takes next is kept in the table itself, so a table that
    /// is closed and opened again carries on counting where it left off.
    fn flush_header(&mut self) -> Option<HostRequest> {
        let cursor = self.data.cursor_mut()?;
        if !cursor.take_header_changed() {
            return None;
        }
        let handle = cursor.handle()?;
        let (bytes, _) = crate::dbf::write::encode_header(&cursor.header.fields);
        Some(HostRequest::DataWriteHeader { handle, bytes })
    }

    // ----- SELECT-SQL ------------------------------------------------------------------------

    /// Puts one FROM source in a work area and selects it.
    ///
    /// A table the program already has open is borrowed rather than opened again - which is what
    /// Visual FoxPro does, and what keeps two cursors from answering to one alias - and its
    /// record pointer is put back where it was when the query finishes.
    fn sql_open(&mut self, fb: &mut Fiber, named: &str, alias: &str) -> Result<Option<HostRequest>, RtError> {
        // where the statement stands, noted before the first source moves the selection; an
        // open that has to wait comes back through here and must not note it twice
        if fb.query.is_none() && fb.pending_sources.is_empty() && fb.pending_area.is_none() {
            fb.pending_area = Some(self.data.current_area());
        }
        // a FROM that reaches into a container needs the container open first, as USE does
        if self.db_io.is_some() {
            let reply = fb.data_reply.take();
            if let Some(request) = self.db_io_step(reply)? {
                return Ok(Some(request));
            }
        } else if fb.data_reply.is_none()
            && let Some(container) = self.dbc_to_attach(named)
        {
            return Ok(Some(self.attach_database(container)));
        }
        // `SELECT ... FROM orders` with a database open reads the container's ORDERS, wherever
        // its file is and whatever sits in the default directory under that name - measured
        let resolved = self.database_table(named);
        let plain = plain_table(named.to_string());
        let path = resolved.as_ref().map(|(file, _)| file.as_str()).unwrap_or(&plain);
        if let Some(reply) = fb.data_reply.take() {
            self.data.select(&AreaRef::Number(0))?;
            self.open_table(fb, path, Some(alias.to_string()), reply)?;
            let area = self.data.current_area();
            self.sql_source(fb, alias, area, None);
            return Ok(None);
        }

        if let Some(cursor) = self.data.find(&AreaRef::Alias(alias.to_string())) {
            let recno = cursor.recno();
            let area = self.data.select(&AreaRef::Alias(alias.to_string()))?;
            self.sql_source(fb, alias, area, Some(recno));
            return Ok(None);
        }
        Ok(Some(HostRequest::DataOpen {
            path: self.settings.table_at(path),
            exclusive: false,
            search: self.settings.table_search(path),
        }))
    }

    /// Notes a source the query is using, so its record pointer can be put back afterwards. The
    /// list waits on the fiber until `SqlBegin` builds the run, because opening is what comes
    /// first.
    fn sql_source(&mut self, fb: &mut Fiber, alias: &str, area: usize, restore: Option<u64>) {
        let state = QuerySourceState { alias: alias.to_ascii_uppercase(), area, restore };
        match &mut fb.query {
            Some(run) => run.sources.push(state),
            None => fb.pending_sources.push(state),
        }
    }

    fn sql_begin(&mut self, fb: &mut Fiber, plan: QueryPlan, top: Option<f64>, into: Option<String>) -> Result<(), RtError> {
        let sources = std::mem::take(&mut fb.pending_sources);

        // `*` is expanded now, while the sources are open: one result column per field, named as
        // the table names it. An empty result still has its columns this way.
        let mut columns: Vec<String> = Vec::new();
        let mut widths = Vec::new();
        for column in &plan.columns {
            match column {
                PlanColumn::Star(alias) => {
                    let before = columns.len();
                    for source in &sources {
                        if alias.as_ref().is_some_and(|a| !a.eq_ignore_ascii_case(&source.alias)) {
                            continue;
                        }
                        let cursor = self.data.find(&AreaRef::Number(source.area)).ok_or_else(|| {
                            RtError::alias_not_found(&source.alias)
                        })?;
                        columns.extend(cursor.header.fields.iter().map(|f| f.name.to_ascii_uppercase()));
                    }
                    widths.push(columns.len() - before);
                }
                PlanColumn::Value { name } | PlanColumn::Agg { name, .. } => {
                    columns.push(name.clone());
                    widths.push(1);
                }
            }
        }

        // What TOP takes is checked before a row is read, which is where the product checks it:
        // an out-of-range count is the error whether or not the query would have found anything.
        if let Some(n) = top {
            let ok = if plan.top_percent { n > 0.0 && n < 100.0 } else { n >= 1.0 };
            if !ok {
                return Err(RtError::about(RtError::TOP_INVALID, ""));
            }
            if plan.order_by.is_empty() {
                return Err(RtError::about(RtError::TOP_NEEDS_ORDER, ""));
            }
        }

        // opening the sources selected the last of them; the program stood where the first
        // open found it, and that is where INTO ARRAY leaves it again
        let saved_area = fb.pending_area.take().unwrap_or_else(|| self.data.current_area());
        fb.query = Some(QueryRun {
            plan: std::rc::Rc::new(plan),
            columns,
            widths,
            rows: Vec::new(),
            folded: false,
            having: None,
            top,
            sources,
            saved_area,
            frame: fb.frames.len().saturating_sub(1),
            into_alias: into,
        });
        Ok(())
    }

    /// One gathered row: the pushed values, with every `*` filled in from the record each source
    /// is sitting on right now.
    fn sql_row(&mut self, fb: &mut Fiber, pushed: Vec<Value>) -> Result<(), RtError> {
        let Some(run) = fb.query.as_ref() else {
            return Err(RtError::new(RtError::SYNTAX_ERROR, "a query row was gathered outside a query"));
        };
        let plan = run.plan.clone();
        let sources: Vec<(String, usize)> = run.sources.iter().map(|s| (s.alias.clone(), s.area)).collect();

        let mut row = Vec::with_capacity(run.columns.len() + pushed.len());
        let mut next = pushed.into_iter();
        for column in &plan.columns {
            match column {
                PlanColumn::Star(alias) => {
                    for (source_alias, area) in &sources {
                        if alias.as_ref().is_some_and(|a| !a.eq_ignore_ascii_case(source_alias)) {
                            continue;
                        }
                        let names: Vec<String> = self
                            .data
                            .find(&AreaRef::Number(*area))
                            .map(|c| c.header.fields.iter().map(|f| f.name.clone()).collect())
                            .unwrap_or_default();
                        for name in names {
                            let value = self
                                .data
                                .find(&AreaRef::Number(*area))
                                .and_then(|c| c.field(&name))
                                .unwrap_or(Value::Null);
                            row.push(value);
                        }
                    }
                }
                PlanColumn::Value { .. } => row.push(next.next().unwrap_or(Value::Null)),
                PlanColumn::Agg { kind, .. } => {
                    if kind.takes_value() {
                        row.push(next.next().unwrap_or(Value::Null));
                    } else {
                        row.push(Value::number(1.0));
                    }
                }
            }
        }
        // whatever is left is the GROUP BY keys and then the ORDER BY keys
        row.extend(next);

        fb.query.as_mut().expect("query").rows.push(row);
        Ok(())
    }

    /// Puts the work areas a query borrowed back the way it found them: what it only used is
    /// left on the record it was on, the cursors its subqueries filled go, and the area the
    /// program had selected is selected again. The file handles to hand back to the host come
    /// out, because handing them back can need a round trip.
    ///
    /// A table the query opened for itself is not closed. Measured: after `SELECT COUNT(*) FROM
    /// customer, employee INTO ARRAY a` with neither open beforehand, `USED("customer")` and
    /// `USED("employee")` are both true and each answers its own file.
    fn release_query_sources(
        &mut self,
        sources: Vec<(usize, Option<u64>)>,
        temporaries: &[String],
        saved_area: usize,
    ) -> Result<Vec<u32>, RtError> {
        let mut to_close = Vec::new();
        for (area, restore) in sources {
            if let Some(recno) = restore
                && let Some(cursor) = self.data.find_mut(&AreaRef::Number(area))
            {
                cursor.seek(recno);
            }
        }
        // the cursors the subqueries filled have done their work
        for alias in temporaries {
            if self.data.used(&AreaRef::Alias(alias.clone())) {
                self.data.select(&AreaRef::Alias(alias.clone()))?;
                if let Some(handle) = self.data.close_current() {
                    to_close.push(handle);
                }
            }
        }
        self.data.select(&AreaRef::Number(saved_area))?;
        Ok(to_close)
    }

    /// A query an error escaped: nothing is installed, but the sources go back all the same.
    fn unwind_query(&mut self, fb: &mut Fiber) -> Result<Flow, RtError> {
        let Some(run) = fb.query_unwind.take() else { return Ok(Flow::Next) };
        let sources: Vec<(usize, Option<u64>)> = run.sources.iter().map(|s| (s.area, s.restore)).collect();
        let to_close = self.release_query_sources(sources, &run.plan.temporaries, run.saved_area)?;
        Ok(self.release_handles(fb, to_close))
    }

    /// Folds the gathered rows into the result, installs it, and puts the sources back.
    fn sql_end(&mut self, fb: &mut Fiber) -> Result<Flow, RtError> {
        let Some(run) = fb.query.take() else {
            return Err(RtError::new(RtError::SYNTAX_ERROR, "a query ended without starting"));
        };
        let plan = run.plan.clone();
        let saved_area = run.saved_area;
        let into_alias = run.into_alias.clone();
        let sources: Vec<(usize, Option<u64>)> = run.sources.iter().map(|s| (s.area, s.restore)).collect();
        let (fields, rows) = run.finish(&self.settings)?;

        // let the sources go before the result lands, so a query INTO CURSOR named after one of
        // them takes its place rather than fighting it for a work area
        let mut to_close = self.release_query_sources(sources, &plan.temporaries, saved_area)?;

        match &plan.into {
            PlanInto::Cursor(written) => {
                let name = &into_alias.unwrap_or_else(|| written.clone());
                // A cursor of that name is replaced; a table of that name is not. Measured: a
                // second `INTO CURSOR cTmp` takes the first one's place, and so does one over a
                // `CREATE CURSOR` of the same name, while a table open under the name - the
                // query's own source among them - raises "Alias name is already in use.".
                let taken = self.data.find(&AreaRef::Alias(name.clone()));
                if let Some(cursor) = taken {
                    if cursor.handle().is_some() {
                        return Err(RtError::new(RtError::ALIAS_IN_USE, "Alias name is already in use."));
                    }
                    self.data.select(&AreaRef::Alias(name.clone()))?;
                } else {
                    self.data.select(&AreaRef::Number(0))?;
                }
                if let Some(old) = self.data.install(Cursor::in_memory(name.clone(), fields, rows)) {
                    to_close.push(old);
                }
            }
            PlanInto::Array => {
                let cols = fields.len();
                let mut array = FoxArray::new(rows.len().max(1), if cols > 1 { cols } else { 0 });
                for (r, record) in rows.iter().enumerate() {
                    for (c, value) in record.values.iter().enumerate() {
                        let at = if cols > 1 { r * cols + c } else { r };
                        if let Some(slot) = array.items.get_mut(at) {
                            *slot = crate::data::value_of(value);
                        }
                    }
                }
                // the code after the query stores it: INTO ARRAY may name a property
                fb.stack.push(Value::Array(Rc::new(RefCell::new(array))));
            }
        }

        Ok(self.release_handles(fb, to_close))
    }

    /// Installs the table the host just opened. The reply is `[handle, header bytes]`.
    fn open_table(&mut self, fb: &mut Fiber, path: &str, alias: Option<String>, reply: Value) -> Result<(), RtError> {
        let items = match reply.deref() {
            Value::Array(a) => a.borrow().items.clone(),
            other => vec![other],
        };
        let handle = items.first().and_then(|v| v.as_number().ok()).unwrap_or(-1.0);
        let header_bytes = items.get(1).map(bytes_of).unwrap_or_default();
        if handle < 0.0 {
            return Err(RtError::new(RtError::FILE_NOT_FOUND, format!("File '{path}' does not exist")));
        }
        let header = crate::dbf::read_header(&header_bytes)
            .map_err(|e| RtError::new(RtError::FILE_NOT_FOUND, format!("{path}: {}", e.message)))?;

        let alias = alias.unwrap_or_else(|| default_alias(path));
        if let Some(old) = self.data.install(Cursor::new(handle as u32, alias, path.to_string(), header)) {
            self.release_handles(fb, vec![old]);
        }
        self.last_opened = Some(self.data.current_area());
        Ok(())
    }

    /// `USE` with no table, `CLOSE TABLES`, `CLOSE ALL`.
    /// `USE` with nothing after it and `CLOSE TABLES`: the work areas go, and the open database
    /// is told about each of its own tables either side of the closing.
    ///
    /// A table that does not belong to the database says nothing, and a dbc_BeforeCloseTable
    /// that answers .F. leaves that one table open while the others still close - measured. The
    /// arguments are the aliases, not the table names: `USE shortf.dbf ALIAS TheAlias` closes
    /// as "thealias".
    fn close_tables(&mut self, host: &mut dyn Host, fb: &mut Fiber, all: bool) -> Result<Flow, RtError> {
        let areas: Vec<usize> = if all {
            (1..=self.data.highest_free()).filter(|n| self.data.find(&AreaRef::Number(*n)).is_some()).collect()
        } else {
            self.data.cursor().map(|_| vec![self.data.current_area()]).unwrap_or_default()
        };
        // what each area holds, before anything is closed or any procedure gets to look
        let holding: Vec<(usize, String, bool)> = areas
            .iter()
            .filter_map(|area| {
                let cursor = self.data.find(&AreaRef::Number(*area))?;
                let (alias, path) = (cursor.alias.clone(), cursor.path.clone());
                Some((*area, alias, self.db_table_name(&path).is_some()))
            })
            .collect();
        let mut refused = Vec::new();
        let mut closing = Vec::new();
        for (area, alias, owned) in &holding {
            if *owned && !self.dbc_event(host, fb, "dbc_BeforeCloseTable", &[DbcArg::name(alias)]) {
                refused.push(*area);
            } else {
                closing.push((*area, alias.clone(), *owned));
            }
        }
        let flow = if refused.is_empty() {
            self.close_areas(fb, all)?
        } else {
            let here = self.data.current_area();
            let mut handles = Vec::new();
            for (area, _, _) in &closing {
                self.data.select(&AreaRef::Number(*area))?;
                handles.extend(self.data.close_current());
            }
            if self.data.find(&AreaRef::Number(here)).is_some() {
                self.data.select(&AreaRef::Number(here))?;
            }
            self.release_handles(fb, handles)
        };
        for (_, alias, owned) in &closing {
            if *owned {
                self.dbc_event(host, fb, "dbc_AfterCloseTable", &[DbcArg::name(alias)]);
            }
        }
        Ok(flow)
    }

    fn close_areas(&mut self, fb: &mut Fiber, all: bool) -> Result<Flow, RtError> {
        let handles = if all { self.data.close_all() } else { self.data.close_current().into_iter().collect() };
        Ok(self.release_handles(fb, handles))
    }

    /// Tells the host to let go of the files. They travel together, so closing four work areas
    /// is one round trip and none of them is forgotten.
    fn release_handles(&mut self, fb: &mut Fiber, handles: Vec<u32>) -> Flow {
        if handles.is_empty() {
            return Flow::Next;
        }
        fb.pending = Some(Pending::Discard);
        Flow::Suspend(HostRequest::DataClose { handles })
    }

    /// Moves the record pointer of the selected work area.
    ///
    /// With nothing to skip over, nothing is read: the record is decoded when a field is asked
    /// for, so a SKIP that nobody looks at costs nothing. A controlling order, SET DELETED and
    /// SET FILTER each mean records have to be looked at on the way, which means pages, which
    /// means this can stop part-way, ask for one and be resumed where it left off.
    ///
    /// A controlling order turns the table into the list of records the tag holds: TOP is its
    /// first entry, SKIP is a step along it, and a record the tag does not hold - one a FOR
    /// condition left out - is not reached by walking. GO to a record number is that record
    /// whatever the order, which is what makes `GO RECNO()` put the pointer back where it was.
    fn move_pointer(&mut self, host: &mut dyn Host, fb: &mut Fiber, to: Move) -> Result<Option<HostRequest>, RtError> {
        let reply = fb.data_reply.take();
        let hide = self.settings.deleted;
        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        // moving the pointer is what ends an outer join's miss, whatever moved it
        cursor.clear_outer_miss();
        if let Some(page) = reply {
            cursor.accept_page(bytes_of(&page));
        }
        let filter = cursor.filter().to_string();
        let ordered = cursor.ordered() && !matches!(to, Move::Record(_));
        // SET KEY narrows a table to the records whose key matches, and needs an order to do it
        let key_limit = ordered.then(|| cursor.key_limit().cloned()).flatten();
        let key_expr = key_limit.as_ref().and_then(|_| cursor.order().map(|t| t.key_expr.clone())).unwrap_or_default();
        if matches!(to, Move::Record(_)) || (!hide && !ordered && filter.is_empty() && key_limit.is_none()) {
            let target = cursor.target(to);
            cursor.seek(target);
            return self.follow_relations(host, fb);
        }

        let last = if ordered { cursor.order_len() as i64 } else { cursor.count() as i64 };
        let here = if ordered {
            match cursor.position_of(cursor.recno()) {
                Some(pos) => pos as i64,
                // before the first record is position 0, and past the last is one past the end
                None if cursor.recno() == 0 => 0,
                None => last + 1,
            }
        } else {
            cursor.recno() as i64
        };
        let mut walk = cursor.take_walk().unwrap_or(match to {
            Move::Top => Walk { at: 0, remaining: 1, step: 1 },
            Move::Bottom => Walk { at: last + 1, remaining: -1, step: -1 },
            Move::Skip(delta) => Walk { at: here, remaining: delta, step: delta.signum() },
            Move::Record(n) => Walk { at: n as i64, remaining: 0, step: 0 },
        });

        while walk.step != 0 {
            if walk.remaining != 0 {
                walk.at += walk.step;
                walk.remaining -= walk.step;
            }
            // cross the records that are not to be seen: deleted ones with SET DELETED ON, and
            // the ones a filter turns away. Each page they span costs one ask of the host.
            while walk.at >= 1 && walk.at <= last {
                let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
                let recno = if ordered { cursor.record_at(walk.at as usize).unwrap_or(0) } else { walk.at as u64 };
                match cursor.deleted_at(recno) {
                    Some(true) if hide => {
                        walk.at += walk.step;
                        continue;
                    }
                    Some(_) => {}
                    None => {
                        // a query result has every row in hand, so this can only be a file
                        let Some(handle) = cursor.handle() else { break };
                        let first = cursor.page_start(recno);
                        cursor.expect_page(first);
                        cursor.set_walk(walk);
                        return Ok(Some(HostRequest::DataRead { handle, first: first as f64, count: PAGE_RECORDS }));
                    }
                }
                if filter.is_empty() && key_limit.is_none() {
                    break;
                }
                // both tests are written against the record, so the pointer goes there for them
                cursor.seek(recno);
                let mut wanted = true;
                if let Some(limit) = &key_limit {
                    let key = self.eval_in_frame(host, fb, &key_expr).unwrap_or(Value::Null);
                    let same = crate::value::compare(&key, limit, crate::value::CmpOp::Eq, &self.settings);
                    wanted = same.map(|v| v.truthy().unwrap_or(false)).unwrap_or(false);
                }
                if wanted && !filter.is_empty() {
                    wanted = self.passes_filter(host, fb, &filter)?;
                }
                if wanted {
                    break;
                }
                walk.at += walk.step;
            }
            if walk.at < 1 || walk.at > last || walk.remaining == 0 {
                break;
            }
        }

        let cursor = self.data.cursor_mut().ok_or_else(no_table)?;
        let at = walk.at.clamp(0, last + 1);
        let recno = if ordered {
            match cursor.record_at(at as usize) {
                Some(recno) => recno,
                // off the end of the order is end of file, and off the front is before the first
                None if at < 1 => 0,
                None => cursor.count() + 1,
            }
        } else {
            at as u64
        };
        cursor.seek(recno);
        self.follow_relations(host, fb)
    }

    /// Whether the record the pointer is on passes the filter. A condition that cannot be
    /// evaluated lets the record through: a filter nobody can read should not empty the table.
    fn passes_filter(&mut self, host: &mut dyn Host, fb: &mut Fiber, filter: &str) -> Result<bool, RtError> {
        match self.eval_in_frame(host, fb, filter) {
            Ok(v) => Ok(v.truthy().unwrap_or(true)),
            Err(_) => Ok(true),
        }
    }

    /// Moves the work areas this one is related to, now that its own pointer has landed.
    ///
    /// The expression is evaluated against the parent's record and looked up in the child: down
    /// its controlling order when it has one, and as a record number when it has not, which is
    /// what `SET RELATION TO RECNO() INTO x` means.
    fn follow_relations(&mut self, host: &mut dyn Host, fb: &mut Fiber) -> Result<Option<HostRequest>, RtError> {
        let Some(cursor) = self.data.cursor() else { return Ok(None) };
        if cursor.relations().is_empty() {
            return Ok(None);
        }
        let here = self.data.current_area();
        let empty = cursor.recno() == 0 || cursor.recno() > cursor.count();
        let relations: Vec<crate::data::Relation> = cursor.relations().to_vec();
        let near = self.settings.near;
        for relation in relations {
            let key = if empty { None } else { self.eval_in_frame(host, fb, &relation.expr).ok() };
            let target = AreaRef::Alias(relation.into.clone());
            let Some(child) = self.data.find_mut(&target) else { continue };
            match key {
                // the parent is not on a record, so neither is the child
                None => {
                    let past = child.count() + 1;
                    child.seek(past);
                    child.set_found(false);
                }
                Some(key) if child.ordered() => {
                    let found = crate::data::seek_in_order(child, &key, near)?;
                    child.set_found(found);
                }
                Some(key) => {
                    let recno = key.as_number().unwrap_or(0.0) as u64;
                    let landed = recno >= 1 && recno <= child.count();
                    child.seek(if landed { recno } else { child.count() + 1 });
                    child.set_found(landed);
                }
            }
        }
        self.data.select(&AreaRef::Number(here))?;
        Ok(None)
    }

    /// Reads a field, asking the host for the page it lives on when that page is not in hand.
    /// What a memo field is left holding. The alias is empty for the table in hand.
    fn write_memo(&mut self, alias: &str, field: &str, text: &str) -> Result<(), RtError> {
        let cursor = if alias.is_empty() {
            self.data.cursor_mut()
        } else {
            self.data.find_mut(&AreaRef::Alias(alias.to_string()))
        };
        cursor.ok_or_else(no_table)?.set_field(field, &Value::str(text))?;
        Ok(())
    }

    /// The variables `SAVE TO` puts in a file: the ones a program can see from where it is,
    /// under the names it knows them by, and only those the skeleton keeps.
    fn saved_variables(&self, fb: &Fiber, skeleton: &str, except: bool) -> Vec<(String, Value)> {
        let mut out: Vec<(String, Value)> = self.globals.iter().map(|(n, v)| (n.clone(), v.deref())).collect();
        for frame in &fb.frames {
            for (name, value) in &frame.privates {
                out.push((name.clone(), value.deref()));
            }
        }
        // a local belongs to the function it is in, so only the one being run has any
        if let Some(frame) = fb.frames.last()
            && let Some(func) = self.modules.get(frame.module as usize).and_then(|m| m.funcs.get(frame.func as usize))
        {
            for (slot, name) in func.locals.iter().enumerate() {
                if let Some(value) = frame.locals.get(slot).filter(|_| !name.is_empty()) {
                    out.push((name.clone(), value.deref()));
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out.dedup_by(|a, b| a.0 == b.0);
        // a variable's name is the same name however it was written, so the skeleton matches
        // without regard to case
        let skeleton = skeleton.to_ascii_uppercase();
        out.retain(|(name, _)| {
            skeleton.is_empty() || crate::builtins::string::matches_pattern(skeleton.as_bytes(), name.as_bytes()) != except
        });
        out
    }

    /// The source of the query being run that has a field of this name, if one of them does.
    /// Visual FoxPro lets a query name a column without saying which table it came from as long
    /// as only one of them has it; the first that does is that one.
    fn query_field(&self, fb: &Fiber, field: &str) -> Option<String> {
        let run = fb.query.as_ref()?;
        run.sources
            .iter()
            .find(|source| {
                self.data.find(&AreaRef::Number(source.area)).is_some_and(|cursor| cursor.has_field(field))
            })
            .map(|source| source.alias.clone())
    }

    /// What `REPLACE ... ADDITIVE` is really writing: the memo as it stands with the new text on
    /// the end of it.
    ///
    /// Measured in Visual FoxPro 9. ADDITIVE says nothing about any field that is not a memo -
    /// a character, numeric, logical or date field is overwritten as it would be without the
    /// word, and no error is raised - so this answers with the value it was given unless the
    /// field is a memo. The join is plain concatenation: a memo holding "xy   " with "abc"
    /// added to it reads "xy   abc", with neither side trimmed. A value that is not character
    /// goes through untouched, so that `REPLACE m WITH 5 ADDITIVE` raises the data type
    /// mismatch the write itself raises rather than a complaint of this function's own.
    ///
    /// Reading the memo can take a round trip to the host, which is why this can suspend: the
    /// instruction has left its operands on the stack and runs again with the answer in hand.
    fn added_to_memo(&mut self, fb: &mut Fiber, field: &str, value: Value, additive: bool) -> Result<Added, RtError> {
        let memo = self
            .data
            .cursor()
            .and_then(|c| c.header.fields.iter().find(|f| f.name.eq_ignore_ascii_case(field)))
            .is_some_and(|f| f.kind == 'M');
        if !additive || !memo || !matches!(value.deref(), Value::Str(_)) {
            return Ok(Added::Value(value));
        }
        match self.read_field(fb, None, field)? {
            FieldRead::Suspend(req) => Ok(Added::Suspend(req)),
            FieldRead::Value(old) => match (old.deref(), value.deref()) {
                (Value::Str(old), Value::Str(new)) => Ok(Added::Value(Value::str(format!("{old}{new}")))),
                _ => Ok(Added::Value(value)),
            },
        }
    }

    fn read_field(&mut self, fb: &mut Fiber, area: Option<&str>, field: &str) -> Result<FieldRead, RtError> {
        // A HAVING clause is asked about a group rather than a record, and no record is current
        // while it runs. Measured in Visual FoxPro 9: a column there that is neither grouped nor
        // inside an aggregate is error 1803, whether or not the select list holds it too.
        if fb.query.as_ref().and_then(|r| r.having.as_ref()).is_some_and(|h| h.testing) {
            return Err(RtError::about(RtError::HAVING_INVALID, field));
        }
        let reply = fb.data_reply.take();
        let cursor = match area {
            Some(name) => self
                .data
                .find_mut(&AreaRef::Alias(name.to_string()))
                .ok_or_else(|| RtError::alias_not_found(name))?,
            None => self.data.cursor_mut().ok_or_else(no_table)?,
        };
        if let Some(answer) = reply {
            // one instruction can make several round trips: the page of records first, then a
            // block per memo field the record on it points at
            match cursor.take_expected_memo() {
                Some(block) => cursor.store_memo(block, &bytes_of(&answer)),
                None => {
                    let first = cursor.page_start(cursor.recno());
                    cursor.set_page(first, bytes_of(&answer));
                }
            }
            let recno = cursor.recno();
            cursor.seek(recno);
        }
        let recno = cursor.recno();
        if let Some(handle) = cursor.handle()
            && recno >= 1
            && recno <= cursor.count()
        {
            if !cursor.page_holds(recno) {
                let first = cursor.page_start(recno);
                cursor.expect_page(first);
                return Ok(FieldRead::Suspend(HostRequest::DataRead { handle, first: first as f64, count: PAGE_RECORDS }));
            }
            if let Some(block) = cursor.missing_memo(recno) {
                cursor.expect_memo(block);
                return Ok(FieldRead::Suspend(HostRequest::DataReadMemo { handle, block: f64::from(block) }));
            }
        }
        cursor
            .field(field)
            .map(FieldRead::Value)
            .ok_or_else(|| RtError::variable_not_found(field))
    }

    /// `SET TEXTMERGE [ON|OFF] [NOSHOW]`, `SET TEXTMERGE TO [file | MEMVAR var] [ADDITIVE]
    /// [NOSHOW]` and `SET TEXTMERGE DELIMITERS TO [left [, right]]`.
    ///
    /// The parser hands over the words the line carried as the first argument and what it named
    /// after them, so all three forms arrive the way every other setting's do.
    fn set_textmerge(&mut self, fb: &mut Fiber, args: &[Value]) -> Result<(), RtError> {
        let words = args.first().and_then(|v| v.as_str().ok()).unwrap_or_default().to_string();
        let has = |word: &str| words.split(' ').any(|w| w == word);
        if has("DELIMITERS") {
            // one delimiter on its own marks both ends, and naming none puts `<<` and `>>` back
            let left = args.get(1).map(Value::as_str).transpose()?.map(|s| s.to_string());
            let right = args.get(2).map(Value::as_str).transpose()?.map(|s| s.to_string());
            self.settings.textmerge_delimiters = match (left, right) {
                (None, _) => ("<<".to_string(), ">>".to_string()),
                (Some(l), None) => (l.clone(), l),
                (Some(l), Some(r)) => (l, r),
            };
            return Ok(());
        }
        if has("ON") {
            self.settings.textmerge = true;
        } else if has("OFF") {
            self.settings.textmerge = false;
        }
        if has("NOSHOW") {
            self.settings.textmerge_noshow = true;
        } else if has("SHOW") {
            self.settings.textmerge_noshow = false;
        }
        if !has("TO") {
            return Ok(());
        }
        // the destination being left takes what it has collected before the next one opens
        self.close_textmerge(fb);
        let named = args.get(1).and_then(|v| v.as_str().ok()).unwrap_or_default().trim().to_string();
        self.settings.textmerge_memvar = has("MEMVAR") && !named.is_empty();
        self.settings.textmerge_to = named;
        if self.settings.textmerge_to.is_empty() {
            return Ok(());
        }
        if self.settings.textmerge_memvar {
            // ADDITIVE builds on what the variable already holds; without it what was there goes
            let upper = self.settings.textmerge_to.to_ascii_uppercase();
            self.textmerge_held = match has("ADDITIVE").then(|| self.named_value(fb, &upper)).flatten() {
                Some(v) => v.as_str().map(|s| s.to_string()).unwrap_or_default(),
                None => String::new(),
            };
        } else {
            // a file has a line in progress the moment it is opened, so the first `\` ends it
            self.textmerge_begun = true;
            self.textmerge_written = has("ADDITIVE");
        }
        Ok(())
    }

    /// Closes the text merge destination: a variable being built takes what has been collected
    /// for it, which is when Visual FoxPro writes it rather than as each line goes in.
    fn close_textmerge(&mut self, fb: &mut Fiber) {
        if self.settings.textmerge_memvar && !self.settings.textmerge_to.is_empty() {
            let upper = self.settings.textmerge_to.to_ascii_uppercase();
            let text = std::mem::take(&mut self.textmerge_held);
            // as above: the merge is over by the time this runs, so a system variable of the
            // wrong type simply keeps what it had
            let _ = self.store_named(fb, &upper, Value::str(text));
        }
        self.textmerge_held.clear();
        self.textmerge_begun = false;
        self.textmerge_written = false;
    }

    /// The slot a LOCAL of this name sits in, for a name that is resolved while the program
    /// runs: `SET TEXTMERGE TO MEMVAR lcOut` names a variable rather than reading one, and a
    /// LOCAL lives in a slot of its frame rather than under its name.
    fn local_slot(&self, fb: &Fiber, upper: &str) -> Option<u32> {
        let env = env_index(fb, fb.frames.len().checked_sub(1)?);
        let frame = fb.frames.get(env)?;
        let proto = self.proto(frame.module, frame.func);
        proto.locals.iter().position(|l| l.eq_ignore_ascii_case(upper)).map(|i| i as u32)
    }

    fn store_named(&mut self, fb: &mut Fiber, upper: &str, v: Value) -> Result<(), RtError> {
        match self.local_slot(fb, upper) {
            Some(slot) => {
                self.store_local(fb, slot, v);
                Ok(())
            }
            None => self.store_name(fb, upper, v),
        }
    }

    fn named_value(&self, fb: &Fiber, upper: &str) -> Option<Value> {
        match self.local_slot(fb, upper) {
            Some(slot) => self.load_local(fb, slot).ok(),
            None => self.load_name(fb, upper),
        }
    }

    /// `SET LIBRARY TO [<file> [ADDITIVE]]`: hosting a Visual FoxPro library.
    ///
    /// The parser reads the name the way it reads any other file a command names - brackets round
    /// an expression, quotes, or bare words - and hands over whether ADDITIVE was there, then it.
    ///
    /// Measured in Visual FoxPro 9 with vfpencryption71.fll and the two API samples: a name with
    /// no extension gets `.fll`; a file that will not load raises 1726, "API library is not
    /// found."; without ADDITIVE every library already loaded is let go first; with nothing
    /// after TO they are all let go and none is loaded; and loading the same file twice leaves
    /// it named once.
    fn set_library(&mut self, host: &mut dyn Host, args: &[Value]) -> Result<(), RtError> {
        let additive = args.first().and_then(|v| v.truthy().ok()).unwrap_or(false);
        let text = args.get(1).and_then(|v| v.as_str().ok()).unwrap_or_default().trim().to_string();
        let named = text.trim().trim_matches(['"', '\'']).trim();

        if !additive {
            for (id, _) in std::mem::take(&mut self.libraries) {
                host.unload_library(id);
            }
            self.library_funcs.clear();
        }
        if !named.is_empty() {
            // a library named without an extension is a .fll, as every other file the language
            // names has the extension of the thing it is
            let last = named.rsplit(['\\', '/']).next().unwrap_or(named);
            let path = if last.contains('.') { named.to_string() } else { format!("{named}.fll") };
            // The product's own words are "API library is not found.", which is true of a file
            // that is not there and of one whose own dependencies are not - vfpencryption71.fll
            // needs the Visual C++ 7.1 runtimes, which ship with Visual FoxPro and not with
            // Windows. Which of those it was is said after the sentence, the way the runtime
            // says which feature it has not written yet, because a person needs to know.
            let (id, full, names) = host
                .load_library(&path)
                .map_err(|why| RtError::new(RtError::API_LIBRARY_NOT_FOUND, format!("API library is not found: {why}")))?;
            // the host answers with the file it actually opened; the same one twice is the one
            // already loaded, and the second copy is let go rather than named twice
            if self.libraries.iter().any(|(_, p)| p.eq_ignore_ascii_case(&full)) {
                host.unload_library(id);
            } else {
                for (i, name) in names.iter().enumerate() {
                    if !name.is_empty() {
                        self.library_funcs.insert(name.to_ascii_uppercase(), (id, i as u32));
                    }
                }
                self.libraries.push((id, full));
            }
        }
        self.settings.remembered.insert("LIBRARY".to_string(), self.library_setting());
        Ok(())
    }

    /// What `SET("LIBRARY")` answers. Measured: the full paths in capitals, in the order they
    /// were loaded, ", " between them, and a path quoted when it holds anything a plain path
    /// would not - a space and a hyphen both make Visual FoxPro quote it.
    fn library_setting(&self) -> String {
        self.libraries
            .iter()
            .map(|(_, path)| {
                let upper = path.to_ascii_uppercase();
                let plain = upper
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '\\' | '/' | ':' | '.' | '_'));
                if plain { upper } else { format!("\"{upper}\"") }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn set_cmd(
        &mut self,
        host: &mut dyn Host,
        fb: &mut Fiber,
        name: &str,
        args: &[Value],
        to: bool,
    ) -> Result<(), RtError> {
        // `SET TALK &cOld` puts the word back as text, which is how a program restores a setting
        // it saved with SET("TALK"); a logical arrives from `SET DELETED (.T.)`.
        let flag = || match args.first() {
            None => Ok(true),
            Some(Value::Str(s)) => match s.trim().to_ascii_uppercase().as_str() {
                "ON" | "YES" | "TRUE" => Ok(true),
                "OFF" | "NO" | "FALSE" | "" => Ok(false),
                _ => Err(RtError::new(RtError::DATA_TYPE_MISMATCH, "Data type mismatch")),
            },
            Some(v) => v.truthy(),
        };
        match name {
            "LIBRARY" => return self.set_library(host, args),
            "EXACT" => self.settings.exact = flag()?,
            "CENTURY" => match century_arg(args) {
                // SET CENTURY TO nCentury ROLLOVER nYear says how a two-digit year is read
                // and leaves ON/OFF - how one is written - alone
                Some(to) => self.settings.century_to = to,
                None => self.settings.century = flag()?,
            },
            "FIXED" => self.settings.fixed = flag()?,
            "SECONDS" => self.settings.seconds = flag()?,
            "POINT" => self.settings.point = char_arg(args)?.unwrap_or('.'),
            "SEPARATOR" => self.settings.separator = char_arg(args)?.unwrap_or(','),
            // the parser hands over the first word; the reference calls the option SET MARK TO,
            // and the coverage map reads these arms by the name the reference gives
            "MARK" | "MARK TO" => self.settings.mark = char_arg(args)?,
            "NULLDISPLAY" => {
                let text = args.first().map(|v| v.as_str()).transpose()?.filter(|t| !t.is_empty());
                if text.as_ref().is_some_and(|t| t.chars().count() > 15) {
                    return Err(RtError::string_too_long());
                }
                self.settings.null_display = text.map_or_else(|| ".NULL.".to_string(), |t| t.to_string());
            }
            "CURRENCY" => match args.first().map(|v| v.as_str()).transpose()?.filter(|w| !w.is_empty()) {
                // `SET CURRENCY LEFT` and `SET CURRENCY RIGHT` name the side; anything else is
                // `SET CURRENCY TO cSymbol`, of which nine characters are kept
                Some(w) if w.eq_ignore_ascii_case("LEFT") => self.settings.currency_left = true,
                Some(w) if w.eq_ignore_ascii_case("RIGHT") => self.settings.currency_left = false,
                Some(w) => self.settings.currency = w.chars().take(9).collect(),
                None => self.settings.currency = "$".to_string(),
            },
            "HOURS" => {
                self.settings.hours24 = match number_arg(args)? {
                    None | Some(12.0) => false,
                    Some(24.0) => true,
                    Some(_) => return Err(RtError::syntax("Syntax error")),
                }
            }
            "FDOW" => self.settings.fdow = ranged(number_arg(args)?, 1..=7)? as u8,
            "FWEEK" => self.settings.fweek = ranged(number_arg(args)?, 1..=3)? as u8,
            "TALK" => self.settings.talk = flag()?,
            "SAFETY" => self.settings.safety = flag()?,
            "ESCAPE" => self.settings.escape = flag()?,
            "DELETED" => self.settings.deleted = flag()?,
            "NEAR" => self.settings.near = flag()?,
            "HEADINGS" => self.settings.headings = flag()?,
            "ASSERTS" => self.settings.asserts = flag()?,
            "ECHO" => self.settings.echo = flag()?,
            // SET DEBUG says whether the debug windows are on Visual FoxPro's own menu. VFP 9
            // keeps the command for programs written against version 3 and answers ON to
            // SET("DEBUG") whatever it was told, which is what the debugger here does too.
            "DEBUG" => {}
            // the parser hands over the setting's name as the reference writes it, and
            // SET TEXTMERGE DELIMITERS is a name of its own
            "TEXTMERGE" | "TEXTMERGE DELIMITERS" => self.set_textmerge(fb, args)?,
            "DEBUGOUT" => {
                let path = args.first().and_then(|v| v.as_str().ok()).unwrap_or_default().trim().to_string();
                // ADDITIVE keeps what the file holds, so the first line written adds to it
                self.settings.debugout_started = args.get(1).and_then(|v| v.truthy().ok()).unwrap_or(false);
                self.settings.debugout = path;
            }
            "UNIQUE" => self.settings.unique = flag()?,
            // `SET KEY TO eKey [, eTop]`: the records whose key matches, and nothing else. See
            // Cursor::set_key_limit for why the second expression is read and not used.
            "KEY" => {
                let limit = args.first().filter(|v| !matches!(v, Value::Str(s) if s.trim().is_empty())).cloned();
                if let Some(cursor) = self.data.cursor_mut() {
                    cursor.set_key_limit(limit);
                }
            }
            // `SET FIELDS ON|OFF` turns the list on and off; `SET FIELDS TO a, b` names it.
            // Only the list is reported, by FLDLIST(); the switch goes with the rest below.
            //
            // The list is worked out here rather than when FLDLIST() asks, because that is when
            // Visual FoxPro works it out: every name comes back qualified by the work area that
            // was selected when the command ran, upper-cased, and ALL is the fields of that
            // table as they stood then.
            "FIELDS" if !matches!(args.first(), Some(Value::Logical(_))) => {
                let words = args.first().and_then(|v| v.as_str().ok()).unwrap_or_default().to_string();
                let alias = self.data.cursor().map(|c| c.alias.to_uppercase()).unwrap_or_default();
                let mut list = Vec::new();
                for word in words.split(',').map(str::trim).filter(|w| !w.is_empty()) {
                    if word.eq_ignore_ascii_case("ALL") {
                        if let Some(cursor) = self.data.cursor() {
                            let names: Vec<String> = cursor.header.fields.iter().map(|f| f.name.to_uppercase()).collect();
                            list.extend(names.into_iter().map(|n| format!("{alias}.{n}")));
                        }
                    } else if word.contains('.') {
                        list.push(word.to_uppercase());
                    } else {
                        list.push(format!("{alias}.{}", word.to_uppercase()));
                    }
                }
                self.settings.fields = list;
            }
            // SET INDEX TO x, y [ORDER z] [ADDITIVE]: the parser puts the ORDER first, then the
            // words the command carried, then the files, so that they all arrive as arguments
            // the way every other setting's do. The files are read after this returns.
            "INDEX" => {
                let order = args.first().cloned().unwrap_or_else(|| Value::str(""));
                let words = args.get(1).and_then(|v| v.as_str().ok()).unwrap_or_default().to_ascii_uppercase();
                let paths = named_files(args.get(2..).unwrap_or_default());
                let descending = if words.contains("DESCENDING") {
                    Some(true)
                } else if words.contains("ASCENDING") {
                    Some(false)
                } else {
                    None
                };
                self.begin_open_idx(paths, words.contains("ADDITIVE"), order, descending)?;
            }
            // `SET PATH TO [cPathList]`: the folders a relative name is looked for in after the
            // default directory. The list is kept as one piece and taken apart where it is used,
            // because that is what the product reports it as - `SET("PATH")` answers the text
            // that was set, upper-cased, ADDITIVE and all: the command has no such word, and
            // `SET PATH TO b ADDITIVE` leaves the product looking for a folder called
            // `b ADDITIVE`. Naming nothing clears it.
            "PATH" => {
                let given = args.first().and_then(|v| v.as_str().ok()).unwrap_or_default().trim().to_ascii_uppercase();
                self.settings.path = given;
            }
            // CD is this command under a shorter name, and both name a folder relative to the
            // one the program is already in
            "DEFAULT" => {
                let given = args.first().and_then(|v| v.as_str().ok()).unwrap_or_default().to_string();
                self.settings.default_dir = value::join_dir(&self.settings.default_dir, &given);
            }
            "MEMOWIDTH" => {
                let n = args.first().map(Value::as_number).transpose()?.unwrap_or(50.0);
                if !(8.0..=256.0).contains(&n) {
                    return Err(RtError::function_arg_invalid());
                }
                self.settings.memowidth = n as u8;
            }
            "DECIMALS" => {
                let n = number_arg(args)?.unwrap_or(2.0);
                if !(0.0..=18.0).contains(&n) {
                    return Err(RtError::syntax("Syntax error"));
                }
                self.settings.decimals = n as u8;
            }
            "DATE" => {
                if let Some(w) = args.first().and_then(|v| v.as_str().ok()).filter(|w| !w.trim().is_empty()) {
                    // a word it does not know is a syntax error and leaves the setting alone
                    let Some(f) = value::DateFormat::parse(&w) else {
                        return Err(RtError::syntax("Syntax error"));
                    };
                    self.settings.date_format = f;
                }
            }
            // Everything else the reference names is remembered so SET() can report it, and
            // does nothing: see value::REMEMBERED for what that means and why.
            other => {
                // `SET x TO something` names a target - a file, most often - and leaves the
                // switch where it was. The two are reported apart, so they are kept apart.
                // ...but only where the switch is the setting's own answer. `SET REFRESH TO 30`
                // names the setting itself, not a target beside it, and so do the other settings
                // whose answer is a number or a word.
                if to
                    && value::second_answer(other).is_some()
                    && matches!(value::remembered(other), Some((value::SettingShape::Switch, _)))
                {
                    let named = args.first().and_then(|v| v.as_str().ok()).unwrap_or_default().trim().to_string();
                    if named.is_empty() {
                        self.settings.targets.remove(other);
                    } else {
                        self.settings.targets.insert(other.to_string(), named.to_ascii_uppercase());
                    }
                    return Ok(());
                }
                if let Some((shape, _)) = value::remembered(other) {
                    let answer = match shape {
                        value::SettingShape::Switch => match flag()? {
                            true => "ON".to_string(),
                            false => "OFF".to_string(),
                        },
                        // the number arrives written out when the command had no TO in front of
                        // it - `SET ENGINEBEHAVIOR 70`. A word where a number was expected -
                        // SET REPROCESS TO AUTOMATIC - counts as none of them, which is what a
                        // fresh Visual FoxPro says too.
                        value::SettingShape::Count => {
                            let n = match args.first() {
                                Some(Value::Str(text)) => text.trim().parse().unwrap_or(0.0),
                                Some(v) => v.as_number().unwrap_or(0.0),
                                None => 0.0,
                            };
                            format!("{}", n.trunc())
                        }
                        value::SettingShape::Word => {
                            args.first().and_then(|v| v.as_str().ok()).unwrap_or_default().trim().to_string()
                        }
                    };
                    self.settings.remembered.insert(other.to_string(), answer);
                }
            }
        }
        Ok(())
    }
}

/// One frame of a stopped program, as the debugger's call stack shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameInfo {
    /// What `PROGRAM()` answers for it, which is also what a breakpoint on it is named by.
    pub program: String,
    /// The module its code was compiled into: the .prg or the form.
    pub module: String,
    pub line: u32,
}

/// One variable a frame can see of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// Upper-cased, as the runtime keeps every variable name.
    pub name: String,
    /// PRIVATE (or implicitly declared) rather than a LOCAL slot.
    pub private: bool,
    pub value: Value,
}

/// The absolute index of the `level`-th frame the call stack shows, outermost first;
/// `usize::MAX` is the innermost. Inline frames belong to their owner and are not counted.
fn frame_at_level(fb: &Fiber, level: usize) -> Option<usize> {
    let mut shown: Vec<usize> =
        (0..fb.frames.len()).filter(|&i| !matches!(fb.frames[i].kind, FrameKind::Inline { .. })).collect();
    if level == usize::MAX {
        return shown.pop();
    }
    shown.get(level).copied()
}

/// Whether a SET command's argument says ON. `SET STEP ON` arrives as a logical; a program that
/// puts a setting back through a macro sends the word it saved.
fn on_word(v: Option<&Value>) -> bool {
    match v {
        None => true,
        Some(Value::Str(s)) => matches!(s.trim().to_ascii_uppercase().as_str(), "ON" | "YES" | "TRUE"),
        Some(other) => other.truthy().unwrap_or(false),
    }
}

/// The character a display setting was given: its first, or `None` when nothing was given,
/// which is what puts the default back.
fn char_arg(args: &[Value]) -> Result<Option<char>, RtError> {
    match args.first() {
        None => Ok(None),
        Some(v) => Ok(v.as_str()?.chars().next()),
    }
}

/// A numeric setting's argument. A program that saved a setting with `SET("HOURS")` hands it
/// back as text through a macro, so a string of digits counts as the number.
fn number_arg(args: &[Value]) -> Result<Option<f64>, RtError> {
    match args.first().map(Value::deref) {
        None => Ok(None),
        Some(Value::Number(n, ..)) => Ok(Some(n)),
        Some(Value::Str(s)) => match s.trim() {
            "" => Ok(None),
            text => text.parse::<f64>().map(Some).map_err(|_| RtError::syntax("Syntax error")),
        },
        Some(_) => Err(RtError::data_type_mismatch()),
    }
}

/// A whole number a setting only takes inside a range; outside it Visual FoxPro leaves the
/// setting where it was and raises "expression evaluated to an illegal value".
fn ranged(n: Option<f64>, range: std::ops::RangeInclusive<i64>) -> Result<i64, RtError> {
    let n = match n {
        None => return Ok(*range.start()),
        Some(n) => n,
    };
    let whole = n.trunc() as i64;
    if n != whole as f64 || !range.contains(&whole) { Err(RtError::illegal_value()) } else { Ok(whole) }
}

/// `SET CENTURY TO [nCentury [ROLLOVER nYear]]`, as the words after TO. `None` when the line
/// was `SET CENTURY ON` or `OFF` instead; `Some(None)` when it was TO with nothing after it,
/// which puts today's default back.
fn century_arg(args: &[Value]) -> Option<Option<(i32, i32)>> {
    let text = match args.first().map(Value::deref) {
        Some(Value::Str(s)) => s.to_string(),
        Some(Value::Number(n, ..)) => format!("{n}"),
        _ => return None,
    };
    let mut words = text.split_whitespace();
    let Some(first) = words.next() else {
        return Some(None);
    };
    let century: i32 = first.parse().ok()?;
    let rollover = match words.next() {
        None => 0,
        Some(w) if w.eq_ignore_ascii_case("ROLLOVER") => words.next().and_then(|n| n.parse().ok()).unwrap_or(0),
        Some(_) => return None,
    };
    Some(Some((century, rollover)))
}

enum NameLoc {
    Private(usize),
    /// A private of a fiber this one is nested inside: the program parked in READ EVENTS while
    /// an event handler runs.
    Parked(FiberId),
    Global,
}

/// The frame whose storage the frame at `idx` uses (itself unless it is inline).
fn env_index(fb: &Fiber, idx: usize) -> usize {
    match fb.frames[idx].kind {
        FrameKind::Inline { owner, .. } => owner,
        _ => idx,
    }
}

/// Stores through a by-reference cell when the slot holds one.
/// A file operation's answer, as the host sends it: the value and the error number.
fn file_reply(v: &Value) -> (Value, i64) {
    match v.deref() {
        Value::Array(a) => {
            let items = a.borrow();
            let value = items.items.first().cloned().unwrap_or(Value::Null).deref();
            let errno = items.items.get(1).and_then(|e| e.as_number().ok()).unwrap_or(0.0) as i64;
            (value, errno)
        }
        other => (other, 0),
    }
}

/// The error a file command raises, with the number Visual FoxPro gives the failure.
fn file_error(op: &str, errno: i64) -> RtError {
    let (code, text) = match errno {
        2 => (1, "File does not exist"),
        5 => (1705, "File access is denied"),
        _ => (1102, "Cannot create file"),
    };
    RtError::new(code, format!("{text} ({op})"))
}

fn write_through(slot: &mut Value, v: Value) {
    let v = value::held_in_variable(v.deref());
    match slot {
        Value::Ref(cell) => *cell.borrow_mut() = v,
        _ => *slot = v,
    }
}

/// What THIS, THISFORM and THISFORMSET say when no method is running: one code, and the
/// keyword that was written naming itself. Measured against Visual FoxPro 9.
fn this_error(keyword: &str) -> RtError {
    RtError::new(1929, format!("{keyword} can only be used within a method."))
}

/// What THISFORMSET says when the object is in no formset, in the product's own words.
fn not_contained_in(what: &str) -> RtError {
    RtError::new(1938, format!("Object is not contained in a {what}."))
}

fn pop_n(fb: &mut Fiber, n: usize) -> Result<Vec<Value>, RtError> {
    if fb.stack.len() < n {
        return Err(RtError::new(0, "Stack underflow"));
    }
    Ok(fb.stack.split_off(fb.stack.len() - n))
}

/// The outcome of asking a cursor for a field: the value, or the read that has to happen first.
enum FieldRead {
    Value(Value),
    Suspend(HostRequest),
}

/// What `REPLACE ... ADDITIVE` will write, once the memo it adds to has been read.
enum Added {
    Value(Value),
    Suspend(HostRequest),
}

fn no_table() -> RtError {
    RtError::no_table_open()
}

/// A table's alias when `USE` did not give one: the file name without its directory or extension.
fn default_alias(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem).to_string()
}

fn pop_subscripts(fb: &mut Fiber, n: u8) -> Result<Vec<usize>, RtError> {
    pop_n(fb, n as usize)?.iter().map(Value::as_usize).collect()
}

/// What a write to a member of one of the VM's own objects raises: the product's read-only
/// error for a member it has, and its unknown-member error for one it has not.
fn native_write_refused(natives: &crate::foxscript::Natives, h: Handle, name: &str) -> RtError {
    match natives.get(h).and_then(|n| n.class.member(name)) {
        Some(_) => RtError::new(1743, format!("{} is a read-only property.", name.to_ascii_uppercase())),
        None => RtError::unknown_member(name),
    }
}

/// A member of a JSON value by name, or nothing when the value is not one.
///
/// A name a JSON object has not got is the product's own unknown-member error, because that is
/// what reading a property an object has not got already raises; `FoxScript.Json.Get()` is the
/// form that asks rather than insists.
fn json_member(v: &Value, name: &str) -> Option<Result<Value, RtError>> {
    let Value::Json(doc) = v.deref() else { return None };
    Some(match crate::json::member(&doc, name) {
        Some(found) => Ok(crate::json::from_json(found)),
        None => Err(RtError::unknown_member(name)),
    })
}

fn index_array(arr: &Value, subs: &[usize]) -> Result<Value, RtError> {
    match arr.deref() {
        Value::Array(a) => a.borrow().get(subs),
        // a JSON array is subscripted the way every other subscript in this language is: one
        // based, and one subscript deep
        Value::Json(doc) => match subs {
            [i] => crate::json::element(&doc, *i).map(crate::json::from_json).ok_or_else(RtError::invalid_subscript),
            _ => Err(RtError::invalid_subscript()),
        },
        _ => Err(RtError::invalid_subscript()),
    }
}

fn array_dims(dims: &[Value]) -> Result<(usize, usize), RtError> {
    let rows = dims.first().map(Value::as_usize).transpose()?.unwrap_or(0);
    let cols = dims.get(1).map(Value::as_usize).transpose()?.unwrap_or(0);
    if rows == 0 || (dims.len() == 2 && cols == 0) {
        return Err(RtError::invalid_subscript());
    }
    Ok((rows, cols))
}

/// What built-ins see: the VM, the host and the running fiber.
struct Ctx<'a> {
    vm: &'a mut Vm,
    host: &'a mut dyn Host,
    fiber: &'a mut Fiber,
}

impl BuiltinCtx for Ctx<'_> {
    fn data_mut(&mut self) -> &mut DataSession {
        &mut self.vm.data
    }

    fn take_data_reply(&mut self) -> Option<Value> {
        self.fiber.data_reply.take()
    }

    fn data(&self) -> &DataSession {
        &self.vm.data
    }
    fn settings(&self) -> &Settings {
        &self.vm.settings
    }
    fn settings_mut(&mut self) -> &mut Settings {
        &mut self.vm.settings
    }
    fn host(&mut self) -> &mut dyn Host {
        &mut *self.host
    }
    fn class_members(&mut self, class: &str) -> Option<Vec<crate::host::MemberInfo>> {
        let current = self.fiber.frames.last().map(|f| f.module);
        self.vm.class_members(current, class)
    }
    fn object_members(&mut self, obj: Handle) -> Option<Vec<crate::host::MemberInfo>> {
        self.vm.object_members(&mut *self.host, obj)
    }
    fn keep_picture(&mut self, picture: crate::picture::Picture) -> f64 {
        self.vm.pictures.keep(picture)
    }
    fn picture(&self, handle: f64) -> Option<&crate::picture::Picture> {
        self.vm.pictures.get(handle)
    }
    fn pcount(&self) -> usize {
        let env = env_index(self.fiber, self.fiber.frames.len() - 1);
        self.fiber.frames[env].args.len()
    }
    /// Upper-cased, as the product says it: `PROCEDURE MixedCaseName` reads back as
    /// `MIXEDCASENAME`, whatever the source wrote.
    fn program_name(&self) -> String {
        let env = env_index(self.fiber, self.fiber.frames.len() - 1);
        let f = &self.fiber.frames[env];
        self.vm.proto(f.module, f.func).display_name.to_ascii_uppercase()
    }
    fn program_level(&self) -> usize {
        self.fiber.frames.len()
    }
    fn ferror(&self) -> i64 {
        self.vm.ferror
    }
    fn txn_level(&self) -> u32 {
        self.vm.txn
    }
    fn cursor_flag(&self, alias: &str, name: &str) -> bool {
        self.vm.cursor_flags.contains(&(alias.to_ascii_uppercase(), name.to_string()))
    }

    fn set_cursor_flag(&mut self, alias: &str, name: &str, on: bool) {
        let key = (alias.to_ascii_uppercase(), name.to_string());
        if on {
            self.vm.cursor_flags.insert(key);
        } else {
            self.vm.cursor_flags.remove(&key);
        }
    }

    fn result_set(&self) -> usize {
        self.vm.result_set
    }

    fn set_result_set(&mut self, area: usize) {
        self.vm.result_set = area;
    }

    fn view_sql(&self, alias: &str) -> String {
        self.vm.view_named(alias).unwrap_or_default()
    }

    fn install_cursor(&mut self, alias: String, fields: Vec<crate::dbf::DbfField>, rows: Vec<crate::dbf::DbfRecord>) {
        // a cursor of that name is replaced, and one that is not there goes in a free area
        if self.vm.data.used(&AreaRef::Alias(alias.clone())) {
            let _ = self.vm.data.select(&AreaRef::Alias(alias.clone()));
        } else {
            let _ = self.vm.data.select(&AreaRef::Number(0));
        }
        let old = self.vm.data.install(Cursor::in_memory(alias, fields, rows));
        if let Some(handle) = old {
            self.vm.to_release.push(handle);
        }
    }

    fn sql_property(&self, handle: i64, name: &str) -> Value {
        match self.vm.sql_props.get(&(handle, name.to_string())) {
            Some(value) => value.clone(),
            // what a connection is set to when nothing has set it, as the reference has it
            None => match name {
                "ASYNCHRONOUS" | "BATCHMODE" => Value::Logical(name == "BATCHMODE"),
                "CONNECTSTRING" | "DATASOURCE" | "USERID" | "PASSWORD" | "PREPARED" => Value::str(""),
                "CONNECTTIMEOUT" | "IDLETIMEOUT" | "QUERYTIMEOUT" | "WAITTIME" => Value::number(0.0),
                "TRANSACTIONS" => Value::number(1.0),
                "DISPLOGIN" => Value::number(3.0),
                _ => Value::Logical(false),
            },
        }
    }

    fn set_sql_property(&mut self, handle: i64, name: &str, value: Value) {
        self.vm.sql_props.insert((handle, name.to_string()), value);
    }

    fn com_property(&self, obj: u32, name: &str) -> Value {
        self.vm.com_props.get(&(obj, name.to_string())).cloned().unwrap_or(Value::Logical(false))
    }

    fn set_com_property(&mut self, obj: u32, name: &str, value: Value) {
        self.vm.com_props.insert((obj, name.to_string()), value);
    }

    fn store_named(&mut self, name: &str, value: Value) -> Result<(), RtError> {
        self.vm.store_name(self.fiber, &name.to_ascii_uppercase(), value)
    }

    fn running_object(&self) -> Option<u32> {
        let env = env_index(self.fiber, self.fiber.frames.len() - 1);
        self.fiber.frames[env].this.map(|h| h.0)
    }

    fn running_method(&self) -> String {
        // a method is compiled under "OBJECT.EVENT"; DODEFAULT() is about the event
        let env = env_index(self.fiber, self.fiber.frames.len() - 1);
        let f = &self.fiber.frames[env];
        let name = self.vm.proto(f.module, f.func).display_name.clone();
        name.rsplit('.').next().unwrap_or(&name).to_string()
    }

    fn com_setting(&self, obj: u32) -> i64 {
        self.vm.com_arrays.get(&obj).copied().unwrap_or(0)
    }

    fn set_com_setting(&mut self, obj: u32, how: i64) {
        self.vm.com_arrays.insert(obj, how);
    }

    fn dlls(&self) -> Vec<(String, String, String)> {
        let mut out: Vec<(String, String, String)> = self
            .vm
            .dlls
            .iter()
            .map(|(called, (library, proto))| (called.clone(), library.clone(), proto.function.clone()))
            .collect();
        out.sort();
        out
    }

    fn menus(&self) -> &crate::menu::Menus {
        &self.vm.menus
    }
    fn screen(&self) -> &crate::screen::Screen {
        &self.vm.screen
    }
    fn screen_mut(&mut self) -> &mut crate::screen::Screen {
        &mut self.vm.screen
    }
    fn databases(&self) -> &[Database] {
        &self.vm.databases
    }
    fn databases_mut(&mut self) -> &mut Vec<Database> {
        &mut self.vm.databases
    }
    fn current_database(&self) -> Option<usize> {
        self.vm.current_db
    }
    fn write_current_database(&mut self) -> Option<HostRequest> {
        let index = self.vm.current_db?;
        self.vm.write_database(index)
    }
    fn database_io_pending(&self) -> bool {
        self.vm.db_io.is_some()
    }
    fn database_io_step(&mut self, reply: Option<Value>) -> Result<Option<HostRequest>, RtError> {
        self.vm.db_io_step(reply)
    }
    fn database_event(&mut self, name: &str, args: &[DbcArg]) -> bool {
        self.vm.dbc_event(self.host, self.fiber, name, args)
    }
    fn program_at(&self, level: usize) -> Option<String> {
        let f = self.fiber.frames.get(level.checked_sub(1)?)?;
        Some(self.vm.proto(f.module, f.func).display_name.to_ascii_uppercase())
    }
    fn module_at(&self, level: usize) -> Option<String> {
        let f = self.fiber.frames.get(level.checked_sub(1)?)?;
        Some(self.vm.modules.get(f.module as usize)?.name.clone())
    }
    fn line(&self) -> u32 {
        let env = env_index(self.fiber, self.fiber.frames.len() - 1);
        self.fiber.frames[env].line
    }
    fn def_line(&self) -> u32 {
        let env = env_index(self.fiber, self.fiber.frames.len() - 1);
        let f = &self.fiber.frames[env];
        self.vm.proto(f.module, f.func).def_line
    }
    fn evaluate(&mut self, expr: &str) -> Result<Value, RtError> {
        self.vm.eval_in_frame(self.host, self.fiber, expr)
    }
    fn lookup_variable(&self, upper_name: &str) -> Option<Value> {
        let top = self.fiber.frames.last()?;
        let env = env_index(self.fiber, self.fiber.frames.len() - 1);
        let proto = self.vm.proto(top.module, top.func);
        if let Some(slot) = proto.locals.iter().position(|l| l == upper_name)
            && !upper_name.starts_with('#')
        {
            return self.fiber.frames[env].locals.get(slot).map(Value::deref);
        }
        self.vm.load_name(self.fiber, upper_name)
    }
    fn last_error(&self) -> Option<RtError> {
        self.fiber.last_error.clone()
    }
    fn on_error_command(&self) -> Option<String> {
        self.vm.on_error.clone()
    }
    fn handler(&self, what: &str) -> Option<String> {
        self.vm.handler(what).map(str::to_string)
    }
}

/// Whether a stored field value equals what a program is comparing it with, for `IN (SELECT ...)`.
/// The comparison is exact: a key that matches only up to the shorter of the two is not the key.
fn same_value(stored: Option<&crate::dbf::DbfValue>, value: &Value, settings: &Settings) -> bool {
    let Some(stored) = stored else { return false };
    let mut exact = settings.clone();
    exact.exact = true;
    matches!(
        value::compare(&crate::data::value_of(stored), value, value::CmpOp::Eq, &exact),
        Ok(Value::Logical(true))
    )
}

/// The work area an `IN` clause named: a number, or an alias.
/// A program as a procedure file names it: the file's stem, without its folder or extension.
fn program_stem(name: &str) -> String {
    let file = name.trim().rsplit(['\\', '/']).next().unwrap_or(name).to_string();
    let lower = file.to_ascii_lowercase();
    let stem = if lower.ends_with(".prg") || lower.ends_with(".fxp") { &file[..file.len() - 4] } else { file.as_str() };
    stem.to_ascii_uppercase()
}

/// Error 1 for a procedure file that is not there, in the product's words: the name as the
/// program wrote it, given the extension of a program when it had none.
fn program_missing(name: &str) -> RtError {
    let file = name.trim().rsplit(['\\', '/']).next().unwrap_or(name).to_string();
    // a name the compiler upper-cased is given back as the product writes it, in lower case
    let file = if file == file.to_ascii_uppercase() { file.to_ascii_lowercase() } else { file };
    let file = if file.contains('.') { file } else { format!("{file}.prg") };
    RtError::new(RtError::FILE_NOT_FOUND, format!("File '{file}' does not exist."))
}

fn area_ref(v: &Value) -> Result<AreaRef, RtError> {
    Ok(match v.deref() {
        Value::Str(s) => AreaRef::Alias(s.trim().to_string()),
        other => AreaRef::Number(other.as_number()?.max(0.0) as usize),
    })
}

/// The frame of `frames` that holds the private called `upper`, innermost first. Inline frames
/// share their owner's storage, so they are not a scope of their own.
fn private_frame(frames: &[Frame], upper: &str) -> Option<usize> {
    frames
        .iter()
        .enumerate()
        .rev()
        .find(|(_, f)| !matches!(f.kind, FrameKind::Inline { .. }) && f.privates.contains_key(upper))
        .map(|(i, _)| i)
}

/// The items of a value that is an array, or nothing when it is not one.
fn items_of(v: &Value) -> Vec<Value> {
    match v.deref() {
        Value::Array(a) => a.borrow().items.clone(),
        _ => Vec::new(),
    }
}

/// The events a database container raises, for `ALANGUAGE(a, 4)` and for anyone reading what
/// the container can be asked to do. The reference writes them without the `dbc_` in front.
pub const DBC_EVENTS: &[&str] = &[
    "Activate", "AfterAddTable", "AfterAppendProc", "AfterCloseTable", "AfterCopyProc",
    "AfterCreateTable", "AfterCreateView", "AfterDeleteView", "AfterDropTable", "AfterOpenTable",
    "AfterRemoveTable", "AfterRenameTable", "AfterRenameView", "AfterValidateData",
    "BeforeAddTable", "BeforeAppendProc", "BeforeCloseTable", "BeforeCopyProc",
    "BeforeCreateTable", "BeforeCreateView", "BeforeDeleteView", "BeforeDropTable",
    "BeforeOpenTable", "BeforeRemoveTable", "BeforeRenameTable", "BeforeRenameView",
    "BeforeValidateData", "CloseData", "Deactivate", "OpenData", "PackData",
];
