//! The functions that report on work areas, against a session with tables in it.

mod ctx;

use ctx::{TestCtx, n, num_on, s, text_on, value_on};
use foxvm::data::{AreaRef, Cursor, DataSession};
use foxvm::dbf::read_header;
use foxvm::value::Value;

/// A minimal but real DBF header: two character fields, `count` records declared.
fn header_bytes(count: u32) -> Vec<u8> {
    let mut b = vec![0u8; 32];
    b[0] = 0x30; // Visual FoxPro
    b[4..8].copy_from_slice(&count.to_le_bytes());
    let header_len: u16 = 32 + 32 * 2 + 1;
    b[8..10].copy_from_slice(&header_len.to_le_bytes());
    let record_len: u16 = 1 + 10 + 6;
    b[10..12].copy_from_slice(&record_len.to_le_bytes());

    for (name, width) in [("CUSTNO", 10u8), ("STATE", 6u8)] {
        let mut desc = vec![0u8; 32];
        desc[..name.len()].copy_from_slice(name.as_bytes());
        desc[11] = b'C';
        desc[16] = width;
        b.extend_from_slice(&desc);
    }
    b.push(0x0D);
    b
}

fn session() -> DataSession {
    let mut data = DataSession::new();
    let header = read_header(&header_bytes(3)).expect("header");
    data.select(&AreaRef::Number(0)).expect("select 0");
    data.install(Cursor::new(7, "customer".into(), "data\\customer.dbf".into(), header));
    let header2 = read_header(&header_bytes(0)).expect("header");
    data.select(&AreaRef::Number(0)).expect("select 0");
    data.install(Cursor::new(8, "orders".into(), "orders.dbf".into(), header2));
    data
}

fn ctx() -> TestCtx {
    TestCtx { data: session(), ..TestCtx::default() }
}

#[test]
fn a_closed_work_area_answers_rather_than_failing() {
    let mut c = TestCtx::default();
    assert_eq!(num_on(&mut c, "RECNO", vec![]), 0.0);
    assert_eq!(num_on(&mut c, "RECCOUNT", vec![]), 0.0);
    assert_eq!(text_on(&mut c, "ALIAS", vec![]), "");
    assert_eq!(num_on(&mut c, "FCOUNT", vec![]), 0.0);
    assert_eq!(value_on(&mut c, "USED", vec![]), Value::Logical(false));
    // Measured: a work area with no table in it is neither past the end nor before the start,
    // so both are false there. A table with no records in it is at both ends at once, which is
    // the case `DO WHILE NOT EOF()` actually meets.
    assert_eq!(value_on(&mut c, "EOF", vec![]), Value::Logical(false));
    assert_eq!(value_on(&mut c, "BOF", vec![]), Value::Logical(false));
}

#[test]
fn the_selected_area_is_the_one_reported_on() {
    let mut c = ctx();
    // the second USE landed in area 2 and left it selected
    assert_eq!(num_on(&mut c, "SELECT", vec![]), 2.0);
    assert_eq!(text_on(&mut c, "ALIAS", vec![]), "ORDERS");
    assert_eq!(num_on(&mut c, "RECCOUNT", vec![]), 0.0);
    assert_eq!(text_on(&mut c, "DBF", vec![]), "orders.dbf");

    c.data.select(&AreaRef::Number(1)).expect("select 1");
    assert_eq!(num_on(&mut c, "SELECT", vec![]), 1.0);
    assert_eq!(text_on(&mut c, "ALIAS", vec![]), "CUSTOMER");
    assert_eq!(num_on(&mut c, "RECCOUNT", vec![]), 3.0);
    assert_eq!(num_on(&mut c, "RECNO", vec![]), 1.0);
    assert_eq!(text_on(&mut c, "DBF", vec![]), "data\\customer.dbf");
}

#[test]
fn another_area_can_be_named_by_number_or_alias() {
    let mut c = ctx();
    assert_eq!(text_on(&mut c, "ALIAS", vec![n(1.0)]), "CUSTOMER");
    assert_eq!(num_on(&mut c, "RECCOUNT", vec![n(1.0)]), 3.0);
    assert_eq!(num_on(&mut c, "RECCOUNT", vec![s("customer")]), 3.0);
    assert_eq!(value_on(&mut c, "USED", vec![s("CUSTOMER")]), Value::Logical(true));
    assert_eq!(value_on(&mut c, "USED", vec![s("nosuch")]), Value::Logical(false));
    assert_eq!(value_on(&mut c, "USED", vec![n(9.0)]), Value::Logical(false));
    // a work area that holds nothing reads as empty rather than raising
    assert_eq!(text_on(&mut c, "ALIAS", vec![n(9.0)]), "");
}

#[test]
fn an_empty_table_is_at_both_ends_at_once() {
    let mut c = ctx();
    // orders declares no records, so VFP puts it at end of file and beginning of file together
    assert_eq!(value_on(&mut c, "EOF", vec![]), Value::Logical(true));
    assert_eq!(value_on(&mut c, "BOF", vec![]), Value::Logical(true));
    assert_eq!(value_on(&mut c, "EOF", vec![s("customer")]), Value::Logical(false));
    assert_eq!(value_on(&mut c, "BOF", vec![s("customer")]), Value::Logical(false));
}

#[test]
fn the_columns_are_reported_from_the_header() {
    let mut c = ctx();
    assert_eq!(num_on(&mut c, "FCOUNT", vec![s("customer")]), 2.0);
    assert_eq!(text_on(&mut c, "FIELD", vec![n(1.0), s("customer")]), "CUSTNO");
    assert_eq!(text_on(&mut c, "FIELD", vec![n(2.0), s("customer")]), "STATE");
    // out of range is empty, not an error, as VFP reports it
    assert_eq!(text_on(&mut c, "FIELD", vec![n(3.0), s("customer")]), "");
    assert_eq!(text_on(&mut c, "FIELD", vec![n(0.0), s("customer")]), "");
}

#[test]
fn select_one_names_the_area_a_scratch_cursor_would_go_in() {
    let mut c = ctx();
    assert_eq!(num_on(&mut c, "SELECT", vec![s("1")]), 3.0);
    assert_eq!(num_on(&mut TestCtx::default(), "SELECT", vec![s("1")]), 1.0);
}
