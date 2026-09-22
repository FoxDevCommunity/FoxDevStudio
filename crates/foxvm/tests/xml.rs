//! XML in and out: `XMLTOCURSOR()` makes a cursor from a document, `CURSORTOXML()` writes one
//! back, and the two agree with each other.

mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn run(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

/// The shape Visual FoxPro's own `CURSORTOXML()` writes, which is what a program reads back.
const CUSTOMERS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>",
    "<VFPData>",
    "<customer><cust_id>ALFKI</cust_id><company>Alfreds</company><orders>12</orders></customer>",
    "<customer><cust_id>BERGS</cust_id><company>Berglunds</company><orders>7</orders></customer>",
    "</VFPData>",
);

#[test]
fn a_document_becomes_a_cursor_with_a_field_for_each_element() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        &format!(
            "cXML = [{CUSTOMERS}]
? XMLTOCURSOR(cXML, \"custlist\")
? ALIAS(), RECCOUNT()
? FCOUNT(), FIELD(1), FIELD(2), FIELD(3)
? TYPE(\"cust_id\"), TYPE(\"orders\")
GO TOP
? ALLTRIM(cust_id), ALLTRIM(company), orders
GO BOTTOM
? ALLTRIM(cust_id), orders
"
        ),
    );
    assert_eq!(out[0], "         1");
    assert_eq!(out[1], "CUSTLIST          2");
    assert_eq!(out[2], "         3 CUST_ID COMPANY ORDERS");
    assert_eq!(out[3], "C N");
    assert_eq!(out[4], "ALFKI Alfreds 12");
    assert_eq!(out[5], "BERGS  7");
}

#[test]
fn the_fields_may_be_attributes_instead_of_elements() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        "cXML = [<VFPData><row id=\"1\" name=\"Bolt\"/><row id=\"2\" name=\"Nut\"/></VFPData>]
? XMLTOCURSOR(cXML, \"parts\")
? RECCOUNT(), FCOUNT()
GO TOP
? id, ALLTRIM(name)
",
    );
    assert_eq!(out[1], "         2          2");
    assert_eq!(out[2], " 1 Bolt");
}

#[test]
fn a_cursor_goes_out_as_xml_and_reads_back_the_same() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        &format!(
            "cXML = [{CUSTOMERS}]
= XMLTOCURSOR(cXML, \"custlist\")
= CURSORTOXML(\"custlist\", \"cOut\")
? \"declared: \" + IIF(\"<?xml\" $ cOut, \"yes\", \"no\")
? \"rows: \" + ALLTRIM(STR(OCCURS(\"<customer>\", cOut) + OCCURS(\"<custlist>\", cOut)))
= XMLTOCURSOR(cOut, \"again\")
? ALIAS(), RECCOUNT()
GO TOP
? ALLTRIM(cust_id), orders
"
        ),
    );
    assert_eq!(out[0], "declared: yes");
    assert_eq!(out[1], "rows: 2");
    assert_eq!(out[2], "AGAIN          2");
    assert_eq!(out[3], "ALFKI 12");
}

#[test]
fn a_table_on_disk_is_read_whole_before_it_is_written_out() {
    let mut host = MockHost::new();
    host.tables.insert(
        "PARTS.DBF".into(),
        table(&[("CODE", b'C', 4, 0), ("PRICE", b'N', 8, 2)], &[&["A1", "2.50"], &["B2", "1.75"]]),
    );
    let out = run(
        &mut host,
        "USE parts.dbf
= CURSORTOXML(\"parts\", \"cOut\", 0, 1)
? OCCURS(\"<parts>\", cOut)
? IIF(\"A1\" $ cOut, \"first\", \"missing\"), IIF(\"1.75\" $ cOut, \"second\", \"missing\")
",
    );
    assert_eq!(out[0], "         2");
    assert_eq!(out[1], "first second");
}

#[test]
fn the_output_format_writes_the_fields_as_attributes() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        &format!(
            "cXML = [{CUSTOMERS}]
= XMLTOCURSOR(cXML, \"custlist\")
= CURSORTOXML(\"custlist\", \"cOut\", 2, 1)
? IIF('cust_id=\"ALFKI\"' $ cOut, \"attributes\", \"elements\")
"
        ),
    );
    assert_eq!(out[0], "attributes");
}

#[test]
fn xml_that_is_not_well_formed_is_an_error_of_its_own() {
    let mut host = MockHost::new();
    let Err(err) = run_program("? XMLTOCURSOR(\"<a><b></a>\", \"x\")\n", &mut host) else {
        panic!("a document that is not well formed should fail")
    };
    assert_eq!(err.code, 1429);
    assert!(err.message.contains("well formed"), "{}", err.message);
}

#[test]
fn an_updategram_says_what_a_buffered_table_changed() {
    let mut host = MockHost::new();
    host.tables.insert(
        "PARTS.DBF".into(),
        table(&[("CODE", b'C', 4, 0), ("PRICE", b'N', 8, 2)], &[&["A1", "2.50"], &["B2", "1.75"]]),
    );
    let out = run(
        &mut host,
        "USE parts.dbf
= CURSORSETPROP(\"Buffering\", 5)
GO TOP
REPLACE price WITH 9.99
cGram = XMLUPDATEGRAM()
? OCCURS(\"updg:sync\", cGram) / 2
? IIF(\"2.50\" $ cGram, \"before\", \"no before\"), IIF(\"9.99\" $ cGram, \"after\", \"no after\")
",
    );
    assert_eq!(out[0], "         1.0000");
    assert_eq!(out[1], "before after");
}
