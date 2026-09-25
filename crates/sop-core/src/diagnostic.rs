//! Diagnostics produced by parsing and checking.
//!
//! Line numbers are 1-based and refer to the file the diagnostic is about, not to the
//! body or a block.

use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The content is wrong. Validation fails.
    Error,
    /// The content is suspicious but usable. Validation passes.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// 1-based line number, when the diagnostic can be attributed to one.
    pub line: Option<usize>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, line: Option<usize>) -> Self {
        Self {
            severity: Severity::Error,
            line,
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>, line: Option<usize>) -> Self {
        Self {
            severity: Severity::Warning,
            line,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// An ordered collection of diagnostics, with counters.
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn error(&mut self, message: impl Into<String>, line: Option<usize>) {
        self.push(Diagnostic::error(message, line));
    }

    pub fn warning(&mut self, message: impl Into<String>, line: Option<usize>) {
        self.push(Diagnostic::warning(message, line));
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.items.extend(other.items);
    }

    pub fn items(&self) -> &[Diagnostic] {
        &self.items
    }

    pub fn into_items(self) -> Vec<Diagnostic> {
        self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(Diagnostic::is_error)
    }

    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|d| d.is_error()).count()
    }

    pub fn warning_count(&self) -> usize {
        self.items.iter().filter(|d| !d.is_error()).count()
    }

    /// Sort by line, keeping the original order within a line.
    ///
    /// The sort is stable, so diagnostics attributed to the same line stay in the order
    /// the checks produced them. That is what makes the output stable and readable
    /// without needing every check to know about every other check.
    pub fn sort(&mut self) {
        self.items.sort_by_key(|d| d.line.unwrap_or(0));
    }
}
