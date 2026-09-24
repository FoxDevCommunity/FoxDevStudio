//! The functions that report on the menus a program has defined: what is up, what was chosen
//! from it, and what each pad and bar of it says.
//!
//! Visual FoxPro's menu functions come in pairs - one for a pad of a menu bar, one for a bar of
//! a popup - and each pair asks the same question of the two halves of `crate::menu`. The four
//! that take no argument at all answer what the last choice was, which the host tells the VM as
//! it runs the command that choice stands for.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, arg_num, arg_str, opt_str, spec};
use crate::error::RtError;
use crate::value::Value;

fn value(v: Value) -> Result<BuiltinResult, RtError> {
    Ok(BuiltinResult::Value(v))
}

/// The popup a function was given, or the one that is up when it was given none.
fn popup_name(c: &dyn BuiltinCtx, a: &[Value], i: usize) -> Result<String, RtError> {
    let given = opt_str(a, i, "")?.trim().to_string();
    Ok(if given.is_empty() { c.menus().active_popup.clone() } else { given })
}

/// The menu a function was given, or the one that is up when it was given none.
fn menu_name(c: &dyn BuiltinCtx, a: &[Value], i: usize) -> Result<String, RtError> {
    let given = opt_str(a, i, "")?.trim().to_string();
    if !given.is_empty() {
        return Ok(given);
    }
    let up = c.menus().active_menu.clone();
    Ok(if up.is_empty() { "_MSYSMENU".to_string() } else { up })
}

/// BAR(): the number of the bar the program last chose from a popup.
fn f_bar(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::number(f64::from(c.menus().last_bar)))
}

/// BARCOUNT([cPopupName]): how many bars the popup has, separators counted.
fn f_barcount(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = popup_name(c, &a, 0)?;
    let count = c.menus().popup(&name).map_or(0, |p| p.bars.len());
    value(Value::number(count as f64))
}

/// BARPROMPT(nBar [, cPopupName]): what that bar says, without its marks. Unlike BARCOUNT and
/// CNTBAR, which count an unresolved popup as having nothing in it, BARPROMPT has to find the
/// popup itself before it can find a bar of it, and refuses when there is none to find - left
/// out with none active, or named and wrong either way.
fn f_barprompt(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let number = arg_num(&a, 0)? as i32;
    let name = popup_name(c, &a, 1)?;
    let Some(popup) = c.menus().popup(&name) else { return Err(RtError::popup_not_defined()) };
    let prompt = popup
        .bars
        .iter()
        .find(|b| b.number == number)
        .map(|b| crate::menu::plain_prompt(&b.prompt))
        .unwrap_or_default();
    value(Value::str(prompt))
}

/// CNTBAR(cPopupName): the same count BARCOUNT gives, which is the older spelling of it.
fn f_cntbar(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    f_barcount(c, a)
}

/// CNTPAD(cMenuName): how many pads the menu bar has.
fn f_cntpad(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = menu_name(c, &a, 0)?;
    let count = c.menus().menu(&name).map_or(0, |m| m.pads.len());
    value(Value::number(count as f64))
}

/// GETBAR(cPopupName, nPosition): the number of the bar in that place, counting from one, or
/// 0 when the popup has no such place.
fn f_getbar(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = popup_name(c, &a, 0)?;
    let at = arg_num(&a, 1)? as usize;
    let number = c
        .menus()
        .popup(&name)
        .and_then(|p| at.checked_sub(1).and_then(|i| p.bars.get(i)))
        .map_or(0, |b| b.number);
    value(Value::number(f64::from(number)))
}

/// GETPAD(cMenuName, nPosition): the name of the pad in that place, or "" when there is none.
fn f_getpad(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = menu_name(c, &a, 0)?;
    let at = arg_num(&a, 1)? as usize;
    let pad = c
        .menus()
        .menu(&name)
        .and_then(|m| at.checked_sub(1).and_then(|i| m.pads.get(i)))
        .map(|p| p.name.to_ascii_uppercase())
        .unwrap_or_default();
    value(Value::str(pad))
}

/// MENU(): the name of the menu bar that is up, in capitals, or "" when none is.
fn f_menu(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::str(c.menus().active_menu.to_ascii_uppercase()))
}

/// MRKBAR(cPopupName, nBar): whether that bar is ticked.
fn f_mrkbar(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.trim().to_string();
    let number = arg_num(&a, 1)? as i32;
    let marked = c
        .menus()
        .popup(&name)
        .and_then(|p| p.bars.iter().find(|b| b.number == number))
        .is_some_and(|b| b.marked);
    value(Value::Logical(marked))
}

/// MRKPAD(cMenuName, cPadName): whether that pad is ticked.
fn f_mrkpad(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let menu = arg_str(&a, 0)?.trim().to_string();
    let pad = arg_str(&a, 1)?.trim().to_string();
    let marked = c
        .menus()
        .menu(&menu)
        .and_then(|m| m.pads.iter().find(|p| p.name.eq_ignore_ascii_case(&pad)))
        .is_some_and(|p| p.marked);
    value(Value::Logical(marked))
}

/// PAD(): the name of the pad the program last chose, in capitals.
fn f_pad(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::str(c.menus().last_pad.to_ascii_uppercase()))
}

/// POPUP(): the name of the popup that is up, or the one last chosen from.
/// POPUP([cMenuName]). With no argument, the popup that is active. With one, whether a popup of
/// that name is defined - measured: a logical, the name compared without regard to case and
/// without trimming, and a number there error 11.
fn f_popup(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    if let Some(named) = a.first() {
        let Value::Str(name) = named.deref() else { return Err(RtError::function_arg_invalid()) };
        return value(Value::Logical(c.menus().popup(&name).is_some()));
    }
    let menus = c.menus();
    let name = if menus.active_popup.is_empty() { &menus.last_popup } else { &menus.active_popup };
    value(Value::str(name.to_ascii_uppercase()))
}

/// PRMBAR(cPopupName, nBar): what that bar says.
fn f_prmbar(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.trim().to_string();
    let number = arg_num(&a, 1)? as i32;
    let prompt = c
        .menus()
        .popup(&name)
        .and_then(|p| p.bars.iter().find(|b| b.number == number))
        .map(|b| crate::menu::plain_prompt(&b.prompt))
        .unwrap_or_default();
    value(Value::str(prompt))
}

/// PRMPAD(cMenuName, cPadName): what that pad says.
fn f_prmpad(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let menu = arg_str(&a, 0)?.trim().to_string();
    let pad = arg_str(&a, 1)?.trim().to_string();
    let prompt = c
        .menus()
        .menu(&menu)
        .and_then(|m| m.pads.iter().find(|p| p.name.eq_ignore_ascii_case(&pad)))
        .map(|p| crate::menu::plain_prompt(if p.prompt.is_empty() { &p.name } else { &p.prompt }))
        .unwrap_or_default();
    value(Value::str(prompt))
}

/// PROMPT(): what the choice the program last made said.
fn f_prompt(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::str(c.menus().last_prompt.clone()))
}

/// The condition a `SKIP FOR` clause gave, worked out now. A choice with no condition is
/// always available, and one whose condition will not evaluate is left available too - a menu
/// is not the place to stop the program.
fn skipped(c: &mut dyn BuiltinCtx, cond: String) -> bool {
    if cond.trim().is_empty() {
        return false;
    }
    c.evaluate(&cond).ok().and_then(|v| v.truthy().ok()).unwrap_or(false)
}

/// SKPBAR(cPopupName, nBar): whether that bar cannot be chosen just now.
fn f_skpbar(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.trim().to_string();
    let number = arg_num(&a, 1)? as i32;
    let cond = c
        .menus()
        .popup(&name)
        .and_then(|p| p.bars.iter().find(|b| b.number == number))
        .map(|b| b.skip.clone())
        .unwrap_or_default();
    value(Value::Logical(skipped(c, cond)))
}

/// SKPPAD(cMenuName, cPadName): whether that pad cannot be chosen just now.
fn f_skppad(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let menu = arg_str(&a, 0)?.trim().to_string();
    let pad = arg_str(&a, 1)?.trim().to_string();
    let cond = c
        .menus()
        .menu(&menu)
        .and_then(|m| m.pads.iter().find(|p| p.name.eq_ignore_ascii_case(&pad)))
        .map(|p| p.skip.clone())
        .unwrap_or_default();
    value(Value::Logical(skipped(c, cond)))
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("BAR", 0, 0, f_bar),
        spec("BARCOUNT", 0, 1, f_barcount),
        spec("BARPROMPT", 1, 2, f_barprompt),
        spec("CNTBAR", 0, 1, f_cntbar),
        spec("CNTPAD", 0, 1, f_cntpad),
        spec("GETBAR", 2, 2, f_getbar),
        spec("GETPAD", 2, 2, f_getpad),
        spec("MENU", 0, 0, f_menu),
        spec("MRKBAR", 2, 2, f_mrkbar),
        spec("MRKPAD", 2, 2, f_mrkpad),
        spec("PAD", 0, 0, f_pad),
        spec("POPUP", 0, 1, f_popup),
        spec("PRMBAR", 2, 2, f_prmbar),
        spec("PRMPAD", 2, 2, f_prmpad),
        spec("PROMPT", 0, 0, f_prompt),
        spec("SKPBAR", 2, 2, f_skpbar),
        spec("SKPPAD", 2, 2, f_skppad),
    ]
}
