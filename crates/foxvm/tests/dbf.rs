//! DBF/FPT reader tests.
//!
//! The `.pjx`/`.PJT` and `.DBF` fixtures under `tests/fixtures/` are copies of the Visual FoxPro 8
//! sample projects, kept in-tree so the suite never reads outside the repository.

use foxvm::dbf::{DbfValue, read_table};
use foxvm::value::days_from_civil;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display()))
}

// ---- real Visual FoxPro files ----------------------------------------------------------------

#[test]
fn formsui_pjx_header_and_fields() {
    let table = read_table(&fixture("formsui.pjx"), Some(&fixture("formsui.PJT"))).expect("formsui.pjx");

    assert_eq!(table.version, 0x30, "Visual FoxPro table");
    assert_eq!(table.codepage, Some(1252));
    assert_eq!(table.records.len(), 7);
    assert!(table.has_memo());

    let got: Vec<(String, char, u8, u8)> =
        table.fields.iter().map(|f| (f.name.clone(), f.kind, f.length, f.decimals)).collect();
    let want: Vec<(&str, char, u8, u8)> = vec![
        ("NAME", 'M', 4, 0),
        ("TYPE", 'C', 1, 0),
        ("ID", 'N', 10, 0),
        ("TIMESTAMP", 'N', 10, 0),
        ("OUTFILE", 'M', 4, 0),
        ("HOMEDIR", 'M', 4, 0),
        ("EXCLUDE", 'L', 1, 0),
        ("MAINPROG", 'L', 1, 0),
        ("SAVECODE", 'L', 1, 0),
        ("DEBUG", 'L', 1, 0),
        ("ENCRYPT", 'L', 1, 0),
        ("NOLOGO", 'L', 1, 0),
        ("CMNTSTYLE", 'N', 1, 0),
        ("OBJREV", 'N', 5, 0),
        ("DEVINFO", 'M', 4, 0),
        ("SYMBOLS", 'M', 4, 0),
        ("OBJECT", 'M', 4, 0),
        ("CKVAL", 'N', 6, 0),
        ("CPID", 'N', 5, 0),
        ("OSTYPE", 'C', 4, 0),
        ("OSCREATOR", 'C', 4, 0),
        ("COMMENTS", 'M', 4, 0),
        ("RESERVED1", 'M', 4, 0),
        ("RESERVED2", 'M', 4, 0),
        ("SCCDATA", 'M', 4, 0),
        ("LOCAL", 'L', 1, 0),
        ("KEY", 'C', 32, 0),
        ("USER", 'M', 4, 0),
    ];
    let want: Vec<(String, char, u8, u8)> = want.into_iter().map(|(n, k, l, d)| (n.to_string(), k, l, d)).collect();
    assert_eq!(got, want);

    // Every record has exactly one value per field, and a single-character TYPE.
    for rec in &table.records {
        assert_eq!(rec.values.len(), table.fields.len());
        let kind = rec.get(&table, "TYPE").expect("TYPE").as_text();
        assert_eq!(kind.chars().count(), 1, "TYPE should be one character, got {kind:?}");
    }

    // NAME is a memo holding a readable file name, and at least one of them is a .prg.
    let names: Vec<String> =
        table.records.iter().map(|r| r.get(&table, "NAME").unwrap().as_text().to_ascii_lowercase()).collect();
    for name in &names {
        assert!(!name.is_empty(), "every NAME memo should decode to something");
        assert!(name.contains('.'), "NAME {name:?} should look like a file name");
        assert!(name.is_ascii(), "NAME {name:?} should decode to ASCII in this sample");
    }
    assert!(names.iter().any(|n| n.ends_with(".prg")), "the project should contain a .prg");
    assert!(names.iter().any(|n| n.ends_with(".pjx")), "record 0 names the project itself");
}

#[test]
fn formsui_pjx_has_one_main_program() {
    let table = read_table(&fixture("formsui.pjx"), Some(&fixture("formsui.PJT"))).expect("formsui.pjx");

    let mains: Vec<&str> = table
        .records
        .iter()
        .filter(|r| !r.deleted && r.get(&table, "MAINPROG").is_some_and(DbfValue::as_bool))
        .map(|r| r.get(&table, "NAME").unwrap().as_text())
        .collect();

    assert_eq!(mains.len(), 1, "exactly one record is the main program, got {mains:?}");
    assert!(mains[0].to_ascii_lowercase().ends_with(".prg"), "the main program should be a .prg, got {:?}", mains[0]);
}

#[test]
fn formsui_pjx_first_record_is_the_project_header() {
    let table = read_table(&fixture("formsui.pjx"), Some(&fixture("formsui.PJT"))).expect("formsui.pjx");
    let rec = &table.records[0];

    assert!(!rec.deleted);
    assert_eq!(rec.get(&table, "TYPE").unwrap().as_text(), "H", "record 0 is the project header");
    assert!(rec.get(&table, "NAME").unwrap().as_text().to_ascii_uppercase().ends_with("FORMSUI.PJX"));
    assert!(rec.get(&table, "HOMEDIR").unwrap().as_text().to_ascii_uppercase().contains("FORMSUI"));
    // Numeric fields decode to numbers, logicals to booleans, unset memos to Null.
    assert!(matches!(rec.get(&table, "OBJREV").unwrap(), DbfValue::Number(_)));
    assert!(matches!(rec.get(&table, "DEBUG").unwrap(), DbfValue::Logical(_)));
    assert!(rec.get(&table, "USER").unwrap().is_null(), "the USER memo is empty in the samples");
    // The header record is never the main program.
    assert!(!rec.get(&table, "MAINPROG").unwrap().as_bool());
}

#[test]
fn formsui_pjx_program_records_carry_types_and_ids() {
    let table = read_table(&fixture("formsui.pjx"), Some(&fixture("formsui.PJT"))).expect("formsui.pjx");

    let programs: Vec<&foxvm::dbf::DbfRecord> =
        table.records.iter().filter(|r| r.get(&table, "TYPE").unwrap().as_text() == "P").collect();
    assert_eq!(programs.len(), 5, "the FormsUI sample has five .prg files");
    for rec in programs {
        assert!(rec.get(&table, "NAME").unwrap().as_text().to_ascii_lowercase().ends_with(".prg"));
        assert!(rec.get(&table, "ID").unwrap().as_f64().unwrap_or(0.0) > 0.0, "ID is a positive number");
        assert_eq!(rec.get(&table, "CPID").unwrap().as_f64(), Some(1252.0), "CPID is the code page");
    }
}

#[test]
fn ai_table_dbf_round_trips_field_types() {
    let table = read_table(&fixture("AI_Table.DBF"), None).expect("AI_Table.DBF");

    assert_eq!(table.version, 0x30);
    assert_eq!(table.codepage, Some(1252));
    assert!(!table.has_memo());
    assert_eq!(table.fields.len(), 2);
    assert_eq!(table.fields[0].name, "IID");
    assert_eq!(table.fields[0].kind, 'I');
    assert_eq!(table.fields[0].length, 4);
    assert_eq!(table.fields[1].name, "CUSTNAME");
    assert_eq!(table.fields[1].kind, 'C');
    assert_eq!(table.fields[1].length, 30);

    assert_eq!(table.records.len(), 6);
    for rec in &table.records {
        let iid = rec.get(&table, "IID").expect("IID");
        assert!(matches!(iid, DbfValue::Number(_)), "an I field decodes to a number, got {iid:?}");
        let name = rec.get(&table, "CUSTNAME").expect("CUSTNAME");
        assert!(matches!(name, DbfValue::Text(_)), "a C field decodes to text, got {name:?}");
        assert!(!name.as_text().ends_with(' '), "trailing padding is trimmed");
    }
    // The sample adds the autoincrement column after three rows already exist, so the first three
    // carry generated keys and the rest keep the integer default of 0.
    let ids: Vec<f64> = table.records.iter().map(|r| r.get(&table, "IID").unwrap().as_f64().unwrap()).collect();
    assert_eq!(ids, vec![1.0, 2.0, 3.0, 0.0, 0.0, 0.0]);
    let names: Vec<&str> = table.records.iter().map(|r| r.get(&table, "CUSTNAME").unwrap().as_text()).collect();
    assert_eq!(names, vec!["Jane Smith", "John Doe", "Greg Jones", "Jay Lewis", "Steve Appleton", "Ken Garvy"]);
}

#[test]
fn other_sample_projects_parse() {
    let grid = read_table(&fixture("Grid.pjx"), Some(&fixture("Grid.PJT"))).expect("Grid.pjx");
    assert_eq!(grid.version, 0x30);
    assert!(grid.records.len() >= 8);
    assert!(
        grid.records.iter().any(|r| r.get(&grid, "NAME").unwrap().as_text().to_ascii_lowercase().ends_with(".prg"))
    );

    // No memo file: the table still parses, memo fields simply read as Null.
    let trycatch = read_table(&fixture("TryCatch.pjx"), None).expect("TryCatch.pjx");
    assert_eq!(trycatch.version, 0x30);
    assert!(trycatch.records.iter().all(|r| r.get(&trycatch, "NAME").unwrap().is_null()));
    assert!(trycatch.records.iter().any(|r| r.get(&trycatch, "TYPE").unwrap().as_text() == "P"));
}

#[test]
fn field_lookup_is_case_insensitive() {
    let table = read_table(&fixture("AI_Table.DBF"), None).expect("AI_Table.DBF");
    assert_eq!(table.field_index("custname"), Some(1));
    assert_eq!(table.field_index("CustName"), Some(1));
    assert_eq!(table.field_index("nope"), None);
    assert!(table.records[0].get(&table, "iid").is_some());
    assert!(table.records[0].get(&table, "missing").is_none());
}

// ---- synthetic tables --------------------------------------------------------------------------

/// Builds a table byte-for-byte. `fields` are `(name, kind, length, decimals)`; `records` are the
/// already-encoded field bytes of each row, prefixed here with the deletion flag.
struct Builder {
    version: u8,
    language: u8,
    fields: Vec<(&'static str, u8, u8, u8)>,
    backlink: usize,
}

impl Builder {
    fn vfp() -> Builder {
        Builder { version: 0x30, language: 0x03, fields: Vec::new(), backlink: 263 }
    }

    fn dbase3() -> Builder {
        Builder { version: 0x83, language: 0x00, fields: Vec::new(), backlink: 0 }
    }

    fn field(mut self, name: &'static str, kind: u8, length: u8, decimals: u8) -> Builder {
        self.fields.push((name, kind, length, decimals));
        self
    }

    fn record_len(&self) -> usize {
        1 + self.fields.iter().map(|f| f.2 as usize).sum::<usize>()
    }

    fn build(&self, records: &[(bool, Vec<u8>)]) -> Vec<u8> {
        let record_len = self.record_len();
        let header_len = 32 + self.fields.len() * 32 + 1 + self.backlink;
        let mut out = vec![0u8; header_len];
        out[0] = self.version;
        out[1] = 104; // 2004
        out[2] = 5;
        out[3] = 11;
        out[4..8].copy_from_slice(&(records.len() as u32).to_le_bytes());
        out[8..10].copy_from_slice(&(header_len as u16).to_le_bytes());
        out[10..12].copy_from_slice(&(record_len as u16).to_le_bytes());
        out[29] = self.language;
        let mut pos = 32;
        let mut disp = 1u32;
        for &(name, kind, length, decimals) in &self.fields {
            out[pos..pos + name.len()].copy_from_slice(name.as_bytes());
            out[pos + 11] = kind;
            out[pos + 12..pos + 16].copy_from_slice(&disp.to_le_bytes());
            out[pos + 16] = length;
            out[pos + 17] = decimals;
            disp += length as u32;
            pos += 32;
        }
        out[pos] = 0x0D;
        for (deleted, body) in records {
            assert_eq!(body.len() + 1, record_len, "record body has the wrong length");
            out.push(if *deleted { 0x2A } else { 0x20 });
            out.extend_from_slice(body);
        }
        out.push(0x1A);
        out
    }
}

/// A memo file with a 512-byte header and the given text/binary blocks, returning the file and the
/// block number of each entry.
fn build_memo(block_size: usize, entries: &[(u32, &[u8])]) -> (Vec<u8>, Vec<u32>) {
    let mut data = vec![0u8; 512];
    data[6..8].copy_from_slice(&(block_size as u16).to_be_bytes());
    let mut blocks = Vec::new();
    for (kind, payload) in entries {
        // Blocks are addressed as block_number * block_size from the start of the file.
        while !data.len().is_multiple_of(block_size) {
            data.push(0);
        }
        blocks.push((data.len() / block_size) as u32);
        data.extend_from_slice(&kind.to_be_bytes());
        data.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        data.extend_from_slice(payload);
    }
    while !data.len().is_multiple_of(block_size) {
        data.push(0);
    }
    let next_free = (data.len() / block_size) as u32;
    data[0..4].copy_from_slice(&next_free.to_be_bytes());
    (data, blocks)
}

fn ascii(text: &str, width: usize) -> Vec<u8> {
    let mut v = text.as_bytes().to_vec();
    v.resize(width, b' ');
    v
}

fn right(text: &str, width: usize) -> Vec<u8> {
    let mut v = vec![b' '; width.saturating_sub(text.len())];
    v.extend_from_slice(text.as_bytes());
    v.truncate(width);
    v
}

#[test]
fn every_field_type_decodes() {
    let b = Builder::vfp()
        .field("CH", b'C', 10, 0)
        .field("NUM", b'N', 8, 2)
        .field("FLT", b'F', 8, 3)
        .field("INT", b'I', 4, 0)
        .field("DBL", b'B', 8, 0)
        .field("CUR", b'Y', 8, 4)
        .field("YES", b'L', 1, 0)
        .field("NO", b'L', 1, 0)
        .field("MAYBE", b'L', 1, 0)
        .field("WHEN", b'D', 8, 0)
        .field("NODATE", b'D', 8, 0)
        .field("STAMP", b'T', 8, 0)
        .field("NOSTAMP", b'T', 8, 0)
        .field("VAR", b'V', 6, 0)
        .field("WEIRD", b'0', 2, 0);

    let mut body = Vec::new();
    body.extend(ascii("  hi  ", 10)); // leading spaces kept, trailing trimmed
    body.extend(right("-12.50", 8));
    body.extend(right("0.125", 8));
    body.extend((-70_000i32).to_le_bytes());
    body.extend(2.5f64.to_le_bytes());
    body.extend(123_4567i64.to_le_bytes()); // 123.4567 in currency units
    body.push(b'T');
    body.push(b'F');
    body.push(b'?');
    body.extend(b"20040511");
    body.extend(b"        ");
    // 2004-05-11 13:05:06 -> Julian 2453137, 47106000 ms
    body.extend(2_453_137i32.to_le_bytes());
    body.extend(47_106_000i32.to_le_bytes());
    body.extend([0u8; 8]); // an all-zero datetime is the empty one
    body.extend(b"\x00\x01\xFF\xFE\x7F\x80");
    body.extend(b"\xAB\xCD");

    let bytes = b.build(&[(false, body)]);
    let table = read_table(&bytes, None).expect("synthetic table");
    let rec = &table.records[0];
    let get = |name: &str| rec.get(&table, name).unwrap_or_else(|| panic!("field {name}"));

    assert_eq!(get("CH"), &DbfValue::Text("  hi".into()));
    assert_eq!(get("NUM"), &DbfValue::Number(-12.5));
    assert_eq!(get("FLT"), &DbfValue::Number(0.125));
    assert_eq!(get("INT"), &DbfValue::Number(-70_000.0));
    assert_eq!(get("DBL"), &DbfValue::Number(2.5));
    // a Y field is money, kept in whole ten-thousandths as the file keeps it
    assert_eq!(get("CUR"), &DbfValue::Currency(1_234_567));
    assert_eq!(get("YES"), &DbfValue::Logical(true));
    assert_eq!(get("NO"), &DbfValue::Logical(false));
    assert_eq!(get("MAYBE"), &DbfValue::Null, "'?' is an unset logical");
    assert_eq!(get("WHEN"), &DbfValue::Date(Some(days_from_civil(2004, 5, 11))));
    assert_eq!(get("NODATE"), &DbfValue::Date(None));
    let expect_secs = days_from_civil(2004, 5, 11) as f64 * 86_400.0 + 13.0 * 3600.0 + 5.0 * 60.0 + 6.0;
    assert_eq!(get("STAMP"), &DbfValue::DateTime(Some(expect_secs)));
    assert_eq!(get("NOSTAMP"), &DbfValue::DateTime(None));
    assert_eq!(get("VAR"), &DbfValue::Bytes(vec![0x00, 0x01, 0xFF, 0xFE, 0x7F, 0x80]));
    assert_eq!(get("WEIRD"), &DbfValue::Bytes(vec![0xAB, 0xCD]), "unknown types come back raw");

    // The helpers never panic and convert sensibly.
    assert_eq!(get("CH").as_f64(), None);
    assert_eq!(get("NUM").as_f64(), Some(-12.5));
    assert!(get("YES").as_bool());
    assert!(!get("MAYBE").as_bool());
    assert_eq!(get("INT").as_text(), "");
    assert!(get("MAYBE").is_null());
}

#[test]
fn empty_numeric_and_overflow_read_as_null() {
    let b = Builder::vfp().field("A", b'N', 6, 2).field("B", b'N', 6, 2).field("C", b'N', 6, 2);
    let mut body = Vec::new();
    body.extend(b"      "); // blank
    body.extend(b"******"); // FoxPro's numeric overflow marker
    body.extend(right("42", 6));
    let table = read_table(&b.build(&[(false, body)]), None).expect("table");
    let rec = &table.records[0];
    assert_eq!(rec.get(&table, "A").unwrap(), &DbfValue::Null);
    assert_eq!(rec.get(&table, "B").unwrap(), &DbfValue::Null);
    assert_eq!(rec.get(&table, "C").unwrap(), &DbfValue::Number(42.0));
}

#[test]
fn deleted_records_are_kept_and_flagged() {
    let b = Builder::vfp().field("NAME", b'C', 4, 0);
    let bytes = b.build(&[(false, ascii("live", 4)), (true, ascii("gone", 4)), (false, ascii("also", 4))]);
    let table = read_table(&bytes, None).expect("table");

    assert_eq!(table.records.len(), 3, "deleted records are kept");
    assert_eq!(table.records.iter().map(|r| r.deleted).collect::<Vec<_>>(), vec![false, true, false]);
    assert_eq!(table.records[1].get(&table, "NAME").unwrap().as_text(), "gone", "and are still readable");
    assert_eq!(table.records.iter().filter(|r| !r.deleted).count(), 2);
}

#[test]
fn dbase3_ten_digit_memo_pointer() {
    let (memo, blocks) = build_memo(512, &[(1, b"the quick brown fox")]);
    let b = Builder::dbase3().field("TITLE", b'C', 8, 0).field("BODY", b'M', 10, 0);

    let mut first = ascii("first", 8);
    first.extend(right(&blocks[0].to_string(), 10));
    let mut empty = ascii("second", 8);
    empty.extend(b"          "); // an all-blank pointer is an empty memo

    let table = read_table(&b.build(&[(false, first), (false, empty)]), Some(&memo)).expect("dBASE III table");
    assert_eq!(table.version, 0x83);
    assert_eq!(table.codepage, None, "language id 0 records no code page");
    assert!(table.has_memo());
    assert_eq!(table.records[0].get(&table, "BODY").unwrap(), &DbfValue::Memo("the quick brown fox".into()));
    assert_eq!(table.records[1].get(&table, "BODY").unwrap(), &DbfValue::Null);
}

#[test]
fn memo_blocks_text_binary_and_empty() {
    // The text payload is in the table code page (1252), not UTF-8.
    let (memo, blocks) = build_memo(64, &[(1, &[b'c', b'a', b'f', 0xE9]), (2, &[0u8, 1, 2, 3, 250])]);
    let b = Builder::vfp().field("TXT", b'M', 4, 0).field("PIC", b'G', 4, 0).field("NONE", b'M', 4, 0);

    let mut body = Vec::new();
    body.extend(blocks[0].to_le_bytes());
    body.extend(blocks[1].to_le_bytes());
    body.extend(0u32.to_le_bytes()); // block 0 means "no memo"

    let table = read_table(&b.build(&[(false, body)]), Some(&memo)).expect("table");
    let rec = &table.records[0];
    assert_eq!(rec.get(&table, "TXT").unwrap(), &DbfValue::Memo("caf\u{e9}".into()), "decoded as cp1252");
    assert_eq!(rec.get(&table, "PIC").unwrap(), &DbfValue::Bytes(vec![0, 1, 2, 3, 250]));
    assert_eq!(rec.get(&table, "NONE").unwrap(), &DbfValue::Null, "block 0 is an empty memo");
}

#[test]
fn memo_pointer_past_the_end_of_the_memo_file_is_null() {
    let (memo, blocks) = build_memo(64, &[(1, b"present")]);
    let b = Builder::vfp().field("OK", b'M', 4, 0).field("GONE", b'M', 4, 0).field("HUGE", b'M', 4, 0);

    let mut body = Vec::new();
    body.extend(blocks[0].to_le_bytes());
    body.extend(9_999u32.to_le_bytes()); // a block that does not exist
    body.extend(u32::MAX.to_le_bytes()); // and one far past the end

    let table = read_table(&b.build(&[(false, body)]), Some(&memo)).expect("table");
    let rec = &table.records[0];
    assert_eq!(rec.get(&table, "OK").unwrap(), &DbfValue::Memo("present".into()));
    assert_eq!(rec.get(&table, "GONE").unwrap(), &DbfValue::Null);
    assert_eq!(rec.get(&table, "HUGE").unwrap(), &DbfValue::Null);
}

#[test]
fn memo_block_length_past_the_end_of_the_memo_file_is_null() {
    let (mut memo, blocks) = build_memo(64, &[(1, b"present")]);
    // Claim the block is far longer than the file.
    let at = blocks[0] as usize * 64;
    memo[at + 4..at + 8].copy_from_slice(&100_000u32.to_be_bytes());

    let b = Builder::vfp().field("TXT", b'M', 4, 0);
    let table = read_table(&b.build(&[(false, blocks[0].to_le_bytes().to_vec())]), Some(&memo)).expect("table");
    assert_eq!(table.records[0].get(&table, "TXT").unwrap(), &DbfValue::Null);
}

#[test]
fn zero_block_size_falls_back_to_512() {
    let (mut memo, blocks) = build_memo(512, &[(1, b"hello")]);
    memo[6..8].copy_from_slice(&0u16.to_be_bytes());
    let b = Builder::vfp().field("TXT", b'M', 4, 0);
    let table = read_table(&b.build(&[(false, blocks[0].to_le_bytes().to_vec())]), Some(&memo)).expect("table");
    assert_eq!(table.records[0].get(&table, "TXT").unwrap(), &DbfValue::Memo("hello".into()));
}

// ---- malformed input ---------------------------------------------------------------------------

#[test]
fn a_file_too_short_to_hold_a_header_errors() {
    let err = read_table(&[], None).unwrap_err();
    assert!(err.message.contains("not a DBF file"), "{}", err.message);
    let err = read_table(&[0x30; 16], None).unwrap_err();
    assert!(err.message.contains("not a DBF file"), "{}", err.message);
}

#[test]
fn a_truncated_record_area_errors() {
    let b = Builder::vfp().field("NAME", b'C', 20, 0);
    let full = b.build(&[(false, ascii("one", 20)), (false, ascii("two", 20)), (false, ascii("three", 20))]);
    read_table(&full, None).expect("the whole file parses");

    // Cut the last record and a half away.
    let cut = full.len() - 32;
    let err = read_table(&full[..cut], None).unwrap_err();
    assert!(err.message.contains("truncated"), "{}", err.message);

    // Cutting inside the header is caught too.
    let err = read_table(&full[..40], None).unwrap_err();
    assert!(err.message.contains("runs past the end"), "{}", err.message);
}

#[test]
fn a_bogus_header_length_errors() {
    let b = Builder::vfp().field("NAME", b'C', 20, 0);
    let good = b.build(&[(false, ascii("one", 20))]);

    let mut huge = good.clone();
    huge[8..10].copy_from_slice(&60_000u16.to_le_bytes());
    let err = read_table(&huge, None).unwrap_err();
    assert!(err.message.contains("runs past the end"), "{}", err.message);

    let mut tiny = good.clone();
    tiny[8..10].copy_from_slice(&12u16.to_le_bytes());
    let err = read_table(&tiny, None).unwrap_err();
    assert!(err.message.contains("too small for any field"), "{}", err.message);

    // A header long enough to look plausible but with no 0x0D terminator in it.
    let mut unterminated = good.clone();
    unterminated[32 + 11] = b'C';
    for byte in unterminated.iter_mut().skip(32).take(good.len().min(300) - 32) {
        if *byte == 0x0D {
            *byte = 0x00;
        }
    }
    let err = read_table(&unterminated, None).unwrap_err();
    assert!(err.message.contains("not terminated") || err.message.contains("past the end"), "{}", err.message);
}

#[test]
fn a_zero_record_length_errors() {
    let b = Builder::vfp().field("NAME", b'C', 20, 0);
    let mut bytes = b.build(&[(false, ascii("one", 20))]);
    bytes[10..12].copy_from_slice(&0u16.to_le_bytes());
    let err = read_table(&bytes, None).unwrap_err();
    assert!(err.message.contains("record length is 0"), "{}", err.message);
}

#[test]
fn a_record_length_smaller_than_the_fields_errors() {
    let b = Builder::vfp().field("A", b'C', 20, 0).field("B", b'C', 20, 0);
    let mut bytes = b.build(&[(false, ascii("one", 40))]);
    bytes[10..12].copy_from_slice(&8u16.to_le_bytes());
    let err = read_table(&bytes, None).unwrap_err();
    assert!(err.message.contains("too small for the 2 fields"), "{}", err.message);
}

#[test]
fn a_memo_file_too_short_to_hold_a_header_errors() {
    let b = Builder::vfp().field("TXT", b'M', 4, 0);
    let bytes = b.build(&[(false, 1u32.to_le_bytes().to_vec())]);
    let err = read_table(&bytes, Some(&[0u8; 4])).unwrap_err();
    assert!(err.message.contains("memo file is too short"), "{}", err.message);
}

#[test]
fn a_table_with_no_fields_errors() {
    let mut bytes = vec![0u8; 33 + 8];
    bytes[0] = 0x30;
    bytes[8..10].copy_from_slice(&33u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&8u16.to_le_bytes());
    bytes[32] = 0x0D;
    let err = read_table(&bytes, None).unwrap_err();
    assert!(err.message.contains("no fields"), "{}", err.message);
}

// ---- code pages ---------------------------------------------------------------------------------

#[test]
fn code_pages_decode_high_bytes() {
    let build = |language: u8| {
        let mut b = Builder::vfp().field("TXT", b'C', 4, 0);
        b.language = language;
        b.build(&[(false, vec![0xE9, 0x82, 0xFF, b' '])])
    };

    let win = read_table(&build(0x03), None).expect("1252");
    assert_eq!(win.codepage, Some(1252));
    assert_eq!(win.records[0].get(&win, "TXT").unwrap().as_text(), "\u{e9}\u{201a}\u{ff}");

    let dos = read_table(&build(0x01), None).expect("437");
    assert_eq!(dos.codepage, Some(437));
    assert_eq!(dos.records[0].get(&dos, "TXT").unwrap().as_text(), "\u{398}\u{e9}\u{a0}");

    let latin = read_table(&build(0x02), None).expect("850");
    assert_eq!(latin.codepage, Some(850));
    assert_eq!(latin.records[0].get(&latin, "TXT").unwrap().as_text(), "\u{da}\u{e9}\u{a0}");

    // An unimplemented page falls back to 1252 rather than failing.
    let cee = read_table(&build(0xC8), None).expect("1250");
    assert_eq!(cee.codepage, Some(1250));
    assert_eq!(cee.records[0].get(&cee, "TXT").unwrap().as_text().chars().count(), 3);

    // A byte with no mapping in 1252 becomes U+FFFD instead of failing.
    let mut b = Builder::vfp().field("TXT", b'C', 1, 0);
    b.language = 0x03;
    let odd = read_table(&b.build(&[(false, vec![0x81])]), None).expect("1252");
    assert_eq!(odd.records[0].get(&odd, "TXT").unwrap().as_text(), "\u{fffd}");
}
