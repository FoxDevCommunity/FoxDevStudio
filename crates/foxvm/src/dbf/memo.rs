//! FoxPro memo files (`.fpt`, and the renamed variants `.pjt` / `.sct` / `.vct` / `.mnt`).
//!
//! Layout: a header block whose bytes 6..8 hold the block size as a big-endian `u16`, then
//! fixed-size blocks. A block address is `block_number * block_size` counted from the start of
//! the file, so with a block size below 512 the first usable block still sits past the header.
//! Each block opens with an 8-byte big-endian header: a type (1 = text, 2 = picture/object) and
//! the payload length.

use super::DbfError;

/// The payload of one memo block.
pub enum MemoBlock<'a> {
    /// Type 1: text in the table code page.
    Text(&'a [u8]),
    /// Any other type: opaque bytes (pictures, general fields, OLE blobs).
    Binary(&'a [u8]),
}

/// A borrowed, already-loaded memo file.
pub struct MemoFile<'a> {
    data: &'a [u8],
    block_size: usize,
}

impl<'a> MemoFile<'a> {
    /// Reads the memo header. Fails only when the file is too small to hold one.
    pub fn open(data: &'a [u8]) -> Result<MemoFile<'a>, DbfError> {
        if data.len() < 8 {
            return Err(DbfError::new(format!("memo file is too short: {} bytes", data.len())));
        }
        let raw = u16::from_be_bytes([data[6], data[7]]) as usize;
        let block_size = if raw == 0 { 512 } else { raw };
        Ok(MemoFile { data, block_size })
    }

    /// Block size in bytes, as recorded in the header.
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Returns block `number`, or `None` for block 0 (the "no memo" marker), a block that starts
    /// past the end of the file, or a block whose declared length runs past the end.
    pub fn block(&self, number: u32) -> Option<MemoBlock<'a>> {
        if number == 0 {
            return None;
        }
        let start = (number as usize).checked_mul(self.block_size)?;
        let head_end = start.checked_add(8)?;
        if head_end > self.data.len() {
            return None;
        }
        let head = &self.data[start..head_end];
        let kind = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
        let len = u32::from_be_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let end = head_end.checked_add(len)?;
        if end > self.data.len() {
            return None;
        }
        let payload = &self.data[head_end..end];
        Some(if kind == 1 { MemoBlock::Text(payload) } else { MemoBlock::Binary(payload) })
    }
}
