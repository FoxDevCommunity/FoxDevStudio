//! A minimal `BuiltinCtx` for the built-in function tests: fixed clock, deterministic random,
//! a tiny object map and canned EVALUATE results.

#![allow(dead_code)]

use std::collections::HashMap;

use foxvm::builtins::{BuiltinCtx, BuiltinResult, arity_error, lookup};
use foxvm::error::RtError;
use foxvm::host::{Host, HostRequest, Member, MemberInfo};
use foxvm::value::{DateFormat, FoxArray, Handle, Settings, Value, days_from_civil};

/// 2026-09-07 (a Monday) at 13:05:07.
pub fn today() -> i32 {
    days_from_civil(2026, 9, 7)
}

pub const NOW_SECONDS: f64 = 13.0 * 3600.0 + 5.0 * 60.0 + 7.0;

pub struct TestHost {
    pub output: Vec<String>,
    pub props: HashMap<(u32, String), Value>,
    pub members: HashMap<(u32, String), Member>,
    pub classes: HashMap<u32, String>,
    seed: u64,
}

impl Default for TestHost {
    fn default() -> Self {
        let mut props = HashMap::new();
        props.insert((1, "CAPTION".to_string()), Value::str("Hello"));
        let mut members = HashMap::new();
        members.insert((1, "CAPTION".to_string()), Member::Property);
        members.insert((1, "CLICK".to_string()), Member::Method);
        let mut classes = HashMap::new();
        classes.insert(1, "Form".to_string());
        TestHost { output: Vec::new(), props, members, classes, seed: 1 }
    }
}

impl Host for TestHost {
    fn get_prop(&mut self, obj: Handle, name: &str) -> Result<Value, RtError> {
        self.props.get(&(obj.0, name.to_ascii_uppercase())).cloned().ok_or_else(|| RtError::property_not_found(name))
    }

    fn get_member(&mut self, obj: Handle, name: &str) -> Result<Member, RtError> {
        Ok(self.members.get(&(obj.0, name.to_ascii_uppercase())).cloned().unwrap_or(Member::None))
    }

    fn object_class(&mut self, obj: Handle) -> Option<String> {
        self.classes.get(&obj.0).cloned()
    }

    fn members(&mut self, obj: Handle) -> Option<Vec<MemberInfo>> {
        self.classes.get(&obj.0)?;
        let mut out = Vec::new();
        for ((handle, name), member) in &self.members {
            if *handle != obj.0 {
                continue;
            }
            let mut info = MemberInfo::property(name, Value::Logical(false));
            match member {
                Member::Method => {
                    info.kind = foxvm::host::MemberKind::Method;
                    info.value = None;
                }
                Member::Child(h) => {
                    info.kind = foxvm::host::MemberKind::Object;
                    info.value = Some(Value::Object(*h));
                }
                _ => info.value = self.props.get(&(obj.0, name.clone())).cloned(),
            }
            out.push(info);
        }
        Some(out)
    }

    fn output(&mut self, text: &str, newline: bool) {
        self.output.push(if newline { format!("{text}\n") } else { text.to_string() });
    }

    fn now(&mut self) -> (i32, f64) {
        (today(), NOW_SECONDS)
    }

    fn random(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.seed >> 11) % 1_000_000) as f64 / 1_000_000.0
    }

    fn seed_random(&mut self, seed: f64) {
        self.seed = seed as i64 as u64;
    }

    fn os_info(&mut self) -> String {
        "Windows|10|0|26200".to_string()
    }

    fn resolve_program(&mut self, _name: &str) -> Option<u32> {
        None
    }
}

pub struct TestCtx {
    pub settings: Settings,
    pub host: TestHost,
    /// EVALUATE()/TYPE() answers, keyed by the expression text.
    pub evals: HashMap<String, Value>,
    pub pcount: usize,
    pub program: String,
    pub line: u32,
    pub last_error: Option<RtError>,
    /// Work areas the data functions report on; empty unless a test opens one.
    pub data: foxvm::data::DataSession,
    /// What ON ERROR has installed, for ON("ERROR").
    pub on_error: Option<String>,
    /// The databases the container functions report on; empty unless a test opens one.
    pub databases: Vec<foxvm::vm::Database>,
    /// The menus the menu functions report on; empty unless a test defines one.
    pub menus: foxvm::menu::Menus,
    /// The character screen the screen functions report on.
    pub screen: foxvm::screen::Screen,
    /// The pictures LOADPICTURE() has read on this context.
    pub pictures: foxvm::picture::Pictures,
    /// Variables `store_named()` has written, upper-cased - what `CURSORTOXML()`'s memory
    /// variable output lands in here instead of a real frame's privates.
    pub named: HashMap<String, Value>,
}

impl Default for TestCtx {
    fn default() -> Self {
        let mut evals = HashMap::new();
        evals.insert("nAge".to_string(), Value::number(42.0));
        evals.insert("cName".to_string(), Value::str("Ada"));
        evals.insert("lOk".to_string(), Value::Logical(true));
        evals.insert("oNull".to_string(), Value::Null);
        evals.insert("1+1".to_string(), Value::number(2.0));
        TestCtx {
            settings: Settings::default(),
            host: TestHost::default(),
            evals,
            pcount: 2,
            program: "TESTPRG".to_string(),
            line: 7,
            last_error: None,
            data: foxvm::data::DataSession::new(),
            on_error: None,
            databases: Vec::new(),
            menus: foxvm::menu::Menus::default(),
            screen: foxvm::screen::Screen::default(),
            pictures: foxvm::picture::Pictures::default(),
            named: HashMap::new(),
        }
    }
}

impl TestCtx {
    pub fn with_settings(f: impl FnOnce(&mut Settings)) -> TestCtx {
        let mut ctx = TestCtx::default();
        f(&mut ctx.settings);
        ctx
    }

    pub fn century() -> TestCtx {
        TestCtx::with_settings(|s| s.century = true)
    }

    pub fn dated(format: DateFormat) -> TestCtx {
        TestCtx::with_settings(|s| s.date_format = format)
    }
}

impl BuiltinCtx for TestCtx {
    fn data(&self) -> &foxvm::data::DataSession {
        &self.data
    }
    fn data_mut(&mut self) -> &mut foxvm::data::DataSession {
        &mut self.data
    }
    fn handler(&self, _what: &str) -> Option<String> {
        None
    }
    fn on_error_command(&self) -> Option<String> {
        self.on_error.clone()
    }
    fn take_data_reply(&mut self) -> Option<Value> {
        // nothing here asks the host for bytes; the cursors are built with their pages in hand
        None
    }
    fn settings(&self) -> &Settings {
        &self.settings
    }
    fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
    fn host(&mut self) -> &mut dyn Host {
        &mut self.host
    }
    fn pcount(&self) -> usize {
        self.pcount
    }
    fn program_name(&self) -> String {
        self.program.clone()
    }
    fn program_level(&self) -> usize {
        1
    }
    fn ferror(&self) -> i64 {
        0
    }
    fn txn_level(&self) -> u32 {
        0
    }
    fn cursor_flag(&self, _alias: &str, _name: &str) -> bool {
        false
    }

    fn set_cursor_flag(&mut self, _alias: &str, _name: &str, _on: bool) {}

    fn result_set(&self) -> usize {
        0
    }

    fn set_result_set(&mut self, _area: usize) {}

    fn view_sql(&self, _alias: &str) -> String {
        String::new()
    }

    fn install_cursor(&mut self, _alias: String, _fields: Vec<foxvm::dbf::DbfField>, _rows: Vec<foxvm::dbf::DbfRecord>) {}

    fn sql_property(&self, _handle: i64, _name: &str) -> foxvm::value::Value {
        foxvm::value::Value::Logical(false)
    }

    fn set_sql_property(&mut self, _handle: i64, _name: &str, _value: foxvm::value::Value) {}

    fn com_property(&self, _obj: u32, _name: &str) -> foxvm::value::Value {
        foxvm::value::Value::Logical(false)
    }

    fn set_com_property(&mut self, _obj: u32, _name: &str, _value: foxvm::value::Value) {}

    fn store_named(&mut self, name: &str, value: foxvm::value::Value) -> Result<(), RtError> {
        self.named.insert(name.to_ascii_uppercase(), value);
        Ok(())
    }

    /// The pictures one run has loaded, so LOADPICTURE() has somewhere to leave one.
    fn keep_picture(&mut self, picture: foxvm::picture::Picture) -> f64 {
        self.pictures.keep(picture)
    }

    fn picture(&self, handle: f64) -> Option<&foxvm::picture::Picture> {
        self.pictures.get(handle)
    }

    /// The classes here are the product's own base classes; nothing defines one of its own,
    /// because a DEFINE CLASS needs a compiled program and these tests have none.
    fn object_members(&mut self, obj: Handle) -> Option<Vec<MemberInfo>> {
        match foxvm::foxscript::Natives::default().member_info(obj) {
            Some(members) => Some(members),
            None => self.host().members(obj),
        }
    }

    fn class_members(&mut self, class: &str) -> Option<Vec<MemberInfo>> {
        let base = foxvm::base_classes::find(class)?;
        let mut out: Vec<MemberInfo> = Vec::new();
        for (name, default) in &base.properties {
            let mut info = MemberInfo::property(name, default.to_value());
            info.native = true;
            info.read_only = base.read_only.iter().any(|(n, _)| n == name);
            out.push(info);
        }
        for (names, kind) in [
            (&base.events, foxvm::host::MemberKind::Event),
            (&base.methods, foxvm::host::MemberKind::Method),
        ] {
            for name in names {
                let mut info = MemberInfo::property(name, Value::Logical(false));
                info.kind = kind;
                info.native = true;
                info.value = None;
                out.push(info);
            }
        }
        Some(out)
    }

    fn running_object(&self) -> Option<u32> {
        None
    }

    fn running_method(&self) -> String {
        String::new()
    }

    fn com_setting(&self, _obj: u32) -> i64 {
        0
    }

    fn set_com_setting(&mut self, _obj: u32, _how: i64) {}

    fn dlls(&self) -> Vec<(String, String, String)> {
        Vec::new()
    }

    fn menus(&self) -> &foxvm::menu::Menus {
        &self.menus
    }
    fn screen(&self) -> &foxvm::screen::Screen {
        &self.screen
    }
    fn screen_mut(&mut self) -> &mut foxvm::screen::Screen {
        &mut self.screen
    }
    fn databases(&self) -> &[foxvm::vm::Database] {
        &self.databases
    }
    fn databases_mut(&mut self) -> &mut Vec<foxvm::vm::Database> {
        &mut self.databases
    }
    fn current_database(&self) -> Option<usize> {
        None
    }
    fn write_current_database(&mut self) -> Option<foxvm::host::HostRequest> {
        None
    }
    fn database_io_pending(&self) -> bool {
        false
    }
    fn database_io_step(&mut self, _reply: Option<Value>) -> Result<Option<foxvm::host::HostRequest>, RtError> {
        Ok(None)
    }
    fn database_event(&mut self, _name: &str, _args: &[foxvm::vm::DbcArg]) -> bool {
        true
    }
    fn program_at(&self, level: usize) -> Option<String> {
        (level == 1).then(|| self.program.clone())
    }
    fn module_at(&self, level: usize) -> Option<String> {
        (level == 1).then(|| self.program.clone())
    }
    fn line(&self) -> u32 {
        self.line
    }
    fn def_line(&self) -> u32 {
        1
    }
    fn evaluate(&mut self, expr: &str) -> Result<Value, RtError> {
        self.evals.get(expr).cloned().ok_or_else(|| RtError::variable_not_found(expr))
    }
    fn lookup_variable(&self, upper_name: &str) -> Option<Value> {
        self.evals.get(upper_name).cloned()
    }
    fn last_error(&self) -> Option<RtError> {
        self.last_error.clone()
    }
}

// ---- calling helpers -------------------------------------------------------------------

/// Calls a built-in on `ctx`, asserting the argument count is one the spec accepts.
pub fn call_on(ctx: &mut TestCtx, name: &str, args: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (_, spec) = lookup(name).unwrap_or_else(|| panic!("{name}() is not registered"));
    if let Some(err) = arity_error(spec, args.len()) {
        panic!("{err}");
    }
    (spec.func)(ctx, args)
}

pub fn call(name: &str, args: Vec<Value>) -> Result<BuiltinResult, RtError> {
    call_on(&mut TestCtx::default(), name, args)
}

pub fn value_on(ctx: &mut TestCtx, name: &str, args: Vec<Value>) -> Value {
    match call_on(ctx, name, args) {
        Ok(BuiltinResult::Value(v)) => v,
        Ok(
            BuiltinResult::Suspend(_)
            | BuiltinResult::SuspendFile(_)
            | BuiltinResult::SuspendData { .. }
            | BuiltinResult::RunScript { .. }
            | BuiltinResult::CallFunction { .. },
        ) => panic!("{name}() suspended, expected a value"),
        Err(e) => panic!("{name}() failed: {e}"),
    }
}

pub fn value(name: &str, args: Vec<Value>) -> Value {
    value_on(&mut TestCtx::default(), name, args)
}

/// The character result of a built-in.
pub fn text(name: &str, args: Vec<Value>) -> String {
    text_on(&mut TestCtx::default(), name, args)
}

pub fn text_on(ctx: &mut TestCtx, name: &str, args: Vec<Value>) -> String {
    match value_on(ctx, name, args) {
        Value::Str(s) => s.to_string(),
        other => panic!("{name}() returned {other:?}, expected character"),
    }
}

pub fn num(name: &str, args: Vec<Value>) -> f64 {
    num_on(&mut TestCtx::default(), name, args)
}

pub fn num_on(ctx: &mut TestCtx, name: &str, args: Vec<Value>) -> f64 {
    match value_on(ctx, name, args) {
        Value::Number(n, ..) => n,
        other => panic!("{name}() returned {other:?}, expected numeric"),
    }
}

pub fn flag(name: &str, args: Vec<Value>) -> bool {
    match value(name, args) {
        Value::Logical(b) => b,
        other => panic!("{name}() returned {other:?}, expected logical"),
    }
}

pub fn request(name: &str, args: Vec<Value>) -> HostRequest {
    match call(name, args) {
        Ok(BuiltinResult::Suspend(r) | BuiltinResult::SuspendFile(r)) => r,
        Ok(BuiltinResult::SuspendData { request, .. }) => request,
        Ok(BuiltinResult::Value(v)) => panic!("{name}() returned {v:?}, expected a host request"),
        Ok(BuiltinResult::RunScript { .. }) => panic!("{name}() asked to run a script"),
        Ok(BuiltinResult::CallFunction { .. }) => panic!("{name}() asked to call a function"),
        Err(e) => panic!("{name}() failed: {e}"),
    }
}

pub fn err(name: &str, args: Vec<Value>) -> RtError {
    match call(name, args) {
        Err(e) => e,
        Ok(_) => panic!("{name}() unexpectedly succeeded"),
    }
}

// ---- value shorthands ------------------------------------------------------------------

pub fn s(x: &str) -> Value {
    Value::str(x)
}

pub fn n(x: f64) -> Value {
    Value::number(x)
}

pub fn d(y: i32, m: u32, day: u32) -> Value {
    Value::Date(Some(days_from_civil(y, m, day)))
}

pub fn dt(y: i32, m: u32, day: u32, h: f64, mi: f64, sec: f64) -> Value {
    Value::DateTime(Some(days_from_civil(y, m, day) as f64 * 86400.0 + h * 3600.0 + mi * 60.0 + sec))
}

/// A one-dimensional array of values, shared by reference like a VFP array parameter.
pub fn array(items: Vec<Value>) -> Value {
    let mut a = FoxArray::new(items.len(), 0);
    a.items = items;
    Value::Array(std::rc::Rc::new(std::cell::RefCell::new(a)))
}

pub fn array2(rows: usize, cols: usize, items: Vec<Value>) -> Value {
    let mut a = FoxArray::new(rows, cols);
    for (i, v) in items.into_iter().enumerate() {
        a.items[i] = v;
    }
    Value::Array(std::rc::Rc::new(std::cell::RefCell::new(a)))
}

/// Snapshot of an array value as a vector of elements.
pub fn items(v: &Value) -> Vec<Value> {
    match v.deref() {
        Value::Array(a) => a.borrow().items.clone(),
        other => panic!("{other:?} is not an array"),
    }
}

/// Snapshot of an array of character values.
pub fn texts(v: &Value) -> Vec<String> {
    items(v)
        .iter()
        .map(|v| match v {
            Value::Str(s) => s.to_string(),
            other => format!("{other:?}"),
        })
        .collect()
}
