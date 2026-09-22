//! The database container: the `.dbc` a database is, and the objects it holds.
//!
//! A `.dbc` is a table like any other - Visual FoxPro opens it with USE when it wants to look
//! inside one - with a fixed set of columns and a record per object: the database itself, a
//! record per table and view in it, a record per field of those, and a record per index. The
//! memo file beside it, the `.dct`, holds the properties and, for a view, the SELECT it stands
//! for.
//!
//! This reads one into a list of objects and writes that list back out, both files at once, so
//! a database this runtime makes is one Visual FoxPro can open.

use crate::dbf::{DbfField, DbfValue, read_table};

/// The columns a `.dbc` has, in the order Visual FoxPro writes them.
pub fn columns() -> Vec<DbfField> {
    vec![
        DbfField::new("OBJECTID", 'I', 4, 0),
        DbfField::new("PARENTID", 'I', 4, 0),
        DbfField::new("OBJECTTYPE", 'C', 10, 0),
        DbfField::new("OBJECTNAME", 'C', 128, 0),
        DbfField::new("PROPERTY", 'M', 4, 0),
        DbfField::new("CODE", 'M', 4, 0),
        DbfField::new("RIINFO", 'C', 6, 0),
        DbfField::new("USER", 'M', 4, 0),
    ]
}

/// One thing a database holds: itself, a table, a view, a field of one, or an index.
#[derive(Debug, Clone, PartialEq)]
pub struct DbObject {
    pub id: i32,
    pub parent: i32,
    /// "Database", "Table", "View", "Field", "Index", "Connection".
    pub kind: String,
    pub name: String,
    /// What DBSETPROP set on it, as `name=value` a line each.
    pub property: String,
    /// A view's SELECT, or a table's stored procedure text.
    pub code: String,
}

impl DbObject {
    pub fn new(id: i32, parent: i32, kind: &str, name: &str) -> DbObject {
        DbObject {
            id,
            parent,
            kind: kind.to_string(),
            name: name.to_string(),
            property: String::new(),
            code: String::new(),
        }
    }
}

/// Reads a database container: the `.dbc` bytes, and the `.dct` beside it when there is one.
pub fn read_dbc(dbf: &[u8], memo: Option<&[u8]>) -> Result<Vec<DbObject>, String> {
    let table = read_table(dbf, memo).map_err(|e| e.message)?;
    let at = |name: &str| table.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
    let (id, parent, kind, name) = (at("ObjectId"), at("ParentId"), at("ObjectType"), at("ObjectName"));
    let (property, code) = (at("Property"), at("Code"));
    let text = |values: &[DbfValue], index: Option<usize>| -> String {
        index.and_then(|i| values.get(i)).map(|v| v.as_text().trim_end().to_string()).unwrap_or_default()
    };
    let number = |values: &[DbfValue], index: Option<usize>| -> i32 {
        index.and_then(|i| values.get(i)).and_then(DbfValue::as_f64).unwrap_or(0.0) as i32
    };
    // Visual FoxPro keeps an object's properties as a binary run rather than as text, so the
    // memo reader hands that one over as bytes. It is held a byte to a character, which loses
    // nothing and leaves the `name=value` lines a container this runtime wrote keeps readable.
    let raw = |values: &[DbfValue], index: Option<usize>| -> String {
        match index.and_then(|i| values.get(i)) {
            Some(DbfValue::Bytes(bytes)) => bytes.iter().map(|b| *b as char).collect(),
            other => other.map(|v| v.as_text().trim_end().to_string()).unwrap_or_default(),
        }
    };
    Ok(table
        .records
        .iter()
        .filter(|r| !r.deleted)
        .map(|r| DbObject {
            id: number(&r.values, id),
            parent: number(&r.values, parent),
            kind: text(&r.values, kind),
            name: text(&r.values, name),
            property: raw(&r.values, property),
            code: text(&r.values, code),
        })
        .collect())
}

/// The `.dbf` a table object of a container names, relative to the container's own folder.
///
/// A container records where each of its tables is rather than assuming the table is beside it,
/// which is what lets `USE testdata!products` find a file the command never named. Visual FoxPro
/// keeps the value among the object's properties, and it keeps properties as a run of records -
/// four bytes of length counting the record itself, two of kind, one of property number, then
/// the value - with number 1 on a table being its path. Measured against the containers Visual
/// FoxPro ships: `testdata.dct` records `products.dbf` for PRODUCTS, and `orders.dbf` for
/// ORDERS, which is what `DBGETPROP("products", "Table", "Path")` answers.
///
/// A container this runtime wrote keeps its properties as `name=value` lines instead, so that
/// form is read as well, and a container that records nothing at all leaves the table beside it.
pub fn table_file(object: &DbObject) -> String {
    // held a character to a byte by `read_dbc`, which is how it crossed out of the memo
    let bytes: Vec<u8> = object.property.chars().map(|c| c as u32 as u8).collect();
    let mut at = 0usize;
    while at + 7 <= bytes.len() {
        let len = u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
        if len < 8 || at + len > bytes.len() {
            break;
        }
        if bytes[at + 6] == 1 {
            let text: String = bytes[at + 7..at + len].iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
            if !text.trim().is_empty() {
                return text.trim().to_string();
            }
        }
        at += len;
    }
    let line = object
        .property
        .lines()
        .find_map(|line| line.split_once('=').filter(|(key, _)| key.trim().eq_ignore_ascii_case("Path")))
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default();
    if line.is_empty() { format!("{}.dbf", object.name.trim()) } else { line }
}

/// Records where a table object's `.dbf` is, in the `name=value` form this runtime's own
/// containers keep their properties in.
///
/// A container records a path for every table it holds, which is what makes a rename a change of
/// name and not a change of file: measured, `RENAME TABLE shortf TO aVeryLongTableName` leaves
/// `DBGETPROP("aVeryLongTableName", "Table", "Path")` answering `shortf.dbf`.
pub fn set_table_file(object: &mut DbObject, file: &str) {
    let kept: Vec<&str> = object
        .property
        .lines()
        .filter(|line| !line.split_once('=').is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("Path")))
        .collect();
    let mut text = kept.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(&format!("Path={file}"));
    object.property = text;
}

/// Writes a database container: the table and the memo file beside it, as a pair of files.
pub fn write_dbc(objects: &[DbObject]) -> (Vec<u8>, Vec<u8>) {
    let fields = columns();
    let (mut dbf, _) = crate::dbf::write::encode_header(&fields);
    dbf[4..8].copy_from_slice(&(objects.len() as u32).to_le_bytes());
    // the memo file opens with a 512-byte header saying where the next block goes and how big
    // a block is; every memo written follows, a block at a time
    let block_size = 64usize;
    let mut memo = vec![0u8; 512];
    memo[6..8].copy_from_slice(&(block_size as u16).to_be_bytes());

    let put_memo = |text: &str, memo: &mut Vec<u8>| -> u32 {
        if text.is_empty() {
            return 0;
        }
        let block = (memo.len() / block_size) as u32;
        let bytes = text.as_bytes();
        let mut written = Vec::with_capacity(8 + bytes.len());
        written.extend_from_slice(&1u32.to_be_bytes());
        written.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        written.extend_from_slice(bytes);
        while written.len() % block_size != 0 {
            written.push(0);
        }
        memo.extend_from_slice(&written);
        block
    };

    for object in objects {
        let property = put_memo(&object.property, &mut memo);
        let code = put_memo(&object.code, &mut memo);
        dbf.push(b' ');
        dbf.extend_from_slice(&object.id.to_le_bytes());
        dbf.extend_from_slice(&object.parent.to_le_bytes());
        dbf.extend_from_slice(&fixed(&object.kind, 10));
        dbf.extend_from_slice(&fixed(&object.name, 128));
        dbf.extend_from_slice(&property.to_le_bytes());
        dbf.extend_from_slice(&code.to_le_bytes());
        dbf.extend_from_slice(&fixed("", 6));
        dbf.extend_from_slice(&0u32.to_le_bytes());
    }
    dbf.push(0x1A);
    let next = (memo.len() / block_size) as u32;
    memo[0..4].copy_from_slice(&next.to_be_bytes());
    (dbf, memo)
}

/// Text in a fixed-width column: one byte per character, padded with spaces.
fn fixed(text: &str, width: usize) -> Vec<u8> {
    let mut out: Vec<u8> = text.chars().map(|c| (c as u32 & 0xff) as u8).take(width).collect();
    out.resize(width, b' ');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_database_written_here_reads_back() {
        let mut objects = vec![DbObject::new(1, 0, "Database", "Database")];
        let mut table = DbObject::new(2, 1, "Table", "CUSTOMER");
        table.property = "Comment=the customers".into();
        objects.push(table);
        let mut view = DbObject::new(3, 1, "View", "BIGONES");
        view.code = "SELECT * FROM customer WHERE amount > 100".into();
        objects.push(view);

        let (dbf, memo) = write_dbc(&objects);
        let back = read_dbc(&dbf, Some(&memo)).expect("it reads");
        assert_eq!(back, objects);
    }
}
