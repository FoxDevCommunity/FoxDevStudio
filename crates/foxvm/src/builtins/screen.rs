//! The functions that report on the character screen and the windows on it: where the cursor
//! got to, how big the screen is, and everything a program can ask about a window it opened.
//!
//! All of them read `crate::screen`, which is where `@ ... SAY` draws and `DEFINE WINDOW` keeps
//! what it made. The mouse ones ask the host, because only the host knows where the pointer is.

use super::{BuiltinCtx, BuiltinResult, BuiltinSpec, arg_num, arg_str, opt_str, spec};
use crate::error::RtError;
use crate::value::{Value, Width};

fn value(v: Value) -> Result<BuiltinResult, RtError> {
    Ok(BuiltinResult::Value(v))
}

fn number(n: usize) -> Result<BuiltinResult, RtError> {
    value(Value::number(n as f64))
}

/// A place on the screen: ten wide with three places, which is how Visual FoxPro prints one -
/// `? ROW()` is "     0.000", because a row may be a fraction of one on a graphical form.
fn place(n: usize) -> Result<BuiltinResult, RtError> {
    let width = crate::value::Width { chars: 10, decimals: 3, written: false };
    value(Value::Number(n as f64, width))
}

/// The window a function was given, or the one on top when it was given none.
fn window_name(c: &dyn BuiltinCtx, a: &[Value], i: usize) -> Result<String, RtError> {
    let given = opt_str(a, i, "")?.trim().to_string();
    Ok(if given.is_empty() { c.screen().on_top().to_string() } else { given })
}

/// Something a window knows about itself, or the value a window that is not there gives.
fn about<T>(c: &dyn BuiltinCtx, a: &[Value], missing: T, f: impl Fn(&crate::screen::Window) -> T) -> Result<T, RtError> {
    let name = window_name(c, a, 0)?;
    Ok(c.screen().window(&name).map_or(missing, f))
}

/// ROW(): the row the last `@` left the cursor on.
fn f_row(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    place(c.screen().cursor.0)
}

/// COL(): and the column.
fn f_col(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    place(c.screen().cursor.1)
}

/// PROW(): the same for the printer, which nothing here has printed to.
fn f_prow(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    place(c.screen().printer.0)
}

/// PCOL(): and its column.
fn f_pcol(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    place(c.screen().printer.1)
}

/// SROWS(): how many rows of characters the main screen holds.
fn f_srows(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    place(c.screen().main.rows)
}

/// SCOLS(): and how many columns.
fn f_scols(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    place(c.screen().main.cols)
}

/// WEXIST(cWindow): whether a window of that name has been defined.
fn f_wexist(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.trim().to_string();
    value(Value::Logical(c.screen().window(&name).is_some()))
}

/// WVISIBLE(cWindow): whether it is on the screen.
fn f_wvisible(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.trim().to_string();
    value(Value::Logical(c.screen().window(&name).is_some_and(|w| w.visible)))
}

/// WONTOP(): the name of the window in front, or whether the one named is.
fn f_wontop(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let top = c.screen().on_top().to_ascii_uppercase();
    match a.first() {
        None => value(Value::str(top)),
        Some(_) => {
            let name = arg_str(&a, 0)?.trim().to_ascii_uppercase();
            value(Value::Logical(!top.is_empty() && top == name))
        }
    }
}

/// WOUTPUT(): the name of the window output is going to, or whether the one named is it.
fn f_woutput(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let out = c.screen().output.to_ascii_uppercase();
    match a.first() {
        None => value(Value::str(out)),
        Some(_) => {
            let name = arg_str(&a, 0)?.trim().to_ascii_uppercase();
            value(Value::Logical(!out.is_empty() && out == name))
        }
    }
}

/// WCOLS(cWindow): how many columns of characters it holds.
fn f_wcols(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::number(about(c, &a, 0.0, |w| w.width)?))
}

/// WROWS(cWindow): and how many rows.
fn f_wrows(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::number(about(c, &a, 0.0, |w| w.height)?))
}

/// WLCOL(cWindow): the column its left edge sits on.
fn f_wlcol(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::number(about(c, &a, 0.0, |w| w.col)?))
}

/// WLROW(cWindow): the row its top edge sits on.
fn f_wlrow(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::number(about(c, &a, 0.0, |w| w.row)?))
}

/// WTITLE(cWindow): what its title bar says.
fn f_wtitle(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::str(about(c, &a, String::new(), |w| w.title.clone())?))
}

/// WPARENT(cWindow): the window it was put inside, or "" when it sits on the screen.
fn f_wparent(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::str(about(c, &a, String::new(), |w| w.parent.to_ascii_uppercase())?))
}

/// WCHILD(cWindow [, n]): the nth window inside it, or how many there are.
fn f_wchild(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = window_name(c, &a, 0)?;
    let children: Vec<&crate::screen::Window> =
        c.screen().windows.iter().filter(|w| w.parent.eq_ignore_ascii_case(&name)).collect();
    match a.get(1) {
        None => number(children.len()),
        Some(_) => {
            let at = arg_num(&a, 1)? as usize;
            let found = at.checked_sub(1).and_then(|i| children.get(i)).map(|w| w.name.to_ascii_uppercase());
            value(Value::str(found.unwrap_or_default()))
        }
    }
}

/// WBORDER([cWindow]): whether it was given a border.
fn f_wborder(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::Logical(about(c, &a, false, |w| w.border)?))
}

/// WMAXIMUM([cWindow]): whether it has been zoomed out to fill the screen.
fn f_wmaximum(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::Logical(about(c, &a, false, |w| w.zoomed)?))
}

/// WMINIMUM([cWindow]): whether it has been shrunk to its title.
fn f_wminimum(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::Logical(about(c, &a, false, |w| w.minimized)?))
}

/// WREAD([cWindow]): whether a READ is running in it.
fn f_wread(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = window_name(c, &a, 0)?;
    let reading = c.screen().read_level > 0 && c.screen().output.eq_ignore_ascii_case(&name);
    value(Value::Logical(reading))
}

/// WLAST([cWindow]): the window that had the output before this one, or whether it was that one.
fn f_wlast(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let last = c.screen().last.to_ascii_uppercase();
    match a.first() {
        None => value(Value::str(last)),
        Some(_) => {
            let name = arg_str(&a, 0)?.trim().to_ascii_uppercase();
            value(Value::Logical(!last.is_empty() && last == name))
        }
    }
}

/// WDOCKABLE(cWindow): whether it can be docked. A window drawn on the character screen
/// cannot, which is what Visual FoxPro says of one defined this way too.
fn f_wdockable(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let _ = window_name(c, &a, 0)?;
    value(Value::Logical(false))
}

/// MWINDOW(): the window the pointer is over, or whether it is over the one named.
fn f_mwindow(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (row, col, _) = c.host().mouse();
    let over = c
        .screen()
        .windows
        .iter()
        .rev()
        .find(|w| {
            w.visible && row >= w.row && row < w.row + w.height && col >= w.col && col < w.col + w.width
        })
        .map(|w| w.name.to_ascii_uppercase())
        .unwrap_or_default();
    match a.first() {
        None => value(Value::str(over)),
        Some(_) => {
            let name = arg_str(&a, 0)?.trim().to_ascii_uppercase();
            value(Value::Logical(!over.is_empty() && over == name))
        }
    }
}

/// The window MCOL/MROW ask about: a name, or the literal `0` its own page gives as another
/// way to say "the active one" - not a coercion of every function's window argument, just
/// theirs, so `window_name` is left alone for everything else that takes one.
fn mouse_window(c: &dyn BuiltinCtx, a: &[Value], i: usize) -> Result<String, RtError> {
    match a.get(i).map(|v| v.deref()) {
        None => Ok(c.screen().on_top().to_string()),
        Some(Value::Number(n, ..)) if n == 0.0 => Ok(c.screen().on_top().to_string()),
        Some(_) => Ok(arg_str(a, i)?.trim().to_string()),
    }
}

/// MROW([cWindow | 0] [, nScaleMode]): the row the pointer is on, counted inside that window.
/// `nScaleMode` chooses foxels (0, the default) or pixels (3); there is no pixel grid behind
/// this character screen, so both answer in foxels, the same approximation FONTMETRIC makes.
fn f_mrow(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (row, _, _) = c.host().mouse();
    let name = mouse_window(c, &a, 0)?;
    let top = c.screen().window(&name).map_or(0.0, |w| w.row);
    value(Value::number((row - top).max(0.0)))
}

/// MCOL([cWindow | 0] [, nScaleMode]): and the column.
fn f_mcol(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (_, col, _) = c.host().mouse();
    let name = mouse_window(c, &a, 0)?;
    let left = c.screen().window(&name).map_or(0.0, |w| w.col);
    value(Value::number((col - left).max(0.0)))
}

/// MDOWN(): whether a mouse button is down just now.
fn f_mdown(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let (_, _, down) = c.host().mouse();
    value(Value::Logical(down))
}

/// RDLEVEL(): how many READs are running, one inside another.
fn f_rdlevel(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::number(f64::from(c.screen().read_level)))
}

/// VARREAD(): what the field the user was last in is called.
fn f_varread(c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    value(Value::str(c.screen().read_var.clone()))
}

/// OBJNUM(cVar [, nReadLevel]): which of the fields on the screen reads that variable, counting
/// from one. `nReadLevel` names one of the READs nested inside one another right now - there is
/// nothing to ask about a level that is not actually running, 0 included, and the product
/// refuses that the same way it refuses any other bad argument. This runtime keeps one list of
/// fields rather than one per nesting level, so a level that is running answers the same as
/// leaving it out.
fn f_objnum(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let name = arg_str(&a, 0)?.trim().to_string();
    if a.len() > 1 {
        let level = arg_num(&a, 1)?.trunc() as i64;
        if level < 1 || level > i64::from(c.screen().read_level) {
            return Err(RtError::function_arg_invalid());
        }
    }
    let at = c.screen().gets.iter().position(|g| g.name.eq_ignore_ascii_case(&name));
    number(at.map_or(0, |i| i + 1))
}

/// OBJVAR(n): and the other way about - what the nth field reads.
fn f_objvar(c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let at = arg_num(&a, 0)? as usize;
    let name = at.checked_sub(1).and_then(|i| c.screen().gets.get(i)).map(|g| g.name.to_ascii_uppercase());
    value(Value::str(name.unwrap_or_default()))
}

/// The average width, in "pixels" this runtime has no font engine to measure for real, of one
/// character of the font this screen's own text answers `TXTWIDTH()` with when it is given
/// none of its own - the same ratio and default size `FONTMETRIC()` approximates with.
const TXTWIDTH_DEFAULT_FOXEL: f64 = 9.0 * 0.5;

/// TXTWIDTH(cText [, cFontName, nFontSize [, cFontStyle]]): how wide the text is in foxels -
/// the average character width of a font, the same unit a window's own columns are counted in.
/// With no font named, every character of this screen is one wide, so that is how many there
/// are, and the answer is the plain whole number a golden already measures; a font name means
/// giving its size too - one without the other is "Must specify additional parameters", the
/// product's own wording for a pair asked for half of - and the answer becomes an approximation
/// from the point size, twenty wide with three places, the shape FONTMETRIC's own answers take.
fn f_txtwidth(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let len = arg_str(&a, 0)?.chars().count();
    if a.len() < 2 {
        return number(len);
    }
    if a.len() < 3 {
        return Err(RtError::missing_parameter());
    }
    let size = arg_num(&a, 2)?.max(0.0);
    let style = opt_str(&a, 3, "")?.to_ascii_uppercase();
    let mut per_char = size * 0.5;
    if style.contains('B') {
        per_char *= 1.15;
    }
    let width = Width { chars: 20, decimals: 3, written: false };
    value(Value::Number(len as f64 * per_char / TXTWIDTH_DEFAULT_FOXEL, width))
}

/// The function keys a program can put a command on, in the order FKLABEL counts them.
fn function_keys() -> Vec<String> {
    let mut keys: Vec<String> = (2..=10).map(|n| format!("F{n}")).collect();
    for prefix in ["CTRL+", "SHIFT+", "ALT+"] {
        keys.extend((1..=10).map(|n| format!("{prefix}F{n}")));
    }
    keys.push("F1".to_string());
    keys
}

/// FKLABEL(n): the name of the nth key a command can be put on.
fn f_fklabel(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    let at = arg_num(&a, 0)? as usize;
    let keys = function_keys();
    value(Value::str(at.checked_sub(1).and_then(|i| keys.get(i)).cloned().unwrap_or_default()))
}

/// FKMAX(): how many there are.
fn f_fkmax(_c: &mut dyn BuiltinCtx, _a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    number(function_keys().len())
}

// ------------------------------------------------------------------------------------------
// the colour schemes
// ------------------------------------------------------------------------------------------
//
// A scheme is ten colour pairs and an eleventh field that is always "+". SCHEME() writes a pair
// as the letters FoxPro 2.x used for the sixteen character colours; RGBSCHEME() writes the same
// pair as numbers. Both are the same table read two ways.
//
// The product works the numbers out from the running Windows theme - 240,240,240 is the button
// face, 0,120,212 the highlight - so what is written here is the default light theme, as
// vfp9.exe answered it. Twenty-four schemes with four distinct sets of colours between them.

/// One colour pair: what SCHEME() calls it, and the foreground and background RGBSCHEME() gives.
type ColorPair = (&'static str, (u8, u8, u8), (u8, u8, u8));

/// A scheme is ten of them.
type SchemeRow = [ColorPair; 10];

const ON_A_WINDOW: SchemeRow = [
    ("N/W*", (0, 0, 0), (255, 255, 255)),
    ("N/W*", (0, 0, 0), (240, 240, 240)),
    ("W/W*", (100, 100, 100), (255, 255, 255)),
    ("N/W*", (0, 0, 0), (153, 180, 209)),
    ("N/W*", (0, 0, 0), (191, 205, 219)),
    ("W+/BG*", (255, 255, 255), (0, 120, 212)),
    ("W+/BG*", (255, 255, 255), (0, 120, 212)),
    ("N/N", (0, 0, 0), (0, 0, 0)),
    ("N/W*", (0, 0, 0), (255, 255, 255)),
    ("W/W*", (109, 109, 109), (255, 255, 255)),
];

const ON_PAPER: SchemeRow = [
    ("N/W*", (0, 0, 0), (255, 255, 255)),
    ("N/W*", (0, 0, 0), (255, 255, 255)),
    ("W/W*", (100, 100, 100), (255, 255, 255)),
    ("N/W*", (0, 0, 0), (153, 180, 209)),
    ("N/W*", (0, 0, 0), (191, 205, 219)),
    ("W+/BG*", (255, 255, 255), (0, 120, 212)),
    ("W+/BG*", (255, 255, 255), (0, 120, 212)),
    ("N/N", (0, 0, 0), (0, 0, 0)),
    ("N/W*", (0, 0, 0), (255, 255, 255)),
    ("W/W*", (109, 109, 109), (255, 255, 255)),
];

const ON_A_MENU: SchemeRow = [
    ("W/W*", (109, 109, 109), (240, 240, 240)),
    ("N/W*", (0, 0, 0), (240, 240, 240)),
    ("W/W*", (100, 100, 100), (240, 240, 240)),
    ("N/W*", (0, 0, 0), (191, 205, 219)),
    ("N/W*", (0, 0, 0), (240, 240, 240)),
    ("W+/BG*", (255, 255, 255), (0, 120, 212)),
    ("W+/N", (255, 255, 255), (0, 0, 0)),
    ("W/W*", (160, 160, 160), (240, 240, 240)),
    ("N/W*", (0, 0, 0), (240, 240, 240)),
    ("W/W*", (109, 109, 109), (240, 240, 240)),
];

const ON_A_DIALOG: SchemeRow = [
    ("N/W*", (0, 0, 0), (240, 240, 240)),
    ("N/W*", (0, 0, 0), (255, 255, 255)),
    ("W/W*", (100, 100, 100), (255, 255, 255)),
    ("N/W*", (0, 0, 0), (153, 180, 209)),
    ("N/W*", (0, 0, 0), (191, 205, 219)),
    ("W+/BG*", (255, 255, 255), (0, 120, 212)),
    ("W+/BG*", (255, 255, 255), (0, 120, 212)),
    ("N/N", (0, 0, 0), (0, 0, 0)),
    ("N/W*", (0, 0, 0), (255, 255, 255)),
    ("W/W*", (160, 160, 160), (240, 240, 240)),
];

/// Which set of colours each of the twenty-four schemes uses, scheme 1 first.
const SCHEMES: [&SchemeRow; 24] = [
    &ON_A_WINDOW, &ON_PAPER, &ON_A_MENU, &ON_A_MENU, &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_PAPER,
    &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_A_MENU, &ON_PAPER, &ON_A_DIALOG, &ON_PAPER,
    &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_PAPER, &ON_PAPER,
];

/// The eleventh field of every scheme, which is what the product puts there.
const LAST_FIELD: &str = "+";

/// One scheme written out, either way round: the pair the call asked for, or all of them with
/// commas between.
///
/// The scheme number runs 1 to 24 and the pair 1 to 11; 0 asks for the whole scheme, as leaving
/// the argument out does. Anything outside that is a bad argument, which is what the product
/// says of it.
fn scheme_text(a: &[Value], write: fn(&ColorPair) -> String) -> Result<BuiltinResult, RtError> {
    let which = arg_num(a, 0)? as i64;
    let pair = match a.get(1) {
        Some(v) => v.deref().as_number().map_err(|_| RtError::function_arg_invalid())? as i64,
        None => 0,
    };
    if !(1..=SCHEMES.len() as i64).contains(&which) || !(0..=11).contains(&pair) {
        return Err(RtError::function_arg_invalid());
    }
    let row = SCHEMES[which as usize - 1];
    if pair == 11 {
        return value(Value::str(LAST_FIELD));
    }
    if pair > 0 {
        return value(Value::str(write(&row[pair as usize - 1])));
    }
    let mut all: Vec<String> = row.iter().map(write).collect();
    all.push(LAST_FIELD.to_string());
    value(Value::str(all.join(",")))
}

/// `SCHEME(nScheme [, nPair])`: a colour pair in the letters FoxPro has always written them in.
fn f_scheme(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    scheme_text(&a, |(letters, _, _)| (*letters).to_string())
}

/// `RGBSCHEME(nScheme [, nPair])`: the same pair as six numbers, foreground then background.
fn f_rgbscheme(_c: &mut dyn BuiltinCtx, a: Vec<Value>) -> Result<BuiltinResult, RtError> {
    scheme_text(&a, |(_, fore, back)| {
        format!("RGB({},{},{},{},{},{})", fore.0, fore.1, fore.2, back.0, back.1, back.2)
    })
}

pub fn specs() -> Vec<BuiltinSpec> {
    vec![
        spec("COL", 0, 0, f_col),
        spec("FKLABEL", 1, 1, f_fklabel),
        spec("FKMAX", 0, 0, f_fkmax),
        spec("MCOL", 0, 2, f_mcol),
        spec("MDOWN", 0, 0, f_mdown),
        spec("MROW", 0, 2, f_mrow),
        spec("MWINDOW", 0, 1, f_mwindow),
        spec("OBJNUM", 1, 2, f_objnum),
        spec("OBJVAR", 1, 1, f_objvar),
        spec("PCOL", 0, 0, f_pcol),
        spec("PROW", 0, 0, f_prow),
        spec("RDLEVEL", 0, 0, f_rdlevel),
        spec("RGBSCHEME", 1, 2, f_rgbscheme),
        spec("ROW", 0, 0, f_row),
        spec("SCHEME", 1, 2, f_scheme),
        spec("SCOLS", 0, 0, f_scols),
        spec("SROWS", 0, 0, f_srows),
        spec("TXTWIDTH", 1, 4, f_txtwidth),
        spec("VARREAD", 0, 0, f_varread),
        spec("WBORDER", 0, 1, f_wborder),
        spec("WCHILD", 0, 2, f_wchild),
        spec("WCOLS", 0, 1, f_wcols),
        spec("WDOCKABLE", 1, 1, f_wdockable),
        spec("WEXIST", 1, 1, f_wexist),
        spec("WLAST", 0, 1, f_wlast),
        spec("WLCOL", 0, 1, f_wlcol),
        spec("WLROW", 0, 1, f_wlrow),
        spec("WMAXIMUM", 0, 1, f_wmaximum),
        spec("WMINIMUM", 0, 1, f_wminimum),
        spec("WONTOP", 0, 1, f_wontop),
        spec("WOUTPUT", 0, 1, f_woutput),
        spec("WPARENT", 0, 1, f_wparent),
        spec("WREAD", 0, 1, f_wread),
        spec("WROWS", 0, 1, f_wrows),
        spec("WTITLE", 0, 1, f_wtitle),
        spec("WVISIBLE", 1, 1, f_wvisible),
    ]
}
