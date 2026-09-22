//! The part of a table that is known from its header alone.
//!
//! [`read_table`](super::read_table) takes a whole file and is right for the small DBF-based
//! design formats. A data table is not small, so the cursor engine reads the header once and then
//! asks the host for pages of record bytes; both paths decode records through this module, so
//! there is one field layout and one set of type rules.

use super::encoding;
use super::{DbfError, DbfField};

pub(super) const FIELD_ARRAY_START: usize = 32;
pub(super) const FIELD_DESC_LEN: usize = 32;
pub(super) const FIELD_TERMINATOR: u8 = 0x0D;
pub const FLAG_DELETED: u8 = b'*';

/// The hidden field a Visual FoxPro table keeps its null bits in.
///
/// It is a field like any other in the file - name `_NullFlags`, type `0`, flagged system and
/// binary - and it is the last one, one byte per eight columns that accept a null. Nothing in
/// the language ever sees it: `FCOUNT()` does not count it and `AFIELDS()` does not list it, so
/// it is taken out of the field list here and kept as a run of bytes instead.
pub const NULL_FLAGS_FIELD: &str = "_NullFlags";

/// Bit 0x02 of a field descriptor's flag byte: the column accepts `.NULL.`.
pub const FLAG_NULLABLE: u8 = 0x02;
/// Bit 0x01: the column is the table's own rather than the program's.
pub const FLAG_SYSTEM: u8 = 0x01;

/// Everything the header says, plus where each field sits inside a record.
#[derive(Debug, Clone, PartialEq)]
pub struct DbfHeader {
    /// The version byte at offset 0.
    pub version: u8,
    pub fields: Vec<DbfField>,
    /// `(offset, width)` per field, parallel to `fields`.
    pub layout: Vec<(usize, usize)>,
    /// Records the header claims the file holds.
    pub record_count: u64,
    /// Bytes before the first record.
    pub header_len: usize,
    /// Bytes per record, including the deletion flag.
    pub record_len: usize,
    /// The code page recorded in the header; `None` means 1252 was assumed.
    pub codepage: Option<u16>,
    /// Byte 28: a compound index of the table's own name sits beside it, and opening the table
    /// opens that too.
    pub has_index: bool,
    /// Bytes 1 to 3: the day the table was last written to, as days since 1970.
    pub last_update: Option<i32>,
    /// `(offset, width)` of the hidden `_NullFlags` field, when the table has one. It is not in
    /// `fields`, so nothing outside this module has to know it is there.
    pub null_flags: Option<(usize, usize)>,
}

impl DbfHeader {
    /// Byte offset of record `recno` (1-based) from the start of the file.
    ///
    /// `u64` throughout: Visual FoxPro stops at 2 GB per table because it computes this in signed
    /// 32-bit arithmetic, and doing better than that is the point of the host/VM split.
    pub fn record_offset(&self, recno: u64) -> u64 {
        self.header_len as u64 + recno.saturating_sub(1) * self.record_len as u64
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// True when the version byte or any field says the table needs a memo file.
    pub fn has_memo(&self) -> bool {
        matches!(self.version, 0x83 | 0x8B | 0xF5 | 0xFB)
            || self.fields.iter().any(|f| matches!(f.kind, 'M' | 'G' | 'P'))
    }

    /// Which bit of `_NullFlags` says whether field `index` is null.
    ///
    /// The bits go in field order, one for each column that accepts a null and none for the
    /// ones that do not: measured against a table Visual FoxPro wrote with `(a c(5), b n(10)
    /// NULL, c d NULL, e l NULL)`, a record holding three nulls ends in 0x07.
    pub fn null_bit(&self, index: usize) -> Option<usize> {
        if !self.fields.get(index)?.nullable {
            return None;
        }
        self.null_flags?;
        Some(self.fields[..index].iter().filter(|f| f.nullable).count())
    }

    /// Where in a record the bit for field `index` is: the byte and the mask.
    pub fn null_slot(&self, index: usize) -> Option<(usize, u8)> {
        let bit = self.null_bit(index)?;
        let (start, width) = self.null_flags?;
        let at = start + bit / 8;
        (at < start + width).then(|| (at, 1u8 << (bit % 8)))
    }

    /// Whether the record says field `index` holds `.NULL.`.
    pub fn is_null(&self, record: &[u8], index: usize) -> bool {
        let Some((at, mask)) = self.null_slot(index) else {
            return false;
        };
        record.get(at).is_some_and(|b| b & mask != 0)
    }
}

/// Says in a record whether the field the slot belongs to holds `.NULL.`.
pub fn set_null_slot(record: &mut [u8], slot: Option<(usize, u8)>, null: bool) {
    let Some((at, mask)) = slot else { return };
    if let Some(byte) = record.get_mut(at) {
        *byte = if null { *byte | mask } else { *byte & !mask };
    }
}

/// Reads the header from the first bytes of a table. Needs only `header_len` bytes, which is not
/// known until byte 8 has been read, so a caller streaming the file reads 32 bytes, then the rest.
pub fn read_header(dbf: &[u8]) -> Result<DbfHeader, DbfError> {
    if dbf.len() < FIELD_ARRAY_START {
        return Err(DbfError::new(format!("not a DBF file: only {} bytes", dbf.len())));
    }
    let version = dbf[0];
    let record_count = u32::from_le_bytes([dbf[4], dbf[5], dbf[6], dbf[7]]) as u64;
    let header_len = u16::from_le_bytes([dbf[8], dbf[9]]) as usize;
    let record_len = u16::from_le_bytes([dbf[10], dbf[11]]) as usize;
    let codepage = encoding::codepage_for_language_id(dbf[29]);
    let has_index = dbf[28] != 0;
    // the year is kept as an offset from 1900, which VFP writes for every year past 2000 too
    let last_update = (dbf[2] >= 1 && dbf[2] <= 12 && dbf[3] >= 1 && dbf[3] <= 31)
        .then(|| crate::value::days_from_civil(1900 + i32::from(dbf[1]), u32::from(dbf[2]), u32::from(dbf[3])));

    if header_len < FIELD_ARRAY_START + 1 {
        return Err(DbfError::new(format!("header length {header_len} is too small for any field")));
    }
    if header_len > dbf.len() {
        return Err(DbfError::new(format!(
            "header length {header_len} runs past the end of the {}-byte file",
            dbf.len()
        )));
    }
    if record_len < 1 {
        return Err(DbfError::new("record length is 0"));
    }

    let all = read_fields(dbf, header_len, codepage)?;
    let full = layout(&all, record_len)?;
    // the hidden flags field is taken out of the list before anything else sees it, and where it
    // sits is kept instead; it is laid out with the rest, so this cannot move the other fields
    let hidden = all.iter().position(|f| f.name.eq_ignore_ascii_case(NULL_FLAGS_FIELD));
    let null_flags = hidden.map(|i| full[i]);
    let keep = |i: usize| hidden != Some(i);
    let fields: Vec<DbfField> = all.into_iter().enumerate().filter(|(i, _)| keep(*i)).map(|(_, f)| f).collect();
    let layout: Vec<(usize, usize)> = full.into_iter().enumerate().filter(|(i, _)| keep(*i)).map(|(_, l)| l).collect();
    if fields.is_empty() {
        return Err(DbfError::new("the table has no fields"));
    }
    Ok(DbfHeader {
        version,
        fields,
        layout,
        record_count,
        header_len,
        record_len,
        codepage,
        has_index,
        last_update,
        null_flags,
    })
}

/// Parses the 32-byte field descriptors that follow the header, up to the `0x0D` terminator.
///
/// A VFP `0x30` table carries a 263-byte "backlink" (the path of the owning database) after the
/// terminator, so the descriptor array stops well before `header_len`.
fn read_fields(dbf: &[u8], header_len: usize, codepage: Option<u16>) -> Result<Vec<DbfField>, DbfError> {
    let mut fields = Vec::new();
    let mut pos = FIELD_ARRAY_START;
    loop {
        if pos >= header_len {
            return Err(DbfError::new("the field descriptors are not terminated by 0x0D"));
        }
        if dbf[pos] == FIELD_TERMINATOR {
            break;
        }
        let end = pos + FIELD_DESC_LEN;
        if end > header_len {
            return Err(DbfError::new("a field descriptor runs past the end of the header"));
        }
        let desc = &dbf[pos..end];
        let name_end = desc[..11].iter().position(|&b| b == 0).unwrap_or(11);
        let name = encoding::decode(&desc[..name_end], codepage).trim().to_string();
        let kind = desc[11] as char;
        // byte 18 flags the field, 0x0C being autoincrementing; 19..23 is what it takes next
        // and 23 how much it goes up by
        let autoincrementing = desc[18] & 0x0C == 0x0C;
        fields.push(DbfField {
            name,
            kind,
            length: desc[16],
            decimals: desc[17],
            autoinc_step: if autoincrementing { desc[23].max(1) } else { 0 },
            autoinc_next: if autoincrementing { u32::from_le_bytes([desc[19], desc[20], desc[21], desc[22]]) } else { 0 },
            nullable: desc[18] & FLAG_NULLABLE != 0,
        });
        pos = end;
    }
    if fields.is_empty() {
        return Err(DbfError::new("the table has no fields"));
    }
    Ok(fields)
}

/// `(offset, width)` per field. Fields are laid out end to end after the deletion flag.
fn layout(fields: &[DbfField], record_len: usize) -> Result<Vec<(usize, usize)>, DbfError> {
    for wide_char in [true, false] {
        let mut out = Vec::with_capacity(fields.len());
        let mut pos = 1usize;
        for f in fields {
            let width = f.width(wide_char);
            out.push((pos, width));
            pos += width;
        }
        if pos <= record_len {
            return Ok(out);
        }
        if !fields.iter().any(|f| f.kind == 'C' && f.decimals > 0) {
            break;
        }
    }
    let needed: usize = 1 + fields.iter().map(|f| f.width(false)).sum::<usize>();
    Err(DbfError::new(format!(
        "record length {record_len} is too small for the {} fields, which need {needed} bytes",
        fields.len()
    )))
}
