//! Numeric built-ins.

mod ctx;

use ctx::{d, err, n, num, s, value};
use foxvm::error::RtError;
use foxvm::value::Value;

#[test]
fn truncation_and_rounding_helpers() {
    assert_eq!(num("INT", vec![n(3.9)]), 3.0);
    assert_eq!(num("INT", vec![n(-3.9)]), -3.0);
    assert_eq!(num("ABS", vec![n(-4.5)]), 4.5);
    assert_eq!(num("CEILING", vec![n(3.1)]), 4.0);
    assert_eq!(num("CEILING", vec![n(-3.1)]), -3.0);
    assert_eq!(num("FLOOR", vec![n(3.9)]), 3.0);
    assert_eq!(num("FLOOR", vec![n(-3.1)]), -4.0);
    assert_eq!(num("SIGN", vec![n(-2.0)]), -1.0);
    assert_eq!(num("SIGN", vec![n(0.0)]), 0.0);
    assert_eq!(num("SIGN", vec![n(7.0)]), 1.0);
}

#[test]
fn round_goes_half_away_from_zero() {
    assert_eq!(num("ROUND", vec![n(2.5), n(0.0)]), 3.0);
    assert_eq!(num("ROUND", vec![n(-2.5), n(0.0)]), -3.0);
    assert_eq!(num("ROUND", vec![n(3.4999), n(0.0)]), 3.0);
    assert_eq!(num("ROUND", vec![n(2.675), n(2.0)]), 2.68);
    assert_eq!(num("ROUND", vec![n(-2.675), n(2.0)]), -2.68);
    assert_eq!(num("ROUND", vec![n(1.005), n(2.0)]), 1.01);
    // Negative places round to tens, hundreds...
    assert_eq!(num("ROUND", vec![n(1234.0), n(-2.0)]), 1200.0);
    assert_eq!(num("ROUND", vec![n(1250.0), n(-2.0)]), 1300.0);
    assert_eq!(num("ROUND", vec![n(0.0), n(2.0)]), 0.0);
}

#[test]
fn mod_takes_the_sign_of_the_divisor() {
    assert_eq!(num("MOD", vec![n(10.0), n(3.0)]), 1.0);
    assert_eq!(num("MOD", vec![n(-7.0), n(3.0)]), 2.0);
    assert_eq!(num("MOD", vec![n(7.0), n(-3.0)]), -2.0);
    assert_eq!(num("MOD", vec![n(-7.0), n(-3.0)]), -1.0);
    assert_eq!(err("MOD", vec![n(1.0), n(0.0)]).code, RtError::DIVISION_BY_ZERO);
}

#[test]
fn max_and_min_over_every_comparable_type() {
    assert_eq!(num("MAX", vec![n(3.0), n(9.0), n(4.0)]), 9.0);
    assert_eq!(num("MIN", vec![n(3.0), n(9.0), n(-4.0)]), -4.0);
    assert_eq!(value("MAX", vec![s("apple"), s("banana")]), Value::str("banana"));
    assert_eq!(value("MIN", vec![s("apple"), s("banana")]), Value::str("apple"));
    assert_eq!(value("MAX", vec![d(2026, 1, 1), d(2026, 9, 7)]), d(2026, 9, 7));
    assert_eq!(value("MIN", vec![d(2026, 1, 1), d(2026, 9, 7)]), d(2026, 1, 1));
    assert!(value("MAX", vec![n(1.0), Value::Null]).is_null());
    assert_eq!(err("MAX", vec![n(1.0), s("a")]).code, RtError::TYPE_MISMATCH);
}

#[test]
fn transcendental_functions() {
    assert_eq!(num("SQRT", vec![n(9.0)]), 3.0);
    assert_eq!(num("EXP", vec![n(0.0)]), 1.0);
    assert!((num("LOG", vec![n(std::f64::consts::E)]) - 1.0).abs() < 1e-12);
    assert_eq!(num("LOG10", vec![n(1000.0)]), 3.0);
    assert_eq!(err("SQRT", vec![n(-1.0)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("LOG", vec![n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("LOG10", vec![n(-2.0)]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn rand_uses_the_host_generator() {
    let a = num("RAND", vec![]);
    let b = num("RAND", vec![n(-1.0)]); // the seed argument is accepted and ignored
    assert!((0.0..1.0).contains(&a));
    assert!((0.0..1.0).contains(&b));
}

#[test]
fn bit_functions_are_32_bit() {
    assert_eq!(num("BITAND", vec![n(12.0), n(10.0)]), 8.0);
    assert_eq!(num("BITAND", vec![n(7.0), n(3.0), n(1.0)]), 1.0);
    assert_eq!(num("BITOR", vec![n(12.0), n(10.0)]), 14.0);
    assert_eq!(num("BITXOR", vec![n(12.0), n(10.0)]), 6.0);
    assert_eq!(num("BITNOT", vec![n(0.0)]), -1.0);
    assert_eq!(num("BITNOT", vec![n(1.0)]), -2.0);
    assert_eq!(num("BITLSHIFT", vec![n(1.0), n(4.0)]), 16.0);
    assert_eq!(num("BITLSHIFT", vec![n(1.0), n(31.0)]), -2147483648.0);
    assert_eq!(num("BITLSHIFT", vec![n(1.0), n(32.0)]), 0.0);
    assert_eq!(num("BITRSHIFT", vec![n(16.0), n(4.0)]), 1.0);
    assert_eq!(num("BITRSHIFT", vec![n(-1.0), n(28.0)]), 15.0);
}

#[test]
fn null_propagates() {
    for name in ["INT", "ABS", "CEILING", "FLOOR", "SIGN", "SQRT", "EXP", "BITNOT"] {
        assert!(value(name, vec![Value::Null]).is_null(), "{name}() should return NULL");
    }
    assert!(value("ROUND", vec![Value::Null, n(2.0)]).is_null());
    assert!(value("MOD", vec![n(1.0), Value::Null]).is_null());
    assert!(value("BITAND", vec![n(1.0), Value::Null]).is_null());
}
