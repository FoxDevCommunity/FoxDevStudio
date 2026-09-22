use foxvm::ast::{DateLit, TimeLit};
use foxvm::diagnostics::Severity;
use foxvm::lexer::{TokKind, Token, lex};

fn kinds(src: &str) -> Vec<TokKind> {
    let out = lex(src);
    assert!(out.diagnostics.is_empty(), "unexpected diagnostics: {:#?}", out.diagnostics);
    out.tokens.into_iter().map(|t| t.kind).collect()
}

fn ident(s: &str) -> TokKind {
    TokKind::Ident(s.to_string())
}

/// A number token as the lexer makes one: the value and the width it was written in.
fn num(n: f64) -> TokKind {
    let text = if n == n.trunc() { format!("{n:.0}") } else { format!("{n}") };
    let w = foxvm::value::Width::written(&text);
    TokKind::Num(n, w.chars, w.decimals)
}

fn string(s: &str) -> TokKind {
    TokKind::Str(s.to_string())
}

#[test]
fn star_comment_lines_are_skipped() {
    let k = kinds("* a comment\n  * indented comment\nx = 1\n");
    assert_eq!(k, vec![ident("x"), TokKind::Eq, num(1.0), TokKind::Newline, TokKind::Eof]);
}

#[test]
fn star_mid_line_is_multiplication() {
    let k = kinds("x = 2 * 3");
    assert_eq!(
        k,
        vec![
            ident("x"),
            TokKind::Eq,
            num(2.0),
            TokKind::Star,
            num(3.0),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn note_comment_lines_are_skipped() {
    let k = kinds("NOTE this is ignored\nnote so is this\nNOTES = 1\n");
    assert_eq!(k, vec![ident("NOTES"), TokKind::Eq, num(1.0), TokKind::Newline, TokKind::Eof]);
}

#[test]
fn double_ampersand_comment_mid_line() {
    let k = kinds("x = 1 && trailing comment\ny = 2");
    assert_eq!(
        k,
        vec![
            ident("x"),
            TokKind::Eq,
            num(1.0),
            TokKind::Newline,
            ident("y"),
            TokKind::Eq,
            num(2.0),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn continuation_joins_lines_including_trailing_comment() {
    let out = lex("x = 1 + ;  && continued\n    2 ;\n    + 3\ny = 4");
    assert!(out.diagnostics.is_empty());
    let kinds: Vec<TokKind> = out.tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            ident("x"),
            TokKind::Eq,
            num(1.0),
            TokKind::Plus,
            num(2.0),
            TokKind::Plus,
            num(3.0),
            TokKind::Newline,
            ident("y"),
            TokKind::Eq,
            num(4.0),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
    // physical lines are preserved on the tokens
    assert_eq!(out.tokens[4].line, 2);
    assert_eq!(out.tokens[6].line, 3);
    assert_eq!(out.tokens[8].line, 4);
}

#[test]
fn semicolon_elsewhere_is_a_character_of_the_command() {
    // Visual FoxPro continues a line only on a `;` that ends it. One with more of the line
    // after it is an ordinary character, which is how `SET PATH TO c:\a;c:\b` names two
    // directories; whether a command can use it is the parser's business, not the lexer's - and
    // a line in the half of a `#IF` that was thrown away is never read by anyone.
    let out = lex("x = 1; y = 2");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(out.tokens[3].kind, TokKind::Semi);
    let out = lex("SET PATH TO c:\\a;c:\\b");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}

#[test]
fn blank_lines_collapse_to_one_newline() {
    let k = kinds("\n\nx = 1\n\n\n\ny = 2\n\n");
    let newlines = k.iter().filter(|k| **k == TokKind::Newline).count();
    assert_eq!(newlines, 2);
}

#[test]
fn string_literal_forms() {
    let k = kinds(r#"x = "double 'inner'" + 'single "inner"' + [bracket "both" 'kinds']"#);
    assert_eq!(k[2], string("double 'inner'"));
    assert_eq!(k[4], string("single \"inner\""));
    assert_eq!(k[6], string("bracket \"both\" 'kinds'"));
}

#[test]
fn bracket_after_identifier_is_index() {
    let k = kinds("a[1] = b[2, 3] + [str]");
    assert_eq!(
        k,
        vec![
            ident("a"),
            TokKind::LBracket,
            num(1.0),
            TokKind::RBracket,
            TokKind::Eq,
            ident("b"),
            TokKind::LBracket,
            num(2.0),
            TokKind::Comma,
            num(3.0),
            TokKind::RBracket,
            TokKind::Plus,
            string("str"),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn nested_index_brackets() {
    let k = kinds("x = a[b[1]]");
    assert_eq!(
        k,
        vec![
            ident("x"),
            TokKind::Eq,
            ident("a"),
            TokKind::LBracket,
            ident("b"),
            TokKind::LBracket,
            num(1.0),
            TokKind::RBracket,
            TokKind::RBracket,
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn bracket_after_paren_and_bracket_is_index() {
    let k = kinds("x = f(1)[2][3]");
    assert_eq!(
        k[3..],
        [
            TokKind::LParen,
            num(1.0),
            TokKind::RParen,
            TokKind::LBracket,
            num(2.0),
            TokKind::RBracket,
            TokKind::LBracket,
            num(3.0),
            TokKind::RBracket,
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn bracket_with_space_is_string() {
    let k = kinds("x = a [str]");
    assert_eq!(k[2], ident("a"));
    assert_eq!(k[3], string("str"));
}

#[test]
fn number_forms() {
    let k = kinds("? 123, 1.5, .5, 1e3, 1.5E-2, 0x1F, 7.");
    let nums: Vec<f64> = k.iter().filter_map(|k| if let TokKind::Num(n, ..) = k { Some(*n) } else { None }).collect();
    assert_eq!(nums, vec![123.0, 1.5, 0.5, 1000.0, 0.015, 31.0, 7.0]);
    // `7.` is the number 7 followed by a dot
    assert_eq!(k[k.len() - 4], num(7.0));
    assert_eq!(k[k.len() - 3], TokKind::Dot);
}

#[test]
fn date_and_datetime_literals() {
    let d = DateLit { year: 2024, month: 1, day: 31 };
    let k = kinds(
        "? {^2024-01-31}, {^2024-01-31 10:30}, {^2024-01-31 10:30:15}, {^2024-01-31 10:30 PM}, {^2024-01-31 12:05 AM}",
    );
    let lits: Vec<&TokKind> = k.iter().filter(|k| matches!(k, TokKind::Date(_) | TokKind::DateTime(_))).collect();
    assert_eq!(lits[0], &TokKind::Date(Some(d)));
    assert_eq!(lits[1], &TokKind::DateTime(Some((d, TimeLit { hour: 10, minute: 30, second: 0 }))));
    assert_eq!(lits[2], &TokKind::DateTime(Some((d, TimeLit { hour: 10, minute: 30, second: 15 }))));
    assert_eq!(lits[3], &TokKind::DateTime(Some((d, TimeLit { hour: 22, minute: 30, second: 0 }))));
    assert_eq!(lits[4], &TokKind::DateTime(Some((d, TimeLit { hour: 0, minute: 5, second: 0 }))));
}

#[test]
fn empty_date_forms() {
    let k = kinds("? {}, {//}, {/:}, {  /  /  }, { / / : }");
    let lits: Vec<&TokKind> = k.iter().filter(|k| matches!(k, TokKind::Date(_) | TokKind::DateTime(_))).collect();
    assert_eq!(
        lits,
        vec![
            &TokKind::Date(None),
            &TokKind::Date(None),
            &TokKind::DateTime(None),
            &TokKind::Date(None),
            &TokKind::DateTime(None)
        ]
    );
}

#[test]
fn invalid_date_is_reported() {
    let out = lex("x = {2024-01-31}\ny = {^2024-13-01}\nz = {^2024-01-01");
    let msgs: Vec<&str> = out.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(msgs, vec!["Invalid date/datetime constant"; 3]);
    assert_eq!(out.diagnostics[1].line, 2);
    assert_eq!(out.diagnostics[1].col, 5);
}

#[test]
fn dot_constants() {
    let k = kinds("? .T., .f., .Y., .n., .NULL., .null., .AND., .or., .Not.");
    let consts: Vec<&TokKind> = k
        .iter()
        .filter(|k| !matches!(k, TokKind::Comma | TokKind::Question | TokKind::Newline | TokKind::Eof))
        .collect();
    assert_eq!(
        consts,
        vec![
            &TokKind::True,
            &TokKind::False,
            &TokKind::True,
            &TokKind::False,
            &TokKind::Null,
            &TokKind::Null,
            &TokKind::DotAnd,
            &TokKind::DotOr,
            &TokKind::DotNot,
        ]
    );
}

#[test]
fn dot_after_identifier_is_member_access() {
    let k = kinds("z = x.t.y");
    assert_eq!(
        k,
        vec![
            ident("z"),
            TokKind::Eq,
            ident("x"),
            TokKind::Dot,
            ident("t"),
            TokKind::Dot,
            ident("y"),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn dot_starting_member_inside_with() {
    let k = kinds(".Caption = .T.");
    assert_eq!(k, vec![TokKind::Dot, ident("Caption"), TokKind::Eq, TokKind::True, TokKind::Newline, TokKind::Eof]);
}

#[test]
fn text_endtext_captures_raw_lines() {
    let src = "TEXT TO x TEXTMERGE NOSHOW\n  Hello <<name>>,\n  * not a comment && nor this\n  \"unterminated\nENDTEXT\ny = 1\n";
    let out = lex(src);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let k: Vec<TokKind> = out.tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        k,
        vec![
            ident("TEXT"),
            ident("TO"),
            ident("x"),
            ident("TEXTMERGE"),
            ident("NOSHOW"),
            TokKind::Newline,
            TokKind::RawText("  Hello <<name>>,\n  * not a comment && nor this\n  \"unterminated".to_string()),
            ident("ENDTEXT"),
            TokKind::Newline,
            ident("y"),
            TokKind::Eq,
            num(1.0),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
    assert_eq!(out.tokens[7].line, 5);
    assert_eq!(out.tokens[9].line, 6);
}

#[test]
fn text_endtext_accepts_abbreviation_and_crlf() {
    let out = lex("text\r\nline one\r\nline two\r\n  endt\r\n");
    assert!(out.diagnostics.is_empty());
    assert_eq!(out.tokens[2].kind, TokKind::RawText("line one\nline two".to_string()));
    assert_eq!(out.tokens[3].kind, ident("endt"));
}

#[test]
fn text_without_endtext_is_reported_at_text_line() {
    let out = lex("x = 1\nTEXT\nabc\ndef");
    assert_eq!(out.diagnostics.len(), 1);
    assert_eq!(out.diagnostics[0].message, "TEXT without matching ENDTEXT");
    assert_eq!((out.diagnostics[0].line, out.diagnostics[0].col), (2, 1));
    assert!(out.tokens.iter().any(|t| t.kind == TokKind::RawText("abc\ndef".to_string())));
}

#[test]
fn text_assignment_is_not_a_text_block() {
    let k = kinds("text = \"hi\"\ny = text + \"!\"");
    assert_eq!(k[0], ident("text"));
    assert_eq!(k[2], string("hi"));
    assert!(!k.iter().any(|k| matches!(k, TokKind::RawText(_))));
}

#[test]
fn unterminated_string_has_correct_position_and_keeps_lexing() {
    let out = lex("x = 1\ny = \"oops\nz = 2");
    assert_eq!(out.diagnostics.len(), 1);
    let d = &out.diagnostics[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.message, "Unterminated string: missing closing \"");
    assert_eq!((d.line, d.col), (2, 5));
    assert_eq!((d.end_line, d.end_col), (2, 10));
    let k: Vec<TokKind> = out.tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        k[4..],
        [
            ident("y"),
            TokKind::Eq,
            string("oops"),
            TokKind::Newline,
            ident("z"),
            TokKind::Eq,
            num(2.0),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn operators() {
    let k = kinds("x = a + b - c * d / e % f ^ g ** h");
    let ops: Vec<&TokKind> = k.iter().filter(|k| !matches!(k, TokKind::Ident(_))).collect();
    assert_eq!(
        ops,
        vec![
            &TokKind::Eq,
            &TokKind::Plus,
            &TokKind::Minus,
            &TokKind::Star,
            &TokKind::Slash,
            &TokKind::Percent,
            &TokKind::Caret,
            &TokKind::Caret,
            &TokKind::Newline,
            &TokKind::Eof
        ]
    );
    let k = kinds("x = a = b == c <> d # e != f < g <= h > i >= j $ k");
    let ops: Vec<&TokKind> = k.iter().filter(|k| !matches!(k, TokKind::Ident(_))).collect();
    assert_eq!(
        ops,
        vec![
            &TokKind::Eq,
            &TokKind::Eq,
            &TokKind::EqEq,
            &TokKind::Ne,
            &TokKind::Ne,
            &TokKind::Ne,
            &TokKind::Lt,
            &TokKind::Le,
            &TokKind::Gt,
            &TokKind::Ge,
            &TokKind::Dollar,
            &TokKind::Newline,
            &TokKind::Eof
        ]
    );
    let k = kinds("f(@x, !y, &z., a:b)");
    assert_eq!(
        k,
        vec![
            ident("f"),
            TokKind::LParen,
            TokKind::At,
            ident("x"),
            TokKind::Comma,
            TokKind::Bang,
            ident("y"),
            TokKind::Comma,
            TokKind::Amp,
            ident("z"),
            TokKind::Dot,
            TokKind::Comma,
            ident("a"),
            TokKind::Colon,
            ident("b"),
            TokKind::RParen,
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn hash_and_question_at_line_start() {
    let k = kinds("#DEFINE X 1\n? X # 2\n?? X");
    assert_eq!(k[0], TokKind::Hash);
    assert_eq!(k[5], TokKind::Question);
    assert_eq!(k[7], TokKind::Ne);
    assert_eq!(k[10], TokKind::DoubleQuestion);
}

#[test]
fn question_in_the_middle_of_a_line_is_still_a_token() {
    // `USE ?` asks for a file, so the lexer reads a question mark wherever it is written and
    // leaves the parser to say whether it means anything there
    let out = lex("USE ?");
    assert!(out.diagnostics.is_empty());
    assert_eq!(out.tokens[1].kind, TokKind::Question);
}

#[test]
fn token_spans_are_char_offsets_and_lines() {
    let out = lex("a = 'é'\nx = 'ü'");
    assert!(out.diagnostics.is_empty());
    let t: &Token = &out.tokens[4];
    assert_eq!(out.tokens[2].kind, string("é"));
    assert_eq!(t.kind, ident("x"));
    assert_eq!((t.span.start, t.span.end), (8, 9));
    assert_eq!(t.line, 2);
    assert!(t.is_word("X"));
    assert!(!t.is_word("Y"));
}

#[test]
fn comment_line_ending_in_semicolon_continues_the_comment() {
    let k = kinds("* comment ;\n  still comment\nx = 1");
    assert_eq!(k, vec![ident("x"), TokKind::Eq, num(1.0), TokKind::Newline, TokKind::Eof]);
}

#[test]
fn a_backslash_line_keeps_every_character_after_the_marker() {
    let k = kinds("\\  two spaces\n\\\\ same line\n\\\n");
    assert_eq!(
        k,
        vec![
            TokKind::TextMark(false),
            TokKind::RawText("  two spaces".to_string()),
            TokKind::Newline,
            TokKind::TextMark(true),
            TokKind::RawText(" same line".to_string()),
            TokKind::Newline,
            TokKind::TextMark(false),
            TokKind::RawText(String::new()),
            TokKind::Newline,
            TokKind::Eof
        ]
    );
}

#[test]
fn nothing_in_a_backslash_line_is_read_as_foxpro() {
    // a quotation mark closes nothing, `&&` starts no comment and a trailing `;` continues nothing
    let k = kinds("\\a \"quote, && and ; all stand\ny = 1\n");
    assert_eq!(k[1], TokKind::RawText("a \"quote, && and ; all stand".to_string()));
    assert_eq!(k[3], ident("y"));
}

#[test]
fn text_starts_a_block_only_when_its_own_clauses_follow() {
    // a line that merely begins with the word is a line like any other
    let k = kinds("text/plain=Yes\ny = 1\n");
    assert!(!k.iter().any(|k| matches!(k, TokKind::RawText(_))));
    let out = lex("TEXT TO x NOSHOW\nbody\nENDTEXT\n");
    assert!(out.tokens.iter().any(|t| t.kind == TokKind::RawText("body".to_string())));
}

#[test]
fn what_stands_after_endtext_on_its_line_is_not_read() {
    let k = kinds("TEXT\nbody\nENDTEXT and the rest\ny = 1\n");
    assert_eq!(k[2], TokKind::RawText("body".to_string()));
    assert_eq!(k[3], ident("ENDTEXT"));
    assert_eq!(k[5], ident("y"));
}
