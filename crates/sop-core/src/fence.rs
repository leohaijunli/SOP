//! Fenced code block scanning.
//!
//! Step metadata and run results live in fenced blocks. Only fences that begin at
//! column 0 are recognised, which keeps them distinguishable from indented examples
//! inside prose.

use crate::error::ParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    /// The info string, e.g. `yaml step`.
    pub info: String,
    pub content: String,
    /// 1-based line of the opening fence.
    pub start_line: usize,
    /// 1-based line of the first content line.
    pub content_start_line: usize,
    /// 1-based line of the closing fence.
    pub end_line: usize,
}

/// Number of leading backticks, zero unless the line starts with a fence.
fn backticks(line: &str) -> usize {
    line.chars().take_while(|c| *c == '`').count()
}

/// True when `line` closes a fence opened with `ticks` backticks.
fn closes(line: &str, ticks: usize) -> bool {
    let count = backticks(line);
    count >= ticks && line[count..].trim().is_empty()
}

/// Scan every fenced block in `body`.
///
/// `body_start_line` is the 1-based line number of `body`'s first line in the file, so
/// that reported line numbers refer to the file rather than to the body.
pub fn scan(body: &str, body_start_line: usize) -> Result<Vec<Fence>, ParseError> {
    let lines: Vec<&str> = body.split('\n').collect();
    let mut fences = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let ticks = backticks(lines[index]);
        if ticks < 3 {
            index += 1;
            continue;
        }
        let info = lines[index][ticks..].trim().to_owned();
        let mut close = index + 1;
        while close < lines.len() && !closes(lines[close], ticks) {
            close += 1;
        }
        if close >= lines.len() {
            return Err(ParseError::UnclosedFence {
                line: body_start_line + index,
            });
        }
        fences.push(Fence {
            info,
            content: lines[index + 1..close].join("\n"),
            start_line: body_start_line + index,
            content_start_line: body_start_line + index + 1,
            end_line: body_start_line + close,
        });
        index = close + 1;
    }

    Ok(fences)
}

/// The lines of `body` that are not inside a fenced block, as (line number, text).
pub fn lines_outside_fences(body: &str, body_start_line: usize) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = body.split('\n').collect();
    let mut out = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let ticks = backticks(lines[index]);
        if ticks >= 3 {
            index += 1;
            while index < lines.len() {
                let closing = closes(lines[index], ticks);
                index += 1;
                if closing {
                    break;
                }
            }
            continue;
        }
        out.push((body_start_line + index, lines[index]));
        index += 1;
    }

    out
}
