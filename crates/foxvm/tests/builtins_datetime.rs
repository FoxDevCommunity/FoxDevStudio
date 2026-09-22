//! Date and datetime built-ins, all against the fixed test clock (2026-09-07 13:05:07).

mod ctx;

use ctx::{NOW_SECONDS, TestCtx, d, dt, err, n, num, num_on, s, text, text_on, today, value, value_on};
use foxvm::error::RtError;
use foxvm::value::{DateFormat, Value};

#[test]
fn clock_functions() {
    assert_eq!(value("DATE", vec![]), Value::Date(Some(today())));
    assert_eq!(value("DATETIME", vec![]), dt(2026, 9, 7, 13.0, 5.0, 7.0));
    assert_eq!(text("TIME", vec![]), "13:05:07");
    assert_eq!(text("TIME", vec![n(1.0)]), "13:05:07.00");
    assert_eq!(num("SECONDS", vec![]), NOW_SECONDS);
}

#[test]
fn constructor_forms() {
    assert_eq!(value("DATE", vec![n(2026.0), n(9.0), n(7.0)]), d(2026, 9, 7));
    assert_eq!(value("DATETIME", vec![n(2026.0), n(9.0), n(7.0)]), dt(2026, 9, 7, 0.0, 0.0, 0.0));
    assert_eq!(
        value("DATETIME", vec![n(2026.0), n(9.0), n(7.0), n(13.0), n(5.0), n(7.0)]),
        dt(2026, 9, 7, 13.0, 5.0, 7.0)
    );
    assert_eq!(err("DATE", vec![n(2026.0), n(13.0), n(1.0)]).code, RtError::FUNCTION_ARG_INVALID);
    // Measured: a day that is in the structural 1-31 range but does not exist in that month -
    // February 29th outside a leap year - is not an error at all, it is the empty date.
    assert_eq!(value("DATE", vec![n(2023.0), n(2.0), n(29.0)]), Value::Date(None));
    assert_eq!(value("DATE", vec![n(2024.0), n(2.0), n(29.0)]), d(2024, 2, 29));
    assert_eq!(err("DATETIME", vec![n(2026.0), n(9.0), n(7.0), n(25.0)]).code, RtError::FUNCTION_ARG_INVALID);
    // One or two arguments is error 1229 ("Too few arguments"), not error 11 - measured, it is
    // a different complaint from a bad year, month or day once all three are actually given.
    assert_eq!(err("DATE", vec![n(2026.0)]).code, RtError::TOO_FEW_ARGS);
    assert_eq!(err("DATE", vec![n(2026.0), n(9.0)]).code, RtError::TOO_FEW_ARGS);
    assert_eq!(err("DATETIME", vec![n(2026.0)]).code, RtError::TOO_FEW_ARGS);
    assert_eq!(err("DATETIME", vec![n(2026.0), n(9.0)]).code, RtError::TOO_FEW_ARGS);
}

#[test]
fn extraction() {
    let day = d(2026, 9, 7);
    assert_eq!(num("YEAR", vec![day.clone()]), 2026.0);
    assert_eq!(num("MONTH", vec![day.clone()]), 9.0);
    assert_eq!(num("DAY", vec![day.clone()]), 7.0);
    assert_eq!(text("CDOW", vec![day.clone()]), "Monday");
    assert_eq!(text("CMONTH", vec![day.clone()]), "September");
    assert_eq!(num("DOW", vec![day.clone()]), 2.0); // Sunday is 1
    assert_eq!(num("DOW", vec![day.clone(), n(2.0)]), 1.0); // week starting Monday
    assert_eq!(num("DOW", vec![day.clone(), n(3.0)]), 7.0); // week starting Tuesday

    let stamp = dt(2026, 9, 7, 13.0, 5.0, 7.0);
    assert_eq!(num("HOUR", vec![stamp.clone()]), 13.0);
    assert_eq!(num("MINUTE", vec![stamp.clone()]), 5.0);
    assert_eq!(num("SEC", vec![stamp.clone()]), 7.0);
    assert_eq!(num("YEAR", vec![stamp]), 2026.0);
    // A date has no time of day.
    assert_eq!(num("HOUR", vec![day]), 0.0);
}

#[test]
fn empty_dates() {
    assert_eq!(num("YEAR", vec![Value::Date(None)]), 0.0);
    assert_eq!(num("MONTH", vec![Value::Date(None)]), 0.0);
    assert_eq!(num("DOW", vec![Value::Date(None)]), 0.0);
    assert_eq!(text("CDOW", vec![Value::Date(None)]), "");
    assert_eq!(text("CMONTH", vec![Value::Date(None)]), "");
    assert_eq!(text("DTOS", vec![Value::Date(None)]), "        ");
    assert_eq!(text("DTOC", vec![Value::Date(None)]), "  /  /  ");
}

#[test]
fn date_to_character() {
    let day = d(2026, 9, 7);
    assert_eq!(text("DTOC", vec![day.clone()]), "09/07/26");
    assert_eq!(text("DTOC", vec![day.clone(), n(1.0)]), "20260907");
    assert_eq!(text("DTOS", vec![day.clone()]), "20260907");
    assert_eq!(text_on(&mut TestCtx::century(), "DTOC", vec![day.clone()]), "09/07/2026");
    assert_eq!(text_on(&mut TestCtx::dated(DateFormat::British), "DTOC", vec![day.clone()]), "07/09/26");
    assert_eq!(text_on(&mut TestCtx::dated(DateFormat::Japan), "DTOC", vec![day.clone()]), "26/09/07");
    assert_eq!(text_on(&mut TestCtx::dated(DateFormat::Italian), "DTOC", vec![day.clone()]), "07-09-26");
    assert_eq!(text_on(&mut TestCtx::dated(DateFormat::Taiwan), "DTOC", vec![day.clone()]), "15/09/07");
    assert_eq!(text_on(&mut TestCtx::dated(DateFormat::Short), "DTOC", vec![day.clone()]), "9/7/2026");
    assert_eq!(text_on(&mut TestCtx::dated(DateFormat::Long), "DTOC", vec![day]), "Monday, September 7, 2026");
}

#[test]
fn datetime_to_character() {
    let stamp = dt(2026, 9, 7, 13.0, 5.0, 7.0);
    assert_eq!(text("TTOC", vec![stamp.clone()]), "09/07/26 01:05:07 PM");
    assert_eq!(text("TTOC", vec![stamp.clone(), n(1.0)]), "20260907130507");
    assert_eq!(value("TTOD", vec![stamp]), d(2026, 9, 7));
    assert_eq!(value("DTOT", vec![d(2026, 9, 7)]), dt(2026, 9, 7, 0.0, 0.0, 0.0));
    assert_eq!(value("DTOT", vec![Value::Date(None)]), Value::DateTime(None));
}

#[test]
fn character_to_date_uses_set_date() {
    assert_eq!(value("CTOD", vec![s("09/07/26")]), d(2026, 9, 7));
    assert_eq!(value("CTOD", vec![s("09/07/2026")]), d(2026, 9, 7));
    assert_eq!(value("CTOD", vec![s("9/7/26")]), d(2026, 9, 7));
    assert_eq!(value("CTOD", vec![s("")]), Value::Date(None));
    assert_eq!(value("CTOD", vec![s("not a date")]), Value::Date(None));
    assert_eq!(value("CTOD", vec![s("02/30/26")]), Value::Date(None));
    // A two-digit year rolls over fifty years from today, which the test clock puts at 76.
    assert_eq!(value("CTOD", vec![s("01/01/60")]), d(2060, 1, 1));
    assert_eq!(value("CTOD", vec![s("01/01/76")]), d(1976, 1, 1));
    assert_eq!(value("CTOD", vec![s("01/01/99")]), d(1999, 1, 1));

    let mut british = TestCtx::dated(DateFormat::British);
    assert_eq!(value_on(&mut british, "CTOD", vec![s("07/09/26")]), d(2026, 9, 7));
    let mut german = TestCtx::dated(DateFormat::German);
    assert_eq!(value_on(&mut german, "CTOD", vec![s("07.09.2026")]), d(2026, 9, 7));
    let mut ymd = TestCtx::dated(DateFormat::Ymd);
    assert_eq!(value_on(&mut ymd, "CTOD", vec![s("2026/09/07")]), d(2026, 9, 7));
}

#[test]
fn character_to_datetime() {
    assert_eq!(value("CTOT", vec![s("09/07/26 01:05:07 PM")]), dt(2026, 9, 7, 13.0, 5.0, 7.0));
    assert_eq!(value("CTOT", vec![s("09/07/26 13:05:07")]), dt(2026, 9, 7, 13.0, 5.0, 7.0));
    assert_eq!(value("CTOT", vec![s("09/07/26")]), dt(2026, 9, 7, 0.0, 0.0, 0.0));
    assert_eq!(value("CTOT", vec![s("09/07/26 12:00:00 AM")]), dt(2026, 9, 7, 0.0, 0.0, 0.0));
    assert_eq!(value("CTOT", vec![s("nonsense")]), Value::DateTime(None));
}

#[test]
fn gomonth_clamps_to_the_end_of_the_month() {
    assert_eq!(value("GOMONTH", vec![d(2026, 1, 31), n(1.0)]), d(2026, 2, 28));
    assert_eq!(value("GOMONTH", vec![d(2024, 1, 31), n(1.0)]), d(2024, 2, 29));
    assert_eq!(value("GOMONTH", vec![d(2026, 3, 31), n(-1.0)]), d(2026, 2, 28));
    assert_eq!(value("GOMONTH", vec![d(2026, 9, 7), n(4.0)]), d(2027, 1, 7));
    assert_eq!(value("GOMONTH", vec![d(2026, 1, 7), n(-1.0)]), d(2025, 12, 7));
    assert_eq!(value("GOMONTH", vec![d(2026, 9, 7), n(0.0)]), d(2026, 9, 7));
    // The time of day of a datetime survives.
    assert_eq!(value("GOMONTH", vec![dt(2026, 1, 31, 13.0, 5.0, 7.0), n(1.0)]), dt(2026, 2, 28, 13.0, 5.0, 7.0));
    assert_eq!(value("GOMONTH", vec![Value::Date(None), n(1.0)]), Value::Date(None));
}

#[test]
fn week_numbers() {
    assert_eq!(num("WEEK", vec![d(2026, 1, 1)]), 1.0);
    assert_eq!(num("WEEK", vec![d(2026, 1, 4)]), 2.0);
    // ISO rules: first week with four days, weeks starting Monday.
    assert_eq!(num("WEEK", vec![d(2026, 1, 1), n(2.0), n(2.0)]), 1.0);
    assert_eq!(num("WEEK", vec![d(2026, 9, 7), n(2.0), n(2.0)]), 37.0);
    assert_eq!(num("WEEK", vec![Value::Date(None)]), 0.0);
}

#[test]
fn null_propagates() {
    for name in ["YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SEC", "DOW", "CDOW", "CMONTH", "DTOC", "DTOS", "CTOD"] {
        assert!(value(name, vec![Value::Null]).is_null(), "{name}() should return NULL");
    }
    assert!(value("GOMONTH", vec![Value::Null, n(1.0)]).is_null());
}

#[test]
fn wrong_types_are_reported() {
    assert_eq!(err("YEAR", vec![s("2026")]).code, RtError::FUNCTION_ARG_INVALID);
    assert_eq!(err("DTOS", vec![n(1.0)]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn seconds_and_time_share_the_host_clock() {
    let mut ctx = TestCtx::default();
    let secs = num_on(&mut ctx, "SECONDS", vec![]);
    let stamp = text_on(&mut ctx, "TIME", vec![]);
    assert_eq!(format!("{:02}:{:02}:{:02}", secs as u32 / 3600, (secs as u32 % 3600) / 60, secs as u32 % 60), stamp);
}
