//! Runtime values and the VFP rules for comparing, combining and displaying them.
//!
//! Numbers are f64 (VFP's Numeric); `SET DECIMALS` affects display only. Dates are days
//! since 1970-01-01, datetimes are seconds since 1970-01-01 00:00:00 with no time zone.
//! `Ref` is an internal cell used for by-reference parameters and never reaches the host.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::error::RtError;

/// Host-side object identity (a form, a control, an error object...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(pub u32);

/// Identity of a function value in the VM's own table. Two function values are the same function
/// when their ids are equal, which is what `=` between them answers - the same rule objects
/// already follow, measured in vfp9.exe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

pub type ArrayRef = Rc<RefCell<FoxArray>>;
pub type RefCell_ = Rc<RefCell<Value>>;

/// A VFP array: 1-based, one or two dimensions, row-major, elements default to .F.
#[derive(Debug, Clone, PartialEq)]
pub struct FoxArray {
    pub rows: usize,
    /// 0 for a one-dimensional array.
    pub cols: usize,
    pub items: Vec<Value>,
}

impl FoxArray {
    pub fn new(rows: usize, cols: usize) -> Self {
        let n = rows * cols.max(1);
        FoxArray { rows, cols, items: vec![Value::Logical(false); n] }
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Element index from 1-based subscripts; a single subscript on a 2-D array is the linear index.
    pub fn index(&self, subs: &[usize]) -> Result<usize, RtError> {
        match subs {
            [i] if *i >= 1 && *i <= self.items.len() => Ok(i - 1),
            [r, c] if self.cols > 0 && *r >= 1 && *r <= self.rows && *c >= 1 && *c <= self.cols => {
                Ok((r - 1) * self.cols + (c - 1))
            }
            _ => Err(RtError::invalid_subscript()),
        }
    }
    pub fn get(&self, subs: &[usize]) -> Result<Value, RtError> {
        Ok(self.items[self.index(subs)?].clone())
    }
    pub fn set(&mut self, subs: &[usize], v: Value) -> Result<(), RtError> {
        let i = self.index(subs)?;
        self.items[i] = v;
        Ok(())
    }
    /// A one-dimensional array of the given elements.
    pub fn of(items: Vec<Value>) -> Self {
        FoxArray { rows: items.len(), cols: 0, items }
    }

    /// DIMENSION on an existing array keeps the elements that still fit (row-major).
    pub fn redim(&mut self, rows: usize, cols: usize) {
        let mut next = FoxArray::new(rows, cols);
        for (i, v) in self.items.iter().enumerate() {
            if i < next.items.len() {
                next.items[i] = v.clone();
            }
        }
        *self = next;
    }
}

/// The width a number prints in, and how many places past the point it shows.
///
/// A Visual FoxPro number is not only a double: it carries a width, and `?` right-aligns the
/// value inside it. `? 8` writes "8" and `? 4 * 2` writes "  8", because a one-wide number
/// times a one-wide number makes a three-wide one; `? n` where `n = 4` writes nine spaces and
/// a 4, because a variable's whole part is ten wide whatever was put in it. Every rule here
/// was read off vfp9.exe - `crates/foxvm/tests/programs/ref_numwidth.prg` is the same program
/// run there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Width {
    /// Characters the value is right-aligned in.
    pub chars: u8,
    /// Places shown past the point; 0 means no point at all.
    pub decimals: u8,
    /// True while this is still a number the program wrote down. Visual FoxPro's compiler works
    /// out arithmetic between two written numbers itself, and it does that by different rules
    /// from the ones the running program uses - `? 1.5 * 2` is "  3.00" where `m = 1.5` and
    /// `? m * 2` is "           3.0". The flag is what tells the two apart.
    pub written: bool,
}

/// What a variable gives the whole part of whatever is stored in it. Ten, always, unless the
/// number itself has more digits than that.
pub const VARIABLE_WHOLE: u8 = 10;

/// The widest a number may print.
///
/// Visual FoxPro stops widening at forty characters, measured by watching a product grow:
/// `n * Fact(n - 1)` on a ten-wide parameter reaches 12, 23 and 34 characters for the first
/// three levels and then stays at 40 however deep it goes, and `a * a * a * a` on a ten-wide
/// variable reaches 40 the same way rather than 43.
const WIDEST: u8 = 40;

impl Width {
    /// A width from a whole part and a count of decimal places, with room for the point.
    pub fn of(whole: u8, decimals: u8) -> Width {
        let chars = whole.saturating_add(decimals).saturating_add(u8::from(decimals > 0));
        Width { chars: chars.min(WIDEST), decimals, written: false }
    }

    /// The digits in front of the point: the width less the point and the places behind it.
    pub fn whole(self) -> u8 {
        self.chars.saturating_sub(self.decimals + u8::from(self.decimals > 0))
    }

    /// The same width on a number the program wrote down.
    fn as_written(mut self) -> Width {
        self.written = true;
        self
    }

    /// The width a number written in the program keeps: the characters it was written with.
    ///
    /// `? 8` is "8" and `? 001` is "  1" - the zeros are not printed but the room they were
    /// written in is still there. A bare point counts the zero Visual FoxPro puts in front of
    /// it, so `.5` is three wide. Hexadecimal counts its digits and one more, and a number
    /// written with an exponent counts the digits it stands for.
    pub fn written(text: &str) -> Width {
        let text = text.trim();
        if let Some(digits) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            return Width::of(digits.len().min(WIDEST as usize) as u8 + 1, 0).as_written();
        }
        let (mantissa, exponent) = match text.split_once(['e', 'E']) {
            Some((m, e)) => (m, e.parse::<i32>().unwrap_or(0)),
            None => (text, 0),
        };
        let (whole, decimals) = match mantissa.split_once('.') {
            // `.5` prints as `0.5`, so the zero Visual FoxPro writes counts towards the width
            Some((w, d)) => (w.len().max(1), d.len()),
            None => (mantissa.len(), 0),
        };
        // an exponent moves the point: the digits it stands for are the width it asks for, and
        // Visual FoxPro adds one place for each digit of the exponent itself
        let places = decimals as i32 - exponent;
        let digits = whole as i32 + exponent.max(0) + i32::from(exponent > 0) * exponent.to_string().len() as i32;
        let clamp = |n: i32| n.clamp(0, WIDEST as i32) as u8;
        Width::of(clamp(digits), clamp(places)).as_written()
    }

    /// The width a field of a table gives what is read out of it: the one it was declared with.
    ///
    /// A `N(8,2)` field prints eight wide with two places whatever is in it, which is why
    /// `? price` on a 3200 is "   3200.00". Integer and Double are numbers of their own kind
    /// and carry widths of their own - eleven and twenty-one - and a Double with no declared
    /// places shows as many as the number itself needs.
    pub fn field(kind: char, length: u8, decimals: u8, value: f64) -> Width {
        match kind {
            'I' | '+' => Width { chars: 11, decimals: 0, written: false },
            'Y' => CURRENCY_WIDTH,
            'B' | 'O' => {
                let places = if decimals > 0 { decimals } else { places_needed(value) };
                Width { chars: 21, decimals: places, written: false }
            }
            _ => Width { chars: length.max(1), decimals, written: false },
        }
    }

    /// The width a memory variable gives whatever is stored in it.
    ///
    /// The whole part is ten wide however narrow the number was - `n = 4` then `? n` is nine
    /// spaces and a 4 - and wider than ten only when the number itself has more digits. The
    /// places behind the point are the ones the value arrived with, so `x = 1.50` still shows
    /// two of them.
    pub fn of_variable(value: f64, decimals: u8) -> Width {
        if !value.is_finite() {
            // no digits to count; whatever it was stays as wide as it was
            return Width::of(VARIABLE_WHOLE, decimals);
        }
        let digits = whole_digits(value);
        Width::of(digits.max(VARIABLE_WHOLE), decimals)
    }
}

/// A value as a memory variable holds it.
///
/// A variable gives a number a width of its own: ten for the whole part, and the places the
/// number arrived with. That is why `n = 4` then `? n` writes nine spaces and a 4, and why
/// `x = price` on a `N(8,2)` field prints wider than the field did - the field's eight
/// characters do not follow the value into the variable, only its two places do.
pub fn held_in_variable(v: Value) -> Value {
    match v {
        Value::Number(n, w) => Value::Number(n, Width::of_variable(n, w.decimals)),
        other => other,
    }
}

/// How many characters the whole part of a number takes, sign included.
fn whole_digits(value: f64) -> u8 {
    let text = format!("{:.0}", value.trunc().abs());
    let sign = u8::from(value < 0.0);
    (text.len().min(WIDEST as usize) as u8).saturating_add(sign)
}

/// The fewest places past the point that write a number exactly, as far as a double is exact.
pub fn places_needed(value: f64) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    let text = to_places(value, EXACT_PLACES as usize);
    let trimmed = text.trim_end_matches('0');
    match trimmed.split_once('.') {
        Some((_, places)) => places.len() as u8,
        None => 0,
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Logical(bool),
    /// A number and the width it prints in; `Value::number` gives one the width a variable has.
    Number(f64, Width),
    /// Money, in ten-thousandths. Visual FoxPro's Currency is a 64-bit whole number of them,
    /// which is why `$0.1 + $0.2 = $0.3` is true there and `0.1 + 0.2 = 0.3` needs rounding to
    /// be. `MTON()` hands over the same amount as a Number and `NTOM()` brings one back.
    Currency(i64),
    Str(Rc<str>),
    /// Days since 1970-01-01; `None` is the empty date `{}`.
    Date(Option<i32>),
    /// Seconds since 1970-01-01 00:00:00; `None` is the empty datetime `{/:}`.
    DateTime(Option<f64>),
    Object(Handle),
    /// A lambda: an id into the table of function values the VM owns, exactly as an object is a
    /// handle into the table the host owns. VARTYPE() answers "F"; see docs/foxscript.md.
    Function(FuncId),
    /// A JSON value: an ordered map, arrays, numbers, strings, booleans and null. Not a FoxPro
    /// type, and not an object either - `Empty` plus `ADDPROPERTY` has no arrays, no null and no
    /// order. VARTYPE() answers "J"; see docs/foxscript.md.
    Json(Rc<serde_json::Value>),
    Array(ArrayRef),
    /// By-reference cell; `deref()` before use.
    Ref(RefCell_),
}

impl PartialEq for Value {
    /// Structural equality used by tests and the bytecode round trip; VFP comparison is `compare`.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Logical(a), Value::Logical(b)) => a == b,
            (Value::Number(a, ..), Value::Number(b, ..)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::Json(a), Value::Json(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Value::Ref(a), Value::Ref(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Logical(b)
    }
}
impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::number(n)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(Rc::from(s))
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(Rc::from(s))
    }
}

/// How `SET()` reports a setting this runtime only remembers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingShape {
    /// ON or OFF, however the command spelled it.
    Switch,
    /// A number, reported as one.
    Count,
    /// A word of the setting's own vocabulary, reported as it was given.
    Word,
}

/// The settings a program may set and ask about that change nothing here, with the answer
/// `vfp9.exe` gives for each before anything has set it.
///
/// Visual FoxPro reports every setting whatever it does with it, and real programs save one,
/// change it and put it back around a piece of work. That has to keep working where the
/// setting itself has nothing to act on - one process rather than a network, a window rather
/// than a screen of characters, no printer - so each of these is remembered and reported and
/// no more. Every default below was read out of vfp9.exe rather than off a reference page.
pub const REMEMBERED: &[(&str, SettingShape, &str)] = &[
    ("ANSI", SettingShape::Switch, "OFF"),
    ("AUTOINCERROR", SettingShape::Switch, "ON"),
    ("AUTOSAVE", SettingShape::Switch, "OFF"),
    ("BELL", SettingShape::Switch, "ON"),
    ("BLOCKSIZE", SettingShape::Count, "64"),
    ("BROWSEIME", SettingShape::Switch, "OFF"),
    ("BRSTATUS", SettingShape::Switch, "OFF"),
    ("CARRY", SettingShape::Switch, "OFF"),
    ("CLASSLIB", SettingShape::Word, ""),
    // the list itself is the VM's; this is only what SET() answers when nothing is on it
    ("PROCEDURE", SettingShape::Word, ""),
    ("CLEAR", SettingShape::Switch, "ON"),
    ("CLOCK", SettingShape::Switch, "OFF"),
    ("COLLATE", SettingShape::Word, "MACHINE"),
    ("COMPATIBLE", SettingShape::Switch, "OFF"),
    ("CONFIRM", SettingShape::Switch, "OFF"),
    ("CONSOLE", SettingShape::Switch, "ON"),
    ("COVERAGE", SettingShape::Word, ""),
    ("CPCOMPILE", SettingShape::Count, "1252"),
    ("CPDIALOG", SettingShape::Switch, "ON"),
    ("CURSOR", SettingShape::Switch, "ON"),
    ("DATASESSION", SettingShape::Count, "1"),
    ("DELIMITERS", SettingShape::Switch, "OFF"),
    ("DEVELOPMENT", SettingShape::Switch, "ON"),
    ("DEVICE", SettingShape::Word, "SCREEN"),
    ("DISPLAY", SettingShape::Word, "VGA25"),
    ("DOHISTORY", SettingShape::Switch, "OFF"),
    ("ENGINEBEHAVIOR", SettingShape::Count, "90"),
    ("EVENTTRACKING", SettingShape::Switch, "OFF"),
    ("EXCLUSIVE", SettingShape::Switch, "ON"),
    // whether the list SET FIELDS TO named is in force; FLDLIST() reports the list itself
    ("FIELDS", SettingShape::Switch, "OFF"),
    ("FORMAT", SettingShape::Word, ""),
    ("FULLPATH", SettingShape::Switch, "ON"),
    ("HEADINGS", SettingShape::Switch, "ON"),
    ("HELP", SettingShape::Switch, "ON"),
    ("INTENSITY", SettingShape::Switch, "ON"),
    ("KEYCOMP", SettingShape::Word, "WINDOWS"),
    ("LIBRARY", SettingShape::Word, ""),
    ("LOCK", SettingShape::Switch, "OFF"),
    ("LOGERRORS", SettingShape::Switch, "ON"),
    ("MACKEY", SettingShape::Word, ""),
    ("MARGIN", SettingShape::Count, "0"),
    ("MULTILOCKS", SettingShape::Switch, "OFF"),
    ("NOTIFY", SettingShape::Switch, "ON"),
    ("NULL", SettingShape::Switch, "OFF"),
    ("ODOMETER", SettingShape::Count, "100"),
    ("OLEOBJECT", SettingShape::Switch, "ON"),
    ("OPTIMIZE", SettingShape::Switch, "ON"),
    ("PALETTE", SettingShape::Switch, "ON"),
    ("PDSETUP", SettingShape::Word, ""),
    ("PRINTER", SettingShape::Switch, "OFF"),
    ("READBORDER", SettingShape::Switch, "OFF"),
    ("REFRESH", SettingShape::Count, "0"),
    ("REPROCESS", SettingShape::Count, "0"),
    ("RESOURCE", SettingShape::Switch, "ON"),
    ("SPACE", SettingShape::Switch, "ON"),
    ("STATUS", SettingShape::Switch, "OFF"),
    ("STATUS BAR", SettingShape::Switch, "ON"),
    ("STRICTDATE", SettingShape::Count, "1"),
    ("SYSFORMATS", SettingShape::Switch, "OFF"),
    ("TABLEVALIDATE", SettingShape::Count, "3"),
    ("TOPIC", SettingShape::Word, ""),
    ("TRBETWEEN", SettingShape::Switch, "OFF"),
    ("TYPEAHEAD", SettingShape::Count, "20"),
    ("UDFPARMS", SettingShape::Word, "VALUE"),
    ("VIEW", SettingShape::Word, ""),
];

/// Whether a file name already carries an extension of its own.
pub fn has_extension(name: &str) -> bool {
    let stem = name.rsplit(['/', '\\']).next().unwrap_or(name);
    stem.contains('.')
}

/// The settings that answer something else when `SET()` is given a second argument, with what
/// a fresh product answers for the second and the third.
///
/// `SET(cSetting, nSecondSetting)` is documented for every setting, but only some of them have
/// a second half to report: `SET("CONSOLE", 1)` is the switch again, where `SET("HELP", 1)` is
/// the help file and `SET("DELIMITERS", 1)` the two characters. Every one of these was read out
/// of vfp9.exe by asking all ninety settings for all three answers; a name not here has one
/// answer, whatever it is asked.
///
/// `HELP` and `RESOURCE` answer a path in the product - its own help file, its own foxuser.dbf
/// - and empty here, because this product has neither. `last` is what the third argument
/// answers, which is the second again unless the product said otherwise.
pub struct SecondAnswer {
    pub second: &'static str,
    pub last: &'static str,
    /// The width a numeric answer carries, measured: `SET("REFRESH", 1)` prints as twenty
    /// characters with three places, where `SET("TOPIC", 1)` prints as a plain ten-wide zero.
    pub numeric: Option<(u8, u8)>,
}

pub fn second_answer(name: &str) -> Option<SecondAnswer> {
    let text = |second, last| Some(SecondAnswer { second, last, numeric: None });
    let number = |second, last, width| Some(SecondAnswer { second, last, numeric: Some(width) });
    match name {
        // SET BELL TO <sound file>, SET CLOCK TO <row, col>, SET PRINTER TO <port or file>
        "BELL" | "CLOCK" | "PRINTER" | "DOHISTORY" | "EVENTTRACKING" | "HELP" | "RESOURCE" => text("", ""),
        "COMPATIBLE" => text("PROMPT", "PROMPT"),
        "DELIMITERS" => text("::", "::"),
        // the fields of the list are local or remote, which is the third answer, not the second
        "FIELDS" => text("", "LOCAL"),
        "TALK" => text("NOWINDOW", "NOWINDOW"),
        "REFRESH" => number("5", "5", (20, 3)),
        "TOPIC" => number("0", "0", (10, 0)),
        _ => None,
    }
}

/// The shape and default of a remembered setting, by the name the reference gives it.
pub fn remembered(name: &str) -> Option<(SettingShape, &'static str)> {
    REMEMBERED.iter().find(|(n, _, _)| *n == name).map(|(_, shape, default)| (*shape, *default))
}

/// `SET` state that affects value semantics or display.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub exact: bool,
    pub decimals: u8,
    pub century: bool,
    pub date_format: DateFormat,
    pub talk: bool,
    pub safety: bool,
    pub escape: bool,
    /// Running as a built application rather than in the IDE: what `VERSION(2)` answers, 0 for
    /// the runtime and 2 for the development edition. The IDE's player turns it on.
    pub runtime_only: bool,
    /// SET DELETED ON hides deleted records from every movement command.
    pub deleted: bool,
    /// SET NEAR ON leaves the pointer at the nearest record when a SEEK misses, rather than at
    /// end of file.
    pub near: bool,
    /// SET UNIQUE ON: an index built from then on keeps one record per key.
    pub unique: bool,
    /// SET MEMOWIDTH: the width a memo is taken to wrap at, which is what decides the line a
    /// character of it sits on. 50 is VFP default.
    pub memowidth: u8,
    /// SET HEADINGS ON (the default): LIST and DISPLAY of records write a row of field names
    /// above them. It says nothing about LIST STRUCTURE, which always names its columns.
    pub headings: bool,
    /// SET FIXED ON: every number is shown with `decimals` places, whole ones included.
    pub fixed: bool,
    /// SET POINT TO: the character written where the decimal point goes.
    pub point: char,
    /// SET SEPARATOR TO: the character between groups of three digits, which only a picture
    /// with a comma in it ever asks for.
    pub separator: char,
    /// SET CURRENCY TO: the symbol a `@$` picture puts beside the number, one to nine
    /// characters.
    pub currency: String,
    /// SET CURRENCY LEFT (the default) or RIGHT: which side of the number the symbol goes.
    pub currency_left: bool,
    /// SET NULLDISPLAY TO: the text shown for .NULL.
    pub null_display: String,
    /// SET MARK TO: a character that replaces the separator every date format carries of its
    /// own. `None` is `SET MARK TO` with nothing after it, which puts each format's own back.
    pub mark: Option<char>,
    /// SET HOURS TO 24: a datetime's time written on the 24-hour clock rather than with AM/PM.
    pub hours24: bool,
    /// SET SECONDS ON (the default): seconds shown in the time part of a datetime.
    pub seconds: bool,
    /// SET FDOW TO: the first day of the week, 1 Sunday .. 7 Saturday. Only DOW(d, 0) and
    /// WEEK(d, n, 0) ask for it; both default to Sunday when the argument is left out.
    pub fdow: u8,
    /// SET FWEEK TO: what the first week of a year is, 1 the week holding January 1, 2 the
    /// first week with four days in the year, 3 the first full week. Only WEEK(d, 0, n) asks.
    pub fweek: u8,
    /// SET CENTURY TO nCentury ROLLOVER nYear: which century a two-digit year is read in.
    /// `None` is the default, which cannot be written down because it is worked out from
    /// today's date - see `rollover`.
    pub century_to: Option<(i32, i32)>,
    /// SET DEFAULT TO: the folder a relative path is taken from. Empty means the one the host
    /// would use anyway, which is where the project lives.
    pub default_dir: String,
    /// SET PATH TO: the folders a relative name is looked for in after the default directory,
    /// as the command wrote them and upper-cased, which is how the product reports it -
    /// measured: `SET PATH TO a,b` leaves `SET("PATH")` answering `A,B`.
    pub path: String,
    /// SET ASSERTS: whether a failed assertion says so. Off is what VFP starts with.
    pub asserts: bool,
    /// SET ECHO: the Trace window follows the running program line by line.
    pub echo: bool,
    /// SET DEBUGOUT TO: the file ASSERT and DEBUGOUT write to as well, empty for none.
    pub debugout: String,
    /// Whether that file has been written to since the SET DEBUGOUT that named it. The first
    /// write replaces what was there and the rest add to it, unless ADDITIVE said otherwise.
    pub debugout_started: bool,
    /// SET TEXTMERGE ON|OFF: whether `\`, `\\` and a TEXT block work out what stands between
    /// the merge delimiters. Visual FoxPro starts with it off.
    pub textmerge: bool,
    /// SET TEXTMERGE DELIMITERS TO: what marks the start and the end of an expression in merged
    /// text, `<<` and `>>` until something says otherwise.
    pub textmerge_delimiters: (String, String),
    /// SET TEXTMERGE TO: the file or variable that takes what `\`, `\\` and a TEXT block with
    /// nothing named after it put out. Empty is the screen.
    pub textmerge_to: String,
    /// Whether that destination is a variable being built rather than a file being written.
    pub textmerge_memvar: bool,
    /// SET TEXTMERGE ... NOSHOW: the output goes to the destination without also being shown.
    pub textmerge_noshow: bool,
    /// SET FIELDS TO: the columns a program has said it wants to see, as FLDLIST() reports
    /// them. Empty until a list is named; `SET FIELDS TO` with nothing after it clears it.
    pub fields: Vec<String>,
    /// What the settings in `REMEMBERED` were last set to, for the ones that were set at all.
    /// Everything else answers with the default beside its name there.
    pub remembered: std::collections::BTreeMap<String, String>,
    /// What the `TO` form of those settings named, which is not the same answer as the switch:
    /// `SET HELP OFF` and `SET HELP TO afile` can both be in force at once, and `SET("HELP")`
    /// reports the first where `SET("HELP", 1)` reports the second. Measured: a fresh product
    /// answers the file with its whole path, upper-cased, and an empty string for a setting
    /// nothing has named a target for.
    pub targets: std::collections::BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            exact: false,
            decimals: 2,
            century: false,
            date_format: DateFormat::American,
            talk: true,
            safety: true,
            escape: true,
            runtime_only: false,
            deleted: false,
            near: false,
            unique: false,
            memowidth: 50,
            headings: true,
            fixed: false,
            point: '.',
            separator: ',',
            currency: "$".to_string(),
            currency_left: true,
            null_display: ".NULL.".to_string(),
            mark: None,
            hours24: false,
            seconds: true,
            fdow: 1,
            fweek: 1,
            century_to: None,
            default_dir: String::new(),
            path: String::new(),
            asserts: false,
            echo: false,
            debugout: String::new(),
            debugout_started: false,
            textmerge: false,
            textmerge_delimiters: ("<<".to_string(), ">>".to_string()),
            textmerge_to: String::new(),
            textmerge_memvar: false,
            textmerge_noshow: false,
            fields: Vec::new(),
            remembered: std::collections::BTreeMap::new(),
            targets: std::collections::BTreeMap::new(),
        }
    }
}

impl Settings {
    /// What `SET ENGINEBEHAVIOR` was last told, which is 90 until something says otherwise.
    ///
    /// It is the version of Visual FoxPro whose SQL rules the engine follows. 70 is the one the
    /// language elements ask about: under it a query may name a column it neither groups by nor
    /// aggregates, in the select list and in a HAVING clause alike, where 90 refuses.
    pub fn engine_behavior(&self) -> u32 {
        self.remembered.get("ENGINEBEHAVIOR").and_then(|v| v.trim().parse().ok()).unwrap_or(90)
    }

    /// `SET NULL`: whether a column that says neither NULL nor NOT NULL accepts `.NULL.`.
    ///
    /// Measured - with `SET NULL ON`, `CREATE CURSOR t (a c(5), b n(10))` makes both columns
    /// take a null; the product starts with it OFF.
    pub fn nulls_by_default(&self) -> bool {
        self.remembered.get("NULL").is_some_and(|v| v.eq_ignore_ascii_case("ON"))
    }

    /// A path as the host should see it: relative names are taken from the default directory.
    pub fn at(&self, path: &str) -> String {
        let path = path.trim();
        if self.default_dir.is_empty() || path.is_empty() || is_rooted(path) {
            return path.to_string();
        }
        let dir = self.default_dir.trim_end_matches(['/', '\\']);
        format!("{dir}\\{path}")
    }

    /// The folders `SET PATH TO` named, each one as the host should see it.
    ///
    /// Measured in Visual FoxPro 9: a comma and a semicolon both separate entries and nothing
    /// else does - `SET PATH TO a b` is one entry called `a b`, and finds nothing - and an entry
    /// that does not say where it starts is read from the default directory, so with the default
    /// on `<run>\a` a path of `b` looks in `<run>\a\b` and not in `<run>\b`.
    pub fn search_dirs(&self) -> Vec<String> {
        self.path
            .split([',', ';'])
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| self.at(entry))
            .collect()
    }

    /// Where else `name` is to be looked for, after the place `at` puts it.
    ///
    /// Measured: `USE t` with `SET PATH TO a,b` opens the one in the default directory when
    /// there is one there and otherwise `a\t.dbf` before `b\t.dbf`, so the default directory
    /// comes first and the entries in the order they were written. A name that carries a folder
    /// of its own is not looked for on the path at all: `USE sub\t` with a path set says
    /// `sub\t.dbf` does not exist rather than trying `a\sub\t.dbf`.
    pub fn search(&self, name: &str) -> Vec<String> {
        let name = name.trim();
        if self.path.is_empty() || name.is_empty() || is_rooted(name) || name.contains(['/', '\\']) {
            return Vec::new();
        }
        self.search_dirs().into_iter().map(|dir| format!("{}\\{name}", dir.trim_end_matches(['/', '\\']))).collect()
    }

    /// Where else a table named `name` is to be looked for, the missing `.dbf` filled in as
    /// `table_at` fills it.
    pub fn table_search(&self, name: &str) -> Vec<String> {
        let name = name.trim();
        let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if file.contains('.') || file.is_empty() { self.search(name) } else { self.search(&format!("{name}.dbf")) }
    }

    /// The century a two-digit year belongs to and the year it rolls over at. `SET CENTURY TO`
    /// with nothing after it puts the default back, and the default is read off today's date:
    /// the century is this one, or the one before it while we are still in the first half of a
    /// century, and the rollover is fifty years from now. A year at or above the rollover is in
    /// `century`, one below it in the century after.
    pub fn rollover(&self, today_year: i32) -> (i32, i32) {
        self.century_to.unwrap_or_else(|| {
            let century = if today_year.rem_euclid(100) < 50 { today_year / 100 - 1 } else { today_year / 100 };
            (century, (today_year + 50).rem_euclid(100))
        })
    }

    /// The four-digit year a date written with `digits` digits of year means.
    pub fn full_year(&self, year: i32, digits: usize, today_year: i32) -> i32 {
        if digits > 2 {
            return year;
        }
        let (century, rollover) = self.rollover(today_year);
        (if year >= rollover { century } else { century + 1 }) * 100 + year
    }

    /// A table as the host should see it. `USE customer` means `customer.dbf`: a name with no
    /// extension of its own gets one, which is how every table in a data environment is written.
    pub fn table_at(&self, name: &str) -> String {
        let name = name.trim();
        let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if file.contains('.') || file.is_empty() { self.at(name) } else { self.at(&format!("{name}.dbf")) }
    }

    /// A class library as the host should see it. `SET CLASSLIB TO ..\solution` and
    /// `NEWOBJECT("x", "europa")` both name `.vcx` files without saying so - measured - and both
    /// are read from the default directory, as every relative name in the product is.
    pub fn class_library_at(&self, name: &str) -> String {
        let name = name.trim();
        let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if file.contains('.') || file.is_empty() { self.at(name) } else { self.at(&format!("{name}.vcx")) }
    }
}

/// True when a path says where it starts: a drive letter, a leading slash, or a UNC name.
/// Where `CD given` lands when the program is in `base`.
///
/// `CD` and `SET DEFAULT TO` name a folder relative to the one the program is already in, so
/// `CD shed` and then `CD ..` is back where it started rather than in a folder called `..`.
/// A rooted path replaces the lot, and naming nothing puts back the host's own folder.
pub fn join_dir(base: &str, given: &str) -> String {
    let given = given.trim();
    if given.is_empty() {
        return String::new();
    }
    if is_rooted(given) {
        return given.to_string();
    }
    let mut parts: Vec<&str> = base.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    for part in given.split(['/', '\\']).filter(|p| !p.is_empty()) {
        match part {
            "." => {}
            // above the folder the host would use anyway, which has no name here
            ".." => {
                parts.pop();
            }
            name => parts.push(name),
        }
    }
    parts.join("\\")
}

/// True when a path says where it starts: a drive letter, a leading slash, or a UNC name.
pub fn is_rooted(path: &str) -> bool {
    let bytes = path.as_bytes();
    matches!(bytes.first(), Some(b'/' | b'\\')) || (bytes.len() >= 2 && bytes[1] == b':')
}

/// A word `SET DATE` takes. Several of them lay a date out the same way - MDY is AMERICAN,
/// FRENCH is BRITISH - but SET("DATE") answers with the word that was set, so each keeps its
/// own name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    American,
    Ansi,
    British,
    French,
    German,
    Italian,
    Japan,
    Taiwan,
    Usa,
    Mdy,
    Dmy,
    Ymd,
    Short,
    Long,
}

/// How a format lays a date out: the order of the three fields, the character it writes
/// between them, and whether the year is the Republic of China's rather than the Christian
/// era's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateStyle {
    /// `b'y'`, `b'm'` and `b'd'` in the order they are written.
    pub order: [u8; 3],
    pub sep: char,
    pub roc_year: bool,
}

/// Every word and what it lays out. SHORT and LONG take their shape from the Windows Control
/// Panel instead and `format_date` writes them itself; the layout here is the one their blank
/// date and their reading of text still use.
const DATE_FORMATS: [(&str, DateFormat, [u8; 3], char, bool); 14] = [
    ("AMERICAN", DateFormat::American, *b"mdy", '/', false),
    ("ANSI", DateFormat::Ansi, *b"ymd", '.', false),
    ("BRITISH", DateFormat::British, *b"dmy", '/', false),
    ("FRENCH", DateFormat::French, *b"dmy", '/', false),
    ("GERMAN", DateFormat::German, *b"dmy", '.', false),
    ("ITALIAN", DateFormat::Italian, *b"dmy", '-', false),
    ("JAPAN", DateFormat::Japan, *b"ymd", '/', false),
    ("TAIWAN", DateFormat::Taiwan, *b"ymd", '/', true),
    ("USA", DateFormat::Usa, *b"mdy", '-', false),
    ("MDY", DateFormat::Mdy, *b"mdy", '/', false),
    ("DMY", DateFormat::Dmy, *b"dmy", '/', false),
    ("YMD", DateFormat::Ymd, *b"ymd", '/', false),
    ("SHORT", DateFormat::Short, *b"mdy", '/', false),
    ("LONG", DateFormat::Long, *b"mdy", '/', false),
];

/// The Republic of China counts its first year as 1912, so there is no year before it to
/// count: a date older than that is written with the year it has.
fn year_shown(y: i32, style: DateStyle) -> i32 {
    if style.roc_year && y > 1911 { y - 1911 } else { y }
}

impl DateFormat {
    pub fn parse(word: &str) -> Option<DateFormat> {
        let word = word.trim().to_ascii_uppercase();
        DATE_FORMATS.iter().find(|row| row.0 == word).map(|row| row.1)
    }

    /// The word SET("DATE") answers with.
    pub fn word(self) -> &'static str {
        DATE_FORMATS.iter().find(|row| row.1 == self).map_or("AMERICAN", |row| row.0)
    }

    pub fn style(self) -> DateStyle {
        DATE_FORMATS
            .iter()
            .find(|row| row.1 == self)
            .map_or(DateStyle { order: *b"mdy", sep: '/', roc_year: false }, |row| DateStyle {
                order: row.2,
                sep: row.3,
                roc_year: row.4,
            })
    }
}

impl Value {
    pub fn str(s: impl AsRef<str>) -> Value {
        Value::Str(Rc::from(s.as_ref()))
    }

    /// A number with the width a variable would give it: ten for the whole part, and as many
    /// places past the point as the value itself needs.
    ///
    /// This is what a number the runtime worked out for itself gets - a record number, a count,
    /// the answer of a function that declares no width of its own - and it is what Visual
    /// FoxPro gives those too: `? RECNO()` is nine spaces and a 1.
    pub fn number(n: f64) -> Value {
        Value::Number(n, Width::of_variable(n, places_needed(n)))
    }

    /// The width a number carries, or the one a variable would give it.
    pub fn width(&self) -> Width {
        match self.deref() {
            Value::Number(_, w) => w,
            Value::Currency(_) => CURRENCY_WIDTH,
            other => Width::of_variable(other.as_number().unwrap_or(0.0), 0),
        }
    }

    /// Follows `Ref` cells.
    pub fn deref(&self) -> Value {
        match self {
            Value::Ref(cell) => cell.borrow().deref(),
            other => other.clone(),
        }
    }

    /// VFP `VARTYPE()`: C N L D T O A X (null) U (undefined; not produced here).
    pub fn vartype(&self) -> char {
        match self {
            Value::Null => 'X',
            Value::Logical(_) => 'L',
            Value::Number(..) => 'N',
            Value::Currency(_) => 'Y',
            Value::Str(_) => 'C',
            Value::Date(_) => 'D',
            Value::DateTime(_) => 'T',
            Value::Object(_) => 'O',
            Value::Function(_) => 'F',
            Value::Json(_) => 'J',
            Value::Array(_) => 'A',
            Value::Ref(cell) => cell.borrow().vartype(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.vartype() {
            'C' => "Character",
            'N' => "Numeric",
            'L' => "Logical",
            'D' => "Date",
            'T' => "DateTime",
            'O' => "Object",
            'F' => "Function",
            'J' => "Json",
            'A' => "Array",
            _ => "Null",
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self.deref(), Value::Null)
    }

    /// VFP `EMPTY()`: "", 0, .F., empty date/datetime, NULL, and strings of blanks/tabs/CR/LF.
    pub fn is_empty(&self) -> bool {
        match self.deref() {
            Value::Null => true,
            Value::Logical(b) => !b,
            Value::Number(n, ..) => n == 0.0,
            Value::Currency(c) => c == 0,
            Value::Str(s) => s.chars().all(|c| matches!(c, ' ' | '\t' | '\r' | '\n')),
            Value::Date(d) => d.is_none(),
            Value::DateTime(d) => d.is_none(),
            Value::Object(_) => false,
            Value::Function(_) => false,
            // an empty object, an empty array and a null are empty; a scalar is not
            Value::Json(j) => match j.as_ref() {
                serde_json::Value::Null => true,
                serde_json::Value::Object(m) => m.is_empty(),
                serde_json::Value::Array(a) => a.is_empty(),
                _ => false,
            },
            Value::Array(_) => false,
            Value::Ref(_) => unreachable!(),
        }
    }

    /// Condition of IF / DO WHILE / IIF: logical only; NULL is false.
    pub fn truthy(&self) -> Result<bool, RtError> {
        match self.deref() {
            Value::Logical(b) => Ok(b),
            Value::Null => Ok(false),
            _ => Err(RtError::data_type_mismatch()),
        }
    }

    pub fn as_number(&self) -> Result<f64, RtError> {
        match self.deref() {
            Value::Number(n, ..) => Ok(n),
            Value::Currency(c) => Ok(c as f64 / CURRENCY_SCALE as f64),
            _ => Err(RtError::type_mismatch()),
        }
    }

    pub fn as_str(&self) -> Result<Rc<str>, RtError> {
        match self.deref() {
            Value::Str(s) => Ok(s),
            _ => Err(RtError::type_mismatch()),
        }
    }

    pub fn as_object(&self) -> Result<Handle, RtError> {
        match self.deref() {
            Value::Object(h) => Ok(h),
            _ => Err(RtError::type_mismatch()),
        }
    }

    /// Integer subscript/argument conversion (truncates like VFP).
    pub fn as_usize(&self) -> Result<usize, RtError> {
        let n = self.as_number()?;
        if n < 0.0 { Err(RtError::invalid_subscript()) } else { Ok(n.trunc() as usize) }
    }
}

// ---------------------------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------------------------

fn null_or<T>(a: &Value, b: &Value, f: impl FnOnce() -> Result<T, RtError>) -> Result<Option<T>, RtError> {
    if a.is_null() || b.is_null() { Ok(None) } else { f().map(Some) }
}

/// How far past the point a Visual FoxPro number is exact.
///
/// Its numbers are IEEE doubles - `1e15 + 1 - 1e15` is 1 while `1e16 + 1 - 1e16` is 0, and
/// `2^53 + 1` is 9007199254740992, all of which only a double does. But a double's error never
/// shows: `0.1 + 0.2 = 0.3` is true, `STR(1/3, 20, 16)` is 0.3333333333333330 where the double
/// is 0.33333333333333331483, and `STR(0.1, 20, 17)` is 0.10000000000000000 where it is
/// 0.10000000000000000555.
///
/// Every one of those is what rounding to fifteen places past the point does - and it is places,
/// not significant digits, which is why 9007199254740992 and 123456789012345.6 keep every digit
/// they have. Read off vfp9.exe; `crates/foxvm/tests/programs/ref_numbers.prg` is the same
/// program run there.
pub const EXACT_PLACES: i32 = 15;

/// Currency is kept in ten-thousandths: four places, exactly, however it was worked out.
pub const CURRENCY_SCALE: i64 = 10_000;

/// Money always prints twenty-one wide with its four places, whatever it was worked out from
/// and whatever `SET DECIMALS` says.
pub const CURRENCY_WIDTH: Width = Width { chars: 21, decimals: 4, written: false };

/// The most places past the point Visual FoxPro will show. `SET DECIMALS TO 18` is as far as it
/// goes, and a division asked for more than that is held here.
const MOST_DECIMALS: u8 = 18;

/// The widths of the two sides of an arithmetic operator, and what the operator makes of them.
///
/// The two halves of every rule are the whole part and the places past the point, because that
/// is how Visual FoxPro thinks about them: `? b * c` on N(8,2) and N(5,1) fields is thirteen
/// wide with three places, which is five whole digits plus three whole digits plus one for the
/// carry, and two places plus one place. Read off vfp9.exe.
///
/// Two numbers the program wrote down are a case of their own. Visual FoxPro's compiler works
/// those out itself, and it widens a written number that has any places at all to `SET
/// DECIMALS` of them before it starts - which is why `? 1.5 * 2` is "  3.00" and `? 4 * 2`,
/// where neither side has a point, is plain "  8".
fn widened(w: Width, settings: &Settings) -> Width {
    if w.written && w.decimals > 0 && w.decimals < settings.decimals {
        return Width::of(w.whole(), settings.decimals).as_written();
    }
    w
}

/// `+` and `-` and `%`: room for one more digit than the wider side, and the places of
/// whichever side has more of them.
fn width_sum(a: Width, b: Width, settings: &Settings) -> Width {
    let both = a.written && b.written;
    let (a, b) = if both { (widened(a, settings), widened(b, settings)) } else { (a, b) };
    let decimals = a.decimals.max(b.decimals);
    let carried = a.chars.max(b.chars).saturating_add(1);
    let held = Width::of(a.whole().max(b.whole()), decimals).chars;
    Width { chars: carried.max(held).min(WIDEST), decimals, written: both }
}

/// `*`: the whole parts add, with one more for the carry, and so do the places.
fn width_product(a: Width, b: Width, answer: f64, settings: &Settings) -> Width {
    let both = a.written && b.written;
    let mut decimals = a.decimals.saturating_add(b.decimals).min(MOST_DECIMALS);
    if both && (decimals > 0 || !is_whole(answer)) {
        decimals = decimals.max(settings.decimals);
    }
    let whole = a.whole().saturating_add(b.whole()).saturating_add(1);
    Width { written: both, ..Width::of(whole, decimals) }
}

/// `/`: the answer keeps the whole part of what was divided, and gains places.
///
/// A running program gives it two places more than `SET DECIMALS` - `? n / 2` where `n = 4` is
/// "2.0000" - and room for both whole parts, since dividing by a small number makes a big one.
/// Two written numbers are the compiler's work again: it keeps the places the two sides had,
/// widened to `SET DECIMALS`, and a division of two whole numbers that comes out whole stays
/// a whole number - `? 100 / 4` is " 25", not " 25.00".
fn width_quotient(a: Width, b: Width, answer: f64, divisor: f64, settings: &Settings) -> Width {
    if a.written && b.written && divisor != 0.0 {
        if a.decimals == 0 && b.decimals == 0 && is_whole(answer) {
            return Width { chars: a.whole().max(1), decimals: 0, written: true };
        }
        let decimals = a.decimals.saturating_add(b.decimals).max(settings.decimals).min(MOST_DECIMALS);
        return Width { written: true, ..Width::of(a.whole(), decimals) };
    }
    let decimals = a.decimals.max(b.decimals).max(settings.decimals.saturating_add(2)).min(MOST_DECIMALS);
    // one whole part is enough for the answer itself; the other is the room a small divisor
    // needs, less the carry the two of them would otherwise both claim
    let carry = u8::from(a.decimals > 0 && b.decimals > 0);
    let whole = a.whole().saturating_add(b.whole()).saturating_add(carry).saturating_sub(1);
    Width::of(whole, decimals)
}

/// `^`: ten wide whatever was raised to what, with `SET DECIMALS` places unless a side had more.
fn width_power(a: Width, b: Width, settings: &Settings) -> Width {
    let decimals = a.decimals.max(b.decimals).max(settings.decimals).min(MOST_DECIMALS);
    Width::of(VARIABLE_WHOLE, decimals)
}

/// Whether a number is still a whole one a double can vouch for. Past that scale the digits are
/// no longer the number's own, so Visual FoxPro stops treating it as a whole number.
fn is_whole(n: f64) -> bool {
    n.is_finite() && n == n.trunc() && n.abs() < 9.007_199_254_740_992e15
}

/// An amount of money from a number of them.
///
/// The half-way case goes to the even ten-thousandth, which is what Visual FoxPro does:
/// `$0.0001 / 2` is $0.0000 and `$1.23455` is $1.2346.
pub fn to_currency(n: f64) -> Value {
    let scaled = n * CURRENCY_SCALE as f64;
    if !scaled.is_finite() {
        return Value::Currency(0);
    }
    Value::Currency(scaled.round_ties_even().clamp(i64::MIN as f64, i64::MAX as f64) as i64)
}

/// A number as Visual FoxPro keeps it: rounded to where a double stops being exact.
///
/// Above that scale there is nothing to round - the value has no places left to lose - so it is
/// handed back as it is rather than scaled into infinity.
/// A number written with `places` digits after the point, as Visual FoxPro writes one.
///
/// Two things the platform's own formatter does not do. Only the first fifteen digits are the
/// number's - past that a double has nothing left to say, so Visual FoxPro pads with zeros
/// rather than printing the noise, and `STR(0.1, 20, 17)` is 0.10000000000000000 there where
/// the double would give 0.10000000000000001. And a tie goes away from zero: `STR(2.5, 4, 0)`
/// is 3 and `STR(-2.5, 4, 0)` is -3, where the formatter would round both to the even digit.
pub fn to_places(n: f64, places: usize) -> String {
    let exact = places.min(EXACT_PLACES as usize);
    let scale = 10f64.powi(exact as i32);
    let scaled = n * scale;
    // Rounding by scaling is only right while the scaled number is still a whole number the
    // double can hold; past that it loses the number itself - 2^53 came out as
    // 9007199254741000 - and a number that large has no fraction left to round anyway.
    let text = if scaled.is_finite() && scaled.abs() < 9.007_199_254_740_992e15 {
        format!("{:.*}", exact, scaled.round() / scale)
    } else {
        format!("{n:.exact$}")
    };
    if places <= exact { text } else { text + &"0".repeat(places - exact) }
}

pub fn settle(n: f64) -> f64 {
    if !n.is_finite() {
        return n;
    }
    // written out to fifteen places and read back: the rounding is decimal, so it cannot cost
    // the number its own magnitude the way scaling by 1e15 would
    format!("{n:.*}", EXACT_PLACES as usize).parse().unwrap_or(n)
}

/// Whether an arithmetic result is money.
///
/// Money is contagious through `+`, `-`, `*`, `/` and `%`: `$1 + 1.5` is $2.50 and even
/// `$1 / $2` is money. It is not contagious through `^`, which VFP answers as a Number -
/// raising an amount to a power leaves the units behind, and so do SQRT, LOG, EXP and the rest.
fn either_is_money(a: &Value, b: &Value) -> bool {
    matches!(a.deref(), Value::Currency(_)) || matches!(b.deref(), Value::Currency(_))
}

/// The two amounts of an exact money operation, in ten-thousandths.
fn money_pair(a: &Value, b: &Value) -> Result<(i128, i128), RtError> {
    let scale = |v: &Value| -> Result<i128, RtError> {
        match v.deref() {
            Value::Currency(c) => Ok(c as i128),
            other => match to_currency(other.as_number()?) {
                Value::Currency(c) => Ok(c as i128),
                _ => Err(RtError::type_mismatch()),
            },
        }
    };
    Ok((scale(a)?, scale(b)?))
}

/// A whole number of ten-thousandths back as money, saturating rather than wrapping.
fn money(amount: i128) -> Value {
    Value::Currency(amount.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
}

/// One money division or multiplication, rounded to the even ten-thousandth as VFP rounds it.
fn money_div(top: i128, bottom: i128) -> i128 {
    if bottom == 0 {
        return 0;
    }
    let (q, r) = (top / bottom, top % bottom);
    let twice = (r * 2).abs();
    let magnitude = bottom.abs();
    let away = if (top < 0) != (bottom < 0) { -1 } else { 1 };
    // half goes to the even quotient: `$0.0001 / 2` is $0.0000, not $0.0001
    if twice > magnitude || (twice == magnitude && q % 2 != 0) { q + away } else { q }
}

/// `+`: numbers, string concatenation, date/datetime plus days/seconds. NULL propagates.
pub fn add(a: &Value, b: &Value, settings: &Settings) -> Result<Value, RtError> {
    let (a, b) = (a.deref(), b.deref());
    if a.is_null() || b.is_null() {
        return Ok(Value::Null);
    }
    Ok(match (&a, &b) {
        (Value::Number(x, wx), Value::Number(y, wy)) => Value::Number(x + y, width_sum(*wx, *wy, settings)),
        // money is added in ten-thousandths, which is why $0.1 + $0.2 is exactly $0.3
        _ if either_is_money(&a, &b) => {
            let (x, y) = money_pair(&a, &b)?;
            money(x + y)
        }
        (Value::Str(x), Value::Str(y)) => Value::Str(Rc::from(format!("{x}{y}"))),
        (Value::Date(Some(d)), Value::Number(n, _)) | (Value::Number(n, _), Value::Date(Some(d))) => {
            Value::Date(Some(d + n.trunc() as i32))
        }
        (Value::Date(None), Value::Number(..)) | (Value::Number(..), Value::Date(None)) => Value::Date(None),
        (Value::DateTime(Some(t)), Value::Number(n, _)) | (Value::Number(n, _), Value::DateTime(Some(t))) => {
            Value::DateTime(Some(t + n))
        }
        (Value::DateTime(None), Value::Number(..)) | (Value::Number(..), Value::DateTime(None)) => {
            Value::DateTime(None)
        }
        _ => return Err(RtError::type_mismatch()),
    })
}

/// `-`: numbers, VFP string subtraction (trailing blanks of the left operand move to the end),
/// date differences in days, datetime differences in seconds.
///
/// The width is the one `+` gives, except that between two numbers the program wrote down the
/// right-hand side is negated first, and a written minus sign takes a character: `? 100 - 4` is
/// four wide like `? 100 + 4`, but `? 4 - 2` is three wide where `? 4 + 2` is two.
pub fn sub(a: &Value, b: &Value, settings: &Settings) -> Result<Value, RtError> {
    let (a, b) = (a.deref(), b.deref());
    if a.is_null() || b.is_null() {
        return Ok(Value::Null);
    }
    Ok(match (&a, &b) {
        (Value::Number(x, wx), Value::Number(y, wy)) => {
            let right = if wx.written && wy.written { negated(widened(*wy, settings)) } else { *wy };
            Value::Number(x - y, width_sum(*wx, right, settings))
        }
        _ if either_is_money(&a, &b) => {
            let (x, y) = money_pair(&a, &b)?;
            money(x - y)
        }
        (Value::Str(x), Value::Str(y)) => {
            let trimmed = x.trim_end_matches(' ');
            let blanks = x.len() - trimmed.len();
            Value::Str(Rc::from(format!("{trimmed}{y}{}", " ".repeat(blanks))))
        }
        (Value::Date(Some(d)), Value::Number(n, _)) => Value::Date(Some(d - n.trunc() as i32)),
        (Value::Date(None), Value::Number(..)) => Value::Date(None),
        (Value::Date(Some(x)), Value::Date(Some(y))) => Value::number((x - y) as f64),
        (Value::Date(_), Value::Date(_)) => Value::number(0.0),
        (Value::DateTime(Some(t)), Value::Number(n, _)) => Value::DateTime(Some(t - n)),
        (Value::DateTime(None), Value::Number(..)) => Value::DateTime(None),
        (Value::DateTime(Some(x)), Value::DateTime(Some(y))) => Value::number(x - y),
        (Value::DateTime(_), Value::DateTime(_)) => Value::number(0.0),
        _ => return Err(RtError::type_mismatch()),
    })
}

pub fn mul(a: &Value, b: &Value, settings: &Settings) -> Result<Value, RtError> {
    if either_is_money(a, b) && !a.is_null() && !b.is_null() {
        return money_mul_div(a, b, false);
    }
    let (wa, wb) = (a.width(), b.width());
    Ok(null_or(a, b, || Ok(a.as_number()? * b.as_number()?))?
        .map_or(Value::Null, |n| Value::Number(n, width_product(wa, wb, n, settings))))
}

/// `/`. Dividing by zero is not an error in Visual FoxPro: the answer is a number too big to
/// print, and `?` writes a row of asterisks where it would have gone.
pub fn div(a: &Value, b: &Value, settings: &Settings) -> Result<Value, RtError> {
    if either_is_money(a, b) && !a.is_null() && !b.is_null() {
        return money_mul_div(a, b, true);
    }
    let (wa, wb) = (a.width(), b.width());
    Ok(null_or(a, b, || Ok((a.as_number()?, b.as_number()?)))?.map_or(Value::Null, |(x, y)| {
        Value::Number(x / y, width_quotient(wa, wb, x / y, y, settings))
    }))
}

/// The money form of `*` and `/`, kept beside them so the rule reads in one place.
fn money_mul_div(a: &Value, b: &Value, divide: bool) -> Result<Value, RtError> {
    let (x, y) = money_pair(a, b)?;
    if divide {
        if y == 0 {
            return Err(RtError::division_by_zero());
        }
        return Ok(money(money_div(x * CURRENCY_SCALE as i128, y)));
    }
    Ok(money(money_div(x * y, CURRENCY_SCALE as i128)))
}

/// `%` and `MOD()`: result takes the sign of the divisor, like VFP. Unlike `/`, a zero divisor
/// here is error 1307 - Visual FoxPro raises it for the remainder and not for the quotient.
pub fn modulo(a: &Value, b: &Value, settings: &Settings) -> Result<Value, RtError> {
    if either_is_money(a, b) && !a.is_null() && !b.is_null() {
        let (x, y) = money_pair(a, b)?;
        if y == 0 {
            return Err(RtError::division_by_zero());
        }
        // the same flooring remainder as the numeric one, in whole ten-thousandths
        return Ok(money(x - y * (x.div_euclid(y) - i128::from((x % y != 0) && ((x < 0) != (y < 0))))));
    }
    let (wa, wb) = (a.width(), b.width());
    Ok(null_or(a, b, || {
        let (x, y) = (a.as_number()?, b.as_number()?);
        if y == 0.0 { Err(RtError::division_by_zero()) } else { Ok(x - y * (x / y).floor()) }
    })?
    .map_or(Value::Null, |n| Value::Number(n, width_sum(wa, wb, settings))))
}

pub fn pow(a: &Value, b: &Value, settings: &Settings) -> Result<Value, RtError> {
    let (wa, wb) = (a.width(), b.width());
    Ok(null_or(a, b, || Ok(a.as_number()?.powf(b.as_number()?)))?
        .map_or(Value::Null, |n| Value::Number(n, width_power(wa, wb, settings))))
}

/// Unary `-`. The minus sign needs a character of its own, so the number gets one wider.
pub fn neg(a: &Value) -> Result<Value, RtError> {
    match a.deref() {
        Value::Null => Ok(Value::Null),
        Value::Currency(c) => Ok(Value::Currency(-c)),
        Value::Number(n, w) => Ok(Value::Number(-n, negated(w))),
        v => Ok(Value::number(-v.as_number()?)),
    }
}

/// The same number with room for a minus sign in front of it.
fn negated(w: Width) -> Width {
    Width { chars: w.chars.saturating_add(1).min(WIDEST), ..w }
}

pub fn not(a: &Value) -> Result<Value, RtError> {
    match a.deref() {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Logical(!b)),
        _ => Err(RtError::type_mismatch()),
    }
}

/// Three-valued AND (used after the short-circuit jump).
pub fn and(a: &Value, b: &Value) -> Result<Value, RtError> {
    match (a.deref(), b.deref()) {
        (Value::Logical(false), _) | (_, Value::Logical(false)) => Ok(Value::Logical(false)),
        (Value::Logical(true), Value::Logical(true)) => Ok(Value::Logical(true)),
        (Value::Null, Value::Logical(_) | Value::Null) | (Value::Logical(_), Value::Null) => Ok(Value::Null),
        _ => Err(RtError::type_mismatch()),
    }
}

/// Three-valued OR.
pub fn or(a: &Value, b: &Value) -> Result<Value, RtError> {
    match (a.deref(), b.deref()) {
        (Value::Logical(true), _) | (_, Value::Logical(true)) => Ok(Value::Logical(true)),
        (Value::Logical(false), Value::Logical(false)) => Ok(Value::Logical(false)),
        (Value::Null, Value::Logical(_) | Value::Null) | (Value::Logical(_), Value::Null) => Ok(Value::Null),
        _ => Err(RtError::type_mismatch()),
    }
}

/// `$`: substring containment (case-sensitive).
/// `a $ b`: whether the left string turns up inside the right one, case and all.
///
/// Nothing is never found. Visual FoxPro answers `.F.` to `"" $ "abc"`, and `AT("", "abc")`
/// agrees with it at 0, where Rust's own `contains` would say the empty string is everywhere.
pub fn contains(a: &Value, b: &Value) -> Result<Value, RtError> {
    let seek = || {
        let needle = a.as_str()?;
        Ok(!needle.is_empty() && b.as_str()?.contains(&*needle))
    };
    Ok(null_or(a, b, seek)?.map_or(Value::Null, Value::Logical))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    ExactEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// VFP string equality for `=`: with EXACT OFF the comparison stops at the end of the right
/// operand (`"abc" = "ab"` is true, `"ab" = "abc"` is false); with EXACT ON trailing blanks
/// are ignored on both sides. `==` requires an exact match.
fn str_eq(a: &str, b: &str, op: CmpOp, exact: bool) -> bool {
    match op {
        CmpOp::ExactEq => a == b,
        _ if exact => a.trim_end_matches(' ') == b.trim_end_matches(' '),
        _ => a.starts_with(b),
    }
}

/// VFP string ordering for `<`, `<=`, `>` and `>=`, which follows SET EXACT the same way `=`
/// does rather than comparing the two as they stand.
///
/// Measured in Visual FoxPro 9. With EXACT OFF the comparison stops at the end of the right
/// operand, exactly as equality does: `"abc" > "ab"` is false because the first two characters
/// settle it, and `"A  " > "A"` is false for the same reason, while `"A" < "A  "` is true
/// because the left runs out with the right still going. With EXACT ON the shorter of the two
/// is padded with blanks first, so `"ab " > "ab"` is false and `"abc" > "ab"` is true.
fn str_order(a: &str, b: &str, exact: bool) -> Ordering {
    if exact {
        let blanks = std::iter::repeat(' ');
        let width = a.chars().count().max(b.chars().count());
        return a.chars().chain(blanks.clone()).take(width).cmp(b.chars().chain(blanks).take(width));
    }
    a.chars().take(b.chars().count()).cmp(b.chars())
}

/// Comparison; returns `Value::Null` when either side is NULL.
pub fn compare(a: &Value, b: &Value, op: CmpOp, settings: &Settings) -> Result<Value, RtError> {
    let (a, b) = (a.deref(), b.deref());
    if a.is_null() || b.is_null() {
        return Ok(Value::Null);
    }
    let ord = match (&a, &b) {
        (Value::Str(x), Value::Str(y)) => {
            if matches!(op, CmpOp::Eq | CmpOp::ExactEq | CmpOp::Ne) {
                let eq = str_eq(x, y, op, settings.exact);
                return Ok(Value::Logical(if op == CmpOp::Ne { !eq } else { eq }));
            }
            str_order(x, y, settings.exact)
        }
        // two numbers are compared as Visual FoxPro keeps them, which is rounded to where a
        // double stops being exact: 0.1 + 0.2 = 0.3 is true, and so is 1 / 7 * 7 = 1
        (Value::Number(x, ..), Value::Number(y, ..)) => settle(*x).partial_cmp(&settle(*y)).unwrap_or(Ordering::Equal),
        // money against money is exact, being whole ten-thousandths on both sides; against a
        // number it is the amount that is compared, rounded the way any two numbers are
        (Value::Currency(x), Value::Currency(y)) => x.cmp(y),
        (Value::Currency(_), Value::Number(..)) | (Value::Number(..), Value::Currency(_)) => {
            settle(a.as_number()?).partial_cmp(&settle(b.as_number()?)).unwrap_or(Ordering::Equal)
        }
        (Value::Logical(x), Value::Logical(y)) => x.cmp(y),
        (Value::Date(x), Value::Date(y)) => x.unwrap_or(i32::MIN).cmp(&y.unwrap_or(i32::MIN)),
        (Value::DateTime(x), Value::DateTime(y)) => {
            x.unwrap_or(f64::MIN).partial_cmp(&y.unwrap_or(f64::MIN)).unwrap_or(Ordering::Equal)
        }
        (Value::Date(x), Value::DateTime(y)) => {
            let xs = x.map(|d| d as f64 * 86400.0).unwrap_or(f64::MIN);
            xs.partial_cmp(&y.unwrap_or(f64::MIN)).unwrap_or(Ordering::Equal)
        }
        (Value::DateTime(x), Value::Date(y)) => {
            let ys = y.map(|d| d as f64 * 86400.0).unwrap_or(f64::MIN);
            x.unwrap_or(f64::MIN).partial_cmp(&ys).unwrap_or(Ordering::Equal)
        }
        (Value::Object(x), Value::Object(y)) if matches!(op, CmpOp::Eq | CmpOp::ExactEq | CmpOp::Ne) => {
            let eq = x == y;
            return Ok(Value::Logical(if op == CmpOp::Ne { !eq } else { eq }));
        }
        // two function values are the same function or they are not, which is the rule objects
        // already follow; the other comparisons have no meaning between them
        (Value::Function(x), Value::Function(y)) if matches!(op, CmpOp::Eq | CmpOp::ExactEq | CmpOp::Ne) => {
            let eq = x == y;
            return Ok(Value::Logical(if op == CmpOp::Ne { !eq } else { eq }));
        }
        // a JSON value has no methods and no identity worth keeping, so unlike an object or a
        // lambda it is compared by what is in it
        (Value::Json(x), Value::Json(y)) if matches!(op, CmpOp::Eq | CmpOp::ExactEq | CmpOp::Ne) => {
            let eq = x == y;
            return Ok(Value::Logical(if op == CmpOp::Ne { !eq } else { eq }));
        }
        _ => return Err(RtError::type_mismatch()),
    };
    Ok(Value::Logical(match op {
        CmpOp::Eq | CmpOp::ExactEq => ord == Ordering::Equal,
        CmpOp::Ne => ord != Ordering::Equal,
        CmpOp::Lt => ord == Ordering::Less,
        CmpOp::Le => ord != Ordering::Greater,
        CmpOp::Gt => ord == Ordering::Greater,
        CmpOp::Ge => ord != Ordering::Less,
    }))
}

// ---------------------------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------------------------

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i32 - 719468
}

/// (year, month, day) from days since 1970-01-01.
pub fn civil_from_days(z: i32) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn is_valid_date(y: i32, m: u32, d: u32) -> bool {
    if !(1..=12).contains(&m) || d == 0 {
        return false;
    }
    let days_in_month = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    };
    d <= days_in_month
}

/// 0 = Sunday ... 6 = Saturday.
pub fn day_of_week(days: i32) -> u32 {
    ((days + 4).rem_euclid(7)) as u32
}

pub const DAY_NAMES: [&str; 7] =
    ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The date as `?`, DTOC() and every other display path writes it. `SET DATE` says which
/// fields go where and what stands between them, `SET MARK` replaces that character, and
/// `SET CENTURY` says whether the year is written in full. SHORT and LONG are the two
/// Windows formats and ignore all three, as the reference says they do.
pub fn format_date(days: Option<i32>, settings: &Settings) -> String {
    let style = settings.date_format.style();
    match (settings.date_format, days) {
        (DateFormat::Short, Some(days)) => {
            let (y, m, d) = civil_from_days(days);
            format!("{m}/{d}/{y:04}")
        }
        (DateFormat::Long, Some(days)) => {
            let (y, m, d) = civil_from_days(days);
            let day = DAY_NAMES[day_of_week(days) as usize];
            format!("{day}, {} {d}, {y:04}", MONTH_NAMES[(m - 1) as usize])
        }
        _ => {
            let sep = if matches!(settings.date_format, DateFormat::Short | DateFormat::Long) {
                style.sep
            } else {
                settings.mark.unwrap_or(style.sep)
            };
            let (y, m, d) = days.map_or((0, 0, 0), civil_from_days);
            let year_width = if settings.century { 4 } else { 2 };
            let fields: Vec<String> = style
                .order
                .iter()
                .map(|slot| match (slot, days) {
                    (b'y', None) => " ".repeat(year_width),
                    (_, None) => "  ".to_string(),
                    (b'y', Some(_)) => {
                        let y = year_shown(y, style);
                        if settings.century { format!("{y:04}") } else { format!("{:02}", y.rem_euclid(100)) }
                    }
                    (b'm', Some(_)) => format!("{m:02}"),
                    (_, Some(_)) => format!("{d:02}"),
                })
                .collect();
            fields.join(&sep.to_string())
        }
    }
}

/// The time of day on its own, which is what TTOC(t, 2) answers with. `SET HOURS` chooses the
/// clock and `SET SECONDS` whether the seconds are written; `pad` is false only for the two
/// Windows formats, which write a bare hour.
pub fn format_time(secs: f64, settings: &Settings, pad: bool) -> String {
    let rem = secs.rem_euclid(86400.0).floor() as u32;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (shown, suffix) = if settings.hours24 {
        (h, String::new())
    } else {
        let (h12, ampm) = match h {
            0 => (12, "AM"),
            1..=11 => (h, "AM"),
            12 => (12, "PM"),
            _ => (h - 12, "PM"),
        };
        (h12, format!(" {ampm}"))
    };
    let hour = if pad { format!("{shown:02}") } else { shown.to_string() };
    if settings.seconds { format!("{hour}:{mi:02}:{s:02}{suffix}") } else { format!("{hour}:{mi:02}{suffix}") }
}

pub fn format_datetime(secs: Option<f64>, settings: &Settings) -> String {
    let Some(secs) = secs else {
        // an empty datetime keeps the width of a full one: blanks where the digits would be,
        // seconds and all, and the AM of a 12-hour clock
        let windows = matches!(settings.date_format, DateFormat::Short | DateFormat::Long);
        let clock = if settings.hours24 && !windows { "  :  :  " } else { "  :  :   AM" };
        return format!("{} {clock}", format_date(None, settings));
    };
    let days = secs.div_euclid(86400.0) as i32;
    let date = format_date(Some(days), settings);
    let time_of_day = secs.rem_euclid(86400.0);
    match settings.date_format {
        // the Windows formats have a clock of their own: 12 hours with seconds, whatever
        // SET HOURS and SET SECONDS say, and LONG puts a comma before it
        DateFormat::Short | DateFormat::Long => {
            let windows = Settings { hours24: false, seconds: true, ..settings.clone() };
            let time = format_time(time_of_day, &windows, false);
            let join = if settings.date_format == DateFormat::Long { "," } else { "" };
            format!("{date}{join} {time}")
        }
        _ => format!("{date} {}", format_time(time_of_day, settings, true)),
    }
}

// ---------------------------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------------------------

/// Number text with a decimal point of `.`: whole numbers plain and everything else to
/// `decimals` places, unless `fixed` says every number gets those places whether it needs them
/// or not. (VFP keeps a literal's own precision through an expression; this runtime does not,
/// so a value that is not whole always reads at `SET DECIMALS`.)
pub fn number_text(n: f64, decimals: u8, fixed: bool) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if !fixed && n == n.trunc() && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    // half goes away from zero, as VFP rounds, not to the nearest even that Rust would write
    let s = format!("{:.*}", decimals as usize, crate::builtins::numeric::round_to(n, decimals as i32));
    // a value that rounds away to nothing is not negative any more
    match s.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_string(),
        _ => s,
    }
}

/// The same under `SET DECIMALS`, `SET FIXED` and `SET POINT`: what `?` and `TRANSFORM()`
/// show for a number with no picture.
pub fn format_number(n: f64, settings: &Settings) -> String {
    with_point(number_text(n, settings.decimals, settings.fixed), settings.point)
}

/// Puts `SET POINT`'s character where the decimal point is. Every number that reaches a
/// display goes through here, so the setting is honoured once rather than at each caller.
pub fn with_point(text: String, point: char) -> String {
    if point == '.' { text } else { text.replace('.', &point.to_string()) }
}

/// Money as Visual FoxPro writes it: the symbol on the side `SET CURRENCY` puts it, the
/// digits grouped by `SET SEPARATOR` and pointed by `SET POINT`, to `SET DECIMALS` places.
///
/// The minus sign goes between the symbol and the digits - `$-1,234.50` - which is where
/// Visual FoxPro puts it rather than in front of the symbol.
pub fn format_currency(amount: i64, settings: &Settings) -> String {
    let places = settings.decimals as usize;
    let magnitude = (amount as f64 / CURRENCY_SCALE as f64).abs();
    let digits = grouped(&to_places(magnitude, places), settings);
    let sign = if amount < 0 { "-" } else { "" };
    if settings.currency_left {
        format!("{}{sign}{digits}", settings.currency)
    } else {
        format!("{sign}{digits}{}", settings.currency)
    }
}

/// The digits of a number with a separator every three, and the point the settings ask for.
fn grouped(text: &str, settings: &Settings) -> String {
    let (whole, rest) = match text.split_once('.') {
        Some((w, r)) => (w, Some(r)),
        None => (text, None),
    };
    let mut out = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push(settings.separator);
        }
        out.push(ch);
    }
    match rest {
        Some(r) => {
            out.push(settings.point);
            out.push_str(r);
            out
        }
        None => out,
    }
}

/// What `?` and `??` write for one item.
///
/// A number goes in the width it carries, right-aligned, so a column of them lines up: `? 8`
/// is "8" and `? 4 * 2` is "  8". Money is always twenty-one wide with its four places and no
/// currency symbol, which is what Visual FoxPro writes for `? $12.34` - the symbol belongs to
/// `TRANSFORM()`, not to `?`. Everything else is written as it reads.
pub fn printed(v: &Value, settings: &Settings) -> String {
    let (n, width) = match v.deref() {
        Value::Number(n, w) => (n, w),
        Value::Currency(c) => (c as f64 / CURRENCY_SCALE as f64, CURRENCY_WIDTH),
        other => return display(&other, settings),
    };
    // SET FIXED ON gives every number `SET DECIMALS` places, and room for them
    let width = if settings.fixed { Width::of(width.whole(), settings.decimals) } else { width };
    let text = with_point(to_places(n, width.decimals as usize), settings.point);
    let text = match text.strip_prefix('-') {
        // a value that rounds away to nothing is not negative any more
        Some(rest) if rest.chars().all(|c| c == '0' || c == settings.point) => rest.to_string(),
        _ => text,
    };
    let room = width.chars as usize;
    // A number that rounds away to nothing is not nothing: the places on offer cannot show
    // 7.5766328932124E-305 at all, and the product writes it with an exponent rather than as a
    // zero. Only a value that really is zero prints as one.
    let vanished = n != 0.0 && text.chars().all(|c| c == '0' || c == settings.point || c == '-');
    if n.is_finite() && !vanished && text.chars().count() <= room {
        return format!("{text:>room$}");
    }
    // too many digits for the room it has: Visual FoxPro falls back to an exponent, and when
    // even that will not fit - or the number is not a number at all - it fills the field with
    // asterisks rather than print something misleading
    match scientific(n, width.chars, settings) {
        Some(short) => format!("{short:>room$}"),
        None => "*".repeat(room),
    }
}

/// A number written with an exponent, filling all but one character of the room it has.
///
/// `? 10 ^ 20` is " 1.000000E+20" - thirteen wide, because that is what `^` gives, with the
/// places chosen so the text reaches the character before the end.
fn scientific(n: f64, chars: u8, settings: &Settings) -> Option<String> {
    if !n.is_finite() || n == 0.0 {
        return None;
    }
    let exponent = n.abs().log10().floor() as i32;
    let digits = exponent.abs().to_string().len();
    // The width holds the leading digit, the point, `E`, the exponent's sign and its digits. A
    // minus sign takes the place of the leading space rather than a column of its own, which is
    // why `-2.1307E+9` fills ten characters where `1.000000E+20` fills thirteen.
    let places = (chars as i32) - 5 - digits as i32;
    if places < 0 {
        return None;
    }
    // the digits that fit are the ones that are there: -2147483647 written in ten characters is
    // -2.1474E+9 and not the -2.1475E+9 rounding would give - measured
    let scale = 10f64.powi(places);
    let mantissa = (n / 10f64.powi(exponent) * scale).trunc() / scale;
    let sign = if exponent < 0 { '-' } else { '+' };
    // the exponent is written in as many digits as it has, so it is E+9 rather than E+09
    let text = format!("{}E{sign}{}", to_places(mantissa, places as usize), exponent.abs());
    Some(with_point(text, settings.point))
}

/// Text shown by `TRANSFORM(x)` with no format, and by everything that writes a value out
/// without a field to put it in.
pub fn display(v: &Value, settings: &Settings) -> String {
    match v.deref() {
        Value::Null => settings.null_display.clone(),
        Value::Logical(true) => ".T.".into(),
        Value::Logical(false) => ".F.".into(),
        Value::Number(n, ..) => format_number(n, settings),
        Value::Currency(c) => format_currency(c, settings),
        Value::Str(s) => s.to_string(),
        Value::Date(d) => format_date(d, settings),
        Value::DateTime(t) => format_datetime(t, settings),
        Value::Object(h) => format!("(Object {})", h.0),
        Value::Function(f) => format!("(Function {})", f.0),
        Value::Json(j) => j.to_string(),
        Value::Array(_) => "(Array)".into(),
        Value::Ref(_) => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Value {
        Value::str(x)
    }

    #[test]
    fn string_equality_follows_set_exact() {
        let off = Settings::default();
        let on = Settings { exact: true, ..Settings::default() };
        assert_eq!(compare(&s("abc"), &s("ab"), CmpOp::Eq, &off).unwrap(), Value::Logical(true));
        assert_eq!(compare(&s("ab"), &s("abc"), CmpOp::Eq, &off).unwrap(), Value::Logical(false));
        assert_eq!(compare(&s("abc"), &s("ab"), CmpOp::Eq, &on).unwrap(), Value::Logical(false));
        assert_eq!(compare(&s("abc  "), &s("abc"), CmpOp::Eq, &on).unwrap(), Value::Logical(true));
        assert_eq!(compare(&s("abc  "), &s("abc"), CmpOp::ExactEq, &on).unwrap(), Value::Logical(false));
        assert_eq!(compare(&s("abc"), &s("abc"), CmpOp::ExactEq, &off).unwrap(), Value::Logical(true));
    }

    #[test]
    fn arithmetic_rules() {
        let st = Settings::default();
        assert_eq!(add(&s("a"), &s("b"), &st).unwrap(), s("ab"));
        assert_eq!(sub(&s("ab  "), &s("cd"), &st).unwrap(), s("abcd  "));
        assert_eq!(add(&Value::number(1.0), &Value::Null, &st).unwrap(), Value::Null);
        assert!(add(&Value::number(1.0), &s("x"), &st).is_err());
        assert_eq!(modulo(&Value::number(-7.0), &Value::number(3.0), &st).unwrap(), Value::number(2.0));
        assert_eq!(modulo(&Value::number(7.0), &Value::number(-3.0), &st).unwrap(), Value::number(-2.0));
        // dividing by zero is not an error in Visual FoxPro: the answer is a number `?` cannot
        // print, and it prints asterisks instead. `%` by zero still is one.
        assert!(div(&Value::number(1.0), &Value::number(0.0), &st).unwrap().as_number().unwrap().is_infinite());
        assert_eq!(
            modulo(&Value::number(1.0), &Value::number(0.0), &st).unwrap_err().code,
            RtError::DIVISION_BY_ZERO
        );
        let d = Value::Date(Some(days_from_civil(2024, 1, 31)));
        assert_eq!(add(&d, &Value::number(1.0), &st).unwrap(), Value::Date(Some(days_from_civil(2024, 2, 1))));
        assert_eq!(sub(&Value::Date(Some(10)), &Value::Date(Some(3)), &st).unwrap(), Value::number(7.0));
    }

    #[test]
    fn three_valued_logic() {
        let (t, f, n) = (Value::Logical(true), Value::Logical(false), Value::Null);
        assert_eq!(and(&f, &n).unwrap(), f);
        assert_eq!(and(&n, &t).unwrap(), n);
        assert_eq!(or(&t, &n).unwrap(), t);
        assert_eq!(or(&n, &f).unwrap(), n);
        assert_eq!(not(&n).unwrap(), n);
    }

    #[test]
    fn dates_round_trip() {
        for (y, m, d) in [(1970, 1, 1), (2000, 2, 29), (2024, 12, 31), (1899, 12, 30), (2100, 3, 1)] {
            assert_eq!(civil_from_days(days_from_civil(y, m, d)), (y, m, d));
        }
        assert_eq!(day_of_week(days_from_civil(2026, 9, 7)), 1); // Monday
        assert!(!is_valid_date(2023, 2, 29));
        assert!(is_valid_date(2024, 2, 29));
        let st = Settings::default();
        assert_eq!(format_date(Some(days_from_civil(2026, 9, 7)), &st), "09/07/26");
        let century = Settings { century: true, date_format: DateFormat::Ansi, ..Settings::default() };
        assert_eq!(format_date(Some(days_from_civil(2026, 9, 7)), &century), "2026.09.07");
        assert_eq!(
            format_datetime(Some(days_from_civil(2026, 9, 7) as f64 * 86400.0 + 13.0 * 3600.0 + 5.0), &st),
            "09/07/26 01:00:05 PM"
        );
    }

    #[test]
    fn display_rules() {
        let st = Settings::default();
        assert_eq!(display(&Value::number(2.0), &st), "2");
        assert_eq!(display(&Value::number(1.0 / 3.0), &st), "0.33");
        assert_eq!(display(&Value::number(-0.001), &st), "0.00");
        assert_eq!(display(&Value::number(-1234.5), &st), "-1234.50");
        assert_eq!(display(&Value::Logical(true), &st), ".T.");
        assert_eq!(display(&Value::Null, &st), ".NULL.");
        assert!(s("  \t").is_empty());
        assert!(!s("a").is_empty());
        assert!(Value::number(0.0).is_empty());
    }
    /// The width a number carries, and what `?` does with it. Every number here was read off
    /// vfp9.exe; `crates/foxvm/tests/programs/ref_numwidth.prg` runs the same cases end to end.
    #[test]
    fn a_number_carries_the_width_it_prints_in() {
        let st = Settings::default();
        let written = |text: &str, n: f64| Value::Number(n, Width::written(text));
        // a number the program wrote down keeps the characters it was written with
        assert_eq!(printed(&written("8", 8.0), &st), "8");
        assert_eq!(printed(&written("001", 1.0), &st), "  1");
        assert_eq!(printed(&written(".5", 0.5), &st), "0.5");
        assert_eq!(printed(&written("1.50", 1.5), &st), "1.50");
        // a variable's whole part is ten wide, and it keeps the places it was given
        assert_eq!(printed(&Value::number(4.0), &st), "         4");
        assert_eq!(printed(&held_in_variable(written("1.5", 1.5)), &st), "         1.5");
        assert_eq!(printed(&held_in_variable(written("1.50", 1.5)), &st), "         1.50");
        // a field prints in the width it was declared with
        let price = Value::Number(3200.0, Width::field('N', 8, 2, 3200.0));
        assert_eq!(printed(&price, &st), " 3200.00");
        // money is twenty-one wide with its four places, and `?` shows no currency symbol
        assert_eq!(printed(&Value::Currency(123_400), &st), "              12.3400");
    }

    #[test]
    fn arithmetic_works_out_how_wide_the_answer_is() {
        let st = Settings::default();
        let written = |text: &str, n: f64| Value::Number(n, Width::written(text));
        let n = Value::number(4.0);
        // `+` takes one more than the wider side, `*` adds the whole parts and one for the carry
        assert_eq!(printed(&add(&n, &written("1", 1.0), &st).unwrap(), &st), "          5");
        assert_eq!(printed(&mul(&n, &written("2", 2.0), &st).unwrap(), &st), "           8");
        // `/` gains two places on SET DECIMALS, and room for both whole parts
        assert_eq!(printed(&div(&n, &written("2", 2.0), &st).unwrap(), &st), "         2.0000");
        // two numbers the program wrote down are the compiler's work, and follow its rules
        let four_times_two = mul(&written("4", 4.0), &written("2", 2.0), &st).unwrap();
        assert_eq!(printed(&four_times_two, &st), "  8");
        let half_times_two = mul(&written("1.5", 1.5), &written("2", 2.0), &st).unwrap();
        assert_eq!(printed(&half_times_two, &st), "  3.00");
        // a whole number divided evenly by a whole number stays whole
        assert_eq!(printed(&div(&written("100", 100.0), &written("4", 4.0), &st).unwrap(), &st), " 25");
        assert_eq!(printed(&div(&written("1", 1.0), &written("3", 3.0), &st).unwrap(), &st), "0.33");
        // a minus sign takes a character of its own
        assert_eq!(printed(&neg(&written("7", 7.0)).unwrap(), &st), "-7");
    }

    #[test]
    fn a_number_too_wide_for_its_room_says_so() {
        let st = Settings::default();
        // dividing by zero is not an error: the answer is a number `?` cannot write
        let one = Value::Number(1.0, Width::written("1"));
        let over = div(&one, &Value::Number(0.0, Width::written("0")), &st).unwrap();
        assert_eq!(printed(&over, &st), "******");
        // a number that will not fit falls back to an exponent, filling the room it has
        let big = Value::Number(1e20, Width::of(VARIABLE_WHOLE, 2));
        assert_eq!(printed(&big, &st), " 1.000000E+20");
    }

    #[test]
    fn set_fixed_gives_every_number_the_same_places() {
        let fixed = Settings { fixed: true, ..Settings::default() };
        assert_eq!(printed(&Value::Number(8.0, Width::written("8")), &fixed), "8.00");
        assert_eq!(printed(&Value::number(4.0), &fixed), "         4.00");
    }
    #[test]
    fn arrays() {
        let mut a = FoxArray::new(2, 3);
        assert_eq!(a.len(), 6);
        a.set(&[2, 1], Value::number(9.0)).unwrap();
        assert_eq!(a.get(&[4]).unwrap(), Value::number(9.0));
        assert!(a.get(&[3, 1]).is_err());
        a.redim(1, 4);
        assert_eq!(a.get(&[4]).unwrap(), Value::number(9.0));
    }
}
