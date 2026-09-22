//! `IDispatch` automation: the Windows half of the bridge.
//!
//! Everything here runs on the thread that calls it, which is the Electron main thread, and the
//! objects stay in that thread's apartment. Nothing is sent anywhere else, and nothing is kept
//! but the objects a program is still holding.

use std::cell::RefCell;
use std::collections::HashMap;

use windows::Win32::Foundation::{DISP_E_EXCEPTION, E_FAIL, VARIANT_BOOL};
use windows::Win32::System::Com::{
    CoGetObject,
    CLSCTX_ALL, CLSIDFromProgID, CLSIDFromString, CoCreateInstance, CoInitializeEx, COINIT_APARTMENTTHREADED, DISPATCH_METHOD,
    DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS, EXCEPINFO, IDispatch,
};
use windows::Win32::System::Ole::GetActiveObject;
use windows::Win32::System::Variant::{
    VARENUM, VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_BOOL, VT_BSTR, VT_DATE, VT_DISPATCH, VT_EMPTY, VT_NULL, VT_R8,
    VT_UNKNOWN, VariantChangeType,
};
use windows::core::{BSTR, GUID, HRESULT, IUnknown, Interface, PCWSTR};

use crate::OleValue;

/// A COM failure, as the runtime will report it.
pub struct OleError {
    pub code: u32,
    pub message: String,
}

impl OleError {
    fn new(code: HRESULT, message: impl Into<String>) -> Self {
        OleError { code: code.0 as u32, message: message.into() }
    }
}

impl From<windows::core::Error> for OleError {
    fn from(error: windows::core::Error) -> Self {
        OleError { code: error.code().0 as u32, message: error.message().trim().to_string() }
    }
}

thread_local! {
    /// Objects a program is holding, by handle. Dropping one releases it.
    static OBJECTS: RefCell<HashMap<u32, IDispatch>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: RefCell<u32> = const { RefCell::new(1) };
    static APARTMENT: RefCell<bool> = const { RefCell::new(false) };
}

/// Joins the thread's apartment, once. Electron's main thread is already in one, which is not a
/// failure: `CoInitializeEx` says so and the objects are made there either way.
fn enter_apartment() {
    APARTMENT.with(|done| {
        let mut done = done.borrow_mut();
        if *done {
            return;
        }
        unsafe {
            // the HRESULT is deliberately dropped: S_FALSE and RPC_E_CHANGED_MODE both mean
            // "someone got here first", which is exactly what is wanted
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        *done = true;
    });
}

fn keep(object: IDispatch) -> u32 {
    let handle = NEXT_HANDLE.with(|n| {
        let mut n = n.borrow_mut();
        let handle = *n;
        *n += 1;
        handle
    });
    OBJECTS.with(|objects| objects.borrow_mut().insert(handle, object));
    handle
}

fn find(handle: u32) -> Result<IDispatch, OleError> {
    OBJECTS
        .with(|objects| objects.borrow().get(&handle).cloned())
        .ok_or_else(|| OleError::new(HRESULT(0), format!("object {handle} has been released")))
}

/// `CREATEOBJECT("Word.Application")`, or a class id in braces.
pub fn create(name: &str) -> Result<u32, OleError> {
    enter_apartment();
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let clsid = unsafe {
        if name.trim_start().starts_with('{') {
            CLSIDFromString(PCWSTR(wide.as_ptr()))?
        } else {
            CLSIDFromProgID(PCWSTR(wide.as_ptr()))?
        }
    };
    let object: IDispatch = unsafe { CoCreateInstance(&clsid, None, CLSCTX_ALL)? };
    Ok(keep(object))
}

/// `GETOBJECT()`: an object that is already running, or one a file stands for.
///
/// A name is a class that is running - Word, Excel - and is found in the running object table.
/// A path is a document, and what comes back is the object that knows how to open it.
pub fn active(name: &str, class: &str) -> Result<u32, OleError> {
    enter_apartment();
    let wide = |text: &str| text.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    // a name with no class beside it and no path in it is a class that is already running
    let looks_like_a_file = name.contains(['\\', '/']) || (name.contains(':') && !class.is_empty());
    if !looks_like_a_file && !name.is_empty() {
        let text = wide(name);
        let clsid = unsafe {
            if name.trim_start().starts_with('{') {
                CLSIDFromString(PCWSTR(text.as_ptr()))?
            } else {
                CLSIDFromProgID(PCWSTR(text.as_ptr()))?
            }
        };
        let mut running: Option<IUnknown> = None;
        unsafe { GetActiveObject(&clsid, None, &mut running)? };
        let object: IDispatch = running.ok_or_else(|| OleError::new(E_FAIL, "nothing of that class is running"))?.cast()?;
        return Ok(keep(object));
    }
    // a file: the moniker for it is what knows which server opens it
    let path = if class.is_empty() { name.to_string() } else { format!("{name}!{class}") };
    let text = wide(&path);
    let object: IDispatch = unsafe { CoGetObject(PCWSTR(text.as_ptr()), None)? };
    Ok(keep(object))
}

pub fn release(handle: u32) {
    OBJECTS.with(|objects| objects.borrow_mut().remove(&handle));
}

pub fn release_all() {
    OBJECTS.with(|objects| objects.borrow_mut().clear());
}

pub fn get(handle: u32, name: &str, args: &[OleValue]) -> Result<OleValue, OleError> {
    // a member with arguments may be an indexed property or a method; COM is asked for either
    invoke(handle, name, args, DISPATCH_PROPERTYGET.0 | DISPATCH_METHOD.0)
}

pub fn call(handle: u32, name: &str, args: &[OleValue]) -> Result<OleValue, OleError> {
    invoke(handle, name, args, DISPATCH_METHOD.0 | DISPATCH_PROPERTYGET.0)
}

pub fn set(handle: u32, name: &str, value: &OleValue) -> Result<(), OleError> {
    invoke(handle, name, std::slice::from_ref(value), DISPATCH_PROPERTYPUT.0).map(|_| ())
}

/// The id a name has on an object. COM looks names up per object, not per class.
fn dispid(object: &IDispatch, name: &str) -> Result<i32, OleError> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut id = 0i32;
    unsafe {
        object.GetIDsOfNames(&GUID::zeroed(), &PCWSTR(wide.as_ptr()), 1, 0x0400, &mut id)?;
    }
    Ok(id)
}

/// The name of the property being written, which COM passes as a named argument.
const DISPID_PROPERTYPUT: i32 = -3;

fn invoke(handle: u32, name: &str, args: &[OleValue], flags: u16) -> Result<OleValue, OleError> {
    enter_apartment();
    let object = find(handle)?;
    let id = dispid(&object, name)?;

    // COM takes the arguments backwards
    let mut variants: Vec<VARIANT> = args.iter().rev().map(from_value).collect::<Result<_, _>>()?;
    let mut put = DISPID_PROPERTYPUT;
    let is_put = flags & DISPATCH_PROPERTYPUT.0 != 0;
    let params = DISPPARAMS {
        rgvarg: if variants.is_empty() { std::ptr::null_mut() } else { variants.as_mut_ptr() },
        rgdispidNamedArgs: if is_put { &mut put } else { std::ptr::null_mut() },
        cArgs: variants.len() as u32,
        cNamedArgs: u32::from(is_put),
    };

    let mut result = VARIANT::default();
    let mut fault = EXCEPINFO::default();
    let outcome = unsafe {
        object.Invoke(
            id,
            &GUID::zeroed(),
            0x0400,
            windows::Win32::System::Com::DISPATCH_FLAGS(flags),
            &params,
            Some(&mut result),
            Some(&mut fault),
            None,
        )
    };
    if let Err(error) = outcome {
        // an exception from the object itself carries its own description, which is the useful one
        if error.code() == DISP_E_EXCEPTION && !fault.bstrDescription.is_empty() {
            return Err(OleError::new(HRESULT(fault.scode), fault.bstrDescription.to_string()));
        }
        return Err(error.into());
    }
    unsafe { to_value(&result) }
}

/// Milliseconds since 1970 from an OLE date, which counts days from 1899-12-30.
fn millis_from_ole(date: f64) -> f64 {
    (date - 25569.0) * 86_400_000.0
}

fn ole_from_millis(millis: f64) -> f64 {
    millis / 86_400_000.0 + 25569.0
}

fn variant_of(vt: VARENUM, value: VARIANT_0_0_0) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(VARIANT_0_0 { vt, wReserved1: 0, wReserved2: 0, wReserved3: 0, Anonymous: value }),
        },
    }
}

fn from_value(value: &OleValue) -> Result<VARIANT, OleError> {
    Ok(match value.kind.as_str() {
        "number" => VARIANT::from(value.num.unwrap_or(0.0)),
        "string" => VARIANT::from(BSTR::from(value.text.clone().unwrap_or_default())),
        "bool" => VARIANT::from(value.flag.unwrap_or(false)),
        "date" => variant_of(VT_DATE, VARIANT_0_0_0 { date: ole_from_millis(value.num.unwrap_or(0.0)) }),
        "object" => {
            let object = find(value.handle.unwrap_or(0))?;
            variant_of(VT_DISPATCH, VARIANT_0_0_0 { pdispVal: std::mem::ManuallyDrop::new(Some(object)) })
        }
        _ => VARIANT::default(),
    })
}

/// # Safety
/// `value` must be a valid VARIANT; it is only read.
unsafe fn to_value(value: &VARIANT) -> Result<OleValue, OleError> {
    let vt = value.vt();
    let raw = unsafe { &value.Anonymous.Anonymous.Anonymous };
    Ok(match vt {
        VT_EMPTY | VT_NULL => OleValue::null(),
        VT_BOOL => OleValue::boolean(unsafe { raw.boolVal } != VARIANT_BOOL(0)),
        VT_BSTR => OleValue::string(unsafe { (*raw.bstrVal).to_string() }),
        VT_DATE => OleValue::date(millis_from_ole(unsafe { raw.date })),
        VT_DISPATCH => match unsafe { (*raw.pdispVal).clone() } {
            Some(object) => OleValue::object(keep(object)),
            None => OleValue::null(),
        },
        VT_UNKNOWN => match unsafe { (*raw.punkVal).clone() } {
            Some(unknown) => match unknown.cast::<IDispatch>() {
                Ok(object) => OleValue::object(keep(object)),
                // an object with no automation interface is nothing a FoxPro program can hold
                Err(_) => OleValue::null(),
            },
            None => OleValue::null(),
        },
        // every number COM has: let COM widen it rather than listing the types here
        _ => {
            let mut number = VARIANT::default();
            if unsafe { VariantChangeType(&mut number, value, Default::default(), VT_R8) }.is_ok() {
                return Ok(OleValue::number(unsafe { number.Anonymous.Anonymous.Anonymous.dblVal }));
            }
            let mut text = VARIANT::default();
            if unsafe { VariantChangeType(&mut text, value, Default::default(), VT_BSTR) }.is_ok() {
                return Ok(OleValue::string(unsafe { (*text.Anonymous.Anonymous.Anonymous.bstrVal).to_string() }));
            }
            OleValue::null()
        }
    })
}

/// Kept so the unused-import warning does not hide a real one: `IUnknown` is only named in a
/// cast above when the compiler needs it spelled out.
const _: Option<IUnknown> = None;
