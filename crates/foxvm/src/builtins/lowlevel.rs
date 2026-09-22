//! Low-level file functions and the directory functions: the `FOPEN()` family, `ADIR()`,
//! `FULLPATH()` and their kin.
//!
//! Every one of them is a question for the host, which owns the files, so each yields a
//! `HostRequest::FileOp` and takes its answer as its result. The answer is a pair: the value,
//! and the error number Visual FoxPro would leave for `FERROR()` - 0 when all went well - which
//! the VM keeps so a program can ask for it afterwards, as VFP programs do after every open.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, arg_int, arg_str, ok, opt_int, opt_str, spec};
use crate::error::RtError;
use crate::host::HostRequest;
use crate::value::{FoxArray, Value};

/// A request with everything a file operation might need; each operation fills in its own.
fn op(name: &str) -> HostRequest {
    HostRequest::FileOp {
        op: name.to_string(),
        handle: 0,
        path: String::new(),
        target: String::new(),
        text: String::new(),
        count: 0.0,
        offset: 0.0,
        whence: 0,
        search: Vec::new(),
    }
}

fn file_op(request: HostRequest) -> Result<BuiltinResult, RtError> {
    Ok(BuiltinResult::SuspendFile(request))
}

/// The file handle an argument names, with a negative one folded onto 0.
///
/// A handle nothing opened is not an error in Visual FoxPro: the call fails the way that
/// function fails, which is .F. for FFLUSH and FCLOSE, "" for FREAD, .T. for FEOF and 0 for
/// FSEEK and FWRITE - measured. No host hands out 0, so it is the handle that is never open.
fn handle_of(a: &[Value]) -> Result<u32, RtError> {
    let n = arg_int(a, 0)?;
    Ok(if n < 0 { 0 } else { n as u32 })
}

/// FOPEN(path [, attributes]): 0 read-only (the default), 1 write-only, 2 read and write; 10, 11
/// and 12 are the same unbuffered, which makes no difference here.
fn f_fopen(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let named = arg_str(&a, 0)?.to_string();
    let found = c.settings().search(&named);
    let path = c.settings().at(&named);
    let mode = opt_int(&a, 1, 0)? % 10;
    let mut r = op("open");
    if let HostRequest::FileOp { path: p, count, search, .. } = &mut r {
        *p = path;
        *count = mode as f64;
        *search = found;
    }
    file_op(r)
}

/// FCREATE(path [, attribute]): a new file, or an existing one emptied, open for reading and
/// writing. Hidden and system are not something a file has here, but read-only is measured: a
/// file made with any attribute other than 0 sets Windows' own read-only bit, which blocks
/// FPUTS() and FWRITE() to it - even after FCLOSE() and FOPEN() again - the same as it would on
/// a real disk, until something outside this file's own functions clears the bit.
fn f_fcreate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let path = c.settings().at(&arg_str(&a, 0)?);
    let attribute = opt_int(&a, 1, 0)?;
    let mut r = op("create");
    if let HostRequest::FileOp { path: p, count, .. } = &mut r {
        *p = path;
        *count = attribute as f64;
    }
    file_op(r)
}

fn f_fclose(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("close");
    if let HostRequest::FileOp { handle, .. } = &mut r {
        *handle = handle_of(&a)?;
    }
    file_op(r)
}

/// FREAD(handle, bytes): up to that many bytes from where the file is positioned.
fn f_fread(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("read");
    if let HostRequest::FileOp { handle, count, .. } = &mut r {
        *handle = handle_of(&a)?;
        *count = arg_int(&a, 1)?.max(0) as f64;
    }
    file_op(r)
}

/// FGETS(handle [, bytes]): one line, without its line ending; at most `bytes` of it (254 by
/// default, as VFP has it).
fn f_fgets(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("gets");
    if let HostRequest::FileOp { handle, count, .. } = &mut r {
        *handle = handle_of(&a)?;
        *count = opt_int(&a, 1, 254)?.max(0) as f64;
    }
    file_op(r)
}

/// FWRITE(handle, text [, bytes]): writes the text, or its first `bytes`; answers with how many
/// bytes were written.
fn f_fwrite(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let text = arg_str(&a, 1)?.to_string();
    let limit = opt_int(&a, 2, text.len() as i64)?.max(0) as usize;
    let mut r = op("write");
    if let HostRequest::FileOp { handle, text: t, .. } = &mut r {
        *handle = handle_of(&a)?;
        *t = text.chars().take(limit).collect();
    }
    file_op(r)
}

/// FPUTS(handle, text [, bytes]): FWRITE with a carriage return and line feed after it.
fn f_fputs(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let text = arg_str(&a, 1)?.to_string();
    let limit = opt_int(&a, 2, text.len() as i64)?.max(0) as usize;
    let mut r = op("write");
    if let HostRequest::FileOp { handle, text: t, .. } = &mut r {
        *handle = handle_of(&a)?;
        *t = format!("{}\r\n", text.chars().take(limit).collect::<String>());
    }
    file_op(r)
}

/// FSEEK(handle, offset [, from]): 0 from the start (the default), 1 from where it is, 2 from
/// the end. Answers with the new position.
fn f_fseek(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("seek");
    if let HostRequest::FileOp { handle, offset, whence, .. } = &mut r {
        *handle = handle_of(&a)?;
        *offset = arg_int(&a, 1)? as f64;
        *whence = opt_int(&a, 2, 0)?.clamp(0, 2) as u32;
    }
    file_op(r)
}

fn f_feof(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("eof");
    if let HostRequest::FileOp { handle, .. } = &mut r {
        *handle = handle_of(&a)?;
    }
    file_op(r)
}

fn f_fflush(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("flush");
    if let HostRequest::FileOp { handle, .. } = &mut r {
        *handle = handle_of(&a)?;
    }
    file_op(r)
}

/// FCHSIZE(handle, size): the file made that long; answers with the new size, or -1.
fn f_fchsize(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("chsize");
    if let HostRequest::FileOp { handle, count, .. } = &mut r {
        *handle = handle_of(&a)?;
        *count = arg_int(&a, 1)?.max(0) as f64;
    }
    file_op(r)
}

/// FERROR(): the error the last file function left, or 0.
fn f_ferror(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(c.ferror() as f64))
}

/// Joins `name` onto the folder of `base` and works out what `.` and `..` in it come to.
///
/// Nothing here touches the disk: a path is a path whether or not anything of that name exists,
/// which is why Visual FoxPro can answer for a file that is not there.
fn resolve_against(base: &str, name: &str) -> String {
    let separators = ['/', '\\'];
    // an absolute name ignores the base; a bare drive letter counts as absolute
    let absolute = name.starts_with(separators) || name.chars().nth(1) == Some(':');
    let folder = match base.rfind(separators) {
        Some(at) => &base[..=at],
        None => "",
    };
    let joined = if absolute { name.to_string() } else { format!("{folder}{name}") };
    let (drive, rest) = match joined.chars().nth(1) {
        Some(':') => joined.split_at(2),
        _ => ("", joined.as_str()),
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split(separators) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    // a name that ended in a separator names a folder, and keeps saying so
    let trailing = if name.ends_with(separators) || name.is_empty() { "\\" } else { "" };
    format!("{drive}\\{}{trailing}", parts.join("\\"))
}

/// FULLPATH(file [, relative to]): the file's path from the root, in upper case.
///
/// With a second argument nothing needs the disk: the answer is the name resolved against that
/// path's folder, `.` and `..` worked out. Without one the host says what the default directory
/// is, because only it knows. Visual FoxPro answers in upper case either way - measured.
fn f_fullpath(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.to_string();
    if let Some(base) = a.get(1) {
        let base = base.as_str()?.to_string();
        if !base.trim().is_empty() {
            return ok(Value::str(resolve_against(&base, &name).to_ascii_uppercase()));
        }
    }
    let path = c.settings().at(&name);
    let mut r = op("fullpath");
    if let HostRequest::FileOp { path: p, .. } = &mut r {
        *p = path;
    }
    file_op(r)
}

/// DEFAULTEXT(file, extension): the file with that extension when it has none. Pure string
/// work, and the extension may or may not come with its dot.
fn f_defaultext(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let file = arg_str(&a, 0)?.to_string();
    let ext = arg_str(&a, 1)?.trim_start_matches('.').to_string();
    let name = file.rsplit(['\\', '/']).next().unwrap_or(&file);
    if name.contains('.') || ext.is_empty() {
        return ok(Value::str(file));
    }
    ok(Value::str(format!("{file}.{ext}")))
}

/// DISKSPACE([drive]): free bytes on the drive of the default directory, or the one named.
fn f_diskspace(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let where_ = match a.first() {
        Some(v) => c.settings().at(&v.as_str()?),
        None => c.settings().at("."),
    };
    let mut r = op("diskspace");
    if let HostRequest::FileOp { path, .. } = &mut r {
        *path = where_;
    }
    file_op(r)
}

/// DRIVETYPE(drive): 2 removable, 3 fixed, 4 remote, 5 CD-ROM, 6 RAM disk, 1 when there is no
/// such drive.
fn f_drivetype(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut r = op("drivetype");
    if let HostRequest::FileOp { path, .. } = &mut r {
        *path = arg_str(&a, 0)?.to_string();
    }
    file_op(r)
}

/// The first extension `cFileExtensions` offers a name with no extension of its own.
///
/// The reference lets the argument be a bare extension (`"TXT"`), several separated by commas
/// or semicolons, or Windows' own `"Description:EXT"` pairing; a wildcard in any of them means
/// "whatever is there" rather than one name to try, which is not something asking for one file
/// can answer, so only a plain extension is worth trying here and the name is asked for exactly
/// as given otherwise - which still finds it whenever the caller wrote the extension in already.
fn first_extension(spec: &str) -> Option<String> {
    let first = spec.split([',', ';']).next()?.trim();
    let ext = first.rsplit(':').next().unwrap_or(first).trim();
    (!ext.is_empty() && !ext.contains(['*', '?'])).then(|| ext.to_string())
}

/// LOCFILE(file [, extensions [, caption]]): the file's full path when it is there. VFP shows a
/// dialog when it is not; here the answer is the error the program would get on opening it.
fn f_locfile(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.to_string();
    let extensions = opt_str(&a, 1, "")?;
    // a name that already has an extension is asked for exactly as given; one that does not
    // tries the first extension the second argument offers
    let candidate = match (name.rsplit(['\\', '/']).next().unwrap_or(&name).contains('.'), first_extension(&extensions)) {
        (false, Some(ext)) => format!("{name}.{ext}"),
        _ => name,
    };
    let found = c.settings().search(&candidate);
    let path = c.settings().at(&candidate);
    let mut r = op("locfile");
    if let HostRequest::FileOp { path: p, target, search, .. } = &mut r {
        *p = path;
        *target = extensions;
        *search = found;
    }
    file_op(r)
}

/// ADIR(array [, mask [, attributes [, nFlag]]]): fills the array with one row per file - name,
/// size, date, time, attributes - and answers with how many. The host lists; the array is filled
/// when the answer comes back and this runs again.
///
/// `attributes` including `D` brings the folder's own subdirectories into the same list, `.` and
/// `..` besides; `nFlag` of 1 asks for each name in the case it was actually given rather than
/// upper-cased, which is the default and the only other value this runtime tells apart from it.
fn f_adir(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(reply) = c.take_data_reply() else {
        let mask = match a.get(1) {
            Some(v) if !v.is_null() => v.as_str()?.to_string(),
            _ => "*.*".to_string(),
        };
        let mut r = op("dir");
        if let HostRequest::FileOp { path, target, whence, .. } = &mut r {
            *path = c.settings().at(&mask);
            *target = opt_str(&a, 2, "")?;
            *whence = opt_int(&a, 3, 0)?.clamp(0, 2) as u32;
        }
        return Ok(BuiltinResult::SuspendData { request: r, args: a });
    };
    let arr = super::array::array_of(&a[0])?;
    // the answer is the file reply: the listing, then the error number
    let listing = match reply.deref() {
        Value::Array(pair) => pair.borrow().items.first().cloned().unwrap_or(Value::Null).deref(),
        other => other,
    };
    let rows: Vec<Value> = match listing {
        Value::Array(list) => list.borrow().items.clone(),
        _ => Vec::new(),
    };
    let mut b = arr.borrow_mut();
    if rows.is_empty() {
        return ok(Value::number(0.0));
    }
    *b = FoxArray::new(rows.len(), 5);
    for (i, row) in rows.iter().enumerate() {
        if let Value::Array(cells) = row.deref() {
            for (j, cell) in cells.borrow().items.iter().take(5).enumerate() {
                b.items[i * 5 + j] = cell.clone();
            }
        }
    }
    ok(Value::number(rows.len() as f64))
}

/// What the directory says about one file: the row ADIR would have given for it.
///
/// The host answers a directory listing, and a name with no wildcards in it lists that one file,
/// which is where the size, the date and the time come from.
fn file_row(c: &mut dyn BuiltinCtx, a: &[Value]) -> Result<Option<Vec<Value>>, RtError> {
    let Some(reply) = c.take_data_reply() else {
        return Ok(None);
    };
    let listing = match reply.deref() {
        Value::Array(pair) => pair.borrow().items.first().cloned().unwrap_or(Value::Null).deref(),
        other => other,
    };
    let rows: Vec<Value> = match listing {
        Value::Array(list) => list.borrow().items.clone(),
        _ => Vec::new(),
    };
    let _ = a;
    Ok(Some(match rows.first().map(Value::deref) {
        Some(Value::Array(cells)) => cells.borrow().items.clone(),
        _ => Vec::new(),
    }))
}

/// Asks the host to list the one file a function was given.
fn ask_for_file(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = a.first().map(|v| v.as_str()).transpose()?.unwrap_or_default().to_string();
    let mut request = op("dir");
    if let HostRequest::FileOp { path, .. } = &mut request {
        *path = c.settings().at(&name);
    }
    Ok(BuiltinResult::SuspendData { request, args: a })
}

/// FDATE(cFileName [, 1]): the day the file was last written to, or the datetime with 1.
///
/// A file that is not there has no date to give, and Visual FoxPro says so with error 1 rather
/// than with an empty date - measured, and unlike FSIZE, which answers 0 for the same file.
fn f_fdate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(row) = file_row(c, &a)? else { return ask_for_file(c, a) };
    if row.is_empty() {
        return Err(RtError::new(RtError::FILE_NOT_FOUND, "File does not exist"));
    }
    let wants_datetime = a.get(1).map(Value::as_number).transpose()?.unwrap_or(0.0) == 1.0;
    let date = row.get(2).cloned().unwrap_or(Value::Date(None)).deref();
    if !wants_datetime {
        return ok(date);
    }
    let seconds = match (date, row.get(3).map(Value::deref)) {
        (Value::Date(Some(days)), Some(Value::Str(time))) => {
            let parts: Vec<f64> = time.split(':').map(|p| p.trim().parse::<f64>().unwrap_or(0.0)).collect();
            let hms = parts.first().copied().unwrap_or(0.0) * 3600.0 + parts.get(1).copied().unwrap_or(0.0) * 60.0;
            Some(f64::from(days) * 86_400.0 + hms + parts.get(2).copied().unwrap_or(0.0))
        }
        (Value::Date(Some(days)), _) => Some(f64::from(days) * 86_400.0),
        _ => None,
    };
    ok(Value::DateTime(seconds))
}

/// FTIME(cFileName): the time of day it was last written to, and error 1 when there is no file.
fn f_ftime(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(row) = file_row(c, &a)? else { return ask_for_file(c, a) };
    if row.is_empty() {
        return Err(RtError::new(RtError::FILE_NOT_FOUND, "File does not exist"));
    }
    ok(row.get(3).cloned().unwrap_or_else(|| Value::str("")).deref())
}

// FSIZE lives in builtins/data.rs, because it is a question about a field and not about a file:
// Visual FoxPro answers 0 for the name of a file that is sitting right there.

/// DIRECTORY(cDirectoryName): whether the folder is there.
fn f_directory(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Some(reply) = c.take_data_reply() else {
        let name = a.first().map(|v| v.as_str()).transpose()?.unwrap_or_default().to_string();
        let mut request = op("dir");
        if let HostRequest::FileOp { path, target, .. } = &mut request {
            *path = format!("{}/*.*", c.settings().at(&name).trim_end_matches(['/', '\\']));
            *target = "D".to_string();
        }
        return Ok(BuiltinResult::SuspendData { request, args: a });
    };
    // a folder that is not there lists nothing at all, and one that is lists at least itself
    let listing = match reply.deref() {
        Value::Array(pair) => pair.borrow().items.first().cloned().unwrap_or(Value::Null).deref(),
        other => other,
    };
    let found = matches!(listing, Value::Array(_));
    ok(Value::Logical(found))
}

/// JUSTDRIVE(cPath): the drive a path is on, with its colon and nothing else.
fn f_justdrive(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let path = a[0].as_str()?.to_string();
    let drive = match path.chars().nth(1) {
        Some(':') => path.chars().take(2).collect::<String>().to_ascii_uppercase(),
        _ => String::new(),
    };
    ok(Value::str(drive))
}

/// DISPLAYPATH(cPath, nLength): a path shortened to fit, with the middle left out.
///
/// Measured against Visual FoxPro: the drive stays, then `...\`, then as many of the folders
/// nearest the file as still fit; when not even one of them fits, the file name is shown on its
/// own; and nothing is ever cut in half. Anything narrower than ten is refused with error 11 - a
/// width that small could not hold a path a person could read.
///
/// `nLength` reads as optional on the reference page's own syntax line - no brackets around it -
/// unlike almost every other second argument in this family, and the product holds to that:
/// calling with the path alone measures as error 1229, "Too few arguments", not a default width.
/// The compiler's own arity check only knows the widest range a name takes, 1 to 2 here, so the
/// call gets through and this is where the miscount is actually caught.
fn f_displaypath(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    const NARROWEST: usize = 10;
    let path = a[0].as_str()?.to_string();
    let Some(width) = a.get(1) else { return Err(RtError::too_few_args()) };
    let width = width.as_number()?;
    if width < NARROWEST as f64 {
        return Err(RtError::function_arg_invalid());
    }
    let width = width as usize;
    if path.chars().count() <= width {
        return ok(Value::str(path));
    }
    let head: String = match path.chars().nth(1) {
        Some(':') => path.chars().take(2).collect(),
        _ => String::new(),
    };
    // the parts after the drive, nearest the file first, so the run that fits can be taken off
    let parts: Vec<&str> = path[head.len()..].split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    let mut kept = 0usize;
    for take in 1..=parts.len() {
        let tail = parts[parts.len() - take..].join("\\");
        // the drive, then the five characters of `\...\`, then the folders that fit
        if head.chars().count() + 5 + tail.chars().count() > width {
            break;
        }
        kept = take;
    }
    if kept == 0 {
        return ok(Value::str(parts.last().copied().unwrap_or(&path)));
    }
    ok(Value::str(format!("{head}\\...\\{}", parts[parts.len() - kept..].join("\\"))))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("ADIR", 1, 4, f_adir),
        spec("DEFAULTEXT", 2, 2, f_defaultext),
        spec("DIRECTORY", 1, 2, f_directory),
        spec("DISKSPACE", 0, 2, f_diskspace),
        spec("DISPLAYPATH", 1, 2, f_displaypath),
        spec("DRIVETYPE", 1, 1, f_drivetype),
        spec("FCHSIZE", 2, 2, f_fchsize),
        spec("FCLOSE", 1, 1, f_fclose),
        spec("FCREATE", 1, 2, f_fcreate),
        spec("FDATE", 1, 2, f_fdate),
        spec("FEOF", 1, 1, f_feof),
        spec("FERROR", 0, 0, f_ferror),
        spec("FFLUSH", 1, 2, f_fflush),
        spec("FGETS", 1, 2, f_fgets),
        spec("FOPEN", 1, 2, f_fopen),
        spec("FPUTS", 2, 3, f_fputs),
        spec("FREAD", 2, 2, f_fread),
        spec("FSEEK", 2, 3, f_fseek),
        spec("FTIME", 1, 1, f_ftime),
        spec("FULLPATH", 1, 2, f_fullpath),
        spec("FWRITE", 2, 3, f_fwrite),
        spec("JUSTDRIVE", 1, 1, f_justdrive),
        spec("LOCFILE", 1, 3, f_locfile),
    ]
}
