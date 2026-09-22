//! Memo fields: the text goes to the file beside the table and the record holds the block.

mod dbf_fixture;

use foxvm::mock_host::{MockHost, run_program};

fn run_on(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn a_memo_is_written_and_read_back() {
    let mut host = MockHost::new();
    let out = run_on(
        &mut host,
        r#"
CREATE TABLE notes (title C(10), body M)
APPEND BLANK
REPLACE title WITH "First", body WITH "The whole story, at length."
? ALLTRIM(title), body
GO TOP
? body
APPEND BLANK
REPLACE title WITH "Second", body WITH "Another one."
GO TOP
? ALLTRIM(body)
SKIP
? ALLTRIM(body), LEN(ALLTRIM(body))
"#,
    );
    assert_eq!(
        out,
        vec![
            "First The whole story, at length.", "The whole story, at length.", "The whole story, at length.", "Another one.         12",
        ]
    );
    assert!(host.memo_files.contains_key("NOTES.DBF"), "a memo file was written");
}

#[test]
fn a_memo_read_from_the_file_comes_back_after_the_table_is_reopened() {
    let mut host = MockHost::new();
    run_on(
        &mut host,
        r#"
CREATE TABLE notes (title C(10), body M)
APPEND BLANK
REPLACE title WITH "First", body WITH "Kept on disk."
"#,
    );
    let out = run_on(
        &mut host,
        r#"
USE notes.dbf
GO TOP
? ALLTRIM(title), ALLTRIM(body)
"#,
    );
    assert_eq!(out, vec!["First Kept on disk."]);
}
