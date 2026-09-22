//! What `THROW` leaves behind, and what an error number is worded as.
//!
//! Read off Visual FoxPro 9. Two things there surprise anyone who has only read the reference
//! page: the message of a thrown error says nothing about what was thrown - it is the one
//! sentence `User Thrown Error .` whatever the value was, with the gap where a name would go
//! left open - and the value itself arrives with its own type in `UserValue`, so an object
//! thrown is an object caught. Every sentence the product has for an error number ends in a
//! full stop.

use foxvm::error::RtError;
use foxvm::mock_host::{MockHost, run_program};

fn run(src: &str) -> Vec<String> {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn a_thrown_value_keeps_its_type_and_the_message_is_the_same_sentence() {
    let out = run(
        "LOCAL e\n\
         TRY\n\
           THROW \"boom\"\n\
         CATCH TO e\n\
           ? e.ErrorNo, \"[\" + e.Message + \"]\", VARTYPE(e.UserValue), e.UserValue\n\
         ENDTRY\n\
         TRY\n\
           THROW 42\n\
         CATCH TO e\n\
           ? \"[\" + e.Message + \"]\", VARTYPE(e.UserValue), e.UserValue\n\
         ENDTRY\n\
         TRY\n\
           THROW .T.\n\
         CATCH TO e\n\
           ? \"[\" + e.Message + \"]\", VARTYPE(e.UserValue), e.UserValue\n\
         ENDTRY\n",
    );
    assert_eq!(
        out,
        vec![
            "      2071 [User Thrown Error .] C boom",
            "[User Thrown Error .] N         42",
            "[User Thrown Error .] L .T.",
        ]
    );
}

#[test]
fn an_error_no_one_threw_has_no_user_value() {
    let out = run(
        "LOCAL e\n\
         TRY\n\
           ? nosuchvariable\n\
         CATCH TO e\n\
           ? e.ErrorNo, \"[\" + e.Message + \"]\", VARTYPE(e.UserValue), e.UserValue\n\
         ENDTRY\n",
    );
    // `?` ends the line that is open before it works out what to print, so the one that failed
    // leaves a blank line behind it - which is what the product does
    assert_eq!(out, vec!["", "        12 [Variable 'NOSUCHVARIABLE' is not found.] L .F."]);
}

#[test]
fn error_with_text_writes_that_text_into_the_number_s_own_sentence() {
    // `ERROR 12, "x"` is not an error whose message is "x": it is error 12 about x, so the
    // product words it "Variable 'x' is not found."
    let out = run(
        "LOCAL e\n\
         TRY\n\
           ERROR 12, \"no such thing\"\n\
         CATCH TO e\n\
           ? e.ErrorNo, \"[\" + e.Message + \"]\"\n\
         ENDTRY\n\
         TRY\n\
           ERROR 1734\n\
         CATCH TO e\n\
           ? \"[\" + e.Message + \"]\"\n\
         ENDTRY\n",
    );
    // 1734 keeps its gap open, which is why two spaces show
    assert_eq!(out, vec!["        12 [Variable 'no such thing' is not found.]", "[Property  is not found.]"]);
}

#[test]
fn every_sentence_the_product_has_for_a_number_ends_in_a_full_stop() {
    for code in [1u32, 9, 10, 11, 12, 13, 16, 26, 31, 36, 46, 47, 52, 107, 1001, 1098, 1102, 1162, 1230, 1234,
        1307, 1579, 1588, 1683, 1712, 1713, 1734, 1903, 1924, 1925, 1943, 2009, 2071]
    {
        let alone = RtError::standard_message(code).unwrap_or_else(|| panic!("no words for {code}"));
        assert!(alone.ends_with('.'), "{code}: {alone}");
        let about = RtError::message_about(code, "X").expect("the same numbers");
        assert!(about.ends_with('.'), "{code}: {about}");
    }
}

#[test]
fn the_wordings_are_the_ones_the_product_gives() {
    assert_eq!(RtError::division_by_zero().message, "Cannot divide by 0.");
    assert_eq!(RtError::no_table_open().message, "No table is open in the current work area.");
    assert_eq!(RtError::variable_not_found("nope").message, "Variable 'NOPE' is not found.");
    assert_eq!(RtError::standard_message(12).unwrap(), "Variable is not found.");
    assert_eq!(RtError::not_an_object("nx").message, "NX is not an object.");
    assert_eq!(RtError::unknown_member("gone").message, "Unknown member GONE.");
    assert_eq!(RtError::object_not_valid().message, "Member  does not evaluate to an object.");
    assert_eq!(RtError::standard_message(2071).unwrap(), "User Thrown Error .");
}
