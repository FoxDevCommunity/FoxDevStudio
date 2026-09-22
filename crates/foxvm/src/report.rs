//! Reports and labels: what a `.frx` holds, and how a band of it becomes lines of text.
//!
//! A report file is a table. Each record is one thing the report has - the page setup, a band,
//! a label, a field, a line, a rectangle - and the record's `OBJTYPE` says which. Everything is
//! placed in ten-thousandths of an inch from the top left of the layout, so a band's objects are
//! the ones whose place falls inside it, and the bands stack down the layout in the order they
//! are written, with the designer's separator bar between them.
//!
//! Nothing here evaluates anything: the VM works out each object's expression against the record
//! the pointer is on and hands the answers back to `lay_out`, which puts them where they go.

use crate::dbf::{DbfValue, read_table};

/// A ten-thousandth of an inch, which is what a report file measures in.
const PER_INCH: f64 = 10_000.0;

/// The bar the report designer draws between two bands. It takes up room in the layout, so a
/// band's place is the heights of the bands above it plus one bar each.
const BAND_BAR: f64 = PER_INCH / 4.8;

/// How wide a character is, and how tall a line is, when a report is written as text. Courier
/// at ten point is six points wide and twelve tall, which is what a report designed for a
/// character printer assumes.
const CHAR_WIDTH: f64 = PER_INCH / 12.0;
const LINE_HEIGHT: f64 = PER_INCH / 6.0;

/// Which band an object belongs to. The numbers are the file's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Title,
    PageHeader,
    ColumnHeader,
    GroupHeader,
    Detail,
    GroupFooter,
    ColumnFooter,
    PageFooter,
    Summary,
    DetailHeader,
    DetailFooter,
}

impl Band {
    fn of(code: i32) -> Band {
        match code {
            0 => Band::Title,
            1 => Band::PageHeader,
            2 => Band::ColumnHeader,
            3 => Band::GroupHeader,
            4 => Band::Detail,
            5 => Band::GroupFooter,
            6 => Band::ColumnFooter,
            7 => Band::PageFooter,
            8 => Band::Summary,
            9 => Band::DetailHeader,
            _ => Band::DetailFooter,
        }
    }

    /// Whether the band is printed once for every record, rather than once for the report.
    pub fn per_record(self) -> bool {
        matches!(self, Band::Detail | Band::DetailHeader | Band::DetailFooter)
    }
}

/// One thing the report draws: a piece of text worked out from an expression, or a rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub band: Band,
    /// The expression to work out, or "" for a line or a rectangle.
    pub expr: String,
    /// The PICTURE clause it was given.
    pub picture: String,
    /// Which line of its band it sits on, and which column it starts at.
    pub row: usize,
    pub col: usize,
    /// How wide it may be, in characters.
    pub width: usize,
    /// A rule rather than a value: it is drawn, not worked out.
    pub rule: bool,
    /// The Print When condition, as written. An item that has one is drawn only when it is
    /// true, which is how one report prints the same line three ways.
    pub print_when: String,
}

/// A report, as much of it as text output needs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// The bands it has, in the order they print, and how many lines high each one is.
    pub bands: Vec<(Band, usize)>,
    pub items: Vec<Item>,
    /// The expression the file says to order the records by, if any.
    pub order: String,
    /// What a label file says: how many columns of labels there are across the page.
    pub columns: usize,
}

impl Report {
    /// The items of one band, in the order they are read: down the band, then across it.
    pub fn band_items(&self, band: Band) -> Vec<&Item> {
        let mut found: Vec<&Item> = self.items.iter().filter(|i| i.band == band).collect();
        found.sort_by(|a, b| a.row.cmp(&b.row).then(a.col.cmp(&b.col)));
        found
    }

    pub fn height_of(&self, band: Band) -> usize {
        self.bands.iter().find(|(b, _)| *b == band).map_or(0, |(_, h)| *h)
    }

    pub fn has(&self, band: Band) -> bool {
        self.bands.iter().any(|(b, _)| *b == band)
    }
}

/// Reads a report or label file: the table and the memo file beside it.
pub fn read_report(frx: &[u8], memo: Option<&[u8]>) -> Result<Report, String> {
    let table = read_table(frx, memo).map_err(|e| e.message)?;
    let at = |name: &str| table.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
    let (objtype, objcode) = (at("OBJTYPE"), at("OBJCODE"));
    let (vpos, hpos, height, width) = (at("VPOS"), at("HPOS"), at("HEIGHT"), at("WIDTH"));
    let (expr, picture, order, print_when) = (at("EXPR"), at("PICTURE"), at("ORDER"), at("SUPEXPR"));
    let number = |values: &[DbfValue], index: Option<usize>| -> f64 {
        index.and_then(|i| values.get(i)).and_then(DbfValue::as_f64).unwrap_or(0.0)
    };
    let text = |values: &[DbfValue], index: Option<usize>| -> String {
        index.and_then(|i| values.get(i)).map(|v| v.as_text().trim_end().to_string()).unwrap_or_default()
    };

    let live: Vec<&crate::dbf::DbfRecord> = table.records.iter().filter(|r| !r.deleted).collect();

    // the bands, in the order the file writes them, which is the order they print
    let mut report = Report::default();
    let mut ranges: Vec<(Band, f64, f64)> = Vec::new();
    let mut top = 0.0;
    for record in live.iter().filter(|r| number(&r.values, objtype) as i32 == 9) {
        let band = Band::of(number(&record.values, objcode) as i32);
        let tall = number(&record.values, height);
        ranges.push((band, top, top + tall));
        report.bands.push((band, (tall / LINE_HEIGHT).round().max(0.0) as usize));
        top += tall + BAND_BAR;
    }

    for record in &live {
        let kind = number(&record.values, objtype) as i32;
        // 5 a label, 8 a field, 6 a line, 7 a rectangle; the rest are the file's own furniture
        let rule = matches!(kind, 6 | 7);
        if !matches!(kind, 5 | 6 | 7 | 8) {
            if kind == 1 {
                report.order = text(&record.values, order);
            }
            continue;
        }
        let v = number(&record.values, vpos);
        let Some((band, start, _)) = ranges.iter().copied().find(|(_, s, e)| v + 2.0 >= *s && v <= *e) else {
            continue;
        };
        report.items.push(Item {
            band,
            expr: if rule { String::new() } else { text(&record.values, expr) },
            picture: text(&record.values, picture),
            row: ((v - start) / LINE_HEIGHT).round().max(0.0) as usize,
            col: (number(&record.values, hpos) / CHAR_WIDTH).round().max(0.0) as usize,
            width: (number(&record.values, width) / CHAR_WIDTH).round().max(1.0) as usize,
            rule,
            print_when: text(&record.values, print_when),
        });
    }
    Ok(report)
}

/// One band as lines of text: each item's answer put at the column it belongs in.
///
/// `values` are the answers in the order `band_items` gave the items, so the caller works out
/// the expressions and this only places them.
pub fn lay_out(report: &Report, band: Band, values: &[String]) -> Vec<String> {
    let items = report.band_items(band);
    let mut lines: Vec<String> = vec![String::new(); report.height_of(band).max(items.iter().map(|i| i.row + 1).max().unwrap_or(0))];
    for (item, value) in items.iter().zip(values.iter()) {
        while lines.len() <= item.row {
            lines.push(String::new());
        }
        let text = if item.rule { "-".repeat(item.width) } else { value.clone() };
        let line = &mut lines[item.row];
        let mut chars: Vec<char> = line.chars().collect();
        while chars.len() < item.col {
            chars.push(' ');
        }
        chars.truncate(item.col);
        chars.extend(text.chars().take(item.width.max(text.chars().count().min(item.width))));
        *line = chars.into_iter().collect();
    }
    for line in &mut lines {
        while line.ends_with(' ') {
            line.pop();
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_band_lays_its_items_out_at_the_columns_they_were_given() {
        let report = Report {
            bands: vec![(Band::Detail, 1)],
            items: vec![
                Item { band: Band::Detail, expr: "a".into(), picture: String::new(), row: 0, col: 0, width: 6, rule: false, print_when: String::new() },
                Item { band: Band::Detail, expr: "b".into(), picture: String::new(), row: 0, col: 10, width: 8, rule: false, print_when: String::new() },
            ],
            order: String::new(),
            columns: 0,
        };
        assert_eq!(lay_out(&report, Band::Detail, &["Bolt".into(), "2.50".into()]), vec!["Bolt      2.50"]);
    }
}
