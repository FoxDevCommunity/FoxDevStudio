//! COUNT, SUM, AVERAGE and CALCULATE: one pass over the records, one number per column.

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
fn count_counts_the_records_in_scope() {
    let out = run(
        r#"
USE customer.dbf
COUNT TO n
? n
COUNT FOR amount > 100 TO n
? n
GO TOP
COUNT NEXT 2 TO n
? n
COUNT WHILE amount > 100 TO n
? n
"#,
    );
    assert_eq!(out, vec!["         4", "         3", "         2", "         1"]);
}

#[test]
fn sum_and_average_add_the_columns_up() {
    let out = run(
        r#"
USE customer.dbf
SUM amount TO total
? total
SUM amount, amount * 2 TO one, two
? one, two
AVERAGE amount TO mean
? mean
AVERAGE amount FOR amount < 200 TO mean
? mean
SUM amount TO ARRAY aSums
? ALEN(aSums), aSums(1)
"#,
    );
    assert_eq!(
        out,
        vec![
            "      1455.75",
            "      1455.75       2911.50",
            "       363.94",
            "       102.75",
            "         1       1455.75",
        ]
    );
}

#[test]
fn calculate_works_out_a_column_each() {
    let out = run(
        r#"
USE customer.dbf
CALCULATE CNT(), SUM(amount), AVG(amount), MIN(amount), MAX(amount) TO c, s, a, lo, hi
? c, s, a, lo, hi
CALCULATE MIN(name), MAX(name) TO first, last
? ALLTRIM(first), ALLTRIM(last)
CALCULATE STD(amount), VAR(amount) TO sd, vr
? ROUND(sd, 2), ROUND(vr, 2)
CALCULATE CNT() FOR amount > 200 TO n
? n
"#,
    );
    assert_eq!(
        out,
        vec![
            "         4       1455.75        363.94         80.00       1000.25",
            "Acme Delta",
            "        372.61      138837.07",
            "         2",
        ]
    );
}

#[test]
fn deleted_records_are_left_out_when_they_are_hidden() {
    let bytes = table_with_deleted(
        &[("NAME", b'C', 10, 0), ("AMOUNT", b'N', 8, 2)],
        &[&["Acme", "100.00"], &["Beta", "200.00"], &["Cirrus", "300.00"]],
        &[2],
    );
    let mut host = MockHost::new();
    host.tables.insert("CUSTOMER.DBF".into(), bytes);
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COUNT TO n
? n
SET DELETED ON
COUNT TO n
SUM amount TO total
? n, total
"#,
    );
    assert_eq!(out, vec!["         3", "         2        400.00"]);
}
