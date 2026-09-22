//! The menus a program builds as it runs: what `DEFINE` puts together, what `ACTIVATE` hands
//! the host, and what the menu functions say about it afterwards.

use foxvm::host::{HostRequest, MenuDoc};
use foxvm::mock_host::{MockHost, run_program};

fn run(src: &str) -> (MockHost, Vec<String>) {
    let mut host = MockHost::new();
    match run_program(src, &mut host) {
        Ok((_, output)) => (host, output),
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

/// The menu the last `ACTIVATE MENU` handed the host.
fn installed(host: &MockHost) -> MenuDoc {
    host.requests
        .iter()
        .rev()
        .find_map(|r| match r {
            HostRequest::SetMenu { menu } => menu.clone(),
            _ => None,
        })
        .expect("a menu was handed to the host")
}

const SAMPLE: &str = r#"
DEFINE MENU mainbar
DEFINE PAD padfile OF mainbar PROMPT "\<File" KEY ALT+F MESSAGE "File things"
DEFINE PAD padhelp OF mainbar PROMPT "\<Help"
ON PAD padfile OF mainbar ACTIVATE POPUP popfile
ON SELECTION PAD padhelp OF mainbar ? "help chosen"
DEFINE POPUP popfile
DEFINE BAR 1 OF popfile PROMPT "\<New" KEY CTRL+N
DEFINE BAR 2 OF popfile PROMPT "\-"
DEFINE BAR 3 OF popfile PROMPT "E\<xit" SKIP FOR .F.
ON SELECTION BAR 1 OF popfile ? "new chosen"
ON SELECTION BAR 3 OF popfile CLEAR EVENTS
ACTIVATE MENU mainbar NOWAIT
"#;

#[test]
fn defining_a_menu_builds_the_document_the_host_puts_up() {
    let (host, _) = run(SAMPLE);
    let doc = installed(&host);
    assert_eq!(doc.name, "mainbar");
    assert_eq!(doc.location, "Replace");
    assert_eq!(doc.items.len(), 2);

    let file = &doc.items[0];
    assert_eq!(file.prompt, "\\<File");
    assert_eq!(file.result.kind, "submenu");
    assert_eq!(file.message.as_deref(), Some("File things"));
    assert_eq!(file.hotkey.as_ref().map(|k| k.key.as_str()), Some("F"));
    assert_eq!(file.hotkey.as_ref().and_then(|k| k.alt), Some(true));

    let bars = file.children.as_ref().expect("the popup is under the pad");
    assert_eq!(bars.len(), 3);
    assert_eq!(bars[0].prompt, "\\<New");
    assert_eq!(bars[0].result.text.as_deref(), Some("? \"new chosen\""));
    assert_eq!(bars[1].prompt, "\\-");
    assert_eq!(bars[2].skip_for.as_deref(), Some(".F."));
    assert_eq!(bars[2].result.text.as_deref(), Some("CLEAR EVENTS"));

    // a pad with no popup runs the command ON SELECTION gave it
    let help = &doc.items[1];
    assert_eq!(help.result.kind, "command");
    assert_eq!(help.result.text.as_deref(), Some("? \"help chosen\""));
}

#[test]
fn the_menu_functions_report_what_was_defined() {
    let (_, out) = run(&format!(
        "{SAMPLE}
? MENU()
? CNTPAD('mainbar'), CNTBAR('popfile'), BARCOUNT('popfile')
? GETPAD('mainbar', 1), GETPAD('mainbar', 2)
? PRMPAD('mainbar', 'padfile')
? GETBAR('popfile', 3), PRMBAR('popfile', 3)
? BARPROMPT(1, 'popfile')
? MRKPAD('mainbar', 'padfile'), SKPPAD('mainbar', 'padfile')
? SKPBAR('popfile', 3)
"
    ));
    assert_eq!(out[0], "MAINBAR");
    assert_eq!(out[1], "         2          3          3");
    assert_eq!(out[2], "PADFILE PADHELP");
    assert_eq!(out[3], "File");
    assert_eq!(out[4], "         3 Exit");
    assert_eq!(out[5], "New");
    assert_eq!(out[6], ".F. .F.");
    assert_eq!(out[7], ".F.");
}

#[test]
fn a_mark_and_a_skip_condition_reach_the_choice_they_name() {
    let (host, out) = run(&format!(
        "{SAMPLE}
lready = .F.
SET MARK OF PAD padfile OF mainbar TO .T.
SET SKIP OF BAR 1 OF popfile TO NOT lready
? MRKPAD('mainbar', 'padfile')
? SKPBAR('popfile', 1)
ACTIVATE MENU mainbar
"
    ));
    assert_eq!(out[0], ".T.");
    assert_eq!(out[1], ".T.");
    let doc = installed(&host);
    let bars = doc.items[0].children.as_ref().expect("bars");
    assert_eq!(bars[0].skip_for.as_deref(), Some("NOT lready"));
}

#[test]
fn push_keeps_the_menus_and_pop_puts_them_back() {
    let (_, out) = run(&format!(
        "{SAMPLE}
PUSH MENU _MSYSMENU
DEFINE MENU mainbar
? CNTPAD('mainbar')
POP MENU _MSYSMENU
? CNTPAD('mainbar')
"
    ));
    assert_eq!(out[0], "         0");
    assert_eq!(out[1], "         2");
}

#[test]
fn deactivating_takes_the_menu_down() {
    let (host, out) = run(&format!(
        "{SAMPLE}
DEACTIVATE MENU mainbar
? MENU()
"
    ));
    assert_eq!(out[0], "");
    assert!(
        matches!(host.requests.last(), Some(HostRequest::SetMenu { menu: None })),
        "the last thing the host was told is that the menu came down"
    );
}

#[test]
fn releasing_a_menu_that_is_up_takes_it_down_too() {
    let (host, _) = run(&format!(
        "{SAMPLE}
RELEASE MENU mainbar
"
    ));
    assert!(matches!(host.requests.last(), Some(HostRequest::SetMenu { menu: None })));
}

#[test]
fn a_popup_goes_up_as_one_pad_with_its_bars_under_it() {
    let (host, _) = run(&format!(
        "{SAMPLE}
ACTIVATE POPUP popfile
"
    ));
    let doc = installed(&host);
    assert_eq!(doc.items.len(), 1);
    assert_eq!(doc.items[0].prompt, "popfile");
    assert_eq!(doc.items[0].children.as_ref().map(Vec::len), Some(3));
}
