//! Suspension protocol: host requests, resuming, parked fibers, aborting and error injection.

use foxvm::compiler::{MethodSource, compile_form, compile_program};
use foxvm::error::RtError;
use foxvm::host::{HostRequest, JsonValue};
use foxvm::mock_host::MockHost;
use foxvm::value::{Handle, Value};
use foxvm::vm::{Step, Vm};

fn load(vm: &mut Vm, src: &str, name: &str) -> u32 {
    let r = compile_program(src, name);
    vm.load_module(r.module.unwrap_or_else(|| panic!("{:#?}", r.diagnostics)))
}

#[test]
fn set_member_suspends_and_resumes() {
    let mut host = MockHost::new();
    let form = host.add_hello_world_form();
    let mut vm = Vm::new();
    vm.set_global("o", Value::Object(form));
    let id = load(&mut vm, "o.Caption = \"new\"\n? \"after\"", "t");
    let fiber = vm.start(id, 0, None, Vec::new());
    match vm.step(&mut host, fiber) {
        Step::Suspend(HostRequest::SetProp { obj, name, value }) => {
            assert_eq!(obj, form.0);
            assert_eq!(name, "Caption");
            assert_eq!(value, JsonValue::Str("new".into()));
        }
        other => panic!("{other:?}"),
    }
    assert!(host.output.is_empty());
    vm.resume(fiber, Value::Null);
    assert!(matches!(vm.step(&mut host, fiber), Step::Done { .. }));
    assert_eq!(host.output, vec!["after"]);
    assert!(!vm.has_fiber(fiber));
}

#[test]
fn read_events_parks_the_fiber_while_another_runs() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let main = load(&mut vm, "? \"main start\"\nREAD EVENTS\n? \"main end\"", "main");
    let click = load(&mut vm, "? \"click\"\nCLEAR EVENTS", "click");
    let f1 = vm.start(main, 0, None, Vec::new());
    assert_eq!(vm.step(&mut host, f1), Step::Suspend(HostRequest::ReadEvents));
    let f2 = vm.start(click, 0, None, Vec::new());
    assert_eq!(vm.step(&mut host, f2), Step::Suspend(HostRequest::ClearEvents));
    vm.resume(f2, Value::Null);
    assert!(matches!(vm.step(&mut host, f2), Step::Done { .. }));
    assert_eq!(vm.call_stack(f1), vec![("main".to_string(), 2)]);
    vm.resume(f1, Value::Null);
    assert!(matches!(vm.step(&mut host, f1), Step::Done { .. }));
    assert_eq!(host.output, vec!["main start", "click", "main end"]);
}

#[test]
fn abort_drops_a_suspended_fiber() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = load(&mut vm, "WAIT WINDOW \"x\"\n? \"never\"", "t");
    let fiber = vm.start(id, 0, None, Vec::new());
    assert!(matches!(vm.step(&mut host, fiber), Step::Suspend(HostRequest::WaitWindow { .. })));
    assert!(vm.has_fiber(fiber));
    vm.abort(fiber);
    assert!(!vm.has_fiber(fiber));
    assert!(matches!(vm.step(&mut host, fiber), Step::Error(_)));
    assert!(host.output.is_empty());
}

#[test]
fn resume_error_is_caught_by_try() {
    let mut host = MockHost::new();
    let form = host.add_hello_world_form();
    let mut vm = Vm::new();
    vm.set_global("o", Value::Object(form));
    // the exception is not called `m`: `m.` is the memvar prefix, so `m.Message` would read a
    // memory variable called Message rather than the exception's property, as it does in VFP
    let src = "TRY\n  o.Caption = \"x\"\n  ? \"not here\"\nCATCH TO oErr\n  ? \"caught \" + oErr.Message\nENDTRY\no.Caption = \"y\"";
    let id = load(&mut vm, src, "t");
    let fiber = vm.start(id, 0, None, Vec::new());
    assert!(matches!(vm.step(&mut host, fiber), Step::Suspend(HostRequest::SetProp { .. })));
    vm.resume_error(fiber, RtError::property_not_found("Caption"));
    // CATCH TO asks the host for the Exception object before the handler body runs.
    let exception = match vm.step(&mut host, fiber) {
        Step::Suspend(req @ HostRequest::CreateException { .. }) => host.default_answer(&req),
        other => panic!("expected CreateException, got {other:?}"),
    };
    vm.resume(fiber, exception);
    // The second SetProp (outside the TRY) suspends again, then an injected error is fatal.
    assert!(matches!(vm.step(&mut host, fiber), Step::Suspend(HostRequest::SetProp { .. })));
    assert_eq!(host.output, vec!["caught Property CAPTION is not found."]);
    vm.resume_error(fiber, RtError::new(1234, "host says no"));
    match vm.step(&mut host, fiber) {
        Step::Error(e) => {
            assert_eq!(e.code, 1234);
            assert_eq!(e.line, 7);
            assert_eq!(e.program, "T");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn call_stack_reports_frames_in_call_order() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let src = "DO Outer\nPROCEDURE Outer\n  ? 1\n  DO Inner\nPROCEDURE Inner\n  WAIT WINDOW \"deep\"";
    let id = load(&mut vm, src, "main");
    let fiber = vm.start(id, 0, None, Vec::new());
    assert!(matches!(vm.step(&mut host, fiber), Step::Suspend(HostRequest::WaitWindow { text, .. }) if text == "deep"));
    assert_eq!(vm.call_stack(fiber), vec![("main".to_string(), 1), ("Outer".to_string(), 4), ("Inner".to_string(), 6)]);
    vm.abort_all();
    assert_eq!(vm.call_stack(fiber), Vec::<(String, u32)>::new());
}

#[test]
fn do_form_and_load_program_requests() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let src = "DO FORM HelloWorld NAME oF LINKED WITH 1, \"a\" TO nResult\n? nResult\n? oF.Caption\nDO missing\n? \"after load\"";
    let id = load(&mut vm, src, "main");
    let fiber = vm.start(id, 0, None, Vec::new());
    match vm.step(&mut host, fiber) {
        Step::Suspend(HostRequest::DoForm { name, args, linked, want_object, want_result, noshow, .. }) => {
            assert_eq!(name, "HelloWorld");
            assert_eq!(args, vec![JsonValue::Num(1.0), JsonValue::Str("a".into())]);
            assert!(linked && want_object && want_result && !noshow);
        }
        other => panic!("{other:?}"),
    }
    let form = host.add_hello_world_form();
    let pair = JsonValue::Arr { arr: vec![JsonValue::Obj { obj: form.0 }, JsonValue::Num(42.0)], cols: 0 };
    vm.resume(fiber, pair.to_value());
    match vm.step(&mut host, fiber) {
        Step::Suspend(HostRequest::LoadProgram { name }) => assert_eq!(name, "MISSING"),
        other => panic!("{other:?}"),
    }
    assert_eq!(host.output, vec!["        42", "Hello, World"]);
    let other = load(&mut vm, "? \"loaded on demand\"", "missing");
    vm.resume(fiber, Value::number(other as f64));
    assert!(matches!(vm.step(&mut host, fiber), Step::Done { .. }));
    assert_eq!(host.output, vec!["        42", "Hello, World", "loaded on demand", "after load"]);
}

#[test]
fn this_outside_a_method_and_invalid_objects() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = load(&mut vm, "? THIS.Caption", "t");
    let fiber = vm.start(id, 0, None, Vec::new());
    match vm.step(&mut host, fiber) {
        Step::Error(e) => assert_eq!((e.code, e.message.as_str()), (1929, "THIS can only be used within a method.")),
        other => panic!("{other:?}"),
    }
    // `RELEASE THISFORM` yields ReleaseObject; afterwards the handle is invalid.
    let form = host.add_hello_world_form();
    let button = host.find(form, "cmdClose").unwrap();
    let methods = [MethodSource {
        object_path: "cmdClose".into(),
        event: "Click".into(),
        params: String::new(),
        source: "RELEASE THISFORM\n? THIS.Caption".into(),
        include: String::new(),
    }];
    let id = vm.load_module(compile_form("HelloWorld", &methods).module.unwrap());
    let fiber = vm.start_method(id, "cmdClose", "Click", button, Vec::new()).unwrap();
    assert!(matches!(vm.step(&mut host, fiber), Step::Suspend(HostRequest::ReleaseObject { obj }) if obj == form.0));
    host.release(form);
    vm.resume(fiber, Value::Null);
    match vm.step(&mut host, fiber) {
        Step::Error(e) => {
            assert_eq!(e.code, RtError::OBJECT_NOT_VALID);
            assert_eq!(e.program, "CMDCLOSE.CLICK");
            assert_eq!(e.line, 2);
        }
        other => panic!("{other:?}"),
    }
    // `RELEASE name` on a plain variable drops the variable instead.
    vm.set_global("o", Value::Object(Handle(0)));
    let id = load(&mut vm, "RELEASE o\n? o", "t2");
    let fiber = vm.start(id, 0, None, Vec::new());
    match vm.step(&mut host, fiber) {
        Step::Error(e) => assert_eq!(e.code, RtError::VARIABLE_NOT_FOUND),
        other => panic!("{other:?}"),
    }
    let id = load(&mut vm, "? _SCREEN.Nope", "t3");
    let fiber = vm.start(id, 0, None, Vec::new());
    match vm.step(&mut host, fiber) {
        Step::Error(e) => assert_eq!(e.code, RtError::UNKNOWN_MEMBER),
        other => panic!("{other:?}"),
    }
    assert_eq!(Handle(0), Handle(0));
}

#[test]
fn do_of_a_menu_file_yields_a_menu_request() {
    let out = compile_program("DO Main.fxm\n? \"after\"", "m");
    let module = out.module.expect("module");
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let mut host = MockHost::new();
    let fiber = vm.start(id, 0, None, Vec::new());

    match vm.step(&mut host, fiber) {
        Step::Suspend(HostRequest::DoMenu { name }) => assert_eq!(name, "MAIN.FXM"),
        other => panic!("expected a DoMenu request, got {other:?}"),
    }
    vm.resume(fiber, Value::Null);
    assert!(matches!(vm.step(&mut host, fiber), Step::Done { .. }));
    assert_eq!(host.output, vec!["after".to_string()]);
}

#[test]
fn a_private_of_a_parked_program_is_visible_to_what_runs_while_it_waits() {
    // The shape every Visual FoxPro application has: a program sets things up, parks in READ
    // EVENTS, and its forms run. In VFP that is one stack, so the form methods can see the
    // program's private variables - and put them back, which is what ON ERROR restoring is.
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let main = load(&mut vm, "cOldError = \"DO OldHandler\"\nREAD EVENTS\n? cOldError", "main");
    let click = load(&mut vm, "? cOldError\ncOldError = \"changed\"\nCLEAR EVENTS", "click");

    let f1 = vm.start(main, 0, None, Vec::new());
    assert_eq!(vm.step(&mut host, f1), Step::Suspend(HostRequest::ReadEvents));

    let f2 = vm.start(click, 0, None, Vec::new());
    assert_eq!(vm.step(&mut host, f2), Step::Suspend(HostRequest::ClearEvents));
    vm.resume(f2, Value::Null);
    assert!(matches!(vm.step(&mut host, f2), Step::Done { .. }));

    vm.resume(f1, Value::Null);
    assert!(matches!(vm.step(&mut host, f1), Step::Done { .. }));
    // the handler read the program's variable, and the change it made is the one the program sees
    assert_eq!(host.output, vec!["DO OldHandler", "changed"]);
}

#[test]
fn a_private_of_a_fiber_that_has_finished_is_gone() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let first = load(&mut vm, "cGone = \"here\"", "first");
    let second = load(&mut vm, "? cGone", "second");

    let f1 = vm.start(first, 0, None, Vec::new());
    assert!(matches!(vm.step(&mut host, f1), Step::Done { .. }));

    let f2 = vm.start(second, 0, None, Vec::new());
    match vm.step(&mut host, f2) {
        Step::Error(e) => assert_eq!(e.code, 12),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_failed_fiber_can_be_told_to_carry_on_at_the_next_line() {
    // The Ignore button of the error dialog: Visual FoxPro carries on at the next line, which
    // means the fiber has to still be there after the error rather than thrown away.
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = load(&mut vm, "? \"before\"\n? nowhere\n? \"after\"", "t");
    let fiber = vm.start(id, 0, None, Vec::new());

    match vm.step(&mut host, fiber) {
        Step::Error(e) => assert_eq!(e.code, 12),
        other => panic!("{other:?}"),
    }
    assert!(vm.has_fiber(fiber), "the fiber is kept so the host can choose to carry on");

    vm.resume(fiber, Value::Null);
    assert!(matches!(vm.step(&mut host, fiber), Step::Done { .. }));
    // `?` ends the line before it works out what to print, so the line that failed left a blank
    assert_eq!(host.output, vec!["before", "", "after"]);
    assert!(!vm.has_fiber(fiber));
}
