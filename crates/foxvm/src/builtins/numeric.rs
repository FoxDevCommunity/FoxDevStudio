//! Numeric functions. NULL propagates; the bit functions work on 32-bit signed integers, like
//! VFP on Windows.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, VARIADIC, any_null, arg_int, arg_num, ok, opt_int, spec};
use crate::error::RtError;
use crate::value::{CmpOp, Settings, VARIABLE_WHOLE, Value, Width, compare, modulo};

/// Rounds half away from zero at `places` decimals (negative places round to tens, hundreds...).
/// A relative epsilon keeps values such as 2.675 (stored as 2.67499999...) rounding the way a
/// FoxPro user expects.
pub(crate) fn round_to(n: f64, places: i32) -> f64 {
    if !n.is_finite() {
        return n;
    }
    let factor = 10f64.powi(places.clamp(-300, 300));
    let scaled = n * factor;
    if !scaled.is_finite() {
        return n;
    }
    let eps = scaled.abs() * f64::EPSILON * 4.0;
    let rounded = if n >= 0.0 { (scaled + 0.5 + eps).floor() } else { (scaled - 0.5 - eps).ceil() };
    let out = rounded / factor;
    if out == 0.0 { 0.0 } else { out }
}


// ---- how wide a function's answer prints ----
//
// Most functions answer as wide as a variable would - `? RECNO()` is nine spaces and a 1 - and
// `Value::number` gives them that for nothing. The ones below have widths of their own, each
// read off vfp9.exe: `? PI()` is twenty wide, `? SQRT(9)` is four, `? CEILING(3.1)` is twenty
// and whole. Nothing here depends on the argument's value, only on how it was written.

/// Twenty wide with `SET DECIMALS` places: PI, the trigonometry family, LOG, LOG10, RAND.
fn a_real_number(_arg: Width, s: &Settings) -> Width {
    Width { chars: 20, decimals: s.decimals, written: false }
}

/// Twenty wide and whole: CEILING, FLOOR, SIGN.
fn a_whole_number(_arg: Width, _s: &Settings) -> Width {
    Width { chars: 20, decimals: 0, written: false }
}

/// As many whole digits as the argument had, to `SET DECIMALS` places: SQRT.
fn as_wide_as_asked(arg: Width, s: &Settings) -> Width {
    Width::of(arg.whole(), s.decimals)
}

/// A variable's ten whole digits, to `SET DECIMALS` places: EXP.
fn as_wide_as_a_variable(_arg: Width, s: &Settings) -> Width {
    Width::of(VARIABLE_WHOLE, s.decimals)
}

/// One argument, one answer, in a width of the function's own choosing.
fn unary_wide(
    c: &mut dyn BuiltinCtx,
    a: &[Value],
    width: fn(Width, &Settings) -> Width,
    f: impl FnOnce(f64) -> Result<f64, RtError>,
) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let shape = width(a.first().map_or(Width::of(1, 0), Value::width), c.settings());
    ok(Value::Number(f(arg_num(a, 0)?)?, shape))
}

/// The same, for the functions that hand back an amount in the units they were given.
///
/// Visual FoxPro keeps money through INT, ABS, CEILING, FLOOR, ROUND and MOD - each answers in
/// the same units as its argument - and drops it through SIGN, SQRT, EXP, LOG and the rest,
/// which answer in units of their own. Measured, function by function, against vfp9.exe.
fn unary_keeping_money(
    c: &mut dyn BuiltinCtx,
    a: &[Value],
    width: fn(Width, &Settings) -> Width,
    f: impl FnOnce(f64) -> Result<f64, RtError>,
) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let answer = f(arg_num(a, 0)?)?;
    if matches!(a[0].deref(), Value::Currency(_)) {
        return ok(crate::value::to_currency(answer));
    }
    let shape = width(a.first().map_or(Width::of(1, 0), Value::width), c.settings());
    ok(Value::Number(answer, shape))
}

/// INT() keeps the room its argument had and drops the places: `? INT(price)` on a `N(8,2)`
/// field is eight wide and whole.
fn cut_short(arg: Width, _s: &Settings) -> Width {
    Width { chars: arg.chars, decimals: 0, written: false }
}

/// ABS() hands back the argument's own width when it had places, and a variable's ten when it
/// had none: `? ABS(price)` on a `N(8,2)` is eight wide, `? ABS(-7)` is ten.
fn without_the_sign(arg: Width, _s: &Settings) -> Width {
    if arg.decimals > 0 { Width { written: false, ..arg } } else { Width::of(VARIABLE_WHOLE, 0) }
}

fn f_int(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_keeping_money(c, &a, cut_short, |n| Ok(n.trunc()))
}

fn f_abs(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_keeping_money(c, &a, without_the_sign, |n| Ok(n.abs()))
}

fn f_ceiling(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_keeping_money(c, &a, a_whole_number, |n| Ok(n.ceil()))
}

fn f_floor(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_keeping_money(c, &a, a_whole_number, |n| Ok(n.floor()))
}

fn f_exp(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, as_wide_as_a_variable, |n| Ok(n.exp()))
}

fn f_sqrt(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, as_wide_as_asked, |n| {
        if n < 0.0 { Err(RtError::function_arg_invalid()) } else { Ok(n.sqrt()) }
    })
}

fn f_log(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| if n <= 0.0 { Err(RtError::function_arg_invalid()) } else { Ok(n.ln()) })
}

fn f_log10(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| if n <= 0.0 { Err(RtError::function_arg_invalid()) } else { Ok(n.log10()) })
}

fn f_sign(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_whole_number, |n| {
        Ok(if n > 0.0 {
            1.0
        } else if n < 0.0 {
            -1.0
        } else {
            0.0
        })
    })
}

/// ROUND(x, n) shows `n` places, in a width one wider than the argument's - and wider still
/// when it is asked for more places than the argument had room for.
fn f_round(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let n = arg_num(&a, 0)?;
    let places = opt_int(&a, 1, 0)?.clamp(-300, 300) as i32;
    if matches!(a[0].deref(), Value::Currency(_)) {
        return ok(crate::value::to_currency(round_to(n, places)));
    }
    let arg = a[0].width();
    let asked = places.clamp(0, 255) as u8;
    let chars = arg.chars.saturating_add(1).saturating_add(asked.saturating_sub(arg.decimals));
    ok(Value::Number(round_to(n, places), Width { chars, decimals: asked, written: false }))
}

fn f_mod(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let settings = c.settings().clone();
    ok(modulo(&a[0], &a[1], &settings)?)
}

/// MAX()/MIN() compare with the VFP rules, so they also work on strings, dates and datetimes.
fn extremum(ctx: &mut dyn BuiltinCtx, a: &[Value], op: CmpOp) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let settings = ctx.settings().clone();
    let mut best = a[0].deref();
    for v in &a[1..] {
        let v = v.deref();
        if compare(&v, &best, op, &settings)?.truthy()? {
            best = v;
        }
    }
    ok(best)
}

fn f_max(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    extremum(c, &a, CmpOp::Gt)
}

fn f_min(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    extremum(c, &a, CmpOp::Lt)
}

/// `RAND([nSeed])`: the next number in [0, 1), or the first of the sequence `nSeed` starts.
///
/// A seed restarts the sequence, so the same seed twice running gives the same number and a
/// plain `RAND()` after it carries on - measured. Zero and anything below it mean the sequence
/// the generator starts on, which is why Visual FoxPro answers `RAND(0)` and `RAND(-1)` alike.
/// The numbers themselves are this generator's; nothing can reproduce Visual FoxPro's.
fn f_rand(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if let Some(v) = a.first() {
        let seed = v.as_number()?;
        c.host().seed_random(if seed > 0.0 { seed } else { 0.0 });
    }
    ok(Value::number(c.host().random()))
}

// ---- bit functions (32-bit, signed like VFP) ----

fn bits(args: &[Value], i: usize) -> Result<i32, RtError> {
    Ok(arg_int(args, i)? as i32)
}

fn bit_fold(a: &[Value], f: impl Fn(i32, i32) -> i32) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let mut acc = bits(a, 0)?;
    for i in 1..a.len() {
        acc = f(acc, bits(a, i)?);
    }
    ok(Value::number(acc as f64))
}

fn f_bitand(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    bit_fold(&a, |x, y| x & y)
}

fn f_bitor(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    bit_fold(&a, |x, y| x | y)
}

fn f_bitxor(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    bit_fold(&a, |x, y| x ^ y)
}

fn f_bitnot(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::number(!bits(&a, 0)? as f64))
}

fn f_bitlshift(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let n = bits(&a, 0)?;
    let by = arg_int(&a, 1)?;
    let out = if !(0..32).contains(&by) { 0 } else { ((n as u32) << by) as i32 };
    ok(Value::number(out as f64))
}

fn f_bitrshift(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let n = bits(&a, 0)?;
    let by = arg_int(&a, 1)?;
    let out = if !(0..32).contains(&by) { 0 } else { ((n as u32) >> by) as i32 };
    ok(Value::number(out as f64))
}


// ------------------------------------------------------------------------------------------
// trigonometry
//
// Angles are radians, as Visual FoxPro's are; DTOR() and RTOD() convert. The inverse functions
// answer within their own range: ACOS() 0 to pi, ASIN() and ATAN() -pi/2 to pi/2.
// ------------------------------------------------------------------------------------------

/// FV(nPayment, nInterestRate, nPeriods): what a run of payments is worth at the end of it.
///
/// The rate is per period, as Visual FoxPro takes it: a yearly rate over months is the yearly
/// one divided by twelve, and the caller does that division.
fn f_fv(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (payment, rate, periods) = (arg_num(&a, 0)?, arg_num(&a, 1)?, arg_num(&a, 2)?);
    let value = if rate == 0.0 { payment * periods } else { payment * ((1.0 + rate).powf(periods) - 1.0) / rate };
    answered(c, value, as_wide_as_a_variable)
}

/// PV(nPayment, nInterestRate, nPeriods): what that run of payments is worth now.
fn f_pv(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (payment, rate, periods) = (arg_num(&a, 0)?, arg_num(&a, 1)?, arg_num(&a, 2)?);
    let value = if rate == 0.0 { payment * periods } else { payment * (1.0 - (1.0 + rate).powf(-periods)) / rate };
    answered(c, value, as_wide_as_a_variable)
}

/// PAYMENT(nPrincipal, nInterestRate, nPayments): what each payment of a loan comes to.
fn f_payment(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (principal, rate, periods) = (arg_num(&a, 0)?, arg_num(&a, 1)?, arg_num(&a, 2)?);
    if periods == 0.0 {
        return Err(RtError::function_arg_invalid());
    }
    let value = if rate == 0.0 { principal / periods } else { principal * rate / (1.0 - (1.0 + rate).powf(-periods)) };
    answered(c, value, as_wide_as_a_variable)
}

/// A number a function names for itself, with no argument to take a width from.
fn answered(c: &mut dyn BuiltinCtx, n: f64, width: fn(Width, &Settings) -> Width) -> Result<BuiltinResult, RtError> {
    let shape = width(Width::of(1, 0), c.settings());
    ok(Value::Number(n, shape))
}

fn f_pi(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    answered(c, std::f64::consts::PI, a_real_number)
}

fn f_sin(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| Ok(n.sin()))
}

fn f_cos(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| Ok(n.cos()))
}

fn f_tan(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| Ok(n.tan()))
}

/// ASIN() and ACOS() are undefined outside -1 to 1, which VFP reports as a bad argument rather
/// than answering with a number that is not one.
fn f_asin(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |x| {
        if !(-1.0..=1.0).contains(&x) { Err(RtError::function_arg_invalid()) } else { Ok(x.asin()) }
    })
}

fn f_acos(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |x| {
        if !(-1.0..=1.0).contains(&x) { Err(RtError::function_arg_invalid()) } else { Ok(x.acos()) }
    })
}

fn f_atan(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| Ok(n.atan()))
}

/// ATN2(y, x): the angle of the point (x, y), which unlike ATAN() knows which quadrant it is in.
fn f_atn2(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let x = arg_num(&a, 1)?;
    unary_wide(c, &a, a_real_number, |y| Ok(y.atan2(x)))
}

fn f_dtor(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| Ok(n.to_radians()))
}

fn f_rtod(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    unary_wide(c, &a, a_real_number, |n| Ok(n.to_degrees()))
}

// ------------------------------------------------------------------------------------------
// bits
// ------------------------------------------------------------------------------------------

/// BITSET(n, position [, ...]) sets bits, BITCLEAR() clears them and BITTEST() reports one.
/// Bit 0 is the least significant, as VFP counts them.
fn bit_positions(a: &[Value], from: usize) -> Result<Vec<u32>, RtError> {
    a.iter()
        .skip(from)
        .map(|v| {
            let n = v.as_number()?.trunc();
            if !(0.0..32.0).contains(&n) {
                return Err(RtError::function_arg_invalid());
            }
            Ok(n as u32)
        })
        .collect()
}

fn f_bitset(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut n = arg_num(&a, 0)?.trunc() as i64 as u32;
    for bit in bit_positions(&a, 1)? {
        n |= 1 << bit;
    }
    ok(Value::number(n as i32 as f64))
}

fn f_bitclear(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut n = arg_num(&a, 0)?.trunc() as i64 as u32;
    for bit in bit_positions(&a, 1)? {
        n &= !(1 << bit);
    }
    ok(Value::number(n as i32 as f64))
}

fn f_bittest(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let n = arg_num(&a, 0)?.trunc() as i64 as u32;
    let bit = bit_positions(&a, 1)?;
    ok(Value::Logical(bit.first().is_some_and(|b| n & (1 << b) != 0)))
}

// ------------------------------------------------------------------------------------------
// currency
//
// This runtime has one numeric type, so a currency value is a number rounded to the four
// decimal places Visual FoxPro's Currency keeps. The conversions are still real work: NTOM()
// rounds, and MTON() hands the number back.
// ------------------------------------------------------------------------------------------

/// `NTOM(n)`: the amount as money, which is to say kept in whole ten-thousandths.
fn f_ntom(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(crate::value::to_currency(arg_num(&a, 0)?))
}

/// MTON() hands the amount over as a Number, twenty wide with money's four places.
fn f_mton(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Number(arg_num(&a, 0)?, Width { chars: 20, decimals: 4, written: false }))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("ABS", 1, 1, f_abs),
        spec("ACOS", 1, 1, f_acos),
        spec("ASIN", 1, 1, f_asin),
        spec("ATAN", 1, 1, f_atan),
        spec("ATN2", 2, 2, f_atn2),
        spec("BITAND", 2, VARIADIC, f_bitand),
        spec("BITCLEAR", 2, VARIADIC, f_bitclear),
        spec("BITLSHIFT", 2, 2, f_bitlshift),
        spec("BITNOT", 1, 1, f_bitnot),
        spec("BITOR", 2, VARIADIC, f_bitor),
        spec("BITRSHIFT", 2, 2, f_bitrshift),
        spec("BITSET", 2, VARIADIC, f_bitset),
        spec("BITTEST", 2, 2, f_bittest),
        spec("BITXOR", 2, VARIADIC, f_bitxor),
        spec("CEILING", 1, 1, f_ceiling),
        spec("COS", 1, 1, f_cos),
        spec("DTOR", 1, 1, f_dtor),
        spec("EXP", 1, 1, f_exp),
        spec("FLOOR", 1, 1, f_floor),
        spec("FV", 3, 3, f_fv),
        spec("INT", 1, 1, f_int),
        spec("LOG", 1, 1, f_log),
        spec("LOG10", 1, 1, f_log10),
        spec("MAX", 2, VARIADIC, f_max),
        spec("MIN", 2, VARIADIC, f_min),
        spec("MOD", 2, 2, f_mod),
        spec("MTON", 1, 1, f_mton),
        spec("NTOM", 1, 1, f_ntom),
        spec("PAYMENT", 3, 3, f_payment),
        spec("PI", 0, 0, f_pi),
        spec("PV", 3, 3, f_pv),
        spec("RAND", 0, 1, f_rand),
        spec("ROUND", 2, 2, f_round),
        spec("RTOD", 1, 1, f_rtod),
        spec("SIGN", 1, 1, f_sign),
        spec("SIN", 1, 1, f_sin),
        spec("SQRT", 1, 1, f_sqrt),
        spec("TAN", 1, 1, f_tan),
    ]
}
