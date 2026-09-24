//! Environment, dialog, path and type built-ins, plus the shape of the registry itself.

mod ctx;

use ctx::{TestCtx, call, call_on, d, err, flag, n, num, num_on, request, s, text, text_on, value, value_on};
use foxvm::builtins::{BuiltinResult, arity_error, array, data, datetime, event, lookup, numeric, object, menu, registry, report, screen, xml as xmlfns, string, system, ui, lowlevel};
use foxvm::error::RtError;
use foxvm::host::HostRequest;
use foxvm::value::{DateFormat, Value};

#[test]
fn registry_is_sorted_and_unique() {
    let reg = registry();
    assert!(reg.len() > 100, "expected a full library, got {}", reg.len());
    for pair in reg.windows(2) {
        assert!(pair[0].name < pair[1].name, "{} and {} are out of order or duplicated", pair[0].name, pair[1].name);
    }
    for spec in reg {
        assert_eq!(spec.name, spec.name.to_ascii_uppercase(), "names are stored upper-case");
        assert!(spec.max_args == u8::MAX || spec.min_args <= spec.max_args, "{} has an impossible arity", spec.name);
    }
    // Nothing was silently dropped by the dedup in registry().
    let module_total = string::specs().len()
        + numeric::specs().len()
        + datetime::specs().len()
        + system::specs().len()
        + array::specs().len()
        + object::specs().len()
        + ui::specs().len()
        + event::specs().len()
        + data::specs().len()
        + lowlevel::specs().len()
        + menu::specs().len()
        + screen::specs().len()
        + report::specs().len()
        + xmlfns::specs().len();
    assert_eq!(module_total, reg.len(), "two modules registered the same name");
    // Each module's own table is kept sorted too, so a new entry lands in the obvious place.
    for specs in [
        string::specs(),
        numeric::specs(),
        datetime::specs(),
        system::specs(),
        array::specs(),
        object::specs(),
        ui::specs(),
        event::specs(),
        data::specs(),
        menu::specs(),
        screen::specs(),
        report::specs(),
        xmlfns::specs(),
        lowlevel::specs(),
    ] {
        for pair in specs.windows(2) {
            assert!(pair[0].name <= pair[1].name, "{} and {} are out of order", pair[0].name, pair[1].name);
        }
    }
}

#[test]
fn arity_is_checked_against_the_spec() {
    let (_, len) = lookup("LEN").unwrap();
    assert!(arity_error(len, 1).is_none());
    assert!(arity_error(len, 0).unwrap().contains("at least"));
    assert!(arity_error(len, 2).unwrap().contains("at most"));
    let (_, inlist) = lookup("INLIST").unwrap();
    assert!(arity_error(inlist, 9).is_none(), "INLIST is variadic");
    assert!(arity_error(inlist, 1).is_some());
    assert!(lookup("IIF").is_none(), "IIF is inlined by the compiler, not a built-in");
    assert!(lookup("len").is_none(), "lookup takes an upper-case name");
}

#[test]
fn messagebox_yields_a_host_request() {
    match request("MESSAGEBOX", vec![s("Saved"), n(36.0), s("Confirm"), n(2000.0)]) {
        HostRequest::MessageBox { text, flags, title, timeout } => {
            assert_eq!((text.as_str(), flags, title.as_str(), timeout), ("Saved", 36, "Confirm", Some(2000)));
        }
        other => panic!("unexpected request {other:?}"),
    }
    // Defaults, and a non-character message formatted with display().
    match request("MESSAGEBOX", vec![n(42.0)]) {
        HostRequest::MessageBox { text, flags, title, timeout } => {
            assert_eq!((text.as_str(), flags, title.as_str(), timeout), ("42", 0, "FoxDev Studio", None));
        }
        other => panic!("unexpected request {other:?}"),
    }
}

#[test]
fn inputbox_yields_a_host_request() {
    match request("INPUTBOX", vec![s("Name?"), s("Ask"), s("Ada"), n(5000.0), s("none")]) {
        HostRequest::InputBox { prompt, title, default, timeout, timeout_value } => {
            assert_eq!(prompt, "Name?");
            assert_eq!(title, "Ask");
            assert_eq!(default, "Ada");
            assert_eq!(timeout, Some(5000));
            assert_eq!(timeout_value, "none");
        }
        other => panic!("unexpected request {other:?}"),
    }
}

#[test]
fn file_requests() {
    assert_eq!(request("FILE", vec![s("a.txt")]), HostRequest::FileExists { path: "a.txt".into(), search: Vec::new() });
    assert_eq!(request("FILETOSTR", vec![s("a.txt")]), HostRequest::FileRead { path: "a.txt".into(), search: Vec::new() });
    assert_eq!(
        request("STRTOFILE", vec![s("body"), s("a.txt"), Value::Logical(true)]),
        HostRequest::FileWrite { path: "a.txt".into(), text: "body".into(), append: true }
    );
    assert_eq!(
        request("STRTOFILE", vec![s("body"), s("a.txt")]),
        HostRequest::FileWrite { path: "a.txt".into(), text: "body".into(), append: false }
    );
    assert_eq!(
        request("GETFILE", vec![s("PRG"), s("Pick one")]),
        HostRequest::GetFile { extensions: "PRG".into(), title: "Pick one".into() }
    );
    assert_eq!(
        request("PUTFILE", vec![s("Save"), s("out.txt"), s("TXT")]),
        HostRequest::PutFile { prompt: "Save".into(), default_name: "out.txt".into(), extension: "TXT".into() }
    );
    assert_eq!(
        request("GETDIR", vec![]),
        HostRequest::GetFile { extensions: String::new(), title: "Select Directory".into() }
    );
}

#[test]
fn environment_functions() {
    assert_eq!(text("VERSION", vec![]), format!("FoxDev Studio Runtime {}", foxvm::VERSION));
    assert_eq!(text("VERSION", vec![n(4.0)]), foxvm::VERSION);
    // measured: the edition and the version number are numbers, a program compares both, and
    // the edition is the development one unless the host says this is a built application
    assert_eq!(num("VERSION", vec![n(2.0)]), 2.0);
    assert_eq!(text("VERSION", vec![n(3.0)]), "00");
    assert_eq!(num("VERSION", vec![n(5.0)]), 900.0);
    assert_eq!(err("VERSION", vec![n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("VERSION", vec![n(6.0)]).code, RtError::FUNCTION_ARG_INVALID);
    // a FoxPro program that guards on the Windows version reads OS(3) and OS(4)
    assert_eq!(text("OS", vec![]), "Windows 10.00");
    // OS(2) is DBCS support, not the version - measured against the product empty on the code
    // page this runtime works in
    assert_eq!(text("OS", vec![n(2.0)]), "");
    assert_eq!(text("OS", vec![n(3.0)]), "10");
    assert_eq!(text("OS", vec![n(4.0)]), "0");
    assert_eq!(text("OS", vec![n(5.0)]), "26200");
    // 6 through 11 measure as fixed values on any Windows from 8 up, because GetVersionEx lies
    // about the version to a caller without a manifest saying otherwise
    assert_eq!(text("OS", vec![n(6.0)]), "2");
    assert_eq!(text("OS", vec![n(7.0)]), "");
    assert_eq!(text("OS", vec![n(8.0)]), "0");
    assert_eq!(text("OS", vec![n(9.0)]), "0");
    assert_eq!(text("OS", vec![n(10.0)]), "256");
    assert_eq!(text("OS", vec![n(11.0)]), "1");
    assert_eq!(num("PCOUNT", vec![]), 2.0);
    assert_eq!(num("PARAMETERS", vec![]), 2.0);
    assert_eq!(text("PROGRAM", vec![]), "TESTPRG");
    assert_eq!(num("LINENO", vec![]), 7.0);
}

#[test]
fn set_reports_the_current_settings() {
    assert_eq!(text("SET", vec![s("EXACT")]), "OFF");
    assert_eq!(text("SET", vec![s("TALK")]), "ON");
    assert_eq!(text("SET", vec![s("SAFETY")]), "ON");
    assert_eq!(text("SET", vec![s("CENTURY")]), "OFF");
    assert_eq!(text("SET", vec![s("ESCAPE")]), "ON");
    assert_eq!(num("SET", vec![s("DECIMALS")]), 2.0);
    assert_eq!(text("SET", vec![s("DATE")]), "AMERICAN");
    assert_eq!(text("SET", vec![s("NOT A SETTING")]), "");
    let mut ctx = TestCtx::with_settings(|s| {
        s.exact = true;
        s.century = true;
        s.decimals = 4;
        s.date_format = DateFormat::German;
    });
    assert_eq!(text_on(&mut ctx, "SET", vec![s("exact")]), "ON");
    assert_eq!(text_on(&mut ctx, "SET", vec![s("CENTURY")]), "ON");
    assert_eq!(num_on(&mut ctx, "SET", vec![s("DECIMALS")]), 4.0);
    assert_eq!(text_on(&mut ctx, "SET", vec![s("DATE")]), "GERMAN");
}

#[test]
fn type_and_evaluate() {
    assert_eq!(text("TYPE", vec![s("nAge")]), "N");
    assert_eq!(text("TYPE", vec![s("cName")]), "C");
    assert_eq!(text("TYPE", vec![s("lOk")]), "L");
    assert_eq!(text("TYPE", vec![s("oNull")]), "X");
    assert_eq!(text("TYPE", vec![s("nowhere")]), "U");
    assert_eq!(value("EVALUATE", vec![s("1+1")]), Value::number(2.0));
    assert_eq!(err("EVALUATE", vec![s("nowhere")]).code, RtError::VARIABLE_NOT_FOUND);
}

#[test]
fn vartype_letters() {
    assert_eq!(text("VARTYPE", vec![s("x")]), "C");
    assert_eq!(text("VARTYPE", vec![n(1.0)]), "N");
    assert_eq!(text("VARTYPE", vec![Value::Logical(false)]), "L");
    assert_eq!(text("VARTYPE", vec![d(2026, 9, 7)]), "D");
    assert_eq!(text("VARTYPE", vec![Value::DateTime(Some(0.0))]), "T");
    assert_eq!(text("VARTYPE", vec![Value::Null]), "X");
    // a bare NULL memory variable is not tied to any column, and Visual FoxPro's own answer for
    // "the type under it" is what a NULL is internally: a Logical with a null bit set
    assert_eq!(text("VARTYPE", vec![Value::Null, Value::Logical(true)]), "L");
}

#[test]
fn emptiness_helpers() {
    assert!(flag("EMPTY", vec![s("   ")]));
    assert!(flag("EMPTY", vec![n(0.0)]));
    assert!(flag("EMPTY", vec![Value::Date(None)]));
    assert!(!flag("EMPTY", vec![n(1.0)]));
    assert!(flag("ISNULL", vec![Value::Null]));
    assert!(!flag("ISNULL", vec![s("")]));
    assert_eq!(value("NVL", vec![Value::Null, s("fallback")]), Value::str("fallback"));
    assert_eq!(value("NVL", vec![s("kept"), s("fallback")]), Value::str("kept"));
    assert_eq!(value("EVL", vec![s(""), s("fallback")]), Value::str("fallback"));
    assert_eq!(value("EVL", vec![n(0.0), n(7.0)]), Value::number(7.0));
    assert_eq!(value("EVL", vec![s("kept"), s("fallback")]), Value::str("kept"));
    assert!(flag("ISBLANK", vec![s("  ")]));
    assert!(flag("ISBLANK", vec![Value::Null]));
    assert!(!flag("ISBLANK", vec![n(0.0)]), "a zero is not blank in VFP");
    assert!(!flag("ISBLANK", vec![Value::Logical(false)]));
}

#[test]
fn inlist_and_between() {
    assert!(flag("INLIST", vec![s("b"), s("a"), s("b"), s("c")]));
    assert!(!flag("INLIST", vec![s("z"), s("a"), s("b")]));
    assert!(flag("INLIST", vec![n(2.0), n(1.0), n(2.0)]));
    assert!(value("INLIST", vec![Value::Null, n(1.0)]).is_null());
    assert!(flag("BETWEEN", vec![n(5.0), n(1.0), n(10.0)]));
    assert!(!flag("BETWEEN", vec![n(0.0), n(1.0), n(10.0)]));
    assert!(flag("BETWEEN", vec![n(1.0), n(1.0), n(10.0)]), "the bounds are inclusive");
    assert!(flag("BETWEEN", vec![d(2026, 9, 7), d(2026, 1, 1), d(2026, 12, 31)]));
    assert!(value("BETWEEN", vec![Value::Null, n(1.0), n(2.0)]).is_null());
}

#[test]
fn character_class_tests() {
    assert!(flag("ISDIGIT", vec![s("7x")]));
    assert!(!flag("ISDIGIT", vec![s("x7")]));
    assert!(!flag("ISDIGIT", vec![s("")]));
    assert!(flag("ISALPHA", vec![s("abc")]));
    assert!(!flag("ISALPHA", vec![s("1bc")]));
    assert!(flag("ISLOWER", vec![s("abc")]));
    assert!(!flag("ISLOWER", vec![s("Abc")]));
    assert!(flag("ISUPPER", vec![s("Abc")]));
    assert!(!flag("ISUPPER", vec![s("abc")]));
}

#[test]
fn path_helpers() {
    let p = s("c:\\dev\\app\\main.prg");
    assert_eq!(text("JUSTFNAME", vec![p.clone()]), "main.prg");
    assert_eq!(text("JUSTSTEM", vec![p.clone()]), "main");
    assert_eq!(text("JUSTEXT", vec![p.clone()]), "prg");
    assert_eq!(text("JUSTPATH", vec![p.clone()]), "c:\\dev\\app");
    assert_eq!(text("JUSTPATH", vec![s("c:\\main.prg")]), "c:\\");
    assert_eq!(text("JUSTPATH", vec![s("main.prg")]), "");
    assert_eq!(text("JUSTFNAME", vec![s("/usr/local/app.txt")]), "app.txt");
    assert_eq!(text("JUSTEXT", vec![s("noextension")]), "");
    assert_eq!(text("JUSTSTEM", vec![s("archive.tar.gz")]), "archive.tar");
    assert_eq!(text("ADDBS", vec![s("c:\\dev")]), "c:\\dev\\");
    assert_eq!(text("ADDBS", vec![s("c:\\dev\\")]), "c:\\dev\\");
    assert_eq!(text("ADDBS", vec![s("c:/dev/")]), "c:/dev/");
    assert_eq!(text("ADDBS", vec![s("")]), "");
    assert_eq!(text("FORCEEXT", vec![s("main.prg"), s("fxp")]), "main.fxp");
    assert_eq!(text("FORCEEXT", vec![p.clone(), s(".txt")]), "c:\\dev\\app\\main.txt");
    assert_eq!(text("FORCEEXT", vec![s("noextension"), s("dbf")]), "noextension.dbf");
    assert_eq!(text("FORCEPATH", vec![s("main.prg"), s("c:\\tmp")]), "c:\\tmp\\main.prg");
    assert_eq!(text("FORCEPATH", vec![p, s("d:\\out\\")]), "d:\\out\\main.prg");
    assert_eq!(text("FORCEPATH", vec![s("main.prg"), s("")]), "main.prg");
}

#[test]
fn sys_returns_unique_and_empty_answers() {
    let a = text("SYS", vec![n(2015.0)]);
    let b = text("SYS", vec![n(2015.0)]);
    assert_ne!(a, b, "SYS(2015) must be unique per call");
    assert!(a.starts_with('_') && a.len() == 10, "unexpected procedure name {a}");
    assert!(a[1..].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    let f = text("SYS", vec![n(3.0)]);
    assert_eq!(f.len(), 8);
    assert_ne!(f, text("SYS", vec![n(3.0)]));
    assert_eq!(text("SYS", vec![n(16.0)]), "TESTPRG");
    assert_eq!(text("SYS", vec![n(5.0)]), "");
    assert_eq!(text("SYS", vec![n(2003.0)]), "");
    assert_eq!(text("SYS", vec![n(1037.0)]), "");
    assert_eq!(text("SYS", vec![n(9999.0)]), "", "an unknown SYS() must not fail");
}

#[test]
fn error_and_message_read_the_last_error() {
    let mut ctx = TestCtx::default();
    assert_eq!(num("ERROR", vec![]), 0.0);
    assert_eq!(text("MESSAGE", vec![]), "");
    ctx.last_error = Some(RtError::division_by_zero());
    assert_eq!(value_on(&mut ctx, "ERROR", vec![]), Value::number(f64::from(RtError::DIVISION_BY_ZERO)),);
    assert_eq!(text_on(&mut ctx, "MESSAGE", vec![]), "Cannot divide by 0.");
    assert_eq!(text_on(&mut ctx, "MESSAGE", vec![n(1.0)]), "");
}

#[test]
fn the_pass_through_functions_ask_the_host_for_a_connection() {
    // SQLCONNECT and its family reach a data source through the host, which is what a
    // suspend is; nothing here is answered without one
    for (name, args) in [("SQLCONNECT", vec![s("books")]), ("SQLSTRINGCONNECT", vec![s("DSN=books")])] {
        assert!(matches!(request(name, args), HostRequest::Sql { what: 0, .. }), "{name}()");
    }
    assert!(matches!(request("SQLDISCONNECT", vec![n(1.0)]), HostRequest::Sql { what: 1, handle: 1, .. }));
    assert!(matches!(request("SQLEXEC", vec![n(1.0), s("SELECT 1")]), HostRequest::Sql { what: 2, .. }));
}

#[test]
fn execscript_hands_the_vm_the_text_to_run() {
    // the function cannot run FoxPro itself: it says what to run and with what, and the VM
    // compiles it and calls it in a frame of its own
    match call("EXECSCRIPT", vec![s("RETURN 1"), n(7.0)]) {
        Ok(BuiltinResult::RunScript { source, args }) => {
            assert_eq!(source, "RETURN 1");
            assert_eq!(args, vec![n(7.0)]);
        }
        _ => panic!("EXECSCRIPT() did not ask for the script to be run"),
    }
    // a number is not a program
    assert_eq!(err("EXECSCRIPT", vec![n(5.0)]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn evaluate_reaches_the_context() {
    let mut ctx = TestCtx::default();
    ctx.evals.insert("custom".to_string(), Value::str("hi"));
    assert_eq!(value_on(&mut ctx, "EVALUATE", vec![s("custom")]), Value::str("hi"));
    assert!(matches!(call_on(&mut ctx, "TYPE", vec![s("custom")]), Ok(BuiltinResult::Value(Value::Str(_)))));
    assert!(matches!(call("EMPTY", vec![Value::Null]), Ok(BuiltinResult::Value(Value::Logical(true)))));
}

#[test]
fn home_asks_the_host_for_the_runtime_directory() {
    assert_eq!(request("HOME", vec![]), HostRequest::HomeDir { which: 0 }, "no argument means the project directory");
    assert_eq!(request("HOME", vec![n(0.0)]), HostRequest::HomeDir { which: 0 });
    assert_eq!(request("HOME", vec![n(1.0)]), HostRequest::HomeDir { which: 1 });
    assert_eq!(request("HOME", vec![n(2.0)]), HostRequest::HomeDir { which: 2 });
    assert_eq!(request("HOME", vec![n(-3.0)]), HostRequest::HomeDir { which: 0 }, "a negative variant clamps to 0");
}

#[test]
fn xmladapter_functions_report_the_missing_milestone() {
    for (name, args) in [
        ("LOADXML", vec![s("<x/>")]),
        ("TOXML", vec![]),
        ("TOCURSOR", vec![]),
        ("ADDTABLESCHEMA", vec![s("t")]),
        ("APPLYDIFFGRAM", vec![]),
    ] {
        let e = err(name, args);
        assert_eq!(e.code, RtError::FEATURE_NOT_AVAILABLE, "{name}()");
        assert_eq!(
            e.message,
            format!(
                "Feature is not available: {name}(): XMLAdapter needs the data engine, which arrives in a later milestone"
            )
        );
    }
}

/// Every recognised-but-unimplemented built-in has to say what is missing and when it is
/// expected, so the user reads a plan instead of "procedure not found". This list is the whole
/// set of `not_available!` bodies in the crate; add to it when one is added.
#[test]
fn every_unavailable_builtin_names_the_milestone() {
    let cases: Vec<(&str, Vec<Value>)> = vec![
        // system.rs - XMLAdapter
        ("LOADXML", vec![s("<x/>")]),
        ("TOXML", vec![]),
        ("TOCURSOR", vec![]),
        ("ADDTABLESCHEMA", vec![s("t")]),
        ("APPLYDIFFGRAM", vec![]),

        // object.rs
        ("COMCLASSINFO", vec![Value::Object(foxvm::value::Handle(1))]),
        ("GETINTERFACE", vec![Value::Object(foxvm::value::Handle(1)), s("IDispatch")]),
        ("EVENTHANDLER", vec![Value::Object(foxvm::value::Handle(1)), Value::Object(foxvm::value::Handle(2))]),
        // system.rs - what needs a window or a conversation this runtime has not
        ("DDEINITIATE", vec![s("Excel"), s("System")]),
        ("DDEREQUEST", vec![n(1.0), s("item")]),
        // ui.rs
        ("PARSFONT", vec![]),
    ];
    for (name, args) in cases {
        let e = err(name, args);
        assert_eq!(e.code, RtError::FEATURE_NOT_AVAILABLE, "{name}()");
        assert!(
            e.message.starts_with(&format!("Feature is not available: {name}():")),
            "{name}() must name itself: {}",
            e.message
        );
        // Either it is coming (name the milestone), it is never coming (say so plainly), or
        // it would take something this runtime is not (say what).
        assert!(
            e.message.contains("milestone")
                || e.message.contains("no such Visual FoxPro function")
                || e.message.contains("this runtime"),
            "{name}() must say when it arrives, that it never will, or what it would take: {}",
            e.message
        );
        assert!(e.message.len() > 40 + name.len(), "{name}() must explain what is missing: {}", e.message);
    }
}
