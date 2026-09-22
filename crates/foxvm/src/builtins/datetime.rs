//! Date and datetime functions. "Now" always comes from the host clock, never from the system,
//! so programs are reproducible under test and in the browser.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, any_null, arg_int, arg_str, ok, opt_int, spec};
use crate::error::RtError;
use crate::value::{
    DAY_NAMES, MONTH_NAMES, Settings, Value, civil_from_days, day_of_week, days_from_civil, format_date,
    format_datetime, format_time, is_valid_date,
};

/// Days for a Date or the date part of a DateTime.
fn as_days(v: &Value) -> Result<Option<i32>, RtError> {
    match v.deref() {
        Value::Date(d) => Ok(d),
        Value::DateTime(None) => Ok(None),
        Value::DateTime(Some(t)) => Ok(Some(t.div_euclid(86400.0) as i32)),
        _ => Err(RtError::function_arg_invalid()),
    }
}

/// Seconds since midnight for a DateTime; a Date has none.
fn as_time_of_day(v: &Value) -> Result<Option<f64>, RtError> {
    match v.deref() {
        Value::DateTime(Some(t)) => Ok(Some(t.rem_euclid(86400.0))),
        Value::DateTime(None) | Value::Date(_) => Ok(None),
        _ => Err(RtError::function_arg_invalid()),
    }
}

fn hms(secs: f64) -> (u32, u32, u32) {
    let s = secs.rem_euclid(86400.0).floor() as u32;
    (s / 3600, (s % 3600) / 60, s % 60)
}

fn make_datetime(days: i32, h: f64, mi: f64, s: f64) -> Value {
    Value::DateTime(Some(days as f64 * 86400.0 + h * 3600.0 + mi * 60.0 + s))
}

// ------------------------------------------------------------------------------------------
// clock and constructors
// ------------------------------------------------------------------------------------------

/// nYear, nMonth and nDay each have a documented range (100-9999, 1-12, 1-31) that DATE() and
/// DATETIME() enforce structurally - a value outside it is error 11. A day that is in range for
/// the type but does not exist in that particular month (`DATE(2024, 2, 30)`) is not one of
/// these: measured, it is not an error at all, it is the empty date, so it is left for
/// `is_valid_date` to catch separately once these bounds have already passed.
fn ymd_in_range(y: i64, m: i64, d: i64) -> bool {
    (100..=9999).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
}

/// DATE() and the VFP constructor form DATE(year, month, day).
fn f_date(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a.is_empty() {
        return ok(Value::Date(Some(c.host().now().0)));
    }
    if any_null(&a) {
        return ok(Value::Null);
    }
    // One or two arguments is not a shorter constructor - measured, `DATE(2024)` is error 1229
    // ("Too few arguments"), the same complaint as no arguments to QUARTER() or ALANGUAGE(),
    // and a different one from the range checks below once all three are actually given.
    if a.len() < 3 {
        return Err(RtError::too_few_args());
    }
    let (y, m, d) = (arg_int(&a, 0)?, arg_int(&a, 1)?, arg_int(&a, 2)?);
    if !ymd_in_range(y, m, d) {
        return Err(RtError::function_arg_invalid());
    }
    let (y, m, d) = (y as i32, m as u32, d as u32);
    if !is_valid_date(y, m, d) {
        return ok(Value::Date(None));
    }
    ok(Value::Date(Some(days_from_civil(y, m, d))))
}

/// DATETIME() and DATETIME(year, month, day [, hour, minute, second]).
fn f_datetime(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a.is_empty() {
        let (days, secs) = c.host().now();
        return ok(Value::DateTime(Some(days as f64 * 86400.0 + secs.floor())));
    }
    if any_null(&a) {
        return ok(Value::Null);
    }
    // Same as DATE(): one or two arguments is error 1229 ("Too few arguments"), not the same
    // error a bad year, month or day gives once all three are there.
    if a.len() < 3 {
        return Err(RtError::too_few_args());
    }
    let (y, m, d) = (arg_int(&a, 0)?, arg_int(&a, 1)?, arg_int(&a, 2)?);
    if !ymd_in_range(y, m, d) {
        return Err(RtError::function_arg_invalid());
    }
    let (y, m, d) = (y as i32, m as u32, d as u32);
    let h = opt_int(&a, 3, 0)? as f64;
    let mi = opt_int(&a, 4, 0)? as f64;
    let s = opt_int(&a, 5, 0)? as f64;
    if !(0.0..24.0).contains(&h) || !(0.0..60.0).contains(&mi) || !(0.0..60.0).contains(&s) {
        return Err(RtError::function_arg_invalid());
    }
    if !is_valid_date(y, m, d) {
        return ok(Value::DateTime(None));
    }
    ok(make_datetime(days_from_civil(y, m, d), h, mi, s))
}

/// TIME(): "HH:MM:SS", or "HH:MM:SS.hh" when any argument is given.
fn f_time(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let secs = c.host().now().1;
    let (h, mi, s) = hms(secs);
    if a.is_empty() {
        return ok(Value::str(format!("{h:02}:{mi:02}:{s:02}")));
    }
    let frac = (secs.rem_euclid(1.0) * 100.0).floor() as u32;
    ok(Value::str(format!("{h:02}:{mi:02}:{s:02}.{frac:02}")))
}

fn f_seconds(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(c.host().now().1))
}

// ------------------------------------------------------------------------------------------
// extraction
// ------------------------------------------------------------------------------------------

/// One piece of a date, in the width Visual FoxPro gives that piece: a year prints five wide, a
/// month or a day three. Measured against vfp9.exe.
fn part(a: &[Value], chars: u8, f: impl FnOnce(i32, u32, u32) -> f64) -> Result<BuiltinResult, RtError> {
    let width = crate::value::Width { chars, decimals: 0, written: false };
    if any_null(a) {
        return ok(Value::Null);
    }
    match as_days(&a[0])? {
        None => ok(Value::Number(0.0, width)),
        Some(d) => {
            let (y, m, day) = civil_from_days(d);
            ok(Value::Number(f(y, m, day), width))
        }
    }
}

fn f_year(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    part(&a, 5, |y, _, _| y as f64)
}

/// DMY() and MDY(): a date written out the way a letter would write it, the day always in two
/// digits. SET CENTURY decides whether the year is written in full.
fn written_date(c: &mut dyn BuiltinCtx, a: &[Value], day_first: bool) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let Some(days) = as_days(&a[0])? else { return ok(Value::str("")) };
    let (y, m, d) = crate::value::civil_from_days(days);
    let month = MONTH_NAMES[(m as usize).clamp(1, 12) - 1];
    let year = if c.settings().century { format!("{y:04}") } else { format!("{:02}", y % 100) };
    ok(Value::str(if day_first { format!("{d:02} {month} {year}") } else { format!("{month} {d:02}, {year}") }))
}

fn f_dmy(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    written_date(c, &a, true)
}

fn f_mdy(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    written_date(c, &a, false)
}

/// QUARTER(dExpression | tExpression [, nMonth]): which quarter of the year a date falls in.
///
/// The reference page's syntax line never brackets the date argument, and measured, the
/// product means it: `QUARTER()` with nothing at all is error 1229 ("Too few arguments"), not
/// today's quarter the way the registry's lenient 0-1 arity would otherwise default it to.
fn f_quarter(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(first) = a.first() else { return Err(RtError::too_few_args()) };
    let Some(days) = as_days(first)? else { return ok(Value::number(0.0)) };
    let (_, m, _) = crate::value::civil_from_days(days);
    ok(Value::number(f64::from((m - 1) / 3 + 1)))
}

fn f_month(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    part(&a, 3, |_, m, _| m as f64)
}

fn f_day(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    part(&a, 3, |_, _, d| d as f64)
}

/// The hour, minute or second of a time, three wide as Visual FoxPro prints them.
fn time_part(a: &[Value], which: usize) -> Result<BuiltinResult, RtError> {
    if any_null(a) {
        return ok(Value::Null);
    }
    let (h, mi, s) = as_time_of_day(&a[0])?.map_or((0, 0, 0), hms);
    let width = crate::value::Width { chars: 3, decimals: 0, written: false };
    ok(Value::Number([h, mi, s][which] as f64, width))
}

fn f_hour(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    time_part(&a, 0)
}

fn f_minute(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    time_part(&a, 1)
}

fn f_sec(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    time_part(&a, 2)
}

/// Which day a week starts on, and what the first week of a year is. The argument names the
/// day or the rule; 0 is the only value that asks `SET FDOW` / `SET FWEEK`, and leaving the
/// argument out is Sunday and the week holding January 1 whatever the settings say.
fn week_rule(a: &[Value], i: usize, from_setting: u8, high: i64) -> Result<i64, RtError> {
    match opt_int(a, i, 1)? {
        0 => Ok(from_setting as i64),
        n if (1..=high).contains(&n) => Ok(n),
        _ => Err(RtError::illegal_value()),
    }
}

/// DOW(d [, nFirstDay]): 1..7, Sunday first unless `nFirstDay` says otherwise.
fn f_dow(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let first = week_rule(&a, 1, c.settings().fdow, 7)?;
    // one digit and room for a sign, which is the width Visual FoxPro gives a day of the week
    let width = crate::value::Width { chars: 2, decimals: 0, written: false };
    let Some(d) = as_days(&a[0])? else {
        return ok(Value::Number(0.0, width));
    };
    let dow = day_of_week(d) as i64;
    ok(Value::Number((((dow - (first - 1)).rem_euclid(7)) + 1) as f64, width))
}

fn f_cdow(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    match as_days(&a[0])? {
        None => ok(Value::str("")),
        Some(d) => ok(Value::str(DAY_NAMES[day_of_week(d) as usize])),
    }
}

fn f_cmonth(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    match as_days(&a[0])? {
        None => ok(Value::str("")),
        Some(d) => ok(Value::str(MONTH_NAMES[(civil_from_days(d).1 - 1) as usize])),
    }
}

// ------------------------------------------------------------------------------------------
// conversion
// ------------------------------------------------------------------------------------------

fn ymd8(days: i32) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}{m:02}{d:02}")
}

/// DTOC(d [, 1]): SET DATE / SET CENTURY text, or "YYYYMMDD" with a second argument.
///
/// The second argument switches the answer by being *there*, not by what it is: measured,
/// `DTOC(d, 0)` gives the same unformatted "YYYYMMDD" as `DTOC(d, 1)` does, and so does any
/// other value - only leaving the argument out entirely goes back to the SET DATE-formatted
/// text.
fn f_dtoc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let days = as_days(&a[0])?;
    if a.get(1).is_some() {
        return ok(Value::str(days.map_or_else(|| "        ".to_string(), ymd8)));
    }
    let settings = c.settings().clone();
    ok(Value::str(format_date(days, &settings)))
}

fn f_dtos(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::str(as_days(&a[0])?.map_or_else(|| "        ".to_string(), ymd8)))
}

/// TTOC(t [, 1]): SET DATE text, or "YYYYMMDDHHMMSS" with the flag. Flag 2 gives just the time.
fn f_ttoc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let flag = opt_int(&a, 1, 0)?;
    let days = as_days(&a[0])?;
    let tod = as_time_of_day(&a[0])?.unwrap_or(0.0);
    let (h, mi, s) = hms(tod);
    match flag {
        1 => ok(Value::str(match days {
            None => "              ".to_string(),
            Some(d) => format!("{}{h:02}{mi:02}{s:02}", ymd8(d)),
        })),
        // the time on its own still goes by SET HOURS and SET SECONDS
        2 => ok(Value::str(format_time(tod, c.settings(), true))),
        _ => {
            let settings = c.settings().clone();
            let secs = match a[0].deref() {
                Value::DateTime(t) => t,
                Value::Date(d) => d.map(|d| d as f64 * 86400.0),
                _ => return Err(RtError::function_arg_invalid()),
            };
            ok(Value::str(format_datetime(secs, &settings)))
        }
    }
}

fn f_ttod(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(Value::Date(as_days(&a[0])?))
}

fn f_dtot(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    ok(match as_days(&a[0])? {
        None => Value::DateTime(None),
        Some(d) => make_datetime(d, 0.0, 0.0, 0.0),
    })
}

/// The year a date written in `digits` digits means. `SET CENTURY TO ... ROLLOVER` says which
/// century a two-digit one falls in; the Taiwan format counts its years from 1912 instead, and
/// only a four-digit one there is a year of the Christian era.
fn expand_year(text: &str, n: i32, settings: &Settings, today_year: i32) -> i32 {
    if settings.date_format.style().roc_year {
        return if text.len() > 3 { n } else { n + 1911 };
    }
    settings.full_year(n, text.len(), today_year)
}

/// Parses the date part of `s` for the current SET DATE; returns the days and the rest.
fn parse_date_text(s: &str, settings: &Settings, today_year: i32) -> (Option<i32>, String) {
    let mut groups: Vec<String> = Vec::new();
    let mut rest_at = s.len();
    let mut cur = String::new();
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() {
            cur.push(ch);
            continue;
        }
        if !cur.is_empty() {
            groups.push(std::mem::take(&mut cur));
            if groups.len() == 3 {
                rest_at = i;
                break;
            }
        }
        if !matches!(ch, ' ' | '/' | '-' | '.' | ',') {
            rest_at = i;
            break;
        }
    }
    if !cur.is_empty() && groups.len() < 3 {
        groups.push(cur);
        rest_at = s.len();
    }
    if groups.len() < 3 {
        return (None, String::new());
    }
    let mut y = 0;
    let mut m = 0;
    let mut d = 0;
    for (slot, g) in settings.date_format.style().order.iter().zip(groups.iter()) {
        let n: i32 = g.parse().unwrap_or(-1);
        match slot {
            b'y' => y = expand_year(g, n, settings, today_year),
            b'm' => m = n,
            _ => d = n,
        }
    }
    if m < 0 || d < 0 || y < 0 || !is_valid_date(y, m as u32, d as u32) {
        return (None, s[rest_at.min(s.len())..].to_string());
    }
    (Some(days_from_civil(y, m as u32, d as u32)), s[rest_at.min(s.len())..].to_string())
}

fn f_ctod(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let today = civil_from_days(c.host().now().0).0;
    let settings = c.settings().clone();
    let s = arg_str(&a, 0)?;
    ok(Value::Date(parse_date_text(&s, &settings, today).0))
}

/// CTOT(): the date in the current SET DATE followed by an optional "HH:MM:SS" and AM/PM.
fn f_ctot(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let today = civil_from_days(c.host().now().0).0;
    let settings = c.settings().clone();
    let s = arg_str(&a, 0)?;
    let (days, rest) = parse_date_text(&s, &settings, today);
    let Some(days) = days else {
        return ok(Value::DateTime(None));
    };
    let rest = rest.trim();
    let upper = rest.to_ascii_uppercase();
    let pm = upper.contains("PM");
    let am = upper.contains("AM");
    let nums: Vec<f64> = upper
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();
    let mut h = nums.first().copied().unwrap_or(0.0);
    let mi = nums.get(1).copied().unwrap_or(0.0);
    let sec = nums.get(2).copied().unwrap_or(0.0);
    if pm && h < 12.0 {
        h += 12.0;
    }
    if am && h == 12.0 {
        h = 0.0;
    }
    if h >= 24.0 || mi >= 60.0 || sec >= 60.0 {
        return ok(make_datetime(days, 0.0, 0.0, 0.0));
    }
    ok(make_datetime(days, h, mi, sec))
}

// ------------------------------------------------------------------------------------------
// arithmetic
// ------------------------------------------------------------------------------------------

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

/// GOMONTH(d, n): the same day n months away, clamped to the end of the target month.
fn f_gomonth(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let months = arg_int(&a, 1)?;
    let Some(days) = as_days(&a[0])? else {
        return ok(a[0].deref());
    };
    let (y, m, d) = civil_from_days(days);
    let total = (y as i64) * 12 + (m as i64 - 1) + months;
    let ny = total.div_euclid(12) as i32;
    let nm = total.rem_euclid(12) as u32 + 1;
    let nd = d.min(days_in_month(ny, nm));
    let out = days_from_civil(ny, nm, nd);
    match a[0].deref() {
        Value::DateTime(_) => {
            let tod = as_time_of_day(&a[0])?.unwrap_or(0.0);
            ok(Value::DateTime(Some(out as f64 * 86400.0 + tod)))
        }
        _ => ok(Value::Date(Some(out))),
    }
}

/// WEEK(d [, nFirstWeek] [, nFirstDay]). nFirstWeek: 1 the week holding January 1 (default),
/// 2 the first week with four days in the new year, 3 the first full week; 0 asks SET FWEEK.
/// nFirstDay: 1 Sunday (default) .. 7 Saturday, and 0 asks SET FDOW.
fn f_week(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let rule = week_rule(&a, 1, c.settings().fweek, 3)?;
    let first_day = week_rule(&a, 2, c.settings().fdow, 7)?;
    // two wide, as Visual FoxPro prints a week number
    let width = crate::value::Width { chars: 2, decimals: 0, written: false };
    let Some(days) = as_days(&a[0])? else {
        return ok(Value::Number(0.0, width));
    };
    let fdow = (first_day - 1) as i32;
    let week_start = |d: i32| d - (day_of_week(d) as i32 - fdow).rem_euclid(7);
    let year_first_week = |y: i32| {
        let jan1 = days_from_civil(y, 1, 1);
        let start = week_start(jan1);
        let in_new_year = 7 - (jan1 - start);
        match rule {
            2 if in_new_year < 4 => start + 7,
            3 if in_new_year < 7 => start + 7,
            _ => start,
        }
    };
    let mut y = civil_from_days(days).0;
    let mut base = year_first_week(y);
    if days < base {
        y -= 1;
        base = year_first_week(y);
    } else {
        let next = year_first_week(y + 1);
        if days >= next {
            base = next;
        }
    }
    ok(Value::Number(((days - base) / 7 + 1) as f64, width))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("CDOW", 1, 1, f_cdow),
        spec("CMONTH", 1, 1, f_cmonth),
        spec("CTOD", 1, 1, f_ctod),
        spec("CTOT", 1, 1, f_ctot),
        spec("DATE", 0, 3, f_date),
        spec("DATETIME", 0, 6, f_datetime),
        spec("DAY", 1, 1, f_day),
        spec("DMY", 1, 1, f_dmy),
        spec("DOW", 1, 2, f_dow),
        spec("DTOC", 1, 2, f_dtoc),
        spec("DTOS", 1, 1, f_dtos),
        spec("DTOT", 1, 1, f_dtot),
        spec("GOMONTH", 2, 2, f_gomonth),
        spec("HOUR", 1, 1, f_hour),
        spec("MDY", 1, 1, f_mdy),
        spec("MINUTE", 1, 1, f_minute),
        spec("MONTH", 1, 1, f_month),
        spec("QUARTER", 0, 1, f_quarter),
        spec("SEC", 1, 1, f_sec),
        spec("SECONDS", 0, 0, f_seconds),
        spec("TIME", 0, 1, f_time),
        spec("TTOC", 1, 2, f_ttoc),
        spec("TTOD", 1, 1, f_ttod),
        spec("WEEK", 1, 3, f_week),
        spec("YEAR", 1, 1, f_year),
    ]
}
