//! The data engine end to end: `USE` a real table's bytes, move the record pointer, read fields.
//!
//! The host here is [`MockHost`], which answers the data requests out of a whole file held in
//! memory, exactly what the Electron main process does against a file handle. Nothing in this
//! test decodes a record outside the VM, which is the split the engine is built on.

mod dbf_fixture;

use dbf_fixture::{table, table_with_deleted};
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table(
        &[("CUSTNO", b'C', 6, 0), ("NAME", b'C', 10, 0), ("AMOUNT", b'N', 8, 2)],
        &[
            &["A100", "Acme", "125.50"],
            &["B200", "Beta", "80.00"],
            &["C300", "Cirrus", "1000.25"],
        ],
    );
    let mut host = MockHost::new();
    host.tables.insert("CUSTOMER.DBF".into(), bytes);
    host
}

fn run(src: &str) -> Vec<String> {
    let mut host = host();
    match run_program(src, &mut host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn use_opens_a_table_and_reports_on_it() {
    let out = run(
        r#"
USE customer.dbf
? ALIAS()
? RECCOUNT()
? FCOUNT()
? FIELD(2)
? USED()
"#,
    );
    assert_eq!(out, vec!["CUSTOMER", "         3", "         3", "NAME", ".T."]);
}

#[test]
fn fields_of_the_current_record_read_as_names() {
    // the table opens at record 1, and a field wins over a memory variable of the same name
    let out = run(
        r#"
custno = "not this"
USE customer.dbf
? "[" + custno + "]"
? ALLTRIM(name)
? amount
? m.custno
"#,
    );
    // a character field keeps the blanks it is padded with on disk
    assert_eq!(out, vec!["[A100  ]", "Acme", "  125.50", "not this"]);
}

#[test]
fn go_and_skip_move_the_pointer() {
    let out = run(
        r#"
USE customer.dbf
GO 3
? RECNO() 
? ALLTRIM(name)
SKIP -1
? ALLTRIM(name)
GO TOP
? RECNO()
GO BOTTOM
? RECNO()
? EOF()
SKIP
? EOF()
? RECNO()
"#,
    );
    assert_eq!(out, vec!["         3", "Cirrus", "Beta", "         1", "         3", ".F.", ".T.", "         4"]);
}

#[test]
fn an_alias_qualifies_a_field_wherever_the_pointer_is() {
    let out = run(
        r#"
USE customer.dbf
GO 2
? ALLTRIM(customer.name)
SELECT 0
? ALLTRIM(customer.name)
? USED("customer")
"#,
    );
    assert_eq!(out, vec!["Beta", "Beta", ".T."]);
}

#[test]
fn closing_the_table_leaves_the_work_area_empty() {
    let out = run(
        r#"
USE customer.dbf
USE
? USED()
? ALIAS()
? EOF()
"#,
    );
    // a work area with no table in it is neither past the end nor before the start - measured
    assert_eq!(out, vec![".F.", "", ".F."]);
}

#[test]
fn close_lets_go_of_every_work_area() {
    let out = run(
        r#"
USE customer.dbf
SELECT 0
USE customer.dbf ALIAS second
CLOSE TABLES
? USED("customer")
? USED("second")
? ALIAS()
"#,
    );
    assert_eq!(out, vec![".F.", ".F.", ""]);
}

#[test]
fn close_of_something_this_runtime_does_not_have_is_a_warning_not_an_error() {
    // CLOSE INDEXES has nothing to close here, but a program full of them still has to run
    let out = run(
        r#"
USE customer.dbf
CLOSE INDEXES
? ALIAS()
"#,
    );
    assert_eq!(out, vec!["CUSTOMER"]);
}
#[test]
fn a_table_that_is_not_there_is_a_file_error() {
    let mut h = host();
    let Err(err) = run_program("USE nope.dbf", &mut h) else { panic!("missing table opened") };
    assert_eq!(err.code, 1);
    assert!(err.message.contains("nope.dbf"), "{}", err.message);
}

// ---- scanning and searching --------------------------------------------------------------------

/// Five customers in three states, with the third marked deleted.
fn scan_host() -> MockHost {
    let bytes = table_with_deleted(
        &[("CUSTNO", b'C', 6, 0), ("STATE", b'C', 2, 0), ("AMOUNT", b'N', 8, 2)],
        &[
            &["A100", "NY", "100.00"],
            &["B200", "CA", "200.00"],
            &["C300", "NY", "300.00"],
            &["D400", "TX", "400.00"],
            &["E500", "NY", "500.00"],
        ],
        &[3],
    );
    let mut host = MockHost::new();
    host.tables.insert("CUSTOMER.DBF".into(), bytes);
    host
}

fn scan_run(src: &str) -> Vec<String> {
    let mut host = scan_host();
    match run_program(src, &mut host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn scan_visits_every_record() {
    let out = scan_run(
        r#"
USE customer.dbf
LOCAL cAll
cAll = ""
SCAN
  cAll = cAll + ALLTRIM(custno) + " "
ENDSCAN
? cAll
? EOF()
"#,
    );
    assert_eq!(out, vec!["A100 B200 C300 D400 E500 ", ".T."]);
}

#[test]
fn scan_for_filters_and_exit_and_loop_work_inside_it() {
    let out = scan_run(
        r#"
USE customer.dbf
LOCAL cNy, n
cNy = ""
SCAN FOR state = "NY"
  cNy = cNy + ALLTRIM(custno) + " "
ENDSCAN
? cNy

n = 0
SCAN
  IF state = "CA"
    LOOP
  ENDIF
  n = n + 1
  IF n = 2
    EXIT
  ENDIF
ENDSCAN
? n
? ALLTRIM(custno)
"#,
    );
    // EXIT leaves the pointer where it was, as it does in Visual FoxPro
    assert_eq!(out, vec!["A100 C300 E500 ", "         2", "C300"]);
}

#[test]
fn scan_scopes_limit_how_many_records_are_visited() {
    let out = scan_run(
        r#"
USE customer.dbf
LOCAL c
c = ""
GO 2
SCAN NEXT 3
  c = c + ALLTRIM(custno) + " "
ENDSCAN
? c

c = ""
GO 4
SCAN REST
  c = c + ALLTRIM(custno) + " "
ENDSCAN
? c

c = ""
SCAN RECORD 2
  c = c + ALLTRIM(custno)
ENDSCAN
? c
"#,
    );
    assert_eq!(out, vec!["B200 C300 D400 ", "D400 E500 ", "B200"]);
}

#[test]
fn set_deleted_hides_deleted_records_from_every_movement() {
    let out = scan_run(
        r#"
USE customer.dbf
? DELETED()
GO 3
? DELETED()
SET DELETED ON
GO TOP
SKIP 2
? ALLTRIM(custno)
GO BOTTOM
SKIP -2
? ALLTRIM(custno)
LOCAL c
c = ""
SCAN
  c = c + ALLTRIM(custno) + " "
ENDSCAN
? c
SET DELETED OFF
GO 3
? ALLTRIM(custno)
"#,
    );
    // record 3 is deleted: SKIP 2 from the top lands on 4, not 3, and the scan leaves it out.
    // GO 3 still goes to record 3 - GOTO addresses a record, it does not search for one.
    assert_eq!(out, vec![".F.", ".T.", "D400", "B200", "A100 B200 D400 E500 ", "C300"]);
}

#[test]
fn locate_and_continue_walk_the_matches() {
    let out = scan_run(
        r#"
USE customer.dbf
LOCATE FOR state = "NY"
? FOUND()
? ALLTRIM(custno)
CONTINUE
? ALLTRIM(custno)
CONTINUE
? ALLTRIM(custno)
CONTINUE
? FOUND()
? EOF()

LOCATE FOR state = "ZZ"
? FOUND()
"#,
    );
    assert_eq!(out, vec![".T.", "A100", "C300", "E500", ".F.", ".T.", ".F."]);
}

#[test]
fn locate_reports_on_the_table_it_searched() {
    let out = scan_run(
        r#"
USE customer.dbf
? FOUND()
LOCATE FOR amount > 350
? FOUND()
? ALLTRIM(custno)
LOCATE REST FOR state = "NY"
? ALLTRIM(custno)
"#,
    );
    assert_eq!(out, vec![".F.", ".T.", "D400", "E500"]);
}

// ---- writing -----------------------------------------------------------------------------------

#[test]
fn replace_changes_the_record_the_pointer_is_on() {
    let out = scan_run(
        r#"
USE customer.dbf
GO 2
REPLACE state WITH "WA", amount WITH 12.5
? ALLTRIM(state)
? amount
GO 1
? ALLTRIM(state)
GO 2
? ALLTRIM(state)
? amount
"#,
    );
    // the last two read the record back after the pointer left it and came back
    assert_eq!(out, vec!["WA", "   12.50", "NY", "WA", "   12.50"]);
}

#[test]
fn replace_all_walks_the_scope() {
    let out = scan_run(
        r#"
USE customer.dbf
REPLACE ALL state WITH "ZZ" FOR amount > 250
GO TOP
LOCAL c
c = ""
SCAN
  c = c + ALLTRIM(state) + " "
ENDSCAN
? c
"#,
    );
    // record 3 counts too: SET DELETED is off, and REPLACE ALL means all of them
    assert_eq!(out, vec!["NY CA ZZ ZZ ZZ "]);
}

#[test]
fn delete_and_recall_mark_records_without_removing_them() {
    let out = scan_run(
        r#"
USE customer.dbf
GO 2
DELETE
? DELETED()
? RECCOUNT()
RECALL
? DELETED()
DELETE ALL FOR state = "NY"
SET DELETED ON
GO TOP
LOCAL c
c = ""
SCAN
  c = c + ALLTRIM(custno) + " "
ENDSCAN
? c
"#,
    );
    // record 3 was already deleted in this table, and the three NY rows join it
    assert_eq!(out, vec![".T.", "         5", ".F.", "B200 D400 "]);
}

#[test]
fn append_blank_adds_a_record_the_header_admits_to() {
    let out = scan_run(
        r#"
USE customer.dbf
APPEND BLANK
? RECNO()
? RECCOUNT()
? EOF()
REPLACE custno WITH "F600", state WITH "OR"
? ALLTRIM(custno) + "/" + ALLTRIM(state)
GO TOP
GO BOTTOM
? RECNO()
? ALLTRIM(custno)
? amount
"#,
    );
    assert_eq!(out, vec!["         6", "         6", ".F.", "F600/OR", "         6", "F600", "    0.00"]);
}

#[test]
fn a_write_reaches_the_file() {
    let mut host = scan_host();
    run_program("USE customer.dbf\nGO 2\nREPLACE state WITH \"WA\"", &mut host).expect("run");
    let file = host.open_file(1).expect("the table stayed open");
    let text: String = file.iter().map(|&b| b as char).collect();
    assert!(text.contains("B200  WA"), "{text:?}");
}

// ---- the default directory, and reading back what ON installed ---------------------------------

#[test]
fn on_reports_the_command_it_installed_so_a_program_can_put_it_back() {
    // the shape every VFP sample opens with: keep the old handler, install one, restore it
    let out = scan_run(
        r#"
? "[" + ON("ERROR") + "]"
ON ERROR DO Handler
? ON("ERROR")
LOCAL cOld
cOld = ON("ERROR")
ON ERROR
? "[" + ON("ERROR") + "]"
? "[" + ON("SHUTDOWN") + "]"
? "[" + cOld + "]"
"#,
    );
    assert_eq!(out, vec!["[]", "DO Handler", "[]", "[]", "[DO Handler]"]);
}

#[test]
fn set_default_to_is_where_a_relative_name_is_looked_for() {
    let mut host = scan_host();
    // the table answers to its full path, so finding it proves the default directory was used
    let bytes = host.tables.remove("CUSTOMER.DBF").expect("table");
    host.tables.insert(r"C:\APP\DATA\CUSTOMER.DBF".into(), bytes);

    let out = match run_program(
        r#"
SET DEFAULT TO c:\app\data
? CURDIR()
USE customer.dbf
? ALIAS()
? RECCOUNT()
"#,
        &mut host,
    ) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    };
    assert_eq!(out, vec![r"c:\app\data\", "CUSTOMER", "         5"]);
}

// ---- the commands that ask the development environment to open something -----------------------

#[test]
fn modify_opens_a_file_and_browse_shows_the_records() {
    use foxvm::compiler::compile_program;
    use foxvm::host::HostRequest;
    use foxvm::vm::{Step, Vm};

    let mut host = scan_host();
    let src = "USE customer.dbf\nMODIFY COMMAND main.prg\nBROWSE\n";
    let module = compile_program(src, "main").module.expect("compiles");
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());

    let mut opened = Vec::new();
    let mut browsed: Option<foxvm::host::BrowseTable> = None;
    let mut answers = std::collections::VecDeque::new();
    loop {
        match vm.step(&mut host, fiber) {
            Step::Suspend(req) => {
                match &req {
                    HostRequest::OpenDocument { path } => opened.push(path.clone()),
                    HostRequest::Browse { browse, .. } => browsed = Some(browse.clone()),
                    _ => {}
                }
                let answer = answers.pop_front().unwrap_or_else(|| host.default_answer(&req));
                vm.resume(fiber, answer);
            }
            Step::Done { .. } => break,
            Step::Error(e) => panic!("{}: {}", e.code, e.message),
        }
    }
    // MODIFY opens what it names; BROWSE shows the records rather than opening the file
    assert_eq!(opened, vec!["main.prg".to_string()]);
    let browse = browsed.expect("BROWSE handed the records over");
    assert_eq!(browse.alias.to_uppercase(), "CUSTOMER");
    assert_eq!(browse.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["CUSTNO", "STATE", "AMOUNT"]);
    // the table holds five records and one of them is deleted, which a browse leaves out
    assert_eq!(browse.count, 5);
    assert_eq!(browse.rows.len(), 4);
    assert!(browse.rows.iter().all(|r| !r.deleted));
}

#[test]
fn a_document_named_without_an_extension_takes_the_one_its_kind_brings() {
    use foxvm::compiler::compile_program;
    use foxvm::host::HostRequest;
    use foxvm::vm::{Step, Vm};

    let mut host = scan_host();
    // the word after MODIFY says what kind of file it is, and that is what supplies the
    // extension: measured in Visual FoxPro 9, MODIFY COMMAND zzz1 wrote zzz1.PRG and
    // MODIFY FILE zzz2 wrote zzz2.TXT. A name written with an extension keeps it.
    let src = r#"
MODIFY COMMAND main
MODIFY FILE notes
MODIFY DATABASE dvds
MODIFY FORM edit
MODIFY REPORT sales
MODIFY LABEL tags
MODIFY MENU top
MODIFY CLASS mylib
MODIFY PROJECT app
MODIFY FILE dvds.log
lcName = "dvds"
MODIFY DATABASE (lcName)
"#;
    let module = compile_program(src, "main").module.expect("compiles");
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());

    let mut opened = Vec::new();
    loop {
        match vm.step(&mut host, fiber) {
            Step::Suspend(req) => {
                if let HostRequest::OpenDocument { path } = &req {
                    opened.push(path.clone());
                }
                let answer = host.default_answer(&req);
                vm.resume(fiber, answer);
            }
            Step::Done { .. } => break,
            Step::Error(e) => panic!("{}: {}", e.code, e.message),
        }
    }
    assert_eq!(
        opened,
        vec![
            "main.prg", "notes.txt", "dvds.dbc", "edit.scx", "sales.frx", "tags.lbx", "top.mnx", "mylib.vcx",
            "app.pjx", "dvds.log", "dvds.dbc",
        ]
    );
}

// ---- cursors a program makes for itself --------------------------------------------------------

#[test]
fn create_cursor_and_insert_build_a_table_out_of_nothing() {
    let out = scan_run(
        r#"
CREATE CURSOR tally (code C(6), n I, note M)
? ALIAS()
? FCOUNT()
? RECCOUNT()
INSERT INTO tally VALUES ("A100", 3, "")
INSERT INTO tally (n, code) VALUES (7, "B200")
? RECCOUNT()
GO TOP
? ALLTRIM(code) + "/" + LTRIM(STR(n))
SKIP
? ALLTRIM(code) + "/" + LTRIM(STR(n))
"#,
    );
    assert_eq!(out, vec!["TALLY", "         3", "         0", "         2", "A100/3", "B200/7"]);
}

#[test]
fn a_cursor_takes_the_rows_a_scan_over_a_table_puts_in_it() {
    let out = scan_run(
        r#"
CREATE CURSOR big (custno C(6))
USE customer.dbf IN 0
SELECT customer
SCAN FOR amount > 250
  INSERT INTO big VALUES (custno)
ENDSCAN
SELECT big
? RECCOUNT()
GO TOP
LOCAL c
c = ""
SCAN
  c = c + ALLTRIM(custno) + " "
ENDSCAN
? c
"#,
    );
    assert_eq!(out, vec!["         3", "C300 D400 E500 "]);
}

#[test]
fn a_terminator_may_say_what_it_closed() {
    // `ENDIF NOT .lFound` and `NEXT i` are how FoxPro programmers annotate a block
    let out = scan_run(
        r#"
LOCAL i, c
c = ""
FOR i = 1 TO 3
  IF i = 2
    c = c + "two "
  ENDIF i = 2
  c = c + LTRIM(STR(i)) + " "
NEXT i
? c
"#,
    );
    assert_eq!(out, vec!["1 two 2 3 "]);
}

#[test]
/// Only a property that was declared with subscripts can be sized: measured, `DIMENSION` over a
/// property holding a plain value raises 232 rather than turning it into an array.
fn dimension_can_size_an_array_that_belongs_to_an_object() {
    let out = scan_run(
        r#"
PUBLIC oHolder
oHolder = CREATEOBJECT("Empty")
ADDPROPERTY(oHolder, "aRows[1]", .F.)
DIMENSION oHolder.aRows[2, 3]
? ALEN(oHolder.aRows)
? ALEN(oHolder.aRows, 1)
? ALEN(oHolder.aRows, 2)
"#,
    );
    assert_eq!(out, vec!["         6", "         2", "         3"]);
}

#[test]
fn delete_file_removes_a_file_rather_than_a_record() {
    use foxvm::host::HostRequest;
    let mut host = scan_host();
    let (_, _) = run_program("DELETE FILE (\"scratch.tmp\")", &mut host).expect("run");
    assert!(
        host.requests.iter().any(|r| matches!(r, HostRequest::FileDelete { path } if path == "scratch.tmp")),
        "{:?}",
        host.requests
    );
}

#[test]
fn zap_empties_the_table_and_the_header_agrees() {
    let out = scan_run(
        r#"
USE customer.dbf
? RECCOUNT()
ZAP
? RECCOUNT()
? EOF()
APPEND BLANK
REPLACE custno WITH "Z100"
? RECCOUNT()
? ALLTRIM(custno)
"#,
    );
    assert_eq!(out, vec!["         5", "         0", ".T.", "         1", "Z100"]);
}

#[test]
fn a_zapped_table_says_so_in_the_file() {
    let mut host = scan_host();
    run_program("USE customer.dbf\nZAP", &mut host).expect("run");
    let file = host.open_file(1).expect("the table stayed open");
    assert_eq!(&file[4..8], &[0, 0, 0, 0], "the header's record count");
}

#[test]
fn create_table_writes_a_table_and_leaves_it_in_use() {
    let out = scan_run(
        r#"
CREATE TABLE people (name C(10), age N(3), born D, notes M)
? ALIAS()
? RECCOUNT()
APPEND BLANK
REPLACE name WITH "Ann", age WITH 41
? ALLTRIM(name) + "/" + ALLTRIM(STR(age))
? RECCOUNT()
"#,
    );
    assert_eq!(out, vec!["PEOPLE", "         0", "Ann/41", "         1"]);
}
