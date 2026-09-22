//! `EXPORT` and `IMPORT`: the interchange files a table is written to, and the table a
//! spreadsheet is read into.
//!
//! The two fixtures were written by Visual FoxPro 9 itself, by the program in
//! `exports_what_visual_foxpro_would_have_written` run there, so the assertion is byte equality
//! with the real product rather than with the documentation.

use std::fs;
use std::path::PathBuf;

use foxvm::mock_host::{MockHost, run_program};

const SAMPLE: &str = r#"
CREATE TABLE goods FREE (code C(6), price N(9,2), ok L, born D)
INSERT INTO goods VALUES ("A100", 125.50, .T., {^2024-01-31})
INSERT INTO goods VALUES ("B200", -8, .F., {})
"#;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn run(src: &str) -> MockHost {
    let mut host = MockHost::new();
    match run_program(src, &mut host) {
        Ok(_) => host,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn written(host: &MockHost, name: &str) -> Vec<u8> {
    host.files
        .get(&name.to_ascii_uppercase())
        .cloned()
        .unwrap_or_else(|| panic!("nothing was written to {name}; files: {:?}", host.files.keys().collect::<Vec<_>>()))
}

#[test]
fn exports_what_visual_foxpro_would_have_written() {
    // the same program, run by vfp9.exe, produced the two fixtures
    let host = run(&format!("{SAMPLE}EXPORT TO out TYPE DIF\nEXPORT TO out TYPE SYLK\n"));

    assert_eq!(
        String::from_utf8_lossy(&written(&host, "out.dif")),
        String::from_utf8_lossy(&fixture("vfp9-export.dif")),
        "DIF"
    );
    // a SYLK file goes by the name it was given, with no extension of its own
    assert_eq!(
        String::from_utf8_lossy(&written(&host, "out")),
        String::from_utf8_lossy(&fixture("vfp9-export.slk")),
        "SYLK"
    );
}

#[test]
fn a_text_export_takes_the_extension_its_format_goes_by() {
    let host = run(&format!("{SAMPLE}COPY TO plain TYPE SDF\nCOPY TO listed TYPE CSV\n"));
    assert!(host.files.contains_key("PLAIN.TXT"), "{:?}", host.files.keys().collect::<Vec<_>>());
    assert!(host.files.contains_key("LISTED.CSV"), "{:?}", host.files.keys().collect::<Vec<_>>());
}

#[test]
fn a_memo_is_left_out_of_an_exported_sheet() {
    let host = run(
        "CREATE TABLE notes FREE (code C(4), note M)\nINSERT INTO notes VALUES (\"A100\", \"long\")\nEXPORT TO out TYPE DIF\n",
    );
    let text = String::from_utf8_lossy(&written(&host, "out.dif")).to_string();
    assert!(text.contains("\"code\""), "{text}");
    assert!(!text.contains("\"note\""), "the memo column is not written: {text}");
    // one vector, because the memo is not one of them
    assert!(text.contains("VECTORS\r0,1"), "{text}");
}

#[test]
fn import_of_a_format_nothing_reads_says_so_by_name() {
    let mut host = MockHost::new();
    let Err(e) = run_program("IMPORT FROM old.wk1 TYPE WK1", &mut host) else {
        panic!("WK1 imported")
    };
    assert!(e.message.contains("TYPE WK1 is not supported"), "{}", e.message);
}

#[test]
fn export_of_a_worksheet_says_what_to_use_instead() {
    let mut host = MockHost::new();
    let Err(e) = run_program("EXPORT TO book TYPE XL5", &mut host) else { panic!("XL5 exported") };
    assert!(e.message.contains("TYPE XL5 is not supported"), "{}", e.message);
    assert!(e.message.contains("COPY TO ... TYPE CSV"), "{}", e.message);
}

#[test]
fn imports_the_sheet_into_the_table_visual_foxpro_makes_of_it() {
    // the fixture is what VFP wrote with EXPORT TO sheet TYPE XL5, and what it read back was a
    // table of character columns named A, B, C, D - the sheet's own first row a record like the
    // rest - which is what this asserts
    let mut host = MockHost::new();
    host.files.insert("SHEET.XLS".into(), fixture("vfp9-export.xls"));
    let out = match run_program(
        "IMPORT FROM sheet.xls TYPE XL5\n? ALIAS(), RECCOUNT(), FCOUNT()\n? FIELD(1), FIELD(4)\n? ALLTRIM(a), ALLTRIM(d)\nGO 2\n? ALLTRIM(a), ALLTRIM(b)\n",
        &mut host,
    ) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    };
    assert_eq!(out[0], "SHEET          3          4");
    assert_eq!(out[1], "A D");
    assert_eq!(out[2], "code born", "the sheet's first row is the first record");
    // the cell as the workbook holds it: this one is text, and a cell that is a number
    // carrying a date format reads as the number, because the format is the sheet's and not
    // the cell's type
    assert_eq!(out[3], "A100 125.50");
    assert!(host.tables.contains_key("SHEET.DBF"), "{:?}", host.tables.keys().collect::<Vec<_>>());
}
