//! Compile-time diagnostics shared by the lexer, parser and compiler.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// Source range in character offsets (not bytes) so the editor can map it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
    pub fn to(self, other: Span) -> Span {
        Span { start: self.start.min(other.start), end: self.end.max(other.end) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// 1-based physical line of the start position.
    pub line: u32,
    /// 1-based character column of the start position.
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    /// Character offsets into the source.
    pub start: usize,
    pub end: usize,
}

impl Diagnostic {
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Maps character offsets to 1-based line/column pairs.
pub struct LineMap {
    /// Character offset at which each line starts; `starts[0] == 0`.
    starts: Vec<usize>,
}

impl LineMap {
    pub fn new(src: &str) -> Self {
        let mut starts = vec![0];
        for (i, ch) in src.chars().enumerate() {
            if ch == '\n' {
                starts.push(i + 1);
            }
        }
        LineMap { starts }
    }

    /// (line, col), both 1-based.
    pub fn position(&self, offset: usize) -> (u32, u32) {
        let idx = match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        ((idx + 1) as u32, (offset - self.starts[idx] + 1) as u32)
    }

    pub fn line_of(&self, offset: usize) -> u32 {
        self.position(offset).0
    }

    pub fn diagnostic(&self, severity: Severity, span: Span, message: impl Into<String>) -> Diagnostic {
        let (line, col) = self.position(span.start);
        let (end_line, end_col) = self.position(span.end.max(span.start));
        Diagnostic { severity, message: message.into(), line, col, end_line, end_col, start: span.start, end: span.end }
    }

    pub fn error(&self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.diagnostic(Severity::Error, span, message)
    }

    pub fn warning(&self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.diagnostic(Severity::Warning, span, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_offsets_to_lines() {
        let m = LineMap::new("ab\ncd\n\nx");
        assert_eq!(m.position(0), (1, 1));
        assert_eq!(m.position(1), (1, 2));
        assert_eq!(m.position(3), (2, 1));
        assert_eq!(m.position(6), (3, 1));
        assert_eq!(m.position(7), (4, 1));
    }
}
