//! Parses the real sample sources shipped in `resources/samples`.

use std::fs;
use std::path::PathBuf;

use foxvm::parser::{parse_method, parse_program};

fn sample(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/samples").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Decodes a JSON string literal starting at `start` (the opening quote); returns the value
/// and the index just past the closing quote.
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
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
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

/// Every string value that appears as `"key": "value"` for the given key, in document order.
fn values_of(json: &str, key: &str) -> Vec<String> {
    let chars: Vec<char> = json.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            let (s, next) = json_string(&chars, i);
            i = next;
            if s == key {
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                if chars.get(i) == Some(&':') {
                    i += 1;
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if chars.get(i) == Some(&'"') {
                        let (v, next) = json_string(&chars, i);
                        out.push(v);
                        i = next;
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

/// All method bodies: the string values inside every `"methods": { ... }` object.
fn method_bodies(json: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = json.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '"' {
            i += 1;
            continue;
        }
        let (s, next) = json_string(&chars, i);
        i = next;
        if s != "methods" {
            continue;
        }
        while i < chars.len() && chars[i] != '{' {
            i += 1;
        }
        i += 1;
        loop {
            while i < chars.len() && chars[i] != '"' && chars[i] != '}' {
                i += 1;
            }
            if chars[i] == '}' {
                break;
            }
            let (name, next) = json_string(&chars, i);
            i = next;
            while chars[i] != '"' {
                i += 1;
            }
            let (body, next) = json_string(&chars, i);
            i = next;
            out.push((name, body));
        }
    }
    out
}

#[test]
fn main_prg_parses_cleanly() {
    let out = parse_program(&sample("main.prg"));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(out.program.body.stmts.len(), 2);
}

#[test]
fn hello_world_form_methods_parse_cleanly() {
    let bodies = method_bodies(&sample("HelloWorld.fxf"));
    assert_eq!(bodies.len(), 3, "expected Init and two Click methods: {bodies:?}");
    for (name, body) in bodies {
        let out = parse_method(&body);
        assert!(out.diagnostics.is_empty(), "method {name}: {:#?}", out.diagnostics);
        assert!(!out.program.body.stmts.is_empty(), "method {name} produced no statements");
    }
}

#[test]
fn main_menu_commands_parse_cleanly() {
    let texts = values_of(&sample("Main.fxm"), "text");
    assert_eq!(texts.len(), 3, "{texts:?}");
    for text in texts {
        let out = parse_program(&text);
        assert!(out.diagnostics.is_empty(), "{text}: {:#?}", out.diagnostics);
        assert_eq!(out.program.body.stmts.len(), 1, "{text}");
    }
}

#[test]
fn wait_window_parses() {
    let out = parse_program("WAIT WINDOW \"ok\"");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert!(matches!(out.program.body.stmts[0].kind, foxvm::ast::StmtKind::WaitWindow { text: Some(_), .. }));
}
