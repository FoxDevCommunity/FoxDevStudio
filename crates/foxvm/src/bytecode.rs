//! Bytecode: the serializable output of the compiler and the input of the VM (VFP's p-code
//! analogue). A stack machine with a deliberately fat instruction set so the interpreter loop
//! is one `match` and statement boundaries are explicit (line tracking, future breakpoints).
//!
//! Stack conventions are documented per instruction as `[before] -> [after]`, top on the right.

use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8; 4] = b"FXVM";
pub const FORMAT_VERSION: u16 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    /// A .prg: `funcs[0]` is the implicit main, the rest are its PROCEDUREs/FUNCTIONs.
    Program,
    /// A form: one function per method with source, listed in `methods`; no main.
    Form,
    /// Menu command/procedure text, EXECSCRIPT, command window lines: `funcs[0]` is the body.
    Snippet,
    /// EVALUATE / `&macro` / skipFor: `funcs[0]` pushes one value and returns it.
    Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Constant {
    /// A number and the width it was written in: the characters it took, and how many of them
    /// were past the point. `? 001` prints "  1" because the constant remembers the three.
    Num(f64, u8, u8),
    /// `$12.34` as it was written: money, in ten-thousandths.
    Money(i64),
    Str(String),
    /// Days since 1970-01-01; `None` is the empty date.
    Date(Option<i32>),
    /// Seconds since 1970-01-01; `None` is the empty datetime.
    DateTime(Option<f64>),
    /// `.T.` / `.F.`; only class property values need these (code pushes `True`/`False`).
    Bool(bool),
    /// `.NULL.`, likewise.
    Null,
    /// `DIMENSION aRGB[3]` in a class body: an array property, one dimension.
    Array(Vec<Constant>),
}

impl Constant {
    /// A number the compiler needed for itself - the 1 a FOR loop steps by, the 0 a flag starts
    /// at - as wide as that number reads when it is written out.
    pub fn num(n: f64) -> Constant {
        let text = crate::value::to_places(n, crate::value::places_needed(n) as usize);
        let width = crate::value::Width::written(text.trim_start_matches('-'));
        Constant::Num(n, width.chars, width.decimals)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub kind: ModuleKind,
    pub consts: Vec<Constant>,
    /// Variable, procedure and setting names, upper-cased.
    pub names: Vec<String>,
    /// Property and method names as written (compared case-insensitively by the host).
    pub members: Vec<String>,
    pub funcs: Vec<FuncProto>,
    /// Form modules: `"OBJPATH.EVENT"` (upper-cased, path relative to the form, empty path for
    /// the form itself, e.g. `"PGFMAIN.PAGE1.LBLGREETING.CLICK"` or `".INIT"`) -> index into `funcs`.
    ///
    /// Program modules also list the methods of every `DEFINE CLASS` here, keyed
    /// `"CLASSNAME.EVENT"` and `"CLASSNAME.MEMBER.EVENT"` (upper-cased).
    pub methods: Vec<(String, u32)>,
    /// `DEFINE CLASS` declarations of a program module, in source order.
    pub classes: Vec<ClassProto>,
    /// One entry per SELECT-SQL statement compiled into this module.
    pub queries: Vec<crate::query::QueryPlan>,
    /// One entry per `CREATE CURSOR`: the alias, and the columns it makes.
    pub cursors: Vec<(String, Vec<ColumnDef>)>,
    /// One entry per `DECLARE ... DLL`, in source order.
    pub dlls: Vec<DllProto>,
}

/// One column of a `CREATE CURSOR`, `CREATE TABLE` or `ALTER TABLE` as the statement wrote it.
///
/// Everything but nullability is settled by the declaration itself. Whether the column takes
/// `.NULL.` is not: a column that said neither NULL nor NOT NULL takes whatever `SET NULL` is
/// when the statement runs, and that is not known while it is being compiled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub field: crate::dbf::DbfField,
    #[serde(default)]
    pub nullable: Option<bool>,
}

impl ColumnDef {
    /// The column as the table will hold it, with `SET NULL` deciding what the declaration
    /// left open.
    pub fn field(&self, nulls_by_default: bool) -> crate::dbf::DbfField {
        self.field.clone().accepting_null(self.nullable.unwrap_or(nulls_by_default))
    }
}

/// One change an `ALTER TABLE` makes to a table: 0 adds a column, 1 alters one, 2 drops one,
/// 3 renames one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterStep {
    pub kind: u8,
    /// The column the change is to, as a name index, or None when the program worked the name
    /// out and left it on the stack.
    pub name: Option<u32>,
    /// Adding and altering: the column's new shape, in `Module::cursors`. Renaming: the new
    /// name, or None when that was worked out as well.
    pub extra: Option<u32>,
}

/// A function of a Windows library, as `DECLARE ... DLL` describes it. The library itself is
/// evaluated when the declaration runs, so it is not part of the proto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DllProto {
    /// The name inside the library.
    pub function: String,
    /// What the program calls it: the alias, or the function's own name. Upper-cased.
    pub called: String,
    /// The VFP type word of the return value, upper-cased; empty when the declaration omits it.
    pub returns: String,
    /// The VFP type word of each parameter, upper-cased, and whether it is passed by reference.
    pub params: Vec<(String, bool)>,
}

/// A class defined in source with `DEFINE CLASS ... ENDDEFINE`. Inheritance is *not* flattened
/// here: a proto carries only what its own declaration says, and the host walks the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassProto {
    /// As written in the source; compared case-insensitively.
    pub name: String,
    /// The parent class as written: a VFP base class or another class of the program.
    pub parent: String,
    /// Property values, constant-folded at compile time, in source order.
    pub properties: Vec<(String, Constant)>,
    pub members: Vec<MemberProto>,
    /// `("Init", func)` / `("image1.Click", func)`: the name as written and the function index.
    pub methods: Vec<(String, u32)>,
}

/// Where `GO` puts the record pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoTarget {
    Top,
    Bottom,
    /// The record number is on the stack.
    Record,
}

/// One `ADD OBJECT` member of a [`ClassProto`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberProto {
    pub name: String,
    pub class: String,
    pub noinit: bool,
    pub properties: Vec<(String, Constant)>,
}

impl Module {
    pub fn find_func(&self, upper_name: &str) -> Option<u32> {
        self.funcs.iter().position(|f| f.name.eq_ignore_ascii_case(upper_name)).map(|i| i as u32)
    }
    pub fn find_method(&self, obj_path_upper: &str, event_upper: &str) -> Option<u32> {
        let key = format!("{obj_path_upper}.{event_upper}");
        self.methods.iter().find(|(k, _)| *k == key).map(|(_, f)| *f)
    }
    /// A `DEFINE CLASS` of this module by name, case-insensitively.
    pub fn find_class(&self, name: &str) -> Option<&ClassProto> {
        self.classes.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuncProto {
    /// Procedure name (upper-cased), `MAIN` for a program body, `OBJPATH.EVENT` for methods.
    pub name: String,
    /// Name as shown in error messages and `PROGRAM()`: the program name for a main body, the
    /// procedure name as written, `objPath.Event` for methods.
    pub display_name: String,
    /// Number of declared parameters; they occupy local slots `0..nparams` in order.
    pub nparams: u32,
    /// Slot -> upper-cased name for every LOCAL/LPARAMETERS variable (params first). Lets
    /// runtime-compiled code (`EVALUATE`, `&macro`) resolve locals by name.
    pub locals: Vec<String>,
    /// The source line of the first statement this function runs - one past the
    /// `PROCEDURE`/`FUNCTION` that opened it, or the program's own first line for a main body.
    /// What `LINENO(1)` counts from; `LINENO()` always counts from the main program's own first
    /// line instead, whatever is running.
    pub def_line: u32,
    /// Where a lambda's captured values go: the slot in the enclosing frame each one is read
    /// from when the lambda is made, and the slot in this function's own frame it is written to
    /// when the lambda is called. Empty for everything that is not a lambda.
    pub captures: Vec<Capture>,
    pub code: Vec<Instr>,
}

/// One captured LOCAL of an enclosing routine. A local is a slot in a frame and the frame is
/// gone by the time a lambda runs, so the value is copied at the moment the lambda is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capture {
    pub from: u32,
    pub to: u32,
}

/// Target of `Dim`, `Ref` and FOR loops: a local slot or a dynamically scoped name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Var {
    Local(u32),
    Name(u32),
}

/// Flags for `WaitWindow` (bit set).
/// `COMPILE`'s clauses.
pub mod compile_flags {
    /// `ALL`: every file the skeleton matches, not only the ones that changed.
    pub const ALL: u8 = 1;
    /// `ENCRYPT`: object code without the source it came from.
    pub const ENCRYPT: u8 = 2;
    /// `NODEBUG`: object code without the line table a debugger reads.
    pub const NODEBUG: u8 = 4;
}

pub mod wait_flags {
    pub const HAS_TEXT: u8 = 1;
    pub const NOWAIT: u8 = 2;
    pub const HAS_TIMEOUT: u8 = 4;
    pub const CLEAR: u8 = 8;
    /// `TO var`: the key pressed (the resumed value, "" for Null) is pushed for the following store.
    pub const WANT_KEY: u8 = 16;
}

/// Absent jump target in `TryPush` (no CATCH / no FINALLY).
pub const NO_TARGET: u32 = u32::MAX;

/// Flags for `DoForm` (bit set).
pub mod form_flags {
    pub const LINKED: u8 = 1;
    pub const NOSHOW: u8 = 2;
    /// `TO var`: the form's Unload return value is pushed.
    pub const WANT_RESULT: u8 = 4;
    /// `NAME var`: the created form object is pushed (before the result, if both).
    pub const WANT_OBJECT: u8 = 8;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instr {
    // ---- statements
    /// Statement boundary: records the current source line (and is the breakpoint hook).
    Stmt(u32),

    // ---- constants: [] -> [v]
    Const(u32),
    True,
    False,
    Null,
    /// The value of a skipped argument (`f(1,,3)`): .F.
    Omitted,

    // ---- stack
    Pop,
    Dup,

    // ---- variables
    /// [] -> [v]
    LoadLocal(u32),
    /// [v] -> []
    StoreLocal(u32),
    /// Dynamic lookup: privates in this frame, then callers, then PUBLIC. [] -> [v]
    LoadName(u32),
    /// Dynamic store; an unknown name becomes a PRIVATE of the current frame. [v] -> []
    StoreName(u32),
    /// Sets the slot to .F. (LOCAL declaration).
    DeclLocal(u32),
    /// Declares a PRIVATE (hides any outer variable of that name) initialised to .F.
    DeclPrivate(u32),
    /// Declares a PUBLIC initialised to .F. (keeps an existing value).
    DeclPublic(u32),
    /// [name] -> []   `PRIVATE (cName)` / `PUBLIC (cName)`: the same declaration for a name
    /// the program worked out when it ran.
    DeclareNamed {
        public: bool,
    },
    /// [value, name] -> []   `STORE x TO (cName)`: writes the value to whatever the name
    /// names - a variable, or a property some way down an object - which is only known when
    /// it runs. The name is read as a target and the value taken off the stack by the code
    /// compiled for it, so anything that can be assigned to can be named here.
    StoreByName,
    /// `PARAMETERS`-style: pops the n-th argument and stores it as a PRIVATE. [] -> []
    ParamToPrivate {
        arg: u32,
        name: u32,
    },
    /// Creates an array in the target. [dims...] -> []
    Dim {
        target: Var,
        ndims: u8,
    },
    /// [array, i (, j)] -> [v]
    LoadIndex(u8),
    /// [v, array, i (, j)] -> []
    StoreIndex(u8),
    /// Pushes a by-reference cell for the variable (converts it in place). [] -> [ref]
    Ref(Var),
    /// The same, for a name that need not exist yet: an array function is handed the array it
    /// is to fill, and Visual FoxPro makes one when the program never declared it. [] -> [ref]
    RefOrMakeArray(Var),
    /// Releases variables: `RELEASE a, b`.
    ReleaseName(u32),
    /// `RELEASE` of a LOCAL. A local lives in a numbered slot, so there is nowhere to take it
    /// from: the slot is marked instead, and reads as a variable that is not there until
    /// something writes it again.
    ReleaseLocal(u32),
    ReleaseAll,

    // ---- operators: [a, b] -> [r] / [a] -> [r]
    Neg,
    Not,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    ExactEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    /// Three-valued AND/OR applied after the short-circuit jump kept the left operand.
    And,
    Or,

    // ---- control flow (targets are absolute pcs)
    Jump(u32),
    /// Pops; jumps when .F. or NULL; errors on a non-logical.
    JumpIfFalse(u32),
    JumpIfTrue(u32),
    /// Short-circuit for AND: peeks (never pops); jumps when the top is .F. The left operand stays
    /// on the stack either way, so `a AND b` compiles to `a; JumpIfFalseKeep(end); b; And; end:`.
    JumpIfFalseKeep(u32),
    /// Short-circuit for OR: peeks; jumps when the top is .T.
    JumpIfTrueKeep(u32),
    /// FOR loop test. [var, end, step] -> [] ; jumps to the exit when the loop is finished.
    ForTest(u32),
    /// FOR EACH support: [array, index] -> [array, index+1, element]; at the end pops both and
    /// jumps to the exit.
    ForEachNext(u32),

    // ---- calls
    /// `name(args)`: array element access when `name` resolves to an array, else a user function
    /// (this module, then loaded programs). [args...] -> [v]
    IndexOrCall {
        name: u32,
        argc: u8,
    },
    /// Built-in function by id (see `builtins::REGISTRY`). [args...] -> [v]
    CallBuiltin {
        id: u16,
        argc: u8,
    },
    /// `DO name [WITH args] [IN prog]`: procedure call as a statement. [args...] -> []
    Do {
        name: u32,
        argc: u8,
        /// A program name went on the stack under the arguments.
        in_prog: bool,
    },
    /// [name, args...] -> []   `DO (cProgram)`: the name is computed, not written in the source.
    DoDynamic {
        argc: u8,
        /// A program name went on the stack under the arguments.
        in_prog: bool,
    },
    /// [v] -> returns v to the caller (RETURN without a value pushes .T. first).
    Return,
    /// `LAMBDA(...) ... ENDLAMBDA`: builds a function value out of the function at this index,
    /// the captured slots its proto lists, and the frame's THIS. [] -> [f]
    MakeLambda(u32),
    /// `f(args)` where `f` is a local: the value under the arguments is subscripted when it is
    /// an array and called when it is a function, which is the same question `IndexOrCall` asks
    /// about a name. [v, args...] -> [result]
    IndexOrCallValue(u8),

    // ---- objects
    LoadThis,
    LoadThisForm,
    /// `THISFORMSET`: the formset the object belongs to. A formset is a container whose members
    /// are whole forms, shown together, so this is not the form above the object but the thing
    /// above that; an object in no formset raises error 1938 rather than answering.
    LoadThisFormSet,
    LoadScreen,
    /// [obj] -> [value or child object]
    GetMember(u32),
    /// [obj, name] -> [value]   the same, for a member named by a variable: `obj.&cName`.
    GetMemberByName,
    /// `obj.&cName = v`: the member a variable names is written. [value, obj, name] -> []
    SetMemberByName,
    /// [obj, name, a1 .. an] -> [value]   (yields HostRequest::CallMethod) for `obj.&cName(...)`.
    CallMethodByName(u8),
    /// [v, obj] -> []   (yields HostRequest::SetProp)
    SetMember(u32),
    /// [v, obj, subs...] -> []   (yields HostRequest::SetPropIndex) for `obj.Prop[1] = v`
    SetMemberIndex {
        name: u32,
        argc: u8,
    },
    /// [obj, d1 .. dn] -> []   (yields HostRequest::DimProp) for `DIMENSION obj.aProp[2, 3]`.
    ///
    /// Sizing an array property is its own instruction rather than an assignment of a fresh
    /// array, because the two are different statements: `DIMENSION` keeps the elements that
    /// still fit, and a plain assignment of an array to a property is not a way to give a
    /// property an array at all.
    DimMember {
        name: u32,
        ndims: u8,
    },
    /// [obj, args...] -> [v]   (yields HostRequest::CallMethod)
    CallMethod {
        name: u32,
        argc: u8,
    },
    /// [obj] -> []  pushes onto the WITH stack
    PushWith,
    PopWith,
    /// [] -> [obj]
    LoadWith,
    /// [obj] -> []  (yields HostRequest::ReleaseObject)
    ReleaseObject,
    /// `DO menu.fxm`: installs a menu. [] -> []  (yields HostRequest::DoMenu)
    DoMenu(u32),

    // ---- runtime compilation
    /// `&name` in an expression: compiles the text on the stack as an expression in this
    /// frame. [text] -> [v]
    Macro,
    /// TEXTMERGE `<<expr>>`: replaces the top of the stack with its display text. [v] -> [text]
    ToText,
    /// Text merge: replaces the raw text on the stack with the same text, every expression
    /// between the merge delimiters worked out and put in its place. Which characters those
    /// delimiters are, and whether the merge happens at all, is what SET TEXTMERGE says at the
    /// time; `always` is the TEXTMERGE clause of TEXT, which merges whatever the setting says.
    /// [raw] -> [text]
    MergeText {
        always: bool,
    },
    /// `\`, `\\` and a TEXT block with nowhere to go: sends the text on the stack to wherever
    /// SET TEXTMERGE TO points. `newline` ends the line before it first, and `noshow` is the
    /// command's own NOSHOW, which stops it also being shown. [text] -> []
    TextOut {
        newline: bool,
        noshow: bool,
    },

    // ---- statements with host effects
    /// `?` / `??`. [items...] -> []
    Print {
        newline: bool,
        argc: u8,
    },
    /// [text? , timeout?] -> [] per `wait_flags`; `TO var` is a following StoreName.
    WaitWindow(u8),
    ReadEvents,
    ClearEvents,
    Quit,
    Cancel,
    /// [name, args...] -> [obj?, result?] per `form_flags`.
    DoForm {
        flags: u8,
        argc: u8,
    },
    /// `SET name ON|OFF|TO args`. [args...] -> [] ; ON/OFF push True/False first.
    ///
    /// `to` says the command was the TO form. One setting can hold two things at once - `SET
    /// HELP ON` and `SET HELP TO afile` are both remembered, and `SET("HELP")` answers the
    /// first where `SET("HELP", 1)` answers the second - so which of them the command wrote
    /// has to travel with the instruction. The values alone cannot say.
    SetCmd {
        name: u32,
        argc: u8,
        to: bool,
    },
    NoDefault,
    /// [args...] -> [v]
    DoDefault(u8),

    // ---- exceptions
    /// Installs a handler; either target may be `NO_TARGET`. On an error the VM unwinds to the
    /// stack depth at the push, then jumps to `catch` (installing a finally-only handler for the
    /// CATCH block) or, without CATCH, to `finally` with the error pending for `EndFinally`.
    TryPush {
        catch: u32,
        finally: u32,
    },
    /// Removes the innermost handler; when it has a FINALLY block a "no error pending" entry is
    /// recorded and the compiler jumps to the block next.
    TryPop,
    /// [path] -> []   (yields HostRequest::FileDelete) for `ERASE`.
    Erase,

    // ---- data
    /// [path] -> []   `USE table [ALIAS x] [EXCLUSIVE|SHARED] [AGAIN]`; an empty path closes the
    /// work area. `alias` is the name index of an explicit ALIAS, if there was one.
    Use {
        alias: Option<u32>,
        /// An alias went on the stack under the path, worked out when the statement ran.
        named_alias: bool,
        exclusive: bool,
        /// `ONLINE` or `ADMIN`: the table opens, and then the command answers for what those
        /// two ask of it, which only an offline view can give.
        online: bool,
        /// True when a work area was pushed under the path: `USE x IN 0` opens the table
        /// somewhere else and leaves the selected area where it was.
        in_area: bool,
    },
    /// `SELECT n` / `SELECT alias`: `name` is a name index, or none when a number is on the stack.
    SelectArea(Option<u32>),
    /// `GO TOP` / `GO BOTTOM` / `GO n` (the record number is on the stack for `Record`).
    Go(GoTarget),
    /// `SKIP [n]`, with the count on the stack.
    Skip,
    /// [area] -> []   The work area a command says `IN`: it is selected, and what was selected
    /// before is remembered so `PopArea` can put it back.
    PushArea,
    /// [] -> []   The work area `PushArea` remembered, selected again.
    PopArea,
    /// `USE` with nothing open to close, or `CLOSE TABLES`/`CLOSE ALL`.
    CloseTables {
        all: bool,
    },
    /// [flag] -> []   what FOUND() will report for the selected work area, after a LOCATE.
    SetFound,
    /// [value] -> []   Changes one field of the record the pointer is on, in the page in hand.
    /// `additive` is `REPLACE ... ADDITIVE`, which adds to a memo field rather than replacing it.
    ReplaceField {
        field: u32,
        additive: bool,
    },
    /// [name, value] -> []   `REPLACE (expr) WITH value`: the field the name works out to.
    ReplaceFieldNamed {
        additive: bool,
    },
    /// [] -> []   DELETE and RECALL: the flag at the front of the record.
    MarkDeleted(bool),
    /// ZAP: empties the selected table.
    Zap,
    /// A file command: 1 COPY FILE, 2 RENAME, 3 MD, 4 RD, 5 DIR, 6 TYPE. Pops its names.
    FileCommand(u8),
    /// DECLARE ... DLL: pops the library name and registers the declaration at that index.
    DeclareDll(u32),
    /// The routine ran out of statements: it returns the value on top of the stack, and stops
    /// there. A `RETURN` the program wrote goes further - out of a line a macro put together
    /// and out of the routine that line belongs to - so the two are not the same instruction.
    EndOfCode,
    /// A command with a macro in it, kept as the source text at that constant: every `&name`
    /// in it is replaced by the text that variable holds, and what comes out is compiled and
    /// run in this frame.
    ExecMacroText(u32),
    /// CREATE TABLE: pops a path and asks the host to write an empty table of the fields in
    /// `Module::cursors` at that index. With `from_array` the columns are described by an array
    /// on the stack under the path instead, and the module holds none.
    CreateTable {
        /// The definition in the module: the columns the new table has.
        index: u32,
        from_array: bool,
        /// How many column names went on the stack instead, worked out when it ran.
        columns: u8,
    },
    /// [path] -> [logical]   Asks the open database whether a table of its own may be made:
    /// dbc_BeforeCreateTable, answering whether the statement is to go ahead. A .F. stops the
    /// whole of CREATE TABLE - no file is written and no work area taken - so the rest of the
    /// statement is jumped over rather than run. A free table never reaches this.
    CreateTableAllowed,
    /// ERROR: pops a message (or Omitted) and a number or text, and raises it.
    RaiseError,
    /// [] -> []   APPEND BLANK: an empty record at the end, which the pointer moves to.
    AppendBlank,
    /// [] -> []   `INSERT [BEFORE] [BLANK]`: an empty record beside the one the pointer is on,
    /// with everything below it moved down one. The pointer ends on the new record.
    InsertBlank {
        before: bool,
    },
    /// [source, row] -> [more, next]   One record of `INSERT INTO ... FROM ARRAY | MEMVAR |
    /// NAME`: appends a blank and fills it from that row of the source, then says whether
    /// another row follows and which it is. An array of two dimensions holds a record per row,
    /// so the statement loops over this; the other two sources have the one row.
    InsertFrom {
        /// 0 an array, 1 a variable per field, 2 an object - as GATHER numbers them.
        from: u8,
    },
    /// [array] -> []   `CREATE CURSOR name FROM ARRAY a`: a cursor whose columns the array
    /// describes, in the shape `AFIELDS()` hands back. The name works the same way as
    /// `CreateCursor`'s.
    CreateCursorFromArray {
        /// The name written in the statement, when it was not worked out on the stack.
        index: u32,
        named: bool,
    },
    /// [] -> []   Sends the changed record back to the host, if anything changed it.
    FlushRecord,
    /// [path] -> []   `CREATE FORM`, `CREATE MENU` and the rest: a new one of that kind, in
    /// its designer. The constant says which kind.
    NewDocument(u32),
    /// `IMPORT`: the file is on the stack, with the sheet name over it when there is one.
    /// Reads the workbook, gathers the table it describes, and pushes where it is to go.
    Import { sheet: bool },
    /// `BUILD`: the kind is a constant, the target and `count` sources are on the stack.
    Build { what: u32, count: u16, recompile: bool },
    /// `COMPILE`: the kind is a constant and the file expression is on the stack.
    Compile { what: u32, flags: u8 },
    /// [path] -> []   `MODIFY TABLE`, `MODIFY VIEW`, `MODIFY PROCEDURE` and `MODIFY DATABASE`:
    /// the designer for something the open database holds, which the database is told about
    /// either side of. 0 a table, 1 a view, 2 the stored procedures, 3 the database itself.
    ModifyInContainer {
        what: u8,
        /// `NOWAIT` and `NOEDIT`, which dbc_ModifyData is handed.
        flags: u16,
    },
    /// [] -> []   A command this runtime cannot honour: raises "Feature is not available" with
    /// the constant's text when the line is reached. It compiles, as it does in the product.
    Unsupported(u32),
    /// [] -> []   Assigning to THIS, THISFORM, THISFORMSET or _SCREEN: "Cannot redefine X.",
    /// which the constant names. The line compiles, as it does in the product.
    Redefined(u32),
    /// [path] -> []   (yields HostRequest::OpenDocument) for MODIFY and BROWSE. The constant
    /// holds the word that followed MODIFY, because each kind of document brings its own
    /// extension when the name written has none: MODIFY DATABASE dvds opens dvds.dbc.
    OpenDocument(u32),
    /// [] -> [path]   The file behind the selected work area, for BROWSE.
    LoadTablePath,
    /// [d1 .. dn] -> [array]   A fresh array of that shape, for `DIMENSION` of something that is
    /// not a variable: an array property, which is assigned to rather than declared.
    MakeArray(u8),
    /// [] -> []   `CREATE CURSOR`: an empty table of that shape, in a free work area.
    CreateCursor {
        /// The definition in the module: its columns, and the name it was written with.
        index: u32,
        /// A name went on the stack instead, worked out when the statement ran.
        named: bool,
        /// How many column names went on the stack too, under that name.
        columns: u8,
    },
    /// [value] -> []   Changes the field at that position of the record the pointer is on, for an
    /// `INSERT` that gave its values in the table's own order.
    ReplaceFieldAt(u8),
    /// [] -> []   `CLEAR MEMORY` and, with `tables`, `CLEAR ALL`.
    ClearAll {
        tables: bool,
    },

    // ---- SELECT-SQL. The loops are ordinary bytecode; these four hold the query together.
    /// [name?] -> []   Opens (or borrows) one FROM source and selects it. `table` is a
    /// constant and `alias` a name index; `named` says the name is on the stack instead,
    /// which is what `FROM (cPath)` compiles to.
    SqlOpen {
        table: u32,
        alias: u32,
        named: bool,
    },
    /// [] -> []   `RETURN TO`: leave every routine between here and the one this names, which
    /// is the program the run started in when the name is empty (`RETURN TO MASTER`).
    ReturnTo(u32),
    /// [] -> []   Selects the work area of the query's nth FROM source. The source is named
    /// by where it is in the FROM clause rather than by what it is called, because a source
    /// named by an expression is only called something once it has been opened.
    SelectSource(u16),
    /// [] -> []   Selects the nth source and reads its current record into the buffer. A `*`
    /// in the select list takes the record straight from there, and nothing else in a query
    /// with only a `*` would have asked the host for the page.
    SqlTouch(u16),
    /// [] -> []   The nth FROM source has no record matching the row being built, so it is
    /// parked past its last record and every field of it reads as .NULL. until it is moved
    /// again. That is what an outer join puts on the side that missed.
    JoinMiss(u16),
    /// [top?] -> []   Starts gathering rows for the plan at `plan`, expanding its `*` columns
    /// against the sources just opened.
    SqlBegin(u32),
    /// [v1 .. vn] -> []   One gathered row: the column values, then the GROUP BY keys, then the
    /// ORDER BY keys.
    SqlRow(u16),
    /// [] -> []   Folds, sorts and installs the result, then lets go of the sources.
    SqlEnd,
    /// [] -> [logical]   Folds the gathered rows the first time, then moves the HAVING clause on
    /// to the next group and says whether there is one. The three that follow are the loop the
    /// compiler emits between the gathering and `SqlEnd`: HAVING is an expression of the
    /// program's own, so it is run as bytecode, a group at a time.
    SqlHavingNext,
    /// [] -> [value]   The nth thing the plan's HAVING clause names, out of the group being
    /// asked about.
    SqlHavingValue(u16),
    /// [logical] -> []   What the predicate came to for that group: keeps it or drops it.
    SqlHavingKeep,
    /// [] -> [value]   The field of that name in whichever of the query's sources has one, read
    /// from the record that source is on, or .NULL. when none of them has such a field. It is
    /// what a HAVING clause gathers for a name the select list gave with AS, since the name
    /// itself means nothing to a record but a field of the same name would shadow it.
    SqlSourceField(u32),
    /// [] -> []   Raises 1807 unless SET ENGINEBEHAVIOR 70 is in force. It stands in front of a
    /// query whose HAVING clause names an aggregate the select list has not got and which
    /// groups by nothing, which the older rules allow and the current ones do not.
    SqlRequireGroupBy,
    /// [value] -> [logical]   Is the value one of those in the first column of that cursor: what
    /// `IN (SELECT ...)` and `= ANY (SELECT ...)` come to once the subquery has been run.
    InCursor(u32),
    /// [] -> [value]   a field, or a member: `area` is a bare name the VM resolves - an object
    /// variable, the `m.` memory-variable prefix, or the alias of an open table.
    LoadField {
        area: Option<u32>,
        field: u32,
    },
    /// Returns from this frame and runs the statement that called it again, for `RETRY`.
    Retry,
    /// [v] -> raises a user error carrying v (a NULL re-raises the last caught error).
    Throw,
    /// [] -> [exception object]   (yields HostRequest::CreateException) for `CATCH TO oErr`.
    CatchObject,
    /// End of a FINALLY block: re-raises the error that was propagating when the block was entered.
    EndFinally,
    /// `ON ERROR command`: installs (`Some(const)`) or clears (`None`) the error handler text.
    OnError(Option<u32>),

    // ---- indexes
    /// [] -> []   Reads the compound index beside the table a USE has just opened, when the
    /// table's header says one sits there.
    OpenIndex,
    /// [tag] -> []   `SET ORDER TO`: a tag name, a tag number, or Omitted for record order.
    /// `descending` overrides the tag's own direction when the command said which way to go.
    SetOrder {
        descending: Option<bool>,
    },
    /// [key] -> []   `SEEK`: to the first record the controlling order holds under that key,
    /// or to end of file. FOUND() answers with whether it was there.
    Seek,
    /// [path?, name, key, for, flags] -> []   `INDEX ON`: starts gathering the keys of a new
    /// index. The flags are 1 UNIQUE, 2 CANDIDATE, 4 DESCENDING, 8 a single-entry index of its
    /// own rather than a tag, 16 COMPACT, 32 ADDITIVE; the path is there when 8 is set.
    IndexBegin,
    /// [key] -> []   One key of the tag being built, for the record the pointer is on.
    IndexKey,
    /// [] -> []   The tag is complete: it is sorted, becomes the controlling order, and the
    /// index is written back beside the table.
    IndexEnd,
    /// [name] -> []   `DELETE TAG`: that tag, or every tag, and the index is written back.
    DeleteTag {
        all: bool,
    },
    /// [] -> []   `REINDEX`: writes the index back beside the table.
    Reindex,
    /// [path...] -> []   `USE ... INDEX x, y`: single-entry indexes opened beside the table,
    /// the first of them the controlling order. What `SET INDEX TO` does goes through
    /// `set_cmd` instead, so that the setting and the command stay one thing.
    OpenIdx {
        count: u8,
    },
    /// [path..., target] -> []   `COPY INDEXES`: a tag per single-entry index, in the
    /// structural compound index or in the file the target names. `all` copies every index
    /// open beside the table and leaves the list empty.
    CopyIndexes {
        count: u8,
        all: bool,
    },
    /// [tag, of, target] -> []   `COPY TAG`: that tag of the compound index, or of the file
    /// `of` names, written out as a single-entry index of its own.
    CopyTag,
    // ---- tables, databases and transactions
    /// [record?] -> []   `UNLOCK`: one record, or everything the work area has locked.
    Unlock {
        record: bool,
        area: Option<u32>,
        all: bool,
    },
    /// [path, names...] -> []   `ALTER TABLE`: the table is read, given its new columns and
    /// written back. The names the program worked out sit above the path, in the order the
    /// changes were written.
    AlterTable(Vec<AlterStep>),
    /// [] -> []   A transaction: 0 begins one, 1 writes what it held, 2 throws it away.
    Transaction(u8),
    /// [name?, target?] -> []   A command that works on a database container. `kind` is
    /// 0 create, 1 open, 2 close, 3 set, 4 delete, 5 validate, 6 add a table, 7 remove one,
    /// 8 free one, 9 rename one, 10 create a view, 11 drop one, 12 list what is in it.
    DbCommand {
        kind: u8,
        /// A name was pushed for it.
        named: bool,
        /// A second name was pushed, for a rename.
        target: bool,
        /// The SELECT a view stands for, as a constant.
        sql: Option<u32>,
        /// The clauses the command was written with, as `crate::ast::db_flags` bits: what the
        /// database event that goes with the command is handed.
        flags: u16,
    },
    // ---- listing, reports and browsing
    /// [] -> []   `LIST` and `DISPLAY` of something the runtime holds: 0 the structure of the
    /// table, 1 the variables, 2 the settings and work areas, 3 the files, 4 the database's
    /// tables, 5 its views, 6 the declared library functions, 7 the loaded programs, 8 the
    /// objects, 9 the connections.
    ShowInfo {
        kind: u8,
        /// A `LIKE` skeleton is on the stack, narrowing what is listed to the names it matches.
        skeleton: bool,
    },
    /// [] -> []   A record listing is starting, so the row of field names is due. It is written
    /// above the first record there is to write and not before, because a `LIST` that matches
    /// nothing writes nothing at all. `SET HEADINGS OFF` means none is due.
    ListBegin,
    /// [] -> []   One record, written out as `LIST` writes it. `fields` is a constant naming the
    /// fields the command asked for, a comma apart, or empty for every field of the table;
    /// `numbers` is false when the command said `OFF`, which drops the record-number column.
    ListRecord {
        fields: u32,
        numbers: bool,
    },
    /// [path, file?] -> []   `REPORT FORM`: reads the report file and starts it, writing the
    /// bands that print once at the top. `flags` is the statement's own.
    ReportBegin {
        flags: u8,
        label: bool,
        to_file: bool,
    },
    /// [] -> []   One record of it: the detail band, worked out against the record the pointer
    /// is on.
    ReportRow,
    /// [] -> []   The bands that print once at the end, and the report goes where it was told.
    ReportEnd,
    /// [title?] -> []   `BROWSE`: the records of the work area go to the host, which shows
    /// them in a window of their own. The constant names the columns, a comma apart.
    Browse {
        fields: Option<u32>,
        cond: Option<u32>,
        flags: u8,
        titled: bool,
    },
    // ---- events, memos and saved variables
    /// [command?] -> []   The `ON` family: what it hangs off, and the command it hangs there.
    /// A command of nothing takes the one that was there away.
    OnEvent {
        what: u32,
        given: bool,
    },
    /// [field?, path?] -> []   `APPEND MEMO`, `COPY MEMO`, `MODIFY MEMO`, `CLOSE MEMO` and
    /// `APPEND GENERAL`. The constant names the fields, a comma apart; `named` says the one
    /// field is on the stack instead, under the file, because the program worked its name out.
    Memo {
        what: u8,
        fields: u32,
        pathed: bool,
        named: bool,
        flags: u8,
    },
    /// [target, skeleton?] -> []   `SAVE TO` and `RESTORE FROM`.
    Variables {
        save: bool,
        memo: bool,
        /// A skeleton went on the stack after the target.
        skeleton: bool,
        except: bool,
        additive: bool,
    },
    // ---- input, windows and menus
    /// [row?, col?, drag rows and columns ...] -> []   `MOUSE`: the pointer moves and presses.
    Mouse {
        clicks: u8,
        /// An AT position went on the stack first.
        at: bool,
        /// How many DRAG TO positions followed it.
        drags: u8,
        /// A WINDOW name went on last.
        window: bool,
        /// The words that came after, as one constant.
        style: u32,
    },
    /// [prompt?] -> []   `INPUT`, `ACCEPT` or `GETEXPR`: ask, and put the answer where the
    /// statement said. `kind` is 0 text, 1 worked out as an expression, 2 an expression kept
    /// as text.
    Ask {
        kind: u8,
        prompted: bool,
        /// The variable the answer goes into.
        target: u32,
    },
    /// [command] -> []   `RUN`: a command line handed to the operating system.
    Run {
        nowait: bool,
    },
    /// [cond?, v1 .. vn] -> []   `ASSERT` and `DEBUGOUT`.
    Diagnostic {
        checked: bool,
        argc: u8,
    },
    /// [] -> []   `FLUSH` and `DOEVENTS`: let the host catch up.
    Yield {
        events: bool,
    },
    /// [] -> []   `BLANK`: the fields of the record in hand go back to empty. The constant
    /// names the fields, a comma apart, or is empty for all of them.
    Blank(u32),
    /// [keys] -> []   `KEYBOARD`: keys into the buffer as if they had been typed.
    Keyboard {
        plain: bool,
        clear: bool,
    },
    /// [] -> []   `PUSH KEY` / `POP KEY`.
    KeyStack {
        push: bool,
        clear: bool,
    },
    /// [] -> []   `EJECT`: the page ends here.
    Eject,
    /// [name?, r1?, c1?, r2?, c2?, title?] -> []   A window command. `kind` says which one,
    /// and `text` the words that say how the window looks.
    WindowCommand {
        kind: u8,
        /// One bit each from the least significant: the name, the two corners and the title.
        given: u8,
        text: Option<u32>,
        flags: u8,
    },
    /// [row?, col?, value?, r2?, c2?, picture?, function?, amount?] -> []   One thing an `@`
    /// line draws. `given` is one bit each, in that order.
    AtCommand {
        kind: u8,
        given: u16,
        /// The words that say how it is drawn, what a GET is called, and its two conditions.
        style: Option<u32>,
        name: Option<u32>,
        valid: Option<u32>,
        when: Option<u32>,
    },
    /// [name?, of?, number?, prompt?, key?, message?] -> []   A menu command. `kind` says
    /// which one, and `text` the command a choice runs or the condition it is skipped for.
    MenuCommand {
        kind: u8,
        /// Which of the operands were pushed, one bit each from the least significant: the name,
        /// the OF, the number, the prompt, the key and the message.
        given: u8,
        text: Option<u32>,
        flags: u8,
    },
    // ---- copying, appending and aggregates
    /// [] -> []   `PACK`: the table is read, the records that are left are written back from
    /// the top, and the header is told how many there are.
    Pack,
    /// [path, name1 .. nameN] -> []   `APPEND FROM`: the file to read and which columns to
    /// take from it. `cond` is the FOR condition as written, or 0 when there is none.
    AppendFrom {
        except: bool,
        count: u16,
        cond: Option<u32>,
        /// 0 a table, 1 fixed-width text, 2 comma-separated, 3 delimited.
        text: u8,
    },
    /// [path, name1 .. nameN] -> []   `COPY TO`, `SORT TO` and `TOTAL ON`: the table to make
    /// and which of the columns go in it. `kind` is 0 records, 1 structure, 2 sorted, 3 totals.
    CopyBegin {
        kind: u8,
        except: bool,
        count: u16,
        /// Which sort keys run backwards, one bit per key from the least significant.
        descending: u32,
        /// 0 a table, 1 fixed-width text, 2 comma-separated, 3 delimited by what follows.
        text: u8,
    },
    /// [key1 .. keyN] -> []   One record for the copy: its keys, and the record itself.
    CopyRow(u16),
    /// [] -> []   The copy is complete: the file is written.
    CopyEnd,
    /// [] -> []   `COUNT`/`SUM`/`AVERAGE`/`CALCULATE`: starts one accumulator per column,
    /// each named by what it works out - 0 CNT, 1 SUM, 2 AVG, 3 MIN, 4 MAX, 5 STD, 6 VAR, 7 NPV.
    AggBegin(Vec<u8>),
    /// [value] -> []   One record's worth for the column at that index. NPV takes the rate and
    /// the flow, so it pops two.
    AggStep(u16),
    /// [] -> [array]   The columns worked out, in the order they were asked for.
    AggEnd,
    /// [name1 .. nameN] -> [row?]   `SCATTER`: the record's fields, as an array or an object
    /// on the stack, or as a variable each. The field names named by FIELDS are on the stack,
    /// or none of them when the command named none.
    Scatter {
        /// 0 an array, 1 a variable each, 2 an object.
        to: u8,
        /// The names are the fields to leave out rather than the ones to take.
        except: bool,
        count: u16,
        /// BLANK: the shape of the record rather than what is in it.
        blank: bool,
    },
    /// [row?, name1 .. nameN] -> []   `GATHER`: the other way, into the record in hand.
    Gather {
        to: u8,
        except: bool,
        count: u16,
    },
    /// [] -> []   `SET FILTER TO`: the condition at that constant, empty for none.
    SetFilter(u32),
    /// [expr1, alias1, ...] -> []   `SET RELATION TO ... INTO ...`: `count` pairs of expression
    /// text and work-area alias. `off` takes one relation away instead of setting them.
    SetRelation {
        count: u16,
        additive: bool,
        off: bool,
    },
    // ---- the debugger
    /// [] -> []   `SUSPEND` and `RESUME`: hand the program over to the debugger, or take it back.
    Debug(DebugVerb),

    Nop,
}

/// What a debugger command asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugVerb {
    /// `SUSPEND`: the program stops where it stands and the developer gets the environment.
    Suspend,
    /// `RESUME`: the program that is stopped carries on from the line after the one it stopped at.
    Resume,
}

/// Encodes a module as `MAGIC` + version + postcard body.
pub fn encode(m: &Module) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend(postcard::to_allocvec(m).expect("module serializes"));
    out
}

pub fn decode(bytes: &[u8]) -> Result<Module, String> {
    if bytes.len() < 6 || &bytes[..4] != MAGIC {
        return Err("Not a FoxVM module".into());
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != FORMAT_VERSION {
        return Err(format!(
            "Module was built with bytecode version {version}; this runtime expects {FORMAT_VERSION}. Rebuild the project."
        ));
    }
    postcard::from_bytes(&bytes[6..]).map_err(|e| format!("Corrupt FoxVM module: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let m = Module {
            name: "main".into(),
            kind: ModuleKind::Program,
            consts: vec![Constant::num(1.5), Constant::Str("hi".into()), Constant::Date(None)],
            names: vec!["X".into()],
            members: vec!["Caption".into()],
            funcs: vec![FuncProto {
                name: "MAIN".into(),
                display_name: "main".into(),
                nparams: 0,
                locals: vec![],
                def_line: 1,
                captures: vec![Capture { from: 1, to: 2 }],
                code: vec![
                    Instr::MakeLambda(0),
                    Instr::IndexOrCallValue(1),
                    Instr::Stmt(1),
                    Instr::Const(0),
                    Instr::StoreName(0),
                    Instr::Stmt(2),
                    Instr::LoadThisForm,
                    Instr::GetMember(0),
                    Instr::Print { newline: true, argc: 1 },
                    Instr::True,
                    Instr::Return,
                ],
            }],
            methods: vec![],
            classes: vec![ClassProto {
                name: "form1".into(),
                parent: "form".into(),
                properties: vec![
                    ("Caption".into(), Constant::Str("Form1".into())),
                    ("Visible".into(), Constant::Bool(true)),
                ],
                members: vec![MemberProto {
                    name: "image1".into(),
                    class: "image".into(),
                    noinit: true,
                    properties: vec![("Left".into(), Constant::num(100.0)), ("Picture".into(), Constant::Null)],
                }],
                methods: vec![("Init".into(), 0)],
            }],
            queries: vec![crate::query::QueryPlan {
                columns: vec![crate::query::PlanColumn::Star(None)],
                distinct: true,
                group_keys: 0,
                order_by: vec![
                    crate::query::OrderTerm { descending: false, key: crate::query::OrderKey::Pushed },
                    crate::query::OrderTerm { descending: true, key: crate::query::OrderKey::Result(1) },
                ],
                has_top: false,
                top_percent: false,
                having: vec![crate::query::HavingRef::Key(0)],
                hidden: 1,
                into: crate::query::PlanInto::Cursor("Q".into()),
                into_named: false,
                temporaries: vec!["__SUB1".into()],
            }],
            cursors: vec![("TEMP".into(), vec![ColumnDef { field: crate::dbf::DbfField::new("CODE", 'C', 10, 0), nullable: None }])],
            dlls: vec![DllProto {
                function: "GetTickCount".into(),
                called: "GETTICKCOUNT".into(),
                returns: "INTEGER".into(),
                params: vec![("STRING".into(), true)],
            }],
        };
        let bytes = encode(&m);
        assert_eq!(decode(&bytes).unwrap(), m);
        assert!(decode(b"nope").is_err());
        let mut bad = bytes.clone();
        bad[4] = 99;
        assert!(decode(&bad).unwrap_err().contains("version"));
    }
}
