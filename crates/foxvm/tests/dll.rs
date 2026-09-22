//! `DECLARE ... DLL`: the declaration, and the request a call to it becomes.
//!
//! What the library actually does is the host's business; what the VM must get right is which
//! function is being called, with which types, and which arguments the library is given the
//! address of.

mod common;

use foxvm::host::{HostRequest, JsonValue};
use foxvm::mock_host::MockHost;
use foxvm::value::{FoxArray, Value};
use foxvm::vm::{Step, Vm};

fn array_of(items: Vec<Value>) -> Value {
    Value::Array(std::rc::Rc::new(std::cell::RefCell::new(FoxArray::of(items))))
}

/// Runs `src` and returns the first library call it makes, with the fiber left suspended.
fn first_call(src: &str) -> HostRequest {
    let module = foxvm::compiler::compile_program(src, "main").module.expect("compiles");
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());
    let mut host = MockHost::new();
    loop {
        match vm.step(&mut host, fiber) {
            Step::Suspend(request) => return request,
            Step::Done { .. } => panic!("the program made no library call"),
            Step::Error(e) => panic!("{e:?}"),
        }
    }
}

#[test]
fn a_declaration_makes_the_name_callable() {
    let request = first_call(
        r#"
DECLARE INTEGER GetTickCount IN kernel32
? GetTickCount()
"#,
    );
    match request {
        HostRequest::CallDll { library, function, returns, params, by_ref, args } => {
            assert_eq!(library, "kernel32");
            assert_eq!(function, "GetTickCount");
            assert_eq!(returns, "INTEGER");
            assert!(params.is_empty());
            assert!(by_ref.is_empty());
            assert!(args.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_types_and_the_by_reference_marks_reach_the_host() {
    let request = first_call(
        r#"
DECLARE INTEGER GetPrivateProfileString IN Win32API ;
    STRING lpAppName, STRING lpKeyName, STRING lpDefault, ;
    STRING @lpReturnedString, INTEGER nSize, STRING lpFileName
LOCAL lcBuffer
lcBuffer = SPACE(8)
? GetPrivateProfileString("sec", "key", "fallback", @lcBuffer, 8, "app.ini")
"#,
    );
    match request {
        HostRequest::CallDll { library, function, params, by_ref, args, .. } => {
            assert_eq!(library, "Win32API");
            assert_eq!(function, "GetPrivateProfileString");
            assert_eq!(params, vec!["STRING", "STRING", "STRING", "STRING", "INTEGER", "STRING"]);
            assert_eq!(by_ref, vec![false, false, false, true, false, false]);
            assert_eq!(args.len(), 6);
            assert_eq!(args[0], JsonValue::Str("sec".into()));
            assert_eq!(args[5], JsonValue::Str("app.ini".into()));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_alias_is_the_name_the_program_calls() {
    let request = first_call(
        r#"
DECLARE INTEGER MessageBox IN user32 AS Win32MessageBox ;
    INTEGER hWnd, STRING lpText, STRING lpCaption, INTEGER uType
? Win32MessageBox(0, "text", "caption", 0)
"#,
    );
    match request {
        HostRequest::CallDll { function, library, .. } => {
            assert_eq!(function, "MessageBox");
            assert_eq!(library, "user32");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_library_named_by_an_expression_is_evaluated_when_it_is_declared() {
    let request = first_call(
        r#"
LOCAL lcLib
lcLib = "vendor.dll"
DECLARE INTEGER Ping IN (lcLib) INTEGER n
? Ping(1)
"#,
    );
    match request {
        HostRequest::CallDll { library, .. } => assert_eq!(library, "vendor.dll"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_quoted_library_keeps_its_path() {
    let request = first_call("DECLARE INTEGER Ping IN \"c:/libs/vendor.dll\"\n? Ping()");
    match request {
        HostRequest::CallDll { library, .. } => assert_eq!(library, "c:/libs/vendor.dll"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn what_the_library_writes_back_reaches_the_variable() {
    let module = foxvm::compiler::compile_program(
        r#"
DECLARE INTEGER Fill IN vendor.dll STRING @buffer
LOCAL lcBuffer, lnResult
lcBuffer = SPACE(8)
lnResult = Fill(@lcBuffer)
? lnResult
? lcBuffer
"#,
        "main",
    )
    .module
    .expect("compiles");

    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());
    let mut host = MockHost::new();
    loop {
        match vm.step(&mut host, fiber) {
            Step::Suspend(HostRequest::CallDll { .. }) => {
                // the host answers with the return value and what it wrote into the arguments
                let written = array_of(vec![Value::str("written")]);
                vm.resume(fiber, array_of(vec![Value::number(7.0), written]));
            }
            Step::Suspend(other) => panic!("{other:?}"),
            Step::Done { .. } => break,
            Step::Error(e) => panic!("{e:?}"),
        }
    }
    assert_eq!(host.output, vec!["         7", "written"]);
}
