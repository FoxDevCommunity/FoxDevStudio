//! Array functions. Arrays arrive as `Value::Array` (possibly wrapped in a by-reference cell),
//! so every one of these mutates the array of the caller in place through the shared `RefCell`.

use std::cmp::Ordering;

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, VARIADIC, arg_int, ok, opt_int, spec};
use crate::error::RtError;
use crate::value::{ArrayRef, CmpOp, Settings, Value, compare};

/// The array behind an argument, following the by-reference cell a VFP array parameter arrives in.
pub(crate) fn array_of(v: &Value) -> Result<ArrayRef, RtError> {
    match v.deref() {
        Value::Array(a) => Ok(a),
        _ => Err(RtError::function_arg_invalid()),
    }
}

/// Rows of a 1-D array are its elements; `cols == 0` marks one dimension.
fn shape(a: &ArrayRef) -> (usize, usize, usize) {
    let b = a.borrow();
    (b.rows, b.cols, b.items.len())
}

fn cmp_values(x: &Value, y: &Value, settings: &Settings) -> Ordering {
    let less =
        |p: &Value, q: &Value| compare(p, q, CmpOp::Lt, settings).ok().and_then(|v| v.truthy().ok()).unwrap_or(false);
    if less(x, y) {
        Ordering::Less
    } else if less(y, x) {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// The same ordering, folding case on a character pair first - what ASORT()'s nFlags of 1 asks
/// for. Two values that differ only in case compare equal here and are then broken by case
/// alone, lowercase first: measured, `ASORT` of `("banana","Apple","cherry","apple")` with
/// flags 1 answers `"apple","Apple",...` - `apple` before `Apple`, the reverse of plain
/// byte order and of the two values' own input order, so neither a stable sort's tiebreak nor
/// a case-sensitive one explains it.
fn cmp_values_ci(x: &Value, y: &Value, settings: &Settings) -> Ordering {
    if let (Value::Str(p), Value::Str(q)) = (x.deref(), y.deref()) {
        let ord = p.to_lowercase().cmp(&q.to_lowercase());
        if ord != Ordering::Equal {
            return ord;
        }
        return cmp_values(y, x, settings);
    }
    cmp_values(x, y, settings)
}

/// ALEN(a [, n]): 0 or absent the element count, 1 the rows, 2 the columns.
fn f_alen(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let (rows, cols, len) = shape(&arr);
    ok(Value::number(match opt_int(&a, 1, 0)? {
        1 => rows as f64,
        2 => cols as f64,
        _ => len as f64,
    }))
}

/// ASCAN(a, v [, start] [, count] [, column] [, flags]): the 1-based element number of the
/// first match, or 0. Flag 1 compares exactly (ignoring SET EXACT), flag 4 returns the row.
fn f_ascan(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let needle = a[1].deref();
    let (_, cols, len) = shape(&arr);
    let start = opt_int(&a, 2, 1)?.max(1) as usize;
    let count = match opt_int(&a, 3, -1)? {
        n if n < 0 => len,
        n => n as usize,
    };
    let column = opt_int(&a, 4, 0)?.max(0) as usize;
    let flags = opt_int(&a, 5, 0)?;
    let op = if flags & 1 != 0 { CmpOp::ExactEq } else { CmpOp::Eq };
    let settings = c.settings().clone();
    let items = arr.borrow();
    for i in start..=len.min(start.saturating_add(count).saturating_sub(1)) {
        if column > 0 && cols > 0 && (i - 1) % cols + 1 != column {
            continue;
        }
        let hit =
            compare(&items.items[i - 1], &needle, op, &settings).ok().and_then(|v| v.truthy().ok()).unwrap_or(false);
        if hit {
            if flags & 4 != 0 && cols > 0 {
                return ok(Value::number(((i - 1) / cols + 1) as f64));
            }
            return ok(Value::number(i as f64));
        }
    }
    ok(Value::number(0.0))
}

/// ASORT(a [, start] [, count] [, order] [, flags]): order 0 ascending (default), any positive
/// value descending. Flags 1 is a case-insensitive sort - measured, not merely "compare equal
/// and let a stable sort decide": `ASORT` of `("banana","Apple","cherry","apple")` with flags 1
/// comes back `"apple","Apple","banana","cherry"`, `apple` before `Apple` though it started
/// after it, so ties are broken by `cmp_values_ci`'s own tiebreak and not left to input order.
/// A 2-D array is sorted by whole rows, keyed on the column of the start element.
fn f_asort(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let (rows, cols, len) = shape(&arr);
    let start = opt_int(&a, 1, 1)?.max(1) as usize;
    let count = opt_int(&a, 2, -1)?;
    let descending = opt_int(&a, 3, 0)? != 0;
    let case_insensitive = opt_int(&a, 4, 0)? != 0;
    let cmp = if case_insensitive { cmp_values_ci } else { cmp_values };
    let settings = c.settings().clone();
    let mut b = arr.borrow_mut();
    if cols == 0 {
        // measured: a start element past the array's own size is error 1234, the same
        // "Subscript is outside defined range" ASORT's own doc return value of -1 never
        // actually comes up for - not the numeric 1 a Logical(true) used to print as ".T.".
        if start > len {
            return Err(RtError::subscript_out_of_range());
        }
        let end = match count {
            n if n < 0 => len,
            n => (start + n as usize - 1).min(len),
        };
        if end < start {
            return ok(Value::number(1.0));
        }
        let slice = &mut b.items[start - 1..end];
        slice.sort_by(|x, y| cmp(x, y, &settings));
        if descending {
            slice.reverse();
        }
        return ok(Value::number(1.0));
    }
    let key = (start - 1) % cols;
    let first_row = (start - 1) / cols;
    if first_row >= rows {
        return Err(RtError::subscript_out_of_range());
    }
    let last_row = match count {
        n if n < 0 => rows,
        n => (first_row + n as usize).min(rows),
    };
    if last_row <= first_row {
        return ok(Value::number(1.0));
    }
    let mut block: Vec<Vec<Value>> =
        (first_row..last_row).map(|r| b.items[r * cols..(r + 1) * cols].to_vec()).collect();
    block.sort_by(|x, y| cmp(&x[key], &y[key], &settings));
    if descending {
        block.reverse();
    }
    for (i, row) in block.into_iter().enumerate() {
        let base = (first_row + i) * cols;
        b.items[base..base + cols].clone_from_slice(&row);
    }
    ok(Value::number(1.0))
}

/// ADEL(a, n [, flags]): removes an element (or, in a 2-D array, a row; flag 2 a column) and
/// leaves .F. in the freed slot. The array keeps its size, as in VFP.
fn f_adel(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let n = arg_int(&a, 1)?;
    let column_mode = opt_int(&a, 2, 0)? == 2;
    let (rows, cols, len) = shape(&arr);
    if n < 1 {
        return Err(RtError::invalid_subscript());
    }
    let n = n as usize;
    let mut b = arr.borrow_mut();
    if cols == 0 {
        if n > len {
            return Err(RtError::invalid_subscript());
        }
        b.items.remove(n - 1);
        b.items.push(Value::Logical(false));
    } else if column_mode {
        if n > cols {
            return Err(RtError::invalid_subscript());
        }
        for r in 0..rows {
            let base = r * cols;
            b.items.remove(base + n - 1);
            b.items.insert(base + cols - 1, Value::Logical(false));
        }
    } else {
        if n > rows {
            return Err(RtError::invalid_subscript());
        }
        let base = (n - 1) * cols;
        b.items.drain(base..base + cols);
        b.items.extend(std::iter::repeat_n(Value::Logical(false), cols));
    }
    ok(Value::number(1.0))
}

/// AINS(a, n [, flags]): inserts a blank element/row (flag 2: column) at n and drops what falls
/// off the end.
fn f_ains(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let n = arg_int(&a, 1)?;
    let column_mode = opt_int(&a, 2, 0)? == 2;
    let (rows, cols, len) = shape(&arr);
    if n < 1 {
        return Err(RtError::invalid_subscript());
    }
    let n = n as usize;
    let mut b = arr.borrow_mut();
    if cols == 0 {
        if n > len {
            return Err(RtError::invalid_subscript());
        }
        b.items.insert(n - 1, Value::Logical(false));
        b.items.truncate(len);
    } else if column_mode {
        if n > cols {
            return Err(RtError::invalid_subscript());
        }
        for r in 0..rows {
            let base = r * cols;
            b.items.insert(base + n - 1, Value::Logical(false));
            b.items.remove(base + cols);
        }
    } else {
        if n > rows {
            return Err(RtError::invalid_subscript());
        }
        let base = (n - 1) * cols;
        for _ in 0..cols {
            b.items.insert(base, Value::Logical(false));
        }
        b.items.truncate(len);
    }
    ok(Value::number(1.0))
}

/// ACOPY(src, dest [, start] [, count] [, dest start]): copies elements linearly, growing the
/// destination when it is too small (VFP requires it to be dimensioned first).
fn f_acopy(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let src = array_of(&a[0])?;
    let dest = array_of(&a[1])?;
    let (_, _, len) = shape(&src);
    let start = opt_int(&a, 2, 1)?.max(1) as usize;
    let count = match opt_int(&a, 3, -1)? {
        n if n < 0 => len.saturating_sub(start - 1),
        n => n as usize,
    };
    let dest_start = opt_int(&a, 4, 1)?.max(1) as usize;
    if start > len {
        return ok(Value::number(0.0));
    }
    let count = count.min(len - (start - 1));
    let values: Vec<Value> = src.borrow().items[start - 1..start - 1 + count].to_vec();
    let mut d = dest.borrow_mut();
    let needed = dest_start - 1 + count;
    if d.items.len() < needed {
        let (rows, cols) = (needed, d.cols);
        if cols == 0 {
            d.redim(rows, 0);
        } else {
            d.redim(needed.div_ceil(cols), cols);
        }
    }
    for (i, v) in values.into_iter().enumerate() {
        d.items[dest_start - 1 + i] = v;
    }
    ok(Value::number(count as f64))
}

/// AELEMENT(a, row [, col]): the linear element number of a subscript pair. Two args on a
/// 2-D array is not "row only, column missing" - row is used directly as the element number,
/// same as on a 1-D array, and it is checked against the row count (not the element count):
/// measured, `AELEMENT` of a row past the array's own rows is error 1234 even when the row
/// would still be a valid element number taken linearly.
fn f_aelement(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let (rows, cols, len) = shape(&arr);
    let row = arg_int(&a, 1)?;
    let col = opt_int(&a, 2, 0)?;
    if row < 1 {
        return Err(RtError::subscript_out_of_range());
    }
    if col <= 0 || cols == 0 {
        let bound = if cols == 0 { len } else { rows };
        if row as usize > bound {
            return Err(RtError::subscript_out_of_range());
        }
        return ok(Value::number(row as f64));
    }
    if row as usize > rows || col as usize > cols {
        return Err(RtError::subscript_out_of_range());
    }
    ok(Value::number(((row as usize - 1) * cols + col as usize) as f64))
}

/// ASUBSCRIPT(a, n, which): which 1 the row, 2 the column of element n.
fn f_asubscript(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let (_, cols, len) = shape(&arr);
    let n = arg_int(&a, 1)?;
    let which = arg_int(&a, 2)?;
    if n < 1 || n as usize > len {
        return Err(RtError::invalid_subscript());
    }
    let n = n as usize;
    if cols == 0 {
        return match which {
            1 => ok(Value::number(n as f64)),
            _ => Err(RtError::invalid_subscript()),
        };
    }
    match which {
        1 => ok(Value::number(((n - 1) / cols + 1) as f64)),
        2 => ok(Value::number(((n - 1) % cols + 1) as f64)),
        _ => Err(RtError::invalid_subscript()),
    }
}

/// AEMPTY(a): drops every element, leaving a zero-length array. VFP 9 spells it AEMPTYNEW as
/// well; both names run this. Returns 0, the new element count.
fn f_aempty(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = array_of(&a[0])?;
    let mut b = arr.borrow_mut();
    b.rows = 0;
    b.cols = 0;
    b.items.clear();
    ok(Value::number(0.0))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("ACLASS", 2, 2, f_aclass),
        spec("ACOPY", 2, 5, f_acopy),
        spec("ADEL", 2, 3, f_adel),
        spec("ADLLS", 1, 2, f_adlls),
        spec("ADOCKSTATE", 1, 1, f_adockstate),
        spec("AELEMENT", 2, 3, f_aelement),
        spec("AEMPTY", 1, 1, f_aempty),
        spec("AEMPTYNEW", 1, 1, f_aempty),
        spec("AEVENTS", 1, 2, f_aevents),
        spec("AGETCLASS", 1, 7, f_agetclass),
        spec("AGETFILEVERSION", 2, 2, f_agetfileversion),
        spec("AINS", 2, 3, f_ains),
        spec("AINSTANCE", 2, 2, f_ainstance),
        spec("ALANGUAGE", 1, 2, f_alanguage),
        spec("ALEN", 1, 2, f_alen),
        spec("AMOUSEOBJ", 1, 2, f_amouseobj),
        spec("ANETRESOURCES", 1, 3, f_anetresources),
        spec("APROCINFO", 2, 3, f_aprocinfo),
        spec("ASCAN", 2, VARIADIC, f_ascan),
        spec("ASELOBJ", 1, 2, f_aselobj),
        spec("ASORT", 1, 5, f_asort),
        spec("ASQLHANDLES", 1, 2, f_asqlhandles),
        spec("ASUBSCRIPT", 3, 3, f_asubscript),
        spec("AVCXCLASSES", 2, 2, f_avcxclasses),
    ]
}

// ------------------------------------------------------------------------------------------
// the arrays that report on the program itself
// ------------------------------------------------------------------------------------------

/// Fills the array with rows and answers how many there are, which is what every `A...()`
/// function does with what it found. A row of one value makes a one-dimensional array, as the
/// reference says of each of them.
pub(super) fn fill(target: &Value, rows: Vec<Vec<Value>>) -> Result<BuiltinResult, RtError> {
    // an empty answer leaves the array as it was, which is what the reference says of
    // AINSTANCE and what the rest are read as doing
    if rows.is_empty() {
        return ok(Value::number(0.0));
    }
    let array = array_of(target)?;
    let cols = rows.iter().map(Vec::len).max().unwrap_or(1);
    {
        let mut a = array.borrow_mut();
        a.redim(rows.len(), if cols > 1 { cols } else { 0 });
        for (r, row) in rows.iter().enumerate() {
            for (c, value) in row.iter().enumerate() {
                let at = if cols > 1 { r * cols + c } else { r };
                if let Some(slot) = a.items.get_mut(at) {
                    *slot = value.clone();
                }
            }
        }
    }
    ok(Value::number(rows.len() as f64))
}

/// The rows of an answer the host sent back: an array of arrays, or of single values.
fn rows_of(reply: &Value) -> Vec<Vec<Value>> {
    match reply.deref() {
        Value::Array(a) => a
            .borrow()
            .items
            .iter()
            .map(|row| match row.deref() {
                Value::Array(inner) => inner.borrow().items.clone(),
                other => vec![other],
            })
            .collect(),
        Value::Null => Vec::new(),
        other => vec![vec![other]],
    }
}

/// Asks the host to list something it holds, and fills the array with what comes back.
fn enumerated(c: &mut dyn BuiltinCtx, a: &[Value], what: u8, name: String) -> Result<BuiltinResult, RtError> {
    match c.take_data_reply() {
        Some(reply) => fill(&a[0], rows_of(&reply)),
        // the instruction runs again with the reply in hand, because the answer goes into
        // the array the call named rather than being the value of the call
        None => Ok(BuiltinResult::SuspendData {
            request: crate::host::HostRequest::Enumerate { what, name },
            args: a.to_vec(),
        }),
    }
}

/// `ACLASS(a, oObject)`: the object's class, then the class that came from, and so on down to
/// the base class it is built on.
fn f_aclass(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Value::Object(handle) = a[1].deref() else { return Err(RtError::function_arg_invalid()) };
    let word = |c: &mut dyn BuiltinCtx, name: &str| {
        c.host().get_prop(handle, name).ok().and_then(|v| v.as_str().ok().map(|s| s.to_string()))
    };
    let base = word(c, "BASECLASS");
    let mut name = word(c, "CLASS");
    let mut rows = Vec::new();
    while let Some(class) = name.filter(|c| !c.is_empty()) {
        rows.push(vec![Value::str(class.clone())]);
        if base.as_deref().is_some_and(|b| b.eq_ignore_ascii_case(&class)) || rows.len() > 64 {
            break;
        }
        let parent = word(c, "PARENTCLASS");
        // a class whose parent is itself has nothing further above it but the base class
        name = match parent {
            Some(p) if p.eq_ignore_ascii_case(&class) => base.clone(),
            Some(p) if !p.is_empty() => Some(p),
            _ => base.clone(),
        };
    }
    fill(&a[0], rows)
}

/// `ADLLS(a [, cLibrary])`: the library functions the program has declared.
fn f_adlls(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let only = match a.get(1) {
        Some(v) => v.deref().as_str()?.to_string(),
        None => String::new(),
    };
    let rows: Vec<Vec<Value>> = c
        .dlls()
        .into_iter()
        .filter(|(_, library, _)| only.is_empty() || library.eq_ignore_ascii_case(&only))
        .map(|(called, library, function)| vec![Value::str(called), Value::str(library), Value::str(function)])
        .collect();
    fill(&a[0], rows)
}

/// `ADOCKSTATE(a)`: where the docked windows are. Nothing docks in this runtime, so there is
/// nothing to report and the array is left as it was.
fn f_adockstate(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    fill(&a[0], Vec::new())
}

/// `ASQLHANDLES(a [, nHandle])`: the connections SQLCONNECT() has open, of which there are none
/// until a program opens one. Asking about a specific handle when none is open is not "0
/// handles found" the way the bare form answers it - measured, any nHandle at all, valid-looking
/// or not, is error 1466 ("Connection handle is invalid") when nothing has ever connected.
fn f_asqlhandles(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a.get(1).is_some() {
        return Err(RtError::connection_handle_invalid());
    }
    fill(&a[0], Vec::new())
}

fn f_ainstance(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let class = a.get(1).map(Value::deref).and_then(|v| v.as_str().ok().map(|s| s.to_string())).unwrap_or_default();
    enumerated(c, &a, 0, class)
}

fn f_aevents(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let of = match a.get(1).map(Value::deref) {
        Some(Value::Object(h)) => h.0.to_string(),
        Some(Value::Number(n, ..)) => format!("{}", n as i64),
        _ => "1".to_string(),
    };
    enumerated(c, &a, 1, of)
}

fn f_aselobj(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = opt_int(&a, 1, 0)?;
    enumerated(c, &a, 2, which.to_string())
}

fn f_amouseobj(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = opt_int(&a, 1, 0)?;
    enumerated(c, &a, 3, which.to_string())
}

fn f_agetclass(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let library = match a.get(1) {
        Some(v) => v.deref().as_str().map(|s| s.to_string()).unwrap_or_default(),
        None => String::new(),
    };
    enumerated(c, &a, 4, library)
}

fn f_avcxclasses(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let library = a.get(1).map(Value::deref).and_then(|v| v.as_str().ok().map(|s| s.to_string())).unwrap_or_default();
    enumerated(c, &a, 5, library)
}

fn f_agetfileversion(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let path = a.get(1).map(Value::deref).and_then(|v| v.as_str().ok().map(|s| s.to_string())).unwrap_or_default();
    enumerated(c, &a, 6, path)
}

fn f_anetresources(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let what = a.get(1).map(Value::deref).and_then(|v| v.as_str().ok().map(|s| s.to_string())).unwrap_or_default();
    enumerated(c, &a, 7, what)
}

/// `ALANGUAGE(a, nType)`: the language itself - its commands, its functions, its base classes
/// and the events a database container raises.
/// ALANGUAGE(ArrayName, nType): the reference page's syntax line never brackets `nType`, and
/// measured, the product means it - `ALANGUAGE(a)` with the type left out is error 1229 ("Too
/// few arguments"), not nType defaulting to 1 the way the registry's lenient 1-2 arity would
/// otherwise let it.
fn f_alanguage(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if a.len() < 2 {
        return Err(RtError::too_few_args());
    }
    match opt_int(&a, 1, 1)? {
        1 => {
            let rows = crate::parser::command_verbs().iter().map(|v| vec![Value::str(*v)]).collect();
            fill(&a[0], rows)
        }
        2 => {
            // the name, then how many arguments it needs and how many it takes at most
            let rows = super::registry()
                .iter()
                .map(|spec| {
                    let most = if spec.max_args == VARIADIC { "*".to_string() } else { spec.max_args.to_string() };
                    vec![Value::str(spec.name), Value::str(format!("{}-{most}", spec.min_args))]
                })
                .collect();
            fill(&a[0], rows)
        }
        4 => {
            let rows = crate::vm::DBC_EVENTS.iter().map(|e| vec![Value::str(*e)]).collect();
            fill(&a[0], rows)
        }
        // the base classes are the host's to list: it is what makes one when a program asks
        _ => enumerated(c, &a, 8, String::new()),
    }
}

/// `APROCINFO(a, cFile [, nType])`: what is in a program file - its classes, its procedures and
/// the directives before them.
fn f_aprocinfo(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let path = a.get(1).map(Value::deref).and_then(|v| v.as_str().ok().map(|s| s.to_string())).unwrap_or_default();
    let kind = opt_int(&a, 2, 0)?;
    let Some(reply) = c.take_data_reply() else {
        return Ok(BuiltinResult::SuspendData { request: crate::host::HostRequest::FileRead { path, search: Vec::new() }, args: a.to_vec() });
    };
    let source = reply.as_str().map(|s| s.to_string()).unwrap_or_default();
    fill(&a[0], crate::parser::outline(&source, kind as u8))
}
