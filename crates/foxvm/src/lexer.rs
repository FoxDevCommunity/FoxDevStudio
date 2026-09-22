//! Tokenizer for the FoxPro-like language.
//!
//! The lexer works on characters (spans are character offsets) and understands the VFP line
//! model: `;` continues a logical line, `*`/`NOTE` comment whole lines, `&&` comments to the
//! end of a line, and `TEXT ... ENDTEXT` captures raw lines. One `Newline` token is emitted per
//! logical line (blank lines are collapsed) followed by a final `Eof`.

use crate::ast::{DateLit, TimeLit};
use crate::diagnostics::{Diagnostic, LineMap, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    /// Identifier or keyword, original spelling.
    Ident(String),
    /// A number and the width it was written in: the characters it took, and how many of them
    /// were past the point. `? 001` prints "  1", so the width has to travel with the token -
    /// a number that reaches the parser through a `#DEFINE` has no source text left to read.
    Num(f64, u8, u8),
    Str(String),
    /// `{^2024-01-31}`; `None` for the empty date `{}` / `{//}`.
    Date(Option<DateLit>),
    /// `{^2024-01-31 10:30}`; `None` for the empty datetime `{/:}`.
    DateTime(Option<(DateLit, TimeLit)>),
    /// `{^&lcYear./&lcMonth./&lcDay.}`: a date or datetime constant with a macro in it. What
    /// day it is cannot be worked out here, because the text is not there until the line runs.
    MacroDate,
    /// `.T.` / `.Y.`
    True,
    /// `.F.` / `.N.`
    False,
    /// `.NULL.`
    Null,
    /// `.AND.`
    DotAnd,
    /// `.OR.`
    DotOr,
    /// `.NOT.`
    DotNot,
    /// Raw lines between `TEXT` and `ENDTEXT`, joined with `\n`.
    RawText(String),
    /// The `\` or `\\` that starts a line of text merge output; true for `\\`, which carries on
    /// the line before rather than starting a new one. A `RawText` with the rest of the line
    /// follows it.
    TextMark(bool),
    /// End of a logical line.
    Newline,
    Eof,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    /// `^` or `**`
    Caret,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `<>`, `#`, `!=`
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `$`
    Dollar,
    LParen,
    RParen,
    /// `[` used as an index bracket (immediately after an identifier, `)` or `]`).
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    At,
    /// `&` (macro substitution).
    Amp,
    /// `!` not followed by `=`.
    Bang,
    /// `\`, part of a bare path such as `SET PATH TO ..\Utils`.
    Backslash,
    /// `;` with something after it on the line, so it continues nothing. Only a `;` that ends a
    /// line continues it; one in the middle is an ordinary character of the command, as in
    /// `SET PATH TO c:\a;c:\b`. Whether it belongs there is for whoever reads the line to say:
    /// the half of a `#IF` that was thrown away is full of text that is not FoxPro, and Visual
    /// FoxPro never looks at it.
    Semi,
    /// `?` at the start of a line.
    Question,
    /// `??` at the start of a line.
    DoubleQuestion,
    /// `#` at the start of a line (preprocessor directive).
    Hash,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokKind,
    /// Character offsets into the source.
    pub span: Span,
    /// 1-based physical line the token starts on.
    pub line: u32,
    /// A closed `[...]` the lexer read as a string. `[` also opens a subscript, and which one
    /// it is depends on where the parser is, so the parser can still read this as `[expr]`.
    pub bracketed: bool,
}

impl Token {
    /// True when the token is the identifier `upper` (compared case-insensitively).
    pub fn is_word(&self, upper: &str) -> bool {
        matches!(&self.kind, TokKind::Ident(t) if t.eq_ignore_ascii_case(upper))
    }

    /// Identifier text as written, if this token is an identifier.
    pub fn ident(&self) -> Option<&str> {
        match &self.kind {
            TokKind::Ident(t) => Some(t),
            _ => None,
        }
    }

    pub fn is_newline(&self) -> bool {
        matches!(self.kind, TokKind::Newline | TokKind::Eof)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// A carriage return with no line feed after it is a line break of its own.
///
/// Program text put together while the program runs is written that way: `EXECSCRIPT("a" +
/// CHR(13) + "b")` is two lines to Visual FoxPro, and so is a memo field, which stores its
/// breaks as bare CR. Swapping the character keeps every span where it was, so nothing that
/// counts characters has to know.
fn with_bare_returns_as_breaks(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(if c == '\r' && chars.peek() != Some(&'\n') { '\n' } else { c });
    }
    out
}

pub fn lex(src: &str) -> LexOutput {
    let src = &with_bare_returns_as_breaks(src);
    let mut lexer = Lexer {
        chars: src.chars().collect(),
        pos: 0,
        line: 1,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
        map: LineMap::new(src),
        at_line_start: true,
        bracket_depth: 0,
        pending_text: None,
        on_line: false,
    };
    lexer.run();
    LexOutput { tokens: lexer.tokens, diagnostics: lexer.diagnostics }
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    map: LineMap,
    /// No token has been emitted on the current logical line yet.
    at_line_start: bool,
    /// Open index brackets on the current logical line.
    bracket_depth: usize,
    /// Span of a `TEXT` statement keyword whose raw body starts on the next physical line.
    pending_text: Option<Span>,
    /// The current logical line starts with `ON` (`ON ERROR ...` takes a whole command).
    on_line: bool,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// A number token that remembers how wide it was written, which is how wide it prints.
fn numeric(value: f64, text: &str) -> TokKind {
    let width = crate::value::Width::written(text);
    TokKind::Num(value, width.chars, width.decimals)
}

pub(crate) fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

impl Lexer {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(self.map.error(span, message));
    }

    fn push(&mut self, kind: TokKind, start: usize, end: usize) {
        if self.at_line_start {
            self.on_line = matches!(&kind, TokKind::Ident(t) if t.eq_ignore_ascii_case("ON"));
        }
        self.tokens.push(Token { kind, span: Span::new(start, end), line: self.line, bracketed: false });
        self.at_line_start = false;
    }

    /// Index of the end of the current physical line (position of `\n` or the source length).
    fn eol(&self, from: usize) -> usize {
        let mut i = from;
        while i < self.chars.len() && self.chars[i] != '\n' {
            i += 1;
        }
        i
    }

    /// True when the previous token ends exactly at `pos` and can be followed by an index
    /// bracket or a member dot (identifier, `)` or `]`).
    fn prev_is_adjacent_operand(&self) -> bool {
        match self.tokens.last() {
            Some(t) if t.span.end == self.pos => {
                matches!(t.kind, TokKind::Ident(_) | TokKind::RParen | TokKind::RBracket)
            }
            _ => false,
        }
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\t' | '\r' | '\u{0c}' | '\u{feff}' => self.pos += 1,
                '\n' => self.newline(),
                '*' if self.at_line_start => self.skip_comment_line(),
                // `\ text` and `\\ text` put the rest of the line out as it stands, so nothing
                // in it is read as FoxPro: a quotation mark in it closes nothing.
                '\\' if self.at_line_start => self.text_line(),
                '&' if self.peek_at(1) == Some('&') => self.pos = self.eol(self.pos),
                ';' => self.semicolon(),
                '"' | '\'' => self.string(c),
                '[' => {
                    if self.prev_is_adjacent_operand() {
                        self.bracket_depth += 1;
                        self.push(TokKind::LBracket, self.pos, self.pos + 1);
                        self.pos += 1;
                    } else {
                        self.string(']');
                    }
                }
                ']' => {
                    if self.bracket_depth > 0 {
                        self.bracket_depth -= 1;
                        self.push(TokKind::RBracket, self.pos, self.pos + 1);
                    } else {
                        self.error(Span::new(self.pos, self.pos + 1), "Unexpected ']'");
                    }
                    self.pos += 1;
                }
                '{' => self.date(),
                '.' => self.dot(),
                '0'..='9' => self.number(),
                c if is_ident_start(c) => self.ident(),
                '#' if self.at_line_start => {
                    self.push(TokKind::Hash, self.pos, self.pos + 1);
                    self.pos += 1;
                }
                // `?` starts a statement, is the command of `ON ERROR ? x`, and stands for a
                // file the user is asked to choose: `USE ?`. Which of those it is is the
                // parser's to say, so it is always a token.
                '?' => {
                    if self.peek_at(1) == Some('?') && (self.at_line_start || self.on_line) {
                        self.push(TokKind::DoubleQuestion, self.pos, self.pos + 2);
                        self.pos += 2;
                    } else {
                        self.push(TokKind::Question, self.pos, self.pos + 1);
                        self.pos += 1;
                    }
                }
                _ => self.operator(c),
            }
        }
        if let Some(span) = self.pending_text.take() {
            // `TEXT` on the last line without a newline: no body, no ENDTEXT.
            self.error(span, "TEXT without matching ENDTEXT");
        }
        let end = self.chars.len();
        if !self.at_line_start {
            self.push(TokKind::Newline, end, end);
        }
        self.at_line_start = true;
        self.push(TokKind::Eof, end, end);
    }

    /// Handles a physical newline that ends a logical line.
    fn newline(&mut self) {
        if !self.at_line_start {
            self.push(TokKind::Newline, self.pos, self.pos);
            self.at_line_start = true;
        }
        self.bracket_depth = 0;
        self.pos += 1;
        self.line += 1;
        if let Some(span) = self.pending_text.take() {
            self.capture_raw_text(span);
        }
    }

    /// `*` or `NOTE` comment: the rest of the physical line, continued while it ends in `;`.
    fn skip_comment_line(&mut self) {
        loop {
            let end = self.eol(self.pos);
            let text: String = self.chars[self.pos..end].iter().collect();
            let continued = text.trim_end().ends_with(';');
            self.pos = end;
            if continued && self.pos < self.chars.len() {
                self.pos += 1;
                self.line += 1;
            } else {
                break;
            }
        }
    }

    fn semicolon(&mut self) {
        let start = self.pos;
        let mut j = start + 1;
        while j < self.chars.len() && matches!(self.chars[j], ' ' | '\t' | '\r') {
            j += 1;
        }
        if j + 1 < self.chars.len() && self.chars[j] == '&' && self.chars[j + 1] == '&' {
            j = self.eol(j);
        }
        if j >= self.chars.len() {
            self.pos = j;
        } else if self.chars[j] == '\n' {
            self.pos = j + 1;
            self.line += 1;
        } else {
            // Visual FoxPro continues a line only on a `;` that ends it. One with more of the
            // line after it is just a character the command gets, and `SET PATH TO c:\a;c:\b`
            // is how people write two directories.
            self.push(TokKind::Semi, start, start + 1);
            self.pos = start + 1;
        }
    }

    /// String literal starting at the current position and ending with `close`.
    fn string(&mut self, close: char) {
        let start = self.pos;
        let mut i = start + 1;
        while i < self.chars.len() && self.chars[i] != close && self.chars[i] != '\n' {
            i += 1;
        }
        if i < self.chars.len() && self.chars[i] == close {
            let text: String = self.chars[start + 1..i].iter().collect();
            self.push(TokKind::Str(text), start, i + 1);
            // a closed `[...]` may yet turn out to be a subscript; only the parser can tell
            if close == ']' && let Some(t) = self.tokens.last_mut() {
                t.bracketed = true;
            }
            self.pos = i + 1;
        } else {
            let mut end = i;
            while end > start + 1 && self.chars[end - 1] == '\r' {
                end -= 1;
            }
            let text: String = self.chars[start + 1..end].iter().collect();
            self.error(Span::new(start, end), format!("Unterminated string: missing closing {close}"));
            self.push(TokKind::Str(text), start, end);
            self.pos = i;
        }
    }

    fn date(&mut self) {
        let start = self.pos;
        let mut i = start + 1;
        while i < self.chars.len() && self.chars[i] != '}' && self.chars[i] != '\n' {
            i += 1;
        }
        if i < self.chars.len() && self.chars[i] == '}' {
            let content: String = self.chars[start + 1..i].iter().collect();
            match parse_date_literal(&content) {
                Some(kind) => self.push(kind, start, i + 1),
                // a constant built out of macros is not a bad constant, it is one whose text
                // arrives when the line runs; the parser hands the whole line over to then
                None if content.contains('&') => self.push(TokKind::MacroDate, start, i + 1),
                None => {
                    self.error(Span::new(start, i + 1), "Invalid date/datetime constant");
                    self.push(TokKind::Date(None), start, i + 1);
                }
            }
            self.pos = i + 1;
        } else {
            self.error(Span::new(start, i), "Invalid date/datetime constant");
            self.push(TokKind::Date(None), start, i);
            self.pos = i;
        }
    }

    fn dot(&mut self) {
        let start = self.pos;
        let member_position = self.prev_is_adjacent_operand();
        if !member_position && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            self.number();
            return;
        }
        if !member_position {
            let mut i = start + 1;
            while i < self.chars.len() && self.chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i > start + 1 && i < self.chars.len() && self.chars[i] == '.' {
                let word: String = self.chars[start + 1..i].iter().collect::<String>().to_ascii_uppercase();
                let kind = match word.as_str() {
                    "T" | "Y" => Some(TokKind::True),
                    "F" | "N" => Some(TokKind::False),
                    "NULL" => Some(TokKind::Null),
                    "AND" => Some(TokKind::DotAnd),
                    "OR" => Some(TokKind::DotOr),
                    "NOT" => Some(TokKind::DotNot),
                    _ => None,
                };
                if let Some(kind) = kind {
                    self.push(kind, start, i + 1);
                    self.pos = i + 1;
                    return;
                }
            }
        }
        self.push(TokKind::Dot, start, start + 1);
        self.pos = start + 1;
    }

    fn number(&mut self) {
        let start = self.pos;
        let chars = &self.chars;
        let mut i = start;
        if chars[i] == '0' && matches!(chars.get(i + 1), Some('x') | Some('X')) {
            i += 2;
            let digits_start = i;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            let digits: String = chars[digits_start..i].iter().collect();
            let value = i64::from_str_radix(&digits, 16).ok();
            self.pos = i;
            match value {
                Some(v) if i > digits_start => self.push(numeric(v as f64, &chars[start..i].iter().collect::<String>()), start, i),
                _ => {
                    self.error(Span::new(start, i), "Invalid hexadecimal constant");
                    self.push(TokKind::Num(0.0, 1, 0), start, i);
                }
            }
            return;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i < chars.len() && chars[i] == '.' && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i < chars.len() && matches!(chars[i], 'e' | 'E') {
            let mut j = i + 1;
            if j < chars.len() && matches!(chars[j], '+' | '-') {
                j += 1;
            }
            if j < chars.len() && chars[j].is_ascii_digit() {
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        let text: String = chars[start..i].iter().collect();
        let value = text.parse::<f64>().unwrap_or(0.0);
        self.push(numeric(value, &text), start, i);
        self.pos = i;
    }

    fn ident(&mut self) {
        let start = self.pos;
        let mut i = start;
        while i < self.chars.len() && is_ident_char(self.chars[i]) {
            i += 1;
        }
        let text: String = self.chars[start..i].iter().collect();
        if self.at_line_start && text.eq_ignore_ascii_case("NOTE") {
            self.skip_comment_line();
            return;
        }
        let starts_text = self.at_line_start && text.eq_ignore_ascii_case("TEXT") && self.text_clauses_follow(i);
        self.push(TokKind::Ident(text), start, i);
        self.pos = i;
        if starts_text {
            self.pending_text = Some(Span::new(start, i));
        }
    }

    /// Whether what stands after a `TEXT` at `from` is the clause list the command takes, which
    /// is the only thing that may follow it.
    ///
    /// The word alone does not make a text block: `text = "hi"` assigns to a variable, and a
    /// line like `text/plain=Yes` sitting in the half of a `#IF` that was thrown away is a bad
    /// command rather than the start of a block that swallows the rest of the file.
    fn text_clauses_follow(&self, from: usize) -> bool {
        const CLAUSES: [&str; 6] = ["TO", "ADDITIVE", "NOSHOW", "TEXTMERGE", "PRETEXT", "FLAGS"];
        let mut j = from;
        while j < self.chars.len() && matches!(self.chars[j], ' ' | '\t' | '\r') {
            j += 1;
        }
        // `TEXT` on its own, or with nothing after it but a comment, starts a block
        if j >= self.chars.len() || self.chars[j] == '\n' {
            return true;
        }
        if self.chars[j] == '&' && self.chars.get(j + 1) == Some(&'&') {
            return true;
        }
        let mut end = j;
        while end < self.chars.len() && is_ident_char(self.chars[end]) {
            end += 1;
        }
        let word = self.chars[j..end].iter().collect::<String>().to_ascii_uppercase();
        CLAUSES.contains(&word.as_str())
    }

    fn operator(&mut self, c: char) {
        let start = self.pos;
        let next = self.peek_at(1);
        let (kind, len) = match (c, next) {
            ('*', Some('*')) => (TokKind::Caret, 2),
            ('*', _) => (TokKind::Star, 1),
            ('+', _) => (TokKind::Plus, 1),
            ('-', _) => (TokKind::Minus, 1),
            ('/', _) => (TokKind::Slash, 1),
            ('%', _) => (TokKind::Percent, 1),
            ('^', _) => (TokKind::Caret, 1),
            ('=', Some('=')) => (TokKind::EqEq, 2),
            // Visual FoxPro reads `=>` and `=<` as the same comparisons written the other way
            // round, and people do write them: one of the shipped samples asks for
            // `company => lcLow` and means `company >= lcLow`.
            ('=', Some('>')) => (TokKind::Ge, 2),
            ('=', Some('<')) => (TokKind::Le, 2),
            ('=', _) => (TokKind::Eq, 1),
            ('<', Some('>')) => (TokKind::Ne, 2),
            ('<', Some('=')) => (TokKind::Le, 2),
            ('<', _) => (TokKind::Lt, 1),
            ('>', Some('=')) => (TokKind::Ge, 2),
            ('>', _) => (TokKind::Gt, 1),
            ('!', Some('=')) => (TokKind::Ne, 2),
            ('!', _) => (TokKind::Bang, 1),
            ('#', _) => (TokKind::Ne, 1),
            ('$', _) => (TokKind::Dollar, 1),
            ('(', _) => (TokKind::LParen, 1),
            (')', _) => (TokKind::RParen, 1),
            (',', _) => (TokKind::Comma, 1),
            (':', _) => (TokKind::Colon, 1),
            ('@', _) => (TokKind::At, 1),
            ('&', _) => (TokKind::Amp, 1),
            ('\\', _) => (TokKind::Backslash, 1),
            _ => {
                self.error(Span::new(start, start + 1), format!("Unexpected character '{c}'"));
                self.pos += 1;
                return;
            }
        };
        self.push(kind, start, start + len);
        self.pos = start + len;
    }

    /// A `\` or `\\` line: the marker, then the rest of the line as one piece of text.
    ///
    /// Every character after the marker belongs to the text, spaces and tabs included: Visual
    /// FoxPro puts out what is written and eats nothing.
    fn text_line(&mut self) {
        let start = self.pos;
        let same_line = self.peek_at(1) == Some('\\');
        let markers = if same_line { 2 } else { 1 };
        self.push(TokKind::TextMark(same_line), start, start + markers);
        self.pos = start + markers;
        let raw_start = self.pos;
        let mut end = self.eol(raw_start);
        while end > raw_start && self.chars[end - 1] == '\r' {
            end -= 1;
        }
        let raw: String = self.chars[raw_start..end].iter().collect();
        self.push(TokKind::RawText(raw), raw_start, end);
        self.pos = end;
    }

    /// Captures raw physical lines after a `TEXT` line until `ENDTEXT`.
    fn capture_raw_text(&mut self, text_span: Span) {
        let raw_start = self.pos;
        let mut lines: Vec<String> = Vec::new();
        loop {
            if self.pos >= self.chars.len() {
                let raw = lines.join("\n");
                self.error(text_span, "TEXT without matching ENDTEXT");
                self.push(TokKind::RawText(raw), raw_start, self.pos);
                return;
            }
            let line_start = self.pos;
            let end = self.eol(line_start);
            // First word of the line.
            let mut w = line_start;
            while w < end && matches!(self.chars[w], ' ' | '\t' | '\r') {
                w += 1;
            }
            let mut we = w;
            while we < end && self.chars[we].is_ascii_alphabetic() {
                we += 1;
            }
            let word: String = self.chars[w..we].iter().collect();
            if word.len() >= 4 && "ENDTEXT".starts_with(&word.to_ascii_uppercase()) {
                let raw = lines.join("\n");
                let mut raw_end = line_start;
                if raw_end > raw_start {
                    raw_end -= 1; // exclude the newline before ENDTEXT
                }
                self.push(TokKind::RawText(raw), raw_start, raw_end.max(raw_start));
                self.push(TokKind::Ident(word), w, we);
                // whatever else stands on the ENDTEXT line is not read: the word ends the
                // block and Visual FoxPro ignores the rest of the line
                self.pos = end;
                return;
            }
            let mut text_end = end;
            while text_end > line_start && self.chars[text_end - 1] == '\r' {
                text_end -= 1;
            }
            lines.push(self.chars[line_start..text_end].iter().collect());
            self.pos = end;
            if self.pos < self.chars.len() {
                self.pos += 1;
                self.line += 1;
            }
        }
    }
}

/// Parses the text between `{` and `}`.
fn parse_date_literal(content: &str) -> Option<TokKind> {
    let t = content.trim();
    if t.is_empty() {
        return Some(TokKind::Date(None));
    }
    if t.chars().all(|c| matches!(c, '/' | ':' | ' ' | '\t')) {
        return Some(if t.contains(':') { TokKind::DateTime(None) } else { TokKind::Date(None) });
    }
    let rest = t.strip_prefix('^')?;
    let rest = rest.replace(',', " ");
    let mut parts = rest.split_whitespace();
    let date_part = parts.next()?;
    let fields: Vec<&str> = date_part.split(['-', '/', '.']).collect();
    if fields.len() != 3 {
        return None;
    }
    let year: i32 = fields[0].parse().ok()?;
    let month: u32 = fields[1].parse().ok()?;
    let day: u32 = fields[2].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || !(1..=9999).contains(&year) {
        return None;
    }
    let date = DateLit { year, month, day };
    let Some(time_part) = parts.next() else {
        return Some(TokKind::Date(Some(date)));
    };
    let mut time_part = time_part.to_ascii_uppercase();
    let mut meridian: Option<String> = parts.next().map(|s| s.to_ascii_uppercase());
    if meridian.is_none() {
        for suffix in ["AM", "PM"] {
            if let Some(stripped) = time_part.strip_suffix(suffix) {
                meridian = Some(suffix.to_string());
                time_part = stripped.to_string();
                break;
            }
        }
    }
    if parts.next().is_some() {
        return None;
    }
    let tf: Vec<&str> = time_part.split(':').collect();
    if tf.len() < 2 || tf.len() > 3 {
        return None;
    }
    let mut hour: u32 = tf[0].parse().ok()?;
    let minute: u32 = tf[1].parse().ok()?;
    let second: u32 = if tf.len() == 3 { tf[2].parse().ok()? } else { 0 };
    match meridian.as_deref() {
        None => {}
        Some("AM") => {
            if !(1..=12).contains(&hour) {
                return None;
            }
            if hour == 12 {
                hour = 0;
            }
        }
        Some("PM") => {
            if !(1..=12).contains(&hour) {
                return None;
            }
            if hour != 12 {
                hour += 12;
            }
        }
        Some(_) => return None,
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(TokKind::DateTime(Some((date, TimeLit { hour, minute, second }))))
}

/// `NORMALIZE()`: an expression written the one way Visual FoxPro writes it internally.
///
/// The reference says what changes: the text goes to upper case except inside strings,
/// abbreviated names are written out in full, `->` between an alias and a field becomes a dot,
/// the logical operators are surrounded by periods, and the spaces between terms go. Reading it
/// back as tokens and writing them out again is all of that at once, because a token knows
/// which of those it is.
pub fn normalize(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let text = |span: &Span| chars[span.start.min(chars.len())..span.end.min(chars.len())].iter().collect::<String>();
    let tokens = lex(src).tokens;
    let mut out = String::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match &token.kind {
            TokKind::Eof | TokKind::Newline => {}
            // a string keeps what is inside it and is written between quotes
            TokKind::Str(s) => {
                out.push('"');
                out.push_str(s);
                out.push('"');
            }
            TokKind::DotAnd => out.push_str(".AND."),
            TokKind::DotOr => out.push_str(".OR."),
            TokKind::DotNot => out.push_str(".NOT."),
            TokKind::Ident(name) => {
                let upper = name.to_ascii_uppercase();
                match upper.as_str() {
                    "AND" => out.push_str(".AND."),
                    "OR" => out.push_str(".OR."),
                    "NOT" => out.push_str(".NOT."),
                    _ => out.push_str(&expand_name(&upper)),
                }
            }
            // `alias->field` is the same thing as `alias.field`
            TokKind::Minus if matches!(tokens.get(i + 1).map(|t| &t.kind), Some(TokKind::Gt)) => {
                out.push('.');
                i += 1;
            }
            _ => out.push_str(&text(&token.span).to_ascii_uppercase()),
        }
        i += 1;
    }
    out
}

/// A name written out in full: Visual FoxPro lets a function be abbreviated to four letters, and
/// normalizing puts back the one it stands for.
fn expand_name(upper: &str) -> String {
    if upper.len() < 4 {
        return upper.to_string();
    }
    let mut found = None;
    for spec in crate::builtins::registry() {
        if spec.name.starts_with(upper) {
            if found.is_some() {
                return upper.to_string();
            }
            found = Some(spec.name);
        }
    }
    found.unwrap_or(upper).to_string()
}
