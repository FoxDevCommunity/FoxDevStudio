//! `BUILD` and `COMPILE`: what a program that builds another program hands the host.
//!
//! Building is the IDE's to do - it reads the project, compiles what is in it and writes the
//! file - so all the VM owes is the request, with the file names resolved the way every other
//! command resolves them.

use foxvm::host::HostRequest;
use foxvm::mock_host::{MockHost, run_program};

fn run(src: &str) -> MockHost {
    let mut host = MockHost::new();
    match run_program(src, &mut host) {
        Ok(_) => host,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn builds(host: &MockHost) -> Vec<&HostRequest> {
    host.requests.iter().filter(|r| matches!(r, HostRequest::Build { .. } | HostRequest::Compile { .. })).collect()
}

#[test]
fn build_app_names_the_file_and_the_project_it_comes_from() {
    let host = run("BUILD APP orders FROM orders.fxp");
    match builds(&host).as_slice() {
        [HostRequest::Build { what, target, from, recompile }] => {
            assert_eq!(what, "APP");
            assert!(target.ends_with("orders"), "{target}");
            assert_eq!(from.len(), 1);
            assert!(from[0].ends_with("orders.fxp"), "{}", from[0]);
            assert!(!recompile);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn recompile_is_read_wherever_the_command_puts_it() {
    // BUILD EXE takes it last; BUILD PROJECT takes it before FROM
    let host = run(
        "BUILD EXE ship FROM ship.fxp RECOMPILE
BUILD PROJECT ship RECOMPILE FROM main.prg, entry.fxf",
    );
    match builds(&host).as_slice() {
        [HostRequest::Build { what: exe, recompile: exe_again, .. }, HostRequest::Build { what: project, from, recompile: project_again, .. }] =>
        {
            assert_eq!((exe.as_str(), *exe_again), ("EXE", true));
            assert_eq!((project.as_str(), *project_again), ("PROJECT", true));
            assert_eq!(from.len(), 2, "both files the project is made of: {from:?}");
            assert!(from[0].ends_with("main.prg") && from[1].ends_with("entry.fxf"), "{from:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn build_project_on_its_own_refreshes_what_is_already_there() {
    let host = run("BUILD PROJECT ship");
    match builds(&host).as_slice() {
        [HostRequest::Build { what, from, .. }] => {
            assert_eq!(what, "PROJECT");
            assert!(from.is_empty(), "no FROM clause means refresh: {from:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn compile_reads_the_kind_the_file_is_and_the_clauses_after_it() {
    let host = run(
        "COMPILE *.prg
COMPILE FORM entry
COMPILE report.prg ENCRYPT NODEBUG",
    );
    match builds(&host).as_slice() {
        [
            HostRequest::Compile { what: plain, files: skeleton, .. },
            HostRequest::Compile { what: form, files: entry, .. },
            HostRequest::Compile { what: also_plain, encrypt, nodebug, .. },
        ] => {
            assert_eq!(plain, "", "a program has no word before its name");
            assert!(skeleton.ends_with("*.prg"), "{skeleton}");
            assert_eq!(form, "FORM");
            assert!(entry.ends_with("entry"), "{entry}");
            assert_eq!((also_plain.as_str(), *encrypt, *nodebug), ("", true, true));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn build_of_something_that_is_not_a_kind_is_a_syntax_error() {
    let mut host = MockHost::new();
    let Err(err) = run_program("BUILD WIDGET thing", &mut host) else { panic!("BUILD WIDGET compiled") };
    assert!(err.message.contains("APP, EXE, DLL, MTDLL and PROJECT"), "{}", err.message);
}
