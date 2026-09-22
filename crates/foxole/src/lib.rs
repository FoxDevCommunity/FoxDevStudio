//! COM automation for FoxDev Studio.
//!
//! Visual FoxPro programs reach out to the rest of Windows through COM - Word, Excel, ADO,
//! the shell, any registered ActiveX object - and `CREATEOBJECT()` is how they do it. This is
//! the other side of that call: a Node addon that creates COM objects and forwards property
//! reads, property writes and method calls to them through `IDispatch`.
//!
//! It is deliberately synchronous. The VM asks the host for a property while it is on the
//! stack, so an answer that arrived later would be no answer at all; every call here returns
//! before it comes back. It is also single-threaded: the objects live in the apartment of the
//! thread that made them, which is the Electron main thread, and every call arrives there.
//!
//! Off Windows the addon is not built at all, and the runtime says so rather than pretending.

use napi_derive::napi;

#[cfg(windows)]
mod com;

/// What a value crossing this bridge looks like.
///
/// COM's own `VARIANT` holds far more than a FoxPro program can hold; this is the part both
/// sides have. An object is a handle into the table this addon keeps, so a program can hold
/// `oWord.Documents` and go on calling into it.
#[napi(object)]
#[derive(Default)]
pub struct OleValue {
    /// `null`, `number`, `string`, `bool`, `date` or `object`.
    pub kind: String,
    pub num: Option<f64>,
    pub text: Option<String>,
    pub flag: Option<bool>,
    pub handle: Option<u32>,
}

impl OleValue {
    pub fn null() -> Self {
        OleValue { kind: "null".into(), ..Default::default() }
    }
    pub fn number(n: f64) -> Self {
        OleValue { kind: "number".into(), num: Some(n), ..Default::default() }
    }
    pub fn string(s: String) -> Self {
        OleValue { kind: "string".into(), text: Some(s), ..Default::default() }
    }
    pub fn boolean(b: bool) -> Self {
        OleValue { kind: "bool".into(), flag: Some(b), ..Default::default() }
    }
    pub fn object(handle: u32) -> Self {
        OleValue { kind: "object".into(), handle: Some(handle), ..Default::default() }
    }
    /// A date, as milliseconds since the epoch: what JavaScript counts in.
    pub fn date(millis: f64) -> Self {
        OleValue { kind: "date".into(), num: Some(millis), ..Default::default() }
    }
}

/// Whether this build can talk to COM at all. False everywhere but Windows.
#[napi]
pub fn available() -> bool {
    cfg!(windows)
}

/// Creates an object by ProgID (`Word.Application`) or by class id in braces.
#[napi]
pub fn create(name: String) -> napi::Result<u32> {
    #[cfg(windows)]
    {
        com::create(&name).map_err(to_napi)
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        Err(unsupported())
    }
}

/// `GETOBJECT()`: an object that is already running, or the one a file stands for.
#[napi]
pub fn active(name: String, class_name: String) -> napi::Result<u32> {
    #[cfg(windows)]
    {
        com::active(&name, &class_name).map_err(to_napi)
    }
    #[cfg(not(windows))]
    {
        let _ = (name, class_name);
        Err(unsupported())
    }
}

/// Reads a property, or calls a method that takes arguments and is written as one.
#[napi]
pub fn get(handle: u32, name: String, args: Vec<OleValue>) -> napi::Result<OleValue> {
    #[cfg(windows)]
    {
        com::get(handle, &name, &args).map_err(to_napi)
    }
    #[cfg(not(windows))]
    {
        let _ = (handle, name, args);
        Err(unsupported())
    }
}

/// Writes a property.
#[napi]
pub fn set(handle: u32, name: String, value: OleValue) -> napi::Result<()> {
    #[cfg(windows)]
    {
        com::set(handle, &name, &value).map_err(to_napi)
    }
    #[cfg(not(windows))]
    {
        let _ = (handle, name, value);
        Err(unsupported())
    }
}

/// Calls a method.
#[napi]
pub fn call(handle: u32, name: String, args: Vec<OleValue>) -> napi::Result<OleValue> {
    #[cfg(windows)]
    {
        com::call(handle, &name, &args).map_err(to_napi)
    }
    #[cfg(not(windows))]
    {
        let _ = (handle, name, args);
        Err(unsupported())
    }
}

/// Lets go of one object. An object nobody holds is released by the server.
#[napi]
pub fn release(handle: u32) {
    #[cfg(windows)]
    com::release(handle);
    #[cfg(not(windows))]
    let _ = handle;
}

/// Lets go of everything: the end of a run.
#[napi]
pub fn release_all() {
    #[cfg(windows)]
    com::release_all();
}

#[cfg(windows)]
fn to_napi(error: com::OleError) -> napi::Error {
    // the code travels in the message: the runtime turns it back into a VFP error number
    napi::Error::new(napi::Status::GenericFailure, format!("{:#010x}|{}", error.code, error.message))
}

#[cfg(not(windows))]
fn unsupported() -> napi::Error {
    napi::Error::new(napi::Status::GenericFailure, "COM is only available on Windows".to_string())
}
