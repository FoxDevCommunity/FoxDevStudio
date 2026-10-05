//! `DEFINE CLASS ... ENDDEFINE`: parsing, the compiled `ClassProto`, and how a class instance
//! reaches the host.

mod common;

use std::collections::VecDeque;

use common::{dump, errors, messages};
use foxvm::bytecode::{Constant, decode, encode};
use foxvm::compiler::compile_program;
use foxvm::host::{HostRequest, JsonValue};
use foxvm::mock_host::{MockHost, run_program, run_with_requests};
use foxvm::parser::parse_program;
use foxvm::value::{Handle, Value};
use foxvm::vm::Vm;

/// `FormsUI/1_formsui_gdiplusimaging.prg`, the shape ~20 sample programs share.
const SAMPLE: &str = "oForm = CREATEOBJECT(\"form1\")\n\
oForm.Show()\n\
oForm.Image1.RotateFlip = 0\n\
RETURN\n\
\n\
DEFINE CLASS form1 AS form\n\
\tCaption = \"Form1\"\n\
\tName = \"Form1\"\n\
\n\
\tADD OBJECT image1 AS image WITH ;\n\
\t\tLeft = 100, ;\n\
\t\tTop = 75, ;\n\
\t\tName = \"Image1\"\n\
\n\
\tPROCEDURE Init\n\
\t\tTHIS.Caption = \"ready\"\n\
\tENDPROC\n\
ENDDEFINE\n";

fn module(src: &str) -> foxvm::bytecode::Module {
    let r = compile_program(src, "main");
    assert!(!r.diagnostics.iter().any(|d| d.is_error()), "{:#?}", r.diagnostics);
    r.module.expect("module")
}

fn num(c: &Constant) -> f64 {
    match c {
        Constant::Num(n, ..) => *n,
        other => panic!("expected a number, got {other:?}"),
    }
}

fn text(c: &Constant) -> &str {
    match c {
        Constant::Str(s) => s,
        other => panic!("expected a string, got {other:?}"),
    }
}

// ----- parsing -------------------------------------------------------------------------------

#[test]
fn the_sample_program_parses() {
    let out = parse_program(SAMPLE);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(
        dump(&out.program),
        "(= OFORM CREATEOBJECT(\"form1\"))\n\
         (expr OFORM.SHOW())\n\
         (= OFORM.IMAGE1.ROTATEFLIP 0)\n\
         (return)\n\
         (class FORM1 AS FORM)\n\
         \x20 (prop CAPTION \"Form1\")\n\
         \x20 (prop NAME \"Form1\")\n\
         \x20 (object IMAGE1 AS IMAGE)\n\
         \x20   (prop LEFT 100)\n\
         \x20   (prop TOP 75)\n\
         \x20   (prop NAME \"Image1\")\n\
         \x20 (procedure INIT ())\n\
         \x20   (= THIS.CAPTION \"ready\")"
    );
}

#[test]
fn classes_may_follow_procedures_without_endproc() {
    let out = parse_program(
        "DO greet\nRETURN\n\nPROCEDURE greet\n  ? \"hi\"\n\nDEFINE CLASS c AS Custom\n  X = 1\nENDDEFINE\n",
    );
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(out.program.procs.len(), 1);
    assert_eq!(out.program.classes.len(), 1);
}

#[test]
fn several_classes_and_a_procedure_in_one_file() {
    let out = parse_program(
        "RETURN\nDEFINE CLASS a AS Custom\nENDDEFINE\nPROCEDURE helper\n  ? 1\nENDPROC\nDEFINE CLASS b AS a\nENDDEFINE\n",
    );
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let names: Vec<&str> = out.program.classes.iter().map(|c| c.name.text.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
    assert_eq!(out.program.procs.len(), 1);
}

#[test]
fn define_window_is_a_window_rather_than_a_class() {
    // DEFINE only opens a block when it opens a class; DEFINE WINDOW is a statement of its own
    let out = parse_program("DEFINE WINDOW w FROM 1,1 TO 10,10\nx = 1\n");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(dump(&out.program), "(window Define)\n(= X 1)");
    // and one that draws on a screen this runtime does not have still is not a class
    let out = parse_program("DEFINE BOX FROM 1 TO 10 HEIGHT 3\nx = 1\n");
    assert!(errors(&out.diagnostics).is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(dump(&out.program), "(= X 1)");
}

// ----- the compiled proto --------------------------------------------------------------------

#[test]
fn the_sample_compiles_to_one_class_proto() {
    let m = module(SAMPLE);
    assert_eq!(m.classes.len(), 1);
    let c = &m.classes[0];
    assert_eq!((c.name.as_str(), c.parent.as_str()), ("form1", "form"));
    assert_eq!(c.properties.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["Caption", "Name"]);
    assert_eq!(text(&c.properties[0].1), "Form1");
    assert_eq!(text(&c.properties[1].1), "Form1");

    assert_eq!(c.members.len(), 1);
    let m0 = &c.members[0];
    assert_eq!((m0.name.as_str(), m0.class.as_str(), m0.noinit), ("image1", "image", false));
    assert_eq!(m0.properties.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["Left", "Top", "Name"]);
    assert_eq!(num(&m0.properties[0].1), 100.0);
    assert_eq!(num(&m0.properties[1].1), 75.0);
    assert_eq!(text(&m0.properties[2].1), "Image1");

    assert_eq!(c.methods.len(), 1);
    assert_eq!(c.methods[0].0, "Init");
    let func = &m.funcs[c.methods[0].1 as usize];
    assert_eq!(func.name, "FORM1.INIT");
    assert_eq!(func.display_name, "form1.Init");
    assert_eq!(m.find_method("FORM1", "INIT"), Some(c.methods[0].1));
    assert!(m.find_class("FORM1").is_some(), "class lookup is case-insensitive");
}

#[test]
fn a_class_may_inherit_another_class_of_the_same_file() {
    let m = module(
        "RETURN\n\
         DEFINE CLASS base1 AS Custom\n  Caption = \"base\"\n  PROCEDURE Init\n    ? 1\n  ENDPROC\nENDDEFINE\n\
         DEFINE CLASS child1 AS base1\n  Caption = \"child\"\nENDDEFINE\n",
    );
    assert_eq!(m.classes.len(), 2);
    assert_eq!(m.classes[1].name, "child1");
    assert_eq!(m.classes[1].parent, "base1");
    // Inheritance is not flattened here: the child declares only its own property.
    assert_eq!(m.classes[1].properties.len(), 1);
    assert!(m.classes[1].methods.is_empty());
}

#[test]
fn a_method_of_a_member_is_registered_under_the_member_path() {
    let m = module(
        "RETURN\n\
         DEFINE CLASS form1 AS form\n\
         \tADD OBJECT image1 AS image\n\
         \tPROCEDURE image1.Click\n\t\t? \"clicked\"\n\tENDPROC\n\
         ENDDEFINE\n",
    );
    let c = &m.classes[0];
    assert_eq!(c.methods[0].0, "image1.Click");
    let idx = c.methods[0].1;
    assert_eq!(m.funcs[idx as usize].name, "FORM1.IMAGE1.CLICK");
    assert_eq!(m.funcs[idx as usize].display_name, "form1.image1.Click");
    assert_eq!(m.methods.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), vec!["FORM1.IMAGE1.CLICK"]);
    assert_eq!(m.find_method("FORM1.IMAGE1", "CLICK"), Some(idx));
}

#[test]
fn add_object_noinit_is_recorded() {
    let m =
        module("RETURN\nDEFINE CLASS c AS Custom\n  ADD OBJECT tmr AS Timer NOINIT WITH Interval = 500\nENDDEFINE\n");
    let member = &m.classes[0].members[0];
    assert!(member.noinit);
    assert_eq!(member.class, "Timer");
    assert_eq!(num(&member.properties[0].1), 500.0);

    // NOINIT is also accepted after the WITH list, as some samples write it.
    let m =
        module("RETURN\nDEFINE CLASS c AS Custom\n  ADD OBJECT tmr AS Timer WITH Interval = 500 NOINIT\nENDDEFINE\n");
    assert!(m.classes[0].members[0].noinit);
}

#[test]
fn protected_and_hidden_members_behave_like_ordinary_ones() {
    let m = module(
        "RETURN\n\
         DEFINE CLASS c AS Custom\n\
         \tPROTECTED nCount\n\
         \tHIDDEN cName\n\
         \tPROTECTED nTotal = 10\n\
         \tCaption = \"visible\"\n\
         \tADD OBJECT PROTECTED lbl AS Label WITH Caption = \"x\"\n\
         \tPROTECTED PROCEDURE Recalc\n\t\tTHIS.nTotal = 1\n\tENDPROC\n\
         \tHIDDEN FUNCTION Secret\n\t\tRETURN 1\n\tENDFUNC\n\
         ENDDEFINE\n",
    );
    let c = &m.classes[0];
    assert_eq!(
        c.properties.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        vec!["nCount", "cName", "nTotal", "Caption"]
    );
    assert_eq!(c.properties[0].1, Constant::Bool(false), "a bare PROTECTED name starts at .F.");
    assert_eq!(num(&c.properties[2].1), 10.0);
    assert_eq!(c.members[0].name, "lbl");
    assert_eq!(c.methods.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["Recalc", "Secret"]);
}

#[test]
fn property_values_are_constant_folded() {
    let m = module(
        "RETURN\n\
         DEFINE CLASS c AS Custom\n\
         \tBackColor = RGB(255, 128, 0)\n\
         \tLeft = -10\n\
         \tVisible = .T.\n\
         \tTag = .NULL.\n\
         \tWidth = 100 + 20\n\
         \tCaption = \"a\" + \"b\"\n\
         \tBorn = {^2026-09-07}\n\
         ENDDEFINE\n",
    );
    let p = &m.classes[0].properties;
    assert_eq!(num(&p[0].1), 255.0 + 128.0 * 256.0);
    assert_eq!(num(&p[1].1), -10.0);
    assert_eq!(p[2].1, Constant::Bool(true));
    assert_eq!(p[3].1, Constant::Null);
    assert_eq!(num(&p[4].1), 120.0);
    assert_eq!(text(&p[5].1), "ab");
    assert!(matches!(p[6].1, Constant::Date(Some(_))));
}

// ----- diagnostics ---------------------------------------------------------------------------

#[test]
fn a_property_value_that_is_an_expression_is_kept_as_its_source() {
    // Measured: Visual FoxPro works these out when an object is made, so the host is handed the
    // text to evaluate then rather than a value folded now.
    let r = compile_program(
        "RETURN\nDEFINE CLASS c AS Custom\n  Caption = TRANSFORM(DTOS(DATE()), \"@R xxxx-xx-xx\")\n  Left = nStart + 1\n  Top = 5\nENDDEFINE\n",
        "main",
    );
    let m = r.module.expect("compiles");
    assert_eq!(
        m.classes[0].expressions,
        vec![
            ("Caption".to_string(), "TRANSFORM(DTOS(DATE()), \"@R xxxx-xx-xx\")".to_string()),
            ("Left".to_string(), "nStart + 1".to_string())
        ]
    );
    assert_eq!(m.classes[0].properties.len(), 1);

    let r = compile_program(
        "RETURN\nDEFINE CLASS c AS Custom\n  ADD OBJECT img AS Image WITH Picture = GetPic()\nENDDEFINE\n",
        "main",
    );
    let m = r.module.expect("compiles");
    assert_eq!(m.classes[0].members[0].expressions, vec![("Picture".to_string(), "GetPic()".to_string())]);
}

#[test]
fn a_class_inside_a_procedure_is_an_error() {
    let out = parse_program("PROCEDURE foo\n  x = 1\n  DEFINE CLASS c AS Custom\n  ENDDEFINE\nENDPROC\n");
    assert_eq!(errors(&out.diagnostics), vec!["5:1: DEFINE CLASS is not allowed inside a PROCEDURE or FUNCTION"]);

    // Nested inside a block, the class is reported where it stands.
    let out = parse_program("IF .T.\n  DEFINE CLASS c AS Custom\n  ENDDEFINE\nENDIF\n");
    assert_eq!(errors(&out.diagnostics), vec!["2:3: DEFINE CLASS must appear at the top level of a program"]);

    // A method body is not a program either.
    let out = foxvm::parser::parse_method("DEFINE CLASS c AS Custom\nENDDEFINE\n");
    assert_eq!(errors(&out.diagnostics), vec!["1:1: DEFINE CLASS must appear at the top level of a program"]);
}

#[test]
fn a_stray_enddefine_is_an_error() {
    let out = parse_program("x = 1\nENDDEFINE\n");
    assert_eq!(errors(&out.diagnostics), vec!["2:1: ENDDEFINE without matching DEFINE CLASS"]);
}

#[test]
fn a_duplicate_class_name_is_an_error() {
    let r = compile_program("RETURN\nDEFINE CLASS c AS Custom\nENDDEFINE\nDEFINE CLASS C AS Form\nENDDEFINE\n", "main");
    assert!(r.module.is_none());
    assert_eq!(
        r.diagnostics.iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect::<Vec<_>>(),
        vec!["Class 'C' is already defined"]
    );
}

#[test]
fn of_classlib_only_warns_and_access_methods_are_plain_methods() {
    let src = "RETURN\n\
        DEFINE CLASS c AS Custom OF mylib.vcx OLEPUBLIC\n\
        \tCaption = \"x\"\n\
        \tPROCEDURE Caption_ACCESS\n\t\tRETURN THIS.Caption\n\tENDPROC\n\
        \tPROCEDURE Caption_ASSIGN(vNew)\n\t\tRETURN\n\tENDPROC\n\
        ENDDEFINE\n";
    let out = parse_program(src);
    assert!(!foxvm::parser::has_errors(&out.diagnostics), "{:#?}", out.diagnostics);
    assert_eq!(
        messages(&out.diagnostics),
        vec!["OF mylib.vcx is ignored: the class is taken from this program"]
    );
    assert_eq!(out.program.classes[0].class_lib.as_deref(), Some("mylib.vcx"));
    // They are compiled as ordinary methods; a property read or write is what calls them.
    let m = module(src);
    assert_eq!(m.classes[0].methods.len(), 2);
}

// ----- the VM and the host -------------------------------------------------------------------

#[test]
fn createobject_of_a_defined_class_carries_its_definition() {
    let mut host = MockHost::new();
    let (_vm, _out) = run_program(SAMPLE, &mut host).expect("runs");
    let create = host
        .requests
        .iter()
        .find_map(|r| match r {
            HostRequest::CreateObject { class, definition, .. } => Some((class.clone(), definition.clone())),
            _ => None,
        })
        .expect("a CreateObject request");
    assert_eq!(create.0, "form1");
    let def = create.1.expect("definition");
    assert_eq!(def.name, "form1");
    assert_eq!(def.base_class, "form");
    assert_eq!(def.module, 0);
    assert_eq!(
        def.properties.iter().map(|p| (p.name.as_str(), p.value.clone())).collect::<Vec<_>>(),
        vec![("Caption", JsonValue::Str("Form1".into())), ("Name", JsonValue::Str("Form1".into()))]
    );
    assert_eq!(def.members.len(), 1);
    assert_eq!((def.members[0].name.as_str(), def.members[0].class.as_str()), ("image1", "image"));
    assert!(!def.members[0].noinit);
    assert_eq!(def.members[0].properties[0].value, JsonValue::Num(100.0));
    assert_eq!(def.methods.len(), 1);
    assert_eq!(
        (def.methods[0].name.as_str(), def.methods[0].owner.as_str(), def.methods[0].module),
        ("Init", "form1", 0)
    );

    // The mock builds the object the definition describes, so member access works.
    let form = host.find(Handle(1), "").expect("the created form");
    assert_eq!(host.prop(form, "Caption"), Some(Value::str("Form1")));
    let image = host.find(form, "image1").expect("the member");
    assert_eq!(host.prop(image, "Top"), Some(Value::number(75.0)));
    assert_eq!(host.prop(image, "RotateFlip"), Some(Value::number(0.0)));
}

#[test]
fn createobject_of_a_base_class_carries_no_definition() {
    let mut host = MockHost::new();
    run_program("o = CREATEOBJECT(\"Custom\")\n", &mut host).expect("runs");
    match &host.requests[0] {
        HostRequest::CreateObject { class, definition, .. } => {
            assert_eq!(class, "Custom");
            assert!(definition.is_none());
        }
        other => panic!("expected a CreateObject request, got {other:?}"),
    }
}

#[test]
fn find_class_searches_every_loaded_module_last_first() {
    let mut vm = Vm::new();
    let a = vm.load_module(module("RETURN\nDEFINE CLASS c AS Custom\n  Tag = \"first\"\nENDDEFINE\n"));
    assert_eq!(vm.find_class("C").map(|(id, c)| (id, c.parent.clone())), Some((a, "Custom".into())));
    let b = vm.load_module(module("RETURN\nDEFINE CLASS c AS Form\n  Tag = \"second\"\nENDDEFINE\n"));
    assert_eq!(vm.find_class("c").map(|(id, c)| (id, c.parent.clone())), Some((b, "Form".into())));
    assert!(vm.find_class("Nope").is_none());
    assert_eq!(vm.classes(a).len(), 1);
    assert!(vm.classes(99).is_empty());
}

#[test]
fn start_class_method_runs_the_body_and_reports_unknown_methods() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = vm.load_module(module(SAMPLE));
    let obj = host.add_object("form1", "Form1", None);
    host.set_prop(obj, "Caption", Value::str("Form1"));

    let fiber = vm.start_class_method(id, "form1", "", "Init", obj.0, Vec::new()).expect("Init");
    run_with_requests(&mut vm, &mut host, fiber, &mut VecDeque::new()).expect("runs");
    assert_eq!(host.prop(obj, "Caption"), Some(Value::str("ready")));

    assert!(vm.start_class_method(id, "form1", "", "Destroy", obj.0, Vec::new()).is_none());
    assert!(vm.start_class_method(id, "form1", "image1", "Click", obj.0, Vec::new()).is_none());
    assert!(vm.start_class_method(id, "nosuch", "", "Init", obj.0, Vec::new()).is_none());
}

#[test]
fn start_class_method_reaches_a_members_method() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = vm.load_module(module(
        "RETURN\n\
         DEFINE CLASS form1 AS form\n\
         \tADD OBJECT image1 AS image\n\
         \tPROCEDURE image1.Click\n\t\t? \"clicked\"\n\tENDPROC\n\
         ENDDEFINE\n",
    ));
    let obj = host.add_object("image", "Image1", None);
    let fiber = vm.start_class_method(id, "form1", "image1", "Click", obj.0, Vec::new()).expect("Click");
    run_with_requests(&mut vm, &mut host, fiber, &mut VecDeque::new()).expect("runs");
    assert_eq!(host.output, vec!["clicked"]);
}

#[test]
fn a_class_method_takes_parameters_like_any_procedure() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = vm.load_module(module(
        "RETURN\n\
         DEFINE CLASS c AS Custom\n\
         \tPROCEDURE Greet(cName)\n\t\t? \"hi \" + cName\n\tENDPROC\n\
         \tPROCEDURE Twice\n\t\tLPARAMETERS nX\n\t\t? nX * 2\n\tENDPROC\n\
         ENDDEFINE\n",
    ));
    let obj = host.add_object("c", "c", None);
    let fiber = vm.start_class_method(id, "c", "", "Greet", obj.0, vec![Value::str("Ana")]).expect("Greet");
    run_with_requests(&mut vm, &mut host, fiber, &mut VecDeque::new()).expect("runs");
    let fiber = vm.start_class_method(id, "c", "", "Twice", obj.0, vec![Value::number(21.0)]).expect("Twice");
    run_with_requests(&mut vm, &mut host, fiber, &mut VecDeque::new()).expect("runs");
    assert_eq!(host.output, vec!["hi Ana", "          42"]);
}

// ----- THISFORMSET ---------------------------------------------------------------------------

/// A formset holding one form with one button on it, as the host holds them.
fn formset_tree(host: &mut MockHost) -> (Handle, Handle, Handle) {
    let set = host.add_object("Formset", "Formset1", None);
    let form = host.add_object("Form", "frmLeft", Some(set));
    let button = host.add_object("CommandButton", "cmdGo", Some(form));
    (set, form, button)
}

#[test]
fn thisformset_is_the_formset_above_the_object_wherever_it_sits() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = vm.load_module(module(
        "RETURN\n\
         DEFINE CLASS probe AS Custom\n\
         \tPROCEDURE Say\n\t\t? THISFORMSET.Name, THISFORMSET.BaseClass\n\tENDPROC\n\
         ENDDEFINE\n",
    ));
    let (set, form, button) = formset_tree(&mut host);
    // the formset itself, a form in it and a control on that form all reach the same object
    for obj in [set, form, button] {
        let fiber = vm.start_class_method(id, "probe", "", "Say", obj.0, Vec::new()).expect("Say");
        run_with_requests(&mut vm, &mut host, fiber, &mut VecDeque::new()).expect("runs");
    }
    assert_eq!(host.output, vec!["Formset1 Formset"; 3]);
}

#[test]
fn thisformset_outside_a_formset_says_so() {
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let id = vm.load_module(module(
        "RETURN\n\
         DEFINE CLASS probe AS Custom\n\
         \tPROCEDURE Say\n\t\t? THISFORMSET.Name\n\tENDPROC\n\
         ENDDEFINE\n",
    ));
    let form = host.add_hello_world_form();
    let fiber = vm.start_class_method(id, "probe", "", "Say", form.0, Vec::new()).expect("Say");
    let err = run_with_requests(&mut vm, &mut host, fiber, &mut VecDeque::new()).expect_err("no formset");
    assert_eq!((err.code, err.message.as_str()), (1938, "Object is not contained in a FORMSET."));
}

#[test]
fn a_module_with_classes_survives_encode_decode() {
    let m = module(SAMPLE);
    let bytes = encode(&m);
    let back = decode(&bytes).expect("decodes");
    assert_eq!(back, m);
    assert_eq!(back.classes[0].members[0].properties.len(), 3);
    assert_eq!(back.find_method("FORM1", "INIT"), Some(m.classes[0].methods[0].1));
}
