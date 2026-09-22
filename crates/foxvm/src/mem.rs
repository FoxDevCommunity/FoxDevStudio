//! `.mem` files: the memory variables a program puts away with `SAVE TO` and takes back with
//! `RESTORE FROM`.
//!
//! The format is a run of 32-byte headers, each followed by its value, and a `0x1A` at the end.
//! It is not written down anywhere, so this is read off the files Visual FoxPro 9 itself writes:
//!
//! | offset | what |
//! |---|---|
//! | 0-10 | the name, padded with nulls |
//! | 11 | the type: `C` or `H` text, `N` number, `L` logical, `D` date, `T` datetime, `Y` money, `A` array, `0` null |
//! | 16-17 | how many bytes the text takes, or the width and the decimals of a number |
//! | 25 | 3 for a variable, 0 for one element of an array |
//!
//! Text is written with a null after it and counted with that null; `H` is the same as `C` and
//! says only that the count did not fit in a byte. A number, a date and a datetime are all an
//! eight-byte double - a date is the Julian day it falls on, and a datetime is that day and the
//! fraction of it that has gone by. Money is an eight-byte whole number of ten-thousandths. An
//! array's own record holds its two dimensions, and one record per element follows it, row by
//! row, each under the array's name.

use std::cell::RefCell;
use std::rc::Rc;

use encoding_rs::WINDOWS_1252;

use crate::value::{FoxArray, Value};

/// The header before every value.
const HEADER: usize = 32;
/// What the last byte of the file is.
const END: u8 = 0x1A;
/// Days from the start of the Julian period to 1970-01-01, which is where `Value::Date` counts
/// from.
const JULIAN_EPOCH: f64 = 2_440_588.0;
/// Where the type letter sits in the header.
const TYPE_AT: usize = 11;
/// Where the count of bytes, or the width and the decimals, sit.
const WIDTH_AT: usize = 16;
/// Where the byte that tells a variable from an array's element sits.
const KIND_AT: usize = 25;

/// Writes the variables as a `.mem` file. A variable holding an object is left out, as Visual
/// FoxPro leaves it out: there is nothing in the file that could bring one back.
pub fn write(vars: &[(String, Value)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, value) in vars {
        match value.deref() {
            Value::Object(_) | Value::Ref(_) => {}
            Value::Array(array) => {
                let array = array.borrow();
                let mut header = header_of(name, 'A', 0, true);
                header.extend_from_slice(&(array.rows.min(0xFFFF) as u16).to_le_bytes());
                header.extend_from_slice(&(array.cols.min(0xFFFF) as u16).to_le_bytes());
                out.extend_from_slice(&header);
                for item in &array.items {
                    out.extend_from_slice(&one(name, &item.deref(), false));
                }
            }
            other => out.extend_from_slice(&one(name, &other, true)),
        }
    }
    out.push(END);
    out
}

/// Reads a `.mem` file back. Anything the file says that this cannot make sense of ends the
/// read, so a truncated file gives back what was whole.
pub fn read(bytes: &[u8]) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = Vec::new();
    let mut at = 0usize;
    let mut filled = 0usize;
    while at + HEADER <= bytes.len() && bytes[at] != END {
        let header = &bytes[at..at + HEADER];
        let name = name_of(header);
        let kind = header[TYPE_AT] as char;
        let width = u16::from_le_bytes([header[WIDTH_AT], header[WIDTH_AT + 1]]) as usize;
        let data = at + HEADER;
        let (value, len) = match kind {
            'C' | 'H' => {
                let end = (data + width).min(bytes.len());
                let text = &bytes[data..end];
                // the count takes in the null that ends the text
                let text = text.strip_suffix(&[0]).unwrap_or(text);
                (Value::str(WINDOWS_1252.decode(text).0.into_owned()), width)
            }
            'N' => (Value::number(double(bytes, data)), 8),
            'Y' => (Value::number(money(bytes, data)), 8),
            'L' => (Value::Logical(bytes.get(data).is_some_and(|b| *b != 0)), 1),
            'D' => (date_of(double(bytes, data)), 8),
            'T' => (datetime_of(double(bytes, data)), 8),
            '0' => (Value::Null, width.max(1)),
            'A' => {
                let rows = u16::from_le_bytes([bytes[data], bytes[data + 1]]) as usize;
                let cols = u16::from_le_bytes([bytes[data + 2], bytes[data + 3]]) as usize;
                (Value::Array(Rc::new(RefCell::new(FoxArray::new(rows.max(1), cols)))), 4)
            }
            _ => break,
        };
        // an element carries its array's name, and goes into the array that came before it,
        // one after another in the order they were written
        let element = header[KIND_AT] == 0;
        match out.last_mut().filter(|(last, _)| element && last.eq_ignore_ascii_case(&name)) {
            Some((_, Value::Array(array))) => {
                if let Some(slot) = array.borrow_mut().items.get_mut(filled) {
                    *slot = value;
                }
                filled += 1;
            }
            _ => {
                filled = 0;
                out.push((name, value));
            }
        }
        at = data + len;
    }
    out
}

/// One variable's record: the header and the value after it.
fn one(name: &str, value: &Value, own: bool) -> Vec<u8> {
    match value {
        Value::Str(text) => {
            let (bytes, ..) = WINDOWS_1252.encode(text);
            let len = bytes.len() + 1;
            // a count that does not fit in a byte is what tells `H` from `C`
            let mut out = header_of(name, if len > 255 { 'H' } else { 'C' }, len as u16, own);
            out.extend_from_slice(&bytes);
            out.push(0);
            out
        }
        Value::Number(n, ..) => {
            let decimals = decimals_of(*n);
            // Visual FoxPro writes ten columns for the whole part and the point and the
            // decimals on top of that
            let width = 10 + if decimals > 0 { u16::from(decimals) + 1 } else { 0 };
            let mut out = header_of(name, 'N', width | (u16::from(decimals) << 8), own);
            out.extend_from_slice(&n.to_le_bytes());
            out
        }
        Value::Logical(b) => {
            let mut out = header_of(name, 'L', 1, own);
            out.push(u8::from(*b));
            out
        }
        Value::Date(days) => {
            let mut out = header_of(name, 'D', 0, own);
            let day = days.map_or(0.0, |d| f64::from(d) + JULIAN_EPOCH);
            out.extend_from_slice(&day.to_le_bytes());
            out
        }
        Value::DateTime(secs) => {
            let mut out = header_of(name, 'T', 0, own);
            let day = secs.map_or(0.0, |s| s / 86_400.0 + JULIAN_EPOCH);
            out.extend_from_slice(&day.to_le_bytes());
            out
        }
        // a null says what it would have been if it had a value, which is a logical when
        // nothing has said otherwise
        _ => {
            let mut out = header_of(name, '0', 1, own);
            out.push(b'L');
            out
        }
    }
}

fn header_of(name: &str, kind: char, width: u16, own: bool) -> Vec<u8> {
    let mut out = vec![0u8; HEADER];
    let upper = name.to_ascii_uppercase();
    let (bytes, ..) = WINDOWS_1252.encode(&upper);
    for (i, b) in bytes.iter().take(TYPE_AT).enumerate() {
        out[i] = *b;
    }
    out[TYPE_AT] = kind as u8;
    out[WIDTH_AT] = width as u8;
    out[WIDTH_AT + 1] = (width >> 8) as u8;
    out[KIND_AT] = if own { 3 } else { 0 };
    out
}

fn name_of(header: &[u8]) -> String {
    let end = header[..TYPE_AT].iter().position(|b| *b == 0).unwrap_or(TYPE_AT);
    WINDOWS_1252.decode(&header[..end]).0.into_owned().to_ascii_uppercase()
}

fn double(bytes: &[u8], at: usize) -> f64 {
    match bytes.get(at..at + 8).and_then(|b| <[u8; 8]>::try_from(b).ok()) {
        Some(eight) => f64::from_le_bytes(eight),
        None => 0.0,
    }
}

fn money(bytes: &[u8], at: usize) -> f64 {
    match bytes.get(at..at + 8).and_then(|b| <[u8; 8]>::try_from(b).ok()) {
        Some(eight) => i64::from_le_bytes(eight) as f64 / 10_000.0,
        None => 0.0,
    }
}

fn date_of(julian: f64) -> Value {
    if julian == 0.0 { Value::Date(None) } else { Value::Date(Some((julian - JULIAN_EPOCH) as i32)) }
}

fn datetime_of(julian: f64) -> Value {
    if julian == 0.0 {
        Value::DateTime(None)
    } else {
        Value::DateTime(Some(((julian - JULIAN_EPOCH) * 86_400.0).round()))
    }
}

/// How many decimals a number is written with, which is what the header says about it.
fn decimals_of(n: f64) -> u8 {
    let text = format!("{n}");
    match text.split_once('.') {
        Some((_, after)) => after.len().min(18) as u8,
        None => 0,
    }
}
