//! The character screen: what `@` draws on it, the windows a program opens over it, and what
//! a `READ` does with the fields `@ ... GET` put up.

use foxvm::host::HostRequest;
use foxvm::mock_host::{MockHost, run_program};
use foxvm::screen::ScreenDoc;
use foxvm::value::Value;

fn run_with(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn run(src: &str) -> (MockHost, Vec<String>) {
    let mut host = MockHost::new();
    let out = run_with(&mut host, src);
    (host, out)
}

fn screen(host: &MockHost) -> ScreenDoc {
    host.screen.clone().expect("the screen was drawn")
}

/// The rows of a surface that have anything on them, trimmed at the right.
fn written(lines: &[String]) -> Vec<String> {
    lines.iter().map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect()
}

#[test]
fn say_puts_text_where_the_line_said() {
    let (host, _) = run("@ 1, 4 SAY \"Name\"\n@ 2, 4 SAY 1234.5 PICTURE \"9,999.99\"\n");
    assert_eq!(written(&screen(&host).lines), vec!["    Name", "    1,234.50"]);
}

#[test]
fn row_and_col_follow_the_last_thing_drawn() {
    let (_, out) = run("@ 3, 10 SAY \"abc\"\n? ROW(), COL()\n? SROWS(), SCOLS()\n");
    assert_eq!(out[0], "     3.000     13.000");
    assert_eq!(out[1], "    25.000     80.000");
}

#[test]
fn a_box_and_a_clear_draw_and_undraw_a_region() {
    let (host, _) = run("@ 0, 0, 2, 4 BOX \"-|++++\"\n");
    assert_eq!(written(&screen(&host).lines), vec!["+---+", "|   |", "+---+"]);
    let (host, _) = run("@ 0, 0 SAY \"hello\"\n@ 0, 1 CLEAR TO 0, 3\n");
    assert_eq!(written(&screen(&host).lines), vec!["h   o"]);
}

#[test]
fn fill_and_scroll_move_what_is_there() {
    let (host, _) = run("@ 0, 0 SAY \"ab\"\n@ 1, 0 SAY \"cd\"\n@ 0, 0 SCROLL TO 1, 1 UP 1\n");
    assert_eq!(written(&screen(&host).lines), vec!["cd"]);
    let (host, _) = run("@ 0, 0 FILL TO 0, 3\n@ 1, 0 SAY \"x\"\n");
    assert_eq!(written(&screen(&host).lines), vec!["x"]);
}

#[test]
fn a_window_takes_the_output_until_it_is_deactivated() {
    let (host, out) = run(
        "DEFINE WINDOW wnote FROM 2, 5 TO 8, 40 TITLE \"Notes\"
ACTIVATE WINDOW wnote
@ 0, 0 SAY \"in the window\"
? WOUTPUT(), WONTOP()
? WEXIST(\"wnote\"), WVISIBLE(\"wnote\")
? WROWS(\"wnote\"), WCOLS(\"wnote\"), WLROW(\"wnote\"), WLCOL(\"wnote\")
? WTITLE(\"wnote\")
DEACTIVATE WINDOW wnote
? WOUTPUT(), WVISIBLE(\"wnote\"), WLAST()
",
    );
    assert_eq!(out[0], "WNOTE WNOTE");
    assert_eq!(out[1], ".T. .T.");
    assert_eq!(out[2], "         7         36          2          5");
    assert_eq!(out[3], "Notes");
    assert_eq!(out[4], " .F. WNOTE");
    // what was drawn went into the window, not onto the screen behind it
    let doc = screen(&host);
    assert_eq!(written(&doc.lines), Vec::<String>::new());
}

#[test]
fn the_window_the_host_draws_carries_its_place_and_its_text() {
    let (host, _) = run(
        "DEFINE WINDOW wnote FROM 2, 5 TO 4, 20 TITLE \"Notes\"
ACTIVATE WINDOW wnote
@ 0, 1 SAY \"hello\"
",
    );
    let doc = screen(&host);
    assert_eq!(doc.windows.len(), 1);
    let window = &doc.windows[0];
    assert_eq!(window.name, "wnote");
    assert_eq!(window.title, "Notes");
    assert_eq!((window.row, window.col, window.height, window.width), (2.0, 5.0, 3.0, 16.0));
    assert_eq!(written(&window.lines), vec![" hello"]);
}

#[test]
fn moving_sizing_and_zooming_change_where_a_window_is() {
    let (_, out) = run(
        "DEFINE WINDOW w FROM 1, 1 TO 5, 20
ACTIVATE WINDOW w
MOVE WINDOW w TO 3, 7
? WLROW(\"w\"), WLCOL(\"w\")
SIZE WINDOW w TO 10, 30
? WROWS(\"w\"), WCOLS(\"w\")
ZOOM WINDOW w MAX
? WMAXIMUM(\"w\"), WMINIMUM(\"w\")
ZOOM WINDOW w MIN
? WMAXIMUM(\"w\"), WMINIMUM(\"w\")
",
    );
    assert_eq!(out[0], "         3          7");
    assert_eq!(out[1], "        10         30");
    assert_eq!(out[2], ".T. .F.");
    assert_eq!(out[3], ".F. .T.");
}

#[test]
fn hiding_a_window_leaves_it_defined_and_releasing_does_not() {
    let (_, out) = run(
        "DEFINE WINDOW w FROM 1, 1 TO 5, 20
ACTIVATE WINDOW w
HIDE WINDOW w
? WEXIST(\"w\"), WVISIBLE(\"w\")
SHOW WINDOW w
? WVISIBLE(\"w\")
RELEASE WINDOW w
? WEXIST(\"w\")
",
    );
    assert_eq!(out[0], ".T. .F.");
    assert_eq!(out[1], ".T.");
    assert_eq!(out[2], ".F.");
}

#[test]
fn the_screen_can_be_kept_and_put_back() {
    let (host, _) = run(
        "@ 0, 0 SAY \"first\"
SAVE SCREEN
CLEAR
@ 0, 0 SAY \"second\"
RESTORE SCREEN
",
    );
    assert_eq!(written(&screen(&host).lines), vec!["first"]);
}

#[test]
fn a_read_puts_what_was_typed_back_where_it_came_from() {
    let mut host = MockHost::new();
    host.reads.push_back(vec![Value::str("Jorge"), Value::number(42.0)]);
    let out = run_with(
        &mut host,
        "cName = SPACE(10)
nAge = 0
@ 1, 1 SAY \"Name\" GET cName
@ 2, 1 SAY \"Age\" GET nAge PICTURE \"999\"
? OBJNUM(\"cName\"), OBJVAR(2)
? RDLEVEL()
READ
? ALLTRIM(cName), nAge
? VARREAD(), RDLEVEL()
",
    );
    assert_eq!(out[0], "         1 NAGE");
    assert_eq!(out[1], "         0");
    assert_eq!(out[2], "Jorge         42");
    assert_eq!(out[3], "NAGE          0");
}

#[test]
fn a_read_with_nothing_to_read_asks_the_host_nothing() {
    let mut host = MockHost::new();
    let out = run_with(&mut host, "READ\n? RDLEVEL()\n");
    assert_eq!(out[0], "         0");
    assert!(host.reads.is_empty());
}

#[test]
fn menu_to_stores_the_choice_the_user_made() {
    let mut host = MockHost::new();
    host.choices.push_back(2.0);
    let out = run_with(
        &mut host,
        "nPick = 0
@ 1, 1 PROMPT \"Add\"
@ 2, 1 PROMPT \"Edit\"
@ 3, 1 PROMPT \"Quit\"
MENU TO nPick
? nPick
",
    );
    assert_eq!(out[0], "         2");
}

#[test]
fn the_mouse_functions_answer_what_the_host_reports() {
    let mut host = MockHost::new();
    host.pointer = (4.0, 9.0, true);
    let out = run_with(
        &mut host,
        "DEFINE WINDOW w FROM 3, 5 TO 10, 40
ACTIVATE WINDOW w
? MROW(\"w\"), MCOL(\"w\")
? MDOWN(), MWINDOW()
",
    );
    assert_eq!(out[0], "         1          4");
    assert_eq!(out[1], ".T. W");
}

#[test]
fn the_scroll_command_moves_a_region_on_both_axes() {
    // rows go up and columns go right when the count is positive, so the second line takes the
    // place of the first and moves two columns across
    let (host, _) = run("@ 1, 0 SAY \"one\"\n@ 2, 0 SAY \"two\"\nSCROLL 1, 0, 3, 9, 1, 2\n");
    assert_eq!(written(&screen(&host).lines), vec!["  two"]);
}

#[test]
fn a_scroll_of_no_rows_empties_the_region() {
    let (host, _) = run("@ 1, 0 SAY \"one\"\n@ 5, 0 SAY \"five\"\nSCROLL 1, 0, 3, 9, 0\n");
    assert_eq!(written(&screen(&host).lines), vec!["five"]);
}

#[test]
fn the_mouse_command_asks_the_host_to_press_where_it_said() {
    let mut host = MockHost::new();
    run_with(&mut host, "MOUSE CLICK AT 3, 5\n");
    let asked = host.requests.iter().any(|r| matches!(r, HostRequest::MousePress { clicks: 1, at: Some((3.0, 5.0)), .. }));
    assert!(asked, "the host was asked to press: {:?}", host.requests);
}
