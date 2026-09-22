//! The `FoxScript` namespace: the objects the VM answers for itself.
//!
//! Everything else in the object model lives in the host, because that is where a form and its
//! controls live. `FoxScript` does not, and the reason is `FoxScript.Data.CursorToJson()`: a
//! cursor is in the VM's own data engine and the host cannot read one. `FoxScript.Json` is the
//! same, because a JSON value is a `Value`. So the namespace is an object of the object model
//! whose members the VM answers, and the things it makes - a server's sockets, a request, a
//! response - are the host's, because that is where the sockets are.
//!
//! Nothing new was needed to make `THIS`, a member read and a method call work on one. A native
//! object is an ordinary `Value::Object` whose handle comes out of a range the host never
//! allocates from, so every path that already carries an object carries this one, and a handle
//! that does escape to the host is refused by name rather than quietly read as another object.
//!
//! See docs/foxscript.md.

use std::collections::HashMap;

use crate::error::RtError;
use crate::host::{MemberInfo, MemberKind};
use crate::value::{Handle, Value};

/// Where the handles of the VM's own objects start.
///
/// The host counts its handles up from 1 and reserves `0x7fff_ffff` for the application object,
/// so this range is its own. It is checked rather than assumed: `tests/foxscript.rs` asserts
/// that a handle in it is never mistaken for a host object.
pub const NATIVE_BASE: u32 = 0xF000_0000;

/// The root: the `FoxScript` name itself.
pub const ROOT: u32 = 0;
/// `FoxScript.Http`.
pub const HTTP: u32 = 1;
/// `FoxScript.Data`.
pub const DATA: u32 = 2;
/// `FoxScript.Json`.
pub const JSON: u32 = 3;

/// What this runtime calls itself when a program asks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The error number FoxScript's own failures carry.
///
/// Visual FoxPro's numbers stop a long way below this, so a program can tell one of its errors
/// from one of the product's by the number alone and nothing here ever collides with a number
/// the product may add. The message says which failure it was, as the product's own do.
pub const FOXSCRIPT_ERROR: u32 = 3001;

/// The HTTP methods a route may be registered under. A server's method of the same name
/// registers one, so the list is both the grammar and the documentation.
pub const HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeClass {
    Root,
    Http,
    Data,
    Json,
    Server,
}

pub struct NativeMember {
    /// Upper-cased, which is how AMEMBERS() writes it.
    pub name: &'static str,
    pub kind: MemberKind,
    /// The object this member is, for a member that is one; unread otherwise.
    pub child: u32,
}

const fn property(name: &'static str) -> NativeMember {
    NativeMember { name, kind: MemberKind::Property, child: ROOT }
}

const fn method(name: &'static str) -> NativeMember {
    NativeMember { name, kind: MemberKind::Method, child: ROOT }
}

const fn child(name: &'static str, child: u32) -> NativeMember {
    NativeMember { name, kind: MemberKind::Object, child }
}

const ROOT_MEMBERS: &[NativeMember] =
    &[child("DATA", DATA), child("HTTP", HTTP), child("JSON", JSON), property("VERSION")];
const HTTP_MEMBERS: &[NativeMember] = &[method("CREATESERVER")];
const DATA_MEMBERS: &[NativeMember] = &[method("CURSORTOJSON"), method("JSONTOCURSOR")];
const JSON_MEMBERS: &[NativeMember] =
    &[method("COUNT"), method("GET"), method("HAS"), method("KEYS"), method("PARSE"), method("STRINGIFY")];
const SERVER_MEMBERS: &[NativeMember] = &[
    method("CLOSE"),
    method("DELETE"),
    method("GET"),
    method("HEAD"),
    method("LISTEN"),
    method("OPTIONS"),
    method("PATCH"),
    method("POST"),
    method("PUT"),
    property("PORT"),
];

impl NativeClass {
    pub fn name(self) -> &'static str {
        match self {
            NativeClass::Root => "FoxScript",
            NativeClass::Http => "FoxScript.Http",
            NativeClass::Data => "FoxScript.Data",
            NativeClass::Json => "FoxScript.Json",
            NativeClass::Server => "FoxScript.Http.Server",
        }
    }

    pub fn members(self) -> &'static [NativeMember] {
        match self {
            NativeClass::Root => ROOT_MEMBERS,
            NativeClass::Http => HTTP_MEMBERS,
            NativeClass::Data => DATA_MEMBERS,
            NativeClass::Json => JSON_MEMBERS,
            NativeClass::Server => SERVER_MEMBERS,
        }
    }

    pub fn member(self, name: &str) -> Option<&'static NativeMember> {
        self.members().iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }
}

/// A server's routes and the port it is listening on. The routes are matched by `matchit`, the
/// crate everything else in this shape uses, rather than by a pattern language of our own.
#[derive(Default)]
pub struct Server {
    /// Upper-cased HTTP method -> the routes registered under it, each holding a function id.
    routers: HashMap<String, matchit::Router<u32>>,
    /// The paths registered under each method, so a duplicate is refused by name.
    paths: HashMap<String, Vec<String>>,
    pub port: Option<u16>,
}

impl Server {
    /// Registers a handler. A route registered twice under one method is refused rather than
    /// silently replaced, because a program that does it has two answers for one question.
    pub fn route(&mut self, method: &str, path: &str, func: u32) -> Result<(), RtError> {
        let method = method.to_ascii_uppercase();
        let taken = self.paths.entry(method.clone()).or_default();
        if taken.iter().any(|p| p == path) {
            return Err(RtError::new(FOXSCRIPT_ERROR, format!("{method} {path} already has a handler.")));
        }
        let router = self.routers.entry(method).or_default();
        router
            .insert(path.to_string(), func)
            .map_err(|e| RtError::new(FOXSCRIPT_ERROR, format!("{path} is not a route: {e}")))?;
        taken.push(path.to_string());
        Ok(())
    }

    /// The handler for a request, and what its named parts matched.
    pub fn handler(&self, method: &str, path: &str) -> Option<(u32, Vec<(String, String)>)> {
        let router = self.routers.get(&method.to_ascii_uppercase())?;
        let found = router.at(path).ok()?;
        let params = found.params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Some((*found.value, params))
    }
}

pub struct Native {
    pub class: NativeClass,
    pub server: Option<Server>,
}

/// Every object of the namespace that is alive, indexed by handle.
pub struct Natives {
    items: Vec<Native>,
}

impl Default for Natives {
    fn default() -> Self {
        Natives {
            // in the order their ids name them, because a handle is where an object sits here
            items: vec![
                Native { class: NativeClass::Root, server: None },
                Native { class: NativeClass::Http, server: None },
                Native { class: NativeClass::Data, server: None },
                Native { class: NativeClass::Json, server: None },
            ],
        }
    }
}

impl Natives {
    pub fn get(&self, h: Handle) -> Option<&Native> {
        index(h).and_then(|i| self.items.get(i))
    }

    pub fn get_mut(&mut self, h: Handle) -> Option<&mut Native> {
        index(h).and_then(|i| self.items.get_mut(i))
    }

    pub fn server(&self, h: Handle) -> Option<&Server> {
        self.get(h)?.server.as_ref()
    }

    pub fn server_mut(&mut self, h: Handle) -> Option<&mut Server> {
        self.get_mut(h)?.server.as_mut()
    }

    /// A new object of the namespace; the handle is where it sits in the table.
    pub fn add(&mut self, class: NativeClass) -> Handle {
        let server = (class == NativeClass::Server).then(Server::default);
        self.items.push(Native { class, server });
        handle(self.items.len() as u32 - 1)
    }

    pub fn contains(&self, h: Handle) -> bool {
        self.get(h).is_some()
    }

    /// What `AMEMBERS()` is to list. Every member is native and none of them can be written, so
    /// a program that asks is told the truth rather than a shape borrowed from a form.
    pub fn member_info(&self, h: Handle) -> Option<Vec<MemberInfo>> {
        let native = self.get(h)?;
        Some(
            native
                .class
                .members()
                .iter()
                .map(|m| {
                    let mut info = MemberInfo::property(m.name, self.property(h, m.name));
                    info.kind = m.kind;
                    info.native = true;
                    info.read_only = m.kind == MemberKind::Property;
                    if m.kind != MemberKind::Property {
                        info.value = None;
                    }
                    info
                })
                .collect(),
        )
    }

    /// What a property of one of these objects holds.
    pub fn property(&self, h: Handle, name: &str) -> Value {
        let Some(native) = self.get(h) else { return Value::Logical(false) };
        match (native.class, name.to_ascii_uppercase().as_str()) {
            (NativeClass::Root, "VERSION") => Value::str(VERSION),
            // a server that has not been told to listen has no port, and says so as the product
            // says it: .F. rather than a number nobody may use
            (NativeClass::Server, "PORT") => match native.server.as_ref().and_then(|s| s.port) {
                Some(port) => Value::number(f64::from(port)),
                None => Value::Logical(false),
            },
            _ => Value::Logical(false),
        }
    }
}

/// The handle that stands for the object at that place in the table.
pub const fn handle(id: u32) -> Handle {
    Handle(NATIVE_BASE + id)
}

/// True for a handle in the VM's own range, whether or not an object of that handle is alive.
pub fn in_range(h: Handle) -> bool {
    h.0 >= NATIVE_BASE
}

fn index(h: Handle) -> Option<usize> {
    in_range(h).then(|| (h.0 - NATIVE_BASE) as usize)
}

/// The root object as a value, which is what the name `FoxScript` reads as.
pub fn root() -> Value {
    Value::Object(handle(ROOT))
}
