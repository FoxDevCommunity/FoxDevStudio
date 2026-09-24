//! Abstract syntax tree produced by the parser and consumed by the compiler.
//!
//! Identifiers keep their original spelling for messages plus an upper-cased form for
//! case-insensitive matching. Every node carries a `Span` in character offsets; statements
//! also carry the 1-based physical line they start on, which becomes the runtime line number.

use crate::diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// As written in the source.
    pub text: String,
    /// Upper-cased for case-insensitive comparison.
    pub upper: String,
    pub span: Span,
}

impl Name {
    pub fn new(text: impl Into<String>, span: Span) -> Self {
        let text = text.into();
        let upper = text.to_ascii_uppercase();
        Name { text, upper, span }
    }
    pub fn is(&self, upper: &str) -> bool {
        self.upper == upper
    }
}

/// A whole program (.prg) or a single method body.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Top-level code (everything before the first PROCEDURE/FUNCTION/DEFINE CLASS).
    pub body: Block,
    /// Procedures and functions declared in the file, in source order.
    pub procs: Vec<ProcDecl>,
    /// `DEFINE CLASS ... ENDDEFINE` declarations, in source order. A class declaration is not a
    /// statement: it may only appear at the top level of a program.
    pub classes: Vec<ClassDecl>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcKind {
    Procedure,
    Function,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcDecl {
    pub kind: ProcKind,
    pub name: Name,
    /// Parameters given inline: `PROCEDURE foo(a, b)`. A leading LPARAMETERS/PARAMETERS
    /// statement in the body is kept as a statement and merged by the compiler.
    pub params: Vec<Param>,
    pub body: Block,
    pub line: u32,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Name,
}

/// `DEFINE CLASS <name> AS <parent> [OF <lib>] [OLEPUBLIC] ... ENDDEFINE`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: Name,
    /// What the class inherits from: a VFP base class or another class in the same program.
    pub parent: Name,
    /// `OF <classlib>` as written; parsed and ignored (the class must live in this program).
    pub class_lib: Option<String>,
    /// `<propname> = <expr>` lines, in source order.
    pub properties: Vec<ClassProperty>,
    /// `ADD OBJECT` members, in source order.
    pub members: Vec<ClassMember>,
    /// Method bodies; a name may be dotted (`image1.Click`).
    pub procs: Vec<ProcDecl>,
    pub line: u32,
    pub span: Span,
}

/// One `name = value` line of a class body, or one entry of an `ADD OBJECT ... WITH` list.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassProperty {
    pub name: Name,
    pub value: Expr,
    /// `DIMENSION aRGB[3]` in a class body: the property is an array of this many elements.
    pub dim: Option<Expr>,
    /// `aRGB[2] = 255` in a class body: one element of an array the body dimensioned.
    pub index: Option<Vec<Expr>>,
}

/// `ADD OBJECT [PROTECTED] <name> AS <class> [NOINIT] [WITH <prop> = <expr>, ...]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassMember {
    pub name: Name,
    pub class: Name,
    /// `NOINIT`: the member's Init does not run when the container is created.
    pub noinit: bool,
    pub properties: Vec<ClassProperty>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    /// 1-based physical line the statement starts on.
    pub line: u32,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    /// Set when the declaration sizes an array that belongs to an object rather than declaring a
    /// variable: `DIMENSION THISFORM.aRows[1, 2]`.
    pub member: Option<Expr>,
    /// `PRIVATE (cName)` names the variable by an expression, so a program can declare one
    /// whose name it has just worked out.
    pub name: NameRef,
    /// Array dimensions when declared as `a(3)`, `a[2, 2]` or `ARRAY a[3]`.
    pub dims: Option<Vec<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub expr: Expr,
    /// `@var`: pass by reference.
    pub by_ref: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetValue {
    On,
    Off,
    /// `SET COMPATIBLE OFF NOPROMPT`, `SET TALK OFF NOWINDOW`: the switch, and the words after it.
    Switch { on: bool, words: String },
    To(Vec<Expr>),
    /// `SET DATE TO AMERICAN`, `SET CLASSLIB TO x ADDITIVE`: bare words after TO.
    Word(String),
}

/// What `SELECT` names: a work area number, or an alias.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectTarget {
    Number(Expr),
    Alias(Name),
}

/// Where `GO` puts the record pointer.
#[derive(Debug, Clone, PartialEq)]
pub enum GoWhere {
    Top,
    Bottom,
    Record(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub var: Option<Name>,
    pub when: Option<Expr>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `target = value`
    Assign {
        target: Expr,
        value: Expr,
    },
    /// `STORE value TO a, b`
    Store {
        value: Expr,
        targets: Vec<Target>,
    },
    Local(Vec<VarDecl>),
    Private(Vec<VarDecl>),
    Public(Vec<VarDecl>),
    Dimension(Vec<VarDecl>),
    Parameters(Vec<Param>),
    LParameters(Vec<Param>),
    If {
        cond: Expr,
        then: Block,
        else_: Option<Block>,
    },
    DoCase {
        cases: Vec<(Expr, Block)>,
        otherwise: Option<Block>,
    },
    DoWhile {
        cond: Expr,
        body: Block,
    },
    For {
        var: Name,
        from: Expr,
        to: Expr,
        step: Option<Expr>,
        body: Block,
    },
    ForEach {
        var: Name,
        collection: Expr,
        body: Block,
    },
    Exit,
    Loop,
    Return(Option<Expr>),
    /// `DO name [WITH args] [IN prog]`
    Do {
        name: Name,
        args: Vec<Arg>,
        in_prog: Option<Expr>,
    },
    /// `DO (cProgram) [WITH args] [IN prog]`: the program is named by an expression.
    DoExpr {
        name: Expr,
        args: Vec<Arg>,
        in_prog: Option<Expr>,
    },
    /// `DO FORM name [NAME var [LINKED]] [WITH args] [TO var] [NOSHOW]`
    DoForm {
        name: Expr,
        name_var: Option<Target>,
        linked: bool,
        args: Vec<Arg>,
        to_var: Option<Name>,
        noshow: bool,
    },
    /// `=expr`, `obj.Method()`, `func()` used as a statement.
    ExprStmt(Expr),
    /// `?` / `??`
    Print {
        items: Vec<Expr>,
        newline: bool,
    },
    WaitWindow {
        text: Option<Expr>,
        nowait: bool,
        timeout: Option<Expr>,
        clear: bool,
        to_var: Option<Name>,
    },
    ReadEvents,
    ClearEvents,
    /// `RELEASE a, b` / `RELEASE THISFORM`
    Release(Vec<Expr>),
    /// `RELEASE ALL`
    ReleaseAll,
    Quit,
    Cancel,
    With {
        obj: Expr,
        body: Block,
    },
    Set {
        setting: Name,
        value: SetValue,
    },
    Try {
        body: Block,
        /// One per CATCH clause, in source order; each is tried in turn.
        catches: Vec<CatchClause>,
        finally: Option<Block>,
    },
    /// A command this runtime cannot honour, kept so the rest of the routine still compiles.
    ///
    /// Visual FoxPro compiles `BUILD DLL`, `APPEND GENERAL` and the rest of them without a
    /// murmur - measured, by compiling each and finding no `.err` file beside the `.fxp` - and
    /// only fails if the line is reached. Refusing at compile time takes the whole method with
    /// it, so every other line in it stops working too.
    Unsupported(String),
    /// `ERASE file` / `ERASE (cFile)`: deletes a file.
    Erase(Expr),
    /// `USE [table] [ALIAS name] [EXCLUSIVE|SHARED] [ORDER tag]`. No table closes the area.
    Use {
        table: Option<Expr>,
        /// `ALIAS name`, `ALIAS (cName)` or `ALIAS "name"`: what the open table is called.
        alias: Option<NameRef>,
        exclusive: bool,
        /// `ONLINE` or `ADMIN`: both ask for an offline view rather than a table.
        online: bool,
        /// `IN 0` or `IN alias`: which work area to open it in, leaving the selection alone.
        in_area: Option<Expr>,
        /// `ORDER [TAG] x`: the tag the table opens in the order of.
        order: Option<Expr>,
        /// ASCENDING or DESCENDING said which way that order runs.
        order_desc: Option<bool>,
        /// `INDEX x, y`: the single-entry indexes opened with the table, the first controlling.
        indexes: Vec<Expr>,
    },
    /// `SET ORDER TO [TAG] x [ASCENDING|DESCENDING]`, and `SET ORDER TO` on its own, which
    /// puts the table back in record order.
    SetOrder {
        tag: Option<Expr>,
        descending: Option<bool>,
        /// `IN alias`: the table whose order changes, when it is not the selected one.
        area: Option<NameRef>,
    },
    /// `SEEK eExpr [ORDER tag] [ASCENDING|DESCENDING]`: down the controlling order to a key.
    Seek {
        key: Expr,
        order: Option<Expr>,
        descending: Option<bool>,
    },
    /// `INDEX ON eExpr TAG name | TO file [FOR x] [UNIQUE|CANDIDATE] [ASCENDING|DESCENDING]
    /// [COMPACT] [ADDITIVE]`.
    IndexOn {
        key: Expr,
        /// The key expression as it was written: what the index stores, and what a record
        /// changed later is put back in its place by.
        key_text: String,
        tag: Name,
        /// `TO file`: a single-entry index of its own rather than a tag of the compound one.
        to_file: Option<Expr>,
        /// Which of the two `.idx` layouts `TO` writes.
        compact: bool,
        cond: Option<Expr>,
        cond_text: String,
        unique: bool,
        candidate: bool,
        descending: bool,
        /// ADDITIVE keeps the single-entry indexes already open; without it they are closed.
        additive: bool,
    },
    /// `COPY INDEXES IndexFileList | ALL [TO CDXFileName]`: a tag per single-entry index, in
    /// the structural compound index or in the one named.
    CopyIndexes {
        files: Vec<Expr>,
        /// Every single-entry index open beside the table, which is what ALL means.
        all: bool,
        target: Option<Expr>,
    },
    /// `COPY TAG TagName [OF CDXFileName] TO IndexFileName`: a tag written out on its own.
    CopyTag {
        tag: Expr,
        of: Option<Expr>,
        target: Expr,
    },
    /// `COPY TO`, `COPY STRUCTURE TO`, `SORT TO` and `TOTAL ON ... TO`: a new table made
    /// from the one in the work area.
    CopyTo {
        path: Expr,
        kind: CopyKind,
        fields: Vec<Name>,
        except: Vec<Name>,
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
        /// `SORT ON name /D`, and the key of a `TOTAL ON`.
        keys: Vec<SortKey>,
        /// `TYPE SDF | CSV | DELIMITED [WITH x]`: a text file rather than a table.
        text: Option<TextFormat>,
    },
    /// `APPEND FROM file [FIELDS list] [FOR x] [TYPE ...]`: the records of another table, or
    /// of a text file, added to the end of this one.
    AppendFrom {
        path: Expr,
        fields: Vec<Name>,
        except: Vec<Name>,
        /// The condition as it was written: it is tested against each record as it arrives.
        cond: String,
        text: Option<TextFormat>,
    },
    /// `UNLOCK [RECORD n] [IN area] [ALL]`: the locks this program holds are let go of.
    Unlock {
        record: Option<Expr>,
        area: Option<NameRef>,
        all: bool,
    },
    /// `ALTER TABLE name ADD|ALTER COLUMN f T(n) | DROP COLUMN f | RENAME COLUMN a TO b`
    AlterTable {
        path: Expr,
        ops: Vec<AlterOp>,
    },
    /// `BEGIN TRANSACTION`, `END TRANSACTION` and `ROLLBACK`.
    Transaction(TransactionStep),
    /// The commands that work on a database container: making one, opening it, saying which
    /// is current, and what it holds.
    Database {
        what: DbCommand,
        /// The database, table or view the command names.
        name: Option<Expr>,
        /// The second name: what RENAME TABLE renames to.
        target: Option<Expr>,
        /// A view's SELECT, as it was written.
        sql: String,
        /// The clauses the program wrote, as `db_flags` bits. A database event is told which of
        /// them were there - dbc_OpenData is handed EXCLUSIVE, NOUPDATE and VALIDATE one flag
        /// each - so they are carried rather than passed over.
        flags: u16,
    },
    /// `LIST` and `DISPLAY`: what the runtime holds, written out. Without a word after it,
    /// the records of the table in the selected work area.
    ListInfo {
        /// STRUCTURE, MEMORY, STATUS, FILES, TABLES, VIEWS, DLLS, PROCEDURES, OBJECTS, or empty
        /// for the records themselves.
        what: String,
        /// The fields a record listing names, or none for all of them.
        fields: Vec<Name>,
        /// `LIKE zz*`: which names a listing of what the program holds keeps.
        skeleton: Option<Expr>,
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
        /// False when the command said `OFF`, which leaves the record-number column out.
        numbers: bool,
    },
    /// `REPORT FORM` and `LABEL FORM`: a report file run over the records of a table.
    ReportForm {
        /// The `.frx` or `.lbx` to run.
        path: Expr,
        /// True for `LABEL FORM`, which reads a label file rather than a report file.
        label: bool,
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
        /// `TO FILE <name>`, where it was given one.
        to_file: Option<Expr>,
        /// 1 PREVIEW, 2 TO PRINTER, 4 NOCONSOLE, 8 SUMMARY, 16 PLAIN.
        flags: u8,
    },
    /// `EJECT` and `EJECT PAGE`: the page ends here.
    Eject,
    /// The `ON` family: a command hung off something that may happen. `what` names it -
    /// ESCAPE, SHUTDOWN, READERROR, PAGE, or KEY with the label it is for.
    OnEvent {
        what: String,
        command: Option<String>,
    },
    /// `APPEND MEMO`, `COPY MEMO`, `MODIFY MEMO` and `CLOSE MEMO`: what a memo field holds,
    /// between a file and a window on it.
    Memo {
        /// 0 APPEND MEMO, 1 COPY MEMO, 2 MODIFY MEMO, 3 CLOSE MEMO, 4 APPEND GENERAL.
        what: u8,
        /// The fields the command names, each of them `field` or `alias.field`.
        fields: Vec<Name>,
        /// `APPEND GENERAL (THIS.cField)`: the one field is named by an expression rather than
        /// written out, so which field it is is only known when the command runs.
        field_expr: Option<Expr>,
        /// The file APPEND MEMO reads, or the one COPY MEMO writes.
        path: Option<Expr>,
        /// 1 OVERWRITE or ADDITIVE, 2 NOEDIT, 4 NOWAIT, 8 ALL.
        flags: u8,
    },
    /// `SAVE TO` and `RESTORE FROM`: the memory variables put away in a file or a memo
    /// field, and taken back out again.
    Variables {
        /// True for SAVE TO, false for RESTORE FROM.
        save: bool,
        /// The file, or the memo field when `memo` is set.
        target: Expr,
        memo: bool,
        /// `ALL LIKE` or `ALL EXCEPT`: which names are saved.
        skeleton: Option<Expr>,
        /// The skeleton says which to leave out rather than which to keep.
        except: bool,
        /// `ADDITIVE`: what is in memory already stays.
        additive: bool,
    },
    /// `MOUSE`: the pointer moved, pressed, or dragged, as if a hand had done it.
    Mouse {
        /// 0 moved only, 1 CLICK, 2 DBLCLICK.
        clicks: u8,
        /// `AT nRow, nColumn`. Without it the pointer presses where it already is.
        at: Option<(Expr, Expr)>,
        /// `DRAG TO`: the positions it is dragged through, in order.
        drag: Vec<(Expr, Expr)>,
        /// `WINDOW cName`: the window the positions are measured in.
        window: Option<Expr>,
        /// PIXELS, LEFT, MIDDLE, RIGHT, SHIFT, CONTROL and ALT, as written.
        style: String,
    },
    /// `INPUT`, `ACCEPT` and `GETEXPR`: ask the user for something and keep the answer.
    Ask {
        /// What to say while asking.
        prompt: Option<Expr>,
        /// Where the answer goes.
        target: Expr,
        /// 0 ACCEPT keeps the text, 1 INPUT works it out as an expression, 2 GETEXPR asks
        /// for an expression and keeps it as text.
        kind: u8,
    },
    /// `RUN` / `!`: hand a command line to the operating system.
    Run {
        command: Expr,
        /// The program carries on without waiting for it to finish.
        nowait: bool,
    },
    /// `SUSPEND` and `RESUME`: hand the running program to the debugger, or take it back.
    Debug(crate::bytecode::DebugVerb),
    /// `ASSERT` and `DEBUGOUT`: say something while the program is being worked on.
    Diagnostic {
        /// The condition an ASSERT checks, or nothing for DEBUGOUT.
        cond: Option<Expr>,
        message: Vec<Expr>,
    },
    /// `FLUSH` and `DOEVENTS`: let the host catch up with what the program has done.
    Yield {
        /// True for DOEVENTS, false for FLUSH.
        events: bool,
    },
    /// `BLANK`: the record's fields go back to empty.
    Blank {
        fields: Vec<Name>,
        scope: Scope,
        cond: Option<Expr>,
    },
    /// `KEYBOARD`: keys put into the buffer as if they had been typed.
    Keyboard {
        keys: Expr,
        /// PLAIN leaves the braces alone rather than reading {ENTER} as a key.
        plain: bool,
        clear: bool,
    },
    /// `PUSH KEY` and `POP KEY`: what the `ON KEY` settings are, kept and put back.
    KeyStack {
        push: bool,
        clear: bool,
    },
    /// The window commands, and the two that work on the screen behind them.
    WindowCommand {
        what: WindowVerb,
        /// The window it names, where it names one.
        name: Option<Expr>,
        /// `FROM r1, c1 TO r2, c2`, or the place a MOVE or a SIZE gives.
        corners: Vec<Expr>,
        title: Option<Expr>,
        /// The words that say how it looks or what it may do, kept as they were written.
        text: String,
        /// 1 ALL, 2 NOSHOW, 4 the screen rather than a window.
        flags: u8,
        /// What `READ` runs around itself, when it named any.
        read: Option<Box<ReadClauses>>,
    },
    /// An `@` line: one part for each of the things it draws, because `@ 2,3 SAY x GET y`
    /// says and gets on the same line.
    AtCommand(Vec<AtPart>),
    /// The menu commands: what is defined, what it does, and what is put up.
    MenuCommand {
        /// Which command it is; the compiler turns it into the instruction's number.
        what: MenuVerb,
        /// The menu, pad or popup it names.
        name: Option<Expr>,
        /// `OF <menu>` or `OF <popup>`.
        of: Option<Expr>,
        /// The bar number, where the command takes one.
        number: Option<Expr>,
        /// `PROMPT <expr>`, and the key and message that go with it.
        prompt: Option<Expr>,
        key: Option<Expr>,
        message: Option<Expr>,
        /// The command a choice runs, or the condition SKIP FOR and MARK were given, as written.
        text: String,
        /// 1 ALL, 2 NOWAIT, 4 the name is a popup rather than a menu.
        flags: u8,
    },
    /// `PACK`: the records marked deleted go, and the ones after them move up.
    Pack,
    /// `DROP TABLE name`: the file goes.
    DropTable {
        path: Expr,
        /// `RECYCLE`, which the container's dbc_BeforeDropTable is handed.
        flags: u16,
    },
    /// `COUNT`, `SUM`, `AVERAGE` and `CALCULATE`: one pass over the records working out
    /// one number per column asked for, and putting each in a variable.
    Aggregate {
        calls: Vec<AggCall>,
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
        /// `TO x, y`: where each result goes.
        targets: Vec<Expr>,
        /// `TO ARRAY a`: all of them, in one array.
        array: Option<Expr>,
    },
    /// `SCATTER [FIELDS ...] [MEMO] [BLANK] TO array | MEMVAR | NAME oVar`: the record's
    /// fields as somewhere to work on them away from the table.
    Scatter {
        fields: Vec<Name>,
        except: Vec<Name>,
        blank: bool,
        to: ScatterWhere,
    },
    /// `GATHER FROM array | MEMVAR | NAME oObj [FIELDS ...]`: the way back.
    Gather {
        from: ScatterWhere,
        fields: Vec<Name>,
        except: Vec<Name>,
    },
    /// `RETURN TO routine` and `RETURN TO MASTER` (`None`): every routine between here and the
    /// one named is left, rather than only the one this RETURN is in.
    ReturnTo(Option<Name>),
    /// `APPEND FROM ARRAY a`: a record added per row of the array, which is what
    /// `INSERT INTO ... FROM ARRAY` does to a table it names rather than to the one in hand.
    AppendFromArray(Expr),
    /// `REPLACE FROM ARRAY a [scope] [FOR x] [WHILE y] [IN area]`: a GATHER over a scope, where
    /// each record in turn takes the next row of the array.
    ReplaceFromArray {
        source: Expr,
        area: Option<NameRef>,
        fields: Vec<Name>,
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
    },
    /// `SET FILTER TO [lExpr]`: the condition a record has to pass to be seen. The text is
    /// kept as written, because it is evaluated again at every record the pointer crosses.
    SetFilter(String),
    /// `SET RELATION TO eExpr INTO cAlias [, ...] [ADDITIVE]`, and `SET RELATION OFF INTO x`.
    SetRelation {
        /// The expression text and the work area it is looked up in, one pair per relation.
        pairs: Vec<(String, Name)>,
        additive: bool,
        /// `OFF INTO alias`: that one relation goes and the others stay.
        off: Option<Name>,
    },
    /// `REINDEX`: the index is written back out.
    Reindex,
    /// `DELETE TAG name`, or `DELETE TAG ALL`.
    DeleteTag {
        tag: Option<Name>,
    },
    /// `SELECT 0` / `SELECT 2` / `SELECT customer`.
    SelectArea(SelectTarget),
    /// `GO TOP` / `GO BOTTOM` / `GO 5` / `GOTO RECORD 5`.
    Go {
        where_: GoWhere,
        /// `IN <alias>`: the work area it moves in, which stays unselected afterwards.
        area: Option<Expr>,
    },
    /// `SKIP` / `SKIP 5` / `SKIP -1`.
    Skip {
        count: Option<Expr>,
        /// `IN <alias>`, as GO takes it.
        area: Option<Expr>,
    },
    /// `CLOSE TABLES` / `CLOSE ALL` / `CLOSE DATABASES`.
    CloseTables {
        all: bool,
    },
    /// `SCAN [scope] [FOR x] [WHILE x] ... ENDSCAN`.
    Scan {
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
        body: Block,
    },
    /// `LOCATE [scope] [FOR x] [WHILE x]`.
    Locate {
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
    },
    /// `CONTINUE`: the last LOCATE of this procedure, carried on from the next record.
    Continue,
    /// `UPDATE table SET field = x [, ...] [WHERE condition]`: the SQL spelling of REPLACE,
    /// which opens the table when it is not open already.
    UpdateSql {
        table: Name,
        path: Expr,
        assignments: Vec<(Name, Expr, bool)>,
        cond: Option<Expr>,
    },
    /// `DELETE FROM table [WHERE condition]`: the SQL spelling of DELETE, which marks every
    /// record the condition holds for and opens the table when it is not open already.
    DeleteSql {
        table: Name,
        path: Expr,
        cond: Option<Expr>,
    },
    /// `REPLACE field WITH x [, field2 WITH x2] [scope] [FOR y] [WHILE z]`.
    Replace {
        /// The work area, from `IN alias`; the selected one when absent.
        area: Option<NameRef>,
        assignments: Vec<(NameRef, Expr, bool)>,
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
    },
    /// `DELETE` and `RECALL`: the flag at the front of each record in scope.
    MarkDeleted {
        deleted: bool,
        area: Option<NameRef>,
        scope: Scope,
        cond: Option<Expr>,
        while_: Option<Expr>,
    },
    /// `BUILD APP | EXE | DLL | MTDLL | PROJECT`: the file a project is turned into, or the
    /// project itself, built out of the files it is made of.
    Build {
        /// APP, EXE, DLL, MTDLL or PROJECT.
        what: Name,
        /// The file being built.
        target: Expr,
        /// The project an application is built from; for BUILD PROJECT, the files that go in it.
        from: Vec<Expr>,
        recompile: bool,
    },
    /// `COMPILE`: source turned into object code, with its syntax checked on the way.
    Compile {
        /// DATABASE, FORM, CLASSLIB, LABEL or REPORT; empty when it is a program.
        what: Name,
        /// The file to compile, or a skeleton like `*.prg` standing for several.
        files: Expr,
        /// ALL, ENCRYPT and NODEBUG, as `crate::bytecode::compile_flags`.
        flags: u8,
    },
    /// `IMPORT FROM file`: a table made from a spreadsheet.
    Import {
        path: Expr,
        /// `SHEET cSheetName`: which sheet of the workbook, or the first one.
        sheet: Option<Expr>,
    },
    /// `CREATE FileName1 FROM FileName2`: a table with the structure that the table
    /// `COPY STRUCTURE EXTENDED` wrote describes.
    CreateFrom {
        target: Expr,
        source: Expr,
    },
    /// `CREATE FORM`, `CREATE MENU`, `CREATE PROJECT` and the rest: something new of that
    /// kind, opened in the designer for it.
    NewDocument {
        what: Name,
        path: Expr,
    },
    /// `MODIFY COMMAND/FORM/DATABASE/...`: open a file for editing. In an IDE that is a real
    /// thing to do, so it is done rather than refused.
    Modify {
        /// The word after MODIFY, for the message when there is no file to open.
        what: Name,
        path: Expr,
        /// `NOWAIT` and `NOEDIT`, which dbc_ModifyData is handed.
        flags: u16,
    },
    /// `BROWSE`: show the table in the selected work area in a window of its own.
    Browse {
        /// `FIELDS a, b`: the columns to show, or none for every field of the table.
        fields: Vec<Name>,
        /// The `FOR` condition as written, worked out per record the way SET FILTER is.
        cond: String,
        /// `TITLE` for the window, where one was given.
        title: Option<Expr>,
        /// 1 NOWAIT, 2 NOEDIT or NOMODIFY, 4 the window is not closed by the user.
        flags: u8,
    },
    /// `CREATE TABLE name [FREE] (field type(width[, decimals]), ...)`: a new table on disk,
    /// which is then in use in the selected work area, as VFP leaves it.
    CreateTable {
        path: Expr,
        fields: Vec<CursorField>,
        /// `FROM ARRAY a`: the columns are described by an array instead, and `fields` is empty.
        from_array: Option<Expr>,
        /// `CREATE TABLE x FREE`: the new table is not listed in the open database. Without it
        /// a database that is open takes the table in, which is what makes it a database table.
        free: bool,
    },
    /// `CREATE CURSOR name (field type(width[, decimals]), ...)`.
    CreateCursor {
        alias: Name,
        /// `CREATE CURSOR (cName)`: the name, worked out when the statement runs.
        named: Option<Expr>,
        fields: Vec<CursorField>,
        /// `FROM ARRAY a`: the columns are described by an array instead, and `fields` is empty.
        from_array: Option<Expr>,
    },
    /// `INSERT INTO table [(fields)] VALUES (values) | FROM ARRAY a | FROM MEMVAR | FROM NAME o`.
    Insert {
        alias: Name,
        /// `INSERT INTO (cName)`: the table worked out when the statement runs.
        named: Option<Expr>,
        /// The named fields, or empty for the table's own order. Only VALUES names fields.
        fields: Vec<NameRef>,
        source: InsertSource,
    },
    /// `INSERT [BEFORE] [BLANK]`: the Xbase record-insert, which puts an empty record next to
    /// the one the pointer is on rather than at the end of the table.
    InsertBlank {
        /// BEFORE puts it above the current record instead of below it.
        before: bool,
    },
    /// `CLEAR ALL` and `CLEAR MEMORY`: let go of the variables, and for ALL the tables too.
    ClearAll {
        tables: bool,
    },
    /// `APPEND BLANK`.
    AppendBlank {
        area: Option<NameRef>,
    },
    /// `ERROR nNumber [, cMessage]` or `ERROR cMessage`: the program raises an error of its own.
    RaiseError {
        what: Expr,
        message: Option<Expr>,
    },
    /// `DECLARE [type] Function IN library [AS alias] [type [@] name, ...]`: a function of a
    /// Windows library, callable from then on as if the program had written it.
    DeclareDll {
        /// The word before the function name, when there is one: what the library returns.
        returns: Option<Name>,
        function: Name,
        /// The library: a bare name, a path, or an expression in parentheses.
        library: Expr,
        /// The name the program calls it by, when it is not the function's own.
        alias: Option<Name>,
        params: Vec<DllParam>,
    },
    /// COPY FILE, RENAME, MD, RD, DIR and TYPE: a file command with one or two names.
    FileCommand {
        /// 1 COPY FILE, 2 RENAME, 3 MD, 4 RD, 5 DIR, 6 TYPE.
        kind: u8,
        path: Expr,
        target: Option<Expr>,
    },
    /// `ZAP [IN area]`: every record of a table goes.
    Zap {
        area: Option<NameRef>,
    },
    /// `SELECT ... FROM ...`, the query language rather than the work-area command.
    Query(Box<Query>),
    /// `RETRY`: returns to the caller and runs the statement that called again.
    Retry,
    Throw(Option<Expr>),
    NoDefault,
    DoDefault(Vec<Arg>),
    /// A command with a macro in it that this parser could not read as it stands - `&cmd`,
    /// `SCAN &lcScope`, `SET ORDER TO (lcTag) &lcAlias ASCENDING`. Visual FoxPro puts the text
    /// a macro stands for into the line before it reads the line at all, so where the shape of
    /// the command is not known until then, the source is kept exactly as it was written and
    /// substituted, compiled and run at the moment it is reached. A command that opens a block
    /// keeps its whole body, because the line that opens it cannot be run on its own.
    MacroText(String),
    /// `TEXT [TO var [ADDITIVE]] [TEXTMERGE] [NOSHOW] ... ENDTEXT`
    Text {
        target: Option<Expr>,
        additive: bool,
        textmerge: bool,
        noshow: bool,
        raw: String,
    },
    /// `\ text` and `\\ text`: one line of text merge output. `newline` is the single `\`,
    /// which ends the line before it; `\\` carries that line on.
    TextLine {
        newline: bool,
        raw: String,
    },
    /// `ON ERROR [command]`; `None` clears the handler. The command is kept as source text.
    OnError(Option<String>),
    /// `ON KEY LABEL key [command]`
    OnKeyLabel {
        key: String,
        command: Option<String>,
    },
    /// `#DEFINE`, `#INCLUDE` and friends that the parser accepted but has no effect for.
    Directive(String),
}

/// One change `ALTER TABLE` makes to a table's columns.
#[derive(Debug, Clone, PartialEq)]
pub enum AlterOp {
    /// A column at the end.
    Add(CursorField),
    /// A column with a new type or width; what is in it is read as the old type and written as
    /// the new one.
    Alter(CursorField),
    Drop(NameRef),
    Rename(NameRef, NameRef),
}

/// The three things a program does with a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStep {
    Begin,
    End,
    Rollback,
}

/// Which window command a statement is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowVerb {
    Define,
    Activate,
    Deactivate,
    Show,
    Hide,
    Move,
    Size,
    Zoom,
    Modify,
    Release,
    SaveWindows,
    RestoreWindows,
    ActivateScreen,
    SaveScreen,
    RestoreScreen,
    ClearScreen,
    Read,
    ShowGets,
    ClearGets,
    MenuTo,
}

/// One thing an `@` line draws.
#[derive(Debug, Clone, PartialEq)]
pub struct AtPart {
    pub what: AtVerb,
    pub row: Expr,
    pub col: Expr,
    /// The second corner, where the command gave one.
    pub corners: Vec<Expr>,
    /// SAY's expression, BOX's frame characters, FILL's character.
    pub value: Option<Expr>,
    pub picture: Option<Expr>,
    pub function: Option<Expr>,
    /// How far a SCROLL moves.
    pub amount: Option<Expr>,
    /// How far it moves across, for the `SCROLL` command, which moves a region on both axes
    /// at once. `@ ... SCROLL` names one direction in a word and leaves this empty.
    pub amount2: Option<Expr>,
    /// DOUBLE, PANEL, the direction a SCROLL goes, the kind of control a GET asks for.
    pub style: String,
    /// What a GET is called, for VARREAD() and for writing the answer back.
    pub name: String,
    /// VALID and WHEN as written, worked out when the READ runs.
    pub valid: String,
    pub when: String,
}

/// Which `@` command a part is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtVerb {
    Say,
    Get,
    Clear,
    To,
    Box,
    Fill,
    Scroll,
    Menu,
    Prompt,
    Edit,
}

/// Which menu command a statement is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuVerb {
    DefineMenu,
    DefinePad,
    DefinePopup,
    DefineBar,
    OnPad,
    OnBar,
    OnSelectionPad,
    OnSelectionBar,
    OnSelectionPopup,
    OnSelectionMenu,
    OnExit,
    Activate,
    Deactivate,
    Show,
    Hide,
    Release,
    /// `RELEASE BAR n | ALL OF popup`: one bar out of a popup that stays.
    ReleaseBar,
    /// `RELEASE PAD name | ALL OF menu`: one pad out of a menu bar that stays.
    ReleasePad,
    Push,
    Pop,
    SetMark,
    SetSkip,
    SetMessage,
    SetSysMenu,
}

/// The clauses a database command may carry, one bit each.
///
/// Visual FoxPro hands the clauses a command was written with to the database event that goes
/// with it - `OPEN DATABASE sales EXCLUSIVE` reaches dbc_OpenData as .T. for its second
/// argument - so the parser keeps them instead of passing over them.
pub mod db_flags {
    pub const EXCLUSIVE: u16 = 1 << 0;
    pub const NOUPDATE: u16 = 1 << 1;
    pub const VALIDATE: u16 = 1 << 2;
    pub const ALL: u16 = 1 << 3;
    pub const RECYCLE: u16 = 1 << 4;
    pub const DELETE: u16 = 1 << 5;
    /// `ADDITIVE` on COPY PROCEDURES, and `OVERWRITE` on APPEND PROCEDURES: the one flag each
    /// of those two commands has, and the one the event is handed.
    pub const ADDITIVE: u16 = 1 << 6;
    pub const RECOVER: u16 = 1 << 7;
    pub const NOCONSOLE: u16 = 1 << 8;
    pub const PRINTER: u16 = 1 << 9;
    pub const TO_FILE: u16 = 1 << 10;
    pub const NOWAIT: u16 = 1 << 11;
    pub const NOEDIT: u16 = 1 << 12;
    pub const REMOTE: u16 = 1 << 13;
    /// `CREATE CONNECTION ... DATASOURCE`, as against `CONNSTRING`: the two name the same thing
    /// here, but dbc_AfterCreateConnection is handed them in different places.
    pub const DATASOURCE: u16 = 1 << 14;
}

/// What a command does to a database container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbCommand {
    /// `APPEND PROCEDURES FROM file`: the stored procedures of the current database.
    AppendProcedures,
    /// `COPY PROCEDURES TO file`: the same, written back out.
    CopyProcedures,
    /// `PACK DATABASE`: the container is written out without what was crossed off.
    Pack,
    /// `RENAME VIEW a TO b`.
    RenameView,
    Create,
    Open,
    Close,
    Set,
    Delete,
    Validate,
    AddTable,
    RemoveTable,
    FreeTable,
    RenameTable,
    CreateView,
    DropView,
    /// `CREATE CONNECTION`: a named connection string kept in the container.
    CreateConnection,
    /// `DELETE CONNECTION`.
    DeleteConnection,
    /// `RENAME CONNECTION a TO b`.
    RenameConnection,
    /// `MODIFY CONNECTION`: the designer, which is the container's to say happened.
    ModifyConnection,
    List,
}

/// Where an INSERT takes the record it adds from.
#[derive(Debug, Clone, PartialEq)]
pub enum InsertSource {
    /// `VALUES (...)`: one value per field the statement names, or per field of the table.
    Values(Vec<Expr>),
    /// `FROM ARRAY a`, `FROM MEMVAR` and `FROM NAME oRec` read the record from the same three
    /// places GATHER does, which is what they are: an APPEND BLANK and a GATHER.
    From(ScatterWhere),
    /// `INSERT INTO t [(fields)] SELECT ...`: a record per row of the query, the columns going
    /// to the fields named, or to the table's fields in order when none are.
    Query(Box<Query>),
}

/// What a copy makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyKind {
    /// `COPY STRUCTURE EXTENDED TO`: a table describing the fields of this one, one record
    /// per field. `CREATE ... FROM` reads it back.
    StructureExtended,
    /// Not a form of COPY: what `CREATE ... FROM` gathers. The records read are field
    /// descriptions, and what is written is the table they describe.
    FromDescription,
    /// `COPY TO`: the records as they are.
    Records,
    /// `COPY STRUCTURE TO`: the columns and no records.
    Structure,
    /// `SORT TO`: the records in the order of the keys.
    Sorted,
    /// `TOTAL ON key TO`: one record per run of equal keys, its numbers added up.
    Totals,
    /// `COPY TO ARRAY`: the records as a two-dimensional array, one row each and one column
    /// per field. Nothing is written to disk, and no records at all leaves the array alone.
    Array,
}

/// A name a command is given: written out, or worked out when the command runs.
///
/// Almost anywhere Visual FoxPro wants a name - an alias, a table, a cursor, a field, a
/// column, a variable, a form - a program may put an expression in parentheses instead, and
/// the string it comes to is the name. The reference calls that a name expression, and it is
/// how a program that walks the columns of a table writes to each of them in turn
/// (`REPLACE (m.lcField) WITH x`) or reads a table whose folder is only known when it runs.
/// A quoted name is a name too: `USE customer ALIAS "keywords"` opens it under that alias.
#[derive(Debug, Clone, PartialEq)]
pub enum NameRef {
    Named(Name),
    Computed(Expr),
}

impl NameRef {
    /// Where it was written, for a message about it.
    pub fn span(&self) -> Span {
        match self {
            NameRef::Named(n) => n.span,
            NameRef::Computed(e) => e.span,
        }
    }
}

/// Where a command puts a value: a target written out, or one the command works the name of
/// out when it runs.
///
/// The second is the name-expression rule again, on the writing side: `STORE lnValue TO
/// ("THIS.oObject." + lcProperty)` writes to whatever that string names - a variable, or a
/// property some way down an object - exactly as macro substitution would have named it.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Written(Expr),
    ByName(Expr),
}

impl Target {
    pub fn span(&self) -> Span {
        match self {
            Target::Written(e) | Target::ByName(e) => e.span,
        }
    }
}

/// The procedures a `READ` names in its own clauses, which the reference calls its events.
///
/// Each is worked out at the moment it belongs to: WHEN before the read happens at all, SHOW
/// and ACTIVATE as it starts, DEACTIVATE when it ends, and VALID to say whether it may - a
/// VALID that comes out false starts the read again.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReadClauses {
    pub when: Option<Expr>,
    pub show: Option<Expr>,
    pub activate: Option<Expr>,
    pub deactivate: Option<Expr>,
    pub valid: Option<Expr>,
}

impl ReadClauses {
    /// True when the read named any of them at all.
    pub fn any(&self) -> bool {
        self.when.is_some()
            || self.show.is_some()
            || self.activate.is_some()
            || self.deactivate.is_some()
            || self.valid.is_some()
    }
}

/// A key of `SORT ON`: what to sort by and which way.
#[derive(Debug, Clone, PartialEq)]
pub struct SortKey {
    pub expr: Expr,
    pub descending: bool,
}

/// The text formats `COPY TO ... TYPE` writes.
#[derive(Debug, Clone, PartialEq)]
pub enum TextFormat {
    /// `EXPORT ... TYPE DIF`: one vector per field and one tuple per record.
    Dif,
    /// `EXPORT ... TYPE SYLK`: one cell per field and record.
    Sylk,
    /// Fixed-width columns, one record a line.
    Sdf,
    /// Commas between the fields and quotes around the text.
    Csv,
    /// `DELIMITED [WITH x] [WITH CHARACTER y]`.
    Delimited {
        quote: String,
        separator: String,
    },
}

/// One column of a `CALCULATE`, or the whole of a `COUNT`, `SUM` or `AVERAGE`.
#[derive(Debug, Clone, PartialEq)]
pub struct AggCall {
    pub func: AggFunc,
    /// What is added up. COUNT has nothing to add up.
    pub arg: Option<Expr>,
    /// `NPV(rate, flow)` discounts the flow by the rate, so it has a second.
    pub rate: Option<Expr>,
}

/// What an aggregate works out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggFunc {
    Cnt,
    Sum,
    Avg,
    Min,
    Max,
    Std,
    Var,
    Npv,
}

/// Where SCATTER puts a record, and where GATHER takes one from.
#[derive(Debug, Clone, PartialEq)]
pub enum ScatterWhere {
    /// `TO aRow`: an array, one element per field.
    Array(Expr),
    /// `MEMVAR`: a variable per field, named after it.
    Memvar,
    /// `NAME oRow`: an object with a property per field.
    Name(Expr),
}

/// A `SELECT-SQL` query.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    /// `SELECT DISTINCT`: identical result rows are folded into one.
    pub distinct: bool,
    /// `TOP n`, which VFP allows only with an ORDER BY.
    pub top: Option<Expr>,
    /// `TOP n PERCENT`.
    pub top_percent: bool,
    pub columns: Vec<QueryColumn>,
    pub from: Vec<QuerySource>,
    pub where_: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderTerm>,
    pub into: QueryInto,
    /// `UNION [ALL] SELECT ...`: the next query whose rows go under this one's. The ORDER BY and
    /// the INTO belong to the union as a whole and are kept on the first query, however many
    /// SELECTs down the chain they were written after.
    pub union: Option<Box<QueryUnion>>,
    pub span: Span,
}

/// One `UNION` step of a query.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryUnion {
    /// `UNION ALL`: rows that appear in both sides are kept twice. Without it they are folded.
    pub all: bool,
    pub query: Query,
}

/// One item of the select list.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryColumn {
    /// `*`, or `alias.*`: every field of every source, or of one of them.
    All(Option<Name>),
    /// An expression, with the name it is given by `AS` or worked out from the expression.
    Value { expr: Expr, name: Option<Name> },
}

/// One table in the FROM clause, and how it is joined to the one before it.
#[derive(Debug, Clone, PartialEq)]
pub struct QuerySource {
    /// The table, as written: a name, a path, or an already-open alias.
    pub table: String,
    /// `FROM (cPath)`: the name worked out when the query runs rather than written out. A
    /// program that keeps its data folder in a variable names its tables this way.
    pub table_expr: Option<Expr>,
    /// The name its fields are qualified by: the AS alias, or the table's own name.
    pub alias: Name,
    pub join: JoinKind,
    /// True when it was written with JOIN rather than listed after a comma, so an ON that
    /// comes later knows which sources it may belong to.
    pub joined: bool,
    /// The ON condition, for a source that was joined rather than listed.
    pub on: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    /// Listed with a comma, or joined with INNER JOIN: only matching rows survive.
    Inner,
    /// LEFT OUTER JOIN: a row with no match keeps the left side and nulls the right.
    Left,
    /// FULL OUTER JOIN: a LEFT JOIN, and then the right-hand rows that matched nothing.
    /// RIGHT JOIN is not here: it is written down as the LEFT JOIN of the same two tables the
    /// other way round, because that is what it is.
    Full,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderTerm {
    pub expr: Expr,
    pub descending: bool,
}

/// One parameter of a `DECLARE ... DLL`: its type, and whether the library is given the
/// address of the value rather than the value.
#[derive(Debug, Clone, PartialEq)]
pub struct DllParam {
    pub kind: Name,
    pub by_ref: bool,
}

/// Where the result goes.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum QueryInto {
    /// No INTO: VFP browses the result. Here it goes to a cursor named QUERY, which is what a
    /// program can then read, and the IDE shows.
    #[default]
    Browse,
    Cursor(NameRef),
    /// `INTO TABLE` / `INTO DBF`: a table of its own on disk, left open in the work area the
    /// query ran into.
    Table(Expr),
    /// The array is whatever can be assigned to: a variable, or a property such as
    /// `THISFORM.aSamples`, which is where a form keeps the answer it just asked for.
    Array(Expr),
}

/// One column of a `CREATE CURSOR`.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorField {
    pub name: NameRef,
    /// The type letter as written: C, N, L, D, T, I, M and the rest.
    pub kind: char,
    pub width: u8,
    pub decimals: u8,
    /// `AUTOINC`: what the field takes for the first record, and how much it goes up by. A
    /// step of zero is every field that was not declared AUTOINC.
    pub autoinc_next: u32,
    pub autoinc_step: u8,
    /// `NULL` or `NOT NULL` as the column was written. `None` is a column that said neither,
    /// which takes whatever `SET NULL` is when the statement runs.
    pub nullable: Option<bool>,
}

/// Which records a table command applies to.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Scope {
    /// Every record, from the top. The default for SCAN and LOCATE.
    #[default]
    All,
    /// From where the pointer is to the end of the table.
    Rest,
    /// This record and the next n - 1, whether they match or not.
    Next(Expr),
    /// One record, by number.
    Record(Expr),
    /// The record the pointer is on, and it stays there. What REPLACE and DELETE mean with no
    /// scope of their own: Visual FoxPro does not move the pointer for those.
    Current,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Expr { kind, span }
    }
}

/// Date literal parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateLit {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeLit {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Plus,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    /// `=` (honours SET EXACT for strings)
    Eq,
    /// `==`
    ExactEq,
    /// `<>`, `#`, `!=`
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `$` substring containment
    Contains,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A number and the width it was written in: the characters it took and how many of them
    /// were past the point. `? 001` prints "  1", so the width has to survive the parse.
    Num(f64, u8, u8),
    /// `$12.34`: money, in ten-thousandths.
    Money(i64),
    Str(String),
    Bool(bool),
    Null,
    /// `{^2024-01-31}`; `None` is the empty date `{}` / `{//}`.
    Date(Option<DateLit>),
    /// `{^2024-01-31 10:30:00}`; `None` is the empty datetime `{/:}`.
    DateTime(Option<(DateLit, TimeLit)>),
    /// A skipped argument: `f(1, , 3)`. Evaluates to .F.
    Omitted,
    /// `LAMBDA(a, b)` ... `ENDLAMBDA`: a function value written where a value is wanted. It is
    /// the one expression in this language that contains statements.
    Lambda {
        params: Vec<Param>,
        body: Block,
        /// The line the `LAMBDA` is on, so the function it compiles to has one.
        line: u32,
    },
    Var(Name),
    This,
    ThisForm,
    ThisFormSet,
    Screen,
    /// The object of the innermost enclosing WITH; `.Caption` parses as `Member { WithRef, Caption }`.
    WithRef,
    Member {
        obj: Box<Expr>,
        name: Name,
    },
    /// `obj.&cName`: the member is named by a variable, and which one it is is only known when
    /// the expression runs. A call is the same thing with arguments.
    MemberByName {
        obj: Box<Expr>,
        name: Box<Expr>,
        args: Option<Vec<Arg>>,
    },
    /// `name(args)`: function call, or array element access when `name` is an array (decided at runtime).
    Call {
        name: Name,
        args: Vec<Arg>,
    },
    /// `laRoutes[1, 2](req, res)`: a call of whatever the expression in front of it yields,
    /// rather than of a name. Visual FoxPro refuses a `(` there with error 36, "Command
    /// contains unrecognized phrase/keyword." - measured - so nothing that works there changes
    /// meaning by this being read.
    CallValue {
        target: Box<Expr>,
        args: Vec<Arg>,
    },
    /// `base[args]`
    Index {
        base: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        obj: Box<Expr>,
        name: Name,
        args: Vec<Arg>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    /// `x IN (SELECT ...)`, `x = ANY (SELECT ...)` and `x = SOME (SELECT ...)`, which all ask the
    /// same question: is this value one of the ones that query returns.
    InSubquery {
        value: Box<Expr>,
        query: Box<Query>,
        negated: bool,
    },
    /// `EXISTS (SELECT ...)`: did that query return anything at all.
    ExistsSubquery(Box<Query>),
    /// `ANY`/`SOME` on the right of a comparison, before it is folded into [`ExprKind::InSubquery`].
    /// It never survives parsing; a comparison that cannot use it is a diagnostic.
    AnySubquery(Box<Query>),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `&name` in expression position, and `&aNames[i]`: what stands here is the text held by
    /// a memory variable, or by one element of an array of them.
    Macro(Box<Expr>),
}
