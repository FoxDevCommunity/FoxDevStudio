//! Macro substitution, and the system variables a program finds already there.
//!
//! Both were read off Visual FoxPro 9 rather than off a reference page, and both had a belief
//! written into this runtime that the product disagreed with.
//!
//! A macro is a pass over the characters of the line before the line is read, and nothing else.
//! That means it works inside quotes too: `? "&lcScope"` prints what lcScope holds, and a macro
//! holding a quote closes the string it sits in. It also means a name that is not a character
//! variable is no error - the text is simply left as the program wrote it.

use foxvm::mock_host::{MockHost, run_program};
use foxvm::value::Value;
use foxvm::vm::Vm;

fn run(src: &str) -> Vec<String> {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn a_macro_inside_a_string_is_put_into_the_line_like_any_other() {
    let out = run(
        "LOCAL lcScope, c\n\
         lcScope = \"FOR n > 1\"\n\
         ? \"&lcScope\"\n\
         ? [&lcScope]\n\
         ? 'a' + \"&lcScope\" + 'b'\n\
         c = \"abc\"\n\
         ? LEN(\"&c\"), UPPER(\"&c\"), \"&c-&c\"\n",
    );
    assert_eq!(out, vec!["FOR n > 1", "FOR n > 1", "aFOR n > 1b", "         3 ABC abc-abc"]);
}

#[test]
fn the_text_goes_into_the_line_rather_than_into_the_value_of_the_string() {
    // c holds a quote, so what it stands for closes the string it was written inside and the
    // rest of the line is read as source: `? 'x' + 'y'` prints xy.
    let out = run("LOCAL c\nc = \"x' + 'y\"\n? '&c'\n");
    assert_eq!(out, vec!["xy"]);
}

#[test]
fn a_name_that_is_not_a_character_variable_leaves_the_text_alone() {
    let out = run(
        "LOCAL n\n\
         n = 42\n\
         ? \"n=&n.\"\n\
         ? \"x&nosuchvar.y\"\n\
         ? \"100 & 200\"\n",
    );
    assert_eq!(out, vec!["n=&n.", "x&nosuchvar.y", "100 & 200"]);
}

#[test]
fn what_a_macro_stands_for_is_read_again_for_macros_of_its_own() {
    let out = run("LOCAL a, c\nc = \"abc\"\na = \"&c\"\n? \"[&a.]\"\n");
    assert_eq!(out, vec!["[abc]"]);
}

#[test]
fn the_system_variables_start_where_visual_foxpro_starts_them() {
    let vm = Vm::new();
    let read = |name: &str| vm.get_global(name).unwrap_or_else(|| panic!("{name} is not there"));
    assert_eq!(read("_INCSEEK"), Value::number(0.5));
    assert_eq!(read("_DBLCLICK"), Value::number(0.5));
    // a number, not the empty string a reference page might lead you to write
    assert_eq!(read("_CALCMEM"), Value::number(0.0));
    assert_eq!(read("_ASCIICOLS"), Value::number(80.0));
    assert_eq!(read("_ASCIIROWS"), Value::number(63.0));
    assert_eq!(read("_RMARGIN"), Value::number(80.0));
    assert_eq!(read("_LMARGIN"), Value::number(0.0));
    assert_eq!(read("_WRAP"), Value::Logical(false));
    assert_eq!(read("_BOX"), Value::Logical(true));
    assert_eq!(read("_ALIGNMENT"), Value::str("LEFT"));
    assert_eq!(read("_PADVANCE"), Value::str("FORMFEED"));
    assert_eq!(read("_PEJECT"), Value::str("NONE"));
    assert_eq!(read("_PPITCH"), Value::str("DEFAULT"));
    assert_eq!(read("_PEPAGE"), Value::number(32767.0));
    assert_eq!(read("_PLENGTH"), Value::number(66.0));
    assert_eq!(read("_SPELLCHK"), Value::str(""));
    assert_eq!(read("_TEXT"), Value::number(-1.0));
}

#[test]
fn program_below_the_first_level_is_the_first_level() {
    // PROGRAM(0) is not the empty string: the levels start at one and 0 asks for the first.
    let out = run("? PROGRAM(0) == PROGRAM(1), PROGRAM(-1)\n");
    assert_eq!(out, vec![".T.          1"]);
}

#[test]
fn the_name_of_a_routine_is_upper_cased_however_it_was_written() {
    let out = run(
        "? PROGRAM(), SYS(16)\n\
         DO MixedCaseName\n\
         PROCEDURE MixedCaseName\n\
         ? PROGRAM(), PROGRAM(1), PROGRAM(2), SYS(16)\n\
         RETURN\n",
    );
    assert_eq!(out, vec!["MAIN MAIN", "MIXEDCASENAME MAIN MIXEDCASENAME MIXEDCASENAME"]);
}

#[test]
fn an_error_says_which_routine_it_happened_in_upper_cased_too() {
    let Err(e) = run_program("DO Broken\nPROCEDURE Broken\n? nope\n", &mut MockHost::new()) else {
        panic!("the procedure reads a name that is not there");
    };
    assert_eq!(e.program, "BROKEN");
    assert_eq!(e.line, 3);
}
