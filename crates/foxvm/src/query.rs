//! `SELECT-SQL`: the plan a query is compiled to, and what turns collected rows into a result.
//!
//! The query itself is compiled to ordinary bytecode - nested loops over the work areas, with the
//! WHERE clause as an ordinary test - because everything those loops need already suspends and
//! resumes correctly against a host that hands out bytes. What cannot be expressed that way is
//! what happens *after* the rows are gathered: grouping, aggregates, DISTINCT, ORDER BY and TOP.
//! That is this module, and the plan below is the compiler's instructions for it.

use std::cmp::Ordering;

use crate::dbf::{DbfField, DbfRecord, DbfValue};
use crate::error::RtError;
use crate::value::{self, CmpOp, Settings, Value};

/// Everything about a query that is decided when it is compiled.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QueryPlan {
    pub columns: Vec<PlanColumn>,
    pub distinct: bool,
    /// How many values are pushed after the column values as GROUP BY keys.
    pub group_keys: u16,
    /// One entry per ORDER BY term, in the order they are written.
    pub order_by: Vec<OrderTerm>,
    /// True when a `TOP n` count is pushed before the query starts.
    pub has_top: bool,
    /// `TOP n PERCENT`: the number is a share of the result rows rather than a count of them.
    pub top_percent: bool,
    /// What each `SqlHavingValue` reads out of the row the HAVING clause is being asked about,
    /// in the order the compiled predicate refers to them.
    pub having: Vec<HavingRef>,
    /// Plan columns on the end that only the HAVING clause asked for. They are gathered and
    /// folded like any other and then dropped, because the result is what the select list says.
    pub hidden: u16,
    pub into: PlanInto,
    /// `INTO CURSOR (cName)`: the cursor is named by an expression pushed before the `TOP`
    /// count, so what the result is called is only known when the query runs.
    pub into_named: bool,
    /// Aliases of the cursors this query's subqueries were run into, to let go of afterwards.
    pub temporaries: Vec<String>,
}

/// One column of the result.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PlanColumn {
    /// `*` or `alias.*`: every field of every source, or of the one named. What that comes to is
    /// only known when the query runs, so the names are worked out from the open cursors.
    Star(Option<String>),
    /// An expression, whose value is pushed per row.
    Value { name: String },
    /// An aggregate over the pushed value, or over the rows themselves for `COUNT(*)`.
    Agg { kind: AggKind, name: String },
}

/// One ORDER BY term: which way round it sorts, and what it sorts on.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrderTerm {
    pub descending: bool,
    pub key: OrderKey,
}

/// What one ORDER BY term sorts on.
///
/// Measured in Visual FoxPro 9: an ORDER BY term is a result column - its number, or the name
/// the select list gave it - or a column that can be read from the record. Only the second can
/// be worked out while the rows are being gathered, and over a query that folds its rows it is
/// the first that has to be: `ORDER BY 1` over a select list whose first column is `SUM(x)`
/// sorts the groups by their sums, and there is no such thing to read from a record.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum OrderKey {
    /// Evaluated per row and pushed after the group keys, in the order the terms are written.
    Pushed,
    /// The nth result column, counting from zero - `ORDER BY n` counts from one.
    Result(u16),
    /// The plan column at this index, which is where a name the select list gave with AS leads.
    /// It is not the same as `Result` when a `*` before it stands for more than one column.
    Column(u16),
}

/// One thing a HAVING clause names, and where it sits in a row that has already been folded.
///
/// Measured in Visual FoxPro 9: with a GROUP BY, a HAVING clause takes a grouped column, an
/// aggregate (whether or not the select list has that aggregate too), or a name given by AS in
/// the select list. A column that is none of those is error 1803, "SQL: HAVING clause is
/// invalid.", even when the select list holds it - unless SET ENGINEBEHAVIOR 70 is in force,
/// which is the older rules, where it is read from the group's last record instead.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HavingRef {
    /// The plan column at this index, which for an aggregate is the folded value of the group.
    Column(u16),
    /// The GROUP BY key at this index.
    Key(u16),
    /// A name that is not one of the three: a column of a source that is neither grouped nor
    /// inside an aggregate, or an AS alias that a field of one of the sources is called too -
    /// a field beats an alias in Visual FoxPro, so `SELECT city AS amt ... HAVING amt` over a
    /// table with an `amt` field tests the field, not `city`.
    ///
    /// `index` is a plan column gathered for it and dropped again, holding whatever the name
    /// means to a record. `alias` is the select-list column to read instead when no source has
    /// a field of that name at all, which is what makes an unshadowed alias work.
    Ungrouped { name: String, index: u16, alias: Option<u16> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AggKind {
    /// `COUNT(*)`: rows in the group. Nothing is pushed for it.
    CountAll,
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl AggKind {
    /// Whether the column pushes a value per row. Only `COUNT(*)` does not.
    pub fn takes_value(self) -> bool {
        !matches!(self, AggKind::CountAll)
    }

    pub fn from_name(upper: &str) -> Option<AggKind> {
        match upper {
            "COUNT" => Some(AggKind::Count),
            "SUM" => Some(AggKind::Sum),
            "AVG" => Some(AggKind::Avg),
            "MIN" => Some(AggKind::Min),
            "MAX" => Some(AggKind::Max),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PlanInto {
    Cursor(String),
    /// The rows come back as an array on the stack; the code after the query stores it wherever
    /// the INTO ARRAY clause named, which may be a property rather than a variable.
    Array,
}

impl QueryPlan {
    /// True when the result is one row per group rather than one per record.
    pub fn grouped(&self) -> bool {
        self.group_keys > 0 || self.columns.iter().any(|c| matches!(c, PlanColumn::Agg { .. }))
    }

    /// How many values the compiled code pushes for the columns, before the keys.
    pub fn pushed_columns(&self) -> usize {
        self.columns
            .iter()
            .filter(|c| match c {
                PlanColumn::Star(_) => false,
                PlanColumn::Value { .. } => true,
                PlanColumn::Agg { kind, .. } => kind.takes_value(),
            })
            .count()
    }
}

/// A query part-way through: the rows gathered so far, and what to do with them.
#[derive(Debug)]
pub struct QueryRun {
    pub plan: std::rc::Rc<QueryPlan>,
    /// The name of each result column, with every `*` already expanded.
    pub columns: Vec<String>,
    /// How many result columns each plan column stands for, in the same order.
    pub widths: Vec<usize>,
    /// One entry per row: the column values, then the group keys, then the order keys.
    pub rows: Vec<Vec<Value>>,
    /// Whether `rows` has already been folded into one row per group, which it has when a
    /// HAVING clause asked about the groups before the query finished.
    pub folded: bool,
    /// Where the HAVING clause has got to, once it has started.
    pub having: Option<HavingState>,
    /// `TOP n`, once the expression has been evaluated. It is kept as written rather than as a
    /// count of rows because `TOP n PERCENT` only becomes a count when the rows are all in.
    pub top: Option<f64>,
    /// The work areas the query opened, and the record each reused area was sitting on.
    pub sources: Vec<QuerySourceState>,
    /// The work area that was selected before the query started.
    pub saved_area: usize,
    /// How deep the frames were when the query started. An error caught deeper than this was
    /// caught inside something the query called, and the query carries on; one caught here or
    /// shallower has abandoned the query, and its sources have to go back.
    pub frame: usize,
    /// What `INTO CURSOR (cName)` came to, when the plan said the name would be worked out.
    pub into_alias: Option<String>,
}

/// A HAVING clause part-way through: the groups it has kept, and which one it is asking about.
///
/// The predicate is ordinary bytecode - it may call a function of the program's own - so it is
/// run a group at a time between gathering the rows and finishing the query, rather than from
/// inside the fold.
#[derive(Debug, Default)]
pub struct HavingState {
    /// The rows the predicate has said yes to.
    pub kept: Vec<Vec<Value>>,
    /// The row the predicate is about to be, or is being, asked about.
    pub at: usize,
    /// True while the predicate is running, which is when no record is current: a field read
    /// there is a column the clause may not name, and the product's 1803.
    pub testing: bool,
}

#[derive(Debug)]
pub struct QuerySourceState {
    pub alias: String,
    /// 1-based work area.
    pub area: usize,
    /// The record the area was on when the query borrowed it, for a table it did not open.
    pub restore: Option<u64>,
}

impl QueryRun {
    /// The column values of a gathered row, without the trailing keys.
    fn values<'a>(&self, row: &'a [Value]) -> &'a [Value] {
        &row[..self.columns.len().min(row.len())]
    }

    fn group_key<'a>(&self, row: &'a [Value]) -> &'a [Value] {
        let start = self.columns.len();
        &row[start.min(row.len())..(start + self.plan.group_keys as usize).min(row.len())]
    }

    /// Where the values of one plan column sit in a row.
    pub fn column_slot(&self, index: u16) -> usize {
        self.widths.iter().take(index as usize).sum()
    }

    /// Where one GROUP BY key sits in a row.
    pub fn key_slot(&self, which: u16) -> usize {
        self.columns.len() + which as usize
    }

    /// Folds the gathered rows into one row per group, which is what a HAVING clause asks about.
    /// It happens once: the query itself does not fold again afterwards.
    pub fn fold_for_having(&mut self, settings: &Settings) -> Result<(), RtError> {
        if !self.folded {
            let groups = self.fold_groups(settings)?;
            self.rows = groups;
            self.folded = true;
        }
        Ok(())
    }

    /// Folds, sorts and trims the gathered rows into the result.
    pub fn finish(mut self, settings: &Settings) -> Result<(Vec<DbfField>, Vec<DbfRecord>), RtError> {
        if !self.folded && self.plan.grouped() {
            let groups = self.fold_groups(settings)?;
            self.rows = groups;
            self.folded = true;
        }
        self.drop_hidden();
        if self.plan.distinct {
            self.dedup(settings);
        }
        self.sort(settings)?;
        if let Some(n) = self.top {
            let keep = self.top_rows(n);
            self.rows.truncate(keep);
        }

        let widths = column_widths(&self.rows, self.columns.len());
        let fields = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, name)| field_for(name, self.rows.iter().map(|r| &r[i]), widths[i]))
            .collect();
        let rows = self
            .rows
            .iter()
            .map(|r| DbfRecord { deleted: false, values: self.values(r).iter().map(as_dbf).collect() })
            .collect();
        Ok((fields, rows))
    }

    /// How many rows `TOP` keeps of the result as it now stands.
    ///
    /// `TOP n` is the count itself. `TOP n PERCENT` is that share of the rows rounded *up*,
    /// measured in Visual FoxPro 9: half of seven rows is four, and a tenth of seven - seven
    /// tenths of a row - is one, so no percentage of a table with anything in it comes to
    /// nothing. The arithmetic is `rows * n / 100` in that order, which is what makes
    /// `TOP 99.9 PERCENT` of a thousand rows a thousand rather than the 999 it reads as: 99.9
    /// is a little over 99.9 as a binary fraction, and the product rounds that up too.
    ///
    /// The share is of the rows the query has after grouping and DISTINCT, not of the records
    /// read - measured: four groups, `TOP 50 PERCENT`, two rows.
    fn top_rows(&self, n: f64) -> usize {
        if !self.plan.top_percent {
            return n.max(0.0) as usize;
        }
        (self.rows.len() as f64 * n / 100.0).ceil().max(0.0) as usize
    }

    /// Lets go of the columns only the HAVING clause asked for. They sit on the end of the
    /// column block, so the rows and the names lose the same slots.
    fn drop_hidden(&mut self) {
        let hidden = self.plan.hidden as usize;
        if hidden == 0 {
            return;
        }
        let start = self.columns.len().saturating_sub(hidden);
        for row in &mut self.rows {
            let end = (start + hidden).min(row.len());
            row.drain(start.min(end)..end);
        }
        self.columns.truncate(start);
        self.widths.truncate(self.widths.len().saturating_sub(hidden));
    }

    /// One row per distinct group key, with the aggregate columns folded over its rows.
    fn fold_groups(&self, settings: &Settings) -> Result<Vec<Vec<Value>>, RtError> {
        let mut groups: Vec<(Vec<Value>, Vec<usize>)> = Vec::new();
        for (i, row) in self.rows.iter().enumerate() {
            let key = self.group_key(row).to_vec();
            match groups.iter_mut().find(|(k, _)| same(k, &key, settings)) {
                Some((_, members)) => members.push(i),
                None => groups.push((key, vec![i])),
            }
        }
        // Without an ORDER BY the groups come out in order of their key, not in order of the
        // record each was first seen on - measured in Visual FoxPro 9: rows inserted zz, aa,
        // mm and grouped by that column come back aa, mm, zz, whatever ENGINEBEHAVIOR says,
        // and grouping by two columns orders by both. An ORDER BY is a stable sort over this,
        // so it is also what decides ties the ORDER BY leaves.
        groups.sort_by(|(a, _), (b, _)| {
            for (x, y) in a.iter().zip(b) {
                let ordering = compare(x, y, settings);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            Ordering::Equal
        });

        let mut out = Vec::with_capacity(groups.len());
        for (_, members) in groups {
            // A column that is not an aggregate and not one of the keys still has to come from
            // somewhere, and measured in Visual FoxPro 9 it comes from the *last* record of the
            // group: rows (A,USA) and (A,CAN) grouped by the first column answer CAN. It is the
            // same value SET ENGINEBEHAVIOR 70 lets a HAVING clause ask about.
            let last = &self.rows[members[members.len() - 1]];
            let mut row = last.clone();
            let mut at = 0usize;
            for (column, width) in self.plan.columns.iter().zip(&self.widths) {
                if let PlanColumn::Agg { kind, .. } = column {
                    row[at] = fold(*kind, members.iter().map(|&i| &self.rows[i][at]), settings)?;
                }
                at += width;
            }
            out.push(row);
        }
        Ok(out)
    }

    fn dedup(&mut self, settings: &Settings) {
        let mut seen: Vec<Vec<Value>> = Vec::new();
        let mut out = Vec::with_capacity(self.rows.len());
        for row in std::mem::take(&mut self.rows) {
            let values = self.values(&row).to_vec();
            if seen.iter().any(|s| same(s, &values, settings)) {
                continue;
            }
            seen.push(values);
            out.push(row);
        }
        self.rows = out;
    }

    fn sort(&mut self, settings: &Settings) -> Result<(), RtError> {
        if self.plan.order_by.is_empty() {
            return Ok(());
        }
        // an unorderable pair (a number against a string, say) leaves the two where they were
        // rather than failing the whole query
        let terms = self.plan.order_by.clone();
        let start = self.columns.len() + self.plan.group_keys as usize;
        // where each term reads its value: the pushed keys are taken in the order they were
        // pushed, and a term that names a result column reads it out of the row itself, folded
        let mut pushed = 0usize;
        let slots: Vec<usize> = terms
            .iter()
            .map(|term| match term.key {
                OrderKey::Pushed => {
                    pushed += 1;
                    start + pushed - 1
                }
                OrderKey::Result(n) => n as usize,
                OrderKey::Column(i) => self.widths.iter().take(i as usize).sum(),
            })
            .collect();
        self.rows.sort_by(|a, b| {
            for (term, slot) in terms.iter().zip(&slots) {
                let (x, y) = (a.get(*slot), b.get(*slot));
                let ordering = match (x, y) {
                    (Some(x), Some(y)) => compare(x, y, settings),
                    _ => Ordering::Equal,
                };
                if ordering != Ordering::Equal {
                    return if term.descending { ordering.reverse() } else { ordering };
                }
            }
            Ordering::Equal
        });
        Ok(())
    }
}

/// Folds one aggregate over the values of a group.
fn fold<'a>(kind: AggKind, values: impl Iterator<Item = &'a Value>, settings: &Settings) -> Result<Value, RtError> {
    let mut count = 0usize;
    let mut sum = 0.0;
    let mut best: Option<Value> = None;
    let mut numeric = true;
    for v in values {
        if v.is_null() {
            // SQL aggregates skip nulls, and COUNT(x) counts the values that are there
            continue;
        }
        count += 1;
        match kind {
            AggKind::Sum | AggKind::Avg => sum += v.as_number().unwrap_or(0.0),
            AggKind::Min | AggKind::Max => {
                numeric &= matches!(v, Value::Number(..));
                let take = match &best {
                    None => true,
                    Some(b) => {
                        let ordering = compare(v, b, settings);
                        if kind == AggKind::Min { ordering == Ordering::Less } else { ordering == Ordering::Greater }
                    }
                };
                if take {
                    best = Some(v.clone());
                }
            }
            AggKind::Count | AggKind::CountAll => {}
        }
    }
    Ok(match kind {
        AggKind::CountAll | AggKind::Count => Value::number(count as f64),
        AggKind::Sum => Value::number(sum),
        AggKind::Avg => Value::number(if count == 0 { 0.0 } else { sum / count as f64 }),
        AggKind::Min | AggKind::Max => match best {
            Some(v) => v,
            None if numeric => Value::number(0.0),
            None => Value::Null,
        },
    })
}

/// Row equality for DISTINCT and for grouping: exact, whatever SET EXACT says, because two keys
/// that differ only in trailing blanks are the same key.
fn same(a: &[Value], b: &[Value], settings: &Settings) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| compare(x, y, settings) == Ordering::Equal)
}

fn compare(a: &Value, b: &Value, settings: &Settings) -> Ordering {
    match (a, b) {
        // .NULL. is not a value to compare, and ORDER BY has to put it somewhere: Visual
        // FoxPro sorts it before everything else, which is where an outer join's misses land
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Less,
        (_, Value::Null) => Ordering::Greater,
        (Value::Str(x), Value::Str(y)) => x.trim_end().cmp(y.trim_end()),
        _ => {
            let mut exact = settings.clone();
            exact.exact = true;
            match value::compare(a, b, CmpOp::Lt, &exact) {
                Ok(Value::Logical(true)) => Ordering::Less,
                _ => match value::compare(a, b, CmpOp::Gt, &exact) {
                    Ok(Value::Logical(true)) => Ordering::Greater,
                    _ => Ordering::Equal,
                },
            }
        }
    }
}

/// The widest text each column holds, so a character column is as wide as its contents.
fn column_widths(rows: &[Vec<Value>], columns: usize) -> Vec<usize> {
    let mut widths = vec![1usize; columns];
    for row in rows {
        for (i, width) in widths.iter_mut().enumerate() {
            if let Some(Value::Str(s)) = row.get(i).map(Value::deref).as_ref() {
                *width = (*width).max(s.chars().count());
            }
        }
    }
    widths
}

/// The column type, taken from the first value that has one. A column of nothing but nulls is
/// character, which is what VFP falls back to as well.
fn field_for<'a>(name: &str, mut values: impl Iterator<Item = &'a Value>, width: usize) -> DbfField {
    let kind = values
        .find_map(|v| match v.deref() {
            Value::Number(..) => Some('N'),
            Value::Logical(_) => Some('L'),
            Value::Date(_) => Some('D'),
            Value::DateTime(_) => Some('T'),
            Value::Str(_) => Some('C'),
            _ => None,
        })
        .unwrap_or('C');
    let (length, decimals) = match kind {
        'C' => (width.min(254) as u8, 0),
        'N' => (20, 6),
        'L' => (1, 0),
        'D' => (8, 0),
        _ => (8, 0),
    };
    DbfField::new(name.to_ascii_uppercase(), kind, length, decimals)
}

fn as_dbf(v: &Value) -> DbfValue {
    match v.deref() {
        Value::Null => DbfValue::Null,
        Value::Str(s) => DbfValue::Text(s.to_string()),
        Value::Number(n, ..) => DbfValue::Number(n),
        Value::Logical(b) => DbfValue::Logical(b),
        Value::Date(d) => DbfValue::Date(d),
        Value::DateTime(t) => DbfValue::DateTime(t),
        other => DbfValue::Text(value::display(&other, &Settings::default())),
    }
}
