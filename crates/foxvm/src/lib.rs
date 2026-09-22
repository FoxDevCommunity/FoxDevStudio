//! FoxVM: the FoxDev Studio FoxPro-like compiler and bytecode virtual machine.
//!
//! The crate is plain Rust; the `wasm` feature adds the wasm-bindgen surface used by the IDE
//! and the runtime player.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod ast;
pub mod base_classes;
pub mod builtins;
pub mod bytecode;
pub mod cdx;
pub mod dbc;
pub mod exchange;
pub mod idx;
pub mod json;
pub mod mem;
pub mod menu;
pub mod report;
pub mod screen;
pub mod xml;
pub mod compiler;
pub mod data;
pub mod dbf;
pub mod diagnostics;
pub mod error;
pub mod foxscript;
pub mod host;
pub mod lexer;
pub mod mock_host;
pub mod parser;
pub mod picture;
pub mod query;
pub mod value;
pub mod vm;

#[cfg(feature = "wasm")]
pub mod wasm;
