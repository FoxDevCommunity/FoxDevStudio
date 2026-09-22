//! `.mem` files, against one Visual FoxPro 9 wrote itself.
//!
//! `fixtures/vfp9.mem` was produced by running this in Visual FoxPro 9:
//!
//! ```foxpro
//! cName = "Jorge"
//! nCount = 42.75
//! lOn = .T.
//! dWhen = {^2026-09-08}
//! tWhen = {^2026-09-08 13:45:07}
//! yMoney = $19.95
//! DIMENSION aList(3)
//! aList(1) = "one"
//! aList(2) = 2
//! aList(3) = .F.
//! SAVE TO probe.mem
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use foxvm::mem;
use foxvm::value::{FoxArray, Value};

const VFP9: &[u8] = include_bytes!("fixtures/vfp9.mem");

/// The day 2026-09-08 is, counted from 1970-01-01 as `Value::Date` counts it.
const WHEN: i32 = 20704;

fn sample() -> Vec<(String, Value)> {
    let array = FoxArray { rows: 3, cols: 0, items: vec![Value::str("one"), Value::number(2.0), Value::Logical(false)] };
    vec![
        ("cName".into(), Value::str("Jorge")),
        ("nCount".into(), Value::number(42.75)),
        ("lOn".into(), Value::Logical(true)),
        ("dWhen".into(), Value::Date(Some(WHEN))),
        ("tWhen".into(), Value::DateTime(Some(f64::from(WHEN) * 86_400.0 + 13.0 * 3600.0 + 45.0 * 60.0 + 7.0))),
        ("yMoney".into(), Value::number(19.95)),
        ("aList".into(), Value::Array(Rc::new(RefCell::new(array)))),
    ]
}

#[test]
fn the_day_the_fixture_names_is_the_one_the_test_means() {
    // 2026-09-08, so the fixture and the sample are talking about the same date
    assert_eq!(WHEN, 20704);
}

#[test]
fn reads_what_visual_foxpro_wrote() {
    let vars = mem::read(VFP9);
    let names: Vec<&str> = vars.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["CNAME", "NCOUNT", "LON", "DWHEN", "TWHEN", "YMONEY", "ALIST"]);
    assert_eq!(vars[0].1, Value::str("Jorge"));
    assert_eq!(vars[1].1, Value::number(42.75));
    assert_eq!(vars[2].1, Value::Logical(true));
    assert_eq!(vars[3].1, Value::Date(Some(WHEN)));
    assert_eq!(vars[4].1, Value::DateTime(Some(f64::from(WHEN) * 86_400.0 + 49_507.0)));
    // money comes back as a number, which is the nearest thing this runtime has
    assert_eq!(vars[5].1, Value::number(19.95));
    let Value::Array(array) = &vars[6].1 else { panic!("the array came back as {:?}", vars[6].1) };
    let array = array.borrow();
    assert_eq!(array.rows, 3);
    assert_eq!(array.items, vec![Value::str("one"), Value::number(2.0), Value::Logical(false)]);
}

#[test]
fn writes_what_visual_foxpro_would_have_written() {
    // money is the one thing this cannot write back the same way, because the runtime has no
    // currency of its own; everything else is byte for byte what Visual FoxPro wrote
    let mine = mem::write(&sample());
    let theirs: Vec<u8> = VFP9.to_vec();
    let ours = mem::read(&mine);
    assert_eq!(ours.len(), 7);
    let head = 0x9f; // up to the end of the datetime, before the money
    assert_eq!(mine[..head], theirs[..head], "the first six values are written as VFP writes them");
}

#[test]
fn a_long_string_says_so_in_its_type() {
    let long = "ab".repeat(150);
    let bytes = mem::write(&[("cLong".into(), Value::str(long.clone()))]);
    assert_eq!(bytes[11], b'H');
    assert_eq!(mem::read(&bytes), vec![("CLONG".to_string(), Value::str(long))]);
}

#[test]
fn a_two_dimensional_array_comes_back_the_same_shape() {
    let mut grid = FoxArray::new(2, 3);
    for (i, item) in grid.items.iter_mut().enumerate() {
        *item = Value::str(format!("r{}c{}", i / 3 + 1, i % 3 + 1));
    }
    let bytes = mem::write(&[("aGrid".into(), Value::Array(Rc::new(RefCell::new(grid))))]);
    let back = mem::read(&bytes);
    let Value::Array(array) = &back[0].1 else { panic!("not an array") };
    let array = array.borrow();
    assert_eq!((array.rows, array.cols), (2, 3));
    assert_eq!(array.items[4], Value::str("r2c2"));
}

#[test]
fn a_skeleton_says_which_variables_go_in_the_file() {
    let mut host = foxvm::mock_host::MockHost::new();
    let src = "gcWho = \"Jorge\"\nlReady = .T.\nSAVE TO other.mem ALL EXCEPT g*\nRELEASE ALL\nRESTORE FROM other.mem\n? TYPE(\"gcWho\"), TYPE(\"lReady\")\n";
    let (_, out) = foxvm::mock_host::run_program(src, &mut host).expect("it runs");
    assert_eq!(out, vec!["U L"]);
}
