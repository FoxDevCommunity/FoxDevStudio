//! The printer functions: what printers there are, which one is chosen, and whether one is ready.
//!
//! Only the host knows what is attached, so each of these yields a request and runs again when
//! the answer is in. A host with no printer answers an empty list, which is what Visual FoxPro
//! reports when there is nothing to print to.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, opt_int, opt_str, spec};
use crate::error::RtError;
use crate::host::HostRequest;
use crate::value::{FoxArray, Value};

fn ok(v: Value) -> Result<BuiltinResult, RtError> {
    Ok(BuiltinResult::Value(v))
}

/// The names the host sent back.
fn printers(reply: &Value) -> Vec<Value> {
    match reply.deref() {
        Value::Array(list) => list.borrow().items.clone(),
        Value::Str(name) if !name.is_empty() => vec![Value::str(name.to_string())],
        _ => Vec::new(),
    }
}

/// APRINTERS(ArrayName): fills an array with the printers there are, and answers how many.
/// Visual FoxPro gives two columns - the name and the port - so this does too.
fn f_aprinters(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(reply) = c.take_data_reply() else {
        return Ok(BuiltinResult::SuspendData { request: HostRequest::Printers { choose: false }, args: a });
    };
    let names = printers(&reply);
    let array = super::array::array_of(&a[0])?;
    let mut held = array.borrow_mut();
    if names.is_empty() {
        return ok(Value::number(0.0));
    }
    *held = FoxArray::new(names.len(), 2);
    for (i, name) in names.iter().enumerate() {
        held.items[i * 2] = name.clone();
        held.items[i * 2 + 1] = Value::str("");
    }
    ok(Value::number(names.len() as f64))
}

/// GETPRINTER(): shows the printer chooser and answers the one that was picked, or "" when
/// the user changed their mind.
fn f_getprinter(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(reply) = c.take_data_reply() else {
        return Ok(BuiltinResult::SuspendData { request: HostRequest::Printers { choose: true }, args: a });
    };
    ok(Value::str(reply.as_str().map(|s| s.to_string()).unwrap_or_default()))
}

/// PRINTSTATUS(): whether there is a printer to print to.
fn f_printstatus(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(reply) = c.take_data_reply() else {
        return Ok(BuiltinResult::SuspendData { request: HostRequest::Printers { choose: false }, args: a });
    };
    ok(Value::Logical(!printers(&reply).is_empty()))
}

/// PRTINFO(nSetting [, cPrinter]): one setting of the printer, as a number. Nothing here drives
/// a printer itself, so each setting reads back as the one a job would start with; naming a
/// printer that is not among the host's own answers -1, measured off a name this machine does
/// not have, the same way the reference says an unavailable setting does.
fn f_prtinfo(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = opt_int(&a, 0, 0)?;
    let named = opt_str(&a, 1, "")?.trim().to_string();
    if !named.is_empty() {
        let Some(reply) = c.take_data_reply() else {
            return Ok(BuiltinResult::SuspendData { request: HostRequest::Printers { choose: false }, args: a });
        };
        let known = printers(&reply)
            .iter()
            .any(|v| v.as_str().is_ok_and(|s| s.eq_ignore_ascii_case(&named)));
        if !known {
            return ok(Value::number(-1.0));
        }
    }
    // 1 the orientation, portrait; 2 the paper size, Letter; 3 the paper size again for want of
    // a real one; 6 how many copies; 9 the colour capability, colour; the rest none
    ok(Value::number(match which {
        1 => 0.0,
        2 | 3 | 6 => 1.0,
        9 => 2.0,
        _ => 0.0,
    }))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("APRINTERS", 1, 1, f_aprinters),
        spec("GETPRINTER", 0, 0, f_getprinter),
        spec("PRINTSTATUS", 0, 0, f_printstatus),
        spec("PRTINFO", 1, 2, f_prtinfo),
    ]
}
