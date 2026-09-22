//! Menus a program defines while it runs: the bar across the top, the popups under it, and the
//! commands each choice stands for.
//!
//! Visual FoxPro builds a menu from a menu file, which this runtime reads as a document, or a
//! command at a time - `DEFINE MENU`, `DEFINE PAD`, `DEFINE POPUP`, `DEFINE BAR` - which is what
//! this holds. Nothing here draws anything: `ACTIVATE MENU` hands the whole thing to the host,
//! which puts it where the running program's menu goes.

use crate::host::{MenuDoc, MenuDocItem, MenuDocKey, MenuDocResult};

/// One choice of a popup: what it says, what it does, and whether it can be chosen at all.
#[derive(Debug, Clone, Default)]
pub struct Bar {
    pub number: i32,
    pub prompt: String,
    /// The command `ON SELECTION BAR` gave it.
    pub command: String,
    /// The popup `ON BAR ... ACTIVATE POPUP` opens instead.
    pub popup: String,
    pub key: String,
    pub message: String,
    /// `SET MARK OF BAR`: the tick beside it.
    pub marked: bool,
    /// `SKIP FOR`: the condition that greys it out, as it was written.
    pub skip: String,
}

/// A popup: a list of bars, and what a choice from it does when the bar itself says nothing.
#[derive(Debug, Clone, Default)]
pub struct Popup {
    pub name: String,
    pub bars: Vec<Bar>,
    pub command: String,
    pub marked: bool,
    pub visible: bool,
}

/// One word of a menu bar, and the popup it opens.
#[derive(Debug, Clone, Default)]
pub struct Pad {
    pub name: String,
    pub prompt: String,
    pub popup: String,
    pub command: String,
    pub key: String,
    pub message: String,
    pub marked: bool,
    pub skip: String,
}

/// A menu bar: the pads across it, and what any choice from it does.
#[derive(Debug, Clone, Default)]
pub struct MenuBar {
    pub name: String,
    pub pads: Vec<Pad>,
    pub command: String,
    pub visible: bool,
}

/// Everything a program has defined, and what it last chose.
#[derive(Debug, Default)]
pub struct Menus {
    pub bars: Vec<MenuBar>,
    pub popups: Vec<Popup>,
    /// The menu `ACTIVATE MENU` put up, and the popup `ACTIVATE POPUP` opened.
    pub active_menu: String,
    pub active_popup: String,
    /// What the last choice was, for BAR(), PAD(), POPUP() and MENU().
    pub last_bar: i32,
    pub last_pad: String,
    pub last_popup: String,
    pub last_menu: String,
    /// What the message line says, from SET MESSAGE TO.
    pub message: String,
    /// What the last choice said, for PROMPT().
    pub last_prompt: String,
    /// The popup the last `DEFINE BAR` named, so the next one without an `OF` joins it. This
    /// is not what POPUP() answers: defining a popup is not choosing from one.
    pub defining_popup: String,
    /// The words the last `SET SYSMENU` was given.
    pub sysmenu: String,
    /// PUSH MENU and PUSH POPUP keep what was defined, so POP can put it back.
    saved: Vec<(Vec<MenuBar>, Vec<Popup>)>,
}

impl Menus {
    pub fn menu_mut(&mut self, name: &str) -> Option<&mut MenuBar> {
        self.bars.iter_mut().find(|m| m.name.eq_ignore_ascii_case(name))
    }

    pub fn menu(&self, name: &str) -> Option<&MenuBar> {
        self.bars.iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }

    pub fn popup_mut(&mut self, name: &str) -> Option<&mut Popup> {
        self.popups.iter_mut().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    pub fn popup(&self, name: &str) -> Option<&Popup> {
        self.popups.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// `DEFINE MENU`: a menu of that name, empty, replacing one of the same name.
    pub fn define_menu(&mut self, name: &str) {
        self.bars.retain(|m| !m.name.eq_ignore_ascii_case(name));
        self.bars.push(MenuBar { name: name.to_string(), ..MenuBar::default() });
    }

    /// `DEFINE POPUP`: the same for a popup.
    pub fn define_popup(&mut self, name: &str) {
        self.popups.retain(|p| !p.name.eq_ignore_ascii_case(name));
        self.popups.push(Popup { name: name.to_string(), ..Popup::default() });
    }

    /// `DEFINE PAD`: a word of a menu bar, replacing one of the same name.
    pub fn define_pad(&mut self, menu: &str, pad: &str, prompt: &str, key: &str, message: &str) {
        if self.menu(menu).is_none() {
            self.define_menu(menu);
        }
        let bar = self.menu_mut(menu).expect("the menu was just made");
        bar.pads.retain(|p| !p.name.eq_ignore_ascii_case(pad));
        bar.pads.push(Pad {
            name: pad.to_string(),
            prompt: prompt.to_string(),
            key: key.to_string(),
            message: message.to_string(),
            ..Pad::default()
        });
    }

    /// `DEFINE BAR`: a choice of a popup, in the order the numbers put them.
    pub fn define_bar(&mut self, popup: &str, number: i32, prompt: &str, key: &str, message: &str) {
        if self.popup(popup).is_none() {
            self.define_popup(popup);
        }
        let p = self.popup_mut(popup).expect("the popup was just made");
        p.bars.retain(|b| b.number != number);
        p.bars.push(Bar {
            number,
            prompt: prompt.to_string(),
            key: key.to_string(),
            message: message.to_string(),
            ..Bar::default()
        });
        p.bars.sort_by_key(|b| b.number);
    }

    /// PUSH MENU / PUSH POPUP: what is defined now, kept to be put back.
    pub fn push(&mut self) {
        self.saved.push((self.bars.clone(), self.popups.clone()));
    }

    /// POP MENU / POP POPUP: back to what was kept.
    pub fn pop(&mut self) {
        if let Some((bars, popups)) = self.saved.pop() {
            self.bars = bars;
            self.popups = popups;
        }
    }

    /// `SET MARK OF`: the tick beside a pad, a bar, a whole menu or a whole popup. `flags`
    /// says which was named - bit 2 a bar or a popup rather than a pad or a menu, bit 3 the
    /// menu or popup itself rather than one of its choices.
    pub fn set_mark(&mut self, name: &str, number: i32, of: &str, flags: u8, on: bool) {
        let whole = flags & 8 != 0;
        let bar_side = flags & 4 != 0;
        match (whole, bar_side) {
            (true, true) => {
                if let Some(p) = self.popup_mut(name) {
                    p.marked = on;
                }
            }
            (true, false) => {
                if let Some(m) = self.menu_mut(name) {
                    for pad in &mut m.pads {
                        pad.marked = on;
                    }
                }
            }
            (false, true) => {
                if let Some(bar) = self.popup_mut(of).and_then(|p| p.bars.iter_mut().find(|b| b.number == number)) {
                    bar.marked = on;
                }
            }
            (false, false) => {
                let menu = if of.is_empty() { "_MSYSMENU" } else { of };
                if let Some(pad) = self.menu_mut(menu).and_then(|m| m.pads.iter_mut().find(|p| p.name.eq_ignore_ascii_case(name))) {
                    pad.marked = on;
                }
            }
        }
    }

    /// `SET SKIP OF`: the condition that greys one out, kept as it was written so the host
    /// works it out again every time the menu is opened.
    pub fn set_skip(&mut self, name: &str, number: i32, of: &str, flags: u8, cond: &str) {
        let whole = flags & 8 != 0;
        let bar_side = flags & 4 != 0;
        match (whole, bar_side) {
            (true, true) => {
                if let Some(p) = self.popup_mut(name) {
                    for bar in &mut p.bars {
                        bar.skip = cond.to_string();
                    }
                }
            }
            (true, false) => {
                if let Some(m) = self.menu_mut(name) {
                    for pad in &mut m.pads {
                        pad.skip = cond.to_string();
                    }
                }
            }
            (false, true) => {
                if let Some(bar) = self.popup_mut(of).and_then(|p| p.bars.iter_mut().find(|b| b.number == number)) {
                    bar.skip = cond.to_string();
                }
            }
            (false, false) => {
                let menu = if of.is_empty() { "_MSYSMENU" } else { of };
                if let Some(pad) = self.menu_mut(menu).and_then(|m| m.pads.iter_mut().find(|p| p.name.eq_ignore_ascii_case(name))) {
                    pad.skip = cond.to_string();
                }
            }
        }
    }

    /// `RELEASE MENU` / `RELEASE POPUP`: one of them goes, or all of that kind.
    pub fn release(&mut self, name: Option<&str>, popup: bool) {
        match (name, popup) {
            (Some(name), false) => self.bars.retain(|m| !m.name.eq_ignore_ascii_case(name)),
            (Some(name), true) => self.popups.retain(|p| !p.name.eq_ignore_ascii_case(name)),
            (None, false) => self.bars.clear(),
            (None, true) => self.popups.clear(),
        }
        if self.menu(&self.active_menu.clone()).is_none() {
            self.active_menu.clear();
        }
        if self.popup(&self.active_popup.clone()).is_none() {
            self.active_popup.clear();
        }
    }

    /// `RELEASE BAR n OF popup`, or every bar of it when no number is given.
    pub fn release_bar(&mut self, popup: &str, number: Option<i32>) {
        if let Some(p) = self.popup_mut(popup) {
            match number {
                Some(n) => p.bars.retain(|b| b.number != n),
                None => p.bars.clear(),
            }
        }
    }

    /// `RELEASE PAD name OF menu`, or every pad of it when no name is given.
    pub fn release_pad(&mut self, menu: &str, name: Option<&str>) {
        if let Some(m) = self.menu_mut(menu) {
            match name {
                Some(pad) => m.pads.retain(|p| !p.name.eq_ignore_ascii_case(pad)),
                None => m.pads.clear(),
            }
        }
    }

    /// What a program chose from the menu that is up, so BAR(), PAD(), POPUP() and PROMPT()
    /// can say what it was while the command it stands for runs.
    pub fn chose(&mut self, pad: &str, bar: i32, popup: &str, prompt: &str) {
        if !pad.is_empty() {
            self.last_pad = pad.to_string();
        }
        if bar > 0 {
            self.last_bar = bar;
        }
        if !popup.is_empty() {
            self.last_popup = popup.to_string();
        }
        self.last_prompt = prompt.to_string();
        self.last_menu = self.active_menu.clone();
    }

    /// The menu of that name as a document the host can put up, or nothing when there is no
    /// menu of that name. A popup is given as one pad with the popup's bars under it, which is
    /// how a floating popup shows where the only place a menu goes is the bar across the top.
    pub fn document(&self, name: &str, popup: bool) -> Option<MenuDoc> {
        let items = if popup {
            let p = self.popup(name)?;
            vec![MenuDocItem {
                id: format!("popup:{}", p.name),
                prompt: p.name.clone(),
                name: Some(p.name.clone()),
                result: MenuDocResult { kind: "submenu".into(), text: None },
                hotkey: None,
                skip_for: None,
                message: None,
                enabled: None,
                children: Some(p.bars.iter().map(|b| self.bar_item(&p.name, b, 1)).collect()),
            }]
        } else {
            let menu = self.menu(name)?;
            menu.pads.iter().map(|pad| self.pad_item(pad, 0)).collect()
        };
        Some(MenuDoc {
            schema: "foxdev-menu".into(),
            version: 1,
            name: name.to_string(),
            location: "Replace".into(),
            items,
        })
    }

    /// One pad, with the popup it opens under it.
    fn pad_item(&self, pad: &Pad, depth: u8) -> MenuDocItem {
        let children = if pad.popup.is_empty() || depth >= 8 {
            None
        } else {
            self.popup(&pad.popup)
                .map(|p| p.bars.iter().map(|b| self.bar_item(&p.name, b, depth + 1)).collect())
        };
        MenuDocItem {
            id: format!("pad:{}", pad.name),
            prompt: if pad.prompt.is_empty() { pad.name.clone() } else { pad.prompt.clone() },
            name: Some(pad.name.clone()),
            result: match children {
                Some(_) => MenuDocResult { kind: "submenu".into(), text: None },
                None => MenuDocResult { kind: "command".into(), text: Some(pad.command.clone()) },
            },
            hotkey: hotkey_of(&pad.key),
            skip_for: (!pad.skip.is_empty()).then(|| pad.skip.clone()),
            message: (!pad.message.is_empty()).then(|| pad.message.clone()),
            enabled: None,
            children,
        }
    }

    /// One bar of a popup, with the popup it opens in turn under it.
    fn bar_item(&self, popup: &str, bar: &Bar, depth: u8) -> MenuDocItem {
        let children = if bar.popup.is_empty() || depth >= 8 {
            None
        } else {
            self.popup(&bar.popup)
                .map(|p| p.bars.iter().map(|b| self.bar_item(&p.name, b, depth + 1)).collect())
        };
        MenuDocItem {
            id: format!("bar:{popup}:{}", bar.number),
            prompt: bar.prompt.clone(),
            name: None,
            result: match children {
                Some(_) => MenuDocResult { kind: "submenu".into(), text: None },
                None => MenuDocResult {
                    kind: "command".into(),
                    text: Some(if bar.command.is_empty() {
                        self.popup(popup).map(|p| p.command.clone()).unwrap_or_default()
                    } else {
                        bar.command.clone()
                    }),
                },
            },
            hotkey: hotkey_of(&bar.key),
            skip_for: (!bar.skip.is_empty()).then(|| bar.skip.clone()),
            message: (!bar.message.is_empty()).then(|| bar.message.clone()),
            enabled: None,
            children,
        }
    }
}

/// A `KEY` clause, split the way a menu document keeps a hotkey. `CTRL+S` and `ALT+F4` are
/// how they are written; anything without a key of its own is left off.
/// What a prompt says with the marks taken out of it: `\<` puts the hot key on the letter after
/// it, `\-` makes a separator, a `\` in front disables the item and `\\` is a backslash of its
/// own. What a program is told a prompt says has none of them left in it.
pub fn plain_prompt(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('\\') => {
                out.push('\\');
                chars.next();
            }
            Some('<' | '-') => {
                chars.next();
            }
            _ => {}
        }
    }
    out
}

fn hotkey_of(text: &str) -> Option<MenuDocKey> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    let mut key = String::new();
    for part in text.split('+') {
        match part.trim().to_ascii_uppercase().as_str() {
            "CTRL" => ctrl = true,
            "ALT" => alt = true,
            "SHIFT" => shift = true,
            _ => key = part.trim().to_string(),
        }
    }
    if key.is_empty() {
        return None;
    }
    Some(MenuDocKey {
        key,
        ctrl: ctrl.then_some(true),
        alt: alt.then_some(true),
        shift: shift.then_some(true),
        label: Some(text.to_string()),
    })
}
