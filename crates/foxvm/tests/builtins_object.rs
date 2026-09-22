//! Object built-ins.

mod ctx;

use ctx::{err, flag, n, num, request, s, value};
use foxvm::error::RtError;
use foxvm::host::{HostRequest, JsonValue};
use foxvm::value::{Handle, Value};

fn form() -> Value {
    Value::Object(Handle(1))
}

#[test]
fn createobject_yields_a_host_request() {
    assert_eq!(
        request("CREATEOBJECT", vec![s("Form")]),
        HostRequest::CreateObject { class: "Form".into(), args: vec![], definition: None, module: String::new() }
    );
    assert_eq!(
        request("CREATEOBJECT", vec![s("Custom"), n(1.0), s("x")]),
        HostRequest::CreateObject {
            class: "Custom".into(),
            args: vec![JsonValue::Num(1.0), JsonValue::Str("x".into())],
            definition: None,
            module: String::new(),
        }
    );
}

#[test]
fn addproperty_yields_a_host_request() {
    assert_eq!(
        request("ADDPROPERTY", vec![form(), s("Total"), n(5.0)]),
        HostRequest::AddProperty { obj: 1, name: "Total".into(), value: JsonValue::Num(5.0) }
    );
    assert_eq!(
        request("ADDPROPERTY", vec![form(), s("Flag")]),
        HostRequest::AddProperty { obj: 1, name: "Flag".into(), value: JsonValue::Bool(false) }
    );
    assert_eq!(err("ADDPROPERTY", vec![s("not an object"), s("X")]).code, RtError::TYPE_MISMATCH);
}

#[test]
fn pemstatus_asks_the_host_whether_a_member_exists() {
    assert!(flag("PEMSTATUS", vec![form(), s("Caption"), n(5.0)]));
    assert!(flag("PEMSTATUS", vec![form(), s("click"), n(5.0)]));
    assert!(!flag("PEMSTATUS", vec![form(), s("Nope"), n(5.0)]));
    assert!(!flag("PEMSTATUS", vec![form(), s("Caption"), n(1.0)]), "only n = 5 is answered");
}

#[test]
fn newobject_creates_like_createobject() {
    // the sample programs call NEWOBJECT("classname") for a class defined in the same file
    for name in ["CREATEOBJECT", "NEWOBJECT"] {
        assert_eq!(
            request(name, vec![s("form1")]),
            HostRequest::CreateObject { class: "form1".into(), args: vec![], definition: None, module: String::new() },
            "{name}()"
        );
    }
    // the module argument names the file the class is read out of, and carries the .vcx
    // extension a bare name has none of; later arguments reach the constructor
    assert_eq!(
        request("NEWOBJECT", vec![s("form1"), s("lib.vcx"), s(""), n(7.0)]),
        HostRequest::CreateObject {
            class: "form1".into(),
            args: vec![JsonValue::Num(7.0)],
            definition: None,
            module: "lib.vcx".into(),
        }
    );
    assert_eq!(
        request("NEWOBJECT", vec![s("form1"), s("lib")]),
        HostRequest::CreateObject {
            class: "form1".into(),
            args: vec![],
            definition: None,
            module: "lib.vcx".into(),
        }
    );
}

#[test]
fn unsupported_object_functions_are_reported() {
    for (name, args) in [
        ("COMCLASSINFO", vec![form()]),
        ("GETINTERFACE", vec![form(), s("IDispatch")]),
        ("EVENTHANDLER", vec![form(), form()]),
    ] {
        let e = err(name, args);
        assert_eq!(e.code, RtError::FEATURE_NOT_AVAILABLE, "{name}()");
        // the message has to say why and what to reach for instead, not just the name
        assert!(e.message.starts_with(&format!("Feature is not available: {name}()")), "{name}(): {}", e.message);
        // the message says what it would take, so a program that meets it knows where it stands
        assert!(e.message.len() > 60, "{name}() must say why: {}", e.message);
        // and what to reach for in its place
        let advice = ["instead", "use ", "; "];
        assert!(advice.iter().any(|w| e.message.contains(w)), "{name}(): {}", e.message);
    }
}

#[test]
fn removeproperty_yields_a_host_request() {
    assert_eq!(
        request("REMOVEPROPERTY", vec![form(), s("Total")]),
        HostRequest::RemoveProperty { obj: 1, name: "Total".into() }
    );
    assert_eq!(err("REMOVEPROPERTY", vec![s("not an object"), s("X")]).code, RtError::TYPE_MISMATCH);
}

/// An empty array for the functions that fill one in.
fn names() -> Value {
    Value::Array(std::rc::Rc::new(std::cell::RefCell::new(foxvm::value::FoxArray::new(1, 0))))
}

#[test]
fn amembers_lists_what_an_object_is_made_of() {
    // the test host's object has a Caption property and a Click method
    assert_eq!(num("AMEMBERS", vec![names(), form()]), 1.0, "nType 0 lists properties only");
    assert_eq!(num("AMEMBERS", vec![names(), form(), n(1.0)]), 2.0, "nType 1 lists methods too");
    assert_eq!(num("AMEMBERS", vec![names(), form(), n(2.0)]), 0.0, "it contains no member objects");
    // a flag picks members out; a letter that is not a flag is a bad argument
    assert_eq!(num("AMEMBERS", vec![names(), form(), n(1.0), s("B")]), 0.0, "nothing was added at run time");
    assert_eq!(num("AMEMBERS", vec![names(), form(), n(1.0), s("I")]), 2.0, "everything was declared");
    assert_eq!(err("AMEMBERS", vec![names(), form(), n(1.0), s("Z")]).code, RtError::FUNCTION_ARG_INVALID);
    // a class answers by name, without one being made
    assert!(num("AMEMBERS", vec![names(), s("CommandButton"), n(1.0)]) > 20.0);
    assert_eq!(err("AMEMBERS", vec![names(), s("NoSuchClass")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("AMEMBERS", vec![names(), n(5.0)]).code, RtError::FUNCTION_ARG_INVALID);
    // the four-column form says what it would take rather than answering with empty columns
    assert_eq!(err("AMEMBERS", vec![names(), form(), n(3.0)]).code, RtError::FEATURE_NOT_AVAILABLE);
}

#[test]
fn objtoclient_asks_where_a_control_sits() {
    // the test host's object has no geometry, so every position is the corner and no size
    assert_eq!(num("OBJTOCLIENT", vec![form(), n(1.0)]), 0.0);
    assert_eq!(num("OBJTOCLIENT", vec![form(), n(3.0)]), 0.0);
    assert_eq!(err("OBJTOCLIENT", vec![form(), n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("OBJTOCLIENT", vec![form(), n(5.0)]).code, RtError::FUNCTION_ARG_INVALID);
    // something that is not an object is the other error the product gives
    assert_eq!(err("OBJTOCLIENT", vec![n(1.0), n(1.0)]).code, RtError::DATA_TYPE_MISMATCH);
}

#[test]
fn getpem_reads_a_member() {
    assert_eq!(value("GETPEM", vec![form(), s("Caption")]), Value::str("Hello"));
    // a method answers with its source, which is not there to hand over outside the IDE
    assert_eq!(value("GETPEM", vec![form(), s("Click")]), Value::str(""));
    // a class answers with what it starts the property at
    assert_eq!(value("GETPEM", vec![s("CommandButton"), s("Caption")]), Value::str("Command"));
    assert_eq!(err("GETPEM", vec![form(), s("NotThere")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("GETPEM", vec![s("NoSuchClass"), s("Caption")]).code, RtError::FUNCTION_ARG_INVALID);
}
