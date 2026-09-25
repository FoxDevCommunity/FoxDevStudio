//! Built-in function library. Functions are resolved at compile time to an index into
//! `REGISTRY` (VFP built-ins cannot be shadowed by user procedures), and arity errors are
//! compile diagnostics. Implementations only see `BuiltinCtx`, never the VM, so they are
//! unit-testable with a mock.

use crate::data::DataSession;
use crate::error::RtError;
use crate::host::{Host, HostRequest};
use crate::value::{Settings, Value};

pub mod array;
pub mod data;
pub mod datetime;
pub mod event;
pub mod lowlevel;
pub mod menu;
pub mod report;
pub mod xml;
pub mod screen;
pub mod numeric;
pub mod object;
pub mod string;
pub mod system;
pub mod ui;

/// What a built-in may ask of the VM.
pub trait BuiltinCtx {
    fn settings(&self) -> &Settings;
    fn settings_mut(&mut self) -> &mut Settings;
    fn host(&mut self) -> &mut dyn Host;
    /// Number of arguments the *current user procedure* was called with (PCOUNT()).
    fn pcount(&self) -> usize;
    /// Name of the current program/method (PROGRAM()).
    fn program_name(&self) -> String;
    /// How deep the call stack is, counting the running program as the top level: PROGRAM(-1).
    fn program_level(&self) -> usize;
    /// Name of the program at a 1-based call level, outermost first: PROGRAM(n).
    fn program_at(&self, level: usize) -> Option<String>;
    /// The file a stack level's routine was compiled from, for ASTACKINFO().
    fn module_at(&self, level: usize) -> Option<String>;
    /// Current line (LINENO()).
    fn line(&self) -> u32;
    /// The first line the current procedure/program runs (LINENO(1) counts from here).
    fn def_line(&self) -> u32;
    /// Evaluates FoxPro expression text in the current frame (EVALUATE(), TYPE()).
    fn evaluate(&mut self, expr: &str) -> Result<Value, RtError>;
    /// Value of a variable visible from the current frame, if any (TYPE("x"), VARTYPE fallbacks).
    fn lookup_variable(&self, upper_name: &str) -> Option<Value>;
    /// Last runtime error, for ERROR() / MESSAGE() inside ON ERROR handlers and CATCH blocks.
    fn last_error(&self) -> Option<RtError>;
    /// The command `ON ERROR` has installed, for ON("ERROR").
    fn on_error_command(&self) -> Option<String>;
    /// The command the rest of the `ON` family has installed, for the rest of ON().
    fn handler(&self, what: &str) -> Option<String>;
    /// A flag kept about one cursor: whether it is offline, whether it can be undone as one.
    fn cursor_flag(&self, alias: &str, name: &str) -> bool;
    /// The same, being set.
    fn set_cursor_flag(&mut self, alias: &str, name: &str, on: bool);
    /// SETRESULTSET(): the work area of the cursor a server hands back, which GETRESULTSET()
    /// reads - 0 when none is marked.
    fn result_set(&self) -> usize;
    fn set_result_set(&mut self, area: usize);
    /// The SELECT a view stands for, or nothing when the alias is not a view.
    fn view_sql(&self, alias: &str) -> String;
    /// The cursor a SQL statement filled: it replaces one of that name, as VFP replaces it.
    fn install_cursor(&mut self, alias: String, fields: Vec<crate::dbf::DbfField>, rows: Vec<crate::dbf::DbfRecord>);
    /// SQLGETPROP(): what a connection is set to, or the default when nothing set it.
    fn sql_property(&self, handle: i64, name: &str) -> crate::value::Value;
    /// SQLSETPROP(): the same, being set.
    fn set_sql_property(&mut self, handle: i64, name: &str, value: crate::value::Value);
    /// COMPROP(): a setting on how the runtime talks to that COM object.
    fn com_property(&self, obj: u32, name: &str) -> crate::value::Value;
    /// The same, being set.
    fn set_com_property(&mut self, obj: u32, name: &str, value: crate::value::Value);
    /// Creates or writes a variable by the name a string gives, the way `&x = value` or
    /// `CURSORTOXML()`'s memory-variable form does: the existing variable of that name wherever
    /// it is visible, or a new private of the calling scope when there is none yet.
    fn store_named(&mut self, name: &str, value: crate::value::Value) -> Result<(), RtError>;
    /// Keeps a picture LOADPICTURE() has decoded, and answers with the handle that finds it
    /// again. The pixels stay here rather than in the host, so one decoder serves both.
    fn keep_picture(&mut self, picture: crate::picture::Picture) -> f64;
    /// The picture that handle names, or nothing when it names none.
    fn picture(&self, handle: f64) -> Option<&crate::picture::Picture>;
    /// The members a class of that name has and what each starts out holding: a class the
    /// running program defines, or a Visual FoxPro base class. `None` when nothing knows the
    /// name. AMEMBERS() and GETPEM() take a class name as readily as an object.
    fn class_members(&mut self, class: &str) -> Option<Vec<crate::host::MemberInfo>>;
    /// The members an object has: the VM's own objects answer for themselves and everything
    /// else is the host's to answer for. `None` when nothing can enumerate it.
    fn object_members(&mut self, obj: crate::value::Handle) -> Option<Vec<crate::host::MemberInfo>>;
    /// The object whose method is running, for DODEFAULT().
    fn running_object(&self) -> Option<u32>;
    /// What that method is compiled as: `CLASS.EVENT` for a class of a program, `PATH.EVENT` for
    /// an object's own, with an ancestor's copy of an overridden method marked `EVENT#n`.
    fn running_method(&self) -> String;
    /// COMARRAY(): how an array is passed to that COM object, as it was last set.
    fn com_setting(&self, obj: u32) -> i64;
    /// The same, being set.
    fn set_com_setting(&mut self, obj: u32, how: i64);
    /// The library functions a program has declared: what it calls each one, the library it
    /// is in, and the name it has there.
    fn dlls(&self) -> Vec<(String, String, String)>;
    /// The work areas, for the functions that report on them.
    fn data(&self) -> &DataSession;
    /// The work areas, mutably, for the functions that have to read a record to answer.
    fn data_mut(&mut self) -> &mut DataSession;
    /// The answer to the data request this function asked for, when it is being run again.
    fn take_data_reply(&mut self) -> Option<Value>;
    /// The error the last low-level file function left, for FERROR().
    fn ferror(&self) -> i64;
    /// How deep the program is in transactions, for TXNLEVEL().
    fn txn_level(&self) -> u32;
    /// The menus the program has defined, for the functions that report on them.
    fn menus(&self) -> &crate::menu::Menus;
    /// The character screen and its windows, for the functions that report on those.
    fn screen(&self) -> &crate::screen::Screen;
    /// The same, to be changed: reading a key takes it out of the type-ahead buffer.
    fn screen_mut(&mut self) -> &mut crate::screen::Screen;
    /// The databases that are open, for the functions that report on one.
    fn databases(&self) -> &[crate::vm::Database];
    /// The same, to be changed: DBSETPROP writes a property into the container.
    fn databases_mut(&mut self) -> &mut Vec<crate::vm::Database>;
    /// Which of them is current, as SET DATABASE left it.
    fn current_database(&self) -> Option<usize>;
    /// Starts writing the current database's container back: the request that writes the first
    /// of its two files, or nothing when there is no database to write.
    fn write_current_database(&mut self) -> Option<HostRequest>;
    /// True while a container is part-way through being written.
    fn database_io_pending(&self) -> bool;
    /// The next file of that container, or nothing when it is all written.
    fn database_io_step(&mut self, reply: Option<Value>) -> Result<Option<HostRequest>, RtError>;
    /// Calls the stored procedure of that name in the current database, if it has one. False
    /// when the procedure said no to what was about to happen.
    fn database_event(&mut self, name: &str, args: &[crate::vm::DbcArg]) -> bool;
}

pub enum BuiltinResult {
    Value(Value),
    /// The VM yields the request; the resumed value becomes the function's result.
    Suspend(HostRequest),
    /// The VM yields a file request; the answer is a value and the error number FERROR() then
    /// reports, and the value becomes the function's result.
    SuspendFile(HostRequest),
    /// The VM yields a data request and runs this function again when the answer is in, with
    /// `args` back where they were. Reading a record can take more than one round trip, and the
    /// answer is not the function's result but something it needs to work it out.
    SuspendData { request: HostRequest, args: Vec<Value> },
    /// The VM compiles `source` as a program and calls it with `args`; what it returns becomes
    /// the function's result. `EXECSCRIPT()` is the whole of this: a builtin cannot run FoxPro
    /// itself, because the code it runs may stop for the host half way through.
    RunScript { source: String, args: Vec<Value> },
    /// The VM calls the function value with `args`; what it returns becomes the function's
    /// result. A built-in that has been handed a lambda returns this rather than calling it: a
    /// built-in cannot run FoxPro itself, because the code it runs may stop for the host half
    /// way through. See docs/foxscript.md.
    CallFunction { function: Value, args: Vec<Value> },
}

impl From<Value> for BuiltinResult {
    fn from(v: Value) -> Self {
        BuiltinResult::Value(v)
    }
}

pub type BuiltinFn = fn(&mut dyn BuiltinCtx, Vec<Value>) -> Result<BuiltinResult, RtError>;

pub struct BuiltinSpec {
    /// Upper-case name.
    pub name: &'static str,
    pub min_args: u8,
    /// `u8::MAX` for variadic.
    pub max_args: u8,
    pub func: BuiltinFn,
}

pub const VARIADIC: u8 = u8::MAX;

/// All built-ins; a function's id is its index here. Kept sorted by name.
pub fn registry() -> &'static [BuiltinSpec] {
    use std::sync::OnceLock;
    static REG: OnceLock<Vec<BuiltinSpec>> = OnceLock::new();
    REG.get_or_init(|| {
        let mut all: Vec<BuiltinSpec> = Vec::new();
        all.extend(string::specs());
        all.extend(numeric::specs());
        all.extend(datetime::specs());
        all.extend(system::specs());
        all.extend(array::specs());
        all.extend(object::specs());
        all.extend(ui::specs());
        all.extend(event::specs());
        all.extend(data::specs());
        all.extend(lowlevel::specs());
        all.extend(menu::specs());
        all.extend(report::specs());
        all.extend(xml::specs());
        all.extend(screen::specs());
        all.sort_by(|a, b| a.name.cmp(b.name));
        all.dedup_by(|a, b| a.name == b.name);
        all
    })
}

/// (id, spec) for an upper-cased name.
pub fn lookup(upper: &str) -> Option<(u16, &'static BuiltinSpec)> {
    let reg = registry();
    reg.binary_search_by(|s| s.name.cmp(upper)).ok().map(|i| (i as u16, &reg[i]))
}

/// The built-in a name means, allowing the abbreviations FoxPro allows.
///
/// FoxPro reads a function name by its first four letters: `ALLT()` is ALLTRIM, `TRANS()` is
/// TRANSFORM, `PROG()` is PROGRAM, `EVAL()` is EVALUATE, `CREATE()` is CREATEOBJECT. A name that
/// several functions could be short for is not resolved - the program has to say which - and a
/// name shorter than four letters is only ever itself.
pub fn lookup_abbreviated(upper: &str) -> Option<(u16, &'static BuiltinSpec)> {
    if let Some(found) = lookup(upper) {
        return Some(found);
    }
    if upper.len() < 4 {
        return None;
    }
    let reg = registry();
    let start = reg.partition_point(|s| &*s.name < upper);
    let mut matches = reg[start..].iter().enumerate().take_while(|(_, s)| s.name.starts_with(upper));
    let (offset, spec) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(((start + offset) as u16, spec))
}

pub fn by_id(id: u16) -> &'static BuiltinSpec {
    &registry()[id as usize]
}

/// Compile-time arity check. `None` when the count is acceptable.
pub fn arity_error(spec: &BuiltinSpec, argc: usize) -> Option<String> {
    if argc < spec.min_args as usize {
        Some(format!("{}() requires at least {} argument(s)", spec.name, spec.min_args))
    } else if spec.max_args != VARIADIC && argc > spec.max_args as usize {
        Some(format!("{}() takes at most {} argument(s)", spec.name, spec.max_args))
    } else {
        None
    }
}

/// The functions that are handed an array to fill, and which argument names it.
///
/// Every one of them makes the array when the program never declared it - `AFIELDS(laFields)`
/// on its own is how real Visual FoxPro code is written - so the compiler has to pass that
/// argument as a place to write to and not as a value it must read first.
const FILLS_AN_ARRAY: &[(&str, usize)] = &[
    ("ACLASS", 0),
    ("ACOPY", 1),
    ("ADATABASES", 0),
    ("ADBOBJECTS", 0),
    ("ADIR", 0),
    ("ADLLS", 0),
    ("AERROR", 0),
    ("AEVENTS", 0),
    ("AFIELDS", 0),
    ("AFONT", 0),
    ("AGETCLASS", 0),
    ("AGETFILEVERSION", 0),
    ("AINSTANCE", 0),
    ("ALINES", 0),
    ("AMEMBERS", 0),
    ("AMOUSEOBJ", 0),
    ("ANETRESOURCES", 0),
    ("APRINTERS", 0),
    ("APROCINFO", 0),
    ("ASELOBJ", 0),
    ("ASESSIONS", 0),
    ("ASQLHANDLES", 0),
    ("ASTACKINFO", 0),
    ("ATAGINFO", 0),
    ("AUSED", 0),
    ("AVCXCLASSES", 0),
];

/// Which argument of `upper`, if any, is an array the call fills in rather than reads.
pub fn array_to_fill(upper: &str) -> Option<usize> {
    FILLS_AN_ARRAY.iter().find(|(name, _)| *name == upper).map(|(_, at)| *at)
}

// ---- helpers shared by the implementations ----

pub fn arg_str(args: &[Value], i: usize) -> Result<std::rc::Rc<str>, RtError> {
    args.get(i).ok_or_else(RtError::function_arg_invalid)?.as_str().map_err(|_| RtError::function_arg_invalid())
}

pub fn arg_num(args: &[Value], i: usize) -> Result<f64, RtError> {
    args.get(i).ok_or_else(RtError::function_arg_invalid)?.as_number().map_err(|_| RtError::function_arg_invalid())
}

pub fn opt_num(args: &[Value], i: usize, default: f64) -> Result<f64, RtError> {
    match args.get(i) {
        None => Ok(default),
        Some(v) => v.as_number().map_err(|_| RtError::function_arg_invalid()),
    }
}

pub fn opt_str(args: &[Value], i: usize, default: &str) -> Result<String, RtError> {
    match args.get(i) {
        None => Ok(default.to_string()),
        Some(v) => Ok(v.as_str().map_err(|_| RtError::function_arg_invalid())?.to_string()),
    }
}

pub fn opt_bool(args: &[Value], i: usize, default: bool) -> bool {
    match args.get(i).map(|v| v.deref()) {
        None | Some(Value::Null) => default,
        Some(Value::Logical(b)) => b,
        Some(Value::Number(n, ..)) => n != 0.0,
        Some(_) => default,
    }
}

/// `Ok(BuiltinResult::Value(v))`, the shape almost every implementation ends with.
pub fn ok(v: Value) -> Result<BuiltinResult, RtError> {
    Ok(BuiltinResult::Value(v))
}

/// True when any argument is (or references) NULL; most built-ins then return NULL.
pub fn any_null(args: &[Value]) -> bool {
    args.iter().any(|v| v.is_null())
}

/// A truncating integer argument that accepts negatives (unlike `Value::as_usize`).
pub fn arg_int(args: &[Value], i: usize) -> Result<i64, RtError> {
    Ok(arg_num(args, i)?.trunc() as i64)
}

pub fn opt_int(args: &[Value], i: usize, default: i64) -> Result<i64, RtError> {
    match args.get(i).map(|v| v.deref()) {
        None | Some(Value::Null) => Ok(default),
        Some(_) => arg_int(args, i),
    }
}

/// Constructor used by the per-module `specs()` tables.
pub const fn spec(name: &'static str, min_args: u8, max_args: u8, func: BuiltinFn) -> BuiltinSpec {
    BuiltinSpec { name, min_args, max_args, func }
}

/// Bodies for functions that are recognised (so a call reports a clear "not available yet"
/// instead of "procedure not found") but are not implemented in this milestone.
///
/// The message is what the user sees, so it has a shape: `NAME(): what is missing, when it
/// arrives, and what to reach for meanwhile`. A message that is only the function name tells
/// the reader nothing they did not already know - `builtins_system.rs` asserts against that.
/// When something is never coming (COM automation), say so instead of naming a milestone.
macro_rules! not_available {
    ($($fname:ident => $what:expr;)*) => {
        $(
            fn $fname(_ctx: &mut dyn crate::builtins::BuiltinCtx, _args: Vec<crate::value::Value>)
                -> Result<crate::builtins::BuiltinResult, crate::error::RtError> {
                Err(crate::error::RtError::feature_not_available($what))
            }
        )*
    };
}
pub(crate) use not_available;
