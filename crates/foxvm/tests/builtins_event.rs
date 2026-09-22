//! BINDEVENT() / UNBINDEVENT() / RAISEEVENT(). The VM validates the arguments and yields; the
//! host owns the binding table and dispatches.

mod ctx;

use ctx::{err, n, request, s};
use foxvm::error::RtError;
use foxvm::host::{HostRequest, JsonValue};
use foxvm::value::{Handle, Value};

fn obj(h: u32) -> Value {
    Value::Object(Handle(h))
}

#[test]
fn bindevent_yields_a_binding_request() {
    assert_eq!(
        request("BINDEVENT", vec![obj(1), s("Click"), obj(2), s("OnClick")]),
        HostRequest::BindEvent { source: 1, event: "Click".into(), handler: 2, delegate: "OnClick".into(), flags: 0 }
    );
    assert_eq!(
        request("BINDEVENT", vec![obj(3), s("MouseMove"), obj(4), s("Track"), n(5.0)]),
        HostRequest::BindEvent { source: 3, event: "MouseMove".into(), handler: 4, delegate: "Track".into(), flags: 5 }
    );
}

#[test]
fn bindevent_checks_its_arguments() {
    assert_eq!(err("BINDEVENT", vec![s("no"), s("Click"), obj(2), s("OnClick")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("BINDEVENT", vec![obj(1), s("Click"), n(2.0), s("OnClick")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(
        err("BINDEVENT", vec![obj(1), s("  "), obj(2), s("OnClick")]).code,
        RtError::FUNCTION_ARG_INVALID,
        "a blank event name is rejected"
    );
    assert_eq!(err("BINDEVENT", vec![obj(1), s("Click"), obj(2), s("")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(
        err("BINDEVENT", vec![obj(1), s("Click"), obj(1), s("OnClick")]).code,
        RtError::FUNCTION_ARG_INVALID,
        "an object may not handle its own event"
    );
}

/// Measured against the product: only the full four-argument binding and the single-object form
/// are valid. Two arguments or three are neither shape VFP recognises - error 11 - and none at
/// all is error 1229, "Too few arguments", not a wildcard that drops every binding.
#[test]
fn unbindevent_takes_only_its_two_documented_shapes() {
    assert_eq!(
        request("UNBINDEVENTS", vec![obj(1)]),
        HostRequest::UnbindEvent { source: Some(1), event: None, handler: None, delegate: None }
    );
    assert_eq!(
        request("UNBINDEVENTS", vec![Value::Null]),
        HostRequest::UnbindEvent { source: None, event: None, handler: None, delegate: None },
        ".NULL. alone is still the one-argument form, matching everything"
    );
    assert_eq!(
        request("UNBINDEVENTS", vec![obj(1), s("Click"), obj(2), s("OnClick")]),
        HostRequest::UnbindEvent {
            source: Some(1),
            event: Some("Click".into()),
            handler: Some(2),
            delegate: Some("OnClick".into()),
        }
    );
    assert_eq!(err("UNBINDEVENTS", vec![]).code, RtError::TOO_FEW_ARGS);
    assert_eq!(err("UNBINDEVENTS", vec![Value::Null, s("Click")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("UNBINDEVENTS", vec![obj(1), s("Click"), obj(2)]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn raiseevent_carries_the_event_arguments() {
    assert_eq!(
        request("RAISEEVENT", vec![obj(1), s("Click")]),
        HostRequest::RaiseEvent { source: 1, event: "Click".into(), args: vec![] }
    );
    assert_eq!(
        request("RAISEEVENT", vec![obj(1), s("MouseDown"), n(1.0), n(0.0), s("x")]),
        HostRequest::RaiseEvent {
            source: 1,
            event: "MouseDown".into(),
            args: vec![JsonValue::Num(1.0), JsonValue::Num(0.0), JsonValue::Str("x".into())],
        }
    );
    assert_eq!(err("RAISEEVENT", vec![n(1.0), s("Click")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("RAISEEVENT", vec![obj(1), s("")]).code, RtError::FUNCTION_ARG_INVALID);
}
