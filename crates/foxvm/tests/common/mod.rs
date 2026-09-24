//! Shared test helpers: an S-expression dump of the AST for readable `assert_eq!` expectations.

#![allow(dead_code)]

use foxvm::ast::*;
use foxvm::diagnostics::Diagnostic;
use foxvm::parser::{ParseOutput, parse_program};

pub fn errors(diags: &[Diagnostic]) -> Vec<String> {
    diags.iter().filter(|d| d.is_error()).map(|d| format!("{}:{}: {}", d.line, d.col, d.message)).collect()
}

pub fn messages(diags: &[Diagnostic]) -> Vec<String> {
    diags.iter().map(|d| d.message.clone()).collect()
}

/// Parses a program and asserts it is diagnostic-free; returns the dump.
pub fn ok(src: &str) -> String {
    let out = parse_program(src);
    assert!(out.diagnostics.is_empty(), "unexpected diagnostics for {src:?}: {:#?}", out.diagnostics);
    dump(&out.program)
}

pub fn dump_output(out: &ParseOutput) -> String {
    dump(&out.program)
}

pub fn dump(p: &Program) -> String {
    let mut lines: Vec<String> = p.body.stmts.iter().map(stmt).collect();
    for proc in &p.procs {
        let kind = match proc.kind {
            ProcKind::Procedure => "procedure",
            ProcKind::Function => "function",
        };
        let params: Vec<&str> = proc.params.iter().map(|p| p.name.upper.as_str()).collect();
        lines.push(format!("({kind} {} ({}))", proc.name.upper, params.join(" ")));
        for s in &proc.body.stmts {
            lines.push(format!("  {}", stmt(s)));
        }
    }
    for class in &p.classes {
        lines.push(format!("(class {} AS {})", class.name.upper, class.parent.upper));
        for p in &class.properties {
            lines.push(format!("  (prop {} {})", p.name.upper, expr(&p.value)));
        }
        for m in &class.members {
            let noinit = if m.noinit { " noinit" } else { "" };
            lines.push(format!("  (object {} AS {}{noinit})", m.name.upper, m.class.upper));
            for p in &m.properties {
                lines.push(format!("    (prop {} {})", p.name.upper, expr(&p.value)));
            }
        }
        for proc in &class.procs {
            let kind = match proc.kind {
                ProcKind::Procedure => "procedure",
                ProcKind::Function => "function",
            };
            let params: Vec<&str> = proc.params.iter().map(|p| p.name.upper.as_str()).collect();
            lines.push(format!("  ({kind} {} ({}))", proc.name.upper, params.join(" ")));
            for s in &proc.body.stmts {
                lines.push(format!("    {}", stmt(s)));
            }
        }
    }
    lines.join("\n")
}

pub fn block(b: &Block) -> String {
    let inner: Vec<String> = b.stmts.iter().map(stmt).collect();
    if inner.is_empty() { "(block)".to_string() } else { format!("(block {})", inner.join(" ")) }
}

/// A name a command was given: written out, or worked out when it runs.
pub fn name_ref(n: &NameRef) -> String {
    match n {
        NameRef::Named(n) => n.upper.clone(),
        NameRef::Computed(e) => format!("({})", expr(e)),
    }
}

fn target(t: &Target) -> String {
    match t {
        Target::Written(e) => expr(e),
        Target::ByName(e) => format!("(by-name {})", expr(e)),
    }
}

fn decls(d: &[VarDecl]) -> String {
    d.iter()
        .map(|v| match &v.dims {
            Some(dims) => format!("{}[{}]", name_ref(&v.name), exprs(dims)),
            None => name_ref(&v.name),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn params(p: &[Param]) -> String {
    p.iter().map(|p| p.name.upper.as_str()).collect::<Vec<_>>().join(" ")
}

fn exprs(e: &[Expr]) -> String {
    e.iter().map(expr).collect::<Vec<_>>().join(", ")
}

pub fn args(a: &[Arg]) -> String {
    a.iter()
        .map(|a| if a.by_ref { format!("@{}", expr(&a.expr)) } else { expr(&a.expr) })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn query(q: &Query) -> String {
    format!(
        "(select{}{} ({}) (from {}){}{}{}{}{})",
        if q.distinct { " distinct" } else { "" },
        q.top.as_ref().map(|t| format!(" top {}", expr(t))).unwrap_or_default(),
        q.columns.iter().map(query_column).collect::<Vec<_>>().join(" "),
        q.from.iter().map(|f| format!("{}:{}", f.table, f.alias.upper)).collect::<Vec<_>>().join(" "),
        q.where_.as_ref().map(|w| format!(" (where {})", expr(w))).unwrap_or_default(),
        if q.group_by.is_empty() { String::new() } else { format!(" (group {})", q.group_by.iter().map(expr).collect::<Vec<_>>().join(" ")) },
        q.union.as_ref().map(|u| format!(" (union{} {})", if u.all { "-all" } else { "" }, query(&u.query))).unwrap_or_default(),
        if q.order_by.is_empty() { String::new() } else { format!(" (order {})", q.order_by.iter().map(|o| format!("{}{}", expr(&o.expr), if o.descending { " desc" } else { "" })).collect::<Vec<_>>().join(" ")) },
        match &q.into {
            QueryInto::Browse => String::new(),
            QueryInto::Cursor(n) => format!(" (into-cursor {})", name_ref(n)),
            QueryInto::Table(e) => format!(" (into-table {})", expr(e)),
            QueryInto::Array(e) => format!(" (into-array {})", expr(e)),
        },
    )
}

pub fn stmt(s: &Stmt) -> String {
    match &s.kind {
        StmtKind::Assign { target, value } => format!("(= {} {})", expr(target), expr(value)),
        StmtKind::Store { value, targets } => format!(
            "(store {} {})",
            expr(value),
            targets.iter().map(target).collect::<Vec<_>>().join(", ")
        ),
        StmtKind::Local(d) => format!("(local {})", decls(d)),
        StmtKind::Private(d) => format!("(private {})", decls(d)),
        StmtKind::Public(d) => format!("(public {})", decls(d)),
        StmtKind::Dimension(d) => format!("(dimension {})", decls(d)),
        StmtKind::Parameters(p) => format!("(parameters {})", params(p)),
        StmtKind::LParameters(p) => format!("(lparameters {})", params(p)),
        StmtKind::If { cond, then, else_ } => match else_ {
            Some(e) => format!("(if {} {} {})", expr(cond), block(then), block(e)),
            None => format!("(if {} {})", expr(cond), block(then)),
        },
        StmtKind::DoCase { cases, otherwise } => {
            let mut parts: Vec<String> =
                cases.iter().map(|(c, b)| format!("(case {} {})", expr(c), block(b))).collect();
            if let Some(o) = otherwise {
                parts.push(format!("(otherwise {})", block(o)));
            }
            format!("(docase {})", parts.join(" "))
        }
        StmtKind::DoWhile { cond, body } => format!("(while {} {})", expr(cond), block(body)),
        StmtKind::For { var, from, to, step, body } => match step {
            Some(st) => format!("(for {} {} {} {} {})", var.upper, expr(from), expr(to), expr(st), block(body)),
            None => format!("(for {} {} {} {})", var.upper, expr(from), expr(to), block(body)),
        },
        StmtKind::ForEach { var, collection, body } => {
            format!("(foreach {} {} {})", var.upper, expr(collection), block(body))
        }
        StmtKind::Exit => "(exit)".into(),
        StmtKind::Loop => "(loop)".into(),
        StmtKind::Return(None) => "(return)".into(),
        StmtKind::Return(Some(e)) => format!("(return {})", expr(e)),
        StmtKind::DoExpr { name, args: a, in_prog } => {
            let mut s = format!("(do-expr {}", expr(name));
            if !a.is_empty() {
                s.push_str(&format!(" (with {})", args(a)));
            }
            if let Some(p) = in_prog {
                s.push_str(&format!(" (in {})", expr(p)));
            }
            s + ")"
        }
        StmtKind::Do { name, args: a, in_prog } => {
            let mut s = format!("(do {}", name.text);
            if !a.is_empty() {
                s.push_str(&format!(" (with {})", args(a)));
            }
            if let Some(p) = in_prog {
                s.push_str(&format!(" (in {})", expr(p)));
            }
            s + ")"
        }
        StmtKind::DoForm { name, name_var, linked, args: a, to_var, noshow } => {
            let mut s = format!("(doform {}", expr(name));
            if let Some(n) = name_var {
                s.push_str(&format!(" (name {})", target(n)));
            }
            if *linked {
                s.push_str(" linked");
            }
            if !a.is_empty() {
                s.push_str(&format!(" (with {})", args(a)));
            }
            if let Some(t) = to_var {
                s.push_str(&format!(" (to {})", t.upper));
            }
            if *noshow {
                s.push_str(" noshow");
            }
            s + ")"
        }
        StmtKind::ExprStmt(e) => format!("(expr {})", expr(e)),
        StmtKind::Print { items, newline } => {
            let head = if *newline { "?" } else { "??" };
            if items.is_empty() { format!("({head})") } else { format!("({head} {})", exprs(items)) }
        }
        StmtKind::WaitWindow { text, nowait, timeout, clear, to_var } => {
            let mut s = String::from("(wait");
            if let Some(t) = text {
                s.push_str(&format!(" {}", expr(t)));
            }
            if *nowait {
                s.push_str(" nowait");
            }
            if let Some(t) = timeout {
                s.push_str(&format!(" (timeout {})", expr(t)));
            }
            if *clear {
                s.push_str(" clear");
            }
            if let Some(v) = to_var {
                s.push_str(&format!(" (to {})", v.upper));
            }
            s + ")"
        }
        StmtKind::ReadEvents => "(read-events)".into(),
        StmtKind::ClearEvents => "(clear-events)".into(),
        StmtKind::Release(items) => format!("(release {})", exprs(items)),
        StmtKind::ReleaseAll => "(release-all)".into(),
        StmtKind::Quit => "(quit)".into(),
        StmtKind::Cancel => "(cancel)".into(),
        StmtKind::With { obj, body } => format!("(with {} {})", expr(obj), block(body)),
        StmtKind::Set { setting, value } => {
            let v = match value {
                SetValue::On => "on".to_string(),
                SetValue::Off => "off".to_string(),
                SetValue::Switch { on, words } => format!("{} {words}", if *on { "on" } else { "off" }),
                SetValue::To(e) => format!("(to {})", exprs(e)),
                SetValue::Word(w) => format!("(word \"{w}\")"),
            };
            format!("(set {} {})", setting.upper, v)
        }
        StmtKind::Try { body, catches, finally } => {
            let mut s = format!("(try {}", block(body));
            for c in catches {
                s.push_str(" (catch");
                if let Some(v) = &c.var {
                    s.push_str(&format!(" (to {})", v.upper));
                }
                if let Some(w) = &c.when {
                    s.push_str(&format!(" (when {})", expr(w)));
                }
                s.push_str(&format!(" {})", block(&c.body)));
            }
            if let Some(f) = finally {
                s.push_str(&format!(" (finally {})", block(f)));
            }
            s + ")"
        }
        StmtKind::Unsupported(what) => format!("(unsupported {what:?})"),
        StmtKind::Erase(p) => format!("(erase {})", expr(p)),
        StmtKind::Use { table, alias, exclusive, in_area, order, .. } => {
            let mut s = format!("(use {})", table.as_ref().map(expr).unwrap_or_else(|| "-".into()));
            s.pop();
            if let Some(a) = alias {
                s.push_str(&format!(" (alias {})", name_ref(a)));
            }
            if *exclusive {
                s.push_str(" exclusive");
            }
            if let Some(a) = in_area {
                s.push_str(&format!(" (in {})", expr(a)));
            }
            if let Some(o) = order {
                s.push_str(&format!(" (order {})", expr(o)));
            }
            s + ")"
        }
        StmtKind::SetOrder { tag, descending, area } => {
            let mut s = format!("(set-order {})", tag.as_ref().map(expr).unwrap_or_else(|| "-".into()));
            s.pop();
            if let Some(a) = area {
                s.push_str(&format!(" in {}", name_ref(a)));
            }
            if let Some(true) = descending {
                s.push_str(" descending");
            }
            s + ")"
        }
        StmtKind::Seek { key, order, .. } => match order {
            Some(o) => format!("(seek {} (order {}))", expr(key), expr(o)),
            None => format!("(seek {})", expr(key)),
        },
        StmtKind::IndexOn { key, tag, cond, unique, candidate, descending, .. } => {
            let mut s = format!("(index-on {} (tag {})", expr(key), tag.upper);
            if let Some(c) = cond {
                s.push_str(&format!(" (for {})", expr(c)));
            }
            for (on, word) in [(*unique, "unique"), (*candidate, "candidate"), (*descending, "descending")] {
                if on {
                    s.push(' ');
                    s.push_str(word);
                }
            }
            s + ")"
        }
        StmtKind::CopyIndexes { files, all, target } => {
            let what = if *all { "all".to_string() } else { exprs(files) };
            match target {
                Some(t) => format!("(copy-indexes {what} (to {}))", expr(t)),
                None => format!("(copy-indexes {what})"),
            }
        }
        StmtKind::CopyTag { tag, of, target } => match of {
            Some(o) => format!("(copy-tag {} (of {}) (to {}))", expr(tag), expr(o), expr(target)),
            None => format!("(copy-tag {} (to {}))", expr(tag), expr(target)),
        },
        StmtKind::Reindex => "(reindex)".into(),
        StmtKind::Pack => "(pack)".into(),
        StmtKind::SetFilter(text) => format!("(set-filter {text})"),
        StmtKind::SetRelation { pairs, .. } => format!("(set-relation {})", pairs.len()),
        StmtKind::Unlock { record, area, all } => format!(
            "(unlock{}{}{})",
            record.as_ref().map(|r| format!(" record {}", expr(r))).unwrap_or_default(),
            area.as_ref().map(|a| format!(" in {}", name_ref(a))).unwrap_or_default(),
            if *all { " all" } else { "" }
        ),
        StmtKind::Transaction(step) => format!("(transaction {step:?})").to_lowercase(),
        StmtKind::CopyTo { path, kind, .. } => format!("(copy-to {} {:?})", expr(path), kind).to_lowercase(),
        StmtKind::AppendFrom { path, .. } => format!("(append-from {})", expr(path)),
        StmtKind::DropTable { path, .. } => format!("(drop-table {})", expr(path)),
        StmtKind::AlterTable { path, ops } => format!(
            "(alter-table {} {})",
            expr(path),
            ops.iter()
                .map(|op| match op {
                    AlterOp::Add(f) => format!("(add ({} {}({},{})))", name_ref(&f.name), f.kind, f.width, f.decimals),
                    AlterOp::Alter(f) => format!("(alter ({} {}({},{})))", name_ref(&f.name), f.kind, f.width, f.decimals),
                    AlterOp::Drop(n) => format!("(drop {})", name_ref(n)),
                    AlterOp::Rename(a, b) => format!("(rename {} {})", name_ref(a), name_ref(b)),
                })
                .collect::<Vec<_>>()
                .join(" ")
        ),
        StmtKind::Aggregate { .. } => "(aggregate)".into(),
        StmtKind::Scatter { .. } => "(scatter)".into(),
        StmtKind::Gather { .. } => "(gather)".into(),
        StmtKind::Database { what, .. } => format!("(database {what:?})").to_lowercase(),
        StmtKind::DeleteTag { tag } => match tag {
            Some(t) => format!("(delete-tag {})", t.upper),
            None => "(delete-tag all)".into(),
        },
        StmtKind::SelectArea(SelectTarget::Alias(n)) => format!("(select {})", n.upper),
        StmtKind::SelectArea(SelectTarget::Number(e)) => format!("(select {})", expr(e)),
        StmtKind::Go { where_: GoWhere::Top, .. } => "(go top)".into(),
        StmtKind::Go { where_: GoWhere::Bottom, .. } => "(go bottom)".into(),
        StmtKind::Go { where_: GoWhere::Record(e), .. } => format!("(go {})", expr(e)),
        StmtKind::Skip { count: None, .. } => "(skip)".into(),
        StmtKind::Skip { count: Some(e), .. } => format!("(skip {})", expr(e)),
        StmtKind::CloseTables { all } => format!("(close {})", if *all { "all" } else { "tables" }),
        StmtKind::Retry => "(retry)".into(),
        StmtKind::Scan { scope, cond, while_, body } => format!(
            "(scan {}{}{} {})",
            scope_text(scope),
            cond.as_ref().map(|c| format!(" for {}", expr(c))).unwrap_or_default(),
            while_.as_ref().map(|c| format!(" while {}", expr(c))).unwrap_or_default(),
            block(body)
        ),
        StmtKind::Locate { scope, cond, while_ } => format!(
            "(locate {}{}{})",
            scope_text(scope),
            cond.as_ref().map(|c| format!(" for {}", expr(c))).unwrap_or_default(),
            while_.as_ref().map(|c| format!(" while {}", expr(c))).unwrap_or_default()
        ),
        StmtKind::Continue => "(continue)".into(),
        StmtKind::Replace { area, assignments, scope, cond, .. } => format!(
            "(replace{} {} {}{})",
            area.as_ref().map(|a| format!(" in {}", name_ref(a))).unwrap_or_default(),
            scope_text(scope),
            assignments
                .iter()
                .map(|(f, e, _)| format!("({} {})", name_ref(f), expr(e)))
                .collect::<Vec<_>>()
                .join(" "),
            cond.as_ref().map(|c| format!(" for {}", expr(c))).unwrap_or_default()
        ),
        StmtKind::MarkDeleted { deleted, scope, cond, .. } => format!(
            "({} {}{})",
            if *deleted { "delete" } else { "recall" },
            scope_text(scope),
            cond.as_ref().map(|c| format!(" for {}", expr(c))).unwrap_or_default()
        ),
        StmtKind::Modify { what, path, .. } => format!("(modify {} {})", what.upper, expr(path)),
        StmtKind::Browse { .. } => "(browse)".into(),
        StmtKind::CreateCursor { alias, fields, .. } => format!(
            "(create-cursor {} {})",
            alias.upper,
            fields.iter().map(|f| format!("({} {} {})", name_ref(&f.name), f.kind, f.width)).collect::<Vec<_>>().join(" ")
        ),
        StmtKind::InsertBlank { before } => {
            format!("(insert-blank{})", if *before { " before" } else { "" })
        }
        StmtKind::Insert { alias, named, fields, source } => format!(
            "(insert {}{} {})",
            named.as_ref().map_or_else(|| alias.upper.clone(), expr),
            if fields.is_empty() {
                String::new()
            } else {
                format!(
                    " ({})",
                    fields
                        .iter()
                        .map(name_ref)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            },
            match source {
                InsertSource::Values(values) => values.iter().map(expr).collect::<Vec<_>>().join(" "),
                InsertSource::From(ScatterWhere::Memvar) => "from-memvar".into(),
                InsertSource::From(ScatterWhere::Array(e)) => format!("from-array {}", expr(e)),
                InsertSource::From(ScatterWhere::Name(e)) => format!("from-name {}", expr(e)),
                InsertSource::Query(q) => query(q),
            }
        ),
        StmtKind::ClearAll { tables } => format!("(clear {})", if *tables { "all" } else { "memory" }),
        StmtKind::AppendBlank { area } => format!(
            "(append{})",
            area.as_ref().map(|a| format!(" in {}", name_ref(a))).unwrap_or_default()
        ),
        StmtKind::RaiseError { what, message } => format!("(error {}{})", expr(what), message.as_ref().map(|m| format!(" {}", expr(m))).unwrap_or_default()),
        StmtKind::CreateTable { path, fields, from_array, .. } => format!(
            "(create-table {} ({}))",
            expr(path),
            match from_array {
                Some(e) => format!("from-array {}", expr(e)),
                None => fields
                    .iter()
                    .map(|f| format!("{} {}({},{})", name_ref(&f.name), f.kind, f.width, f.decimals))
                    .collect::<Vec<_>>()
                    .join(" "),
            }
        ),

        StmtKind::DeclareDll { returns, function, library, alias, params } => format!(
            "(declare-dll {} {} {}{} ({}))",
            returns.as_ref().map(|r| r.upper.clone()).unwrap_or_else(|| "-".into()),
            function.upper,
            expr(library),
            alias.as_ref().map(|a| format!(" as {}", a.upper)).unwrap_or_default(),
            params.iter().map(|p| format!("{}{}", p.kind.upper, if p.by_ref { "@" } else { "" })).collect::<Vec<_>>().join(" ")
        ),
        StmtKind::FileCommand { kind, path, target } => format!(
            "(file-{} {}{})",
            kind,
            expr(path),
            target.as_ref().map(|t| format!(" {}", expr(t))).unwrap_or_default()
        ),
        StmtKind::Zap { area } => format!(
            "(zap{})",
            area.as_ref().map(|a| format!(" in {}", name_ref(a))).unwrap_or_default()
        ),
        StmtKind::Query(q) => query(q),
        StmtKind::Throw(None) => "(throw)".into(),
        StmtKind::Throw(Some(e)) => format!("(throw {})", expr(e)),
        StmtKind::NoDefault => "(nodefault)".into(),
        StmtKind::DoDefault(a) => format!("(dodefault {})", args(a)),
        StmtKind::MacroText(text) => format!("(macro-text {text:?})"),
        StmtKind::Text { target, additive, textmerge, noshow, raw } => {
            let mut s = String::from("(text");
            if let Some(t) = target {
                s.push_str(&format!(" (to {})", expr(t)));
            }
            if *additive {
                s.push_str(" additive");
            }
            if *textmerge {
                s.push_str(" textmerge");
            }
            if *noshow {
                s.push_str(" noshow");
            }
            format!("{s} {raw:?})")
        }
        StmtKind::TextLine { newline, raw } => {
            format!("(text-line {} {raw:?})", if *newline { "\\" } else { "\\\\" })
        }
        StmtKind::OnError(None) => "(onerror)".into(),
        StmtKind::OnError(Some(c)) => format!("(onerror {c:?})"),
        StmtKind::OnKeyLabel { key, command } => match command {
            Some(c) => format!("(onkey {key:?} {c:?})"),
            None => format!("(onkey {key:?})"),
        },
        StmtKind::Directive(d) => format!("(directive {d:?})"),
        // statements no parse test writes back out yet
        StmtKind::ListInfo { .. } => "(list)".into(),
        StmtKind::MenuCommand { what, .. } => format!("(menu {what:?})"),
        StmtKind::WindowCommand { what, .. } => format!("(window {what:?})"),
        StmtKind::ReportForm { label, .. } => format!("(report {})", if *label { "label" } else { "form" }),
        StmtKind::Eject => "(eject)".into(),
        StmtKind::Ask { kind, .. } => format!("(ask {kind})"),
        StmtKind::Run { .. } => "(run)".into(),
        StmtKind::Debug(verb) => format!("(debug {verb:?})"),
        StmtKind::Diagnostic { cond, .. } => format!("({})", if cond.is_some() { "assert" } else { "debugout" }),
        StmtKind::Yield { events } => format!("({})", if *events { "doevents" } else { "flush" }),
        StmtKind::Blank { .. } => "(blank)".into(),
        StmtKind::Mouse { clicks, .. } => format!("(mouse {clicks})"),
        StmtKind::Variables { save, .. } => format!("({})", if *save { "save" } else { "restore" }),
        StmtKind::Memo { what, .. } => format!("(memo {what})"),
        StmtKind::Import { path, sheet } => match sheet {
            Some(e) => format!("(import {} {})", expr(path), expr(e)),
            None => format!("(import {})", expr(path)),
        },
        StmtKind::CreateFrom { target, source } => format!("(create-from {} {})", expr(target), expr(source)),
        StmtKind::Build { what, target, from, recompile } => format!(
            "(build {} {}{}{})",
            what.upper,
            expr(target),
            if from.is_empty() { String::new() } else { format!(" (from {})", exprs(from)) },
            if *recompile { " recompile" } else { "" }
        ),
        StmtKind::Compile { what, files, flags } => {
            format!("(compile {} {} {})", what.upper, expr(files), flags)
        }
        StmtKind::NewDocument { what, path } => format!("(new {} {})", what.upper, expr(path)),
        StmtKind::OnEvent { what, command } => match command {
            Some(c) => format!("(on {what:?} {c:?})"),
            None => format!("(on {what:?})"),
        },
        StmtKind::Keyboard { .. } => "(keyboard)".into(),
        StmtKind::KeyStack { push, .. } => format!("(keystack {})", if *push { "push" } else { "pop" }),
        StmtKind::AtCommand(parts) => format!("(at {:?})", parts.iter().map(|p| p.what).collect::<Vec<_>>()),
        StmtKind::UpdateSql { .. } => "(update)".into(),
        StmtKind::DeleteSql { .. } => "(delete-sql)".into(),
        StmtKind::ReplaceFromArray { source, .. } => format!("(replace-from-array {})", expr(source)),
        StmtKind::AppendFromArray(source) => format!("(append-from-array {})", expr(source)),
        StmtKind::ReturnTo(to) => format!("(return-to {})", to.as_ref().map_or("master", |n| n.upper.as_str())),
    }
}

pub fn expr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::MemberByName { obj, name, args: a } => match a {
            Some(a) => format!("({}.&{}({}))", expr(obj), expr(name), args(a)),
            None => format!("({}.&{})", expr(obj), expr(name)),
        },
        ExprKind::InSubquery { value, negated, .. } => {
            format!("({}in {} (select ...))", if *negated { "not " } else { "" }, expr(value))
        }
        ExprKind::ExistsSubquery(_) => "(exists (select ...))".into(),
        ExprKind::AnySubquery(_) => "(any (select ...))".into(),
        ExprKind::Money(c) => format!("${:.4}", *c as f64 / 10_000.0),
        ExprKind::Num(n, ..) => format!("{n}"),
        ExprKind::Str(s) => format!("{s:?}"),
        ExprKind::Bool(true) => ".T.".into(),
        ExprKind::Bool(false) => ".F.".into(),
        ExprKind::Null => ".NULL.".into(),
        ExprKind::Date(None) => "{}".into(),
        ExprKind::Date(Some(d)) => format!("{{^{:04}-{:02}-{:02}}}", d.year, d.month, d.day),
        ExprKind::DateTime(None) => "{/:}".into(),
        ExprKind::DateTime(Some((d, t))) => {
            format!("{{^{:04}-{:02}-{:02} {:02}:{:02}:{:02}}}", d.year, d.month, d.day, t.hour, t.minute, t.second)
        }
        ExprKind::Omitted => "_".into(),
        ExprKind::CallValue { target, args: a } => format!("{}({})", expr(target), args(a)),
        ExprKind::Lambda { params, body, .. } => {
            let names: Vec<&str> = params.iter().map(|p| p.name.upper.as_str()).collect();
            format!("(lambda ({}) {})", names.join(" "), block(body))
        }
        ExprKind::Var(n) => n.upper.clone(),
        ExprKind::This => "THIS".into(),
        ExprKind::ThisForm => "THISFORM".into(),
        ExprKind::ThisFormSet => "THISFORMSET".into(),
        ExprKind::Screen => "_SCREEN".into(),
        ExprKind::WithRef => "<with>".into(),
        ExprKind::Member { obj, name } => format!("{}.{}", expr(obj), name.upper),
        ExprKind::Call { name, args: a } => format!("{}({})", name.upper, args(a)),
        ExprKind::Index { base, args: a } => format!("{}[{}]", expr(base), exprs(a)),
        ExprKind::MethodCall { obj, name, args: a } => format!("{}.{}({})", expr(obj), name.upper, args(a)),
        ExprKind::Unary { op, expr: inner } => {
            let o = match op {
                UnOp::Neg => "-",
                UnOp::Plus => "+",
                UnOp::Not => "!",
            };
            format!("({o} {})", expr(inner))
        }
        ExprKind::Binary { op, left, right } => {
            let o = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Pow => "^",
                BinOp::Eq => "=",
                BinOp::ExactEq => "==",
                BinOp::Ne => "<>",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::Contains => "$",
                BinOp::And => "AND",
                BinOp::Or => "OR",
            };
            format!("({o} {} {})", expr(left), expr(right))
        }
        ExprKind::Macro(operand) => format!("&{}", expr(operand)),
    }
}

fn scope_text(s: &Scope) -> String {
    match s {
        Scope::All => "all".into(),
        Scope::Rest => "rest".into(),
        Scope::Next(e) => format!("next {}", expr(e)),
        Scope::Record(e) => format!("record {}", expr(e)),
        Scope::Current => "current".into(),
    }
}

fn query_column(c: &QueryColumn) -> String {
    match c {
        QueryColumn::All(None) => "*".into(),
        QueryColumn::All(Some(a)) => format!("{}.*", a.upper),
        QueryColumn::Value { expr: e, name } => match name {
            Some(n) => format!("{} as {}", expr(e), n.upper),
            None => expr(e),
        },
    }
}
