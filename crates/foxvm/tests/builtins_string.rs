//! Character built-ins.

mod ctx;

use ctx::{TestCtx, array, call, d, err, n, num, s, text, text_on, texts, value};
use foxvm::builtins::BuiltinResult;
use foxvm::error::RtError;
use foxvm::value::Value;

#[test]
fn trimming() {
    assert_eq!(text("ALLTRIM", vec![s("  hi  ")]), "hi");
    assert_eq!(text("LTRIM", vec![s("  hi  ")]), "hi  ");
    assert_eq!(text("RTRIM", vec![s("  hi  ")]), "  hi");
    assert_eq!(text("TRIM", vec![s("  hi  ")]), "  hi");
    assert_eq!(text("ALLTRIM", vec![s("")]), "");
    assert_eq!(text("ALLTRIM", vec![s("     ")]), "");
    assert_eq!(text("ALLTRIM", vec![s("**hi**"), s("*")]), "hi");
    assert_eq!(text("ALLTRIM", vec![s("  hi  "), n(0.0)]), "hi");
}

#[test]
fn case_conversion() {
    assert_eq!(text("UPPER", vec![s("Hello, world!")]), "HELLO, WORLD!");
    assert_eq!(text("LOWER", vec![s("Hello, World!")]), "hello, world!");
    assert_eq!(text("PROPER", vec![s("hello world")]), "Hello World");
    assert_eq!(text("PROPER", vec![s("O'BRIEN mcdonald")]), "O'Brien Mcdonald");
    assert_eq!(text("PROPER", vec![s("")]), "");
}

#[test]
fn length_and_substrings() {
    assert_eq!(num("LEN", vec![s("Hello")]), 5.0);
    assert_eq!(num("LEN", vec![s("")]), 0.0);
    assert_eq!(text("SUBSTR", vec![s("Hello"), n(2.0)]), "ello");
    assert_eq!(text("SUBSTR", vec![s("Hello"), n(2.0), n(3.0)]), "ell");
    assert_eq!(text("SUBSTR", vec![s("Hello"), n(3.0), n(99.0)]), "llo");
    assert_eq!(text("SUBSTR", vec![s("Hello"), n(10.0)]), "");
    assert_eq!(text("SUBSTR", vec![s("Hello"), n(0.0), n(2.0)]), "He");
    assert_eq!(text("SUBSTR", vec![s("Hello"), n(2.0), n(0.0)]), "");
    assert_eq!(text("SUBSTR", vec![s("Hello"), n(2.0), n(-4.0)]), "");
    assert_eq!(text("SUBSTR", vec![s(""), n(1.0), n(3.0)]), "");
}

#[test]
fn left_and_right() {
    assert_eq!(text("LEFT", vec![s("Hello"), n(2.0)]), "He");
    assert_eq!(text("LEFT", vec![s("Hello"), n(0.0)]), "");
    assert_eq!(text("LEFT", vec![s("Hello"), n(-3.0)]), "");
    assert_eq!(text("LEFT", vec![s("Hello"), n(50.0)]), "Hello");
    assert_eq!(text("RIGHT", vec![s("Hello"), n(3.0)]), "llo");
    assert_eq!(text("RIGHT", vec![s("Hello"), n(0.0)]), "");
    assert_eq!(text("RIGHT", vec![s("Hello"), n(50.0)]), "Hello");
    assert_eq!(text("RIGHT", vec![s(""), n(2.0)]), "");
}

#[test]
fn searching() {
    assert_eq!(num("AT", vec![s("l"), s("Hello")]), 3.0);
    assert_eq!(num("AT", vec![s("l"), s("Hello"), n(2.0)]), 4.0);
    assert_eq!(num("AT", vec![s("l"), s("Hello"), n(3.0)]), 0.0);
    assert_eq!(num("AT", vec![s("L"), s("Hello")]), 0.0);
    assert_eq!(num("ATC", vec![s("L"), s("Hello")]), 3.0);
    assert_eq!(num("AT", vec![s(""), s("Hello")]), 0.0);
    assert_eq!(num("AT", vec![s("Hello there"), s("Hello")]), 0.0);
    assert_eq!(num("RAT", vec![s("l"), s("Hello")]), 4.0);
    assert_eq!(num("RAT", vec![s("l"), s("Hello"), n(2.0)]), 3.0);
    // RATC minds case: the C on the end is the character set, not the case - measured, in
    // ref_string.prg, where RATC("A", "banana") is 0 and RATC("a", "banana") is 6
    assert_eq!(num("RATC", vec![s("L"), s("Hello")]), 0.0);
    assert_eq!(num("RATC", vec![s("l"), s("Hello")]), 4.0);
    assert_eq!(num("OCCURS", vec![s("l"), s("Hello")]), 2.0);
    assert_eq!(num("OCCURS", vec![s("z"), s("Hello")]), 0.0);
    assert_eq!(num("OCCURS", vec![s("aa"), s("aaaa")]), 3.0);
    // Measured: an occurrence of 0 is error 11, not a lenient clamp up to the first occurrence.
    assert_eq!(err("AT", vec![s("l"), s("Hello"), n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("RAT", vec![s("l"), s("Hello"), n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn padding() {
    assert_eq!(text("PADL", vec![s("7"), n(3.0)]), "  7");
    assert_eq!(text("PADL", vec![s("7"), n(3.0), s("0")]), "007");
    assert_eq!(text("PADR", vec![s("7"), n(3.0), s(".")]), "7..");
    assert_eq!(text("PADC", vec![s("ab"), n(6.0), s("-")]), "--ab--");
    assert_eq!(text("PADC", vec![s("ab"), n(5.0), s("-")]), "-ab--");
    // A non-character value is formatted first.
    assert_eq!(text("PADL", vec![n(42.0), n(5.0), s("0")]), "00042");
    assert_eq!(text("PADR", vec![Value::Logical(true), n(4.0)]), ".T. ");
    // Too long: truncated on the side the alignment would have padded.
    assert_eq!(text("PADL", vec![s("Hello"), n(3.0)]), "llo");
    assert_eq!(text("PADR", vec![s("Hello"), n(3.0)]), "Hel");
    assert_eq!(text("PADC", vec![s("Hello"), n(3.0)]), "ell");
    assert_eq!(text("PADL", vec![s("Hello"), n(0.0)]), "");
}

#[test]
fn space_and_replicate() {
    assert_eq!(text("SPACE", vec![n(3.0)]), "   ");
    assert_eq!(text("SPACE", vec![n(0.0)]), "");
    assert_eq!(text("SPACE", vec![n(-2.0)]), "");
    assert_eq!(text("REPLICATE", vec![s("ab"), n(3.0)]), "ababab");
    assert_eq!(text("REPLICATE", vec![s("ab"), n(0.0)]), "");
    assert_eq!(text("REPLICATE", vec![s(""), n(5.0)]), "");
}

#[test]
fn strtran_occurrence_count_and_flags() {
    assert_eq!(text("STRTRAN", vec![s("a.b.c"), s("."), s("-")]), "a-b-c");
    assert_eq!(text("STRTRAN", vec![s("a.b.c"), s(".")]), "abc");
    assert_eq!(text("STRTRAN", vec![s("aXbXcX"), s("X"), s("-"), n(2.0)]), "aXb-c-");
    assert_eq!(text("STRTRAN", vec![s("aXbXcX"), s("X"), s("-"), n(2.0), n(1.0)]), "aXb-cX");
    assert_eq!(text("STRTRAN", vec![s("AbaB"), s("ab"), s("-"), n(1.0), n(-1.0), n(1.0)]), "--");
    assert_eq!(text("STRTRAN", vec![s("AbaB"), s("ab"), s("-")]), "AbaB");
    assert_eq!(text("STRTRAN", vec![s("abc"), s(""), s("-")]), "abc");
    assert_eq!(text("STRTRAN", vec![s(""), s("a"), s("-")]), "");
    // Measured: an explicit 0 in either occurrence argument is error 11, the same complaint as
    // AT()'s own occurrence of 0 - not "replace nothing" the way a lenient default would read it.
    assert_eq!(err("STRTRAN", vec![s("aXbXcX"), s("X"), s("-"), n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("STRTRAN", vec![s("aXbXcX"), s("X"), s("-"), n(1.0), n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
    // Flag 2 (case-adjust) reshapes the replacement to match each match's own case.
    assert_eq!(
        text("STRTRAN", vec![s("One two ONE"), s("one"), s("x"), n(1.0), n(-1.0), n(3.0)]),
        "X two X"
    );
}

#[test]
fn stuff_and_chrtran() {
    assert_eq!(text("STUFF", vec![s("abcdef"), n(2.0), n(3.0), s("XY")]), "aXYef");
    assert_eq!(text("STUFF", vec![s("abc"), n(2.0), n(0.0), s("-")]), "a-bc");
    assert_eq!(text("STUFF", vec![s("abc"), n(10.0), n(1.0), s("X")]), "abcX");
    assert_eq!(text("STUFF", vec![s("abc"), n(1.0), n(99.0), s("Z")]), "Z");
    assert_eq!(text("CHRTRAN", vec![s("ABC"), s("AB"), s("XY")]), "XYC");
    assert_eq!(text("CHRTRAN", vec![s("ABC"), s("B"), s("")]), "AC");
    assert_eq!(text("CHRTRAN", vec![s("a-b-c"), s("-"), s(" ")]), "a b c");
    assert_eq!(text("CHRTRAN", vec![s(""), s("a"), s("b")]), "");
}

#[test]
fn chr_and_asc() {
    assert_eq!(text("CHR", vec![n(65.0)]), "A");
    assert_eq!(num("ASC", vec![s("A")]), 65.0);
    assert_eq!(num("ASC", vec![s("")]), 0.0);
    assert_eq!(err("CHR", vec![n(300.0)]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn str_formatting() {
    assert_eq!(text("STR", vec![n(1234.5678)]), "      1235");
    assert_eq!(text("STR", vec![n(std::f64::consts::PI), n(8.0), n(2.0)]), "    3.14");
    assert_eq!(text("STR", vec![n(-5.0), n(4.0)]), "  -5");
    assert_eq!(text("STR", vec![n(12345.0), n(3.0)]), "***");
    // Decimals are dropped one at a time until the number fits.
    assert_eq!(text("STR", vec![n(1234.567), n(6.0), n(3.0)]), "1234.6");
    assert_eq!(text("STR", vec![n(0.0), n(1.0)]), "0");
    assert_eq!(text("STR", vec![n(2.5), n(4.0), n(0.0)]), "   3");
    assert_eq!(text("STR", vec![n(-2.5), n(4.0), n(0.0)]), "  -3");
}

#[test]
fn val_reads_a_numeric_prefix() {
    assert_eq!(num("VAL", vec![s("12.5abc")]), 12.5);
    assert_eq!(num("VAL", vec![s("   -3")]), -3.0);
    assert_eq!(num("VAL", vec![s("+7")]), 7.0);
    assert_eq!(num("VAL", vec![s("1.2.3")]), 1.2);
    assert_eq!(num("VAL", vec![s("abc")]), 0.0);
    assert_eq!(num("VAL", vec![s("")]), 0.0);
    assert_eq!(num("VAL", vec![s("-")]), 0.0);
    assert_eq!(num("VAL", vec![s("007")]), 7.0);
}

#[test]
fn transform_masks() {
    assert_eq!(text("TRANSFORM", vec![n(1234.5)]), "1234.50");
    assert_eq!(text("TRANSFORM", vec![s("abc")]), "abc");
    assert_eq!(text("TRANSFORM", vec![Value::Logical(true)]), ".T.");
    assert_eq!(text("TRANSFORM", vec![s("abc"), s("@!")]), "ABC");
    assert_eq!(text("TRANSFORM", vec![n(5.0), s("@L 999")]), "005");
    assert_eq!(text("TRANSFORM", vec![n(1234567.891), s("9,999,999.99")]), "1,234,567.89");
    assert_eq!(text("TRANSFORM", vec![n(12.0), s("99999")]), "   12");
    assert_eq!(text("TRANSFORM", vec![n(0.0), s("@Z 9999")]), "    ");
    assert_eq!(text("TRANSFORM", vec![s("  x  "), s("@T")]), "x");
    assert_eq!(text("TRANSFORM", vec![d(2026, 9, 7), s("@D")]), "09/07/26");
    // A literal template keeps the picture characters.
    assert_eq!(text("TRANSFORM", vec![s("8095551234"), s("@R (999) 999-9999")]), "(809) 555-1234");
    // Unsupported masks fall back to the plain display without an error.
    assert_eq!(text("TRANSFORM", vec![s("abc"), s("@Q")]), "abc");
    assert_eq!(text("TRANSFORM", vec![Value::Null]), ".NULL.");
}

#[test]
fn transform_follows_set_decimals() {
    let mut ctx = TestCtx::with_settings(|s| s.decimals = 4);
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1.5)]), "1.5000");
}

#[test]
fn words() {
    assert_eq!(num("GETWORDCOUNT", vec![s("one two  three")]), 3.0);
    assert_eq!(num("GETWORDCOUNT", vec![s("")]), 0.0);
    assert_eq!(num("GETWORDCOUNT", vec![s("a,b,c"), s(",")]), 3.0);
    assert_eq!(text("GETWORDNUM", vec![s("one two three"), n(2.0)]), "two");
    assert_eq!(text("GETWORDNUM", vec![s("one two three"), n(9.0)]), "");
    assert_eq!(text("GETWORDNUM", vec![s("one two three"), n(0.0)]), "");
    assert_eq!(text("GETWORDNUM", vec![s("a,b,c"), n(3.0), s(",")]), "c");
}

#[test]
fn strextract() {
    assert_eq!(text("STREXTRACT", vec![s("<a>one</a>"), s("<a>"), s("</a>")]), "one");
    assert_eq!(text("STREXTRACT", vec![s("[1][2][3]"), s("["), s("]"), n(2.0)]), "2");
    assert_eq!(text("STREXTRACT", vec![s("<A>one</A>"), s("<a>"), s("</a>"), n(1.0), n(1.0)]), "one");
    // Measured against vfp9.exe: flag 2 is the missing end delimiter being fine (read to the
    // end), flag 4 is keeping the delimiters - the reverse of what this runtime had them as.
    assert_eq!(text("STREXTRACT", vec![s("<a>one</a>"), s("<a>"), s("</a>"), n(1.0), n(4.0)]), "<a>one</a>");
    assert_eq!(text("STREXTRACT", vec![s("<a>one"), s("<a>"), s("</a>")]), "");
    assert_eq!(text("STREXTRACT", vec![s("<a>one"), s("<a>"), s("</a>"), n(1.0), n(2.0)]), "one");
    assert_eq!(text("STREXTRACT", vec![s("key=value"), s("=")]), "value");
    assert_eq!(text("STREXTRACT", vec![s("abc"), s("<")]), "");
}

#[test]
fn alines_fills_the_array() {
    let arr = array(vec![Value::Logical(false)]);
    assert_eq!(num("ALINES", vec![arr.clone(), s("one\r\ntwo\nthree\n")]), 3.0);
    assert_eq!(texts(&arr), vec!["one", "two", "three"]);

    let arr = array(vec![Value::Logical(false)]);
    assert_eq!(num("ALINES", vec![arr.clone(), s(" a ; b "), Value::Logical(true), s(";")]), 2.0);
    assert_eq!(texts(&arr), vec!["a", "b"]);

    // Through a by-reference cell that does not hold an array yet.
    let cell = Value::Ref(std::rc::Rc::new(std::cell::RefCell::new(Value::Logical(false))));
    assert_eq!(num("ALINES", vec![cell.clone(), s("x\ny")]), 2.0);
    assert_eq!(texts(&cell), vec!["x", "y"]);

    let arr = array(vec![Value::Logical(false)]);
    assert_eq!(num("ALINES", vec![arr.clone(), s("")]), 1.0);
    assert_eq!(texts(&arr), vec![""]);
}

#[test]
fn null_propagates() {
    for name in ["UPPER", "LOWER", "ALLTRIM", "LEN", "PROPER", "VAL", "ASC"] {
        assert!(value(name, vec![Value::Null]).is_null(), "{name}() should return NULL");
    }
    assert!(value("SUBSTR", vec![Value::Null, n(1.0)]).is_null());
    assert!(value("AT", vec![s("a"), Value::Null]).is_null());
    assert!(value("STRTRAN", vec![Value::Null, s("a"), s("b")]).is_null());
}

#[test]
fn wrong_types_are_reported() {
    assert_eq!(err("LEN", vec![n(1.0)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("UPPER", vec![d(2026, 9, 7)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("STR", vec![s("x")]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn non_ascii_input_does_not_panic() {
    // A character above 127 is one byte, not two: measured, in ref_bytes_are_bytes.prg. Rust
    // holds the string as UTF-8, where it takes two, and counting the storage rather than the
    // characters shifted every field a program cut out of a Windows structure.
    assert_eq!(num("LEN", vec![s("caf\u{e9}")]), 4.0);
    assert_eq!(text("UPPER", vec![s("caf\u{e9}")]), "CAF\u{e9}");
    // Cutting a multi-byte character is lossy but must not panic.
    assert!(matches!(call("SUBSTR", vec![s("caf\u{e9}"), n(1.0), n(4.0)]), Ok(BuiltinResult::Value(Value::Str(_)))));
    assert_eq!(num("AT", vec![s("\u{e9}"), s("caf\u{e9}")]), 4.0);
}
