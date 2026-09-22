//! Turning a value back into the bytes a field holds.
//!
//! The reader is the authority on what a field means; this is its inverse, and the two are tested
//! against each other. A field is a fixed run of bytes, so encoding never moves anything: the
//! caller drops the result into the record at the offset the layout gives, and the record keeps
//! the length the header says it has.
//!
//! Memo fields are the exception and are refused. Writing one means appending a block to the
//! `.fpt` and rewriting its header, which is a change to a second file rather than a field.

use super::{DbfField, JULIAN_EPOCH, encoding};
use crate::error::RtError;
use crate::value::{Value, civil_from_days};

/// The bytes a field holds for `value`, exactly `width` of them.
///
/// `width` comes from the table's layout rather than from the field descriptor, because FoxPro 2
/// encodes a character field wider than 255 by borrowing the decimals byte.
pub fn encode_field(field: &DbfField, width: usize, value: &Value, codepage: Option<u16>) -> Result<Vec<u8>, RtError> {
    let mut out = vec![b' '; width];
    // `.NULL.` is a value a column either accepts or refuses, and the bytes it leaves behind are
    // the blank ones: what marks the field null is the record's flag bit, which the caller sets.
    if matches!(value.deref(), Value::Null) && field.nullable {
        blank_field(field.kind, &mut out);
        return Ok(out);
    }
    if matches!(value.deref(), Value::Null) {
        return Err(null_refused(field));
    }
    match field.kind {
        'C' => {
            let text = as_text(value)?;
            let bytes = encoding::encode(&text, codepage);
            let take = bytes.len().min(width);
            out[..take].copy_from_slice(&bytes[..take]);
        }
        'N' | 'F' => {
            let text = numeric_text(as_number(value)?, width, field.decimals as usize);
            let bytes = text.as_bytes();
            if bytes.len() > width {
                // FoxPro writes a field of asterisks when a value will not fit, and so do we
                out.fill(b'*');
            } else {
                out[width - bytes.len()..].copy_from_slice(bytes);
            }
        }
        'I' | '+' => {
            let n = as_number(value)?.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
            put(&mut out, &n.to_le_bytes());
        }
        'B' | 'O' => put(&mut out, &as_number(value)?.to_le_bytes()),
        'Y' => {
            let amount = match value.deref() {
                Value::Currency(c) => c,
                other => match crate::value::to_currency(as_number(&other)?) {
                    Value::Currency(c) => c,
                    _ => 0,
                },
            };
            put(&mut out, &amount.to_le_bytes());
        }
        'L' => out[0] = if value.truthy().map_err(|_| type_mismatch(field))? { b'T' } else { b'F' },
        'D' => match value.deref() {
            Value::Date(Some(days)) => {
                let (y, m, d) = civil_from_days(days);
                put(&mut out, format!("{y:04}{m:02}{d:02}").as_bytes());
            }
            Value::Date(None) | Value::Null => out.fill(b' '),
            _ => return Err(type_mismatch(field)),
        },
        'T' | '@' => match value.deref() {
            Value::DateTime(Some(secs)) => {
                let julian = (secs / 86_400.0).floor() as i32 + JULIAN_EPOCH;
                let millis = ((secs - (secs / 86_400.0).floor() * 86_400.0) * 1000.0).round() as i32;
                put(&mut out, &julian.to_le_bytes());
                if out.len() >= 8 {
                    out[4..8].copy_from_slice(&millis.to_le_bytes());
                }
            }
            Value::DateTime(None) | Value::Null => out.fill(0),
            _ => return Err(type_mismatch(field)),
        },
        // a memo field holds the block its text was written at, not the text: four bytes of
        // binary in a Visual FoxPro table, ten characters of digits in an older one
        'M' | 'G' | 'P' => {
            let block = match value.deref() {
                Value::Number(n, ..) => n.max(0.0) as u32,
                Value::Null => 0,
                // an empty memo points at nothing, which is what a block of 0 means. Text
                // that is not empty has to have been written to the memo file first.
                Value::Str(ref s) if s.trim().is_empty() => 0,
                _ => {
                    return Err(RtError::new(
                        RtError::FEATURE_NOT_AVAILABLE,
                        format!("field {} is a memo: it holds the block its text was written at", field.name),
                    ));
                }
            };
            if width == 4 {
                put(&mut out, &block.to_le_bytes());
            } else if block == 0 {
                out.fill(b' ');
            } else {
                let text = block.to_string();
                let start = width.saturating_sub(text.len());
                out[start..].copy_from_slice(text.as_bytes());
            }
        }
        _ => return Err(type_mismatch(field)),
    }
    Ok(out)
}

/// What a field of that type holds when it holds nothing.
fn blank_field(kind: char, out: &mut [u8]) {
    match kind {
        // the fixed-width binary types read as a value rather than as blanks, so they start at 0
        'I' | '+' | 'B' | 'O' | 'Y' | 'T' | '@' | 'M' | 'G' | 'P' => out.fill(0),
        _ => out.fill(b' '),
    }
}

/// Error 1581: a column that was not declared NULL was handed one.
///
/// Measured - `CREATE CURSOR t (a c(5), b n(10))` then `INSERT INTO t VALUES ("x", .NULL.)`
/// says "Field B does not accept null values." and names the column, not the value.
fn null_refused(field: &DbfField) -> RtError {
    RtError::new(RtError::NULL_REFUSED, format!("Field {} does not accept null values.", field.name.to_uppercase()))
}

/// The bytes of an empty record: every field blank, and not deleted. What APPEND BLANK adds.
///
/// Blank is not null, even where the column accepts one: measured, `APPEND BLANK` on a table
/// with a nullable column leaves `ISNULL()` false there, with `SET NULL` either way.
pub fn blank_record(header: &super::DbfHeader) -> Vec<u8> {
    let mut out = vec![b' '; header.record_len];
    for (field, &(start, width)) in header.fields.iter().zip(&header.layout) {
        let end = (start + width).min(out.len());
        if start >= end {
            continue;
        }
        blank_field(field.kind, &mut out[start..end]);
        // a blank numeric field is zero in Visual FoxPro; writing the zero says so plainly
        if matches!(field.kind, 'N' | 'F') {
            out[end - 1] = b'0';
        }
    }
    if let Some((start, width)) = header.null_flags {
        let end = (start + width).min(out.len());
        if start < end {
            out[start..end].fill(0);
        }
    }
    out
}

fn put(out: &mut [u8], bytes: &[u8]) {
    let take = bytes.len().min(out.len());
    out[..take].copy_from_slice(&bytes[..take]);
}

/// A number as `N`/`F` fields hold it: right-aligned, with exactly `decimals` places.
fn numeric_text(n: f64, width: usize, decimals: usize) -> String {
    if decimals > 0 {
        return format!("{n:.decimals$}");
    }
    let text = format!("{:.0}", n);
    if text.len() > width { format!("{n:.0}") } else { text }
}

fn as_text(v: &Value) -> Result<String, RtError> {
    match v.deref() {
        Value::Str(s) => Ok(s.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err(RtError::data_type_mismatch()),
    }
}

fn as_number(v: &Value) -> Result<f64, RtError> {
    match v.deref() {
        Value::Null => Ok(0.0),
        other => other.as_number().map_err(|_| RtError::data_type_mismatch()),
    }
}

/// A field cannot hold a value of that type. Measured in Visual FoxPro 9: `REPLACE m WITH 5`
/// over a memo, `REPLACE c WITH 5` over a character field and `REPLACE n WITH "x"` over a
/// numeric one all raise error 9, "Data type mismatch." - not the 107 an operator raises.
fn type_mismatch(field: &DbfField) -> RtError {
    RtError::new(
        RtError::DATA_TYPE_MISMATCH,
        format!("the value does not fit field {} of type {}", field.name, field.kind),
    )
}

/// The header of an empty Visual FoxPro table holding `fields`, and whether it needs a memo
/// file. This is what CREATE TABLE writes; the file is the header, then the end-of-file byte.
///
/// The layout is the one `read_header` reads: 32 bytes of file header, 32 per field, the
/// terminator, then the 263-byte backlink a VFP table carries. Version 0x30 is Visual FoxPro's,
/// and the memo flag is the header's own bit for it, so the same reader opens the result.
pub fn encode_header(given: &[super::DbfField]) -> (Vec<u8>, bool) {
    // a table with a column that accepts a null carries the hidden field the bits live in, one
    // byte to every eight such columns, written last - which is where Visual FoxPro puts it
    let nullable = given.iter().filter(|f| f.nullable).count();
    let mut owned = given.to_vec();
    if nullable > 0 {
        owned.push(super::DbfField::new(super::layout::NULL_FLAGS_FIELD, '0', nullable.div_ceil(8) as u8, 0));
    }
    let fields: &[super::DbfField] = &owned;
    let memo = fields.iter().any(|f| matches!(f.kind, 'M' | 'G' | 'P'));
    let header_len = 32 + fields.len() * 32 + 1 + 263;
    let record_len: usize = 1 + fields.iter().map(|f| f.width(false)).sum::<usize>();
    let mut out = vec![0u8; header_len];
    out[0] = 0x30;
    // last update: today is not known here, and VFP tolerates any date
    out[1] = 26;
    out[2] = 1;
    out[3] = 1;
    out[8..10].copy_from_slice(&(header_len as u16).to_le_bytes());
    out[10..12].copy_from_slice(&(record_len as u16).to_le_bytes());
    out[28] = if memo { 0x02 } else { 0 };
    // language driver: Windows ANSI, the code page every table here is decoded with by default
    out[29] = 0x03;
    let mut offset = 1usize;
    for (i, field) in fields.iter().enumerate() {
        let at = 32 + i * 32;
        // a column the program declared is held under its name in capitals, as Visual FoxPro
        // writes one; the hidden flags field keeps the name the format gives it
        let hidden = i >= given.len();
        let name = if hidden { field.name.clone() } else { field.name.to_ascii_uppercase() };
        for (k, b) in name.bytes().take(10).enumerate() {
            out[at + k] = b;
        }
        out[at + 11] = field.kind as u8;
        out[at + 12..at + 16].copy_from_slice(&(offset as u32).to_le_bytes());
        out[at + 16] = field.length;
        out[at + 17] = field.decimals;
        // binary memo and general fields keep their bytes rather than a code page; an
        // autoincrementing field is flagged, and carries what it takes next and its step
        out[at + 18] = if hidden {
            // the flags field is the table's own and holds bytes rather than text
            super::layout::FLAG_SYSTEM | 0x04
        } else if field.autoincrements() {
            0x0C
        } else if matches!(field.kind, 'G' | 'P') {
            0x04
        } else if field.nullable {
            super::layout::FLAG_NULLABLE
        } else {
            0
        };
        if field.autoincrements() {
            out[at + 19..at + 23].copy_from_slice(&field.autoinc_next.to_le_bytes());
            out[at + 23] = field.autoinc_step;
        }
        offset += field.width(false);
    }
    out[32 + fields.len() * 32] = super::layout::FIELD_TERMINATOR;
    (out, memo)
}
