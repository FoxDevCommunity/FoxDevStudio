//! Indexes end to end: INDEX ON builds a tag, SET ORDER walks the table down it, SEEK finds a
//! key, and a record changed afterwards moves to where its new key belongs.
//!
//! The host is [`MockHost`], which keeps the table and the index beside it in memory the way the
//! main process keeps them on disk. What the VM writes is a real `.cdx`, so the reader that
//! Visual FoxPro's own files are tested against is the one that reads it back.

mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table(
        &[("CUSTNO", b'C', 6, 0), ("NAME", b'C', 10, 0), ("AMOUNT", b'N', 8, 2)],
        &[
            &["C300", "Cirrus", "1000.25"],
            &["A100", "Acme", "125.50"],
            &["B200", "Beta", "80.00"],
            &["D400", "Delta", "250.00"],
        ],
    );
    let mut host = MockHost::new();
    host.tables.insert("CUSTOMER.DBF".into(), bytes);
    host
}

fn run_on(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn run(src: &str) -> Vec<String> {
    run_on(&mut host(), src)
}

#[test]
fn index_on_builds_a_tag_and_orders_the_table() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON custno TAG bycust
? ORDER(), TAGCOUNT(), TAG(1), KEY(1), TAGNO()
GO TOP
? custno
SKIP
? custno
GO BOTTOM
? custno, RECNO()
SKIP
? EOF()
"#,
    );
    assert_eq!(
        out,
        vec![
            "BYCUST          1 BYCUST CUSTNO          1",
            "A100  ",
            "B200  ",
            "D400            4",
            ".T.",
        ]
    );
}

#[test]
fn set_order_puts_the_table_back_in_record_order() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON custno TAG bycust
GO TOP
? custno
SET ORDER TO
? ORDER() == "", TAGNO()
GO TOP
? custno
SET ORDER TO bycust
? custno, ORDER()
"#,
    );
    assert_eq!(out, vec!["A100  ", ".T.          0", "C300  ", "A100   BYCUST"]);
}

#[test]
fn a_descending_order_walks_the_other_way() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON custno TAG bycust DESCENDING
? DESCENDING(1)
GO TOP
? custno
SKIP
? custno
GO BOTTOM
? custno
"#,
    );
    assert_eq!(out, vec![".T.", "D400  ", "C300  ", "A100  "]);
}

#[test]
fn seek_finds_a_key_and_says_when_it_is_not_there() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON custno TAG bycust
SEEK "B200"
? FOUND(), RECNO(), ALLTRIM(name)
SEEK "Z999"
? FOUND(), EOF()
* a partial key finds the first record that begins with it
SEEK "C"
? FOUND(), ALLTRIM(name)
? SEEK("A100"), ALLTRIM(name)
? INDEXSEEK("D400", .F.), ALLTRIM(name)
? INDEXSEEK("D400", .T.), ALLTRIM(name)
"#,
    );
    assert_eq!(
        out,
        vec![
            ".T.          3 Beta", ".F. .T.", ".T. Cirrus", ".T. Acme", ".T. Acme", ".T. Delta",
        ]
    );
}

#[test]
fn a_numeric_key_sorts_as_a_number() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON amount TAG byamount
GO TOP
? ALLTRIM(name), amount
GO BOTTOM
? ALLTRIM(name), amount
SEEK 125.50
? FOUND(), ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["Beta    80.00", "Cirrus  1000.25", ".T. Acme"]);
}

#[test]
fn a_for_condition_leaves_records_out() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON custno TAG big FOR amount > 100
? TAGCOUNT()
GO TOP
? custno
n = 0
SCAN
   n = n + 1
ENDSCAN
? n
"#,
    );
    // the count walks the order, so it sees only the records the tag holds
    assert_eq!(out, vec!["         1", "A100  ", "         3"]);
}

#[test]
fn a_replaced_record_moves_to_where_its_new_key_belongs() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON custno TAG bycust
GO TOP
REPLACE custno WITH "Z999"
GO TOP
? custno
GO BOTTOM
? custno, ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["B200  ", "Z999   Acme"]);
}

#[test]
fn the_index_is_written_beside_the_table_and_read_back_on_the_next_use() {
    let mut host = host();
    run_on(
        &mut host,
        r#"
USE customer.dbf
INDEX ON custno TAG bycust
INDEX ON amount TAG byamount
"#,
    );
    // the index is a file of its own, and the table's header says it is there
    let index = host.indexes.get("CUSTOMER.DBF").expect("an index was written");
    let tags = foxvm::cdx::read_cdx(index).expect("it is a compound index");
    // the tags come back numbered the way they were made, not the way the directory sorts them
    let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["BYCUST", "BYAMOUNT"]);
    assert_eq!(host.tables["CUSTOMER.DBF"][28], 1);

    // and a program that opens the table afterwards finds the tags without building them
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf ORDER bycust
? TAGCOUNT(), ORDER()
? custno
SET ORDER TO byamount
? ALLTRIM(name)
DELETE TAG bycust
? TAGCOUNT(), TAG(1)
"#,
    );
    assert_eq!(out, vec!["         2 BYCUST", "A100  ", "Beta", "         1 BYAMOUNT"]);
}

#[test]
fn a_tag_that_is_not_there_is_an_error() {
    let mut host = host();
    let Err(err) = run_program("USE customer.dbf\nSET ORDER TO nosuchtag\n", &mut host) else {
        panic!("a tag that is not there is an error");
    };
    assert_eq!(err.code, 1683);
    assert_eq!(err.message, "Index tag is not found.");
}

#[test]
fn a_filter_hides_the_records_it_turns_away() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON name TAG byname
SET ORDER TO
SET FILTER TO amount > 200
? FILTER()
GO TOP
? ALLTRIM(name)
SKIP
? ALLTRIM(name)
SKIP
? EOF()
GO BOTTOM
? ALLTRIM(name)
* the filter and an order together: down the tag, past what the filter turns away
SET ORDER TO byname
GO TOP
? ALLTRIM(name)
SKIP
? ALLTRIM(name)
SET FILTER TO
? FILTER() == ""
GO TOP
? ALLTRIM(name)
"#,
    );
    assert_eq!(
        out,
        vec![
            "AMOUNT>200",
            "Cirrus",
            "Delta",
            ".T.",
            "Delta",
            "Cirrus",
            "Delta",
            ".T.",
            "Acme",
        ]
    );
}

#[test]
fn a_relation_moves_the_child_with_the_parent() {
    let mut host = host();
    host.tables.insert(
        "ORDERS.DBF".into(),
        table(
            &[("CUSTNO", b'C', 6, 0), ("ITEM", b'C', 10, 0)],
            &[&["B200", "Widget"], &["A100", "Bolt"], &["D400", "Cog"], &["A100", "Nut"]],
        ),
    );
    let out = run_on(
        &mut host,
        r#"
SELECT 2
USE orders.dbf
INDEX ON custno TAG bycust
SELECT 1
USE customer.dbf
SET RELATION TO custno INTO orders
? RELATION(1), TARGET(1)
GO TOP
? ALLTRIM(customer.name), ALLTRIM(orders.item), FOUND("orders")
GO BOTTOM
? ALLTRIM(customer.name), ALLTRIM(orders.item)
GO 3
? ALLTRIM(customer.name), ALLTRIM(orders.item), FOUND("orders")
SET RELATION OFF INTO orders
? RELATION(1) == ""
"#,
    );
    assert_eq!(
        out,
        vec![
            "CUSTNO ORDERS",
            "Cirrus  .F.",
            "Delta Cog",
            "Beta Widget .T.",
            ".T.",
        ]
    );
}

#[test]
fn index_on_to_writes_a_single_entry_index_and_opens_it() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
INDEX ON custno TO bycust
? ORDER(), TAGCOUNT(), KEY(1), NDX(1), NDX(2) == "", CDX(1) == ""
GO TOP
? custno
GO BOTTOM
? custno
"#,
    );
    assert_eq!(out, vec!["BYCUST          1 CUSTNO BYCUST.IDX .T. .T.", "A100  ", "D400  "]);
    // what was written is a standard .idx, and it holds a key per record
    let bytes = host.files.get("BYCUST.IDX").expect("the index file was written");
    assert!(!foxvm::idx::is_compact(bytes));
    let tag = foxvm::idx::read(bytes).expect("it reads");
    assert_eq!(tag.key_expr, "custno");
    assert_eq!(tag.entries.iter().map(|e| e.recno).collect::<Vec<_>>(), vec![2, 3, 1, 4]);
}

#[test]
fn a_compact_single_entry_index_is_written_when_the_command_asks_for_one() {
    let mut host = host();
    run_on(&mut host, "USE customer.dbf\nINDEX ON amount TO byamount COMPACT\n");
    let bytes = host.files.get("BYAMOUNT.IDX").expect("the index file was written");
    assert!(foxvm::idx::is_compact(bytes));
    let tag = foxvm::idx::read(bytes).expect("it reads");
    assert!(tag.numeric);
    assert_eq!(tag.entries.iter().map(|e| e.recno).collect::<Vec<_>>(), vec![3, 2, 4, 1]);
}

#[test]
fn a_single_entry_index_opened_again_finds_records() {
    let mut host = host();
    run_on(&mut host, "USE customer.dbf\nINDEX ON custno TO bycust\nINDEX ON name TO byname COMPACT\nUSE\n");
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf INDEX byname, bycust
? ORDER(), NDX(1), NDX(2)
GO TOP
? ALLTRIM(name)
? SEEK("Delta"), RECNO(), ALLTRIM(custno)
SET ORDER TO 2
? ORDER(), SEEK("B200"), ALLTRIM(name)
SEEK "Z999"
? FOUND(), EOF()
SET INDEX TO
? ORDER() == "", NDX(1) == ""
GO TOP
? ALLTRIM(custno)
SET INDEX TO bycust
? ORDER(), SEEK("C300"), ALLTRIM(name)
"#,
    );
    assert_eq!(
        out,
        vec![
            "BYNAME BYNAME.IDX BYCUST.IDX", "Acme", ".T.          4 D400", "BYCUST .T. Beta", ".F. .T.", ".T. .T.", "C300", "BYCUST .T. Cirrus",
        ]
    );
}

#[test]
fn additive_keeps_the_indexes_already_open_and_the_new_one_controls() {
    let mut host = host();
    run_on(&mut host, "USE customer.dbf\nINDEX ON custno TO bycust\nINDEX ON name TO byname COMPACT\nUSE\n");
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf INDEX bycust
SET INDEX TO byname ADDITIVE
? ORDER(), NDX(1), NDX(2), TAGCOUNT()
GO TOP
? ALLTRIM(name)
SET INDEX TO bycust
? ORDER(), NDX(2) == ""
INDEX ON amount TAG byamount
? ORDER(), NDX(1) == "", TAGCOUNT()
"#,
    );
    assert_eq!(
        out,
        vec![
            "BYNAME BYCUST.IDX BYNAME.IDX          2",
            "Acme",
            "BYCUST .T.",
            // building an index closes the single-entry ones, ADDITIVE not having been said
            "BYAMOUNT .T.          1",
        ]
    );
}

#[test]
fn copy_tag_writes_a_tag_out_as_an_index_of_its_own() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
INDEX ON UPPER(name) TAG upname
COPY TAG upname TO fromtag
SET INDEX TO fromtag
? ORDER(), KEY(1), NDX(1)
GO TOP
? ALLTRIM(name)
? SEEK("DELTA"), ALLTRIM(custno)
"#,
    );
    assert_eq!(out, vec!["FROMTAG UPPER(NAME) FROMTAG.IDX", "Acme", ".T. D400"]);
    let bytes = host.files.get("FROMTAG.IDX").expect("the index file was written");
    // Visual FoxPro writes the compact kind from a tag, and so does this
    assert!(foxvm::idx::is_compact(bytes));
    assert_eq!(foxvm::idx::read(bytes).expect("it reads").key_expr, "UPPER(name)");
}

#[test]
fn copy_indexes_makes_a_tag_out_of_each_single_entry_index() {
    let mut host = host();
    run_on(&mut host, "USE customer.dbf\nINDEX ON custno TO bycust\nINDEX ON name TO byname COMPACT\nUSE\n");
    // into a compound index of its own
    run_on(&mut host, "USE customer.dbf\nCOPY INDEXES bycust, byname TO made.cdx\n");
    let written = host.files.get("MADE.CDX").expect("the compound index was written");
    let made = foxvm::cdx::read_cdx(written).expect("it reads");
    assert_eq!(made.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["BYCUST", "BYNAME"]);
    assert_eq!(made[0].key_expr, "custno");
    assert_eq!(made[1].entries.iter().map(|e| e.recno).collect::<Vec<_>>(), vec![2, 3, 1, 4]);

    // and into the structural one beside the table, which is what ALL and no TO ask for
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf INDEX bycust, byname
COPY INDEXES ALL
USE
USE customer.dbf
? TAGCOUNT(), TAG(1), TAG(2), KEY(TAGNO("BYNAME"))
SET ORDER TO BYNAME
GO TOP
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         2 BYCUST BYNAME NAME", "Acme"]);
}

#[test]
fn a_copy_that_names_nothing_stops_and_the_next_one_still_works() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
INDEX ON custno TAG bycust
TRY
   COPY TAG missing TO nowhere
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   COPY INDEXES notthere TO made.cdx
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
COPY TAG bycust TO fromtag
SET INDEX TO fromtag
? ORDER(), NDX(1)
"#,
    );
    assert_eq!(out, vec!["      1683", "         1", "FROMTAG FROMTAG.IDX"]);
    assert!(!host.files.contains_key("NOWHERE.IDX"), "nothing was written for the tag that is not there");
    assert!(!host.files.contains_key("MADE.CDX"), "nothing was written for the file that is not there");
}

#[test]
fn an_expression_is_kept_as_written_and_read_back_upper_cased() {
    // What Visual FoxPro keeps is the compiled expression, so the spaces between the pieces go
    // and the names come back upper-cased. Inside quotes is a value rather than a name, so it
    // is left alone: `INDEX ON UPPER(name) + "tail"` reads back as `UPPER(NAME)+"tail"`.
    let out = run(
        "USE customer.dbf\n\
         INDEX ON UPPER(name) + \"tail\" TAG mixed\n\
         INDEX ON custno TAG plain FOR amount > 100\n\
         LOCAL ARRAY laTags[1]\n\
         ? KEY(1), KEY(2)\n\
         ? FOR(2)\n\
         ? ATAGINFO(laTags)\n\
         ? laTags[1, 3], laTags[2, 4]\n",
    );
    assert_eq!(
        out,
        vec![
            "UPPER(NAME)+\"tail\" CUSTNO",
            "AMOUNT>100",
            "         2",
            // the key column is read off the compiled expression, the filter column off what the
            // index file holds, which is the product's own inconsistency
            "UPPER(NAME)+\"tail\" amount>100",
        ]
    );
}

#[test]
fn opening_something_that_is_not_an_index_says_so() {
    let mut host = host();
    host.files.insert("NOTANINDEX.IDX".into(), vec![0u8; 40]);
    for src in ["USE customer.dbf\nSET INDEX TO notanindex\n", "USE customer.dbf\nSET INDEX TO missing\n"] {
        match run_program(src, &mut host) {
            Ok(_) => panic!("{src} should have been refused"),
            Err(e) => assert_eq!(e.code, 1, "{src}: {}", e.message),
        }
    }
}
