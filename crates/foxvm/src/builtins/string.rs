//! Character functions.
//!
//! VFP strings are byte strings with 1-based positions, so everything here works on bytes and
//! converts back with `from_utf8_lossy`: ASCII behaves exactly like VFP and non-ASCII input can
//! never panic (it may be re-encoded, which is out of scope for M2). A NULL argument makes the
//! whole call NULL.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, VARIADIC, any_null, arg_int, arg_num, arg_str, ok, opt_int, spec};
use crate::error::RtError;
use crate::value::{FoxArray, Settings, Value, display, format_date, format_datetime};

// ------------------------------------------------------------------------------------------
// byte helpers
// ------------------------------------------------------------------------------------------

/// One argument as the bytes it is.
///
/// A FoxPro string is bytes, not characters: a program packs a Windows structure into one, and
/// `LEN`, `SUBSTR` and the rest count and cut those bytes. Rust holds the string as UTF-8, where
/// a byte above 127 takes two, so counting the storage instead of the characters made every
/// string holding one longer than it is - `LEN(BINTOC(2002, "2RS"))` answered 3 for a two-byte
/// word, and every struct read past such a byte came out shifted.
fn bytes(args: &[Value], i: usize) -> Result<Vec<u8>, RtError> {
    Ok(arg_str(args, i)?.chars().map(|c| c as u32 as u8).collect())
}

fn text(b: &[u8]) -> Value {
    Value::str(b.iter().map(|c| *c as char).collect::<String>())
}

/// The argument as text: characters as they are, anything else through `display`.
fn as_text(v: &Value, settings: &Settings) -> String {
    match v.deref() {
        Value::Str(s) => s.to_string(),
        other => display(&other, settings),
    }
}

fn lower_bytes(b: &[u8]) -> Vec<u8> {
    b.iter().map(|c| c.to_ascii_lowercase()).collect()
}

/// Byte offsets of every occurrence of `needle` in `hay` (overlapping occurrences included,
/// like VFP OCCURS/AT).
fn find_all(hay: &[u8], needle: &[u8], case_insensitive: bool) -> Vec<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return Vec::new();
    }
    let (h, n) =
        if case_insensitive { (lower_bytes(hay), lower_bytes(needle)) } else { (hay.to_vec(), needle.to_vec()) };
    (0..=h.len() - n.len()).filter(|&i| h[i..i + n.len()] == n[..]).collect()
}

/// Trim characters for ALLTRIM/LTRIM/RTRIM: numeric extra arguments are the VFP flags, which
/// this runtime ignores, character ones add to the set. Defaults to the space.
fn trim_set(args: &[Value], from: usize) -> Result<Vec<u8>, RtError> {
    let mut set = Vec::new();
    for v in args.iter().skip(from) {
        match v.deref() {
            Value::Number(..) | Value::Logical(_) => {}
            Value::Str(s) => set.extend_from_slice(s.as_bytes()),
            _ => return Err(RtError::function_arg_invalid()),
        }
    }
    if set.is_empty() {
        set.push(b' ');
    }
    Ok(set)
}

fn trimmed(b: &[u8], set: &[u8], left: bool, right: bool) -> Vec<u8> {
    let mut start = 0;
    let mut end = b.len();
    if left {
        while start < end && set.contains(&b[start]) {
            start += 1;
        }
    }
    if right {
        while end > start && set.contains(&b[end - 1]) {
            end -= 1;
        }
    }
    b[start..end].to_vec()
}

// ------------------------------------------------------------------------------------------
// trimming and case
// ------------------------------------------------------------------------------------------

fn f_alltrim(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(text(&trimmed(&bytes(&a, 0)?, &trim_set(&a, 1)?, true, true)))
}

fn f_ltrim(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(text(&trimmed(&bytes(&a, 0)?, &trim_set(&a, 1)?, true, false)))
}

fn f_rtrim(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(text(&trimmed(&bytes(&a, 0)?, &trim_set(&a, 1)?, false, true)))
}

fn f_upper(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::str(arg_str(&a, 0)?.to_ascii_uppercase()))
}

fn f_lower(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::str(arg_str(&a, 0)?.to_ascii_lowercase()))
}

fn f_proper(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let mut prev_alpha = false;
    let out: Vec<u8> = bytes(&a, 0)?
        .iter()
        .map(|&c| {
            let r = if prev_alpha { c.to_ascii_lowercase() } else { c.to_ascii_uppercase() };
            prev_alpha = c.is_ascii_alphanumeric();
            r
        })
        .collect();
    ok(text(&out))
}

// ------------------------------------------------------------------------------------------
// slicing
// ------------------------------------------------------------------------------------------

fn f_len(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::number(bytes(&a, 0)?.len() as f64))
}

fn f_substr(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let s = bytes(&a, 0)?;
    let start = arg_int(&a, 1)?.max(1) as usize;
    if start > s.len() {
        return ok(Value::str(""));
    }
    let rest = s.len() - (start - 1);
    let len = match a.get(2) {
        None => rest,
        Some(_) => (arg_int(&a, 2)?.max(0) as usize).min(rest),
    };
    ok(text(&s[start - 1..start - 1 + len]))
}

fn f_left(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let s = bytes(&a, 0)?;
    let n = (arg_int(&a, 1)?.max(0) as usize).min(s.len());
    ok(text(&s[..n]))
}

fn f_right(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let s = bytes(&a, 0)?;
    let n = (arg_int(&a, 1)?.max(0) as usize).min(s.len());
    ok(text(&s[s.len() - n..]))
}

// ------------------------------------------------------------------------------------------
// searching
// ------------------------------------------------------------------------------------------

fn at_impl(a: &[Value], case_insensitive: bool, from_end: bool) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let needle = bytes(a, 0)?;
    let hay = bytes(a, 1)?;
    // nOccurrence must be a real occurrence number - measured: AT("a", "banana", 0) is error 11,
    // not the first occurrence a lenient clamp would have given it.
    let occurrence = opt_int(a, 2, 1)?;
    if occurrence < 1 {
        return Err(RtError::function_arg_invalid());
    }
    let occurrence = occurrence as usize;
    let hits = find_all(&hay, &needle, case_insensitive);
    let hit = if from_end { hits.iter().rev().nth(occurrence - 1) } else { hits.get(occurrence - 1) };
    ok(Value::number(hit.map_or(0.0, |i| (i + 1) as f64)))
}

fn f_at(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    at_impl(&a, false, false)
}

fn f_atc(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    at_impl(&a, true, false)
}

fn f_rat(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    at_impl(&a, false, true)
}

/// RATC(): RAT for a double-byte character set.
///
/// The C on the end of these names is the character set, not the case: `ATC` ignores case and
/// `ATCC` is the double-byte one that also ignores it, but `RATC` is `RAT` and minds it.
/// Measured: `RATC("A", "banana")` is 0 where `RATC("a", "banana")` is 6.
fn f_ratc(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    at_impl(&a, false, true)
}

fn f_occurs(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::number(find_all(&bytes(&a, 1)?, &bytes(&a, 0)?, false).len() as f64))
}

// ------------------------------------------------------------------------------------------
// padding and filling
// ------------------------------------------------------------------------------------------

enum Pad {
    Left,
    Right,
    Center,
}

fn pad(ctx: &mut dyn BuiltinCtx, a: &[Value], mode: Pad) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let s = as_text(&a[0], ctx.settings()).into_bytes();
    let len = arg_int(a, 1)?.max(0) as usize;
    let fill = match a.get(2) {
        None => b' ',
        Some(_) => *bytes(a, 2)?.first().unwrap_or(&b' '),
    };
    if s.len() >= len {
        // Truncation keeps the part the padding would have aligned.
        let extra = s.len() - len;
        return ok(text(match mode {
            Pad::Left => &s[extra..],
            Pad::Right => &s[..len],
            Pad::Center => &s[extra / 2..extra / 2 + len],
        }));
    }
    let missing = len - s.len();
    let mut out = Vec::with_capacity(len);
    match mode {
        Pad::Left => {
            out.extend(std::iter::repeat_n(fill, missing));
            out.extend_from_slice(&s);
        }
        Pad::Right => {
            out.extend_from_slice(&s);
            out.extend(std::iter::repeat_n(fill, missing));
        }
        Pad::Center => {
            out.extend(std::iter::repeat_n(fill, missing / 2));
            out.extend_from_slice(&s);
            out.extend(std::iter::repeat_n(fill, missing - missing / 2));
        }
    }
    ok(text(&out))
}

fn f_padl(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    pad(c, &a, Pad::Left)
}

fn f_padr(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    pad(c, &a, Pad::Right)
}

fn f_padc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    pad(c, &a, Pad::Center)
}

/// Guards REPLICATE/SPACE against a runaway allocation.
const MAX_STRING: i64 = 1 << 24;

fn f_space(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let n = arg_int(&a, 0)?;
    if n > MAX_STRING {
        return Err(RtError::function_arg_invalid());
    }
    ok(Value::str(" ".repeat(n.max(0) as usize)))
}

fn f_replicate(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let s = bytes(&a, 0)?;
    let n = arg_int(&a, 1)?.max(0);
    if n.saturating_mul(s.len() as i64) > MAX_STRING {
        return Err(RtError::function_arg_invalid());
    }
    ok(text(&s.repeat(n as usize)))
}

// ------------------------------------------------------------------------------------------
// replacing
// ------------------------------------------------------------------------------------------

/// Changes `repl`'s case to match what was found, the way STRTRAN's flag 2 does: the whole of
/// `repl` is uppercased, lowercased, or (for a found match that starts with one capital and
/// has no other capitals) given an initial capital and the rest lowered. A match that is none
/// of those - mixed case, or no letters at all - leaves `repl` exactly as given.
fn case_like(found: &[u8], repl: &[u8]) -> Vec<u8> {
    let letters: Vec<u8> = found.iter().copied().filter(u8::is_ascii_alphabetic).collect();
    if letters.iter().all(u8::is_ascii_uppercase) && letters.iter().any(u8::is_ascii_alphabetic) {
        return repl.to_ascii_uppercase();
    }
    if letters.iter().all(u8::is_ascii_lowercase) && !letters.is_empty() {
        return repl.to_ascii_lowercase();
    }
    if letters.first().is_some_and(u8::is_ascii_uppercase) && letters[1..].iter().all(u8::is_ascii_lowercase) {
        let mut out = repl.to_ascii_lowercase();
        if let Some(c) = out.first_mut() {
            c.make_ascii_uppercase();
        }
        return out;
    }
    repl.to_vec()
}

/// STRTRAN(cSearched, cSought [, cReplacement] [, nStartOccurrence] [, nNumberOfOccurrences]
/// [, nFlags]). Flags: 1 case-insensitive search, 2 case-adjust the replacement to match what
/// was found. nStartOccurrence and nNumberOfOccurrences are occurrence counts, not lenient
/// sizes - measured: an explicit 0 in either is error 11, the same as AT()'s occurrence.
fn f_strtran(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let hay = bytes(&a, 0)?;
    let needle = bytes(&a, 1)?;
    let repl = match a.get(2) {
        None => Vec::new(),
        Some(_) => bytes(&a, 2)?,
    };
    let start = match a.get(3) {
        None => 1usize,
        Some(_) => match arg_int(&a, 3)? {
            n if n < 1 => return Err(RtError::function_arg_invalid()),
            n => n as usize,
        },
    };
    let count = match a.get(4) {
        None => -1i64,
        Some(_) => match arg_int(&a, 4)? {
            0 => return Err(RtError::function_arg_invalid()),
            n => n,
        },
    };
    let flags = opt_int(&a, 5, 0)?;
    let case_adjust = flags & 2 != 0;
    let hits = find_all(&hay, &needle, flags & 1 != 0);
    if hits.is_empty() {
        return ok(text(&hay));
    }
    let mut out = Vec::with_capacity(hay.len());
    let mut cursor = 0usize;
    let mut done = 0i64;
    for (nth, &at) in hits.iter().enumerate() {
        if at < cursor || nth + 1 < start {
            continue;
        }
        if count >= 0 && done >= count {
            break;
        }
        out.extend_from_slice(&hay[cursor..at]);
        let this_repl = if case_adjust { case_like(&hay[at..at + needle.len()], &repl) } else { repl.clone() };
        out.extend_from_slice(&this_repl);
        cursor = at + needle.len();
        done += 1;
    }
    out.extend_from_slice(&hay[cursor..]);
    ok(text(&out))
}

fn f_stuff(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let s = bytes(&a, 0)?;
    let start = (arg_int(&a, 1)?.max(1) as usize - 1).min(s.len());
    let len = (arg_int(&a, 2)?.max(0) as usize).min(s.len() - start);
    let repl = bytes(&a, 3)?;
    let mut out = s[..start].to_vec();
    out.extend_from_slice(&repl);
    out.extend_from_slice(&s[start + len..]);
    ok(text(&out))
}

fn f_chrtran(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let s = bytes(&a, 0)?;
    let from = bytes(&a, 1)?;
    let to = bytes(&a, 2)?;
    let mut out = Vec::with_capacity(s.len());
    for c in s {
        match from.iter().position(|&f| f == c) {
            None => out.push(c),
            Some(i) if i < to.len() => out.push(to[i]),
            Some(_) => {} // no replacement character: the byte is removed
        }
    }
    ok(text(&out))
}

// ------------------------------------------------------------------------------------------
// conversions
// ------------------------------------------------------------------------------------------

fn f_chr(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let n = arg_int(&a, 0)?;
    if !(0..=255).contains(&n) {
        return Err(RtError::function_arg_invalid());
    }
    ok(Value::str(char::from(n as u8).to_string()))
}

fn f_asc(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    // a byte is three digits at the widest, and that is the width the answer carries: measured,
    // `? ASC(CHR(233))` is a space and 233 rather than the ten a working-out gets
    let width = crate::value::Width { chars: 4, decimals: 0, written: false };
    ok(Value::Number(bytes(&a, 0)?.first().copied().unwrap_or(0) as f64, width))
}

/// STR(): right-aligned in `len` (default 10) with `decimals` places (default 0). Decimal
/// places are dropped one at a time to make the number fit; if the integer part still does not
/// fit the result is all asterisks, like VFP.
fn f_str(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let point = c.settings().point;
    // what is written is the number as the runtime keeps it, which is rounded to where a
    // double stops being exact: STR(0.1, 20, 17) is 0.10000000000000000, not the double
    let n = crate::value::settle(arg_num(&a, 0)?);
    let len = opt_int(&a, 1, 10)?.clamp(0, 512) as usize;
    let mut dec = opt_int(&a, 2, 0)?.clamp(0, 18) as usize;
    if len == 0 {
        return ok(Value::str(""));
    }
    if !n.is_finite() {
        return ok(Value::str("*".repeat(len)));
    }
    loop {
        // formatted straight from the value: rounding it first by multiplying up to the
        // decimals asked for and back loses the number itself at any size - 2^53 came out as
        // 9007199254741000 - and the formatter rounds correctly anyway
        let body = crate::value::with_point(crate::value::to_places(n, dec), point);
        if body.len() <= len {
            return ok(Value::str(format!("{body:>len$}")));
        }
        if dec == 0 {
            return ok(Value::str("*".repeat(len)));
        }
        dec -= 1;
    }
}

/// VAL(): the leading numeric prefix (spaces, sign, digits, one decimal point); 0 when there
/// is none. The point it reads is `SET POINT`'s, so text written for the display reads back.
/// Exponent notation is not recognised, as in VFP.
fn f_val(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let point = u8::try_from(c.settings().point).unwrap_or(b'.');
    let s = bytes(&a, 0)?;
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t') {
        i += 1;
    }
    let mut lit = String::new();
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        lit.push(s[i] as char);
        i += 1;
    }
    let mut seen_dot = false;
    let mut digits = 0;
    while i < s.len() {
        match s[i] {
            b'0'..=b'9' => {
                lit.push(s[i] as char);
                digits += 1;
            }
            c if c == point && !seen_dot => {
                seen_dot = true;
                lit.push('.');
            }
            _ => break,
        }
        i += 1;
    }
    // the answer is as wide as the text it was read out of, with SET DECIMALS places on the
    // end: `? VAL("12")` is "12.00" and `? VAL("  12  ")` is nine wide
    let places = c.settings().decimals;
    let width = crate::value::Width::of(s.len().min(200) as u8, places);
    if digits == 0 {
        return ok(Value::Number(0.0, width));
    }
    ok(Value::Number(lit.parse::<f64>().unwrap_or(0.0), width))
}

// ------------------------------------------------------------------------------------------
// TRANSFORM
// ------------------------------------------------------------------------------------------

fn is_placeholder(c: u8) -> bool {
    matches!(c, b'9' | b'#' | b'X' | b'x' | b'A' | b'a' | b'N' | b'n' | b'!' | b'L' | b'l' | b'Y' | b'y' | b'$' | b'*')
}

fn group_thousands(int_part: &str, separator: char) -> String {
    let (sign, digits) = match int_part.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", int_part),
    };
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(separator);
        }
        out.push(ch);
    }
    format!("{sign}{out}")
}

/// A number too big for its picture fills every digit position with an asterisk and keeps the
/// picture's own punctuation, which is how Visual FoxPro says it did not fit.
fn overflowed(picture: &str, settings: &Settings) -> String {
    picture
        .bytes()
        .map(|c| match c {
            c if is_placeholder(c) => '*',
            b',' => settings.separator,
            b'.' => settings.point,
            other => other as char,
        })
        .collect()
}

fn transform_number(n: f64, picture: &str, codes: &str, settings: &Settings) -> String {
    let symbol = || if codes.contains('$') { settings.currency.as_str() } else { "" };
    if picture.is_empty() {
        let body = crate::value::format_number(n, settings);
        return if settings.currency_left {
            format!("{}{body}", symbol())
        } else {
            format!("{body}{}", symbol())
        };
    }
    let (int_pic, dec_pic) = match picture.split_once('.') {
        Some((a, b)) => (a, b),
        None => (picture, ""),
    };
    let dec = dec_pic.bytes().filter(|&c| is_placeholder(c)).count();
    let rounded = super::numeric::round_to(n, dec as i32);
    let body = format!("{:.*}", dec, rounded);
    let (ipart, fpart) = match body.split_once('.') {
        Some((a, b)) => (a.to_string(), format!(".{b}")),
        None => (body.clone(), String::new()),
    };
    // the separator goes in before the point is put where SET POINT says, so that a locale
    // which separates with a full stop does not have its groups read as decimal points
    let ipart = if int_pic.contains(',') { group_thousands(&ipart, settings.separator) } else { ipart };
    let mut s = format!("{ipart}{}", crate::value::with_point(fpart, settings.point));
    let width = picture.len();
    if s.len() > width {
        return overflowed(picture, settings);
    }
    // a `$` written into the picture is a dollar sign of its own; `@$` is the SET CURRENCY
    // symbol, which goes outside the number and may be wider than the picture allows for
    if int_pic.contains('$') {
        s = format!("${s}");
    }
    if !symbol().is_empty() && settings.currency_left {
        s = format!("{}{s}", symbol());
    }
    let shown = s.chars().count();
    if shown < width {
        let missing = width - shown;
        let fill = if codes.contains('L') {
            '0'
        } else if int_pic.contains('*') {
            '*'
        } else {
            ' '
        };
        if codes.contains('B') {
            s = format!("{s}{}", " ".repeat(missing));
        } else if fill == '0' && (s.starts_with('-') || s.starts_with('$')) {
            let head = s.remove(0);
            s = format!("{head}{}{s}", "0".repeat(missing));
        } else {
            s = format!("{}{s}", String::from(fill).repeat(missing));
        }
    } else if shown > width {
        // the symbol pushed it past the picture: what is kept is the right-hand end
        s = s.chars().skip(shown - width).collect();
    }
    if !symbol().is_empty() && !settings.currency_left {
        s = format!("{s}{}", symbol());
    }
    s
}

/// Applies a character picture: placeholders consume a character of the value (`!` upper-cases
/// it), everything else in the picture is a literal.
fn transform_text(value: &str, picture: &str) -> String {
    let src: Vec<u8> = value.bytes().collect();
    let mut i = 0;
    let mut out = Vec::new();
    for p in picture.bytes() {
        if is_placeholder(p) {
            if i < src.len() {
                out.push(if p == b'!' { src[i].to_ascii_uppercase() } else { src[i] });
                i += 1;
            }
        } else {
            out.push(p);
        }
    }
    out.extend_from_slice(&src[i.min(src.len())..]);
    String::from_utf8_lossy(&out).into_owned()
}

/// TRANSFORM(). Supported: no format at all (`display`), the function codes `@!` (upper),
/// `@L` (leading zeros), `@R` (literal template), `@D` (date/datetime with SET DATE), `@T`
/// (trim), `@B` (left-align a number), `@Z` (blanks when zero), `@0` (a number in hexadecimal),
/// and numeric pictures built from `9 # $ * , .`. Anything else falls back to `display` without
/// raising an error.
fn f_transform(ctx: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let v = a[0].deref();
    let settings = ctx.settings().clone();
    let fmt = match a.get(1).map(|f| f.deref()) {
        None | Some(Value::Null) => None,
        Some(Value::Str(s)) => Some(s.to_string()),
        Some(_) => return Err(RtError::function_arg_invalid()),
    };
    ok(Value::str(transform(&v, fmt.as_deref(), &settings)))
}

/// The same, where a value has to be shown as a picture asked for it and there is no built-in
/// call to make: `@ 2, 5 SAY nTotal PICTURE "999,999.99"` goes through here.
pub fn transform(v: &Value, fmt: Option<&str>, settings: &Settings) -> String {
    if v.is_null() {
        return settings.null_display.clone();
    }
    let Some(fmt) = fmt.filter(|f| !f.is_empty()) else {
        return display(v, settings);
    };
    let fmt = fmt.to_string();
    let (codes, picture) = if let Some(rest) = fmt.strip_prefix('@') {
        match rest.split_once(' ') {
            Some((c, p)) => (c.to_ascii_uppercase(), p.to_string()),
            None => (rest.to_ascii_uppercase(), String::new()),
        }
    } else {
        (String::new(), fmt.clone())
    };

    let mut out = match v {
        // `@0` writes a number as the 32-bit integer it truncates to, in hexadecimal. Measured
        // in Visual FoxPro 9: TRANSFORM(255, "@0") is "0x000000FF", TRANSFORM(-1, "@0") is
        // "0xFFFFFFFF", 5000000000 comes out as its low 32 bits, a picture after it is ignored,
        // and Z does not blank a zero. Only a numeric is written this way - a currency, a date
        // and a string are left as they were.
        Value::Number(n, ..) if codes.contains('0') => format!("0x{:08X}", *n as i64 as i32 as u32),
        Value::Number(n, ..) => {
            if codes.contains('Z') && *n == 0.0 {
                " ".repeat(picture.len())
            } else {
                transform_number(*n, &picture, &codes, settings)
            }
        }
        Value::Date(d) if codes.contains('D') => format_date(*d, settings),
        Value::DateTime(t) if codes.contains('D') => format_datetime(*t, settings),
        Value::Str(s) => {
            if picture.is_empty() {
                s.to_string()
            } else {
                transform_text(s, &picture)
            }
        }
        other => display(other, settings),
    };
    if codes.contains('!') {
        out = out.to_ascii_uppercase();
    }
    if codes.contains('T') {
        out = out.trim_matches(' ').to_string();
    }
    out
}

// ------------------------------------------------------------------------------------------
// words, extraction, lines
// ------------------------------------------------------------------------------------------

fn word_delims(a: &[Value], i: usize) -> Result<Vec<u8>, RtError> {
    Ok(match a.get(i) {
        None => b" \t\r\n".to_vec(),
        Some(_) => {
            let d = bytes(a, i)?;
            if d.is_empty() { b" \t\r\n".to_vec() } else { d }
        }
    })
}

fn words(s: &[u8], delims: &[u8]) -> Vec<Vec<u8>> {
    s.split(|c| delims.contains(c)).filter(|w| !w.is_empty()).map(|w| w.to_vec()).collect()
}

fn f_getwordcount(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::number(words(&bytes(&a, 0)?, &word_delims(&a, 1)?).len() as f64))
}

fn f_getwordnum(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let n = arg_int(&a, 1)?;
    let ws = words(&bytes(&a, 0)?, &word_delims(&a, 2)?);
    if n < 1 {
        return ok(Value::str(""));
    }
    ok(ws.get(n as usize - 1).map_or_else(|| Value::str(""), |w| text(w)))
}

/// STREXTRACT(c, begin, end [, occurrence] [, flags]). Flags: 1 case-insensitive delimiters,
/// 2 return the remainder when the end delimiter is missing, 4 include the delimiters in the
/// result - measured: `STREXTRACT("<a>hello", "<a>", "</a>", 1, 2)` is "hello" (bit 1's job) and
/// `STREXTRACT(s, "<a>", "</a>", 1, 4)` keeps the "<a>"..."</a>" wrapper (bit 2's), the reverse
/// of what the two bits had been swapped to do.
fn f_strextract(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let s = bytes(&a, 0)?;
    let begin = bytes(&a, 1)?;
    let end = match a.get(2) {
        None => Vec::new(),
        Some(_) => bytes(&a, 2)?,
    };
    let occurrence = opt_int(&a, 3, 1)?.max(1) as usize;
    let flags = opt_int(&a, 4, 0)?;
    let ci = flags & 1 != 0;
    let rest_on_missing = flags & 2 != 0;
    let keep = flags & 4 != 0;

    let start = if begin.is_empty() {
        if occurrence > 1 {
            return ok(Value::str(""));
        }
        0
    } else {
        match find_all(&s, &begin, ci).get(occurrence - 1) {
            None => return ok(Value::str("")),
            Some(&i) => i + begin.len(),
        }
    };
    let tail = &s[start..];
    let stop = if end.is_empty() { None } else { find_all(tail, &end, ci).first().copied() };
    let body = match stop {
        Some(i) => &tail[..i],
        None if end.is_empty() || rest_on_missing => tail,
        None => return ok(Value::str("")),
    };
    if keep {
        let mut out = begin.clone();
        out.extend_from_slice(body);
        if stop.is_some() {
            out.extend_from_slice(&end);
        }
        return ok(text(&out));
    }
    ok(text(body))
}

/// Splits on the parse characters, treating CRLF/CR/LF as one line break by default.
fn split_lines(s: &[u8], delims: &[Vec<u8>], trim: bool) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < s.len() {
        match delims.iter().find(|d| !d.is_empty() && s[i..].starts_with(d)) {
            Some(d) => {
                out.push(std::mem::take(&mut cur));
                i += d.len();
            }
            None => {
                cur.push(s[i]);
                i += 1;
            }
        }
    }
    out.push(cur);
    // A trailing line break does not open a new line.
    if out.len() > 1 && out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    if trim {
        out = out.into_iter().map(|l| trimmed(&l, b" \t", true, true)).collect();
    }
    out
}

/// ALINES(@array, c [, flags] [, parse chars...]). `flags` may be a logical (trim) or a number
/// whose bit 1 trims each line; any character argument replaces the default CRLF/CR/LF line
/// breaks. Returns the number of lines and redimensions the array in place.
fn f_alines(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let target = a[0].clone();
    let s = bytes(&a, 1)?;
    let mut trim = false;
    let mut delims: Vec<Vec<u8>> = Vec::new();
    for v in a.iter().skip(2) {
        match v.deref() {
            Value::Logical(b) => trim = b,
            Value::Number(n, ..) => trim = (n as i64) & 1 != 0,
            Value::Str(t) => delims.extend(t.bytes().map(|b| vec![b])),
            _ => {}
        }
    }
    if delims.is_empty() {
        delims = vec![b"\r\n".to_vec(), b"\n".to_vec(), b"\r".to_vec()];
    }
    let lines = split_lines(&s, &delims, trim);
    let mut arr = FoxArray::new(lines.len().max(1), 0);
    for (i, l) in lines.iter().enumerate() {
        arr.items[i] = text(l);
    }
    match &target {
        Value::Array(cell) => *cell.borrow_mut() = arr,
        Value::Ref(cell) => {
            let existing = cell.borrow().deref();
            match existing {
                Value::Array(inner) => *inner.borrow_mut() = arr,
                _ => *cell.borrow_mut() = Value::Array(std::rc::Rc::new(std::cell::RefCell::new(arr))),
            }
        }
        _ => return Err(RtError::function_arg_invalid()),
    }
    // eleven characters wide, which is what the product gives the count - measured
    ok(Value::Number(lines.len() as f64, crate::value::Width { chars: 11, decimals: 0, written: false }))
}

// ------------------------------------------------------------------------------------------
// double-byte variants
//
// Visual FoxPro's C functions count characters where the plain ones count bytes, which matters
// only on a double-byte code page. Every string here is single-byte, so each C function is its
// plain counterpart - named, so a program written for a Japanese build still runs.
// ------------------------------------------------------------------------------------------

fn f_lenc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_len(c, a)
}

fn f_substrc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_substr(c, a)
}

fn f_leftc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_left(c, a)
}

fn f_rightc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_right(c, a)
}

fn f_stuffc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_stuff(c, a)
}

fn f_chrtranc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_chrtran(c, a)
}

fn f_at_c(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_at(c, a)
}

fn f_atcc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_atc(c, a)
}

// ------------------------------------------------------------------------------------------
// memo lines
//
// A memo has no lines of its own as far as these are concerned: Visual FoxPro wraps it at
// SET MEMOWIDTH, so the line a character sits on depends on that setting. The wrap is at a word
// boundary where there is one, which is what makes MLINE() give back what a memo window shows.
// ------------------------------------------------------------------------------------------

fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    // CR/LF together are one hard break, not two: splitting on either character on its own
    // turns every "\r\n" into a break plus a spurious empty line between them - measured,
    // `MLINE` of a memo built with `CHR(13) + CHR(10)` between lines counts exactly the lines
    // that were joined that way, not twice as many.
    let normalized = text.replace("\r\n", "\n");
    for hard in normalized.split(['\r', '\n']) {
        if hard.len() <= width {
            out.push(hard.to_string());
            continue;
        }
        let mut rest = hard;
        while rest.len() > width {
            // the break is the last space that fits; a word longer than the line is cut
            let cut = rest[..=width].rfind(' ').filter(|i| *i > 0).unwrap_or(width);
            out.push(rest[..cut].to_string());
            rest = rest[cut..].trim_start_matches(' ');
        }
        out.push(rest.to_string());
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn memo_lines(c: &mut dyn BuiltinCtx, a: &[Value], i: usize) -> Result<Vec<String>, RtError> {
    let width = c.settings().memowidth as usize;
    Ok(wrap_lines(&arg_str(a, i)?, width))
}

/// MEMLINES(memo): how many lines it takes at the current SET MEMOWIDTH.
fn f_memlines(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let lines = memo_lines(c, &a, 0)?;
    ok(Value::number(lines.len() as f64))
}

/// MLINE(memo, n [, nCharacterOffset]): one line of it, counting from 1.
///
/// With a third argument, `n` stops being simply which line to return: measured against
/// vfp9.exe, `nCharacterOffset` counts from the start of the whole memo, in a numbering where
/// each line's own break - however many bytes CR/LF really is - counts as exactly one slot.
/// `MLINE(memo, row, off)` starts at `row`, walks forward a line at a time for as long as
/// `off` has moved past the line it is standing on, and returns what is left of the line it
/// lands on from the local offset onward (a local offset before that line's own start, from a
/// `row` later than where `off` would have landed, returns the line whole; one past its end
/// returns ""). Two lines never covered by the docs and only found this way: an `off` that
/// overruns `row`'s own line does not fail or spill mid-line into the next one - it hands back
/// the *entire* next line - and the walk never goes backward, so a `row` later than where `off`
/// points also just hands back that later line whole.
fn f_mline(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let lines = memo_lines(c, &a, 0)?;
    let n = arg_int(&a, 1)?;
    if n < 1 || n as usize > lines.len() {
        return ok(Value::str(""));
    }
    let Some(_) = a.get(2) else {
        return ok(Value::str(lines[n as usize - 1].clone()));
    };
    let skip = arg_int(&a, 2)?;
    let mut starts = Vec::with_capacity(lines.len());
    let mut at = 0i64;
    for l in &lines {
        starts.push(at);
        at += l.len() as i64 + 1;
    }
    let mut idx = n as usize - 1;
    while idx + 1 < lines.len() && skip >= starts[idx + 1] {
        idx += 1;
    }
    let local = (skip - starts[idx]).max(0) as usize;
    let line = lines[idx].as_bytes();
    ok(Value::str(if local >= line.len() {
        String::new()
    } else {
        String::from_utf8_lossy(&line[local..]).into_owned()
    }))
}

/// ATLINE and ATCLINE: the number of the line a match is on, or 0.
fn line_of(c: &mut dyn BuiltinCtx, a: &[Value], fold: bool) -> Result<BuiltinResult, RtError> {
    let needle = arg_str(a, 0)?.to_string();
    let needle = if fold { needle.to_lowercase() } else { needle };
    for (i, line) in memo_lines(c, a, 1)?.iter().enumerate() {
        let hay = if fold { line.to_lowercase() } else { line.clone() };
        if hay.contains(&needle) {
            return ok(Value::number(i as f64 + 1.0));
        }
    }
    ok(Value::number(0.0))
}

fn f_atline(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    line_of(c, &a, false)
}

fn f_atcline(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    line_of(c, &a, true)
}

/// RATLINE: the same, counting from the end, so the last line a match is on.
fn f_ratline(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let needle = arg_str(&a, 0)?.to_string();
    let lines = memo_lines(c, &a, 1)?;
    for (i, line) in lines.iter().enumerate().rev() {
        if line.contains(&needle) {
            return ok(Value::number(i as f64 + 1.0));
        }
    }
    ok(Value::number(0.0))
}

// ------------------------------------------------------------------------------------------
// phonetics and patterns
// ------------------------------------------------------------------------------------------

/// SOUNDEX: the letter a word starts with and three digits for the consonants after it, which
/// is what makes two spellings of a name compare equal.
fn soundex(text: &str) -> String {
    let code = |c: char| match c.to_ascii_uppercase() {
        'B' | 'F' | 'P' | 'V' => Some(b'1'),
        'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => Some(b'2'),
        'D' | 'T' => Some(b'3'),
        'L' => Some(b'4'),
        'M' | 'N' => Some(b'5'),
        'R' => Some(b'6'),
        _ => None,
    };
    let letters: Vec<char> = text.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    let Some(&first) = letters.first() else { return "0000".to_string() };

    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    let mut last = code(first);
    for &c in letters.iter().skip(1) {
        let digit = code(c);
        if let Some(d) = digit
            && digit != last
        {
            out.push(d as char);
            if out.len() == 4 {
                return out;
            }
        }
        // H and W do not break a run of the same code; a vowel does
        if !matches!(c.to_ascii_uppercase(), 'H' | 'W') {
            last = digit;
        }
    }
    while out.len() < 4 {
        out.push('0');
    }
    out
}

fn f_soundex(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let settings = c.settings().clone();
    ok(Value::str(soundex(&as_text(&a[0], &settings))))
}

/// DIFFERENCE: how alike two words sound, 0 to 4, by comparing their SOUNDEX codes position by
/// position.
fn f_difference(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let settings = c.settings().clone();
    let left = soundex(&as_text(&a[0], &settings));
    let right = soundex(&as_text(&a[1], &settings));
    let same = left.chars().zip(right.chars()).filter(|(x, y)| x == y).count();
    ok(Value::number(same as f64))
}

/// LIKE(pattern, text): a star stands for any run of characters and a question mark for one,
/// and the comparison is case-sensitive, as VFP's is.
pub fn matches_pattern(pattern: &[u8], text: &[u8]) -> bool {
    let (mut p, mut t) = (0usize, 0usize);
    let (mut star, mut mark) = (usize::MAX, 0usize);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = p;
            mark = t;
            p += 1;
        } else if star != usize::MAX {
            // the last star swallows one more character and the match is tried again
            p = star + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&c| c == b'*')
}

fn f_like(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(matches_pattern(&bytes(&a, 0)?, &bytes(&a, 1)?)))
}

// ------------------------------------------------------------------------------------------
// STRCONV
// ------------------------------------------------------------------------------------------

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - i * 6)) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn base64_decode(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let digits: Vec<u8> = text.bytes().filter_map(value).collect();
    let mut out = Vec::with_capacity(digits.len() * 3 / 4);
    for chunk in digits.chunks(4) {
        let mut n = 0u32;
        for (i, d) in chunk.iter().enumerate() {
            n |= u32::from(*d) << (18 - i * 6);
        }
        for i in 0..chunk.len().saturating_sub(1) {
            out.push(((n >> (16 - i * 8)) & 0xff) as u8);
        }
    }
    out
}

/// Bytes as the string that stands for them: one character per byte, which is how bytes travel
/// everywhere else in this runtime.
fn as_byte_string(bytes: &[u8]) -> Value {
    Value::str(bytes.iter().map(|&b| b as char).collect::<String>())
}

fn byte_values(text: &str) -> Vec<u8> {
    text.chars().map(|c| (c as u32 & 0xff) as u8).collect()
}

fn from_utf16le(raw: &[u8]) -> String {
    let wide: Vec<u16> = raw.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
    String::from_utf16_lossy(&wide)
}

fn to_utf16le(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// STRCONV(text, n): the conversions of the reference, by number.
///
/// 1 to 4 convert between single-byte and double-byte characters and between the Japanese
/// syllabaries. There are no double-byte characters in a single-byte code page, so those hand
/// the string back, which is what Visual FoxPro does on one too.
fn f_strconv(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let settings = c.settings().clone();
    let text = as_text(&a[0], &settings);
    let how = arg_int(&a, 1)?;
    Ok(BuiltinResult::Value(match how {
        1..=4 => Value::str(text),
        // Unicode here is UTF-16, little-endian, as Windows writes it
        5 => as_byte_string(&to_utf16le(&text)),
        6 => Value::str(from_utf16le(&byte_values(&text))),
        7 => Value::str(text.to_lowercase()),
        8 => Value::str(text.to_uppercase()),
        9 => as_byte_string(text.as_bytes()),
        10 => as_byte_string(from_utf16le(&byte_values(&text)).as_bytes()),
        11 => Value::str(String::from_utf8_lossy(&byte_values(&text)).to_string()),
        12 => as_byte_string(&to_utf16le(&String::from_utf8_lossy(&byte_values(&text)))),
        13 => Value::str(base64_encode(&byte_values(&text))),
        14 => as_byte_string(&base64_decode(&text)),
        15 => Value::str(byte_values(&text).iter().map(|b| format!("{b:02x}")).collect::<String>()),
        16 => {
            let digits: Vec<u8> = text.bytes().filter(u8::is_ascii_hexdigit).collect();
            let raw: Vec<u8> = digits
                .chunks_exact(2)
                .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap_or("0"), 16).unwrap_or(0))
                .collect();
            as_byte_string(&raw)
        }
        _ => return Err(RtError::function_arg_invalid()),
    }))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("ALINES", 2, VARIADIC, f_alines),
        spec("ALLTRIM", 1, VARIADIC, f_alltrim),
        spec("ASC", 1, 1, f_asc),
        spec("AT", 2, 3, f_at),
        spec("ATC", 2, 3, f_atc),
        spec("ATCC", 2, 3, f_atcc),
        spec("ATCLINE", 2, 2, f_atcline),
        spec("ATLINE", 2, 2, f_atline),
        spec("AT_C", 2, 3, f_at_c),
        spec("BINTOC", 1, 2, f_bintoc),
        spec("CHR", 1, 1, f_chr),
        spec("CHRTRAN", 3, 3, f_chrtran),
        spec("CHRTRANC", 3, 3, f_chrtranc),
        spec("CREATEBINARY", 1, 1, f_createbinary),
        spec("CTOBIN", 1, 2, f_ctobin),
        spec("DIFFERENCE", 2, 2, f_difference),
        spec("GETWORDCOUNT", 1, 2, f_getwordcount),
        spec("GETWORDNUM", 2, 3, f_getwordnum),
        spec("ICASE", 1, VARIADIC, f_icase),
        spec("LEFT", 2, 2, f_left),
        spec("LEFTC", 2, 2, f_leftc),
        spec("LEN", 1, 1, f_len),
        spec("LENC", 1, 1, f_lenc),
        spec("LIKE", 2, 2, f_like),
        spec("LIKEC", 2, 2, f_likec),
        spec("LOWER", 1, 1, f_lower),
        spec("LTRIM", 1, VARIADIC, f_ltrim),
        spec("MEMLINES", 1, 1, f_memlines),
        spec("MLINE", 2, 3, f_mline),
        spec("NORMALIZE", 1, 1, f_normalize),
        spec("OCCURS", 2, 2, f_occurs),
        spec("PADC", 2, 3, f_padc),
        spec("PADL", 2, 3, f_padl),
        spec("PADR", 2, 3, f_padr),
        spec("PROPER", 1, 1, f_proper),
        spec("RAT", 2, 3, f_rat),
        spec("RATC", 2, 3, f_ratc),
        spec("RATLINE", 2, 2, f_ratline),
        spec("REPLICATE", 2, 2, f_replicate),
        spec("RIGHT", 2, 2, f_right),
        spec("RIGHTC", 2, 2, f_rightc),
        spec("RTRIM", 1, VARIADIC, f_rtrim),
        spec("SOUNDEX", 1, 1, f_soundex),
        spec("SPACE", 1, 1, f_space),
        spec("STR", 1, 3, f_str),
        spec("STRCONV", 2, 4, f_strconv),
        spec("STREXTRACT", 2, 5, f_strextract),
        spec("STRTRAN", 2, 6, f_strtran),
        spec("STUFF", 4, 4, f_stuff),
        spec("STUFFC", 4, 4, f_stuffc),
        spec("SUBSTR", 2, 3, f_substr),
        spec("SUBSTRC", 2, 3, f_substrc),
        spec("TEXTMERGE", 1, 4, f_textmerge),
        spec("TRANSFORM", 1, 2, f_transform),
        spec("TRIM", 1, VARIADIC, f_rtrim),
        spec("UPPER", 1, 1, f_upper),
        spec("VAL", 1, 1, f_val),
    ]
}

// ------------------------------------------------------------------------------------------
// binary strings, conditions and merged text
// ------------------------------------------------------------------------------------------

/// The bytes a string stands for, one per character, the way CHR() and ASC() count them.
///
/// The rest of this module works on the UTF-8 bytes of the text, which is right for the
/// functions that count characters; a binary string is not text, and a byte of it is a
/// character of its own whatever its value.
fn raw_bytes(v: &Value) -> Vec<u8> {
    crate::data::bytes_of(v)
}

fn raw_text(b: &[u8]) -> Value {
    Value::str(b.iter().map(|c| *c as char).collect::<String>())
}

/// How many bytes a BINTOC or CTOBIN flag asks for, and what kind of number it is.
///
/// The flags are read off the files Visual FoxPro writes: `1`, `2` and `4` are whole numbers of
/// that many bytes, `8` and `B` are a double, `F` is a four-byte float, `R` reverses the bytes
/// and `S` leaves the sign bit alone.
struct BinFlags {
    size: usize,
    /// The bytes are those of a floating-point number rather than a whole one.
    real: bool,
    reversed: bool,
    /// The top bit is turned over, so that comparing the strings byte by byte orders the
    /// numbers they stand for. Visual FoxPro does this unless the flags say `S`.
    signed: bool,
    /// The size came from a string of letters rather than from a number, which is the form the
    /// product refuses eight bytes in.
    lettered: bool,
}

fn bin_flags(args: &[Value], i: usize, default: usize) -> Result<BinFlags, RtError> {
    let mut flags = BinFlags { size: default, real: default == 8, reversed: false, signed: true, lettered: false };
    let Some(value) = args.get(i) else { return Ok(flags) };
    let written = match value.deref() {
        Value::Number(n, ..) => format!("{}", n as i64),
        other => {
            flags.lettered = true;
            other.as_str().map(|s| s.to_string()).unwrap_or_default()
        }
    };
    for c in written.chars() {
        match c.to_ascii_uppercase() {
            '1' => flags = BinFlags { size: 1, real: false, ..flags },
            '2' => flags = BinFlags { size: 2, real: false, ..flags },
            '4' => flags = BinFlags { size: 4, real: false, ..flags },
            '8' | 'B' | 'Y' | 'N' => flags = BinFlags { size: 8, real: c.to_ascii_uppercase() != 'Y', ..flags },
            'F' => flags = BinFlags { size: 4, real: true, ..flags },
            'R' => flags.reversed = true,
            'S' => flags.signed = false,
            _ => {}
        }
    }
    Ok(flags)
}

fn f_bintoc(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let n = arg_num(&a, 0)?;
    let flags = bin_flags(&a, 1, 4)?;
    // One, two or four bytes when the flags are written as letters: the product refuses
    // `BINTOC(n, "8RS")` with error 11 - measured - where `BINTOC(n, 8)`, the size given as a
    // number, is the double it has always been.
    if flags.size == 8 && flags.lettered {
        return Err(RtError::new(RtError::FUNCTION_ARG_INVALID, "Function argument value, type, or count is invalid."));
    }
    let mut out: Vec<u8> = match (flags.real, flags.size) {
        (true, 4) => (n as f32).to_be_bytes().to_vec(),
        (true, _) => n.to_be_bytes().to_vec(),
        (false, size) => (n as i64).to_be_bytes()[8 - size..].to_vec(),
    };
    if flags.signed && !out.is_empty() {
        out[0] ^= 0x80;
    }
    if flags.reversed {
        out.reverse();
    }
    ok(raw_text(&out))
}

fn f_ctobin(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let mut given = raw_bytes(&a[0]);
    let flags = bin_flags(&a, 1, given.len())?;
    if flags.reversed {
        given.reverse();
    }
    if flags.signed && !given.is_empty() {
        given[0] ^= 0x80;
    }
    // Eight bytes is a double: `CTOBIN(c, "8RS")` of the eight bytes of 1000000000 as a
    // little-endian whole number answers 7.5766328932124E-305, which is those bytes read as one -
    // measured. There is no 64-bit whole number in this language, which is why a program reading
    // a file size out of a Windows structure reads it as two four-byte halves.
    let value = match (flags.real, given.len()) {
        (true, 4) => f64::from(f32::from_be_bytes(given[..4].try_into().unwrap_or_default())),
        (_, 8) => f64::from_be_bytes(given[..8].try_into().unwrap_or_default()),
        (true, _) if given.len() >= 8 => f64::from_be_bytes(given[..8].try_into().unwrap_or_default()),
        // a whole number keeps its sign, which is the top bit of the first byte it was given
        (_, len) if len > 0 => {
            let negative = given[0] & 0x80 != 0;
            let mut wide = [if negative { 0xFF } else { 0 }; 8];
            wide[8 - len.min(8)..].copy_from_slice(&given[given.len() - len.min(8)..]);
            i64::from_be_bytes(wide) as f64
        }
        _ => 0.0,
    };
    // The answer carries the product's width, not one worked out from the value: ten characters
    // for a whole number, twenty for the double eight bytes make. A value too big or too small
    // for that room prints with an exponent - measured: reading four bytes of 0x01000000 the
    // wrong way round answers -2.1307E+9 rather than the eleven digits it would take.
    // A whole number answers in ten characters, which is why reading four bytes the wrong way
    // round shows as -2.1307E+9 rather than as eleven digits - measured. The double eight bytes
    // make is written out in full instead, and how wide the product makes that one is still
    // open: docs/parity-plan.md has what it answered.
    let width = if given.len() == 8 {
        return ok(Value::number(value));
    } else {
        crate::value::Width { chars: 10, decimals: 0, written: false }
    };
    ok(Value::Number(value, width))
}

fn f_createbinary(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    // a string here already holds bytes, so marking one as binary changes nothing about it
    ok(a[0].deref())
}

fn f_likec(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_like(c, a)
}

fn f_icase(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // pairs of a condition and what it gives; an odd one left over at the end is the answer
    // when none of the conditions held
    let mut i = 0;
    while i + 1 < a.len() {
        if a[i].deref().truthy().unwrap_or(false) {
            return ok(a[i + 1].deref());
        }
        i += 2;
    }
    ok(a.get(i).map(Value::deref).unwrap_or(Value::Null))
}

fn f_textmerge(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let mut text = arg_str(&a, 0)?.to_string();
    let recursive = a.get(1).map(|v| v.deref().truthy().unwrap_or(false)).unwrap_or(false);
    let open = match a.get(2) {
        Some(v) => v.deref().as_str()?.to_string(),
        None => "<<".to_string(),
    };
    let close = match a.get(3) {
        Some(v) => v.deref().as_str()?.to_string(),
        None => ">>".to_string(),
    };
    // a merge that puts delimiters back in is only followed through when the call asked for it
    for _ in 0..if recursive { 32 } else { 1 } {
        let (merged, any) = merge_once(c, &text, &open, &close)?;
        text = merged;
        if !any {
            break;
        }
    }
    ok(Value::str(text))
}

/// One pass of text merge over `text`, which is what `\`, `\\` and a TEXT block do to what is
/// written in them as well as what TEXTMERGE() does to its argument.
pub(crate) fn merge(c: &mut dyn BuiltinCtx, text: &str, open: &str, close: &str) -> Result<String, RtError> {
    Ok(merge_once(c, text, open, close)?.0)
}

/// One pass over the text, working out every `<<expression>>` it finds.
fn merge_once(c: &mut dyn BuiltinCtx, text: &str, open: &str, close: &str) -> Result<(String, bool), RtError> {
    let mut out = String::new();
    let mut rest = text;
    let mut any = false;
    while let Some(at) = rest.find(open) {
        let after = &rest[at + open.len()..];
        let Some(end) = after.find(close) else { break };
        out.push_str(&rest[..at]);
        let settings = c.settings().clone();
        let value = c.evaluate(&after[..end])?;
        out.push_str(&as_text(&value, &settings));
        rest = &after[end + close.len()..];
        any = true;
    }
    out.push_str(rest);
    Ok((out, any))
}

fn f_normalize(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::str(crate::lexer::normalize(&arg_str(&a, 0)?)))
}
