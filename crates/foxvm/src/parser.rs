//! Recursive-descent, error-recovering parser producing [`crate::ast`].
//!
//! Statements are line based: each logical line (see the lexer) is one statement, block
//! statements collect the lines up to their terminator keyword. Errors are reported with the
//! VFP wording where one exists, the offending line is skipped, and parsing continues.

use std::collections::HashMap;

use crate::ast::*;
use crate::bytecode::{DebugVerb, compile_flags};
use crate::diagnostics::{Diagnostic, LineMap, Span};
use crate::lexer::{TokKind, Token, lex};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutput {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn has_errors(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.is_error())
}

/// Parses a whole program: top-level code followed by PROCEDURE/FUNCTION declarations.
pub fn parse_program(src: &str) -> ParseOutput {
    parse_program_with(src, &std::collections::HashMap::new())
}

/// The same, with the header files the program may `#INCLUDE`, by name without folder or
/// extension. Reading files is the caller's to do; the parser only asks for their text.
pub fn parse_program_with(src: &str, headers: &std::collections::HashMap<String, String>) -> ParseOutput {
    let mut p = Parser::new(src, Mode::Program);
    p.headers = headers.clone();
    let program = p.program();
    ParseOutput { program, diagnostics: p.diags }
}

/// The name a `#INCLUDE` line means, without its folder or its extension.
fn header_stem(file: &str) -> String {
    let name = file.trim().trim_matches(['"', '\'']).rsplit(['/', '\\']).next().unwrap_or(file);
    match name.rfind('.') {
        Some(dot) => name[..dot].to_ascii_uppercase(),
        None => name.to_ascii_uppercase(),
    }
}

/// Parses a method body: same grammar, but PROCEDURE/FUNCTION declarations are errors.
pub fn parse_method(src: &str) -> ParseOutput {
    parse_method_with(src, &std::collections::HashMap::new(), "")
}

/// The same, with the header files the method may `#INCLUDE` and the one its own file declared.
///
/// A form and a class library each name a header file, and every method the file holds is
/// compiled with that header's constants already defined - as though the method began with an
/// `#INCLUDE` of it. `include` is that file's name without folder or extension; `""` when the
/// file named none, or when the header could not be found, which Visual FoxPro compiles through
/// as well.
pub fn parse_method_with(src: &str, headers: &std::collections::HashMap<String, String>, include: &str) -> ParseOutput {
    let mut p = Parser::new(src, Mode::Method);
    p.headers = headers.clone();
    let stem = header_stem(include);
    if let Some(text) = p.headers.get(&stem).cloned() {
        p.read_header(&stem, &text);
    }
    let program = p.program();
    ParseOutput { program, diagnostics: p.diags }
}

/// The same, for a line that has already been through macro substitution: it is read as it
/// stands. A `&name` still in it is text - the name was not a character variable - so it is not
/// held back to be read again when it runs, which would never end.
pub fn parse_expanded_line(src: &str) -> ParseOutput {
    let mut p = Parser::new(src, Mode::Method);
    p.macros_done = true;
    let program = p.program();
    ParseOutput { program, diagnostics: p.diags }
}

/// Parses the whole input as a single expression.
pub fn parse_expression(src: &str) -> Result<Expr, Vec<Diagnostic>> {
    parse_expression_in(src)
}

/// The same, for text that only exists once the program is running: what a name expression
/// came to, or what a macro put there.
pub fn parse_expression_in(src: &str) -> Result<Expr, Vec<Diagnostic>> {
    let mut p = Parser::new(src, Mode::Method);
    let expr = p.expr();
    if expr.is_ok() {
        while matches!(p.peek().kind, TokKind::Newline) {
            p.advance();
        }
        if !matches!(p.peek().kind, TokKind::Eof) {
            let t = p.peek().clone();
            let msg = format!("Syntax error: unexpected {} after expression", describe(&t));
            p.error(t.span, msg);
        }
    }
    match expr {
        Ok(e) if !has_errors(&p.diags) => Ok(e),
        _ => Err(p.diags),
    }
}

/// The words that end the file name of a copy and start a clause of its own.
const COPY_WORDS: &[&str] = &[
    "FIELDS", "FOR", "WHILE", "ALL", "NEXT", "REST", "RECORD", "TYPE", "SDF", "CSV", "DELIMITED", "WITH",
    "ON", "DATABASE", "NAME", "TO", "IN", "ADD", "ALTER", "DROP", "RENAME", "COLUMN", "AS",
    "EXCLUSIVE", "SHARED", "NOUPDATE", "VALIDATE", "DELETETABLES", "RECYCLE", "SET", "WHERE",
    "ADDITIVE", "OVERWRITE", "CONNSTRING", "DATASOURCE", "SHARED", "USERID", "PASSWORD",
    // the rest of the clauses a database command carries, which a dbc event is handed
    "DELETE", "RECOVER", "NOCONSOLE", "NOWAIT", "NOEDIT", "PRINTER",
];

/// The words that open a clause of `USE` and the commands that name a file the way it does,
/// rather than being part of the name.
const CLAUSE_WORDS: &[&str] = &[
    "ALIAS", "IN", "EXCLUSIVE", "SHARED", "AGAIN", "NOUPDATE", "ORDER", "INDEX",
    // what USE says about a view rather than about the file it names
    "ONLINE", "ADMIN", "NODATA", "NOREQUERY", "CONNSTRING",
    // what MODIFY and the rest say about how the file is opened, not about the file
    "NOEDIT", "NOWAIT", "NOMENU", "NOSHOW", "SAVE", "WINDOW", "RANGE", "AS", "ENCRYPT",
];

/// VFP keyword abbreviation: keywords of at most four letters must match exactly; longer
/// keywords match any case-insensitive prefix of at least four letters.
pub fn kw(tok: &Token, keyword: &str) -> bool {
    match &tok.kind {
        TokKind::Ident(t) => kw_text(t, keyword),
        _ => false,
    }
}

fn kw_text(ident: &str, keyword: &str) -> bool {
    if keyword.len() <= 4 {
        ident.eq_ignore_ascii_case(keyword)
    } else {
        ident.len() >= 4
            && ident.len() <= keyword.len()
            && keyword.as_bytes()[..ident.len()].eq_ignore_ascii_case(ident.as_bytes())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Program,
    Method,
}

/// Marker for "a diagnostic has already been recorded".
#[derive(Debug, Clone, Copy)]
struct PErr;

type PResult<T> = Result<T, PErr>;

/// Block terminators and the opener they belong to, for stray-keyword messages.
const TERMINATORS: &[(&str, &str)] = &[
    ("ENDIF", "IF"),
    ("ELSE", "IF"),
    ("ENDDO", "DO WHILE"),
    ("ENDFOR", "FOR"),
    ("NEXT", "FOR"),
    ("ENDCASE", "DO CASE"),
    ("CASE", "DO CASE"),
    ("OTHERWISE", "DO CASE"),
    ("ENDWITH", "WITH"),
    ("ENDTRY", "TRY"),
    ("CATCH", "TRY"),
    ("FINALLY", "TRY"),
    ("ENDPROC", "PROCEDURE"),
    ("ENDFUNC", "FUNCTION"),
    ("ENDTEXT", "TEXT"),
    ("ENDSCAN", "SCAN"),
    ("ENDDEFINE", "DEFINE CLASS"),
];

/// Commands that only tell Visual FoxPro's project manager what a program references. They
/// have no effect when the program runs, so they are read and dropped without a diagnostic.
/// How many times one constant may stand for another before it is called a loop.
const MAX_EXPANSIONS: usize = 64;

const IGNORED: &[&str] = &["EXTERNAL"];

/// Commands that ask the development environment for something a running program does not have.
/// They are read, warned about once, and dropped, because a form with a Help button is otherwise
/// perfectly runnable.
const IDE_ONLY: &[&str] = &["HELP", "ASSIST", "DOCK"];

/// Commands that put something on a Visual FoxPro screen this runtime does not have: its own
/// windows, its menus and popups, its printer. They are warned about and dropped rather than
/// failing the method, because the rest of the method is usually the part that matters - the
/// sample launcher that DEFINEs a window around a BROWSE still has a BROWSE in it.
const SCREEN_ONLY: &[&str] = &[
    "OPEN",
    "ACTIVATE", "DEACTIVATE", "SHOW", "HIDE", "ZOOM", "MOVE", "PUSH", "POP", "RESTORE", "SAVE", "EJECT", "KEYBOARD",
    "PLAY", "REPORT", "LABEL",
];

/// Commands VFP has that the FoxDev runtime does not support.
const UNSUPPORTED: &[&str] = &["CALL", "LOAD"];

/// Statement keywords the parser dispatches on (besides `UNSUPPORTED` and `TERMINATORS`).
const STATEMENT_KEYWORDS: &[&str] = &[
    "BUILD",
    "COMPILE",
    "IMPORT",
    "EXPORT",
    "LOCAL",
    "PRIVATE",
    "PUBLIC",
    "DIMENSION",
    "DECLARE",
    "LPARAMETERS",
    "PARAMETERS",
    "IF",
    "DO",
    "FOR",
    "EXIT",
    "LOOP",
    "RETURN",
    "STORE",
    "WAIT",
    "READ",
    "CLEAR",
    "RELEASE",
    "QUIT",
    "CANCEL",
    "CLOSE",
    "MODIFY",
    "BROWSE",
    "CREATE",
    "INSERT",
    "CD",
    "CHDIR",
    "RETRY",
    "SCAN",
    "LOCATE",
    "CONTINUE",
    "SEEK",
    "INDEX",
    "REINDEX",
    "TOTAL",
    "SORT",
    "DROP",
    "PACK",
    "UPDATE",
    "UNLOCK",
    "ALTER",
    "OPEN",
    "LIST",
    "DISPLAY",
    "VALIDATE",
    "ADD",
    "REMOVE",
    "FREE",
    "RENAME",
    "BEGIN",
    "END",
    "ROLLBACK",
    "SCATTER",
    "GATHER",
    "COUNT",
    "SUM",
    "AVERAGE",
    "CALCULATE",
    "REPLACE",
    "DELETE",
    "RECALL",
    "APPEND",
    "ZAP",
    "ERROR",
    "MD",
    "MKDIR",
    "RD",
    "RMDIR",
    "DIR",
    "DIRECTORY",
    "TYPE",
    "COPY",
    "RENAME",
    "WITH",
    "SET",
    "TRY",
    "THROW",
    "ERASE",
    "USE",
    "SELECT",
    "SKIP",
    "GO",
    "GOTO",
    "NODEFAULT",
    "DODEFAULT",
    "PROCEDURE",
    "FUNCTION",
    "DEFINE",
    "TEXT",
    "ON",
    "ACTIVATE",
    "DEACTIVATE",
    "SHOW",
    "HIDE",
    "PUSH",
    "POP",
    "MOVE",
    "ZOOM",
    "SIZE",
    "SAVE",
    "RESTORE",
    "MENU",
    "KEYBOARD",
    "INPUT",
    "ACCEPT",
    "GETEXPR",
    "RUN",
    "FLUSH",
    "DOEVENTS",
    "ASSERT",
    "DEBUGOUT",
    "SUSPEND",
    "RESUME",
    "FIND",
    "BLANK",
    "EDIT",
    "CHANGE",
    "SCROLL",
    "JOIN",
    "MOUSE",
    "REPORT",
    "LABEL",
    "EJECT",
    "PRINTJOB",
];

/// The words that say how a window looks and what it may do. None of them changes what a
/// program can see, so they are kept as they were written and reported back by `WBORDER()`
/// and the rest.
const WINDOW_WORDS: &[&str] = &[
    "DOUBLE", "PANEL", "NONE", "SYSTEM", "CLOSE", "NOCLOSE", "FLOAT", "NOFLOAT", "GROW", "NOGROW",
    "MINIMIZE", "NOMINIMIZE", "ZOOM", "NOZOOM", "SHADOW", "HALFHEIGHT", "MAX", "MIN", "NORM",
    "SAME", "PREFERENCE", "NAME", "AS", "DESKTOP", "TOP", "BOTTOM",
];

/// The words that say how an `@` line is drawn, and which control a GET asks for.
const AT_WORDS: &[&str] = &[
    "DOUBLE", "PANEL", "UP", "DOWN", "LEFT", "RIGHT", "ENABLE", "DISABLE", "SCROLL", "NOMODIFY",
];

/// The words `DEFINE` takes when it is defining a menu rather than a class or a window.
const MENU_DEFINES: &[&str] = &["MENU", "PAD", "POPUP", "BAR"];

/// The things `ON EXIT` can be given.
const MENU_KINDS: &[&str] = &["MENU", "PAD", "POPUP", "BAR"];

/// What `DEFINE PAD` and `DEFINE BAR` were told, gathered as their clauses are read.
#[derive(Default)]
struct MenuClauses {
    prompt: Option<Expr>,
    key: Option<Expr>,
    message: Option<Expr>,
    skip: String,
}

/// Whether `expr [ 1 ]` could mean a subscript. A literal cannot be indexed, so a `[...]`
/// after one stays the string the lexer read.
fn is_indexable(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Var(_)
            | ExprKind::Member { .. }
            | ExprKind::Index { .. }
            | ExprKind::Call { .. }
            | ExprKind::MethodCall { .. }
            | ExprKind::This
            | ExprKind::ThisForm
            | ExprKind::ThisFormSet
            | ExprKind::Screen
            | ExprKind::WithRef
    )
}

/// Whether a `(` after this expression is a call of the value it yields.
///
/// Only after a subscript or a call, which is to say only where the expression already ended in
/// a bracket. Everywhere else a `(` keeps whatever meaning it has: `oForm.Handler(x)` is a call
/// of the method `Handler` in the product and stays one here, so a lambda kept in a property is
/// called by reading it into a name first or by putting the whole reference in brackets. Where
/// the product already has an answer, the product wins - the rule this whole milestone follows.
fn is_callable_value(kind: &ExprKind) -> bool {
    matches!(kind, ExprKind::Index { .. } | ExprKind::Call { .. } | ExprKind::CallValue { .. } | ExprKind::MethodCall { .. })
}

fn describe(tok: &Token) -> String {
    match &tok.kind {
        TokKind::Ident(t) => format!("'{t}'"),
        TokKind::Num(n, ..) => format!("'{n}'"),
        TokKind::Str(s) => format!("string \"{s}\""),
        TokKind::Date(_) | TokKind::DateTime(_) => "date constant".into(),
        TokKind::MacroDate => "date constant built from macros".into(),
        TokKind::True => "'.T.'".into(),
        TokKind::False => "'.F.'".into(),
        TokKind::Null => "'.NULL.'".into(),
        TokKind::DotAnd => "'.AND.'".into(),
        TokKind::DotOr => "'.OR.'".into(),
        TokKind::DotNot => "'.NOT.'".into(),
        TokKind::RawText(_) => "text block".into(),
        TokKind::TextMark(double) => if *double { "'\\\\'" } else { "'\\'" }.into(),
        TokKind::Newline => "end of line".into(),
        TokKind::Eof => "end of file".into(),
        TokKind::Plus => "'+'".into(),
        TokKind::Minus => "'-'".into(),
        TokKind::Star => "'*'".into(),
        TokKind::Slash => "'/'".into(),
        TokKind::Percent => "'%'".into(),
        TokKind::Caret => "'^'".into(),
        TokKind::Eq => "'='".into(),
        TokKind::EqEq => "'=='".into(),
        TokKind::Ne => "'<>'".into(),
        TokKind::Lt => "'<'".into(),
        TokKind::Le => "'<='".into(),
        TokKind::Gt => "'>'".into(),
        TokKind::Ge => "'>='".into(),
        TokKind::Dollar => "'$'".into(),
        TokKind::LParen => "'('".into(),
        TokKind::RParen => "')'".into(),
        TokKind::LBracket => "'['".into(),
        TokKind::RBracket => "']'".into(),
        TokKind::Comma => "','".into(),
        TokKind::Dot => "'.'".into(),
        TokKind::Colon => "':'".into(),
        TokKind::At => "'@'".into(),
        TokKind::Amp => "'&'".into(),
        TokKind::Backslash => "'\\\\'".into(),
        TokKind::Semi => "';'".into(),
        TokKind::Bang => "'!'".into(),
        TokKind::Question => "'?'".into(),
        TokKind::DoubleQuestion => "'??'".into(),
        TokKind::Hash => "'#'".into(),
    }
}

struct Parser {
    src: Vec<char>,
    /// The header files this program may include, by name without folder or extension.
    headers: std::collections::HashMap<String, String>,
    /// The headers being read right now, innermost last. A header that includes itself, or one
    /// that includes it back, is read once: a second `#INCLUDE` of a header already open brings
    /// nothing the first did not, and following it would never end.
    headers_open: Vec<String>,
    toks: Vec<Token>,
    /// Parallel to `toks`: false for tokens produced by `#DEFINE` expansion (never re-expanded).
    expandable: Vec<bool>,
    pos: usize,
    diags: Vec<Diagnostic>,
    map: LineMap,
    defines: HashMap<String, Vec<Token>>,
    mode: Mode,
    /// Terminator sets of the enclosing blocks, innermost last.
    term_stack: Vec<&'static [&'static str]>,
    /// How many SELECT-SQL statements are being read. IN is a comparison only inside one.
    in_query: usize,
    /// True for a line that has already been through macro substitution, so no part of it is
    /// held back to be read again when it runs.
    macros_done: bool,
}

impl Parser {
    fn new(src: &str, mode: Mode) -> Self {
        let lexed = lex(src);
        let n = lexed.tokens.len();
        Parser {
            src: src.chars().collect(),
            headers: std::collections::HashMap::new(),
            headers_open: Vec::new(),
            toks: lexed.tokens,
            expandable: vec![true; n],
            pos: 0,
            diags: lexed.diagnostics,
            map: LineMap::new(src),
            defines: HashMap::new(),
            mode,
            term_stack: Vec::new(),

            in_query: 0,
            macros_done: false,
        }
    }

    // ----- token access ---------------------------------------------------------------------

    /// Expands a `#DEFINE` constant at token index `idx`, if any.
    ///
    /// What a constant stands for can name constants of its own, and Visual FoxPro expands those
    /// too - the substitution is textual and it keeps going. The Solution samples rely on it:
    ///
    /// ```text
    /// #DEFINE MSG_LOC "The table must contain 3 character fields:" + CHR(13) + ;
    ///    CHR(13) + FIELD1_LOC + ;
    ///    CHR(13) + FIELD2_LOC
    /// ```
    ///
    /// Left unexpanded, FIELD1_LOC reaches the program as a name and it stops with "Variable
    /// 'FIELD1_LOC' is not found".
    fn expand(&mut self, idx: usize) {
        // a constant that names itself would go round for ever, so the substitution is bounded
        // and says so rather than hanging
        for round in 0.. {
            if idx >= self.toks.len() || !self.expandable[idx] {
                return;
            }
            let Some(name) = self.toks[idx].ident().map(|s| s.to_ascii_uppercase()) else {
                return;
            };
            let Some(body) = self.defines.get(&name) else {
                return;
            };
            let at = &self.toks[idx];
            let (span, line) = (at.span, at.line);
            if round >= MAX_EXPANSIONS {
                self.expandable[idx] = false;
                self.error(span, format!("Constant '{name}' is defined in terms of itself"));
                return;
            }
            let replacement: Vec<Token> = body.iter().map(|t| Token { kind: t.kind.clone(), span, line, bracketed: false }).collect();
            let count = replacement.len();
            self.toks.splice(idx..idx + 1, replacement);
            self.expandable.splice(idx..idx + 1, std::iter::repeat_n(true, count));
        }
    }

    fn peek(&mut self) -> &Token {
        self.expand(self.pos);
        &self.toks[self.pos]
    }

    fn peek_at(&mut self, n: usize) -> &Token {
        for i in 0..=n {
            self.expand(self.pos + i);
        }
        let idx = (self.pos + n).min(self.toks.len() - 1);
        &self.toks[idx]
    }

    fn peek_kind(&mut self) -> TokKind {
        self.peek().kind.clone()
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if !matches!(tok.kind, TokKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    /// Next token without `#DEFINE` expansion (used inside directives).
    fn advance_raw(&mut self) -> Token {
        let tok = self.toks[self.pos].clone();
        if !matches!(tok.kind, TokKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn at_eol(&mut self) -> bool {
        self.peek().is_newline()
    }

    fn is(&mut self, kind: &TokKind) -> bool {
        &self.peek().kind == kind
    }

    fn eat(&mut self, kind: &TokKind) -> bool {
        if self.is(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, keyword: &str) -> bool {
        if kw(self.peek(), keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_kw(&mut self, keyword: &str) -> bool {
        kw(self.peek(), keyword)
    }

    /// Span of the previously consumed token.
    fn prev_span(&self) -> Span {
        if self.pos == 0 { Span::default() } else { self.toks[self.pos - 1].span }
    }

    fn error(&mut self, span: Span, message: impl Into<String>) -> PErr {
        self.diags.push(self.map.error(span, message));
        PErr
    }

    fn warning(&mut self, span: Span, message: impl Into<String>) {
        self.diags.push(self.map.warning(span, message));
    }

    fn error_here(&mut self, message: impl Into<String>) -> PErr {
        let span = self.peek().span;
        self.error(span, message)
    }

    fn expected(&mut self, what: &str) -> PErr {
        let t = self.peek().clone();
        let msg = format!("Syntax error: expected {what}, found {}", describe(&t));
        self.error(t.span, msg)
    }

    fn expect(&mut self, kind: &TokKind, what: &str) -> PResult<Token> {
        if self.is(kind) { Ok(self.advance()) } else { Err(self.expected(what)) }
    }

    fn expect_ident(&mut self, what: &str) -> PResult<Name> {
        match self.peek_kind() {
            TokKind::Ident(t) => {
                let tok = self.advance();
                Ok(Name::new(t, tok.span))
            }
            _ => Err(self.expected(what)),
        }
    }

    /// A name a command is given, and everything Visual FoxPro lets a program write in its
    /// place.
    ///
    /// Almost anywhere a name is expected - an alias, a table, a cursor, a field, a column, a
    /// variable - a program may put an expression in parentheses instead, and the string it
    /// comes to is the name; a quoted name is a name as well. That rule lives here, and every
    /// command that wants a name asks here for one rather than for a word of its own.
    fn name_ref(&mut self, what: &str) -> PResult<NameRef> {
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(NameRef::Computed(e));
        }
        let span = self.peek().span;
        if let TokKind::Str(text) = self.peek_kind() {
            self.advance();
            return Ok(NameRef::Named(Name::new(text, span)));
        }
        Ok(NameRef::Named(self.expect_ident(what)?))
    }

    /// The work area an `IN` clause names: its number, or the alias open in it.
    fn area_ref(&mut self) -> PResult<NameRef> {
        if matches!(self.peek_kind(), TokKind::Num(..)) {
            return Ok(NameRef::Computed(self.expr()?));
        }
        // `IN SELECT('orders')`: a name with a bracket after it is a call that answers with
        // the work area, not an alias called SELECT
        if matches!(self.peek_kind(), TokKind::Ident(_)) && matches!(self.peek_at(1).kind, TokKind::LParen) {
            return Ok(NameRef::Computed(self.expr()?));
        }
        self.name_ref("alias")
    }

    /// Where a command puts something.
    ///
    /// The name-expression rule reaches the writing side too: `STORE x TO (cName)` writes to
    /// whatever the string names, which may be a variable or a property some way down an
    /// object, so the target is only worked out when the statement runs.
    fn name_target(&mut self) -> PResult<Target> {
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(Target::ByName(e));
        }
        Ok(Target::Written(self.target()?))
    }

    /// Skips to and over the end of the current logical line.
    fn skip_line(&mut self) {
        while !self.at_eol() {
            self.advance();
        }
        if self.is(&TokKind::Newline) {
            self.advance();
        }
    }

    /// Reports leftover tokens on the line (if any) and skips them.
    fn end_of_line(&mut self) {
        if !self.at_eol() {
            // `ENDIF NOT .lFound` and `NEXT i` are how FoxPro programmers say what a block closed,
            // and FoxPro reads nothing after the terminator itself, so neither does this
            if self.prev_is_terminator() {
                self.skip_line();
                return;
            }
            self.expected("end of line");
            self.skip_line();
        } else if self.is(&TokKind::Newline) {
            self.advance();
        }
    }

    /// Whether the token just consumed closes a block.
    fn prev_is_terminator(&self) -> bool {
        const CLOSERS: &[&str] = &[
            "ENDIF", "ELSE", "ENDDO", "ENDFOR", "NEXT", "ENDCASE", "ENDSCAN", "ENDWITH", "ENDTRY", "ENDPROC",
            "ENDFUNC", "ENDDEFINE", "ENDTEXT", "OTHERWISE",
        ];
        let Some(prev) = self.toks.get(self.pos.wrapping_sub(1)) else { return false };
        prev.ident().is_some_and(|t| CLOSERS.iter().any(|c| kw_text(t, c)))
    }

    fn source_text(&self, start: usize, end: usize) -> String {
        let end = end.min(self.src.len());
        if start >= end {
            return String::new();
        }
        self.src[start..end].iter().collect::<String>().trim().to_string()
    }

    /// Refuses when the rest of the line still has a macro in it. A command that reads words
    /// rather than an expression - `SET PATH TO c:\a;c:\b`, `FIND Smith` - has not got its
    /// words until the macro is put in, so the whole line is left to be read when it runs
    /// rather than guessed at now.
    fn no_macro_ahead(&mut self) -> PResult<()> {
        if self.line_has_macro(self.pos) {
            let span = self.peek().span;
            return Err(self.error(span, "the words of this command are only there once its macro is read"));
        }
        Ok(())
    }

    /// Source text of the remaining tokens on the line (consumed).
    fn rest_of_line_text(&mut self) -> Option<String> {
        if self.at_eol() {
            return None;
        }
        let start = self.peek().span.start;
        let mut end = start;
        while !self.at_eol() {
            end = self.advance().span.end;
        }
        Some(self.source_text(start, end))
    }

    // ----- program structure --------------------------------------------------------------

    fn program(&mut self) -> Program {
        let top: &'static [&'static str] = match self.mode {
            Mode::Program => &["PROCEDURE", "FUNCTION", "DEFINE"],
            Mode::Method => &[],
        };
        let (body, mut term) = self.block(top);
        let mut procs = Vec::new();
        let mut classes = Vec::new();
        while let Some(t) = term {
            if t == "DEFINE" {
                let (decl, next) = self.class_decl();
                if let Some(d) = decl {
                    classes.push(d);
                }
                term = next;
            } else {
                let (decl, next) = self.proc_decl();
                procs.push(decl);
                term = next;
            }
        }
        Program { body, procs, classes }
    }

    /// After an ENDPROC/ENDFUNC/ENDDEFINE: skips blank lines and reports anything that is not the
    /// start of another top-level declaration. `after_class` sharpens the message for the
    /// `PROCEDURE ... DEFINE CLASS ... ENDDEFINE ... ENDPROC` shape.
    fn next_top_decl(&mut self, after_class: bool) -> Option<&'static str> {
        loop {
            while self.eat(&TokKind::Newline) {}
            if self.is(&TokKind::Eof) {
                return None;
            }
            if self.is_kw("PROCEDURE") {
                return Some("PROCEDURE");
            }
            if self.is_kw("FUNCTION") {
                return Some("FUNCTION");
            }
            if self.at_define_class() {
                return Some("DEFINE");
            }
            if after_class && (self.is_kw("ENDPROC") || self.is_kw("ENDFUNC")) {
                let span = self.peek().span;
                self.error(span, "DEFINE CLASS is not allowed inside a PROCEDURE or FUNCTION");
                self.skip_line();
                continue;
            }
            if after_class {
                self.error_here("Statements after ENDDEFINE must belong to a PROCEDURE, FUNCTION or DEFINE CLASS");
            } else {
                self.error_here("Statements after ENDPROC/ENDFUNC must belong to a PROCEDURE or FUNCTION");
            }
            loop {
                if self.is(&TokKind::Eof) || self.is_kw("PROCEDURE") || self.is_kw("FUNCTION") {
                    break;
                }
                if self.at_define_class() {
                    break;
                }
                self.skip_line();
            }
        }
    }

    /// The current token starts a `DEFINE CLASS` line.
    fn at_define_class(&mut self) -> bool {
        self.is_kw("DEFINE") && kw(self.peek_at(1), "CLASS")
    }

    /// Parses `PROCEDURE name ... ENDPROC` at the current position; returns the declaration and
    /// whether another PROCEDURE/FUNCTION follows.
    fn proc_decl(&mut self) -> (ProcDecl, Option<&'static str>) {
        let head = self.advance();
        let kind = if kw(&head, "FUNCTION") { ProcKind::Function } else { ProcKind::Procedure };
        let name = match self.expect_ident("procedure name") {
            Ok(n) => n,
            Err(_) => {
                self.skip_line_keep_newline();
                Name::new("", head.span)
            }
        };
        let mut params = Vec::new();
        if self.eat(&TokKind::LParen) {
            match self.param_list(&TokKind::RParen) {
                Ok(p) => params = p,
                Err(_) => self.skip_line_keep_newline(),
            }
        }
        self.end_of_line();
        let (body, term) = self.block(&["ENDPROC", "ENDFUNC", "PROCEDURE", "FUNCTION", "DEFINE"]);
        let mut next = None;
        match term {
            Some("ENDPROC") | Some("ENDFUNC") => {
                self.advance();
                self.end_of_line();
                next = self.next_top_decl(false);
            }
            Some(k) => next = Some(k),
            None => {}
        }
        let span = head.span.to(self.prev_span());
        (ProcDecl { kind, name, params, body, line: head.line, span }, next)
    }

    // ----- DEFINE CLASS ---------------------------------------------------------------------

    /// The current token is `ENDDEFINE` (five letters at least, so `ENDD` stays ambiguous with
    /// ENDDO the way VFP leaves it).
    fn is_enddefine(&mut self) -> bool {
        self.peek().ident().is_some_and(|t| t.len() >= 5 && kw_text(t, "ENDDEFINE"))
    }

    /// `DEFINE CLASS name AS parent [OF lib] [OLEPUBLIC] ... ENDDEFINE`, positioned at DEFINE.
    /// Returns the declaration and the keyword that starts the next top-level declaration.
    fn class_decl(&mut self) -> (Option<ClassDecl>, Option<&'static str>) {
        let head = self.advance(); // DEFINE
        self.advance(); // CLASS
        let name = match self.expect_ident("class name") {
            Ok(n) => n,
            Err(_) => {
                self.skip_line_keep_newline();
                Name::new("", head.span)
            }
        };
        let mut parent = Name::new("Custom", name.span);
        let mut class_lib = None;
        if self.eat_kw("AS") {
            match self.expect_ident("parent class name") {
                Ok(p) => parent = p,
                Err(_) => self.skip_line_keep_newline(),
            }
        } else if !self.at_eol() {
            self.expected("'AS'");
            self.skip_line_keep_newline();
        }
        if self.is_kw("OF") {
            let of = self.advance();
            let lib = self.words_until_olepublic();
            let span = of.span.to(self.prev_span());
            self.warning(span, format!("OF {lib} is ignored: the class is taken from this program"));
            class_lib = Some(lib);
        }
        self.eat_kw("OLEPUBLIC");
        self.end_of_line();

        let mut properties: Vec<ClassProperty> = Vec::new();
        let mut members: Vec<ClassMember> = Vec::new();
        let mut procs: Vec<ProcDecl> = Vec::new();
        loop {
            while self.eat(&TokKind::Newline) {}
            if self.is(&TokKind::Eof) {
                self.error(head.span, "DEFINE CLASS without matching ENDDEFINE");
                break;
            }
            if self.is_enddefine() {
                self.advance();
                self.end_of_line();
                break;
            }
            if self.is(&TokKind::Hash) {
                let _ = self.directive();
                self.end_of_line();
                continue;
            }
            // PROTECTED / HIDDEN are parsed and otherwise ignored.
            let restricted = if self.is_kw("PROTECTED") || self.is_kw("HIDDEN") {
                self.advance();
                true
            } else {
                false
            };
            if self.is_kw("PROCEDURE") || self.is_kw("FUNCTION") {
                procs.push(self.class_method());
                continue;
            }
            if self.is_kw("ADD") && kw(self.peek_at(1), "OBJECT") {
                if let Some(m) = self.class_member() {
                    members.push(m);
                }
                continue;
            }
            // `DIMENSION aRGB[3]`: an array property, every element starting at .F. as in VFP
            if self.is_kw("DIMENSION") {
                self.advance();
                for decl in self.array_decls() {
                    properties.push(decl);
                }
                self.end_of_line();
                continue;
            }
            if self.is_kw("IMPLEMENTS") {
                let span = self.peek().span;
                self.warning(span, "IMPLEMENTS in a class definition is ignored by the FoxDev runtime");
                self.skip_line();
                continue;
            }
            let Ok(pname) = self.class_property_name("a property, ADD OBJECT, PROCEDURE or ENDDEFINE") else {
                self.skip_line();
                continue;
            };
            // `wbaSearchOrder[1] = "WIZARDS"`: one element of an array the body dimensioned
            if self.is(&TokKind::LBracket) || self.is(&TokKind::LParen) {
                let close = if self.is(&TokKind::LBracket) { TokKind::RBracket } else { TokKind::RParen };
                self.advance();
                let mut subs = Vec::new();
                while let Ok(e) = self.expr() {
                    subs.push(e);
                    if !self.eat(&TokKind::Comma) {
                        break;
                    }
                }
                if !self.eat(&close) || !self.eat(&TokKind::Eq) {
                    self.expected("'='");
                    self.skip_line();
                    continue;
                }
                let value = self.expr_or_recover();
                properties.push(ClassProperty { name: pname, value, dim: None, index: Some(subs) });
                self.end_of_line();
                continue;
            }
            if self.eat(&TokKind::Eq) {
                let value = self.expr_or_recover();
                properties.push(ClassProperty { name: pname, value, dim: None, index: None });
                self.end_of_line();
            } else if restricted {
                // `PROTECTED a, b` declares properties without a value; VFP starts them at .F.
                let mut names = vec![pname];
                loop {
                    if !self.eat(&TokKind::Comma) {
                        break;
                    }
                    match self.expect_ident("property name") {
                        Ok(n) => names.push(n),
                        Err(_) => break,
                    }
                }
                for n in names {
                    let span = n.span;
                    properties.push(ClassProperty { name: n, value: Expr::new(ExprKind::Bool(false), span), dim: None, index: None });
                }
                self.end_of_line();
            } else {
                self.expected("'='");
                self.skip_line();
            }
        }
        let span = head.span.to(self.prev_span());
        let decl = ClassDecl { name, parent, class_lib, properties, members, procs, line: head.line, span };
        (Some(decl), self.next_top_decl(true))
    }

    /// Source text of the rest of the line, stopping before OLEPUBLIC (used for `OF classlib`).
    fn words_until_olepublic(&mut self) -> String {
        let start = self.peek().span.start;
        let mut end = start;
        while !self.at_eol() && !self.is_kw("OLEPUBLIC") {
            end = self.advance().span.end;
        }
        self.source_text(start, end)
    }

    /// `DIMENSION a[3], b[2]` inside a class body: each name becomes an array property. Only
    /// the first subscript is kept, since a class array property here is one-dimensional.
    fn array_decls(&mut self) -> Vec<ClassProperty> {
        let mut out = Vec::new();
        loop {
            let Ok(name) = self.expect_ident("array name") else {
                self.skip_line();
                break;
            };
            let span = name.span;
            let mut dim = None;
            if self.eat(&TokKind::LBracket) || self.eat(&TokKind::LParen) {
                dim = self.expr().ok();
                while !self.at_eol() && !self.is(&TokKind::RBracket) && !self.is(&TokKind::RParen) {
                    self.advance();
                }
                let _ = self.eat(&TokKind::RBracket) || self.eat(&TokKind::RParen);
            }
            out.push(ClassProperty { name, value: Expr::new(ExprKind::Bool(false), span), dim, index: None });
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        out
    }

    /// A property name in a class body or a `WITH` list. VFP allows a dotted path so a member
    /// of the object being added can be set too: `ADD OBJECT pgf AS pageframe WITH Page1.Caption = "x"`.
    fn class_property_name(&mut self, what: &str) -> PResult<Name> {
        let first = self.expect_ident(what)?;
        let mut text = first.text.clone();
        let mut span = first.span;
        while matches!(self.peek().kind, TokKind::Dot) {
            self.advance();
            let part = self.expect_ident("member name")?;
            text.push('.');
            text.push_str(&part.text);
            span = span.to(part.span);
        }
        Ok(Name::new(text, span))
    }

    /// `ADD OBJECT [PROTECTED] name AS class [OF lib] [NOINIT] [WITH prop = expr, ...]`.
    fn class_member(&mut self) -> Option<ClassMember> {
        self.advance(); // ADD
        self.advance(); // OBJECT
        if self.is_kw("PROTECTED") || self.is_kw("HIDDEN") {
            self.advance();
        }
        let name = match self.expect_ident("object name") {
            Ok(n) => n,
            Err(_) => {
                self.skip_line();
                return None;
            }
        };
        if !self.eat_kw("AS") {
            self.expected("'AS'");
            self.skip_line();
            return None;
        }
        let class = match self.expect_ident("class name") {
            Ok(n) => n,
            Err(_) => {
                self.skip_line();
                return None;
            }
        };
        if self.is_kw("OF") {
            let of = self.advance();
            while !self.at_eol() && !self.is_kw("NOINIT") && !self.is_kw("WITH") {
                self.advance();
            }
            let span = of.span.to(self.prev_span());
            self.warning(span, "OF is ignored: the class is taken from this program");
        }
        let mut noinit = self.eat_kw("NOINIT");
        let mut properties = Vec::new();
        if self.eat_kw("WITH") {
            loop {
                let Ok(pname) = self.class_property_name("property name") else {
                    self.skip_line_keep_newline();
                    break;
                };
                if !self.eat(&TokKind::Eq) {
                    self.expected("'='");
                    self.skip_line_keep_newline();
                    break;
                }
                let value = self.expr_or_recover();
                properties.push(ClassProperty { name: pname, value, dim: None, index: None });
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        if !noinit {
            noinit = self.eat_kw("NOINIT");
        }
        self.end_of_line();
        Some(ClassMember { name, class, noinit, properties })
    }

    /// A method inside a class body; its name may be dotted (`image1.Click`).
    fn class_method(&mut self) -> ProcDecl {
        let head = self.advance();
        let kind = if kw(&head, "FUNCTION") { ProcKind::Function } else { ProcKind::Procedure };
        let name = match self.dotted_name() {
            Some(n) => n,
            None => {
                self.skip_line_keep_newline();
                Name::new("", head.span)
            }
        };
        let mut params = Vec::new();
        if self.eat(&TokKind::LParen) {
            match self.param_list(&TokKind::RParen) {
                Ok(p) => params = p,
                Err(_) => self.skip_line_keep_newline(),
            }
        }
        self.skip_as_type();
        self.end_of_line();
        let (body, term) =
            self.block(&["ENDPROC", "ENDFUNC", "PROCEDURE", "FUNCTION", "ENDDEFINE", "PROTECTED", "HIDDEN"]);
        if matches!(term, Some("ENDPROC") | Some("ENDFUNC")) {
            self.advance();
            self.end_of_line();
        }
        let span = head.span.to(self.prev_span());
        ProcDecl { kind, name, params, body, line: head.line, span }
    }

    /// `name` or `name.member`.
    fn dotted_name(&mut self) -> Option<Name> {
        let first = self.expect_ident("procedure name").ok()?;
        let mut text = first.text;
        let mut span = first.span;
        while self.is(&TokKind::Dot) {
            self.advance();
            let Ok(part) = self.expect_ident("member name") else { break };
            text.push('.');
            text.push_str(&part.text);
            span = span.to(part.span);
        }
        Some(Name::new(text, span))
    }

    /// `a, b AS type, c` up to (and consuming) `close` or end of line.
    /// Skips a leading `m.` where a variable is being named. It says the name is a memory
    /// variable, which in a declaration it could not fail to be.
    fn eat_memvar_prefix(&mut self) {
        if self.is_kw("M") && matches!(self.peek_at(1).kind, TokKind::Dot) {
            self.advance();
            self.advance();
        }
    }

    fn param_list(&mut self, close: &TokKind) -> PResult<Vec<Param>> {
        let mut params = Vec::new();
        if self.eat(close) {
            return Ok(params);
        }
        loop {
            self.eat_memvar_prefix();
            let name = self.expect_ident("parameter name")?;
            params.push(Param { name });
            self.skip_as_type();
            if self.eat(&TokKind::Comma) {
                continue;
            }
            if close == &TokKind::Newline {
                self.eat_list_close();
                if self.at_eol() {
                    return Ok(params);
                }
                return Err(self.expected("',' or end of line"));
            }
            self.expect(close, "')'")?;
            return Ok(params);
        }
    }

    /// Skips an optional `AS type [OF classlib]` clause.
    fn skip_as_type(&mut self) {
        if self.is_kw("AS") {
            self.advance();
            while !self.at_eol() && !self.is(&TokKind::Comma) && !self.is(&TokKind::RParen) {
                self.advance();
            }
        }
    }

    // ----- blocks ---------------------------------------------------------------------------

    /// Parses statements until a keyword from `terms` (returned, not consumed), a terminator of
    /// an enclosing block (returns `None`, not consumed) or end of file (`None`).
    fn block(&mut self, terms: &'static [&'static str]) -> (Block, Option<&'static str>) {
        self.term_stack.push(terms);
        let mut stmts = Vec::new();
        let result = loop {
            while self.eat(&TokKind::Newline) {}
            if self.is(&TokKind::Eof) {
                break None;
            }
            if let Some(ident) = self.peek().ident().map(str::to_string) {
                // DEFINE only ends a block when it opens a class; `DEFINE WINDOW` is a statement.
                if let Some((depth, term)) = self.find_terminator(&ident)
                    && (term != "DEFINE" || self.at_define_class())
                {
                    if depth + 1 == self.term_stack.len() {
                        break Some(term);
                    }
                    // A class always belongs to the top level; report it where it stands.
                    if term != "DEFINE" {
                        break None;
                    }
                }
                if let Some((term, opener)) = TERMINATORS.iter().find(|(t, _)| kw_text(&ident, t)) {
                    let span = self.peek().span;
                    self.error(span, format!("{term} without matching {opener}"));
                    self.skip_line();
                    continue;
                }
            }
            if let Some(stmt) = self.statement() {
                stmts.push(stmt);
            }
        };
        self.term_stack.pop();
        (Block { stmts }, result)
    }

    /// True when the line beginning at `start` has a macro in it - a `&`, or a date constant
    /// built out of macros. Either one means the line's text is not all there until it runs.
    fn line_has_macro(&self, start: usize) -> bool {
        let mut at = start;
        while at < self.toks.len() && !self.toks[at].is_newline() {
            match &self.toks[at].kind {
                // `&` glued to a name is a macro; standing alone it is something else
                TokKind::Amp => {
                    if self.toks.get(at + 1).is_some_and(|t| {
                        matches!(t.kind, TokKind::Ident(_)) && t.span.start == self.toks[at].span.end
                    }) {
                        return true;
                    }
                }
                TokKind::MacroDate => return true,
                TokKind::Str(text) if Self::has_macro_in_text(text) => return true,
                _ => {}
            }
            at += 1;
        }
        false
    }

    /// True when the line beginning at `start` has a macro inside one of its strings.
    ///
    /// Everywhere else a macro can stand, this parser reads it where it stands. Inside a string
    /// it cannot: the product puts the text into the line and then reads the line, so what the
    /// macro stands for is not the value of a string but part of the source. `? "&lcScope"`
    /// prints what lcScope holds, and a macro holding a quote closes the string it sits in. So a
    /// line with one has not been read until it runs, however well it parsed.
    fn line_has_macro_in_string(&self, start: usize) -> bool {
        let mut at = start;
        while at < self.toks.len() && !self.toks[at].is_newline() {
            if let TokKind::Str(text) = &self.toks[at].kind
                && Self::has_macro_in_text(text)
            {
                return true;
            }
            at += 1;
        }
        false
    }

    /// The statement starting at `start` kept as source text to be read when it runs, if there
    /// is a macro in the line that opens it.
    ///
    /// A command that got as far as opening a block has read its body and its terminator on the
    /// way, and keeps all of it: the line that opens a block cannot be run on its own. One that
    /// stopped inside its own line keeps just that line.
    fn macro_stmt(&mut self, start: usize) -> Option<StmtKind> {
        if self.macros_done || !self.line_has_macro(start) {
            return None;
        }
        let mut eol = start;
        while eol < self.toks.len() && !self.toks[eol].is_newline() {
            eol += 1;
        }
        if self.pos <= eol {
            self.pos = start;
            self.skip_line_keep_newline();
        }
        let from = self.toks[start].span.start;
        Some(StmtKind::MacroText(self.source_text(from, self.prev_span().end)))
    }

    /// True when a piece of text has a `&` glued to the start of a name in it, which is what a
    /// macro looks like wherever it stands.
    fn has_macro_in_text(text: &str) -> bool {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '&' && chars.peek().is_some_and(|n| n.is_ascii_alphabetic() || *n == '_') {
                return true;
            }
        }
        false
    }

    /// Finds which enclosing block (index into `term_stack`) the identifier terminates.
    fn find_terminator(&self, ident: &str) -> Option<(usize, &'static str)> {
        for (depth, terms) in self.term_stack.iter().enumerate().rev() {
            if let Some(t) = terms.iter().find(|t| kw_text(ident, t)) {
                return Some((depth, t));
            }
        }
        None
    }

    /// Parses a block and its terminator; reports a missing terminator at `open`.
    fn block_until(
        &mut self,
        terms: &'static [&'static str],
        open: &Token,
        what: &str,
    ) -> (Block, Option<&'static str>) {
        let (block, term) = self.block(terms);
        if term.is_none() {
            self.error(open.span, format!("{what} without matching {}", terms[0]));
        }
        (block, term)
    }

    // ----- statements -----------------------------------------------------------------------

    fn statement(&mut self) -> Option<Stmt> {
        let first = self.peek().clone();
        let start = self.pos;
        let diags = self.diags.len();
        let mut result = self.statement_kind(&first);
        // Visual FoxPro puts the text a macro stands for into the line before it reads the
        // line at all. This parser reads a macro inside an expression as it stands, which is
        // the same thing and faster; a macro anywhere else - the name of a field, of a table,
        // a whole clause list - cannot be read until it is known, so a command that will not
        // parse and has one in it is left to be read when it runs.
        //
        // `SCAN &lcScope` is the same thing said differently: it reads as far as its macro and
        // then complains, having taken the whole block on the way. The complaint is what says
        // the line was not understood, so a statement that came back with one counts too.
        // A macro inside a string is the one place a line that parsed is still not read: what
        // the macro stands for goes into the source, not into the value of the string.
        let failed = result.is_err()
            || self.diags[diags..].iter().any(|d| d.is_error())
            || (!self.macros_done && self.line_has_macro_in_string(start));
        if failed
            && matches!(result, Err(_) | Ok(Some(_)))
            && let Some(kind) = self.macro_stmt(start)
        {
            self.diags.truncate(diags);
            result = Ok(Some(kind));
        }
        match result {
            Ok(Some(kind)) => {
                let span = first.span.to(self.prev_span());
                // A block whose terminator was missing stops at an enclosing block's
                // terminator; leave that token for the enclosing block.
                let is_block = matches!(
                    kind,
                    StmtKind::If { .. }
                        | StmtKind::DoCase { .. }
                        | StmtKind::DoWhile { .. }
                        | StmtKind::For { .. }
                        | StmtKind::ForEach { .. }
                        | StmtKind::With { .. }
                        | StmtKind::Try { .. }
                        | StmtKind::Scan { .. }
                );
                let outer_term =
                    self.peek().ident().map(str::to_string).is_some_and(|t| self.find_terminator(&t).is_some());
                // a statement that read as far as a macro has not read the line: what the
                // macro stands for is part of it, and only known when it runs
                if !self.at_eol()
                    && !(is_block && outer_term)
                    && let Some(macro_kind) = self.macro_stmt(start)
                {
                    self.diags.truncate(diags);
                    return Some(Stmt { kind: macro_kind, line: first.line, span });
                }
                if !(is_block && outer_term) {
                    self.end_of_line();
                }
                Some(Stmt { kind, line: first.line, span })
            }
            Ok(None) => {
                // Skipped statement (unsupported command, PRIVATE ALL, ...).
                if self.pos == start {
                    self.skip_line();
                } else {
                    self.end_of_line();
                }
                None
            }
            Err(PErr) => {
                self.skip_line();
                None
            }
        }
    }

    fn statement_kind(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        match &first.kind {
            TokKind::Hash => return self.directive(),
            TokKind::Question | TokKind::DoubleQuestion => {
                let newline = matches!(first.kind, TokKind::Question);
                self.advance();
                let mut items = Vec::new();
                if !self.at_eol() {
                    items = self.expr_list()?;
                }
                // `?? x AT 19` places the output on a Visual FoxPro screen; there is no column here
                if self.eat_kw("AT") {
                    self.expr()?;
                }
                return Ok(Some(StmtKind::Print { items, newline }));
            }
            TokKind::Eq => {
                self.advance();
                let e = self.expr()?;
                return Ok(Some(StmtKind::ExprStmt(e)));
            }
            // `&cmd`, `&cRef = .NULL.`, `&cArray[n] = x`: a line that begins with a macro is a
            // line whose first word is not there yet, so all of it waits until it runs
            TokKind::Amp => return Err(self.expected("command verb")),
            // `\ text` and `\\ text` are a one-line TEXT block: everything after the marker is
            // output as it stands, with <<expressions>> merged into it.
            TokKind::TextMark(same_line) => {
                // two backslashes carry the line before on; one ends it and starts a new one
                let newline = !same_line;
                self.advance();
                // the lexer read the rest of the line as one piece of text, whatever is in it
                let raw = match self.peek_kind() {
                    TokKind::RawText(text) => {
                        self.advance();
                        text
                    }
                    _ => String::new(),
                };
                return Ok(Some(StmtKind::TextLine { newline, raw }));
            }
            TokKind::Dot => return self.assign_or_call(),
            TokKind::At => return self.at_stmt(),
            TokKind::Bang => return self.run_stmt(first),
            TokKind::Ident(_) => {}
            _ => return Err(self.error(first.span, match first.ident() { Some(v) => format!("Unrecognized command verb '{}'", v.to_ascii_uppercase()), None => "Unrecognized command verb".to_string() })),
        }
        let text = first.ident().unwrap_or("").to_string();
        // `verb = expr` can only be an assignment: no command takes `=` as its next token.
        if matches!(self.peek_at(1).kind, TokKind::Eq) {
            return self.assign_or_call();
        }
        // `loCat.Connect()` is an object, however much its name looks like LOCATE; and
        // `myfunc(1)` is a call unless the word is a statement written out in full
        let exact = |list: &[&str]| list.iter().any(|k| k.eq_ignore_ascii_case(&text));
        let written_out = exact(STATEMENT_KEYWORDS) || exact(UNSUPPORTED) || exact(SCREEN_ONLY) || exact(IDE_ONLY);
        if !written_out && matches!(self.peek_at(1).kind, TokKind::Dot | TokKind::LParen) {
            return self.assign_or_call();
        }
        // Supported statements win over unsupported ones on ambiguous abbreviations (LOCA).
        let Some(verb) = STATEMENT_KEYWORDS.iter().find(|k| kw_text(&text, k)) else {
            if IGNORED.iter().any(|k| kw_text(&text, k)) {
                self.skip_line_keep_newline();
                return Ok(None);
            }
            if let Some(verb) = SCREEN_ONLY.iter().find(|k| kw_text(&text, k)) {
                self.warning(first.span, format!("{verb} is ignored: it draws on a Visual FoxPro screen this runtime does not have"));
                self.skip_line_keep_newline();
                return Ok(None);
            }
            if let Some(verb) = IDE_ONLY.iter().find(|k| kw_text(&text, k)) {
                self.warning(first.span, format!("{verb} is ignored: it asks the development environment for something a running program does not have"));
                self.skip_line_keep_newline();
                return Ok(None);
            }
            if let Some(verb) = UNSUPPORTED.iter().find(|k| kw_text(&text, k)) {
                self.error(first.span, format!("{verb} is not supported in the FoxDev runtime"));
                self.skip_line_keep_newline();
                return Ok(None);
            }
            return self.assign_or_call();
        };
        match *verb {
            "LOCAL" => self.var_decls(true).map(|d| Some(StmtKind::Local(d))),
            "PRIVATE" => {
                self.advance();
                if self.is_kw("ALL") {
                    self.warning(first.span, "PRIVATE ALL is not supported");
                    self.skip_line_keep_newline();
                    return Ok(None);
                }
                // PRIVATE only hides a variable the caller owns; ARRAY says the one being
                // hidden is an array. Unlike LOCAL and PUBLIC it does not need a size, and
                // any size written after it is thrown away: real VFP leaves the name
                // undefined until something DIMENSIONs it.
                self.eat_kw("ARRAY");
                let decls = self.decl_list(false)?;
                Ok(Some(StmtKind::Private(decls)))
            }
            "PUBLIC" => self.var_decls(true).map(|d| Some(StmtKind::Public(d))),
            "DIMENSION" => {
                self.advance();
                let decls = self.decl_list(true)?;
                Ok(Some(StmtKind::Dimension(decls)))
            }
            "DECLARE" => {
                // `DECLARE INTEGER GetTickCount IN kernel32` names a library; `DECLARE a(3)`
                // sizes an array. The IN is what tells them apart.
                if self.line_has_kw("IN") {
                    return self.declare_dll().map(Some);
                }
                self.advance();
                let decls = self.decl_list(true)?;
                Ok(Some(StmtKind::Dimension(decls)))
            }
            "LPARAMETERS" => {
                self.advance();
                let p = self.param_list(&TokKind::Newline)?;
                Ok(Some(StmtKind::LParameters(p)))
            }
            "PARAMETERS" => {
                self.advance();
                let p = self.param_list(&TokKind::Newline)?;
                Ok(Some(StmtKind::Parameters(p)))
            }
            "IF" => self.if_stmt(first).map(Some),
            "DO" => self.do_stmt(first),
            "FOR" => self.for_stmt(first).map(Some),
            "EXIT" => {
                self.advance();
                Ok(Some(StmtKind::Exit))
            }
            "LOOP" => {
                self.advance();
                Ok(Some(StmtKind::Loop))
            }
            // `ERASE (cFile)` names the file with an expression; anything else is the literal
            // path as written, which is why the rest of the line is taken verbatim.
            "ERASE" => {
                let start = self.advance().span;
                let path = if self.is(&TokKind::LParen) {
                    self.advance();
                    let e = self.expr()?;
                    self.expect(&TokKind::RParen, "')'")?;
                    // whether the file goes to the recycle bin is not a distinction made here
                    self.eat_kw("RECYCLE");
                    self.eat_kw("NORECYCLE");
                    e
                } else {
                    self.no_macro_ahead()?;
                    let text = self.rest_of_line_text().unwrap_or_default();
                    let text = ["RECYCLE", "NORECYCLE"]
                        .iter()
                        .find_map(|w| text.to_ascii_uppercase().strip_suffix(w).map(|t| text[..t.len()].trim_end().to_string()))
                        .unwrap_or(text);
                    Expr::new(ExprKind::Str(text), start)
                };
                Ok(Some(StmtKind::Erase(path)))
            }
            "USE" => self.use_stmt().map(Some),
            "SELECT" => self.select_stmt().map(Some),
            "GO" | "GOTO" => self.go_stmt().map(Some),
            "SKIP" => {
                self.advance();
                let count = if self.at_eol() || self.is_kw("IN") { None } else { Some(self.expr()?) };
                let area = self.in_clause()?;
                Ok(Some(StmtKind::Skip { count, area }))
            }
            "RETURN" => {
                self.advance();
                // `RETURN TO routine` leaves every routine between here and that one, and
                // `RETURN TO MASTER` leaves all of them but the program the run started in
                if self.eat_kw("TO") {
                    let to = match self.eat_kw("MASTER") {
                        true => None,
                        false => Some(self.expect_ident("a routine to return to")?),
                    };
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::ReturnTo(to)));
                }
                // `RETURN @This.aRGB` returns an array property by reference in VFP; here the
                // value travels through the host as a copy, which is what the callers read.
                self.eat(&TokKind::At);
                let value = if self.at_eol() { None } else { Some(self.expr()?) };
                Ok(Some(StmtKind::Return(value)))
            }
            "STORE" => {
                self.advance();
                let value = self.expr()?;
                if !self.eat_kw("TO") {
                    return Err(self.expected("TO"));
                }
                let mut targets = vec![self.name_target()?];
                while self.eat(&TokKind::Comma) {
                    targets.push(self.name_target()?);
                }
                Ok(Some(StmtKind::Store { value, targets }))
            }
            "WAIT" => self.wait_stmt().map(Some),
            "READ" => {
                self.advance();
                if self.eat_kw("EVENTS") {
                    return Ok(Some(StmtKind::ReadEvents));
                }
                // `READ` on its own, and `READ MENU`, let the user into the fields `@ ... GET`
                // put up. Five of its clauses name a procedure to run at a moment of the read;
                // the words that are left say how the read behaves, not what it reads.
                let mut read = ReadClauses::default();
                let mut words = String::new();
                loop {
                    if self.eat_kw("WHEN") {
                        read.when = Some(self.expr()?);
                    } else if self.eat_kw("SHOW") {
                        read.show = Some(self.expr()?);
                    } else if self.eat_kw("ACTIVATE") {
                        read.activate = Some(self.expr()?);
                    } else if self.eat_kw("DEACTIVATE") {
                        read.deactivate = Some(self.expr()?);
                    } else if self.eat_kw("VALID") {
                        read.valid = Some(self.expr()?);
                    } else if self.at_eol() {
                        break;
                    } else {
                        // CYCLE, MODAL, NOMOUSE, TIMEOUT and the rest: how it behaves
                        let at = self.advance().span;
                        words.push_str(&self.source_text(at.start, at.end));
                        words.push(' ');
                    }
                }
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::WindowCommand {
                    what: WindowVerb::Read,
                    name: None,
                    corners: Vec::new(),
                    title: None,
                    text: words.trim().to_string(),
                    flags: 0,
                    read: read.any().then(|| Box::new(read)),
                }))
            }
            "CLEAR" => {
                self.advance();
                if self.eat_kw("EVENTS") {
                    return Ok(Some(StmtKind::ClearEvents));
                }
                if self.eat_kw("ALL") {
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::ClearAll { tables: true }));
                }
                if self.eat_kw("MEMORY") {
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::ClearAll { tables: false }));
                }
                if self.eat_kw("GETS") {
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::WindowCommand {
                        read: None,
                        what: WindowVerb::ClearGets,
                        name: None,
                        corners: Vec::new(),
                        title: None,
                        text: String::new(),
                        flags: 0,
                    }));
                }
                // `CLEAR` on its own, and `CLEAR WINDOWS`, wipe the character screen
                let bare = self.at_eol();
                if bare || self.eat_kw("WINDOWS") {
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::WindowCommand {
                        read: None,
                        what: WindowVerb::ClearScreen,
                        name: None,
                        corners: Vec::new(),
                        title: None,
                        text: String::new(),
                        flags: if bare { 0 } else { 1 },
                    }));
                }
                // emptying the type-ahead buffer is what `KEYBOARD "" CLEAR` does, so it is that
                if self.eat_kw("TYPEAHEAD") {
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Keyboard {
                        keys: Expr::new(ExprKind::Str(String::new()), first.span),
                        plain: true,
                        clear: true,
                    }));
                }
                // The rest clear something this runtime does not have - the loaded DLLs, the
                // program history - and clearing nothing is exactly right for those.
                self.skip_line_keep_newline();
                Ok(None)
            }
            "REPLACE" => self.replace_stmt().map(Some),
            "DELETE" if kw(self.peek_at(1), "CONNECTION") => {
                self.advance();
                self.advance();
                let name = self.table_name()?;
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Database {
                    what: DbCommand::DeleteConnection,
                    name: Some(name),
                    target: None,
                    sql: String::new(), flags: 0 }))
            }
            "DELETE" => self.mark_deleted_stmt(first, true),
            "RECALL" => self.mark_deleted_stmt(first, false),
            "APPEND" => self.append_stmt(first),
            "ERROR" => {
                self.advance();
                let what = self.expr()?;
                let message = if self.eat(&TokKind::Comma) { Some(self.expr()?) } else { None };
                Ok(Some(StmtKind::RaiseError { what, message }))
            }
            "MD" | "MKDIR" => self.file_command(3, false),
            "RD" | "RMDIR" => self.file_command(4, false),
            "DIR" | "DIRECTORY" => {
                // DIR [mask] [TO PRINTER | TO FILE]: the listing goes to output here
                self.advance();
                let start = self.prev_span();
                let path = if self.at_eol() || self.is_kw("TO") {
                    Expr::new(ExprKind::Str("*.*".to_string()), start)
                } else if self.is(&TokKind::LParen) {
                    self.advance();
                    let e = self.expr()?;
                    self.expect(&TokKind::RParen, "')'")?;
                    e
                } else {
                    Expr::new(ExprKind::Str(self.library_name()), start)
                };
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::FileCommand { kind: 5, path, target: None }))
            }
            "TYPE" => self.file_command(6, false),
            "COPY" => {
                if kw(self.peek_at(1), "FILE") {
                    self.advance();
                    return self.file_command(1, true);
                }
                if kw(self.peek_at(1), "PROCEDURES") {
                    self.advance();
                    self.advance();
                    let _ = self.eat_kw("TO");
                    let path = self.table_name()?;
                    let flags = self.db_clauses();
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database {
                        what: DbCommand::CopyProcedures,
                        name: Some(path),
                        target: None,
                        sql: String::new(),
                        flags,
                    }));
                }
                if kw(self.peek_at(1), "TO") || kw(self.peek_at(1), "STRUCTURE") {
                    return self.copy_stmt().map(Some);
                }
                if kw(self.peek_at(1), "MEMO") {
                    self.advance();
                    self.advance();
                    return self.memo_stmt(1);
                }
                if kw(self.peek_at(1), "INDEXES") || kw(self.peek_at(1), "INDEX") {
                    self.advance();
                    self.advance();
                    return self.copy_indexes_stmt().map(Some);
                }
                if kw(self.peek_at(1), "TAG") {
                    self.advance();
                    self.advance();
                    return self.copy_tag_stmt().map(Some);
                }
                let what = self.peek_at(1).ident().map(|s| format!("COPY {}", s.to_ascii_uppercase()));
                self.skip_line_keep_newline();
                Err(self.error(first.span, format!("{} is not supported in the FoxDev runtime", what.unwrap_or("COPY".into()))))
            }
            "UNLOCK" => {
                self.advance();
                let mut record = None;
                let mut area = None;
                let mut all = false;
                loop {
                    if self.eat_kw("RECORD") {
                        record = Some(self.expr()?);
                    } else if self.eat_kw("IN") {
                        area = self.area_ref().ok();
                    } else if self.eat_kw("ALL") {
                        all = true;
                    } else {
                        break;
                    }
                }
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Unlock { record, area, all }))
            }
            "ALTER" => self.alter_table_stmt().map(Some),
            "BEGIN" | "END" | "ROLLBACK" => {
                let word = first.ident().unwrap_or_default().to_ascii_uppercase();
                self.advance();
                // BEGIN and END belong to a transaction; nothing else here begins or ends
                if word != "ROLLBACK" && !self.eat_kw("TRANSACTION") {
                    self.skip_line_keep_newline();
                    return Err(self.error(first.span, format!("{word} is not supported in the FoxDev runtime")));
                }
                self.skip_line_keep_newline();
                let step = match word.as_str() {
                    "BEGIN" => TransactionStep::Begin,
                    "END" => TransactionStep::End,
                    _ => TransactionStep::Rollback,
                };
                Ok(Some(StmtKind::Transaction(step)))
            }
            "OPEN" => {
                // OPEN DATABASE is the only OPEN this runtime has; the rest draw on a screen
                if !kw(self.peek_at(1), "DATABASE") {
                    let what = self.peek_at(1).ident().unwrap_or_default().to_ascii_uppercase();
                    self.advance();
                    self.warning(first.span, format!("OPEN {what} is ignored: it opens something this runtime does not have"));
                    self.skip_line_keep_newline();
                    return Ok(None);
                }
                self.advance();
                self.advance();
                let name = self.table_name()?;
                let flags = self.db_clauses();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Database { what: DbCommand::Open, name: Some(name), target: None, sql: String::new(), flags }))
            }
            "VALIDATE" => {
                self.advance();
                let _ = self.eat_kw("DATABASE");
                let mut flags = self.db_clauses();
                // `TO PRINTER` and `TO FILE name` both reach dbc_BeforeValidateData, the second
                // with the name it was given, so where the report goes is read rather than
                // passed over. The file is kept in `target`, which nothing else of VALIDATE uses.
                let mut target = None;
                if self.eat_kw("TO") {
                    if self.eat_kw("PRINTER") {
                        flags |= crate::ast::db_flags::PRINTER;
                    } else {
                        let _ = self.eat_kw("FILE");
                        flags |= crate::ast::db_flags::TO_FILE;
                        target = Some(self.table_name()?);
                    }
                }
                flags |= self.db_clauses();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Database { what: DbCommand::Validate, name: None, target, sql: String::new(), flags }))
            }
            "ADD" | "REMOVE" | "FREE" => {
                let word = first.ident().unwrap_or_default().to_ascii_uppercase();
                self.advance();
                if !self.eat_kw("TABLE") {
                    self.skip_line_keep_newline();
                    return Err(self.error(first.span, format!("{word} is not supported in the FoxDev runtime")));
                }
                let name = self.table_name()?;
                let flags = self.db_clauses();
                self.skip_line_keep_newline();
                let what = match word.as_str() {
                    "ADD" => DbCommand::AddTable,
                    "REMOVE" => DbCommand::RemoveTable,
                    _ => DbCommand::FreeTable,
                };
                Ok(Some(StmtKind::Database { what, name: Some(name), target: None, sql: String::new(), flags }))
            }
            "LIST" | "DISPLAY" => {
                if kw(self.peek_at(1), "DATABASE") {
                    self.advance();
                    self.advance();
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database { what: DbCommand::List, name: None, target: None, sql: String::new(), flags: 0 }));
                }
                let all_by_default = first.ident().is_some_and(|w| kw_text(&w.to_ascii_uppercase(), "LIST"));
                self.advance();
                self.list_stmt(all_by_default)
            }
            "UPDATE" => self.update_stmt(first).map(Some),
            "PACK" => {
                self.advance();
                if self.eat_kw("DATABASE") {
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database {
                        what: DbCommand::Pack,
                        name: None,
                        target: None,
                        sql: String::new(), flags: 0 }));
                }
                // PACK MEMO and PACK DBF ask for one half of the work; both are done here
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Pack))
            }
            "SORT" => self.sort_stmt().map(Some),
            "TOTAL" => self.total_stmt().map(Some),
            "DROP" => {
                self.advance();
                if self.eat_kw("VIEW") {
                    let name = self.table_name()?;
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database {
                        what: DbCommand::DropView,
                        name: Some(name),
                        target: None,
                        sql: String::new(), flags: 0 }));
                }
                if !self.eat_kw("TABLE") {
                    self.skip_line_keep_newline();
                    return Err(self.error(first.span, "DROP is not supported in the FoxDev runtime"));
                }
                let path = self.table_name()?;
                let flags = self.db_clauses();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::DropTable { path, flags }))
            }
            "RENAME" => {
                if kw(self.peek_at(1), "CONNECTION") {
                    self.advance();
                    self.advance();
                    let name = self.table_name()?;
                    let _ = self.eat_kw("TO");
                    let target = self.table_name()?;
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database {
                        what: DbCommand::RenameConnection,
                        name: Some(name),
                        target: Some(target),
                        sql: String::new(), flags: 0 }));
                }
                if kw(self.peek_at(1), "VIEW") {
                    self.advance();
                    self.advance();
                    let name = self.table_name()?;
                    let _ = self.eat_kw("TO");
                    let target = self.table_name()?;
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database {
                        what: DbCommand::RenameView,
                        name: Some(name),
                        target: Some(target),
                        sql: String::new(), flags: 0 }));
                }
                if kw(self.peek_at(1), "TABLE") {
                    self.advance();
                    self.advance();
                    let name = self.table_name()?;
                    let _ = self.eat_kw("TO");
                    let target = self.table_name()?;
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database {
                        what: DbCommand::RenameTable,
                        name: Some(name),
                        target: Some(target),
                        sql: String::new(), flags: 0 }));
                }
                self.file_command(2, true)
            }
            "ZAP" => {
                self.advance();
                let (_, _, _, area) = self.record_clauses_in();
                Ok(Some(StmtKind::Zap { area }))
            }
            "CD" | "CHDIR" => {
                // CD is SET DEFAULT TO wearing a shorter name
                self.advance();
                let path = if self.eat(&TokKind::LParen) {
                    let e = self.expr()?;
                    self.expect(&TokKind::RParen, "')'")?;
                    e
                } else {
                    self.path_or_expression(first.span)
                };
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Set { setting: Name::new("DEFAULT", first.span), value: SetValue::To(vec![path]) }))
            }
            "IMPORT" => self.import_stmt(first),
            "EXPORT" => self.export_stmt(first),
            "BUILD" => self.build_stmt(first),
            "COMPILE" => self.compile_stmt(first),
            "CREATE" => self.create_stmt(first),
            "INSERT" => self.insert_stmt(first).map(Some),
            "MODIFY" => self.modify_stmt(first),
            "BROWSE" => self.browse_stmt(),
            "SCAN" => self.scan_stmt(first).map(Some),
            "LOCATE" => {
                self.advance();
                let (scope, cond, while_) = self.record_clauses();
                Ok(Some(StmtKind::Locate { scope, cond, while_ }))
            }
            "CONTINUE" => {
                self.advance();
                Ok(Some(StmtKind::Continue))
            }
            "SEEK" => {
                self.advance();
                let key = self.expr()?;
                let mut order = None;
                let mut descending = None;
                loop {
                    if self.eat_kw("ORDER") {
                        let (tag, desc) = self.order_clause()?;
                        order = tag;
                        descending = descending.or(desc);
                    } else if self.eat_kw("ASCENDING") {
                        descending = Some(false);
                    } else if self.eat_kw("DESCENDING") {
                        descending = Some(true);
                    } else if self.eat_kw("IN") {
                        // the work area to seek in: the selected one is the only one seen here
                        let _ = self.advance();
                    } else {
                        break;
                    }
                }
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Seek { key, order, descending }))
            }
            "INDEX" => self.index_stmt().map(Some),
            "COUNT" => self.aggregate_stmt(AggFunc::Cnt).map(Some),
            "SUM" => self.aggregate_stmt(AggFunc::Sum).map(Some),
            "AVERAGE" => self.aggregate_stmt(AggFunc::Avg).map(Some),
            "CALCULATE" => self.aggregate_stmt(AggFunc::Std).map(Some),
            "SCATTER" => self.scatter_stmt().map(Some),
            "GATHER" => self.gather_stmt().map(Some),
            "REINDEX" => {
                self.advance();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Reindex))
            }
            "CLOSE" => {
                self.advance();
                // CLOSE ALL and CLOSE DATABASES close every table too; the windows, indexes and
                // procedure files they also close are things this runtime does not have.
                if self.is_kw("DATABASES") {
                    self.advance();
                    let flags = self.db_clauses();
                    self.skip_line_keep_newline();
                    return Ok(Some(StmtKind::Database {
                        what: DbCommand::Close,
                        name: None,
                        target: None,
                        sql: String::new(),
                        flags,
                    }));
                }
                if self.eat_kw("MEMO") {
                    return self.memo_stmt(3);
                }
                if self.eat_kw("TABLES") || self.eat_kw("ALL") {
                    // a trailing ALL or NOENVIRONMENT adds nothing once every table is closed
                    self.skip_line_keep_newline();
                    Ok(Some(StmtKind::CloseTables { all: true }))
                } else {
                    let what = self.peek().ident().map(|s| format!("CLOSE {}", s.to_ascii_uppercase()));
                    let msg = format!("{} is not supported in the FoxDev runtime", what.unwrap_or("CLOSE".into()));
                    self.warning(first.span, msg);
                    self.skip_line_keep_newline();
                    Ok(None)
                }
            }
            "RETRY" => {
                self.advance();
                Ok(Some(StmtKind::Retry))
            }
            "RELEASE" => self.release_stmt(first),
            "QUIT" => {
                self.advance();
                Ok(Some(StmtKind::Quit))
            }
            "CANCEL" => {
                self.advance();
                Ok(Some(StmtKind::Cancel))
            }
            "WITH" => {
                self.advance();
                let obj = self.expr_or_recover();
                self.end_of_line();
                let (body, term) = self.block_until(&["ENDWITH"], first, "WITH");
                if term.is_some() {
                    self.advance();
                }
                Ok(Some(StmtKind::With { obj, body }))
            }
            "SET" => self.set_stmt().map(Some),
            "TRY" => self.try_stmt(first).map(Some),
            "THROW" => {
                self.advance();
                let value = if self.at_eol() { None } else { Some(self.expr()?) };
                Ok(Some(StmtKind::Throw(value)))
            }
            "NODEFAULT" => {
                self.advance();
                Ok(Some(StmtKind::NoDefault))
            }
            "DODEFAULT" => {
                self.advance();
                let args = if self.eat(&TokKind::LParen) { self.call_args()? } else { Vec::new() };
                Ok(Some(StmtKind::DoDefault(args)))
            }
            "PROCEDURE" | "FUNCTION" => {
                // Only reachable in method mode (program mode treats these as block terminators).
                Err(self.error(first.span, format!("{verb} declarations are not allowed inside a method")))
            }
            "ACTIVATE" | "DEACTIVATE" | "SHOW" | "HIDE" => self.activate_stmt(first),
            "PUSH" | "POP" => self.push_pop_stmt(first),
            "MOVE" | "ZOOM" | "SIZE" => self.move_window_stmt(first),
            "REPORT" | "LABEL" => self.report_stmt(first),
            "KEYBOARD" => {
                self.advance();
                let keys = if self.at_eol() {
                    Expr::new(ExprKind::Str(String::new()), first.span)
                } else {
                    self.expr()?
                };
                let mut plain = false;
                let mut clear = false;
                loop {
                    if self.eat_kw("PLAIN") {
                        plain = true;
                    } else if self.eat_kw("CLEAR") {
                        clear = true;
                    } else {
                        break;
                    }
                }
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Keyboard { keys, plain, clear }))
            }
            "INPUT" | "ACCEPT" | "GETEXPR" => self.ask_stmt(first),
            "RUN" => self.run_stmt(first),
            "SCROLL" => {
                self.advance();
                // SCROLL nRow1, nCol1, nRow2, nCol2, nRows [, nColumns]: the region, then how
                // far it moves on each axis
                let mut numbers = vec![self.expr()?];
                while self.eat(&TokKind::Comma) {
                    numbers.push(self.expr()?);
                }
                self.skip_line_keep_newline();
                if numbers.len() < 5 {
                    self.error(first.span, "SCROLL needs a region and how far it moves");
                    return Ok(None);
                }
                let mut it = numbers.into_iter();
                let (row, col) = (it.next().unwrap(), it.next().unwrap());
                let corners = vec![it.next().unwrap(), it.next().unwrap()];
                let amount = it.next();
                let amount2 = it.next();
                Ok(Some(StmtKind::AtCommand(vec![AtPart {
                    what: AtVerb::Scroll,
                    row,
                    col,
                    corners,
                    value: None,
                    picture: None,
                    function: None,
                    amount,
                    amount2,
                    style: String::new(),
                    name: String::new(),
                    valid: String::new(),
                    when: String::new(),
                }])))
            }
            "JOIN" => self.join_stmt(first),
            "MOUSE" => self.mouse_stmt(),
            "FLUSH" | "DOEVENTS" => {
                let events = kw(first, "DOEVENTS");
                self.advance();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Yield { events }))
            }
            "ASSERT" => {
                self.advance();
                let cond = self.expr()?;
                let mut message = Vec::new();
                if self.eat_kw("MESSAGE") {
                    message.push(self.expr()?);
                }
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Diagnostic { cond: Some(cond), message }))
            }
            "DEBUGOUT" => {
                self.advance();
                let message = if self.at_eol() { Vec::new() } else { self.expr_list()? };
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Diagnostic { cond: None, message }))
            }
            "SUSPEND" | "RESUME" => {
                let verb = if kw(first, "SUSPEND") { DebugVerb::Suspend } else { DebugVerb::Resume };
                self.advance();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Debug(verb)))
            }
            // FIND looks a key up in the master index, which is what SEEK does; the key is
            // written without quotes, as the command of FoxPro 2.x had it
            "FIND" => {
                self.advance();
                self.no_macro_ahead()?;
                let text = self.rest_of_line_text().unwrap_or_default();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Seek {
                    key: Expr::new(ExprKind::Str(text.trim().to_string()), first.span),
                    order: None,
                    descending: None,
                }))
            }
            "BLANK" => {
                self.advance();
                let (fields, _) = self.field_clause()?;
                let (scope, cond, _) = self.record_clauses();
                // BLANK with no scope and no condition empties the record it is on, and only
                // that one, the way REPLACE and DELETE do
                let scope = match (&scope, &cond) {
                    (Scope::All, None) => Scope::Current,
                    _ => scope,
                };
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Blank { fields, scope, cond }))
            }
            // EDIT and CHANGE show the records the same way BROWSE does, one at a time rather
            // than in a grid; the window is the host's to draw either way
            "EDIT" | "CHANGE" => self.browse_stmt(),
            "EJECT" => {
                self.advance();
                self.skip_line_keep_newline();
                Ok(Some(StmtKind::Eject))
            }
            // PRINTJOB ... ENDPRINTJOB sets the printer up round what is between them, which
            // is where the report goes here whether it is said or not
            "PRINTJOB" => {
                self.advance();
                self.end_of_line();
                let (body, term) = self.block_until(&["ENDPRINTJOB"], first, "PRINTJOB");
                if term.is_some() {
                    self.advance();
                }
                Ok(Some(StmtKind::With { obj: Expr::new(ExprKind::Screen, first.span), body }))
            }
            "MENU" => {
                // `MENU TO <var>` reads the choices `@ ... PROMPT` put up; the rest of the
                // MENU family draws a Visual FoxPro menu, which the menu commands do here
                self.advance();
                let name = if self.eat_kw("TO") { Some(self.menu_name()?) } else { None };
                self.skip_line_keep_newline();
                Ok(name.map(|name| StmtKind::WindowCommand {
                        read: None,
                    what: WindowVerb::MenuTo,
                    name: Some(name),
                    corners: Vec::new(),
                    title: None,
                    text: String::new(),
                    flags: 0,
                }))
            }
            "SAVE" | "RESTORE" => self.save_restore_stmt(first),
            "DEFINE" => {
                if kw(self.peek_at(1), "CLASS") {
                    self.error(first.span, "DEFINE CLASS must appear at the top level of a program");
                    self.skip_line();
                    loop {
                        if self.is(&TokKind::Eof) {
                            break;
                        }
                        let is_end = self.peek().ident().is_some_and(|t| t.len() >= 5 && kw_text(t, "ENDDEFINE"));
                        if is_end {
                            self.skip_line_keep_newline();
                            break;
                        }
                        self.skip_line();
                    }
                } else if kw(self.peek_at(1), "WINDOW") {
                    return self.define_window_stmt();
                } else if MENU_DEFINES.iter().any(|w| kw(self.peek_at(1), w)) {
                    return self.define_menu_stmt();
                } else {
                    self.warning(first.span, "DEFINE is ignored: it draws on a Visual FoxPro screen this runtime does not have");
                    self.skip_line_keep_newline();
                }
                Ok(None)
            }
            "TEXT" => self.text_stmt(first).map(Some),
            "ON" => self.on_stmt(first),
            _ => unreachable!("unhandled statement keyword {verb}"),
        }
    }

    /// Skips the rest of the line but leaves the Newline for the statement driver.
    fn skip_line_keep_newline(&mut self) {
        while !self.at_eol() {
            self.advance();
        }
    }

    /// `LOCAL [ARRAY] decls` / `PUBLIC [ARRAY] decls`.
    fn var_decls(&mut self, allow_array: bool) -> PResult<Vec<VarDecl>> {
        self.advance();
        let array = allow_array && self.eat_kw("ARRAY");
        self.decl_list(array)
    }

    /// True when a declaration names a member (`THISFORM.aRows`, `THIS.x`, `.x` inside a WITH)
    /// rather than a plain variable.
    fn at_member_target(&mut self) -> bool {
        if self.is(&TokKind::Dot) {
            return true;
        }
        matches!(self.peek_kind(), TokKind::Ident(_)) && matches!(self.peek_at(1).kind, TokKind::Dot)
    }

    fn decl_list(&mut self, dims_required: bool) -> PResult<Vec<VarDecl>> {
        let mut decls = Vec::new();
        loop {
            // `DIMENSION THISFORM.aRows[1,2]` sizes an array property, which is an assignment to
            // it rather than a declaration, so it is read as a target and handled as one
            if dims_required && self.at_member_target() {
                let target = self.expr()?;
                decls.push(VarDecl { name: NameRef::Named(Name::new("", target.span)), dims: None, member: Some(target) });
                if !self.eat(&TokKind::Comma) {
                    break;
                }
                continue;
            }
            self.eat_memvar_prefix();
            let name = self.name_ref("variable name")?;
            let dims = if self.eat(&TokKind::LParen) {
                Some(self.expr_list_until(&TokKind::RParen, "')'")?)
            } else if self.eat(&TokKind::LBracket) {
                Some(self.expr_list_until(&TokKind::RBracket, "']'")?)
            } else if self.peek().bracketed {
                // `DIMENSION a [3]`: a space before the bracket does not make it a string here
                let tok = self.advance();
                Some(self.subscripts_in_brackets(&tok)?)
            } else {
                None
            };
            if dims_required && dims.is_none() {
                return Err(self.expected("array dimensions"));
            }
            self.skip_as_type();
            decls.push(VarDecl { name, dims, member: None });
            if !self.eat(&TokKind::Comma) {
                self.eat_list_close();
                break;
            }
        }
        Ok(decls)
    }

    /// A `)` closing a list of names that was never opened with a `(`.
    ///
    /// Visual FoxPro reads a declaration's names as if the list were optionally parenthesised,
    /// so `LPARAMETERS ta, tb)` and `LOCAL x, y)` both compile - measured, along with the two
    /// that do not: a word after the list, and a `)` where another name should be. The
    /// Foundation Classes ship a method whose LPARAMETERS ends that way.
    fn eat_list_close(&mut self) {
        self.eat(&TokKind::RParen);
    }

    /// Parses a block-header expression; on error the rest of the line is skipped and a
    /// placeholder is returned so the block structure survives.
    fn expr_or_recover(&mut self) -> Expr {
        let span = self.peek().span;
        match self.expr() {
            Ok(e) => e,
            Err(_) => {
                self.skip_line_keep_newline();
                Expr::new(ExprKind::Bool(false), span)
            }
        }
    }

    fn if_stmt(&mut self, first: &Token) -> PResult<StmtKind> {
        self.advance();
        let cond = self.expr_or_recover();
        self.eat_kw("THEN");
        self.end_of_line();
        let (then, term) = self.block_until(&["ENDIF", "ELSE"], first, "IF");
        let mut else_ = None;
        match term {
            Some("ELSE") => {
                self.advance();
                self.end_of_line();
                let (b, t) = self.block_until(&["ENDIF"], first, "IF");
                else_ = Some(b);
                if t.is_some() {
                    self.advance();
                }
            }
            Some(_) => {
                self.advance();
            }
            None => {}
        }
        Ok(StmtKind::If { cond, then, else_ })
    }

    fn do_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if self.is_kw("CASE") {
            self.advance();
            self.end_of_line();
            return self.do_case(first).map(Some);
        }
        if self.is_kw("WHILE") {
            self.advance();
            let cond = self.expr_or_recover();
            self.end_of_line();
            let (body, term) = self.block_until(&["ENDDO"], first, "DO WHILE");
            if term.is_some() {
                self.advance();
            }
            return Ok(Some(StmtKind::DoWhile { cond, body }));
        }
        if self.is_kw("FORM") {
            self.advance();
            return self.do_form().map(Some);
        }
        // `DO (cProgram)`: the program to run is named by an expression, evaluated each time
        if self.eat(&TokKind::LParen) {
            let name = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            let (args, in_prog) = self.do_clauses()?;
            return Ok(Some(StmtKind::DoExpr { name, args, in_prog }));
        }
        // `DO "C:\tools\run.prg"`: a path with spaces or a drive in it is written in quotes,
        // and a quoted name is the same thing as one worked out
        if let TokKind::Str(text) = self.peek_kind() {
            let tok = self.advance();
            let name = Expr::new(ExprKind::Str(text), tok.span);
            let (args, in_prog) = self.do_clauses()?;
            return Ok(Some(StmtKind::DoExpr { name, args, in_prog }));
        }
        // `DO p_cod+"vx_prios.prg"`: measured, a name with a `+` right against it is an
        // expression that builds the program's name, and the same line written with spaces
        // round the `+` is a syntax error in the product - so only the unspaced form is read so
        if let TokKind::Ident(_) = self.peek_kind()
            && matches!(self.peek_at(1).kind, TokKind::Plus)
            && self.peek_at(1).span.start == self.peek().span.end
        {
            let name = self.expr()?;
            let (args, in_prog) = self.do_clauses()?;
            return Ok(Some(StmtKind::DoExpr { name, args, in_prog }));
        }
        let name = self.file_name("program name")?;
        let (args, in_prog) = self.do_clauses()?;
        Ok(Some(StmtKind::Do { name, args, in_prog }))
    }

    /// `REPLACE field WITH x [ADDITIVE] [, ...] [scope] [FOR y] [WHILE z] [IN area]`.
    fn replace_stmt(&mut self) -> PResult<StmtKind> {
        self.advance();
        // `REPLACE FROM ARRAY a` is a GATHER over a scope rather than a list of assignments
        if self.eat_kw("FROM") {
            if !self.eat_kw("ARRAY") {
                return Err(self.expected("ARRAY"));
            }
            let source = self.expr()?;
            let mut fields = Vec::new();
            if self.eat_kw("FIELDS") {
                loop {
                    fields.push(self.expect_ident("field name")?);
                    if !self.eat(&TokKind::Comma) {
                        break;
                    }
                }
            }
            let (scope, cond, while_, area) = self.record_clauses_in();
            self.skip_line_keep_newline();
            let scope = scope.unwrap_or(match (&cond, &while_) {
                (None, None) => Scope::Current,
                _ => Scope::All,
            });
            return Ok(StmtKind::ReplaceFromArray { source, area, fields, scope, cond, while_ });
        }
        // VFP takes the scope on either side of the assignments, so both are read
        let (mut scope, mut cond, mut while_, mut area) = self.record_clauses_in();
        let mut assignments = Vec::new();
        loop {
            // the field may be qualified - REPLACE customer.balance WITH x - or named by an
            // expression, which is only known when the command runs
            let field = if self.is(&TokKind::LParen) || matches!(self.peek_kind(), TokKind::Str(_)) {
                self.name_ref("field name")?
            } else {
                let mut name = self.expect_ident("field name")?;
                if self.eat(&TokKind::Dot) {
                    name = self.expect_ident("field name")?;
                }
                NameRef::Named(name)
            };
            if !self.eat_kw("WITH") {
                return Err(self.error_here("expected WITH"));
            }
            let value = self.expr()?;
            // ADDITIVE appends to a memo field, and memo fields are not written yet
            let additive = self.eat_kw("ADDITIVE");
            assignments.push((field, value, additive));
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        let (after_scope, after_cond, after_while, after_area) = self.record_clauses_in();
        scope = scope.or(after_scope);
        cond = cond.or(after_cond);
        while_ = while_.or(after_while);
        area = area.or(after_area);
        // REPLACE with no scope and no condition changes the record it is on, and only that one
        let scope = scope.unwrap_or(match (&cond, &while_) {
            (None, None) => Scope::Current,
            _ => Scope::All,
        });
        Ok(StmtKind::Replace { area, assignments, scope, cond, while_ })
    }

    /// `DELETE` and `RECALL`, with the same scope clauses. SQL's `DELETE FROM` is a different
    /// statement wearing the same word, and says so.
    fn mark_deleted_stmt(&mut self, first: &Token, deleted: bool) -> PResult<Option<StmtKind>> {
        self.advance();
        if deleted && self.eat_kw("FROM") {
            let (table, path) = self.sql_table()?;
            let cond = if self.eat_kw("WHERE") { Some(self.expr()?) } else { None };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::DeleteSql { table, path, cond }));
        }
        // DELETE DATABASE and DELETE VIEW work on a database container, not on records
        if deleted && self.is_kw("DATABASE") {
            self.advance();
            let name = self.table_name()?;
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Database {
                what: DbCommand::Delete,
                name: Some(name),
                target: None,
                sql: String::new(), flags: 0 }));
        }
        if deleted && self.is_kw("VIEW") {
            self.advance();
            let name = self.table_name()?;
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Database {
                what: DbCommand::DropView,
                name: Some(name),
                target: None,
                sql: String::new(), flags: 0 }));
        }
        // `DELETE TRIGGER ON table FOR INSERT | UPDATE | DELETE` takes a trigger off a table in
        // a database. Nothing here makes one - CREATE TRIGGER is refused by name - so there is
        // never one to take off, and the command is read and does nothing.
        if deleted && self.eat_kw("TRIGGER") {
            let _ = self.eat_kw("ON");
            let _ = self.table_name()?;
            if self.eat_kw("FOR") {
                let _ = self.expect_ident("INSERT, UPDATE or DELETE")?;
            }
            self.skip_line_keep_newline();
            return Ok(None);
        }
        // DELETE TAG takes a tag out of the index; it has nothing to do with records either
        if deleted && self.eat_kw("TAG") {
            let tag = if self.eat_kw("ALL") { None } else { self.expect_ident("tag name").ok() };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::DeleteTag { tag }));
        }
        // DELETE FILE removes a file; it has nothing to do with the record pointer
        if self.eat_kw("FILE") {
            let path = if self.eat(&TokKind::LParen) {
                let e = self.expr()?;
                self.expect(&TokKind::RParen, "')'")?;
                e
            } else {
                Expr::new(ExprKind::Str(self.bare_path()), first.span)
            };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Erase(path)));
        }
        let (scope, cond, while_, area) = self.record_clauses_in();
        let scope = scope.unwrap_or(match (&cond, &while_) {
            (None, None) => Scope::Current,
            _ => Scope::All,
        });
        Ok(Some(StmtKind::MarkDeleted { deleted, area, scope, cond, while_ }))
    }

    /// `APPEND BLANK [IN area]`. The forms that read from somewhere else are not supported.
    /// True when the rest of the line holds `word` as a keyword of its own.
    fn line_has_kw(&self, word: &str) -> bool {
        let mut i = self.pos;
        while !self.toks[i].is_newline() {
            if self.toks[i].ident().is_some_and(|t| kw_text(t, word)) {
                return true;
            }
            i += 1;
        }
        false
    }

    /// A file command: the verb, a name, and for COPY FILE and RENAME a TO and a second name.
    /// A name is a parenthesised expression or one word, as VFP takes it.
    fn file_command(&mut self, kind: u8, two: bool) -> PResult<Option<StmtKind>> {
        self.advance();
        let path = self.file_operand()?;
        let target = if two {
            if !self.eat_kw("TO") {
                return Err(self.expected("TO"));
            }
            Some(self.file_operand()?)
        } else {
            None
        };
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::FileCommand { kind, path, target }))
    }

    fn file_operand(&mut self) -> PResult<Expr> {
        let start = self.peek().span;
        if self.is(&TokKind::LParen) {
            self.advance();
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(e);
        }
        if let TokKind::Str(text) = self.peek_kind() {
            self.advance();
            return Ok(Expr::new(ExprKind::Str(text), start));
        }
        if self.at_eol() {
            return Err(self.expected("file name"));
        }
        Ok(Expr::new(ExprKind::Str(self.library_name()), start))
    }

    /// The library of a `DECLARE ... DLL`: one word, which may carry a path and an extension.
    ///
    /// It ends at the first space, unlike the rest-of-line a `USE` takes, because what follows
    /// it on the line is the alias and the parameters.
    fn library_name(&mut self) -> String {
        let start = self.peek().span.start;
        let mut end = self.advance().span.end;
        while !self.at_eol()
            && self.peek().span.start == end
            && matches!(self.peek_kind(), TokKind::Dot | TokKind::Slash | TokKind::Backslash | TokKind::Colon | TokKind::Ident(_) | TokKind::Num(..))
        {
            end = self.advance().span.end;
        }
        self.source_text(start, end).trim().to_string()
    }

    /// `DECLARE [type] Function IN library [AS alias] [type [@] [name], ...]`.
    ///
    /// The type words are Visual FoxPro's: SHORT, INTEGER, LONG, SINGLE, DOUBLE, STRING and
    /// OBJECT. A parameter may be followed by a name, which documents the call and is ignored.
    fn declare_dll(&mut self) -> PResult<StmtKind> {
        self.advance();
        // a type before the function name says what the library returns, and it is only
        // there when the word after this one is not the IN that names the library
        let mut returns = None;
        if matches!(self.peek_at(1).kind, TokKind::Ident(_)) && !kw(self.peek_at(1), "IN") && !self.is_kw("IN") {
            returns = Some(self.expect_ident("return type")?);
        }
        let function = self.expect_ident("function name")?;
        if !self.eat_kw("IN") {
            return Err(self.expected("IN"));
        }
        let library = if self.is(&TokKind::LParen) {
            self.advance();
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            e
        } else if let TokKind::Str(text) = self.peek_kind() {
            let tok = self.advance();
            Expr::new(ExprKind::Str(text), tok.span)
        } else {
            let start = self.peek().span;
            Expr::new(ExprKind::Str(self.library_name()), start)
        };
        let alias = if self.eat_kw("AS") { Some(self.expect_ident("alias name")?) } else { None };

        let mut params = Vec::new();
        while !self.at_eol() {
            let kind = self.expect_ident("parameter type")?;
            let by_ref = self.eat(&TokKind::At);
            // the parameter's name is documentation; VFP ignores it too
            if matches!(self.peek_kind(), TokKind::Ident(_)) {
                self.advance();
            }
            params.push(DllParam { kind, by_ref });
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        self.skip_line_keep_newline();
        Ok(StmtKind::DeclareDll { returns, function, library, alias, params })
    }

    fn append_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if self.is_kw("PROCEDURES") {
            self.advance();
            let _ = self.eat_kw("FROM");
            let path = self.table_name()?;
            let flags = self.db_clauses();
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Database {
                what: DbCommand::AppendProcedures,
                name: Some(path),
                target: None,
                sql: String::new(),
                flags,
            }));
        }
        if self.eat_kw("FROM") {
            return self.append_from_stmt().map(Some);
        }
        if self.eat_kw("MEMO") {
            return self.memo_stmt(0);
        }
        // `APPEND GENERAL f [FROM file] [CLASS name] [DATA x] [LINK]`: a General field is a
        // memo holding an OLE object, and what goes into it is a file. Nothing here renders
        // one, so the file's bytes are kept and CLASS, DATA and LINK - which describe the
        // object rather than its contents - are read and passed over.
        if self.eat_kw("GENERAL") {
            // the field may be named by an expression - `APPEND GENERAL (THIS.cGraphField)` -
            // which is only worked out when the command runs
            let mut field_expr = None;
            let mut name = if self.is(&TokKind::LParen) {
                self.advance();
                let e = self.expr()?;
                self.expect(&TokKind::RParen, "')'")?;
                let span = e.span;
                field_expr = Some(e);
                Name::new(String::new(), span)
            } else {
                self.expect_ident("general field")?
            };
            if field_expr.is_none() && self.eat(&TokKind::Dot) {
                let field = self.expect_ident("general field")?;
                name = Name::new(format!("{}.{}", name.text, field.text), name.span);
            }
            let path = match self.eat_kw("FROM") {
                false => None,
                // the file name stops at the first word of a clause rather than swallowing it
                true if self.is(&TokKind::LParen) => Some(self.table_name()?),
                true => {
                    let span = self.peek().span;
                    let text = self.path_before(&["CLASS", "DATA", "LINK"]);
                    Some(Expr::new(ExprKind::Str(text), span))
                }
            };
            loop {
                if self.eat_kw("CLASS") {
                    let _ = self.name_or_expr();
                } else if self.eat_kw("DATA") {
                    let _ = self.expr()?;
                } else if self.eat_kw("LINK") {
                    // the object is kept beside the table rather than in it, which is a
                    // difference only something that renders it would notice
                } else {
                    break;
                }
            }
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Memo { what: 4, fields: vec![name], field_expr, path, flags: 0 }));
        }
        // APPEND on its own adds a blank record and opens an editing window on it, which is
        // the record editor EDIT and CHANGE open
        if self.at_eol() {
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::AppendBlank { area: None }));
        }
        if !self.eat_kw("BLANK") {
            let what = self.peek().ident().map(|s| format!("APPEND {}", s.to_ascii_uppercase()));
            self.skip_line_keep_newline();
            return Err(self.error(first.span, format!("{} is not supported in the FoxDev runtime", what.unwrap_or("APPEND".into()))));
        }
        let (_, _, _, area) = self.record_clauses_in();
        Ok(Some(StmtKind::AppendBlank { area }))
    }

    /// The scope, FOR, WHILE and IN clauses the record-changing commands take.
    ///
    /// The scope comes back as `None` when the command wrote none, because for REPLACE, DELETE
    /// and RECALL a written `ALL` and a missing scope mean different things - every record
    /// against the one the pointer is on - and only the writing of it tells them apart.
    fn record_clauses_in(&mut self) -> (Option<Scope>, Option<Expr>, Option<Expr>, Option<NameRef>) {
        let mut area = None;
        let mut scope = None;
        let mut cond = None;
        let mut while_ = None;
        loop {
            if self.eat_kw("IN") {
                area = self.area_ref().ok();
            } else if self.eat_kw("ALL") {
                scope = Some(Scope::All);
            } else if self.eat_kw("REST") {
                scope = Some(Scope::Rest);
            } else if self.eat_kw("NEXT") {
                scope = Some(Scope::Next(self.expr_or_recover()));
            } else if self.eat_kw("RECORD") {
                scope = Some(Scope::Record(self.expr_or_recover()));
            } else if self.eat_kw("FOR") {
                cond = Some(self.expr_or_recover());
            } else if self.eat_kw("WHILE") {
                while_ = Some(self.expr_or_recover());
            } else if self.eat_kw("NOOPTIMIZE") {
                // Rushmore is VFP's index optimiser; there is nothing here to turn off
            } else {
                break;
            }
        }
        (scope, cond, while_, area)
    }

    /// `CREATE CURSOR name (field type(width[, decimals]) [NULL|NOT NULL], ...)`.
    ///
    /// The other CREATE forms make something on disk - a table, a database, an index - which is
    /// not the same as a cursor that lives for as long as the program does.
    fn create_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if self.eat_kw("TABLE") || self.eat_kw("DBF") {
            // the name is a bare path or a parenthesised expression; FREE says "not in a
            // database", which is the only kind of table there is here
            let path = if self.is(&TokKind::LParen) {
                self.advance();
                let e = self.expr()?;
                self.expect(&TokKind::RParen, "')'")?;
                e
            } else if let TokKind::Str(text) = self.peek_kind() {
                // `CREATE TABLE 'APPREG01.DBF' NAME 'APPREG01' (...)` is what a generated
                // data-definition program writes: the file in quotes, then the long name
                let tok = self.advance();
                Expr::new(ExprKind::Str(text), tok.span)
            } else {
                // a bare name, with the extension VFP lets it carry: CREATE TABLE people.dbf
                let mut name = self.expect_ident("table name")?.text;
                while self.eat(&TokKind::Dot) {
                    name.push('.');
                    name.push_str(&self.expect_ident("extension")?.text);
                }
                Expr::new(ExprKind::Str(name), first.span)
            };
            // `NAME LongTableName` is what the database lists the table as. The container here
            // lists a table by its file stem, which is what every program that writes this
            // clause names it anyway, so the long name is read and not kept.
            if self.eat_kw("NAME") {
                let _ = self.name_ref("long table name")?;
            }
            let free = self.eat_kw("FREE");
            if let Some(from_array) = self.from_array_clause()? {
                self.skip_line_keep_newline();
                return Ok(Some(StmtKind::CreateTable { path, fields: Vec::new(), from_array: Some(from_array), free }));
            }
            let fields = self.cursor_fields()?;
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::CreateTable { path, fields, from_array: None, free }));
        }
        if self.eat_kw("DATABASE") {
            let name = if self.at_eol() { None } else { Some(self.table_name()?) };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Database { what: DbCommand::Create, name, target: None, sql: String::new(), flags: 0 }));
        }
        if self.is_kw("SQL") || self.is_kw("VIEW") {
            let _ = self.eat_kw("SQL");
            if !self.eat_kw("VIEW") {
                self.skip_line_keep_newline();
                return Err(self.error(first.span, "CREATE SQL needs VIEW and a name"));
            }
            let name = self.table_name()?;
            // REMOTE is what dbc_AfterCreateView is handed as its second argument, so which of
            // the two kinds of view was asked for is kept
            let flags = if self.eat_kw("REMOTE") { crate::ast::db_flags::REMOTE } else { 0 };
            if !self.eat_kw("AS") {
                self.skip_line_keep_newline();
                return Err(self.error(first.span, "CREATE SQL VIEW needs AS and a SELECT"));
            }
            // the SELECT is kept as it was written: it is run when the view is used
            let sql = self.rest_of_line_text().unwrap_or_default();
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Database {
                what: DbCommand::CreateView,
                name: Some(name),
                target: None,
                sql,
                flags,
            }));
        }
        // `CREATE new FROM description`: the table a structure-extended table describes.
        // The name comes first and FROM follows it, so this is looked for before the designers,
        // which take a word rather than a file name.
        if !self.at_eol() && !self.is_kw("CURSOR") && !self.is_kw("CONNECTION") && !self.is_kw("TABLE") {
            let mut probe = 0;
            let mut from_at = None;
            while !matches!(self.peek_at(probe).kind, TokKind::Newline | TokKind::Eof) {
                if kw(self.peek_at(probe), "FROM") {
                    from_at = Some(probe);
                    break;
                }
                probe += 1;
            }
            if from_at.is_some() {
                let at = self.peek().span;
                let target = if self.is_kw("FROM") {
                    // `CREATE FROM description` asks for the new table's name
                    Expr::new(ExprKind::Str(String::new()), at)
                } else if self.is(&TokKind::LParen) {
                    self.table_name()?
                } else {
                    Expr::new(ExprKind::Str(self.path_before(&["FROM", "DATABASE", "NAME"])), at)
                };
                // DATABASE and NAME put the new table in a database under a long name
                if self.eat_kw("DATABASE") {
                    let _ = self.table_name()?;
                    if self.eat_kw("NAME") {
                        let _ = self.table_name()?;
                    }
                }
                if !self.eat_kw("FROM") {
                    return Err(self.error_here("CREATE ... FROM needs the table the structure came from"));
                }
                let source = if self.at_eol() {
                    Expr::new(ExprKind::Str(String::new()), at)
                } else if self.is(&TokKind::LParen) {
                    self.table_name()?
                } else {
                    Expr::new(ExprKind::Str(self.path_before(&["DATABASE", "NAME"])), at)
                };
                self.skip_line_keep_newline();
                return Ok(Some(StmtKind::CreateFrom { target, source }));
            }
        }
        // the designers: something new of that kind, opened to be worked on
        const DESIGNED: &[&str] = &["FORM", "SCREEN", "MENU", "PROJECT", "QUERY"];
        if let Some(word) = self.peek().ident().map(|s| s.to_ascii_uppercase())
            && let Some(kind) = DESIGNED.iter().find(|k| kw_text(&word, k))
        {
            let at = self.advance().span;
            let path = if self.at_eol() {
                Expr::new(ExprKind::Str(String::new()), at)
            } else {
                self.table_name()?
            };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::NewDocument { what: Name::new(*kind, at), path }));
        }
        if self.eat_kw("CONNECTION") {
            let name = self.table_name()?;
            // CONNSTRING is the whole of what a connection is here; DATASOURCE and the words
            // that go with it name one the operating system holds, which comes to the same
            let mut connect = None;
            let mut flags = 0;
            if self.eat_kw("CONNSTRING") {
                connect = Some(self.expr()?);
            } else if self.eat_kw("DATASOURCE") {
                connect = Some(self.expr()?);
                flags |= crate::ast::db_flags::DATASOURCE;
            }
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Database {
                what: DbCommand::CreateConnection,
                name: Some(name),
                target: connect,
                sql: String::new(),
                flags,
            }));
        }
        if !self.eat_kw("CURSOR") {
            let what = self.peek().ident().map(|s| format!("CREATE {}", s.to_ascii_uppercase()));
            self.skip_line_keep_newline();
            return Err(self.error(first.span, format!("{} is not supported in the FoxDev runtime", what.unwrap_or("CREATE".into()))));
        }
        // the name may be worked out: `CREATE CURSOR (cTmpCursor) (...)`
        let mut named = None;
        let alias = if self.is(&TokKind::LParen) {
            self.advance();
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            let span = e.span;
            named = Some(e);
            Name::new("CURSOR", span)
        } else {
            self.expect_ident("cursor name")?
        };
        if let Some(from_array) = self.from_array_clause()? {
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::CreateCursor { alias, named, fields: Vec::new(), from_array: Some(from_array) }));
        }
        let fields = self.cursor_fields()?;
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::CreateCursor { alias, named, fields, from_array: None }))
    }

    /// `FROM ARRAY aFields`: the columns described by an array rather than written out, in the
    /// shape `AFIELDS()` hands back. Nothing else follows FROM here, so a bare FROM is an error.
    fn from_array_clause(&mut self) -> PResult<Option<Expr>> {
        if !self.eat_kw("FROM") {
            return Ok(None);
        }
        if !self.eat_kw("ARRAY") {
            return Err(self.expected("ARRAY"));
        }
        Ok(Some(self.expr()?))
    }

    /// The parenthesised field list of CREATE CURSOR and CREATE TABLE.
    fn cursor_fields(&mut self) -> PResult<Vec<CursorField>> {
        self.expect(&TokKind::LParen, "'('")?;
        let mut fields = Vec::new();
        loop {
            let name = self.name_ref("field name")?;
            let kind_word = self.expect_ident("field type")?;
            let kind = kind_word.upper.chars().next().unwrap_or('C');
            let (mut width, mut decimals) = default_width(kind);
            if self.eat(&TokKind::LParen) {
                let first = self.expect_number("field width")? as u8;
                // a Double is eight bytes whatever it holds, so the one number `B(4)` carries
                // is how many places it shows, not how wide it is
                if matches!(kind, 'B' | 'O') {
                    decimals = first;
                } else {
                    width = first;
                }
                if self.eat(&TokKind::Comma) {
                    decimals = self.expect_number("decimal places")? as u8;
                }
                self.expect(&TokKind::RParen, "')'")?;
            }
            let (autoinc_next, autoinc_step, nullable) = self.column_clauses(&[TokKind::Comma, TokKind::RParen])?;
            fields.push(CursorField { name, kind, width, decimals, autoinc_next, autoinc_step, nullable });
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        self.expect(&TokKind::RParen, "')'")?;
        Ok(fields)
    }

    /// `INSERT INTO table [(fields)] VALUES (values) | FROM ARRAY a | FROM MEMVAR | FROM NAME o`,
    /// and the older `INSERT [BEFORE] [BLANK]` that has nothing to do with SQL.
    fn insert_stmt(&mut self, first: &Token) -> PResult<StmtKind> {
        self.advance();
        if !self.eat_kw("INTO") {
            // `INSERT`, `INSERT BLANK`, `INSERT BEFORE` and `INSERT BEFORE BLANK` are the whole
            // of the Xbase command; VFP takes no other word after it. Without BLANK it opens an
            // editing window on the new record, and there is no such window here, so the record
            // is simply left empty - which is what APPEND on its own does too.
            let before = self.eat_kw("BEFORE");
            let _ = self.eat_kw("BLANK");
            if !self.at_eol() {
                let what = self.peek().ident().map(|s| s.to_ascii_uppercase()).unwrap_or_default();
                self.skip_line_keep_newline();
                return Err(self.error(first.span, format!("INSERT takes only BEFORE and BLANK, not {what}")));
            }
            self.skip_line_keep_newline();
            return Ok(StmtKind::InsertBlank { before });
        }
        // the table is a name, or an expression in brackets like everywhere else a table is
        // named: `INSERT INTO (lcDBFName) VALUES ...` after CREATE TABLE (lcDBFName)
        let mut named = None;
        let alias = if self.is(&TokKind::LParen) {
            self.advance();
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            let span = e.span;
            named = Some(e);
            Name::new("", span)
        } else {
            self.expect_ident("table name")?
        };
        // `FROM ARRAY | MEMVAR | NAME` reads the whole record from somewhere else, so it names
        // no fields: VFP calls a field list before it a syntax error.
        if self.eat_kw("FROM") {
            let from = self.gather_source(first)?;
            self.skip_line_keep_newline();
            return Ok(StmtKind::Insert { alias, named, fields: Vec::new(), source: InsertSource::From(from) });
        }
        let mut fields = Vec::new();
        if self.is(&TokKind::LParen) && !kw(self.peek_at(1), "VALUES") {
            self.advance();
            loop {
                // a field is a name like any other: the foundation classes write it out, in
                // quotes, and as an expression worked out when the statement runs
                fields.push(self.name_ref("field name")?);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
            self.expect(&TokKind::RParen, "')'")?;
        }
        // `INSERT INTO t (fields) SELECT ...`: the rows come from a query of the program's own
        if self.is_kw("SELECT") {
            let query = self.query(first.span)?;
            return Ok(StmtKind::Insert { alias, named, fields, source: InsertSource::Query(Box::new(query)) });
        }
        if !self.eat_kw("VALUES") {
            let what = self.peek().ident().map(|s| s.to_ascii_uppercase()).unwrap_or_default();
            self.skip_line_keep_newline();
            return Err(self.error(first.span, format!("INSERT ... {what} is not supported; only VALUES, FROM and SELECT are")));
        }
        self.expect(&TokKind::LParen, "'('")?;
        let mut values = Vec::new();
        loop {
            values.push(self.expr()?);
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        self.expect(&TokKind::RParen, "')'")?;
        self.skip_line_keep_newline();
        Ok(StmtKind::Insert { alias, named, fields, source: InsertSource::Values(values) })
    }

    fn expect_number(&mut self, what: &str) -> PResult<f64> {
        match self.peek_kind() {
            TokKind::Num(n, ..) => {
                self.advance();
                Ok(n)
            }
            _ => Err(self.expected(what)),
        }
    }

    /// `MODIFY COMMAND|FILE|FORM|CLASS|MENU|PROJECT|DATABASE|REPORT|LABEL|STRUCTURE path`.
    ///
    /// These ask the development environment to open something for editing, and this *is* a
    /// development environment, so they open it. The forms that name no file - MODIFY WINDOW,
    /// MODIFY MEMO - draw on a screen that is not here, and are dropped.
    fn modify_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        const EDITABLE: &[&str] =
            &["COMMAND", "FILE", "FORM", "CLASS", "MENU", "PROJECT", "DATABASE", "REPORT", "LABEL", "PROCEDURE", "QUERY", "VIEW", "STRUCTURE"];
        let Some(word) = self.peek().ident().map(|s| s.to_string()) else {
            self.skip_line_keep_newline();
            return Ok(None);
        };
        if kw_text(&word, "WINDOW") {
            self.advance();
            let name = if self.at_eol() { None } else { Some(self.menu_name()?) };
            let (corners, title, text) = self.window_clauses()?;
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::WindowCommand {
                        read: None,
                what: WindowVerb::Modify,
                name,
                corners,
                title,
                text,
                flags: 0,
            }));
        }
        if kw_text(&word, "MEMO") {
            self.advance();
            return self.memo_stmt(2);
        }
        if kw_text(&word, "CONNECTION") {
            self.advance();
            let name = if self.at_eol() {
                Expr::new(ExprKind::Str(String::new()), first.span)
            } else {
                self.table_name()?
            };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Database {
                what: DbCommand::ModifyConnection,
                name: Some(name),
                target: None,
                sql: String::new(), flags: 0 }));
        }
        let Some(what) = EDITABLE.iter().find(|k| kw_text(&word, k)) else {
            self.warning(first.span, format!("MODIFY {} is ignored: it draws on a Visual FoxPro screen this runtime does not have", word.to_ascii_uppercase()));
            self.skip_line_keep_newline();
            return Ok(None);
        };
        let name = Name::new(*what, self.advance().span);
        // MODIFY DATABASE is the Database Designer, which this runtime has not got; what it
        // does have is dbc_ModifyData, which the open database hears with the two clauses the
        // command was written with - measured, with NOWAIT so the product came back at all.
        if kw_text(what, "DATABASE") {
            let path = if self.at_eol() || self.is_copy_word() {
                Expr::new(ExprKind::Str(String::new()), first.span)
            } else {
                self.table_name()?
            };
            let flags = self.db_clauses();
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Modify { what: name, path, flags }));
        }
        if self.at_eol() {
            // MODIFY STRUCTURE is about the table in hand and MODIFY PROCEDURE about the open
            // database, so neither names a file; the rest have nothing to open without one
            let path = match *what {
                "STRUCTURE" => Expr::new(ExprKind::Call { name: Name::new("DBF", first.span), args: Vec::new() }, first.span),
                "PROCEDURE" => Expr::new(ExprKind::Str(String::new()), first.span),
                _ => {
                    self.warning(first.span, format!("MODIFY {what} with no file name has nothing to open here"));
                    return Ok(None);
                }
            };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Modify { what: name, path, flags: 0 }));
        }
        // the file is a path as written, or an expression in brackets, as everywhere else
        let path = if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            e
        } else {
            Expr::new(ExprKind::Str(self.bare_path()), first.span)
        };
        // NOWAIT, NOEDIT, SAVE and the rest say how VFP would show it, which is not up to us
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Modify { what: name, path, flags: 0 }))
    }

    /// `IMPORT FROM FileName [DATABASE db [NAME long]] [TYPE] <type> [SHEET name] [AS n]`.
    ///
    /// The workbook formats are the ones anything still reads. The rest - Framework II,
    /// Multiplan, Paradox, RapidFile, Lotus 1-2-3 and Symphony - are refused by name.
    fn import_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if !self.eat_kw("FROM") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "IMPORT needs FROM and a file name"));
        }
        let path = self.built_file(first);
        if self.eat_kw("DATABASE") {
            let _ = self.table_name();
            if self.eat_kw("NAME") {
                let _ = self.table_name();
            }
            self.error(
                first.span,
                "IMPORT ... DATABASE is not supported in the FoxDev runtime: the table is made \
                 free; ADD TABLE puts it in a database once it is there",
            );
            self.skip_line_keep_newline();
            return Ok(None);
        }
        let _ = self.eat_kw("TYPE");
        const WORKBOOK: &[&str] = &["XL8", "XL5", "XLS"];
        const OTHERS: &[&str] = &["FW2", "MOD", "PDOX", "RPD", "WK1", "WK3", "WKS", "WR1", "WRK"];
        let word = self.peek().ident().unwrap_or_default().to_string();
        if let Some(kind) = OTHERS.iter().find(|k| word.eq_ignore_ascii_case(k)) {
            self.error(
                first.span,
                format!(
                    "IMPORT ... TYPE {kind} is not supported in the FoxDev runtime: it is a \
                     Framework II, Multiplan, Paradox, RapidFile, Lotus 1-2-3 or Symphony file, \
                     and nothing reads those any more; a workbook - XLS, XL5 or XL8 - is read"
                ),
            );
            self.skip_line_keep_newline();
            return Ok(None);
        }
        if !WORKBOOK.iter().any(|k| word.eq_ignore_ascii_case(k)) {
            self.error(first.span, "IMPORT needs a TYPE: XLS, XL5 or XL8");
            self.skip_line_keep_newline();
            return Ok(None);
        }
        self.advance();
        let sheet = if self.eat_kw("SHEET") { Some(self.expr()?) } else { None };
        if self.eat_kw("AS") {
            // the code page the file is read in, which this runtime works out from the bytes
            let _ = self.expr();
        }
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Import { path, sheet }))
    }

    /// `EXPORT TO FileName [TYPE] <type> [FIELDS list] [Scope] [FOR x] [WHILE x] [NOOPTIMIZE]`.
    ///
    /// DIF and SYLK are text and are written; the spreadsheet binaries are refused by name.
    fn export_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if !self.eat_kw("TO") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "EXPORT needs TO and a file name"));
        }
        let path = self.table_name()?;
        let _ = self.eat_kw("TYPE");
        const WRITTEN: &[(&str, TextFormat)] = &[("DIF", TextFormat::Dif), ("SYLK", TextFormat::Sylk)];
        const BINARY: &[&str] = &["MOD", "WK1", "WKS", "WR1", "WRK", "XL5", "XLS"];
        let word = self.peek().ident().unwrap_or_default().to_string();
        if let Some(kind) = BINARY.iter().find(|k| word.eq_ignore_ascii_case(k)) {
            self.error(
                first.span,
                format!(
                    "EXPORT ... TYPE {kind} is not supported in the FoxDev runtime: writing a \
                     Multiplan, Lotus 1-2-3, Symphony or Excel worksheet needs that program's own \
                     format; DIF and SYLK are written, and COPY TO ... TYPE CSV is read by all of them"
                ),
            );
            self.skip_line_keep_newline();
            return Ok(None);
        }
        let Some((_, text)) = WRITTEN.iter().find(|(k, _)| word.eq_ignore_ascii_case(k)) else {
            self.error(first.span, "EXPORT needs a TYPE: DIF or SYLK");
            self.skip_line_keep_newline();
            return Ok(None);
        };
        self.advance();
        let (fields, except) = self.field_clause()?;
        let (scope, cond, while_) = self.record_clauses();
        let (fields2, except2) = self.field_clause()?;
        while self.eat_kw("NOOPTIMIZE") || self.eat_kw("AS") {
            if matches!(self.peek_kind(), TokKind::Num(..)) || self.peek().ident().is_some() {
                let _ = self.expr();
            }
        }
        self.skip_line_keep_newline();
        let fields = if fields.is_empty() { fields2 } else { fields };
        let except = if except.is_empty() { except2 } else { except };
        Ok(Some(StmtKind::CopyTo {
            path,
            kind: CopyKind::Records,
            fields,
            except,
            scope,
            cond,
            while_,
            keys: Vec::new(),
            text: Some(text.clone()),
        }))
    }

    /// `BUILD APP | EXE | DLL | MTDLL FileName FROM ProjectName [RECOMPILE]`, and
    /// `BUILD PROJECT FileName [RECOMPILE] [FROM file1 [, file2 ...]]`, which makes the
    /// project itself out of the files it names.
    fn build_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        const KINDS: &[&str] = &["APP", "EXE", "MTDLL", "DLL", "PROJECT"];
        let word = self.peek().ident().unwrap_or_default().to_string();
        let Some(kind) = KINDS.iter().find(|k| kw_text(&word, k)) else {
            self.error(
                first.span,
                format!("BUILD {} is not one of APP, EXE, DLL, MTDLL and PROJECT", word.to_ascii_uppercase()),
            );
            self.skip_line_keep_newline();
            return Ok(None);
        };
        // an in-process COM server is a .dll Windows loads into the calling program, which is
        // not a thing this runtime can be built into; the same project builds as an application
        if *kind == "DLL" || *kind == "MTDLL" {
            let what = format!(
                "BUILD {kind} builds an in-process COM server, which is a .dll Windows loads into \
                 another program; BUILD EXE makes the same project into an application that runs \
                 on its own"
            );
            // said as a warning, and compiled: the product compiles the line and fails only if it
            // is reached, and refusing here would take every other line of the method with it
            self.warning(first.span, what.clone());
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::Unsupported(what)));
        }
        let what = Name::new(*kind, self.advance().span);
        let target = self.built_file(first);
        // RECOMPILE comes before FROM in BUILD PROJECT and after it in the others
        let mut recompile = self.eat_kw("RECOMPILE");
        let mut from = Vec::new();
        if self.eat_kw("FROM") {
            loop {
                from.push(self.built_file(first));
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        recompile |= self.eat_kw("RECOMPILE");
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Build { what, target, from, recompile }))
    }

    /// `COMPILE [DATABASE | FORM | CLASSLIB | LABEL | REPORT] FileName [ALL]`, and
    /// `COMPILE FileName [ENCRYPT] [NODEBUG] [AS nCodePage]` for a program.
    fn compile_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        const KINDS: &[&str] = &["DATABASE", "CLASSLIB", "REPORT", "LABEL", "FORM"];
        let word = self.peek().ident().unwrap_or_default().to_string();
        // `COMPILE report.prg` compiles a program that happens to be called report: a word
        // carrying its own extension is the file, as it is everywhere else a file is named
        let end = self.peek().span.end;
        let extended = matches!(self.peek_at(1).kind, TokKind::Dot) && self.peek_at(1).span.start == end;
        let what = match KINDS.iter().find(|k| !extended && kw_text(&word, k)) {
            Some(k) => Name::new(*k, self.advance().span),
            None => Name::new(String::new(), first.span),
        };
        let files = self.built_file(first);
        let mut flags = 0u8;
        loop {
            if self.eat_kw("ALL") {
                flags |= compile_flags::ALL;
            } else if self.eat_kw("ENCRYPT") {
                flags |= compile_flags::ENCRYPT;
            } else if self.eat_kw("NODEBUG") {
                flags |= compile_flags::NODEBUG;
            } else if self.eat_kw("AS") {
                // the code page the source is read in, which this runtime works out from the
                // bytes themselves
                let _ = self.expr();
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Compile { what, files, flags }))
    }

    /// The file a build or compile command names: `?` for the dialog, an expression in
    /// brackets, or a path as written that stops at the first word of a clause.
    fn built_file(&mut self, first: &Token) -> Expr {
        const WORDS: &[&str] =
            &["FROM", "RECOMPILE", "ALL", "ENCRYPT", "NODEBUG", "AS", "TYPE", "DATABASE", "NAME", "SHEET"];
        if let Some(ask) = self.ask_for_file("") {
            return ask;
        }
        if self.eat(&TokKind::LParen) {
            let inner = self.expr();
            let _ = self.expect(&TokKind::RParen, "')'");
            if let Ok(e) = inner {
                return e;
            }
            return Expr::new(ExprKind::Str(String::new()), first.span);
        }
        Expr::new(ExprKind::Str(self.path_before(WORDS)), first.span)
    }

    /// `SCAN [scope] [FOR x] [WHILE x] ... ENDSCAN`.
    fn scan_stmt(&mut self, first: &Token) -> PResult<StmtKind> {
        self.advance();
        let (scope, cond, while_) = self.record_clauses();
        self.end_of_line();
        let (body, term) = self.block_until(&["ENDSCAN"], first, "SCAN");
        if term.is_some() {
            self.advance();
        }
        Ok(StmtKind::Scan { scope, cond, while_, body })
    }

    /// The scope, FOR and WHILE clauses a record command takes, in whatever order they come.
    fn record_clauses(&mut self) -> (Scope, Option<Expr>, Option<Expr>) {
        let (scope, cond, while_, _) = self.record_clauses_named();
        (scope, cond, while_)
    }

    /// The same, and whether the line named a scope at all - which is what tells `DISPLAY` apart
    /// from `DISPLAY ALL`.
    fn record_clauses_named(&mut self) -> (Scope, Option<Expr>, Option<Expr>, bool) {
        let mut scope = Scope::All;
        let mut named = false;
        let mut cond = None;
        let mut while_ = None;
        loop {
            if self.eat_kw("ALL") {
                scope = Scope::All;
                named = true;
            } else if self.eat_kw("REST") {
                scope = Scope::Rest;
                named = true;
            } else if self.eat_kw("NEXT") {
                scope = Scope::Next(self.expr_or_recover());
                named = true;
            } else if self.eat_kw("RECORD") {
                scope = Scope::Record(self.expr_or_recover());
                named = true;
            } else if self.eat_kw("FOR") {
                cond = Some(self.expr_or_recover());
            } else if self.eat_kw("WHILE") {
                while_ = Some(self.expr_or_recover());
            } else if self.eat_kw("NOOPTIMIZE") {
                // Rushmore is VFP's index optimiser; there is nothing here to turn off
            } else if self.eat_kw("IN") {
                let _ = self.advance();
            } else {
                break;
            }
        }
        (scope, cond, while_, named)
    }

    /// `ALTER TABLE name ADD COLUMN f T(n) | ALTER COLUMN f T(n) | DROP COLUMN f |
    /// RENAME COLUMN a TO b`, one change or several separated by commas.
    fn alter_table_stmt(&mut self) -> PResult<StmtKind> {
        let first = self.advance();
        if !self.eat_kw("TABLE") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "ALTER needs TABLE and a table name"));
        }
        let path = self.table_name()?;
        let mut ops = Vec::new();
        loop {
            if self.is_kw("ADD") || self.is_kw("ALTER") {
                let adding = self.is_kw("ADD");
                self.advance();
                let _ = self.eat_kw("COLUMN");
                let field = self.one_column()?;
                ops.push(if adding { AlterOp::Add(field) } else { AlterOp::Alter(field) });
            } else if self.eat_kw("DROP") {
                let _ = self.eat_kw("COLUMN");
                ops.push(AlterOp::Drop(self.name_ref("column name")?));
            } else if self.eat_kw("RENAME") {
                let _ = self.eat_kw("COLUMN");
                let from = self.name_ref("column name")?;
                let _ = self.eat_kw("TO");
                let to = self.name_ref("new column name")?;
                ops.push(AlterOp::Rename(from, to));
            } else if self.eat(&TokKind::Comma) {
                continue;
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        if ops.is_empty() {
            return Err(self.error(first.span, "ALTER TABLE needs a column to add, alter, drop or rename"));
        }
        Ok(StmtKind::AlterTable { path, ops })
    }

    /// One column definition, as ADD COLUMN and ALTER COLUMN write it.
    fn one_column(&mut self) -> PResult<CursorField> {
        let name = self.name_ref("column name")?;
        let kind_word = self.expect_ident("column type")?;
        let kind = kind_word.upper.chars().next().unwrap_or('C');
        let (mut width, mut decimals) = default_width(kind);
        if self.eat(&TokKind::LParen) {
            let first = self.expect_number("column width")? as u8;
            // `B(4)` is a Double showing four places, not a Double four bytes wide
            if matches!(kind, 'B' | 'O') {
                decimals = first;
            } else {
                width = first;
            }
            if self.eat(&TokKind::Comma) {
                decimals = self.expect_number("decimal places")? as u8;
            }
            self.expect(&TokKind::RParen, "')'")?;
        }
        let (autoinc_next, autoinc_step, nullable) = self.column_clauses(&[TokKind::Comma])?;
        Ok(CursorField { name, kind, width, decimals, autoinc_next, autoinc_step, nullable })
    }

    /// The clauses a column carries after its type, up to one of `ends` or the end of the line.
    ///
    /// AUTOINC is one of the two that change what the table does: the table fills the field in
    /// itself, counting from NEXTVALUE by STEP. NULL and NOT NULL are the other - they say
    /// whether the column takes `.NULL.`, and a column that says neither takes whatever
    /// `SET NULL` is. CHECK, DEFAULT and the rest describe rules this runtime does not keep,
    /// and are read and dropped.
    fn column_clauses(&mut self, ends: &[TokKind]) -> PResult<(u32, u8, Option<bool>)> {
        let (mut next, mut step) = (0u32, 0u8);
        let mut nullable = None;
        while !self.at_eol() && !ends.iter().any(|e| self.is(e)) && !self.is_alter_word() {
            if self.is_kw("NULL") {
                self.advance();
                nullable = Some(true);
                continue;
            }
            // NOT is only ever the start of NOT NULL here; NOT NOCPTRANS is not a thing
            if self.is_kw("NOT") && kw(self.peek_at(1), "NULL") {
                self.advance();
                self.advance();
                nullable = Some(false);
                continue;
            }
            if self.is_kw("AUTOINC") {
                self.advance();
                next = 1;
                step = 1;
                if self.eat_kw("NEXTVALUE") {
                    next = self.expect_number("the value to start at")?.max(0.0) as u32;
                }
                if self.eat_kw("STEP") {
                    step = (self.expect_number("the step")? as u8).max(1);
                }
                continue;
            }
            self.advance();
        }
        Ok((next, step, nullable))
    }

    /// A word that starts the next change of an ALTER TABLE.
    fn is_alter_word(&mut self) -> bool {
        let tok = self.peek().clone();
        ["ADD", "ALTER", "DROP", "RENAME"].iter().any(|k| kw(&tok, k))
    }

    /// `RUN cCommand`, written out or as `!`: a command line for the operating system.
    fn run_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        // `RUN /N notepad` and `RUN /N7 x` say how the window is shown, which is the host's to
        // decide; what is left is the command line
        let nowait = self.is_kw("N") || matches!(self.peek_kind(), TokKind::Slash);
        let command = match self.rest_of_line_text() {
            Some(text) => text.trim_start_matches('/').trim_start_matches(['N', 'n']).trim().to_string(),
            None => String::new(),
        };
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Run { command: Expr::new(ExprKind::Str(command), first.span), nowait }))
    }

    /// `JOIN WITH alias TO file FOR lCond [FIELDS list]`: a table made of the records of this
    /// one and another that answer the condition. The reference says to write it as a query
    /// instead, so that is what it becomes - one row for every pair the condition keeps.
    fn join_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if !self.eat_kw("WITH") {
            self.error(first.span, "JOIN needs WITH and the table to join to");
            self.skip_line_keep_newline();
            return Ok(None);
        }
        let other = self.query_source(JoinKind::Inner)?;
        if !self.eat_kw("TO") {
            self.error(first.span, "JOIN needs TO and the table to make");
            self.skip_line_keep_newline();
            return Ok(None);
        }
        let into = self.table_name()?;
        let where_ = self.eat_kw("FOR").then(|| self.expr()).transpose()?;
        let mut columns = Vec::new();
        if self.eat_kw("FIELDS") {
            loop {
                let expr = self.expr()?;
                columns.push(QueryColumn::Value { expr, name: None });
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        // NOOPTIMIZE says how VFP reads the records, which changes nothing about the answer
        self.skip_line_keep_newline();
        if columns.is_empty() {
            columns.push(QueryColumn::All(None));
        }
        // the table it joins from is the one that is selected, whichever that turns out to be
        let alias_now = Expr::new(ExprKind::Call { name: Name::new("ALIAS", first.span), args: Vec::new() }, first.span);
        let here = QuerySource {
            table: String::new(),
            table_expr: Some(alias_now),
            alias: Name::new(String::new(), first.span),
            join: JoinKind::Inner,
            joined: false,
            on: None,
            span: first.span,
        };
        Ok(Some(StmtKind::Query(Box::new(Query {
            distinct: false,
            top: None,
            top_percent: false,
            columns,
            from: vec![here, other],
            where_,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            into: QueryInto::Table(into),
            union: None,
            span: first.span,
        }))))
    }

    /// `MOUSE [CLICK | DBLCLICK] [AT r, c] [DRAG TO r, c ...] [PIXELS] [WINDOW name] [...]`.
    fn mouse_stmt(&mut self) -> PResult<Option<StmtKind>> {
        self.advance();
        let clicks = if self.eat_kw("DBLCLICK") {
            2
        } else if self.eat_kw("CLICK") {
            1
        } else {
            0
        };
        let pair = |p: &mut Self| -> PResult<(Expr, Expr)> {
            let row = p.expr()?;
            p.expect(&TokKind::Comma, "','")?;
            let col = p.expr()?;
            Ok((row, col))
        };
        let at = if self.eat_kw("AT") { Some(pair(self)?) } else { None };
        let mut drag = Vec::new();
        if self.eat_kw("DRAG") {
            self.eat_kw("TO");
            loop {
                drag.push(pair(self)?);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        let mut window = None;
        let mut words: Vec<String> = Vec::new();
        const MOUSE_WORDS: &[&str] = &["PIXELS", "LEFT", "MIDDLE", "RIGHT", "SHIFT", "CONTROL", "ALT"];
        while let Some(word) = self.peek().ident().map(|s| s.to_ascii_uppercase()) {
            if kw_text(&word, "WINDOW") {
                self.advance();
                window = Some(self.menu_name()?);
            } else if MOUSE_WORDS.iter().any(|w| kw_text(&word, w)) {
                self.advance();
                words.push(MOUSE_WORDS.iter().find(|w| kw_text(&word, w)).unwrap().to_string());
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Mouse { clicks, at, drag, window, style: words.join(" ") }))
    }

    /// `INPUT [cPrompt] TO var`, `ACCEPT [cPrompt] TO var` and `GETEXPR [cPrompt] TO var`.
    fn ask_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        let word = first.ident().unwrap_or_default().to_ascii_uppercase();
        self.advance();
        let prompt = if self.is_kw("TO") { None } else { Some(self.expr()?) };
        if !self.eat_kw("TO") {
            self.warning(first.span, format!("{word} with nothing to put the answer in does nothing"));
            self.skip_line_keep_newline();
            return Ok(None);
        }
        let target = self.postfix_expr()?;
        // GETEXPR also says what type the expression has to have, which nothing here checks
        self.skip_line_keep_newline();
        let kind = match word.as_str() {
            "INPUT" => 1,
            "GETEXPR" => 2,
            _ => 0,
        };
        Ok(Some(StmtKind::Ask { prompt, target, kind }))
    }

    /// `BROWSE [FIELDS a, b] [FOR x] [TITLE t] [NOWAIT] [NOEDIT] [...]`: the records of the
    /// work area, shown in a window of their own.
    fn browse_stmt(&mut self) -> PResult<Option<StmtKind>> {
        self.advance();
        let mut flags = 0u8;
        let mut title = None;
        let (mut fields, _) = self.field_clause()?;
        loop {
            if self.eat_kw("NOWAIT") {
                flags |= 1;
            } else if self.eat_kw("NOEDIT") || self.eat_kw("NOMODIFY") || self.eat_kw("NOAPPEND") || self.eat_kw("NODELETE") {
                flags |= 2;
            } else if self.eat_kw("NOCLOSE") {
                flags |= 4;
            } else if self.eat_kw("TITLE") {
                title = Some(self.expr()?);
            } else if self.eat_kw("FIELDS") {
                let (more, _) = self.field_clause()?;
                fields.extend(more);
            } else if self.eat_kw("KEY")
                || self.eat_kw("WIDTH")
                || self.eat_kw("FONT")
                || self.eat_kw("STYLE")
                || self.eat_kw("COLOR")
                || self.eat_kw("IN")
                || self.eat_kw("WINDOW")
                || self.eat_kw("NAME")
                || self.eat_kw("WHEN")
                || self.eat_kw("VALID")
                || self.eat_kw("FREEZE")
                || self.eat_kw("TIMEOUT")
            {
                let _ = self.expr();
                if self.eat(&TokKind::Comma) {
                    let _ = self.expr();
                }
            } else if matches!(self.peek_kind(), TokKind::Ident(_)) && !self.is_record_clause() {
                // the rest say how the window looks, which the host decides for itself
                self.advance();
            } else {
                break;
            }
        }
        let mut cond = String::new();
        if self.eat_kw("FOR") {
            let start = self.peek().span.start;
            let _ = self.expr()?;
            cond = self.source_text(start, self.prev_span().end);
        }
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Browse { fields, cond, title, flags }))
    }

    /// `REPORT FORM <file> [scope] [FOR x] [PREVIEW | TO PRINTER | TO FILE y] [NOCONSOLE]`,
    /// and `LABEL FORM` which reads a label file the same way.
    fn report_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        let label = kw(first, "LABEL");
        self.advance();
        if !self.eat_kw("FORM") {
            let verb = first.ident().unwrap_or_default().to_ascii_uppercase();
            self.warning(first.span, format!("{verb} without FORM is ignored: there is nothing else to run"));
            self.skip_line_keep_newline();
            return Ok(None);
        }
        let path = if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            e
        } else {
            Expr::new(ExprKind::Str(self.report_path()), first.span)
        };
        let mut flags = 0u8;
        let mut to_file = None;
        loop {
            if self.eat_kw("PREVIEW") {
                flags |= 1;
                // PREVIEW IN WINDOW <name> / IN SCREEN: where it is shown is the host's business
                if self.eat_kw("IN") {
                    let _ = self.eat_kw("WINDOW") || self.eat_kw("SCREEN");
                    if !self.at_eol() {
                        let _ = self.menu_word();
                    }
                }
            } else if self.eat_kw("NOCONSOLE") {
                flags |= 4;
            } else if self.eat_kw("SUMMARY") {
                flags |= 8;
            } else if self.eat_kw("PLAIN") {
                flags |= 16;
            } else if self.eat_kw("TO") {
                if self.eat_kw("PRINTER") {
                    flags |= 2;
                    self.eat_kw("PROMPT");
                } else if self.eat_kw("FILE") {
                    to_file = Some(if self.eat(&TokKind::LParen) {
                        let e = self.expr()?;
                        self.expect(&TokKind::RParen, "')'")?;
                        e
                    } else {
                        Expr::new(ExprKind::Str(self.report_path()), first.span)
                    });
                    self.eat_kw("ASCII");
                    self.eat_kw("ADDITIVE");
                }
            } else if self.eat_kw("HEADING") || self.eat_kw("NAME") || self.eat_kw("OBJECT") {
                let _ = self.expr()?;
            } else if self.eat_kw("RANGE") {
                let _ = self.expr()?;
                if self.eat(&TokKind::Comma) {
                    let _ = self.expr()?;
                }
            } else if self.eat_kw("NOOPTIMIZE")
                || self.eat_kw("NODIALOG")
                || self.eat_kw("NORESET")
                || self.eat_kw("NOPAGEEJECT")
                || self.eat_kw("NOEJECT")
                || self.eat_kw("ENVIRONMENT")
                || self.eat_kw("SAMPLE")
                || self.eat_kw("ASCII")
            {
                // how the report is set up and shown, which the host decides for itself
            } else {
                break;
            }
        }
        let (scope, cond, while_) = self.record_clauses();
        // the clauses can come either side of the scope, so read the rest of them too
        loop {
            if self.eat_kw("PREVIEW") {
                flags |= 1;
            } else if self.eat_kw("NOCONSOLE") {
                flags |= 4;
            } else if self.eat_kw("SUMMARY") {
                flags |= 8;
            } else if self.eat_kw("PLAIN") {
                flags |= 16;
            } else if self.eat_kw("TO") {
                if self.eat_kw("PRINTER") {
                    flags |= 2;
                    self.eat_kw("PROMPT");
                } else if self.eat_kw("FILE") {
                    to_file = Some(Expr::new(ExprKind::Str(self.report_path()), first.span));
                }
            } else if matches!(self.peek_kind(), TokKind::Ident(_)) && !self.at_eol() {
                self.advance();
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::ReportForm { path, label, scope, cond, while_, to_file, flags }))
    }

    /// The clauses a window command carries: where it goes, what it is called, and the words
    /// that say how it looks and what it may do.
    fn window_clauses(&mut self) -> PResult<(Vec<Expr>, Option<Expr>, String)> {
        let mut corners = Vec::new();
        let mut title = None;
        let mut words = Vec::new();
        loop {
            if self.eat_kw("FROM") || self.eat_kw("AT") || self.eat_kw("TO") || self.eat_kw("SIZE") {
                corners.push(self.expr()?);
                if self.eat(&TokKind::Comma) {
                    corners.push(self.expr()?);
                }
            } else if self.eat_kw("TITLE") {
                title = Some(self.expr()?);
            } else if self.eat_kw("FOOTER") {
                let _ = self.expr()?;
            } else if self.eat_kw("IN") {
                // IN WINDOW <name>: which window this one sits inside, kept apart from the
                // style words with a marker `window_command` strips back out - `IN SCREEN` and
                // `IN DESKTOP` name no window, and leave the window with no parent, same as
                // saying nothing at all.
                let _ = self.eat_kw("WINDOW") || self.eat_kw("SCREEN") || self.eat_kw("DESKTOP") || self.eat_kw("MACDESKTOP");
                if !self.at_eol() && !self.is_window_word() {
                    words.push(format!("\u{1}{}", self.menu_word()));
                }
            } else if self.eat_kw("COLOR") || self.eat_kw("FONT") || self.eat_kw("STYLE") || self.eat_kw("ICON") || self.eat_kw("FILL") {
                // colours, fonts and icons: the host draws a window its own way
                self.skip_line_keep_newline();
                break;
            } else if let Some(word) = self.peek().ident().map(|s| s.to_ascii_uppercase())
                && WINDOW_WORDS.iter().any(|w| kw_text(&word, w))
            {
                self.advance();
                words.push(word);
            } else {
                break;
            }
        }
        Ok((corners, title, words.join(" ")))
    }

    /// True when the next word is one of the words that say how a window looks, rather than
    /// something the clause before it was going to take.
    fn is_window_word(&mut self) -> bool {
        let Some(word) = self.peek().ident().map(|s| s.to_string()) else { return false };
        WINDOW_WORDS.iter().chain(["TITLE", "FOOTER", "FROM", "TO", "AT", "IN", "SIZE"].iter()).any(|w| kw_text(&word, w))
    }

    /// `DEFINE WINDOW <name> FROM r1, c1 TO r2, c2 [TITLE ...] [...]`.
    fn define_window_stmt(&mut self) -> PResult<Option<StmtKind>> {
        self.advance();
        self.advance();
        let name = Some(self.menu_name()?);
        let (corners, title, text) = self.window_clauses()?;
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::WindowCommand { what: WindowVerb::Define, name, corners, title, text, flags: 0, read: None }))
    }

    /// `ACTIVATE`, `DEACTIVATE`, `SHOW` and `HIDE` of a window or of the screen itself.
    fn window_visibility_stmt(&mut self, verb: &str, screen: bool) -> PResult<Option<StmtKind>> {
        self.advance();
        self.advance();
        let mut flags = if screen { 4 } else { 0 };
        let mut names = Vec::new();
        if !screen {
            if self.is_kw("ALL") {
                self.advance();
                flags |= 1;
            } else {
                while !self.at_eol() && !self.is_window_word() && !self.is_kw("NOSHOW") && !self.is_kw("SAME") && !self.is_kw("BOTTOM") && !self.is_kw("TOP") {
                    names.push(self.menu_name()?);
                    if !self.eat(&TokKind::Comma) {
                        break;
                    }
                }
            }
        }
        if self.eat_kw("NOSHOW") {
            flags |= 2;
        }
        let (corners, title, text) = self.window_clauses()?;
        self.skip_line_keep_newline();
        let what = if screen {
            WindowVerb::ActivateScreen
        } else if verb.starts_with("ACTI") {
            WindowVerb::Activate
        } else if verb.starts_with("DEAC") {
            WindowVerb::Deactivate
        } else if verb.starts_with("SHOW") {
            WindowVerb::Show
        } else {
            WindowVerb::Hide
        };
        // several names on one line are several commands; the first is the one that matters
        let name = names.into_iter().next();
        Ok(Some(StmtKind::WindowCommand { what, name, corners, title, text, flags, read: None }))
    }

    /// `MOVE WINDOW`, `SIZE WINDOW` and `ZOOM WINDOW`. The same words move a popup, which
    /// this runtime draws where the host puts it.
    fn move_window_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        let verb = first.ident().unwrap_or_default().to_ascii_uppercase();
        let word = self.peek_at(1).ident().unwrap_or_default().to_ascii_uppercase();
        if !kw_text(&word, "WINDOW") && !kw_text(&word, "POPUP") {
            self.warning(
                first.span,
                format!("{verb} is ignored: it draws on a Visual FoxPro screen this runtime does not have"),
            );
            self.skip_line_keep_newline();
            return Ok(None);
        }
        let popup = kw_text(&word, "POPUP");
        self.advance();
        self.advance();
        let name = if self.at_eol() { None } else { Some(self.menu_name()?) };
        let (corners, title, text) = self.window_clauses()?;
        self.skip_line_keep_newline();
        if popup {
            // where a popup sits is the host's business, but the command is a command
            return Ok(None);
        }
        let what = if verb.starts_with("MOVE") {
            WindowVerb::Move
        } else if verb.starts_with("SIZE") {
            WindowVerb::Size
        } else {
            WindowVerb::Zoom
        };
        Ok(Some(StmtKind::WindowCommand { what, name, corners, title, text, flags: 0, read: None }))
    }

    /// `SAVE SCREEN`, `SAVE WINDOW`, and the two that put them back. `SAVE TO` and
    /// `RESTORE FROM` keep variables in a file, which is not this.
    fn save_restore_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        let verb = first.ident().unwrap_or_default().to_ascii_uppercase();
        let word = self.peek_at(1).ident().unwrap_or_default().to_ascii_uppercase();
        let save = verb.starts_with("SAVE");
        // `SAVE TO file` and `RESTORE FROM file` are the memory variables rather than the screen
        if (save && word == "TO") || (!save && kw_text(&word, "FROM")) {
            return self.variables_stmt(save);
        }
        let screen = kw_text(&word, "SCREEN");
        if !screen && !kw_text(&word, "WINDOWS") {
            self.error(first.span, format!("{verb} {word} is not supported in the FoxDev runtime"));
            self.skip_line_keep_newline();
            return Ok(None);
        }
        self.advance();
        self.advance();
        // SAVE SCREEN TO <var> and SAVE WINDOW <name> TO <var> both keep it under a name
        let mut name = None;
        if !screen && !self.at_eol() && !self.is_kw("TO") && !self.is_kw("ALL") {
            name = Some(self.menu_name()?);
        }
        self.eat_kw("ALL");
        if self.eat_kw("TO") || self.eat_kw("FROM") {
            let target = self.menu_name()?;
            if name.is_none() {
                name = Some(target);
            }
        }
        self.skip_line_keep_newline();
        let what = match (verb.starts_with("SAVE"), screen) {
            (true, true) => WindowVerb::SaveScreen,
            (true, false) => WindowVerb::SaveWindows,
            (false, true) => WindowVerb::RestoreScreen,
            (false, false) => WindowVerb::RestoreWindows,
        };
        Ok(Some(StmtKind::WindowCommand {
                        read: None,
            what,
            name,
            corners: Vec::new(),
            title: None,
            text: String::new(),
            flags: if screen { 4 } else { 0 },
        }))
    }

    /// The memo commands: `APPEND MEMO f FROM file [OVERWRITE]`, `COPY MEMO f TO file
    /// [ADDITIVE]`, `MODIFY MEMO f1, f2 [NOEDIT] [NOWAIT]` and `CLOSE MEMO f1, f2 | ALL`.
    fn memo_stmt(&mut self, what: u8) -> PResult<Option<StmtKind>> {
        let mut fields = Vec::new();
        let mut flags = 0u8;
        if what == 3 && self.eat_kw("ALL") {
            flags |= 8;
        } else {
            loop {
                let mut name = self.expect_ident("memo field")?;
                // a field of a table open somewhere else is written with its alias
                if self.eat(&TokKind::Dot) {
                    let field = self.expect_ident("memo field")?;
                    name = Name::new(format!("{}.{}", name.text, field.text), name.span);
                }
                fields.push(name);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        let mut path = None;
        if self.eat_kw("FROM") || self.eat_kw("TO") {
            path = Some(self.table_name()?);
        }
        loop {
            if self.eat_kw("OVERWRITE") || self.eat_kw("ADDITIVE") {
                flags |= 1;
            } else if self.eat_kw("NOEDIT") {
                flags |= 2;
            } else if self.eat_kw("NOWAIT") {
                flags |= 4;
            } else if self.eat_kw("RANGE") {
                let _ = self.expr()?;
                if self.eat(&TokKind::Comma) {
                    let _ = self.expr()?;
                }
            } else if self.eat_kw("AS") {
                // the code page the file is in, which this runtime reads for itself
                let _ = self.expr()?;
            } else {
                break;
            }
        }
        // NOMENU, WINDOW, IN, SAME and SAVE say how VFP would show the window
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Memo { what, fields, field_expr: None, path, flags }))
    }

    /// A name skeleton: `g*`, `a?c`, or an expression in brackets. The wildcards are
    /// punctuation to the lexer, so what is written is taken as it stands.
    fn skeleton(&mut self) -> PResult<Expr> {
        let span = self.peek().span;
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(e);
        }
        let start = self.peek().span.start;
        let mut end = start;
        while !self.at_eol() && !self.is_kw("ADDITIVE") {
            end = self.advance().span.end;
        }
        Ok(Expr::new(ExprKind::Str(self.source_text(start, end).trim().to_string()), span))
    }

    /// `SAVE TO file | MEMO field [ALL LIKE|EXCEPT skel]` and
    /// `RESTORE FROM file | MEMO field [ADDITIVE]`.
    fn variables_stmt(&mut self, save: bool) -> PResult<Option<StmtKind>> {
        self.advance();
        self.advance();
        let memo = self.eat_kw("MEMO");
        let target = if memo { self.menu_name()? } else { self.table_name()? };
        let mut skeleton = None;
        let mut except = false;
        if self.eat_kw("ALL") {
            except = self.is_kw("EXCEPT");
            if !self.eat_kw("EXCEPT") {
                self.eat_kw("LIKE");
            }
            skeleton = Some(self.skeleton()?);
        }
        let additive = self.eat_kw("ADDITIVE");
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::Variables { save, target, memo, skeleton, except, additive }))
    }

    /// An `@` line: two or four numbers, then what to draw at them - and often two things,
    /// because `@ 2,3 SAY "Name" GET cName` is a label and a field on one line.
    fn at_stmt(&mut self) -> PResult<Option<StmtKind>> {
        self.advance();
        let row = self.expr()?;
        self.expect(&TokKind::Comma, "','")?;
        let col = self.expr()?;
        let mut corners = Vec::new();
        while self.eat(&TokKind::Comma) {
            corners.push(self.expr()?);
        }
        let mut parts = Vec::new();
        loop {
            let Some(word) = self.peek().ident().map(|s| s.to_ascii_uppercase()) else { break };
            let what = if kw_text(&word, "SAY") {
                AtVerb::Say
            } else if word == "GET" {
                AtVerb::Get
            } else if kw_text(&word, "CLEAR") {
                AtVerb::Clear
            } else if word == "TO" {
                AtVerb::To
            } else if word == "BOX" {
                AtVerb::Box
            } else if kw_text(&word, "FILL") {
                AtVerb::Fill
            } else if kw_text(&word, "SCROLL") {
                AtVerb::Scroll
            } else if kw_text(&word, "MENU") {
                AtVerb::Menu
            } else if kw_text(&word, "PROMPT") {
                AtVerb::Prompt
            } else if kw_text(&word, "EDIT") {
                AtVerb::Edit
            } else {
                break;
            };
            self.advance();
            parts.push(self.at_part(what, &row, &col, &corners)?);
        }
        self.skip_line_keep_newline();
        if parts.is_empty() {
            // `@ 5, 10` on its own only moves where the next one starts
            parts.push(AtPart {
                what: AtVerb::Say,
                row,
                col,
                corners,
                value: None,
                picture: None,
                function: None,
                amount: None,
                amount2: None,
                style: String::new(),
                name: String::new(),
                valid: String::new(),
                when: String::new(),
            });
        }
        Ok(Some(StmtKind::AtCommand(parts)))
    }

    /// One part of that line: what it draws, and the clauses that say how.
    fn at_part(&mut self, what: AtVerb, row: &Expr, col: &Expr, corners: &[Expr]) -> PResult<AtPart> {
        let mut part = AtPart {
            what,
            row: row.clone(),
            col: col.clone(),
            corners: corners.to_vec(),
            value: None,
            picture: None,
            function: None,
            amount: None,
            amount2: None,
            style: String::new(),
            name: String::new(),
            valid: String::new(),
            when: String::new(),
        };
        let mut words: Vec<String> = Vec::new();
        match what {
            // `@ r1,c1 TO r2,c2` and `@ r1,c1 SCROLL TO r2,c2` take their corner here
            AtVerb::To => {
                part.corners = vec![self.expr()?];
                if self.eat(&TokKind::Comma) {
                    part.corners.push(self.expr()?);
                }
            }
            AtVerb::Say | AtVerb::Prompt | AtVerb::Box | AtVerb::Menu => {
                if !self.at_eol() && !self.is_at_clause() {
                    part.value = Some(self.expr()?);
                    if what == AtVerb::Menu && self.eat(&TokKind::Comma) {
                        part.amount = Some(self.expr()?);
                    }
                }
            }
            AtVerb::Get | AtVerb::Edit => {
                let start = self.peek().span.start;
                part.value = Some(self.postfix_expr()?);
                part.name = self.source_text(start, self.prev_span().end);
            }
            AtVerb::Clear | AtVerb::Fill | AtVerb::Scroll => {}
        }
        loop {
            if self.eat_kw("PICTURE") {
                part.picture = Some(self.expr()?);
            } else if self.eat_kw("FUNCTION") {
                part.function = Some(self.expr()?);
            } else if self.eat_kw("DEFAULT") {
                let _ = self.expr()?;
            } else if self.eat_kw("VALID") {
                let start = self.peek().span.start;
                let _ = self.expr()?;
                part.valid = self.source_text(start, self.prev_span().end);
            } else if self.eat_kw("WHEN") {
                let start = self.peek().span.start;
                let _ = self.expr()?;
                part.when = self.source_text(start, self.prev_span().end);
            } else if self.eat_kw("TO") {
                part.corners = vec![self.expr()?];
                if self.eat(&TokKind::Comma) {
                    part.corners.push(self.expr()?);
                }
            } else if self.eat_kw("RANGE") || self.eat_kw("SIZE") {
                let _ = self.expr()?;
                if self.eat(&TokKind::Comma) {
                    let _ = self.expr()?;
                }
            } else if self.eat_kw("MESSAGE") {
                let _ = self.expr()?;
            } else if self.eat_kw("COLOR") || self.eat_kw("FONT") || self.eat_kw("STYLE") || self.eat_kw("CLASS") {
                self.skip_line_keep_newline();
                break;
            } else if let Some(word) = self.peek().ident().map(|s| s.to_ascii_uppercase())
                && AT_WORDS.iter().any(|w| kw_text(&word, w))
            {
                self.advance();
                words.push(word);
                // `SCROLL ... UP 3`: how far it goes
                if matches!(self.peek_kind(), TokKind::Num(..)) {
                    part.amount = Some(self.expr()?);
                }
            } else {
                break;
            }
        }
        part.style = words.join(" ");
        Ok(part)
    }

    /// True when the next word starts a clause of an `@` line rather than being its value.
    fn is_at_clause(&mut self) -> bool {
        const WORDS: &[&str] =
            &["PICTURE", "FUNCTION", "VALID", "WHEN", "MESSAGE", "RANGE", "SIZE", "COLOR", "FONT", "STYLE", "GET", "SAY", "DEFAULT"];
        let Some(word) = self.peek().ident().map(|s| s.to_string()) else { return false };
        WORDS.iter().any(|w| kw_text(&word, w))
    }

    /// A name a menu command gives: a bare word, or an expression in brackets.
    fn menu_name(&mut self) -> PResult<Expr> {
        let span = self.peek().span;
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(e);
        }
        match self.peek_kind() {
            TokKind::Ident(name) => {
                let tok = self.advance();
                Ok(Expr::new(ExprKind::Str(name), tok.span))
            }
            TokKind::Str(text) => {
                let tok = self.advance();
                Ok(Expr::new(ExprKind::Str(text), tok.span))
            }
            _ => Ok(Expr::new(ExprKind::Str(String::new()), span)),
        }
    }

    /// The bare word a `ON PAD ... ACTIVATE POPUP <name>` clause names.
    fn menu_word(&mut self) -> String {
        match self.peek_kind() {
            TokKind::Ident(name) => {
                self.advance();
                name
            }
            _ => String::new(),
        }
    }

    /// The clauses a `DEFINE PAD`, `DEFINE BAR` or `DEFINE POPUP` carries after its name:
    /// what it says, what key it answers to, what the message line shows, and when it cannot
    /// be chosen. The rest say where it is drawn and in what colours, which the host decides.
    fn menu_clauses(&mut self) -> PResult<MenuClauses> {
        let mut c = MenuClauses::default();
        loop {
            if self.eat_kw("PROMPT") {
                c.prompt = Some(self.expr()?);
            } else if self.eat_kw("KEY") {
                let start = self.peek().span.start;
                let mut end = start;
                while !self.at_eol() && !self.is(&TokKind::Comma) && !self.is_menu_clause() {
                    end = self.advance().span.end;
                }
                let text = self.source_text(start, end);
                c.key = Some(Expr::new(ExprKind::Str(text), self.prev_span()));
                // `KEY ALT+F, "Alt+F"` gives the text to show beside the prompt as well
                if self.eat(&TokKind::Comma) {
                    let _ = self.expr()?;
                }
            } else if self.eat_kw("MESSAGE") {
                c.message = Some(self.expr()?);
            } else if self.eat_kw("SKIP") {
                if self.eat_kw("FOR") {
                    let start = self.peek().span.start;
                    let _ = self.expr()?;
                    c.skip = self.source_text(start, self.prev_span().end);
                } else {
                    // a bare SKIP greys it out for good
                    c.skip = ".T.".to_string();
                }
            } else if self.eat_kw("MARK") {
                let _ = self.expr()?;
            } else if self.eat_kw("BEFORE")
                || self.eat_kw("AFTER")
                || self.eat_kw("AT")
                || self.eat_kw("FROM")
                || self.eat_kw("TO")
            {
                let _ = self.expr();
                if self.eat(&TokKind::Comma) {
                    let _ = self.expr();
                }
            } else if self.eat_kw("IN") || self.eat_kw("COLOR") || self.eat_kw("FONT") || self.eat_kw("STYLE") {
                // where it is drawn and what it is drawn with: the host draws menus its own way
                self.skip_line_keep_newline();
                break;
            } else if self.eat_kw("SHADOW")
                || self.eat_kw("MARGIN")
                || self.eat_kw("SCROLL")
                || self.eat_kw("RELATIVE")
                || self.eat_kw("MOVER")
                || self.eat_kw("MULTISELECT")
                || self.eat_kw("NOMARGIN")
                || self.eat_kw("SHORTCUT")
                || self.eat_kw("TITLE")
                || self.eat_kw("FOOTER")
                || self.eat_kw("NEGOTIATE")
            {
                // words that change how a popup looks, which nothing here draws differently
            } else {
                break;
            }
        }
        Ok(c)
    }

    /// True when the next word starts another menu clause rather than being part of this one.
    fn is_menu_clause(&mut self) -> bool {
        const WORDS: &[&str] = &["PROMPT", "MESSAGE", "SKIP", "MARK", "BEFORE", "AFTER", "COLOR", "FONT", "STYLE"];
        let Some(word) = self.peek().ident().map(|s| s.to_string()) else { return false };
        WORDS.iter().any(|w| kw_text(&word, w))
    }

    /// `DEFINE MENU`, `DEFINE PAD`, `DEFINE POPUP` and `DEFINE BAR`.
    fn define_menu_stmt(&mut self) -> PResult<Option<StmtKind>> {
        self.advance();
        let word = self.peek().ident().unwrap_or_default().to_ascii_uppercase();
        self.advance();
        let what = if kw_text(&word, "MENU") {
            MenuVerb::DefineMenu
        } else if kw_text(&word, "POPUP") {
            MenuVerb::DefinePopup
        } else if word.eq_ignore_ascii_case("PAD") {
            MenuVerb::DefinePad
        } else {
            MenuVerb::DefineBar
        };
        let mut number = None;
        let mut name = None;
        if what == MenuVerb::DefineBar {
            number = Some(self.expr()?);
        } else {
            name = Some(self.menu_name()?);
        }
        let of = if self.eat_kw("OF") { Some(self.menu_name()?) } else { None };
        let c = self.menu_clauses()?;
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::MenuCommand {
            what,
            name,
            of,
            number,
            prompt: c.prompt,
            key: c.key,
            message: c.message,
            text: c.skip,
            flags: 0,
        }))
    }

    /// `ACTIVATE`, `DEACTIVATE`, `SHOW` and `HIDE` of a menu or a popup. The same words put
    /// up a window, a screen or a browse, none of which this runtime has.
    fn activate_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        let verb = first.ident().unwrap_or_default().to_ascii_uppercase();
        let word = self.peek_at(1).ident().unwrap_or_default().to_ascii_uppercase();
        let is_menu = kw_text(&word, "MENU");
        let is_popup = kw_text(&word, "POPUP");
        if kw_text(&word, "WINDOW") || kw_text(&word, "SCREEN") {
            return self.window_visibility_stmt(&verb, kw_text(&word, "SCREEN"));
        }
        // SHOW GETS, SHOW GET <var> and SHOW OBJECT all draw the fields again
        if verb.starts_with("SHOW") && (kw_text(&word, "GETS") || word == "GET" || kw_text(&word, "OBJECT")) {
            self.advance();
            self.advance();
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::WindowCommand {
                        read: None,
                what: WindowVerb::ShowGets,
                name: None,
                corners: Vec::new(),
                title: None,
                text: String::new(),
                flags: 0,
            }));
        }
        if !is_menu && !is_popup {
            self.warning(
                first.span,
                format!("{verb} is ignored: it draws on a Visual FoxPro screen this runtime does not have"),
            );
            self.skip_line_keep_newline();
            return Ok(None);
        }
        self.advance();
        self.advance();
        let mut flags = if is_popup { 4 } else { 0 };
        let name = if self.at_eol() || self.is_kw("ALL") {
            if self.eat_kw("ALL") {
                flags |= 1;
            }
            None
        } else {
            Some(self.menu_name()?)
        };
        loop {
            if self.eat_kw("NOWAIT") {
                flags |= 2;
            } else if self.eat_kw("PAD") || self.eat_kw("BAR") || self.eat_kw("AT") || self.eat_kw("IN") {
                let _ = self.expr();
                if self.eat(&TokKind::Comma) {
                    let _ = self.expr();
                }
            } else if self.eat_kw("REST") || self.eat_kw("SAVE") || self.eat_kw("EXTENDED") || self.eat_kw("NAME") {
                // what is left showing and what is kept: one menu is up at a time here
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        let what = if verb.starts_with("ACTI") {
            MenuVerb::Activate
        } else if verb.starts_with("DEAC") {
            MenuVerb::Deactivate
        } else if verb.starts_with("SHOW") {
            MenuVerb::Show
        } else {
            MenuVerb::Hide
        };
        Ok(Some(StmtKind::MenuCommand {
            what,
            name,
            of: None,
            number: None,
            prompt: None,
            key: None,
            message: None,
            text: String::new(),
            flags,
        }))
    }

    /// `PUSH MENU` and `POP MENU`, and the same for popups: what is defined now, kept and put
    /// back. The key and the alternate they also apply to are not things this runtime keeps.
    fn push_pop_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        let verb = first.ident().unwrap_or_default().to_ascii_uppercase();
        let word = self.peek_at(1).ident().unwrap_or_default().to_ascii_uppercase();
        let is_popup = kw_text(&word, "POPUP");
        // PUSH KEY and POP KEY keep the ON KEY settings rather than a menu
        if word == "KEY" {
            self.advance();
            self.advance();
            let clear = self.eat_kw("CLEAR");
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::KeyStack { push: verb.starts_with("PUSH"), clear }));
        }
        if !kw_text(&word, "MENU") && !is_popup {
            self.warning(
                first.span,
                format!("{verb} is ignored: it draws on a Visual FoxPro screen this runtime does not have"),
            );
            self.skip_line_keep_newline();
            return Ok(None);
        }
        self.advance();
        self.advance();
        let name = if self.at_eol() { None } else { Some(self.menu_name()?) };
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::MenuCommand {
            what: if verb.starts_with("PUSH") { MenuVerb::Push } else { MenuVerb::Pop },
            name,
            of: None,
            number: None,
            prompt: None,
            key: None,
            message: None,
            text: String::new(),
            flags: if is_popup { 4 } else { 0 },
        }))
    }

    /// `ON PAD`, `ON BAR` and the `ON SELECTION` family: what a choice opens, or what it runs.
    fn on_menu_stmt(&mut self, selection: bool) -> PResult<Option<StmtKind>> {
        let word = self.peek().ident().unwrap_or_default().to_ascii_uppercase();
        self.advance();
        let is_bar = word.eq_ignore_ascii_case("BAR");
        let mut number = None;
        let mut name = None;
        if is_bar {
            number = Some(self.expr()?);
        } else if self.is_kw("ALL") {
            self.advance();
        } else if !self.at_eol() && !self.is_kw("OF") {
            name = Some(self.menu_name()?);
        }
        let of = if self.eat_kw("OF") { Some(self.menu_name()?) } else { None };
        let what = match (selection, word.as_str()) {
            (false, "BAR") => MenuVerb::OnBar,
            (false, _) => MenuVerb::OnPad,
            (true, "BAR") => MenuVerb::OnSelectionBar,
            (true, w) if kw_text(w, "POPUP") => MenuVerb::OnSelectionPopup,
            (true, w) if kw_text(w, "MENU") => MenuVerb::OnSelectionMenu,
            (true, _) => MenuVerb::OnSelectionPad,
        };
        // `ON PAD x OF y ACTIVATE POPUP z` names a popup; every other form gives a command.
        let text = if !selection && self.is_kw("ACTIVATE") {
            self.advance();
            let _ = self.eat_kw("POPUP") || self.eat_kw("MENU");
            let target = self.menu_word();
            self.skip_line_keep_newline();
            target
        } else {
            let command = self.rest_of_line_text().unwrap_or_default();
            self.skip_line_keep_newline();
            command
        };
        Ok(Some(StmtKind::MenuCommand {
            what,
            name,
            of,
            number,
            prompt: None,
            key: None,
            message: None,
            text,
            flags: 0,
        }))
    }

    /// `SET MARK OF` and `SET SKIP OF`, each of which names a pad of a menu or a bar of a
    /// popup, or the menu or popup itself.
    fn set_of_stmt(&mut self, mark: bool) -> PResult<StmtKind> {
        self.advance();
        let word = self.peek().ident().unwrap_or_default().to_ascii_uppercase();
        let mut number = None;
        let mut name = None;
        let mut flags = 0;
        if word.eq_ignore_ascii_case("BAR") {
            self.advance();
            number = Some(self.expr()?);
            flags = 4;
        } else if word.eq_ignore_ascii_case("PAD") {
            self.advance();
            name = Some(self.menu_name()?);
        } else if kw_text(&word, "POPUP") {
            self.advance();
            name = Some(self.menu_name()?);
            flags = 4 | 8;
        } else if kw_text(&word, "MENU") {
            self.advance();
            name = Some(self.menu_name()?);
            flags = 8;
        }
        let of = if self.eat_kw("OF") { Some(self.menu_name()?) } else { None };
        let mut value = None;
        let mut text = String::new();
        if self.eat_kw("TO") {
            if mark {
                value = Some(self.expr()?);
            } else {
                let start = self.peek().span.start;
                let _ = self.expr()?;
                text = self.source_text(start, self.prev_span().end);
            }
        }
        self.skip_line_keep_newline();
        Ok(StmtKind::MenuCommand {
            what: if mark { MenuVerb::SetMark } else { MenuVerb::SetSkip },
            name,
            of,
            number,
            prompt: value,
            key: None,
            message: None,
            text,
            flags,
        })
    }

    /// `LIST` and `DISPLAY`: what the word after them names, or the records themselves.
    ///
    /// The two commands differ in what they do with nothing said about which records: `LIST`
    /// walks the whole table, `DISPLAY` shows the one the pointer is on.
    fn list_stmt(&mut self, all_by_default: bool) -> PResult<Option<StmtKind>> {
        const KINDS: &[&str] = &[
            "STRUCTURE",
            "MEMORY",
            "STATUS",
            "FILES",
            "TABLES",
            "VIEWS",
            "DLLS",
            "PROCEDURES",
            "OBJECTS",
            "CONNECTIONS",
        ];
        let word = self.peek().ident().unwrap_or_default().to_ascii_uppercase();
        if let Some(kind) = KINDS.iter().find(|k| kw_text(&word, k)) {
            self.advance();
            // `LIKE zz*` keeps the names that match. There is no NOLIKE here: Visual FoxPro
            // refuses one, which was measured rather than assumed.
            let skeleton = self
                .eat_kw("LIKE")
                .then(|| self.skeleton())
                .transpose()?;
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::ListInfo {
                what: (*kind).to_string(),
                fields: Vec::new(),
                skeleton,

                scope: Scope::All,
                cond: None,
                while_: None,
                numbers: true,
            }));
        }
        // OFF may be written before the fields or after the clauses, so the whole line is asked
        // rather than one place in it
        let numbers = !self.line_says(self.pos, "OFF");
        // The fields and the clauses may be written in either order - `LIST ALL code` says the
        // same as `LIST code ALL` - so both are read until neither reads any further.
        let mut fields: Vec<Name> = Vec::new();
        let mut scope = Scope::All;
        let mut named_scope = false;
        let mut cond = None;
        let mut while_ = None;
        loop {
            let before = self.pos;
            let (found, c, w, named) = self.record_clauses_named();
            if named {
                scope = found;
                named_scope = true;
            }
            if c.is_some() {
                cond = c;
            }
            if w.is_some() {
                while_ = w;
            }
            let (named_fields, _) = self.field_clause()?;
            fields.extend(named_fields);
            if fields.is_empty() && !self.at_eol() && !self.is_record_clause() && !self.is_kw("TO") {
                loop {
                    match self.peek_kind() {
                        TokKind::Ident(name) if !kw_text(&name.to_ascii_uppercase(), "OFF") => {
                            let tok = self.advance();
                            fields.push(Name::new(name, tok.span));
                        }
                        _ => break,
                    }
                    if !self.eat(&TokKind::Comma) {
                        break;
                    }
                }
            }
            if self.pos == before {
                break;
            }
        }
        // `DISPLAY` with nothing said about which records shows the one the pointer is on,
        // which is the same as NEXT 1 from where it stands
        let scope = match named_scope || all_by_default {
            true => scope,
            false => Scope::Next(Expr::new(ExprKind::Num(1.0, 1, 0), self.prev_span())),
        };
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::ListInfo { what: String::new(), fields, skeleton: None, scope, cond, while_, numbers }))
    }

    /// True when the line beginning at `start` has that word standing on its own in it.
    fn line_says(&self, start: usize, word: &str) -> bool {
        let mut at = start;
        while at < self.toks.len() && !self.toks[at].is_newline() {
            if self.toks[at].ident().is_some_and(|w| kw_text(&w.to_ascii_uppercase(), word)) {
                return true;
            }
            at += 1;
        }
        false
    }

    /// The table an SQL statement names, as both the file to open and the alias it opens under.
    ///
    /// SQL names a table where the rest of the language names a work area, so the alias is the
    /// stem of whatever path was written: `UPDATE ..\data\crew.dbf` writes to `CREW`.
    fn sql_table(&mut self) -> PResult<(Name, Expr)> {
        let path = self.table_name()?;
        let table = Name::new(
            match &path.kind {
                ExprKind::Str(text) => {
                    let name = text.rsplit(['/', '\\']).next().unwrap_or(text);
                    match name.rfind('.') {
                        Some(dot) => name[..dot].to_string(),
                        None => name.to_string(),
                    }
                }
                _ => String::new(),
            },
            path.span,
        );
        Ok((table, path))
    }

    /// `UPDATE table SET field = x [, ...] [WHERE condition]`
    fn update_stmt(&mut self, first: &Token) -> PResult<StmtKind> {
        self.advance();
        // `UPDATE ON key FROM alias` is the FoxPro 2 command, which this runtime does not have
        if self.is_kw("ON") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "UPDATE ON is not supported in the FoxDev runtime"));
        }
        let (table, path) = self.sql_table()?;
        if !self.eat_kw("SET") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "UPDATE needs SET and a column to set"));
        }
        let mut assignments = Vec::new();
        loop {
            let field = self.expect_ident("column name")?;
            // `SET customer.balance = 0` names the table as well, which changes nothing here
            let field = if self.eat(&TokKind::Dot) { self.expect_ident("column name")? } else { field };
            self.expect(&TokKind::Eq, "'='")?;
            let value = self.expr()?;
            assignments.push((field, value, false));
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        let cond = if self.eat_kw("WHERE") { Some(self.expr()?) } else { None };
        self.skip_line_keep_newline();
        Ok(StmtKind::UpdateSql { table, path, assignments, cond })
    }

    /// `APPEND FROM file [FIELDS list] [FOR x] [TYPE ...]`
    fn append_from_stmt(&mut self) -> PResult<StmtKind> {
        // `APPEND FROM ARRAY a` reads no file: a row of the array is a record, which is what
        // `INSERT INTO ... FROM ARRAY` reads too
        if self.eat_kw("ARRAY") {
            let span = self.prev_span();
            let source = self.postfix_expr()?;
            if self.is_kw("FIELDS") || self.is_kw("FOR") {
                self.skip_line_keep_newline();
                return Err(self.error(span, "APPEND FROM ARRAY takes the whole of every row; FIELDS and FOR are not supported"));
            }
            self.skip_line_keep_newline();
            return Ok(StmtKind::AppendFromArray(source));
        }
        let path = self.table_name()?;
        let mut fields = Vec::new();
        let mut except = Vec::new();
        let mut cond = String::new();
        let mut text = None;
        loop {
            if self.is_kw("FIELDS") {
                let (f, e) = self.field_clause()?;
                fields = f;
                except = e;
            } else if self.eat_kw("FOR") {
                let start = self.peek().span.start;
                let _ = self.expr()?;
                cond = self.source_text(start, self.prev_span().end);
            } else if self.is_kw("TYPE") || self.is_kw("SDF") || self.is_kw("CSV") || self.is_kw("DELIMITED") {
                text = self.text_format()?;
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        Ok(StmtKind::AppendFrom { path, fields, except, cond, text })
    }

    /// `COPY TO file [FIELDS list] [scope] [FOR x] [TYPE ...]` and `COPY STRUCTURE TO file`.
    fn copy_stmt(&mut self) -> PResult<StmtKind> {
        self.advance();
        let kind = if self.eat_kw("STRUCTURE") {
            // EXTENDED writes the structure as data - one record per field - rather than as
            // an empty table of that shape
            if self.eat_kw("EXTENDED") { CopyKind::StructureExtended } else { CopyKind::Structure }
        } else {
            CopyKind::Records
        };
        if !self.eat_kw("TO") {
            self.skip_line_keep_newline();
            return Err(self.error_here("COPY needs TO and a file name"));
        }
        // `COPY TO ARRAY a` writes no file: the array named takes the records, so what is read
        // here is somewhere to store to rather than a path
        let (kind, path) = match kind == CopyKind::Records && self.eat_kw("ARRAY") {
            true => (CopyKind::Array, self.postfix_expr()?),
            false => (kind, self.table_name()?),
        };
        let (fields, except) = self.field_clause()?;
        let (scope, cond, while_) = self.record_clauses();
        let (fields2, except2) = self.field_clause()?;
        let text = self.text_format()?;
        self.skip_line_keep_newline();
        let fields = if fields.is_empty() { fields2 } else { fields };
        let except = if except.is_empty() { except2 } else { except };
        Ok(StmtKind::CopyTo { path, kind, fields, except, scope, cond, while_, keys: Vec::new(), text })
    }

    /// `COPY INDEXES IndexFileList | ALL [TO CDXFileName]`: a tag per single-entry index.
    fn copy_indexes_stmt(&mut self) -> PResult<StmtKind> {
        let span = self.prev_span();
        let all = self.eat_kw("ALL");
        let files = if all { Vec::new() } else { self.index_file_list() };
        let target = if self.eat_kw("TO") { Some(self.table_name()?) } else { None };
        self.skip_line_keep_newline();
        if !all && files.is_empty() {
            return Err(self.error(span, "COPY INDEXES needs the index files to copy, or ALL"));
        }
        Ok(StmtKind::CopyIndexes { files, all, target })
    }

    /// `COPY TAG TagName [OF CDXFileName] TO IndexFileName`: a single-entry index from a tag.
    fn copy_tag_stmt(&mut self) -> PResult<StmtKind> {
        let span = self.prev_span();
        let tag = self.name_or_expr();
        let of = if self.eat_kw("OF") { Some(self.table_name()?) } else { None };
        if !self.eat_kw("TO") {
            self.skip_line_keep_newline();
            return Err(self.error(span, "COPY TAG needs TO and the name of the index file to write"));
        }
        let target = self.table_name()?;
        self.skip_line_keep_newline();
        Ok(StmtKind::CopyTag { tag, of, target })
    }

    /// A name written out, or an expression in brackets that works one out.
    fn name_or_expr(&mut self) -> Expr {
        let span = self.peek().span;
        if self.eat(&TokKind::LParen) {
            let e = self.expr_or_recover();
            self.expect(&TokKind::RParen, "')'").ok();
            return e;
        }
        match self.peek_kind() {
            TokKind::Ident(name) => {
                self.advance();
                Expr::new(ExprKind::Str(name), span)
            }
            _ => self.expr_or_recover(),
        }
    }

    /// `SORT TO file ON field [/A | /D] [, ...] [FIELDS list] [scope] [FOR x]`
    fn sort_stmt(&mut self) -> PResult<StmtKind> {
        self.advance();
        if !self.eat_kw("TO") {
            self.skip_line_keep_newline();
            return Err(self.error_here("SORT needs TO and a file name"));
        }
        let path = self.table_name()?;
        let mut keys = Vec::new();
        if self.eat_kw("ON") {
            loop {
                // a sort key is a field of the table, not an expression: `/D` after it is a
                // flag, and reading it as an expression would make that a division
                let name = self.expect_ident("a field to sort on")?;
                let expr = Expr::new(ExprKind::Var(name.clone()), name.span);
                let mut descending = false;
                // the /A, /D and /C flags come after the field name
                while self.eat(&TokKind::Slash) {
                    let flag = self.expect_ident("a sort flag")?;
                    descending = descending || flag.upper.starts_with('D');
                }
                keys.push(SortKey { expr, descending });
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        let (fields, except) = self.field_clause()?;
        let (scope, cond, while_) = self.record_clauses();
        self.skip_line_keep_newline();
        Ok(StmtKind::CopyTo { path, kind: CopyKind::Sorted, fields, except, scope, cond, while_, keys, text: None })
    }

    /// `TOTAL ON key TO file [FIELDS list] [scope] [FOR x]`
    fn total_stmt(&mut self) -> PResult<StmtKind> {
        let first = self.advance();
        if !self.eat_kw("ON") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "TOTAL needs ON and a key"));
        }
        let key = self.expr()?;
        if !self.eat_kw("TO") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "TOTAL needs TO and a file name"));
        }
        let path = self.table_name()?;
        let (fields, except) = self.field_clause()?;
        let (scope, cond, while_) = self.record_clauses();
        self.skip_line_keep_newline();
        let keys = vec![SortKey { expr: key, descending: false }];
        Ok(StmtKind::CopyTo { path, kind: CopyKind::Totals, fields, except, scope, cond, while_, keys, text: None })
    }

    /// `[TYPE] SDF | CSV | DELIMITED [WITH x] [WITH CHARACTER y]`, when a copy asks for text.
    fn text_format(&mut self) -> PResult<Option<TextFormat>> {
        let _ = self.eat_kw("TYPE");
        if self.eat_kw("SDF") {
            return Ok(Some(TextFormat::Sdf));
        }
        if self.eat_kw("CSV") {
            return Ok(Some(TextFormat::Csv));
        }
        if !self.eat_kw("DELIMITED") {
            return Ok(None);
        }
        let mut quote = "\"".to_string();
        let mut separator = ",".to_string();
        while self.eat_kw("WITH") {
            if self.eat_kw("BLANK") {
                separator = " ".into();
            } else if self.eat_kw("TAB") {
                separator = "\t".into();
            } else if self.eat_kw("CHARACTER") {
                separator = self.delimiter_text()?;
            } else {
                quote = self.delimiter_text()?;
            }
        }
        Ok(Some(TextFormat::Delimited { quote, separator }))
    }

    /// The one character a DELIMITED clause names, written as a string or on its own.
    fn delimiter_text(&mut self) -> PResult<String> {
        match self.peek_kind() {
            TokKind::Str(s) => {
                self.advance();
                Ok(s)
            }
            TokKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            _ => {
                let e = self.expr()?;
                match &e.kind {
                    ExprKind::Str(s) => Ok(s.clone()),
                    _ => Ok(String::new()),
                }
            }
        }
    }

    /// A table named on a command line: a bare name, or an expression in parentheses. The name
    /// ends at the first word that starts a clause of the command it belongs to.
    fn table_name(&mut self) -> PResult<Expr> {
        let span = self.peek().span;
        if let Some(ask) = self.ask_for_file("DBF") {
            return Ok(ask);
        }
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(e);
        }
        // A file name is a name and not an expression - `USE customer` opens customer.dbf rather
        // than reading a variable called customer - but a name that could not be one is read as
        // an expression. Measured: `USE UPPER('ttwo')` and `USE lcName + ''` both open TTWO.DBF,
        // and `USE tone + ''` says there is no variable TONE rather than looking for a file,
        // where `USE lcName` on its own looks for lcname.dbf. That is how the samples write it -
        // `USE ADDBS(HOME()) + 'Samples\Northwind\Orders'` - so a word with a bracket straight
        // after it, or a line with a quoted string in it, is an expression.
        if self.name_is_expression(COPY_WORDS) {
            return self.expr();
        }
        let start = self.peek().span.start;
        let mut end = start;
        let mut after_dot = false;
        // `rows.csv` is a file name, not a file named `rows` followed by the word CSV, so a
        // clause only starts at a word that does not follow a dot
        while !self.at_eol() && (after_dot || !self.is_copy_word()) {
            after_dot = self.peek_kind() == TokKind::Dot;
            end = self.advance().span.end;
        }
        Ok(Expr::new(ExprKind::Str(self.source_text(start, end).trim().to_string()), span))
    }

    /// The clauses left on the line, as `db_flags` bits.
    ///
    /// A database command is written with its clauses in any order and the database event that
    /// goes with it is handed which of them were there, so they are read off the line rather
    /// than skipped. Anything else on the line is left for the caller to pass over.
    fn db_clauses(&mut self) -> u16 {
        use crate::ast::db_flags as f;
        let mut flags = 0;
        loop {
            let bit = if self.eat_kw("EXCLUSIVE") {
                f::EXCLUSIVE
            } else if self.eat_kw("SHARED") {
                0
            } else if self.eat_kw("NOUPDATE") {
                f::NOUPDATE
            } else if self.eat_kw("VALIDATE") {
                f::VALIDATE
            } else if self.eat_kw("ALL") {
                f::ALL
            } else if self.eat_kw("RECYCLE") {
                f::RECYCLE
            } else if self.eat_kw("DELETE") {
                f::DELETE
            } else if self.eat_kw("ADDITIVE") || self.eat_kw("OVERWRITE") {
                f::ADDITIVE
            } else if self.eat_kw("RECOVER") {
                f::RECOVER
            } else if self.eat_kw("NOCONSOLE") {
                f::NOCONSOLE
            } else if self.eat_kw("NOWAIT") {
                f::NOWAIT
            } else if self.eat_kw("NOEDIT") {
                f::NOEDIT
            } else {
                break;
            };
            flags |= bit;
        }
        flags
    }

    /// Whether what stands where a file name is expected is an expression rather than a name.
    ///
    /// `stop` is the clause words of the command, which end the name.
    ///
    /// Only the two shapes that could not be a file name count: a word with a bracket straight
    /// after it, which is a function call, and a run with a quoted string in it. Everything else
    /// - `rows.csv`, `..\data\customer.dbf`, `c:\app\data\x` - is the name as written.
    fn name_is_expression(&mut self, stop: &[&str]) -> bool {
        if matches!(self.peek_kind(), TokKind::Ident(_)) && self.peek_at(1).kind == TokKind::LParen {
            return true;
        }
        let mut n = 0;
        let mut after_dot = false;
        while !self.peek_at(n).is_newline() && !matches!(self.peek_at(n).kind, TokKind::Eof) {
            if matches!(self.peek_at(n).kind, TokKind::Str(_)) {
                return true;
            }
            let here = self.peek_at(n).clone();
            if !after_dot && stop.iter().any(|k| kw(&here, k)) {
                break;
            }
            after_dot = here.kind == TokKind::Dot;
            n += 1;
        }
        false
    }

    /// A word that ends the file name of a copy and starts a clause of its own.
    fn is_copy_word(&mut self) -> bool {
        let tok = self.peek().clone();
        COPY_WORDS.iter().any(|k| kw(&tok, k))
    }

    /// `COUNT`, `SUM`, `AVERAGE` and `CALCULATE`: the same pass over the records, differing
    /// only in what each column works out. CALCULATE names it per column; the others do not.
    fn aggregate_stmt(&mut self, kind: AggFunc) -> PResult<StmtKind> {
        let first = self.advance();
        let mut calls = Vec::new();
        if kind == AggFunc::Cnt {
            calls.push(AggCall { func: AggFunc::Cnt, arg: None, rate: None });
        } else {
            while !self.at_eol() && !self.is_record_clause() && !self.is_kw("TO") {
                calls.push(self.aggregate_call(kind)?);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        let (scope, cond, while_) = self.record_clauses();
        let mut targets = Vec::new();
        let mut array = None;
        if self.eat_kw("TO") {
            if self.eat_kw("ARRAY") {
                array = Some(self.expr()?);
            } else {
                loop {
                    targets.push(self.expr()?);
                    if !self.eat(&TokKind::Comma) {
                        break;
                    }
                }
            }
        }
        self.skip_line_keep_newline();
        if calls.is_empty() {
            return Err(self.error(first.span, "there is nothing here to work out"));
        }
        Ok(StmtKind::Aggregate { calls, scope, cond, while_, targets, array })
    }

    /// One column: `amount` for SUM and AVERAGE, `SUM(amount)` and friends for CALCULATE.
    fn aggregate_call(&mut self, kind: AggFunc) -> PResult<AggCall> {
        if kind != AggFunc::Std {
            return Ok(AggCall { func: kind, arg: Some(self.expr()?), rate: None });
        }
        let name = self.peek().ident().unwrap_or_default().to_ascii_uppercase();
        let func = match name.as_str() {
            "CNT" => AggFunc::Cnt,
            "SUM" => AggFunc::Sum,
            "AVG" => AggFunc::Avg,
            "MIN" => AggFunc::Min,
            "MAX" => AggFunc::Max,
            "STD" => AggFunc::Std,
            "VAR" => AggFunc::Var,
            "NPV" => AggFunc::Npv,
            _ => {
                let span = self.peek().span;
                return Err(self.error(span, format!("CALCULATE works out CNT, SUM, AVG, MIN, MAX, STD, VAR and NPV, not {name}")));
            }
        };
        self.advance();
        self.expect(&TokKind::LParen, "'('")?;
        let mut arg = None;
        let mut rate = None;
        if self.peek_kind() != TokKind::RParen {
            let e = self.expr()?;
            if func == AggFunc::Npv && self.eat(&TokKind::Comma) {
                rate = Some(e);
                arg = Some(self.expr()?);
            } else {
                arg = Some(e);
            }
        }
        self.expect(&TokKind::RParen, "')'")?;
        Ok(AggCall { func, arg, rate })
    }

    /// True at a word that starts the scope or condition of a record command.
    fn is_record_clause(&mut self) -> bool {
        let tok = self.peek().clone();
        ["ALL", "NEXT", "REST", "RECORD", "FOR", "WHILE", "IN"].iter().any(|k| kw(&tok, k))
    }

    /// `SCATTER [FIELDS <list> [EXCEPT <list>]] [MEMO] [BLANK] TO aRow | MEMVAR | NAME oRow`
    fn scatter_stmt(&mut self) -> PResult<StmtKind> {
        let first = self.advance();
        let (fields, except) = self.field_clause()?;
        let mut blank = false;
        let mut to = None;
        loop {
            if self.eat_kw("MEMO") {
                // a memo field is a string here like any other, so this asks for nothing extra
            } else if self.eat_kw("BLANK") {
                blank = true;
            } else if self.eat_kw("MEMVAR") {
                to = Some(ScatterWhere::Memvar);
            } else if self.eat_kw("NAME") {
                to = Some(ScatterWhere::Name(self.expr()?));
            } else if self.eat_kw("TO") {
                if self.eat_kw("NAME") {
                    to = Some(ScatterWhere::Name(self.expr()?));
                } else {
                    to = Some(ScatterWhere::Array(self.expr()?));
                }
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        let Some(to) = to else {
            return Err(self.error(first.span, "SCATTER needs TO an array, MEMVAR or NAME"));
        };
        Ok(StmtKind::Scatter { fields, except, blank, to })
    }

    /// `GATHER FROM aRow | MEMVAR | NAME oRow [FIELDS <list> [EXCEPT <list>]] [MEMO]`
    fn gather_stmt(&mut self) -> PResult<StmtKind> {
        let first = self.advance();
        let mut from = None;
        let mut fields = Vec::new();
        let mut except = Vec::new();
        loop {
            if self.eat_kw("MEMO") {
            } else if self.eat_kw("MEMVAR") {
                from = Some(ScatterWhere::Memvar);
            } else if self.eat_kw("NAME") {
                from = Some(ScatterWhere::Name(self.expr()?));
            } else if self.eat_kw("FROM") {
                if self.eat_kw("NAME") {
                    from = Some(ScatterWhere::Name(self.expr()?));
                } else if self.eat_kw("MEMVAR") {
                    from = Some(ScatterWhere::Memvar);
                } else {
                    from = Some(ScatterWhere::Array(self.expr()?));
                }
            } else if self.is_kw("FIELDS") {
                let (f, e) = self.field_clause()?;
                fields = f;
                except = e;
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        let Some(from) = from else {
            return Err(self.error(first.span, "GATHER needs FROM an array, MEMVAR or NAME"));
        };
        Ok(StmtKind::Gather { from, fields, except })
    }

    /// What comes after `FROM` in `INSERT INTO t FROM ARRAY a | MEMVAR | NAME oRec`. GATHER
    /// reads the same three places and writes them without the word ARRAY, so this is the
    /// same clause with that word allowed.
    fn gather_source(&mut self, first: &Token) -> PResult<ScatterWhere> {
        if self.eat_kw("MEMVAR") {
            return Ok(ScatterWhere::Memvar);
        }
        if self.eat_kw("NAME") {
            return Ok(ScatterWhere::Name(self.expr()?));
        }
        if self.eat_kw("ARRAY") {
            return Ok(ScatterWhere::Array(self.expr()?));
        }
        let what = self.peek().ident().map(|s| s.to_ascii_uppercase()).unwrap_or_default();
        self.skip_line_keep_newline();
        Err(self.error(first.span, format!("FROM takes ARRAY, MEMVAR or NAME, not {what}")))
    }

    /// `FIELDS name1, name2 [EXCEPT name3]`: which of a table's columns a command works on.
    fn field_clause(&mut self) -> PResult<(Vec<Name>, Vec<Name>)> {
        let mut fields = Vec::new();
        let mut except = Vec::new();
        if self.eat_kw("FIELDS") {
            let _ = self.eat_kw("LIKE");
            loop {
                if self.eat_kw("EXCEPT") {
                    loop {
                        except.push(self.expect_ident("field name")?);
                        if !self.eat(&TokKind::Comma) {
                            break;
                        }
                    }
                    break;
                }
                fields.push(self.expect_ident("field name")?);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
        }
        Ok((fields, except))
    }

    /// `SET RELATION TO eExpr INTO cAlias [, ...] [ADDITIVE]`: what moves the child area, and
    /// where it is. The expression is kept as text because it is evaluated at every record.
    fn set_relation_stmt(&mut self) -> PResult<StmtKind> {
        // `SET RELATION OFF INTO x` takes one relation away and leaves the others
        if self.eat_kw("OFF") {
            let _ = self.eat_kw("INTO");
            let off = self.expect_ident("alias").ok();
            self.skip_line_keep_newline();
            return Ok(StmtKind::SetRelation { pairs: Vec::new(), additive: true, off });
        }
        let _ = self.eat_kw("TO");
        let mut pairs = Vec::new();
        let mut additive = false;
        while !self.at_eol() {
            if self.eat_kw("ADDITIVE") {
                additive = true;
                break;
            }
            let start = self.peek().span.start;
            let _ = self.expr()?;
            let text = self.source_text(start, self.prev_span().end);
            if !self.eat_kw("INTO") {
                return Err(self.error_here("SET RELATION needs INTO and a work area"));
            }
            let alias = match self.peek_kind() {
                TokKind::Ident(name) => {
                    let tok = self.advance();
                    Name::new(name, tok.span)
                }
                _ => return Err(self.error_here("SET RELATION needs a work area after INTO")),
            };
            pairs.push((text, alias));
            if !self.eat(&TokKind::Comma) {
                let _ = self.eat_kw("ADDITIVE") && {
                    additive = true;
                    true
                };
                break;
            }
        }
        self.skip_line_keep_newline();
        Ok(StmtKind::SetRelation { pairs, additive, off: None })
    }

    /// The tag named by `ORDER [TAG] x [OF file] [ASCENDING|DESCENDING]`, and which way it runs.
    /// A bare `ORDER` with nothing after it puts the table back in record order.
    fn order_clause(&mut self) -> PResult<(Option<Expr>, Option<bool>)> {
        let _ = self.eat_kw("TAG");
        let mut tag = None;
        if !self.at_eol() && !self.is_order_word() {
            tag = Some(if self.eat(&TokKind::LParen) {
                let e = self.expr()?;
                self.expect(&TokKind::RParen, "')'")?;
                e
            } else if let TokKind::Ident(name) = self.peek_kind() {
                let tok = self.advance();
                Expr::new(ExprKind::Str(name), tok.span)
            } else {
                self.expr()?
            });
        }
        let mut descending = None;
        loop {
            if self.eat_kw("ASCENDING") {
                descending = Some(false);
            } else if self.eat_kw("DESCENDING") {
                descending = Some(true);
            } else if self.eat_kw("OF") || self.eat_kw("IN") {
                // the index file the tag is in, and the work area: one index per table here
                let _ = self.advance();
            } else {
                break;
            }
        }
        Ok((tag, descending))
    }

    /// A word that ends the tag name of an ORDER clause rather than being one.
    fn is_order_word(&mut self) -> bool {
        let tok = self.peek().clone();
        ["ASCENDING", "DESCENDING", "OF", "IN", "ALIAS", "EXCLUSIVE", "SHARED", "AGAIN", "NOUPDATE"]
            .iter()
            .any(|k| kw(&tok, k))
    }

    /// `INDEX ON eExpr TAG name [OF file] [FOR x] [COMPACT] [ASCENDING|DESCENDING]
    /// [UNIQUE|CANDIDATE] [ADDITIVE]`
    fn index_stmt(&mut self) -> PResult<StmtKind> {
        let first = self.advance();
        if !self.eat_kw("ON") {
            self.skip_line_keep_newline();
            return Err(self.error(first.span, "INDEX needs ON and a key expression"));
        }
        let start = self.peek().span.start;
        let key = self.expr()?;
        let key_text = self.source_text(start, self.prev_span().end);
        let mut tag = None;
        let mut to_file = None;
        let mut compact = false;
        let mut cond = None;
        let mut cond_text = String::new();
        let mut unique = false;
        let mut candidate = false;
        let mut descending = false;
        let mut additive = false;
        loop {
            if self.eat_kw("TAG") {
                tag = self.expect_ident("tag name").ok();
            } else if self.eat_kw("TO") {
                // `INDEX ON x TO name` writes a single-entry index of its own rather than a tag
                to_file = Some(self.name_or_expr());
            } else if self.eat_kw("FOR") {
                let at = self.peek().span.start;
                cond = Some(self.expr()?);
                cond_text = self.source_text(at, self.prev_span().end);
            } else if self.eat_kw("UNIQUE") {
                unique = true;
            } else if self.eat_kw("CANDIDATE") {
                candidate = true;
            } else if self.eat_kw("DESCENDING") {
                descending = true;
            } else if self.eat_kw("COMPACT") {
                compact = true;
            } else if self.eat_kw("ADDITIVE") {
                // without it, building an index closes the single-entry ones already open
                additive = true;
            } else if self.eat_kw("ASCENDING") {
            } else if self.eat_kw("OF") || self.eat_kw("IN") {
                let _ = self.advance();
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        if tag.is_none() && to_file.is_none() {
            return Err(self.error(first.span, "INDEX ON needs a TAG name or a TO file name"));
        }
        // a tag is what the file stem is called when the index is one of its own
        let tag = tag.unwrap_or_else(|| Name::new(String::new(), first.span));
        Ok(StmtKind::IndexOn { key, key_text, tag, to_file, compact, cond, cond_text, unique, candidate, descending, additive })
    }

    /// `USE [table] [IN area] [ALIAS name] [EXCLUSIVE|SHARED] [AGAIN] [NOUPDATE]`.
    ///
    /// The table is a bare path unless it is parenthesised, which is how VFP tells
    /// `USE customer` from `USE (cTableName)`.
    fn use_stmt(&mut self) -> PResult<StmtKind> {
        let start = self.advance().span;
        let mut table = None;
        if !self.at_eol() && !self.is_clause_word() {
            table = Some(if self.eat(&TokKind::LParen) {
                let e = self.expr()?;
                self.expect(&TokKind::RParen, "')'")?;
                e
            } else if self.name_is_expression(CLAUSE_WORDS) {
                // `USE ADDBS(HOME()) + 'Samples\Northwind\Orders'` is how a sample writes it:
                // a name that could not be one is read as an expression. See name_is_expression.
                self.expr()?
            } else {
                Expr::new(ExprKind::Str(self.bare_path()), start)
            });
        }

        let mut alias = None;
        let mut exclusive = false;
        let mut online = false;
        let mut in_area = None;
        let mut order = None;
        let mut order_desc = None;
        let mut indexes = Vec::new();
        loop {
            if self.eat_kw("ALIAS") {
                alias = self.name_ref("alias name").ok();
            } else if self.eat_kw("EXCLUSIVE") {
                exclusive = true;
            } else if self.eat_kw("SHARED") || self.eat_kw("AGAIN") || self.eat_kw("NOUPDATE") {
                // accepted and ignored: nothing here shares a table between processes yet
            } else if self.eat_kw("ONLINE") || self.eat_kw("ADMIN") {
                // both ask for a view that was taken offline: ONLINE puts it back, ADMIN opens
                // it to be managed. Neither means anything to a table, and VFP says so.
                online = true;
            } else if self.eat_kw("NODATA") {
                // a view opened with none of its rows. On a table VFP does nothing with it,
                // which is what a table opened here is.
            } else if self.eat_kw("NOREQUERY") {
                // "do not run the view's query again"; the number after it is a data session,
                // and both are nothing to a table
                if matches!(self.peek_kind(), TokKind::Num(..)) {
                    self.advance();
                }
            } else if self.eat_kw("CONNSTRING") {
                // how a remote view reaches its server, which is not something this reads yet
                let _ = self.expr()?;
            } else if self.eat_kw("IN") {
                in_area = Some(self.area_target());
            } else if self.eat_kw("ORDER") {
                let (tag, desc) = self.order_clause()?;
                order = tag;
                order_desc = desc;
            } else if self.eat_kw("INDEX") {
                // `USE ... INDEX x, y`: single-entry indexes opened beside the table, the
                // first of them controlling. The structural compound index opens anyway.
                indexes = self.index_file_list();
            } else {
                break;
            }
        }
        Ok(StmtKind::Use { table, alias, exclusive, online, in_area, order, order_desc, indexes })
    }

    /// `SELECT 0` and `SELECT customer` pick a work area; anything else is SQL.
    fn select_stmt(&mut self) -> PResult<StmtKind> {
        let start = self.peek().span;
        // a work area is a number, or one name and nothing else on the line
        let first = self.peek_at(1).kind.clone();
        let second = self.peek_at(2).kind.clone();
        let area = match (&first, &second) {
            (TokKind::Num(..) | TokKind::Ident(_), TokKind::Newline | TokKind::Eof) => true,
            (TokKind::LParen, _) => true,
            _ => false,
        };
        if !area {
            return self.query(start).map(|q| StmtKind::Query(Box::new(q)));
        }
        self.advance();
        if let TokKind::Ident(name) = self.peek_kind() {
            let tok = self.advance();
            return Ok(StmtKind::SelectArea(SelectTarget::Alias(Name::new(name, tok.span))));
        }
        Ok(StmtKind::SelectArea(SelectTarget::Number(self.expr()?)))
    }

    // ----- SELECT-SQL ------------------------------------------------------------------------

    /// `SELECT [ALL|DISTINCT] [TOP n] columns FROM sources [WHERE] [GROUP BY] [ORDER BY] [INTO]`.
    ///
    /// A query is one statement however many lines it is written over, so the clause words are
    /// what ends each part rather than the end of a line.
    fn query(&mut self, start: Span) -> PResult<Query> {
        self.advance(); // SELECT
        self.in_query += 1;
        let query = self.query_body(start);
        self.in_query -= 1;
        query
    }

    fn query_body(&mut self, start: Span) -> PResult<Query> {
        let mut distinct = false;
        if self.eat_kw("DISTINCT") {
            distinct = true;
        } else {
            self.eat_kw("ALL");
        }
        let mut top = None;
        let mut top_percent = false;
        if self.eat_kw("TOP") {
            // the count is a number and nothing more: `SELECT TOP 16 * FROM x` would otherwise
            // read the `*` that stands for every column as a multiplication
            top = Some(self.postfix_expr()?);
            top_percent = self.eat_kw("PERCENT");
            // the reference writes DISTINCT before TOP, and Visual FoxPro takes it either way
            // round: measured, `SELECT TOP 2 DISTINCT tag FROM t ORDER BY 1` is the same query
            // as `SELECT DISTINCT TOP 2 tag FROM t ORDER BY 1`
            if !distinct && self.eat_kw("DISTINCT") {
                distinct = true;
            }
        }

        let columns = self.query_columns()?;

        // The clauses after the columns come in any order: measured, Visual FoxPro reads
        // `INTO ARRAY a ORDER BY name GROUP BY name`, a WHERE after INTO CURSOR and an ORDER BY
        // before WHERE as the same query written the usual way round.
        let mut from = Vec::new();
        let mut where_ = None;
        let mut group_by = Vec::new();
        let mut having = None;
        let mut order_by = Vec::new();
        let mut into = QueryInto::Browse;
        loop {
            if self.eat_kw("FROM") {
                from = self.query_sources()?;
            } else if self.eat_kw("WHERE") {
                where_ = Some(self.expr()?);
            } else if self.eat_kw("GROUP") {
                self.eat_kw("BY");
                group_by = self.expr_list()?;
            } else if self.eat_kw("HAVING") {
                having = Some(self.expr()?);
            } else if self.eat_kw("ORDER") {
                self.eat_kw("BY");
                order_by = self.order_terms()?;
            } else if self.eat_kw("INTO") {
                into = self.query_into(start)?;
            } else if self.eat_kw("TO") {
                let what = self.peek().ident().unwrap_or("").to_ascii_uppercase();
                self.skip_line_keep_newline();
                return Err(self.error(start, format!("SELECT ... TO {what} is not supported in the FoxDev runtime")));
            } else {
                break;
            }
        }

        // `UNION [ALL] SELECT ...`: the next query is read whole, and its ORDER BY and INTO -
        // which the syntax puts after the last SELECT - belong to the union, so they come up
        // here and the query that carried them is left with none
        if self.eat_kw("UNION") {
            let all = self.eat_kw("ALL");
            if !self.is_kw("SELECT") {
                return Err(self.expected("SELECT"));
            }
            self.advance();
            let mut rest = self.query_body(start)?;
            let order_by = std::mem::take(&mut rest.order_by);
            let into = std::mem::replace(&mut rest.into, into);
            if from.is_empty() {
                return Err(self.error(start, "SELECT needs a FROM clause"));
            }
            return Ok(Query {
                distinct,
                top,
                top_percent,
                columns,
                from,
                where_,
                group_by,
                having,
                order_by,
                into,
                union: Some(Box::new(QueryUnion { all, query: rest })),
                span: start.to(self.prev_span()),
            });
        }

        if from.is_empty() {
            return Err(self.error(start, "SELECT needs a FROM clause"));
        }
        Ok(Query {
            distinct,
            top,
            top_percent,
            columns,
            from,
            where_,
            group_by,
            having,
            order_by,
            into,
            union: None,
            span: start.to(self.prev_span()),
        })
    }

    /// `ORDER BY a [ASC | DESC], ...`, the ORDER BY already taken.
    fn order_terms(&mut self) -> PResult<Vec<OrderTerm>> {
        let mut out = Vec::new();
        loop {
            let expr = self.expr()?;
            let descending = if self.eat_kw("DESC") {
                true
            } else {
                self.eat_kw("ASC");
                false
            };
            out.push(OrderTerm { expr, descending });
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        Ok(out)
    }

    /// Where a query's answer goes, the INTO already taken.
    fn query_into(&mut self, start: Span) -> PResult<QueryInto> {
        if self.eat_kw("CURSOR") {
            let into = QueryInto::Cursor(self.name_ref("cursor name")?);
            // READWRITE, NOFILTER and the rest describe a cursor this runtime already is
            self.skip_to_query_clause();
            Ok(into)
        } else if self.eat_kw("ARRAY") {
            Ok(QueryInto::Array(self.expr()?))
        } else if self.eat_kw("TABLE") || self.eat_kw("DBF") {
            let into = QueryInto::Table(self.table_name()?);
            // DATABASE, NAME and the rest say where the table is listed, which this runtime
            // answers from the file itself
            self.skip_to_query_clause();
            Ok(into)
        } else {
            let what = self.peek().ident().unwrap_or("").to_ascii_uppercase();
            self.skip_line_keep_newline();
            Err(self.error(start, format!("SELECT ... INTO {what} is not supported in the FoxDev runtime")))
        }
    }

    /// Passes over the words of an INTO clause this runtime has no use for, up to the next
    /// clause of the query or the end of it.
    fn skip_to_query_clause(&mut self) {
        const CLAUSES: [&str; 8] = ["FROM", "WHERE", "GROUP", "HAVING", "ORDER", "INTO", "UNION", "TO"];
        while !self.at_eol() && !CLAUSES.iter().any(|w| self.is_kw(w)) {
            self.advance();
        }
    }

    fn query_columns(&mut self) -> PResult<Vec<QueryColumn>> {
        let mut out = Vec::new();
        loop {
            out.push(self.query_column()?);
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        Ok(out)
    }

    fn query_column(&mut self) -> PResult<QueryColumn> {
        // a star stands for the fields of a source, which are only known when the query runs
        if self.eat(&TokKind::Star) {
            return Ok(QueryColumn::All(None));
        }
        if let TokKind::Ident(name) = self.peek_kind()
            && matches!(self.peek_at(1).kind, TokKind::Dot)
            && matches!(self.peek_at(2).kind, TokKind::Star)
        {
            let tok = self.advance();
            self.advance();
            self.advance();
            return Ok(QueryColumn::All(Some(Name::new(name, tok.span))));
        }
        // `COUNT(*)` is read by the expression grammar itself, which knows it is inside a query.
        let expr = self.expr()?;
        let name = if self.eat_kw("AS") { Some(self.expect_ident("column name")?) } else { None };
        Ok(QueryColumn::Value { expr, name })
    }

    fn query_sources(&mut self) -> PResult<Vec<QuerySource>> {
        let mut out = vec![self.query_source(JoinKind::Inner)?];
        // set while a RIGHT JOIN is being read, and acted on once its source is in hand
        let mut swap = false;
        loop {
            if self.eat(&TokKind::Comma) {
                out.push(self.query_source(JoinKind::Inner)?);
                continue;
            }
            let join = if self.eat_kw("INNER") {
                JoinKind::Inner
            } else if self.eat_kw("LEFT") {
                self.eat_kw("OUTER");
                JoinKind::Left
            } else if self.is_kw("JOIN") {
                JoinKind::Inner
            } else if self.is_kw("RIGHT") || self.is_kw("FULL") {
                // `a RIGHT JOIN b` keeps every row of b, which is what `b LEFT JOIN a` does, so
                // it is read as that: the two swap places below and nothing else changes. FULL
                // keeps the unmatched rows of both sides and is run twice, once each way round.
                // Both identities are between two tables, so a chain of them is refused rather
                // than guessed at.
                let span = self.peek().span;
                let what = self.peek().ident().unwrap_or("").to_ascii_uppercase();
                self.advance();
                self.eat_kw("OUTER");
                if out.len() != 1 {
                    self.skip_line_keep_newline();
                    return Err(self.error(
                        span,
                        format!("{what} JOIN is read as the LEFT JOIN of the same two tables the other way round, which only works when there are two"),
                    ));
                }
                swap = what == "RIGHT";
                if swap { JoinKind::Left } else { JoinKind::Full }
            } else {
                break;
            };
            if !self.eat_kw("JOIN") {
                return Err(self.error_here("expected JOIN"));
            }
            let source = self.query_source(join)?;
            out.push(QuerySource { joined: true, ..source });
            if std::mem::take(&mut swap) {
                out.swap(0, 1);
                out[0] = QuerySource { joined: false, join: JoinKind::Inner, ..out[0].clone() };
                out[1] = QuerySource { joined: true, join: JoinKind::Left, ..out[1].clone() };
            }
            // SQL lets the joins be written first and their conditions after, innermost first:
            // `a LEFT JOIN b INNER JOIN c ON x ON y` joins c on x and b on y, so a condition
            // belongs to the last join that has not got one yet.
            while self.eat_kw("ON") {
                let on = self.expr()?;
                match out.iter_mut().rev().find(|s| s.on.is_none() && s.joined) {
                    Some(source) => source.on = Some(on),
                    None => {
                        self.error(on.span, "ON has no JOIN to belong to");
                        break;
                    }
                }
            }
        }
        Ok(out)
    }

    fn query_source(&mut self, join: JoinKind) -> PResult<QuerySource> {
        let start = self.peek().span;
        // a source named by an expression is worked out when the query runs
        if self.eat(&TokKind::LParen) {
            let table_expr = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            let alias = if self.eat_kw("AS") {
                self.expect_ident("alias")?
            } else if let TokKind::Ident(name) = self.peek_kind()
                && !self.at_query_clause()
            {
                let tok = self.advance();
                Name::new(name, tok.span)
            } else {
                // nothing said what to call it, so the alias is only known once it is opened
                Name::new(String::new(), start)
            };
            return Ok(QuerySource {
                table: String::new(),
                table_expr: Some(table_expr),
                alias,
                join,
                joined: false,
                on: None,
                span: start.to(self.prev_span()),
            });
        }
        let table = self.table_ref()?;
        self.eat_kw("AS");
        // a bare name after the table is its alias; a clause word is the next part of the query
        let alias = match self.peek_kind() {
            TokKind::Ident(name) if !self.at_query_clause() => {
                let tok = self.advance();
                Name::new(name, tok.span)
            }
            _ => Name::new(query_alias(&table), start),
        };
        Ok(QuerySource { table, table_expr: None, alias, join, joined: false, on: None, span: start.to(self.prev_span()) })
    }

    /// A table in a FROM clause: an alias, a file name, or a path with directories in it.
    fn table_ref(&mut self) -> PResult<String> {
        if self.at_eol() {
            return Err(self.error_here("expected a table name"));
        }
        // a quoted name is a name here too: `SELECT * FROM "people"` reads that table
        if let TokKind::Str(text) = self.peek_kind() {
            self.advance();
            return Ok(text);
        }
        let start = self.peek().span.start;
        let mut end = self.advance().span.end;
        // a path runs on through the punctuation the lexer split it at, as long as there is no
        // white space: data\customer.dbf is one name, three tokens
        while !self.at_eol()
            && self.peek().span.start == end
            && matches!(self.peek_kind(), TokKind::Dot | TokKind::Slash | TokKind::Backslash | TokKind::Bang | TokKind::Ident(_) | TokKind::Num(..))
        {
            end = self.advance().span.end;
        }
        let text = self.source_text(start, end).trim().to_string();
        // `testdata!customer` names the database the table belongs to; the table is what is opened
        Ok(text.rsplit('!').next().unwrap_or(&text).to_string())
    }

    /// True when the next word begins another part of the query rather than naming something.
    fn at_query_clause(&mut self) -> bool {
        const CLAUSES: &[&str] = &[
            "WHERE", "GROUP", "ORDER", "HAVING", "INTO", "FROM", "INNER", "LEFT", "RIGHT", "FULL", "JOIN", "ON", "TO", "UNION",
        ];
        self.peek().ident().is_some_and(|w| CLAUSES.iter().any(|c| w.eq_ignore_ascii_case(c)))
    }

    /// `GO TOP | BOTTOM | [RECORD] n [IN area]`.
    fn go_stmt(&mut self) -> PResult<StmtKind> {
        self.advance();
        let where_ = if self.eat_kw("TOP") {
            GoWhere::Top
        } else if self.eat_kw("BOTTOM") {
            GoWhere::Bottom
        } else {
            self.eat_kw("RECORD");
            GoWhere::Record(self.expr()?)
        };
        let area = self.in_clause()?;
        Ok(StmtKind::Go { where_, area })
    }

    /// `IN <area>`: the work area a movement command works in rather than the selected one. It
    /// is an alias, a work-area number, or an expression in brackets.
    fn in_clause(&mut self) -> PResult<Option<Expr>> {
        if !self.eat_kw("IN") {
            return Ok(None);
        }
        let span = self.peek().span;
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(Some(e));
        }
        match self.peek_kind() {
            TokKind::Num(..) => Ok(Some(self.expr()?)),
            TokKind::Ident(name) => {
                self.advance();
                Ok(Some(Expr::new(ExprKind::Str(name), span)))
            }
            _ => Ok(None),
        }
    }

    /// True when the next token opens a clause rather than naming a table.
    fn is_clause_word(&mut self) -> bool {
        let tok = self.peek().clone();
        CLAUSE_WORDS.iter().any(|k| kw(&tok, k))
    }

    /// The file a report command names, which stops at the first word of a clause rather than
    /// swallowing it: `REPORT FORM parts FOR price > 2` runs `parts`.
    fn report_path(&mut self) -> String {
        const WORDS: &[&str] = &[
            "FOR", "WHILE", "ALL", "NEXT", "REST", "RECORD", "TO", "PREVIEW", "NOCONSOLE", "SUMMARY",
            "PLAIN", "HEADING", "NAME", "OBJECT", "RANGE", "NOOPTIMIZE", "NODIALOG", "NORESET",
            "NOPAGEEJECT", "NOEJECT", "ENVIRONMENT", "SAMPLE", "ASCII", "IN", "FILE", "PRINTER",
        ];
        self.path_before(WORDS)
    }

    /// A file name as written, stopping before the first word of a clause or a comma rather
    /// than swallowing it.
    fn path_before(&mut self, words: &[&str]) -> String {
        let start = self.peek().span.start;
        let mut end = start;
        // the first word is the file, whatever it is called: a table named `sheet` or `type`
        // is still the file the command names, and a clause can only follow it
        let mut first = true;
        while !self.at_eol() && !self.is(&TokKind::Comma) {
            let tok = self.peek().clone();
            if !first && words.iter().any(|k| kw(&tok, k)) {
                break;
            }
            first = false;
            end = self.advance().span.end;
        }
        self.source_text(start, end).trim().to_string()
    }

    /// The index files an `INDEX` clause names: `x, y.idx, (cName)`, each stopping at a comma
    /// or at the first word that starts a clause of its own.
    fn index_file_list(&mut self) -> Vec<Expr> {
        const WORDS: &[&str] = &[
            "ORDER", "ADDITIVE", "ALIAS", "IN", "EXCLUSIVE", "SHARED", "AGAIN", "NOUPDATE", "TAG", "OF", "TO",
            "ASCENDING", "DESCENDING",
        ];
        let mut files = Vec::new();
        loop {
            let span = self.peek().span;
            if self.eat(&TokKind::LParen) {
                let e = self.expr_or_recover();
                self.expect(&TokKind::RParen, "')'").ok();
                files.push(e);
            } else {
                let text = self.path_before(WORDS);
                if text.is_empty() {
                    break;
                }
                files.push(Expr::new(ExprKind::Str(text), span));
            }
            if !self.eat(&TokKind::Comma) {
                break;
            }
        }
        files
    }

    /// `SET TEXTMERGE [ON | OFF] [NOSHOW]`, `SET TEXTMERGE TO [file | MEMVAR var] [ADDITIVE]
    /// [NOSHOW]` and `SET TEXTMERGE DELIMITERS TO [left [, right]]`.
    ///
    /// All three go to `set_cmd` the way SET INDEX does: the words the line carried are the
    /// first argument, and what it named follows them. A MEMVAR destination is named rather
    /// than read - the text is written into it when the output is closed - so the name goes
    /// through as text.
    fn set_textmerge_stmt(&mut self, setting: Name, delimiters: bool) -> PResult<StmtKind> {
        let span = setting.span;
        let mut words: Vec<&str> = Vec::new();
        let mut args: Vec<Expr> = Vec::new();
        if delimiters {
            words.push("DELIMITERS");
            let _ = self.eat_kw("TO");
            if !self.at_eol() {
                args = self.expr_list()?;
            }
        } else {
            if self.eat_kw("ON") {
                words.push("ON");
            } else if self.eat_kw("OFF") {
                words.push("OFF");
            }
            if self.eat_kw("TO") {
                words.push("TO");
                if self.eat_kw("MEMVAR") {
                    words.push("MEMVAR");
                    self.eat_memvar_prefix();
                    let name = self.expect_ident("variable name")?;
                    args.push(Expr::new(ExprKind::Str(name.upper.clone()), name.span));
                } else if !self.at_eol() && !self.is_kw("ADDITIVE") && !self.is_kw("NOSHOW") && !self.is_kw("SHOW") {
                    args.push(self.merge_file_name(span)?);
                }
            }
            loop {
                if self.eat_kw("ADDITIVE") {
                    words.push("ADDITIVE");
                } else if self.eat_kw("NOSHOW") {
                    words.push("NOSHOW");
                } else if self.eat_kw("SHOW") {
                    words.push("SHOW");
                } else {
                    break;
                }
            }
        }
        self.skip_line_keep_newline();
        let mut exprs = vec![Expr::new(ExprKind::Str(words.join(" ")), span)];
        exprs.append(&mut args);
        Ok(StmtKind::Set { setting, value: SetValue::To(exprs) })
    }

    /// The file `SET TEXTMERGE TO` names: written bare, in quotes, or as an expression in
    /// brackets, as file names are everywhere.
    fn merge_file_name(&mut self, span: Span) -> PResult<Expr> {
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(e);
        }
        if matches!(self.peek_kind(), TokKind::Str(_)) {
            return self.expr();
        }
        let start = self.peek().span.start;
        let mut end = start;
        while !self.at_eol() && !self.is_kw("ADDITIVE") && !self.is_kw("NOSHOW") && !self.is_kw("SHOW") {
            end = self.advance().span.end;
        }
        Ok(Expr::new(ExprKind::Str(self.source_text(start, end).trim().to_string()), span))
    }

    /// `SET INDEX TO [IndexFileList] [ORDER n | file | [TAG] tag] [ADDITIVE]`.
    ///
    /// It goes to `set_cmd` like every other setting, so the arguments carry what it said: the
    /// ORDER first, then the words that changed how the files were opened, then the files.
    fn set_index_stmt(&mut self, setting: Name) -> PResult<StmtKind> {
        let span = setting.span;
        let _ = self.eat_kw("TO");
        let files = self.index_file_list();
        let mut order = None;
        let mut words = String::new();
        loop {
            if self.eat_kw("ORDER") {
                let (tag, descending) = self.order_clause()?;
                order = tag;
                if let Some(down) = descending {
                    words.push_str(if down { " DESCENDING" } else { " ASCENDING" });
                }
            } else if self.eat_kw("ADDITIVE") {
                words.push_str(" ADDITIVE");
            } else {
                break;
            }
        }
        self.skip_line_keep_newline();
        let mut args = vec![
            order.unwrap_or_else(|| Expr::new(ExprKind::Str(String::new()), span)),
            Expr::new(ExprKind::Str(words.trim().to_string()), span),
        ];
        args.extend(files);
        Ok(StmtKind::Set { setting, value: SetValue::To(args) })
    }

    /// A table name as written: `customer`, `data\\customer.dbf`, `..\\shared\\orders`.
    /// The folder a `CD` names, which is a path written as it stands or an expression that comes
    /// to one.
    ///
    /// Measured in Visual FoxPro 9: `CD C:\Windows` goes there and `CD cTarget` is error 202,
    /// "Invalid path or file name.", even with a path in `cTarget` - a bare word is the folder's
    /// own name. Anything that is not a bare word is read as an expression instead, which is what
    /// `CD JUSTPATH(SYS(1271, this))` and `CD SYS(2004) + 'Samples'` both need; `SET DEFAULT TO`
    /// already reads its argument the same way, and CD is that command wearing a shorter name.
    fn path_or_expression(&mut self, span: Span) -> Expr {
        let bare_word = matches!(self.peek_kind(), TokKind::Ident(_))
            && matches!(self.peek_at(1).kind, TokKind::Ident(_) | TokKind::Newline | TokKind::Eof);
        if !bare_word {
            let save_pos = self.pos;
            let save_diags = self.diags.len();
            if let Ok(e) = self.expr()
                && self.at_eol()
            {
                return e;
            }
            self.pos = save_pos;
            self.diags.truncate(save_diags);
        }
        Expr::new(ExprKind::Str(self.bare_path()), span)
    }

    fn bare_path(&mut self) -> String {
        let start = self.peek().span.start;
        let mut end = start;
        while !self.at_eol() && !self.is_clause_word() {
            end = self.advance().span.end;
        }
        self.source_text(start, end).trim().to_string()
    }

    /// The work area an `IN` clause names: a number, an alias, or an expression in brackets.
    /// `IN 0` means the lowest free one, which is how a program opens a table without caring.
    fn area_target(&mut self) -> Expr {
        let span = self.peek().span;
        if self.eat(&TokKind::LParen) {
            let e = self.expr_or_recover();
            self.expect(&TokKind::RParen, "')'").ok();
            return e;
        }
        match self.peek_kind() {
            TokKind::Num(..) => self.expr_or_recover(),
            // `USE IN SELECT('orders')` is how a program closes a table it may or may not have
            // open: a name with a bracket after it is a call, whose answer is the work area
            TokKind::Ident(_) if matches!(self.peek_at(1).kind, TokKind::LParen) => self.expr_or_recover(),
            TokKind::Ident(name) => {
                self.advance();
                Expr::new(ExprKind::Str(name), span)
            }
            _ => Expr::new(ExprKind::Num(0.0, 1, 0), span),
        }
    }

    /// The `WITH args` and `IN program` clauses a `DO` may carry, in either order.
    fn do_clauses(&mut self) -> PResult<(Vec<Arg>, Option<Expr>)> {
        let mut args = Vec::new();
        let mut in_prog = None;
        loop {
            if self.eat_kw("WITH") {
                args = self.arg_list_to_eol()?;
            } else if self.eat_kw("IN") {
                in_prog = Some(self.program_ref()?);
            } else {
                break;
            }
        }
        Ok((args, in_prog))
    }

    /// The program a `DO ... IN` names: a file name as written, or an expression that works
    /// one out. A generated menu writes `IN LOCFILE(...)`, which is the second kind.
    fn program_ref(&mut self) -> PResult<Expr> {
        if self.is(&TokKind::LParen) {
            self.advance();
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(e);
        }
        // a name with a bracket after it is a call rather than a file called that
        if matches!(self.peek_kind(), TokKind::Ident(_)) && matches!(self.peek_at(1).kind, TokKind::LParen) {
            return self.expr();
        }
        let name = self.file_name("program name")?;
        Ok(Expr::new(ExprKind::Str(name.upper.clone()), name.span))
    }

    /// `?` where a file name goes: Visual FoxPro asks for one, which is what GETFILE() does.
    /// The extension it offers is the one the command works on.
    fn ask_for_file(&mut self, extensions: &str) -> Option<Expr> {
        if !self.is(&TokKind::Question) {
            return None;
        }
        let span = self.advance().span;
        let args = vec![Arg { expr: Expr::new(ExprKind::Str(extensions.to_string()), span), by_ref: false }];
        Some(Expr::new(ExprKind::Call { name: Name::new("GETFILE", span), args }, span))
    }

    /// An identifier optionally followed by a `.ext` suffix, joined into one name.
    fn file_name(&mut self, what: &str) -> PResult<Name> {
        let mut name = self.expect_ident(what)?;
        while self.is(&TokKind::Dot) && self.peek().span.start == name.span.end {
            let dot_end = self.peek().span.end;
            if let TokKind::Ident(ext) = self.peek_at(1).kind.clone()
                && self.peek_at(1).span.start == dot_end
            {
                self.advance();
                let ext_tok = self.advance();
                name = Name::new(format!("{}.{}", name.text, ext), name.span.to(ext_tok.span));
                continue;
            }
            break;
        }
        Ok(name)
    }

    fn do_case(&mut self, first: &Token) -> PResult<StmtKind> {
        let mut cases = Vec::new();
        let mut otherwise = None;
        // Nothing but blank lines may appear before the first CASE.
        loop {
            while self.eat(&TokKind::Newline) {}
            if self.is(&TokKind::Eof) || self.is_kw("CASE") || self.is_kw("OTHERWISE") || self.is_kw("ENDCASE") {
                break;
            }
            self.error_here("Statements are not allowed between DO CASE and the first CASE");
            self.skip_line();
        }
        loop {
            while self.eat(&TokKind::Newline) {}
            if self.is(&TokKind::Eof) {
                self.error(first.span, "DO CASE without matching ENDCASE");
                break;
            }
            if self.is_kw("ENDCASE") {
                self.advance();
                break;
            }
            if self.is_kw("OTHERWISE") {
                if otherwise.is_some() {
                    self.error_here("OTHERWISE without matching DO CASE");
                    self.skip_line();
                    continue;
                }
                self.advance();
                self.end_of_line();
                let (body, term) = self.block(&["ENDCASE", "CASE", "OTHERWISE"]);
                otherwise = Some(body);
                match term {
                    Some("ENDCASE") => {
                        self.advance();
                        break;
                    }
                    Some(_) => continue,
                    None => {
                        self.error(first.span, "DO CASE without matching ENDCASE");
                        break;
                    }
                }
            }
            if self.is_kw("CASE") {
                let case_tok = self.advance();
                if otherwise.is_some() {
                    self.error(case_tok.span, "CASE after OTHERWISE");
                }
                let cond = self.expr_or_recover();
                self.end_of_line();
                let (body, term) = self.block(&["ENDCASE", "CASE", "OTHERWISE"]);
                if otherwise.is_none() {
                    cases.push((cond, body));
                }
                match term {
                    Some("ENDCASE") => {
                        self.advance();
                        break;
                    }
                    Some(_) => continue,
                    None => {
                        self.error(first.span, "DO CASE without matching ENDCASE");
                        break;
                    }
                }
            }
            // A terminator of an enclosing block.
            self.error(first.span, "DO CASE without matching ENDCASE");
            break;
        }
        Ok(StmtKind::DoCase { cases, otherwise })
    }

    fn do_form(&mut self) -> PResult<StmtKind> {
        // The form is named by a name expression, so anything that works out to a file name
        // will do: `DO FORM (HOME(2) + "x.scx")`, and `DO FORM LOCFILE("ParamAsk.scx")` where
        // the call is written without parentheses round the whole of it.
        //
        // Written plainly it is a file name rather than a name: it is taken as it stands, up to
        // the first word that starts a clause, so a path and an extension come through and so
        // does a name no identifier could be. Measured - `DO FORM 1_many` opens `1_many.scx`,
        // which is one of the forms Visual FoxPro itself ships, and `DO FORM c:\x\y.scx` reports
        // that exact path when it is missing.
        const CLAUSES: &[&str] = &["NAME", "LINKED", "WITH", "TO", "NOSHOW", "NOREAD"];
        let calls = matches!(self.peek_kind(), TokKind::Ident(_)) && self.peek_at(1).kind == TokKind::LParen;
        let name = if self.is(&TokKind::LParen) || matches!(self.peek_kind(), TokKind::Str(_)) || calls {
            self.expr()?
        } else {
            let span = self.peek().span;
            let text = self.path_before(CLAUSES);
            if text.is_empty() {
                return Err(self.error_here("expected form name"));
            }
            Expr::new(ExprKind::Str(text), span)
        };
        let mut name_var = None;
        let mut linked = false;
        let mut args = Vec::new();
        let mut to_var = None;
        let mut noshow = false;
        loop {
            if self.eat_kw("NAME") {
                // `NAME oForm`, `NAME THISFORM.aForms[n]` and `NAME (cName)`: wherever the
                // form object is to be kept, that is where it goes
                name_var = Some(self.name_target()?);
            } else if self.eat_kw("LINKED") {
                // LINKED belongs to NAME, but it is written wherever the line has room for it:
                // the samples put it after the WITH list. What it asks for, measured: with
                // LINKED, RELEASE of the variable closes the form (_SCREEN.FormCount goes back
                // down); without it the variable goes and the form stays. The flag reaches the
                // host, which does not act on it yet.
                linked = true;
            } else if self.eat_kw("WITH") {
                args = self.arg_list_to_eol()?;
            } else if self.eat_kw("TO") {
                to_var = Some(self.expect_ident("variable name")?);
            } else if self.eat_kw("NOSHOW") {
                noshow = true;
            } else if self.eat_kw("NOREAD") {
                // accepted and ignored
            } else {
                break;
            }
        }
        Ok(StmtKind::DoForm { name, name_var, linked, args, to_var, noshow })
    }

    fn for_stmt(&mut self, first: &Token) -> PResult<StmtKind> {
        self.advance();
        let placeholder = Expr::new(ExprKind::Bool(false), first.span);
        if self.eat_kw("EACH") {
            let (var, collection) = match self.for_each_header() {
                Ok(h) => h,
                Err(_) => {
                    self.skip_line_keep_newline();
                    (Name::new("", first.span), placeholder)
                }
            };
            self.end_of_line();
            let (body, term) = self.block_until(&["ENDFOR", "NEXT"], first, "FOR EACH");
            self.end_for(term);
            return Ok(StmtKind::ForEach { var, collection, body });
        }
        let (var, from, to, step) = match self.for_header() {
            Ok(h) => h,
            Err(_) => {
                self.skip_line_keep_newline();
                (Name::new("", first.span), placeholder.clone(), placeholder, None)
            }
        };
        self.end_of_line();
        let (body, term) = self.block_until(&["ENDFOR", "NEXT"], first, "FOR");
        self.end_for(term);
        Ok(StmtKind::For { var, from, to, step, body })
    }

    fn for_header(&mut self) -> PResult<(Name, Expr, Expr, Option<Expr>)> {
        // `FOR m.i = 1 TO n`: the m. prefix says the counter is a memory variable, which it is
        if self.is_kw("M") && matches!(self.peek_at(1).kind, TokKind::Dot) {
            self.advance();
            self.advance();
        }
        let var = self.expect_ident("loop variable")?;
        self.expect(&TokKind::Eq, "'='")?;
        let from = self.expr()?;
        if !self.eat_kw("TO") {
            return Err(self.expected("TO"));
        }
        let to = self.expr()?;
        let step = if self.eat_kw("STEP") { Some(self.expr()?) } else { None };
        Ok((var, from, to, step))
    }

    fn for_each_header(&mut self) -> PResult<(Name, Expr)> {
        // `FOR EACH m.loForm IN ...`: the m. prefix says the loop variable is a memory
        // variable, which is the only thing a loop variable can be
        self.eat_memvar_prefix();
        let var = self.expect_ident("loop variable")?;
        if !self.eat_kw("IN") {
            return Err(self.expected("IN"));
        }
        // what is walked is any expression - an array, a Collection, `_SCREEN.Forms`
        let collection = self.expr()?;
        // `AS type` and FOXOBJECT can come in either order and neither changes what is walked
        // here: AS is a declaration for the designer's benefit, and FOXOBJECT asks that a COM
        // collection be walked the way FoxPro walks one, which is the only way this runtime
        // walks anything.
        while self.eat_kw("FOXOBJECT") || self.is_kw("AS") {
            self.skip_as_type();
        }
        Ok((var, collection))
    }

    /// Consumes `ENDFOR` / `NEXT [var]`.
    fn end_for(&mut self, term: Option<&'static str>) {
        if term.is_some() {
            self.advance();
            if !self.at_eol() && self.expect_ident("loop variable").is_err() {
                self.skip_line_keep_newline();
            }
        }
    }

    fn wait_stmt(&mut self) -> PResult<StmtKind> {
        self.advance();
        let mut text = None;
        let mut nowait = false;
        let mut timeout = None;
        let mut clear = false;
        let mut to_var = None;
        loop {
            if self.at_eol() {
                break;
            }
            if self.eat_kw("WINDOW") || self.eat_kw("NOCLEAR") {
                continue;
            }
            if self.eat_kw("NOWAIT") {
                nowait = true;
            } else if self.eat_kw("CLEAR") {
                clear = true;
            } else if self.eat_kw("TIMEOUT") {
                timeout = Some(self.expr()?);
            } else if self.eat_kw("TO") {
                to_var = Some(self.expect_ident("variable name")?);
            } else if self.eat_kw("AT") {
                self.expr()?;
                self.expect(&TokKind::Comma, "','")?;
                self.expr()?;
            } else if text.is_none() {
                text = Some(self.expr()?);
            } else {
                return Err(self.expected("WAIT clause"));
            }
        }
        Ok(StmtKind::WaitWindow { text, nowait, timeout, clear, to_var })
    }

    fn release_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if self.is_kw("ALL") {
            self.advance();
            if !self.at_eol() {
                self.warning(
                    first.span,
                    "RELEASE ALL LIKE/EXCEPT skeletons are not supported; all variables are released",
                );
                self.skip_line_keep_newline();
            }
            return Ok(Some(StmtKind::ReleaseAll));
        }
        if let Some(word) = self.peek().ident().map(|s| s.to_string())
            && (kw_text(&word, "MENUS") || kw_text(&word, "POPUPS"))
        {
            self.advance();
            let popup = kw_text(&word, "POPUPS");
            let name = if self.at_eol() || self.is_kw("ALL") {
                self.eat_kw("ALL");
                None
            } else {
                Some(self.menu_name()?)
            };
            // EXTENDED releases the popups a menu owns as well, which is what happens anyway
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::MenuCommand {
                what: MenuVerb::Release,
                name,
                of: None,
                number: None,
                prompt: None,
                key: None,
                message: None,
                text: String::new(),
                flags: if popup { 4 } else { 0 },
            }));
        }
        if let Some(word) = self.peek().ident().map(|s| s.to_string())
            && kw_text(&word, "WINDOWS")
        {
            self.advance();
            let name = if self.at_eol() || self.is_kw("ALL") {
                self.eat_kw("ALL");
                None
            } else {
                Some(self.menu_name()?)
            };
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::WindowCommand {
                        read: None,
                what: WindowVerb::Release,
                name,
                corners: Vec::new(),
                title: None,
                text: String::new(),
                flags: 0,
            }));
        }
        // `RELEASE BAR n | ALL OF popup` and `RELEASE PAD name | ALL OF menu`: one item out of
        // a menu that stays, rather than the whole of it
        if self.is_kw("BAR") || self.is_kw("PAD") {
            let bar = self.is_kw("BAR");
            self.advance();
            let (name, number) = match self.eat_kw("ALL") {
                true => (None, None),
                false if bar => (None, Some(self.expr()?)),
                false => (Some(self.menu_name()?), None),
            };
            let of = self.eat_kw("OF").then(|| self.menu_name()).transpose()?;
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::MenuCommand {
                what: if bar { MenuVerb::ReleaseBar } else { MenuVerb::ReleasePad },
                name,
                of,
                number,
                prompt: None,
                key: None,
                message: None,
                text: String::new(),
                flags: 0,
            }));
        }
        // `RELEASE PROCEDURE a, b` takes those files off the SET PROCEDURE list. It is carried
        // as a setting of its own name, which is where the list is kept.
        if self.is_kw("PROCEDURE") && !self.peek_at(1).is_newline() && !matches!(self.peek_at(1).kind, TokKind::Eof) {
            let tok = self.advance();
            let mut files = Vec::new();
            while !self.at_eol() {
                files.push(self.file_name_word(tok.span, &[])?);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
            self.skip_line_keep_newline();
            let setting = Name::new("RELEASE PROCEDURE", first.span.to(tok.span));
            return Ok(Some(StmtKind::Set { setting, value: SetValue::To(files) }));
        }
        const FORMS: &[&str] = &["LIBRARY", "PROCEDURE", "CLASSLIB"];
        if let Some(word) = self.peek().ident().map(|s| s.to_string()) {
            let next_is_more = !self.peek_at(1).is_newline();
            if let Some(form) =
                FORMS.iter().find(|f| kw_text(&word, f) || (**f == "WINDOWS" && kw_text(&word, "WINDOW")))
                && next_is_more
            {
                self.warning(first.span, format!("RELEASE {form} is ignored: there is nothing of that kind to release"));
                self.skip_line_keep_newline();
                return Ok(None);
            }
        }
        let mut items = vec![self.postfix_expr()?];
        while self.eat(&TokKind::Comma) {
            items.push(self.postfix_expr()?);
        }
        Ok(Some(StmtKind::Release(items)))
    }

    /// One file name in a SET command: `(expression)`, a quoted string, or bare words.
    ///
    /// A bare name runs to the end of the line, to a comma, or to one of the words the command
    /// itself uses - so `SET CLASSLIB TO ..\solution ADDITIVE` names the file and not the word
    /// after it. It is source text rather than an expression because a path is not one:
    /// `..\solution` read as FoxPro is a subtraction.
    fn file_name_word(&mut self, span: Span, stop_at: &[&str]) -> PResult<Expr> {
        if self.eat(&TokKind::LParen) {
            let e = self.expr()?;
            self.expect(&TokKind::RParen, "')'")?;
            return Ok(e);
        }
        if matches!(self.peek_kind(), TokKind::Str(_)) {
            return self.expr();
        }
        let start = self.peek().span.start;
        let mut end = start;
        while !self.at_eol() && !self.is(&TokKind::Comma) && !stop_at.iter().any(|w| self.is_kw(w)) {
            end = self.advance().span.end;
        }
        Ok(Expr::new(ExprKind::Str(self.source_text(start, end)), span))
    }

    fn set_stmt(&mut self) -> PResult<StmtKind> {
        self.advance();
        let mut setting = self.expect_ident("setting name")?;
        // Two-word settings such as `SET STATUS BAR ON` / `SET NOTIFY CURSOR OFF`.
        if let TokKind::Ident(second) = self.peek_kind()
            && !kw_text(&second, "ON")
            && !kw_text(&second, "OFF")
            && !kw_text(&second, "TO")
        {
            let third = self.peek_at(1).clone();
            if kw(&third, "ON") || kw(&third, "OFF") || kw(&third, "TO") {
                let t = self.advance();
                setting = Name::new(format!("{} {}", setting.text, second), setting.span.to(t.span));
            }
        }
        // Every SET is words - a path, a tag, a filter, its own vocabulary - rather than an
        // expression, so `SET PATH TO &lcPath.` is a path only once the variable has been read.
        // The line waits and is read then, whole, the way Visual FoxPro reads every line.
        self.no_macro_ahead()?;
        if kw_text(&setting.text, "DATABASE") {
            let _ = self.eat_kw("TO");
            let name = if self.at_eol() { None } else { Some(self.table_name()?) };
            self.skip_line_keep_newline();
            return Ok(StmtKind::Database { what: DbCommand::Set, name, target: None, sql: String::new(), flags: 0 });
        }
        if kw_text(&setting.text, "SYSMENU") {
            let words = self.rest_of_line_text().unwrap_or_default();
            self.skip_line_keep_newline();
            return Ok(StmtKind::MenuCommand {
                what: MenuVerb::SetSysMenu,
                name: None,
                of: None,
                number: None,
                prompt: None,
                key: None,
                message: None,
                text: words,
                flags: 0,
            });
        }
        // `SET MARK OF` and `SET SKIP OF` name a pad or a bar; `SET MARK TO` is the date
        // separator and `SET SKIP TO` a relation, so the OF is what tells them apart.
        if (kw_text(&setting.text, "MARK") || kw_text(&setting.text, "SKIP")) && self.is_kw("OF") {
            let mark = kw_text(&setting.text, "MARK");
            return self.set_of_stmt(mark);
        }
        // `SET MESSAGE TO <expr>` gives the words the message line shows. `SET MESSAGE TO 2
        // LEFT` says which line of a character screen that is, which is not a screen this
        // runtime has, so it is left to the settings to accept and ignore.
        if kw_text(&setting.text, "MESSAGE") && self.is_kw("TO") {
            let save_pos = self.pos;
            let save_diags = self.diags.len();
            self.advance();
            let text = if self.at_eol() {
                Some(Expr::new(ExprKind::Str(String::new()), setting.span))
            } else {
                match self.expr() {
                    Ok(e) if self.at_eol() && !matches!(e.kind, ExprKind::Num(..)) => Some(e),
                    _ => None,
                }
            };
            if let Some(text) = text {
                return Ok(StmtKind::MenuCommand {
                    what: MenuVerb::SetMessage,
                    name: None,
                    of: None,
                    number: None,
                    prompt: Some(text),
                    key: None,
                    message: None,
                    text: String::new(),
                    flags: 0,
                });
            }
            self.pos = save_pos;
            self.diags.truncate(save_diags);
        }
        // `SET DEBUGOUT TO [<file> [ADDITIVE]]`: the file that takes what ASSERT and DEBUGOUT
        // say. The name is written bare or as an expression in brackets, as file names are, and
        // ADDITIVE adds to what is in the file rather than replacing it. Both go through as
        // arguments, so the setting reads them the way every other one does.
        if kw_text(&setting.text, "DEBUGOUT") {
            let span = setting.span;
            let _ = self.eat_kw("TO");
            if self.at_eol() {
                self.skip_line_keep_newline();
                return Ok(StmtKind::Set { setting, value: SetValue::To(Vec::new()) });
            }
            let file = if self.eat(&TokKind::LParen) {
                let e = self.expr()?;
                self.expect(&TokKind::RParen, "')'")?;
                e
            } else if matches!(self.peek_kind(), TokKind::Str(_)) {
                self.expr()?
            } else {
                let start = self.peek().span.start;
                let mut end = start;
                while !self.at_eol() && !self.is_kw("ADDITIVE") {
                    end = self.advance().span.end;
                }
                Expr::new(ExprKind::Str(self.source_text(start, end).trim().to_string()), span)
            };
            let additive = self.eat_kw("ADDITIVE");
            self.skip_line_keep_newline();
            return Ok(StmtKind::Set {
                setting,
                value: SetValue::To(vec![file, Expr::new(ExprKind::Bool(additive), span)]),
            });
        }
        // `SET CLASSLIB TO [FileName [, FileName...]] [IN cApp] [ALIAS cAlias] [ADDITIVE]`: the
        // class libraries whose classes a program may name. The file is written bare, in
        // brackets or as a string, the way file names are everywhere, and a bare one carries no
        // extension - `SET CLASSLIB TO ..\solution` is the Solution samples' own way of writing
        // it. Measured: several files separated by commas load in the order they are written.
        //
        // The alias and the file list go through as arguments: the flag first, then the alias,
        // then the files, which is what the CLASSLIB arm reads them back as.
        // `SET LIBRARY TO (expression)` is how real code names a library, because the path is
        // nearly always worked out: foxtools.fll is under HOME(), and an application's own is
        // beside itself. Read as the tail of the line, the brackets became part of the name and
        // the file was never found. One name only - unlike CLASSLIB, a library list is not a
        // thing the command takes.
        if kw_text(&setting.text, "LIBRARY") {
            let span = setting.span;
            let _ = self.eat_kw("TO");
            let named = match self.at_eol() {
                true => Expr::new(ExprKind::Str(String::new()), span),
                false => self.file_name_word(span, &["ADDITIVE"])?,
            };
            let additive = self.eat_kw("ADDITIVE");
            self.skip_line_keep_newline();
            let args = vec![Expr::new(ExprKind::Bool(additive), span), named];
            return Ok(StmtKind::Set { setting, value: SetValue::To(args) });
        }
        // `SET PROCEDURE TO a, b [ADDITIVE]`: the files are names, as a class library's are, and
        // `SET PROCEDURE TO` with nothing after it empties the list
        if kw_text(&setting.text, "PROCEDURE") && self.is_kw("TO") {
            let span = setting.span;
            self.advance();
            let mut files: Vec<Expr> = Vec::new();
            while !self.at_eol() && !self.is_kw("ADDITIVE") {
                files.push(self.file_name_word(span, &["ADDITIVE"])?);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
            let additive = self.eat_kw("ADDITIVE");
            self.skip_line_keep_newline();
            let mut args = vec![Expr::new(ExprKind::Bool(additive), span)];
            args.append(&mut files);
            return Ok(StmtKind::Set { setting, value: SetValue::To(args) });
        }
        if kw_text(&setting.text, "CLASSLIB") {
            let span = setting.span;
            let _ = self.eat_kw("TO");
            let mut files: Vec<Expr> = Vec::new();
            while !self.at_eol() && !self.is_kw("ADDITIVE") && !self.is_kw("ALIAS") && !self.is_kw("IN") {
                files.push(self.file_name_word(span, &["ADDITIVE", "ALIAS", "IN"])?);
                if !self.eat(&TokKind::Comma) {
                    break;
                }
            }
            // IN names an .app or .exe to read the library out of, which this runtime does not
            // open; the words are taken so the rest of the line still reads.
            if self.eat_kw("IN") {
                let _ = self.file_name_word(span, &["ADDITIVE", "ALIAS"])?;
            }
            let alias = match self.eat_kw("ALIAS") {
                true => self.file_name_word(span, &["ADDITIVE"])?,
                false => Expr::new(ExprKind::Str(String::new()), span),
            };
            let additive = self.eat_kw("ADDITIVE");
            self.skip_line_keep_newline();
            let mut args = vec![Expr::new(ExprKind::Bool(additive), span), alias];
            args.append(&mut files);
            return Ok(StmtKind::Set { setting, value: SetValue::To(args) });
        }
        // `SET PATH TO a,b` names folders rather than expressions: read as an expression list
        // `a,b` would be two variables. Measured in the product, which tells the two apart by
        // how the line starts: `SET PATH TO a,b` answers `A,B`, `SET PATH TO ("b, a")` answers
        // what the expression comes to, and `SET PATH TO 'a' , 'b'` answers `A` - an expression
        // and then nothing, the rest of the line dropped. So a line that opens with a quote or a
        // bracket is an expression and every other one is the folders as written.
        if kw_text(&setting.text, "PATH") && self.is_kw("TO") {
            self.advance();
            let quoted = matches!(self.peek_kind(), TokKind::Str(_)) || self.is(&TokKind::LParen);
            if quoted
                && let Ok(text) = self.expr()
            {
                self.skip_line_keep_newline();
                return Ok(StmtKind::Set { setting, value: SetValue::To(vec![text]) });
            }
            let words = self.rest_of_line_text().unwrap_or_default();
            self.skip_line_keep_newline();
            return Ok(StmtKind::Set { setting, value: SetValue::Word(words) });
        }
        // `SET EVENTLIST TO Click, DblClick` names events, and `SET TOPIC TO <expr>` keeps the
        // expression to work out when help is asked for - SET("TOPIC") answers with it as it was
        // written, quotes and all. Neither is an expression to read here.
        if (kw_text(&setting.text, "EVENTLIST") || kw_text(&setting.text, "TOPIC")) && self.is_kw("TO") {
            self.advance();
            let words = self.rest_of_line_text().unwrap_or_default();
            self.skip_line_keep_newline();
            return Ok(StmtKind::Set { setting, value: SetValue::Word(words) });
        }
        // `SET FIELDS TO name, pay` names columns rather than expressions: read as expressions,
        // `name` would be the field's value and the list would come out as the record's contents.
        if kw_text(&setting.text, "FIELDS") && self.is_kw("TO") {
            self.advance();
            let words = self.rest_of_line_text().unwrap_or_default();
            self.skip_line_keep_newline();
            return Ok(StmtKind::Set { setting, value: SetValue::Word(words) });
        }
        if kw_text(&setting.text, "FILTER") {
            let _ = self.eat_kw("TO");
            let text = self.rest_of_line_text().unwrap_or_default();
            self.skip_line_keep_newline();
            return Ok(StmtKind::SetFilter(text));
        }
        if kw_text(&setting.text, "RELATION") {
            return self.set_relation_stmt();
        }
        if kw_text(&setting.text, "ORDER") {
            let _ = self.eat_kw("TO");
            let (tag, descending) = self.order_clause()?;
            // `IN alias` puts another table in that order without selecting it first
            let area = if self.eat_kw("IN") { self.area_ref().ok() } else { None };
            self.skip_line_keep_newline();
            return Ok(StmtKind::SetOrder { tag, descending, area });
        }
        if kw_text(&setting.text, "INDEX") {
            return self.set_index_stmt(setting);
        }
        // `SET TEXTMERGE DELIMITERS TO` is a two-word setting, so the rule above has already
        // taken DELIMITERS into the name; every other form leaves the name one word
        let (first_word, second_word) = setting.text.split_once(' ').unwrap_or((setting.text.as_str(), ""));
        if kw_text(first_word, "TEXTMERGE") {
            let delimiters = !second_word.is_empty();
            return self.set_textmerge_stmt(setting, delimiters);
        }
        // A switch can have words after it - `SET COMPATIBLE OFF NOPROMPT`, `SET TALK OFF
        // NOWINDOW` - which are kept for the setting to make of them.
        for (word, on) in [("ON", true), ("OFF", false)] {
            if self.eat_kw(word) {
                if self.at_eol() {
                    return Ok(StmtKind::Set { setting, value: if on { SetValue::On } else { SetValue::Off } });
                }
                let words = self.rest_of_line_text().unwrap_or_default();
                self.skip_line_keep_newline();
                return Ok(StmtKind::Set { setting, value: SetValue::Switch { on, words } });
            }
        }
        if self.eat_kw("TO") {
            if self.at_eol() {
                return Ok(StmtKind::Set { setting, value: SetValue::Word(String::new()) });
            }
            // a lone word after TO is normally the setting's own vocabulary - SET DATE TO
            // AMERICAN - but the settings below take a value, and every reference example
            // restores one by naming the variable it was saved in
            let takes_value = matches!(
                setting.upper.as_str(),
                "CURRENCY" | "DECIMALS" | "FDOW" | "FWEEK" | "HOURS" | "MARK" | "MEMOWIDTH" | "NULLDISPLAY" | "POINT" | "SEPARATOR"
            );
            let bare_word = !takes_value
                && matches!(self.peek_kind(), TokKind::Ident(_))
                && matches!(self.peek_at(1).kind, TokKind::Ident(_) | TokKind::Newline | TokKind::Eof);
            if !bare_word {
                let save_pos = self.pos;
                let save_diags = self.diags.len();
                if let Ok(exprs) = self.expr_list()
                    && self.at_eol()
                {
                    return Ok(StmtKind::Set { setting, value: SetValue::To(exprs) });
                }
                self.pos = save_pos;
                self.diags.truncate(save_diags);
            }
            let words = self.rest_of_line_text().unwrap_or_default();
            return Ok(StmtKind::Set { setting, value: SetValue::Word(words) });
        }
        // `SET DATE AMERICAN` and `SET ENGINEBEHAVIOR 70`: the setting's own vocabulary with no
        // TO in front of it, which is a word for some settings and a number for others.
        let mut n = 0;
        while matches!(self.peek_at(n).kind, TokKind::Ident(_) | TokKind::Num(..)) {
            n += 1;
        }
        if n > 0 && self.peek_at(n).is_newline() {
            let words = self.rest_of_line_text().unwrap_or_default();
            return Ok(StmtKind::Set { setting, value: SetValue::Word(words) });
        }
        self.skip_line_keep_newline();
        Ok(StmtKind::Set { setting, value: SetValue::Word(String::new()) })
    }

    fn try_stmt(&mut self, first: &Token) -> PResult<StmtKind> {
        self.advance();
        self.end_of_line();
        let (body, mut term) = self.block_until(&["ENDTRY", "CATCH", "FINALLY"], first, "TRY");
        let mut catches = Vec::new();
        let mut finally = None;
        while let Some(t) = term {
            match t {
                // VFP allows several CATCH clauses, each with its own WHEN, tried in order
                "CATCH" => {
                    self.advance();
                    let mut var = None;
                    let mut when = None;
                    if self.eat_kw("TO") {
                        // `CATCH TO m.oError`: the m. prefix says the error object goes into
                        // a memory variable, which is the only place it could have gone
                        self.eat_memvar_prefix();
                        var = Some(self.expect_ident("variable name")?);
                    }
                    if self.eat_kw("WHEN") {
                        when = Some(self.expr()?);
                    }
                    self.end_of_line();
                    let (b, next) = self.block_until(&["ENDTRY", "CATCH", "FINALLY"], first, "TRY");
                    catches.push(CatchClause { var, when, body: b });
                    term = next;
                }
                "FINALLY" => {
                    let fin_tok = self.advance();
                    self.end_of_line();
                    let (b, next) = self.block_until(&["ENDTRY", "CATCH", "FINALLY"], first, "TRY");
                    if finally.is_some() {
                        self.error(fin_tok.span, "FINALLY without matching TRY");
                    } else {
                        finally = Some(b);
                    }
                    term = next;
                }
                _ => {
                    self.advance();
                    break;
                }
            }
        }
        Ok(StmtKind::Try { body, catches, finally })
    }

    fn text_stmt(&mut self, first: &Token) -> PResult<StmtKind> {
        self.advance();
        let mut target = None;
        let mut additive = false;
        let mut textmerge = false;
        let mut noshow = false;
        loop {
            if self.eat_kw("TO") {
                target = Some(self.target()?);
                additive = self.eat_kw("ADDITIVE");
            } else if self.eat_kw("TEXTMERGE") {
                textmerge = true;
            } else if self.eat_kw("NOSHOW") {
                noshow = true;
            } else if self.eat_kw("ADDITIVE") {
                additive = true;
            } else if self.eat_kw("FLAGS") || self.eat_kw("PRETEXT") {
                self.expr()?;
            } else {
                break;
            }
        }
        if !self.is(&TokKind::Newline) || !matches!(self.peek_at(1).kind, TokKind::RawText(_)) {
            if self.at_eol() {
                return Err(self.error(first.span, "TEXT without matching ENDTEXT"));
            }
            return Err(self.expected("end of line"));
        }
        self.advance();
        let raw = match self.advance().kind {
            TokKind::RawText(s) => s,
            _ => unreachable!(),
        };
        if self.is_kw("ENDTEXT") {
            self.advance();
        }
        Ok(StmtKind::Text { target, additive, textmerge, noshow, raw })
    }

    fn on_stmt(&mut self, first: &Token) -> PResult<Option<StmtKind>> {
        self.advance();
        if self.eat_kw("ERROR") {
            let command = self.rest_of_line_text();
            return Ok(Some(StmtKind::OnError(command)));
        }
        if self.is_kw("KEY") && kw(self.peek_at(1), "LABEL") {
            self.advance();
            self.advance();
            if self.at_eol() {
                return Err(self.expected("key label"));
            }
            // a key label is one unbroken run - F5, CTRL+W, ALT+F5 - so it ends at the first
            // space, and what follows the space is the command
            let start = self.peek().span.start;
            let mut end = self.advance().span.end;
            while !self.at_eol() && self.peek().span.start == end && !self.is(&TokKind::Eq) {
                end = self.advance().span.end;
            }
            if end == start {
                return Err(self.expected("key label"));
            }
            let key = self.source_text(start, end);
            let command = self.rest_of_line_text();
            return Ok(Some(StmtKind::OnKeyLabel { key, command }));
        }
        if self.is_kw("PAD") || self.is_kw("BAR") {
            return self.on_menu_stmt(false);
        }
        if self.eat_kw("SELECTION") {
            return self.on_menu_stmt(true);
        }
        // ON EXIT BAR/PAD/POPUP/MENU runs a command as the choice is left. Nothing here can
        // leave a choice without making it, so the command is read and dropped.
        if self.is_kw("EXIT") && MENU_KINDS.iter().any(|w| kw(self.peek_at(1), w)) {
            self.advance();
            self.skip_line_keep_newline();
            return Ok(Some(StmtKind::MenuCommand {
                what: MenuVerb::OnExit,
                name: None,
                of: None,
                number: None,
                prompt: None,
                key: None,
                message: None,
                text: String::new(),
                flags: 0,
            }));
        }
        // the rest hang a command off something that may happen: the Escape key, a shutdown,
        // a page break, a read that could not read what it was given
        const EVENTS: &[&str] = &["ESCAPE", "SHUTDOWN", "READERROR", "PAGE", "KEY", "APLABOUT", "SELECTION"];
        let word = self.peek().ident().unwrap_or_default().to_ascii_uppercase();
        let Some(event) = EVENTS.iter().find(|e| kw_text(&word, e)) else {
            let what = if word.is_empty() { "ON".to_string() } else { format!("ON {word}") };
            self.warning(first.span, format!("{what} is ignored: the FoxDev runtime has nothing to hang it on"));
            self.skip_line_keep_newline();
            return Ok(None);
        };
        self.advance();
        let mut what = (*event).to_string();
        // ON PAGE AT LINE n, and ON KEY = <code>, each name which one of them it is
        if event == &"PAGE" && self.eat_kw("AT") {
            self.eat_kw("LINE");
            let at = self.peek().span.start;
            let _ = self.expr();
            what = format!("PAGE {}", self.source_text(at, self.prev_span().end));
        } else if event == &"KEY" && self.eat(&TokKind::Eq) {
            let at = self.peek().span.start;
            let _ = self.expr();
            what = format!("KEY {}", self.source_text(at, self.prev_span().end));
        }
        let command = self.rest_of_line_text();
        self.skip_line_keep_newline();
        Ok(Some(StmtKind::OnEvent { what, command }))
    }

    fn directive(&mut self) -> PResult<Option<StmtKind>> {
        let hash = self.advance_raw();
        let name_tok = self.advance_raw();
        let Some(name) = name_tok.ident().map(|s| s.to_ascii_uppercase()) else {
            return Err(self.error(hash.span, "Syntax error: expected preprocessor directive after '#'"));
        };
        let line_start = hash.span.start;
        let mut line_end = name_tok.span.end;
        match name.as_str() {
            "DEFINE" | "UNDEF" => {
                let ident = self.advance_raw();
                let Some(macro_name) = ident.ident().map(|s| s.to_ascii_uppercase()) else {
                    return Err(self.error(ident.span, format!("Syntax error: expected constant name after #{name}")));
                };
                let mut body = Vec::new();
                while !self.toks[self.pos].is_newline() {
                    let t = self.advance_raw();
                    line_end = t.span.end;
                    body.push(t);
                }
                line_end = line_end.max(ident.span.end);
                if name == "DEFINE" {
                    // A name that already stands for something keeps what it stands for:
                    // measured, a program with `#DEFINE PICK "first"` above
                    // `#DEFINE PICK "second"` prints "first". So a method that defines a name
                    // its form's header file already defined is the one that loses, and
                    // `#UNDEF` in between is what lets the second one through.
                    self.defines.entry(macro_name).or_insert(body);
                } else {
                    self.defines.remove(&macro_name);
                }
            }
            // the conditional directives: what the condition says decides which lines the
            // parser sees at all, which is what makes a header file's #DEFINEs switch code on
            "IF" | "IFDEF" | "IFNDEF" => {
                let start = self.pos;
                while !self.toks[self.pos].is_newline() {
                    line_end = self.advance_raw().span.end;
                }
                let condition = match name.as_str() {
                    "IF" => self.constant_condition(start, self.pos),
                    _ => {
                        let named = self.toks[start].ident().map(|s| s.to_ascii_uppercase()).unwrap_or_default();
                        self.defines.contains_key(&named) == (name == "IFDEF")
                    }
                };
                if !condition {
                    self.skip_to_else();
                }
            }
            // reaching #ELSE or #ELIF from inside a block that was kept means the rest is not
            "ELSE" | "ELIF" => {
                while !self.toks[self.pos].is_newline() {
                    line_end = self.advance_raw().span.end;
                }
                self.skip_to_endif();
            }
            "ENDIF" => {
                while !self.toks[self.pos].is_newline() {
                    line_end = self.advance_raw().span.end;
                }
            }
            // a header file's constants belong to this program: whoever asked for the parse
            // hands the text over, because reading files is not the parser's to do
            "INCLUDE" | "INSERT" => {
                let mut file = String::new();
                while !self.toks[self.pos].is_newline() {
                    let t = self.advance_raw();
                    line_end = t.span.end;
                    match &t.kind {
                        TokKind::Str(s) => file.push_str(s),
                        TokKind::Ident(word) => file.push_str(word),
                        TokKind::Dot => file.push('.'),
                        _ => {}
                    }
                }
                let stem = header_stem(&file);
                match self.headers.get(&stem) {
                    Some(text) => {
                        let text = text.clone();
                        self.read_header(&stem, &text);
                    }
                    None => {
                        let span = hash.span.to(name_tok.span);
                        self.warning(span, format!("#{name} {file}: the file was not found"));
                    }
                }
            }
            // `#NAME` names the object a class definition makes, which nothing here reads
            "NAME" | "ITSEXPRESSION" => {
                while !self.toks[self.pos].is_newline() {
                    line_end = self.advance_raw().span.end;
                }
            }
            _ => {
                let span = hash.span.to(name_tok.span);
                return Err(self.error(span, format!("Unknown preprocessor directive #{name}")));
            }
        }
        Ok(Some(StmtKind::Directive(self.source_text(line_start, line_end))))
    }

    /// Reads a header file's constants into this parse: its `#DEFINE` lines, and the ones of
    /// any header it includes in turn.
    fn read_header(&mut self, stem: &str, text: &str) {
        if self.headers_open.iter().any(|open| open == stem) {
            return;
        }
        self.headers_open.push(stem.to_string());
        self.read_header_text(text);
        self.headers_open.pop();
    }

    fn read_header_text(&mut self, text: &str) {
        let tokens = crate::lexer::lex(text).tokens;
        let mut at = 0usize;
        while at < tokens.len() {
            if !matches!(tokens[at].kind, TokKind::Hash) {
                // a line that is not a directive is nothing a header brings with it
                while at < tokens.len() && !tokens[at].is_newline() {
                    at += 1;
                }
                at += 1;
                continue;
            }
            let word = tokens.get(at + 1).and_then(|t| t.ident()).unwrap_or_default().to_ascii_uppercase();
            at += 2;
            let start = at;
            while at < tokens.len() && !tokens[at].is_newline() {
                at += 1;
            }
            match word.as_str() {
                "DEFINE" => {
                    if let Some(name) = tokens.get(start).and_then(|t| t.ident()).map(|s| s.to_ascii_uppercase()) {
                        // as in a program: the first definition of a name is the one that stands
                        self.defines.entry(name).or_insert_with(|| tokens[start + 1..at].to_vec());
                    }
                }
                "UNDEF" => {
                    if let Some(name) = tokens.get(start).and_then(|t| t.ident()).map(|s| s.to_ascii_uppercase()) {
                        self.defines.remove(&name);
                    }
                }
                "INCLUDE" | "INSERT" => {
                    let file: String = tokens[start..at]
                        .iter()
                        .map(|t| match &t.kind {
                            TokKind::Str(s) => s.clone(),
                            TokKind::Ident(w) => w.clone(),
                            TokKind::Dot => ".".to_string(),
                            _ => String::new(),
                        })
                        .collect();
                    let stem = header_stem(&file);
                    if let Some(text) = self.headers.get(&stem).cloned() {
                        self.read_header(&stem, &text);
                    }
                }
                _ => {}
            }
            at += 1;
        }
    }

    /// Whether a `#IF` condition holds, out of the tokens between `start` and `end`.
    ///
    /// The condition has to be answerable while the program is being read, so it is a constant
    /// or a `#DEFINE`d name standing for one: a number that is not zero, `.T.`, or a name that
    /// was defined as either.
    fn constant_condition(&mut self, start: usize, end: usize) -> bool {
        let mut tokens: Vec<Token> = self.toks[start..end].to_vec();
        // a defined name stands for what it was defined as, once
        if tokens.len() == 1
            && let Some(name) = tokens[0].ident().map(|s| s.to_ascii_uppercase())
            && let Some(body) = self.defines.get(&name)
        {
            tokens = body.clone();
        }
        match tokens.first().map(|t| &t.kind) {
            Some(TokKind::Num(n, ..)) => *n != 0.0,
            Some(TokKind::True) => true,
            Some(TokKind::False) => false,
            Some(TokKind::Ident(word)) => {
                let upper = word.to_ascii_uppercase();
                upper == "T" || upper == "Y" || self.defines.contains_key(&upper)
            }
            _ => false,
        }
    }

    /// Skips the lines a false `#IF` covers, stopping after the `#ELSE`, `#ELIF` or `#ENDIF`
    /// that ends it. A nested conditional inside is skipped whole.
    fn skip_to_else(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.directive_ahead() {
                None => return,
                Some(word) => match word.as_str() {
                    "IF" | "IFDEF" | "IFNDEF" => {
                        depth += 1;
                        self.skip_directive_line();
                    }
                    "ENDIF" => {
                        self.skip_directive_line();
                        if depth == 0 {
                            return;
                        }
                        depth -= 1;
                    }
                    "ELSE" | "ELIF" => {
                        self.skip_directive_line();
                        if depth == 0 {
                            return;
                        }
                    }
                    _ => self.skip_directive_line(),
                },
            }
        }
    }

    /// Skips to the end of a conditional whose kept half has already been read.
    fn skip_to_endif(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.directive_ahead() {
                None => return,
                Some(word) => match word.as_str() {
                    "IF" | "IFDEF" | "IFNDEF" => {
                        depth += 1;
                        self.skip_directive_line();
                    }
                    "ENDIF" => {
                        self.skip_directive_line();
                        if depth == 0 {
                            return;
                        }
                        depth -= 1;
                    }
                    _ => self.skip_directive_line(),
                },
            }
        }
    }

    /// The directive the next line starts with, when it starts with one; the line is not read.
    fn directive_ahead(&mut self) -> Option<String> {
        // every line up to the next directive is skipped, so this is what stops the skipping
        loop {
            while self.toks[self.pos].is_newline() {
                if matches!(self.toks[self.pos].kind, TokKind::Eof) {
                    return None;
                }
                self.pos += 1;
            }
            if matches!(self.toks[self.pos].kind, TokKind::Eof) {
                return None;
            }
            if matches!(self.toks[self.pos].kind, TokKind::Hash) {
                return self.toks.get(self.pos + 1).and_then(|t| t.ident()).map(|s| s.to_ascii_uppercase());
            }
            self.skip_directive_line();
        }
    }

    /// Reads to the end of the line without looking at what is on it, stopping on the line
    /// end itself: whoever asked for the skip still has to be given one.
    fn skip_directive_line(&mut self) {
        while !self.toks[self.pos].is_newline() {
            self.pos += 1;
        }
    }

    /// Assignment (`target = value`), a call used as a statement, or "Unrecognized command verb".
    fn assign_or_call(&mut self) -> PResult<Option<StmtKind>> {
        let first = self.peek().clone();
        let expr = self.postfix_expr()?;
        if self.eat(&TokKind::Eq) {
            self.check_target(&expr)?;
            let value = self.expr()?;
            return Ok(Some(StmtKind::Assign { target: expr, value }));
        }
        match expr.kind {
            ExprKind::Call { .. } | ExprKind::MethodCall { .. } => Ok(Some(StmtKind::ExprStmt(expr))),
            // `THIS.ResizeChars` on its own line calls the method; reading a property and
            // throwing the value away is not something a statement can mean.
            ExprKind::Member { obj, name } => {
                let span = expr.span;
                Ok(Some(StmtKind::ExprStmt(Expr::new(ExprKind::MethodCall { obj, name, args: Vec::new() }, span))))
            }
            _ => Err(self.error(first.span, match first.ident() { Some(v) => format!("Unrecognized command verb '{}'", v.to_ascii_uppercase()), None => "Unrecognized command verb".to_string() })),
        }
    }

    /// An assignment target: variable, member, index or array element.
    fn target(&mut self) -> PResult<Expr> {
        let e = self.postfix_expr()?;
        self.check_target(&e)?;
        Ok(e)
    }

    fn check_target(&mut self, e: &Expr) -> PResult<()> {
        let ok = match &e.kind {
            ExprKind::Var(_) | ExprKind::Member { .. } | ExprKind::Index { .. } => true,
            // `.&cMemberCount = 0`: the member a variable names can be written like any other
            ExprKind::MemberByName { args: None, .. } => true,
            ExprKind::Call { args, .. } => !args.is_empty(),
            // `oCombo.Selected(1) = .T.`: an array property indexed with parentheses, which VFP
            // allows anywhere brackets are allowed.
            ExprKind::MethodCall { args, .. } => !args.is_empty(),
            // `THISFORM = .NULL.` compiles in Visual FoxPro - measured, by writing it out and
            // finding no `.err` beside the `.fxp` - and refuses only when it runs, with 1930.
            // The shipped `builderd.vcx` writes exactly that, so refusing it here would take the
            // whole method with it.
            ExprKind::This | ExprKind::ThisForm | ExprKind::ThisFormSet | ExprKind::Screen => true,
            _ => false,
        };
        if ok { Ok(()) } else { Err(self.error(e.span, "Invalid assignment target")) }
    }

    // ----- expressions ----------------------------------------------------------------------

    fn expr(&mut self) -> PResult<Expr> {
        self.or_expr()
    }

    fn expr_list(&mut self) -> PResult<Vec<Expr>> {
        let mut items = vec![self.expr()?];
        while self.eat(&TokKind::Comma) {
            items.push(self.expr()?);
        }
        Ok(items)
    }

    /// Comma-separated expressions up to and including `close`.
    fn expr_list_until(&mut self, close: &TokKind, what: &str) -> PResult<Vec<Expr>> {
        let mut items = Vec::new();
        if self.eat(close) {
            return Ok(items);
        }
        loop {
            items.push(self.expr()?);
            if self.eat(&TokKind::Comma) {
                continue;
            }
            self.expect(close, what)?;
            return Ok(items);
        }
    }

    fn binary(op: BinOp, left: Expr, right: Expr) -> Expr {
        let span = left.span.to(right.span);
        Expr::new(ExprKind::Binary { op, left: Box::new(left), right: Box::new(right) }, span)
    }

    fn or_expr(&mut self) -> PResult<Expr> {
        let mut left = self.and_expr()?;
        while self.is(&TokKind::DotOr) || self.is_kw("OR") {
            self.advance();
            let right = self.and_expr()?;
            left = Self::binary(BinOp::Or, left, right);
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> PResult<Expr> {
        let mut left = self.not_expr()?;
        while self.is(&TokKind::DotAnd) || self.is_kw("AND") {
            self.advance();
            let right = self.not_expr()?;
            left = Self::binary(BinOp::And, left, right);
        }
        Ok(left)
    }

    fn is_not(&mut self) -> bool {
        self.is(&TokKind::DotNot) || self.is(&TokKind::Bang) || self.is_kw("NOT")
    }

    fn not_expr(&mut self) -> PResult<Expr> {
        if self.is_not() {
            let t = self.advance();
            let e = self.not_expr()?;
            let span = t.span.to(e.span);
            return Ok(Expr::new(ExprKind::Unary { op: UnOp::Not, expr: Box::new(e) }, span));
        }
        self.cmp_expr()
    }

    fn cmp_expr(&mut self) -> PResult<Expr> {
        let mut left = self.add_expr()?;
        loop {
            // `IN` is SQL's, so it is a comparison only inside a query; elsewhere in FoxPro it is
            // a clause word (DO x IN y, SKIP IN alias) and has to stay one
            if self.in_query > 0 && (self.is_kw("IN") || (self.is_not() && kw(self.peek_at(1), "IN"))) {
                let negated = self.is_not();
                if negated {
                    self.advance();
                }
                let span = self.advance().span;
                let right = self.add_expr()?;
                left = match right.kind {
                    ExprKind::AnySubquery(q) => {
                        let s = left.span.to(right.span);
                        Expr::new(ExprKind::InSubquery { value: Box::new(left), query: q, negated }, s)
                    }
                    _ => return Err(self.error(span, "IN needs a subquery in brackets: IN (SELECT ...)")),
                };
                continue;
            }
            let op = match self.peek_kind() {
                TokKind::Eq => BinOp::Eq,
                TokKind::EqEq => BinOp::ExactEq,
                TokKind::Ne => BinOp::Ne,
                TokKind::Lt => BinOp::Lt,
                TokKind::Le => BinOp::Le,
                TokKind::Gt => BinOp::Gt,
                TokKind::Ge => BinOp::Ge,
                TokKind::Dollar => BinOp::Contains,
                _ => break,
            };
            let op_span = self.advance().span;
            let right = self.add_expr()?;
            left = match right.kind {
                ExprKind::AnySubquery(q) if op == BinOp::Eq || op == BinOp::ExactEq => {
                    let s = left.span.to(right.span);
                    Expr::new(ExprKind::InSubquery { value: Box::new(left), query: q, negated: false }, s)
                }
                ExprKind::AnySubquery(_) => {
                    return Err(self.error(op_span, "only = ANY and = SOME are supported; the other comparisons against a subquery are not"));
                }
                _ => Self::binary(op, left, right),
            };
        }
        Ok(left)
    }

    fn add_expr(&mut self) -> PResult<Expr> {
        let mut left = self.mul_expr()?;
        loop {
            let op = match self.peek_kind() {
                TokKind::Plus => BinOp::Add,
                TokKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.mul_expr()?;
            left = Self::binary(op, left, right);
        }
        Ok(left)
    }

    fn mul_expr(&mut self) -> PResult<Expr> {
        let mut left = self.unary_expr()?;
        loop {
            let op = match self.peek_kind() {
                TokKind::Star => BinOp::Mul,
                TokKind::Slash => BinOp::Div,
                TokKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.unary_expr()?;
            left = Self::binary(op, left, right);
        }
        Ok(left)
    }

    fn unary_expr(&mut self) -> PResult<Expr> {
        let op = match self.peek_kind() {
            TokKind::Minus => Some(UnOp::Neg),
            TokKind::Plus => Some(UnOp::Plus),
            TokKind::DotNot | TokKind::Bang => Some(UnOp::Not),
            TokKind::Ident(ref t) if t.eq_ignore_ascii_case("NOT") => Some(UnOp::Not),
            _ => None,
        };
        if let Some(UnOp::Not) = op {
            let t = self.advance();
            let e = self.unary_expr()?;
            let span = t.span.to(e.span);
            return Ok(Expr::new(ExprKind::Unary { op: UnOp::Not, expr: Box::new(e) }, span));
        }
        self.power_expr()
    }

    /// A sign in front of a value belongs to the value, not to what is done with it: `-2 ^ 2`
    /// is 4 in Visual FoxPro, because it squares minus two. Measured against vfp9.exe.
    fn signed_operand(&mut self) -> PResult<Expr> {
        let op = match self.peek_kind() {
            TokKind::Minus => UnOp::Neg,
            TokKind::Plus => UnOp::Plus,
            _ => return self.postfix_expr(),
        };
        let t = self.advance();
        let e = self.signed_operand()?;
        let span = t.span.to(e.span);
        Ok(Expr::new(ExprKind::Unary { op, expr: Box::new(e) }, span))
    }

    /// `^` and `**`, left to right: `2 ^ 3 ^ 2` is 64 in Visual FoxPro, not 512, because it
    /// raises 2 to the 3 and then squares that. Measured against vfp9.exe.
    fn power_expr(&mut self) -> PResult<Expr> {
        let mut base = self.signed_operand()?;
        while self.eat(&TokKind::Caret) {
            let exp = self.signed_operand()?;
            base = Self::binary(BinOp::Pow, base, exp);
        }
        Ok(base)
    }

    /// What a `&` stands for: the name of a memory variable, which may be one element of an
    /// array of them, and an optional `.` saying where the name ends and the line goes on.
    /// `&` is already consumed. `paren_subscript` says whether `(...)` after the name is a
    /// subscript: it is where a value is wanted, and is not after `obj.`, where the parentheses
    /// belong to the member the macro names and hold its arguments.
    fn macro_operand(&mut self, paren_subscript: bool) -> PResult<Expr> {
        let name = self.expect_ident("macro name")?;
        let mut e = Expr::new(ExprKind::Var(name.clone()), name.span);
        // `&laObjs[m.i]` takes the text out of one element; VFP subscripts an array with
        // parentheses as readily as with brackets
        let subs = if self.eat(&TokKind::LBracket) {
            Some(self.expr_list_until(&TokKind::RBracket, "']'")?)
        } else if paren_subscript && self.eat(&TokKind::LParen) {
            Some(self.expr_list_until(&TokKind::RParen, "')'")?)
        } else {
            None
        };
        if let Some(args) = subs {
            let span = name.span.to(self.prev_span());
            e = Expr::new(ExprKind::Index { base: Box::new(e), args }, span);
        }
        // a dot glued to the end closes the macro, so `loControl.&laObjs[m.i]..Class` reads
        // the member the array names and then that one's Class
        if self.is(&TokKind::Dot) && self.peek().span.start == self.prev_span().end {
            self.advance();
        }
        Ok(e)
    }

    /// `obj.&cName` and `obj.&cName(args)`: the member of `obj` a variable names. The `.` is
    /// consumed and the `&` is next.
    fn member_by_macro(&mut self, obj: Box<Expr>, start: Span) -> PResult<Expr> {
        self.advance();
        let by = self.macro_operand(false)?;
        let args = if self.eat(&TokKind::LParen) { Some(self.call_args()?) } else { None };
        let span = start.to(self.prev_span());
        Ok(Expr::new(ExprKind::MemberByName { obj, name: Box::new(by), args }, span))
    }

    fn postfix_expr(&mut self) -> PResult<Expr> {
        let mut e = self.primary()?;
        loop {
            if self.is(&TokKind::Dot) {
                self.advance();
                // `obj.&cName`: the member is whatever that variable says it is
                if self.is(&TokKind::Amp) {
                    let start = e.span;
                    e = self.member_by_macro(Box::new(e), start)?;
                    continue;
                }
                let name = self.expect_ident("member name")?;
                if self.eat(&TokKind::LParen) {
                    let args = self.call_args()?;
                    let span = e.span.to(self.prev_span());
                    e = Expr::new(ExprKind::MethodCall { obj: Box::new(e), name, args }, span);
                } else {
                    let span = e.span.to(name.span);
                    e = Expr::new(ExprKind::Member { obj: Box::new(e), name }, span);
                }
            } else if self.is(&TokKind::Colon) && matches!(self.peek_at(1).kind, TokKind::Colon) {
                // `_MOVER.cmdAdd::Click(1)` runs the parent class's method, which DODEFAULT does
                self.advance();
                self.advance();
                let method = self.expect_ident("method name")?;
                let args = if self.eat(&TokKind::LParen) { self.call_args()? } else { Vec::new() };
                let span = e.span.to(self.prev_span());
                e = Expr::new(ExprKind::Call { name: Name::new("DODEFAULT", method.span), args }, span);
            } else if self.eat(&TokKind::LBracket) {
                let args = self.expr_list_until(&TokKind::RBracket, "']'")?;
                let span = e.span.to(self.prev_span());
                e = Expr::new(ExprKind::Index { base: Box::new(e), args }, span);
            } else if is_callable_value(&e.kind) && self.is(&TokKind::LParen) {
                // `laRoutes[1, 2](req, res)`: a lambda taken out of an array and called where it
                // stands. Only after something that already ended in a bracket, which is where
                // Visual FoxPro answers a `(` with error 36 rather than with a meaning of its
                // own - measured - so no program that runs there is read differently here.
                self.advance();
                let args = self.call_args()?;
                let span = e.span.to(self.prev_span());
                e = Expr::new(ExprKind::CallValue { target: Box::new(e), args }, span);
            } else if is_indexable(&e.kind) && self.peek().bracketed {
                // `aFontInfo [ 4 ]`: the space made the lexer read a string, but a `[` after
                // something that can be indexed is a subscript, which only the parser knows.
                let tok = self.advance();
                let args = self.subscripts_in_brackets(&tok)?;
                let span = e.span.to(tok.span);
                e = Expr::new(ExprKind::Index { base: Box::new(e), args }, span);
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// Re-reads a `[...]` the lexer had to guess about, as the subscripts it really is.
    ///
    /// The text is parsed against a copy of the source with everything before it blanked out, so
    /// the spans and line numbers in the result still point at the real file.
    fn subscripts_in_brackets(&mut self, tok: &Token) -> PResult<Vec<Expr>> {
        let inner = Span::new(tok.span.start + 1, tok.span.end - 1);
        let mut padded: String =
            self.src[..inner.start].iter().map(|c| if *c == '\n' { '\n' } else { ' ' }).collect();
        padded.extend(self.src[inner.start..inner.end].iter());

        let mut sub = Parser::new(&padded, self.mode);
        sub.pos = sub.toks.iter().position(|t| t.span.start >= inner.start).unwrap_or(0);
        let args = sub.expr_list();
        let complete = matches!(sub.peek().kind, TokKind::Eof | TokKind::Newline);
        match args {
            Ok(args) if complete && !has_errors(&sub.diags) => {
                self.diags.append(&mut sub.diags);
                Ok(args)
            }
            _ => Err(self.error(tok.span, "Syntax error: expected array subscripts")),
        }
    }

    /// Whether the `LAMBDA` the parser is looking at opens a lambda rather than reading a
    /// variable of that name.
    ///
    /// It does when what follows is `(`, a possibly empty list of plain names, `)`, and then
    /// the end of the line. An array reference or a call is followed by an operator, a comma or
    /// a bracket, and its subscripts are expressions rather than names - `LAMBDA(1)` at the end
    /// of a line is still element 1 of an array called LAMBDA. Neither `LAMBDA` nor `END` may
    /// become reserved: both are legal variable and field names in Visual FoxPro, measured.
    fn opens_lambda(&mut self) -> bool {
        if !matches!(self.peek_at(1).kind, TokKind::LParen) {
            return false;
        }
        let mut i = 2;
        if matches!(self.peek_at(i).kind, TokKind::RParen) {
            return self.peek_at(i + 1).is_newline();
        }
        loop {
            if !matches!(self.peek_at(i).kind, TokKind::Ident(_)) {
                return false;
            }
            i += 1;
            if matches!(self.peek_at(i).kind, TokKind::Comma) {
                i += 1;
                continue;
            }
            if matches!(self.peek_at(i).kind, TokKind::RParen) {
                return self.peek_at(i + 1).is_newline();
            }
            return false;
        }
    }

    /// `LAMBDA(a, b)` newline, statements, `ENDLAMBDA`. The body is a list of newline-terminated
    /// statements inside an argument list, so the block reader takes over until its own
    /// terminator; a body that never closes is reported at the `LAMBDA` rather than swallowing
    /// the rest of the file.
    fn lambda_expr(&mut self) -> PResult<Expr> {
        let open = self.advance();
        let line = open.line;
        self.advance();
        let mut params: Vec<Param> = Vec::new();
        if !self.eat(&TokKind::RParen) {
            loop {
                let name = self.expect_ident("parameter name")?;
                params.push(Param { name });
                if self.eat(&TokKind::Comma) {
                    continue;
                }
                self.expect(&TokKind::RParen, "')'")?;
                break;
            }
        }
        let (body, term) = self.block_until(&["ENDLAMBDA"], &open, "LAMBDA");
        if term.is_some() {
            self.advance();
        }
        // The parameters are written in `LAMBDA(...)`, so a second declaration has nowhere to
        // mean anything.
        for stmt in &body.stmts {
            if matches!(stmt.kind, StmtKind::LParameters(_) | StmtKind::Parameters(_)) {
                self.error(stmt.span, "LPARAMETERS is not valid inside a lambda");
            }
        }
        let span = open.span.to(self.prev_span());
        Ok(Expr::new(ExprKind::Lambda { params, body, line }, span))
    }

    fn primary(&mut self) -> PResult<Expr> {
        let tok = self.peek().clone();
        let kind = match tok.kind.clone() {
            TokKind::Num(n, chars, decimals) => ExprKind::Num(n, chars, decimals),
            // `WHERE clogon = ?lcUser` inside a SELECT-SQL: the mark is how a parameter is
            // written for SQL pass-through, and a local query takes it too. Measured in Visual
            // FoxPro 9: what follows it is an ordinary expression, and a name that is both a
            // field of a source and a variable is the field, as it is without the mark -
            // `?m.name` is how the variable is asked for. INSERT, UPDATE and DELETE - SQL do
            // not take it: each is a syntax error in the product.
            TokKind::Question if self.in_query > 0 => {
                self.advance();
                let e = self.postfix_expr()?;
                let span = tok.span.to(e.span);
                return Ok(Expr::new(e.kind, span));
            }
            // `$` in front of a number is money written down; between two values it is the
            // substring operator, and that is the other place this token turns up
            TokKind::Dollar if matches!(self.peek_at(1).kind, TokKind::Num(..) | TokKind::Minus) => {
                self.advance();
                let negative = self.eat(&TokKind::Minus);
                let TokKind::Num(n, ..) = self.peek().kind.clone() else {
                    return Err(self.error_here("Syntax error: expected an amount after '$'"));
                };
                self.advance();
                let amount = crate::value::to_currency(if negative { -n } else { n });
                let crate::value::Value::Currency(c) = amount else { unreachable!("to_currency") };
                return Ok(Expr::new(ExprKind::Money(c), tok.span));
            }
            TokKind::Str(s) => ExprKind::Str(s),
            TokKind::True => ExprKind::Bool(true),
            TokKind::False => ExprKind::Bool(false),
            TokKind::Null => ExprKind::Null,
            TokKind::Date(d) => ExprKind::Date(d),
            TokKind::DateTime(d) => ExprKind::DateTime(d),
            TokKind::LParen if kw(self.peek_at(1), "SELECT") => {
                // `(SELECT ...)` on the right of IN, and the argument of ANY/SOME/EXISTS
                self.advance();
                let at = self.peek().span;
                let q = self.query(at)?;
                let close = self.expect(&TokKind::RParen, "')'")?;
                return Ok(Expr::new(ExprKind::AnySubquery(Box::new(q)), tok.span.to(close.span)));
            }
            TokKind::LParen => {
                self.advance();
                let e = self.expr()?;
                let close = self.expect(&TokKind::RParen, "')'")?;
                return Ok(Expr::new(e.kind, tok.span.to(close.span)));
            }
            // ANY, SOME and EXISTS look like calls, but their argument is a query rather than an
            // expression, so they are read here before the call grammar gets to them
            TokKind::Ident(text)
                if matches!(text.to_ascii_uppercase().as_str(), "ANY" | "SOME" | "EXISTS")
                    && matches!(self.peek_at(1).kind, TokKind::LParen)
                    && kw(self.peek_at(2), "SELECT") =>
            {
                let exists = text.eq_ignore_ascii_case("exists");
                self.advance();
                self.advance();
                let at = self.peek().span;
                let q = self.query(at)?;
                let close = self.expect(&TokKind::RParen, "')'")?;
                let span = tok.span.to(close.span);
                let kind =
                    if exists { ExprKind::ExistsSubquery(Box::new(q)) } else { ExprKind::AnySubquery(Box::new(q)) };
                return Ok(Expr::new(kind, span));
            }
            // `SomeClass::SomeMethod(args)` calls that class's own version of a method the object
            // has overridden. The class named is the parent in every use of it worth having, and
            // the parent's version is what DODEFAULT() calls, so that is what this becomes.
            TokKind::Ident(_)
                if matches!(self.peek_at(1).kind, TokKind::Colon) && matches!(self.peek_at(2).kind, TokKind::Colon) =>
            {
                let start = self.advance().span;
                self.advance();
                self.advance();
                let name = self.expect_ident("method name")?;
                let args = if self.eat(&TokKind::LParen) { self.call_args()? } else { Vec::new() };
                let span = start.to(self.prev_span());
                return Ok(Expr::new(ExprKind::Call { name: Name::new("DODEFAULT", name.span), args }, span));
            }
            // `LAMBDA(a, b)` at the end of a line opens a function value. Everywhere else the
            // word is an ordinary name, because it is one in the product.
            TokKind::Ident(ref text) if text.eq_ignore_ascii_case("LAMBDA") && self.opens_lambda() => {
                return self.lambda_expr();
            }
            TokKind::Ident(text) => {
                self.advance();
                let upper = text.to_ascii_uppercase();
                let special = match upper.as_str() {
                    // the Foundation Classes write the null value as a bare NULL as well as .NULL.
                    "NULL" => Some(ExprKind::Null),
                    "THIS" => Some(ExprKind::This),
                    "THISFORM" => Some(ExprKind::ThisForm),
                    "THISFORMSET" => Some(ExprKind::ThisFormSet),
                    "_SCREEN" => Some(ExprKind::Screen),
                    "_VFP" => Some(ExprKind::Member {
                        obj: Box::new(Expr::new(ExprKind::Screen, tok.span)),
                        name: Name::new("Application", tok.span),
                    }),
                    _ => None,
                };
                if let Some(k) = special {
                    return Ok(Expr::new(k, tok.span));
                }
                let name = Name::new(text, tok.span);
                // `COUNT(*)` is the one call whose argument is not an expression. Inside a
                // SELECT-SQL it may stand anywhere an expression may - a select list column, a
                // HAVING clause, an argument to something else - so it is read here, as a COUNT
                // of nothing, which the expression grammar can never otherwise produce.
                if self.in_query > 0
                    && name.is("COUNT")
                    && matches!(self.peek_kind(), TokKind::LParen)
                    && matches!(self.peek_at(1).kind, TokKind::Star)
                    && matches!(self.peek_at(2).kind, TokKind::RParen)
                {
                    self.advance();
                    self.advance();
                    self.advance();
                    let span = tok.span.to(self.prev_span());
                    return Ok(Expr::new(ExprKind::Call { name, args: Vec::new() }, span));
                }
                if self.eat(&TokKind::LParen) {
                    let args = self.call_args()?;
                    let span = tok.span.to(self.prev_span());
                    // `CAST(...)` only ever reaches here through the `AS` rewrite in `arg()`,
                    // which leaves it holding one argument that is itself the real two-argument
                    // call. `CAST(a, b)` written directly - without `AS` - parses as an
                    // ordinary call and lands here with something else, which Visual FoxPro
                    // refuses as a syntax error rather than accepting as a plain function.
                    if name.is("CAST") {
                        let mut only = args;
                        let inner = if only.len() == 1 {
                            match only.pop().map(|a| a.expr) {
                                Some(Expr { kind: ExprKind::Call { name: inner, args }, .. })
                                    if inner.is("CAST") && args.len() == 2 =>
                                {
                                    Some(Expr::new(ExprKind::Call { name: inner, args }, span))
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };
                        return inner.ok_or_else(|| self.error(span, "CAST requires 'AS'"));
                    }
                    return Ok(Expr::new(ExprKind::Call { name, args }, span));
                }
                return Ok(Expr::new(ExprKind::Var(name), tok.span));
            }
            TokKind::Amp => {
                self.advance();
                let operand = self.macro_operand(true)?;
                let span = tok.span.to(self.prev_span());
                return Ok(Expr::new(ExprKind::Macro(Box::new(operand)), span));
            }
            TokKind::Dot => {
                // Whether a WITH is open is a question for the moment the line runs, not for the
                // compiler: Visual FoxPro compiles `CursorGetProp("sourcetype", .ALIAS)` in a
                // method with no WITH anywhere in it, with no error file at all. A method that
                // is only ever called from inside one is perfectly good FoxPro, and the shipped
                // Foundation Classes contain such a method.
                self.advance();
                let obj = Box::new(Expr::new(ExprKind::WithRef, tok.span));
                // `.&cMemberCount` inside a WITH: the member is whatever that variable says
                if self.is(&TokKind::Amp) {
                    return self.member_by_macro(obj, tok.span);
                }
                let name = self.expect_ident("member name")?;
                if self.eat(&TokKind::LParen) {
                    let args = self.call_args()?;
                    let span = tok.span.to(self.prev_span());
                    return Ok(Expr::new(ExprKind::MethodCall { obj, name, args }, span));
                }
                let span = tok.span.to(name.span);
                return Ok(Expr::new(ExprKind::Member { obj, name }, span));
            }
            TokKind::At => return Err(self.error(tok.span, "'@' is only valid in an argument list")),
            _ => return Err(self.expected("expression")),
        };
        self.advance();
        Ok(Expr::new(kind, tok.span))
    }

    /// Arguments after `(` up to and including `)`; empty slots become `Omitted`.
    fn call_args(&mut self) -> PResult<Vec<Arg>> {
        let mut args = Vec::new();
        if self.eat(&TokKind::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.arg(&[TokKind::Comma, TokKind::RParen])?);
            if self.eat(&TokKind::Comma) {
                continue;
            }
            self.expect(&TokKind::RParen, "',' or ')'")?;
            return Ok(args);
        }
    }

    /// Arguments of `DO ... WITH` / `DO FORM ... WITH`: up to end of line or the next clause.
    fn arg_list_to_eol(&mut self) -> PResult<Vec<Arg>> {
        let mut args = Vec::new();
        loop {
            args.push(self.arg(&[TokKind::Comma, TokKind::Newline, TokKind::Eof])?);
            if self.eat(&TokKind::Comma) {
                continue;
            }
            return Ok(args);
        }
    }

    fn arg(&mut self, omitted_before: &[TokKind]) -> PResult<Arg> {
        let kind = self.peek_kind();
        if omitted_before.contains(&kind) {
            let span = Span::new(self.peek().span.start, self.peek().span.start);
            return Ok(Arg { expr: Expr::new(ExprKind::Omitted, span), by_ref: false });
        }
        if self.eat(&TokKind::At) {
            let e = self.postfix_expr()?;
            return Ok(Arg { expr: e, by_ref: true });
        }
        let e = self.expr()?;
        // `CAST(x AS I)`: the type follows the value; it becomes a second argument
        if self.is_kw("AS") {
            self.advance();
            let kind = self.expect_ident("type")?;
            if self.eat(&TokKind::LParen) {
                self.expr_list_until(&TokKind::RParen, "')'")?;
            }
            let span = e.span.to(self.prev_span());
            let args = vec![Arg { expr: e, by_ref: false }, Arg { expr: Expr::new(ExprKind::Str(kind.upper.clone()), kind.span), by_ref: false }];
            return Ok(Arg { expr: Expr::new(ExprKind::Call { name: Name::new("CAST", kind.span), args }, span), by_ref: false });
        }
        Ok(Arg { expr: e, by_ref: false })
    }
}

/// The name a table answers to in a query when no AS alias was given: its file stem, which is
/// also what USE would call it.
fn query_alias(table: &str) -> String {
    let file = table.rsplit(['/', '\\']).next().unwrap_or(table);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem).to_string()
}

/// How wide a field of each type is when the declaration does not say. The types with a fixed
/// size never say; the ones that vary always do.
fn default_width(kind: char) -> (u8, u8) {
    match kind {
        'L' => (1, 0),
        'D' => (8, 0),
        'T' | '@' | 'B' | 'O' | 'Y' => (8, 0),
        'I' | '+' => (4, 0),
        'M' | 'G' | 'P' => (4, 0),
        _ => (10, 0),
    }
}

/// Every command verb the parser dispatches on, in order, for `ALANGUAGE(a, 1)`.
///
/// It is the same list the parser reads statements with, so what a program is told the language
/// has is what the language has.
pub fn command_verbs() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = STATEMENT_KEYWORDS
        .iter()
        .chain(SCREEN_ONLY)
        .chain(IDE_ONLY)
        .chain(UNSUPPORTED)
        .chain(IGNORED)
        .copied()
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// `APROCINFO()`: what is in a program file.
///
/// `kind` picks what is listed, as the reference has it: 0 everything, 1 the class definitions,
/// 2 the procedures inside them, 3 the preprocessor directives. Each row is what the Document
/// View would show, the line it is on, and what kind of thing it is.
pub fn outline(source: &str, kind: u8) -> Vec<Vec<crate::value::Value>> {
    use crate::value::Value;
    let mut out = Vec::new();
    let mut class: Option<String> = None;
    for (i, raw) in source.lines().enumerate() {
        let line = raw.trim();
        let at = Value::number(i as f64 + 1.0);
        let mut word = line.split_whitespace();
        let first = word.next().unwrap_or("").to_ascii_uppercase();
        let rest: Vec<&str> = word.collect();
        let name = rest.first().copied().unwrap_or("").trim_end_matches(',').to_string();
        match first.as_str() {
            // kind 0's combined outline names a `#DEFINE` by its symbol and a fixed "Define",
            // where kind 3's defines-only listing keeps the whole line - measured, the two
            // are not the same row shape reused at different widths.
            _ if first.starts_with('#') => {
                if kind == 0 {
                    out.push(vec![Value::str(name), at, Value::str("Define"), Value::number(0.0)]);
                } else if kind == 3 {
                    out.push(vec![Value::str(line), at, Value::str(first.trim_start_matches('#'))]);
                }
            }
            "DEFINE" if rest.first().is_some_and(|w| w.eq_ignore_ascii_case("CLASS")) => {
                let named = rest.get(1).copied().unwrap_or("").to_string();
                let of = rest
                    .iter()
                    .position(|w| w.eq_ignore_ascii_case("AS"))
                    .and_then(|i| rest.get(i + 1))
                    .copied()
                    .unwrap_or("");
                class = Some(named.clone());
                if kind == 0 {
                    // measured: the name column is the whole "Name AS Base" clause, not the
                    // class name alone.
                    out.push(vec![Value::str(format!("{named} AS {of}")), at, Value::str("Class"), Value::number(0.0)]);
                } else if kind == 1 {
                    let public = rest.iter().any(|w| w.eq_ignore_ascii_case("OLEPUBLIC"));
                    out.push(vec![Value::str(named), at, Value::str(of), Value::Logical(public)]);
                }
            }
            "ENDDEFINE" => class = None,
            "PROCEDURE" | "FUNCTION" => {
                if kind == 0 || kind == 2 {
                    let shown = match &class {
                        Some(c) if kind == 0 => format!("{c}.{name}"),
                        _ => name.clone(),
                    };
                    let mut row = vec![Value::str(shown), at];
                    if kind == 0 {
                        row.push(Value::str("Procedure"));
                        row.push(Value::number(0.0));
                    }
                    out.push(row);
                }
            }
            _ => {}
        }
    }
    out
}
