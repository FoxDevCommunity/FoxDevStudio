//! Compiles the HelloWorld sample form and checks the module survives encode/decode.

use std::fs;
use std::path::PathBuf;

use foxvm::bytecode::{ModuleKind, decode, encode};
use foxvm::compiler::{MethodSource, compile_form, compile_program, compile_snippet};

fn sample(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/samples").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Decodes a JSON string literal starting at `start` (the opening quote).
fn json_string(chars: &[char], start: usize) -> (String, usize) {
    assert_eq!(chars[start], '"');
    let mut out = String::new();
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '"' => return (out, i + 1),
            '\\' => {
                i += 1;
                match chars[i] {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'u' => {
                        let hex: String = chars[i + 1..i + 5].iter().collect();
                        out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap());
                        i += 4;
                    }
                    c => out.push(c),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    panic!("unterminated JSON string");
}

/// Walks the sample's object tree collecting (object path, event, source) for every method.
fn method_sources(json: &str) -> Vec<MethodSource> {
    let chars: Vec<char> = json.chars().collect();
    let mut out = Vec::new();
    // Path of names for the nesting of "children" arrays: each object with a "name" pushes it.
    let mut path: Vec<String> = Vec::new();
    let mut depth_at: Vec<usize> = Vec::new(); // brace depth at which each path entry was pushed
    let mut depth = 0usize;
    let mut i = 0;
    let mut in_form = false;
    while i < chars.len() {
        match chars[i] {
            '{' => {
                depth += 1;
                i += 1;
            }
            '}' => {
                while depth_at.last().is_some_and(|d| *d >= depth) {
                    depth_at.pop();
                    path.pop();
                }
                depth -= 1;
                i += 1;
            }
            '"' => {
                let (key, next) = json_string(&chars, i);
                i = next;
                let mut j = i;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if chars.get(j) != Some(&':') {
                    continue;
                }
                j += 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                i = j;
                match key.as_str() {
                    "form" => in_form = true,
                    "name" if in_form && chars.get(i) == Some(&'"') => {
                        let (v, next) = json_string(&chars, i);
                        i = next;
                        if depth > 2 {
                            path.push(v);
                            depth_at.push(depth);
                        }
                    }
                    "methods" if chars.get(i) == Some(&'{') => {
                        i += 1;
                        loop {
                            while i < chars.len() && chars[i] != '"' && chars[i] != '}' {
                                i += 1;
                            }
                            if chars[i] == '}' {
                                i += 1;
                                break;
                            }
                            let (event, next) = json_string(&chars, i);
                            i = next;
                            while chars[i] != '"' {
                                i += 1;
                            }
                            let (body, next) = json_string(&chars, i);
                            i = next;
                            out.push(MethodSource {
                                object_path: path.join("."),
                                event,
                                params: String::new(),
                                source: body,
                                include: String::new(),
                            });
                        }
                    }
                    _ => {}
                }
            }
            _ => i += 1,
        }
    }
    out
}

#[test]
fn hello_world_form_round_trips() {
    let methods = method_sources(&sample("HelloWorld.fxf"));
    let keys: Vec<String> = methods.iter().map(|m| format!("{}.{}", m.object_path, m.event)).collect();
    assert_eq!(keys, vec![".Init", "cmdSayHi.Click", "cmdClose.Click"], "{methods:#?}");
    let result = compile_form("HelloWorld", &methods);
    assert!(result.method_diagnostics.iter().all(|(_, d)| !d.is_error()), "{:#?}", result.method_diagnostics);
    let module = result.module.expect("module");
    assert_eq!(module.kind, ModuleKind::Form);
    assert_eq!(module.funcs.len(), 3);
    assert!(module.find_method("", "INIT").is_some());
    assert!(module.find_method("CMDSAYHI", "CLICK").is_some());
    assert!(module.methods.iter().any(|(k, _)| k == ".INIT"));
    assert_eq!(module.funcs[1].display_name, "cmdSayHi.Click");
    assert_eq!(module.funcs[1].locals, vec!["CMSG".to_string()]);
    let bytes = encode(&module);
    assert_eq!(decode(&bytes).unwrap(), module);
}

#[test]
fn programs_and_snippets_round_trip() {
    let main = compile_program(&sample("main.prg"), "main").module.unwrap();
    assert_eq!(decode(&encode(&main)).unwrap(), main);
    assert_eq!(main.funcs[0].name, "MAIN");
    let snip = compile_snippet("DO FORM HelloWorld\nx = 1", "menu").module.unwrap();
    assert_eq!(snip.kind, ModuleKind::Snippet);
    assert_eq!(decode(&encode(&snip)).unwrap(), snip);
}
