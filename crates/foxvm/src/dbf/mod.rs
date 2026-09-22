//! A reader for dBASE / FoxPro `.dbf` tables and their `.fpt` memo files.
//!
//! The reader is pure and allocation-only: it takes the bytes of a table (and, when the table has
//! memo fields, the bytes of the sibling memo file) and returns a fully decoded [`DbfTable`]. It
//! never seeks, never opens files and never panics on malformed input - every structural problem
//! is either a [`DbfError`] or, for a single unreadable field, a [`DbfValue::Null`].
//!
//! Two consumers share it: the Visual FoxPro project importer, which reads `.pjx` project tables
//! (a `.dbf` whose memo file is named `.pjt`), and the data engine.
//!
//! ```no_run
//! # fn main() -> Result<(), foxvm::dbf::DbfError> {
//! let dbf = std::fs::read("formsui.pjx").unwrap();
//! let pjt = std::fs::read("formsui.PJT").ok();
//! let table = foxvm::dbf::read_table(&dbf, pjt.as_deref())?;
//! for rec in table.records.iter().filter(|r| !r.deleted) {
//!     let name = rec.get(&table, "NAME").map(|v| v.as_text()).unwrap_or("");
//!     let kind = rec.get(&table, "TYPE").map(|v| v.as_text()).unwrap_or("");
//!     println!("{kind}: {name}");
//! }
//! # Ok(())
//! # }
//! ```

pub mod encoding;
pub mod layout;
pub mod memo;
pub mod write;

use crate::value::{days_from_civil, is_valid_date};
pub use layout::{DbfHeader, read_header};
use memo::{MemoBlock, MemoFile};

/// Julian day number of 1970-01-01, the epoch the `T` field type is converted to.
pub(crate) const JULIAN_EPOCH: i32 = 2_440_588;

/// Anything that stops a table from being read at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbfError {
    pub message: String,
}

impl DbfError {
    pub fn new(message: impl Into<String>) -> DbfError {
        DbfError { message: message.into() }
    }
}

impl std::fmt::Display for DbfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DbfError {}

/// One column of a table, in file order.
///
/// `length` is the raw descriptor byte. FoxPro 2.x encodes character fields wider than 255 by
/// borrowing `decimals` as the high byte, so the effective width of a `C` field is
/// `length as usize + decimals as usize * 256` when that still fits the record; see
/// [`DbfField::width`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DbfField {
    pub name: String,
    pub kind: char,
    pub length: u8,
    pub decimals: u8,
    /// How much an autoincrementing field goes up by, and what it takes next. A step of zero
    /// means the field does not increment, which is every field but the ones declared AUTOINC.
    pub autoinc_step: u8,
    pub autoinc_next: u32,
    /// Whether the field accepts `.NULL.`. It is bit 0x02 of the descriptor's flag byte, and it
    /// is what `NULL` in a column declaration asks for. A field that does not have it refuses:
    /// measured, `INSERT INTO t VALUES ("x", .NULL.)` into a plain column raises error 1581,
    /// "Field B does not accept null values."
    #[serde(default)]
    pub nullable: bool,
}

impl DbfField {
    /// A plain field of that name, type and width.
    pub fn new(name: impl Into<String>, kind: char, length: u8, decimals: u8) -> DbfField {
        DbfField { name: name.into(), kind, length, decimals, autoinc_step: 0, autoinc_next: 0, nullable: false }
    }

    /// The same field, declared to accept `.NULL.` or not.
    pub fn accepting_null(mut self, nullable: bool) -> DbfField {
        self.nullable = nullable;
        self
    }

    /// Whether the table fills it in itself as each record is added.
    pub fn autoincrements(&self) -> bool {
        self.autoinc_step > 0
    }
}

impl DbfField {
    /// The width of the field in a record, as stored.
    pub(super) fn width(&self, wide_char: bool) -> usize {
        if wide_char && self.kind == 'C' && self.decimals > 0 {
            self.length as usize + self.decimals as usize * 256
        } else {
            self.length as usize
        }
    }
}

/// A Y field holds whole ten-thousandths, and so does a Currency value.
pub const CURRENCY_SCALE: i64 = 10_000;

/// A decoded field value. One per field of the table, in field order.
#[derive(Debug, Clone, PartialEq)]
pub enum DbfValue {
    /// An empty numeric/logical field, an empty memo pointer, or a field that could not be read.
    Null,
    Text(String),
    Number(f64),
    /// A Y field: money, in ten-thousandths, as the file keeps it.
    Currency(i64),
    Logical(bool),
    /// Days since 1970-01-01; `None` for an empty date.
    Date(Option<i32>),
    /// Seconds since 1970-01-01T00:00:00; `None` for an empty datetime.
    DateTime(Option<f64>),
    Memo(String),
    Bytes(Vec<u8>),
}

impl DbfValue {
    /// The text of a character or memo field, `""` for anything else. Never panics.
    pub fn as_text(&self) -> &str {
        match self {
            DbfValue::Text(s) | DbfValue::Memo(s) => s,
            _ => "",
        }
    }

    /// The numeric value of a field, converting text, logicals and dates where that is meaningful.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DbfValue::Number(n) => Some(*n),
            DbfValue::Currency(c) => Some(*c as f64 / CURRENCY_SCALE as f64),
            DbfValue::Logical(b) => Some(if *b { 1.0 } else { 0.0 }),
            DbfValue::Date(d) => d.map(|d| d as f64),
            DbfValue::DateTime(t) => *t,
            DbfValue::Text(s) | DbfValue::Memo(s) => s.trim().parse::<f64>().ok(),
            DbfValue::Null | DbfValue::Bytes(_) => None,
        }
    }

    /// The truth of a field. `Null` and unreadable values are false; never panics.
    pub fn as_bool(&self) -> bool {
        match self {
            DbfValue::Logical(b) => *b,
            DbfValue::Number(n) => *n != 0.0,
            DbfValue::Currency(c) => *c != 0,
            DbfValue::Text(s) | DbfValue::Memo(s) => {
                matches!(s.trim().as_bytes().first(), Some(b'T' | b't' | b'Y' | b'y' | b'1'))
            }
            DbfValue::Date(d) => d.is_some(),
            DbfValue::DateTime(t) => t.is_some(),
            DbfValue::Null => false,
            DbfValue::Bytes(b) => !b.is_empty(),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, DbfValue::Null)
    }
}

/// One record. `values` has exactly one entry per field of the table, in field order.
///
/// Deleted records are kept, flagged; the caller decides whether to skip them.
#[derive(Debug, Clone, PartialEq)]
pub struct DbfRecord {
    pub deleted: bool,
    pub values: Vec<DbfValue>,
}

impl DbfRecord {
    /// Looks a field up by name, case-insensitively.
    pub fn get<'a>(&'a self, table: &DbfTable, name: &str) -> Option<&'a DbfValue> {
        self.values.get(table.field_index(name)?)
    }
}

/// A decoded table.
#[derive(Debug, Clone, PartialEq)]
pub struct DbfTable {
    /// The version byte at offset 0: `0x03` dBASE III, `0x30`/`0x31`/`0x32` Visual FoxPro,
    /// `0x83`/`0xF5` dBASE III / FoxPro 2 with a memo file.
    pub version: u8,
    pub fields: Vec<DbfField>,
    pub records: Vec<DbfRecord>,
    /// The code page the text was decoded with, `None` when the header records none (in which
    /// case 1252 was used).
    pub codepage: Option<u16>,
}

impl DbfTable {
    /// Index of the field called `name`, case-insensitively.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// The field called `name`, case-insensitively.
    pub fn field(&self, name: &str) -> Option<&DbfField> {
        self.fields.get(self.field_index(name)?)
    }

    /// True when the version byte or any field says the table needs a memo file.
    pub fn has_memo(&self) -> bool {
        matches!(self.version, 0x83 | 0x8B | 0xF5 | 0xFB)
            || self.fields.iter().any(|f| matches!(f.kind, 'M' | 'G' | 'P'))
    }
}

/// Reads a whole table.
///
/// `memo` is the contents of the sibling `.fpt` / `.pjt` / `.sct` / `.vct` / `.mnt` file, when
/// there is one; memo fields decode to [`DbfValue::Null`] without it. Memo file names must be
/// resolved case-insensitively by the caller - VFP 8 samples ship `formsui.pjx` next to
/// `formsui.PJT`.
pub fn read_table(dbf: &[u8], memo: Option<&[u8]>) -> Result<DbfTable, DbfError> {
    let header = layout::read_header(dbf)?;
    let record_count = header.record_count as usize;

    let body = &dbf[header.header_len..];
    let available = body.len() / header.record_len;
    if available < record_count {
        return Err(DbfError::new(format!(
            "file is truncated: the header declares {record_count} records of {} bytes but only {available} are present",
            header.record_len
        )));
    }

    let memo = match memo {
        Some(bytes) => Some(MemoFile::open(bytes)?),
        None => None,
    };

    let mut records = Vec::with_capacity(record_count);
    for i in 0..record_count {
        let raw = &body[i * header.record_len..(i + 1) * header.record_len];
        records.push(decode_record(&header, raw, Padding::Trim, |block| memo.as_ref().and_then(|m| m.block(block)).map(owned_block)));
    }

    Ok(DbfTable { version: header.version, fields: header.fields, records, codepage: header.codepage })
}

/// One memo block, detached from the file it came from, as the cursor engine receives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedMemoBlock {
    Text(Vec<u8>),
    Binary(Vec<u8>),
}

fn owned_block(block: MemoBlock<'_>) -> OwnedMemoBlock {
    match block {
        MemoBlock::Text(b) => OwnedMemoBlock::Text(b.to_vec()),
        MemoBlock::Binary(b) => OwnedMemoBlock::Binary(b.to_vec()),
    }
}

/// What to do with the blanks a character field is padded with on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Padding {
    /// Keep them, which is what the language sees: `LEN(cust.name)` is the field width.
    Keep,
    /// Drop them. The design formats store names and paths padded and every reader of them
    /// wants the name, not the column.
    Trim,
}

/// Decodes one record's bytes against a header. `memo` resolves a memo block number, which the
/// cursor engine answers from blocks it fetched from the host and the whole-file reader answers
/// from the memo file it already holds.
pub fn decode_record(
    header: &DbfHeader,
    raw: &[u8],
    padding: Padding,
    mut memo: impl FnMut(u32) -> Option<OwnedMemoBlock>,
) -> DbfRecord {
    let deleted = raw.first() == Some(&layout::FLAG_DELETED);
    let values = header
        .fields
        .iter()
        .zip(&header.layout)
        .enumerate()
        .map(|(index, (field, &(start, width)))| {
            let end = (start + width).min(raw.len());
            // the record's own flags say which of the columns that accept a null are holding one
            if start >= end || header.is_null(raw, index) {
                DbfValue::Null
            } else {
                decode_value(field.kind, &raw[start..end], header.codepage, padding, &mut memo)
            }
        })
        .collect();
    DbfRecord { deleted, values }
}

/// The memo block number a record field points at, or `None` when it points at nothing.
pub fn memo_pointer(raw: &[u8]) -> Option<u32> {
    let block = if raw.len() == 4 {
        u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])
    } else {
        let text: String = raw.iter().map(|&b| b as char).collect();
        text.trim_matches([' ', '\0']).parse::<u32>().ok()?
    };
    (block != 0).then_some(block)
}

fn decode_value(
    kind: char,
    raw: &[u8],
    codepage: Option<u16>,
    padding: Padding,
    memo: &mut impl FnMut(u32) -> Option<OwnedMemoBlock>,
) -> DbfValue {
    match kind {
        'C' => {
            let s = encoding::decode(raw, codepage);
            DbfValue::Text(match padding {
                Padding::Keep => s.trim_end_matches('\0').to_string(),
                Padding::Trim => s.trim_end_matches([' ', '\0']).to_string(),
            })
        }
        'N' | 'F' => parse_number(raw),
        'I' | '+' => match raw.get(..4) {
            Some(b) => DbfValue::Number(i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64),
            None => DbfValue::Null,
        },
        'B' | 'O' => match raw.get(..8) {
            Some(b) => DbfValue::Number(f64::from_le_bytes(b.try_into().unwrap_or([0; 8]))),
            None => DbfValue::Null,
        },
        // money is kept in the file the way it is kept in a variable: whole ten-thousandths
        'Y' => match raw.get(..8) {
            Some(b) => DbfValue::Currency(i64::from_le_bytes(b.try_into().unwrap_or([0; 8]))),
            None => DbfValue::Null,
        },
        'L' => match raw.first() {
            Some(b'T' | b't' | b'Y' | b'y') => DbfValue::Logical(true),
            Some(b'F' | b'f' | b'N' | b'n') => DbfValue::Logical(false),
            _ => DbfValue::Null,
        },
        'D' => parse_date(raw),
        'T' | '@' => parse_datetime(raw),
        'M' | 'G' | 'P' => read_memo(raw, codepage, memo),
        _ => DbfValue::Bytes(raw.to_vec()),
    }
}

/// `N`/`F` fields are right-aligned ASCII. FoxPro writes all-`*` when a value overflows the field.
fn parse_number(raw: &[u8]) -> DbfValue {
    let text: String = raw.iter().map(|&b| b as char).collect();
    let text = text.trim_matches([' ', '\0']);
    if text.is_empty() {
        return DbfValue::Null;
    }
    match text.parse::<f64>() {
        Ok(n) if n.is_finite() => DbfValue::Number(n),
        _ => DbfValue::Null,
    }
}

/// `D` fields are 8 ASCII digits, `YYYYMMDD`; blank or nonsense reads as an empty date.
fn parse_date(raw: &[u8]) -> DbfValue {
    let Some(b) = raw.get(..8) else {
        return DbfValue::Date(None);
    };
    if !b.iter().all(|c| c.is_ascii_digit()) {
        return DbfValue::Date(None);
    }
    let num = |s: &[u8]| s.iter().fold(0i32, |a, &c| a * 10 + (c - b'0') as i32);
    let (y, m, d) = (num(&b[0..4]), num(&b[4..6]) as u32, num(&b[6..8]) as u32);
    if !is_valid_date(y, m, d) {
        return DbfValue::Date(None);
    }
    DbfValue::Date(Some(days_from_civil(y, m, d)))
}

/// `T`/`@` fields are two little-endian 4-byte integers: a Julian day and milliseconds past
/// midnight. A zero Julian day is the empty datetime.
fn parse_datetime(raw: &[u8]) -> DbfValue {
    let Some(b) = raw.get(..8) else {
        return DbfValue::DateTime(None);
    };
    let julian = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let millis = i32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    if julian == 0 {
        return DbfValue::DateTime(None);
    }
    let secs = (julian - JULIAN_EPOCH) as f64 * 86_400.0 + millis as f64 / 1000.0;
    DbfValue::DateTime(Some(secs))
}

/// Whether a memo block holds something other than text.
///
/// Text memos end with the NUL that VFP writes after them and hold nothing below a tab otherwise;
/// a compound file or a block of p-code is full of such bytes from its first one.
fn looks_binary(bytes: &[u8]) -> bool {
    let text = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    text.iter().any(|&b| b < 0x09)
}

/// A memo pointer is a 4-byte little-endian block number in FoxPro tables and 10 ASCII digits in
/// dBASE III ones. Block 0, a missing memo file and an out-of-range block all read as `Null`.
fn read_memo(raw: &[u8], codepage: Option<u16>, memo: &mut impl FnMut(u32) -> Option<OwnedMemoBlock>) -> DbfValue {
    let Some(block) = memo_pointer(raw) else {
        return DbfValue::Null;
    };
    match memo(block) {
        // VFP counts the NUL terminator it writes after memo text in the block length.
        Some(OwnedMemoBlock::Text(bytes)) if !looks_binary(&bytes) => {
            DbfValue::Memo(encoding::decode(&bytes, codepage).trim_end_matches('\0').to_string())
        }
        // A memo marked as text that is not text: an OLE control's persisted state and a form's
        // compiled p-code are both written that way. Decoding those through a code page would
        // lose the bytes, and the bytes are the whole content.
        Some(OwnedMemoBlock::Text(bytes)) => DbfValue::Bytes(bytes.to_vec()),
        Some(OwnedMemoBlock::Binary(bytes)) => DbfValue::Bytes(bytes),
        None => DbfValue::Null,
    }
}
