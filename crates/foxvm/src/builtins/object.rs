//! Object functions. Creating an object and adding a property are side effects, so they are
//! yielded as host requests; asking what a member is stays synchronous.

use super::{
    BuiltinCtx, BuiltinResult, BuiltinSpec, VARIADIC, any_null, arg_int, arg_str, not_available, ok, opt_int,
    opt_str, spec,
};
use crate::error::RtError;
use crate::host::{HostRequest, JsonValue, Member, MemberInfo, MemberKind};
use crate::value::Value;

/// CREATEOBJECT(class [, args...]). Which classes exist is up to the host; M2 offers Empty,
/// Custom and Form.
fn f_createobject(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let class = arg_str(&a, 0)?.to_string();
    let args = a[1..].iter().map(JsonValue::from_value).collect();
    // `definition` is filled in by the VM when the program defines a class of that name.
    Ok(BuiltinResult::Suspend(HostRequest::CreateObject { class, args, definition: None, module: String::new() }))
}

/// NEWOBJECT(class [, module] [, inapp] [, args...]). VFP's difference from CREATEOBJECT is
/// that it can name the file the class lives in, and the file is loaded for the call whether or
/// not `SET CLASSLIB` has it: measured, `SET("CLASSLIB")` is no longer for it afterwards, so
/// the file is read for this object alone.
///
/// The third argument names an application to look in rather than a folder, which this runtime
/// does not open; it is accepted and the class is looked for in the module named instead.
fn f_newobject(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let class = arg_str(&a, 0)?.to_string();
    let module = match a.get(1).map(Value::deref) {
        Some(Value::Str(s)) if !s.trim().is_empty() => c.settings().class_library_at(&s),
        _ => String::new(),
    };
    // arguments past the class name and its module/inapp pair are the constructor's
    let args = if a.len() > 3 { a[3..].iter().map(JsonValue::from_value).collect() } else { Vec::new() };
    Ok(BuiltinResult::Suspend(HostRequest::CreateObject { class, args, definition: None, module }))
}

/// ADDPROPERTY(obj, name, value): the value defaults to .F., like VFP.
fn f_addproperty(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let obj = a[0].as_object()?.0;
    let name = arg_str(&a, 1)?.to_string();
    let value = JsonValue::from_value(a.get(2).unwrap_or(&Value::Logical(false)));
    Ok(BuiltinResult::Suspend(HostRequest::AddProperty { obj, name, value }))
}

/// REMOVEPROPERTY(obj, name): the host owns the object's property bag, so removing one is a
/// request. Resumes with .T. when the property was there to remove.
fn f_removeproperty(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let obj = a[0].as_object()?.0;
    let name = arg_str(&a, 1)?.to_string();
    Ok(BuiltinResult::Suspend(HostRequest::RemoveProperty { obj, name }))
}

/// PEMSTATUS(obj, name, n): only n = 5 ("does this member exist") is answered; the other
/// attributes VFP reports (read-only, protected, changed...) are .F. here.
fn f_pemstatus(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let obj = a[0].as_object()?;
    let name = arg_str(&a, 1)?.to_string();
    let which = arg_int(&a, 2)?;
    if which != 5 {
        return ok(Value::Logical(false));
    }
    let member = c.host().get_member(obj, &name).unwrap_or(Member::None);
    ok(Value::Logical(!matches!(member, Member::None)))
}

// ------------------------------------------------------------------------------------------
// asking an object what it is made of
// ------------------------------------------------------------------------------------------

/// The members of whatever the call named: an object that exists, or a class by name.
///
/// `None` when the name means nothing here, which is what both functions refuse on.
fn members_of(c: &mut dyn BuiltinCtx, of: &Value) -> Option<Vec<MemberInfo>> {
    match of.deref() {
        Value::Object(h) => c.object_members(h),
        Value::Str(s) => c.class_members(&s),
        _ => None,
    }
}

/// The letters `cFlags` may hold, and what each one picks out.
///
/// Protected and hidden are in the list because the product takes them, and pick nothing here:
/// this runtime has no protected or hidden members, so every member is a public one.
fn flag_matches(flag: char, m: &MemberInfo) -> Option<bool> {
    Some(match flag {
        'P' | 'H' => false,
        'G' => true,
        'N' => m.native,
        'U' => !m.native,
        'C' => m.changed,
        'I' => !m.added,
        'B' => m.added,
        'R' => m.read_only,
        _ => return None,
    })
}

/// `AMEMBERS(ArrayName, oObject | cClass [, nType] [, cFlags])`: what an object is made of.
///
/// `nType` says what to list - 0 the property names in one column, 1 every member with the word
/// for what it is beside it, 2 the member objects. The flags filter, and a call that gives
/// several is asking for a member that answers to any of them, which is what the product does
/// rather than the "and" the grouping in the reference suggests.
fn f_amembers(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let kind = opt_int(&a, 2, 0)?;
    if kind == 3 {
        return Err(RtError::feature_not_available(
            "AMEMBERS(): the four-column form (nArrayContentsID 3) answers with each member's \
             parameter list and the sentence the help file gives for it, and this runtime carries \
             neither; AMEMBERS(a, o, 1) gives the same names with what each one is",
        ));
    }
    if !(0..=2).contains(&kind) {
        return Err(RtError::function_arg_invalid());
    }
    let flags = opt_str(&a, 3, "")?.to_ascii_uppercase();
    let members = members_of(c, &a[1]).ok_or_else(RtError::function_arg_invalid)?;

    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut names: Vec<&MemberInfo> = Vec::new();
    for m in &members {
        let wanted = match kind {
            0 => m.kind == MemberKind::Property,
            2 => m.kind == MemberKind::Object,
            _ => true,
        };
        if !wanted {
            continue;
        }
        // no flags at all asks for everything a program can reach
        let passes = if flags.is_empty() {
            true
        } else {
            let mut any = false;
            for f in flags.chars() {
                any |= flag_matches(f, m).ok_or_else(RtError::function_arg_invalid)?;
            }
            any
        };
        if passes {
            names.push(m);
        }
    }
    names.sort_by(|x, y| x.name.cmp(&y.name));
    for m in names {
        rows.push(match kind {
            1 => vec![Value::str(m.name.clone()), Value::str(m.kind.word())],
            _ => vec![Value::str(m.name.clone())],
        });
    }
    super::array::fill(&a[0], rows)
}

/// `GETPEM(oObject | cClass, cName)`: what a member holds.
///
/// A property answers with its value - the object's now, or the one the class starts it at when
/// the call named a class. A method or an event answers with its source, which outside an
/// interactive session the product does not hand over: it answers with an empty string, and so
/// does this.
fn f_getpem(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 1)?.trim().to_ascii_uppercase();
    let members = members_of(c, &a[0]).ok_or_else(RtError::function_arg_invalid)?;
    let member = members.iter().find(|m| m.name == name).ok_or_else(RtError::function_arg_invalid)?;
    match member.kind {
        MemberKind::Method | MemberKind::Event => ok(Value::str("")),
        _ => ok(member.value.clone().unwrap_or(Value::Logical(false))),
    }
}

/// The classes a control's position is measured from, rather than counted through.
const A_FRAME_OF_ITS_OWN: &[&str] = &["FORM", "FORMSET", "TOOLBAR", "SCREEN"];

/// A property of an object read as a number, or 0 when it has no such property.
fn number_prop(c: &mut dyn BuiltinCtx, h: crate::value::Handle, name: &str) -> f64 {
    c.host().get_prop(h, name).ok().and_then(|v| v.as_number().ok()).unwrap_or(0.0)
}

/// `OBJTOCLIENT(oControl, nWhich)`: where a control sits on the form it is on, and how big it is.
///
/// 1 and 2 are the top and the left, counted through every container between the control and
/// the form, because a control's own Top is measured from whatever holds it. 3 and 4 are its
/// own width and height. The form is where the counting stops: it is what the answer is
/// relative to.
///
/// What the product also adds, and this cannot, is the chrome each container takes off its own
/// inside - the tab strip of a page frame, a container's border - which needs a layout pass
/// this runtime has no equivalent of. A control straight on a form, which is most of them,
/// answers exactly.
fn f_objtoclient(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Value::Object(obj) = a[0].deref() else { return Err(RtError::data_type_mismatch()) };
    let which = arg_int(&a, 1)?;
    let name = match which {
        1 => "TOP",
        2 => "LEFT",
        3 => return ok(Value::number(number_prop(c, obj, "WIDTH"))),
        4 => return ok(Value::number(number_prop(c, obj, "HEIGHT"))),
        _ => return Err(RtError::function_arg_invalid()),
    };
    let mut total = number_prop(c, obj, name);
    let mut at = obj;
    // a container that holds itself would never end, and no form is a hundred deep
    for _ in 0..100 {
        let Ok(Value::Object(parent)) = c.host().get_prop(at, "PARENT").map(|v| v.deref()) else { break };
        let class = c.host().get_prop(parent, "BASECLASS").ok().and_then(|v| v.as_str().ok().map(|s| s.to_string()));
        if class.is_some_and(|k| A_FRAME_OF_ITS_OWN.iter().any(|f| k.eq_ignore_ascii_case(f))) {
            break;
        }
        total += number_prop(c, parent, name);
        at = parent;
    }
    ok(Value::number(total))
}

not_available! {
    f_comclassinfo => "COMCLASSINFO(): what a COM class says about itself is read from its type \
        library, which the addon this runtime talks to COM through does not open; the object \
        itself answers to CREATEOBJECT() and to its own members";
    f_getinterface => "GETINTERFACE(): asking an object for another of its interfaces needs the \
        interface's own description from a type library, which this runtime does not read; the automation \
        interface is the one every member is reached through";
    f_eventhandler => "EVENTHANDLER(): binding to the events a COM object raises needs a \
        connection point of our own for it to call back into, which this runtime does not put \
        up; poll the object, or have it call a program with DO";
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("ADDPROPERTY", 2, 3, f_addproperty),
        spec("AMEMBERS", 2, 4, f_amembers),
        spec("COMARRAY", 1, 2, f_comarray),
        spec("COMCLASSINFO", 1, 2, f_comclassinfo),
        spec("COMPOBJ", 2, 2, f_compobj),
        spec("COMPROP", 2, 3, f_comprop),
        spec("COMRETURNERROR", 2, 2, f_comreturnerror),
        spec("CREATEOBJECT", 1, VARIADIC, f_createobject),
        spec("CREATEOBJECTEX", 1, 3, f_createobjectex),
        spec("DODEFAULT", 0, VARIADIC, f_dodefault),
        spec("EVENTHANDLER", 2, 3, f_eventhandler),
        spec("GETINTERFACE", 2, 3, f_getinterface),
        spec("GETOBJECT", 1, 2, f_getobject),
        spec("GETPEM", 2, 2, f_getpem),
        spec("NEWOBJECT", 1, VARIADIC, f_newobject),
        spec("OBJTOCLIENT", 2, 2, f_objtoclient),
        spec("PEMSTATUS", 3, 3, f_pemstatus),
        spec("REMOVEPROPERTY", 2, 2, f_removeproperty),
    ]
}

// ------------------------------------------------------------------------------------------
// the COM surface: reaching something that is already there, and how values cross to it
// ------------------------------------------------------------------------------------------

/// `GETOBJECT(cFileName | cName [, cClassName])`: an object that is already running, or the one
/// a document stands for.
fn f_getobject(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let name = arg_str(&a, 0)?.trim().to_string();
    let class = match a.get(1) {
        Some(v) => v.deref().as_str()?.trim().to_string(),
        None => String::new(),
    };
    Ok(BuiltinResult::Suspend(HostRequest::GetObject { name, class }))
}

/// `CREATEOBJECTEX(cCLSID | cProgID, cComputerName, cIID)`: the same object CREATEOBJECT()
/// makes, on the machine the call names.
fn f_createobjectex(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // the reference page gives cComputerName with no brackets around it at all, and the product
    // holds to that: the compiler's own arity check only knows the widest range the name takes,
    // 1 to 3, so a call with the class name alone gets through and is caught here - measured as
    // error 1229, "Too few arguments", the same shape of gap as `DISPLAYPATH(cFileName)`'s.
    if a.len() < 2 {
        return Err(RtError::too_few_args());
    }
    if any_null(&a) {
        return ok(Value::Null);
    }
    let class = arg_str(&a, 0)?.trim().to_string();
    let computer = match a.get(1) {
        Some(v) => v.deref().as_str()?.trim().to_string(),
        None => String::new(),
    };
    // an empty computer name is this one, which is the only machine this runtime reaches
    if !computer.is_empty() {
        return Err(RtError::new(
            1429,
            format!(
                "CREATEOBJECTEX(): an object on another machine - '{computer}' - needs distributed COM, \
                 which this runtime does not ask for; leave the computer name out to make it here"
            ),
        ));
    }
    Ok(BuiltinResult::Suspend(HostRequest::CreateObject { class, args: Vec::new(), definition: None, module: String::new() }))
}

/// `DODEFAULT([eParameter, ...])`: the method of the same name on the class this one was built
/// from. It only means anything inside a method, which is where the object and the name come
/// from.
fn f_dodefault(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(obj) = c.running_object() else {
        return Err(RtError::new(1928, "DODEFAULT() can only be used inside a method"));
    };
    let from = c.running_method();
    let method = from.rsplit('.').next().unwrap_or(&from).to_string();
    if method.is_empty() {
        return Err(RtError::new(1928, "DODEFAULT() can only be used inside a method"));
    }
    // `DODEFAULT(@m.nError, ...)` hands the parent the same variables
    let args = a.iter().map(JsonValue::from_arg).collect();
    Ok(BuiltinResult::Suspend(HostRequest::CallParentMethod { obj, method, args, from }))
}

/// `COMPROP(oObject, cProperty [, eValue])`: a setting on how the runtime talks to that COM
/// object, and what it was.
fn f_comprop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Value::Object(handle) = a[0].deref() else { return Err(RtError::function_arg_invalid()) };
    let name = arg_str(&a, 1)?.trim().to_ascii_uppercase();
    let before = c.com_property(handle.0, &name);
    if let Some(wanted) = a.get(2) {
        c.set_com_property(handle.0, &name, wanted.deref());
    }
    ok(before)
}

/// `COMPOBJ(oObject1, oObject2)`: are the two references the same object.
fn f_compobj(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let same = match (a[0].deref(), a[1].deref()) {
        (Value::Object(one), Value::Object(two)) => one == two,
        _ => false,
    };
    ok(Value::Logical(same))
}

/// `COMARRAY(oObject, nArrayType)`: how an array is passed to that object, and what it was.
///
/// The runtime passes arrays one way - by value, as COM's own default - so the setting is kept
/// and answered rather than acted on.
fn f_comarray(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Value::Object(handle) = a[0].deref() else { return Err(RtError::function_arg_invalid()) };
    let before = c.com_setting(handle.0);
    if let Some(wanted) = a.get(1) {
        c.set_com_setting(handle.0, wanted.deref().as_number()? as i64);
    }
    ok(Value::number(before as f64))
}

/// `COMRETURNERROR(cSource, cDescription)`: the error a COM server hands back to whoever called
/// it. Nothing here is a server, so the error is kept as the last one raised.
fn f_comreturnerror(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let source = arg_str(&a, 0)?.trim().to_string();
    let description = match a.get(1) {
        Some(v) => v.deref().as_str()?.to_string(),
        None => String::new(),
    };
    let _ = c;
    Err(RtError::new(1429, format!("{source}: {description}")))
}
