//! `SET PROCEDURE`, `RELEASE PROCEDURE`, and where a bare call or `DO` finds its routine. The
//! behaviour is measured against Visual FoxPro 9 in the `set_procedure` golden; these pin the
//! parse shapes and the load path a host takes.

mod common;

use foxvm::mock_host::{MockHost, run_program};
use foxvm::host::HostRequest;

fn host() -> MockHost {
    let mut host = MockHost::new();
    host.program_sources.insert("LIBA".into(), "? 'liba main ran'\nFUNCTION greet\nRETURN 'greet in liba'\n".into());
    host.program_sources.insert("LIBB".into(), "FUNCTION greet\nRETURN 'greet in libb'\n".into());
    host.program_sources.insert("TOOL".into(), "LPARAMETERS n\nRETURN n * 2\n".into());
    host
}

fn run(src: &str) -> (Vec<String>, MockHost) {
    let mut host = host();
    match run_program(src, &mut host) {
        Ok((_, output)) => (output, host),
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn set_procedure_parses_its_files_and_additive() {
    assert_eq!(common::ok("SET PROCEDURE TO a, b ADDITIVE"), "(set PROCEDURE (to .T., \"a\", \"b\"))");
    assert_eq!(common::ok("SET PROCEDURE TO"), "(set PROCEDURE (to .F.))");
    assert_eq!(common::ok("SET PROCEDURE TO ..\\common50\\codemine"), "(set PROCEDURE (to .F., \"..\\\\common50\\\\codemine\"))");
    assert_eq!(common::ok("RELEASE PROCEDURE a, b"), "(set RELEASE PROCEDURE (to \"a\", \"b\"))");
}

#[test]
fn do_reads_a_name_built_right_against_a_plus() {
    assert_eq!(common::ok("DO p_cod+\"vx_prios.prg\" WITH 1"), "(do-expr (+ P_COD \"vx_prios.prg\") (with 1))");
    // measured: with spaces round the `+` the product calls it a syntax error
    assert!(!foxvm::parser::parse_program("DO p_cod + \"vx_prios\"").diagnostics.is_empty());
}

#[test]
fn a_procedure_file_is_loaded_through_the_host_and_its_main_does_not_run() {
    let (out, host) = run("SET PROCEDURE TO liba\n? greet()");
    assert_eq!(out, ["greet in liba"]);
    assert!(host.requests.iter().any(|r| matches!(r, HostRequest::LoadProgram { name } if name == "LIBA")));
}

#[test]
fn a_released_file_is_not_searched_but_a_program_that_ran_is() {
    let (out, _) = run("SET PROCEDURE TO liba\nSET PROCEDURE TO libb\n? greet()\nRELEASE PROCEDURE libb\nTRY\n  x = greet()\nCATCH TO e\n  ? e.ErrorNo\nENDTRY");
    assert_eq!(out, ["greet in libb", "         1"]);
}

#[test]
fn a_call_to_no_routine_runs_the_program_of_that_name() {
    let (out, _) = run("? TRANSFORM(tool(21))");
    assert_eq!(out, ["42"]);
}

#[test]
fn a_call_to_nothing_at_all_is_error_1_naming_the_file() {
    let mut host = host();
    let e = run_program("x = nosuch()", &mut host).err().expect("an error");
    assert_eq!((e.code, e.message.as_str()), (1, "File 'nosuch.prg' does not exist."));
}
