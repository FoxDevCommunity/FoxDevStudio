//! `foxvm run <file.prg>` executes a program and prints its `?` output; `foxvm check <file>`
//! prints diagnostics; `foxvm disasm <file.prg>` prints the bytecode.
//!
//! The host is a `MockHost` (no forms): MESSAGEBOX answers 1, INPUTBOX answers "", WAIT WINDOW
//! continues at once, and `DO other` loads `other.prg` from the program's directory.
//!
//! **Its filesystem is in memory, rooted at `C:\WORK\`.** That is what a golden program wants -
//! the same files every time, on any machine - and it is why this is not a general-purpose
//! runner: `SET DEFAULT` to a real folder does nothing, `FILE()` answers no for a file that is
//! on disk, and a table that is not seeded into the mock host cannot be opened. Anything that
//! has to touch real files, real sockets or a real form belongs in the vitest harness, which
//! runs the same VM against the real host.

mod audit;

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use foxvm::compiler::{compile_program, disassemble};
use foxvm::diagnostics::Diagnostic;
use foxvm::host::{Host, HostRequest};
use foxvm::mock_host::MockHost;
use foxvm::value::Value;
use foxvm::vm::{Step, Vm};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, file] if cmd == "run" => run(Path::new(file)),
        [cmd, file] if cmd == "check" => check(Path::new(file)),
        [cmd, file] if cmd == "disasm" => disasm(Path::new(file)),
        [cmd, dir] if cmd == "audit" => audit::audit(Path::new(dir)),
        _ => {
            eprintln!(
                "usage: foxvm run <file.prg> | foxvm check <file.prg> | foxvm disasm <file.prg> | foxvm audit <dir>"
            );
            ExitCode::from(2)
        }
    }
}

fn read(path: &Path) -> Result<String, ExitCode> {
    fs::read_to_string(path).map_err(|e| {
        eprintln!("cannot read {}: {e}", path.display());
        ExitCode::from(2)
    })
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "main".into())
}

fn print_diagnostics(path: &Path, diags: &[Diagnostic]) {
    for d in diags {
        let sev = match d.severity {
            foxvm::diagnostics::Severity::Error => "error",
            foxvm::diagnostics::Severity::Warning => "warning",
        };
        println!(
            "{}:{}:{}: {sev}: {}",
            path.display(),
            d.line,
            d.col,
            d.message
        );
    }
}

fn check(path: &Path) -> ExitCode {
    let src = match read(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let result = compile_program(&src, &stem(path));
    print_diagnostics(path, &result.diagnostics);
    if result.module.is_some() {
        println!(
            "{}: ok ({} warning(s))",
            path.display(),
            result.diagnostics.len()
        );
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn disasm(path: &Path) -> ExitCode {
    let src = match read(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let result = compile_program(&src, &stem(path));
    print_diagnostics(path, &result.diagnostics);
    match result.module {
        Some(m) => {
            print!("{}", disassemble(&m));
            ExitCode::SUCCESS
        }
        None => ExitCode::from(1),
    }
}

/// Prints `?` output as it happens and loads sibling programs on demand.
struct CliHost {
    inner: MockHost,
    dir: PathBuf,
}

impl Host for CliHost {
    fn get_prop(
        &mut self,
        obj: foxvm::value::Handle,
        name: &str,
    ) -> Result<Value, foxvm::error::RtError> {
        self.inner.get_prop(obj, name)
    }
    fn get_member(
        &mut self,
        obj: foxvm::value::Handle,
        name: &str,
    ) -> Result<foxvm::host::Member, foxvm::error::RtError> {
        self.inner.get_member(obj, name)
    }
    fn object_class(&mut self, obj: foxvm::value::Handle) -> Option<String> {
        self.inner.object_class(obj)
    }
    fn output(&mut self, text: &str, newline: bool) {
        if newline {
            println!();
        }
        print!("{text}");
    }
    fn now(&mut self) -> (i32, f64) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        ((secs / 86400.0).floor() as i32, secs.rem_euclid(86400.0))
    }
    fn random(&mut self) -> f64 {
        self.inner.random()
    }
    fn seed_random(&mut self, seed: f64) {
        self.inner.seed_random(seed);
    }
    fn resolve_program(&mut self, name: &str) -> Option<u32> {
        self.inner.resolve_program(name)
    }
}

fn run(path: &Path) -> ExitCode {
    let src = match read(path) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let name = stem(path);
    let result = compile_program(&src, &name);
    print_diagnostics(path, &result.diagnostics);
    let Some(module) = result.module else {
        return ExitCode::from(1);
    };
    let mut vm = Vm::new();
    let mut host = CliHost {
        inner: MockHost::new(),
        dir: path.parent().map(Path::to_path_buf).unwrap_or_default(),
    };
    let id = vm.load_module(module);
    host.inner.programs.insert(name.to_ascii_uppercase(), id);
    let fiber = vm.start(id, 0, None, Vec::new());
    let mut pending: VecDeque<Value> = VecDeque::new();
    let code = loop {
        match vm.step(&mut host, fiber) {
            Step::Done { .. } => break ExitCode::SUCCESS,
            Step::Error(e) => {
                println!();
                eprintln!(
                    "Error {} in {} line {}: {}",
                    e.code, e.program, e.line, e.message
                );
                break ExitCode::from(1);
            }
            Step::Suspend(req) => {
                let answer = match &req {
                    HostRequest::MessageBox { text, title, .. } => {
                        println!();
                        println!(
                            "[MessageBox{}] {text}",
                            if title.is_empty() {
                                String::new()
                            } else {
                                format!(" {title}")
                            }
                        );
                        Value::number(1.0)
                    }
                    HostRequest::InputBox { prompt, .. } => {
                        println!();
                        println!("[InputBox] {prompt}");
                        Value::str("")
                    }
                    HostRequest::WaitWindow { text, .. } => {
                        println!();
                        println!("[Wait] {text}");
                        Value::Null
                    }
                    HostRequest::LoadProgram { name } => {
                        let file = host.dir.join(format!("{}.prg", name.to_ascii_lowercase()));
                        match fs::read_to_string(&file)
                            .ok()
                            .and_then(|s| compile_program(&s, &name.to_ascii_lowercase()).module)
                        {
                            Some(m) => {
                                let id = vm.load_module(m);
                                host.inner.programs.insert(name.to_ascii_uppercase(), id);
                                Value::number(id as f64)
                            }
                            None => {
                                vm.resume_error(
                                    fiber,
                                    foxvm::error::RtError::file_not_found(
                                        &file.display().to_string(),
                                    ),
                                );
                                continue;
                            }
                        }
                    }
                    HostRequest::Quit | HostRequest::Cancel => Value::Null,
                    other => pending
                        .pop_front()
                        .unwrap_or_else(|| host.inner.default_answer(other)),
                };
                vm.resume(fiber, answer);
            }
        }
    };
    println!();
    code
}
