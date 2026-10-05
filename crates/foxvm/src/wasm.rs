//! wasm-bindgen exports. Kept thin: marshalling only, no language logic.
//!
//! Values cross the bridge in the `host::JsonValue` shape: `null`, booleans, numbers, strings,
//! `{ $obj: handle }`, `{ $date: 'YYYY-MM-DD' | '' }`, `{ $dt: seconds | NaN }` and
//! `{ $arr: [...], $cols }`. Host reads are methods on the JavaScript object given to
//! `FoxVm.new`: `getProp(obj, name)`, `getMember(obj, name)` (a number for a child handle or
//! `"prop" | "method" | "none"`), `objectClass(obj)` (string or null), `output(text, newline)`,
//! `now()` (an object `{ days, secs }`), `random()` and `resolveProgram(name)` (-1 for none).

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::bytecode::{Module, decode, encode};
use crate::compiler::{self, CompileResult, MethodSource};
use crate::dbf::{DbfField, DbfValue, read_table};
use crate::diagnostics::Diagnostic;
use crate::error::RtError;
use crate::host::{ClassDefOut, Host, JsonValue, Member, StepMode};
use crate::value::{Handle, Value};
use crate::vm::{Step, Vm};

#[wasm_bindgen]
pub fn version() -> String {
    crate::VERSION.to_string()
}

#[derive(Serialize)]
struct CheckResult {
    diagnostics: Vec<Diagnostic>,
}

fn to_js<T: Serialize>(v: &T) -> JsValue {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    v.serialize(&serializer).unwrap_or(JsValue::NULL)
}

fn from_js(v: JsValue) -> Value {
    if v.is_undefined() || v.is_null() {
        return Value::Null;
    }
    serde_wasm_bindgen::from_value::<JsonValue>(v).map(|j| j.to_value()).unwrap_or(Value::Null)
}

fn args_from_js(args: JsValue) -> Vec<Value> {
    if args.is_undefined() || args.is_null() {
        return Vec::new();
    }
    serde_wasm_bindgen::from_value::<Vec<JsonValue>>(args)
        .map(|v| v.iter().map(JsonValue::to_value).collect())
        .unwrap_or_default()
}

/// Parses `src` as a `"program"`, `"method"` or `"expression"` and returns
/// `{ diagnostics: Diagnostic[] }` as a plain JavaScript object.
#[wasm_bindgen]
pub fn check(src: &str, kind: &str) -> JsValue {
    let diagnostics = match kind {
        "expression" => match crate::parser::parse_expression(src) {
            Ok(_) => Vec::new(),
            Err(diags) => diags,
        },
        "method" => crate::parser::parse_method(src).diagnostics,
        _ => crate::parser::parse_program(src).diagnostics,
    };
    to_js(&CheckResult { diagnostics })
}

// ---- DBF tables ------------------------------------------------------------------------------

#[derive(Serialize)]
struct DbfOk<'a> {
    ok: bool,
    version: u8,
    codepage: Option<u16>,
    fields: &'a [DbfField],
    records: Vec<DbfRecordOut<'a>>,
}

#[derive(Serialize)]
struct DbfErr {
    ok: bool,
    error: String,
}

#[derive(Serialize)]
struct DbfRecordOut<'a> {
    deleted: bool,
    values: Vec<DbfValueOut<'a>>,
}

/// A field value as a plain JavaScript value: string, number, boolean, `null`, or one of the
/// tagged objects `{ $date }`, `{ $dt }` and `{ $bytes }`.
struct DbfValueOut<'a>(&'a DbfValue);

impl Serialize for DbfValueOut<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self.0 {
            DbfValue::Null => s.serialize_none(),
            DbfValue::Text(t) | DbfValue::Memo(t) => s.serialize_str(t),
            DbfValue::Number(n) => s.serialize_f64(*n),
            // the host reads money as the amount; the type stays in the VM, where VARTYPE() is
            DbfValue::Currency(c) => s.serialize_f64(*c as f64 / crate::dbf::CURRENCY_SCALE as f64),
            DbfValue::Logical(b) => s.serialize_bool(*b),
            DbfValue::Date(None) | DbfValue::DateTime(None) => s.serialize_none(),
            DbfValue::Date(Some(days)) => {
                let (y, m, d) = crate::value::civil_from_days(*days);
                let mut map = s.serialize_map(Some(1))?;
                map.serialize_entry("$date", &format!("{y:04}-{m:02}-{d:02}"))?;
                map.end()
            }
            DbfValue::DateTime(Some(secs)) => {
                let mut map = s.serialize_map(Some(1))?;
                map.serialize_entry("$dt", secs)?;
                map.end()
            }
            DbfValue::Bytes(b) => {
                let mut map = s.serialize_map(Some(1))?;
                map.serialize_entry("$bytes", &base64(b))?;
                map.end()
            }
        }
    }
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n =
            (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(ALPHABET[(n >> 18 & 63) as usize] as char);
        out.push(ALPHABET[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 { ALPHABET[(n >> 6 & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { ALPHABET[(n & 63) as usize] as char } else { '=' });
    }
    out
}

#[derive(Serialize)]
struct DbfHeaderOut<'a> {
    ok: bool,
    version: u8,
    codepage: Option<u16>,
    fields: &'a [DbfField],
    /// Records the header claims the file holds.
    record_count: f64,
    /// Bytes before the first record, and bytes per record: what turns a record number into an
    /// offset, which is the only arithmetic a browser has to do to page through a table.
    header_len: usize,
    record_len: usize,
    has_memo: bool,
}

/// Reads only the header of a table, from its first `header_len` bytes.
///
/// This is the table browser's way in: the header says how wide a record is, the browser asks the
/// host for a run of them, and [`decode_dbf_page`] turns those bytes into values. A table larger
/// than memory is read the same way as a small one.
#[wasm_bindgen]
pub fn read_dbf_header(dbf: &[u8]) -> JsValue {
    match crate::dbf::read_header(dbf) {
        Ok(h) => to_js(&DbfHeaderOut {
            ok: true,
            version: h.version,
            codepage: h.codepage,
            record_count: h.record_count as f64,
            header_len: h.header_len,
            record_len: h.record_len,
            has_memo: h.has_memo(),
            fields: &h.fields,
        }),
        Err(e) => to_js(&DbfErr { ok: false, error: e.message }),
    }
}

/// Decodes a run of records against a header.
///
/// `page` is `n * record_len` bytes taken from `header_len + (first - 1) * record_len`; a short
/// block simply yields fewer records. Memo fields come back as their block number rather than
/// their text - the browser shows "Memo" for them, as Visual FoxPro's BROWSE does, and fetches
/// one only when it is opened.
#[wasm_bindgen]
pub fn decode_dbf_page(header: &[u8], page: &[u8]) -> JsValue {
    let Ok(h) = crate::dbf::read_header(header) else {
        return to_js(&DbfErr { ok: false, error: "the header could not be read".into() });
    };
    let decoded: Vec<crate::dbf::DbfRecord> = page
        .chunks_exact(h.record_len)
        .map(|raw| {
            let mut r = crate::dbf::decode_record(&h, raw, crate::dbf::Padding::Keep, |_| None);
            for (i, (field, &(start, width))) in h.fields.iter().zip(&h.layout).enumerate() {
                if matches!(field.kind, 'M' | 'G' | 'P') {
                    r.values[i] = match raw.get(start..start + width).and_then(crate::dbf::memo_pointer) {
                        Some(block) => DbfValue::Number(f64::from(block)),
                        None => DbfValue::Null,
                    };
                }
            }
            r
        })
        .collect();
    let records = decoded
        .iter()
        .map(|r| DbfRecordOut { deleted: r.deleted, values: r.values.iter().map(DbfValueOut).collect() })
        .collect();
    to_js(&DbfPageOut { ok: true, records })
}

#[derive(Serialize)]
struct DbfPageOut<'a> {
    ok: bool,
    records: Vec<DbfRecordOut<'a>>,
}

/// Reads a `.dbf` table, with the bytes of its `.fpt`/`.pjt` memo file when it has one.
///
/// Returns `{ ok: true, version, codepage, fields, records }` or, for a table that cannot be
/// read at all, `{ ok: false, error }` - it never throws.
#[wasm_bindgen]
pub fn read_dbf(dbf: &[u8], memo: Option<Box<[u8]>>) -> JsValue {
    match read_table(dbf, memo.as_deref()) {
        Ok(table) => {
            let records = table
                .records
                .iter()
                .map(|r| DbfRecordOut { deleted: r.deleted, values: r.values.iter().map(DbfValueOut).collect() })
                .collect();
            to_js(&DbfOk { ok: true, version: table.version, codepage: table.codepage, fields: &table.fields, records })
        }
        Err(e) => to_js(&DbfErr { ok: false, error: e.message }),
    }
}

#[derive(Serialize)]
struct EncodedOut {
    ok: bool,
    /// Where the field starts inside a record, and the bytes to put there, one character each.
    offset: usize,
    bytes: String,
}

/// Encodes one field of a record, for the table browser writing a cell back.
///
/// `text` is what the user typed; it is read as the field's type. The answer is where in the
/// record the field starts and the bytes that belong there, so the caller writes a fixed run at a
/// known offset and never has to know what a DBF is.
#[wasm_bindgen]
pub fn encode_dbf_field(header: &[u8], field: usize, text: &str) -> JsValue {
    let h = match crate::dbf::read_header(header) {
        Ok(h) => h,
        Err(e) => return to_js(&DbfErr { ok: false, error: e.message }),
    };
    let (Some(descriptor), Some(&(offset, width))) = (h.fields.get(field), h.layout.get(field)) else {
        return to_js(&DbfErr { ok: false, error: format!("the table has no field {field}") });
    };
    let value = match parse_for(descriptor.kind, text) {
        Ok(v) => v,
        Err(e) => return to_js(&DbfErr { ok: false, error: e }),
    };
    match crate::dbf::write::encode_field(descriptor, width, &value, h.codepage) {
        Ok(bytes) => to_js(&EncodedOut { ok: true, offset, bytes: bytes.iter().map(|&b| b as char).collect() }),
        Err(e) => to_js(&DbfErr { ok: false, error: e.message }),
    }
}

/// Reads typed text as the value a field of that type holds. The messages are what the browser
/// shows next to the cell, so they say what was expected rather than naming a type letter.
fn parse_for(kind: char, text: &str) -> Result<Value, String> {
    let trimmed = text.trim();
    match kind {
        'N' | 'F' | 'I' | '+' | 'B' | 'O' | 'Y' => {
            if trimmed.is_empty() {
                return Ok(Value::number(0.0));
            }
            trimmed.parse::<f64>().map(Value::number).map_err(|_| format!("{text:?} is not a number"))
        }
        'L' => Ok(Value::Logical(matches!(trimmed.chars().next(), Some('T' | 't' | 'Y' | 'y' | '1')))),
        'D' => {
            if trimmed.is_empty() {
                return Ok(Value::Date(None));
            }
            parse_iso_date(trimmed).map(|d| Value::Date(Some(d))).ok_or_else(|| "a date reads as yyyy-mm-dd".into())
        }
        _ => Ok(Value::str(text)),
    }
}

fn parse_iso_date(text: &str) -> Option<i32> {
    let mut parts = text.split(['-', '/', '.']);
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    crate::value::is_valid_date(y, m, d).then(|| crate::value::days_from_civil(y, m, d))
}

// ---- host bridge --------------------------------------------------------------------------

#[wasm_bindgen]
extern "C" {
    pub type HostReads;
    #[wasm_bindgen(method, structural, js_name = getProp)]
    fn get_prop(this: &HostReads, obj: u32, name: &str) -> JsValue;
    #[wasm_bindgen(method, structural, js_name = getMember)]
    fn get_member(this: &HostReads, obj: u32, name: &str) -> JsValue;
    #[wasm_bindgen(method, structural, js_name = objectClass)]
    fn object_class(this: &HostReads, obj: u32) -> JsValue;
    #[wasm_bindgen(method, structural, js_name = objectFile)]
    fn object_file(this: &HostReads, obj: u32) -> JsValue;
    #[wasm_bindgen(method, structural)]
    fn output(this: &HostReads, text: &str, newline: bool);
    #[wasm_bindgen(method, structural)]
    fn now(this: &HostReads) -> JsValue;
    #[wasm_bindgen(method, structural)]
    fn random(this: &HostReads) -> f64;
    #[wasm_bindgen(method, structural, js_name = osInfo)]
    fn os_info(this: &HostReads) -> JsValue;
    #[wasm_bindgen(method, structural, js_name = mouse)]
    fn mouse(this: &HostReads) -> JsValue;
    #[wasm_bindgen(method, structural, js_name = hasClassMethod)]
    fn class_method(this: &HostReads, obj: u32, name: &str) -> bool;
    #[wasm_bindgen(method, structural, js_name = resolveProgram)]
    fn resolve_program(this: &HostReads, name: &str) -> f64;
}

/// `errorRaised` is optional: a host that does not log errors simply does not define it, and a
/// structural call to a member that is not there would throw across the wasm boundary.
fn optional_method(reads: &HostReads, name: &str) -> Option<js_sys::Function> {
    let value = js_sys::Reflect::get(reads.as_ref(), &JsValue::from_str(name)).ok()?;
    value.dyn_into::<js_sys::Function>().ok()
}

struct JsHost {
    reads: HostReads,
    /// Set once a program calls `RAND(nSeed)`: from then on the sequence is this runtime's, not
    /// the browser's, because a seeded sequence has to be the same one every time.
    seeded: Option<crate::host::SeededRandom>,
}

impl Host for JsHost {
    fn get_prop(&mut self, obj: Handle, name: &str) -> Result<Value, RtError> {
        let v = self.reads.get_prop(obj.0, name);
        if v.is_undefined() { Err(RtError::property_not_found(name)) } else { Ok(from_js(v)) }
    }

    fn get_member(&mut self, obj: Handle, name: &str) -> Result<Member, RtError> {
        let v = self.reads.get_member(obj.0, name);
        if let Some(n) = v.as_f64() {
            return Ok(Member::Child(Handle(n as u32)));
        }
        Ok(match v.as_string().as_deref() {
            Some("prop") => Member::Property,
            Some("method") => Member::Method,
            _ => Member::None,
        })
    }

    /// Optional as well: `hasCodeMethod(obj, name)`, which is how Access and Assign methods are found.
    fn has_code_method(&mut self, obj: Handle, name: &str) -> bool {
        optional_method(&self.reads, "hasCodeMethod")
            .and_then(|f| f.call2(self.reads.as_ref(), &JsValue::from(obj.0), &JsValue::from_str(name)).ok())
            .and_then(|answer| answer.as_bool())
            .unwrap_or(false)
    }

    /// Optional as well: `released(obj)` is true for a form that has gone.
    fn released(&mut self, obj: Handle) -> bool {
        optional_method(&self.reads, "released")
            .and_then(|f| f.call1(self.reads.as_ref(), &JsValue::from(obj.0)).ok())
            .and_then(|answer| answer.as_bool())
            .unwrap_or(false)
    }

    /// Optional too: `enumerate(obj, member)` answers the collection as an array, or nothing.
    fn enumerate(&mut self, obj: Handle, member: &str) -> Option<Value> {
        let f = optional_method(&self.reads, "enumerate")?;
        let answer = f.call2(self.reads.as_ref(), &JsValue::from(obj.0), &JsValue::from_str(member)).ok()?;
        if answer.is_undefined() || answer.is_null() {
            return None;
        }
        match from_js(answer) {
            items @ Value::Array(_) => Some(items),
            _ => None,
        }
    }

    fn object_class(&mut self, obj: Handle) -> Option<String> {
        self.reads.object_class(obj.0).as_string()
    }

    fn object_file(&mut self, obj: Handle) -> Option<String> {
        self.reads.object_file(obj.0).as_string().filter(|file| !file.is_empty())
    }

    fn output(&mut self, text: &str, newline: bool) {
        self.reads.output(text, newline);
    }

    fn now(&mut self) -> (i32, f64) {
        let v = self.reads.now();
        let days = js_sys::Reflect::get(&v, &"days".into()).ok().and_then(|d| d.as_f64()).unwrap_or(0.0);
        let secs = js_sys::Reflect::get(&v, &"secs".into()).ok().and_then(|d| d.as_f64()).unwrap_or(0.0);
        (days as i32, secs)
    }

    fn random(&mut self) -> f64 {
        match &mut self.seeded {
            Some(rng) => rng.next_number(),
            None => self.reads.random(),
        }
    }

    fn seed_random(&mut self, seed: f64) {
        self.seeded = Some(crate::host::SeededRandom::from_seed(seed));
    }

    fn mouse(&mut self) -> (f64, f64, bool) {
        // a host with no character screen to point at need not answer at all
        let at = self.reads.mouse();
        if at.is_undefined() || at.is_null() {
            return (0.0, 0.0, false);
        }
        let field = |name: &str| js_sys::Reflect::get(&at, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED);
        (
            field("row").as_f64().unwrap_or(0.0),
            field("col").as_f64().unwrap_or(0.0),
            field("down").as_bool().unwrap_or(false),
        )
    }
    fn os_info(&mut self) -> String {
        self.reads.os_info().as_string().unwrap_or_else(|| "Windows|10|0|0".to_string())
    }

    fn class_method(&mut self, obj: Handle, name: &str) -> bool {
        self.reads.class_method(obj.0, name)
    }

    fn resolve_program(&mut self, name: &str) -> Option<u32> {
        let id = self.reads.resolve_program(name);
        if id >= 0.0 { Some(id as u32) } else { None }
    }

    /// The three below are optional the way `errorRaised` is: a host with nowhere to put a
    /// Visual FoxPro library simply does not define them, and `SET LIBRARY TO` then says the
    /// library is not found, which is what a program can act on.
    ///
    /// `loadLibrary` answers `{ id, path, functions }` or `{ error }`; `path` is the file the
    /// host actually opened, which is what `SET("LIBRARY")` reports.
    fn load_library(&mut self, path: &str) -> Result<(u32, String, Vec<String>), String> {
        let f = optional_method(&self.reads, "loadLibrary").ok_or("no library host")?;
        let answer = f
            .call1(self.reads.as_ref(), &JsValue::from_str(path))
            .map_err(|e| format!("{:?}", e.as_string().unwrap_or_default()))?;
        let field = |name: &str| js_sys::Reflect::get(&answer, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED);
        if let Some(error) = field("error").as_string() {
            return Err(error);
        }
        let id = field("id").as_f64().unwrap_or(-1.0);
        if id < 0.0 {
            return Err("the library host answered with nothing".to_string());
        }
        let names: js_sys::Array = field("functions").dyn_into().map_err(|_| "no function list".to_string())?;
        Ok((
            id as u32,
            field("path").as_string().unwrap_or_else(|| path.to_string()),
            names.iter().map(|n| n.as_string().unwrap_or_default()).collect(),
        ))
    }

    /// `callLibrary` answers `{ ok: true, value }` with what the function returned, or
    /// `{ ok: false, code, message }` when the library raised an error of its own.
    fn call_library(&mut self, library: u32, function: u32, args: &[Value]) -> Result<Value, RtError> {
        let missing = || RtError::new(RtError::API_LIBRARY_NOT_FOUND, "API library is not found.");
        let f = optional_method(&self.reads, "callLibrary").ok_or_else(missing)?;
        let list: Vec<JsonValue> = args.iter().map(JsonValue::from_value).collect();
        let answer = f
            .apply(
                self.reads.as_ref(),
                &js_sys::Array::of3(&JsValue::from_f64(f64::from(library)), &JsValue::from_f64(f64::from(function)), &to_js(&list)),
            )
            .map_err(|e| RtError::new(0, e.as_string().unwrap_or_else(|| "the library host went away".into())))?;
        let field = |name: &str| js_sys::Reflect::get(&answer, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED);
        if field("ok").as_bool() != Some(true) {
            let code = field("code").as_f64().unwrap_or(0.0) as u32;
            // a library that called _Error(n) named a number and no words; the words are the
            // ones the product uses for that number
            match field("message").as_string().unwrap_or_default() {
                m if m.is_empty() => return Err(RtError::about(code, "")),
                m => return Err(RtError::new(code, m)),
            }
        }
        Ok(from_js(field("value")))
    }

    fn unload_library(&mut self, library: u32) {
        if let Some(f) = optional_method(&self.reads, "unloadLibrary") {
            let _ = f.call1(self.reads.as_ref(), &JsValue::from_f64(f64::from(library)));
        }
    }

    /// `members` is optional the way `errorRaised` is: a host that cannot enumerate an object
    /// simply does not define it, and AMEMBERS() then refuses rather than half-answering.
    ///
    /// Each entry crosses as `{ name, kind, native, added, readOnly, changed, value }`, with
    /// `kind` one of the words AMEMBERS() writes.
    fn members(&mut self, obj: Handle) -> Option<Vec<crate::host::MemberInfo>> {
        use crate::host::{MemberInfo, MemberKind};
        let f = optional_method(&self.reads, "members")?;
        let list = f.call1(self.reads.as_ref(), &JsValue::from_f64(f64::from(obj.0))).ok()?;
        let list: js_sys::Array = list.dyn_into().ok()?;
        let field = |m: &JsValue, name: &str| js_sys::Reflect::get(m, &JsValue::from_str(name)).ok();
        let flag = |m: &JsValue, name: &str| field(m, name).and_then(|v| v.as_bool()).unwrap_or(false);
        let mut out = Vec::with_capacity(list.length() as usize);
        for m in list.iter() {
            let name = field(&m, "name").and_then(|v| v.as_string()).unwrap_or_default();
            let kind = match field(&m, "kind").and_then(|v| v.as_string()).as_deref() {
                Some("Event") => MemberKind::Event,
                Some("Method") => MemberKind::Method,
                Some("Object") => MemberKind::Object,
                _ => MemberKind::Property,
            };
            let value = field(&m, "value").filter(|v| !v.is_undefined()).map(from_js);
            out.push(MemberInfo {
                name: name.to_ascii_uppercase(),
                kind,
                native: flag(&m, "native"),
                added: flag(&m, "added"),
                read_only: flag(&m, "readOnly"),
                changed: flag(&m, "changed"),
                value,
            });
        }
        Some(out)
    }

    fn error_raised(&mut self, err: &RtError, handled: bool) {
        let Some(f) = optional_method(&self.reads, "errorRaised") else { return };
        let Ok(detail) = serde_wasm_bindgen::to_value(err) else { return };
        // a logger that throws must not unwind the VM; there is nothing to report about a report
        let _ = f.call2(self.reads.as_ref(), &detail, &JsValue::from_bool(handled));
    }
}

// ---- compile results ------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompileOut {
    bytes: Option<serde_bytes_vec::Bytes>,
    diagnostics: Vec<Diagnostic>,
    method_diagnostics: Vec<MethodDiag>,
}

#[derive(Serialize)]
struct MethodDiag {
    method: String,
    diagnostic: Diagnostic,
}

/// Serializes a byte vector as a `Uint8Array` instead of a JS array of numbers.
mod serde_bytes_vec {
    use serde::{Serialize, Serializer};

    pub struct Bytes(pub Vec<u8>);

    impl Serialize for Bytes {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_bytes(&self.0)
        }
    }
}

fn compile_out(r: CompileResult) -> JsValue {
    let out = CompileOut {
        bytes: r.module.as_ref().map(|m| serde_bytes_vec::Bytes(encode(m))),
        diagnostics: r.diagnostics,
        method_diagnostics: r
            .method_diagnostics
            .into_iter()
            .map(|(method, diagnostic)| MethodDiag { method, diagnostic })
            .collect(),
    };
    to_js(&out)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MethodIn {
    object_path: String,
    event: String,
    #[serde(default)]
    params: String,
    #[serde(default)]
    source: String,
    /// The header file the method's own file declared, by name without folder or extension.
    #[serde(default)]
    include: String,
}

// ---- step results ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
enum StepOut {
    Done { value: JsonValue, nodefault: bool },
    Error { error: RtError, stack: Vec<StackFrame> },
    Suspend { request: crate::host::HostRequest },
}

#[derive(Serialize)]
struct StackFrame {
    program: String,
    line: u32,
}

/// What a request matched: the lambda to run, and the named parts of the path.
#[derive(Serialize)]
struct RouteOut {
    function: u32,
    params: std::collections::BTreeMap<String, String>,
}

/// One frame of a stopped program, with the module its source came from.
#[derive(Serialize)]
struct FrameOut {
    program: String,
    module: String,
    line: u32,
}

/// One variable a frame can see of its own.
#[derive(Serialize)]
struct VariableOut {
    name: String,
    private: bool,
    value: JsonValue,
}

// ---- the VM ---------------------------------------------------------------------------------

#[wasm_bindgen]
pub struct FoxVm {
    vm: Vm,
    host: JsHost,
}

#[wasm_bindgen]
impl FoxVm {
    #[wasm_bindgen(constructor)]
    pub fn new(reads: JsValue) -> FoxVm {
        FoxVm { vm: Vm::new(), host: JsHost { reads: reads.unchecked_into(), seeded: None } }
    }

    /// Loads an encoded module; returns its id.
    pub fn load_module(&mut self, bytes: &[u8]) -> Result<u32, JsValue> {
        let m: Module = decode(bytes).map_err(|e| JsValue::from_str(&e))?;
        Ok(self.vm.load_module(m))
    }

    /// `{ bytes: Uint8Array | null, diagnostics, methodDiagnostics }`.
    /// `headers` is an object of `{ NAME: text }` for the files the program may `#INCLUDE`,
    /// keyed by name without folder or extension, upper-cased. Reading them is the caller's.
    pub fn compile_program(src: &str, name: &str, headers: JsValue) -> JsValue {
        let headers: std::collections::HashMap<String, String> =
            serde_wasm_bindgen::from_value(headers).unwrap_or_default();
        compile_out(compiler::compile_program_with(src, name, &headers))
    }

    /// `methods` is an array of `{ objectPath, event, params, source, include }`; `headers` is an
    /// object of `{ NAME: text }` for the files a method's `include` or an `#INCLUDE` names, keyed
    /// by name without folder or extension, upper-cased.
    pub fn compile_form(name: &str, methods: JsValue, headers: JsValue) -> JsValue {
        let methods: Vec<MethodIn> = match serde_wasm_bindgen::from_value(methods) {
            Ok(m) => m,
            Err(e) => return JsValue::from_str(&e.to_string()),
        };
        let headers: std::collections::HashMap<String, String> =
            serde_wasm_bindgen::from_value(headers).unwrap_or_default();
        let methods: Vec<MethodSource> = methods
            .into_iter()
            .map(|m| MethodSource {
                object_path: m.object_path,
                event: m.event,
                params: m.params,
                source: m.source,
                include: m.include,
            })
            .collect();
        compile_out(compiler::compile_form_with(name, &methods, &headers))
    }

    pub fn compile_snippet(src: &str, name: &str) -> JsValue {
        compile_out(compiler::compile_snippet(src, name))
    }

    pub fn compile_expression(src: &str) -> JsValue {
        compile_out(compiler::compile_expression(src, &[]))
    }

    /// Starts `func_name` of `module` (`"MAIN"` for a program body); `args` is an array of values.
    pub fn start(&mut self, module: u32, func_name: &str, this: Option<u32>, args: JsValue) -> Result<u32, JsValue> {
        let func = self
            .vm
            .module(module)
            .find_func(&func_name.to_ascii_uppercase())
            .ok_or_else(|| JsValue::from_str(&format!("Procedure '{func_name}' is not found")))?;
        Ok(self.vm.start(module, func, this.map(Handle), args_from_js(args)))
    }

    /// The handler a request matches: `{ function, params }`, or `undefined` when no route
    /// answers it.
    ///
    /// Routing lives here rather than in the host because that is where the routes were
    /// registered, and because a pattern language wants one implementation and not two. It is a
    /// read and not a side effect, so it is a plain export like a property read: the host asks
    /// it the moment a request arrives, with the VM off the stack.
    pub fn route(&self, server: u32, method: &str, path: &str) -> JsValue {
        match self.vm.route(Handle(server), method, path) {
            Some((function, params)) => {
                let params: std::collections::BTreeMap<String, String> = params.into_iter().collect();
                to_js(&RouteOut { function, params })
            }
            None => JsValue::UNDEFINED,
        }
    }

    /// Starts a fiber whose first frame is a lambda: the host waking the runtime with a payload.
    /// `function` is the id a function value crossed as. `undefined` when the VM has no such
    /// function, which is what a stale handler from an earlier run looks like.
    pub fn start_function(&mut self, function: u32, args: JsValue) -> Option<u32> {
        self.vm.start_function(crate::value::FuncId(function), args_from_js(args))
    }

    pub fn start_method(&mut self, module: u32, obj_path: &str, event: &str, this: u32, args: JsValue) -> Option<u32> {
        self.vm.start_method(module, obj_path, event, Handle(this), args_from_js(args))
    }

    /// Starts a method of a `DEFINE CLASS` instance: `obj_path` is `""` for the class's own
    /// method and the member's name for a member's. `undefined` when the class has no such method.
    pub fn start_class_method(
        &mut self,
        module: u32,
        class: &str,
        obj_path: &str,
        event: &str,
        this: u32,
        args: JsValue,
    ) -> Option<u32> {
        self.vm.start_class_method(module, class, obj_path, event, this, args_from_js(args))
    }

    /// Every `DEFINE CLASS` of a module in the shape `CreateObject`'s `definition` uses, so the
    /// host can resolve a parent class it has not instantiated yet.
    pub fn class_definitions(&self, module: u32) -> JsValue {
        let defs: Vec<ClassDefOut> = self.vm.classes(module).iter().map(|c| ClassDefOut::new(module, c)).collect();
        to_js(&defs)
    }

    /// `{state:'done', value, nodefault} | {state:'error', error, stack} | {state:'suspend', request}`.
    pub fn step(&mut self, fiber: u32) -> JsValue {
        let stack: Vec<StackFrame> =
            self.vm.call_stack(fiber).into_iter().map(|(program, line)| StackFrame { program, line }).collect();
        let out = match self.vm.step(&mut self.host, fiber) {
            Step::Done { value, nodefault } => StepOut::Done { value: JsonValue::from_value(&value), nodefault },
            Step::Error(error) => StepOut::Error { error, stack },
            Step::Suspend(request) => StepOut::Suspend { request },
        };
        to_js(&out)
    }

    pub fn resume(&mut self, fiber: u32, value: JsValue) {
        self.vm.resume(fiber, from_js(value));
    }

    pub fn resume_error(&mut self, fiber: u32, code: u32, message: &str) {
        self.vm.resume_error(fiber, RtError::new(code, message));
    }

    pub fn abort(&mut self, fiber: u32) {
        self.vm.abort(fiber);
    }

    pub fn abort_all(&mut self) {
        self.vm.abort_all();
    }

    /// Evaluates an expression in the current frame of a suspended fiber.
    pub fn evaluate(&mut self, fiber: u32, expr: &str) -> JsValue {
        match self.vm.evaluate(&mut self.host, fiber, expr) {
            Ok(v) => to_js(&JsonValue::from_value(&v)),
            Err(e) => to_js(&e),
        }
    }

    pub fn call_stack(&self, fiber: u32) -> JsValue {
        let stack: Vec<StackFrame> =
            self.vm.call_stack(fiber).into_iter().map(|(program, line)| StackFrame { program, line }).collect();
        to_js(&stack)
    }

    // ---- the debugger ----

    /// Sets or clears a breakpoint. `program` is the name the debugger's call stack shows for
    /// frames of that source: the program name for a .prg, `objPath.Event` for a form method.
    pub fn set_breakpoint(&mut self, program: &str, line: u32, on: bool) {
        self.vm.set_breakpoint(program, line, on);
    }

    pub fn clear_breakpoints(&mut self) {
        self.vm.clear_breakpoints();
    }

    /// Every breakpoint that is set, as `{ program, line }`.
    pub fn breakpoints(&self) -> JsValue {
        let all: Vec<StackFrame> =
            self.vm.breakpoints().iter().map(|(program, line)| StackFrame { program: program.clone(), line: *line }).collect();
        to_js(&all)
    }

    /// How far a stopped fiber runs when it is let go: `"go"`, `"into"`, `"over"` or `"out"`.
    /// Called before `resume`, while the VM is off the stack.
    pub fn set_step_mode(&mut self, fiber: u32, mode: &str) {
        let mode = match mode.to_ascii_lowercase().as_str() {
            "into" => StepMode::Into,
            "over" => StepMode::Over,
            "out" => StepMode::Out,
            _ => StepMode::Go,
        };
        self.vm.set_step_mode(fiber, mode);
    }

    /// The frames of a stopped fiber, outermost first, as `{ program, module, line }`.
    pub fn frames(&self, fiber: u32) -> JsValue {
        let frames: Vec<FrameOut> = self
            .vm
            .frames(fiber)
            .into_iter()
            .map(|f| FrameOut { program: f.program, module: f.module, line: f.line })
            .collect();
        to_js(&frames)
    }

    /// The variables one frame can see of its own, as `{ name, private, value }`.
    pub fn frame_variables(&self, fiber: u32, level: u32) -> JsValue {
        let vars: Vec<VariableOut> = self
            .vm
            .frame_variables(fiber, level as usize)
            .into_iter()
            .map(|v| VariableOut { name: v.name, private: v.private, value: JsonValue::from_value(&v.value) })
            .collect();
        to_js(&vars)
    }

    /// Evaluates an expression in a chosen frame of a stopped fiber - a watch expression.
    /// `{ value }` when it worked, the error when it did not.
    pub fn evaluate_in(&mut self, fiber: u32, level: u32, expr: &str) -> JsValue {
        match self.vm.evaluate_in(&mut self.host, fiber, level as usize, expr) {
            Ok(v) => to_js(&JsonValue::from_value(&v)),
            Err(e) => to_js(&e),
        }
    }

    /// What the host says was chosen from the menu that is up, so BAR(), PAD(), POPUP() and
    /// PROMPT() answer it while the command that choice stands for runs.
    pub fn menu_chosen(&mut self, pad: &str, bar: f64, popup: &str, prompt: &str) {
        self.vm.menu_chosen(pad, bar as i32, popup, prompt);
    }

    /// The text of a document from its bytes, decoded as its own declaration says it is
    /// written. The host has the bytes; only the reader knows what they mean.
    pub fn xml_text(&self, bytes: &str) -> String {
        crate::xml::decode(&bytes.chars().map(|c| c as u32 as u8).collect::<Vec<u8>>())
    }

    /// What a document holds, for the XMLAdapter: one table for each kind of row, and the
    /// fields those rows carry between them. The reading is the same one XMLTOCURSOR() does.
    pub fn xml_shape(&self, text: &str) -> JsValue {
        to_js(&crate::xml::shape_of(text))
    }

    /// Puts a value in a public variable, so the host can hand the VM something too big to
    /// write into a line of source - the text of an XML document, say.
    pub fn set_global(&mut self, name: &str, value: JsValue) {
        self.vm.set_global(name, from_js(value));
    }

    /// And reads one back.
    pub fn get_global(&mut self, name: &str) -> JsValue {
        to_js(&JsonValue::from_value(&self.vm.get_global(name).unwrap_or(crate::value::Value::Null)))
    }

    /// `exact`, `decimals`, `century`, `date`, `talk`, `safety`, `escape`.
    pub fn set_setting(&mut self, name: &str, value: JsValue) {
        let s = self.vm.settings_mut();
        let v = from_js(value);
        match name.to_ascii_uppercase().as_str() {
            "EXACT" => s.exact = v.truthy().unwrap_or(false),
            "CENTURY" => s.century = v.truthy().unwrap_or(false),
            "TALK" => s.talk = v.truthy().unwrap_or(true),
            "SAFETY" => s.safety = v.truthy().unwrap_or(true),
            "ESCAPE" => s.escape = v.truthy().unwrap_or(true),
            "RUNTIME" => s.runtime_only = v.truthy().unwrap_or(false),
            "DECIMALS" => s.decimals = v.as_number().unwrap_or(2.0).clamp(0.0, 18.0) as u8,
            "DATE" => {
                if let Some(f) = v.as_str().ok().and_then(|w| crate::value::DateFormat::parse(&w)) {
                    s.date_format = f;
                }
            }
            _ => {}
        }
    }

    pub fn get_setting(&self, name: &str) -> JsValue {
        let s = self.vm.settings();
        match name.to_ascii_uppercase().as_str() {
            "EXACT" => JsValue::from_bool(s.exact),
            "CENTURY" => JsValue::from_bool(s.century),
            "TALK" => JsValue::from_bool(s.talk),
            "SAFETY" => JsValue::from_bool(s.safety),
            "ESCAPE" => JsValue::from_bool(s.escape),
            "DECIMALS" => JsValue::from_f64(s.decimals as f64),
            "DATE" => JsValue::from_str(&format!("{:?}", s.date_format).to_ascii_uppercase()),
            _ => JsValue::UNDEFINED,
        }
    }
}
