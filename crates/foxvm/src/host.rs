//! The boundary between the VM and the environment that owns forms, dialogs and files.
//!
//! Two halves, on purpose: `Host` is synchronous and side-effect free (property reads, member
//! classification, output), implemented by JS imports in the wasm build. Anything with a side
//! effect or that may block (setting a property, calling a method, MESSAGEBOX, DO FORM, READ
//! EVENTS, file I/O) is a `HostRequest` the VM *yields*; the scheduler performs it while the
//! VM is off the stack and then resumes the fiber with the result. This is what keeps the
//! wasm-bindgen object non-re-entrant and makes modal dialogs work without threads.

use serde::{Deserialize, Serialize};

use crate::error::RtError;
use crate::value::{Handle, Value};

/// What a name after `.` refers to on a host object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member {
    /// A contained object (control, page, column...).
    Child(Handle),
    Property,
    Method,
    None,
}

/// What a member is, which is the word AMEMBERS() writes in its second column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Property,
    Event,
    Method,
    /// A contained object: a control on a form, a page of a page frame.
    Object,
}

impl MemberKind {
    pub fn word(self) -> &'static str {
        match self {
            MemberKind::Property => "Property",
            MemberKind::Event => "Event",
            MemberKind::Method => "Method",
            MemberKind::Object => "Object",
        }
    }
}

/// One member of an object or of a class: what AMEMBERS() lists and GETPEM() reads.
///
/// The flags are the ones AMEMBERS()' `cFlags` filters on. Protected and hidden are not among
/// them, because nothing in this runtime is either: a member a program can name it can reach.
#[derive(Debug, Clone)]
pub struct MemberInfo {
    /// Upper-cased, which is how AMEMBERS() writes it.
    pub name: String,
    pub kind: MemberKind,
    /// Declared by the Visual FoxPro base class the object stands on, rather than by a class
    /// definition above it or by ADDPROPERTY().
    pub native: bool,
    /// Put there by ADDPROPERTY() while the program ran.
    pub added: bool,
    /// The product refuses to have a program write it.
    pub read_only: bool,
    /// It holds something other than what its class starts it at.
    pub changed: bool,
    /// What it holds now, for GETPEM(); nothing for a method or an event.
    pub value: Option<Value>,
}

impl MemberInfo {
    /// A property of an object, with everything else left as a plain declared member.
    pub fn property(name: &str, value: Value) -> Self {
        MemberInfo {
            name: name.to_ascii_uppercase(),
            kind: MemberKind::Property,
            native: false,
            added: false,
            read_only: false,
            changed: false,
            value: Some(value),
        }
    }
}

pub trait Host {
    /// Property value; `Err(1734)` when unknown. Intrinsics (`Name`, `Parent`, `Class`,
    /// `BaseClass`, `ControlCount`...) are answered here too.
    fn get_prop(&mut self, obj: Handle, name: &str) -> Result<Value, RtError>;
    fn get_member(&mut self, obj: Handle, name: &str) -> Result<Member, RtError>;
    /// Every member the object has, in no particular order: what AMEMBERS() lists and GETPEM()
    /// reads. `None` when the handle names nothing, which is what AMEMBERS() refuses on.
    ///
    /// A host that cannot enumerate an object says so by leaving this alone; the functions that
    /// need it then refuse the call rather than answering with half a list.
    fn members(&mut self, _obj: Handle) -> Option<Vec<MemberInfo>> {
        None
    }
    /// The objects a collection member holds, in the order `FOR EACH` visits them - an array
    /// value - for a member that is a collection rather than a property: `_SCREEN.Forms`, which
    /// Visual FoxPro will not read as a value (1924, measured) but will walk. `None` when the
    /// member is not one, and `FOR EACH` then reads it as it reads anything else.
    fn enumerate(&mut self, _obj: Handle, _member: &str) -> Option<Value> {
        None
    }
    /// Whether the object was a form (or a member of one) that has since been released. A
    /// variable or array element still holding it reads as .NULL. from then on - measured:
    /// `oForm.Release()`, then `ISNULL(oForm)` is .T. and `VARTYPE(oForm)` is "X".
    fn released(&mut self, _obj: Handle) -> bool {
        false
    }
    /// `None` when the handle has been released ("Object is not valid").
    fn object_class(&mut self, obj: Handle) -> Option<String>;
    /// The file the object was built from, for `SYS(1271, oObject)`: a form's `.scx`. Measured:
    /// the product answers .F. for everything else, an object of a class library included, so a
    /// host with nothing to say leaves this alone.
    fn object_file(&mut self, _obj: Handle) -> Option<String> {
        None
    }
    /// `?` / `??` output.
    fn output(&mut self, text: &str, newline: bool);
    /// (days since 1970-01-01, seconds since midnight) in local time.
    fn now(&mut self) -> (i32, f64);
    /// Uniform in [0, 1).
    fn random(&mut self) -> f64;
    /// Starts the sequence `random()` walks again from `seed`, for `RAND(nSeed)`.
    ///
    /// The same seed gives the same sequence, which is what a program seeds for; the numbers
    /// themselves are this generator's and not Visual FoxPro's, which nothing can reproduce.
    fn seed_random(&mut self, seed: f64);
    /// True when the object's own class defines this method in FoxPro source, as opposed to
    /// merely having an event of that name. `DEFINE CLASS ... PROCEDURE Error` is the case
    /// that matters: VFP routes an error inside a method to it.
    fn class_method(&mut self, _obj: Handle, _name: &str) -> bool {
        false
    }
    /// True when the object carries FoxPro source for a method of that name, whether its class
    /// wrote it or the form or library it came from did. What asks is a property read or write,
    /// looking for the `Prop_Access` or `Prop_Assign` method that stands in for it.
    fn has_code_method(&mut self, _obj: Handle, _name: &str) -> bool {
        false
    }
    /// Where the mouse pointer is over the character screen, in rows and columns from its
    /// top left, and whether a button is down. A host that does not track it says so by
    /// leaving the pointer at the corner with nothing pressed.
    fn mouse(&mut self) -> (f64, f64, bool) {
        (0.0, 0.0, false)
    }
    /// `name|major|minor|build` for OS(). Only the host knows what it is running on.
    fn os_info(&mut self) -> String {
        "Windows|10|0|0".to_string()
    }
    /// Every error the VM raises, before anything decides what to do with it.
    ///
    /// An error the program handles - TRY/CATCH, an Error method, ON ERROR - leaves no other
    /// trace, and a handler that reports it in its own words (`MESSAGEBOX(MESSAGE())`) gives the
    /// message without the place. This is the last point at which the program and line are still
    /// known, so the host is told here and can log it.
    fn error_raised(&mut self, _err: &RtError, _handled: bool) {}
    /// `DO <prog>`: id of an already loaded program module, or `None` (the VM then yields
    /// `HostRequest::LoadProgram` so the host can compile/load it and resume with the id).
    fn resolve_program(&mut self, name: &str) -> Option<u32>;

    /// `SET LIBRARY TO`: loads a Visual FoxPro library and answers with the number the host
    /// will know it by, the file it actually opened, and the name of each function the library
    /// adds, in the order the host numbers them. A name that comes back empty is one the
    /// library declared but does not want called - internal, or run on load or on unload - and
    /// its place is kept so the numbers still line up.
    ///
    /// This is not a `HostRequest` and cannot be. A program may call a library function from
    /// inside an expression the runtime is already evaluating - `TYPE([Hash("a", 5)])` is the
    /// smallest case - and there is no way to suspend a fiber in the middle of one, so both
    /// this and `call_library` answer while the VM is still on the stack.
    ///
    /// A host with nowhere to put a library says so in its own words, and the runtime turns
    /// that into error 1726, which is what the product raises.
    fn load_library(&mut self, _path: &str) -> Result<(u32, String, Vec<String>), String> {
        Err("a Visual FoxPro library cannot be hosted here".to_string())
    }

    /// One of a loaded library's functions, by the library and the function's number. What
    /// the function printed with `_PutStr` has already gone to `output` by the time this
    /// answers, because that is what the product does with it.
    fn call_library(&mut self, _library: u32, _function: u32, _args: &[Value]) -> Result<Value, RtError> {
        Err(RtError::new(RtError::API_LIBRARY_NOT_FOUND, "API library is not found."))
    }

    /// Lets a loaded library go: `SET LIBRARY TO` without ADDITIVE, and `SET LIBRARY TO` alone.
    fn unload_library(&mut self, _library: u32) {}
}

/// The sequence `RAND(nSeed)` starts: xorshift64*, so that a seed reproduces it exactly.
///
/// A host with a generator of its own still needs this one, because a program that seeds is
/// asking for a sequence it can get back, and `Math.random()` is not that.
#[derive(Debug, Clone, Copy)]
pub struct SeededRandom(u64);

/// The state the generator starts on, and the multiplier its output is scrambled with.
const XORSHIFT_START: u64 = 0x2545_F491_4F6C_DD1D;

impl SeededRandom {
    /// The state a seed puts it in. Xorshift stands still on zero, so a seed that would land
    /// there takes the starting state instead.
    pub fn from_seed(seed: f64) -> Self {
        match (seed as i64 as u64) ^ XORSHIFT_START {
            0 => SeededRandom(XORSHIFT_START),
            x => SeededRandom(x),
        }
    }

    /// The next number, uniform in [0, 1).
    pub fn next_number(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(XORSHIFT_START) >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Default for SeededRandom {
    fn default() -> Self {
        SeededRandom(XORSHIFT_START)
    }
}

/// Side effects yielded by the VM. Each variant documents what the fiber is resumed with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum HostRequest {
    /// Resume with Null. The host may run ProgrammaticChange etc. as nested fibers first.
    SetProp {
        obj: u32,
        name: String,
        value: JsonValue,
    },
    /// `obj.Prop[2] = v`, for an array property or an indexed list property. Resume with Null.
    SetPropIndex {
        obj: u32,
        name: String,
        index: Vec<u32>,
        value: JsonValue,
    },
    /// `DIMENSION obj.aProp[2, 3]`: the property is given an array of that shape.
    ///
    /// `cols` is 0 for a list of one dimension. Measured in Visual FoxPro: the elements that
    /// still fit are kept, row by row, and a property the object has never heard of raises 1734
    /// rather than being created. Resume with Null.
    DimProp {
        obj: u32,
        name: String,
        rows: u32,
        cols: u32,
    },
    /// `CATCH TO oErr`: the host builds VFP's Exception object. Resume with its handle.
    CreateException {
        code: u32,
        message: String,
        program: String,
        line: u32,
        user_value: JsonValue,
    },
    /// Resume with the method's return value (Null when it has none).
    CallMethod {
        obj: u32,
        name: String,
        args: Vec<JsonValue>,
    },
    /// Resume with `[obj?, result?]` as an array per `form_flags`, or Null when neither is wanted.
    DoForm {
        name: String,
        args: Vec<JsonValue>,
        modal: Option<bool>,
        linked: bool,
        noshow: bool,
        want_object: bool,
        want_result: bool,
    },
    /// Resume with Null once Destroy/Unload have run.
    ReleaseObject {
        obj: u32,
    },
    /// `CREATEOBJECT(class, args...)`: resume with the new object's handle. `definition` is the
    /// program's `DEFINE CLASS` for that name when there is one, and `None` when the class is the
    /// host's business (the VFP base classes: `Empty`, `Custom`, `Form`, ...).
    CreateObject {
        class: String,
        args: Vec<JsonValue>,
        #[serde(default)]
        definition: Option<ClassDefOut>,
        /// `NEWOBJECT()`'s second argument: the file the class is to be read out of, loading it
        /// if it is not loaded already. Empty for `CREATEOBJECT()`, which only names a class.
        #[serde(default)]
        module: String,
    },
    /// `SET CLASSLIB TO [file[, file...]] [ALIAS name] [ADDITIVE]`: the class libraries whose
    /// classes a program may name. Resume with what `SET("CLASSLIB")` is to answer afterwards -
    /// the host owns the libraries, so it owns the list - or raise error 1 for a file that is
    /// not there, which is what the product does.
    LoadClassLib {
        /// The libraries to load; empty means forget the ones that are loaded.
        files: Vec<String>,
        /// The name the library answers to, when `ALIAS` gave it one instead of its file stem.
        alias: String,
        /// Keep the libraries already loaded and add to them.
        additive: bool,
    },
    /// The SQL pass-through functions: a connection to a data source, and what is asked of it.
    ///
    /// `what` is 0 connect, 1 disconnect, 2 run a statement, 3 the tables of the source,
    /// 4 the columns of one of them, 5 commit, 6 roll back, 7 cancel, 8 more results.
    /// Resume with the connection number for a connect; for a statement, with an array of the
    /// columns - each of them name, type letter, width and decimals - followed by an array per
    /// row; and with a number for the rest, negative when it failed.
    Sql {
        what: u8,
        /// The connection the call names, or 0 when it is making one.
        handle: i64,
        /// What to connect with, what to run, or the table to describe.
        text: String,
        /// The user, the cursor to fill, or the table pattern: what the second argument was.
        extra: String,
    },
    /// `DODEFAULT()`: the method of the same name on the class this one was built from, run
    /// with the arguments given. Resume with what it answered, or with .F. when the class it
    /// came from has no such method.
    CallParentMethod {
        obj: u32,
        /// The method being run, which is the one to look for above it: its event, with the
        /// `#n` of an ancestor's copy when that is what is running.
        method: String,
        args: Vec<JsonValue>,
        /// The whole name the running code was compiled under - `CLASS.EVENT` for a class of a
        /// program - which says whose code it is, and so where above it to start looking.
        #[serde(default)]
        from: String,
    },
    /// `BUILD APP | EXE | DLL | MTDLL | PROJECT`: a project turned into the file that ships,
    /// or the project itself built out of the files it names. Resume with Null; a build that
    /// cannot be done raises the error rather than answering.
    Build {
        /// APP, EXE, DLL, MTDLL or PROJECT.
        what: String,
        /// The file being built, with the path already resolved.
        target: String,
        /// The project it is built from; for PROJECT, the files that go in it. Empty when
        /// BUILD PROJECT is refreshing a project that already exists.
        from: Vec<String>,
        recompile: bool,
    },
    /// `COMPILE`: source read, compiled and reported on. Resume with Null.
    Compile {
        /// DATABASE, FORM, CLASSLIB, LABEL or REPORT; empty when it is a program.
        what: String,
        /// The file, or a skeleton like `*.prg` standing for several.
        files: String,
        all: bool,
        encrypt: bool,
        nodebug: bool,
    },
    /// `CREATE FORM`, `CREATE MENU` and the rest: something new of that kind, opened in its
    /// designer. Resume with Null.
    NewDocument {
        /// FORM, SCREEN, MENU, PROJECT or QUERY, as the command wrote it.
        what: String,
        /// What to call it, which may be empty: the designer then asks when it is saved.
        path: String,
    },
    /// A line of FoxPro for the host to compile and run, which is what a function that has to
    /// run a whole statement asks for. Resume with Null.
    RunLine {
        text: String,
    },
    /// `GETOBJECT()`: an object that is already running under that name, or the one a file
    /// stands for. Resume with the object.
    GetObject {
        /// A class that is running, or the path of a file.
        name: String,
        /// The class the file is to be opened as, when the call named one.
        class: String,
    },
    /// `ADDPROPERTY(obj, name, value)`: resume with .T.
    AddProperty {
        obj: u32,
        name: String,
        value: JsonValue,
    },
    /// Resume with the button number (1 OK, 2 Cancel, 3 Abort, 4 Retry, 5 Ignore, 6 Yes, 7 No).
    MessageBox {
        text: String,
        flags: u32,
        title: String,
        timeout: Option<u32>,
    },
    /// Resume with the entered text (or `timeout_value` / "" on cancel).
    InputBox {
        prompt: String,
        title: String,
        default: String,
        timeout: Option<u32>,
        timeout_value: String,
    },
    /// Resume with Null (immediately for NOWAIT, after the key/timeout otherwise).
    WaitWindow {
        text: String,
        nowait: bool,
        timeout: Option<f64>,
        clear: bool,
    },
    /// `oServer.Listen(nPort)`: the host opens a socket on that port and starts handing
    /// requests back as host events. `port` 0 asks for whatever port is free. Resume with the
    /// port actually bound.
    HttpListen {
        server: u32,
        port: u32,
    },
    /// `oServer.Close()`: the host stops listening and refuses what is still in flight. Resume
    /// with .T.
    HttpClose {
        server: u32,
    },
    /// Park until CLEAR EVENTS; resume with Null.
    ReadEvents,
    ClearEvents,
    Quit,
    /// CANCEL: abort the program (the scheduler ends the session).
    Cancel,
    /// Resume with the chosen path or "".
    GetFile {
        extensions: String,
        title: String,
    },
    PutFile {
        prompt: String,
        default_name: String,
        extension: String,
    },
    /// Resume with the text (`Err` via `resume_error` when missing).
    FileRead {
        path: String,
        /// The other folders `SET PATH TO` named, each with the file's name on the end, to be
        /// tried in order when `path` is not there. See `Settings::search`.
        #[serde(default)]
        search: Vec<String>,
    },
    /// Resume with the number of bytes written.
    FileWrite {
        path: String,
        text: String,
        append: bool,
    },
    /// Resume with a logical.
    FileExists {
        path: String,
        /// Where else to look, as on `FileRead`: FILE() answers .T. for a file the path list
        /// leads to - measured.
        #[serde(default)]
        search: Vec<String>,
    },
    /// `ERASE`. Resume with a logical: was there a file to remove.
    FileDelete {
        path: String,
    },

    // ---- data engine ------------------------------------------------------------------------
    //
    // The host owns bytes; the VM owns meaning. Record numbers travel as f64, which is exact
    // past 2^53 records, so a table is not held to the 2 GB Visual FoxPro stops at. The host
    // never parses a record and the VM never sees a path after the open.
    /// A low-level file operation: FOPEN() and its family, ADIR(), FULLPATH(), COPY FILE and
    /// the rest. Which one is `op`; the other fields are whatever it needs. Resume with an array
    /// of two: the value, and the error number FERROR() then reports (0 for none).
    FileOp {
        op: String,
        handle: u32,
        path: String,
        target: String,
        text: String,
        count: f64,
        offset: f64,
        whence: u32,
        /// Where else to look for `path`, as on `FileRead`. Only the operations that look for a
        /// file that is already there fill it: measured, FOPEN() and LOCFILE() find a file
        /// through `SET PATH` and ADIR() does not.
        #[serde(default)]
        search: Vec<String>,
    },
    /// A function of a Windows library, declared with `DECLARE ... DLL` and now called.
    ///
    /// `params` is one VFP type word per argument, upper-cased, and `by_ref` says which of them
    /// the library is given the address of. Resume with the return value, or with an array whose
    /// first element is the return value and whose second is the arguments written back, when
    /// any parameter was by reference.
    CallDll {
        library: String,
        function: String,
        returns: String,
        params: Vec<String>,
        by_ref: Vec<bool>,
        args: Vec<JsonValue>,
    },
    /// `CREATE TABLE`: writes an empty table - the header given, then the end-of-file byte -
    /// and, when `memo` is set, an empty memo file beside it. Resume with anything; the VM
    /// opens the table with USE next.
    DataCreate {
        path: String,
        header: Vec<u8>,
        memo: bool,
    },
    /// `USE`: opens a table for reading. Resume with an array of
    /// `[handle, header bytes, memo block size]`, or raise the error.
    DataOpen {
        path: String,
        exclusive: bool,
        /// Where else to look for the table, as on `FileRead`.
        #[serde(default)]
        search: Vec<String>,
    },
    /// `count` records from `first` (1-based) as one block of bytes. A short block means the
    /// file ends there, which the caller notices rather than the host reporting it.
    DataRead {
        handle: u32,
        first: f64,
        count: u32,
    },
    /// A whole file of bytes, written as it stands: the database container and the memo file
    /// beside it are made this way. Resume with Null.
    FileWriteBytes {
        path: String,
        bytes: Vec<u8>,
    },
    /// The bytes of a file, whole, one character each. Resume with them, or with an empty
    /// string when there is no such file.
    FileReadBytes {
        path: String,
    },
    /// `LOADPICTURE()`: the object a program gets back, holding the four things a picture
    /// answers to. The pixels stay in the VM; this is a bag with a handle in it. Resume with
    /// the object.
    MakePicture {
        /// Where the VM is keeping the picture, which SAVEPICTURE() hands back. 0 is the null
        /// picture, which is what LOADPICTURE() with no file answers with.
        picture: f64,
        /// 1 a bitmap, 0 nothing at all.
        of_kind: f64,
        /// Both in HIMETRIC units, which is how Windows measures a picture.
        width: f64,
        height: f64,
    },
    /// A memo field's text, on its way to the file beside the table. Resume with the block it
    /// was written at, which is what goes in the record.
    DataWriteMemo {
        handle: u32,
        bytes: Vec<u8>,
    },
    /// The compound index beside a table, whole. Resume with its bytes, one character each,
    /// or with an empty string when the table has none.
    DataIndex {
        handle: u32,
    },
    /// Writes that index back, and marks the table as having one. Resume with Null.
    DataWriteIndex {
        handle: u32,
        bytes: Vec<u8>,
    },
    /// The payload of one memo block, by the block number stored in the record.
    DataReadMemo {
        handle: u32,
        block: f64,
    },
    /// Writes one record back where it already is. `count` is the table's new record count when
    /// the write added a record, which the host puts in the header at the same time so a reader
    /// never sees a record the header does not admit to. Resume with Null.
    DataWrite {
        handle: u32,
        recno: f64,
        bytes: Vec<u8>,
        count: Option<f64>,
    },
    /// `USE` with no table, CLOSE ALL, or a query letting go of what it borrowed. Every handle
    /// travels together so nothing is left open when several areas close at once.
    DataClose {
        handles: Vec<u32>,
    },
    /// `MODIFY COMMAND`, `MODIFY FORM`, `BROWSE`: opens a file in the development environment.
    /// Resume with Null. A host with nowhere to open it does nothing, which is what a packaged
    /// application should do with a command that asks to edit its own source.
    OpenDocument {
        path: String,
    },
    /// Resume with the module id after the host compiled and loaded the program.
    LoadProgram {
        name: String,
    },
    /// The character screen and the windows on it, as they now are. Resume with Null once
    /// the host has drawn them.
    SetScreen {
        screen: crate::screen::ScreenDoc,
    },
    /// `READ`: the fields `@ ... GET` put on the screen. Resume with an array of the values
    /// the user left in them, in the same order, or .F. when the read was cancelled.
    ReadGets {
        fields: Vec<GetField>,
    },
    /// `MENU TO`: the choices `@ ... PROMPT` put up. Resume with the number of the one that
    /// was chosen, counting from one, or 0 when none was.
    ChooseFrom {
        prompts: Vec<String>,
    },
    /// The printers the host can reach. `choose` asks it to put the chooser up and resume
    /// with the one that was picked; otherwise resume with an array of their names.
    Printers {
        choose: bool,
    },
    /// `BROWSE`: the records of a work area, for the host to show in a window of their own.
    /// Resume with Null once it is up, or when the user closed it where the command waits.
    Browse {
        browse: BrowseTable,
        /// Whether the program carries on without waiting for the window to be closed.
        nowait: bool,
    },
    /// The table's header, written over the front of the file. What an autoincrementing
    /// field takes next lives there, so it has to be kept as records are added. Resume with
    /// Null.
    DataWriteHeader {
        handle: u32,
        bytes: Vec<u8>,
    },
    /// The `A...()` functions that ask about something the host holds rather than the VM.
    /// Resume with an array of rows - each row an array of values, or a value of its own for a
    /// list of one column - or with Null when there is nothing to report.
    Enumerate {
        /// 0 the instances of a class, 1 the events bound to an object, 2 what the designer has
        /// selected, 3 the object under the pointer, 4 the class chosen from the Open Class
        /// dialog, 5 the classes in a class library, 6 a file's version information,
        /// 7 the network resources, 8 the base classes the runtime can make.
        what: u8,
        /// The class, object, library or file the question is about.
        name: String,
    },
    /// `GETENV()`: what the operating system says a name is set to. Resume with the text, or
    /// with an empty string when nothing is set under that name.
    Environment {
        name: String,
    },
    /// `MODIFY MEMO`: a window on what a memo field holds. Resume with the text the window
    /// was left with, or with Null when the command said NOWAIT and did not wait for it.
    EditMemo {
        /// The table the field belongs to.
        alias: String,
        field: String,
        text: String,
        /// The window shows the text and does not take changes to it.
        noedit: bool,
        /// The program carries on rather than waiting for the window to close.
        nowait: bool,
    },
    /// `CLOSE MEMO`: the windows `MODIFY MEMO` opened are closed. Resume with an array of
    /// alias, field and text for each one that was open, so what was typed is kept.
    CloseMemo {
        /// The fields to close windows for, or empty for all of them.
        fields: Vec<String>,
    },
    /// `MOUSE`: the pointer is put where the command said and pressed there, so whatever is
    /// under it reacts as it would to a hand. Resume with Null.
    MousePress {
        /// 0 moved only, 1 clicked, 2 double-clicked.
        clicks: u8,
        /// Where it goes first: a row and a column, or a pair of pixels with PIXELS.
        at: Option<(f64, f64)>,
        /// Where it is dragged, with the button held down.
        drag: Vec<(f64, f64)>,
        /// The window the positions are measured in, or empty for the main window.
        window: String,
        /// PIXELS, LEFT, MIDDLE, RIGHT, SHIFT, CONTROL and ALT, a space apart.
        style: String,
    },
    /// `RUN`: a command line for the operating system. Resume with the number it exited
    /// with, or with Null when the host has nowhere to run it.
    RunProgram {
        command: String,
        nowait: bool,
    },
    /// `FLUSH` and `DOEVENTS`: nothing is asked for, only that the host catch up with what
    /// the program has done before it goes on. Resume with Null.
    Settle {
        events: bool,
    },
    /// Resume with Null after the menu is installed.
    DoMenu {
        name: String,
    },
    /// `ACTIVATE MENU`: the menu the program built as it ran, in the shape a menu file's
    /// document takes, so the host puts it where a menu file's would go. Nothing means the
    /// menu comes down. Resume with Null once it is up.
    SetMenu {
        menu: Option<MenuDoc>,
    },
    /// The program has stopped and the debugger has it: the frame it stopped in, the line it is
    /// about to run, and what stopped it. The fiber stays parked until the host resumes it, so
    /// the environment is live while it waits. Set the fiber's step mode first when the developer
    /// asked to step rather than continue; resume with Null either way.
    Break {
        /// The routine the stopped frame belongs to, as `PROGRAM()` names it.
        program: String,
        line: u32,
        reason: BreakReason,
    },
    /// `RESUME`: let the program that is stopped carry on. Resume with Null.
    DebugResume,
    /// `REMOVEPROPERTY(obj, name)`: resume with a logical (.T. when the property was there).
    RemoveProperty {
        obj: u32,
        name: String,
    },
    /// `HOME([n])`: resume with a directory path. VFP's product directories do not exist here,
    /// so the host answers the project directory for `which == 0` and "" for the rest.
    HomeDir {
        which: u32,
    },
    /// `GETKEY()`: resume with the key code of the next key pressed.
    GetKey,
    /// `GETCOLOR([n])`: resume with the chosen RGB integer, or -1 when cancelled.
    /// `default` is -1 when the call named no starting colour.
    GetColor {
        default: f64,
    },
    /// `GETFONT()`: resume with "name,size,style", or "" when cancelled. Any of the three
    /// fields is empty/0 when the call did not preselect it.
    GetFont {
        name: String,
        size: f64,
        style: String,
    },
    /// `BINDEVENT(source, event, handler, delegate [, flags])`: resume with a logical.
    BindEvent {
        source: u32,
        event: String,
        handler: u32,
        delegate: String,
        flags: u32,
    },
    /// `UNBINDEVENT([source] [, event] [, handler] [, delegate])`: every field the call omitted
    /// is `None` and matches anything. Resume with a logical.
    UnbindEvent {
        source: Option<u32>,
        event: Option<String>,
        handler: Option<u32>,
        delegate: Option<String>,
    },
    /// `RAISEEVENT(source, event [, args...])`: resume with Null once the bound handlers ran.
    RaiseEvent {
        source: u32,
        event: String,
        args: Vec<JsonValue>,
    },
}

/// Why a program stopped where it did, so the debugger can say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BreakReason {
    /// A breakpoint the developer set on this line.
    Breakpoint,
    /// The step the developer asked for has finished.
    Step,
    /// `SUSPEND`.
    Suspend,
    /// `SET STEP ON`.
    SetStep,
}

/// How far a stopped program runs before it stops again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepMode {
    /// Until the next breakpoint, wherever that is.
    #[default]
    Go,
    /// To the next statement, whichever routine it belongs to.
    Into,
    /// To the next statement of this frame or of one that called it: a routine this statement
    /// calls runs whole.
    Over,
    /// Until this routine returns, at the next statement of whoever called it.
    Out,
}

/// A `DEFINE CLASS` resolved for the host: exactly what the class itself declares, with its
/// property values already folded to values. Inheritance is *not* flattened - the host walks the
/// chain, resolving each parent through `class_definitions` of the module the class came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDefOut {
    /// As written in the source.
    pub name: String,
    /// The parent as written: a VFP base class, or another class of the program.
    pub base_class: String,
    pub properties: Vec<ClassPropOut>,
    pub members: Vec<ClassMemberOut>,
    pub methods: Vec<ClassMethodOut>,
    /// Module the class (and its method bodies) live in.
    pub module: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassPropOut {
    pub name: String,
    pub value: JsonValue,
    /// The source of a value that is an expression, which the host works out when the object is
    /// made; `value` is only a placeholder then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassMemberOut {
    pub name: String,
    pub class: String,
    pub noinit: bool,
    pub properties: Vec<ClassPropOut>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassMethodOut {
    /// `Init`, or `image1.Click` for a method attached to a member.
    pub name: String,
    /// The class that declares the body (this class: the host flattens inheritance itself).
    pub owner: String,
    pub module: u32,
}

impl ClassDefOut {
    /// Serializes a compiled class for the host.
    pub fn new(module: u32, proto: &crate::bytecode::ClassProto) -> ClassDefOut {
        ClassDefOut {
            name: proto.name.clone(),
            base_class: proto.parent.clone(),
            properties: props_out(&proto.properties, &proto.expressions),
            members: proto
                .members
                .iter()
                .map(|m| ClassMemberOut {
                    name: m.name.clone(),
                    class: m.class.clone(),
                    noinit: m.noinit,
                    properties: props_out(&m.properties, &m.expressions),
                })
                .collect(),
            methods: proto
                .methods
                .iter()
                .map(|(name, _)| ClassMethodOut { name: name.clone(), owner: proto.name.clone(), module })
                .collect(),
            module,
        }
    }
}

fn props_out(props: &[(String, crate::bytecode::Constant)], expressions: &[(String, String)]) -> Vec<ClassPropOut> {
    let values = props.iter().map(|(name, c)| ClassPropOut { name: name.clone(), value: JsonValue::from_constant(c), expression: None });
    // an expression is worked out by the host when the object is made; until then it is .F.
    let worked_out = expressions
        .iter()
        .map(|(name, text)| ClassPropOut { name: name.clone(), value: JsonValue::Bool(false), expression: Some(text.clone()) });
    values.chain(worked_out).collect()
}

/// Values crossing the bridge. Objects travel as handles; arrays by value; dates as ISO text
/// so the JS side never has to know the VM's epoch conventions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Obj {
        #[serde(rename = "$obj")]
        obj: u32,
    },
    /// A lambda. It travels as its id in the VM's own table, the way an object travels as its
    /// handle in the host's: the host keeps it, hands it back, and the VM finds the same
    /// function. See docs/foxscript.md.
    Fun {
        #[serde(rename = "$fn")]
        function: u32,
    },
    /// A variable passed by reference to a method - `o.GetClassName(cAlias, @m.cLibrary)`. It
    /// travels as a number naming the variable's cell, with the value beside it for a host that
    /// only wants to read it; when the host runs the method's code the number comes back, and
    /// the parameter is that same cell, so what the method writes the caller sees.
    Ref {
        #[serde(rename = "$ref")]
        cell: u32,
        #[serde(rename = "$val")]
        value: Box<JsonValue>,
    },
    Date {
        #[serde(rename = "$date")]
        date: String,
    },
    DateTime {
        #[serde(rename = "$dt")]
        dt: f64,
    },
    Arr {
        #[serde(rename = "$arr")]
        arr: Vec<JsonValue>,
        #[serde(rename = "$cols")]
        cols: u32,
    },
}

impl JsonValue {
    pub fn from_value(v: &Value) -> JsonValue {
        match v.deref() {
            Value::Null => JsonValue::Null,
            Value::Logical(b) => JsonValue::Bool(b),
            Value::Number(n, ..) => JsonValue::Num(n),
            // the host has no money of its own; what crosses is the amount, and the type stays
            // behind in the VM where VARTYPE() reads it
            Value::Currency(c) => JsonValue::Num(c as f64 / crate::value::CURRENCY_SCALE as f64),
            Value::Str(s) => JsonValue::Str(s.to_string()),
            Value::Date(None) => JsonValue::Date { date: String::new() },
            Value::Date(Some(d)) => {
                let (y, m, day) = crate::value::civil_from_days(d);
                JsonValue::Date { date: format!("{y:04}-{m:02}-{day:02}") }
            }
            Value::DateTime(None) => JsonValue::DateTime { dt: f64::NAN },
            Value::DateTime(Some(t)) => JsonValue::DateTime { dt: t },
            Value::Object(h) => JsonValue::Obj { obj: h.0 },
            Value::Function(f) => JsonValue::Fun { function: f.0 },
            // a JSON value crosses as its text, which is what a host that wants to write it on
            // a socket needs; nothing in the host reaches into one
            Value::Json(j) => JsonValue::Str(j.to_string()),
            Value::Array(a) => {
                let a = a.borrow();
                JsonValue::Arr { arr: a.items.iter().map(JsonValue::from_value).collect(), cols: a.cols as u32 }
            }
            Value::Ref(_) => unreachable!("deref"),
        }
    }

    /// A compile-time constant (a class property value) in the bridge shape.
    pub fn from_constant(c: &crate::bytecode::Constant) -> JsonValue {
        use crate::bytecode::Constant;
        match c {
            Constant::Num(n, ..) => JsonValue::Num(*n),
            Constant::Money(c) => JsonValue::from_value(&Value::Currency(*c)),
            Constant::Str(s) => JsonValue::Str(s.clone()),
            Constant::Bool(b) => JsonValue::Bool(*b),
            Constant::Null => JsonValue::Null,
            Constant::Date(d) => JsonValue::from_value(&Value::Date(*d)),
            Constant::DateTime(t) => JsonValue::from_value(&Value::DateTime(*t)),
            Constant::Array(items) => JsonValue::Arr { arr: items.iter().map(JsonValue::from_constant).collect(), cols: 0 },
        }
    }

    /// A call's argument: a variable passed by reference crosses as a `Ref`, everything else as
    /// its value.
    pub fn from_arg(v: &Value) -> JsonValue {
        match v {
            Value::Ref(cell) => JsonValue::Ref { cell: ref_cells::hold(cell), value: Box::new(JsonValue::from_value(v)) },
            other => JsonValue::from_value(other),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            // the variable itself, when it is still there; otherwise the value it crossed with
            JsonValue::Ref { cell, value } => ref_cells::find(*cell).map_or_else(|| value.to_value(), Value::Ref),
            JsonValue::Null => Value::Null,
            JsonValue::Bool(b) => Value::Logical(*b),
            JsonValue::Num(n) => Value::number(*n),
            JsonValue::Str(s) => Value::str(s),
            JsonValue::Obj { obj } => Value::Object(Handle(*obj)),
            JsonValue::Fun { function } => Value::Function(crate::value::FuncId(*function)),
            JsonValue::Date { date } => Value::Date(parse_iso_date(date)),
            JsonValue::DateTime { dt } => Value::DateTime(if dt.is_nan() { None } else { Some(*dt) }),
            JsonValue::Arr { arr, cols } => {
                let cols = *cols as usize;
                let rows = if cols == 0 { arr.len() } else { arr.len().div_ceil(cols.max(1)) };
                let mut fa = crate::value::FoxArray::new(rows, cols);
                for (i, v) in arr.iter().enumerate() {
                    if i < fa.items.len() {
                        fa.items[i] = v.to_value();
                    }
                }
                Value::Array(std::rc::Rc::new(std::cell::RefCell::new(fa)))
            }
        }
    }
}

fn parse_iso_date(s: &str) -> Option<i32> {
    let mut it = s.split('-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if crate::value::is_valid_date(y, m, d) { Some(crate::value::days_from_civil(y, m, d)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_values_round_trip() {
        let vals = [
            Value::Null,
            Value::Logical(true),
            Value::number(2.5),
            Value::str("x"),
            Value::Date(Some(19723)),
            Value::Date(None),
            Value::DateTime(Some(12.0)),
            Value::Object(Handle(3)),
        ];
        for v in vals {
            assert_eq!(JsonValue::from_value(&v).to_value(), v);
        }
        assert_eq!(JsonValue::from_value(&Value::Date(Some(0))), JsonValue::Date { date: "1970-01-01".into() });
    }
}

/// A menu a program built while it ran, named the way a menu document names things so the host
/// can put it exactly where a menu file's would go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuDoc {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: u8,
    pub name: String,
    /// Whether it replaces the menu that is up or joins it: always "Replace" here, because
    /// `ACTIVATE MENU` puts its own menu up in place of whatever was there.
    pub location: String,
    pub items: Vec<MenuDocItem>,
}

/// One pad of that menu, or one bar of one of its popups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuDocItem {
    pub id: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub result: MenuDocResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<MenuDocKey>,
    #[serde(rename = "skipFor", default, skip_serializing_if = "Option::is_none")]
    pub skip_for: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<MenuDocItem>>,
}

/// What choosing it does: open the popup under it, or run the command it stands for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuDocResult {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// The key a `KEY` clause gave it, split into the shape a menu document keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuDocKey {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctrl: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// One field a `READ` is waiting on: where it is on the screen, how wide, and what it holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetField {
    /// The variable or field it reads and writes, as the program wrote it.
    pub name: String,
    pub row: usize,
    pub col: usize,
    pub width: usize,
    /// The PICTURE clause it was given, which says what may be typed into it.
    pub picture: String,
    pub enabled: bool,
    pub value: JsonValue,
}

/// A work area as a Browse window shows it: the columns, the rows, and what it is called.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowseTable {
    /// The work area's alias, which is what the window is called when nothing else says.
    pub alias: String,
    pub title: String,
    /// The columns, in the order they are shown.
    pub columns: Vec<BrowseColumn>,
    /// The rows, each one value per column, already worked out by the VM.
    pub rows: Vec<BrowseRow>,
    /// How many records the work area holds, which is more than `rows` when it is a long table.
    pub count: u64,
    /// Whether the user may change what is in it.
    pub editable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowseColumn {
    pub name: String,
    /// The one letter the field's type is written as, and how wide it is.
    pub kind: String,
    pub width: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowseRow {
    /// The record number, so a program that browses a filtered table still knows which is which.
    pub recno: u64,
    pub deleted: bool,
    pub values: Vec<JsonValue>,
}

/// The variables passed by reference that are out in the host, by the number they crossed as.
///
/// Held weakly: the variable belongs to the routine that declared it, and a number the host keeps
/// after that routine has returned finds nothing, rather than keeping the variable alive.
mod ref_cells {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};

    use crate::value::Value;

    thread_local! {
        static CELLS: RefCell<(u32, HashMap<u32, Weak<RefCell<Value>>>)> = RefCell::new((0, HashMap::new()));
    }

    pub fn hold(cell: &Rc<RefCell<Value>>) -> u32 {
        CELLS.with(|c| {
            let mut c = c.borrow_mut();
            // numbers for variables that have gone are let go of now and then
            if c.1.len() > 4096 {
                c.1.retain(|_, w| w.strong_count() > 0);
            }
            c.0 = c.0.wrapping_add(1);
            let id = c.0;
            c.1.insert(id, Rc::downgrade(cell));
            id
        })
    }

    pub fn find(id: u32) -> Option<Rc<RefCell<Value>>> {
        CELLS.with(|c| c.borrow().1.get(&id).and_then(Weak::upgrade))
    }
}
