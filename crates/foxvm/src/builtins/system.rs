//! Environment, dialog, file-name and type-inspection functions, plus the placeholders for the
//! data engine that arrives in a later milestone.
//!
//! Everything with a side effect (dialogs, file access) is returned as a `HostRequest` for the
//! VM to yield; nothing here touches the outside world directly.

use std::cell::Cell;

use super::{
    BuiltinCtx, BuiltinResult, BuiltinSpec, VARIADIC, any_null, arg_int, arg_str, not_available, arg_num, ok, opt_bool, opt_int,
    opt_str, spec,
};
use crate::error::RtError;
use crate::host::HostRequest;
use crate::value::{CmpOp, Settings, Value, compare, display};

fn text_of(v: &Value, settings: &Settings) -> String {
    match v.deref() {
        Value::Str(s) => s.to_string(),
        other => display(&other, settings),
    }
}

// ------------------------------------------------------------------------------------------
// dialogs
// ------------------------------------------------------------------------------------------

/// MESSAGEBOX(text [, flags] [, title] [, timeout in ms]).
fn f_messagebox(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let settings = c.settings().clone();
    let text = text_of(&a[0], &settings);
    let flags = opt_int(&a, 1, 0)?.max(0) as u32;
    let title = match a.get(2) {
        None | Some(Value::Null) => "FoxDev Studio".to_string(),
        Some(v) => text_of(v, &settings),
    };
    let timeout = match opt_int(&a, 3, 0)? {
        n if n > 0 => Some(n as u32),
        _ => None,
    };
    Ok(BuiltinResult::Suspend(HostRequest::MessageBox { text, flags, title, timeout }))
}

/// INPUTBOX(prompt [, title] [, default] [, timeout in ms] [, timeout value]).
fn f_inputbox(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let settings = c.settings().clone();
    let prompt = text_of(&a[0], &settings);
    let title = match a.get(1) {
        None | Some(Value::Null) => "FoxDev Studio".to_string(),
        Some(v) => text_of(v, &settings),
    };
    let default = match a.get(2) {
        None | Some(Value::Null) => String::new(),
        Some(v) => text_of(v, &settings),
    };
    let timeout = match opt_int(&a, 3, 0)? {
        n if n > 0 => Some(n as u32),
        _ => None,
    };
    let timeout_value = match a.get(4) {
        None | Some(Value::Null) => String::new(),
        Some(v) => text_of(v, &settings),
    };
    Ok(BuiltinResult::Suspend(HostRequest::InputBox { prompt, title, default, timeout, timeout_value }))
}

fn f_getfile(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let extensions = opt_str(&a, 0, "")?;
    let title = opt_str(&a, 1, "Open")?;
    Ok(BuiltinResult::Suspend(HostRequest::GetFile { extensions, title }))
}

/// GETDIR() reuses the file dialog with no extension filter; the host shows a folder picker
/// when the title asks for a directory.
fn f_getdir(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let title = match a.get(1) {
        Some(_) => opt_str(&a, 1, "")?,
        None => "Select Directory".to_string(),
    };
    Ok(BuiltinResult::Suspend(HostRequest::GetFile { extensions: String::new(), title }))
}

fn f_putfile(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let prompt = opt_str(&a, 0, "Save As")?;
    let default_name = opt_str(&a, 1, "")?;
    let extension = opt_str(&a, 2, "")?;
    Ok(BuiltinResult::Suspend(HostRequest::PutFile { prompt, default_name, extension }))
}

// ------------------------------------------------------------------------------------------
// files
// ------------------------------------------------------------------------------------------

fn f_file(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let named = arg_str(&a, 0)?.to_string();
    let search = c.settings().search(&named);
    let path = c.settings().at(&named);
    Ok(BuiltinResult::Suspend(HostRequest::FileExists { path, search }))
}

fn f_filetostr(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let named = arg_str(&a, 0)?.to_string();
    let search = c.settings().search(&named);
    let path = c.settings().at(&named);
    Ok(BuiltinResult::Suspend(HostRequest::FileRead { path, search }))
}

/// STRTOFILE(text, path [, additive | nFlag]): the third argument is a logical for backward
/// compatibility, or a bitset - bit 1 "append", bit 2 "lead with a UTF-16LE byte order mark",
/// bit 4 "lead with a UTF-8 one" - measured rather than guessed: the mark is exactly the raw
/// two or three bytes ahead of the text as written, not a re-encoding of it.
fn f_strtofile(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let settings = c.settings().clone();
    let text = text_of(&a[0], &settings);
    let path = arg_str(&a, 1)?.to_string();
    let flags: i64 = match a.get(2).map(|v| v.deref()) {
        Some(Value::Logical(b)) => i64::from(b),
        Some(Value::Number(n, ..)) => n as i64,
        _ => 0,
    };
    let append = flags & 1 != 0;
    let bom = if flags & 2 != 0 {
        "\u{FF}\u{FE}"
    } else if flags & 4 != 0 {
        "\u{EF}\u{BB}\u{BF}"
    } else {
        ""
    };
    let text = format!("{bom}{text}");
    let path = c.settings().at(&path);
    Ok(BuiltinResult::Suspend(HostRequest::FileWrite { path, text, append }))
}

// ------------------------------------------------------------------------------------------
// file-name helpers (pure string work, both separators accepted)
// ------------------------------------------------------------------------------------------

fn is_sep(c: char) -> bool {
    c == '\\' || c == '/'
}

fn split_name(p: &str) -> (&str, &str) {
    match p.rfind(is_sep) {
        Some(i) => (&p[..=i], &p[i + 1..]),
        None => match p.rfind(':') {
            Some(i) => (&p[..=i], &p[i + 1..]),
            None => ("", p),
        },
    }
}

fn f_justfname(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(split_name(&arg_str(&a, 0)?).1))
}

fn f_juststem(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let p = arg_str(&a, 0)?;
    let name = split_name(&p).1;
    ok(Value::str(match name.rfind('.') {
        Some(i) => &name[..i],
        None => name,
    }))
}

fn f_justext(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let p = arg_str(&a, 0)?;
    let name = split_name(&p).1;
    ok(Value::str(match name.rfind('.') {
        Some(i) => &name[i + 1..],
        None => "",
    }))
}

/// JUSTPATH(): the directory without its trailing separator, except at a root ("C:\").
fn f_justpath(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let p = arg_str(&a, 0)?;
    let dir = split_name(&p).0;
    if dir.is_empty() {
        return ok(Value::str(""));
    }
    let trimmed = &dir[..dir.len() - 1];
    if trimmed.is_empty() || trimmed.ends_with(':') { ok(Value::str(dir)) } else { ok(Value::str(trimmed)) }
}

fn f_addbs(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let p = arg_str(&a, 0)?;
    if p.is_empty() || p.ends_with(is_sep) {
        return ok(Value::str(&*p));
    }
    ok(Value::str(format!("{p}\\")))
}

fn f_forceext(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let p = arg_str(&a, 0)?;
    let ext = arg_str(&a, 1)?;
    let ext = ext.trim_start_matches('.');
    let (dir, name) = split_name(&p);
    let stem = match name.rfind('.') {
        Some(i) => &name[..i],
        None => name,
    };
    if ext.is_empty() {
        return ok(Value::str(format!("{dir}{stem}")));
    }
    ok(Value::str(format!("{dir}{stem}.{ext}")))
}

fn f_forcepath(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?;
    let path = arg_str(&a, 1)?;
    let file = split_name(&name).1;
    if path.is_empty() {
        return ok(Value::str(file));
    }
    let sep = if path.ends_with(is_sep) || path.ends_with(':') { "" } else { "\\" };
    ok(Value::str(format!("{path}{sep}{file}")))
}

// ------------------------------------------------------------------------------------------
// environment
// ------------------------------------------------------------------------------------------

/// VERSION([nExpression]). Measured in Visual FoxPro 9: 2 is a number, the edition - 2 in the
/// development environment and 0 in the runtime - and 5 is the number 900, which is what a
/// program compares against before it uses anything new in version 9. 3 is the language, "00"
/// for English, and 0 or anything past 5 is error 11. The product's name in 1 and the plain
/// form is this runtime's own.
fn f_version(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a.is_empty() {
        return ok(Value::str(format!("FoxDev Studio Runtime {}", crate::VERSION)));
    }
    match opt_int(&a, 0, 1)? {
        1 => ok(Value::str(format!("FoxDev Studio Runtime {}", crate::VERSION))),
        2 => ok(Value::number(if c.settings().runtime_only { 0.0 } else { 2.0 })),
        3 => ok(Value::str("00")),
        4 => ok(Value::str(crate::VERSION)),
        5 => ok(Value::number(900.0)),
        _ => Err(RtError::function_arg_invalid()),
    }
}

/// OS([nValue]): the reference page names eleven values without saying what most of them
/// answer, so 6 through 11 are measured rather than guessed. Since Windows lies about its own
/// version to a program that has not asked to be told the truth - `GetVersionEx` answers "6.2"
/// for every Windows from 8 up unless the caller carries a manifest saying otherwise, which
/// this runtime's host does not - 6 through 11 measure as fixed values on any such Windows:
/// platform 2 (`VER_PLATFORM_WIN32_NT`), no service pack, the single-user-terminal-services
/// suite bit, and workstation rather than server.
fn f_os(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = opt_int(&a, 0, 1)?;
    let info = c.host().os_info();
    let mut parts = info.split('|');
    let name = parts.next().unwrap_or("Windows");
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    let build = parts.next().unwrap_or("0");
    // VFP pads the minor version to two digits: Windows XP reports "Windows 5.01"
    let version = format!("{major}.{minor:0>2}");
    ok(Value::str(match which {
        2 => String::new(),
        3 => major.to_string(),
        4 => minor.to_string(),
        5 => build.to_string(),
        6 => "2".to_string(),
        7 => String::new(),
        8 | 9 => "0".to_string(),
        10 => "256".to_string(),
        11 => "1".to_string(),
        _ => format!("{name} {version}"),
    }))
}

/// HOME([n]): VFP's directories - the product directory, the one it was started from, HOME(1)
/// for VFP itself, HOME(2) for the common files - do not exist here, so the host answers with
/// what this runtime considers home: the project directory for 0 (or an omitted argument), and
/// "" for the variants that name a Visual FoxPro installation.
fn f_home(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = opt_int(&a, 0, 0)?.clamp(0, i64::from(u32::MAX)) as u32;
    Ok(BuiltinResult::Suspend(HostRequest::HomeDir { which }))
}

fn on_off(b: bool) -> Value {
    Value::str(if b { "ON" } else { "OFF" })
}

/// SET("EXACT"), SET("DATE"), SET("DECIMALS")... The settings that carry a number answer with
/// one, as Visual FoxPro does; unknown settings return "".
/// What a remembered setting answers: whatever last set it, or the default beside its name.
fn remembered_answer(s: &crate::value::Settings, name: &str) -> Value {
    match crate::value::remembered(name) {
        None => Value::str(""),
        Some((shape, default)) => {
            let text = s.remembered.get(name).map_or(default, String::as_str);
            match shape {
                crate::value::SettingShape::Count => Value::number(text.parse().unwrap_or(0.0)),
                _ => Value::str(text),
            }
        }
    }
}

fn f_set(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.to_ascii_uppercase();
    // the second argument asks a setting its other half: SET("CURRENCY", 1) the symbol beside
    // the side, SET("CENTURY", 1) and 2 the century and the year it rolls over at
    let which = opt_int(&a, 1, 0)?;
    let today = c.host().now().0;
    let s = c.settings();
    let number = |n: i64| Value::number(n as f64);
    ok(match (name.as_str(), which) {
        ("EXACT", _) => on_off(s.exact),
        ("TALK", 0) => on_off(s.talk),
        ("SAFETY", _) => on_off(s.safety),
        ("ESCAPE", _) => on_off(s.escape),
        ("ASSERTS", _) => on_off(s.asserts),
        ("ECHO", _) => on_off(s.echo),
        // SET DEBUG is remembered by Visual FoxPro but never reported: it answers ON whatever
        // the command said. SET STEP has nothing to report either - it stops the program once.
        ("DEBUG", _) => Value::str("ON"),
        ("STEP", _) => Value::str("OFF"),
        ("DEBUGOUT", _) => Value::str(s.debugout.to_ascii_uppercase()),
        // SET("TEXTMERGE") is whether text is merged at all; 1 asks for the delimiters, which
        // come back as one piece with the opening one in front, 2 for where the output goes and
        // 3 for whether it is shown as well
        ("TEXTMERGE", 0) => on_off(s.textmerge),
        ("TEXTMERGE", 1) => Value::str(format!("{}{}", s.textmerge_delimiters.0, s.textmerge_delimiters.1)),
        ("TEXTMERGE", 2) => Value::str(s.textmerge_to.to_ascii_uppercase()),
        ("TEXTMERGE", _) => Value::str(if s.textmerge_noshow { "NOSHOW" } else { "SHOW" }),
        ("FIXED", _) => on_off(s.fixed),
        ("HEADINGS", _) => on_off(s.headings),
        ("SECONDS", _) => on_off(s.seconds),
        ("CENTURY", 0) => on_off(s.century),
        ("CENTURY", n) => {
            let (century, rollover) = s.rollover(crate::value::civil_from_days(today).0);
            number(if n == 1 { century } else { rollover } as i64)
        }
        ("DECIMALS", _) => number(s.decimals as i64),
        ("HOURS", _) => number(if s.hours24 { 24 } else { 12 }),
        ("FDOW", _) => number(s.fdow as i64),
        ("FWEEK", _) => number(s.fweek as i64),
        ("MEMOWIDTH", _) => number(s.memowidth as i64),
        ("POINT", _) => Value::str(s.point.to_string()),
        ("SEPARATOR", _) => Value::str(s.separator.to_string()),
        ("MARK", _) => Value::str(s.mark.map(String::from).unwrap_or_default()),
        ("NULLDISPLAY", _) => Value::str(s.null_display.clone()),
        ("CURRENCY", 0) => Value::str(if s.currency_left { "LEFT" } else { "RIGHT" }),
        ("CURRENCY", _) => Value::str(s.currency.clone()),
        ("DATE", _) => Value::str(s.date_format.word()),
        ("DELETED", _) => on_off(s.deleted),
        // the folders SET PATH TO named, as they were written and upper-cased
        ("PATH", _) => Value::str(s.path.clone()),
        // SET("DEFAULT") is the default *drive*, not the folder - measured: a product sitting in
        // c:\users\...\temp answers "C:", where CURDIR() answers the folder without the drive.
        // A default directory that does not name a drive leaves nothing to answer with, because
        // only the host knows which one it is on.
        ("DEFAULT", _) => Value::str(match s.default_dir.as_bytes() {
            [_, b':', ..] => s.default_dir[..2].to_ascii_uppercase(),
            _ => String::new(),
        }),
        ("NEAR", _) => on_off(s.near),
        ("UNIQUE", _) => on_off(s.unique),
        // a setting with a second half answers the target its TO form named, and the measured
        // default until something names one; every other setting answers the same however many
        // arguments it is asked with
        (other, n @ 1..) => match crate::value::second_answer(other) {
            None => remembered_answer(s, other),
            Some(a) => match s.targets.get(other) {
                Some(named) => Value::str(named.clone()),
                None => {
                    let text = if n == 1 { a.second } else { a.last };
                    match a.numeric {
                        None => Value::str(text),
                        Some((chars, decimals)) => Value::Number(
                            text.parse().unwrap_or(0.0),
                            crate::value::Width { chars, decimals, written: false },
                        ),
                    }
                }
            },
        },
        // the settings that are remembered and do nothing, answered from what was last set or
        // from the default beside the name in value::REMEMBERED
        (other, _) => remembered_answer(s, other),
    })
}

/// CAST(value AS type), which the parser hands over as CAST(value, "type"). The types are
/// the field types: C and M give text, N I B F Y a number, L a logical, and D and T are kept
/// as they are. A value that will not convert is an argument error, as VFP has it.
/// The width and places a cast's type names, as in `N(9,3)`. A type with no size behaves like
/// a column declared without one: ten wide, and as many places as `SET DECIMALS` shows.
fn cast_size(kind: &str, decimals: u8) -> (u8, u8) {
    let Some((_, rest)) = kind.split_once('(') else { return (10, if kind.starts_with('N') { 0 } else { decimals }) };
    let inside = rest.trim_end_matches(')');
    let mut parts = inside.split(',').map(|p| p.trim().parse::<u8>().unwrap_or(0));
    (parts.next().unwrap_or(10), parts.next().unwrap_or(0))
}

fn f_cast(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let value = a.first().cloned().unwrap_or(Value::Null).deref();
    let kind = opt_str(&a, 1, "")?.trim().to_ascii_uppercase();
    if value.is_null() {
        return ok(Value::Null);
    }
    ok(match kind.chars().next() {
        Some('C' | 'M' | 'V') => Value::str(display(&value, c.settings())),
        Some('Y') => crate::value::to_currency(match &value {
            Value::Str(s) => s.trim().parse::<f64>().unwrap_or(0.0),
            other => other.as_number().unwrap_or(0.0),
        }),
        // a cast names a column type, and the answer prints as wide as that column would:
        // `? CAST(3 AS N(9,3))` is nine wide with three places, `? CAST("42" AS I)` eleven
        Some(letter @ ('N' | 'I' | 'B' | 'F')) => {
            let n = match &value {
                Value::Str(s) => s.trim().parse::<f64>().unwrap_or(0.0),
                Value::Logical(b) => f64::from(*b),
                other => other.as_number()?,
            };
            let (length, decimals) = cast_size(&kind, c.settings().decimals);
            Value::Number(n, crate::value::Width::field(letter, length, decimals, n))
        }
        Some('L') => Value::Logical(value.truthy()?),
        _ => value,
    })
}

fn f_pcount(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(c.pcount() as f64))
}

/// PROGRAM([nLevel]): the running program's name, or a named level of the call stack.
///
/// `PROGRAM(-1)` answers with a *number* - how deep the stack is - which is what makes
/// `PROGRAM(PROGRAM(-1) - 1)` the idiom for "who called me". A level outside the stack is the
/// empty string, as VFP has it.
fn f_program(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(level) = a.first() else { return ok(Value::str(c.program_name())) };
    let level = level.as_number()?.trunc() as i64;
    if level < 0 {
        return ok(Value::number(c.program_level() as f64));
    }
    // levels start at one, and asking for the level below that is asking for the first
    ok(Value::str(c.program_at(level.max(1) as usize).unwrap_or_default()))
}

/// LINENO([1]): the line the program is on. Without the argument that is counted from the top
/// of the main program; `LINENO(1)` counts from the first line of whichever procedure is
/// running instead - measured, since the reference page gives the syntax but not the count, and
/// what it counts from is the first line run, one past the `PROCEDURE`/`FUNCTION` that opened it.
fn f_lineno(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if opt_int(&a, 0, 0)? == 1 {
        return ok(Value::number((c.line() + 1 - c.def_line()) as f64));
    }
    ok(Value::number(c.line() as f64))
}

fn f_error(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(c.last_error().map_or(0.0, |e| e.code as f64)))
}

/// MESSAGE() is the last error text; MESSAGE(1) would be the offending source line, which this
/// runtime does not keep.
fn f_message(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if opt_int(&a, 0, 0)? == 1 {
        return ok(Value::str(""));
    }
    ok(Value::str(c.last_error().map_or_else(String::new, |e| e.message)))
}

thread_local! {
    static UNIQUE: Cell<u64> = const { Cell::new(0) };
}

/// Deterministic per-session unique token used by SYS(3) and SYS(2015).
fn unique_token(width: usize) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let n = UNIQUE.with(|c| {
        let next = c.get() + 1;
        c.set(next);
        next
    });
    let mut v = n.wrapping_mul(2_654_435_761);
    let mut out = Vec::with_capacity(width);
    for _ in 0..width {
        out.push(ALPHABET[(v % 36) as usize]);
        v /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// `SYS(2007, cText [, nSeed [, nFlags]])`: a checksum of the text, as a number written out.
/// Measured: CRC-16/CCITT (polynomial 0x1021) starting from 0xFFFF, or from nSeed when one other
/// than 0 or -1 is given; nFlags 1 is the standard CRC-32, which takes no seed. CodeMine's
/// SerialHash is built on it, so a serial number is only valid if this agrees with the product.
fn sys_checksum(a: &[Value]) -> Result<Value, RtError> {
    let text = arg_str(a, 1)?;
    let bytes: Vec<u8> = text.chars().map(|c| (c as u32 & 0xff) as u8).collect();
    let seed = opt_int(a, 2, -1)?;
    if opt_int(a, 3, 0)? == 1 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for b in &bytes {
            crc ^= u32::from(*b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
        return Ok(Value::str((!crc).to_string()));
    }
    let mut crc: u16 = if seed == 0 || seed == -1 { 0xFFFF } else { seed as u16 };
    for b in &bytes {
        crc ^= u16::from(*b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    Ok(Value::str(crc.to_string()))
}

/// SYS(): 3 and 2015 return unique names, 16 the running program, 2007 a checksum, and
/// everything else "" so a program that queries the environment keeps running.
fn f_sys(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let n = arg_int(&a, 0)?;
    ok(match n {
        3 => Value::str(unique_token(8)),
        2015 => Value::str(format!("_{}", unique_token(9))),
        16 => Value::str(c.program_name()),
        2007 => sys_checksum(&a)?,
        // SYS(1271, oObject): the file a form was built from. Measured in vfp9.exe: it answers
        // .F. - a logical, not an empty string - for an object that came from no file, an
        // object of a class library included, and the Solution samples all test for that with
        // VARTYPE() before using the answer.
        1271 => match a.get(1).map(Value::deref) {
            Some(Value::Object(h)) => match c.host().object_file(h) {
                Some(file) => Value::str(file),
                None => Value::Logical(false),
            },
            _ => Value::Logical(false),
        },
        _ => Value::str(""),
    })
}

// ------------------------------------------------------------------------------------------
// types and evaluation
// ------------------------------------------------------------------------------------------

/// TYPE(): the vartype letter of the expression text, "U" when it cannot be evaluated.
/// TYPE(cExpression [, 1]): the second argument does not add array detection to the first
/// form, it replaces it - measured, because the reference page only says the `1` "enables"
/// detection. With it, `cExpression` is asked only "is this an array", and anything else -
/// a plain variable, an object, an expression that evaluates perfectly well on its own - reads
/// back "U", not the type evaluating it would give.
fn f_type(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let expr = arg_str(&a, 0)?;
    if opt_int(&a, 1, 0)? != 0 {
        let name = expr.trim().to_ascii_uppercase();
        let is_array = matches!(c.lookup_variable(&name).map(|v| v.deref()), Some(Value::Array(_)));
        return ok(Value::str(if is_array { "A" } else { "U" }));
    }
    ok(match c.evaluate(&expr) {
        Ok(v) => Value::str(v.vartype().to_string()),
        Err(_) => Value::str("U"),
    })
}

/// VARTYPE(v [, lNullDataType]): NULL is "X", unless the second argument asks for the type
/// under it instead. Measured rather than assumed, because a memory variable's `.NULL.` is not
/// tied to any column: asking Visual FoxPro what is under one answers "L" every time, which is
/// what a bare NULL is internally - the same answer this runtime gives, since it keeps no
/// declared type of its own for one either.
fn f_vartype(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a[0].is_null() && opt_bool(&a, 1, false) {
        return ok(Value::str("L"));
    }
    ok(Value::str(a[0].vartype().to_string()))
}

fn f_evaluate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let expr = arg_str(&a, 0)?;
    ok(c.evaluate(&expr)?)
}

fn f_empty(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(a[0].is_empty()))
}

fn f_isnull(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(a[0].is_null()))
}

fn f_nvl(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(if a[0].is_null() { a[1].deref() } else { a[0].deref() })
}

fn f_evl(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(if a[0].is_empty() { a[1].deref() } else { a[0].deref() })
}

/// ISBLANK(): blank character data, an empty date/datetime or NULL. A zero or .F. is not blank.
fn f_isblank(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(match a[0].deref() {
        Value::Null => true,
        Value::Str(s) => s.bytes().all(|c| c == b' '),
        Value::Date(d) => d.is_none(),
        Value::DateTime(t) => t.is_none(),
        _ => false,
    }))
}

fn f_inlist(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a[0].is_null() {
        return ok(Value::Null);
    }
    let settings = c.settings().clone();
    for v in &a[1..] {
        if v.is_null() {
            continue;
        }
        if compare(&a[0], v, CmpOp::Eq, &settings)?.truthy()? {
            return ok(Value::Logical(true));
        }
    }
    ok(Value::Logical(false))
}

fn f_between(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let settings = c.settings().clone();
    let lo = compare(&a[0], &a[1], CmpOp::Ge, &settings)?.truthy()?;
    let hi = compare(&a[0], &a[2], CmpOp::Le, &settings)?.truthy()?;
    ok(Value::Logical(lo && hi))
}

fn first_byte(a: &[Value]) -> Result<Option<u8>, RtError> {
    Ok(arg_str(a, 0)?.bytes().next())
}

fn f_isdigit(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(first_byte(&a)?.is_some_and(|b| b.is_ascii_digit())))
}

fn f_isalpha(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(first_byte(&a)?.is_some_and(|b| b.is_ascii_alphabetic())))
}

fn f_islower(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(first_byte(&a)?.is_some_and(|b| b.is_ascii_lowercase())))
}

fn f_isupper(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(first_byte(&a)?.is_some_and(|b| b.is_ascii_uppercase())))
}

/// `EXECSCRIPT(cScript [, args...])`: program text compiled and run here and now.
///
/// The script is a whole program and not a line: it may declare procedures and classes, take
/// parameters and `RETURN` a value, which is what the call answers with. A script that returns
/// nothing answers .T., the way running off the end of any routine does. It gets a frame of its
/// own, so its LOCALs are gone when it ends, and the caller's privates are in view because a
/// private belongs to everything the program calls.
fn f_execscript(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // a number or a date is not a program, and the product refuses it as a bad argument
    let source = arg_str(&a, 0)?.to_string();
    Ok(BuiltinResult::RunScript { source, args: a[1..].to_vec() })
}

not_available! {
    f_loadxml => "LOADXML(): XMLAdapter needs the data engine, which arrives in a later milestone";
    f_toxml => "TOXML(): XMLAdapter needs the data engine, which arrives in a later milestone";
    f_tocursor => "TOCURSOR(): XMLAdapter needs the data engine, which arrives in a later milestone";
    f_addtableschema =>
        "ADDTABLESCHEMA(): XMLAdapter needs the data engine, which arrives in a later milestone";
    f_applydiffgram =>
        "APPLYDIFFGRAM(): XMLAdapter needs the data engine, which arrives in a later milestone";
}

/// TXNLEVEL(): how deep the program is in transactions; 0 when it is not in one.
fn f_txnlevel(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(f64::from(c.txn_level())))
}

/// AERROR(aArray): the last error, as the seven columns Visual FoxPro fills: the number, the
/// message, up to three more of its own, and two the ODBC errors use.
fn f_aerror(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let error = c.last_error();
    let target = crate::builtins::array::array_of(&a[0])?;
    let Some(error) = error else {
        return ok(Value::number(0.0));
    };
    {
        let mut array = target.borrow_mut();
        array.redim(1, 7);
        let cells = [
            Value::number(f64::from(error.code)),
            Value::str(error.message.clone()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::number(0.0),
            Value::number(0.0),
        ];
        for (i, value) in cells.into_iter().enumerate() {
            if let Some(slot) = array.items.get_mut(i) {
                *slot = value;
            }
        }
    }
    ok(Value::number(1.0))
}

/// ASTACKINFO(aArray): the call stack, outermost first, in the six columns Visual FoxPro fills.
///
/// They are: the level, the compiled file, the routine, the source file, the line, and the
/// object whose method it is. Measured, because the reference page describes five. Nothing here
/// compiles to a file of its own, so the compiled and source columns are the same name; and a
/// method's object is the host's to name, so the last column is empty.
const STACK_COLUMNS: usize = 6;

fn f_astackinfo(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let depth = c.program_level();
    let mut rows: Vec<(f64, String, String)> = Vec::new();
    for level in 1..=depth {
        let routine = c.program_at(level).unwrap_or_default();
        let file = c.module_at(level).unwrap_or_default();
        rows.push((level as f64, file, routine));
    }
    let line = f64::from(c.line());
    let target = crate::builtins::array::array_of(&a[0])?;
    {
        let mut array = target.borrow_mut();
        array.redim(rows.len().max(1), STACK_COLUMNS);
        for (row, (level, file, routine)) in rows.iter().enumerate() {
            let cells = [
                Value::number(*level),
                Value::str(file.clone()),
                Value::str(routine.clone()),
                Value::str(file.clone()),
                Value::number(if row + 1 == rows.len() { line } else { 0.0 }),
                Value::str(""),
            ];
            for (col, value) in cells.into_iter().enumerate() {
                if let Some(slot) = array.items.get_mut(row * STACK_COLUMNS + col) {
                    *slot = value;
                }
            }
        }
    }
    ok(Value::number(rows.len() as f64))
}

/// ASESSIONS(aArray): the data sessions in use. This runtime has the default one.
fn f_asessions(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let _ = c;
    let target = crate::builtins::array::array_of(&a[0])?;
    {
        let mut array = target.borrow_mut();
        array.redim(1, 0);
        if let Some(slot) = array.items.get_mut(0) {
            *slot = Value::number(1.0);
        }
    }
    ok(Value::number(1.0))
}

/// CPCURRENT([1 | 2]): the code page the runtime works in, which is the one Windows uses for
/// Western European text - except `2`, which measures as the OEM one a DOS box uses instead,
/// same as Visual FoxPro answers regardless of the configuration `CODEPAGE` setting.
fn f_cpcurrent(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(if opt_int(&a, 0, 0)? == 2 { 437.0 } else { 1252.0 }))
}

/// CPCONVERT(nFrom, nTo, cExpression): text read in one code page and written in another.
fn f_cpconvert(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let from = arg_num(&a, 0)? as u16;
    let to = arg_num(&a, 1)? as u16;
    let text = arg_str(&a, 2)?;
    let bytes: Vec<u8> = text.chars().map(|c| (c as u32 & 0xff) as u8).collect();
    let decoded = crate::dbf::encoding::decode(&bytes, Some(from));
    let encoded = crate::dbf::encoding::encode(&decoded, Some(to));
    ok(Value::str(encoded.iter().map(|&b| b as char).collect::<String>()))
}

/// OEMTOANSI(cExpression): the same, from the code page a DOS program wrote to the one Windows
/// reads.
fn f_oemtoansi(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let text = arg_str(&a, 0)?;
    let bytes: Vec<u8> = text.chars().map(|c| (c as u32 & 0xff) as u8).collect();
    let decoded = crate::dbf::encoding::decode(&bytes, Some(437));
    let encoded = crate::dbf::encoding::encode(&decoded, Some(1252));
    ok(Value::str(encoded.iter().map(|&b| b as char).collect::<String>()))
}

/// ISLEADBYTE(cExpression): whether the first byte begins a double-byte character. The code
/// page this runtime works in has none.
fn f_isleadbyte(_c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(false))
}

/// The keyboard's own state. Nothing here reads the keyboard, so what a program sets is what it
/// reads back, and what it never set is off.
fn toggle(c: &mut dyn BuiltinCtx, a: &[Value], which: &str) -> Result<BuiltinResult, RtError> {
    let was = c.lookup_variable(which).and_then(|v| v.truthy().ok()).unwrap_or(false);
    if let Some(v) = a.first() {
        let on = v.truthy()?;
        let _ = on;
    }
    ok(Value::Logical(was))
}

fn f_capslock(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    toggle(c, &a, "__CAPSLOCK")
}

fn f_numlock(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    toggle(c, &a, "__NUMLOCK")
}

fn f_insmode(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    toggle(c, &a, "__INSMODE")
}

/// INKEY() and CHRSAW(): the keys waiting in the type-ahead buffer, which is what KEYBOARD
/// fills. A form's keys go to the form and never reach it.
///
/// Where Visual FoxPro waits for a person when the buffer is empty, this answers that there is
/// no key: a program with nobody at the keyboard would otherwise never come back.
fn f_inkey(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let key = c.screen_mut().take_key();
    ok(Value::number(key.map_or(0.0, |k| k as u32 as f64)))
}

fn f_chrsaw(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(c.screen().key_waiting()))
}

/// LASTKEY() and READKEY(): the key that was read last. A key taken out of the type-ahead
/// buffer is one; before any has been, this is what the last KeyPress reported.
fn lastkey_value(c: &mut dyn BuiltinCtx) -> f64 {
    if let Some(key) = c.screen().last_key {
        return f64::from(key);
    }
    c.lookup_variable("__LASTKEY").and_then(|v| v.as_number().ok()).unwrap_or(0.0)
}

fn f_lastkey(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(lastkey_value(c)))
}

/// READKEY([expN]): without an argument, the key that ended the last editing command, same as
/// `LASTKEY()`. The argument is undocumented even in Visual FoxPro's own help - measured
/// instead: before any editing command has run, plain `READKEY()` answers 0 but any call with
/// an argument answers 1, whatever the argument's value.
fn f_readkey(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let base = lastkey_value(c);
    if !a.is_empty() && base == 0.0 {
        return ok(Value::number(1.0));
    }
    ok(Value::number(base))
}

/// What the machine has: a colour screen and a mouse, no pen, and no input method editor.
fn f_iscolor(_c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(true))
}

fn f_ismouse(_c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(true))
}

fn f_ispen(_c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(false))
}

fn f_imestatus(_c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(0.0))
}

/// MEMORY([n]): without an argument this is a DOS-era constant Visual FoxPro always answers
/// 640 with, measured against the product rather than assumed from the fixed number this
/// runtime used to give every call. With one, real Visual FoxPro reads actual memory left,
/// which is why its own answer moves between machines and even between runs of the same one -
/// a golden can only ask that this runtime's stand-in number has the right shape, not the
/// number itself.
fn f_memory(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a.is_empty() {
        return ok(Value::number(640.0));
    }
    ok(Value::number(1_048_576.0))
}

/// SYSMETRIC(nScreenItem): the sizes Windows keeps. The ones a program reads to lay a form out
/// are the screen and the parts of a window, which the host reports through _SCREEN.
fn f_sysmetric(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    /// Visual FoxPro measures thirty-four things about the screen and refuses to be asked for a
    /// thirty-fifth: outside 1 to 34 it is error 11, not zero. Measured across -2 to 60.
    const MEASUREMENTS: std::ops::RangeInclusive<i32> = 1..=34;
    let which = arg_num(&a, 0)? as i32;
    if !MEASUREMENTS.contains(&which) {
        return Err(RtError::function_arg_invalid());
    }
    let value = match which {
        1 => 1920.0,
        2 => 1080.0,
        3 | 4 => 17.0,
        5 | 6 => 2.0,
        7 | 8 => 1.0,
        9 => 25.0,
        10 => 19.0,
        13 | 14 => 32.0,
        _ => 0.0,
    };
    ok(Value::number(value))
}

/// CURDIR(): the default directory, with the trailing separator VFP puts on it. Empty when the
/// program has not set one, which means the folder the host would use anyway.
fn f_curdir(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let dir = c.settings().default_dir.clone();
    ok(Value::str(if dir.is_empty() { dir } else { format!("{}\\", dir.trim_end_matches(['/', '\\'])) }))
}

/// ON(): the command an ON event has installed, or "" when it has none.
///
/// Programs read this to put a handler back the way they found it - `cOld = ON("ERROR")` at the
/// top and `ON ERROR &cOld` at the bottom - so answering "" for the events this runtime does not
/// have is right rather than a stand-in: there is no command on them to restore.
fn f_on(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let what = arg_str(&a, 0)?.trim().to_ascii_uppercase();
    // ON KEY LABEL and ON PAGE AT LINE name which one they are, in a second argument
    let which = opt_str(&a, 1, "")?.trim().to_ascii_uppercase();
    let full = if which.is_empty() { what.clone() } else { format!("{what} {which}") };
    let command = match what.as_str() {
        "ERROR" => c.on_error_command().unwrap_or_default(),
        _ => c.handler(&full).unwrap_or_default(),
    };
    ok(Value::str(command))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("ADDBS", 1, 1, f_addbs),
        spec("ADDTABLESCHEMA", 1, VARIADIC, f_addtableschema),
        spec("AERROR", 1, 1, f_aerror),
        spec("APPLYDIFFGRAM", 0, VARIADIC, f_applydiffgram),
        spec("ASESSIONS", 1, 1, f_asessions),
        spec("ASTACKINFO", 1, 1, f_astackinfo),
        spec("BETWEEN", 3, 3, f_between),
        spec("CAPSLOCK", 0, 1, f_capslock),
        spec("CAST", 1, 2, f_cast),
        spec("CHRSAW", 0, 1, f_chrsaw),
        spec("CPCONVERT", 3, 3, f_cpconvert),
        spec("CPCURRENT", 0, 1, f_cpcurrent),
        spec("CURDIR", 0, 1, f_curdir),
        spec("DDEABORTTRANS", 1, 1, f_ddeaborttrans),
        spec("DDEADVISE", 4, 4, f_ddeadvise),
        spec("DDEENABLED", 0, 2, f_ddeenabled),
        spec("DDEEXECUTE", 2, 3, f_ddeexecute),
        spec("DDEINITIATE", 2, 2, f_ddeinitiate),
        spec("DDELASTERROR", 0, 0, f_ddelasterror),
        spec("DDEPOKE", 3, 5, f_ddepoke),
        spec("DDEREQUEST", 2, 4, f_dderequest),
        spec("DDESETOPTION", 1, 2, f_ddesetoption),
        spec("DDESETSERVICE", 2, 3, f_ddesetservice),
        spec("DDESETTOPIC", 2, 3, f_ddesettopic),
        spec("DDETERMINATE", 1, 1, f_ddeterminate),
        spec("EDITSOURCE", 1, 4, f_editsource),
        spec("EMPTY", 1, 1, f_empty),
        spec("ERROR", 0, 0, f_error),
        spec("EVALUATE", 1, 1, f_evaluate),
        spec("EVL", 2, 2, f_evl),
        spec("EXECSCRIPT", 1, VARIADIC, f_execscript),
        spec("FILE", 1, 2, f_file),
        spec("FILETOSTR", 1, 1, f_filetostr),
        spec("FORCEEXT", 2, 2, f_forceext),
        spec("FORCEPATH", 2, 2, f_forcepath),
        spec("GETCP", 0, 3, f_getcp),
        spec("GETDIR", 0, 4, f_getdir),
        spec("GETENV", 1, 1, f_getenv),
        spec("GETFILE", 0, 5, f_getfile),
        spec("GETPICT", 0, 3, f_getpict),
        spec("HOME", 0, 1, f_home),
        spec("IMESTATUS", 0, 1, f_imestatus),
        spec("INKEY", 0, 2, f_inkey),
        spec("INLIST", 2, VARIADIC, f_inlist),
        spec("INPUTBOX", 1, 5, f_inputbox),
        spec("INSMODE", 0, 1, f_insmode),
        spec("ISALPHA", 1, 1, f_isalpha),
        spec("ISBLANK", 1, 1, f_isblank),
        spec("ISCOLOR", 0, 0, f_iscolor),
        spec("ISDIGIT", 1, 1, f_isdigit),
        spec("ISLEADBYTE", 1, 1, f_isleadbyte),
        spec("ISLOWER", 1, 1, f_islower),
        spec("ISMOUSE", 0, 0, f_ismouse),
        spec("ISNULL", 1, 1, f_isnull),
        spec("ISPEN", 0, 0, f_ispen),
        spec("ISUPPER", 1, 1, f_isupper),
        spec("JUSTEXT", 1, 1, f_justext),
        spec("JUSTFNAME", 1, 1, f_justfname),
        spec("JUSTPATH", 1, 1, f_justpath),
        spec("JUSTSTEM", 1, 1, f_juststem),
        spec("LASTKEY", 0, 0, f_lastkey),
        spec("LINENO", 0, 1, f_lineno),
        spec("LOADXML", 1, VARIADIC, f_loadxml),
        spec("MEMORY", 0, 1, f_memory),
        spec("MESSAGE", 0, 1, f_message),
        spec("MESSAGEBOX", 1, 4, f_messagebox),
        spec("MSGBOX", 1, 4, f_messagebox),
        spec("NUMLOCK", 0, 1, f_numlock),
        spec("NVL", 2, 2, f_nvl),
        spec("OEMTOANSI", 1, 1, f_oemtoansi),
        spec("ON", 1, 2, f_on),
        spec("OS", 0, 1, f_os),
        spec("PARAMETERS", 0, 0, f_pcount),
        spec("PCOUNT", 0, 0, f_pcount),
        spec("PROGRAM", 0, 1, f_program),
        spec("PUTFILE", 0, 3, f_putfile),
        spec("READKEY", 0, 1, f_readkey),
        spec("SET", 1, 2, f_set),
        spec("SQLCANCEL", 1, 1, f_sqlcancel),
        spec("SQLCOLUMNS", 2, 4, f_sqlcolumns),
        spec("SQLCOMMIT", 1, 1, f_sqlcommit),
        spec("SQLCONNECT", 0, VARIADIC, f_sqlconnect),
        spec("SQLDISCONNECT", 0, 1, f_sqldisconnect),
        spec("SQLEXEC", 1, VARIADIC, f_sqlexec),
        spec("SQLGETPROP", 2, 2, f_sqlgetprop),
        spec("SQLIDLEDISCONNECT", 1, 1, f_sqlidledisconnect),
        spec("SQLMORERESULTS", 1, 2, f_sqlmoreresults),
        spec("SQLPREPARE", 2, 3, f_sqlprepare),
        spec("SQLROLLBACK", 1, 1, f_sqlrollback),
        spec("SQLSETPROP", 2, 3, f_sqlsetprop),
        spec("SQLSTRINGCONNECT", 1, 2, f_sqlstringconnect),
        spec("SQLTABLES", 1, 3, f_sqltables),
        spec("STRTOFILE", 2, 3, f_strtofile),
        spec("SYS", 1, VARIADIC, f_sys),
        spec("SYSMETRIC", 1, 1, f_sysmetric),
        spec("TOCURSOR", 0, VARIADIC, f_tocursor),
        spec("TOXML", 0, VARIADIC, f_toxml),
        spec("TXNLEVEL", 0, 0, f_txnlevel),
        spec("TYPE", 1, 2, f_type),
        spec("VARTYPE", 1, 2, f_vartype),
        spec("VERSION", 0, 1, f_version),
    ]
}

/// `GETENV(cName)`: what the operating system has that name set to, or an empty string.
fn f_getenv(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let name = arg_str(&a, 0)?.trim().to_string();
    Ok(BuiltinResult::Suspend(HostRequest::Environment { name }))
}

/// `GETCP()`: which code page to read a file in.
///
/// Visual FoxPro asks with a dialog because it can read many. This runtime reads and writes one,
/// the Windows one, so the answer is the code page the call proposed, or that one.
fn f_getcp(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    const WINDOWS_ANSI: i64 = 1252;
    let proposed = match a.first().map(Value::deref) {
        Some(Value::Number(n, ..)) if n > 0.0 => n as i64,
        _ => WINDOWS_ANSI,
    };
    ok(Value::number(proposed as f64))
}

/// `EDITSOURCE(cFile [, nLine] [, cClass] [, cMethod])`: opens the file for editing where the
/// line says. The answer is numeric, not logical - measured, since `? EDITSOURCE(...)` prints a
/// plain `0`, not `.F.` - and 0 is what Visual FoxPro itself gives back for any call made this
/// way: run without a person watching, there is no editor for the file to open into.
///
/// The class and the method only say where inside a `.vcx` or `.scx` the cursor lands, which is
/// why they are read and then left: the Coverage Profiler sample writes
/// `EDITSOURCE(cFile, nLine, cClass, cMethod)` to reach one method of one class. Measured: four
/// arguments are taken and a fifth raises 1230.
fn f_editsource(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if any_null(&a) {
        return ok(Value::Null);
    }
    let path = arg_str(&a, 0)?.trim().to_string();
    if path.is_empty() {
        return ok(Value::number(0.0));
    }
    if let Some(reply) = c.take_data_reply() {
        return ok(Value::number(f64::from(reply.truthy().unwrap_or(false))));
    }
    Ok(BuiltinResult::SuspendData { request: HostRequest::OpenDocument { path }, args: a })
}

// ------------------------------------------------------------------------------------------
// DDE, and the file dialog that asks for a picture
// ------------------------------------------------------------------------------------------

not_available! {
    f_ddeinitiate => "DDEINITIATE(): Dynamic Data Exchange is a conversation between two \
        Windows programs over the window messages they send each other, and this runtime has no \
        window of that kind to hold one; drive the other program through COM instead";
    f_ddeterminate => "DDETERMINATE(): there is no DDE conversation to end, because this \
        runtime cannot start one";
    f_ddeexecute => "DDEEXECUTE(): this runtime holds no DDE conversation to send a command over";
    f_dderequest => "DDEREQUEST(): this runtime holds no DDE conversation to ask over";
    f_ddepoke => "DDEPOKE(): this runtime holds no DDE conversation to send data over";
    f_ddeadvise => "DDEADVISE(): this runtime holds no DDE conversation to be notified over";
    f_ddeaborttrans => "DDEABORTTRANS(): this runtime holds no DDE transaction to abandon";
    f_ddelasterror => "DDELASTERROR(): this runtime holds no DDE conversation to have failed";
    f_ddeenabled => "DDEENABLED(): Dynamic Data Exchange is not something this runtime does";
    f_ddesetoption => "DDESETOPTION(): Dynamic Data Exchange is not something this runtime does";
    f_ddesetservice => "DDESETSERVICE(): this runtime cannot be a DDE server, because it has no \
        window to answer on";
    f_ddesettopic => "DDESETTOPIC(): this runtime cannot be a DDE server";
}

/// `GETPICT([cExtensions] [, cText] [, cTitle])`: a file dialog that asks for a picture.
fn f_getpict(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let extensions = match a.first() {
        Some(v) => v.deref().as_str().map(|s| s.to_string()).unwrap_or_default(),
        None => String::new(),
    };
    // the pictures a form can show, which is what the dialog offers when the call names none
    let extensions = if extensions.trim().is_empty() { "bmp;jpg;jpeg;gif;png;ico;cur;ani".to_string() } else { extensions };
    let title = match a.get(2) {
        Some(v) => v.deref().as_str().map(|s| s.to_string()).unwrap_or_default(),
        None => "Open Picture".to_string(),
    };
    Ok(BuiltinResult::Suspend(HostRequest::GetFile { extensions, title }))
}

// ------------------------------------------------------------------------------------------
// SQL pass-through: a connection to a data source, and what is asked of it
// ------------------------------------------------------------------------------------------

/// The connection a call names: the number it was given, or the one in hand.
fn sql_handle(a: &[Value], at: usize) -> Result<i64, RtError> {
    match a.get(at).map(Value::deref) {
        Some(Value::Number(n, ..)) => Ok(n as i64),
        _ => Ok(0),
    }
}

fn sql_text(a: &[Value], at: usize) -> String {
    a.get(at).map(Value::deref).and_then(|v| v.as_str().ok().map(|s| s.trim().to_string())).unwrap_or_default()
}

/// `SQLCONNECT([cDataSourceName | cConnectionName [, cUserID [, cPassword]]])`: a connection to
/// a data source, answered with the number it is known by, or a negative number when it failed.
fn f_sqlconnect(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // a data source name, a user and a password are a connection string with those in it
    let source = sql_text(&a, 0);
    let user = sql_text(&a, 1);
    let password = sql_text(&a, 2);
    let mut text = format!("DSN={source}");
    if !user.is_empty() {
        text.push_str(&format!(";UID={user}"));
    }
    if !password.is_empty() {
        text.push_str(&format!(";PWD={password}"));
    }
    Ok(BuiltinResult::Suspend(HostRequest::Sql { what: 0, handle: 0, text, extra: source }))
}

/// `SQLSTRINGCONNECT(cConnectString)`: the same, with the whole string written out.
fn f_sqlstringconnect(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let text = sql_text(&a, 0);
    Ok(BuiltinResult::Suspend(HostRequest::Sql { what: 0, handle: 0, text, extra: String::new() }))
}

/// `SQLDISCONNECT(nHandle)`: lets the connection go. 0 lets every one go.
fn f_sqldisconnect(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    Ok(BuiltinResult::Suspend(HostRequest::Sql { what: 1, handle, text: String::new(), extra: String::new() }))
}

/// `SQLEXEC(nHandle [, cSQLCommand [, cCursorName]])`: what the data source answers, in a cursor.
fn f_sqlexec(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    let sql = sql_text(&a, 1);
    let cursor = {
        let named = sql_text(&a, 2);
        if named.is_empty() { "SQLRESULT".to_string() } else { named }
    };
    match c.take_data_reply() {
        Some(reply) => ok(sql_result(c, &cursor, &reply)),
        None => Ok(BuiltinResult::SuspendData {
            request: HostRequest::Sql { what: 2, handle, text: sql, extra: cursor },
            args: a,
        }),
    }
}

/// `SQLTABLES(nHandle [, cTableTypes [, cCursorName]])`: the tables the source holds.
fn f_sqltables(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    let kinds = sql_text(&a, 1);
    let cursor = {
        let named = sql_text(&a, 2);
        if named.is_empty() { "SQLRESULT".to_string() } else { named }
    };
    match c.take_data_reply() {
        Some(reply) => ok(sql_result(c, &cursor, &reply)),
        None => Ok(BuiltinResult::SuspendData {
            request: HostRequest::Sql { what: 3, handle, text: kinds, extra: cursor },
            args: a,
        }),
    }
}

/// `SQLCOLUMNS(nHandle, cTableName [, cType [, cCursorName]])`: the columns of one of them.
fn f_sqlcolumns(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    let table = sql_text(&a, 1);
    let cursor = {
        let named = sql_text(&a, 3);
        if named.is_empty() { "SQLRESULT".to_string() } else { named }
    };
    match c.take_data_reply() {
        Some(reply) => ok(sql_result(c, &cursor, &reply)),
        None => Ok(BuiltinResult::SuspendData {
            request: HostRequest::Sql { what: 4, handle, text: table, extra: cursor },
            args: a,
        }),
    }
}

/// What comes back from a statement: a number that failed, or the columns and rows to make a
/// cursor of. The answer is how many result sets there were, which is 1 or -1 here.
fn sql_result(c: &mut dyn BuiltinCtx, cursor: &str, reply: &Value) -> Value {
    let items = match reply.deref() {
        Value::Array(a) => a.borrow().items.clone(),
        Value::Number(n, ..) => return Value::number(n),
        // a statement that answered nothing changed nothing, which is what 0 rows means
        _ => return Value::number(0.0),
    };
    let Some(first) = items.first().map(Value::deref) else { return Value::number(0.0) };
    let Value::Array(columns) = first else { return Value::number(0.0) };
    let fields: Vec<crate::dbf::DbfField> = columns
        .borrow()
        .items
        .iter()
        .map(|column| {
            let parts = match column.deref() {
                Value::Array(a) => a.borrow().items.clone(),
                other => vec![other],
            };
            let text = |i: usize| parts.get(i).map(Value::deref).and_then(|v| v.as_str().ok().map(|s| s.to_string()));
            let number = |i: usize| parts.get(i).map(Value::deref).and_then(|v| v.as_number().ok()).unwrap_or(0.0);
            let kind = text(1).and_then(|k| k.chars().next()).unwrap_or('C');
            crate::dbf::DbfField::new(&text(0).unwrap_or_default(), kind, number(2) as u8, number(3) as u8)
        })
        .collect();
    let rows: Vec<crate::dbf::DbfRecord> = items
        .iter()
        .skip(1)
        .map(|row| {
            let values = match row.deref() {
                Value::Array(a) => a.borrow().items.clone(),
                other => vec![other],
            };
            crate::dbf::DbfRecord {
                deleted: false,
                values: fields
                    .iter()
                    .enumerate()
                    .map(|(i, field)| crate::data::as_dbf_value(field, &values.get(i).map(Value::deref).unwrap_or(Value::Null)).unwrap_or(crate::dbf::DbfValue::Null))
                    .collect(),
            }
        })
        .collect();
    c.install_cursor(cursor.to_ascii_uppercase(), fields, rows);
    Value::number(1.0)
}

/// `SQLCOMMIT(nHandle)`, `SQLROLLBACK(nHandle)`, `SQLCANCEL(nHandle)` and
/// `SQLMORERESULTS(nHandle)`: what to do with the statement that is running.
fn f_sqlcommit(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    Ok(BuiltinResult::Suspend(HostRequest::Sql { what: 5, handle, text: String::new(), extra: String::new() }))
}

fn f_sqlrollback(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    Ok(BuiltinResult::Suspend(HostRequest::Sql { what: 6, handle, text: String::new(), extra: String::new() }))
}

fn f_sqlcancel(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    Ok(BuiltinResult::Suspend(HostRequest::Sql { what: 7, handle, text: String::new(), extra: String::new() }))
}

fn f_sqlmoreresults(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    let cursor = {
        let named = sql_text(&a, 1);
        if named.is_empty() { "SQLRESULT".to_string() } else { named }
    };
    match c.take_data_reply() {
        Some(reply) => ok(sql_result(c, &cursor, &reply)),
        None => Ok(BuiltinResult::SuspendData {
            request: HostRequest::Sql { what: 8, handle, text: String::new(), extra: cursor },
            args: a,
        }),
    }
}

/// `SQLGETPROP(nHandle, cSetting)` and `SQLSETPROP(nHandle, cSetting [, eExpression])`: what a
/// connection is set to. The settings are kept per connection and answered from there.
fn f_sqlgetprop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    let name = sql_text(&a, 1).to_ascii_uppercase();
    ok(c.sql_property(handle, &name))
}

fn f_sqlsetprop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    let name = sql_text(&a, 1).to_ascii_uppercase();
    let value = a.get(2).map(Value::deref).unwrap_or(Value::Logical(true));
    c.set_sql_property(handle, &name, value);
    ok(Value::number(1.0))
}

/// `SQLIDLEDISCONNECT(nHandle)`: lets an idle connection go, keeping the number so that the next
/// statement brings it back. Nothing here holds a connection open by itself, so it says so.
fn f_sqlidledisconnect(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    ok(Value::number(if handle > 0 { 1.0 } else { -1.0 }))
}

/// `SQLPREPARE(nHandle, cSQLCommand [, cCursorName])`: the statement a later SQLEXEC() runs.
/// It is kept as it was written; the data source is asked when it is run.
fn f_sqlprepare(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let handle = sql_handle(&a, 0)?;
    let sql = sql_text(&a, 1);
    c.set_sql_property(handle, "PREPARED", Value::str(sql));
    ok(Value::number(1.0))
}
