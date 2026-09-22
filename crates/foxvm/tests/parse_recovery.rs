mod common;

use common::{dump, errors, messages};
use foxvm::diagnostics::Severity;
use foxvm::parser::{has_errors, parse_method, parse_program};

#[test]
fn if_without_endif_reports_the_if_line() {
    let out = parse_program("x = 1\nIF x\n  y = 2\n");
    assert_eq!(errors(&out.diagnostics), vec!["2:1: IF without matching ENDIF"]);
    assert_eq!(out.diagnostics[0].start, 6);
    assert_eq!(out.diagnostics[0].end, 8);
    // the block is still produced
    assert_eq!(dump(&out.program), "(= X 1)\n(if X (block (= Y 2)))");
}

#[test]
fn missing_terminators_for_every_block_kind() {
    for (src, msg) in [
        ("DO WHILE .T.\n x = 1", "DO WHILE without matching ENDDO"),
        ("FOR i = 1 TO 2\n x = 1", "FOR without matching ENDFOR"),
        ("FOR EACH o IN c\n x = 1", "FOR EACH without matching ENDFOR"),
        ("DO CASE\nCASE x\n y = 1", "DO CASE without matching ENDCASE"),
        ("DO CASE\n", "DO CASE without matching ENDCASE"),
        ("WITH o\n .x = 1", "WITH without matching ENDWITH"),
        ("TRY\n x = 1\nCATCH\n y = 2", "TRY without matching ENDTRY"),
        ("IF a\nELSE\n x = 1", "IF without matching ENDIF"),
    ] {
        let out = parse_program(src);
        assert_eq!(errors(&out.diagnostics), vec![format!("1:1: {msg}")], "for {src:?}");
    }
}

#[test]
fn nested_missing_terminator_does_not_cascade() {
    let src = "DO WHILE .T.\n  IF a\n    x = 1\nENDDO\ny = 2";
    let out = parse_program(src);
    assert_eq!(errors(&out.diagnostics), vec!["2:3: IF without matching ENDIF"]);
    assert_eq!(dump(&out.program), "(while .T. (block (if A (block (= X 1)))))\n(= Y 2)");
}

#[test]
fn two_independent_syntax_errors_are_both_reported() {
    let src = "x = 1\ny = 1 +\nz = 2\nw = 3\nv = (4\nu = 5";
    let out = parse_program(src);
    assert_eq!(
        errors(&out.diagnostics),
        vec![
            "2:8: Syntax error: expected expression, found end of line",
            "5:7: Syntax error: expected ')', found end of line"
        ]
    );
    assert_eq!(dump(&out.program), "(= X 1)\n(= Z 2)\n(= W 3)\n(= U 5)");
}

#[test]
fn unsupported_commands() {
    let out = parse_program("CALL binary\nx = 1");
    assert_eq!(errors(&out.diagnostics), vec!["1:1: CALL is not supported in the FoxDev runtime"]);
    assert_eq!(dump(&out.program), "(= X 1)");
    for (src, verb) in [("CALL binary", "CALL"), ("LOAD overlay", "LOAD")] {
        let out = parse_program(src);
        assert!(
            out.diagnostics.iter().any(|d| d.message == format!("{verb} is not supported in the FoxDev runtime")),
            "for {src:?}: {:?}",
            messages(&out.diagnostics)
        );
        assert!(has_errors(&out.diagnostics));
    }
}

#[test]
fn a_format_nothing_reads_is_refused_by_name() {
    // IMPORT and EXPORT are commands now; what is refused is the museum formats inside them
    let out = parse_program("IMPORT FROM sheet TYPE XL5\nEXPORT TO out TYPE DIF\nz = 1");
    assert!(!has_errors(&out.diagnostics), "{:?}", messages(&out.diagnostics));

    for (src, word) in [("IMPORT FROM old TYPE WK1", "WK1"), ("EXPORT TO book TYPE XL5", "XL5")] {
        let out = parse_program(src);
        assert!(
            out.diagnostics.iter().any(|d| d.message.contains(&format!("TYPE {word} is not supported"))),
            "{src}: {:?}",
            messages(&out.diagnostics)
        );
    }
}

#[test]
fn build_and_compile_are_commands_of_their_own() {
    // they were refused wholesale once; now the project ones build, and only the COM server
    // libraries are refused - by name, and with what they would have needed
    let out = parse_program("BUILD APP x FROM y\nCOMPILE x.prg\nz = 1");
    assert!(!has_errors(&out.diagnostics), "{:?}", messages(&out.diagnostics));
    let dumped = dump(&out.program);
    assert!(dumped.starts_with("(build APP "), "{dumped}");
    assert!(dumped.contains("(from "), "{dumped}");
    assert!(dumped.contains("(compile "), "{dumped}");
    assert!(dumped.ends_with("(= Z 1)"), "{dumped}");

    // a COM server library is said to be missing rather than refused: Visual FoxPro compiles the
    // line - written out, run through COMPILE, no .err beside the .fxp - and fails only when it
    // is reached, so refusing here would take the rest of the routine with it
    let dll = parse_program("BUILD DLL server FROM server.fxp\nz = 2");
    assert!(!has_errors(&dll.diagnostics), "{:?}", messages(&dll.diagnostics));
    assert!(dll.diagnostics.iter().any(|d| d.message.starts_with("BUILD DLL builds an in-process COM server")));
    assert!(dump(&dll.program).ends_with("(= Z 2)"), "{}", dump(&dll.program));
}

#[test]
fn statements_after_enddefine_are_reported() {
    let out = parse_program(
        "DEFINE CLASS foo AS Custom\n  PROCEDURE Init\n    DO WHILE .T.\n    ENDDO\n  ENDPROC\nENDDEFINE\nx = 1",
    );
    assert_eq!(
        errors(&out.diagnostics),
        vec!["7:1: Statements after ENDDEFINE must belong to a PROCEDURE, FUNCTION or DEFINE CLASS"]
    );
    assert_eq!(dump(&out.program), "(class FOO AS CUSTOM)\n  (procedure INIT ())\n    (while .T. (block))");
}

#[test]
fn an_unterminated_class_is_reported_once() {
    let out = parse_program("DEFINE CLASS foo AS Custom\n  Caption = \"x\"\n");
    assert_eq!(errors(&out.diagnostics), vec!["1:1: DEFINE CLASS without matching ENDDEFINE"]);
    assert_eq!(dump(&out.program), "(class FOO AS CUSTOM)\n  (prop CAPTION \"x\")");
}

#[test]
fn unrecognized_command_verb() {
    let out = parse_program("foo bar");
    assert_eq!(errors(&out.diagnostics), vec!["1:1: Unrecognized command verb 'FOO'"]);
    let out = parse_program("x");
    assert_eq!(errors(&out.diagnostics), vec!["1:1: Unrecognized command verb 'X'"]);
    // `obj.member` on its own line is a method call in VFP, not a discarded property read
    let out = parse_program("obj.prop");
    assert_eq!(errors(&out.diagnostics), Vec::<String>::new());
    let out = parse_program("\"string\"");
    assert_eq!(errors(&out.diagnostics), vec!["1:1: Unrecognized command verb"]);
    let out = parse_program("foo() bar");
    assert_eq!(errors(&out.diagnostics), vec!["1:7: Syntax error: expected end of line, found 'bar'"]);
}

#[test]
fn stray_terminators() {
    for (src, msg) in [
        ("ENDIF", "ENDIF without matching IF"),
        ("ELSE", "ELSE without matching IF"),
        ("ENDDO", "ENDDO without matching DO WHILE"),
        ("ENDFOR", "ENDFOR without matching FOR"),
        ("NEXT", "NEXT without matching FOR"),
        ("ENDCASE", "ENDCASE without matching DO CASE"),
        ("CASE x", "CASE without matching DO CASE"),
        ("OTHERWISE", "OTHERWISE without matching DO CASE"),
        ("ENDWITH", "ENDWITH without matching WITH"),
        ("ENDTRY", "ENDTRY without matching TRY"),
        ("CATCH", "CATCH without matching TRY"),
        ("FINALLY", "FINALLY without matching TRY"),
        ("ENDPROC", "ENDPROC without matching PROCEDURE"),
        ("ENDFUNC", "ENDFUNC without matching FUNCTION"),
        ("ENDTEXT", "ENDTEXT without matching TEXT"),
    ] {
        let out = parse_program(&format!("x = 1\n{src}\ny = 2"));
        assert_eq!(errors(&out.diagnostics), vec![format!("2:1: {msg}")], "for {src:?}");
        assert_eq!(dump(&out.program), "(= X 1)\n(= Y 2)");
    }
}

#[test]
fn stray_endif_inside_a_loop() {
    let out = parse_program("DO WHILE .T.\n  ENDIF\nENDDO");
    assert_eq!(errors(&out.diagnostics), vec!["2:3: ENDIF without matching IF"]);
    assert_eq!(dump(&out.program), "(while .T. (block))");
}

#[test]
fn parse_method_rejects_procedure_declarations() {
    let out = parse_method("x = 1\nPROCEDURE foo\n  y = 2\nENDPROC\nFUNCTION bar");
    assert_eq!(
        errors(&out.diagnostics),
        vec![
            "2:1: PROCEDURE declarations are not allowed inside a method",
            "4:1: ENDPROC without matching PROCEDURE",
            "5:1: FUNCTION declarations are not allowed inside a method"
        ]
    );
    assert_eq!(dump(&out.program), "(= X 1)\n(= Y 2)");
    assert!(out.program.procs.is_empty());
}

#[test]
fn diagnostic_positions_on_later_lines() {
    let src = "x = 1\n\n   y = \"abc\" +\nz = 3";
    let out = parse_program(src);
    assert_eq!(out.diagnostics.len(), 1);
    let d = &out.diagnostics[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!((d.line, d.col), (3, 15));
    assert_eq!((d.end_line, d.end_col), (3, 15));
    assert_eq!((d.start, d.end), (21, 21));
    assert_eq!(dump(&out.program), "(= X 1)\n(= Z 3)");
}

#[test]
fn lexer_errors_flow_through_the_parser() {
    let out = parse_program("x = \"abc\ny = 1");
    assert_eq!(errors(&out.diagnostics), vec!["1:5: Unterminated string: missing closing \""]);
    assert_eq!(dump(&out.program), "(= X \"abc\")\n(= Y 1)");
}

#[test]
fn bad_block_header_keeps_the_block_structure() {
    let out = parse_program("IF x +\n  y = 1\nELSE\n  y = 2\nENDIF\nz = 3");
    assert_eq!(errors(&out.diagnostics), vec!["1:7: Syntax error: expected expression, found end of line"]);
    assert_eq!(dump(&out.program), "(if .F. (block (= Y 1)) (block (= Y 2)))\n(= Z 3)");
    let out = parse_program("FOR i = 1\n  y = 1\nENDFOR\nz = 3");
    assert_eq!(errors(&out.diagnostics), vec!["1:10: Syntax error: expected TO, found end of line"]);
    assert_eq!(dump(&out.program), "(for  .F. .F. (block (= Y 1)))\n(= Z 3)");
    let out = parse_program("FOR EACH o\n  y = 1\nNEXT\nz = 3");
    assert_eq!(errors(&out.diagnostics), vec!["1:11: Syntax error: expected IN, found end of line"]);
    assert_eq!(dump(&out.program), "(foreach  .F. (block (= Y 1)))\n(= Z 3)");
}

#[test]
fn junk_after_statements() {
    let out = parse_program("LOCAL a b\nx = 1 2\nQUIT now");
    assert_eq!(
        errors(&out.diagnostics),
        vec![
            "1:9: Syntax error: expected end of line, found 'b'",
            "2:7: Syntax error: expected end of line, found '2'",
            "3:6: Syntax error: expected end of line, found 'now'"
        ]
    );
}

#[test]
fn invalid_assignment_targets() {
    let out = parse_program("f() = 1");
    assert_eq!(errors(&out.diagnostics), vec!["1:1: Invalid assignment target"]);
    let out = parse_program("STORE 1 TO f()");
    assert_eq!(errors(&out.diagnostics), vec!["1:12: Invalid assignment target"]);
}

#[test]
fn statements_after_endproc_are_reported_once() {
    let out = parse_program("PROCEDURE a\nENDPROC\nx = 1\ny = 2\nPROCEDURE b\nENDPROC");
    assert_eq!(
        errors(&out.diagnostics),
        vec!["3:1: Statements after ENDPROC/ENDFUNC must belong to a PROCEDURE or FUNCTION"]
    );
    assert_eq!(dump(&out.program), "(procedure A ())\n(procedure B ())");
}

#[test]
fn has_errors_ignores_warnings() {
    let out = parse_program("PRIVATE ALL");
    assert_eq!(out.diagnostics.len(), 1);
    assert!(!has_errors(&out.diagnostics));
    let out = parse_program("IMPORT FROM x");
    assert!(has_errors(&out.diagnostics));
}

#[test]
fn zap_is_a_command_of_its_own() {
    let out = parse_program("ZAP\nZAP IN customer");
    assert_eq!(dump(&out.program), "(zap)
(zap in CUSTOMER)");
    assert!(!has_errors(&out.diagnostics));
}
