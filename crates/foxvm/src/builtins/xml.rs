//! Turning a cursor into XML and XML back into a cursor.
//!
//! Visual FoxPro gives a program two ways at the same thing: the functions here, and the
//! XMLAdapter object, which is these functions with the shape of the document exposed as
//! properties. Both go through `crate::xml`, so a document either of them writes reads back
//! through the other.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, arg_str, opt_int, opt_str, spec};
use crate::data::{AreaRef, Cursor, bytes_of};
use crate::dbf::{DbfField, DbfRecord, DbfValue};
use crate::error::RtError;
use crate::host::HostRequest;
use crate::value::Value;
use crate::xml::{Element, local_name};

fn ok(v: Value) -> Result<BuiltinResult, RtError> {
    Ok(BuiltinResult::Value(v))
}

/// What a document's rows are called, and what one row is called, for a cursor of that name.
fn names(alias: &str) -> (String, String) {
    ("VFPData".to_string(), alias.to_ascii_lowercase())
}

/// CURSORTOXML(nWorkArea | cAlias, cOutput [, nOutputFormat [, nFlags [, nRecords
/// [, cSchemaName]]]]): writes the records of a work area as XML into the memory variable
/// `cOutput` names - creating it if there was none - and answers with the number of bytes
/// written, the way `STRTOFILE()` does; it does not, itself, answer with the XML.
///
/// `nOutputFormat` 2 or 3 writes each field as an attribute rather than as an element; 1, and
/// omitting it, are both the element-centric default - measured, because a call that passes the
/// reference page's own default value back in in full is a form this runtime once answered
/// backwards, treating explicit 1 as attributes and 2 as elements.
///
/// `nRecords` of 0, the default, exports the whole table from the beginning regardless of where
/// the record pointer sits; greater than 0, it exports that many records starting from the
/// record pointer instead. Bit 0 of `nFlags` asks for a continuous string with no line breaks or
/// indentation, in the declaration as well as the body.
///
/// A cursor that lives in memory is written from what it holds. One that lives in a file is
/// read whole first, because a document is written in one piece and the records of a table are
/// only in hand a page at a time.
fn f_cursortoxml(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let area = match a.first().map(Value::deref) {
        Some(Value::Number(n, ..)) => AreaRef::Number(n as usize),
        other => AreaRef::Alias(other.map(|v| v.as_str().unwrap_or_default().to_string()).unwrap_or_default()),
    };
    let cursor = c.data().find(&area).ok_or_else(|| RtError::new(52, "Alias 'name' is not found"))?;
    let (alias, path, deleted_kept, codepage) =
        (cursor.alias.clone(), cursor.path.clone(), c.settings().deleted, cursor.header.codepage);
    let fields: Vec<DbfField> = cursor.header.fields.clone();
    let limit = opt_int(&a, 4, 0)?;
    // 0 means the whole table, from the top, whatever the record pointer holds; a limit takes
    // that many records starting from the pointer - measured against the product
    let from = if limit == 0 { 1 } else { cursor.recno().max(1) };
    let held: Vec<DbfRecord> = cursor.all_rows().to_vec();

    let rows: Vec<DbfRecord> = match c.take_data_reply() {
        // the answer is the file: every record of it, decoded
        Some(reply) => {
            let bytes = bytes_of(&reply);
            crate::dbf::read_table(&bytes, None).map_err(|e| RtError::new(1429, e.message))?.records
        }
        None if !held.is_empty() || path.is_empty() => held,
        None => return Ok(BuiltinResult::SuspendData { request: HostRequest::FileReadBytes { path }, args: a }),
    };

    let format = opt_int(&a, 2, 0)?;
    let attribute_centric = matches!(format, 2 | 3);
    let flags = opt_int(&a, 3, 0)?;
    let formatted = flags & 1 == 0;
    let schema = opt_str(&a, 5, "")?;
    let (root_name, row_name) = names(&alias);
    let mut root = Element::new(&root_name);
    // "1" asks for an inline schema, written as an <xsd:schema> child of the root - which this
    // runtime does not generate; a schema file name is the form measured and implemented, and
    // is written as an attribute rather than a child, with the namespace it belongs to declared
    // alongside it.
    if !schema.is_empty() && schema != "1" {
        root.set_attr("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance");
        root.set_attr("xsi:noNamespaceSchemaLocation", schema);
    }
    let mut written = 0i64;
    for record in rows.iter().skip(from.saturating_sub(1) as usize) {
        if record.deleted && !deleted_kept {
            continue;
        }
        if limit > 0 && written >= limit {
            break;
        }
        written += 1;
        let mut row = Element::new(&row_name);
        for (field, value) in fields.iter().zip(record.values.iter()) {
            let text = xml_text(value, field);
            if text.is_empty() {
                continue;
            }
            let name = field.name.to_ascii_lowercase();
            if attribute_centric {
                row.set_attr(name, text);
            } else {
                let mut child = Element::new(&name);
                child.text = text;
                row.children.push(child);
            }
        }
        root.children.push(row);
    }
    let body = crate::xml::write(&root, formatted.then_some(0));
    let text = declaration(&body, codepage, formatted);
    let bytes = text.len() as f64;
    c.store_named(&arg_str(&a, 1)?, Value::str(text))?;
    ok(Value::number(bytes))
}

/// A document with the declaration Visual FoxPro writes in front of it - measured against the
/// product: a space either side of the first `=` and none around the second, the cursor's own
/// code page rather than a fixed one, and no line break at all after it when the body itself is
/// the continuous, unformatted kind.
fn declaration(body: &str, codepage: Option<u16>, formatted: bool) -> String {
    let head = format!("<?xml version = \"1.0\" encoding=\"{}\" standalone=\"yes\"?>", codepage_name(codepage));
    if formatted { format!("{head}\r\n{body}") } else { format!("{head}{body}") }
}

/// The encoding attribute Visual FoxPro's XML declaration carries for a table's code page -
/// its own table, read off the reference page, not a general codepage-to-name mapping.
fn codepage_name(codepage: Option<u16>) -> &'static str {
    match codepage {
        Some(437) => "ibm437",
        Some(850) => "ibm850",
        Some(866) => "cp866",
        Some(932) => "shift-jis",
        Some(936) => "gb2312",
        Some(950) => "big5",
        Some(1250) => "Windows-1250",
        Some(1251) => "Windows-1251",
        Some(1253) => "Windows-1253",
        Some(1254) => "Windows-1254",
        Some(1255) => "Windows-1255",
        Some(1256) => "Windows-1256",
        _ => "Windows-1252",
    }
}

/// One field's value as XML writes it: dates and datetimes go out the way a schema wants them.
fn xml_text(value: &DbfValue, field: &DbfField) -> String {
    match value {
        DbfValue::Null => String::new(),
        DbfValue::Text(text) | DbfValue::Memo(text) => text.trim_end().to_string(),
        // money is written out as the amount it stands for, to its four places
        DbfValue::Currency(c) => format!("{:.4}", *c as f64 / crate::dbf::CURRENCY_SCALE as f64),
        DbfValue::Number(n) => {
            if field.decimals > 0 {
                format!("{n:.*}", field.decimals as usize)
            } else {
                format!("{n}")
            }
        }
        DbfValue::Logical(b) => if *b { "true" } else { "false" }.to_string(),
        DbfValue::Date(None) | DbfValue::DateTime(None) => String::new(),
        DbfValue::Date(Some(days)) => {
            let (y, m, d) = crate::value::civil_from_days(*days);
            format!("{y:04}-{m:02}-{d:02}")
        }
        DbfValue::DateTime(Some(seconds)) => {
            let (y, m, d) = crate::value::civil_from_days(seconds.div_euclid(86400.0) as i32);
            let rest = seconds.rem_euclid(86400.0).floor() as u32;
            format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}", rest / 3600, (rest % 3600) / 60, rest % 60)
        }
        DbfValue::Bytes(bytes) => bytes.iter().map(|b| format!("{b:02x}")).collect(),
    }
}

/// XMLTOCURSOR(cXMLSource | cVarName, [cCursorName] [, nFlags]): reads XML into a cursor.
///
/// The source is the XML itself, or the name of a file when it looks like one. Every element
/// under the outermost one is a record, and its children and attributes are the fields. Bit 2 of
/// `nFlags` (4) keeps a field's leading and trailing spaces; without it, both sides are trimmed -
/// not just the trailing one, which is what this runtime did before it was measured.
fn f_xmltocursor(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let source = arg_str(&a, 0)?.to_string();
    let alias = opt_str(&a, 1, "")?;
    let preserve_whitespace = opt_int(&a, 2, 0)? & 4 != 0;
    let document = crate::xml::parse(&source).map_err(|e| RtError::new(1429, format!("XML is not well formed: {e}")))?;
    // "Defaults to XMLRESULT if omitted or empty string" - the document's own root name, not
    // this one, which is what this runtime used before it was measured
    let alias = if alias.trim().is_empty() { "XMLRESULT".to_string() } else { alias };

    // an inline schema is not a row: a document written by VFP or by ADO.NET carries one
    let rows: Vec<&Element> = document.children.iter().filter(|c| !crate::xml::is_schema(c)).collect();
    let (fields, values) = shape(&rows, preserve_whitespace);
    if fields.is_empty() {
        return Err(RtError::new(1429, "the XML has no rows in it to make a cursor from"));
    }
    let records: Vec<DbfRecord> = values.into_iter().map(|values| DbfRecord { deleted: false, values }).collect();
    let data = c.data_mut();
    if data.used(&AreaRef::Alias(alias.clone())) {
        data.select(&AreaRef::Alias(alias.clone()))?;
    } else {
        data.select(&AreaRef::Number(0))?;
    }
    data.install(Cursor::in_memory(alias, fields, records));
    ok(Value::number(1.0))
}

/// The fields the rows between them have, and each row's values in that order.
///
/// XML says nothing about how wide a field is, so each one is as wide as the widest value it
/// was given, and a column whose every value reads as a number becomes a numeric field.
fn shape(rows: &[&Element], preserve_whitespace: bool) -> (Vec<DbfField>, Vec<Vec<DbfValue>>) {
    // the field this row's text becomes, spaces kept or trimmed off both ends
    let field_text = |v: &str| if preserve_whitespace { v.to_string() } else { v.trim().to_string() };
    let mut names: Vec<String> = Vec::new();
    let mut cells: Vec<Vec<String>> = Vec::new();
    for row in rows {
        let mut found: Vec<(String, String)> = row.attributes.iter().map(|(k, v)| (local_name(k).to_string(), v.clone())).collect();
        found.extend(row.children.iter().map(|child| (child.local_name().to_string(), child.content())));
        for (name, _) in &found {
            if !names.iter().any(|k| k.eq_ignore_ascii_case(name)) {
                names.push(name.clone());
            }
        }
        cells.push(
            names
                .iter()
                .map(|name| found.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.clone()).unwrap_or_default())
                .collect(),
        );
    }
    // a row read before a later one introduced a field is short; pad them all to the same width
    for row in &mut cells {
        row.resize(names.len(), String::new());
    }

    let fields: Vec<DbfField> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let column: Vec<&String> = cells.iter().filter_map(|row| row.get(i)).collect();
            let numeric = !column.is_empty()
                && column.iter().all(|v| v.trim().is_empty() || v.trim().parse::<f64>().is_ok())
                && column.iter().any(|v| !v.trim().is_empty());
            let width = column.iter().map(|v| field_text(v).chars().count()).max().unwrap_or(1).clamp(1, 254);
            let decimals = column
                .iter()
                .filter_map(|v| v.trim().split_once('.').map(|(_, rest)| rest.chars().count()))
                .max()
                .unwrap_or(0)
                .min(15);
            let name: String =
                name.to_ascii_uppercase().chars().filter(|c| c.is_alphanumeric() || *c == '_').take(10).collect();
            if numeric {
                DbfField::new(name, 'N', (width.max(decimals + 2)) as u8, decimals as u8)
            } else {
                DbfField::new(name, 'C', width as u8, 0)
            }
        })
        .collect();

    let values: Vec<Vec<DbfValue>> = cells
        .iter()
        .map(|row| {
            row.iter()
                .zip(fields.iter())
                .map(|(text, field)| match field.kind {
                    'N' => text.trim().parse::<f64>().map(DbfValue::Number).unwrap_or(DbfValue::Null),
                    _ => DbfValue::Text(field_text(text)),
                })
                .collect()
        })
        .collect();
    (fields, values)
}

/// XMLUPDATEGRAM([cAliasList [, nFlags [, cSchemaLocation]]]): the changes a buffered table is
/// holding, as the updategram a server takes to apply them.
///
/// Every changed record's before and after go in one `updg:sync`, not one each - measured, and
/// unlike this runtime's first attempt at it. A record with nothing to say for one side, an
/// insert's before or a delete's after, still gets the element, empty: `<updg:before/>`, not left
/// out. `cSchemaLocation` is written as `mapping-schema` on that same `updg:sync`, and bit 0 of
/// `nFlags` asks for a continuous string the same way it does for `CURSORTOXML()`.
fn f_xmlupdategram(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let alias = opt_str(&a, 0, "")?;
    // no alias means the work area that is selected, not the lowest free one
    let found = if alias.trim().is_empty() { c.data().cursor() } else { c.data().find(&AreaRef::Alias(alias)) };
    let Some(cursor) = found else {
        return Err(RtError::new(52, "Alias 'name' is not found"));
    };
    let row_name = cursor.alias.to_ascii_lowercase();
    let header = cursor.header.clone();
    let fields: Vec<DbfField> = header.fields.clone();
    let codepage = header.codepage;
    let flags = opt_int(&a, 1, 0)?;
    let formatted = flags & 1 == 0;
    let schema_location = opt_str(&a, 2, "")?;

    let mut root = Element::new("root");
    root.set_attr("xmlns:updg", "urn:schemas-microsoft-com:xml-updategram");
    let mut sync = Element::new("updg:sync");
    if !schema_location.is_empty() {
        sync.set_attr("mapping-schema", schema_location);
    }
    for (recno, held) in cursor.held() {
        // what is held is the changed record; the file still has what it was read with
        let after = crate::dbf::decode_record(&header, &held.bytes, crate::dbf::Padding::Trim, |_| None);
        let before = cursor
            .original_bytes(recno)
            .or_else(|| cursor.stored(recno))
            .map(|bytes| crate::dbf::decode_record(&header, bytes, crate::dbf::Padding::Trim, |_| None));
        for (tag, values) in [("updg:before", before), ("updg:after", Some(after))] {
            let mut side = Element::new(tag);
            if let Some(values) = values {
                let mut row = Element::new(&row_name);
                row.set_attr("updg:id", recno.to_string());
                for (field, value) in fields.iter().zip(values.values.iter()) {
                    let text = xml_text(value, field);
                    if !text.is_empty() {
                        row.set_attr(field.name.to_ascii_lowercase(), text);
                    }
                }
                side.children.push(row);
            }
            sync.children.push(side);
        }
    }
    root.children.push(sync);
    let body = crate::xml::write(&root, formatted.then_some(0));
    ok(Value::str(declaration(&body, codepage, formatted)))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("CURSORTOXML", 2, 6, f_cursortoxml),
        spec("XMLTOCURSOR", 1, 3, f_xmltocursor),
        spec("XMLUPDATEGRAM", 0, 3, f_xmlupdategram),
    ]
}
