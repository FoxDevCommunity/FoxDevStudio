//! Functions that report on the work areas: where the record pointer is, what is open, what the
//! table's columns are.
//!
//! None of these touch the disk. Everything they answer is either header information the cursor
//! kept from `USE` or the record it has already decoded, so they never suspend.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, arg_num, arg_str, ok, spec};

/// ADATABASES(array): the open databases, one row each with its name and its path.
///
/// Every container that is open is counted, current or not: measured, `USE <path>\testdata!
/// products` with nothing open reads the container to find the table, leaves it open, and
/// `ADATABASES()` answers 1 while `DBC()` is still empty. As in VFP, an array that would hold
/// nothing is left as it was.
fn f_adatabases(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let rows: Vec<(String, String)> =
        c.databases().iter().filter(|d| d.open).map(|d| (d.name().to_ascii_uppercase(), d.path.clone())).collect();
    let target = super::array::array_of(&a[0])?;
    if !rows.is_empty() {
        let mut array = target.borrow_mut();
        array.redim(rows.len(), 2);
        for (i, (name, path)) in rows.iter().enumerate() {
            if let Some(slot) = array.items.get_mut(i * 2) {
                *slot = Value::str(name.clone());
            }
            if let Some(slot) = array.items.get_mut(i * 2 + 1) {
                *slot = Value::str(path.clone());
            }
        }
    }
    ok(Value::number(rows.len() as f64))
}
use crate::data::{AreaRef, Cursor, PAGE_RECORDS, bytes_of};
use crate::host::HostRequest;
use crate::error::RtError;
use crate::value::{Value, Width};
use crate::vm::DbcArg;

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("ADATABASES", 1, 1, f_adatabases),
        spec("ADBOBJECTS", 2, 2, f_adbobjects),
        spec("AFIELDS", 1, 2, f_afields),
        spec("ALIAS", 0, 1, f_alias),
        spec("ATAGINFO", 1, 3, f_ataginfo),
        spec("AUSED", 1, 2, f_aused),
        spec("BOF", 0, 1, f_bof),
        spec("CANDIDATE", 0, 2, f_candidate),
        spec("CDX", 0, 2, f_cdx),
        spec("CLEARRESULTSET", 0, 0, f_clearresultset),
        spec("CPDBF", 0, 1, f_cpdbf),
        spec("CREATEOFFLINE", 1, 2, f_createoffline),
        spec("CURSORGETPROP", 1, 2, f_cursorgetprop),
        spec("CURSORSETPROP", 1, 3, f_cursorsetprop),
        spec("CURVAL", 1, 2, f_curval),
        spec("DBC", 0, 0, f_dbc),
        spec("DBF", 0, 1, f_dbf),
        spec("DBGETPROP", 3, 3, f_dbgetprop),
        spec("DBSETPROP", 4, 4, f_dbsetprop),
        spec("DBUSED", 1, 1, f_dbused),
        spec("DELETED", 0, 1, f_deleted),
        spec("DESCENDING", 0, 2, f_descending),
        spec("DROPOFFLINE", 1, 1, f_dropoffline),
        spec("EOF", 0, 1, f_eof),
        spec("FCOUNT", 0, 1, f_fcount),
        spec("FIELD", 1, 2, f_field),
        spec("FILTER", 0, 1, f_filter),
        spec("FLDLIST", 0, 1, f_fldlist),
        spec("FLOCK", 0, 1, f_flock),
        spec("FOR", 0, 3, f_for),
        spec("FOUND", 0, 1, f_found),
        spec("FSIZE", 1, 2, f_fsize),
        spec("GETAUTOINCVALUE", 0, 1, f_getautoincvalue),
        spec("GETCURSORADAPTER", 1, 1, f_getcursoradapter),
        spec("GETFLDSTATE", 1, 2, f_getfldstate),
        spec("GETNEXTMODIFIED", 1, 2, f_getnextmodified),
        spec("GETRESULTSET", 0, 0, f_getresultset),
        spec("HEADER", 0, 1, f_header),
        spec("IDXCOLLATE", 0, 3, f_idxcollate),
        spec("INDBC", 2, 2, f_indbc),
        spec("INDEXSEEK", 1, 4, f_indexseek),
        spec("ISEXCLUSIVE", 0, 2, f_isexclusive),
        spec("ISFLOCKED", 0, 1, f_isflocked),
        spec("ISMEMOFETCHED", 1, 2, f_ismemofetched),
        spec("ISREADONLY", 0, 1, f_isreadonly),
        spec("ISRLOCKED", 0, 2, f_isrlocked),
        spec("ISTRANSACTABLE", 1, 1, f_istransactable),
        spec("KEY", 0, 2, f_key),
        spec("KEYMATCH", 1, 3, f_keymatch),
        spec("LOCK", 0, 2, f_rlock),
        spec("LOOKUP", 3, 4, f_lookup),
        spec("LUPDATE", 0, 1, f_lupdate),
        spec("MAKETRANSACTABLE", 1, 1, f_maketransactable),
        spec("MDX", 1, 2, f_mdx),
        spec("NDX", 0, 2, f_ndx),
        spec("OLDVAL", 1, 2, f_oldval),
        spec("ORDER", 0, 2, f_order),
        spec("PRIMARY", 0, 2, f_primary),
        spec("RECCOUNT", 0, 1, f_reccount),
        spec("RECNO", 0, 1, f_recno),
        spec("RECSIZE", 0, 1, f_recsize),
        spec("REFRESH", 0, 3, f_refresh),
        spec("RELATION", 1, 2, f_relation),
        spec("REQUERY", 0, 1, f_requery),
        spec("RLOCK", 0, 2, f_rlock),
        spec("SEEK", 1, 3, f_seek),
        spec("SELECT", 0, 1, f_select),
        spec("SETFLDSTATE", 2, 3, f_setfldstate),
        spec("SETRESULTSET", 1, 1, f_setresultset),
        spec("TABLEREVERT", 0, 2, f_tablerevert),
        spec("TABLEUPDATE", 0, 3, f_tableupdate),
        spec("TAG", 0, 3, f_tag),
        spec("TAGCOUNT", 0, 2, f_tagcount),
        spec("TAGNO", 0, 3, f_tagno),
        spec("TARGET", 1, 2, f_target),
        spec("UNIQUE", 0, 2, f_unique),
        spec("UPDATED", 0, 0, f_updated),
        spec("USED", 0, 1, f_used),
    ]
}

/// The work area a function is asked about: the selected one when nothing is named, else the
/// number or alias given. VFP accepts either in the same argument.
fn area(a: &[Value]) -> Result<Option<AreaRef>, RtError> {
    match a.first() {
        None => Ok(None),
        Some(Value::Number(n, ..)) => Ok(Some(AreaRef::Number(*n as usize))),
        Some(v) => Ok(Some(AreaRef::Alias(v.as_str()?.to_string()))),
    }
}

/// One work area, named the way an argument names it.
fn area_of(v: &Value) -> Result<Option<AreaRef>, RtError> {
    area(std::slice::from_ref(v))
}

/// Runs `f` against the cursor a function was asked about. A closed work area is not an error:
/// VFP answers with the empty value, which is what keeps `IF EOF("customer")` usable.
fn on_cursor<T>(c: &dyn BuiltinCtx, a: &[Value], empty: T, f: impl FnOnce(&Cursor) -> T) -> Result<T, RtError> {
    let data = c.data();
    let cursor = match area(a)? {
        None => data.cursor(),
        Some(what) => data.find(&what),
    };
    Ok(cursor.map_or(empty, f))
}

// ----- indexes -------------------------------------------------------------------------------
//
// Every one of these reads the tags the cursor holds, which arrived when the table was opened.
// VFP lets each of them name an index file as well as a work area, because a table can have
// several open at once; this runtime opens the one beside the table, so a file name is accepted
// and passed over, and what is left is the tag number and the work area.

/// The arguments of an index function once the index-file name has been dropped: which tag is
/// meant (`None` is the controlling one) and which work area.
fn tag_args(a: &[Value]) -> Result<(Option<usize>, Vec<Value>), RtError> {
    let mut rest: Vec<Value> = a.to_vec();
    // a leading string is the index file, which is the one beside the table whatever it says
    if matches!(rest.first().map(Value::deref), Some(Value::Str(_))) {
        rest.remove(0);
    }
    let which = match rest.first().map(Value::deref) {
        Some(Value::Number(n, ..)) => {
            rest.remove(0);
            let n = n as i64;
            if n < 1 {
                return Err(RtError::function_arg_invalid());
            }
            Some(n as usize - 1)
        }
        _ => None,
    };
    Ok((which, rest))
}

/// The arguments of `TAGCOUNT([cCDXFileName [, area]])`, and of `TAGNO()` once its own tag name
/// has been read off: an index-file name and nothing past it but the work area. Unlike
/// `tag_args`, there is no tag number in between, so a work area given as a bare number must
/// stay one - measured against vfp9.exe, which raises error 11 for `TAGCOUNT(2)` rather than
/// reading the 2 as an index number the way `CDX(2)` would.
fn cdxname_args(a: &[Value]) -> Result<Vec<Value>, RtError> {
    if a.is_empty() {
        return Ok(Vec::new());
    }
    if !matches!(a[0].deref(), Value::Str(_)) {
        return Err(RtError::function_arg_invalid());
    }
    Ok(a[1..].to_vec())
}

/// The tag an index function is asked about: the one numbered, or the controlling order.
fn on_tag<T>(c: &dyn BuiltinCtx, a: &[Value], empty: T, f: impl FnOnce(&crate::cdx::Tag) -> T) -> Result<T, RtError> {
    let (which, rest) = tag_args(a)?;
    on_cursor(c, &rest, None, |cur| match which {
        Some(i) => cur.tags().get(i).cloned(),
        None => cur.order().cloned(),
    })
    .map(|tag| tag.as_ref().map_or(empty, f))
}

/// ORDER([nWorkArea | cTableAlias [, nPath]]): the tag the table is in the order of, or the
/// empty string when it is in record order.
///
/// The first argument names a work area directly - it is never an index-file name the way the
/// tag functions' first argument can be, so this does not go through `tag_args`. `nPath`, when
/// it is anything but zero, switches the answer from the tag's own name to the compound index
/// file that holds it: measured, `ORDER("dept", 1)` answers `dept.cdx` where `ORDER("dept")`
/// answers `BYCODE`.
fn f_order(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let area_arg: Vec<Value> = a.first().cloned().into_iter().collect();
    let want_path = a.get(1).map(|v| v.as_number()).transpose()?.is_some_and(|n| n != 0.0);
    if want_path {
        let path = on_cursor(c, &area_arg, String::new(), |cur| {
            if cur.order().is_none() || cur.cdx_tags().is_empty() {
                return String::new();
            }
            let stem = cur.path.rfind('.').map_or(cur.path.as_str(), |dot| &cur.path[..dot]);
            format!("{stem}.CDX")
        })?;
        return ok(Value::str(path));
    }
    let name = on_cursor(c, &area_arg, String::new(), |cur| cur.order().map(|t| t.name.clone()).unwrap_or_default())?;
    ok(Value::str(name))
}

/// TAG([nTag [, area]]): the name of a tag by number, or of the controlling one.
fn f_tag(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(on_tag(c, &a, String::new(), |t| t.name.clone())?))
}

/// TAGCOUNT([cCDXFileName [, area]]): how many tags the index beside the table holds.
fn f_tagcount(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let rest = cdxname_args(&a)?;
    ok(Value::number(on_cursor(c, &rest, 0.0, |cur| cur.tags().len() as f64)?))
}

/// TAGNO([cTag [, cCDXFileName [, area]]]): the number of a tag, or of the controlling order.
/// 0 is record order.
fn f_tagno(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let named = match a.first().map(Value::deref) {
        Some(Value::Str(s)) => Some(s.trim().to_string()),
        _ => None,
    };
    let rest: Vec<Value> = if named.is_some() { a[1..].to_vec() } else { a.clone() };
    // once the tag's own name is off, what is left is a .cdx file name and then the work area -
    // there is no tag number in between, unlike the rest of this family
    let rest = cdxname_args(&rest)?;
    let n = on_cursor(c, &rest, 0.0, |cur| match &named {
        Some(name) => cur.tag_index(name).map_or(0.0, |i| (i + 1) as f64),
        None => cur.order_no().map_or(0.0, |n| n as f64),
    })?;
    ok(Value::number(n))
}

/// KEY([nTag [, area]]): the expression a tag is built from.
fn f_key(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(on_tag(c, &a, String::new(), |t| crate::cdx::compiled_expr(&t.key_expr))?))
}

/// DESCENDING(), UNIQUE(), CANDIDATE(), PRIMARY(): what kind of tag it is.
fn f_descending(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_tag(c, &a, false, |t| t.descending)?))
}

fn f_unique(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_tag(c, &a, false, |t| t.unique)?))
}

fn f_candidate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_tag(c, &a, false, |t| t.candidate)?))
}

/// PRIMARY(): a primary key is a candidate key of a table in a database, and this runtime has
/// no databases yet, so no tag is one.
fn f_primary(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    on_tag(c, &a, false, |_| false)?;
    ok(Value::Logical(false))
}

/// CDX(): the compound index beside the table, which is the table's own name with .CDX.
fn f_cdx(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (which, rest) = tag_args(&a)?;
    // there is one compound index per table here, so only the first is answered for
    let path = on_cursor(c, &rest, String::new(), |cur| {
        if which.unwrap_or(0) > 0 || cur.cdx_tags().is_empty() {
            String::new()
        } else {
            // measured against vfp9.exe: CDX() and MDX() answer with the file name alone, no
            // directory - the same as NDX() does - and in upper case, whatever case the table's
            // own name was written in
            let leaf = cur.path.rsplit(['/', '\\']).next().unwrap_or(&cur.path);
            let stem = leaf.rfind('.').map_or(leaf, |dot| &leaf[..dot]);
            format!("{stem}.CDX").to_ascii_uppercase()
        }
    })?;
    ok(Value::str(path))
}

/// NDX(nIndex): the single-entry index files the table has open, in the order they were
/// opened. Compound index files are not counted, which is what CDX() is for.
fn f_ndx(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (which, rest) = tag_args(&a)?;
    let path = on_cursor(c, &rest, String::new(), |cur| {
        cur.idx_files().get(which.unwrap_or(0)).map(|f| f.to_ascii_uppercase()).unwrap_or_default()
    })?;
    ok(Value::str(path))
}

/// ATAGINFO(array [, cCDX [, area]]): a row per tag - its name, what kind it is, the key
/// expression, the FOR condition and which way it runs.
fn f_ataginfo(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // a[1] names a .cdx file, which is passed over the way the rest of the family does, so what
    // is left for `on_cursor` is only the work area
    let rest = cdxname_args(&a[1..])?;
    let tags = on_cursor(c, &rest, Vec::new(), |cur| cur.tags().to_vec())?;
    let target = crate::builtins::array::array_of(&a[0])?;
    {
        let mut array = target.borrow_mut();
        array.redim(tags.len().max(1), 5);
        for (row, tag) in tags.iter().enumerate() {
            let kind = if tag.candidate {
                "CANDIDATE"
            } else if tag.unique {
                "UNIQUE"
            } else {
                "REGULAR"
            };
            let cells = [
                Value::str(tag.name.clone()),
                Value::str(kind),
                // the key column comes back off the compiled expression, the filter column
                // off what the index file holds; that is the product's own inconsistency
                Value::str(crate::cdx::compiled_expr(&tag.key_expr)),
                Value::str(tag.for_expr.clone()),
                Value::str(if tag.descending { "DESCENDING" } else { "ASCENDING" }),
            ];
            for (col, value) in cells.into_iter().enumerate() {
                if let Some(slot) = array.items.get_mut(row * 5 + col) {
                    *slot = value;
                }
            }
        }
    }
    ok(Value::number(tags.len() as f64))
}

/// SEEK(eKey [, area [, cTag]]): the same search the SEEK command makes, as a function. It
/// always moves the record pointer, whether the key is found or not - to the match, or to end
/// of file (or the nearest key, with `SET NEAR ON`) when it is not.
fn f_seek(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    search(c, &a[0], a.get(1), a.get(2).cloned(), true, true)
}

/// INDEXSEEK(eKey [, lMove [, area [, cTag]]]): the search that leaves the record pointer where
/// it was unless the key is found and `lMove` says to go there.
///
/// Measured against vfp9.exe: a miss never moves the pointer, whatever `lMove` says - only a
/// `SEEK()` failure does that. `INDEXSEEK("x", .T.)` against a key nothing answers to is the
/// same as leaving `lMove` off.
fn f_indexseek(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let move_pointer = a.get(1).map(Value::truthy).transpose()?.unwrap_or(false);
    search(c, &a[0], a.get(2), a.get(3).cloned(), move_pointer, false)
}

/// The search both of those make: down a tag, in a work area, putting everything back
/// afterwards except where `on_found`/`on_miss` say the pointer belongs once the search is
/// done - `SEEK()` always moves either way, `INDEXSEEK()` only on a found key that `lMove` asks
/// to move to.
fn search(
    c: &mut dyn BuiltinCtx,
    key: &Value,
    area: Option<&Value>,
    tag: Option<Value>,
    on_found: bool,
    on_miss: bool,
) -> Result<BuiltinResult, RtError> {
    let near = c.settings().near;
    let target = match area { Some(v) => area_of(v)?, None => None };
    let data = c.data_mut();
    let cursor = match &target {
        None => data.cursor_mut(),
        Some(what) => data.find_mut(what),
    };
    let cursor = cursor.ok_or_else(RtError::no_table_open)?;
    let was = (cursor.recno(), cursor.order_no());
    // a named tag is searched down without becoming the table's order for good
    if let Some(name) = tag {
        let name = name.as_str()?.trim().to_string();
        let i = cursor
            .tag_index(&name)
            .ok_or_else(RtError::tag_not_found)?;
        cursor.set_order(Some(i));
    }
    let found = crate::data::seek_in_order(cursor, key, near)?;
    cursor.set_found(found);
    if !(if found { on_found } else { on_miss }) {
        cursor.seek(was.0);
    }
    if was.1 != cursor.order_no() {
        cursor.set_order(was.1.map(|n| n - 1));
    }
    ok(Value::Logical(found))
}

// ----- what a table is, and what is in it ----------------------------------------------------

/// AUSED(aArray [, nDataSession]): the work areas in use, an alias and a number per row.
fn f_aused(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let mut rows: Vec<(String, f64)> = Vec::new();
    let data = c.data();
    // the highest-numbered work area was, in the ordinary way of opening tables, the last one
    // opened - and the array lists them in reverse of opening order, measured against vfp9.exe
    for area in (1..=data.highest_free()).rev() {
        if let Some(cursor) = data.find(&AreaRef::Number(area)) {
            rows.push((cursor.alias.to_uppercase(), area as f64));
        }
    }
    let target = crate::builtins::array::array_of(&a[0])?;
    {
        let mut array = target.borrow_mut();
        array.redim(rows.len().max(1), 2);
        for (row, (alias, area)) in rows.iter().enumerate() {
            if let Some(slot) = array.items.get_mut(row * 2) {
                *slot = Value::str(alias.clone());
            }
            if let Some(slot) = array.items.get_mut(row * 2 + 1) {
                *slot = Value::number(*area);
            }
        }
    }
    ok(Value::number(rows.len() as f64))
}

/// RECSIZE([area]) and HEADER([area]): how long a record is, and how long the header before them.
fn f_recsize(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(on_cursor(c, &a, 0.0, |cur| cur.header.record_len as f64)?))
}

fn f_header(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(on_cursor(c, &a, 0.0, |cur| cur.header.header_len as f64)?))
}

/// LUPDATE([area]): the day the table was last written to, as its header records it.
fn f_lupdate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(on_cursor(c, &a, Value::Date(None), |cur| match cur.header.last_update {
        Some(days) => Value::Date(Some(days)),
        None => Value::Date(None),
    })?)
}

/// FLDLIST([nField]): the fields SET FIELDS named, as `ALIAS.FIELD` a comma apart, or one of
/// them by number. It answers with nothing at all when no SET FIELDS has named any - the list
/// is what a program said it wanted, not what the table holds.
fn f_fldlist(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = a.first().map(Value::as_number).transpose()?.unwrap_or(0.0) as usize;
    // the list SET FIELDS named, whether or not SET FIELDS is ON: the switch says whether the
    // rest of the language is held to the list, not whether the list can be read back
    let names = &c.settings().fields;
    let text = match which {
        0 => names.join(","),
        n => names.get(n - 1).cloned().unwrap_or_default(),
    };
    ok(Value::str(text))
}

/// ISREADONLY([area]) and ISEXCLUSIVE([area]): how the table was opened.
fn f_isreadonly(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // nothing here opens a table it cannot write to, so a table that is open can be written
    ok(Value::Logical(!on_cursor(c, &a, false, |_| true)?))
}

fn f_isexclusive(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // one program has the table to itself here, which is what exclusive use means
    ok(Value::Logical(on_cursor(c, &a, false, |_| true)?))
}

/// CURVAL(cField [, area]) and OLDVAL(cField [, area]): what the file holds for a field, and
/// what it held when the record was read. A table that is not buffered has one answer for both.
///
/// Measured against vfp9.exe: with only this program able to write the table, that is also the
/// answer `CURVAL()` gives - what the record held when it was read - so it reads off the same
/// snapshot rather than the field a program's own edit is showing.
fn f_curval(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let field = a[0].as_str()?.trim().to_string();
    ok(on_cursor(c, &a[1..], Value::Null, |cur| cur.original(&field).unwrap_or(Value::Null))?)
}

fn f_oldval(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let field = a[0].as_str()?.trim().to_string();
    // Without buffering there is no old value: the write went straight to the table and nothing
    // kept what was there. Visual FoxPro says so with error 1586 rather than handing back the
    // value that is there now, which would read as though nothing had changed.
    let unbuffered = on_cursor(c, &a[1..], false, |cur| cur.buffering() <= 1)?;
    if unbuffered {
        return Err(RtError::new(1586, "Function requires row or table buffering mode"));
    }
    ok(on_cursor(c, &a[1..], Value::Null, |cur| cur.original(&field).unwrap_or(Value::Null))?)
}

/// UPDATED(): whether the record was changed since the last time the pointer moved to it.
fn f_updated(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_cursor(c, &[], false, |cur| cur.dirty() || cur.field_state(cur.recno(), 0) != 1)?))
}

/// GETAUTOINCVALUE([nDataSession]): the last value an autoincrementing field took, not the next
/// one it is about to. Measured against vfp9.exe: after two rows take 100 and 101 from a field
/// that starts there, this answers 101, where the header's own `autoinc_next` has already moved
/// on to 102 for the row after - and it prints eleven wide, the autoincrementing field's own
/// Integer width. A table with no such field, or nothing generated in it yet, answers `.NULL.`.
fn f_getautoincvalue(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let n = on_cursor(c, &a, None, |cur| {
        cur.header
            .fields
            .iter()
            .find(|f| f.autoincrements())
            .map(|f| f64::from(f.autoinc_next.saturating_sub(u32::from(f.autoinc_step))))
    })?;
    ok(match n {
        Some(n) => Value::Number(n, Width::field('I', 0, 0, n)),
        None => Value::Null,
    })
}

/// ISMEMOFETCHED(cField | nFieldNumber [, area]): whether a memo's text is in hand. It is
/// fetched as it is read here, so a real memo or general field always answers true; asked about
/// a field that holds neither, it refuses the question rather than answering it.
fn f_ismemofetched(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // cFieldName or nFieldNumber, the same choice GETFLDSTATE()'s own first argument makes
    let which = a[0].clone();
    let kind = on_cursor(c, &a[1..], None, |cur| {
        let pos = field_position(cur, &which);
        pos.checked_sub(1).and_then(|i| cur.header.fields.get(i)).map(|f| f.kind)
    })?;
    match kind {
        // measured against vfp9.exe: asked about a field that is not a memo or general one,
        // it refuses rather than answering as though the field had no text held for it
        Some(k) if !matches!(k, 'M' | 'G' | 'P') => Err(RtError::new(350, "Field must be a Memo field.")),
        Some(_) => ok(Value::Logical(true)),
        None => ok(Value::Logical(false)),
    }
}

/// MDX(nIndex [, area]) and IDXCOLLATE(): the index file beside the table, and how a tag sorts.
fn f_mdx(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_cdx(c, a)
}

fn f_idxcollate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // every tag here sorts by the bytes of its key, which is what MACHINE means
    ok(Value::str(on_tag(c, &a, String::new(), |_| "MACHINE".to_string())?))
}

/// FOR([nIndexNumber [, cCDXFileName [, area]]]): the FOR condition of a tag.
/// FOR(n [, area]): the FOR condition of the nth tag, read off the compiled expression, so its
/// names come back upper-cased. `ATAGINFO()` answers the same question from what the index file
/// holds and so keeps the case the program wrote - the product has both, and so does this.
fn f_for(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(on_tag(c, &a, String::new(), |t| crate::cdx::compiled_expr(&t.for_expr))?))
}

/// KEYMATCH(eExpression [, nIndexNumber [, area]]): whether the order holds that key, leaving
/// the record pointer where it was.
fn f_keymatch(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let key = a[0].clone();
    let (which, rest) = tag_args(&a[1..])?;
    let target = match rest.first() {
        Some(v) => area_of(v)?,
        None => None,
    };
    let near = c.settings().near;
    let data = c.data_mut();
    let cursor = match target {
        Some(what) => data.find_mut(&what),
        None => data.cursor_mut(),
    };
    let Some(cursor) = cursor else { return ok(Value::Logical(false)) };
    let was = (cursor.recno(), cursor.order_no());
    if let Some(i) = which {
        cursor.set_order(Some(i));
    }
    let found = crate::data::seek_in_order(cursor, &key, near).unwrap_or(false);
    cursor.seek(was.0);
    if was.1 != cursor.order_no() {
        cursor.set_order(was.1.map(|n| n - 1));
    }
    ok(Value::Logical(found))
}

/// LOOKUP(cReturnField, eSearchExpression, cSearchedField [, cTagName]): the value of one field
/// of the record another field holds a value in. The pointer is left on the record it found.
fn f_lookup(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let ret = a[0].as_str()?.trim().to_string();
    let key = a[1].clone();
    let searched = a[2].as_str()?.trim().to_string();
    let tag = a.get(3).map(|v| v.as_str()).transpose()?.map(|s| s.trim().to_string());
    let settings = c.settings().clone();
    let data = c.data_mut();
    let Some(cursor) = data.cursor_mut() else { return ok(Value::Null) };
    // down a tag when there is one to go down, and record by record when there is not
    let found = match tag.and_then(|name| cursor.tag_index(&name)) {
        Some(i) => {
            cursor.set_order(Some(i));
            crate::data::seek_in_order(cursor, &key, settings.near).unwrap_or(false)
        }
        None => {
            let count = cursor.count();
            let mut hit = false;
            for recno in 1..=count {
                cursor.seek(recno);
                // the same equality a program's own `=` would use, padding included - a
                // character field compared to a shorter key with SET EXACT off, as it is by
                // default, matches on the key's own length, not the field's
                let matched = cursor
                    .field(&searched)
                    .map(|v| crate::value::compare(&v, &key, crate::value::CmpOp::Eq, &settings))
                    .is_some_and(|r| matches!(r, Ok(Value::Logical(true))));
                if matched {
                    hit = true;
                    break;
                }
            }
            hit
        }
    };
    cursor.set_found(found);
    if !found {
        let past = cursor.count() + 1;
        cursor.seek(past);
        return ok(Value::str(""));
    }
    ok(cursor.field(&ret).unwrap_or(Value::Null))
}

// ----- databases -----------------------------------------------------------------------------
//
// A database is a container listing the tables and views that belong to it. These read that
// list, and DBSETPROP writes a property into it, which means writing the container back out:
// the table first and the memo file beside it after, one round trip each.

/// The database a function is asked about: the current one.
fn current_db<T>(c: &dyn BuiltinCtx, empty: T, f: impl FnOnce(&crate::vm::Database) -> T) -> T {
    match c.current_database().and_then(|i| c.databases().get(i)) {
        Some(db) => f(db),
        None => empty,
    }
}

/// DBC(): the container of the database that is current, or an empty string when there is none.
fn f_dbc(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(current_db(c, String::new(), |db| db.path.clone())))
}

/// DBUSED(cDatabase): whether that database is open.
fn f_dbused(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = a[0].as_str()?.trim().to_string();
    ok(Value::Logical(c.databases().iter().any(|db| db.name().eq_ignore_ascii_case(&name))))
}

/// ADBOBJECTS(aArray, cType): the names of what the current database holds - its tables, its
/// views, its relations or its connections.
fn f_adbobjects(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let kind = a[1].as_str()?.trim().to_ascii_uppercase();
    let wanted = match kind.as_str() {
        "TABLE" => "Table",
        "VIEW" => "View",
        "CONNECTION" => "Connection",
        "RELATION" => "Relation",
        _ => return Err(RtError::function_arg_invalid()),
    };
    let names = current_db(c, Vec::new(), |db| {
        db.objects.iter().filter(|o| o.kind == wanted).map(|o| o.name.clone()).collect::<Vec<String>>()
    });
    let target = crate::builtins::array::array_of(&a[0])?;
    {
        let mut array = target.borrow_mut();
        array.redim(names.len().max(1), 0);
        for (i, name) in names.iter().enumerate() {
            if let Some(slot) = array.items.get_mut(i) {
                *slot = Value::str(name.clone());
            }
        }
    }
    ok(Value::number(names.len() as f64))
}

/// INDBC(cName, cType): whether the current database holds something of that name and kind.
fn f_indbc(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = a[0].as_str()?.trim().to_string();
    let kind = a[1].as_str()?.trim().to_ascii_uppercase();
    let found = current_db(c, false, |db| {
        db.objects.iter().any(|o| o.name.eq_ignore_ascii_case(&name) && o.kind.to_ascii_uppercase() == kind)
    });
    ok(Value::Logical(found))
}

/// One property of a container object, out of the `name=value` lines it keeps.
fn property_of(text: &str, name: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.split_once('=').filter(|(key, _)| key.trim().eq_ignore_ascii_case(name)))
        .map(|(_, value)| value.to_string())
}

/// DBGETPROP(cName, cType, cProperty): what the container says about one of its objects.
fn f_dbgetprop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = a[0].as_str()?.trim().to_string();
    let kind = a[1].as_str()?.trim().to_ascii_uppercase();
    let property = a[2].as_str()?.trim().to_string();
    // Both are handed exactly what DBGETPROP was asked, lower-cased, and a Before that says
    // no makes the function answer .NULL. rather than the value - measured.
    let asked = [DbcArg::Name(name.clone()), DbcArg::Name(kind.clone()), DbcArg::Name(property.clone())];
    if !c.database_event("dbc_BeforeDBGetProp", &asked) {
        return ok(Value::Null);
    }
    c.database_event("dbc_AfterDBGetProp", &asked);
    let value = current_db(c, None, |db| {
        let object = db.objects.iter().find(|o| {
            o.kind.to_ascii_uppercase() == kind
                && (o.name.eq_ignore_ascii_case(&name) || (kind == "DATABASE" && db.name().eq_ignore_ascii_case(&name)))
        })?;
        // a view's SQL is what it stands for, and is kept as the object's code
        if property.eq_ignore_ascii_case("SQL") {
            return Some(Value::str(object.code.clone()));
        }
        if property.eq_ignore_ascii_case("Path") {
            return Some(Value::str(db.path.clone()));
        }
        property_of(&object.property, &property).map(Value::str)
    });
    // a property nobody set reads as the empty string, which is what a comment starts as
    ok(value.unwrap_or_else(|| Value::str("")))
}

/// DBSETPROP(cName, cType, cProperty, eValue): sets one, and writes the container back.
///
/// The container is two files, so this runs three times: once to change it and ask for the
/// table to be written, once for the memo file beside it, and once to answer.
fn f_dbsetprop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let reply = c.take_data_reply();
    // the container is written a file at a time, and this runs again for each of them
    if c.database_io_pending() {
        return match c.database_io_step(reply)? {
            Some(request) => Ok(BuiltinResult::SuspendData { request, args: a }),
            None => ok(Value::Logical(true)),
        };
    }
    let Some(index) = c.current_database() else { return ok(Value::Logical(false)) };
    let name = a[0].as_str()?.trim().to_string();
    let kind = a[1].as_str()?.trim().to_ascii_uppercase();
    let property = a[2].as_str()?.trim().to_string();
    // the value goes over as the program wrote it, where the three names are lower-cased;
    // a Before that says no makes DBSETPROP answer .F. and changes nothing - measured
    let asked = [
        DbcArg::Name(name.clone()),
        DbcArg::Name(kind.clone()),
        DbcArg::Name(property.clone()),
        DbcArg::Text(crate::value::display(&a[3], c.settings())),
    ];
    if !c.database_event("dbc_BeforeDBSetProp", &asked) {
        return ok(Value::Logical(false));
    }
    let value = crate::value::display(&a[3], c.settings());
    let db = c.databases_mut().get_mut(index).expect("the current database");
    let own = db.name();
    let Some(object) = db.objects.iter_mut().find(|o| {
        o.kind.to_ascii_uppercase() == kind
            && (o.name.eq_ignore_ascii_case(&name) || (kind == "DATABASE" && own.eq_ignore_ascii_case(&name)))
    }) else {
        return ok(Value::Logical(false));
    };
    // a view's SQL is what it stands for; everything else is one of its `name=value` lines
    if property.eq_ignore_ascii_case("SQL") {
        object.code = value;
    } else {
        let kept: Vec<String> = object
            .property
            .lines()
            .filter(|line| !line.split_once('=').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(&property)))
            .map(str::to_string)
            .collect();
        object.property = kept.join("\n");
        if !object.property.is_empty() {
            object.property.push('\n');
        }
        object.property.push_str(&format!("{property}={value}"));
    }
    // The After event comes once the property is written, which is what lets the very
    // DBSETPROP that switches DBCEvents on be heard: measured, the product calls
    // dbc_AfterDBSetProp for it and not dbc_BeforeDBSetProp, because at the Before the
    // container's events were still off.
    c.database_event("dbc_AfterDBSetProp", &asked);
    match c.write_current_database() {
        Some(request) => Ok(BuiltinResult::SuspendData { request, args: a }),
        None => ok(Value::Logical(true)),
    }
}

// ----- buffering -----------------------------------------------------------------------------
//
// A buffered table holds what is written to it until TABLEUPDATE sends it on or TABLEREVERT
// throws it away. These say what is held and what state each field is in.

/// The properties CURSORGETPROP and CURSORSETPROP answer for, upper-cased.
fn cursor_prop(a: &[Value]) -> Result<String, RtError> {
    Ok(a.first().map(|v| v.as_str()).transpose()?.unwrap_or_default().trim().to_ascii_uppercase())
}

/// CURSORGETPROP(cProperty [, area]): what the work area is set to.
fn f_cursorgetprop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = cursor_prop(&a)?;
    let rest = &a[1..];
    let value = match name.as_str() {
        "BUFFERING" => Value::number(on_cursor(c, rest, 1.0, |cur| f64::from(cur.buffering()))?),
        "SOURCENAME" | "DATABASE" => Value::str(on_cursor(c, rest, String::new(), |cur| cur.path.clone())?),
        "TABLES" => Value::str(on_cursor(c, rest, String::new(), |cur| cur.path.clone())?),
        "ALIAS" => Value::str(on_cursor(c, rest, String::new(), |cur| cur.alias.clone())?),
        // a table opened by USE is not a view, and nothing here is fetched a batch at a time
        "SENDUPDATES" | "FETCHMEMO" => Value::Logical(true),
        "SQL" | "KEYFIELDLIST" | "UPDATENAMELIST" | "UPDATABLEFIELDLIST" => Value::str(""),
        "RECORDSFETCHED" => Value::number(on_cursor(c, rest, 0.0, |cur| cur.count() as f64)?),
        _ => Value::Logical(false),
    };
    ok(value)
}

/// CURSORSETPROP(cProperty [, eValue [, area]]): sets one. Buffering is the one that changes
/// what the runtime does; the rest are remembered only as far as reading them back.
fn f_cursorsetprop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = cursor_prop(&a)?;
    if name != "BUFFERING" {
        return ok(Value::Logical(true));
    }
    let mode = a.get(1).map(Value::as_number).transpose()?.unwrap_or(1.0) as u8;
    if !(1..=5).contains(&mode) {
        return Err(RtError::function_arg_invalid());
    }
    let target = match a.get(2) {
        Some(v) => area_of(v)?,
        None => None,
    };
    let data = c.data_mut();
    let cursor = match target {
        Some(what) => data.find_mut(&what),
        None => data.cursor_mut(),
    };
    let Some(cursor) = cursor else { return ok(Value::Logical(false)) };
    cursor.set_buffering(mode);
    ok(Value::Logical(true))
}

/// The field a state function names: a number from 1, a name, or 0 for the record itself.
fn field_position(cursor: &Cursor, value: &Value) -> usize {
    match value.deref() {
        Value::Number(n, ..) => n as usize,
        other => other
            .as_str()
            .ok()
            .and_then(|name| cursor.header.field_index(&name))
            .map_or(0, |i| i + 1),
    }
}

/// GETFLDSTATE(nField | cField [, area]): 1 unchanged, 2 changed, 3 the deletion flag changed,
/// 4 a field of a record that was appended.
fn f_getfldstate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = a[0].clone();
    let state = on_cursor(c, &a[1..], 1.0, |cur| {
        let index = field_position(cur, &which);
        f64::from(cur.field_state(cur.recno(), index))
    })?;
    ok(Value::number(state))
}

/// SETFLDSTATE(nField | cField, nState [, area]): says a field was changed, or was not.
fn f_setfldstate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = a[0].clone();
    let state = a[1].as_number()? as u8;
    let target = match a.get(2) {
        Some(v) => area_of(v)?,
        None => None,
    };
    let data = c.data_mut();
    let cursor = match target {
        Some(what) => data.find_mut(&what),
        None => data.cursor_mut(),
    };
    let Some(cursor) = cursor else { return ok(Value::Logical(false)) };
    let index = field_position(cursor, &which);
    let recno = cursor.recno();
    cursor.set_field_state(recno, index, state);
    ok(Value::Logical(true))
}

/// GETNEXTMODIFIED(nRecord [, area]): the next record the table is holding a change for.
fn f_getnextmodified(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let from = arg_num(&a, 0)?;
    let next = on_cursor(c, &a[1..], 0.0, |cur| {
        let start = if from <= 0.0 { 0 } else { from as u64 };
        cur.next_modified(start) as f64
    })?;
    ok(Value::number(next))
}

/// TABLEUPDATE([nRows | lAll [, lForce [, cAlias]]]): the records being held are written.
///
/// One record is one write, so this runs again for each of them: the answer that comes back is
/// the last write's, and the function is done when there is nothing left to send.
fn f_tableupdate(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let _ = c.take_data_reply();
    // TABLEUPDATE(.F.) sends the record in hand; anything else sends everything held
    let one = matches!(a.first().map(Value::deref), Some(Value::Logical(false)) | Some(Value::Number(0.0, ..)));
    let target = match a.get(2) {
        Some(v) => area_of(v)?,
        None => None,
    };
    let data = c.data_mut();
    let cursor = match target {
        Some(what) => data.find_mut(&what),
        None => data.cursor_mut(),
    };
    let Some(cursor) = cursor else { return ok(Value::Logical(false)) };
    let Some(handle) = cursor.handle() else {
        // a cursor whose rows are in memory has nowhere to send them, and they are already
        // there; `written()` is what a table on disk gets once its own write completes, and a
        // cursor with nothing to send still needs it - otherwise what a record was before this
        // commit lingers as `originals`' answer for the next change, rather than what it was
        // before that one
        cursor.take_held(None);
        cursor.written();
        return ok(Value::Logical(true));
    };
    let recno = cursor.recno();
    let taken = if one { cursor.take_held(Some(recno)).into_iter().next() } else { cursor.take_one_held() };
    let Some((first, record)) = taken else { return ok(Value::Logical(true)) };
    // a record that was appended tells the header how long the table is now
    let count = record.appended.then(|| cursor.count() as f64);
    let request = HostRequest::DataWrite { handle, recno: first as f64, bytes: record.bytes, count };
    Ok(BuiltinResult::SuspendData { request, args: a })
}

/// TABLEREVERT([lAll [, cAlias]]): what is being held is thrown away and the records go back to
/// what the table holds.
fn f_tablerevert(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let one = matches!(a.first().map(Value::deref), Some(Value::Logical(false)) | Some(Value::Number(0.0, ..)));
    let target = match a.get(1) {
        Some(v) => area_of(v)?,
        None => None,
    };
    let data = c.data_mut();
    let cursor = match target {
        Some(what) => data.find_mut(&what),
        None => data.cursor_mut(),
    };
    let Some(cursor) = cursor else { return ok(Value::number(0.0)) };
    let recno = cursor.recno();
    let n = cursor.revert(if one { Some(recno) } else { None });
    ok(Value::number(n as f64))
}

// ----- locks ---------------------------------------------------------------------------------
//
// A table belongs to one program here, so a lock is always granted. What these report is what
// this program has asked for, which is what a program that locks a record, writes it and lets go
// again needs to see.

/// FLOCK([area]): locks the whole table.
fn f_flock(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let target = match a.first() {
        Some(v) => area_of(v)?,
        None => None,
    };
    let data = c.data_mut();
    let cursor = match target {
        Some(what) => data.find_mut(&what),
        None => data.cursor_mut(),
    };
    ok(Value::Logical(cursor.is_some_and(Cursor::lock_file)))
}

/// RLOCK() and LOCK(): locks the record the pointer is on, or the ones a list names.
fn f_rlock(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // RLOCK("2,5", "customer") locks those records of that table; RLOCK() the current one
    let (list, area) = match a.len() {
        0 => (None, None),
        1 => match a[0].deref() {
            Value::Str(s) if s.contains(',') || s.trim().parse::<f64>().is_ok() => (Some(s.to_string()), None),
            _ => (None, area_of(&a[0])?),
        },
        _ => (Some(a[0].as_str()?.to_string()), area_of(&a[1])?),
    };
    let data = c.data_mut();
    let cursor = match area {
        Some(what) => data.find_mut(&what),
        None => data.cursor_mut(),
    };
    let Some(cursor) = cursor else { return ok(Value::Logical(false)) };
    match list {
        None => {
            let recno = cursor.recno();
            ok(Value::Logical(cursor.lock_record(recno)))
        }
        Some(list) => {
            let mut all = true;
            for part in list.split(',') {
                let recno = part.trim().parse::<f64>().unwrap_or(0.0) as u64;
                all = cursor.lock_record(recno) && all;
            }
            ok(Value::Logical(all))
        }
    }
}

/// ISFLOCKED([area]): whether this program has the whole table locked.
fn f_isflocked(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_cursor(c, &a, false, Cursor::file_locked)?))
}

/// ISRLOCKED([nRecord] [, area]): whether a record is locked.
fn f_isrlocked(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (recno, rest): (Option<f64>, &[Value]) = match a.first().map(Value::deref) {
        Some(Value::Number(n, ..)) => (Some(n), &a[1..]),
        _ => (None, &a[..]),
    };
    let locked = on_cursor(c, rest, false, |cur| cur.record_locked(recno.unwrap_or(cur.recno() as f64) as u64))?;
    ok(Value::Logical(locked))
}

/// FILTER(): the condition SET FILTER put on a work area, as it was written.
fn f_filter(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(on_cursor(c, &a, String::new(), |cur| crate::cdx::compiled_expr(cur.filter()))?))
}

/// RELATION(n [, area]): the expression of the nth relation set from a work area.
fn f_relation(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let n = arg_num(&a, 0)? as usize;
    let text = on_cursor(c, &a[1..], String::new(), |cur| {
        n.checked_sub(1).and_then(|i| cur.relations().get(i)).map(|r| crate::cdx::compiled_expr(&r.expr)).unwrap_or_default()
    })?;
    ok(Value::str(text))
}

/// TARGET(n [, area]): the alias the nth relation moves.
fn f_target(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let n = arg_num(&a, 0)? as usize;
    let text = on_cursor(c, &a[1..], String::new(), |cur| {
        n.checked_sub(1).and_then(|i| cur.relations().get(i)).map(|r| r.into.to_ascii_uppercase()).unwrap_or_default()
    })?;
    ok(Value::str(text))
}

/// AFIELDS(): the table's columns as an array, one row per field and eighteen columns per row.
///
/// Measured against a free table in Visual FoxPro 9: name, type, width, decimals, whether the
/// field takes .NULL., whether it is kept out of code-page translation, then ten strings for the
/// rules and names a database gives a field - validation, defaults, triggers, long names,
/// comments - and last the two numbers an autoincrementing field carries. A free table has none
/// of the middle ten, and the reference page describes sixteen columns rather than eighteen.
const FIELD_COLUMNS: usize = 18;

fn f_afields(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let fields = on_cursor(c, &a[1..], Vec::new(), |cur| cur.header.fields.clone())?;
    let target = crate::builtins::array::array_of(&a[0])?;
    {
        let mut array = target.borrow_mut();
        array.redim(fields.len().max(1), FIELD_COLUMNS);
        for (row, field) in fields.iter().enumerate() {
            let mut cells = vec![
                Value::str(field.name.to_uppercase()),
                Value::str(field.kind.to_string()),
                Value::number(f64::from(field.length)),
                Value::number(f64::from(field.decimals)),
                Value::Logical(false),
                Value::Logical(false),
            ];
            cells.resize(FIELD_COLUMNS - 2, Value::str(""));
            cells.push(Value::number(f64::from(field.autoinc_next)));
            cells.push(Value::number(f64::from(field.autoinc_step)));
            for (col, value) in cells.into_iter().enumerate() {
                if let Some(slot) = array.items.get_mut(row * FIELD_COLUMNS + col) {
                    *slot = value;
                }
            }
        }
    }
    ok(Value::number(fields.len() as f64))
}

/// FOUND(): did the last LOCATE or CONTINUE on this table find a record. False for a work area
/// with no table, and false until something has searched it.
fn f_found(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_cursor(c, &a, false, Cursor::found)?))
}

/// RECNO(): the record the pointer is on. 0 when no table is open, as VFP reports it.
fn f_recno(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(on_cursor(c, &a, 0.0, |cur| cur.recno() as f64)?))
}

fn f_reccount(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(on_cursor(c, &a, 0.0, |cur| cur.count() as f64)?))
}

/// EOF(): true past the last record, and true for a work area with no table, which is what lets
/// `DO WHILE NOT EOF()` end rather than spin when the table failed to open.
/// EOF() and BOF(): a work area with no table in it is neither past the end nor before the
/// start, so both answer .F. there - measured, where a runtime might reasonably have guessed
/// that "no records" means "at the end".
fn f_eof(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_cursor(c, &a, false, Cursor::eof)?))
}

fn f_bof(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::Logical(on_cursor(c, &a, false, Cursor::bof)?))
}

/// DELETED(): is the current record marked deleted.
///
/// The one function here that needs the record and not just the header, so it is the one that can
/// send the VM to the host for the page it lives on and be run again with the answer.
fn f_deleted(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let reply = c.take_data_reply();
    let what = area(&a)?;
    let data = c.data_mut();
    let cursor = match &what {
        None => data.cursor_mut(),
        Some(w) => data.find_mut(w),
    };
    let Some(cursor) = cursor else { return ok(Value::Logical(false)) };

    if let Some(page) = reply {
        cursor.accept_page(bytes_of(&page));
        let recno = cursor.recno();
        cursor.seek(recno);
    }
    let recno = cursor.recno();
    if let Some(handle) = cursor.handle()
        && recno >= 1
        && recno <= cursor.count()
        && !cursor.page_holds(recno)
    {
        let first = cursor.page_start(recno);
        cursor.expect_page(first);
        let request = HostRequest::DataRead { handle, first: first as f64, count: PAGE_RECORDS };
        return Ok(BuiltinResult::SuspendData { request, args: a });
    }
    ok(Value::Logical(cursor.deleted()))
}

/// ALIAS(): the name of the table in a work area, "" when it holds none.
fn f_alias(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(on_cursor(c, &a, String::new(), |cur| cur.alias.to_uppercase())?))
}

/// DBF(): the table's file name. The VM is told the path at `USE` and keeps it for this.
///
/// A table is a .dbf whether or not the command that opened it said so, and Visual FoxPro's
/// answer always carries the extension - `CREATE TABLE staff` still gives back STAFF.DBF.
fn f_dbf(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::str(on_cursor(c, &a, String::new(), |cur| {
        let leaf = cur.path.rsplit(['/', '\\']).next().unwrap_or(&cur.path);
        match leaf.contains('.') || cur.path.is_empty() {
            true => cur.path.clone(),
            false => format!("{}.dbf", cur.path),
        }
    })?))
}

fn f_fcount(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(on_cursor(c, &a, 0.0, |cur| cur.header.fields.len() as f64)?))
}

/// FSIZE(cFieldName [, nWorkArea | cTableAlias]): how many bytes that column takes in a record.
///
/// It is a question about a field and never about a file, whatever the name looks like: Visual
/// FoxPro answers 0 for `FSIZE("made.txt")` with that file sitting in the directory, because no
/// column is called that. Measured, and against the reference page, which reads as though a
/// file name would do.
fn f_fsize(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.to_string();
    let rest = a.get(1..).unwrap_or_default().to_vec();
    ok(Value::number(on_cursor(c, &rest, 0.0, |cur| {
        let found = cur.header.fields.iter().find(|f| f.name.eq_ignore_ascii_case(&name));
        found.map(|f| f64::from(f.length)).unwrap_or(0.0)
    })?))
}

/// FIELD(n [, area]): the name of the n-th column, upper-cased as VFP reports it.
fn f_field(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let n = arg_num(&a, 0)?.trunc() as i64;
    let rest = a.get(1..).unwrap_or_default().to_vec();
    ok(Value::str(on_cursor(c, &rest, String::new(), |cur| {
        if n < 1 {
            return String::new();
        }
        cur.header.fields.get(n as usize - 1).map(|f| f.name.to_uppercase()).unwrap_or_default()
    })?))
}

/// SELECT([0 | 1 | cTableAlias]): the selected work area, the highest unused one, or the area
/// a table alias is open in (0 when nothing answers to it). Measured against vfp9.exe: unlike
/// most work-area functions this one prints twenty wide and whole, a double rather than a
/// variable's ten - `? SELECT()` is nineteen spaces and a 1.
fn f_select(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let data = c.data();
    let n = match a.first().map(Value::deref) {
        None => data.current_area(),
        Some(Value::Str(s)) if s.trim() == "1" => data.highest_free(),
        Some(Value::Str(s)) if s.trim() == "0" || s.trim().is_empty() => data.current_area(),
        Some(Value::Str(s)) => data.area_of_alias(s.trim()),
        Some(Value::Number(n, ..)) if n as i64 == 1 => data.highest_free(),
        _ => data.current_area(),
    };
    ok(Value::Number(n as f64, Width { chars: 20, decimals: 0, written: false }))
}

/// USED(): whether a work area holds a table. With no argument, whether the selected one does.
fn f_used(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let data = c.data();
    let open = match area(&a)? {
        None => data.cursor().is_some(),
        Some(what) => data.used(&what),
    };
    ok(Value::Logical(open))
}


/// `CPDBF([area])`: the code page the table is marked with, or 0 when it is not marked.
fn f_cpdbf(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(Value::number(on_cursor(c, &a, 0.0, |cur| f64::from(cur.header.codepage.unwrap_or(0)))?))
}

/// `REFRESH([nRecords [, nOffset]] [, area])`: the records are read again from where they live.
///
/// The runtime keeps one page of records in hand at a time, so what makes a record fresh is
/// letting go of that page: the next read fetches it. The answer is how many records were let
/// go of, which is what Visual FoxPro answers.
fn f_refresh(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    // the last argument names a work area when it is not a number
    let named = a.iter().rposition(|v| !matches!(v.deref(), Value::Number(..)));
    let where_ = match named {
        Some(i) => area_of(&a[i])?,
        None => None,
    };
    let numbers: Vec<f64> = a.iter().filter_map(|v| v.deref().as_number().ok()).collect();
    let count = numbers.first().copied().unwrap_or(1.0).max(0.0) as u64;
    let data = c.data_mut();
    let cursor = match where_ {
        None => data.cursor_mut(),
        Some(what) => data.find_mut(&what),
    };
    let Some(cursor) = cursor else { return ok(Value::number(0.0)) };
    if count == 0 {
        return ok(Value::number(0.0));
    }
    cursor.forget_page();
    ok(Value::number(count.min(cursor.count()) as f64))
}

// ------------------------------------------------------------------------------------------
// views taken offline, and the result set a cursor came from
// ------------------------------------------------------------------------------------------

/// `CREATEOFFLINE(cViewName [, cPath])`: a view's records taken away from the source, so they
/// can be worked on without it. The records go to a table of their own; the container is told.
/// `cPath` says where that table goes; this runtime has nowhere to put one, so it is read and
/// not otherwise acted on, the same as the view itself asks nothing of the source to answer.
fn f_createoffline(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let view = arg_str(&a, 0)?.trim().to_ascii_uppercase();
    // The events hear the view and the table its records would go to: measured, the product
    // says the view's own name with a .dbf beside it when the call did not say where.
    let written = match a.get(1) {
        Some(path) => path.as_str()?.trim().to_string(),
        None => String::new(),
    };
    let table = if written.is_empty() { c.settings().table_at(&view) } else { c.settings().table_at(&written) };
    let asked = [DbcArg::Name(view.clone()), DbcArg::Name(table)];
    if !c.database_event("dbc_BeforeCreateOffline", &asked) {
        return ok(Value::Logical(false));
    }
    let open = c.data().find(&AreaRef::Alias(view.clone())).is_some();
    c.database_event("dbc_AfterCreateOffline", &asked);
    ok(Value::Logical(open))
}

/// `DROPOFFLINE(cViewName)`: the view goes back to being what the source says.
///
/// Measured against vfp9.exe: there is no second argument - a program gets `TOO_MANY_ARGS` for
/// giving it one, not a form this runtime should have been accepting either. And where
/// `CREATEOFFLINE()` on a view over native data answers true - there is nothing stopping it -
/// `DROPOFFLINE()` answers false whether or not it was ever taken offline, twice running.
/// Nothing here is ever genuinely disconnected from its source for it to reconnect.
///
/// Which is also why `dbc_AfterDropOffline` never runs: the product only fires it when the view
/// really did come back online, and a view over native data never answers anything but false -
/// measured, with both procedures in the container and the Before one firing on its own.
fn f_dropoffline(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let view = arg_str(&a, 0)?.trim().to_ascii_uppercase();
    c.database_event("dbc_BeforeDropOffline", &[DbcArg::Name(view.clone())]);
    ok(Value::Logical(false))
}

/// `ISTRANSACTABLE(nWorkArea | cTableAlias)`: can what happens to this cursor be undone as one.
///
/// Measured against vfp9.exe: a view over native data answers true here already, before
/// `MAKETRANSACTABLE()` is ever called - every cursor this runtime has is over native data
/// (there is no remote source to make one non-transactable), so the answer is just whether a
/// cursor is open there at all.
fn f_istransactable(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let alias = alias_of(c, &a);
    ok(Value::Logical(!alias.is_empty()))
}

/// `MAKETRANSACTABLE(nWorkArea | cTableAlias)`: makes it so, which a table of its own already is.
fn f_maketransactable(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let alias = alias_of(c, &a);
    ok(Value::Logical(!alias.is_empty()))
}

/// The work area a `SETRESULTSET`-family argument names - a number, once a cursor is confirmed
/// open there, or an alias looked up - and 0 when nothing open answers to either.
fn result_set_area(c: &dyn BuiltinCtx, v: &Value) -> Result<usize, RtError> {
    Ok(match area_of(v)? {
        Some(AreaRef::Number(n)) => {
            if c.data().find(&AreaRef::Number(n)).is_some() {
                n
            } else {
                0
            }
        }
        Some(AreaRef::Alias(s)) => c.data().area_of_alias(&s),
        None => 0,
    })
}

/// `SETRESULTSET(nWorkArea | cTableAlias)`, `GETRESULTSET()` and `CLEARRESULTSET()`: which
/// cursor a COM server hands back to whoever called it, kept as a work area number - 0 when
/// none is marked - not the alias a first reading of the reference page suggests.
fn f_setresultset(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let area = result_set_area(c, &a[0])?;
    // measured against vfp9.exe: naming a work area or alias nothing is open in refuses rather
    // than leaving the marker where it was, and all three of these answer eleven characters
    // wide, a work area's own width
    if area == 0 {
        return Err(RtError::function_arg_invalid());
    }
    let old = c.result_set();
    c.set_result_set(area);
    ok(area_number(old as i64))
}

/// A work area as these functions answer with one: eleven characters wide, no places - measured.
fn area_number(n: i64) -> Value {
    Value::Number(n as f64, Width { chars: 11, decimals: 0, written: false })
}

fn f_getresultset(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(area_number(c.result_set() as i64))
}

fn f_clearresultset(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let old = c.result_set();
    c.set_result_set(0);
    ok(area_number(old as i64))
}

/// `GETCURSORADAPTER(cAlias)`: the CursorAdapter that filled the cursor, when one did.
fn f_getcursoradapter(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let _ = arg_str(&a, 0)?;
    let _ = c;
    // A cursor this runtime makes is made by a query or by USE, never by an adapter object, and
    // asking one of those which adapter it came from is not answered with nothing: the product
    // raises 1115, "Invalid operation for the cursor." - measured.
    Err(RtError::new(RtError::INVALID_CURSOR_OPERATION, "Invalid operation for the cursor."))
}

/// `REQUERY([cAlias | nWorkArea])`: a view's SELECT is run again, so the cursor says what the
/// source says now. Answers 1 when it ran and 0 when there was nothing to run.
fn f_requery(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let alias = alias_of(c, &a);
    let sql = c.view_sql(&alias);
    if sql.is_empty() {
        // a table that is open, but not a view, is not what REQUERY() is for - measured
        // against vfp9.exe, which refuses rather than answering 0 for one
        if !alias.is_empty() {
            return Err(RtError::new(1536, "Function is not supported on native tables."));
        }
        return ok(Value::number(0.0));
    }
    let query = if sql.to_ascii_uppercase().contains(" INTO ") { sql } else { format!("{sql} INTO CURSOR {alias}") };
    match c.take_data_reply() {
        Some(_) => ok(Value::number(1.0)),
        None => Ok(BuiltinResult::SuspendData { request: HostRequest::RunLine { text: query }, args: a }),
    }
}

/// The alias a call is about: the one it named, or the one in hand.
fn alias_of(c: &dyn BuiltinCtx, a: &[Value]) -> String {
    match a.first().map(Value::deref) {
        Some(Value::Str(s)) => s.trim().to_ascii_uppercase(),
        Some(Value::Number(n, ..)) => c
            .data()
            .find(&AreaRef::Number(n.max(0.0) as usize))
            .map(|cur| cur.alias.clone())
            .unwrap_or_default(),
        _ => c.data().cursor().map(|cur| cur.alias.clone()).unwrap_or_default(),
    }
}
