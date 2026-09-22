//! Colour, font and keyboard functions.
//!
//! Three kinds live here. `RGB()` is pure arithmetic. `GETCOLOR()`, `GETFONT()` and `GETKEY()`
//! are dialogs, so they are yielded as host requests exactly like MESSAGEBOX. The font *metric*
//! functions are answered here from the requested point size rather than being refused: a
//! program that lays a form out from FONTMETRIC() must keep running, and an approximation is
//! far more useful than an error.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, arg_num, arg_str, not_available, ok, opt_num, opt_str, spec};
use crate::error::RtError;
use crate::host::HostRequest;
use crate::value::Value;

/// Font families the runtime offers; the desktop shell ships with all of these.
const FONT_FAMILIES: [&str; 7] =
    ["Segoe UI", "Arial", "Consolas", "Courier New", "Times New Roman", "Tahoma", "Verdana"];

/// The font a form uses unless it says otherwise, reported by WFONT().
const DEFAULT_FONT: &str = "Segoe UI";
const DEFAULT_FONT_SIZE: f64 = 9.0;

// ------------------------------------------------------------------------------------------
// colour
// ------------------------------------------------------------------------------------------

/// RGB(r, g, b): VFP packs a colour into `r + g * 256 + b * 65536`. Components are truncated
/// and clamped to 0..255. The compiler folds this same expression inside DEFINE CLASS, so the
/// two must agree - see `fold_constant` in the compiler.
fn f_rgb(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let part = |i: usize| -> Result<f64, RtError> { Ok(arg_num(&a, i)?.trunc().clamp(0.0, 255.0)) };
    ok(Value::number(part(0)? + part(1)? * 256.0 + part(2)? * 65536.0))
}

/// GETCOLOR([nDefault]): resumes with the chosen RGB integer, or -1 when cancelled. -1 as the
/// default means the call preselected nothing.
fn f_getcolor(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let default = opt_num(&a, 0, -1.0)?;
    Ok(BuiltinResult::Suspend(HostRequest::GetColor { default }))
}

// ------------------------------------------------------------------------------------------
// fonts
// ------------------------------------------------------------------------------------------

/// GETFONT([cFont] [, nSize] [, cStyle] [, nCharacterSet]): resumes with "name,size,style" or
/// "" when cancelled. The character set only preselects a script in VFP's own dialog, so it is
/// accepted and ignored.
fn f_getfont(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = opt_str(&a, 0, "")?;
    let size = opt_num(&a, 1, 0.0)?;
    let style = opt_str(&a, 2, "")?;
    Ok(BuiltinResult::Suspend(HostRequest::GetFont { name, size, style }))
}

/// FONTMETRIC(nAttribute [, cFontName, nFontSize [, cStyle]]). With the attribute alone VFP
/// measures the font of the object the call is made from; there is no font engine here, so the
/// shell font's size stands in for it.
///
/// These are **approximations derived from the point size**, not the platform's metrics: the VM
/// has no font engine and no access to the renderer, and returning an error instead would stop
/// every program that positions controls from a metric. The usual ratios for a screen font are
/// used - ascent 0.8 of the size, descent 0.2, average width 0.5, maximum width the full size -
/// so heights come out close enough to lay a form out, but they must not be relied on for exact
/// text measurement. Attributes VFP defines that cannot be approximated return 0.
fn f_fontmetric(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let which = arg_num(&a, 0)?.trunc() as i64;
    // A font name means giving its size too - the pair travel together, so a call with the
    // name alone is half of one, which is what the product refuses rather than guessing a size.
    if a.len() == 2 {
        return Err(RtError::missing_parameter());
    }
    // A non-positive size has no metrics; the font name and style do not change the ratios.
    let size = opt_num(&a, 2, DEFAULT_FONT_SIZE)?.max(0.0);
    let ascent = (size * 0.8).round();
    let descent = (size * 0.2).round();
    // Measured against Arial 10 in Visual FoxPro: 1 is the whole cell, 2 what stands above the
    // baseline and 3 what hangs below it, so 2 and 3 add up to 1. 4 is the internal leading -
    // the room inside the ascent that accents use. 8 is the weight, 400 for an upright face.
    ok(Value::number(match which {
        1 => ascent + descent,
        2 => ascent,
        3 => descent,
        4 => descent,
        6 => (size * 0.6).round(),
        7 => size.round(),
        8 => 400.0,
        _ => 0.0,
    }))
}

/// WFONT(nAttribute [, cWindow]): 1 the font name, 2 its size, 3 its style. Every window in
/// this runtime uses the shell font, so the answer does not depend on cWindow.
fn f_wfont(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    ok(match arg_num(&a, 0)?.trunc() as i64 {
        1 => Value::str(DEFAULT_FONT),
        2 => Value::number(DEFAULT_FONT_SIZE),
        // the style, as the letters VFP spells it with: N for an upright face
        3 => Value::str("N"),
        _ => Value::str(""),
    })
}

/// AFONT(array [, cFontName [, nFontSize]]): with no font name it fills the array with every
/// family the runtime offers and answers how many there are - the form a golden already
/// measures. Naming a font asks something else about that one font instead, and answers
/// whether the array was made rather than a count: every family here is a scalable TrueType
/// face, so the array holds the one row a scalable font always does - `-1` on its own, since a
/// scalable font has no fixed sizes to list, or `.T.` when a size was given too, since a
/// scalable font has no size it cannot be asked for.
fn f_afont(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let arr = super::array::array_of(&a[0])?;
    if a.len() < 2 {
        let mut b = arr.borrow_mut();
        b.redim(FONT_FAMILIES.len(), 0);
        for (i, name) in FONT_FAMILIES.iter().enumerate() {
            b.items[i] = Value::str(*name);
        }
        return ok(Value::number(FONT_FAMILIES.len() as f64));
    }
    let mut b = arr.borrow_mut();
    b.redim(1, 0);
    b.items[0] = if a.len() >= 3 { Value::Logical(true) } else { Value::number(-1.0) };
    ok(Value::Logical(true))
}

// ------------------------------------------------------------------------------------------
// keyboard
// ------------------------------------------------------------------------------------------

/// GETKEY(): resumes with the key code of the next key pressed.
fn f_getkey(_c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    Ok(BuiltinResult::Suspend(HostRequest::GetKey))
}

// ------------------------------------------------------------------------------------------
// pictures
// ------------------------------------------------------------------------------------------

/// `LOADPICTURE([cFile])`: a picture read off disk, as the object a program then holds.
///
/// The object is a bag of four: `Handle` says which picture, `Type` what kind it is, and
/// `Width` and `Height` its size in HIMETRIC units, which is how Windows measures a picture.
/// With no file it is the null picture - handle 0, type 0, no size - which is what the product
/// answers with and what SAVEPICTURE() then refuses.
///
/// It takes two turns: the file's bytes come from the host, and the object is the host's to
/// make. What happens in between - working out what the bytes are a picture of - happens here,
/// so that neither host needs a decoder.
fn f_loadpicture(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let path = opt_str(&a, 0, "")?.trim().to_string();
    if path.is_empty() {
        return Ok(BuiltinResult::Suspend(HostRequest::MakePicture {
            picture: 0.0,
            of_kind: 0.0,
            width: 0.0,
            height: 0.0,
        }));
    }
    let Some(reply) = c.take_data_reply() else {
        return Ok(BuiltinResult::SuspendData {
            request: HostRequest::FileReadBytes { path },
            args: a.clone(),
        });
    };
    let bytes = crate::data::bytes_of(&reply);
    if bytes.is_empty() {
        return Err(RtError::file_not_found(&path));
    }
    let picture = crate::picture::Picture::decode(&bytes)?;
    let (of_kind, width, height) = (f64::from(picture.kind), picture.width(), picture.height());
    let handle = c.keep_picture(picture);
    Ok(BuiltinResult::Suspend(HostRequest::MakePicture { picture: handle, of_kind, width, height }))
}

/// `SAVEPICTURE(oPicture, cFile)`: the picture written out as a bitmap.
///
/// .T. when there was a picture to write. The null picture is not one, and the product answers
/// .F. for it rather than writing an empty file.
fn f_savepicture(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let Value::Object(obj) = a[0].deref() else { return Err(RtError::function_arg_invalid()) };
    let path = arg_str(&a, 1)?.trim().to_string();
    if c.take_data_reply().is_some() {
        return ok(Value::Logical(true));
    }
    let handle = c.host().get_prop(obj, "HANDLE").ok().and_then(|v| v.as_number().ok()).unwrap_or(0.0);
    let Some(bytes) = c.picture(handle).map(crate::picture::Picture::to_bitmap).transpose()? else {
        return ok(Value::Logical(false));
    };
    Ok(BuiltinResult::SuspendData { request: HostRequest::FileWriteBytes { path, bytes }, args: a.clone() })
}

not_available! {
    f_parsfont => "PARSFONT(): there is no such Visual FoxPro function, so nothing here implements it; \
        did you mean GETFONT(), WFONT() or FONTMETRIC()?";
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("AFONT", 1, 3, f_afont),
        spec("FONTMETRIC", 1, 4, f_fontmetric),
        spec("GETCOLOR", 0, 1, f_getcolor),
        spec("GETFONT", 0, 4, f_getfont),
        spec("GETKEY", 0, 0, f_getkey),
        spec("LOADPICTURE", 0, 1, f_loadpicture),
        spec("PARSFONT", 0, 4, f_parsfont),
        spec("RGB", 3, 3, f_rgb),
        spec("SAVEPICTURE", 2, 2, f_savepicture),
        spec("WFONT", 1, 2, f_wfont),
    ]
}
