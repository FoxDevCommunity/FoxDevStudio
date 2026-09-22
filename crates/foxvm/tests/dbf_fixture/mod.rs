//! Visual FoxPro tables built byte for byte, for the tests that need a real one.
//!
//! Nothing here uses the reader it is testing against: the bytes are laid out from the format
//! description, so a test that passes says the reader agrees with the format rather than with
//! itself.

/// Builds a Visual FoxPro table byte for byte: `fields` are `(name, kind, width, decimals)` and
/// each record is the already-padded field text.
#[allow(dead_code)]
pub fn table(fields: &[(&str, u8, u8, u8)], records: &[&[&str]]) -> Vec<u8> {
    table_with_deleted(fields, records, &[])
}

/// The same, with the 1-based numbers of the records to mark deleted.
#[allow(dead_code)]
pub fn table_with_deleted(fields: &[(&str, u8, u8, u8)], records: &[&[&str]], deleted: &[usize]) -> Vec<u8> {
    let record_len: usize = 1 + fields.iter().map(|f| f.2 as usize).sum::<usize>();
    let header_len = 32 + fields.len() * 32 + 1;
    let mut out = vec![0u8; header_len];
    out[0] = 0x30; // Visual FoxPro
    out[4..8].copy_from_slice(&(records.len() as u32).to_le_bytes());
    out[8..10].copy_from_slice(&(header_len as u16).to_le_bytes());
    out[10..12].copy_from_slice(&(record_len as u16).to_le_bytes());

    let mut offset = 1usize;
    for (i, (name, kind, width, decimals)) in fields.iter().enumerate() {
        let at = 32 + i * 32;
        out[at..at + name.len()].copy_from_slice(name.as_bytes());
        out[at + 11] = *kind;
        out[at + 12..at + 16].copy_from_slice(&(offset as u32).to_le_bytes());
        out[at + 16] = *width;
        out[at + 17] = *decimals;
        offset += *width as usize;
    }
    out[header_len - 1] = 0x0D;

    for (i, record) in records.iter().enumerate() {
        out.push(if deleted.contains(&(i + 1)) { b'*' } else { b' ' });
        for ((_, kind, width, _), text) in fields.iter().zip(record.iter()) {
            let width = *width as usize;
            let mut cell = vec![b' '; width];
            let bytes = text.as_bytes();
            let take = bytes.len().min(width);
            // character fields pad on the right, numeric ones on the left, as VFP writes them
            let start = if *kind == b'C' { 0 } else { width - take };
            cell[start..start + take].copy_from_slice(&bytes[..take]);
            out.extend_from_slice(&cell);
        }
    }
    out.push(0x1A);
    out
}

// ---- report files ----

/// A ten-thousandth of an inch: what a report file measures in.
pub const INCH: f64 = 10_000.0;
/// The bar the designer draws between two bands, which takes up room in the layout.
pub const BAR: f64 = INCH / 4.8;
pub const LINE: f64 = INCH / 6.0;
pub const CHAR: f64 = INCH / 12.0;

/// One record of a report file: the type, the code, the place, and the memo text.
pub struct Row {
    pub objtype: i32,
    pub objcode: i32,
    pub vpos: f64,
    pub hpos: f64,
    pub height: f64,
    pub width: f64,
    pub expr: String,
    pub picture: String,
    pub print_when: String,
}

pub fn band(code: i32, height: f64) -> Row {
    Row { objtype: 9, objcode: code, vpos: 0.0, hpos: 0.0, height, width: 0.0, expr: String::new(), picture: String::new(), print_when: String::new() }
}

pub fn field(vpos: f64, col: f64, width: f64, expr: &str) -> Row {
    Row {
        objtype: 8,
        objcode: 0,
        vpos,
        hpos: col * CHAR,
        height: LINE,
        width: width * CHAR,
        expr: expr.to_string(),
        picture: String::new(),
        print_when: String::new(),
    }
}

/// The report table and the memo file beside it.
pub fn report_file(rows: &[Row]) -> (Vec<u8>, Vec<u8>) {
    // the memo file: a 512-byte header, then one block per piece of text
    let block = 64usize;
    let mut memo = vec![0u8; 512];
    memo[6..8].copy_from_slice(&(block as u16).to_be_bytes());
    let put = |text: &str, memo: &mut Vec<u8>| -> u32 {
        if text.is_empty() {
            return 0;
        }
        let at = memo.len() / block;
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&1u32.to_be_bytes());
        chunk.extend_from_slice(&(text.len() as u32).to_be_bytes());
        chunk.extend_from_slice(text.as_bytes());
        while chunk.len() % block != 0 {
            chunk.push(0);
        }
        memo.extend_from_slice(&chunk);
        at as u32
    };

    let fields: &[(&str, u8, u8, u8)] = &[
        ("OBJTYPE", b'N', 2, 0),
        ("OBJCODE", b'N', 3, 0),
        ("VPOS", b'N', 9, 3),
        ("HPOS", b'N', 9, 3),
        ("HEIGHT", b'N', 9, 3),
        ("WIDTH", b'N', 9, 3),
        ("EXPR", b'M', 4, 0),
        ("PICTURE", b'M', 4, 0),
        ("SUPEXPR", b'M', 4, 0),
    ];
    // the memo columns hold a block number as four bytes, so they are written as raw text here
    let mut records: Vec<Vec<String>> = Vec::new();
    for row in rows {
        let expr = put(&row.expr, &mut memo);
        let picture = put(&row.picture, &mut memo);
        let when = put(&row.print_when, &mut memo);
        let cell = |n: u32| String::from_utf8_lossy(&n.to_le_bytes()).to_string();
        records.push(vec![
            row.objtype.to_string(),
            row.objcode.to_string(),
            format!("{:.3}", row.vpos),
            format!("{:.3}", row.hpos),
            format!("{:.3}", row.height),
            format!("{:.3}", row.width),
            cell(expr),
            cell(picture),
            cell(when),
        ]);
    }
    // the memo pointers are bytes, not text, so they are written into the record by hand
    let text: Vec<Vec<&str>> = records.iter().map(|r| r.iter().map(String::as_str).collect()).collect();
    let refs: Vec<&[&str]> = text.iter().map(Vec::as_slice).collect();
    let mut frx = table(fields, &refs);
    let header_len = u16::from_le_bytes([frx[8], frx[9]]) as usize;
    let record_len = u16::from_le_bytes([frx[10], frx[11]]) as usize;
    let memo_at: usize = 1 + fields.iter().take_while(|f| f.0 != "EXPR").map(|f| f.2 as usize).sum::<usize>();
    for (i, row) in rows.iter().enumerate() {
        let start = header_len + i * record_len + memo_at;
        let expr = i_th_block(&memo, block, &row.expr);
        let picture = i_th_block(&memo, block, &row.picture);
        let when = i_th_block(&memo, block, &row.print_when);
        frx[start..start + 4].copy_from_slice(&expr.to_le_bytes());
        frx[start + 4..start + 8].copy_from_slice(&picture.to_le_bytes());
        frx[start + 8..start + 12].copy_from_slice(&when.to_le_bytes());
    }
    frx[0] = 0x30;
    (frx, memo)
}

/// Which block a piece of text was written into, found by looking for it again.
pub fn i_th_block(memo: &[u8], block: usize, text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let mut at = 512 / block;
    while at * block < memo.len() {
        let start = at * block;
        let len = u32::from_be_bytes([memo[start + 4], memo[start + 5], memo[start + 6], memo[start + 7]]) as usize;
        if &memo[start + 8..start + 8 + len] == text.as_bytes() {
            return at as u32;
        }
        at += (8 + len).div_ceil(block).max(1);
    }
    0
}

/// A report over a table of parts: a title, a page header and one detail line.
pub fn sample() -> (Vec<u8>, Vec<u8>) {
    let title_h = LINE * 2.0;
    let header_h = LINE * 2.0;
    let detail_h = LINE;
    let footer_h = LINE;
    let title_top = 0.0;
    let header_top = title_h + BAR;
    let detail_top = header_top + header_h + BAR;
    let footer_top = detail_top + detail_h + BAR;
    report_file(&[
        band(0, title_h),
        band(1, header_h),
        band(4, detail_h),
        band(7, footer_h),
        field(title_top, 10.0, 20.0, "\"Parts list\""),
        field(header_top, 0.0, 10.0, "\"Code\""),
        field(header_top, 12.0, 12.0, "\"Name\""),
        field(header_top, 26.0, 8.0, "\"Price\""),
        field(detail_top, 0.0, 10.0, "parts.code"),
        field(detail_top, 12.0, 12.0, "ALLTRIM(parts.name)"),
        field(detail_top, 26.0, 8.0, "parts.price"),
        field(footer_top, 0.0, 20.0, "\"Page \" + ALLTRIM(STR(_pageno))"),
    ])
}

