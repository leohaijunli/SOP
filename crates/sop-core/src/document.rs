//! A parsed content file.
//!
//! Parsing is pure: it takes text and returns a document. Reading files and resolving
//! include paths belong to `sop-repo`.

use crate::error::ParseError;
use crate::fence;
use crate::front::{self, FrontMatter};
use crate::step::{self, ResultBlock, Step};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Include {
    pub line: usize,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub line: usize,
    pub level: usize,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub front: FrontMatter,
    pub body: String,
    pub body_start_line: usize,
    /// Total number of lines in the file, used for span ends.
    pub line_count: usize,
    pub kind: Option<String>,
    pub steps: Vec<Step>,
    pub results: Vec<ResultBlock>,
    pub includes: Vec<Include>,
    pub headings: Vec<Heading>,
}

impl Document {
    pub fn parse(text: &str) -> Result<Document, ParseError> {
        let (front, body, body_start_line) = front::split(text)?;
        let line_count = text.lines().count();
        let fences = fence::scan(&body, body_start_line)?;
        let outside = fence::lines_outside_fences(&body, body_start_line);

        let headings = collect_headings(&outside);
        let includes = collect_includes(&outside);

        let mut steps = Vec::new();
        let mut results = Vec::new();
        for block in &fences {
            match block.info.as_str() {
                "yaml step" => {
                    let mut step = step::parse_step(
                        &block.info,
                        &block.content,
                        block.start_line,
                        block.content_start_line,
                    )?;
                    // The fence itself, which `parse_step` cannot know on its own.
                    step.block_span = (block.start_line, block.end_line + 1);
                    steps.push(step);
                }
                "yaml result" => results.push(step::parse_result(
                    &block.info,
                    &block.content,
                    block.start_line,
                )?),
                _ => {}
            }
        }

        for step in steps.iter_mut() {
            let line = step.line;
            let title_heading = headings
                .iter()
                .rev()
                .find(|heading| heading.level == 2 && heading.line < line)
                .cloned();
            let next_heading = headings
                .iter()
                .find(|heading| heading.level == 2 && heading.line > line)
                .map(|heading| heading.line);

            let span_start = title_heading.as_ref().map_or(line, |heading| heading.line);
            let span_end = next_heading.unwrap_or(line_count + 1);

            step.title = title_heading.as_ref().map(|heading| heading.title.clone());
            step.title_line = title_heading.as_ref().map(|heading| heading.line);
            step.span = (span_start, span_end.max(span_start + 1));
            step.prose = prose_of(&body, body_start_line, step);
        }

        let kind = front
            .get("kind")
            .and_then(|value| value.as_str())
            .map(str::to_owned);

        Ok(Document {
            front,
            body,
            body_start_line,
            line_count,
            kind,
            steps,
            results,
            includes,
            headings,
        })
    }

    /// The `##` headings, in order.
    pub fn step_headings(&self) -> impl Iterator<Item = &Heading> {
        self.headings.iter().filter(|heading| heading.level == 2)
    }
}

/// The prose of a step: its span, minus its own `##` heading and minus the `yaml step`
/// block that carries the metadata.
///
/// The heading is left out because the caller already has it as `title`, and rendering
/// it twice is what "the frontend is a renderer" is supposed to prevent.
///
/// `span` and `block_span` are 1-based file lines, so they are shifted by the line the
/// body starts on.
fn prose_of(body: &str, body_start_line: usize, step: &Step) -> String {
    let lines: Vec<&str> = body.split('\n').collect();
    let index_of = |line: usize| line.saturating_sub(body_start_line);
    let heading = step.title_line.map(index_of).unwrap_or(usize::MAX);
    let start = index_of(step.span.0).min(lines.len());
    let end = index_of(step.span.1).min(lines.len());
    let block_start = index_of(step.block_span.0);
    let block_end = index_of(step.block_span.1);

    let kept: Vec<&str> = lines[start..end]
        .iter()
        .enumerate()
        .filter(|(offset, _)| {
            let index = start + offset;
            index != heading && (index < block_start || index >= block_end)
        })
        .map(|(_, line)| *line)
        .collect();

    // Removing the block leaves the blank lines that framed it behind. Collapse runs of
    // blank lines so the extracted text reads the way the file does.
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in kept {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_owned()
}

fn collect_headings(outside: &[(usize, &str)]) -> Vec<Heading> {
    let mut headings = Vec::new();
    for (line, text) in outside {
        let hashes = text.chars().take_while(|c| *c == '#').count();
        if !(2..=6).contains(&hashes) {
            continue;
        }
        let rest = &text[hashes..];
        if !rest.starts_with(' ') {
            continue;
        }
        headings.push(Heading {
            line: *line,
            level: hashes,
            title: rest.trim().to_owned(),
        });
    }
    headings
}

/// Recognise `<!-- include: procedures/x.md -->`, which must be on its own line.
pub fn parse_include_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let target = inner.strip_prefix("include:")?.trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_owned())
    }
}

fn collect_includes(outside: &[(usize, &str)]) -> Vec<Include> {
    outside
        .iter()
        .filter_map(|(line, text)| {
            parse_include_marker(text).map(|target| Include {
                line: *line,
                target,
            })
        })
        .collect()
}
