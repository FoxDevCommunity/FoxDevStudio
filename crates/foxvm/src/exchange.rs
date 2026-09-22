//! `IMPORT` and `EXPORT`: a table read from a spreadsheet, and a table written out in one of the
//! interchange formats Visual FoxPro writes.
//!
//! Both sides were read off Visual FoxPro 9 itself rather than off the documentation: a table
//! was exported and the bytes compared, and a spreadsheet was imported and the structure it
//! made read back. What that settled, and what this file therefore does:
//!
//! - **DIF and SYLK end their lines with a carriage return alone**, not CR LF.
//! - **Field names are written in lower case**, whatever case the header holds them in.
//! - **Memo fields are left out** of both formats; there is no column for them at all.
//! - **A character field is written trimmed in DIF and padded to its width in SYLK.**
//! - **Anything that is not character, numeric, logical or date** - an integer, currency, a
//!   datetime - is written as the text Visual FoxPro would show, as a character value.
//! - **An imported sheet becomes a table of character columns named A, B, C**, one per column of
//!   the sheet, each as wide as its widest cell; the sheet's first row is a record like any
//!   other, because Visual FoxPro does not treat it as a heading.

use std::io::Cursor;

use calamine::{Data, Range, Reader, Xls, Xlsx};

use crate::dbf::DbfField;
use crate::error::RtError;
use crate::value::{self, Settings};
use crate::value::Value;

/// Lines in both formats end with a carriage return on its own.
const EOL: char = '\r';

/// How wide the count is written in a DIF header line: `0,` and then twenty characters.
const DIF_COUNT_WIDTH: usize = 20;

/// The same, for the SYLK line that gives the size of the sheet.
const SYLK_SIZE_WIDTH: usize = 20;

/// Whether a field goes into an exported sheet at all. A memo does not: what it holds is not in
/// the record, and Visual FoxPro leaves the column out rather than writing the block number.
pub fn exportable(field: &DbfField) -> bool {
    !matches!(field.kind, 'M' | 'G' | 'P')
}

/// `EXPORT TO file TYPE DIF`: one vector per field, one tuple per record, and a tuple of the
/// field names before them.
pub fn dif(fields: &[DbfField], rows: &[&[Value]], settings: &Settings) -> String {
    let columns: Vec<(usize, &DbfField)> = fields.iter().enumerate().filter(|(_, f)| exportable(f)).collect();
    let mut out = String::new();
    let header = |out: &mut String, word: &str, count: usize| {
        out.push_str(word);
        out.push(EOL);
        out.push_str("0,");
        out.push_str(&format!("{count:<DIF_COUNT_WIDTH$}"));
        out.push(EOL);
        out.push_str("\"\"");
        out.push(EOL);
    };

    out.push_str("TABLE");
    out.push(EOL);
    out.push_str("0,1");
    out.push(EOL);
    out.push_str("\"\"");
    out.push(EOL);
    // the tuples are the records and the row of field names before them
    header(&mut out, "TUPLES", rows.len() + 1);
    header(&mut out, "VECTORS", columns.len());
    out.push_str("DATA");
    out.push(EOL);
    out.push_str("0,0");
    out.push(EOL);
    out.push_str("\"\"");
    out.push(EOL);

    out.push_str("-1,0");
    out.push(EOL);
    out.push_str("BOT");
    out.push(EOL);
    for (_, field) in &columns {
        dif_text(&mut out, &field.name.to_lowercase());
    }
    for row in rows {
        out.push_str("-1,0");
        out.push(EOL);
        out.push_str("BOT");
        out.push(EOL);
        for (i, field) in &columns {
            let value = row.get(*i).cloned().unwrap_or(Value::Null);
            dif_value(&mut out, field, &value, settings);
        }
    }
    out.push_str("-1,0");
    out.push(EOL);
    out.push_str("EOD");
    out.push(EOL);
    out
}

/// A character value: type 1, and the text in quotes on the line after it.
fn dif_text(out: &mut String, text: &str) {
    out.push_str("1,0");
    out.push(EOL);
    out.push('"');
    out.push_str(text);
    out.push('"');
    out.push(EOL);
}

/// A numeric value: type 0 with the number on the same line, and what it stands for after it.
fn dif_number(out: &mut String, number: &str, word: &str) {
    out.push_str("0,");
    out.push_str(number);
    out.push(EOL);
    out.push_str(word);
    out.push(EOL);
}

fn dif_value(out: &mut String, field: &DbfField, value: &Value, settings: &Settings) {
    match field.kind {
        'C' => dif_text(out, as_text(value, settings).trim_end()),
        'N' | 'F' => dif_number(out, &number_text(value, field.decimals), "V"),
        'L' => {
            let yes = matches!(value.deref(), Value::Logical(true));
            dif_number(out, if yes { "1" } else { "0" }, if yes { "TRUE" } else { "FALSE" });
        }
        'D' => dif_number(out, &date_digits(value), "V"),
        // an integer, a currency, a datetime: the text Visual FoxPro would show
        _ => dif_text(out, as_text(value, settings).trim_end()),
    }
}

/// `EXPORT TO file TYPE SYLK`: one cell per field and record, the field names in the first row.
pub fn sylk(fields: &[DbfField], rows: &[&[Value]], settings: &Settings) -> String {
    let columns: Vec<(usize, &DbfField)> = fields.iter().enumerate().filter(|(_, f)| exportable(f)).collect();
    let mut out = String::new();
    out.push_str("ID;PFOXPRO");
    out.push(EOL);
    out.push_str("F;DG0G10");
    out.push(EOL);
    let size = format!("B;Y{};X{}", rows.len() + 1, columns.len());
    out.push_str(&format!("{size:<SYLK_SIZE_WIDTH$}"));
    out.push(EOL);

    let cell = |out: &mut String, y: usize, x: usize, text: &str| {
        out.push_str("C;");
        if x == 1 {
            out.push_str(&format!("Y{y};"));
        }
        out.push_str(&format!("X{x};K{text}"));
        out.push(EOL);
    };

    for (x, (_, field)) in columns.iter().enumerate() {
        cell(&mut out, 1, x + 1, &format!("\"{}\"", field.name.to_lowercase()));
    }
    for (y, row) in rows.iter().enumerate() {
        for (x, (i, field)) in columns.iter().enumerate() {
            let value = row.get(*i).cloned().unwrap_or(Value::Null);
            cell(&mut out, y + 2, x + 1, &sylk_value(field, &value, settings));
        }
    }
    out.push_str("W;N1;A1 1");
    out.push(EOL);
    out.push('E');
    out.push(EOL);
    out
}

fn sylk_value(field: &DbfField, value: &Value, settings: &Settings) -> String {
    let width = field.length as usize;
    match field.kind {
        // a cell is as wide as the field, so what is written keeps the field's shape
        'C' => format!("\"{:<width$}\"", as_text(value, settings)),
        'N' | 'F' => format!("{:>width$}", number_text(value, field.decimals)),
        'L' => format!("\"{}\"", if matches!(value.deref(), Value::Logical(true)) { "T" } else { "F" }),
        'D' => {
            let digits = date_digits(value);
            format!("\"{:<8}\"", digits)
        }
        _ => format!("\"{}\"", as_text(value, settings)),
    }
}

fn as_text(value: &Value, settings: &Settings) -> String {
    match value.deref() {
        Value::Str(s) => s.to_string(),
        other => value::display(&other, settings),
    }
}

/// A number with the decimals the field was declared with, which is how it reads back.
fn number_text(value: &Value, decimals: u8) -> String {
    match value.deref() {
        // the decimals the field was declared with, always: a whole number in an N(9,2)
        // field is written -8.00, not -8
        Value::Number(n, ..) => format!("{:.*}", decimals as usize, n),
        Value::Null => String::new(),
        other => value::display(&other, &Settings::default()),
    }
}

/// A date as `YYYYMMDD`, and nothing at all when there is no date.
fn date_digits(value: &Value) -> String {
    match value.deref() {
        Value::Date(Some(days)) => {
            let (y, m, d) = value::civil_from_days(days);
            format!("{y:04}{m:02}{d:02}")
        }
        _ => String::new(),
    }
}

// ----- reading a sheet ---------------------------------------------------------------------

/// `IMPORT FROM file`: the sheet as a table of character columns.
///
/// Visual FoxPro names the columns A, B, C - the sheet's own first row is a record like any
/// other - and makes each as wide as its widest cell, which is what this does. A column whose
/// cells are all numbers is still character when any other cell in it is not, because the width
/// has to hold whatever is written there.
pub fn read_sheet(bytes: &[u8], sheet: Option<&str>) -> Result<(Vec<DbfField>, Vec<Vec<Value>>), RtError> {
    let cells = open_sheet(bytes, sheet)?;
    let width = cells.get_size().1;
    let mut texts: Vec<Vec<String>> = Vec::new();
    for row in cells.rows() {
        let mut line = Vec::with_capacity(width);
        for column in 0..width {
            line.push(cell_text(row.get(column).unwrap_or(&Data::Empty)));
        }
        texts.push(line);
    }

    let mut fields = Vec::with_capacity(width);
    for column in 0..width {
        let widest = texts.iter().filter_map(|r| r.get(column)).map(|t| t.chars().count()).max().unwrap_or(1);
        fields.push(DbfField::new(column_name(column), 'C', widest.clamp(1, 254) as u8, 0));
    }
    let rows = texts.into_iter().map(|r| r.into_iter().map(Value::str).collect()).collect();
    Ok((fields, rows))
}

/// The reader the file's own first bytes call for: a workbook is a zip, and the older one is an
/// OLE compound file.
fn open_sheet(bytes: &[u8], sheet: Option<&str>) -> Result<Range<Data>, RtError> {
    let named = |range: Option<Range<Data>>| {
        range.ok_or_else(|| {
            RtError::new(
                1104,
                match sheet {
                    Some(name) => format!("IMPORT: the workbook has no sheet called '{name}'"),
                    None => "IMPORT: the workbook has no sheets in it".to_string(),
                },
            )
        })
    };
    let failed = |e: calamine::Error| RtError::new(1104, format!("IMPORT: the workbook could not be read. {e}"));

    if bytes.starts_with(b"PK") {
        let mut book = Xlsx::new(Cursor::new(bytes.to_vec())).map_err(|e| failed(e.into()))?;
        return named(match sheet {
            Some(name) => book.worksheet_range(name).ok(),
            None => book.worksheet_range_at(0).and_then(|r| r.ok()),
        });
    }
    let mut book = Xls::new(Cursor::new(bytes.to_vec())).map_err(|e| failed(e.into()))?;
    named(match sheet {
        Some(name) => book.worksheet_range(name).ok(),
        None => book.worksheet_range_at(0).and_then(|r| r.ok()),
    })
}

/// What one cell says, as text. A sheet holds numbers, text, dates and errors; a table column
/// holds characters, so this is what goes in it.
fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        // as the number would be shown, which is what Visual FoxPro puts in the column
        Data::Float(f) => value::display(&Value::number(*f), &Settings::default()),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "T".to_string() } else { "F".to_string() },
        Data::DateTime(d) => value::display(&Value::number(d.as_f64()), &Settings::default()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("{e:?}"),
    }
}

/// A spreadsheet's name for a column: A to Z, then AA and on.
fn column_name(index: usize) -> String {
    let mut name = String::new();
    let mut n = index;
    loop {
        name.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_named_the_way_a_spreadsheet_names_them() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(27), "AB");
        assert_eq!(column_name(51), "AZ");
        assert_eq!(column_name(52), "BA");
    }
}
