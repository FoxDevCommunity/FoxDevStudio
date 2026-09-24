//! AST -> bytecode. One `FuncBuilder` per function; a whole module shares the constant, name
//! and member pools.
//!
//! Scoping is decided here: a name declared LOCAL / LPARAMETERS anywhere in a function is a
//! slot for the whole function (VFP semantics: declarations are not block scoped), everything
//! else is a dynamically scoped name resolved by the VM at run time.
//!
//! Loops and TRY blocks keep enough context that `EXIT`, `LOOP` and `RETURN` can leave them
//! cleanly: WITH objects are popped, FINALLY blocks are inlined at the exit point, and the two
//! FOR EACH bookkeeping slots are dropped from the stack.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::builtins;
use crate::bytecode::*;
use crate::diagnostics::{Diagnostic, LineMap, Span};
use crate::error::RtError;
use crate::parser::{self, has_errors};
use crate::query::{AggKind, HavingRef, OrderKey, OrderTerm, PlanColumn, PlanInto, QueryPlan};

/// One method of a form: what the designer stores for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodSource {
    /// `""` for the form itself, else the dotted path relative to the form
    /// (`"pgfMain.Page1.lblGreeting"`).
    pub object_path: String,
    pub event: String,
    /// Implicit parameter list, e.g. `"nKeyCode, nShiftAltCtrl"` or `""`.
    pub params: String,
    pub source: String,
    /// The header file whose `#DEFINE`s are in scope for this method, by name without folder or
    /// extension, upper-cased; `""` when the file the method came from named none. Visual FoxPro
    /// stores one per file for a form and one per class for a class library, and a method sees
    /// the header of the file its own text lives in - measured: a method inherited from a `.vcx`
    /// class is compiled with that library's header and never with the form's, and a form's own
    /// method never sees the header of a library it was built from.
    pub include: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompileResult {
    /// `Some` only when there are no error diagnostics.
    pub module: Option<Module>,
    pub diagnostics: Vec<Diagnostic>,
    /// Form modules: diagnostics per method (`"cmdSayHi.Click"`, `"Init"` for the form). These
    /// are also counted in the decision to produce a module.
    pub method_diagnostics: Vec<(String, Diagnostic)>,
}

impl CompileResult {
    fn failed(diagnostics: Vec<Diagnostic>) -> Self {
        CompileResult { module: None, diagnostics, method_diagnostics: Vec::new() }
    }
}

/// Compiles a .prg: `funcs[0]` is `MAIN`, the procedures follow in source order.
pub fn compile_program(src: &str, name: &str) -> CompileResult {
    compile_program_with(src, name, &std::collections::HashMap::new())
}

/// The same, with the header files the program may `#INCLUDE`.
pub fn compile_program_with(
    src: &str,
    name: &str,
    headers: &std::collections::HashMap<String, String>,
) -> CompileResult {
    let out = parser::parse_program_with(src, headers);
    if has_errors(&out.diagnostics) {
        return CompileResult::failed(out.diagnostics);
    }
    let mut mc = ModuleCompiler::new(name, ModuleKind::Program, src);
    mc.diags = out.diagnostics;
    mc.declared = declared_names(&out.program);
    let main_params = leading_params(&out.program.body);
    mc.compile_function("MAIN", name, &[], &out.program.body, &main_params, &[]);
    let mut seen: HashMap<String, Span> = HashMap::new();
    for proc in &out.program.procs {
        if seen.insert(proc.name.upper.clone(), proc.span).is_some() {
            mc.error(proc.name.span, format!("Procedure '{}' is already defined", proc.name.text));
            continue;
        }
        let inline: Vec<Name> = proc.params.iter().map(|p| p.name.clone()).collect();
        let body_params = leading_params(&proc.body);
        mc.compile_function(&proc.name.upper, &proc.name.text, &inline, &proc.body, &body_params, &[]);
    }
    let mut seen_classes: HashMap<String, Span> = HashMap::new();
    for class in &out.program.classes {
        if seen_classes.insert(class.name.upper.clone(), class.span).is_some() {
            mc.error(class.name.span, format!("Class '{}' is already defined", class.name.text));
            continue;
        }
        mc.compile_class(class);
    }
    mc.finish()
}

/// Compiles the methods of a form; each method with non-blank source becomes a function named
/// `OBJPATH.EVENT` listed in `Module::methods`.
pub fn compile_form(name: &str, methods: &[MethodSource]) -> CompileResult {
    compile_form_with(name, methods, &std::collections::HashMap::new())
}

/// The same, with the header files the methods name: the one each method's own file declared
/// (`MethodSource::include`) and any a header `#INCLUDE`s in turn, keyed by name without folder
/// or extension. Reading them is the caller's, as it is for a program.
pub fn compile_form_with(
    name: &str,
    methods: &[MethodSource],
    headers: &std::collections::HashMap<String, String>,
) -> CompileResult {
    let mut mc = ModuleCompiler::new(name, ModuleKind::Form, "");
    let mut method_diags: Vec<(String, Diagnostic)> = Vec::new();
    for m in methods {
        if m.source.trim().is_empty() {
            continue;
        }
        let label = if m.object_path.is_empty() { m.event.clone() } else { format!("{}.{}", m.object_path, m.event) };
        let display = if m.object_path.is_empty() { format!("{name}.{}", m.event) } else { label.clone() };
        let key = format!("{}.{}", m.object_path.to_ascii_uppercase(), m.event.to_ascii_uppercase());
        let out = parser::parse_method_with(&m.source, headers, &m.include);
        let failed = has_errors(&out.diagnostics);
        for d in out.diagnostics {
            method_diags.push((label.clone(), d));
        }
        if failed {
            continue;
        }
        mc.src = m.source.clone();
        mc.lines = LineMap::new(&m.source);
        mc.diags.clear();
        let implicit: Vec<Name> = m
            .params
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| Name::new(p, Span::default()))
            .collect();
        let body_params = leading_params(&out.program.body);
        // An explicit LPARAMETERS/PARAMETERS in the body replaces the designer's template.
        let (inline, body): (Vec<Name>, ParamDecl) =
            if body_params.names.is_empty() { (implicit, body_params) } else { (Vec::new(), body_params) };
        let idx = mc.module.funcs.len() as u32;
        mc.compile_function(&key, &display, &inline, &out.program.body, &body, &[]);
        mc.module.methods.push((key, idx));
        for d in mc.diags.drain(..) {
            method_diags.push((label.clone(), d));
        }
    }
    let mut result = mc.finish();
    if method_diags.iter().any(|(_, d)| d.is_error()) {
        result.module = None;
    }
    result.method_diagnostics = method_diags;
    result
}

/// Compiles statements (menu commands, EXECSCRIPT, command window lines) into `funcs[0]`.
///
/// Parsed as a program rather than a method body, so a snippet may declare procedures and
/// classes: the Command Window takes several lines at a time, and `EXECSCRIPT()` runs whole
/// programs. `compile_snippet_in` is the method-shaped variant, for text that runs inside a
/// caller's frame.
pub fn compile_snippet(src: &str, name: &str) -> CompileResult {
    let out = parser::parse_program(src);
    if has_errors(&out.diagnostics) {
        return CompileResult::failed(out.diagnostics);
    }
    let mut mc = ModuleCompiler::new(name, ModuleKind::Snippet, src);
    mc.diags = out.diagnostics;
    let body_params = leading_params(&out.program.body);
    mc.compile_function("MAIN", name, &[], &out.program.body, &body_params, &[]);

    let mut seen: HashMap<String, Span> = HashMap::new();
    for proc in &out.program.procs {
        if seen.insert(proc.name.upper.clone(), proc.span).is_some() {
            mc.error(proc.name.span, format!("Procedure '{}' is already defined", proc.name.text));
            continue;
        }
        let inline: Vec<Name> = proc.params.iter().map(|p| p.name.clone()).collect();
        let proc_params = leading_params(&proc.body);
        mc.compile_function(&proc.name.upper, &proc.name.text, &inline, &proc.body, &proc_params, &[]);
    }

    let mut seen_classes: HashMap<String, Span> = HashMap::new();
    for class in &out.program.classes {
        if seen_classes.insert(class.name.upper.clone(), class.span).is_some() {
            mc.error(class.name.span, format!("Class '{}' is already defined", class.name.text));
            continue;
        }
        mc.compile_class(class);
    }
    mc.finish()
}

/// `compile_snippet` for text that will run *inside* an existing frame (`&cmd` lines): `locals`
/// are that frame's slot names so references to them compile to slots.
pub fn compile_snippet_in(src: &str, name: &str, locals: &[String]) -> CompileResult {
    let out = parser::parse_method(src);
    if has_errors(&out.diagnostics) {
        return CompileResult::failed(out.diagnostics);
    }
    let mut mc = ModuleCompiler::new(name, ModuleKind::Snippet, src);
    mc.diags = out.diagnostics;
    let body_params = leading_params(&out.program.body);
    mc.compile_function("MAIN", name, &[], &out.program.body, &body_params, locals);
    mc.finish()
}

/// The same, for a line that has already been through macro substitution.
///
/// Substitution happens once, before the line is read, so a `&name` still standing in the text
/// is text: the name was not a character variable, and the product leaves it alone rather than
/// looking at it again. Reading it as a macro a second time would never end.
pub fn compile_expanded_line(src: &str, name: &str, locals: &[String]) -> CompileResult {
    let out = parser::parse_expanded_line(src);
    if has_errors(&out.diagnostics) {
        return CompileResult::failed(out.diagnostics);
    }
    let mut mc = ModuleCompiler::new(name, ModuleKind::Snippet, src);
    mc.diags = out.diagnostics;
    let body_params = leading_params(&out.program.body);
    mc.compile_function("MAIN", name, &[], &out.program.body, &body_params, locals);
    mc.finish()
}

/// Compiles one name into a function that writes the value already on the stack to what it
/// names: a variable, a property, an element of an array, however far down it goes.
///
/// `STORE x TO (cName)` needs this. The name is only there once the expression has been worked
/// out, so the assignment the compiler would have made had the name been written out is made
/// then instead, and it is the same one - which is why nothing here knows what a name
/// expression is. `locals` are the slot names of the frame it will run in.
pub fn compile_store_to(src: &str, locals: &[String]) -> CompileResult {
    // the name may name a member of the object a WITH is open on: `STORE 1 TO ('.' + cName)`
    let target = match parser::parse_expression_in(src) {
        Ok(e) => e,
        Err(diags) => return CompileResult::failed(diags),
    };
    let mut mc = ModuleCompiler::new("name", ModuleKind::Snippet, src);
    let mut fb = FuncBuilder::new("MAIN", "name");
    fb.locals = locals.to_vec();
    mc.store_target(&mut fb, &target);
    fb.emit(Instr::True);
    // running out of statements ends the store, and nothing else: `Return` here would mean the
    // program wrote RETURN, which leaves the routine that reached the line
    fb.emit(Instr::EndOfCode);
    mc.module.funcs.push(fb.finish());
    mc.finish()
}

/// Compiles one expression into a function that returns its value. `locals` are the upper-cased
/// slot names of the frame the expression will run in.
pub fn compile_expression(src: &str, locals: &[String]) -> CompileResult {
    let expr = match parser::parse_expression(src) {
        Ok(e) => e,
        Err(diags) => return CompileResult::failed(diags),
    };
    let mut mc = ModuleCompiler::new("expression", ModuleKind::Expression, src);
    let mut fb = FuncBuilder::new("MAIN", "expression");
    fb.locals = locals.to_vec();
    mc.expr(&mut fb, &expr);
    fb.emit(Instr::Return);
    mc.module.funcs.push(fb.finish());
    mc.finish()
}

// ---------------------------------------------------------------------------------------------

/// The leading `LPARAMETERS` / `PARAMETERS` statement of a body (directives may precede it).
#[derive(Default)]
struct ParamDecl {
    names: Vec<Name>,
    private: bool,
    /// Index of the statement in the body, so it is not compiled twice.
    stmt_index: Option<usize>,
}

fn leading_params(body: &Block) -> ParamDecl {
    for (i, s) in body.stmts.iter().enumerate() {
        match &s.kind {
            StmtKind::Directive(_) => continue,
            StmtKind::LParameters(p) => {
                return ParamDecl {
                    names: p.iter().map(|p| p.name.clone()).collect(),
                    private: false,
                    stmt_index: Some(i),
                };
            }
            StmtKind::Parameters(p) => {
                return ParamDecl {
                    names: p.iter().map(|p| p.name.clone()).collect(),
                    private: true,
                    stmt_index: Some(i),
                };
            }
            _ => break,
        }
    }
    ParamDecl::default()
}

/// The functions the compiler emits code for rather than calling the library: what they do
/// with their arguments is not what a call does with them. IIF only works out the branch it
/// takes, so both branches cannot be evaluated first.
const COMPILED_FUNCTIONS: &[&str] = &["IIF"];

/// Where a row copied out of a cursor goes: a table named outright, or one whose name a
/// temporary holds because the program worked it out.
#[derive(Clone, Copy)]
enum RowTarget {
    Named(u32),
    Worked(u32),
}

struct LoopCtx {
    breaks: Vec<usize>,
    continues: Vec<usize>,
    with_depth: usize,
    try_depth: usize,
    /// Values the loop keeps on the stack (2 for FOR EACH).
    stack_extra: u8,
}

struct FuncBuilder {
    name: String,
    display_name: String,
    nparams: u32,
    locals: Vec<String>,
    code: Vec<Instr>,
    loops: Vec<LoopCtx>,
    /// FINALLY bodies of the enclosing TRY blocks (innermost last), for inlining at exits.
    tries: Vec<Option<Block>>,
    with_depth: usize,
    temp_counter: u32,
    /// The FOR and WHILE of the last LOCATE compiled here, for a CONTINUE to carry on with.
    last_locate: Option<(Option<Expr>, Option<Expr>)>,
    /// A lambda's captured LOCALs of the enclosing routine; empty for everything else.
    captures: Vec<Capture>,
}

impl FuncBuilder {
    fn new(name: &str, display_name: &str) -> Self {
        FuncBuilder {
            name: name.to_string(),
            display_name: display_name.to_string(),
            nparams: 0,
            locals: Vec::new(),
            code: Vec::new(),
            loops: Vec::new(),
            tries: Vec::new(),
            with_depth: 0,
            temp_counter: 0,
            last_locate: None,
            captures: Vec::new(),
        }
    }

    fn emit(&mut self, i: Instr) -> usize {
        self.code.push(i);
        self.code.len() - 1
    }

    fn pc(&self) -> u32 {
        self.code.len() as u32
    }

    fn patch(&mut self, at: usize, target: u32) {
        match &mut self.code[at] {
            Instr::Jump(t)
            | Instr::JumpIfFalse(t)
            | Instr::JumpIfTrue(t)
            | Instr::JumpIfFalseKeep(t)
            | Instr::JumpIfTrueKeep(t)
            | Instr::ForTest(t)
            | Instr::ForEachNext(t) => *t = target,
            _ => unreachable!("patching a non-jump"),
        }
    }

    fn patch_here(&mut self, at: usize) {
        let here = self.pc();
        self.patch(at, here);
    }

    fn local(&self, upper: &str) -> Option<u32> {
        self.locals.iter().position(|l| l == upper).map(|i| i as u32)
    }

    fn add_local(&mut self, upper: &str) -> u32 {
        if let Some(i) = self.local(upper) {
            return i;
        }
        self.locals.push(upper.to_string());
        (self.locals.len() - 1) as u32
    }

    /// A hidden slot (name not spellable in source).
    fn temp(&mut self, what: &str) -> u32 {
        self.temp_counter += 1;
        let name = format!("#{what}{}", self.temp_counter);
        self.add_local(&name)
    }

    fn finish(self) -> FuncProto {
        // `LINENO(1)` counts from here: the first statement this function actually runs, which
        // for a program's own body is its first line and for anything `DO` calls into is the
        // first line after the `PROCEDURE`/`FUNCTION` that opened it - measured, not the
        // `PROCEDURE` line itself, which is one earlier than what Visual FoxPro counts from.
        let def_line = self
            .code
            .iter()
            .find_map(|i| if let Instr::Stmt(line) = i { Some(*line) } else { None })
            .unwrap_or(1);
        FuncProto {
            name: self.name,
            display_name: self.display_name,
            nparams: self.nparams,
            locals: self.locals,
            def_line,
            captures: self.captures,
            code: self.code,
        }
    }
}

struct ModuleCompiler {
    module: Module,
    diags: Vec<Diagnostic>,
    lines: LineMap,
    src: String,
    const_index: HashMap<String, u32>,
    name_index: HashMap<String, u32>,
    member_index: HashMap<String, u32>,
    /// The cursor each subquery was run into, by the span of the expression that asks about it.
    sub_alias: HashMap<Span, String>,
    sub_counter: u32,
    /// While a HAVING predicate is being emitted, the pieces of it that are read out of the
    /// folded row rather than out of a record, and which entry of the plan's list each is.
    having_subst: Vec<(Expr, u16)>,
    /// Procedures this module defines. A name it defines is its own, whatever a built-in of a
    /// longer name would be abbreviated to.
    declared: HashSet<String>,
}

impl ModuleCompiler {
    fn new(name: &str, kind: ModuleKind, src: &str) -> Self {
        ModuleCompiler {
            module: Module {
                name: name.to_string(),
                kind,
                consts: Vec::new(),
                names: Vec::new(),
                members: Vec::new(),
                funcs: Vec::new(),
                methods: Vec::new(),
                classes: Vec::new(),
                queries: Vec::new(),
                cursors: Vec::new(),
                dlls: Vec::new(),
            },
            diags: Vec::new(),
            lines: LineMap::new(src),
            src: src.to_string(),
            const_index: HashMap::new(),
            name_index: HashMap::new(),
            member_index: HashMap::new(),
            sub_alias: HashMap::new(),
            sub_counter: 0,
            having_subst: Vec::new(),
            declared: HashSet::new(),
        }
    }

    fn finish(self) -> CompileResult {
        let ok = !has_errors(&self.diags);
        CompileResult {
            module: if ok { Some(self.module) } else { None },
            diagnostics: self.diags,
            method_diagnostics: Vec::new(),
        }
    }

    fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(self.lines.error(span, msg));
    }

    fn warning(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(self.lines.warning(span, msg));
    }

    // ----- pools -----

    fn constant(&mut self, c: Constant) -> u32 {
        let key = match &c {
            Constant::Num(n, w, d) => format!("N{}:{w}:{d}", n.to_bits()),
            Constant::Money(c) => format!("Y{c}"),
            Constant::Str(s) => format!("S{s}"),
            Constant::Date(d) => format!("D{d:?}"),
            Constant::DateTime(d) => format!("T{d:?}"),
            Constant::Bool(b) => format!("B{b}"),
            Constant::Null => "X".to_string(),
            // only class property values are arrays, and those never reach the constant pool
            Constant::Array(items) => format!("A{}", items.len()),
        };
        if let Some(&i) = self.const_index.get(&key) {
            return i;
        }
        self.module.consts.push(c);
        let i = (self.module.consts.len() - 1) as u32;
        self.const_index.insert(key, i);
        i
    }

    fn str_const(&mut self, s: &str) -> u32 {
        self.constant(Constant::Str(s.to_string()))
    }

    fn name(&mut self, upper: &str) -> u32 {
        if let Some(&i) = self.name_index.get(upper) {
            return i;
        }
        self.module.names.push(upper.to_string());
        let i = (self.module.names.len() - 1) as u32;
        self.name_index.insert(upper.to_string(), i);
        i
    }

    fn member(&mut self, name: &Name) -> u32 {
        if let Some(&i) = self.member_index.get(&name.upper) {
            return i;
        }
        self.module.members.push(name.text.clone());
        let i = (self.module.members.len() - 1) as u32;
        self.member_index.insert(name.upper.clone(), i);
        i
    }

    // ----- functions -----

    /// `inline` are `PROCEDURE f(a, b)` parameters (always locals); `body_params` the leading
    /// PARAMETERS/LPARAMETERS statement; `preset_locals` the caller frame's slots for inline code.
    fn compile_function(
        &mut self,
        name: &str,
        display: &str,
        inline: &[Name],
        body: &Block,
        body_params: &ParamDecl,
        preset_locals: &[String],
    ) -> u32 {
        // The slot is taken before the body is compiled, because a lambda in it compiles to a
        // function of its own and would otherwise land in front of the routine that holds it -
        // and `funcs[0]` of a program has to stay its main body.
        let index = self.reserve_func();
        let mut fb = FuncBuilder::new(name, display);
        fb.locals = preset_locals.to_vec();
        let mut private_params: Vec<(u32, Name)> = Vec::new();
        if preset_locals.is_empty() {
            for p in inline {
                fb.add_local(&p.upper);
            }
            for p in &body_params.names {
                if body_params.private {
                    let slot = fb.add_local(&format!("#PARAM_{}", p.upper));
                    private_params.push((slot, p.clone()));
                } else {
                    fb.add_local(&p.upper);
                }
            }
            fb.nparams = fb.locals.len() as u32;
        }
        collect_locals(body, &mut fb);
        for (arg, p) in &private_params {
            let n = self.name(&p.upper);
            fb.emit(Instr::ParamToPrivate { arg: *arg, name: n });
        }
        for (i, s) in body.stmts.iter().enumerate() {
            if body_params.stmt_index == Some(i) {
                continue;
            }
            self.stmt(&mut fb, s);
        }
        fb.emit(Instr::True);
        fb.emit(Instr::EndOfCode);
        self.module.funcs[index as usize] = fb.finish();
        index
    }

    /// Takes the next function index and fills it with a placeholder until the real one is
    /// finished.
    fn reserve_func(&mut self) -> u32 {
        let index = self.module.funcs.len() as u32;
        self.module.funcs.push(FuncProto {
            name: String::new(),
            display_name: String::new(),
            nparams: 0,
            locals: Vec::new(),
            def_line: 1,
            captures: Vec::new(),
            code: vec![Instr::True, Instr::EndOfCode],
        });
        index
    }

    /// `LAMBDA(a, b) ... ENDLAMBDA`: a function of its own in the same module, and the
    /// instruction that builds a value out of it.
    ///
    /// Its frame is laid out parameters first, then the LOCALs the body declares, then one slot
    /// for every LOCAL of the routine the lambda is written in. Those last are the captures: the
    /// value is copied in when the lambda is made, because the frame it lived in is gone by the
    /// time the lambda runs and FoxPro has no reference to a local to keep instead. Every one of
    /// them is captured rather than only the ones the body mentions, which costs a copy and is
    /// what makes the chain work at any depth: a lambda inside a lambda captures from the middle
    /// one's frame, and the middle one has the outer routine's locals in it to be captured from.
    fn lambda(&mut self, outer: &mut FuncBuilder, params: &[Param], body: &Block, line: u32) -> u32 {
        let index = self.reserve_func();
        let display = format!("{} LAMBDA at line {line}", outer.display_name);
        let mut fb = FuncBuilder::new(&format!("#LAMBDA{index}"), &display);
        for p in params {
            fb.add_local(&p.name.upper);
        }
        fb.nparams = fb.locals.len() as u32;
        collect_locals(body, &mut fb);
        for (from, name) in outer.locals.clone().iter().enumerate() {
            // a hidden slot has no name a program can write, so nothing in the body can read it
            if name.starts_with('#') || fb.local(name).is_some() {
                continue;
            }
            let to = fb.add_local(name);
            fb.captures.push(Capture { from: from as u32, to });
        }
        for s in &body.stmts {
            self.stmt(&mut fb, s);
        }
        fb.emit(Instr::True);
        fb.emit(Instr::EndOfCode);
        self.module.funcs[index as usize] = fb.finish();
        index
    }

    // ----- classes -----

    /// A `DEFINE CLASS`: property values are folded to constants and every method becomes a
    /// function named `"CLASSNAME.METHODNAME"` (`"CLASSNAME.MEMBER.EVENT"` for a member's), also
    /// listed in `Module::methods` so the host can start it by name.
    fn compile_class(&mut self, class: &ClassDecl) {
        let properties = self.class_properties(&class.properties, &class.name.text);
        let mut members = Vec::with_capacity(class.members.len());
        for m in &class.members {
            let owner = format!("{}.{}", class.name.text, m.name.text);
            let properties = self.class_properties(&m.properties, &owner);
            members.push(MemberProto {
                name: m.name.text.clone(),
                class: m.class.text.clone(),
                noinit: m.noinit,
                properties,
            });
        }
        let mut methods: Vec<(String, u32)> = Vec::new();
        let mut seen: HashMap<String, Span> = HashMap::new();
        for proc in &class.procs {
            if seen.insert(proc.name.upper.clone(), proc.span).is_some() {
                let msg = format!("Method '{}' is already defined in class '{}'", proc.name.text, class.name.text);
                self.error(proc.name.span, msg);
                continue;
            }
            let key = format!("{}.{}", class.name.upper, proc.name.upper);
            let display = format!("{}.{}", class.name.text, proc.name.text);
            let inline: Vec<Name> = proc.params.iter().map(|p| p.name.clone()).collect();
            let body_params = leading_params(&proc.body);
            let idx = self.module.funcs.len() as u32;
            self.compile_function(&key, &display, &inline, &proc.body, &body_params, &[]);
            self.module.methods.push((key, idx));
            methods.push((proc.name.text.clone(), idx));
        }
        self.module.classes.push(ClassProto {
            name: class.name.text.clone(),
            parent: class.parent.text.clone(),
            properties,
            members,
            methods,
        });
    }

    /// Folds every property value to a constant; a value VFP would not accept there is an error.
    fn class_properties(&mut self, props: &[ClassProperty], owner: &str) -> Vec<(String, Constant)> {
        let mut out = Vec::with_capacity(props.len());
        for p in props {
            // `DIMENSION aRGB[3]`: the property starts as an array of .F., as VFP creates it
            if let Some(dim) = &p.dim {
                match fold_constant(dim) {
                    Some(Constant::Num(n, ..)) if n >= 1.0 && n <= 65_000.0 => {
                        out.push((p.name.text.clone(), Constant::Array(vec![Constant::Bool(false); n as usize])));
                    }
                    _ => {
                        let msg = format!("Array '{}' of '{}' must be dimensioned with a constant size", p.name.text, owner);
                        self.error(dim.span, msg);
                    }
                }
                continue;
            }
            match fold_constant(&p.value) {
                Some(c) => out.push((p.name.text.clone(), c)),
                None => {
                    let msg = format!("Property '{}' of '{}' must be set to a constant value", p.name.text, owner);
                    self.error(p.value.span, msg);
                }
            }
        }
        out
    }

    // ----- statements -----

    fn block(&mut self, fb: &mut FuncBuilder, b: &Block) {
        for s in &b.stmts {
            self.stmt(fb, s);
        }
    }

    fn stmt(&mut self, fb: &mut FuncBuilder, s: &Stmt) {
        if matches!(s.kind, StmtKind::Directive(_)) {
            return;
        }
        fb.emit(Instr::Stmt(s.line));
        match &s.kind {
            StmtKind::Assign { target, value } => {
                self.expr(fb, value);
                self.store_target(fb, target);
            }
            StmtKind::Store { value, targets } => {
                self.expr(fb, value);
                for (i, t) in targets.iter().enumerate() {
                    if i + 1 < targets.len() {
                        fb.emit(Instr::Dup);
                    }
                    self.store_to(fb, t);
                }
            }
            StmtKind::Local(decls) => {
                for d in decls {
                    // a local lives in a slot picked while the program is compiled, so a name
                    // that is only worked out when it runs cannot have one
                    let NameRef::Named(name) = &d.name else {
                        self.error(d.name.span(), "LOCAL (cName) needs a name the compiler can see; PRIVATE takes one worked out as it runs");
                        continue;
                    };
                    let slot = fb.add_local(&name.upper);
                    fb.emit(Instr::DeclLocal(slot));
                    if let Some(dims) = &d.dims {
                        self.dims(fb, dims, name.span);
                        fb.emit(Instr::Dim { target: Var::Local(slot), ndims: dims.len() as u8 });
                    }
                }
            }
            StmtKind::Private(decls) => self.declare_by_name(fb, decls, false),
            StmtKind::Public(decls) => self.declare_by_name(fb, decls, true),
            StmtKind::Dimension(decls) => {
                for d in decls {
                    // `DIMENSION THISFORM.aRows[1, 2]` gives the property a new array of that
                    // shape, which is an assignment; only a variable is declared in place
                    if let Some(target) = &d.member {
                        // `DIMENSION THIS.aRows[2]` and `DIMENSION THIS.aRows(2)` are the same
                        // statement: brackets and parentheses index alike in FoxPro, so the
                        // second reads as a method call until a DIMENSION says it is an array.
                        // The foundation classes write `DIME THIS.aMemos(THIS.iMemos)`.
                        let sized: Option<(Expr, Vec<Expr>)> = match &target.kind {
                            ExprKind::Index { base, args } => Some(((**base).clone(), args.clone())),
                            ExprKind::MethodCall { obj, name, args } if args.iter().all(|a| !a.by_ref) => Some((
                                Expr::new(ExprKind::Member { obj: obj.clone(), name: name.clone() }, target.span),
                                args.iter().map(|a| a.expr.clone()).collect(),
                            )),
                            _ => None,
                        };
                        let Some((base, args)) = sized else {
                            self.error(target.span, "DIMENSION requires array dimensions");
                            continue;
                        };
                        // Sizing an array property is its own instruction and not an assignment
                        // of a fresh array: measured in Visual FoxPro, `DIMENSION` keeps the
                        // elements that still fit - `oObj.a[1,1]` survives a regrow to [4,2] -
                        // and the host is the only place that knows what the property holds now.
                        let ExprKind::Member { obj, name } = &base.kind else {
                            self.error(target.span, "DIMENSION requires an array property");
                            continue;
                        };
                        // `m.aRows[3]` is a memory variable wearing the `m.` prefix and not a
                        // property of anything, so it is declared where a plain name would be
                        if matches!(&obj.kind, ExprKind::Var(b) if b.upper == "M") && fb.local("M").is_none() {
                            for a in &args {
                                self.expr(fb, a);
                            }
                            let v = self.var_target(fb, name);
                            fb.emit(Instr::Dim { target: v, ndims: args.len() as u8 });
                            continue;
                        }
                        self.expr(fb, obj);
                        for a in &args {
                            self.expr(fb, a);
                        }
                        let m = self.member(name);
                        fb.emit(Instr::DimMember { name: m, ndims: args.len() as u8 });
                        continue;
                    }
                    let Some(dims) = &d.dims else {
                        self.error(d.name.span(), "DIMENSION requires array dimensions");
                        continue;
                    };
                    let NameRef::Named(name) = &d.name else {
                        self.error(d.name.span(), "DIMENSION (cName) is not supported yet");
                        continue;
                    };
                    self.dims(fb, dims, name.span);
                    let target = self.var_target(fb, name);
                    fb.emit(Instr::Dim { target, ndims: dims.len() as u8 });
                }
            }
            StmtKind::Parameters(_) | StmtKind::LParameters(_) => {
                self.error(s.span, "PARAMETERS must be the first statement of a procedure");
            }
            StmtKind::If { cond, then, else_ } => {
                self.expr(fb, cond);
                let jf = fb.emit(Instr::JumpIfFalse(0));
                self.block(fb, then);
                match else_ {
                    Some(e) => {
                        let jend = fb.emit(Instr::Jump(0));
                        fb.patch_here(jf);
                        self.block(fb, e);
                        fb.patch_here(jend);
                    }
                    None => fb.patch_here(jf),
                }
            }
            StmtKind::DoCase { cases, otherwise } => {
                let mut ends = Vec::new();
                for (cond, body) in cases {
                    self.expr(fb, cond);
                    let jf = fb.emit(Instr::JumpIfFalse(0));
                    self.block(fb, body);
                    ends.push(fb.emit(Instr::Jump(0)));
                    fb.patch_here(jf);
                }
                if let Some(o) = otherwise {
                    self.block(fb, o);
                }
                for e in ends {
                    fb.patch_here(e);
                }
            }
            StmtKind::DoWhile { cond, body } => {
                let top = fb.pc();
                self.expr(fb, cond);
                let jf = fb.emit(Instr::JumpIfFalse(0));
                self.push_loop(fb, 0);
                self.block(fb, body);
                let ctx = fb.loops.pop().expect("loop");
                for c in ctx.continues {
                    fb.patch(c, top);
                }
                fb.emit(Instr::Jump(top));
                fb.patch_here(jf);
                for b in ctx.breaks {
                    fb.patch_here(b);
                }
            }
            StmtKind::For { var, from, to, step, body } => {
                let target = self.var_target(fb, var);
                self.expr(fb, from);
                self.store_var(fb, target);
                let t_end = fb.temp("FOR_END");
                self.expr(fb, to);
                fb.emit(Instr::StoreLocal(t_end));
                let t_step = fb.temp("FOR_STEP");
                match step {
                    Some(st) => self.expr(fb, st),
                    None => {
                        let c = self.constant(Constant::num(1.0));
                        fb.emit(Instr::Const(c));
                    }
                }
                fb.emit(Instr::StoreLocal(t_step));
                let top = fb.pc();
                self.load_var(fb, target);
                fb.emit(Instr::LoadLocal(t_end));
                fb.emit(Instr::LoadLocal(t_step));
                let test = fb.emit(Instr::ForTest(0));
                self.push_loop(fb, 0);
                self.block(fb, body);
                let ctx = fb.loops.pop().expect("loop");
                let cont = fb.pc();
                for c in ctx.continues {
                    fb.patch(c, cont);
                }
                self.load_var(fb, target);
                fb.emit(Instr::LoadLocal(t_step));
                fb.emit(Instr::Add);
                self.store_var(fb, target);
                fb.emit(Instr::Jump(top));
                fb.patch_here(test);
                for b in ctx.breaks {
                    fb.patch_here(b);
                }
            }
            StmtKind::ForEach { var, collection, body } => {
                let target = self.var_target(fb, var);
                self.expr(fb, collection);
                let zero = self.constant(Constant::num(0.0));
                fb.emit(Instr::Const(zero));
                let top = fb.pc();
                let next = fb.emit(Instr::ForEachNext(0));
                self.store_var(fb, target);
                self.push_loop(fb, 2);
                self.block(fb, body);
                let ctx = fb.loops.pop().expect("loop");
                for c in ctx.continues {
                    fb.patch(c, top);
                }
                fb.emit(Instr::Jump(top));
                fb.patch_here(next);
                for b in ctx.breaks {
                    fb.patch_here(b);
                }
            }
            StmtKind::Exit | StmtKind::Loop => {
                let is_exit = matches!(s.kind, StmtKind::Exit);
                if fb.loops.is_empty() {
                    let what = if is_exit { "EXIT" } else { "LOOP" };
                    self.error(s.span, format!("{what} can only be used inside a loop"));
                    return;
                }
                let last = fb.loops.len() - 1;
                let (with_depth, try_depth, extra) = {
                    let l = &fb.loops[last];
                    (l.with_depth, l.try_depth, l.stack_extra)
                };
                self.leave_tries(fb, try_depth);
                for _ in with_depth..fb.with_depth {
                    fb.emit(Instr::PopWith);
                }
                if is_exit {
                    for _ in 0..extra {
                        fb.emit(Instr::Pop);
                    }
                }
                let j = fb.emit(Instr::Jump(0));
                if is_exit {
                    fb.loops[last].breaks.push(j);
                } else {
                    fb.loops[last].continues.push(j);
                }
            }
            StmtKind::Return(value) => {
                match value {
                    Some(e) => self.expr(fb, e),
                    None => {
                        fb.emit(Instr::True);
                    }
                }
                self.leave_tries(fb, 0);
                fb.emit(Instr::Return);
            }
            StmtKind::ReturnTo(to) => {
                let name = self.name(to.as_ref().map_or("", |n| n.upper.as_str()));
                self.leave_tries(fb, 0);
                fb.emit(Instr::ReturnTo(name));
            }
            StmtKind::Do { name, args, in_prog } => {
                // `DO Main.fxm` installs a menu; every other DO calls a procedure or program
                let lower = name.upper.to_ascii_lowercase();
                if in_prog.is_none() && (lower.ends_with(".fxm") || lower.ends_with(".mnx") || lower.ends_with(".mpr"))
                {
                    let n = self.name(&name.upper);
                    fb.emit(Instr::DoMenu(n));
                    return;
                }
                // the program the procedure lives in goes on first, under the arguments,
                // because it may be worked out rather than written: `DO x IN LOCFILE(...)`
                if let Some(p) = in_prog {
                    self.expr(fb, p);
                }
                let argc = self.args(fb, args, true, true);
                let n = self.name(&name.upper);
                fb.emit(Instr::Do { name: n, argc, in_prog: in_prog.is_some() });
            }
            StmtKind::DoExpr { name, args, in_prog } => {
                if let Some(p) = in_prog {
                    self.expr(fb, p);
                }
                self.expr(fb, name);
                let argc = self.args(fb, args, true, true);
                fb.emit(Instr::DoDynamic { argc, in_prog: in_prog.is_some() });
            }
            StmtKind::DoForm { name, name_var, linked, args, to_var, noshow } => {
                self.expr(fb, name);
                let argc = self.args(fb, args, false, false);
                let mut flags = 0;
                if *linked {
                    flags |= form_flags::LINKED;
                }
                if *noshow {
                    flags |= form_flags::NOSHOW;
                }
                if to_var.is_some() {
                    flags |= form_flags::WANT_RESULT;
                }
                if name_var.is_some() {
                    flags |= form_flags::WANT_OBJECT;
                }
                fb.emit(Instr::DoForm { flags, argc });
                if let Some(t) = to_var {
                    let v = self.var_target(fb, t);
                    self.store_var(fb, v);
                }
                if let Some(n) = name_var {
                    self.store_to(fb, n);
                }
            }
            StmtKind::ExprStmt(e) => {
                self.expr(fb, e);
                fb.emit(Instr::Pop);
            }
            StmtKind::Print { items, newline } => {
                // `?` ends the line that is open before it works out what to print, so an item
                // that raises an error leaves the blank line behind it - which is what Visual
                // FoxPro does, and what a golden's output shows after a failed line.
                if *newline {
                    fb.emit(Instr::Print { newline: true, argc: 0 });
                }
                for it in items {
                    self.expr(fb, it);
                }
                fb.emit(Instr::Print { newline: false, argc: items.len() as u8 });
            }
            StmtKind::WaitWindow { text, nowait, timeout, clear, to_var } => {
                let mut flags = 0;
                if let Some(t) = text {
                    self.expr(fb, t);
                    flags |= wait_flags::HAS_TEXT;
                }
                if let Some(t) = timeout {
                    self.expr(fb, t);
                    flags |= wait_flags::HAS_TIMEOUT;
                }
                if *nowait {
                    flags |= wait_flags::NOWAIT;
                }
                if *clear {
                    flags |= wait_flags::CLEAR;
                }
                if to_var.is_some() {
                    flags |= wait_flags::WANT_KEY;
                }
                fb.emit(Instr::WaitWindow(flags));
                if let Some(v) = to_var {
                    let t = self.var_target(fb, v);
                    self.store_var(fb, t);
                }
            }
            StmtKind::ReadEvents => {
                fb.emit(Instr::ReadEvents);
            }
            StmtKind::ClearEvents => {
                fb.emit(Instr::ClearEvents);
            }
            StmtKind::Release(items) => {
                for it in items {
                    match &it.kind {
                        ExprKind::Var(n) => match fb.local(&n.upper) {
                            Some(slot) => {
                                fb.emit(Instr::ReleaseLocal(slot));
                            }
                            None => {
                                let i = self.name(&n.upper);
                                fb.emit(Instr::ReleaseName(i));
                            }
                        },
                        _ => {
                            self.expr(fb, it);
                            fb.emit(Instr::ReleaseObject);
                        }
                    }
                }
            }
            StmtKind::ReleaseAll => {
                fb.emit(Instr::ReleaseAll);
            }
            StmtKind::Quit => {
                fb.emit(Instr::Quit);
            }
            StmtKind::Cancel => {
                fb.emit(Instr::Cancel);
            }
            StmtKind::With { obj, body } => {
                self.expr(fb, obj);
                fb.emit(Instr::PushWith);
                fb.with_depth += 1;
                self.block(fb, body);
                fb.with_depth -= 1;
                fb.emit(Instr::PopWith);
            }
            StmtKind::Set { setting, value } => {
                let argc = match value {
                    SetValue::On => {
                        fb.emit(Instr::True);
                        1
                    }
                    SetValue::Off => {
                        fb.emit(Instr::False);
                        1
                    }
                    SetValue::To(exprs) => {
                        for e in exprs {
                            self.expr(fb, e);
                        }
                        exprs.len() as u8
                    }
                    SetValue::Word(w) => {
                        let c = self.str_const(w);
                        fb.emit(Instr::Const(c));
                        1
                    }
                };
                let n = self.name(&setting.upper);
                fb.emit(Instr::SetCmd { name: n, argc, to: matches!(value, SetValue::To(_)) });
            }
            StmtKind::Try { body, catches, finally } => self.try_stmt(fb, body, catches, finally.as_ref()),
            StmtKind::Use { table, alias, exclusive, online, in_area, order, order_desc, indexes } => {
                // the work area and the alias go under the path, so the instruction still finds
                // the path on top when a suspend runs it again
                if let Some(area) = in_area {
                    self.expr(fb, area);
                }
                let worked_alias = matches!(alias, Some(NameRef::Computed(_)));
                if let Some(NameRef::Computed(e)) = alias {
                    self.expr(fb, e);
                }
                match table {
                    Some(e) => self.expr(fb, e),
                    // `USE` on its own closes the work area; the empty path says so
                    None => {
                        let c = self.constant(Constant::Str(String::new()));
                        fb.emit(Instr::Const(c));
                    }
                }
                let alias = match alias {
                    Some(NameRef::Named(a)) => Some(self.name(&a.upper)),
                    _ => None,
                };
                fb.emit(Instr::Use {
                    alias,
                    named_alias: worked_alias,
                    exclusive: *exclusive,
                    online: *online,
                    in_area: in_area.is_some(),
                });
                if table.is_some() {
                    // the compound index beside the table opens with it, so the tags are there
                    // for SET ORDER, and a record written later keeps them right
                    fb.emit(Instr::OpenIndex);
                    if !indexes.is_empty() {
                        for file in indexes {
                            self.expr(fb, file);
                        }
                        fb.emit(Instr::OpenIdx { count: indexes.len().min(255) as u8 });
                    }
                    if let Some(tag) = order {
                        self.expr(fb, tag);
                        fb.emit(Instr::SetOrder { descending: *order_desc });
                    }
                }
            }
            StmtKind::SetOrder { tag, descending, area } => {
                // `IN alias` puts that table in the order and leaves the selection alone
                if let Some(a) = area {
                    self.push_area(fb, a);
                    fb.emit(Instr::PushArea);
                }
                match tag {
                    Some(e) => self.expr(fb, e),
                    None => {
                        fb.emit(Instr::Omitted);
                    }
                }
                fb.emit(Instr::SetOrder { descending: *descending });
                if area.is_some() {
                    fb.emit(Instr::PopArea);
                }
            }
            StmtKind::Seek { key, order, descending } => {
                if let Some(tag) = order {
                    self.expr(fb, tag);
                    fb.emit(Instr::SetOrder { descending: *descending });
                }
                self.expr(fb, key);
                fb.emit(Instr::Seek);
            }
            StmtKind::CopyTo { path, kind, fields, except, scope, cond, while_, keys, text } => {
                self.copy_to(fb, path, *kind, fields, except, scope, cond.as_ref(), while_.as_ref(), keys, text.as_ref());
            }
            StmtKind::AppendFrom { path, fields, except, cond, text } => {
                self.expr(fb, path);
                let (kind, count) = self.field_names(fb, fields, except);
                let code = match text {
                    None => 0,
                    Some(TextFormat::Sdf) => 1,
                    Some(TextFormat::Csv) => 2,
                    Some(TextFormat::Delimited { .. }) => 3,
                    // APPEND FROM has no DIF or SYLK form: those are what EXPORT writes
                    Some(TextFormat::Dif) | Some(TextFormat::Sylk) => 0,
                };
                let cond = if cond.is_empty() { None } else { Some(self.constant(Constant::Str(cond.clone()))) };
                fb.emit(Instr::AppendFrom { except: kind, count, cond, text: code });
            }
            StmtKind::Unlock { record, area, all } => {
                if let Some(e) = record {
                    self.expr(fb, e);
                }
                // an alias the program worked out selects the area first, and the selection
                // goes back afterwards, which is what UNLOCK IN means either way
                let named = match area {
                    Some(NameRef::Named(a)) => Some(self.name(&a.upper)),
                    _ => None,
                };
                if let Some(a @ NameRef::Computed(_)) = area {
                    self.push_area(fb, a);
                    fb.emit(Instr::PushArea);
                }
                fb.emit(Instr::Unlock { record: record.is_some(), area: named, all: *all });
                if matches!(area, Some(NameRef::Computed(_))) {
                    fb.emit(Instr::PopArea);
                }
            }
            StmtKind::AlterTable { path, ops } => {
                self.expr(fb, path);
                // a column name the program works out goes on the stack above the path, in the
                // order the changes were written, which is the order they are taken off in
                let mut steps = Vec::new();
                for op in ops {
                    let step = match op {
                        AlterOp::Add(f) | AlterOp::Alter(f) => {
                            let kind = if matches!(op, AlterOp::Add(_)) { 0 } else { 1 };
                            let name = self.written_name(fb, &f.name);
                            let columns = vec![column_of(f)];
                            self.module.cursors.push((String::new(), columns));
                            let def = self.module.cursors.len() as u32 - 1;
                            AlterStep { kind, name, extra: Some(def) }
                        }
                        AlterOp::Drop(name) => AlterStep { kind: 2, name: self.written_name(fb, name), extra: Some(0) },
                        AlterOp::Rename(from, to) => {
                            let name = self.written_name(fb, from);
                            AlterStep { kind: 3, name, extra: self.written_name(fb, to) }
                        }
                    };
                    steps.push(step);
                }
                fb.emit(Instr::AlterTable(steps));
            }
            StmtKind::Transaction(step) => {
                let code = match step {
                    TransactionStep::Begin => 0,
                    TransactionStep::End => 1,
                    TransactionStep::Rollback => 2,
                };
                fb.emit(Instr::Transaction(code));
            }
            StmtKind::Database { what, name, target, sql, flags } => {
                if let Some(e) = name {
                    self.expr(fb, e);
                }
                if let Some(e) = target {
                    self.expr(fb, e);
                }
                let kind = match what {
                    DbCommand::AppendProcedures => 14,
                    DbCommand::CopyProcedures => 15,
                    DbCommand::Pack => 16,
                    DbCommand::RenameView => 17,
                    DbCommand::Create => 0,
                    DbCommand::Open => 1,
                    DbCommand::Close => 2,
                    DbCommand::Set => 3,
                    DbCommand::Delete => 4,
                    DbCommand::Validate => 5,
                    DbCommand::AddTable => 6,
                    DbCommand::RemoveTable => 7,
                    DbCommand::FreeTable => 8,
                    DbCommand::RenameTable => 9,
                    DbCommand::CreateView => 10,
                    DbCommand::DropView => 11,
                    DbCommand::CreateConnection => 18,
                    DbCommand::DeleteConnection => 19,
                    DbCommand::RenameConnection => 20,
                    DbCommand::ModifyConnection => 21,
                    DbCommand::List => 12,
                };
                let sql = (!sql.is_empty()).then(|| self.constant(Constant::Str(sql.clone())));
                fb.emit(Instr::DbCommand { kind, named: name.is_some(), target: target.is_some(), sql, flags: *flags });
            }
            StmtKind::ListInfo { what, fields, skeleton, scope, cond, while_, numbers } => {
                let kind = match what.as_str() {
                    "STRUCTURE" => Some(0),
                    "MEMORY" => Some(1),
                    "STATUS" => Some(2),
                    "FILES" => Some(3),
                    "TABLES" => Some(4),
                    "VIEWS" => Some(5),
                    "DLLS" => Some(6),
                    "PROCEDURES" => Some(7),
                    "OBJECTS" => Some(8),
                    "CONNECTIONS" => Some(9),
                    _ => None,
                };
                match kind {
                    Some(kind) => {
                        if let Some(s) = skeleton {
                            self.expr(fb, s);
                        }
                        fb.emit(Instr::ShowInfo { kind, skeleton: skeleton.is_some() })
                    }
                    // the records themselves: the same loop every record command uses
                    None => {
                        let names = fields.iter().map(|f| f.upper.clone()).collect::<Vec<String>>().join(",");
                        let list = self.constant(Constant::Str(names));
                        let numbers = *numbers;
                        fb.emit(Instr::ListBegin);
                        self.over_records(fb, None, scope, cond.as_ref(), while_.as_ref(), &mut |_, fb| {
                            fb.emit(Instr::ListRecord { fields: list, numbers });
                        });
                        0
                    }
                };
            }
            StmtKind::ReportForm { path, label, scope, cond, while_, to_file, flags } => {
                self.expr(fb, path);
                if let Some(file) = to_file {
                    self.expr(fb, file);
                }
                fb.emit(Instr::ReportBegin { flags: *flags, label: *label, to_file: to_file.is_some() });
                // the detail band is a record loop like every other record command
                self.over_records(fb, None, scope, cond.as_ref(), while_.as_ref(), &mut |_, fb| {
                    fb.emit(Instr::ReportRow);
                });
                fb.emit(Instr::ReportEnd);
            }
            StmtKind::Eject => {
                fb.emit(Instr::Eject);
            }
            StmtKind::Ask { prompt, target, kind } => {
                if let Some(p) = prompt {
                    self.expr(fb, p);
                }
                // the answer goes where a STORE would put it, so the target is a name
                let name = match &target.kind {
                    ExprKind::Var(name) => name.upper.clone(),
                    ExprKind::Member { name, .. } => name.upper.clone(),
                    _ => {
                        self.error(target.span, "INPUT, ACCEPT and GETEXPR put the answer in a variable");
                        String::new()
                    }
                };
                let target = self.name(&name);
                fb.emit(Instr::Ask { kind: *kind, prompted: prompt.is_some(), target });
            }
            StmtKind::Memo { what, fields, field_expr, path, flags } => {
                // CLOSE MEMO names them all at once; the others are about one field each, so
                // each gets an instruction of its own and a round trip of its own
                if *what == 3 {
                    let names = fields.iter().map(|f| f.upper.clone()).collect::<Vec<String>>().join(",");
                    let list = self.constant(Constant::Str(names));
                    fb.emit(Instr::Memo { what: *what, fields: list, pathed: false, named: false, flags: *flags });
                    return;
                }
                for field in fields {
                    // the field's name goes under the file's, so the file is still on top when
                    // a suspend runs the instruction again
                    if let Some(e) = field_expr {
                        self.expr(fb, e);
                    }
                    if let Some(p) = path {
                        self.expr(fb, p);
                    }
                    let list = self.constant(Constant::Str(field.upper.clone()));
                    fb.emit(Instr::Memo {
                        what: *what,
                        fields: list,
                        pathed: path.is_some(),
                        named: field_expr.is_some(),
                        flags: *flags,
                    });
                    // the ones that put something into the record write it back before the next
                    // statement, the way every other record-changing command does
                    if matches!(*what, 0 | 2 | 4) {
                        fb.emit(Instr::FlushRecord);
                    }
                }
            }
            StmtKind::Variables { save, target, memo, skeleton, except, additive } => {
                self.expr(fb, target);
                if let Some(s) = skeleton {
                    self.expr(fb, s);
                }
                fb.emit(Instr::Variables {
                    save: *save,
                    memo: *memo,
                    skeleton: skeleton.is_some(),
                    except: *except,
                    additive: *additive,
                });
            }
            StmtKind::Mouse { clicks, at, drag, window, style } => {
                if let Some((row, col)) = at {
                    self.expr(fb, row);
                    self.expr(fb, col);
                }
                for (row, col) in drag {
                    self.expr(fb, row);
                    self.expr(fb, col);
                }
                if let Some(w) = window {
                    self.expr(fb, w);
                }
                let style = self.constant(Constant::Str(style.clone()));
                fb.emit(Instr::Mouse {
                    clicks: *clicks,
                    at: at.is_some(),
                    drags: drag.len().min(255) as u8,
                    window: window.is_some(),
                    style,
                });
            }
            StmtKind::Run { command, nowait } => {
                self.expr(fb, command);
                fb.emit(Instr::Run { nowait: *nowait });
            }
            StmtKind::Diagnostic { cond, message } => {
                if let Some(c) = cond {
                    self.expr(fb, c);
                }
                for m in message {
                    self.expr(fb, m);
                }
                fb.emit(Instr::Diagnostic { checked: cond.is_some(), argc: message.len().min(255) as u8 });
            }
            StmtKind::Debug(verb) => {
                fb.emit(Instr::Debug(*verb));
            }
            StmtKind::Yield { events } => {
                fb.emit(Instr::Yield { events: *events });
            }
            StmtKind::Blank { fields, scope, cond } => {
                let names = fields.iter().map(|f| f.upper.clone()).collect::<Vec<String>>().join(",");
                let list = self.constant(Constant::Str(names));
                self.over_records(fb, None, scope, cond.as_ref(), None, &mut |_, fb| {
                    fb.emit(Instr::Blank(list));
                });
            }
            StmtKind::OnEvent { what, command } => {
                if let Some(text) = command {
                    let c = self.constant(Constant::Str(text.clone()));
                    fb.emit(Instr::Const(c));
                }
                let what = self.constant(Constant::Str(what.clone()));
                fb.emit(Instr::OnEvent { what, given: command.is_some() });
            }
            StmtKind::Keyboard { keys, plain, clear } => {
                self.expr(fb, keys);
                fb.emit(Instr::Keyboard { plain: *plain, clear: *clear });
            }
            StmtKind::KeyStack { push, clear } => {
                fb.emit(Instr::KeyStack { push: *push, clear: *clear });
            }
            StmtKind::WindowCommand { what, name, corners, title, text, flags, read } => {
                // READ runs what its clauses name around itself: the read is one instruction,
                // and the moments either side of it are ordinary code
                if let Some(clauses) = read {
                    self.read_with_clauses(fb, s, clauses, text, *flags);
                    return;
                }
                let mut given = 0u8;
                let corner = |i: usize| corners.get(i);
                for (bit, operand) in
                    [name.as_ref(), corner(0), corner(1), corner(2), corner(3), title.as_ref()].into_iter().enumerate()
                {
                    if let Some(e) = operand {
                        self.expr(fb, e);
                        given |= 1 << bit;
                    }
                }
                let kind = match what {
                    WindowVerb::Define => 0,
                    WindowVerb::Activate => 1,
                    WindowVerb::Deactivate => 2,
                    WindowVerb::Show => 3,
                    WindowVerb::Hide => 4,
                    WindowVerb::Move => 5,
                    WindowVerb::Size => 6,
                    WindowVerb::Zoom => 7,
                    WindowVerb::Modify => 8,
                    WindowVerb::Release => 9,
                    WindowVerb::SaveWindows => 10,
                    WindowVerb::RestoreWindows => 11,
                    WindowVerb::ActivateScreen => 12,
                    WindowVerb::SaveScreen => 13,
                    WindowVerb::RestoreScreen => 14,
                    WindowVerb::ClearScreen => 15,
                    WindowVerb::Read => 16,
                    WindowVerb::ShowGets => 17,
                    WindowVerb::ClearGets => 18,
                    WindowVerb::MenuTo => 19,
                };
                let text = (!text.is_empty()).then(|| self.constant(Constant::Str(text.clone())));
                fb.emit(Instr::WindowCommand { kind, given, text, flags: *flags });
            }
            StmtKind::AtCommand(parts) => {
                for part in parts {
                    let mut given = 0u16;
                    let corner = |i: usize| part.corners.get(i);
                    for (bit, operand) in [
                        Some(&part.row),
                        Some(&part.col),
                        part.value.as_ref(),
                        corner(0),
                        corner(1),
                        part.picture.as_ref(),
                        part.function.as_ref(),
                        part.amount.as_ref(),
                        part.amount2.as_ref(),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if let Some(e) = operand {
                            self.expr(fb, e);
                            given |= 1 << bit;
                        }
                    }
                    let kind = match part.what {
                        AtVerb::Say => 0,
                        AtVerb::Get => 1,
                        AtVerb::Clear => 2,
                        AtVerb::To => 3,
                        AtVerb::Box => 4,
                        AtVerb::Fill => 5,
                        AtVerb::Scroll => 6,
                        AtVerb::Menu => 7,
                        AtVerb::Prompt => 8,
                        AtVerb::Edit => 9,
                    };
                    let text = |c: &mut Self, s: &String| (!s.is_empty()).then(|| c.constant(Constant::Str(s.clone())));
                    let style = text(self, &part.style);
                    let name = text(self, &part.name);
                    let valid = text(self, &part.valid);
                    let when = text(self, &part.when);
                    fb.emit(Instr::AtCommand { kind, given, style, name, valid, when });
                }
            }
            StmtKind::MenuCommand { what, name, of, number, prompt, key, message, text, flags } => {
                // the operands go on in a fixed order and `given` says which of them are there,
                // so the instruction knows what to take off again
                let mut given = 0u8;
                for (bit, operand) in [name, of, number, prompt, key, message].into_iter().enumerate() {
                    if let Some(e) = operand {
                        self.expr(fb, e);
                        given |= 1 << bit;
                    }
                }
                let kind = match what {
                    MenuVerb::DefineMenu => 0,
                    MenuVerb::DefinePad => 1,
                    MenuVerb::DefinePopup => 2,
                    MenuVerb::DefineBar => 3,
                    MenuVerb::OnPad => 4,
                    MenuVerb::OnBar => 5,
                    MenuVerb::OnSelectionPad => 6,
                    MenuVerb::OnSelectionBar => 7,
                    MenuVerb::OnSelectionPopup => 8,
                    MenuVerb::OnSelectionMenu => 9,
                    MenuVerb::OnExit => 10,
                    MenuVerb::Activate => 11,
                    MenuVerb::Deactivate => 12,
                    MenuVerb::Show => 13,
                    MenuVerb::Hide => 14,
                    MenuVerb::Release => 15,
                    MenuVerb::Push => 16,
                    MenuVerb::Pop => 17,
                    MenuVerb::SetMark => 18,
                    MenuVerb::SetSkip => 19,
                    MenuVerb::SetMessage => 20,
                    MenuVerb::SetSysMenu => 21,
                    MenuVerb::ReleaseBar => 22,
                    MenuVerb::ReleasePad => 23,
                };
                let text = (!text.is_empty()).then(|| self.constant(Constant::Str(text.clone())));
                fb.emit(Instr::MenuCommand { kind, given, text, flags: *flags });
            }
            StmtKind::Pack => {
                fb.emit(Instr::Pack);
            }
            StmtKind::DropTable { path, flags } => {
                self.expr(fb, path);
                fb.emit(Instr::DbCommand { kind: 13, named: true, target: false, sql: None, flags: *flags });
            }
            StmtKind::Aggregate { calls, scope, cond, while_, targets, array } => {
                self.aggregate(fb, calls, scope, cond.as_ref(), while_.as_ref(), targets, array.as_ref());
            }
            StmtKind::Scatter { fields, except, blank, to } => {
                let (kind, count) = self.field_names(fb, fields, except);
                match to {
                    ScatterWhere::Memvar => {
                        fb.emit(Instr::Scatter { to: 1, except: kind, count, blank: *blank });
                    }
                    ScatterWhere::Array(target) => {
                        fb.emit(Instr::Scatter { to: 0, except: kind, count, blank: *blank });
                        self.store_target(fb, target);
                    }
                    ScatterWhere::Name(target) => {
                        fb.emit(Instr::Scatter { to: 2, except: kind, count, blank: *blank });
                        self.store_target(fb, target);
                    }
                }
            }
            StmtKind::Gather { from, fields, except } => {
                match from {
                    ScatterWhere::Memvar => {
                        let (kind, count) = self.field_names(fb, fields, except);
                        fb.emit(Instr::Gather { to: 1, except: kind, count });
                    }
                    ScatterWhere::Array(source) | ScatterWhere::Name(source) => {
                        self.expr(fb, source);
                        let (kind, count) = self.field_names(fb, fields, except);
                        let to = if matches!(from, ScatterWhere::Array(_)) { 0 } else { 2 };
                        fb.emit(Instr::Gather { to, except: kind, count });
                    }
                }
                fb.emit(Instr::FlushRecord);
            }
            StmtKind::AppendFromArray(source) => {
                self.expr(fb, source);
                self.insert_from(fb, 0);
            }
            StmtKind::ReplaceFromArray { source, area, fields, scope, cond, while_ } => {
                // the row the next record in scope takes, counted from one as VFP counts it
                let row = fb.temp("ROW");
                let one = self.constant(Constant::num(1.0));
                fb.emit(Instr::Const(one));
                fb.emit(Instr::StoreLocal(row));
                self.over_records(fb, area.as_ref(), scope, cond.as_ref(), while_.as_ref(), &mut |c, fb| {
                    c.expr(fb, source);
                    fb.emit(Instr::LoadLocal(row));
                    let (kind, count) = c.field_names(fb, fields, &[]);
                    fb.emit(Instr::Gather { to: 3, except: kind, count });
                    fb.emit(Instr::LoadLocal(row));
                    fb.emit(Instr::Const(one));
                    fb.emit(Instr::Add);
                    fb.emit(Instr::StoreLocal(row));
                });
            }
            StmtKind::SetFilter(text) => {
                let c = self.constant(Constant::Str(text.clone()));
                fb.emit(Instr::SetFilter(c));
            }
            StmtKind::SetRelation { pairs, additive, off } => {
                match off {
                    Some(alias) => {
                        let c = self.constant(Constant::Str(alias.upper.clone()));
                        fb.emit(Instr::Const(c));
                        fb.emit(Instr::SetRelation { count: 1, additive: true, off: true });
                    }
                    None => {
                        for (text, alias) in pairs {
                            let e = self.constant(Constant::Str(text.clone()));
                            fb.emit(Instr::Const(e));
                            let a = self.constant(Constant::Str(alias.upper.clone()));
                            fb.emit(Instr::Const(a));
                        }
                        fb.emit(Instr::SetRelation { count: pairs.len() as u16, additive: *additive, off: false });
                    }
                }
            }
            StmtKind::Reindex => {
                fb.emit(Instr::Reindex);
            }
            StmtKind::DeleteTag { tag } => {
                match tag {
                    Some(name) => {
                        let c = self.constant(Constant::Str(name.text.clone()));
                        fb.emit(Instr::Const(c));
                    }
                    None => {
                        fb.emit(Instr::Omitted);
                    }
                }
                fb.emit(Instr::DeleteTag { all: tag.is_none() });
            }
            StmtKind::IndexOn {
                key,
                key_text,
                tag,
                to_file,
                compact,
                cond,
                cond_text,
                unique,
                candidate,
                descending,
                additive,
            } => {
                self.index_on(
                    fb,
                    key,
                    key_text,
                    tag,
                    cond.as_ref(),
                    cond_text,
                    *unique,
                    *candidate,
                    *descending,
                    to_file.as_ref(),
                    *compact,
                    *additive,
                );
            }
            StmtKind::CopyIndexes { files, all, target } => {
                for file in files {
                    self.expr(fb, file);
                }
                match target {
                    Some(e) => self.expr(fb, e),
                    // no TO: the tags go in the structural compound index beside the table
                    None => {
                        let c = self.str_const("");
                        fb.emit(Instr::Const(c));
                    }
                }
                fb.emit(Instr::CopyIndexes { count: files.len().min(255) as u8, all: *all });
            }
            StmtKind::CopyTag { tag, of, target } => {
                self.expr(fb, tag);
                match of {
                    Some(e) => self.expr(fb, e),
                    None => {
                        let c = self.str_const("");
                        fb.emit(Instr::Const(c));
                    }
                }
                self.expr(fb, target);
                fb.emit(Instr::CopyTag);
            }
            StmtKind::SelectArea(target) => match target {
                SelectTarget::Alias(name) => {
                    let n = self.name(&name.upper);
                    fb.emit(Instr::SelectArea(Some(n)));
                }
                SelectTarget::Number(e) => {
                    self.expr(fb, e);
                    fb.emit(Instr::SelectArea(None));
                }
            },
            StmtKind::Go { where_, area } => {
                // the work area the command names is selected for as long as the move takes,
                // and what was selected before comes back afterwards
                if let Some(a) = area {
                    self.expr(fb, a);
                    fb.emit(Instr::PushArea);
                }
                let target = match where_ {
                    GoWhere::Top => GoTarget::Top,
                    GoWhere::Bottom => GoTarget::Bottom,
                    GoWhere::Record(e) => {
                        self.expr(fb, e);
                        GoTarget::Record
                    }
                };
                fb.emit(Instr::Go(target));
                if area.is_some() {
                    fb.emit(Instr::PopArea);
                }
            }
            StmtKind::Skip { count, area } => {
                if let Some(a) = area {
                    self.expr(fb, a);
                    fb.emit(Instr::PushArea);
                }
                match count {
                    Some(e) => self.expr(fb, e),
                    None => {
                        let c = self.constant(Constant::num(1.0));
                        fb.emit(Instr::Const(c));
                    }
                }
                fb.emit(Instr::Skip);
                if area.is_some() {
                    fb.emit(Instr::PopArea);
                }
            }
            StmtKind::Scan { scope, cond, while_, body } => self.scan(fb, scope, cond.as_ref(), while_.as_ref(), body),
            StmtKind::Locate { scope, cond, while_ } => {
                fb.last_locate = Some((cond.clone(), while_.clone()));
                self.locate(fb, Some(scope), cond.as_ref(), while_.as_ref());
            }
            StmtKind::Continue => match fb.last_locate.clone() {
                // The pair belongs to the work area in VFP; here it belongs to the procedure,
                // which is where a CONTINUE and its LOCATE are in every program worth reading.
                Some((cond, while_)) => self.locate(fb, None, cond.as_ref(), while_.as_ref()),
                None => self.error(s.span, "CONTINUE without a LOCATE earlier in the same procedure"),
            },
            StmtKind::Query(q) => self.query(fb, q),
            StmtKind::UpdateSql { table, path, assignments, cond } => {
                self.select_sql_table(fb, table, path);
                self.over_records(fb, None, &Scope::All, cond.as_ref(), None, &mut |c, fb| {
                    for (field, value, _) in assignments {
                        c.expr(fb, value);
                        let f = c.member(field);
                        fb.emit(Instr::ReplaceField { field: f, additive: false });
                    }
                });
            }
            StmtKind::DeleteSql { table, path, cond } => {
                self.select_sql_table(fb, table, path);
                self.over_records(fb, None, &Scope::All, cond.as_ref(), None, &mut |_, fb| {
                    fb.emit(Instr::MarkDeleted(true));
                });
            }
            StmtKind::Replace { area, assignments, scope, cond, while_ } => {
                self.over_records(fb, area.as_ref(), scope, cond.as_ref(), while_.as_ref(), &mut |c, fb| {
                    // ADDITIVE belongs to the one assignment it is written after, not to the
                    // command: measured, `REPLACE m WITH "Q", m2 WITH "R" ADDITIVE` overwrites
                    // the first memo and adds to the second.
                    for (field, value, additive) in assignments {
                        match field {
                            NameRef::Named(name) => {
                                c.expr(fb, value);
                                let f = c.member(name);
                                fb.emit(Instr::ReplaceField { field: f, additive: *additive });
                            }
                            // the name goes on first, because the value is what a suspend and a
                            // second run would otherwise take off twice
                            NameRef::Computed(e) => {
                                c.expr(fb, e);
                                c.expr(fb, value);
                                fb.emit(Instr::ReplaceFieldNamed { additive: *additive });
                            }
                        }
                    }
                });
            }
            StmtKind::MarkDeleted { deleted, area, scope, cond, while_ } => {
                self.over_records(fb, area.as_ref(), scope, cond.as_ref(), while_.as_ref(), &mut |_, fb| {
                    fb.emit(Instr::MarkDeleted(*deleted));
                });
            }
            StmtKind::Build { what, target, from, recompile } => {
                self.expr(fb, target);
                for source in from {
                    self.expr(fb, source);
                }
                let kind = self.constant(Constant::Str(what.upper.clone()));
                fb.emit(Instr::Build { what: kind, count: from.len() as u16, recompile: *recompile });
            }
            StmtKind::Compile { what, files, flags } => {
                self.expr(fb, files);
                let kind = self.constant(Constant::Str(what.upper.clone()));
                fb.emit(Instr::Compile { what: kind, flags: *flags });
            }
            StmtKind::Import { path, sheet } => {
                self.expr(fb, path);
                if let Some(e) = sheet {
                    self.expr(fb, e);
                }
                // the sheet is read into a table of the same name, which is then the one open
                fb.emit(Instr::Import { sheet: sheet.is_some() });
                fb.emit(Instr::CopyEnd);
                fb.emit(Instr::Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: false });
            }
            StmtKind::CreateFrom { target, source } => {
                // the description is opened in a work area of its own and read record by
                // record, the way COPY TO reads a table; then the table it describes is made
                // and takes the work area the command was run in, which is where VFP leaves it
                let zero = self.constant(Constant::num(0.0));
                fb.emit(Instr::Const(zero));
                fb.emit(Instr::PushArea);
                self.expr(fb, source);
                fb.emit(Instr::Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: false });
                self.copy_to(fb, target, CopyKind::FromDescription, &[], &[], &Scope::All, None, None, &[], None);
                // let the description go before the area is left
                let empty = self.constant(Constant::Str(String::new()));
                fb.emit(Instr::Const(empty));
                fb.emit(Instr::Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: false });
                fb.emit(Instr::PopArea);
                self.expr(fb, target);
                fb.emit(Instr::Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: false });
            }
            StmtKind::NewDocument { what, path } => {
                self.expr(fb, path);
                let kind = self.constant(Constant::Str(what.upper.clone()));
                fb.emit(Instr::NewDocument(kind));
            }
            StmtKind::Modify { what, path, flags } => {
                self.expr(fb, path);
                // the designer for a table, a view or the stored procedures is something the
                // open database is told about; the rest are files like any other
                match what.upper.as_str() {
                    "STRUCTURE" => fb.emit(Instr::ModifyInContainer { what: 0, flags: *flags }),
                    "VIEW" => fb.emit(Instr::ModifyInContainer { what: 1, flags: *flags }),
                    "PROCEDURE" => fb.emit(Instr::ModifyInContainer { what: 2, flags: *flags }),
                    "DATABASE" => fb.emit(Instr::ModifyInContainer { what: 3, flags: *flags }),
                    _ => {
                        let kind = self.constant(Constant::Str(what.upper.clone()));
                        fb.emit(Instr::OpenDocument(kind))
                    }
                };
            }
            StmtKind::CreateTable { path, fields, from_array, free } => {
                let columns: Vec<crate::bytecode::ColumnDef> = fields.iter().map(column_of).collect();
                self.module.cursors.push((String::new(), columns));
                let index = self.module.cursors.len() as u32 - 1;
                // The container is asked first, because a .F. from dbc_BeforeCreateTable stops
                // the whole statement: no file, no work area, no entry in the container. So the
                // rest of it is jumped over rather than run.
                let refused = if *free {
                    None
                } else {
                    self.expr(fb, path);
                    fb.emit(Instr::CreateTableAllowed);
                    Some(fb.emit(Instr::JumpIfFalse(0)))
                };
                // a table made while a database is open belongs to it, which is what makes it a
                // database table rather than a free one; FREE says to leave it out
                if !*free {
                    self.expr(fb, path);
                }
                // the file is written, then opened exactly as USE opens one: the path stays on
                // the stack for USE, which is what leaves the new table selected
                self.expr(fb, path);
                fb.emit(Instr::Dup);
                // a column whose name the program works out goes on the stack, and so do the
                // columns an array describes - both over the copy of the path CREATE TABLE takes
                let worked = self.worked_column_names(fb, fields);
                if let Some(e) = from_array {
                    self.expr(fb, e);
                }
                fb.emit(Instr::CreateTable { index, from_array: from_array.is_some(), columns: worked });
                fb.emit(Instr::Use {
                    alias: None,
                    named_alias: false,
                    exclusive: true,
                    online: false,
                    in_area: false,
                });
                if !*free {
                    fb.emit(Instr::DbCommand { kind: 22, named: true, target: false, sql: None, flags: 0 });
                }
                if let Some(at) = refused {
                    fb.patch_here(at);
                }
            }
            StmtKind::CreateCursor { alias, named, fields, from_array } => {
                let columns: Vec<crate::bytecode::ColumnDef> = fields.iter().map(column_of).collect();
                self.module.cursors.push((alias.upper.clone(), columns));
                let index = self.module.cursors.len() as u32 - 1;
                let worked = self.worked_column_names(fb, fields);
                if let Some(e) = named {
                    self.expr(fb, e);
                }
                match from_array {
                    // `FROM ARRAY a` describes the columns instead of naming them, so there are
                    // no worked-out names to take off the stack
                    Some(e) => {
                        self.expr(fb, e);
                        fb.emit(Instr::CreateCursorFromArray { index, named: named.is_some() });
                    }
                    None => {
                        fb.emit(Instr::CreateCursor { index, named: named.is_some(), columns: worked });
                    }
                }
            }
            StmtKind::InsertBlank { before } => {
                fb.emit(Instr::InsertBlank { before: *before });
                fb.emit(Instr::FlushRecord);
            }
            StmtKind::Insert { alias, named, fields, source } => {
                if let InsertSource::Query(q) = source {
                    self.insert_select(fb, alias, named.as_ref(), fields, q);
                    return;
                }
                // An INSERT is an APPEND BLANK and a REPLACE of each value. The values are worked
                // out first, in the work area the statement was written in - `SCAN ... INSERT INTO
                // other VALUES (custno) ... ENDSCAN` reads custno from the table being scanned -
                // and the selected area is put back afterwards, as VFP puts it back.
                let saved = fb.temp("AREA");
                let (select, _) = builtins::lookup("SELECT").expect("SELECT is a builtin");
                fb.emit(Instr::CallBuiltin { id: select, argc: 0 });
                fb.emit(Instr::StoreLocal(saved));

                // whatever the record is built from is worked out before the work area changes,
                // and stays on the stack under the table's name
                let mut source_from = None;
                match source {
                    InsertSource::Values(values) => {
                        // a field whose name is worked out goes on the stack under its value,
                        // which is how the instruction that writes it by name takes the two
                        for (i, value) in values.iter().enumerate() {
                            if let Some(NameRef::Computed(e)) = fields.get(i) {
                                self.expr(fb, e);
                            }
                            self.expr(fb, value);
                        }
                    }
                    InsertSource::From(ScatterWhere::Memvar) => source_from = Some(1),
                    InsertSource::From(ScatterWhere::Array(e)) => {
                        self.expr(fb, e);
                        source_from = Some(0);
                    }
                    InsertSource::From(ScatterWhere::Name(e)) => {
                        self.expr(fb, e);
                        source_from = Some(2);
                    }
                    InsertSource::Query(_) => unreachable!("INSERT ... SELECT was compiled above"),
                }
                // the name is worked out after the values, so it is read in the work area the
                // statement was written in, and then selects the one being written to
                match named {
                    Some(e) => {
                        self.expr(fb, e);
                        fb.emit(Instr::SelectArea(None));
                    }
                    None => {
                        let n = self.name(&alias.upper);
                        fb.emit(Instr::SelectArea(Some(n)));
                    }
                }
                match (source, source_from) {
                    // `FROM ARRAY a` puts in a record per row of the array, so it is a loop; the
                    // other two sources hold one record and go round it once.
                    (_, Some(from)) => self.insert_from(fb, from),
                    (InsertSource::Values(values), None) => {
                        fb.emit(Instr::AppendBlank);
                        // the values are on the stack, last on top, so they are placed from the back
                        for (i, value) in values.iter().enumerate().rev() {
                            match fields.get(i) {
                                Some(NameRef::Named(field)) => {
                                    let m = self.member(field);
                                    fb.emit(Instr::ReplaceField { field: m, additive: false });
                                }
                                Some(NameRef::Computed(_)) => {
                                    fb.emit(Instr::ReplaceFieldNamed { additive: false });
                                }
                                None if fields.is_empty() => {
                                    fb.emit(Instr::ReplaceFieldAt(i as u8));
                                }
                                None => {
                                    self.error(value.span, "there are more values than the INSERT names fields for");
                                    fb.emit(Instr::Pop);
                                }
                            }
                        }
                        fb.emit(Instr::FlushRecord);
                    }
                    (InsertSource::From(_), None) => unreachable!("every FROM source has a number"),
                    (InsertSource::Query(_), _) => unreachable!("INSERT ... SELECT was compiled above"),
                }
                fb.emit(Instr::LoadLocal(saved));
                fb.emit(Instr::SelectArea(None));
            }
            StmtKind::Browse { fields, cond, title, flags } => {
                if let Some(t) = title {
                    self.expr(fb, t);
                }
                let names = fields.iter().map(|f| f.upper.clone()).collect::<Vec<String>>().join(",");
                let fields = (!names.is_empty()).then(|| self.constant(Constant::Str(names)));
                // the condition is kept as it was written and worked out per record, the way
                // SET FILTER and a report's Print When are
                let cond = (!cond.is_empty()).then(|| self.constant(Constant::Str(cond.clone())));
                fb.emit(Instr::Browse { fields, cond, flags: *flags, titled: title.is_some() });
            }
            StmtKind::ClearAll { tables } => {
                fb.emit(Instr::ClearAll { tables: *tables });
            }
            StmtKind::RaiseError { what, message } => {
                self.expr(fb, what);
                match message {
                    Some(m) => self.expr(fb, m),
                    None => {
                        fb.emit(Instr::Omitted);
                    }
                }
                fb.emit(Instr::RaiseError);
            }
            StmtKind::MacroText(text) => {
                let t = self.str_const(text);
                fb.emit(Instr::ExecMacroText(t));
            }
            StmtKind::DeclareDll { returns, function, library, alias, params } => {
                let proto = crate::bytecode::DllProto {
                    function: function.text.clone(),
                    called: alias.as_ref().map_or_else(|| function.upper.clone(), |a| a.upper.clone()),
                    returns: returns.as_ref().map(|r| r.upper.clone()).unwrap_or_default(),
                    params: params.iter().map(|p| (p.kind.upper.clone(), p.by_ref)).collect(),
                };
                self.module.dlls.push(proto);
                let index = self.module.dlls.len() as u32 - 1;
                self.expr(fb, library);
                fb.emit(Instr::DeclareDll(index));
            }
            StmtKind::FileCommand { kind, path, target } => {
                self.expr(fb, path);
                if let Some(t) = target {
                    self.expr(fb, t);
                }
                fb.emit(Instr::FileCommand(*kind));
            }
            StmtKind::Zap { area } => {
                if let Some(a) = area {
                    self.push_area(fb, a);
                    fb.emit(Instr::PushArea);
                }
                fb.emit(Instr::Zap);
                if area.is_some() {
                    fb.emit(Instr::PopArea);
                }
            }
            StmtKind::AppendBlank { area } => {
                if let Some(a) = area {
                    self.push_area(fb, a);
                    fb.emit(Instr::PushArea);
                }
                fb.emit(Instr::AppendBlank);
                fb.emit(Instr::FlushRecord);
                if area.is_some() {
                    fb.emit(Instr::PopArea);
                }
            }
            StmtKind::Retry => {
                fb.emit(Instr::Retry);
            }
            StmtKind::CloseTables { all } => {
                fb.emit(Instr::CloseTables { all: *all });
            }
            StmtKind::Unsupported(what) => {
                let c = self.constant(Constant::Str(what.clone()));
                fb.emit(Instr::Unsupported(c));
            }
            StmtKind::Erase(path) => {
                self.expr(fb, path);
                fb.emit(Instr::Erase);
            }
            StmtKind::Throw(value) => {
                match value {
                    Some(e) => self.expr(fb, e),
                    None => {
                        fb.emit(Instr::Null);
                    }
                }
                fb.emit(Instr::Throw);
            }
            StmtKind::NoDefault => {
                fb.emit(Instr::NoDefault);
            }
            StmtKind::DoDefault(args) => {
                let argc = self.args(fb, args, false, false);
                fb.emit(Instr::DoDefault(argc));
                fb.emit(Instr::Pop);
            }
            StmtKind::Text { target, additive, textmerge, noshow, raw } => {
                self.text_stmt(fb, target.as_ref(), *additive, *textmerge, *noshow, raw);
            }
            StmtKind::TextLine { newline, raw } => {
                let c = self.str_const(raw);
                fb.emit(Instr::Const(c));
                fb.emit(Instr::MergeText { always: false });
                fb.emit(Instr::TextOut { newline: *newline, noshow: false });
            }
            StmtKind::OnError(cmd) => {
                let c = cmd.as_ref().map(|c| self.str_const(c));
                fb.emit(Instr::OnError(c));
            }
            StmtKind::OnKeyLabel { key, command } => {
                // one of the ON family like the rest: the key it is for is part of what it
                // hangs off, so ON("KEY", "F5") reads back what was hung on F5
                if let Some(text) = command {
                    let c = self.constant(Constant::Str(text.clone()));
                    fb.emit(Instr::Const(c));
                }
                let what = self.constant(Constant::Str(format!("KEY {}", key.trim().to_ascii_uppercase())));
                fb.emit(Instr::OnEvent { what, given: command.is_some() });
            }
            StmtKind::Directive(_) => {}
        }
    }

    /// Selects the table an SQL statement names, opening it first when no work area holds it.
    ///
    /// SQL names the table rather than the work area, so `UPDATE crew ...` in a program that
    /// never said `USE crew` still has to work; the rest of the language never opens a table
    /// on a program's behalf, and this is where that difference lives.
    fn select_sql_table(&mut self, fb: &mut FuncBuilder, table: &Name, path: &Expr) {
        let (used, _) = builtins::lookup("USED").expect("USED is a builtin");
        let name = self.constant(Constant::Str(table.upper.clone()));
        fb.emit(Instr::Const(name));
        fb.emit(Instr::CallBuiltin { id: used, argc: 1 });
        let open = fb.emit(Instr::JumpIfTrue(0));
        self.expr(fb, path);
        fb.emit(Instr::Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: false });
        fb.emit(Instr::OpenIndex);
        fb.patch_here(open);
        let area = self.name(&table.upper);
        fb.emit(Instr::SelectArea(Some(area)));
    }

    /// The loop every record-changing command shares: walk the scope, run `body` on each record
    /// the conditions let through, and write it back before moving on.
    ///
    /// It is the SCAN loop again, and for the same reason: everything in it already suspends and
    /// resumes, so REPLACE ALL over a table larger than memory needs nothing of its own.
    fn over_records(
        &mut self,
        fb: &mut FuncBuilder,
        area: Option<&NameRef>,
        scope: &Scope,
        cond: Option<&Expr>,
        while_: Option<&Expr>,
        body: &mut dyn FnMut(&mut Self, &mut FuncBuilder),
    ) {
        // `IN alias` works on that table and leaves the selected one where it was, which is
        // what VFP does with it
        if let Some(a) = area {
            self.push_area(fb, a);
            fb.emit(Instr::PushArea);
        }
        // this record and no other: no loop, and the pointer does not move
        if matches!(scope, Scope::Current) {
            body(self, fb);
            fb.emit(Instr::FlushRecord);
            if area.is_some() {
                fb.emit(Instr::PopArea);
            }
            return;
        }
        let counter = self.scope_setup(fb, scope);
        let top = fb.pc();
        let mut ends = Vec::new();
        if let Some(slot) = counter {
            ends.push(self.count_left(fb, slot));
        }
        self.emit_eof(fb);
        ends.push(fb.emit(Instr::JumpIfTrue(0)));
        if let Some(w) = while_ {
            self.expr(fb, w);
            ends.push(fb.emit(Instr::JumpIfFalse(0)));
        }
        let skipped = cond.map(|c| {
            self.expr(fb, c);
            fb.emit(Instr::JumpIfFalse(0))
        });

        body(self, fb);
        fb.emit(Instr::FlushRecord);

        if let Some(j) = skipped {
            fb.patch_here(j);
        }
        if let Some(slot) = counter {
            self.count_down(fb, slot);
        }
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));
        for e in ends {
            fb.patch_here(e);
        }
        if area.is_some() {
            fb.emit(Instr::PopArea);
        }
    }

    // ----- SELECT-SQL ------------------------------------------------------------------------
    //
    // A query becomes nested loops over its sources with the WHERE clause as an ordinary test.
    // Everything those loops are built from - selecting a work area, GO TOP, EOF(), SKIP, reading
    // a field - already suspends and resumes against a host that hands out bytes a page at a
    // time, so a query over a table larger than memory needs nothing special. What is left is
    // what happens to the rows once they are gathered, and that is the plan in `query.rs`.

    fn query(&mut self, fb: &mut FuncBuilder, q: &Query) {
        if q.union.is_some() {
            return self.union_query(fb, q);
        }
        // `ORDER BY 2` and `GROUP BY 2` name the second column of the result, not the number
        // two, so they are put back to the expression that column is before anything else runs
        let resolved = by_column_number(q);
        let q = resolved.as_ref().unwrap_or(q);
        // A HAVING clause over a query that groups nothing is a WHERE clause by another name.
        // Measured in Visual FoxPro 9: `SELECT city FROM t HAVING amt > 5` filters records, and
        // may name a column the select list left out, which is exactly what WHERE does. Folding
        // the two together here is what makes that true without a rule of its own.
        let flattened = having_as_where(q);
        let q = flattened.as_ref().unwrap_or(q);
        // What is left after that folding is a HAVING clause with an aggregate in it over a
        // query that groups nothing. Measured: `SELECT city FROM t HAVING COUNT(*) > 3` is
        // error 1807, and it is raised when the query runs rather than when it compiles - a
        // program with a line like this in it compiles in the product like any other. Under
        // SET ENGINEBEHAVIOR 70 it is no error at all: the whole table folds into one group and
        // the ungrouped column is read from the last record of it, so the test comes to the run.
        let needs_group_by = q.having.is_some() && q.group_by.is_empty() && !selects_an_aggregate(q);
        // A HAVING clause asks about a group, and a subquery is a query over records: measured
        // in Visual FoxPro 9, one written there is error 1810 however it is written, while the
        // same subquery in a WHERE clause of the same query is fine.
        if q.having.as_ref().is_some_and(has_subquery) {
            return self.raise(fb, RtError::SUBQUERY_INVALID);
        }

        // the plan says what each result column is; `pushed` are the values gathered per row
        let mut columns = Vec::new();
        let mut pushed: Vec<Pushed<'_>> = Vec::new();
        for (i, column) in q.columns.iter().enumerate() {
            match column {
                QueryColumn::All(alias) => columns.push(PlanColumn::Star(alias.as_ref().map(|a| a.upper.clone()))),
                QueryColumn::Value { expr, name } => match aggregate_of(expr) {
                    Some((kind, arg)) => {
                        columns.push(PlanColumn::Agg { kind, name: column_name(name.as_ref(), arg, kind, i) });
                        if let Some(arg) = arg {
                            pushed.push(Pushed::Expr(arg));
                        }
                    }
                    None => {
                        columns.push(PlanColumn::Value { name: column_name(name.as_ref(), Some(expr), AggKind::Count, i) });
                        pushed.push(Pushed::Expr(expr));
                    }
                },
            }
        }

        // What the HAVING clause names, as somewhere in a folded row. An aggregate it asks for
        // that the select list has not got becomes a column of its own, gathered and folded the
        // same way and then dropped before the result is built.
        let visible = columns.len();
        let mut having = Vec::new();
        let mut subst = Vec::new();
        if let Some(e) = &q.having {
            self.having_refs(e, q, &mut columns, &mut pushed, &mut having, &mut subst);
        }
        let hidden = (columns.len() - visible) as u16;

        let into = match &q.into {
            // no INTO browses in VFP; here the rows go somewhere a program can read them
            QueryInto::Browse => PlanInto::Cursor("QUERY".into()),
            QueryInto::Cursor(NameRef::Named(n)) => PlanInto::Cursor(n.upper.clone()),
            // `INTO CURSOR (cName)`: what the result is called goes on the stack instead, and
            // the plan says so, because it is not known until the query runs
            QueryInto::Cursor(NameRef::Computed(_)) => PlanInto::Cursor(String::new()),
            // INTO TABLE gathers the rows the same way; what makes it a table is the write and
            // the open that follow, and until then the cursor is known by the file's own name
            QueryInto::Table(path) => PlanInto::Cursor(query_table_alias(path)),
            QueryInto::Array(_) => PlanInto::Array,
        };
        let plan = QueryPlan {
            temporaries: Vec::new(),
            columns,
            distinct: q.distinct,
            group_keys: q.group_by.len() as u16,
            order_by: order_terms(q),
            has_top: q.top.is_some(),
            top_percent: q.top_percent,
            having,
            hidden,
            into_named: matches!(q.into, QueryInto::Cursor(NameRef::Computed(_))),
            into,
        };
        // a subquery is run first, into a cursor of its own, and what is left in the WHERE clause
        // is a question about that cursor. Only uncorrelated subqueries work this way, which is
        // what the ones in real code are.
        let temporaries = self.hoist_subqueries(fb, q);
        let plan = QueryPlan { temporaries, ..plan };

        let plan_index = self.module.queries.len() as u32;
        self.module.queries.push(plan);

        if needs_group_by {
            fb.emit(Instr::SqlRequireGroupBy);
        }
        for source in &q.from {
            if let Some(named) = &source.table_expr {
                self.expr(fb, named);
            }
            let table = self.constant(Constant::Str(source.table.clone()));
            let alias = self.name(&source.alias.upper);
            fb.emit(Instr::SqlOpen { table, alias, named: source.table_expr.is_some() });
        }
        // the name of the result goes under the TOP count, which is the order SqlBegin
        // takes the two off in
        if let QueryInto::Cursor(NameRef::Computed(e)) = &q.into {
            self.expr(fb, e);
        }
        if let Some(top) = &q.top {
            self.expr(fb, top);
        }
        fb.emit(Instr::SqlBegin(plan_index));
        self.query_level(fb, q, 0, &pushed);
        // FULL JOIN is a LEFT JOIN and then the rows of the right-hand table that matched
        // nothing, which is the same loops the other way round with only the misses kept
        if q.from.iter().any(|s| s.join == JoinKind::Full) {
            self.full_join_misses(fb, q, &pushed);
        }
        // HAVING, once the rows are in: a loop over the groups with the predicate as ordinary
        // bytecode, because it is an expression of the program's own and may call the program's
        // own functions. `subst` is what turns the names in it into places in the folded row.
        if let Some(e) = &q.having {
            let outer = std::mem::replace(&mut self.having_subst, subst);
            let top = fb.pc();
            fb.emit(Instr::SqlHavingNext);
            let end = fb.emit(Instr::JumpIfFalse(0));
            self.expr(fb, e);
            fb.emit(Instr::SqlHavingKeep);
            fb.emit(Instr::Jump(top));
            fb.patch_here(end);
            self.having_subst = outer;
        }
        fb.emit(Instr::SqlEnd);
        if let QueryInto::Array(target) = &q.into {
            self.store_target(fb, target);
        }
        if let QueryInto::Table(path) = &q.into {
            // the rows are a cursor first: writing it out and opening the file again is what
            // leaves the program where INTO TABLE leaves it, on a table of its own
            self.copy_to(fb, path, CopyKind::Records, &[], &[], &Scope::All, None, None, &[], None);
            self.expr(fb, path);
            fb.emit(Instr::Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: false });
            fb.emit(Instr::OpenIndex);
        }
    }

    /// `READ WHEN ... SHOW ... ACTIVATE ... DEACTIVATE ... VALID ...`.
    ///
    /// WHEN decides whether the read happens at all; SHOW and ACTIVATE run as it starts,
    /// DEACTIVATE as it ends, and a VALID that comes out false starts it again. Each is an
    /// expression of the program's own, so all this does is put them in the right order around
    /// the one instruction that reads.
    fn read_with_clauses(
        &mut self,
        fb: &mut FuncBuilder,
        s: &Stmt,
        clauses: &ReadClauses,
        text: &str,
        flags: u8,
    ) {
        let mut skipped = None;
        if let Some(when) = &clauses.when {
            self.expr(fb, when);
            skipped = Some(fb.emit(Instr::JumpIfFalse(0)));
        }
        let again = fb.pc();
        for moment in [&clauses.show, &clauses.activate] {
            if let Some(e) = moment {
                self.expr(fb, e);
                fb.emit(Instr::Pop);
            }
        }
        let words = (!text.is_empty()).then(|| self.constant(Constant::Str(text.to_string())));
        fb.emit(Instr::WindowCommand { kind: 16, given: 0, text: words, flags });
        if let Some(e) = &clauses.deactivate {
            self.expr(fb, e);
            fb.emit(Instr::Pop);
        }
        if let Some(valid) = &clauses.valid {
            self.expr(fb, valid);
            let done = fb.emit(Instr::JumpIfTrue(0));
            fb.emit(Instr::Jump(again));
            fb.patch_here(done);
        }
        if let Some(j) = skipped {
            fb.patch_here(j);
        }
        let _ = s;
    }

    /// Runs each subquery of `q` into a cursor of its own and remembers what it was called, so
    /// the expression that asked for it can ask that cursor instead.
    fn hoist_subqueries(&mut self, fb: &mut FuncBuilder, q: &Query) -> Vec<String> {
        let mut found = Vec::new();
        for e in q.where_.iter().chain(q.having.iter()).chain(q.from.iter().filter_map(|s| s.on.as_ref())) {
            collect_subqueries(e, &mut found);
        }
        let mut aliases = Vec::new();
        for (span, sub) in found {
            self.sub_counter += 1;
            let alias = format!("__SUB{}", self.sub_counter);
            let mut inner = sub.clone();
            inner.into = QueryInto::Cursor(NameRef::Named(Name::new(alias.clone(), span)));
            self.query(fb, &inner);
            self.sub_alias.insert(span, alias.clone());
            aliases.push(alias);
        }
        aliases
    }

    /// `INSERT INTO t [(fields)] SELECT ...`.
    ///
    /// The query runs first, into a cursor of its own, and then each of its rows is a record
    /// added to the table: the row taken whole as an array and put down with GATHER, which is
    /// what makes the columns go by position - to the fields the statement names, in the order
    /// it names them, or to the table's fields in order when it names none. That is what the
    /// product does with it. The work area the statement was written in is selected again at
    /// the end, as it is after every other INSERT.
    fn insert_select(&mut self, fb: &mut FuncBuilder, alias: &Name, named: Option<&Expr>, fields: &[NameRef], q: &Query) {
        let saved = fb.temp("AREA");
        let (select, _) = builtins::lookup("SELECT").expect("SELECT is a builtin");
        fb.emit(Instr::CallBuiltin { id: select, argc: 0 });
        fb.emit(Instr::StoreLocal(saved));
        // the table's name is worked out where the statement stands, before the query moves
        let target = match named {
            Some(e) => {
                self.expr(fb, e);
                let slot = fb.temp("INTO");
                fb.emit(Instr::StoreLocal(slot));
                RowTarget::Worked(slot)
            }
            None => RowTarget::Named(self.name(&alias.upper)),
        };
        let mut names = Vec::new();
        for field in fields {
            match field {
                NameRef::Named(n) => names.push(n.clone()),
                NameRef::Computed(e) => {
                    self.error(e.span, "INSERT ... SELECT names its fields outright; a field worked out from an expression is not supported");
                    return;
                }
            }
        }
        self.sub_counter += 1;
        let rows = format!("__INS{}", self.sub_counter);
        let mut inner = q.clone();
        inner.into = QueryInto::Cursor(NameRef::Named(Name::new(rows.clone(), q.span)));
        self.query(fb, &inner);
        self.append_rows(fb, &rows, target, &names);
        self.close_alias(fb, &rows);
        fb.emit(Instr::LoadLocal(saved));
        fb.emit(Instr::SelectArea(None));
    }

    /// `SELECT ... UNION [ALL] SELECT ... [ORDER BY] [INTO]`.
    ///
    /// Each SELECT of the chain runs into a cursor of its own, the rows of the later ones are
    /// added under the first's, and the whole is then one more query over that cursor: `SELECT
    /// [DISTINCT] * FROM it`, carrying the ORDER BY and the INTO the union was written with.
    /// The columns are named by the first SELECT, which is where the product takes them from
    /// too. Rows the same on both sides are folded unless every UNION said ALL.
    fn union_query(&mut self, fb: &mut FuncBuilder, q: &Query) {
        let saved = fb.temp("AREA");
        let (select, _) = builtins::lookup("SELECT").expect("SELECT is a builtin");
        fb.emit(Instr::CallBuiltin { id: select, argc: 0 });
        fb.emit(Instr::StoreLocal(saved));

        let mut parts: Vec<(&Query, bool)> = vec![(q, true)];
        let mut next = q.union.as_deref();
        while let Some(step) = next {
            parts.push((&step.query, step.all));
            next = step.query.union.as_deref();
        }
        self.sub_counter += 1;
        let first = format!("__UNI{}", self.sub_counter);
        let first_name = self.name(&first);
        for (i, (part, _)) in parts.iter().enumerate() {
            let alias = if i == 0 { first.clone() } else { format!("{first}_{i}") };
            let mut inner = (*part).clone();
            inner.union = None;
            inner.order_by = Vec::new();
            inner.top = None;
            inner.top_percent = false;
            inner.into = QueryInto::Cursor(NameRef::Named(Name::new(alias.clone(), part.span)));
            self.query(fb, &inner);
            if i > 0 {
                self.append_rows(fb, &alias, RowTarget::Named(first_name), &[]);
                self.close_alias(fb, &alias);
            }
        }
        // the statement's own work area is where the whole is asked from, so that what the
        // query leaves selected afterwards is what any query leaves selected
        fb.emit(Instr::LoadLocal(saved));
        fb.emit(Instr::SelectArea(None));
        let whole = Query {
            distinct: parts.iter().any(|(_, all)| !all),
            top: q.top.clone(),
            top_percent: q.top_percent,
            columns: vec![QueryColumn::All(None)],
            from: vec![QuerySource {
                table: first.clone(),
                table_expr: None,
                alias: Name::new(first.clone(), q.span),
                join: JoinKind::Inner,
                joined: false,
                on: None,
                span: q.span,
            }],
            where_: None,
            group_by: Vec::new(),
            having: None,
            order_by: q.order_by.clone(),
            into: q.into.clone(),
            union: None,
            span: q.span,
        };
        self.query(fb, &whole);
        self.close_alias(fb, &first);
    }

    /// A record added to `target` for every record of the cursor `from`, each taken whole as an
    /// array and put down by position. `fields` are the fields the row goes to, or none for the
    /// table's own in order. The cursor `from` is selected afterwards, on EOF.
    fn append_rows(&mut self, fb: &mut FuncBuilder, from: &str, target: RowTarget, fields: &[Name]) {
        let row = fb.temp("ROW");
        let source = self.name(from);
        fb.emit(Instr::SelectArea(Some(source)));
        fb.emit(Instr::Go(GoTarget::Top));
        let top = fb.pc();
        self.emit_eof(fb);
        let end = fb.emit(Instr::JumpIfTrue(0));
        fb.emit(Instr::Scatter { to: 0, except: false, count: 0, blank: false });
        fb.emit(Instr::StoreLocal(row));
        match target {
            RowTarget::Named(n) => fb.emit(Instr::SelectArea(Some(n))),
            RowTarget::Worked(slot) => {
                fb.emit(Instr::LoadLocal(slot));
                fb.emit(Instr::SelectArea(None))
            }
        };
        fb.emit(Instr::AppendBlank);
        if fields.is_empty() {
            // the table's fields in order, which is what GATHER FROM ARRAY does with a row
            fb.emit(Instr::LoadLocal(row));
            fb.emit(Instr::Gather { to: 0, except: false, count: 0 });
        } else {
            // the fields the statement names, in the order it names them - GATHER FIELDS would
            // take the table's order, which is not the same thing when they differ
            let array = Name::new(fb.locals[row as usize].clone(), fields[0].span);
            for (i, field) in fields.iter().enumerate() {
                let at = Expr::new(ExprKind::Num((i + 1) as f64, 1, 0), field.span);
                let element = Expr::new(
                    ExprKind::Index { base: Box::new(Expr::new(ExprKind::Var(array.clone()), field.span)), args: vec![at] },
                    field.span,
                );
                self.expr(fb, &element);
                let m = self.member(field);
                fb.emit(Instr::ReplaceField { field: m, additive: false });
            }
        }
        fb.emit(Instr::FlushRecord);
        fb.emit(Instr::SelectArea(Some(source)));
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));
        fb.patch_here(end);
    }

    /// `USE IN alias`: the cursor is closed and the work area that was selected stays selected.
    fn close_alias(&mut self, fb: &mut FuncBuilder, alias: &str) {
        let area = self.constant(Constant::Str(alias.to_string()));
        fb.emit(Instr::Const(area));
        let none = self.constant(Constant::Str(String::new()));
        fb.emit(Instr::Const(none));
        fb.emit(Instr::Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: true });
    }

    /// Code that raises one of Visual FoxPro's own errors where it stands. It is what a statement
    /// the compiler can see is wrong compiles to when the product only complains about it as it
    /// runs, which is most of them.
    fn raise(&mut self, fb: &mut FuncBuilder, code: u32) {
        let n = self.constant(Constant::num(code as f64));
        fb.emit(Instr::Const(n));
        fb.emit(Instr::Omitted);
        fb.emit(Instr::RaiseError);
    }

    /// Works out what one piece of a HAVING clause names, and where the answer will be found.
    ///
    /// Measured in Visual FoxPro 9, a HAVING clause over a grouped query may name three things,
    /// and nothing else:
    ///
    ///   - an aggregate, whether or not the select list has that aggregate too. `HAVING
    ///     MAX(amt) > 5` works over a query that selects only `SUM(amt)`. Each one becomes a
    ///     column of the plan, so the fold that the select list's aggregates go through is the
    ///     same one these go through.
    ///   - a grouped column, qualified or not: `GROUP BY city HAVING t.city > "A"` is fine, and
    ///     a *grouped expression* is not - `GROUP BY LEFT(city,1) HAVING LEFT(city,1) = "C"` is
    ///     an error in the product, so only a key that is a plain column name is matched here.
    ///   - a name the select list gave with AS, including one for a plain column. A field of one
    ///     of the sources beats it, which is what `guard` is for.
    ///
    /// Anything else is left to the ordinary expression compiler: it is a variable, or it is a
    /// column the clause may not name, and the second is settled when the query runs - no record
    /// is current while the predicate runs, so reading a field there is the product's 1803.
    fn having_refs<'q>(
        &mut self,
        e: &'q Expr,
        q: &'q Query,
        columns: &mut Vec<PlanColumn>,
        pushed: &mut Vec<Pushed<'q>>,
        refs: &mut Vec<HavingRef>,
        subst: &mut Vec<(Expr, u16)>,
    ) {
        if let Some((kind, arg)) = aggregate_of(e) {
            let index = columns.len() as u16;
            columns.push(PlanColumn::Agg { kind, name: format!("HAVING_{}", refs.len() + 1) });
            if let Some(arg) = arg {
                pushed.push(Pushed::Expr(arg));
            }
            subst.push((e.clone(), refs.len() as u16));
            refs.push(HavingRef::Column(index));
            return;
        }
        if let Some(name) = plain_column(e) {
            if let Some(j) = q.group_by.iter().position(|g| plain_column(g).as_deref() == Some(&name)) {
                subst.push((e.clone(), refs.len() as u16));
                refs.push(HavingRef::Key(j as u16));
                return;
            }
            // Everything else is a name only a record can answer, and whether a record may be
            // asked is SET ENGINEBEHAVIOR's to say - so the answer is gathered either way and
            // the refusal left to the run. An AS alias is gathered as "the field of that name,
            // if a source has one", because the alias itself means nothing to a record; any
            // other name is gathered as it is written, which is how a memory variable in a
            // HAVING clause goes on working.
            let alias = as_named_column(q, &name);
            let index = columns.len() as u16;
            columns.push(PlanColumn::Value { name: format!("HAVING_{}", refs.len() + 1) });
            pushed.push(match alias {
                Some(_) => Pushed::SourceField(name.clone()),
                None => Pushed::Expr(e),
            });
            subst.push((e.clone(), refs.len() as u16));
            refs.push(HavingRef::Ungrouped { name, index, alias });
            return;
        }
        for child in expression_parts(e) {
            self.having_refs(child, q, columns, pushed, refs, subst);
        }
    }

    /// One nesting level: the loop over source `level`, or the row itself once past the last.
    fn query_level(&mut self, fb: &mut FuncBuilder, q: &Query, level: usize, pushed: &[Pushed<'_>]) {
        let Some(source) = q.from.get(level) else {
            return self.query_row(fb, q, pushed);
        };
        let matched = matches!(source.join, JoinKind::Left | JoinKind::Full).then(|| {
            let slot = fb.temp("JOINED");
            fb.emit(Instr::False);
            fb.emit(Instr::StoreLocal(slot));
            slot
        });

        fb.emit(Instr::SelectSource(level as u16));
        fb.emit(Instr::Go(GoTarget::Top));
        let top = fb.pc();
        // an inner level leaves its own area selected, so each level says which one it means
        fb.emit(Instr::SelectSource(level as u16));
        self.emit_eof(fb);
        let end = fb.emit(Instr::JumpIfTrue(0));

        let skip = source.on.as_ref().map(|on| {
            self.expr(fb, on);
            fb.emit(Instr::JumpIfFalse(0))
        });
        if let Some(slot) = matched {
            fb.emit(Instr::True);
            fb.emit(Instr::StoreLocal(slot));
        }
        self.query_level(fb, q, level + 1, pushed);
        if let Some(j) = skip {
            fb.patch_here(j);
        }

        fb.emit(Instr::SelectSource(level as u16));
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));
        fb.patch_here(end);

        // LEFT JOIN: an outer row that matched nothing is still a row, with the right-hand side
        // parked past its last record, where every field of it reads as .NULL.
        if let Some(slot) = matched {
            fb.emit(Instr::LoadLocal(slot));
            let done = fb.emit(Instr::JumpIfTrue(0));
            fb.emit(Instr::JoinMiss(level as u16));
            self.query_level(fb, q, level + 1, pushed);
            fb.patch_here(done);
        }
    }

    /// The second half of a FULL JOIN: every record of the right-hand table that the left-hand
    /// one has no match for, with the left-hand side read past its last record.
    ///
    /// The first half is the LEFT JOIN, which `query_level` already emits. This is the same two
    /// loops the other way round, keeping only what that half threw away, and the two together
    /// are what FULL means. The work areas stay as they were opened, so the loops name their
    /// source by number rather than being reordered.
    fn full_join_misses(&mut self, fb: &mut FuncBuilder, q: &Query, pushed: &[Pushed<'_>]) {
        let (outer, inner) = (1u16, 0u16);
        fb.emit(Instr::SelectSource(outer));
        fb.emit(Instr::Go(GoTarget::Top));
        let top = fb.pc();
        fb.emit(Instr::SelectSource(outer));
        self.emit_eof(fb);
        let end = fb.emit(Instr::JumpIfTrue(0));

        let matched = fb.temp("MISSED");
        fb.emit(Instr::False);
        fb.emit(Instr::StoreLocal(matched));
        fb.emit(Instr::SelectSource(inner));
        fb.emit(Instr::Go(GoTarget::Top));
        let scan = fb.pc();
        fb.emit(Instr::SelectSource(inner));
        self.emit_eof(fb);
        let scanned = fb.emit(Instr::JumpIfTrue(0));
        let unmatched = q.from.iter().find_map(|s| s.on.as_ref()).map(|on| {
            self.expr(fb, on);
            fb.emit(Instr::JumpIfFalse(0))
        });
        fb.emit(Instr::True);
        fb.emit(Instr::StoreLocal(matched));
        if let Some(j) = unmatched {
            fb.patch_here(j);
        }
        fb.emit(Instr::SelectSource(inner));
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(scan));
        fb.patch_here(scanned);

        fb.emit(Instr::LoadLocal(matched));
        let done = fb.emit(Instr::JumpIfTrue(0));
        fb.emit(Instr::JoinMiss(inner));
        self.query_row(fb, q, pushed);
        fb.patch_here(done);

        fb.emit(Instr::SelectSource(outer));
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));
        fb.patch_here(end);
    }

    /// The innermost body: test the WHERE clause, then push one row.
    fn query_row(&mut self, fb: &mut FuncBuilder, q: &Query, pushed: &[Pushed<'_>]) {
        // a `*` reads each source's record out of its buffer when the row is made, and only a
        // field read fills the buffer; a select list of nothing but `*` reads no field, so the
        // sources it stands for are read here first. The innermost source is selected again
        // afterwards, which is where the WHERE clause and the select list expect to stand.
        let stars: Vec<&Option<Name>> = q.columns.iter().filter_map(|c| if let QueryColumn::All(a) = c { Some(a) } else { None }).collect();
        if !stars.is_empty() {
            for (i, source) in q.from.iter().enumerate() {
                let wanted = stars.iter().any(|a| a.as_ref().is_none_or(|a| a.upper.eq_ignore_ascii_case(&source.alias.upper)));
                if wanted {
                    fb.emit(Instr::SqlTouch(i as u16));
                }
            }
            fb.emit(Instr::SelectSource((q.from.len() - 1) as u16));
        }
        let skip = q.where_.as_ref().map(|w| {
            self.expr(fb, w);
            fb.emit(Instr::JumpIfFalse(0))
        });
        for value in pushed {
            match value {
                Pushed::Expr(e) => self.expr(fb, e),
                Pushed::SourceField(name) => {
                    let n = self.name(name);
                    fb.emit(Instr::SqlSourceField(n));
                }
            }
        }
        for g in &q.group_by {
            self.expr(fb, g);
        }
        // only the terms that are read from the record are pushed; one that names a result
        // column is read out of the row the query has already made, after any folding
        let keys = order_terms(q);
        let pushed_keys: Vec<&Expr> = q
            .order_by
            .iter()
            .zip(&keys)
            .filter(|(_, term)| term.key == OrderKey::Pushed)
            .map(|(o, _)| &o.expr)
            .collect();
        for e in &pushed_keys {
            self.expr(fb, e);
        }
        let count = pushed.len() + q.group_by.len() + pushed_keys.len();
        fb.emit(Instr::SqlRow(count as u16));
        if let Some(j) = skip {
            fb.patch_here(j);
        }
    }

    // ----- table scanning -------------------------------------------------------------------
    //
    // SCAN, LOCATE and CONTINUE are loops over records, and they are compiled as loops rather
    // than given instructions of their own: the pieces they need - GO, SKIP, EOF() and a test -
    // already suspend and resume correctly, so a scan over a table paged in from the host works
    // for free, and EXIT and LOOP inside a SCAN are the same EXIT and LOOP as anywhere else.

    /// Emits the scope setup and returns the slot counting records left, when the scope limits
    /// how many are visited.
    fn scope_setup(&mut self, fb: &mut FuncBuilder, scope: &Scope) -> Option<u32> {
        match scope {
            Scope::All => {
                fb.emit(Instr::Go(GoTarget::Top));
                None
            }
            Scope::Rest | Scope::Current => None,
            Scope::Next(n) => {
                let slot = fb.temp("SCOPE");
                self.expr(fb, n);
                fb.emit(Instr::StoreLocal(slot));
                Some(slot)
            }
            Scope::Record(n) => {
                self.expr(fb, n);
                fb.emit(Instr::Go(GoTarget::Record));
                let slot = fb.temp("SCOPE");
                let one = self.constant(Constant::num(1.0));
                fb.emit(Instr::Const(one));
                fb.emit(Instr::StoreLocal(slot));
                Some(slot)
            }
        }
    }

    /// A call to `EOF()`, for the loop tests below.
    fn emit_eof(&mut self, fb: &mut FuncBuilder) {
        let (id, _) = builtins::lookup("EOF").expect("EOF is a builtin");
        fb.emit(Instr::CallBuiltin { id, argc: 0 });
    }

    /// Emits `slot = slot - 1`, the record-scope countdown.
    fn count_down(&mut self, fb: &mut FuncBuilder, slot: u32) {
        fb.emit(Instr::LoadLocal(slot));
        let one = self.constant(Constant::num(1.0));
        fb.emit(Instr::Const(one));
        fb.emit(Instr::Sub);
        fb.emit(Instr::StoreLocal(slot));
    }

    /// Emits the test `slot > 0` and returns the jump taken when the scope is used up.
    fn count_left(&mut self, fb: &mut FuncBuilder, slot: u32) -> usize {
        fb.emit(Instr::LoadLocal(slot));
        let zero = self.constant(Constant::num(0.0));
        fb.emit(Instr::Const(zero));
        fb.emit(Instr::Gt);
        fb.emit(Instr::JumpIfFalse(0))
    }

    fn emit_skip_one(&mut self, fb: &mut FuncBuilder) {
        let one = self.constant(Constant::num(1.0));
        fb.emit(Instr::Const(one));
        fb.emit(Instr::Skip);
    }

    fn scan(&mut self, fb: &mut FuncBuilder, scope: &Scope, cond: Option<&Expr>, while_: Option<&Expr>, body: &Block) {
        let counter = self.scope_setup(fb, scope);
        let top = fb.pc();
        let mut ends = Vec::new();

        if let Some(slot) = counter {
            // NEXT n counts records looked at, matching or not, which is what VFP means by it
            ends.push(self.count_left(fb, slot));
        }
        self.emit_eof(fb);
        ends.push(fb.emit(Instr::JumpIfTrue(0)));
        if let Some(w) = while_ {
            self.expr(fb, w);
            ends.push(fb.emit(Instr::JumpIfFalse(0)));
        }
        let skipped = cond.map(|c| {
            self.expr(fb, c);
            fb.emit(Instr::JumpIfFalse(0))
        });

        self.push_loop(fb, 0);
        self.block(fb, body);
        let ctx = fb.loops.pop().expect("loop");

        // LOOP, and a record the FOR rejected, both land here: the record still counts against
        // the scope, and the pointer still has to move or the scan would never end
        let next = fb.pc();
        for c in ctx.continues {
            fb.patch(c, next);
        }
        if let Some(j) = skipped {
            fb.patch_here(j);
        }
        if let Some(slot) = counter {
            self.count_down(fb, slot);
        }
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));

        for e in ends {
            fb.patch_here(e);
        }
        for b in ctx.breaks {
            fb.patch_here(b);
        }
    }

    /// `COPY TO`, `SORT TO` and `TOTAL ON`: a loop over the records handing each to the copy
    /// being built, and one instruction at the end that writes the file.
    #[allow(clippy::too_many_arguments)]
    fn copy_to(
        &mut self,
        fb: &mut FuncBuilder,
        path: &Expr,
        kind: CopyKind,
        fields: &[Name],
        except: &[Name],
        scope: &Scope,
        cond: Option<&Expr>,
        while_: Option<&Expr>,
        keys: &[SortKey],
        text: Option<&TextFormat>,
    ) {
        // COPY TO ARRAY has no path: the expression is where the array goes, and it is stored
        // into once the records have been read
        match kind {
            CopyKind::Array => {
                let empty = self.str_const("");
                fb.emit(Instr::Const(empty));
            }
            _ => self.expr(fb, path),
        }
        let (except_kind, count) = self.field_names(fb, fields, except);
        let descending = keys.iter().enumerate().fold(0u32, |bits, (i, k)| bits | (u32::from(k.descending) << i));
        let (code, quote, separator) = match text {
            None => (0u8, String::new(), String::new()),
            Some(TextFormat::Sdf) => (1, String::new(), String::new()),
            Some(TextFormat::Csv) => (2, String::new(), String::new()),
            Some(TextFormat::Dif) => (4, String::new(), String::new()),
            Some(TextFormat::Sylk) => (5, String::new(), String::new()),
            Some(TextFormat::Delimited { quote, separator }) => (3, quote.clone(), separator.clone()),
        };
        if code == 3 {
            for part in [quote, separator] {
                let c = self.constant(Constant::Str(part));
                fb.emit(Instr::Const(c));
            }
        }
        let code_kind = match kind {
            CopyKind::Records => 0,
            CopyKind::Structure => 1,
            CopyKind::Sorted => 2,
            CopyKind::Totals => 3,
            CopyKind::StructureExtended => 4,
            CopyKind::FromDescription => 5,
            CopyKind::Array => 6,
        };
        fb.emit(Instr::CopyBegin { kind: code_kind, except: except_kind, count, descending, text: code });
        // the two that write a structure read no records: what they write is the shape of the
        // table in hand, which the instruction above already has
        if !matches!(kind, CopyKind::Structure | CopyKind::StructureExtended) {
            let counter = self.scope_setup(fb, scope);
            let top = fb.pc();
            let mut ends = Vec::new();
            if let Some(slot) = counter {
                ends.push(self.count_left(fb, slot));
            }
            self.emit_eof(fb);
            ends.push(fb.emit(Instr::JumpIfTrue(0)));
            if let Some(w) = while_ {
                self.expr(fb, w);
                ends.push(fb.emit(Instr::JumpIfFalse(0)));
            }
            let skipped = cond.map(|c| {
                self.expr(fb, c);
                fb.emit(Instr::JumpIfFalse(0))
            });
            for key in keys {
                self.expr(fb, &key.expr);
            }
            fb.emit(Instr::CopyRow(keys.len() as u16));
            if let Some(j) = skipped {
                fb.patch_here(j);
            }
            if let Some(slot) = counter {
                self.count_down(fb, slot);
            }
            self.emit_skip_one(fb);
            fb.emit(Instr::Jump(top));
            for e in ends {
                fb.patch_here(e);
            }
        }
        fb.emit(Instr::CopyEnd);
        // COPY TO ARRAY leaves the array and how many records went into it on the stack. None
        // at all and the variable is left as it was, which is what Visual FoxPro does: a copy
        // that matched nothing does not make an empty array, it makes no array.
        if kind == CopyKind::Array {
            let zero = self.constant(Constant::num(0.0));
            fb.emit(Instr::Const(zero));
            fb.emit(Instr::Gt);
            let empty = fb.emit(Instr::JumpIfFalse(0));
            self.store_target(fb, path);
            let done = fb.emit(Instr::Jump(0));
            fb.patch_here(empty);
            fb.emit(Instr::Pop);
            fb.patch_here(done);
        }
    }

    /// `COUNT`, `SUM`, `AVERAGE` and `CALCULATE`: a loop over the records, one accumulator
    /// per column, and the results into the variables the command named.
    #[allow(clippy::too_many_arguments)]
    fn aggregate(
        &mut self,
        fb: &mut FuncBuilder,
        calls: &[AggCall],
        scope: &Scope,
        cond: Option<&Expr>,
        while_: Option<&Expr>,
        targets: &[Expr],
        array: Option<&Expr>,
    ) {
        let funcs: Vec<u8> = calls.iter().map(|c| agg_code(c.func)).collect();
        fb.emit(Instr::AggBegin(funcs));
        let counter = self.scope_setup(fb, scope);
        let top = fb.pc();
        let mut ends = Vec::new();
        if let Some(slot) = counter {
            ends.push(self.count_left(fb, slot));
        }
        self.emit_eof(fb);
        ends.push(fb.emit(Instr::JumpIfTrue(0)));
        if let Some(w) = while_ {
            self.expr(fb, w);
            ends.push(fb.emit(Instr::JumpIfFalse(0)));
        }
        let skipped = cond.map(|c| {
            self.expr(fb, c);
            fb.emit(Instr::JumpIfFalse(0))
        });
        for (i, call) in calls.iter().enumerate() {
            // NPV discounts a flow by a rate, so it takes both; COUNT takes nothing and is
            // given the record itself to count
            if let Some(rate) = &call.rate {
                self.expr(fb, rate);
            }
            match &call.arg {
                Some(e) => self.expr(fb, e),
                None => {
                    fb.emit(Instr::True);
                }
            }
            fb.emit(Instr::AggStep(i as u16));
        }
        if let Some(j) = skipped {
            fb.patch_here(j);
        }
        if let Some(slot) = counter {
            self.count_down(fb, slot);
        }
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));
        for e in ends {
            fb.patch_here(e);
        }
        fb.emit(Instr::AggEnd);

        // the results: all of them in one array, or one variable each
        match array {
            Some(target) => self.store_target(fb, target),
            None => {
                let slot = fb.temp("AGG");
                fb.emit(Instr::StoreLocal(slot));
                for (i, target) in targets.iter().enumerate() {
                    fb.emit(Instr::LoadLocal(slot));
                    let n = self.constant(Constant::num((i + 1) as f64));
                    fb.emit(Instr::Const(n));
                    fb.emit(Instr::LoadIndex(1));
                    self.store_target(fb, target);
                }
                if targets.is_empty() {
                    // `COUNT` with no TO says the answer out loud, as VFP does with SET TALK on
                    fb.emit(Instr::LoadLocal(slot));
                    let n = self.constant(Constant::num(1.0));
                    fb.emit(Instr::Const(n));
                    fb.emit(Instr::LoadIndex(1));
                    fb.emit(Instr::Print { newline: true, argc: 1 });
                }
            }
        }
    }

    /// The field names a FIELDS clause named, pushed as constants. Answers with whether they
    /// are the fields to leave out and how many there are.
    fn field_names(&mut self, fb: &mut FuncBuilder, fields: &[Name], except: &[Name]) -> (bool, u16) {
        let (list, kind) = if fields.is_empty() && !except.is_empty() { (except, true) } else { (fields, false) };
        for name in list {
            let c = self.constant(Constant::Str(name.upper.clone()));
            fb.emit(Instr::Const(c));
        }
        (kind, list.len() as u16)
    }

    /// `INDEX ON`: a loop over the table gathering one key per record.
    ///
    /// Like SCAN and LOCATE this is ordinary bytecode - GO TOP, EOF(), the key expression, SKIP -
    /// so a table paged in from the host is indexed without the instruction that gathers a key
    /// knowing anything about pages.
    #[allow(clippy::too_many_arguments)]
    fn index_on(
        &mut self,
        fb: &mut FuncBuilder,
        key: &Expr,
        key_text: &str,
        tag: &Name,
        cond: Option<&Expr>,
        cond_text: &str,
        unique: bool,
        candidate: bool,
        descending: bool,
        to_file: Option<&Expr>,
        compact: bool,
        additive: bool,
    ) {
        // the file a `TO` clause names goes under the rest, so that the instruction still
        // finds what it needs on top when it is run again after a suspend
        if let Some(path) = to_file {
            self.expr(fb, path);
        }
        for text in [tag.text.to_ascii_uppercase(), key_text.to_string(), cond_text.to_string()] {
            let c = self.constant(Constant::Str(text));
            fb.emit(Instr::Const(c));
        }
        let flags = u32::from(unique)
            | (u32::from(candidate) << 1)
            | (u32::from(descending) << 2)
            | (u32::from(to_file.is_some()) << 3)
            | (u32::from(compact) << 4)
            | (u32::from(additive) << 5);
        let c = self.constant(Constant::num(f64::from(flags)));
        fb.emit(Instr::Const(c));
        fb.emit(Instr::IndexBegin);

        // the index is built down the file, whatever order the table is being read in
        fb.emit(Instr::Omitted);
        fb.emit(Instr::SetOrder { descending: None });
        fb.emit(Instr::Go(GoTarget::Top));
        let top = fb.pc();
        self.emit_eof(fb);
        let done = fb.emit(Instr::JumpIfTrue(0));
        let skipped = cond.map(|c| {
            self.expr(fb, c);
            fb.emit(Instr::JumpIfFalse(0))
        });
        self.expr(fb, key);
        fb.emit(Instr::IndexKey);
        if let Some(j) = skipped {
            fb.patch_here(j);
        }
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));
        fb.patch_here(done);
        fb.emit(Instr::IndexEnd);
    }

    /// LOCATE (with a `scope`) and CONTINUE (`None`: carry on from the record after this one).
    fn locate(&mut self, fb: &mut FuncBuilder, scope: Option<&Scope>, cond: Option<&Expr>, while_: Option<&Expr>) {
        let counter = match scope {
            Some(s) => self.scope_setup(fb, s),
            None => {
                self.emit_skip_one(fb);
                None
            }
        };
        let top = fb.pc();
        let mut misses = Vec::new();
        if let Some(slot) = counter {
            misses.push(self.count_left(fb, slot));
        }
        self.emit_eof(fb);
        misses.push(fb.emit(Instr::JumpIfTrue(0)));
        if let Some(w) = while_ {
            self.expr(fb, w);
            misses.push(fb.emit(Instr::JumpIfFalse(0)));
        }
        // LOCATE with no FOR stops on the first record of the scope, which is where it already is
        let hit = match cond {
            Some(c) => {
                self.expr(fb, c);
                fb.emit(Instr::JumpIfTrue(0))
            }
            None => fb.emit(Instr::Jump(0)),
        };
        if let Some(slot) = counter {
            self.count_down(fb, slot);
        }
        self.emit_skip_one(fb);
        fb.emit(Instr::Jump(top));

        fb.patch_here(hit);
        fb.emit(Instr::True);
        let done = fb.emit(Instr::Jump(0));
        for m in misses {
            fb.patch_here(m);
        }
        fb.emit(Instr::False);
        fb.patch_here(done);
        fb.emit(Instr::SetFound);
    }

    fn push_loop(&mut self, fb: &mut FuncBuilder, stack_extra: u8) {
        fb.loops.push(LoopCtx {
            breaks: Vec::new(),
            continues: Vec::new(),
            with_depth: fb.with_depth,
            try_depth: fb.tries.len(),
            stack_extra,
        });
    }

    /// Pops the TRY handlers above `depth` and inlines their FINALLY blocks (innermost first).
    fn leave_tries(&mut self, fb: &mut FuncBuilder, depth: usize) {
        let mut i = fb.tries.len();
        while i > depth {
            i -= 1;
            fb.emit(Instr::TryPop);
            if let Some(body) = fb.tries[i].clone() {
                // The block runs outside its own TRY (already popped): temporarily hide it.
                let saved: Vec<Option<Block>> = fb.tries.drain(i..).collect();
                self.block(fb, &body);
                fb.emit(Instr::EndFinally);
                fb.tries.extend(saved);
            }
        }
    }

    fn try_stmt(&mut self, fb: &mut FuncBuilder, body: &Block, catches: &[CatchClause], finally: Option<&Block>) {
        let push = fb.emit(Instr::TryPush { catch: NO_TARGET, finally: NO_TARGET });
        fb.tries.push(finally.cloned());
        self.block(fb, body);
        fb.emit(Instr::TryPop);
        let mut to_finally = vec![fb.emit(Instr::Jump(0))];
        let mut catch_pc = NO_TARGET;
        if !catches.is_empty() {
            catch_pc = fb.pc();
            // Each clause is tried in turn; a WHEN that fails falls through to the next one, and
            // when none matches the error is rethrown so an outer TRY (and FINALLY) still sees it.
            for c in catches {
                if let Some(v) = &c.var {
                    let target = self.var_target(fb, v);
                    fb.emit(Instr::CatchObject);
                    self.store_var(fb, target);
                }
                let mut skip = None;
                if let Some(w) = &c.when {
                    self.expr(fb, w);
                    skip = Some(fb.emit(Instr::JumpIfFalse(0)));
                }
                self.block(fb, &c.body);
                fb.emit(Instr::TryPop);
                to_finally.push(fb.emit(Instr::Jump(0)));
                match skip {
                    Some(j) => fb.patch_here(j),
                    // a clause without WHEN always matches, so nothing after it can run
                    None => break,
                }
            }
            if catches.last().is_none_or(|c| c.when.is_some()) {
                fb.emit(Instr::Null);
                fb.emit(Instr::Throw);
            }
        }
        fb.tries.pop();
        let mut finally_pc = NO_TARGET;
        if let Some(f) = finally {
            finally_pc = fb.pc();
            self.block(fb, f);
            fb.emit(Instr::EndFinally);
        }
        for j in to_finally {
            if finally_pc == NO_TARGET {
                fb.patch_here(j);
            } else {
                fb.patch(j, finally_pc);
            }
        }
        fb.code[push] = Instr::TryPush { catch: catch_pc, finally: finally_pc };
    }

    fn text_stmt(
        &mut self,
        fb: &mut FuncBuilder,
        target: Option<&Expr>,
        additive: bool,
        textmerge: bool,
        noshow: bool,
        raw: &str,
    ) {
        if let (Some(t), true) = (target, additive) {
            self.expr(fb, t);
        }
        let c = self.str_const(raw);
        fb.emit(Instr::Const(c));
        // the TEXTMERGE clause merges whatever SET TEXTMERGE says; without it the setting decides
        fb.emit(Instr::MergeText { always: textmerge });
        match target {
            Some(t) => {
                if additive {
                    fb.emit(Instr::Add);
                }
                self.store_target(fb, t);
            }
            // a block with nowhere named goes where SET TEXTMERGE TO points, as `\` does
            None => {
                fb.emit(Instr::TextOut { newline: true, noshow });
            }
        }
    }

    /// `PRIVATE` and `PUBLIC`: variables the runtime finds by name, so the name may as well be
    /// worked out when the declaration runs - `PRIVATE (cName)` declares whatever it comes to.
    fn declare_by_name(&mut self, fb: &mut FuncBuilder, decls: &[VarDecl], public: bool) {
        for d in decls {
            match &d.name {
                NameRef::Named(name) => {
                    let n = self.name(&name.upper);
                    fb.emit(if public { Instr::DeclPublic(n) } else { Instr::DeclPrivate(n) });
                    if let Some(dims) = &d.dims {
                        self.dims(fb, dims, name.span);
                        fb.emit(Instr::Dim { target: Var::Name(n), ndims: dims.len() as u8 });
                    }
                }
                NameRef::Computed(e) => {
                    self.expr(fb, e);
                    fb.emit(Instr::DeclareNamed { public });
                    if d.dims.is_some() {
                        self.error(d.name.span(), "an array declared by a name expression cannot be sized here");
                    }
                }
            }
        }
    }

    fn dims(&mut self, fb: &mut FuncBuilder, dims: &[Expr], span: Span) {
        if dims.is_empty() || dims.len() > 2 {
            self.error(span, "Arrays have one or two dimensions");
        }
        for d in dims {
            self.expr(fb, d);
        }
    }

    // ----- variables -----

    fn var_target(&mut self, fb: &mut FuncBuilder, n: &Name) -> Var {
        match fb.local(&n.upper) {
            Some(slot) => Var::Local(slot),
            None => Var::Name(self.name(&n.upper)),
        }
    }

    /// A name for something the module holds a definition of. A name written out is kept with
    /// the definition; one the program works out goes on the stack instead, and `None` here is
    /// what tells the instruction to take it from there.
    fn written_name(&mut self, fb: &mut FuncBuilder, name: &NameRef) -> Option<u32> {
        match name {
            NameRef::Named(n) => Some(self.name(&n.upper)),
            NameRef::Computed(e) => {
                self.expr(fb, e);
                None
            }
        }
    }

    /// The column names of a `CREATE TABLE` or `CREATE CURSOR` that the program works out,
    /// pushed in the order the columns were written. Answers with how many there are.
    fn worked_column_names(&mut self, fb: &mut FuncBuilder, fields: &[CursorField]) -> u8 {
        let mut count = 0u8;
        for f in fields {
            if let NameRef::Computed(e) = &f.name {
                self.expr(fb, e);
                count = count.saturating_add(1);
            }
        }
        count
    }

    /// Puts that work area on the stack instead, for `PushArea`, which selects it and keeps
    /// what was selected before so the command can put it back.
    fn push_area(&mut self, fb: &mut FuncBuilder, area: &NameRef) {
        match area {
            NameRef::Named(n) => {
                let c = self.str_const(&n.upper);
                fb.emit(Instr::Const(c));
            }
            NameRef::Computed(e) => self.expr(fb, e),
        }
    }

    /// Writes the value on top of the stack where a command said to put it.
    fn store_to(&mut self, fb: &mut FuncBuilder, target: &Target) {
        match target {
            Target::Written(e) => self.store_target(fb, e),
            // the name is only known when it runs, so the store itself is worked out then -
            // the same assignment the compiler would have made had the name been written out
            Target::ByName(e) => {
                self.expr(fb, e);
                fb.emit(Instr::StoreByName);
            }
        }
    }

    fn load_var(&mut self, fb: &mut FuncBuilder, v: Var) {
        fb.emit(match v {
            Var::Local(s) => Instr::LoadLocal(s),
            Var::Name(n) => Instr::LoadName(n),
        });
    }

    fn store_var(&mut self, fb: &mut FuncBuilder, v: Var) {
        fb.emit(match v {
            Var::Local(s) => Instr::StoreLocal(s),
            Var::Name(n) => Instr::StoreName(n),
        });
    }

    /// `INSERT INTO t FROM ARRAY a | MEMVAR | NAME oRec`, with the source on top of the stack
    /// and the table already selected.
    ///
    /// An array of two dimensions holds a record per row and Visual FoxPro puts in all of them,
    /// so this is a loop; MEMVAR and NAME hold one record and go round it once. Each turn writes
    /// its record before the next is added, which is what `FlushRecord` inside the loop is for.
    fn insert_from(&mut self, fb: &mut FuncBuilder, from: u8) {
        if from == 1 {
            // MEMVAR reads variables rather than a value, so there is nothing to carry round
            fb.emit(Instr::Null);
        }
        let source = fb.temp("FROM");
        let row = fb.temp("ROW");
        fb.emit(Instr::StoreLocal(source));
        let one = self.constant(Constant::num(1.0));
        fb.emit(Instr::Const(one));
        fb.emit(Instr::StoreLocal(row));
        let top = fb.pc();
        fb.emit(Instr::LoadLocal(source));
        fb.emit(Instr::LoadLocal(row));
        fb.emit(Instr::InsertFrom { from });
        fb.emit(Instr::StoreLocal(row));
        fb.emit(Instr::FlushRecord);
        let again = fb.emit(Instr::JumpIfTrue(0));
        fb.patch(again, top);
    }

    /// Stores the value on top of the stack into an assignment target.
    fn store_target(&mut self, fb: &mut FuncBuilder, target: &Expr) {
        match &target.kind {
            ExprKind::Var(n) => {
                let v = self.var_target(fb, n);
                self.store_var(fb, v);
            }
            // `m.x = v` writes the memory variable, whatever an open table calls its fields
            ExprKind::Member { obj, name }
                if matches!(&obj.kind, ExprKind::Var(base) if base.upper == "M") && fb.local("M").is_none() =>
            {
                let v = self.var_target(fb, name);
                self.store_var(fb, v);
            }
            ExprKind::Member { obj, name } => {
                self.expr(fb, obj);
                let m = self.member(name);
                fb.emit(Instr::SetMember(m));
            }
            // `obj.Prop[2] = v` is not an array in a variable: the object owns it, so the store
            // goes to the host rather than mutating a copy the host would never see.
            ExprKind::Index { base, args } if matches!(base.kind, ExprKind::Member { .. }) => {
                let ExprKind::Member { obj, name } = &base.kind else { unreachable!() };
                self.store_member_index(fb, obj, name, args, target.span);
            }
            ExprKind::Index { base, args } => {
                self.expr(fb, base);
                let n = self.subscripts(fb, args, target.span);
                fb.emit(Instr::StoreIndex(n));
            }
            // `obj.Prop(2) = v`: VFP lets an array property be subscripted with parentheses too
            ExprKind::MethodCall { obj, name, args } => {
                let exprs: Vec<Expr> = args.iter().map(|a| a.expr.clone()).collect();
                self.store_member_index(fb, obj, name, &exprs, target.span);
            }
            // `.&cMemberCount = .&cMemberCount + 1`: the member being written is whatever the
            // variable says, so its name goes on the stack beside the object
            ExprKind::MemberByName { obj, name, args: None } => {
                self.expr(fb, obj);
                self.expr(fb, name);
                fb.emit(Instr::SetMemberByName);
            }
            ExprKind::Call { name, args } => {
                let v = self.var_target(fb, name);
                self.load_var(fb, v);
                let exprs: Vec<Expr> = args.iter().map(|a| a.expr.clone()).collect();
                let n = self.subscripts(fb, &exprs, target.span);
                fb.emit(Instr::StoreIndex(n));
            }
            // one of the words that names what the method is running in: the compiler takes it and
            // the runtime refuses it, which is what the product does
            ExprKind::This => self.cannot_redefine(fb, "THIS"),
            ExprKind::ThisForm => self.cannot_redefine(fb, "THISFORM"),
            ExprKind::ThisFormSet => self.cannot_redefine(fb, "THISFORMSET"),
            ExprKind::Screen => self.cannot_redefine(fb, "_SCREEN"),
            _ => self.error(target.span, "Invalid assignment target"),
        }
    }

    /// Throws the value away and refuses, as Visual FoxPro does when something assigns to the
    /// word that names what it is running in: error 1930, "Cannot redefine THISFORM."
    fn cannot_redefine(&mut self, fb: &mut FuncBuilder, what: &str) {
        fb.emit(Instr::Pop);
        let c = self.constant(Constant::Str(what.to_string()));
        fb.emit(Instr::Redefined(c));
    }

    /// `obj.Prop[subs] = <value already on the stack>`.
    fn store_member_index(&mut self, fb: &mut FuncBuilder, obj: &Expr, name: &Name, args: &[Expr], span: Span) {
        self.expr(fb, obj);
        let m = self.member(name);
        let n = self.subscripts(fb, args, span);
        fb.emit(Instr::SetMemberIndex { name: m, argc: n });
    }

    fn subscripts(&mut self, fb: &mut FuncBuilder, args: &[Expr], span: Span) -> u8 {
        if args.is_empty() || args.len() > 2 {
            self.error(span, "Arrays have one or two subscripts");
        }
        for a in args {
            self.expr(fb, a);
        }
        args.len().clamp(1, 2) as u8
    }

    /// Pushes call arguments; returns the count. `@var` arguments push a `Ref` cell when
    /// `allow_ref`; `DO ... WITH` passes plain variables by reference by default (`ref_default`).
    fn args(&mut self, fb: &mut FuncBuilder, args: &[Arg], allow_ref: bool, ref_default: bool) -> u8 {
        for a in args {
            if a.by_ref || ref_default {
                match &a.expr.kind {
                    ExprKind::Var(n) if allow_ref => {
                        let v = self.var_target(fb, n);
                        fb.emit(Instr::Ref(v));
                        continue;
                    }
                    ExprKind::Var(_) => self.warning(a.expr.span, "Argument is passed by value here"),
                    _ if a.by_ref => self.error(a.expr.span, "Only a variable can be passed by reference"),
                    _ => {}
                }
            }
            self.expr(fb, &a.expr);
        }
        args.len() as u8
    }

    /// The arguments of a call that fills an array, where argument `at` names that array.
    ///
    /// The name is passed as a place to write to rather than as a value, so that a program which
    /// never declared the array still gets one - `AFIELDS(laFields)` is how Visual FoxPro code
    /// is written. Anything else in that position is passed as it stands and the function will
    /// refuse it.
    fn args_filling_array(&mut self, fb: &mut FuncBuilder, args: &[Arg], at: usize) -> u8 {
        for (i, a) in args.iter().enumerate() {
            match &a.expr.kind {
                ExprKind::Var(n) if i == at && !a.by_ref => {
                    let v = self.var_target(fb, n);
                    fb.emit(Instr::RefOrMakeArray(v));
                }
                _ => {
                    self.args(fb, std::slice::from_ref(a), true, false);
                }
            }
        }
        args.len() as u8
    }

    // ----- expressions -----

    fn expr(&mut self, fb: &mut FuncBuilder, e: &Expr) {
        // Inside a HAVING predicate the aggregates and the grouped columns are not read from a
        // record - there is no record, only a folded group - so they are taken out of the row
        // instead. The list is empty everywhere else, which is all of the language but this.
        if !self.having_subst.is_empty()
            && let Some((_, which)) = self.having_subst.iter().find(|(k, _)| k == e)
        {
            let which = *which;
            fb.emit(Instr::SqlHavingValue(which));
            return;
        }
        match &e.kind {
            ExprKind::Money(c) => {
                let k = self.constant(Constant::Money(*c));
                fb.emit(Instr::Const(k));
            }
            ExprKind::Num(n, chars, decimals) => {
                let c = self.constant(Constant::Num(*n, *chars, *decimals));
                fb.emit(Instr::Const(c));
            }
            ExprKind::Lambda { params, body, line } => {
                let index = self.lambda(fb, params, body, *line);
                fb.emit(Instr::MakeLambda(index));
            }
            // `laRoutes[1, 2](req, res)`: the value is on the stack and the arguments go above
            // it, which is the shape `IndexOrCallValue` already reads for a name that holds one
            ExprKind::CallValue { target, args } => {
                self.expr(fb, target);
                let argc = self.args(fb, args, true, false);
                fb.emit(Instr::IndexOrCallValue(argc));
            }
            ExprKind::Str(s) => {
                let c = self.str_const(s);
                fb.emit(Instr::Const(c));
            }
            ExprKind::Bool(true) => {
                fb.emit(Instr::True);
            }
            ExprKind::Bool(false) => {
                fb.emit(Instr::False);
            }
            ExprKind::Null => {
                fb.emit(Instr::Null);
            }
            ExprKind::Omitted => {
                fb.emit(Instr::Omitted);
            }
            ExprKind::Date(d) => {
                let days = d.map(|d| crate::value::days_from_civil(d.year, d.month, d.day));
                let c = self.constant(Constant::Date(days));
                fb.emit(Instr::Const(c));
            }
            ExprKind::DateTime(dt) => {
                let secs = dt.map(|(d, t)| {
                    crate::value::days_from_civil(d.year, d.month, d.day) as f64 * 86400.0
                        + (t.hour * 3600 + t.minute * 60 + t.second) as f64
                });
                let c = self.constant(Constant::DateTime(secs));
                fb.emit(Instr::Const(c));
            }
            ExprKind::Var(n) => {
                let v = self.var_target(fb, n);
                self.load_var(fb, v);
            }
            ExprKind::This => {
                fb.emit(Instr::LoadThis);
            }
            ExprKind::ThisForm => {
                fb.emit(Instr::LoadThisForm);
            }
            ExprKind::ThisFormSet => {
                fb.emit(Instr::LoadThisFormSet);
            }
            ExprKind::Screen => {
                fb.emit(Instr::LoadScreen);
            }
            ExprKind::WithRef => {
                fb.emit(Instr::LoadWith);
            }
            ExprKind::Member { obj, name } => {
                // `m.x` is the memory variable, whatever the open table calls its fields, and a
                // bare `a.b` is either an object or an alias - only the VM can tell which.
                match &obj.kind {
                    // `m.x` where `x` is a slot: the local, whatever a table calls its fields
                    ExprKind::Var(base) if base.upper == "M" && fb.local(&name.upper).is_some() => {
                        let slot = fb.local(&name.upper).expect("local");
                        fb.emit(Instr::LoadLocal(slot));
                    }
                    // A declared local is an object; anything else is settled at run time - an
                    // object variable, the `m.` prefix, or the alias of an open table. `m.` wins
                    // even over a variable called `m`: `CATCH TO m` then `m.Message` reads a
                    // memory variable called Message in Visual FoxPro, not the exception's
                    // property, which is why three of these golden programs error there.
                    ExprKind::Var(base) if base.upper == "M" || fb.local(&base.upper).is_none() => {
                        let a = self.name(&base.upper);
                        let m = self.member(name);
                        fb.emit(Instr::LoadField { area: Some(a), field: m });
                    }
                    _ => {
                        self.expr(fb, obj);
                        let m = self.member(name);
                        fb.emit(Instr::GetMember(m));
                    }
                }
            }
            ExprKind::InSubquery { value, negated, .. } => {
                let Some(alias) = self.sub_alias.get(&e.span).cloned() else {
                    return self.error(e.span, "a subquery is only supported inside a SELECT statement");
                };
                self.expr(fb, value);
                let n = self.name(&alias);
                fb.emit(Instr::InCursor(n));
                if *negated {
                    fb.emit(Instr::Not);
                }
            }
            ExprKind::ExistsSubquery(_) => {
                let Some(alias) = self.sub_alias.get(&e.span).cloned() else {
                    return self.error(e.span, "a subquery is only supported inside a SELECT statement");
                };
                // EXISTS is only asking whether the cursor the subquery filled holds anything
                let c = self.constant(Constant::Str(alias));
                fb.emit(Instr::Const(c));
                let (id, _) = builtins::lookup("RECCOUNT").expect("RECCOUNT is a builtin");
                fb.emit(Instr::CallBuiltin { id, argc: 1 });
                let zero = self.constant(Constant::num(0.0));
                fb.emit(Instr::Const(zero));
                fb.emit(Instr::Gt);
            }
            ExprKind::AnySubquery(_) => {
                self.error(e.span, "ANY and SOME belong on the right of a comparison inside a SELECT statement");
            }
            ExprKind::MemberByName { obj, name, args } => {
                self.expr(fb, obj);
                self.expr(fb, name);
                match args {
                    Some(args) => {
                        let argc = self.args(fb, args, false, false);
                        fb.emit(Instr::CallMethodByName(argc));
                    }
                    None => {
                        fb.emit(Instr::GetMemberByName);
                    }
                }
            }
            ExprKind::Call { name, args } => self.call(fb, e, name, args),
            ExprKind::Index { base, args } => {
                self.expr(fb, base);
                let n = self.subscripts(fb, args, e.span);
                fb.emit(Instr::LoadIndex(n));
            }
            ExprKind::MethodCall { obj, name, args } => {
                self.expr(fb, obj);
                let argc = self.args(fb, args, false, false);
                let m = self.member(name);
                fb.emit(Instr::CallMethod { name: m, argc });
            }
            ExprKind::Unary { op, expr } => {
                self.expr(fb, expr);
                match op {
                    UnOp::Neg => {
                        fb.emit(Instr::Neg);
                    }
                    UnOp::Not => {
                        fb.emit(Instr::Not);
                    }
                    UnOp::Plus => {}
                }
            }
            ExprKind::Binary { op: BinOp::And, left, right } => {
                self.expr(fb, left);
                let j = fb.emit(Instr::JumpIfFalseKeep(0));
                self.expr(fb, right);
                fb.emit(Instr::And);
                fb.patch_here(j);
            }
            ExprKind::Binary { op: BinOp::Or, left, right } => {
                self.expr(fb, left);
                let j = fb.emit(Instr::JumpIfTrueKeep(0));
                self.expr(fb, right);
                fb.emit(Instr::Or);
                fb.patch_here(j);
            }
            ExprKind::Binary { op, left, right } => {
                self.expr(fb, left);
                self.expr(fb, right);
                fb.emit(match op {
                    BinOp::Add => Instr::Add,
                    BinOp::Sub => Instr::Sub,
                    BinOp::Mul => Instr::Mul,
                    BinOp::Div => Instr::Div,
                    BinOp::Mod => Instr::Mod,
                    BinOp::Pow => Instr::Pow,
                    BinOp::Eq => Instr::Eq,
                    BinOp::ExactEq => Instr::ExactEq,
                    BinOp::Ne => Instr::Ne,
                    BinOp::Lt => Instr::Lt,
                    BinOp::Le => Instr::Le,
                    BinOp::Gt => Instr::Gt,
                    BinOp::Ge => Instr::Ge,
                    BinOp::Contains => Instr::Contains,
                    BinOp::And | BinOp::Or => unreachable!(),
                });
            }
            ExprKind::Macro(operand) => {
                self.expr(fb, operand);
                fb.emit(Instr::Macro);
            }
        }
    }

    fn call(&mut self, fb: &mut FuncBuilder, e: &Expr, name: &Name, args: &[Arg]) {
        // A local array element, `a(1)`, or a call of a local holding a lambda, `f(1)`. Which it
        // is depends on what the slot holds when the line runs, exactly as it does for a name.
        if let Some(slot) = fb.local(&name.upper) {
            fb.emit(Instr::LoadLocal(slot));
            // how many arguments are right is not a question the compiler can answer here: a
            // subscript list is one or two long and a parameter list is as long as the lambda
            // says, and which of the two this is depends on what the slot holds when it runs
            let argc = self.args(fb, args, true, false);
            fb.emit(Instr::IndexOrCallValue(argc));
            return;
        }
        if COMPILED_FUNCTIONS.contains(&name.upper.as_str()) {
            if args.len() != 3 || args.iter().any(|a| a.by_ref) {
                self.error(e.span, "IIF() requires exactly 3 arguments");
                fb.emit(Instr::Null);
                return;
            }
            self.expr(fb, &args[0].expr);
            let jf = fb.emit(Instr::JumpIfFalse(0));
            self.expr(fb, &args[1].expr);
            let jend = fb.emit(Instr::Jump(0));
            fb.patch_here(jf);
            self.expr(fb, &args[2].expr);
            fb.patch_here(jend);
            return;
        }
        // LOOKUP names two fields and searches a third: the names are what it wants, not what
        // the fields hold, so a bare field reference is passed as the name it is written with
        if name.upper == "LOOKUP" && args.len() >= 3 {
            for (i, arg) in args.iter().enumerate() {
                match (i, &arg.expr.kind) {
                    (0 | 2, ExprKind::Var(field)) => {
                        let c = self.constant(Constant::Str(field.text.clone()));
                        fb.emit(Instr::Const(c));
                    }
                    (0 | 2, ExprKind::Member { name: field, .. }) => {
                        let c = self.constant(Constant::Str(field.text.clone()));
                        fb.emit(Instr::Const(c));
                    }
                    _ => self.expr(fb, &arg.expr),
                }
            }
            let (id, _) = builtins::lookup("LOOKUP").expect("LOOKUP is a builtin");
            fb.emit(Instr::CallBuiltin { id, argc: args.len() as u8 });
            return;
        }
        let found = builtins::lookup(&name.upper)
            .or_else(|| (!self.declared.contains(&name.upper)).then(|| builtins::lookup_abbreviated(&name.upper)).flatten());
        if let Some((id, spec)) = found {
            if let Some(msg) = builtins::arity_error(spec, args.len()) {
                self.error(e.span, msg);
            }
            let argc = match builtins::array_to_fill(spec.name) {
                Some(at) => self.args_filling_array(fb, args, at),
                None => self.args(fb, args, true, false),
            };
            fb.emit(Instr::CallBuiltin { id, argc });
            return;
        }
        if name.upper == "DODEFAULT" {
            let argc = self.args(fb, args, false, false);
            fb.emit(Instr::DoDefault(argc));
            return;
        }
        let argc = self.args(fb, args, true, false);
        let n = self.name(&name.upper);
        fb.emit(Instr::IndexOrCall { name: n, argc });
    }
}

/// One column of a `CREATE TABLE` or `ALTER TABLE` as the table format holds it.
fn column_of(f: &CursorField) -> crate::bytecode::ColumnDef {
    crate::bytecode::ColumnDef { field: field_of(f), nullable: f.nullable }
}

fn field_of(f: &CursorField) -> crate::dbf::DbfField {
    crate::dbf::DbfField {
        // a column whose name the program works out is left blank here and filled in from
        // the stack when the statement runs
        name: match &f.name {
            NameRef::Named(n) => n.upper.clone(),
            NameRef::Computed(_) => String::new(),
        },
        kind: f.kind,
        length: f.width,
        decimals: f.decimals,
        autoinc_next: f.autoinc_next,
        autoinc_step: f.autoinc_step,
        // settled when the statement runs, from the declaration or from SET NULL
        nullable: false,
    }
}

/// Folds a class property value to a constant. VFP only accepts constants there, so anything
/// else (a variable, a function call other than `RGB()`) is rejected by the caller.
fn fold_constant(e: &Expr) -> Option<Constant> {
    match &e.kind {
        ExprKind::Num(n, chars, decimals) => Some(Constant::Num(*n, *chars, *decimals)),
        ExprKind::Money(c) => Some(Constant::Money(*c)),
        ExprKind::Str(s) => Some(Constant::Str(s.clone())),
        ExprKind::Bool(b) => Some(Constant::Bool(*b)),
        ExprKind::Null => Some(Constant::Null),
        ExprKind::Date(d) => Some(Constant::Date(d.map(|d| crate::value::days_from_civil(d.year, d.month, d.day)))),
        ExprKind::DateTime(dt) => Some(Constant::DateTime(dt.map(|(d, t)| {
            crate::value::days_from_civil(d.year, d.month, d.day) as f64 * 86400.0
                + (t.hour * 3600 + t.minute * 60 + t.second) as f64
        }))),
        ExprKind::Unary { op, expr } => match (op, fold_constant(expr)?) {
            (UnOp::Neg, Constant::Num(n, w, d)) => Some(Constant::Num(-n, w.saturating_add(1), d)),
            (UnOp::Plus, c) => Some(c),
            (UnOp::Not, Constant::Bool(b)) => Some(Constant::Bool(!b)),
            _ => None,
        },
        // `RGB(r, g, b)` is the one call VFP folds in a class definition.
        ExprKind::Call { name, args } if name.upper == "RGB" && args.len() == 3 => {
            let mut parts = [0.0f64; 3];
            for (i, a) in args.iter().enumerate() {
                match fold_constant(&a.expr)? {
                    Constant::Num(n, ..) => parts[i] = n.trunc().clamp(0.0, 255.0),
                    _ => return None,
                }
            }
            Some(Constant::num(parts[0] + parts[1] * 256.0 + parts[2] * 65536.0))
        }
        ExprKind::Binary { op, left, right } => match (fold_constant(left)?, fold_constant(right)?) {
            (Constant::Num(a, ..), Constant::Num(b, ..)) => match op {
                BinOp::Add => Some(Constant::num(a + b)),
                BinOp::Sub => Some(Constant::num(a - b)),
                BinOp::Mul => Some(Constant::num(a * b)),
                BinOp::Div if b != 0.0 => Some(Constant::num(a / b)),
                _ => None,
            },
            (Constant::Str(a), Constant::Str(b)) if matches!(op, BinOp::Add) => Some(Constant::Str(a + &b)),
            _ => None,
        },
        _ => None,
    }
}

/// Records every name declared LOCAL / LPARAMETERS anywhere in the body as a slot.
fn collect_locals(b: &Block, fb: &mut FuncBuilder) {
    for s in &b.stmts {
        match &s.kind {
            StmtKind::Local(decls) => {
                for d in decls {
                    if let NameRef::Named(n) = &d.name {
                        fb.add_local(&n.upper);
                    }
                }
            }
            StmtKind::LParameters(p) => {
                for p in p {
                    fb.add_local(&p.name.upper);
                }
            }
            StmtKind::If { then, else_, .. } => {
                collect_locals(then, fb);
                if let Some(e) = else_ {
                    collect_locals(e, fb);
                }
            }
            StmtKind::DoCase { cases, otherwise } => {
                for (_, b) in cases {
                    collect_locals(b, fb);
                }
                if let Some(o) = otherwise {
                    collect_locals(o, fb);
                }
            }
            StmtKind::DoWhile { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::ForEach { body, .. }
            | StmtKind::With { body, .. } => {
                collect_locals(body, fb);
            }
            StmtKind::Try { body, catches, finally } => {
                collect_locals(body, fb);
                for c in catches {
                    collect_locals(&c.body, fb);
                }
                if let Some(f) = finally {
                    collect_locals(f, fb);
                }
            }
            _ => {}
        }
    }
}

/// Human-readable listing of a module's code (used by `foxvm disasm` and debugging).
pub fn disassemble(m: &Module) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "module {} ({:?})", m.name, m.kind);
    for (i, c) in m.consts.iter().enumerate() {
        let _ = writeln!(out, "  const {i}: {c:?}");
    }
    for (i, n) in m.names.iter().enumerate() {
        let _ = writeln!(out, "  name {i}: {n}");
    }
    for (i, n) in m.members.iter().enumerate() {
        let _ = writeln!(out, "  member {i}: {n}");
    }
    for (k, f) in &m.methods {
        let _ = writeln!(out, "  method {k} -> func {f}");
    }
    for (fi, f) in m.funcs.iter().enumerate() {
        let _ = writeln!(out, "func {fi} {} ({}) nparams={} locals={:?}", f.name, f.display_name, f.nparams, f.locals);
        for (pc, ins) in f.code.iter().enumerate() {
            let _ = writeln!(out, "  {pc:4}  {ins:?}");
        }
    }
    out
}

/// The aggregate a select item is, if it is one. `COUNT(*)` is a COUNT with no argument, which
/// is what the parser produces for it and what the expression grammar never can.
/// What an aggregate works out, as the instruction carries it.
fn agg_code(func: AggFunc) -> u8 {
    match func {
        AggFunc::Cnt => 0,
        AggFunc::Sum => 1,
        AggFunc::Avg => 2,
        AggFunc::Min => 3,
        AggFunc::Max => 4,
        AggFunc::Std => 5,
        AggFunc::Var => 6,
        AggFunc::Npv => 7,
    }
}

/// A query with every `GROUP BY n` put back to the expression that column is, or `None` when it
/// names no column by number.
///
/// SQL lets a result column be named by where it is as well as by what it is called, and a
/// number on its own means nothing else in that clause. A number that names a column of
/// `SELECT *` is left alone: which field that is depends on the tables, and the query is the
/// wrong place to work it out. `ORDER BY n` is not rewritten this way - see [`order_key`] -
/// because the column it names may be an aggregate, which is nothing a record can be asked for.
/// One value a query gathers per row, before the group keys.
///
/// Nearly all of them are expressions the select list wrote down. The other kind is a bare
/// field name a HAVING clause needs the value of, which cannot be compiled as an expression
/// because the name may be one the select list gave with AS and mean nothing to a record.
enum Pushed<'q> {
    Expr(&'q Expr),
    SourceField(String),
}

fn by_column_number(q: &Query) -> Option<Query> {
    let numbered = |e: &Expr| matches!(e.kind, ExprKind::Num(..));
    if !q.group_by.iter().any(numbered) {
        return None;
    }
    let column = |e: &Expr| match &e.kind {
        ExprKind::Num(n, ..) if *n >= 1.0 => match q.columns.get(*n as usize - 1) {
            Some(QueryColumn::Value { expr, .. }) => Some(expr.clone()),
            _ => None,
        },
        _ => None,
    };
    let mut out = q.clone();
    for term in &mut out.group_by {
        if let Some(e) = column(term) {
            *term = e;
        }
    }
    Some(out)
}

/// `HAVING` over a query that groups nothing, folded into the WHERE clause it is the same as.
///
/// Measured in Visual FoxPro 9: with no GROUP BY and no aggregate in the select list, HAVING
/// filters records exactly as WHERE does, down to being allowed to name a column the select
/// list left out. With both written, the two are ANDed. An aggregate in a HAVING clause of
/// such a query is error 1807 instead, which is left to `having_needs_group` to raise.
fn having_as_where(q: &Query) -> Option<Query> {
    let having = q.having.as_ref()?;
    if !q.group_by.is_empty() || selects_an_aggregate(q) || has_aggregate(having) {
        return None;
    }
    let mut out = q.clone();
    out.where_ = match out.where_.take() {
        Some(w) => {
            let span = w.span.to(having.span);
            Some(Expr::new(ExprKind::Binary { op: BinOp::And, left: Box::new(w), right: Box::new(having.clone()) }, span))
        }
        None => Some(having.clone()),
    };
    out.having = None;
    Some(out)
}

/// Whether the select list folds its rows into groups of its own.
fn selects_an_aggregate(q: &Query) -> bool {
    q.columns.iter().any(|c| match c {
        QueryColumn::Value { expr, .. } => aggregate_of(expr).is_some(),
        QueryColumn::All(_) => false,
    })
}

/// Whether an aggregate is written anywhere in an expression.
fn has_aggregate(e: &Expr) -> bool {
    aggregate_of(e).is_some() || expression_parts(e).into_iter().any(has_aggregate)
}

/// What each ORDER BY term of a query sorts on.
fn order_terms(q: &Query) -> Vec<OrderTerm> {
    q.order_by.iter().map(|o| OrderTerm { descending: o.descending, key: order_key(q, &o.expr) }).collect()
}

/// What one ORDER BY term sorts on.
///
/// Measured in Visual FoxPro 9, an ORDER BY term is one of three things and nothing else: a
/// result column's number, the name the select list gave a column with AS, or a column that can
/// be read from a record. Anything else - `ORDER BY SUM(amt)`, `ORDER BY LEFT(city,1)`,
/// `ORDER BY s * -1`, or a column the query groups away - is error 1808 in the product. Nothing
/// here refuses those, because being asked less of than the product asks is no incompatibility;
/// what matters is that the two that name a result column are *not* read from a record, since
/// `ORDER BY 1` over a select list whose first column is `SUM(x)` sorts the groups by their
/// sums and no record has ever held one.
fn order_key(q: &Query, e: &Expr) -> OrderKey {
    match &e.kind {
        ExprKind::Num(n, ..) if *n >= 1.0 => OrderKey::Result(*n as u16 - 1),
        ExprKind::Var(name) => match as_named_column(q, &name.upper) {
            Some(i) => OrderKey::Column(i),
            None => OrderKey::Pushed,
        },
        _ => OrderKey::Pushed,
    }
}

/// Whether a query of its own is written anywhere in an expression.
fn has_subquery(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::InSubquery { .. } | ExprKind::ExistsSubquery(_) | ExprKind::AnySubquery(_)
    ) || expression_parts(e).into_iter().any(has_subquery)
}

/// The name a plain column reference is of, qualified or not. `m.x` is a memory variable rather
/// than a column, whatever a table calls its fields, so it is not one of these.
fn plain_column(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Var(n) => Some(n.upper.clone()),
        ExprKind::Member { obj, name } => match &obj.kind {
            ExprKind::Var(base) if base.upper != "M" => Some(name.upper.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Which result column the select list gave that name to with AS.
fn as_named_column(q: &Query, name: &str) -> Option<u16> {
    q.columns
        .iter()
        .position(|c| matches!(c, QueryColumn::Value { name: Some(as_name), .. } if as_name.upper == name))
        .map(|i| i as u16)
}

/// The expressions one expression is built from, for a walk that is looking at the leaves.
/// A subquery is not one of them: it is a query of its own, with its own columns and sources.
fn expression_parts(e: &Expr) -> Vec<&Expr> {
    match &e.kind {
        ExprKind::Unary { expr, .. } => vec![expr],
        ExprKind::Binary { left, right, .. } => vec![left, right],
        ExprKind::Member { obj, .. } => vec![obj],
        ExprKind::MemberByName { obj, args, .. } => {
            let mut out = vec![obj.as_ref()];
            out.extend(args.iter().flatten().map(|a| &a.expr));
            out
        }
        ExprKind::Index { base, args } => {
            let mut out = vec![base.as_ref()];
            out.extend(args);
            out
        }
        ExprKind::Call { args, .. } => args.iter().map(|a| &a.expr).collect(),
        ExprKind::MethodCall { obj, args, .. } => {
            let mut out = vec![obj.as_ref()];
            out.extend(args.iter().map(|a| &a.expr));
            out
        }
        ExprKind::InSubquery { value, .. } => vec![value],
        ExprKind::Macro(inner) => vec![inner],
        _ => Vec::new(),
    }
}

fn aggregate_of(e: &Expr) -> Option<(AggKind, Option<&Expr>)> {
    let ExprKind::Call { name, args } = &e.kind else { return None };
    let kind = AggKind::from_name(&name.upper)?;
    match args.len() {
        0 if kind == AggKind::Count => Some((AggKind::CountAll, None)),
        1 => Some((kind, Some(&args[0].expr))),
        _ => None,
    }
}

/// What a result column is called: the AS name, else the field the expression reads, else a
/// numbered name, which is what Visual FoxPro falls back to as well.
fn column_name(as_name: Option<&Name>, expr: Option<&Expr>, kind: AggKind, index: usize) -> String {
    if let Some(n) = as_name {
        return n.upper.clone();
    }
    let field = expr.and_then(|e| match &e.kind {
        ExprKind::Var(n) => Some(n.upper.clone()),
        ExprKind::Member { name, .. } => Some(name.upper.clone()),
        _ => None,
    });
    match (field, kind) {
        (Some(f), AggKind::Count) => f,
        (Some(f), _) => {
            let prefix = match kind {
                AggKind::Sum => "SUM_",
                AggKind::Avg => "AVG_",
                AggKind::Min => "MIN_",
                AggKind::Max => "MAX_",
                _ => "CNT_",
            };
            let mut name = String::from(prefix);
            name.push_str(&f);
            name.truncate(10);
            name
        }
        (None, AggKind::CountAll) => "CNT".into(),
        (None, _) => format!("EXP_{}", index + 1),
    }
}

/// Every subquery in an expression, in the order they are written, with the span that identifies
/// each one. The span is what ties the cursor a subquery was run into to the place that asks
/// about it, because two identical subqueries in one WHERE clause are still two subqueries.
fn collect_subqueries<'a>(e: &'a Expr, out: &mut Vec<(Span, &'a Query)>) {
    match &e.kind {
        ExprKind::InSubquery { value, query, .. } => {
            collect_subqueries(value, out);
            out.push((e.span, query));
        }
        ExprKind::ExistsSubquery(query) | ExprKind::AnySubquery(query) => out.push((e.span, query)),
        ExprKind::Unary { expr, .. } => collect_subqueries(expr, out),
        ExprKind::Binary { left, right, .. } => {
            collect_subqueries(left, out);
            collect_subqueries(right, out);
        }
        ExprKind::Member { obj, .. } => collect_subqueries(obj, out),
        ExprKind::Index { base, args } => {
            collect_subqueries(base, out);
            for a in args {
                collect_subqueries(a, out);
            }
        }
        ExprKind::Call { args, .. } => {
            for a in args {
                collect_subqueries(&a.expr, out);
            }
        }
        ExprKind::MethodCall { obj, args, .. } => {
            collect_subqueries(obj, out);
            for a in args {
                collect_subqueries(&a.expr, out);
            }
        }
        _ => {}
    }
}

/// The procedure and method names a program defines. A call to one of these is a call to it, even
/// when a built-in of a longer name could be abbreviated to the same thing: `Second()` in a
/// program with a `PROCEDURE Second` is that procedure, not `SECONDS()`.
fn declared_names(p: &Program) -> HashSet<String> {
    let mut out: HashSet<String> = p.procs.iter().map(|d| d.name.upper.clone()).collect();
    for class in &p.classes {
        out.extend(class.procs.iter().map(|d| d.name.upper.clone()));
    }
    collect_declared(&p.body, &mut out);
    for proc in &p.procs {
        collect_declared(&proc.body, &mut out);
    }
    out
}

/// Every name a program declares as a variable or an array, wherever it declares it.
///
/// A declared name wins over a built-in it is only an abbreviation of: `DIMENSION aPrint(1)`
/// followed by `aPrint(1, 1)` reads that array, not APRINTERS(), which the four-letter rule
/// would otherwise find. A name spelled out in full is still the built-in, as it is in VFP.
fn collect_declared(body: &Block, out: &mut HashSet<String>) {
    for stmt in &body.stmts {
        match &stmt.kind {
            StmtKind::Local(decls) | StmtKind::Public(decls) | StmtKind::Private(decls) | StmtKind::Dimension(decls) => {
                out.extend(decls.iter().filter_map(|d| match &d.name {
                    NameRef::Named(n) => Some(n.upper.clone()),
                    NameRef::Computed(_) => None,
                }));
            }
            StmtKind::Parameters(params) | StmtKind::LParameters(params) => {
                out.extend(params.iter().map(|p| p.name.upper.clone()));
            }
            StmtKind::If { then, else_, .. } => {
                collect_declared(then, out);
                if let Some(block) = else_ {
                    collect_declared(block, out);
                }
            }
            StmtKind::DoCase { cases, otherwise } => {
                for (_, block) in cases {
                    collect_declared(block, out);
                }
                if let Some(block) = otherwise {
                    collect_declared(block, out);
                }
            }
            StmtKind::DoWhile { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::ForEach { body, .. }
            | StmtKind::With { body, .. }
            | StmtKind::Scan { body, .. } => collect_declared(body, out),
            StmtKind::Try { body, catches, finally } => {
                collect_declared(body, out);
                for clause in catches {
                    collect_declared(&clause.body, out);
                }
                if let Some(block) = finally {
                    collect_declared(block, out);
                }
            }
            _ => {}
        }
    }
}

/// What an `INTO TABLE` cursor is called before its file is opened. A written-out path names
/// itself; a path worked out when the query runs is only known then, so the cursor takes a name
/// of its own and the open that follows gives the work area the right one.
fn query_table_alias(path: &Expr) -> String {
    let ExprKind::Str(text) = &path.kind else { return "SQLRESULT".to_string() };
    let stem = text.rsplit(['\\', '/']).next().unwrap_or(text);
    let stem = stem.split('.').next().unwrap_or(stem);
    if stem.is_empty() { "SQLRESULT".to_string() } else { stem.to_ascii_uppercase() }
}
