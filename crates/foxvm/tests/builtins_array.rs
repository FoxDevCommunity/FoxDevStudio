//! Array built-ins. Every one of these mutates the array through the shared reference.

mod ctx;

use ctx::{array, array2, err, items, n, num, s, texts, value};
use foxvm::error::RtError;
use foxvm::value::Value;

fn abc() -> Value {
    array(vec![s("a"), s("b"), s("c")])
}

fn grid() -> Value {
    // 3 rows x 2 columns: ("c", 1), ("a", 2), ("b", 3)
    array2(3, 2, vec![s("c"), n(1.0), s("a"), n(2.0), s("b"), n(3.0)])
}

#[test]
fn alen_reports_the_shape() {
    let a = abc();
    assert_eq!(num("ALEN", vec![a.clone()]), 3.0);
    assert_eq!(num("ALEN", vec![a.clone(), n(0.0)]), 3.0);
    assert_eq!(num("ALEN", vec![a.clone(), n(1.0)]), 3.0);
    assert_eq!(num("ALEN", vec![a, n(2.0)]), 0.0, "a one-dimensional array has no columns");
    let g = grid();
    assert_eq!(num("ALEN", vec![g.clone()]), 6.0);
    assert_eq!(num("ALEN", vec![g.clone(), n(1.0)]), 3.0);
    assert_eq!(num("ALEN", vec![g, n(2.0)]), 2.0);
    assert_eq!(err("ALEN", vec![s("not an array")]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn ascan_finds_elements() {
    let a = abc();
    assert_eq!(num("ASCAN", vec![a.clone(), s("b")]), 2.0);
    assert_eq!(num("ASCAN", vec![a.clone(), s("z")]), 0.0, "a miss returns 0");
    assert_eq!(num("ASCAN", vec![a.clone(), s("b"), n(3.0)]), 0.0, "the search starts at element 3");
    assert_eq!(num("ASCAN", vec![a.clone(), s("c"), n(2.0)]), 3.0);
    assert_eq!(num("ASCAN", vec![a.clone(), s("a"), n(1.0), n(1.0)]), 1.0);
    assert_eq!(num("ASCAN", vec![a, s("c"), n(1.0), n(2.0)]), 0.0, "only two elements are searched");

    // SET EXACT is OFF, so a prefix matches unless flag 1 asks for an exact comparison.
    let words = array(vec![s("apple"), s("banana")]);
    assert_eq!(num("ASCAN", vec![words.clone(), s("app")]), 1.0);
    assert_eq!(num("ASCAN", vec![words, s("app"), n(1.0), n(-1.0), n(0.0), n(1.0)]), 0.0);

    // A column filter and the "return the row" flag.
    let g = grid();
    assert_eq!(num("ASCAN", vec![g.clone(), n(3.0), n(1.0), n(-1.0), n(2.0)]), 6.0);
    assert_eq!(num("ASCAN", vec![g.clone(), s("a")]), 3.0);
    assert_eq!(num("ASCAN", vec![g, s("a"), n(1.0), n(-1.0), n(0.0), n(4.0)]), 2.0);
}

#[test]
fn asort_ascending_and_descending() {
    let a = array(vec![s("c"), s("a"), s("b")]);
    // Numeric, not Logical - measured: ASORT()'s own reference page gives its return value as
    // "Numeric: 1 if successful", and it prints as "1", not ".T.".
    assert_eq!(value("ASORT", vec![a.clone()]), n(1.0));
    assert_eq!(texts(&a), vec!["a", "b", "c"]);
    assert_eq!(value("ASORT", vec![a.clone(), n(1.0), n(-1.0), n(1.0)]), n(1.0));
    assert_eq!(texts(&a), vec!["c", "b", "a"]);

    // Numbers, and a partial range that leaves the head alone.
    let nums = array(vec![n(5.0), n(3.0), n(9.0), n(1.0)]);
    value("ASORT", vec![nums.clone(), n(2.0)]);
    assert_eq!(items(&nums), vec![n(5.0), n(1.0), n(3.0), n(9.0)]);

    // A 2-D array sorts whole rows on the column of the start element.
    let g = grid();
    value("ASORT", vec![g.clone()]);
    assert_eq!(items(&g), vec![s("a"), n(2.0), s("b"), n(3.0), s("c"), n(1.0)]);
    value("ASORT", vec![g.clone(), n(2.0), n(-1.0), n(1.0)]);
    assert_eq!(items(&g), vec![s("b"), n(3.0), s("a"), n(2.0), s("c"), n(1.0)]);
}

#[test]
fn adel_and_ains_shift_elements() {
    let a = array(vec![n(1.0), n(2.0), n(3.0)]);
    assert_eq!(num("ADEL", vec![a.clone(), n(2.0)]), 1.0);
    assert_eq!(items(&a), vec![n(1.0), n(3.0), Value::Logical(false)]);

    let b = array(vec![n(1.0), n(2.0), n(3.0)]);
    assert_eq!(num("AINS", vec![b.clone(), n(2.0)]), 1.0);
    assert_eq!(items(&b), vec![n(1.0), Value::Logical(false), n(2.0)], "the last element falls off");

    // Rows of a 2-D array.
    let g = array2(2, 2, vec![n(1.0), n(2.0), n(3.0), n(4.0)]);
    num("ADEL", vec![g.clone(), n(1.0)]);
    assert_eq!(items(&g), vec![n(3.0), n(4.0), Value::Logical(false), Value::Logical(false)]);

    let g = array2(2, 2, vec![n(1.0), n(2.0), n(3.0), n(4.0)]);
    num("AINS", vec![g.clone(), n(1.0)]);
    assert_eq!(items(&g), vec![Value::Logical(false), Value::Logical(false), n(1.0), n(2.0)]);

    // Columns, with the flag.
    let g = array2(2, 2, vec![n(1.0), n(2.0), n(3.0), n(4.0)]);
    num("ADEL", vec![g.clone(), n(1.0), n(2.0)]);
    assert_eq!(items(&g), vec![n(2.0), Value::Logical(false), n(4.0), Value::Logical(false)]);

    assert_eq!(err("ADEL", vec![abc(), n(9.0)]).code, RtError::INVALID_SUBSCRIPT);
    assert_eq!(err("AINS", vec![abc(), n(0.0)]).code, RtError::INVALID_SUBSCRIPT);
}

#[test]
fn acopy_moves_elements() {
    let src = abc();
    let dest = array(vec![Value::Logical(false); 3]);
    assert_eq!(num("ACOPY", vec![src.clone(), dest.clone()]), 3.0);
    assert_eq!(texts(&dest), vec!["a", "b", "c"]);

    let dest = array(vec![Value::Logical(false); 3]);
    assert_eq!(num("ACOPY", vec![src.clone(), dest.clone(), n(2.0)]), 2.0);
    assert_eq!(items(&dest)[..2], [s("b"), s("c")]);

    let dest = array(vec![Value::Logical(false); 3]);
    assert_eq!(num("ACOPY", vec![src.clone(), dest.clone(), n(1.0), n(2.0), n(2.0)]), 2.0);
    assert_eq!(items(&dest), vec![Value::Logical(false), s("a"), s("b")]);

    // A destination that is too small grows.
    let small = array(vec![Value::Logical(false)]);
    assert_eq!(num("ACOPY", vec![src, small.clone()]), 3.0);
    assert_eq!(texts(&small), vec!["a", "b", "c"]);
}

#[test]
fn element_and_subscript_conversions() {
    let g = grid();
    assert_eq!(num("AELEMENT", vec![g.clone(), n(2.0), n(2.0)]), 4.0);
    assert_eq!(num("AELEMENT", vec![g.clone(), n(1.0), n(1.0)]), 1.0);
    assert_eq!(num("ASUBSCRIPT", vec![g.clone(), n(4.0), n(1.0)]), 2.0);
    assert_eq!(num("ASUBSCRIPT", vec![g.clone(), n(4.0), n(2.0)]), 2.0);
    assert_eq!(num("ASUBSCRIPT", vec![g.clone(), n(5.0), n(1.0)]), 3.0);
    // Measured: a row past the array's own row count is error 1234 ("Subscript is outside
    // defined range"), not the generic 31 ASUBSCRIPT's own out-of-range element number keeps.
    assert_eq!(err("AELEMENT", vec![g.clone(), n(9.0), n(1.0)]).code, RtError::SUBSCRIPT_OUT_OF_RANGE);
    assert_eq!(err("ASUBSCRIPT", vec![g, n(99.0), n(1.0)]).code, RtError::INVALID_SUBSCRIPT);
    assert_eq!(num("AELEMENT", vec![abc(), n(2.0)]), 2.0);
}

#[test]
fn arrays_are_reached_through_a_reference_cell() {
    let a = abc();
    let cell = Value::Ref(std::rc::Rc::new(std::cell::RefCell::new(a.clone())));
    assert_eq!(num("ALEN", vec![cell.clone()]), 3.0);
    num("ADEL", vec![cell, n(1.0)]);
    assert_eq!(items(&a), vec![s("b"), s("c"), Value::Logical(false)]);
}

#[test]
fn aempty_leaves_a_zero_length_array() {
    let a = abc();
    assert_eq!(num("AEMPTY", vec![a.clone()]), 0.0);
    assert!(items(&a).is_empty(), "AEMPTY() removes every element");
    assert_eq!(num("ALEN", vec![a.clone()]), 0.0);
    assert_eq!(num("ALEN", vec![a.clone(), n(1.0)]), 0.0, "no rows are left");
    assert_eq!(num("ALEN", vec![a, n(2.0)]), 0.0, "no columns are left");

    // AEMPTYNEW is the VFP 9 spelling of the same thing, on a two-dimensional array here
    let g = grid();
    assert_eq!(num("AEMPTYNEW", vec![g.clone()]), 0.0);
    assert!(items(&g).is_empty());
    assert_eq!(num("ALEN", vec![g, n(2.0)]), 0.0);

    // emptying an already empty array is not an error
    let e = array(vec![]);
    assert_eq!(num("AEMPTY", vec![e]), 0.0);
    assert_eq!(err("AEMPTY", vec![s("not an array")]).code, RtError::FUNCTION_ARG_INVALID);
}
