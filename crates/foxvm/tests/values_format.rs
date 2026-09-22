//! The one formatter every display path goes through, under the settings that decide what a
//! value looks like.
//!
//! Every expectation here was read off Visual FoxPro 9 itself, not off the documentation. The
//! programs that produced them are in `tests/vfp` beside their captured output; the last test
//! in this file holds the whole of one against the golden program's expectation, so the
//! golden bytes cannot drift away from the product's.

mod ctx;

use std::fs;
use std::path::PathBuf;

use ctx::{TestCtx, d, err, n, num_on, s, text_on, value_on};
use foxvm::error::RtError;
use foxvm::value::{DateFormat, Settings, Value, days_from_civil, format_date, format_datetime, format_number};

fn day() -> Option<i32> {
    Some(days_from_civil(2026, 9, 7))
}

fn stamp(h: f64, mi: f64, sec: f64) -> Option<f64> {
    Some(days_from_civil(2026, 9, 7) as f64 * 86400.0 + h * 3600.0 + mi * 60.0 + sec)
}

fn with(f: impl FnOnce(&mut Settings)) -> Settings {
    let mut s = Settings::default();
    f(&mut s);
    s
}

// ------------------------------------------------------------------------------------------
// numbers
// ------------------------------------------------------------------------------------------

#[test]
fn set_decimals_and_set_fixed_decide_the_places() {
    // FIXED OFF leaves a whole number whole and gives everything else SET DECIMALS places;
    // FIXED ON gives every number those places, 2 included
    for (decimals, third, whole) in [(0u8, "0", "2"), (1, "0.3", "2"), (2, "0.33", "2"), (5, "0.33333", "2")] {
        let st = with(|s| s.decimals = decimals);
        assert_eq!(format_number(1.0 / 3.0, &st), third);
        assert_eq!(format_number(2.0, &st), whole);
    }
    for (decimals, third, whole) in [(0u8, "0", "2"), (1, "0.3", "2.0"), (2, "0.33", "2.00"), (5, "0.33333", "2.00000")]
    {
        let st = with(|s| {
            s.decimals = decimals;
            s.fixed = true;
        });
        assert_eq!(format_number(1.0 / 3.0, &st), third);
        assert_eq!(format_number(2.0, &st), whole);
    }
    // half goes away from zero: 2.5 at no decimal places is 3
    assert_eq!(format_number(2.5, &with(|s| s.decimals = 0)), "3");
    // and a value that rounds away to nothing loses its sign with it
    assert_eq!(format_number(-0.001, &Settings::default()), "0.00");
}

#[test]
fn set_point_is_written_wherever_a_decimal_point_is() {
    let st = with(|s| s.point = '_');
    assert_eq!(format_number(1.0 / 3.0, &st), "0_33");
    let mut ctx = TestCtx::with_settings(|s| s.point = '_');
    assert_eq!(text_on(&mut ctx, "STR", vec![n(1234.5678), n(12.0), n(4.0)]), "   1234_5678");
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1234.5678), s("999,999.99")]), "  1,234_57");
    // VAL() reads the point back, so 3,7 is three and seven tenths where the point is a comma
    let mut euro = TestCtx::with_settings(|s| s.point = ',');
    assert_eq!(num_on(&mut euro, "VAL", vec![s("3,7")]), 3.7);
    assert_eq!(num_on(&mut euro, "VAL", vec![s("3.7")]), 3.0);
}

#[test]
fn set_separator_only_reaches_a_picture_that_asks_for_groups() {
    let mut ctx = TestCtx::with_settings(|s| {
        s.point = '_';
        s.separator = '#';
    });
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1234.5678), s("999,999.99")]), "  1#234_57");
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1234567.8), s("9,999,999.99")]), "1#234#567_80");
    // no comma in the picture, no separator anywhere
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1234567.8), s("9999999.99")]), "1234567_80");
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1234.5678)]), "1234_57");
    // a locale that separates with a full stop and points with a comma keeps the two apart
    let mut euro = TestCtx::with_settings(|s| {
        s.point = ',';
        s.separator = '.';
    });
    assert_eq!(text_on(&mut euro, "TRANSFORM", vec![n(1234.5), s("999,999.99")]), "  1.234,50");
}

#[test]
fn set_currency_puts_its_symbol_on_the_side_it_was_told() {
    let mut dollar = TestCtx::default();
    assert_eq!(text_on(&mut dollar, "TRANSFORM", vec![n(1234.56), s("@$ 999,999.99")]), " $1,234.56");
    // a `$` written into the picture is a dollar sign of its own, not the setting
    assert_eq!(text_on(&mut dollar, "TRANSFORM", vec![n(1234.56), s("$$$,$$$.99")]), " $1,234.56");

    let mut left = TestCtx::with_settings(|s| s.currency = "EUR".into());
    // the symbol goes outside the number and the picture's width still rules: what is left of
    // "EUR1,234.56" in ten characters is its right-hand end
    assert_eq!(text_on(&mut left, "TRANSFORM", vec![n(1234.56), s("@$ 999,999.99")]), "UR1,234.56");
    assert_eq!(text_on(&mut left, "TRANSFORM", vec![n(-1234.56), s("@$ 999,999.99")]), "R-1,234.56");

    let mut right = TestCtx::with_settings(|s| {
        s.currency = "EUR".into();
        s.currency_left = false;
    });
    assert_eq!(text_on(&mut right, "TRANSFORM", vec![n(1234.56), s("@$ 999,999.99")]), "  1,234.56EUR");
    assert_eq!(text_on(&mut right, "TRANSFORM", vec![n(-1234.56), s("@$ 999,999.99")]), " -1,234.56EUR");
}

#[test]
fn a_number_too_big_for_its_picture_fills_with_asterisks() {
    let mut ctx = TestCtx::with_settings(|s| s.point = ',');
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1234.5), s("999.99")]), "***,**");
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![n(1234.5), s("@R 99.99")]), "**,**");
}

#[test]
fn set_nulldisplay_replaces_the_words_null_is_shown_by() {
    let st = with(|s| s.null_display = "<none>".into());
    assert_eq!(foxvm::value::display(&Value::Null, &st), "<none>");
    let mut ctx = TestCtx::with_settings(|s| s.null_display = "<none>".into());
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![Value::Null]), "<none>");
    assert_eq!(text_on(&mut ctx, "TRANSFORM", vec![Value::Null, s("999.99")]), "<none>");
    assert_eq!(text_on(&mut ctx, "SET", vec![s("NULLDISPLAY")]), "<none>");
}

// ------------------------------------------------------------------------------------------
// dates
// ------------------------------------------------------------------------------------------

#[test]
fn every_set_date_word_lays_a_date_out_as_visual_foxpro_does() {
    let table = [
        (DateFormat::American, "09/07/26", "09/07/2026"),
        (DateFormat::Ansi, "26.09.07", "2026.09.07"),
        (DateFormat::British, "07/09/26", "07/09/2026"),
        (DateFormat::French, "07/09/26", "07/09/2026"),
        (DateFormat::German, "07.09.26", "07.09.2026"),
        (DateFormat::Italian, "07-09-26", "07-09-2026"),
        (DateFormat::Japan, "26/09/07", "2026/09/07"),
        // Taiwan counts its years from 1912, so 2026 is its 115th
        (DateFormat::Taiwan, "15/09/07", "0115/09/07"),
        (DateFormat::Usa, "09-07-26", "09-07-2026"),
        (DateFormat::Mdy, "09/07/26", "09/07/2026"),
        (DateFormat::Dmy, "07/09/26", "07/09/2026"),
        (DateFormat::Ymd, "26/09/07", "2026/09/07"),
        // the two Windows formats take no notice of SET CENTURY
        (DateFormat::Short, "9/7/2026", "9/7/2026"),
        (DateFormat::Long, "Monday, September 7, 2026", "Monday, September 7, 2026"),
    ];
    for (format, short, long) in table {
        assert_eq!(format_date(day(), &with(|s| s.date_format = format)), short, "{format:?}");
        assert_eq!(
            format_date(
                day(),
                &with(|s| {
                    s.date_format = format;
                    s.century = true;
                })
            ),
            long,
            "{format:?} with the century"
        );
        assert_eq!(DateFormat::parse(format.word()), Some(format));
    }
}

#[test]
fn set_mark_replaces_the_separator_every_format_carries() {
    for (format, marked) in [
        (DateFormat::American, "09*07*26"),
        (DateFormat::German, "07*09*26"),
        (DateFormat::Italian, "07*09*26"),
        (DateFormat::Taiwan, "15*09*07"),
    ] {
        let st = with(|s| {
            s.date_format = format;
            s.mark = Some('*');
        });
        assert_eq!(format_date(day(), &st), marked, "{format:?}");
        assert_eq!(format_date(None, &st), "  *  *  ", "{format:?} empty");
    }
    assert_eq!(format_date(None, &Settings::default()), "  /  /  ");
    assert_eq!(format_date(None, &with(|s| s.date_format = DateFormat::German)), "  .  .  ");
    assert_eq!(format_date(None, &with(|s| s.century = true)), "  /  /    ");
    assert_eq!(format_date(None, &with(|s| s.date_format = DateFormat::Long)), "  /  /  ");
}

#[test]
fn set_hours_and_set_seconds_decide_the_clock() {
    let table = [
        (false, true, "09/07/26 01:05:09 PM"),
        (false, false, "09/07/26 01:05 PM"),
        (true, true, "09/07/26 13:05:09"),
        (true, false, "09/07/26 13:05"),
    ];
    for (hours24, seconds, want) in table {
        let st = with(|s| {
            s.hours24 = hours24;
            s.seconds = seconds;
        });
        assert_eq!(format_datetime(stamp(13.0, 5.0, 9.0), &st), want);
    }
    let noon = with(|s| s.hours24 = false);
    assert_eq!(format_datetime(stamp(0.0, 0.0, 0.0), &noon), "09/07/26 12:00:00 AM");
    assert_eq!(format_datetime(stamp(12.0, 0.0, 0.0), &noon), "09/07/26 12:00:00 PM");
    // an empty datetime keeps a full one's width, and its AM while the clock has one
    assert_eq!(format_datetime(None, &Settings::default()), "  /  /     :  :   AM");
    assert_eq!(format_datetime(None, &with(|s| s.hours24 = true)), "  /  /     :  :  ");
    // the Windows formats write their own clock whatever SET HOURS says
    let short = with(|s| {
        s.date_format = DateFormat::Short;
        s.hours24 = true;
        s.seconds = false;
    });
    assert_eq!(format_datetime(stamp(13.0, 5.0, 9.0), &short), "9/7/2026 1:05:09 PM");
    let long = with(|s| s.date_format = DateFormat::Long);
    assert_eq!(format_datetime(stamp(13.0, 5.0, 9.0), &long), "Monday, September 7, 2026, 1:05:09 PM");
}

#[test]
fn the_time_on_its_own_follows_the_same_clock() {
    let t = Value::DateTime(stamp(13.0, 5.0, 9.0));
    let mut twelve = TestCtx::default();
    assert_eq!(text_on(&mut twelve, "TTOC", vec![t.clone(), n(2.0)]), "01:05:09 PM");
    let mut twenty_four = TestCtx::with_settings(|s| {
        s.hours24 = true;
        s.seconds = false;
    });
    assert_eq!(text_on(&mut twenty_four, "TTOC", vec![t, n(2.0)]), "13:05");
}

// ------------------------------------------------------------------------------------------
// the week
// ------------------------------------------------------------------------------------------

#[test]
fn only_a_zero_asks_set_fdow_and_set_fweek() {
    let monday = d(2026, 9, 7);
    for (fdow, from_setting) in [(1u8, 2.0), (2, 1.0), (3, 7.0), (7, 3.0)] {
        let mut ctx = TestCtx::with_settings(|s| s.fdow = fdow);
        assert_eq!(num_on(&mut ctx, "DOW", vec![monday.clone(), n(0.0)]), from_setting);
        // leaving the argument out is Sunday whatever the setting says
        assert_eq!(num_on(&mut ctx, "DOW", vec![monday.clone()]), 2.0);
    }
    let jan4 = d(2021, 1, 4);
    for (fweek, week) in [(1u8, 2.0), (2, 1.0), (3, 1.0)] {
        let mut ctx = TestCtx::with_settings(|s| s.fweek = fweek);
        assert_eq!(num_on(&mut ctx, "WEEK", vec![jan4.clone(), n(0.0), n(0.0)]), week);
        assert_eq!(num_on(&mut ctx, "WEEK", vec![jan4.clone()]), 2.0);
    }
    // a week that starts before its year belongs to the year before it
    let mut iso = TestCtx::with_settings(|s| {
        s.fdow = 2;
        s.fweek = 2;
    });
    assert_eq!(num_on(&mut iso, "WEEK", vec![d(2021, 1, 1), n(0.0), n(0.0)]), 53.0);
    assert_eq!(num_on(&mut iso, "WEEK", vec![d(2026, 12, 31), n(0.0), n(0.0)]), 53.0);
    // and a day or a rule outside the table is an illegal value, not a silent default
    assert_eq!(err("DOW", vec![monday, n(9.0)]).code, RtError::ILLEGAL_VALUE);
    assert_eq!(err("WEEK", vec![jan4, n(4.0)]).code, RtError::ILLEGAL_VALUE);
}

// ------------------------------------------------------------------------------------------
// reading a two-digit year
// ------------------------------------------------------------------------------------------

#[test]
fn set_century_to_rollover_says_which_century_a_short_year_is_in() {
    // the test clock is 2026, so the default is the 19th century rolling over at 76
    let (century, rollover) = Settings::default().rollover(2026);
    assert_eq!((century, rollover), (19, 76));
    assert_eq!(Settings::default().rollover(1998), (19, 48));
    assert_eq!(Settings::default().rollover(2050), (20, 0));

    let mut ctx = TestCtx::default();
    assert_eq!(value_on(&mut ctx, "CTOD", vec![s("09/07/26")]), d(2026, 9, 7));
    assert_eq!(value_on(&mut ctx, "CTOD", vec![s("09/07/76")]), d(1976, 9, 7));
    let mut fifty = TestCtx::with_settings(|s| s.century_to = Some((19, 50)));
    assert_eq!(value_on(&mut fifty, "CTOD", vec![s("09/07/49")]), d(2049, 9, 7));
    assert_eq!(value_on(&mut fifty, "CTOD", vec![s("09/07/50")]), d(1950, 9, 7));
    let mut regency = TestCtx::with_settings(|s| s.century_to = Some((18, 0)));
    assert_eq!(value_on(&mut regency, "CTOD", vec![s("09/07/26")]), d(1826, 9, 7));
    // a year written in full is the year it says
    assert_eq!(value_on(&mut regency, "CTOD", vec![s("09/07/1926")]), d(1926, 9, 7));
    // and a Taiwan year is counted from 1912 unless it is written in full
    let mut taiwan = TestCtx::dated(DateFormat::Taiwan);
    assert_eq!(value_on(&mut taiwan, "CTOD", vec![s("115/09/07")]), d(2026, 9, 7));
    assert_eq!(value_on(&mut taiwan, "CTOD", vec![s("2026/09/07")]), d(2026, 9, 7));
}

// ------------------------------------------------------------------------------------------
// the golden program's bytes are the product's bytes
// ------------------------------------------------------------------------------------------

/// `tests/vfp/values_display.prg` is `tests/programs/values_display.prg` with the three lines
/// that make vfp9.exe run headlessly and write its console to a file. This holds what came
/// back against what the golden program expects, so neither can drift on its own.
#[test]
fn the_golden_expectation_is_what_vfp9_printed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let recorded = fs::read_to_string(root.join("vfp/values_display.txt")).expect("the recorded vfp9 output");
    let expected = fs::read_to_string(root.join("programs/values_display.expected")).expect("the golden expectation");
    // SET ALTERNATE writes CRLF, pads the console line out with blanks and ends the file with
    // an end-of-file byte; none of that is the value being shown
    let clean = |text: &str| -> Vec<String> {
        text.replace('\u{1a}', "")
            .lines()
            .map(|l| l.trim_end().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    };
    assert_eq!(clean(&recorded), clean(&expected));
}
