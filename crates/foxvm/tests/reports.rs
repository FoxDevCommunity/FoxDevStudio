//! Reports: what a `.frx` says, and what `REPORT FORM` makes of it.
//!
//! The report file is built here byte for byte from the format, so a test that passes says the
//! reader agrees with what Visual FoxPro writes rather than with itself. The field names, the
//! object types and the ten-thousandths of an inch are all as a real `.frx` has them.

mod dbf_fixture;

use dbf_fixture::{LINE, Row, band, field, report_file, sample};
use foxvm::mock_host::{MockHost, run_program};
use foxvm::report::{Band, read_report};

fn host_with_report() -> MockHost {
    let (frx, frt) = sample();
    let mut host = MockHost::new();
    host.files.insert("PARTS.FRX".into(), frx);
    host.files.insert("PARTS.FRT".into(), frt);
    host
}

fn run(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

const MAKE_TABLE: &str = r#"
CREATE TABLE parts (code C(8), name C(10), price N(8,2))
INSERT INTO parts VALUES ("A1", "Bolt", 2.50)
INSERT INTO parts VALUES ("B2", "Nut", 1.75)
"#;

#[test]
fn the_reader_puts_every_object_in_the_band_it_falls_in() {
    let (frx, frt) = sample();
    let report = read_report(&frx, Some(&frt)).expect("it reads");
    assert_eq!(report.bands, vec![(Band::Title, 2), (Band::PageHeader, 2), (Band::Detail, 1), (Band::PageFooter, 1)]);
    assert_eq!(report.band_items(Band::Title).len(), 1);
    assert_eq!(report.band_items(Band::PageHeader).len(), 3);
    let detail = report.band_items(Band::Detail);
    assert_eq!(detail.iter().map(|i| i.expr.as_str()).collect::<Vec<_>>(), vec!["parts.code", "ALLTRIM(parts.name)", "parts.price"]);
    assert_eq!(detail.iter().map(|i| i.col).collect::<Vec<_>>(), vec![0, 12, 26]);
}

#[test]
fn report_form_writes_every_band_in_order() {
    let mut host = host_with_report();
    let out = run(&mut host, &format!("{MAKE_TABLE}\nREPORT FORM parts\n"));
    assert_eq!(
        out,
        vec![
            "          Parts list",
            "",
            "Code        Name          Price",
            "",
            "A1          Bolt          2.50",
            "B2          Nut           1.75",
            "Page 1",
        ]
    );
}

#[test]
fn a_scope_and_a_condition_choose_the_records_it_covers() {
    let mut host = host_with_report();
    let out = run(&mut host, &format!("{MAKE_TABLE}\nREPORT FORM parts FOR price > 2\n"));
    assert!(out.iter().any(|l| l.contains("Bolt")), "{out:?}");
    assert!(!out.iter().any(|l| l.contains("Nut")), "{out:?}");

    let mut host = host_with_report();
    let out = run(&mut host, &format!("{MAKE_TABLE}\nGO TOP
REPORT FORM parts NEXT 1\n"));
    assert!(out.iter().any(|l| l.contains("Bolt")));
    assert!(!out.iter().any(|l| l.contains("Nut")));
}

#[test]
fn summary_leaves_the_detail_out_and_noconsole_leaves_it_all_off() {
    let mut host = host_with_report();
    let out = run(&mut host, &format!("{MAKE_TABLE}\nREPORT FORM parts SUMMARY\n"));
    assert!(!out.iter().any(|l| l.contains("Bolt")));
    assert!(out.iter().any(|l| l.contains("Parts list")));

    let mut host = host_with_report();
    let out = run(&mut host, &format!("{MAKE_TABLE}\nREPORT FORM parts NOCONSOLE\n"));
    assert_eq!(out, Vec::<String>::new());
}

#[test]
fn to_file_writes_the_report_where_it_was_told() {
    let mut host = host_with_report();
    run(&mut host, &format!("{MAKE_TABLE}\nREPORT FORM parts TO FILE parts.txt NOCONSOLE\n"));
    let written = host.files.get("PARTS.TXT").expect("the report was written");
    let text = String::from_utf8_lossy(written);
    assert!(text.contains("Parts list"));
    assert!(text.contains("Bolt"));
}

#[test]
fn print_when_chooses_between_two_ways_of_printing_the_same_line() {
    let detail_h = LINE;
    let (frx, frt) = report_file(&[
        band(4, detail_h),
        Row { print_when: "parts.price > 2".into(), ..field(0.0, 0.0, 12.0, "\"dear: \" + parts.code") },
        Row { print_when: "parts.price <= 2".into(), ..field(0.0, 0.0, 12.0, "\"cheap: \" + parts.code") },
    ]);
    let mut host = MockHost::new();
    host.files.insert("PARTS.FRX".into(), frx);
    host.files.insert("PARTS.FRT".into(), frt);
    let out = run(&mut host, &format!("{MAKE_TABLE}\nREPORT FORM parts\n"));
    assert_eq!(out, vec!["dear: A1", "cheap: B2"]);
}

#[test]
fn a_report_that_is_not_there_is_a_file_error() {
    let mut host = MockHost::new();
    let Err(err) = run_program(&format!("{MAKE_TABLE}\nREPORT FORM nothing\n"), &mut host) else {
        panic!("a report that is not there should fail")
    };
    assert_eq!(err.code, 1);
    assert!(err.message.contains("nothing.frx"), "{}", err.message);
}
