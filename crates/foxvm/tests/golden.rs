//! Golden programs: every `tests/programs/*.prg` runs against a `MockHost` and its `?` output
//! (or its runtime error) must match the `.expected` file next to it.
//!
//! An expected file whose first line is `ERROR <code> line <n> [in <program>]` asserts that
//! error instead of output. Trailing blanks are ignored line by line.

mod dbf_fixture;

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use foxvm::compiler::{MethodSource, compile_form, compile_program, compile_program_with};
use foxvm::error::RtError;
use foxvm::mock_host::{MockHost, run_with_requests};
use foxvm::value::Value;
use foxvm::vm::Vm;

fn programs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/programs")
}

/// Where the stage a golden runs on is kept as files: the second program, the report.
///
/// `scripts/vfp-expected.mjs` copies the same files beside the program it hands to Visual
/// FoxPro, so the product is asked the same question this test asks.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden")
}

/// The header files on the stage, keyed the way `#INCLUDE` names them: the stem, upper-cased.
///
/// Reading files is not the compiler's to do, so whoever asks for the parse hands the text
/// over. Here that is every `.h` beside the other fixtures, which is also what Visual FoxPro
/// finds in the directory the harness runs the program in.
fn headers() -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let Ok(entries) = fs::read_dir(fixtures_dir()) else { return out };
    for path in entries.filter_map(|e| e.ok().map(|e| e.path())) {
        if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("h"))
            && let (Some(stem), Ok(text)) = (path.file_stem(), fs::read_to_string(&path))
        {
            out.insert(stem.to_string_lossy().to_uppercase(), text);
        }
    }
    out
}

/// Compiles `src` and runs its MAIN with the HelloWorld object tree available as `oForm`;
/// `other.prg` is loaded as a second program so `DO other` resolves.
fn run(src: &str, host: &mut MockHost) -> Result<Vec<String>, RtError> {
    let result = compile_program_with(src, "main", &headers());
    let Some(module) = result.module else {
        let d = result.diagnostics.iter().find(|d| d.is_error()).expect("error diagnostic");
        let mut e = RtError::syntax(format!("compile error: {}", d.message));
        e.line = d.line;
        // Visual FoxPro reports a program that fails to compile the same way it reports one
        // that fails at run time - `DO main` raising `ERROR 10 line n in MAIN` - so a golden
        // that never gets past compiling asserts the same `in MAIN` a runtime error would.
        e.program = "MAIN".to_string();
        return Err(e);
    };
    let mut vm = Vm::new();
    let other_src = fs::read_to_string(fixtures_dir().join("other.prg")).expect("tests/fixtures/golden/other.prg");
    let other = compile_program(&other_src, "other").module.expect("other compiles");
    let other_id = vm.load_module(other);
    host.programs.insert("OTHER".into(), other_id);
    // the stage's other programs are not loaded until something asks for one, as a `.prg` in a
    // project is not: SET PROCEDURE and DO load them through the host
    for name in ["fdvproca", "fdvprocb", "fdvrunme"] {
        let src = fs::read_to_string(fixtures_dir().join(format!("{name}.prg"))).expect("a procedure fixture");
        host.program_sources.insert(name.to_ascii_uppercase(), src);
    }
    // a report file to run, so a golden program can print one
    let (frx, frt) = dbf_fixture::sample();
    host.files.insert("PARTS.FRX".into(), frx);
    host.files.insert("PARTS.FRT".into(), frt);
    // a picture on the stage too, so a golden can load one
    let bitmap = fs::read(fixtures_dir().join("fdvfox.bmp")).expect("tests/fixtures/golden/fdvfox.bmp");
    host.files.insert("FDVFOX.BMP".into(), bitmap);
    // and a class library, so a golden can name a file for `SET CLASSLIB` to load. Nothing here
    // reads a `.vcx`; what the mock host answers is whether the file is on hand, which is the
    // half of the behaviour the VM owns.
    let library = fs::read(fixtures_dir().join("fdvclasses.vcx")).expect("tests/fixtures/golden/fdvclasses.vcx");
    host.files.insert("FDVCLASSES.VCX".into(), library);
    host.printers = vec!["FoxDev PDF".into()];
    let form = host.add_hello_world_form();
    vm.set_global("oForm", Value::Object(form));
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());
    let mut answers = VecDeque::new();
    run_with_requests(&mut vm, host, fiber, &mut answers)?;
    Ok(host.output.clone())
}

fn normalize(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(|l| l.trim_end().to_string()).collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

/// The goldens Visual FoxPro cannot be asked about, read from `programs/not-measured.txt`.
///
/// Their `.expected` files were written by hand, so they say what we believe the product does.
/// A failure on one of them is worth a second look at the expectation itself, which is why the
/// message says so.
fn not_measured() -> Vec<(String, String)> {
    let path = programs_dir().join("not-measured.txt");
    let text = fs::read_to_string(&path).unwrap_or_default();
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once(':'))
        .map(|(name, why)| (name.trim().to_string(), why.trim().to_string()))
        .collect()
}

/// Every name in the manifest is a golden that is still here, so the list cannot rot unnoticed.
#[test]
fn not_measured_names_a_real_golden() {
    for (name, why) in not_measured() {
        assert!(programs_dir().join(&name).exists(), "not-measured.txt names {name}, which is gone");
        assert!(!why.is_empty(), "not-measured.txt gives no reason for {name}");
    }
}

/// The report file the mock host serves, kept on disk so the Visual FoxPro harness can stage it.
///
/// `dbf_fixture::sample()` is the source of truth; run with `REGEN_FIXTURES=1` after changing it.
#[test]
fn report_fixture_files_are_current() {
    let (frx, frt) = dbf_fixture::sample();
    for (name, want) in [("PARTS.FRX", frx), ("PARTS.FRT", frt)] {
        let path = fixtures_dir().join(name);
        if std::env::var_os("REGEN_FIXTURES").is_some() {
            fs::create_dir_all(fixtures_dir()).unwrap();
            fs::write(&path, &want).unwrap();
            continue;
        }
        let have = fs::read(&path).unwrap_or_else(|_| panic!("missing {name}; rerun with REGEN_FIXTURES=1"));
        assert!(have == want, "{name} is out of date with dbf_fixture::sample(); rerun with REGEN_FIXTURES=1");
    }
}

#[test]
fn golden_programs() {
    let dir = programs_dir();
    let hand_written: Vec<String> = not_measured().into_iter().map(|(name, _)| name).collect();
    let mut names: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("programs dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "prg"))
        .collect();
    names.sort();
    assert!(names.len() >= 15, "expected the golden programs, found {}", names.len());
    let mut failures = Vec::new();
    for path in names {
        let src = fs::read_to_string(&path).unwrap();
        let expected = fs::read_to_string(path.with_extension("expected"))
            .unwrap_or_else(|_| panic!("missing .expected for {}", path.display()));
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut host = MockHost::new();
        let outcome = run(&src, &mut host);
        let mut want = normalize(&expected);
        let error_line = want.last().and_then(|l| l.strip_prefix("ERROR ")).map(str::to_string);
        if let Some(rest) = error_line {
            want.pop();
            let mut it = rest.split_whitespace();
            let code: u32 = it.next().unwrap().parse().unwrap();
            assert_eq!(it.next(), Some("line"));
            let line: u32 = it.next().unwrap().parse().unwrap();
            let program = if it.next() == Some("in") { it.next().map(str::to_string) } else { None };
            match outcome {
                Err(e) => {
                    if e.code != code || e.line != line || program.as_deref().is_some_and(|p| p != e.program) {
                        failures.push(format!(
                            "{name}: expected error {code} line {line} in {program:?}, got {e:?} after output {:?}",
                            host.output
                        ));
                    } else if normalize(&host.output.join("\n")) != want {
                        failures.push(format!(
                            "{name}: output before the error differs:\n--- expected\n{}\n--- got\n{}",
                            want.join("\n"),
                            host.output.join("\n")
                        ));
                    }
                }
                Ok(out) => failures
                    .push(format!("{name}: expected error {code} line {line}, program succeeded with output {out:?}")),
            }
            continue;
        }
        match outcome {
            Ok(out) => {
                let got = normalize(&out.join("\n"));
                if got != want {
                    failures.push(format!("{name}:\n--- expected\n{}\n--- got\n{}", want.join("\n"), got.join("\n")));
                }
            }
            Err(e) => failures.push(format!("{name}: unexpected error {e:?} after output {:?}", host.output)),
        }
    }
    // a golden the product was never asked about is only as good as the person who wrote it down
    let failures: Vec<String> = failures
        .into_iter()
        .map(|f| match hand_written.iter().any(|n| f.starts_with(&format!("{n}:"))) {
            true => format!("{f}\n(its .expected was written by hand - see programs/not-measured.txt)"),
            false => f,
        })
        .collect();
    assert!(failures.is_empty(), "{} golden failure(s):\n\n{}", failures.len(), failures.join("\n\n"));
}

#[test]
fn form_method_sets_nested_caption() {
    let methods = vec![
        MethodSource {
            object_path: "cmdSayHi".into(),
            event: "Click".into(),
            params: String::new(),
            source: "LOCAL cMsg\ncMsg = \"Hello, \" + THISFORM.txtName.Value\nIF THISFORM.chkLoud.Value\n  cMsg = cMsg + \"!\"\nENDIF\nTHISFORM.pgfMain.Page1.lblGreeting.Caption = cMsg\nRETURN THIS.Caption".into(),
            include: String::new(),
        },
        MethodSource { object_path: String::new(), event: "Init".into(), params: String::new(), source: "  \n".into(), include: String::new() },
        MethodSource { object_path: "txtName".into(), event: "KeyPress".into(), params: "nKeyCode, nShiftAltCtrl".into(), source: "? nKeyCode + nShiftAltCtrl\nNODEFAULT".into(), include: String::new() },
    ];
    let result = compile_form("HelloWorld", &methods);
    assert!(result.method_diagnostics.is_empty(), "{:#?}", result.method_diagnostics);
    let module = result.module.expect("module");
    assert_eq!(module.methods.len(), 2, "blank Init is skipped");
    assert!(module.find_method("CMDSAYHI", "CLICK").is_some());
    assert!(module.find_method("TXTNAME", "KEYPRESS").is_some());
    assert_eq!(module.funcs[0].display_name, "cmdSayHi.Click");

    let mut host = MockHost::new();
    let form = host.add_hello_world_form();
    let txt = host.find(form, "txtName").unwrap();
    host.set_prop(txt, "Value", Value::str("Ada"));
    let chk = host.find(form, "chkLoud").unwrap();
    host.set_prop(chk, "Value", Value::Logical(true));
    let button = host.find(form, "cmdSayHi").unwrap();

    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start_method(id, "cmdSayHi", "Click", button, Vec::new()).expect("method");
    let value = run_with_requests(&mut vm, &mut host, fiber, &mut VecDeque::new()).unwrap();
    assert_eq!(value, Value::str("Say Hi"));
    let label = host.find(form, "pgfMain.Page1.lblGreeting").unwrap();
    assert_eq!(host.prop(label, "Caption"), Some(Value::str("Hello, Ada!")));
    assert_eq!(host.requests.len(), 1);

    // KeyPress: implicit parameters and NODEFAULT.
    let fiber = vm.start_method(id, "txtName", "KeyPress", txt, vec![Value::number(65.0), Value::number(1.0)]).unwrap();
    match vm.step(&mut host, fiber) {
        foxvm::vm::Step::Done { nodefault, .. } => assert!(nodefault),
        other => panic!("{other:?}"),
    }
    // a parameter plus a written 1: eleven wide, as Visual FoxPro prints it
    assert_eq!(host.output.last().unwrap(), "         66");
    assert!(vm.start_method(id, "cmdClose", "Click", button, Vec::new()).is_none());
}

#[test]
fn compile_errors_are_reported() {
    let r = compile_program("EXIT\n", "t");
    assert!(r.module.is_none());
    assert_eq!(r.diagnostics[0].message, "EXIT can only be used inside a loop");
    assert_eq!(r.diagnostics[0].line, 1);
    let r = compile_program("? IIF(1, 2)\n", "t");
    assert!(r.module.is_none());
    assert!(r.diagnostics[0].message.contains("IIF()"));
    let r = compile_program("PROCEDURE a\nPROCEDURE a\n", "t");
    assert!(r.diagnostics[0].message.contains("already defined"));
    let r = compile_program("IF x\n", "t");
    assert!(r.module.is_none(), "parser errors propagate");
    // ON KEY LABEL hangs a command off a key like the rest of the ON family, and says nothing
    let r = compile_program("ON KEY LABEL F1 ? 1\n", "t");
    assert!(r.module.is_some());
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let r = compile_form(
        "f",
        &[MethodSource {
            object_path: "a".into(),
            event: "Click".into(),
            params: String::new(),
            source: "LOOP".into(),
            include: String::new(),
        }],
    );
    assert!(r.module.is_none());
    assert_eq!(r.method_diagnostics[0].0, "a.Click");
}

#[test]
fn evaluate_in_frame() {
    let src = "LOCAL lv\nlv = 7\nPRIVATE pv\npv = \"s\"\nWAIT WINDOW \"pause\"\n? lv";
    let module = compile_program(src, "main").module.unwrap();
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());
    let mut host = MockHost::new();
    assert!(matches!(vm.step(&mut host, fiber), foxvm::vm::Step::Suspend(foxvm::host::HostRequest::WaitWindow { .. })));
    assert_eq!(vm.evaluate(&mut host, fiber, "lv * 2").unwrap(), Value::number(14.0));
    assert_eq!(vm.evaluate(&mut host, fiber, "pv + \"x\"").unwrap(), Value::str("sx"));
    assert_eq!(vm.evaluate(&mut host, fiber, "nope").unwrap_err().code, RtError::VARIABLE_NOT_FOUND);
    assert_eq!(vm.evaluate(&mut host, fiber, "1 +").unwrap_err().code, RtError::SYNTAX_ERROR);
    vm.resume(fiber, Value::Null);
    assert!(matches!(vm.step(&mut host, fiber), foxvm::vm::Step::Done { .. }));
    assert_eq!(host.output, vec!["         7"]);
}
