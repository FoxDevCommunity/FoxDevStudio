//! The character screen Visual FoxPro still draws on, and the windows a program opens on it.
//!
//! `@ 2, 5 SAY "Name"` puts text at a row and a column of whatever surface output is going to -
//! the main screen, or the window `ACTIVATE WINDOW` made current. That surface is a grid of
//! characters here, and the host is handed the grid to draw; nothing in this file knows what a
//! character looks like.
//!
//! A window is a grid with a title and a place on the screen. `DEFINE WINDOW` makes one,
//! `ACTIVATE WINDOW` shows it and sends output to it, `DEACTIVATE` puts output back where it
//! was, and `RELEASE` takes it away. The `W...()` functions all read this table.

use serde::{Deserialize, Serialize};

use crate::value::Value;

/// How big the main screen is, in characters. Visual FoxPro works this out from the window and
/// the font; there is one size here, and `SROWS()` and `SCOLS()` report it.
pub const SCREEN_ROWS: usize = 25;
pub const SCREEN_COLS: usize = 80;

/// A surface of characters, addressed by row and column from zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub rows: usize,
    pub cols: usize,
    cells: Vec<char>,
}

impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        let (rows, cols) = (rows.clamp(1, 500), cols.clamp(1, 500));
        Grid { rows, cols, cells: vec![' '; rows * cols] }
    }

    fn at(&self, row: usize, col: usize) -> usize {
        row * self.cols + col
    }

    pub fn clear(&mut self) {
        self.cells.fill(' ');
    }

    /// Writes text from a place, stopping at the right-hand edge.
    pub fn put(&mut self, row: usize, col: usize, text: &str) {
        if row >= self.rows {
            return;
        }
        for (i, ch) in text.chars().enumerate() {
            let c = col + i;
            if c >= self.cols {
                break;
            }
            let at = self.at(row, c);
            self.cells[at] = ch;
        }
    }

    /// The corners a command gave, put in order and brought inside the surface.
    fn region(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> (usize, usize, usize, usize) {
        let (top, bottom) = (r1.min(r2), r1.max(r2).min(self.rows.saturating_sub(1)));
        let (left, right) = (c1.min(c2), c1.max(c2).min(self.cols.saturating_sub(1)));
        (top.min(self.rows.saturating_sub(1)), left.min(self.cols.saturating_sub(1)), bottom, right)
    }

    /// Puts one character everywhere in a region: `@ ... CLEAR` with a space, `@ ... FILL`
    /// with whatever was asked for.
    pub fn fill(&mut self, r1: usize, c1: usize, r2: usize, c2: usize, ch: char) {
        let (top, left, bottom, right) = self.region(r1, c1, r2, c2);
        for row in top..=bottom {
            for col in left..=right {
                let at = self.at(row, col);
                self.cells[at] = ch;
            }
        }
    }

    /// `@ ... TO` and `@ ... BOX`: a frame round a region. `chars` is the nine characters VFP
    /// lets a program give - the four sides, the four corners and what fills the middle.
    pub fn draw_box(&mut self, r1: usize, c1: usize, r2: usize, c2: usize, chars: &str) {
        let glyphs: Vec<char> = chars.chars().collect();
        let g = |i: usize, fallback: char| glyphs.get(i).copied().unwrap_or(fallback);
        let (top, left, bottom, right) = self.region(r1, c1, r2, c2);
        let (across, down) = (g(0, '-'), g(1, '|'));
        for col in left..=right {
            let at = self.at(top, col);
            self.cells[at] = across;
            let at = self.at(bottom, col);
            self.cells[at] = across;
        }
        for row in top..=bottom {
            let at = self.at(row, left);
            self.cells[at] = down;
            let at = self.at(row, right);
            self.cells[at] = down;
        }
        for (index, (row, col)) in [(top, left), (top, right), (bottom, right), (bottom, left)].into_iter().enumerate() {
            let at = self.at(row, col);
            self.cells[at] = g(2 + index, '+');
        }
        // the ninth character fills what the frame encloses
        if glyphs.len() > 8 && bottom > top + 1 && right > left + 1 {
            self.fill(top + 1, left + 1, bottom - 1, right - 1, glyphs[8]);
        }
    }

    /// `@ ... SCROLL`: a region moves, and blank lines come in behind it. A positive count
    /// moves it up (or left, when `sideways`), a negative one the other way; zero clears it.
    pub fn scroll(&mut self, r1: usize, c1: usize, r2: usize, c2: usize, by: i32, sideways: bool) {
        let (top, left, bottom, right) = self.region(r1, c1, r2, c2);
        if by == 0 {
            self.fill(top, left, bottom, right, ' ');
            return;
        }
        let taken: Vec<char> = (top..=bottom)
            .flat_map(|row| (left..=right).map(move |col| (row, col)))
            .map(|(row, col)| self.cells[self.at(row, col)])
            .collect();
        let (height, width) = (bottom - top + 1, right - left + 1);
        for row in 0..height {
            for col in 0..width {
                let from = if sideways {
                    (col as i32 + by).try_into().ok().filter(|c| *c < width).map(|c: usize| row * width + c)
                } else {
                    (row as i32 + by).try_into().ok().filter(|r| *r < height).map(|r: usize| r * width + col)
                };
                let at = self.at(top + row, left + col);
                self.cells[at] = from.map_or(' ', |i| taken[i]);
            }
        }
    }

    /// The surface as lines of text, which is what the host is given to draw.
    pub fn lines(&self) -> Vec<String> {
        (0..self.rows).map(|row| self.cells[row * self.cols..(row + 1) * self.cols].iter().collect()).collect()
    }
}

/// The keys `KEYBOARD` was given, with the names in braces read as the keys they stand for.
///
/// A program writes `KEYBOARD "{ENTER}"` rather than a carriage return it cannot type into a
/// line of source, so the braces are what the command is for. `KEYBOARD ... PLAIN` says to
/// leave them as the characters they are.
pub fn expand_keys(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &rest[open + 1..open + close];
        match key_for(name) {
            Some(ch) => out.push(ch),
            // a name nothing answers to is left as it was written, braces and all
            None => out.push_str(&rest[open..=open + close]),
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// The character a key name stands for, for the keys that are characters.
fn key_for(name: &str) -> Option<char> {
    match name.to_ascii_uppercase().as_str() {
        "ENTER" | "RETURN" | "CTRL+M" => Some('\r'),
        "TAB" | "CTRL+I" => Some('\t'),
        "BACKSPACE" | "CTRL+H" => Some('\u{8}'),
        "ESC" | "ESCAPE" => Some('\u{1b}'),
        "SPACEBAR" | "SPACE" => Some(' '),
        "DEL" | "DELETE" => Some('\u{7f}'),
        _ => None,
    }
}

impl Screen {
    /// Takes the next key out of the type-ahead buffer, remembering it for `LASTKEY()`.
    ///
    /// `KEYBOARD` is what fills that buffer, and everything that reads a key - `INKEY()`, a
    /// `WAIT`, a `READ` - takes from it before it asks a person, which is what lets a program
    /// drive itself.
    pub fn take_key(&mut self) -> Option<char> {
        let key = self.typed.chars().next()?;
        self.typed = self.typed.chars().skip(1).collect();
        self.last_key = Some(key as i32);
        Some(key)
    }

    /// Whether a key is waiting in the buffer, for `CHRSAW()`.
    pub fn key_waiting(&self) -> bool {
        !self.typed.is_empty()
    }
}

/// A field an `@ ... GET` put on the screen, waiting for a `READ` to let the user change it.
#[derive(Debug, Clone)]
pub struct Get {
    /// The variable or field it reads and writes, as the program wrote it.
    pub name: String,
    pub row: usize,
    pub col: usize,
    pub width: usize,
    pub picture: String,
    /// The two conditions, as written: WHEN decides whether it can be reached at all, VALID
    /// whether what was typed may stand.
    pub valid: String,
    pub when: String,
    pub enabled: bool,
    pub value: Value,
}

/// One window a program opened, and what has been drawn in it.
#[derive(Debug, Clone)]
pub struct Window {
    pub name: String,
    /// Where its top left corner sits on the screen, in characters.
    pub row: f64,
    pub col: f64,
    pub height: f64,
    pub width: f64,
    pub title: String,
    pub footer: String,
    /// Whether it is on the screen. A window can be defined without being shown.
    pub visible: bool,
    /// `ZOOM WINDOW MAX` and `ZOOM WINDOW MIN`.
    pub zoomed: bool,
    pub minimized: bool,
    /// Whether it was given a border, and whether it can be dragged about.
    pub border: bool,
    pub movable: bool,
    /// `IN WINDOW <name>`: the window it sits inside.
    pub parent: String,
    /// Where it was before it was zoomed, so `ZOOM WINDOW ... NORM` can put it back.
    pub normal: Option<(f64, f64, f64, f64)>,
    pub grid: Grid,
}

impl Window {
    fn new(name: &str, row: f64, col: f64, height: f64, width: f64) -> Self {
        Window {
            name: name.to_string(),
            row,
            col,
            height,
            width,
            title: String::new(),
            footer: String::new(),
            visible: false,
            zoomed: false,
            minimized: false,
            border: true,
            movable: true,
            parent: String::new(),
            normal: None,
            grid: Grid::new(height.max(1.0) as usize, width.max(1.0) as usize),
        }
    }
}

/// The screen and everything on it.
#[derive(Debug)]
pub struct Screen {
    /// The main window's surface, which is where output goes when no window is current.
    pub main: Grid,
    pub windows: Vec<Window>,
    /// The window output is going to, or "" for the main screen. `WOUTPUT()` answers this.
    pub output: String,
    /// The order they were activated in, newest last: `WONTOP()` reads the end of it.
    pub order: Vec<String>,
    /// Where the next `@` without a place would go, for `ROW()` and `COL()`.
    pub cursor: (usize, usize),
    /// The same for the printer, which nothing here prints to.
    pub printer: (usize, usize),
    /// `SAVE SCREEN TO <var>` keeps a copy; `SAVE SCREEN` on its own keeps the one below.
    pub saved: Option<Grid>,
    /// The windows `SAVE WINDOW` kept, by the name it kept them under.
    pub saved_windows: Vec<(String, Vec<Window>)>,
    /// The window `WLAST()` names: the one that was on top before this one.
    pub last: String,
    /// The keys `KEYBOARD` put in the buffer, which `INKEY()` and a `READ` take from.
    pub typed: String,
    /// The key last taken out of that buffer, for `LASTKEY()`. `None` is "none has been taken",
    /// which leaves LASTKEY() answering with whatever the __LASTKEY variable holds.
    pub last_key: Option<i32>,
    /// The fields `@ ... GET` has put up, waiting for a `READ`.
    pub gets: Vec<Get>,
    /// What the field the user was last in is called, for `VARREAD()`.
    pub read_var: String,
    /// How many READs are running, for `RDLEVEL()`.
    pub read_level: u32,
    /// The choices `@ ... PROMPT` has put up, waiting for a `MENU TO`.
    pub prompts: Vec<String>,
    /// Which of them was chosen last, for `PROMPT()` after a `MENU TO`.
    pub chosen: String,
}

impl Default for Screen {
    fn default() -> Self {
        Screen {
            main: Grid::new(SCREEN_ROWS, SCREEN_COLS),
            windows: Vec::new(),
            output: String::new(),
            order: Vec::new(),
            cursor: (0, 0),
            printer: (0, 0),
            saved: None,
            saved_windows: Vec::new(),
            last: String::new(),
            typed: String::new(),
            last_key: None,
            gets: Vec::new(),
            read_var: String::new(),
            read_level: 0,
            prompts: Vec::new(),
            chosen: String::new(),
        }
    }
}

impl Screen {
    pub fn window(&self, name: &str) -> Option<&Window> {
        self.windows.iter().find(|w| w.name.eq_ignore_ascii_case(name))
    }

    pub fn window_mut(&mut self, name: &str) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.name.eq_ignore_ascii_case(name))
    }

    /// The surface `@` writes to: the window that is current, or the main screen.
    pub fn surface(&mut self) -> &mut Grid {
        let current = self.output.clone();
        match self.windows.iter_mut().find(|w| w.name.eq_ignore_ascii_case(&current)) {
            Some(window) => &mut window.grid,
            None => &mut self.main,
        }
    }

    /// `DEFINE WINDOW`: one of that name, replacing any window that had it.
    pub fn define(&mut self, name: &str, row: f64, col: f64, height: f64, width: f64) -> &mut Window {
        self.windows.retain(|w| !w.name.eq_ignore_ascii_case(name));
        self.windows.push(Window::new(name, row, col, height, width));
        self.windows.last_mut().expect("the window was just made")
    }

    /// `ACTIVATE WINDOW`: it is shown, goes on top, and takes the output unless the command
    /// said `NOSHOW`.
    pub fn activate(&mut self, name: &str, take_output: bool) {
        if self.window(name).is_none() {
            self.define(name, 0.0, 0.0, SCREEN_ROWS as f64, SCREEN_COLS as f64);
        }
        if let Some(window) = self.window_mut(name) {
            window.visible = true;
            window.minimized = false;
        }
        let name = self.window(name).map(|w| w.name.clone()).unwrap_or_else(|| name.to_string());
        self.order.retain(|n| !n.eq_ignore_ascii_case(&name));
        self.order.push(name.clone());
        if take_output {
            self.last = std::mem::replace(&mut self.output, name);
        }
    }

    /// `DEACTIVATE WINDOW`: it comes off the screen and output goes back to the main one.
    pub fn deactivate(&mut self, name: Option<&str>) {
        match name {
            Some(name) => {
                if let Some(window) = self.window_mut(name) {
                    window.visible = false;
                }
                self.order.retain(|n| !n.eq_ignore_ascii_case(name));
                if self.output.eq_ignore_ascii_case(name) {
                    self.last = std::mem::take(&mut self.output);
                }
            }
            None => {
                for window in &mut self.windows {
                    window.visible = false;
                }
                self.order.clear();
                self.last = std::mem::take(&mut self.output);
            }
        }
    }

    /// `RELEASE WINDOWS`: one goes, or all of them.
    pub fn release(&mut self, name: Option<&str>) {
        self.deactivate(name);
        match name {
            Some(name) => self.windows.retain(|w| !w.name.eq_ignore_ascii_case(name)),
            None => self.windows.clear(),
        }
    }

    /// The window on top of the rest, or "" when none is showing.
    pub fn on_top(&self) -> &str {
        self.order.last().map(String::as_str).unwrap_or("")
    }

    /// The screen and its windows as the host draws them.
    pub fn document(&self) -> ScreenDoc {
        ScreenDoc {
            rows: self.main.rows,
            cols: self.main.cols,
            lines: self.main.lines(),
            windows: self
                .order
                .iter()
                .filter_map(|name| self.window(name))
                .filter(|w| w.visible)
                .map(|w| WindowDoc {
                    name: w.name.clone(),
                    title: w.title.clone(),
                    footer: w.footer.clone(),
                    row: w.row,
                    col: w.col,
                    height: w.height,
                    width: w.width,
                    minimized: w.minimized,
                    border: w.border,
                    lines: w.grid.lines(),
                })
                .collect(),
        }
    }
}

/// The screen as the host is given it: the main surface, and the windows over it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenDoc {
    pub rows: usize,
    pub cols: usize,
    pub lines: Vec<String>,
    pub windows: Vec<WindowDoc>,
}

/// One window of it, bottom first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowDoc {
    pub name: String,
    pub title: String,
    pub footer: String,
    pub row: f64,
    pub col: f64,
    pub height: f64,
    pub width: f64,
    pub minimized: bool,
    pub border: bool,
    pub lines: Vec<String>,
}

// WindowDoc holds f64s, so it cannot derive Eq; the screen it belongs to compares by hand.
impl Eq for WindowDoc {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_lands_where_it_was_put_and_stops_at_the_edge() {
        let mut grid = Grid::new(3, 6);
        grid.put(1, 2, "abcdefgh");
        assert_eq!(grid.lines(), vec!["      ", "  abcd", "      "]);
    }

    #[test]
    fn a_box_has_sides_and_corners() {
        let mut grid = Grid::new(4, 5);
        grid.draw_box(0, 0, 3, 4, "-|++++");
        assert_eq!(grid.lines(), vec!["+---+", "|   |", "|   |", "+---+"]);
    }

    #[test]
    fn scrolling_moves_the_region_and_blanks_what_it_left() {
        let mut grid = Grid::new(3, 3);
        grid.put(0, 0, "abc");
        grid.put(1, 0, "def");
        grid.put(2, 0, "ghi");
        grid.scroll(0, 0, 2, 2, 1, false);
        assert_eq!(grid.lines(), vec!["def", "ghi", "   "]);
    }
}
