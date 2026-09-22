//! Event binding: BINDEVENT(), UNBINDEVENT() and RAISEEVENT().
//!
//! The binding table cannot live in the VM. A bound event fires when the *host* runs the source
//! object's method - a click, a property change, a timer - and the host is what owns objects and
//! decides when their methods run; the VM only sees the calls a program makes itself. So all
//! three functions do their argument checking here and then yield a request, and the host keeps
//! the table and dispatches.
//!
//! What each answers with was measured: BINDEVENT answers with how many delegates are bound to
//! that event once it is done, so a second delegate on the same event answers 2 and binding the
//! same pair twice changes nothing and answers the same number again; UNBINDEVENTS answers with
//! how many bindings it removed; RAISEEVENT answers with a logical.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, VARIADIC, arg_str, opt_int, spec};
use crate::error::RtError;
use crate::host::{HostRequest, JsonValue};
use crate::value::Value;

/// An object argument. Anything else is error 11, the way Visual FoxPro answers it: these are
/// function arguments, so a string where an object belongs is a bad argument and not the
/// operand type mismatch that the same string would be in an expression.
fn object_arg(a: &[Value], i: usize) -> Result<u32, RtError> {
    let value = a.get(i).ok_or_else(RtError::function_arg_invalid)?;
    Ok(value.as_object().map_err(|_| RtError::function_arg_invalid())?.0)
}

/// An omitted or .NULL. argument means "match anything" in UNBINDEVENT.
fn omitted(a: &[Value], i: usize) -> bool {
    matches!(a.get(i).map(|v| v.deref()), None | Some(Value::Null))
}

/// A non-empty event or method name; VFP rejects a blank one.
fn name_arg(a: &[Value], i: usize) -> Result<String, RtError> {
    let name = arg_str(a, i)?.trim().to_string();
    if name.is_empty() {
        return Err(RtError::function_arg_invalid());
    }
    Ok(name)
}

/// BINDEVENT(oSource, cEvent, oHandler, cDelegate [, nFlags]): binds `oSource.cEvent` so that
/// `oHandler.cDelegate` runs with it. Resumes with how many delegates that event now has.
fn f_bindevent(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let source = object_arg(&a, 0)?;
    let event = name_arg(&a, 1)?;
    let handler = object_arg(&a, 2)?;
    let delegate = name_arg(&a, 3)?;
    // an object may not handle its own event: that is a bad argument, not a binding that does
    // nothing - measured
    if source == handler {
        return Err(RtError::function_arg_invalid());
    }
    // nFlags: bit 1 "delegate runs after the event", bit 2 "the delegate's return value wins",
    // bit 4 "unbind when either object is released". The host interprets them.
    let flags = opt_int(&a, 4, 0)?.clamp(0, i64::from(u32::MAX)) as u32;
    Ok(BuiltinResult::Suspend(HostRequest::BindEvent { source, event, handler, delegate, flags }))
}

/// UNBINDEVENTS(oEventSource, cEvent, oEventHandler, cDelegate) or UNBINDEVENTS(oEventObject):
/// the full binding, named the way BINDEVENT() named it to make it, or every binding the one
/// object is a source or a handler of. Two arguments and three are neither shape - measured as
/// error 11, the same as any other bad argument count for a function whose forms this runtime's
/// own arity check cannot tell apart, because it only knows the widest range the name ever
/// takes; none at all is error 1229, "Too few arguments", the same as `DISPLAYPATH()`'s.
/// Resumes with how many bindings went away.
fn f_unbindevent(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (source, event, handler, delegate) = match a.len() {
        0 => return Err(RtError::too_few_args()),
        1 if omitted(&a, 0) => (None, None, None, None),
        1 => (Some(object_arg(&a, 0)?), None, None, None),
        4 => (Some(object_arg(&a, 0)?), Some(name_arg(&a, 1)?), Some(object_arg(&a, 2)?), Some(name_arg(&a, 3)?)),
        _ => return Err(RtError::function_arg_invalid()),
    };
    Ok(BuiltinResult::Suspend(HostRequest::UnbindEvent { source, event, handler, delegate }))
}

/// RAISEEVENT(oSource, cEvent [, args...]): fires the event as if the source had raised it, so
/// every delegate bound to it runs. Resumes with a logical, which is what VFP answers with.
fn f_raiseevent(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let source = object_arg(&a, 0)?;
    let event = name_arg(&a, 1)?;
    let args = a[2..].iter().map(JsonValue::from_value).collect();
    Ok(BuiltinResult::Suspend(HostRequest::RaiseEvent { source, event, args }))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("BINDEVENT", 4, 5, f_bindevent),
        spec("RAISEEVENT", 2, VARIADIC, f_raiseevent),
        spec("UNBINDEVENTS", 0, 4, f_unbindevent),
    ]
}
