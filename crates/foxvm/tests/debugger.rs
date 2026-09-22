//! The debugger's half of the VM: breakpoints, stepping, and reading a stopped program.
//!
//! A stopped fiber is an ordinary parked fiber - the same `Suspend`/`resume` pair every host
//! request uses - so everything here is done with the VM off the stack, which is what lets the
//! IDE stay alive while a program waits at a breakpoint.

use foxvm::compiler::{MethodSource, compile_form, compile_program};
use foxvm::host::{BreakReason, HostRequest, StepMode};
use foxvm::mock_host::MockHost;
use foxvm::value::Value;
use foxvm::vm::{FiberId, Step, Vm};

/// Runs until the program stops, finishes or fails, answering every other request with the
/// host's default. Returns the break, or `None` when the program ran to the end.
fn run_to_break(vm: &mut Vm, host: &mut MockHost, fiber: FiberId) -> Option<(String, u32, BreakReason)> {
    loop {
        match vm.step(host, fiber) {
            Step::Suspend(HostRequest::Break { program, line, reason }) => return Some((program, line, reason)),
            Step::Suspend(req) => {
                let answer = host.default_answer(&req);
                vm.resume(fiber, answer);
            }
            Step::Done { .. } => return None,
            Step::Error(e) => panic!("unexpected error {e:?}"),
        }
    }
}

/// Lets a stopped fiber go, with the step the developer asked for.
fn go(vm: &mut Vm, fiber: FiberId, mode: StepMode) {
    vm.set_step_mode(fiber, mode);
    vm.resume(fiber, Value::Null);
}

const COUNTING: &str = "\
LOCAL nTotal, i
nTotal = 0
FOR i = 1 TO 3
  nTotal = nTotal + Twice(i)
ENDFOR
? nTotal

PROCEDURE Twice
LPARAMETERS n
LOCAL nOut
nOut = n * 2
RETURN nOut
";

fn start(src: &str, name: &str) -> (Vm, u32) {
    let module = compile_program(src, name).module.expect("compiles");
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());
    (vm, fiber)
}

#[test]
fn a_breakpoint_stops_the_program_and_the_frame_can_be_read() {
    let (mut vm, fiber) = start(COUNTING, "counting");
    let mut host = MockHost::new();
    vm.set_breakpoint("counting", 4, true);

    let stop = run_to_break(&mut vm, &mut host, fiber).expect("stops at the breakpoint");
    assert_eq!(stop, ("counting".to_string(), 4, BreakReason::Breakpoint));
    assert_eq!(vm.call_stack(fiber), vec![("counting".to_string(), 4)]);

    let frames = vm.frames(fiber);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].module, "counting");

    // the loop has not added anything yet, and the counter is on its first turn
    let vars = vm.frame_variables(fiber, 0);
    let named: Vec<(String, Value)> = vars.iter().map(|v| (v.name.clone(), v.value.clone())).collect();
    assert_eq!(named, vec![("NTOTAL".into(), Value::number(0.0)), ("I".into(), Value::number(1.0))]);
    assert!(vars.iter().all(|v| !v.private), "both are LOCAL slots");

    // a watch expression is read in the frame the program stopped in
    assert_eq!(vm.evaluate_in(&mut host, fiber, 0, "nTotal + i").unwrap(), Value::number(1.0));

    // and it stops again on the next turn of the loop, with what the first turn added
    go(&mut vm, fiber, StepMode::Go);
    let stop = run_to_break(&mut vm, &mut host, fiber).expect("stops again");
    assert_eq!(stop.1, 4);
    assert_eq!(vm.frame_variables(fiber, 0)[0].value, Value::number(2.0));

    // cleared, the third turn runs to the end
    vm.set_breakpoint("counting", 4, false);
    assert!(vm.breakpoints().is_empty());
    go(&mut vm, fiber, StepMode::Go);
    assert!(run_to_break(&mut vm, &mut host, fiber).is_none());
    assert_eq!(host.output, vec!["        12"]);
}

#[test]
fn stepping_walks_into_a_call_over_it_and_out_of_it() {
    let (mut vm, fiber) = start(COUNTING, "counting");
    let mut host = MockHost::new();
    vm.set_breakpoint("counting", 4, true);
    run_to_break(&mut vm, &mut host, fiber).expect("stops");
    vm.set_breakpoint("counting", 4, false);

    // into: the next statement is the callee's first, and the stack has grown
    go(&mut vm, fiber, StepMode::Into);
    let (program, line, reason) = run_to_break(&mut vm, &mut host, fiber).expect("steps into Twice");
    assert_eq!((program.as_str(), reason), ("Twice", BreakReason::Step));
    assert_eq!(vm.call_stack(fiber).len(), 2);
    assert_eq!(vm.frames(fiber)[1].line, line);
    // the argument is in the callee's own frame, and the caller's is still readable behind it
    assert_eq!(vm.evaluate_in(&mut host, fiber, 1, "n").unwrap(), Value::number(1.0));
    assert_eq!(vm.evaluate_in(&mut host, fiber, 0, "nTotal").unwrap(), Value::number(0.0));

    // out: back in the caller, one frame shallower
    go(&mut vm, fiber, StepMode::Out);
    let (program, _, reason) = run_to_break(&mut vm, &mut host, fiber).expect("steps out");
    assert_eq!((program.as_str(), reason), ("counting", BreakReason::Step));
    assert_eq!(vm.call_stack(fiber).len(), 1);

    // over: the whole of the next call runs without stopping, so the total has moved on
    let before = vm.frame_variables(fiber, 0)[0].value.clone();
    go(&mut vm, fiber, StepMode::Over);
    let mut seen = Vec::new();
    for _ in 0..4 {
        let Some((program, line, _)) = run_to_break(&mut vm, &mut host, fiber) else { break };
        seen.push((program.clone(), line));
        assert_eq!(program, "counting", "a step over never stops inside Twice");
        go(&mut vm, fiber, StepMode::Over);
    }
    assert!(!seen.is_empty());
    assert_ne!(before, Value::number(12.0));
}

#[test]
fn suspend_stops_the_program_where_it_stands() {
    let src = "LOCAL x\nx = 1\nSUSPEND\nx = 2\n? x\n";
    let (mut vm, fiber) = start(src, "susp");
    let mut host = MockHost::new();
    let stop = run_to_break(&mut vm, &mut host, fiber).expect("SUSPEND stops");
    assert_eq!(stop, ("susp".to_string(), 3, BreakReason::Suspend));
    assert_eq!(vm.frame_variables(fiber, 0)[0].value, Value::number(1.0));
    // RESUME picks up at the line after the one it stopped at, so the rest of the program runs
    go(&mut vm, fiber, StepMode::Go);
    assert!(run_to_break(&mut vm, &mut host, fiber).is_none());
    assert_eq!(host.output, vec!["         2"]);
}

#[test]
fn set_step_on_stops_and_resume_asks_the_host_to_let_the_program_go() {
    let src = "SET STEP ON\n? \"after\"\n";
    let (mut vm, fiber) = start(src, "stepon");
    let mut host = MockHost::new();
    let stop = run_to_break(&mut vm, &mut host, fiber).expect("SET STEP ON stops");
    assert_eq!(stop.2, BreakReason::SetStep);
    go(&mut vm, fiber, StepMode::Go);
    assert!(run_to_break(&mut vm, &mut host, fiber).is_none());

    // RESUME is typed while another program is stopped, so it is the host that acts on it
    let (mut vm, fiber) = start("RESUME\n", "resume");
    let mut host = MockHost::new();
    assert!(matches!(vm.step(&mut host, fiber), Step::Suspend(HostRequest::DebugResume)));
}

#[test]
fn a_breakpoint_in_a_form_method_is_named_by_the_method() {
    let methods = vec![MethodSource {
        object_path: "cmdSayHi".into(),
        event: "Click".into(),
        params: String::new(),
        source: "LOCAL cMsg\ncMsg = \"hi\"\n? cMsg\n".into(),
        include: String::new(),
    }];
    let module = compile_form("HelloWorld", &methods).module.expect("compiles");
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let mut host = MockHost::new();
    let form = host.add_hello_world_form();
    let button = host.find(form, "cmdSayHi").unwrap();
    let fiber = vm.start_method(id, "cmdSayHi", "Click", button, Vec::new()).expect("method");

    // the method editor shows the method's own lines, so that is what a breakpoint names
    vm.set_breakpoint("cmdSayHi.Click", 3, true);
    let stop = run_to_break(&mut vm, &mut host, fiber).expect("stops in the method");
    assert_eq!(stop, ("cmdSayHi.Click".to_string(), 3, BreakReason::Breakpoint));
    assert_eq!(vm.evaluate_in(&mut host, fiber, 0, "cMsg").unwrap(), Value::str("hi"));
    // the form the method belongs to is the module, and it names the same lines
    assert_eq!(vm.frames(fiber)[0].module, "HelloWorld");
    go(&mut vm, fiber, StepMode::Go);
    assert!(run_to_break(&mut vm, &mut host, fiber).is_none());
    assert_eq!(host.output, vec!["hi"]);
}

#[test]
fn privates_of_the_stopped_frame_are_listed_beside_its_locals() {
    let src = "LOCAL lcLocal\nPRIVATE pcOne\nlcLocal = \"L\"\npcOne = \"P\"\nSUSPEND\n? lcLocal\n";
    let (mut vm, fiber) = start(src, "vars");
    let mut host = MockHost::new();
    run_to_break(&mut vm, &mut host, fiber).expect("stops");
    let vars = vm.frame_variables(fiber, 0);
    let shown: Vec<(String, bool, Value)> =
        vars.iter().map(|v| (v.name.clone(), v.private, v.value.clone())).collect();
    assert_eq!(
        shown,
        vec![("LCLOCAL".into(), false, Value::str("L")), ("PCONE".into(), true, Value::str("P"))]
    );
}

/// Runtime-compiled code belongs to the statement that asked for it. A step must walk over a
/// macro line rather than into its expansion, and a watch expression read while the program is
/// stopped must not stop again on its own first statement - EVALUATE() cannot suspend at all.
#[test]
fn a_step_walks_over_runtime_compiled_code_rather_than_into_it() {
    let src = "LOCAL cLine, n\ncLine = \"n = 6 * 7\"\nSUSPEND\n&cLine\n? n\n";
    let (mut vm, fiber) = start(src, "macros");
    let mut host = MockHost::new();
    run_to_break(&mut vm, &mut host, fiber).expect("stops");
    // a watch expression is runtime-compiled too, and answers rather than stopping
    assert_eq!(vm.evaluate_in(&mut host, fiber, 0, "LEN(cLine)").unwrap(), Value::number(9.0));

    let mut lines = Vec::new();
    go(&mut vm, fiber, StepMode::Into);
    while let Some((program, line, _)) = run_to_break(&mut vm, &mut host, fiber) {
        assert_eq!(program, "macros");
        lines.push(line);
        go(&mut vm, fiber, StepMode::Into);
    }
    assert_eq!(lines, vec![4, 5]);
    assert_eq!(host.output, vec!["        42"]);
}

#[test]
fn debugout_writes_to_the_file_set_debugout_named() {
    let src = "SET DEBUGOUT TO trace.txt\nDEBUGOUT \"one\", 2\nSET ASSERTS ON\nASSERT .F. MESSAGE \"bad\"\nSET DEBUGOUT TO\nDEBUGOUT \"gone\"\n";
    let mut host = MockHost::new();
    let (mut vm, fiber) = start(src, "dbg");
    assert!(run_to_break(&mut vm, &mut host, fiber).is_none());
    let written: String = host.files.get("TRACE.TXT").expect("the file").iter().map(|&b| b as char).collect();
    assert_eq!(written, "one 2\r\nbad\r\n");
    // everything said is in the Output as well, which is where a run with no file sees it
    assert_eq!(host.output, vec!["one 2", "bad", "gone"]);
}
