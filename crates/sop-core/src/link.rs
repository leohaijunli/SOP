//! Finding relative Markdown links so a caller can check that they resolve.

use crate::fence;

/// A relative link target found outside a fenced block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub line: usize,
    /// The target exactly as written, including any `#fragment`.
    pub target: String,
}

impl Link {
    /// The file part of the target, with any `#fragment` removed.
    pub fn path(&self) -> &str {
        self.target.split('#').next().unwrap_or(&self.target)
    }
}

/// True for targets that are not files we can check: anchors and URLs with a scheme.
pub fn is_external(target: &str) -> bool {
    if target.starts_with('#') {
        return true;
    }
    match target.find(':') {
        Some(colon) => {
            let scheme = &target[..colon];
            !scheme.is_empty()
                && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        }
        None => false,
    }
}

/// The target of an inline link starting at the `[` at byte `start`, if there is one.
///
/// Mirrors the reference pattern `\[[^\]]*\]\(([^)\s]+)\)`: the label may not contain
/// `]`, the target may not contain `)` or whitespace, and the target may not be empty.
/// Returns the byte index one past the closing `)`.
fn inline_link(text: &str, start: usize) -> Option<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut cursor = start + 1;
    while cursor < bytes.len() && bytes[cursor] != b']' {
        cursor += 1;
    }
    if cursor >= bytes.len() || bytes.get(cursor + 1) != Some(&b'(') {
        return None;
    }
    let target_start = cursor + 2;
    let mut end = target_start;
    while end < bytes.len() && bytes[end] != b')' && !text[end..].starts_with(char::is_whitespace) {
        end += 1;
    }
    if end == target_start || bytes.get(end) != Some(&b')') {
        return None;
    }
    Some((end + 1, &text[target_start..end]))
}

/// Collect relative link targets from a document body.
///
/// Anchors and absolute URLs are skipped, because there is nothing on disk to check
/// them against. A fragment is kept, so the caller can report the link as written.
pub fn scan(body: &str, body_start_line: usize) -> Vec<Link> {
    let mut out = Vec::new();
    for (line, text) in fence::lines_outside_fences(body, body_start_line) {
        let mut index = 0;
        while index < text.len() {
            if text.as_bytes()[index] != b'[' {
                index += 1;
                continue;
            }
            match inline_link(text, index) {
                None => index += 1,
                Some((next, target)) => {
                    if !is_external(target) {
                        out.push(Link {
                            line,
                            target: target.to_owned(),
                        });
                    }
                    index = next;
                }
            }
        }
    }
    out
}
