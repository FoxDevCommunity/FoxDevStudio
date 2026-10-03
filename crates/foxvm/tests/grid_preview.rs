use foxvm::mock_host::{MockHost, run_program};
use foxvm::data::AreaRef;

fn run(source: &str) -> foxvm::vm::Vm {
    run_program(source, &mut MockHost::new()).unwrap().0
}

#[test]
fn preview_preserves_alias_pointer_and_unicode() {
    let vm = run("CREATE CURSOR choices (title C(30))\nINSERT INTO choices VALUES ('Ąžuolas')\nINSERT INTO choices VALUES ('Birch')\nGO TOP\nCREATE CURSOR other (id I)");
    let before = vm.data().current_area();
    let result = vm.grid_preview("CHOICES");
    assert!(result["error"].is_null(), "{result}");
    assert_eq!(result["columns"][0], "TITLE");
    assert!(result["rows"][0][0].as_str().unwrap().starts_with("Ąžuolas"));
    assert_eq!(result["rows"].as_array().unwrap().len(), 2);
    assert_eq!(vm.data().current_area(), before);
    assert_eq!(vm.data().find(&AreaRef::Alias("choices".into())).unwrap().recno(), 1);
}

#[test]
fn closed_and_filtered_aliases_report_limits() {
    let vm = run("CREATE CURSOR choices (id I)\nINSERT INTO choices VALUES (1)\nSET FILTER TO id=1");
    assert!(vm.grid_preview("choices")["error"].is_string());
    assert!(vm.grid_preview("missing")["error"].is_string());
}

#[test]
fn preview_is_bounded_and_respects_deleted_setting() {
    let vm = run("CREATE CURSOR choices (id I)\nFOR n=1 TO 205\nINSERT INTO choices VALUES (n)\nENDFOR\nGO TOP\nDELETE\nSET DELETED ON");
    let result = vm.grid_preview("choices");
    assert_eq!(result["rows"].as_array().unwrap().len(), 200);
    assert_eq!(result["truncated"], true);
    assert_eq!(result["rows"][0][0].as_str().unwrap().trim(), "2");
}
