use foxvm::mock_host::{MockHost, run_program};

#[test]
fn unqualified_field_wins_over_same_named_local() {
    let source = "CREATE CURSOR choices (item_id I)\nINSERT INTO choices VALUES (42)\nLOCAL m.item_id\nm.item_id = item_id\nCREATE CURSOR picked (item_id I)\nINSERT INTO picked VALUES (m.item_id)\n? item_id";
    let (_, out) = run_program(source, &mut MockHost::new()).unwrap();
    assert_eq!(out.last().unwrap().trim(), "42");
}

fn output(source: &str) -> Vec<String> {
    run_program(source, &mut MockHost::new()).unwrap().1.into_iter().map(|s| s.trim().to_string()).collect()
}

#[test]
fn explicit_memory_reads_and_local_fallback_still_work() {
    assert_eq!(output("LOCAL item_id\nm.item_id=7\n? item_id\nCREATE CURSOR choices (item_id I)\nINSERT INTO choices VALUES (42)\n? item_id\n? m.item_id\nUSE IN choices\n? item_id"), ["7", "42", "7", "7"]);
}

#[test]
fn assignments_do_not_write_the_same_named_field() {
    assert_eq!(output("CREATE CURSOR choices (item_id I)\nINSERT INTO choices VALUES (42)\nLOCAL item_id\nitem_id=9\n? item_id\n? m.item_id"), ["42", "9"]);
}

#[test]
fn parameters_follow_field_precedence_without_leaking_callers_locals() {
    assert_eq!(output("LOCAL item_id\nitem_id=8\nCREATE CURSOR choices (item_id I)\nINSERT INTO choices VALUES (42)\nDO show WITH 12\n? m.item_id\nPROCEDURE show\nLPARAMETERS item_id\n? item_id\n? m.item_id\nENDPROC"), ["42", "12", "8"]);
}

#[test]
fn local_fallback_beats_public_variable() {
    assert_eq!(output("PUBLIC amount\namount=99\nDO show\n? amount\nPROCEDURE show\nLOCAL amount\namount=7\n? amount\nENDPROC"), ["7", "99"]);
}

#[test]
fn dynamic_expression_uses_the_same_field_precedence() {
    assert_eq!(output("LOCAL item_id\nitem_id=7\nCREATE CURSOR choices (item_id I)\nINSERT INTO choices VALUES (42)\n? EVALUATE('item_id')\n? EVALUATE('m.item_id')"), ["42", "7"]);
}
