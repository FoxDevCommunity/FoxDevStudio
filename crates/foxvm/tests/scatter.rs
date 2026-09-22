//! SCATTER and GATHER: a record away from the table and back again.

mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table(
        &[("CUSTNO", b'C', 6, 0), ("NAME", b'C', 10, 0), ("AMOUNT", b'N', 8, 2)],
        &[&["A100", "Acme", "125.50"], &["B200", "Beta", "80.00"]],
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
fn scatter_to_an_array_and_gather_back() {
    let out = run(
        r#"
USE customer.dbf
SCATTER TO aRow
? ALEN(aRow), ALLTRIM(aRow(1)), ALLTRIM(aRow(2)), aRow(3)
aRow(2) = "Acme Ltd"
GATHER FROM aRow
? ALLTRIM(name)
GO 2
? ALLTRIM(name)
GO 1
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         3 A100 Acme        125.50", "Acme Ltd", "Beta", "Acme Ltd"]);
}

#[test]
fn scatter_memvar_makes_a_variable_per_field() {
    let out = run(
        r#"
USE customer.dbf
SCATTER MEMVAR
? ALLTRIM(m.custno), ALLTRIM(m.name), m.amount
m.name = "Acme Ltd"
GATHER MEMVAR
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["A100 Acme        125.50", "Acme Ltd"]);
}

#[test]
fn scatter_name_makes_an_object_with_a_property_per_field() {
    let out = run(
        r#"
USE customer.dbf
SCATTER NAME oRow
? ALLTRIM(oRow.custno), ALLTRIM(oRow.name), oRow.amount
oRow.name = "Acme Ltd"
oRow.amount = 200
GATHER NAME oRow
? ALLTRIM(name), STR(amount, 6, 2)
"#,
    );
    assert_eq!(out, vec!["A100 Acme        125.5", "Acme Ltd 200.00"]);
}

#[test]
fn fields_and_blank_narrow_what_is_taken() {
    let out = run(
        r#"
USE customer.dbf
SCATTER FIELDS name, amount TO aTwo
? ALEN(aTwo), ALLTRIM(aTwo(1)), aTwo(2)
SCATTER FIELDS EXCEPT amount TO aOther
? ALEN(aOther), ALLTRIM(aOther(2))
SCATTER BLANK TO aBlank
? ALEN(aBlank), "[" + aBlank(1) + "]", aBlank(3)
"#,
    );
    assert_eq!(out, vec!["         2 Acme        125.50", "         2 Acme", "         3 [      ]          0.00"]);
}
