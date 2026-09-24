//! The preprocessor directives: constants, the conditionals that use them, and header files.

mod common;

use std::collections::HashMap;

use common::dump;
use foxvm::parser::{parse_program, parse_program_with};

/// The statements a source parses to, with the directive lines left out.
fn code(src: &str) -> String {
    let out = parse_program(src);
    assert!(
        out.diagnostics.iter().all(|d| !d.is_error()),
        "{:?}",
        out.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
    dump(&out.program).lines().filter(|l| !l.contains("(directive")).collect::<Vec<_>>().join(" ")
}

#[test]
fn a_constant_stands_for_what_it_was_defined_as() {
    assert_eq!(code("#DEFINE MAX 10\nx = MAX\n"), "(= X 10)");
    assert_eq!(code("#DEFINE MAX 10\n#UNDEF MAX\nx = MAX\n"), "(= X MAX)");
}

#[test]
fn a_conditional_keeps_one_half_and_drops_the_other() {
    assert_eq!(code("#DEFINE DEBUG 1\n#IFDEF DEBUG\nx = 1\n#ELSE\nx = 2\n#ENDIF\ny = 3\n"), "(= X 1) (= Y 3)");
    assert_eq!(code("#IFDEF NOTHING\nx = 1\n#ELSE\nx = 2\n#ENDIF\ny = 3\n"), "(= X 2) (= Y 3)");
    assert_eq!(code("#IFNDEF NOTHING\nx = 1\n#ENDIF\n"), "(= X 1)");
    assert_eq!(code("#DEFINE OFF 0\n#IF OFF\nx = 1\n#ELSE\nx = 2\n#ENDIF\n"), "(= X 2)");
    assert_eq!(code("#DEFINE ON 1\n#IF ON\nx = 1\n#ENDIF\n"), "(= X 1)");
}

#[test]
fn a_conditional_inside_another_is_skipped_whole() {
    let src = "#DEFINE ON 1\n#IF ON\n#IFDEF NOTHING\nx = 9\n#ELSE\nx = 5\n#ENDIF\n#ELSE\nx = 2\n#ENDIF\n";
    assert_eq!(code(src), "(= X 5)");
    let src = "#IFDEF NOTHING\n#IFDEF ALSO_NOTHING\nx = 9\n#ENDIF\nx = 8\n#ELSE\nx = 1\n#ENDIF\n";
    assert_eq!(code(src), "(= X 1)");
}

#[test]
fn an_included_header_brings_its_constants() {
    let mut headers = HashMap::new();
    headers.insert("SIZES".to_string(), "#DEFINE WIDTH 80\n#DEFINE DEBUG 1\n".to_string());
    headers.insert("MORE".to_string(), "#INCLUDE \"sizes.h\"\n#DEFINE HEIGHT 25\n".to_string());

    let out = parse_program_with("#INCLUDE \"sizes.h\"\nx = WIDTH\n#IFDEF DEBUG\ny = 1\n#ENDIF\n", &headers);
    assert!(out.diagnostics.iter().all(|d| !d.is_error()));
    let text = dump(&out.program).lines().filter(|l| !l.contains("(directive")).collect::<Vec<_>>().join(" ");
    assert_eq!(text, "(= X 80) (= Y 1)");

    // a header that includes another brings both
    let out = parse_program_with("#INCLUDE more.h\nx = WIDTH + HEIGHT\n", &headers);
    let text = dump(&out.program).lines().filter(|l| !l.contains("(directive")).collect::<Vec<_>>().join(" ");
    assert_eq!(text, "(= X (+ 80 25))");

    // headers that include themselves, or each other, are read once each rather than forever
    let mut looped = HashMap::new();
    looped.insert("SELF".to_string(), "#INCLUDE self.h\n#DEFINE A 1\n".to_string());
    looped.insert("PING".to_string(), "#INCLUDE pong.h\n#DEFINE B 2\n".to_string());
    looped.insert("PONG".to_string(), "#INCLUDE ping.h\n#DEFINE C 3\n".to_string());
    let out = parse_program_with("#INCLUDE self.h\n#INCLUDE ping.h\nx = A + B + C\n", &looped);
    let text = dump(&out.program).lines().filter(|l| !l.contains("(directive")).collect::<Vec<_>>().join(" ");
    assert_eq!(text, "(= X (+ (+ 1 2) 3))");

    // and one that is not there is a warning, not an error
    let out = parse_program("#INCLUDE nowhere.h\nx = 1\n");
    assert!(out.diagnostics.iter().all(|d| !d.is_error()));
    assert!(out.diagnostics.iter().any(|d| d.message.contains("was not found")), "{:?}", out.diagnostics);
}
