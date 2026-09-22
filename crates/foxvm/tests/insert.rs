//! `INSERT [BEFORE] [BLANK]`, `INSERT INTO ... FROM`, and `CREATE CURSOR ... FROM ARRAY`.
//!
//! Every number here was measured against Visual FoxPro 9 rather than read out of the help:
//! where the record lands, what a short array leaves behind, and which of these commands the
//! product refuses outright.

mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table(
        &[("A", b'C', 5, 0), ("B", b'N', 3, 0)],
        &[&["one", "1"], &["two", "2"], &["three", "3"]],
    );
    let mut host = MockHost::new();
    host.tables.insert("PARTS.DBF".into(), bytes);
    host
}

fn run(src: &str) -> Vec<String> {
    let mut host = host();
    match run_program(src, &mut host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn fails(src: &str) -> (u32, String) {
    let mut host = host();
    match run_program(src, &mut host) {
        Ok((_, output)) => panic!("expected an error, got {output:?}"),
        Err(e) => (e.code, e.message),
    }
}

// ---- INSERT [BEFORE] [BLANK] ----------------------------------------------------------------

#[test]
fn insert_blank_puts_a_record_below_the_one_the_pointer_is_on() {
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3))
INSERT INTO t VALUES ("one", 1)
INSERT INTO t VALUES ("two", 2)
INSERT INTO t VALUES ("three", 3)
GO 2
INSERT BLANK
? RECNO(), RECCOUNT()
GO TOP
SCAN
  ? RECNO(), ALLTRIM(t.a), t.b
ENDSCAN
"#,
    );
    assert_eq!(out, vec!["         3          4", "         1 one   1", "         2 two   2", "         3    0", "         4 three   3"]);
}

#[test]
fn insert_before_blank_puts_it_above_that_record_instead() {
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3))
INSERT INTO t VALUES ("one", 1)
INSERT INTO t VALUES ("two", 2)
INSERT INTO t VALUES ("three", 3)
GO 2
INSERT BEFORE BLANK
? RECNO(), RECCOUNT()
GO TOP
SCAN
  ? RECNO(), ALLTRIM(t.a), t.b
ENDSCAN
"#,
    );
    assert_eq!(out, vec!["         2          4", "         1 one   1", "         2    0", "         3 two   2", "         4 three   3"]);
}

#[test]
fn at_the_end_of_the_table_there_is_nothing_to_go_below_so_it_appends() {
    // measured: on a two-record table the pointer at end of file is record 3, and both forms
    // put the new record there rather than at 4
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3))
INSERT INTO t VALUES ("one", 1)
INSERT INTO t VALUES ("two", 2)
GO BOTTOM
SKIP
? RECNO(), EOF()
INSERT BLANK
? RECNO(), RECCOUNT(), EOF()
"#,
    );
    assert_eq!(out, vec!["         3 .T.", "         3          3 .F."]);
}

#[test]
fn an_empty_table_takes_the_record_at_one_either_way() {
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3))
INSERT BLANK
? RECNO(), RECCOUNT()
INSERT BEFORE BLANK
? RECNO(), RECCOUNT()
"#,
    );
    assert_eq!(out, vec!["         1          1", "         1          2"]);
}

#[test]
fn a_deleted_mark_travels_with_the_record_it_belongs_to() {
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3))
INSERT INTO t VALUES ("one", 1)
INSERT INTO t VALUES ("two", 2)
GO 2
DELETE
GO 1
INSERT BLANK
SET DELETED OFF
GO TOP
SCAN
  ? RECNO(), ALLTRIM(t.a), DELETED()
ENDSCAN
"#,
    );
    assert_eq!(out, vec!["         1 one .F.", "         2  .F.", "         3 two .T."]);
}

#[test]
fn insert_writes_a_table_in_a_file_from_the_insertion_point_down() {
    let mut host = host();
    let out = run_program(
        r#"
USE parts.dbf
GO 2
INSERT BLANK
? RECNO(), RECCOUNT()
GO TOP
SCAN
  ? RECNO(), ALLTRIM(parts.a), parts.b
ENDSCAN
USE parts.dbf
? RECCOUNT()
GO 3
? ALLTRIM(parts.a), parts.b
"#,
        &mut host,
    );
    let (_, out) = out.unwrap_or_else(|e| panic!("{}: {}", e.code, e.message));
    assert_eq!(
        out,
        vec!["         3          4", "         1 one   1", "         2 two   2", "         3    0", "         4 three   3", "         4", "   0"]
    );
}

#[test]
fn an_open_index_makes_visual_foxpro_refuse_the_command() {
    // every entry of every tag points at a record number, and moving the records down one
    // would leave them all pointing at the wrong record. VFP answers 1588 rather than rebuild.
    let (code, _) = fails(
        r#"
CREATE CURSOR t (a c(5), b n(3))
INSERT INTO t VALUES ("ccc", 1)
INSERT INTO t VALUES ("aaa", 2)
INDEX ON a TAG ta
GO TOP
INSERT BLANK
"#,
    );
    assert_eq!(code, 1588);
}

#[test]
fn a_buffered_table_refuses_it_too() {
    let (code, _) = fails(
        r#"
CREATE CURSOR t (a c(5), b n(3))
INSERT INTO t VALUES ("one", 1)
SET MULTILOCKS ON
= CURSORSETPROP("Buffering", 5)
GO 1
INSERT BLANK
"#,
    );
    assert_eq!(code, 1579);
}

#[test]
fn insert_takes_no_clause_but_before_and_blank() {
    // measured: `INSERT BLANK IN t` and `INSERT BLANK NOMENU` are both syntax errors in VFP
    let mut host = host();
    let Err(e) = run_program("CREATE CURSOR t (a c(3))\nINSERT BLANK IN t\n", &mut host) else {
        panic!("INSERT BLANK IN t should not compile")
    };
    assert!(e.message.contains("BEFORE and BLANK"), "{}", e.message);
}

// ---- INSERT INTO ... FROM -------------------------------------------------------------------

#[test]
fn from_array_fills_the_fields_in_order() {
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3), c d, l l)
DIMENSION av[4]
av[1] = "one"
av[2] = 11
av[3] = DATE(2020, 1, 2)
av[4] = .T.
INSERT INTO t FROM ARRAY av
? RECCOUNT(), RECNO(), ALLTRIM(t.a), t.b, DTOC(t.c), t.l
"#,
    );
    assert_eq!(out, vec!["         1          1 one  11 01/02/20 .T."]);
}

#[test]
fn an_array_shorter_than_the_table_leaves_the_rest_of_the_record_empty() {
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3), c d)
DIMENSION av[2]
av[1] = "two"
av[2] = 22
INSERT INTO t FROM ARRAY av
? RECCOUNT(), ALLTRIM(t.a), t.b, EMPTY(t.c)
"#,
    );
    assert_eq!(out, vec!["         1 two  22 .T."]);
}

#[test]
fn an_array_longer_than_the_table_drops_what_is_left_over() {
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3))
DIMENSION av[4]
av[1] = "thre"
av[2] = 33
av[3] = "spare"
av[4] = 99
INSERT INTO t FROM ARRAY av
? RECCOUNT(), ALLTRIM(t.a), t.b
"#,
    );
    assert_eq!(out, vec!["         1 thre  33"]);
}

#[test]
fn an_array_of_two_dimensions_puts_in_a_record_per_row() {
    // this is the whole reason the statement is a loop: VFP inserts every row, not just the first
    let out = run(
        r#"
CREATE CURSOR t (a c(5), b n(3))
DIMENSION am[3, 2]
am[1, 1] = "r1"
am[1, 2] = 1
am[2, 1] = "r2"
am[2, 2] = 2
am[3, 1] = "r3"
am[3, 2] = 3
INSERT INTO t FROM ARRAY am
? RECCOUNT(), RECNO()
GO TOP
SCAN
  ? RECNO(), ALLTRIM(t.a), t.b
ENDSCAN
"#,
    );
    assert_eq!(out, vec!["         3          3", "         1 r1   1", "         2 r2   2", "         3 r3   3"]);
}

#[test]
fn from_memvar_takes_the_variables_named_after_the_fields() {
    let out = run(
        r#"
CREATE CURSOR t (name c(10), age n(3), nope c(4))
m.name = "Jorge"
m.age = 40
INSERT INTO t FROM MEMVAR
? RECCOUNT(), ALLTRIM(t.name), t.age, EMPTY(t.nope), ISNULL(t.nope)
"#,
    );
    assert_eq!(out, vec!["         1 Jorge  40 .T. .F."]);
}

#[test]
fn from_name_takes_the_properties_named_after_the_fields() {
    let out = run(
        r#"
CREATE CURSOR t (name c(10), missing c(4))
oRec = CREATEOBJECT("Empty")
ADDPROPERTY(oRec, "name", "Bob")
ADDPROPERTY(oRec, "spare", "junk")
INSERT INTO t FROM NAME oRec
? RECCOUNT(), ALLTRIM(t.name), EMPTY(t.missing), ISNULL(t.missing)
"#,
    );
    assert_eq!(out, vec!["         1 Bob .T. .F."]);
}

#[test]
fn the_table_may_be_named_by_an_expression_and_the_area_comes_back_after() {
    let out = run(
        r#"
CREATE CURSOR other (a c(3))
CREATE CURSOR t (name c(10))
oRec = CREATEOBJECT("Empty")
ADDPROPERTY(oRec, "name", "Ana")
lcTarget = "t"
SELECT other
INSERT INTO (m.lcTarget) FROM NAME oRec
? ALIAS(), RECCOUNT("t"), ALLTRIM(t.name)
"#,
    );
    assert_eq!(out, vec!["OTHER          1 Ana"]);
}

#[test]
fn a_field_list_with_from_is_a_syntax_error_as_it_is_in_the_product() {
    let mut host = host();
    let src = "CREATE CURSOR t (a c(5), b n(3))\nDIMENSION av[2]\nINSERT INTO t (a, b) FROM ARRAY av\n";
    let Err(e) = run_program(src, &mut host) else { panic!("a field list with FROM should not compile") };
    assert_eq!(e.code, 10);
}

// ---- CREATE CURSOR ... FROM ARRAY -----------------------------------------------------------

#[test]
fn a_structure_read_with_afields_makes_the_same_table_again() {
    let out = run(
        r#"
CREATE CURSOR src (name c(10), age n(5, 2), born d, note m, flag l)
DIMENSION af[1]
= AFIELDS(af)
CREATE CURSOR dst FROM ARRAY af
? ALIAS(), FCOUNT()
DIMENSION af2[1]
= AFIELDS(af2)
FOR i = 1 TO FCOUNT()
  ? af2[i, 1], af2[i, 2], af2[i, 3], af2[i, 4]
ENDFOR
"#,
    );
    assert_eq!(
        out,
        vec!["DST          5", "NAME C         10          0", "AGE N          5          2", "BORN D          8          0", "NOTE M          4          0", "FLAG L          1          0"]
    );
}

#[test]
fn four_columns_are_all_the_array_has_to_carry() {
    let out = run(
        r#"
DIMENSION a4[2, 4]
a4[1, 1] = "colone"
a4[1, 2] = "C"
a4[1, 3] = 12
a4[1, 4] = 0
a4[2, 1] = "coltwo"
a4[2, 2] = "N"
a4[2, 3] = 8
a4[2, 4] = 2
CREATE CURSOR c4 FROM ARRAY a4
DIMENSION af[1]
= AFIELDS(af)
? ALIAS(), FCOUNT()
? af[1, 1], af[1, 2], af[1, 3], af[1, 4]
? af[2, 1], af[2, 2], af[2, 3], af[2, 4]
"#,
    );
    assert_eq!(out, vec!["C4          2", "COLONE C         12          0", "COLTWO N          8          2"]);
}

#[test]
fn an_array_of_one_dimension_describes_a_single_field() {
    let out = run(
        r#"
DIMENSION a1[4]
a1[1] = "flat"
a1[2] = "C"
a1[3] = 10
a1[4] = 0
CREATE CURSOR c1 FROM ARRAY a1
DIMENSION af[1]
= AFIELDS(af)
? ALIAS(), FCOUNT(), af[1, 1], af[1, 2], af[1, 3]
"#,
    );
    assert_eq!(out, vec!["C1          1 FLAT C         10"]);
}

#[test]
fn a_type_that_carries_its_own_width_ignores_the_one_the_array_gives() {
    let out = run(
        r#"
DIMENSION ad[3, 4]
ad[1, 1] = "adate"
ad[1, 2] = "D"
ad[1, 3] = 99
ad[1, 4] = 7
ad[2, 1] = "alog"
ad[2, 2] = "L"
ad[2, 3] = 50
ad[2, 4] = 3
ad[3, 1] = "amemo"
ad[3, 2] = "M"
ad[3, 3] = 77
ad[3, 4] = 1
CREATE CURSOR cfix FROM ARRAY ad
DIMENSION af[1]
= AFIELDS(af)
? af[1, 3], af[1, 4], af[2, 3], af[2, 4], af[3, 3], af[3, 4]
"#,
    );
    assert_eq!(out, vec!["         8          0          1          0          4          0"]);
}

#[test]
fn a_width_written_as_digits_in_a_string_still_reads_as_a_width() {
    let out = run(
        r#"
DIMENSION astr[1, 4]
astr[1, 1] = "asstring"
astr[1, 2] = "C"
astr[1, 3] = "7"
astr[1, 4] = "0"
CREATE CURSOR cstr FROM ARRAY astr
DIMENSION af[1]
= AFIELDS(af)
? af[1, 1], af[1, 2], af[1, 3], af[1, 4]
"#,
    );
    assert_eq!(out, vec!["ASSTRING C          7          0"]);
}

#[test]
fn fewer_than_four_columns_describes_no_field_at_all() {
    let (code, _) = fails(
        r#"
DIMENSION a3[1, 3]
a3[1, 1] = "onlyone"
a3[1, 2] = "C"
a3[1, 3] = 5
CREATE CURSOR c3 FROM ARRAY a3
"#,
    );
    assert_eq!(code, 47);
}

#[test]
fn a_name_that_is_not_a_name_and_a_type_that_is_not_a_type_are_both_refused() {
    let (name, _) = fails(
        r#"
DIMENSION ab[1, 4]
ab[1, 1] = "not a name!"
ab[1, 2] = "C"
ab[1, 3] = 5
ab[1, 4] = 0
CREATE CURSOR cbad FROM ARRAY ab
"#,
    );
    assert_eq!(name, 1712);

    let (kind, _) = fails(
        r#"
DIMENSION ab[1, 4]
ab[1, 1] = "weird"
ab[1, 2] = "Z"
ab[1, 3] = 5
ab[1, 4] = 0
CREATE CURSOR cbad FROM ARRAY ab
"#,
    );
    assert_eq!(kind, 11);
}

#[test]
fn a_width_of_nothing_and_more_decimals_than_width_are_both_refused() {
    let (zero, _) = fails(
        r#"
DIMENSION az[1, 4]
az[1, 1] = "zero"
az[1, 2] = "C"
az[1, 3] = 0
az[1, 4] = 0
CREATE CURSOR cz FROM ARRAY az
"#,
    );
    assert_eq!(zero, 1713);

    let (wide, _) = fails(
        r#"
DIMENSION an[1, 4]
an[1, 1] = "num"
an[1, 2] = "N"
an[1, 3] = 3
an[1, 4] = 5
CREATE CURSOR cn FROM ARRAY an
"#,
    );
    assert_eq!(wide, 1713);
}

#[test]
fn the_cursor_name_may_be_worked_out_when_the_statement_runs() {
    let out = run(
        r#"
DIMENSION a4[1, 4]
a4[1, 1] = "colone"
a4[1, 2] = "C"
a4[1, 3] = 12
a4[1, 4] = 0
lcName = "madeup"
CREATE CURSOR (m.lcName) FROM ARRAY a4
? ALIAS(), FCOUNT()
DIMENSION af2[1]
= AFIELDS(af2)
"#,
    );
    assert_eq!(out, vec!["MADEUP          1"]);
}

#[test]
fn create_table_takes_the_same_array() {
    let mut host = host();
    let (_, out) = run_program(
        r#"
DIMENSION a4[2, 4]
a4[1, 1] = "colone"
a4[1, 2] = "C"
a4[1, 3] = 12
a4[1, 4] = 0
a4[2, 1] = "coltwo"
a4[2, 2] = "N"
a4[2, 3] = 8
a4[2, 4] = 2
CREATE TABLE made FREE FROM ARRAY a4
? ALIAS(), FCOUNT(), FIELD(1), FIELD(2)
"#,
        &mut host,
    )
    .unwrap_or_else(|e| panic!("{}: {}", e.code, e.message));
    assert_eq!(out, vec!["MADE          2 COLONE COLTWO"]);
}

// ---- the USE clause list ---------------------------------------------------------------------

#[test]
fn the_clauses_that_belong_to_views_are_taken_and_mean_nothing_to_a_table() {
    let out = run(
        r#"
USE parts.dbf NODATA NOREQUERY 1 SHARED NOUPDATE AGAIN
? ALIAS(), RECCOUNT()
"#,
    );
    assert_eq!(out, vec!["PARTS          3"]);
}

#[test]
fn online_asks_for_an_offline_view_and_a_table_is_never_one() {
    // measured: VFP opens the table and then answers 2009, because ONLINE is asking for
    // something only a view taken offline can give
    let (code, message) = fails("USE parts.dbf ONLINE EXCLUSIVE\n");
    assert_eq!(code, 2009);
    assert!(message.contains("offline view"), "{message}");
}
