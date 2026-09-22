//! `LAMBDA(...) ... ENDLAMBDA`, the specification in docs/foxscript.md made executable.
//!
//! FoxScript has no vfp9.exe to ask, so these tests are the specification rather than a record
//! of what a product does. Where a test says "measured" the answer is Visual FoxPro's and was
//! read off vfp9.exe; the rest is FoxScript's own and is written down in docs/foxscript.md
//! before it was written here.

use std::collections::VecDeque;

use foxvm::compiler::compile_program;
use foxvm::mock_host::{MockHost, run_program, run_with_requests};
use foxvm::parser::parse_program;
use foxvm::value::Value;
use foxvm::vm::Vm;

fn run(src: &str) -> Vec<String> {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

/// The error a program fails with, as `code: message`.
fn fails(src: &str) -> String {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, out)) => panic!("expected an error, got {out:?}"),
        Err(e) => format!("{}: {}", e.code, e.message),
    }
}

fn errors(src: &str) -> Vec<String> {
    compile_program(src, "main.prg").diagnostics.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect()
}

// ----- the word is not reserved ----------------------------------------------------------

#[test]
fn lambda_and_endlambda_are_still_variable_names() {
    // measured in vfp9.exe: `LAMBDA = 5` and `ENDLAMBDA = 7` both run there, so neither word
    // may become reserved here
    let out = run("LOCAL lambda, endlambda\nlambda = 5\nendlambda = 7\n? lambda + endlambda\n");
    assert_eq!(out, vec!["         12"]);
}

#[test]
fn an_array_called_lambda_keeps_working() {
    // `LAMBDA(1)` at the end of a line is element 1 of an array, not a lambda with a parameter
    // called 1: a parameter list is a list of names and a subscript is an expression
    let out = run(
        "LOCAL ARRAY lambda(2)\n\
         lambda(1) = \"one\"\n\
         lambda(2) = \"two\"\n\
         ? lambda(1)\n\
         ? lambda(2) + \"!\"\n",
    );
    assert_eq!(out, vec!["one", "two!"]);
}

#[test]
fn a_field_and_a_property_may_be_called_lambda() {
    // measured in vfp9.exe: `tst.lambda` reads a property called lambda
    let out = run(
        "LOCAL o\n\
         o = CREATEOBJECT(\"tst\")\n\
         ? o.lambda\n\
         DEFINE CLASS tst AS Custom\n\
           lambda = \"a property\"\n\
         ENDDEFINE\n",
    );
    assert_eq!(out, vec!["a property"]);
}

// ----- making one and calling one ---------------------------------------------------------

#[test]
fn a_lambda_in_a_variable_can_be_called() {
    let out = run(
        "PRIVATE dbl\n\
         dbl = LAMBDA(x)\n\
           RETURN x * 2\n\
         ENDLAMBDA\n\
         ? dbl(21)\n",
    );
    assert_eq!(out, vec!["          42"]);
}

#[test]
fn a_local_may_hold_one_and_call_it() {
    let out = run(
        "LOCAL dbl\n\
         dbl = LAMBDA(x)\n\
           RETURN x * 2\n\
         ENDLAMBDA\n\
         ? dbl(21)\n",
    );
    assert_eq!(out, vec!["          42"]);
}

#[test]
fn a_lambda_passed_to_a_procedure_is_called_there() {
    let out = run(
        "LOCAL f\n\
         f = LAMBDA(x)\n\
           RETURN x + 1\n\
         ENDLAMBDA\n\
         ? apply(f, 41)\n\
         PROCEDURE apply\n\
         LPARAMETERS fn, v\n\
         RETURN fn(v)\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["         42"]);
}

#[test]
fn a_lambda_with_no_parameters_is_legal() {
    let out = run("LOCAL f\nf = LAMBDA()\n  RETURN \"nothing needed\"\nENDLAMBDA\n? f()\n");
    assert_eq!(out, vec!["nothing needed"]);
}

#[test]
fn endlambda_need_not_end_its_line() {
    // the shape the acceptance program is written in: the lambda sits inside an argument list
    // and the call that holds it has still to be closed
    let out = run(
        "? apply(LAMBDA(x)\n\
           RETURN x * 3\n\
         ENDLAMBDA, 14)\n\
         PROCEDURE apply\n\
         LPARAMETERS fn, v\n\
         RETURN fn(v)\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["          42"]);
}

// ----- the value ---------------------------------------------------------------------------

#[test]
fn vartype_of_a_lambda_is_f() {
    // FoxScript's own letter. The product has no letter for a function; measured in vfp9.exe the
    // ones it can return are C N Y D T L O G Q X U, and a Float field answers N, so F is free.
    let out = run(
        "LOCAL f\n\
         f = LAMBDA(x)\n\
           RETURN x\n\
         ENDLAMBDA\n\
         ? VARTYPE(f)\n\
         ? TYPE(\"f\")\n\
         ? VARTYPE(EVALUATE(\"f\"))\n",
    );
    assert_eq!(out, vec!["F", "F", "F"]);
}

#[test]
fn store_an_array_element_and_a_property_all_hold_one() {
    let out = run(
        "LOCAL f, a, b\n\
         LOCAL ARRAY arr(2)\n\
         f = LAMBDA(x)\n\
           RETURN x * 2\n\
         ENDLAMBDA\n\
         STORE f TO a, b\n\
         arr(1) = f\n\
         LOCAL o\n\
         o = CREATEOBJECT(\"Empty\")\n\
         ADDPROPERTY(o, \"Handler\", f)\n\
         ? VARTYPE(a), VARTYPE(b), VARTYPE(arr(1)), VARTYPE(o.Handler)\n\
         ? a(3)\n\
         LOCAL fromarray, fromprop\n\
         fromarray = arr(1)\n\
         fromprop = o.Handler\n\
         ? fromarray(4), fromprop(5)\n",
    );
    assert_eq!(out, vec!["F F F F", "           6", "           8           10"]);
}

#[test]
fn two_names_for_one_lambda_are_equal_and_two_lambdas_are_not() {
    // mirrors objects, measured in vfp9.exe: `o1 = o2` is .T. for two names for one object
    let out = run(
        "LOCAL f, g, h\n\
         f = LAMBDA(x)\n\
           RETURN x\n\
         ENDLAMBDA\n\
         g = f\n\
         h = LAMBDA(x)\n\
           RETURN x\n\
         ENDLAMBDA\n\
         ? f = g, f == g, f = h\n",
    );
    assert_eq!(out, vec![".T. .T. .F."]);
}

#[test]
fn a_lambda_in_an_array_element_is_called_where_it_stands() {
    // measured in vfp9.exe: `a[1](2)` is error 36 there, "Command contains unrecognized
    // phrase/keyword.", so the position is free for FoxScript to give a meaning to
    let out = run(
        "LOCAL ARRAY laRoutes(2, 2)\n\
         laRoutes(1, 1) = \"/double\"\n\
         laRoutes(1, 2) = LAMBDA(x)\n\
           RETURN x * 2\n\
         ENDLAMBDA\n\
         ? laRoutes[1, 2](21)\n\
         ? laRoutes(1, 2)(5)\n",
    );
    assert_eq!(out, vec!["          42", "          10"]);
}

#[test]
fn a_lambda_a_call_answered_with_is_called_where_it_stands() {
    let out = run(
        "? make()(20)\n\
         PROCEDURE make\n\
         RETURN LAMBDA(x)\n\
           RETURN x + 1\n\
         ENDLAMBDA\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["         21"]);
}

#[test]
fn a_property_holding_a_lambda_is_read_into_a_name_before_it_is_called() {
    // `o.Handler(x)` is a call of the method `Handler` in the product and stays one here, so
    // this is the form that works. The rule is written down in docs/foxscript.md.
    let out = run(
        "LOCAL o, f\n\
         o = CREATEOBJECT(\"Empty\")\n\
         ADDPROPERTY(o, \"Handler\", LAMBDA(x)\n\
           RETURN x * 3\n\
         ENDLAMBDA)\n\
         f = o.Handler\n\
         ? f(14)\n",
    );
    assert_eq!(out, vec!["          42"]);
}

// ----- capture ------------------------------------------------------------------------------

#[test]
fn a_local_is_captured_by_value_when_the_lambda_is_made() {
    // the frame the local lived in is gone by the time the lambda runs, and FoxPro has no
    // reference to a local to keep instead
    let out = run(
        "? outer()\n\
         PROCEDURE outer\n\
         LOCAL n, f\n\
         n = 10\n\
         f = LAMBDA(x)\n\
           RETURN x + n\n\
         ENDLAMBDA\n\
         n = 999\n\
         RETURN f(5)\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["         15"]);
}

#[test]
fn writing_a_captured_name_inside_the_lambda_leaves_the_outer_local_alone() {
    let out = run(
        "DO outer\n\
         PROCEDURE outer\n\
         LOCAL n, f\n\
         n = 10\n\
         f = LAMBDA()\n\
           n = n + 1\n\
           RETURN n\n\
         ENDLAMBDA\n\
         ? f(), f(), n\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["        11         11         10"]);
}

#[test]
fn a_private_keeps_its_dynamic_meaning_and_is_resolved_when_the_lambda_runs() {
    let out = run(
        "PRIVATE f\n\
         f = make()\n\
         PRIVATE pn\n\
         pn = 100\n\
         ? f(5)\n\
         pn = 200\n\
         ? f(5)\n\
         PROCEDURE make\n\
         RETURN LAMBDA(x)\n\
           RETURN x + pn\n\
         ENDLAMBDA\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["        105", "        205"]);
}

#[test]
fn a_local_of_the_lambda_wins_over_a_local_of_the_enclosing_routine() {
    let out = run(
        "DO outer\n\
         PROCEDURE outer\n\
         LOCAL n, f\n\
         n = 10\n\
         f = LAMBDA()\n\
           LOCAL n\n\
           n = 3\n\
           RETURN n\n\
         ENDLAMBDA\n\
         ? f(), n\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["         3         10"]);
}

#[test]
fn a_lambda_inside_a_lambda_captures_through() {
    let out = run(
        "? outer()\n\
         PROCEDURE outer\n\
         LOCAL n, outerfn\n\
         n = 7\n\
         outerfn = LAMBDA(a)\n\
           LOCAL innerfn\n\
           innerfn = LAMBDA(b)\n\
             RETURN b + n\n\
           ENDLAMBDA\n\
           RETURN innerfn(a)\n\
         ENDLAMBDA\n\
         RETURN outerfn(35)\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["         42"]);
}

#[test]
fn this_is_captured_with_a_lambda_written_in_a_method() {
    // The mock host has no method dispatch of its own, so the method is started and the lambda
    // it hands back is started the way the host wakes the runtime with an event: as a fiber of
    // its own, through `start_function`. That is the same door an HTTP handler comes through.
    let src = "DEFINE CLASS tst AS Custom\n\
               PROCEDURE MakeHandler\n\
                 RETURN LAMBDA()\n\
                   RETURN THIS.Tag\n\
                 ENDLAMBDA\n\
               ENDPROC\n\
               ENDDEFINE\n";
    let module = compile_program(src, "main.prg").module.expect("compiles");
    let mut host = MockHost::new();
    let obj = host.add_object("tst", "TST1", None);
    host.set_prop(obj, "TAG", Value::str("the object"));

    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let mut answers = VecDeque::new();
    let method = vm.start_class_method(id, "tst", "", "MakeHandler", obj.0, Vec::new()).expect("the method");
    let made = run_with_requests(&mut vm, &mut host, method, &mut answers).expect("the method runs");
    let Value::Function(f) = made else { panic!("expected a function value, got {made:?}") };

    let fiber = vm.start_function(f, Vec::new()).expect("a fiber for the lambda");
    let answer = run_with_requests(&mut vm, &mut host, fiber, &mut answers).expect("the lambda runs");
    assert_eq!(&*answer.as_str().unwrap(), "the object");
}

// ----- what happens inside one --------------------------------------------------------------

#[test]
fn missing_arguments_are_padded_with_false_and_pcount_says_how_many_came() {
    // measured in vfp9.exe: a procedure called with one of three parameters sees .F. in the rest
    let out = run(
        "LOCAL f\n\
         f = LAMBDA(a, b, c)\n\
           RETURN TRANSFORM(a) + \" \" + VARTYPE(b) + TRANSFORM(b) + \" \" + LTRIM(STR(PCOUNT()))\n\
         ENDLAMBDA\n\
         ? f(1)\n\
         ? f(1, 2, 3)\n",
    );
    assert_eq!(out, vec!["1 L.F. 1", "1 N2 3"]);
}

#[test]
fn too_many_arguments_raise_what_a_procedure_call_raises() {
    // a lambda is called by the code that calls a procedure, so it answers the same way; the
    // two are asserted together here so they cannot drift apart
    let lambda = "LOCAL f\nf = LAMBDA(a)\n  RETURN a\nENDLAMBDA\n? f(1, 2)\n";
    let procedure = "? one(1, 2)\nPROCEDURE one\nLPARAMETERS a\nRETURN a\nENDPROC\n";
    assert_eq!(fails(lambda), fails(procedure));
    assert_eq!(fails(lambda), "1230: Too many arguments.");
}

#[test]
fn return_with_no_value_answers_true() {
    // measured in vfp9.exe: a procedure whose RETURN has no value returns .T.
    let out = run("LOCAL f\nf = LAMBDA()\n  RETURN\nENDLAMBDA\n? f(), VARTYPE(f())\n");
    assert_eq!(out, vec![".T. L"]);
}

#[test]
fn a_lambda_that_runs_off_the_end_answers_true() {
    let out = run("LOCAL f\nf = LAMBDA()\n  LOCAL x\n  x = 1\nENDLAMBDA\n? f()\n");
    assert_eq!(out, vec![".T."]);
}

#[test]
fn an_argument_may_be_passed_by_reference() {
    let out = run(
        "PRIVATE bump\n\
         bump = LAMBDA(n)\n\
           n = n + 5\n\
         ENDLAMBDA\n\
         PRIVATE v\n\
         v = 10\n\
         bump(@v)\n\
         ? v\n",
    );
    assert_eq!(out, vec!["        15"]);
}

#[test]
fn a_lambda_recurses_through_a_name_it_can_still_see() {
    // a LOCAL holding the lambda is captured by value before the lambda exists, so recursion
    // goes through a name that is resolved when the lambda runs - which is what dynamic scope
    // already means
    let out = run(
        "PRIVATE fact\n\
         fact = LAMBDA(n)\n\
           IF n <= 1\n\
             RETURN 1\n\
           ENDIF\n\
           RETURN n * fact(n - 1)\n\
         ENDLAMBDA\n\
         ? fact(5)\n",
    );
    assert_eq!(out, vec!["                                     120"]);
}

#[test]
fn a_captured_local_holding_the_lambda_is_the_copy_taken_before_it_existed() {
    // the other half of the rule above: proving it the other way round, so the two cannot be
    // swapped without a test failing
    let out = run(
        "DO outer\n\
         PROCEDURE outer\n\
         LOCAL f\n\
         f = LAMBDA()\n\
           RETURN VARTYPE(f)\n\
         ENDLAMBDA\n\
         ? f(), VARTYPE(f)\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["L F"]);
}

#[test]
fn return_inside_try_behaves_as_it_does_in_a_procedure() {
    // A lambda body is a routine body, so this is not a question a lambda answers on its own.
    // Measured in vfp9.exe the product refuses it - error 2060, "RETURN/RETRY statement not
    // allowed in TRY/CATCH." - after running the FINALLY; this runtime lets it return, which is
    // a parity question about procedures and not about lambdas. What matters here is that the
    // two agree, and they are asserted together so they cannot drift apart.
    let body = "  TRY\n    RETURN 1\n  FINALLY\n    ? \"finally ran\"\n  ENDTRY\n";
    let lambda = run(&format!("LOCAL f\nf = LAMBDA()\n{body}ENDLAMBDA\n? f()\n"));
    let procedure = run(&format!("? one()\nPROCEDURE one\n{body}ENDPROC\n"));
    assert_eq!(lambda, procedure);
}

// ----- what the parser refuses ---------------------------------------------------------------

#[test]
fn lparameters_inside_a_lambda_is_a_syntax_error() {
    assert_eq!(
        errors("LOCAL f\nf = LAMBDA(a)\n  LPARAMETERS b\n  RETURN a\nENDLAMBDA\n"),
        vec!["LPARAMETERS is not valid inside a lambda"]
    );
}

#[test]
fn a_body_that_never_closes_is_reported_at_the_lambda() {
    let out = errors("LOCAL f\nf = LAMBDA(a)\n  RETURN a\n");
    assert_eq!(out, vec!["LAMBDA without matching ENDLAMBDA"]);
}

#[test]
fn a_body_recovers_at_its_own_endlambda_rather_than_swallowing_the_file() {
    // one error for the bad line and nothing after it: the parser picks the file up again at
    // ENDLAMBDA, and the PROCEDURE below is still found
    let src = "PRIVATE f\n\
               f = LAMBDA(a)\n\
                 ? a IS NOT FOXPRO\n\
               ENDLAMBDA\n\
               ? \"carried on\"\n\
               PROCEDURE later\n\
               RETURN 1\n\
               ENDPROC\n";
    assert_eq!(errors(src), vec!["Syntax error: expected end of line, found 'IS'"]);
    let out = parse_program(src);
    assert!(
        out.program.procs.iter().any(|p| p.name.upper == "LATER"),
        "the procedure after the lambda is still read"
    );
}

#[test]
fn a_lambda_is_a_value_wherever_a_value_is_wanted() {
    // it parses in expression position, so the parse is checked as well as the run
    let out = parse_program("LOCAL f\nf = LAMBDA(a, b)\n  RETURN a + b\nENDLAMBDA\n");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}
