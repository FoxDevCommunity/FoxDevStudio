//! `foxvm audit <dir>`: compile everything under a folder and report what does not compile.
//!
//! Fixing an unsupported command one error message at a time is slow and tells you nothing about
//! how much is left. This walks a tree of Visual FoxPro sources - programs, and the methods inside
//! forms, class libraries and menus - compiles every one, and groups what came back. The output is
//! a list of what is missing, commonest first, with a place to look at each.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use foxvm::compiler::compile_program;
use foxvm::dbf::{DbfTable, read_table};
use foxvm::diagnostics::{Diagnostic, Severity};
use foxvm::ast::{Expr, ExprKind, Program, Stmt, StmtKind};
use foxvm::parser::parse_method;

#[derive(Default)]
struct Report {
    files: usize,
    units: usize,
    failed: usize,
    /// message -> (how many, where the first few were)
    errors: BTreeMap<String, (usize, Vec<String>)>,
    warnings: BTreeMap<String, usize>,
    /// Names called as functions that are neither built in nor defined nearby, with an example.
    /// These compile, and fail at run time with "Procedure ... is not found", so a report that
    /// only counts compile errors misses them entirely.
    unknown: BTreeMap<String, (usize, Vec<String>)>,
}

pub fn audit(root: &Path) -> ExitCode {
    let mut report = Report::default();
    walk(root, &mut report);

    println!("{} file(s), {} compiled unit(s), {} failed\n", report.files, report.units, report.failed);
    if !report.errors.is_empty() {
        let mut by_count: Vec<_> = report.errors.iter().collect();
        by_count.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        println!("errors, commonest first:");
        for (message, (count, where_)) in by_count.iter().take(40) {
            println!("  {count:5}  {message}");
            for w in where_ {
                println!("         {w}");
            }
        }
    }
    if !report.unknown.is_empty() {
        let mut by_count: Vec<_> = report.unknown.iter().collect();
        by_count.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        println!("\nnames called that are not built in, commonest first:");
        for (name, (count, where_)) in by_count.iter().take(40) {
            println!("  {count:5}  {name}()");
            for w in where_ {
                println!("         {w}");
            }
        }
    }
    if !report.warnings.is_empty() {
        let mut by_count: Vec<_> = report.warnings.iter().collect();
        by_count.sort_by(|a, b| b.1.cmp(a.1));
        println!("\nwarnings, commonest first:");
        for (message, count) in by_count.iter().take(20) {
            println!("  {count:5}  {message}");
        }
    }
    if report.failed == 0 { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

fn walk(dir: &Path, report: &mut Report) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(&path, report);
            continue;
        }
        let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        match ext.as_str() {
            "prg" | "qpr" | "mpr" => program(&path, report),
            "scx" | "vcx" => designed(&path, report),
            _ => {}
        }
    }
}

fn program(path: &Path, report: &mut Report) {
    let Ok(src) = read_source(path) else { return };
    report.files += 1;
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let result = compile_program(&src, &name);
    record(report, path, "", &src, &result.diagnostics, result.module.is_some());
    let parsed = foxvm::parser::parse_program(&src);
    let known = declared(&parsed.program);
    let mut calls = Vec::new();
    for stmt in every_statement(&parsed.program) {
        collect_calls(stmt, &mut calls);
    }
    note_calls(report, path, "", &src, &known, &calls);
}

/// The methods inside a form or class library: one memo per object, holding `PROCEDURE`s.
fn designed(path: &Path, report: &mut Report) {
    let Ok(dbf) = fs::read(path) else { return };
    let memo = memo_beside(path);
    let Ok(table) = read_table(&dbf, memo.as_deref()) else { return };
    report.files += 1;

    for row in table.records.iter().filter(|r| !r.deleted) {
        let object = row.get(&table, "OBJNAME").map(|v| v.as_text()).unwrap_or("").to_string();
        let text = row.get(&table, "METHODS").map(|v| v.as_text()).unwrap_or("");
        for (event, body) in split_methods(text) {
            let out = parse_method(&body);
            let ok = !out.diagnostics.iter().any(|d| d.severity == Severity::Error);
            let unit = format!("{object}.{event}");
            record(report, path, &unit, &body, &out.diagnostics, ok);
            let known = declared(&out.program);
            let mut calls = Vec::new();
            for stmt in every_statement(&out.program) {
                collect_calls(stmt, &mut calls);
            }
            note_calls(report, path, &unit, &body, &known, &calls);
        }
        // a designed object also carries expression properties, but those are the importer's
        let _ = &table;
    }
}

/// Splits a methods memo into `(event, body)`. Anything before the first PROCEDURE is skipped.
fn split_methods(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("PROCEDURE ") || upper.starts_with("FUNCTION ") {
            if let Some((name, body)) = current.take() {
                out.push((name, body.join("\n")));
            }
            let name = trimmed.split_whitespace().nth(1).unwrap_or("?").to_string();
            current = Some((name, Vec::new()));
        } else if upper == "ENDPROC" || upper == "ENDFUNC" {
            if let Some((name, body)) = current.take() {
                out.push((name, body.join("\n")));
            }
        } else if let Some((_, body)) = current.as_mut() {
            body.push(line);
        }
    }
    if let Some((name, body)) = current {
        out.push((name, body.join("\n")));
    }
    out
}

fn memo_beside(path: &Path) -> Option<Vec<u8>> {
    let want = match path.extension()?.to_string_lossy().to_ascii_lowercase().as_str() {
        "scx" => "sct",
        "vcx" => "vct",
        "mnx" => "mnt",
        "pjx" => "pjt",
        _ => return None,
    };
    // the samples are not consistent about case, so the directory is searched
    let dir = path.parent()?;
    let stem = path.file_stem()?.to_string_lossy().to_ascii_lowercase();
    for entry in fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if let Some((s, e)) = name.rsplit_once('.')
            && s == stem
            && e == want
        {
            return fs::read(entry.path()).ok();
        }
    }
    None
}

/// Source files in the samples are in a DOS code page, so bytes are read as latin-1 rather than
/// refused: a stray accented character in a comment is not a reason to skip a file.
fn read_source(path: &Path) -> std::io::Result<String> {
    Ok(fs::read(path)?.iter().map(|&b| b as char).collect())
}

fn record(report: &mut Report, path: &Path, unit: &str, src: &str, diags: &[Diagnostic], ok: bool) {
    report.units += 1;
    if !ok {
        report.failed += 1;
    }
    for d in diags {
        // the line itself is what says what the parser met; a file and a number is a scavenger hunt
        let text = src.lines().nth(d.line.saturating_sub(1) as usize).unwrap_or("").trim();
        let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let where_ = if unit.is_empty() {
            format!("{file}:{}  {text}", d.line)
        } else {
            format!("{file} {unit}:{}  {text}", d.line)
        };
        match d.severity {
            Severity::Error => {
                let entry = report.errors.entry(generalise(&d.message)).or_insert((0, Vec::new()));
                entry.0 += 1;
                if entry.1.len() < 3 {
                    entry.1.push(where_);
                }
            }
            Severity::Warning => *report.warnings.entry(generalise(&d.message)).or_default() += 1,
        }
    }
}

/// Groups messages that differ only in the name they mention, so the report counts kinds of
/// problem rather than instances of one.
fn generalise(message: &str) -> String {
    // the name a message mentions is what makes two instances the same problem, except when the
    // message is about a token the parser did not expect - there the token is the whole point
    if message.contains("found") || message.starts_with("Unrecognized command verb") {
        return message.to_string();
    }
    match message.split_once('\'') {
        Some((before, rest)) => match rest.split_once('\'') {
            Some((_, after)) => format!("{before}'...'{after}"),
            None => message.to_string(),
        },
        None => message.to_string(),
    }
}

/// Unused, but keeps the table type in scope for the doc comment above.
#[allow(dead_code)]
fn table_kind(t: &DbfTable) -> usize {
    t.fields.len()
}

/// The names a unit defines for itself: its procedures, functions and class methods.
fn declared(p: &Program) -> std::collections::HashSet<String> {
    let mut out: std::collections::HashSet<String> = p.procs.iter().map(|d| d.name.upper.clone()).collect();
    for class in &p.classes {
        out.extend(class.procs.iter().map(|d| d.name.upper.clone()));
    }
    out
}

/// Every statement of a unit, including the ones inside blocks and procedures.
fn every_statement(p: &Program) -> Vec<&Stmt> {
    let mut out: Vec<&Stmt> = p.body.stmts.iter().collect();
    for d in &p.procs {
        out.extend(d.body.stmts.iter());
    }
    for c in &p.classes {
        for d in &c.procs {
            out.extend(d.body.stmts.iter());
        }
    }
    // the blocks inside those statements are walked as the calls are collected
    out
}

fn note_calls(
    report: &mut Report,
    path: &Path,
    unit: &str,
    src: &str,
    known: &std::collections::HashSet<String>,
    calls: &[(String, u32)],
) {
    for (name, line) in calls {
        // IIF and DODEFAULT are compiled in place rather than looked up, so they are not missing
        if known.contains(name) || foxvm::builtins::lookup_abbreviated(name).is_some() || matches!(name.as_str(), "DODEFAULT" | "IIF") {
            continue;
        }
        let text = src.lines().nth(line.saturating_sub(1) as usize).unwrap_or("").trim();
        let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let where_ = if unit.is_empty() {
            format!("{file}:{line}  {text}")
        } else {
            format!("{file} {unit}:{line}  {text}")
        };
        let entry = report.unknown.entry(name.clone()).or_insert((0, Vec::new()));
        entry.0 += 1;
        if entry.1.len() < 2 {
            entry.1.push(where_);
        }
    }
}

/// Every function call in a statement and the blocks it contains, as a name and a line.
fn collect_calls<'a>(s: &'a Stmt, out: &mut Vec<(String, u32)>) {
    let line = s.line;
    let expr_calls = |e: &Expr, out: &mut Vec<(String, u32)>| calls_in(e, line, out);
    match &s.kind {
        StmtKind::Assign { target, value } => {
            expr_calls(target, out);
            expr_calls(value, out);
        }
        StmtKind::ExprStmt(e) | StmtKind::Return(Some(e)) => expr_calls(e, out),
        StmtKind::Print { items, .. } => items.iter().for_each(|e| expr_calls(e, out)),
        StmtKind::If { cond, then, else_ } => {
            expr_calls(cond, out);
            then.stmts.iter().for_each(|s| collect_calls(s, out));
            if let Some(b) = else_ {
                b.stmts.iter().for_each(|s| collect_calls(s, out));
            }
        }
        StmtKind::DoWhile { cond, body } => {
            expr_calls(cond, out);
            body.stmts.iter().for_each(|s| collect_calls(s, out));
        }
        StmtKind::DoCase { cases, otherwise } => {
            for (c, b) in cases {
                expr_calls(c, out);
                b.stmts.iter().for_each(|s| collect_calls(s, out));
            }
            if let Some(b) = otherwise {
                b.stmts.iter().for_each(|s| collect_calls(s, out));
            }
        }
        StmtKind::For { from, to, body, .. } => {
            expr_calls(from, out);
            expr_calls(to, out);
            body.stmts.iter().for_each(|s| collect_calls(s, out));
        }
        StmtKind::ForEach { collection, body, .. } => {
            expr_calls(collection, out);
            body.stmts.iter().for_each(|s| collect_calls(s, out));
        }
        StmtKind::Scan { body, cond, .. } => {
            if let Some(c) = cond {
                expr_calls(c, out);
            }
            body.stmts.iter().for_each(|s| collect_calls(s, out));
        }
        StmtKind::With { obj, body } => {
            expr_calls(obj, out);
            body.stmts.iter().for_each(|s| collect_calls(s, out));
        }
        StmtKind::Try { body, catches, finally, .. } => {
            body.stmts.iter().for_each(|s| collect_calls(s, out));
            for c in catches {
                c.body.stmts.iter().for_each(|s| collect_calls(s, out));
            }
            if let Some(b) = finally {
                b.stmts.iter().for_each(|s| collect_calls(s, out));
            }
        }
        _ => {}
    }
}

fn calls_in(e: &Expr, line: u32, out: &mut Vec<(String, u32)>) {
    match &e.kind {
        ExprKind::Call { name, args } => {
            out.push((name.upper.clone(), line));
            args.iter().for_each(|a| calls_in(&a.expr, line, out));
        }
        ExprKind::MethodCall { obj, args, .. } => {
            calls_in(obj, line, out);
            args.iter().for_each(|a| calls_in(&a.expr, line, out));
        }
        ExprKind::Unary { expr, .. } => calls_in(expr, line, out),
        ExprKind::Binary { left, right, .. } => {
            calls_in(left, line, out);
            calls_in(right, line, out);
        }
        ExprKind::Member { obj, .. } => calls_in(obj, line, out),
        ExprKind::Index { base, args } => {
            calls_in(base, line, out);
            args.iter().for_each(|a| calls_in(a, line, out));
        }
        _ => {}
    }
}
