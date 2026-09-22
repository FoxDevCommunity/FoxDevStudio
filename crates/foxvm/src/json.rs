//! The JSON value, and the two ways a cursor crosses to it and back.
//!
//! `serde_json` is the whole of the parsing and the writing; nothing here reads or writes JSON
//! text by hand. What is here is the mapping: which FoxPro value a JSON one stands for, which
//! JSON value a field of a table becomes, and what a table made out of JSON is shaped like.
//! Nothing about it can be measured - Visual FoxPro has no JSON - so every rule is written down
//! in docs/foxscript.md and tested from there.

use std::rc::Rc;

use serde_json::{Map, Value as J};

use crate::dbf::{DbfField, DbfRecord, DbfValue};
use crate::error::RtError;
use crate::foxscript::FOXSCRIPT_ERROR;
use crate::value::{self, Value};

/// The widest a character field a cursor is built from may be before it becomes a memo.
const WIDEST_CHARACTER: usize = 254;

/// How wide a numeric field a cursor is built from is.
const NUMBER_WIDTH: u8 = 20;

/// The most decimal places one of those may carry.
const MOST_DECIMALS: u8 = 15;

pub fn json(v: J) -> Value {
    Value::Json(Rc::new(v))
}

/// `FoxScript.Json.Parse()`: the text as a value, or the parser's own complaint.
pub fn parse(text: &str) -> Result<Value, RtError> {
    match serde_json::from_str::<J>(text) {
        Ok(v) => Ok(json(v)),
        Err(e) => Err(RtError::new(FOXSCRIPT_ERROR, format!("That is not JSON: {e}"))),
    }
}

/// `FoxScript.Json.Stringify()`: any FoxPro value as JSON text.
pub fn stringify(v: &Value, pretty: bool, settings: &value::Settings) -> Result<String, RtError> {
    let doc = to_json(v, settings)?;
    let text = if pretty { serde_json::to_string_pretty(&doc) } else { serde_json::to_string(&doc) };
    text.map_err(|e| RtError::new(FOXSCRIPT_ERROR, format!("That cannot be written as JSON: {e}")))
}

/// What a FoxPro value is as JSON. A date and a datetime go out in ISO 8601, which is what every
/// client already reads.
pub fn to_json(v: &Value, settings: &value::Settings) -> Result<J, RtError> {
    Ok(match v.deref() {
        Value::Json(j) => j.as_ref().clone(),
        Value::Null => J::Null,
        Value::Logical(b) => J::Bool(b),
        Value::Number(n, ..) => number(n),
        Value::Currency(c) => number(c as f64 / value::CURRENCY_SCALE as f64),
        Value::Str(s) => J::String(s.to_string()),
        Value::Date(None) | Value::DateTime(None) => J::Null,
        Value::Date(Some(d)) => {
            let (y, m, day) = value::civil_from_days(d);
            J::String(format!("{y:04}-{m:02}-{day:02}"))
        }
        Value::DateTime(Some(t)) => J::String(iso_datetime(t)),
        Value::Array(a) => J::Array(a.borrow().items.iter().map(|i| to_json(i, settings)).collect::<Result<_, _>>()?),
        // an object is the host's, and the host is not here; a program that wants one written
        // out builds the JSON for it
        Value::Object(_) | Value::Function(_) => {
            return Err(RtError::new(FOXSCRIPT_ERROR, "An object cannot be written as JSON."));
        }
        Value::Ref(_) => unreachable!("deref"),
    })
}

/// What a JSON value is as a FoxPro one. An object or an array stays JSON, so that reaching goes
/// on; everything else is the FoxPro value JSON's own type stands for.
pub fn from_json(v: &J) -> Value {
    match v {
        J::Null => Value::Null,
        J::Bool(b) => Value::Logical(*b),
        J::Number(n) => Value::number(n.as_f64().unwrap_or(0.0)),
        J::String(s) => Value::str(s.as_str()),
        J::Object(_) | J::Array(_) => json(v.clone()),
    }
}

/// A number as JSON. A whole number is written whole - `7` and not `7.0` - because a FoxPro
/// numeric field carries its decimals in the header rather than in the value, and a client
/// reading an id wants an id.
fn number(n: f64) -> J {
    if n.is_finite() && n == n.trunc() && n.abs() <= i64::MAX as f64 {
        return J::Number((n as i64).into());
    }
    serde_json::Number::from_f64(n).map_or(J::Null, J::Number)
}

/// Seconds since the epoch as `YYYY-MM-DDTHH:MM:SS`.
fn iso_datetime(t: f64) -> String {
    let days = (t / 86_400.0).floor();
    let rest = (t - days * 86_400.0).round().max(0.0) as i64;
    let (y, m, d) = value::civil_from_days(days as i32);
    let (h, min, s) = (rest / 3600, (rest % 3600) / 60, rest % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}")
}

/// A member of a JSON object, by name, without regard to case.
///
/// Every other member read in this language is case-insensitive and a programmer writing
/// `oData.Name` for a key spelled `name` is doing the ordinary FoxPro thing. Where an object has
/// two keys differing only in case the first in document order wins, which `serde_json`'s
/// preserved order makes well defined.
pub fn member<'a>(v: &'a J, name: &str) -> Option<&'a J> {
    let map = v.as_object()?;
    map.get(name).or_else(|| map.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v))
}

/// One element of a JSON array, by its one-based subscript.
pub fn element(v: &J, index: usize) -> Option<&J> {
    v.as_array()?.get(index.checked_sub(1)?)
}

/// How many members an object has or elements an array has; 0 for anything else.
pub fn count(v: &J) -> usize {
    match v {
        J::Object(m) => m.len(),
        J::Array(a) => a.len(),
        _ => 0,
    }
}

/// The names of an object's members, in document order.
pub fn keys(v: &J) -> J {
    match v {
        J::Object(m) => J::Array(m.keys().map(|k| J::String(k.clone())).collect()),
        _ => J::Array(Vec::new()),
    }
}

// ----- a cursor, both ways -------------------------------------------------------------------

/// One record as a JSON object: the field names lower-cased, and each value as its type gives it.
pub fn record_to_json(fields: &[DbfField], record: &DbfRecord) -> J {
    let mut out = Map::new();
    for (i, field) in fields.iter().enumerate() {
        let value = record.values.get(i).unwrap_or(&DbfValue::Null);
        out.insert(field.name.to_lowercase(), field_to_json(field, value));
    }
    J::Object(out)
}

fn field_to_json(field: &DbfField, value: &DbfValue) -> J {
    match value {
        DbfValue::Null => J::Null,
        // a DBF pads a character field to its width and JSON should not carry the padding
        DbfValue::Text(s) | DbfValue::Memo(s) => J::String(s.trim_end().to_string()),
        DbfValue::Number(n) => number(*n),
        DbfValue::Currency(c) => number(*c as f64 / value::CURRENCY_SCALE as f64),
        DbfValue::Logical(b) => J::Bool(*b),
        DbfValue::Date(None) | DbfValue::DateTime(None) => J::Null,
        DbfValue::Date(Some(d)) => {
            let (y, m, day) = value::civil_from_days(*d);
            J::String(format!("{y:04}-{m:02}-{day:02}"))
        }
        DbfValue::DateTime(Some(t)) => J::String(iso_datetime(*t)),
        // a general or blob field has no text of its own
        DbfValue::Bytes(_) => {
            let _ = field;
            J::Null
        }
    }
}

/// The fields and records a cursor built from JSON has.
///
/// The value must be an array of objects, or one object, which becomes one record. The fields
/// are the union of the keys in the order they are first seen, and each one's type comes from
/// the first value for it that is not null. See docs/foxscript.md.
pub fn json_to_records(v: &J) -> Result<(Vec<DbfField>, Vec<DbfRecord>), RtError> {
    let rows: Vec<&J> = match v {
        J::Array(items) => items.iter().collect(),
        J::Object(_) => vec![v],
        _ => return Err(RtError::new(FOXSCRIPT_ERROR, "A cursor is made from an array of objects.")),
    };
    for row in &rows {
        if !row.is_object() {
            return Err(RtError::new(FOXSCRIPT_ERROR, "A cursor is made from an array of objects."));
        }
    }

    // the shape first: every key in the order it is first seen, and what the values under it ask for
    let mut names: Vec<String> = Vec::new();
    let mut shapes: Vec<Shape> = Vec::new();
    for row in &rows {
        for (key, value) in row.as_object().expect("checked above") {
            let at = match names.iter().position(|n| n.eq_ignore_ascii_case(key)) {
                Some(i) => i,
                None => {
                    names.push(key.clone());
                    shapes.push(Shape::default());
                    names.len() - 1
                }
            };
            shapes[at].see(value);
        }
    }
    let fields: Vec<DbfField> = names.iter().zip(&shapes).map(|(n, s)| s.field(n)).collect();

    let records = rows
        .iter()
        .map(|row| {
            let object = row.as_object().expect("checked above");
            let values = names
                .iter()
                .zip(&fields)
                .map(|(name, field)| {
                    let found = object
                        .get(name)
                        .or_else(|| object.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v));
                    to_field(field, found.unwrap_or(&J::Null))
                })
                .collect();
            DbfRecord { deleted: false, values }
        })
        .collect();
    Ok((fields, records))
}

/// What the values seen under one key ask the field to be.
#[derive(Default)]
struct Shape {
    text: bool,
    number: bool,
    logical: bool,
    nested: bool,
    widest: usize,
    decimals: u8,
}

impl Shape {
    fn see(&mut self, v: &J) {
        match v {
            J::String(s) => {
                self.text = true;
                self.widest = self.widest.max(s.chars().count());
            }
            J::Number(n) => {
                self.number = true;
                self.decimals = self.decimals.max(places(n.as_f64().unwrap_or(0.0)));
            }
            J::Bool(_) => self.logical = true,
            J::Object(_) | J::Array(_) => self.nested = true,
            J::Null => {}
        }
    }

    /// The field that shape becomes. A key whose every value was null becomes logical, which is
    /// what vfp9.exe says a bare `.NULL.` is - measured.
    fn field(&self, name: &str) -> DbfField {
        let (kind, length, decimals) = if self.nested || (self.text && self.widest > WIDEST_CHARACTER) {
            ('M', 4, 0)
        } else if self.text {
            ('C', self.widest.max(1).min(WIDEST_CHARACTER) as u8, 0)
        } else if self.number {
            ('N', NUMBER_WIDTH, self.decimals.min(MOST_DECIMALS))
        } else {
            ('L', 1, 0)
        };
        DbfField::new(name.to_uppercase(), kind, length, decimals)
    }
}

/// How many places past the point a number needs, up to the most a field may carry.
fn places(n: f64) -> u8 {
    if !n.is_finite() || n == n.trunc() {
        return 0;
    }
    for places in 1..=MOST_DECIMALS {
        let scale = 10f64.powi(i32::from(places));
        if (n * scale).round() / scale == n {
            return places;
        }
    }
    MOST_DECIMALS
}

/// One JSON value as the field it is going into. A null becomes the empty value of the field's
/// type, because the fields a cursor built from JSON has are not nullable.
fn to_field(field: &DbfField, v: &J) -> DbfValue {
    match field.kind {
        'M' => DbfValue::Memo(match v {
            J::Null => String::new(),
            J::String(s) => s.clone(),
            other => other.to_string(),
        }),
        'C' => DbfValue::Text(match v {
            J::Null => String::new(),
            J::String(s) => s.clone(),
            other => other.to_string(),
        }),
        'N' => DbfValue::Number(v.as_f64().unwrap_or(0.0)),
        _ => DbfValue::Logical(v.as_bool().unwrap_or(false)),
    }
}
