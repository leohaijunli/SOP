//! The Markdown renderer, in one place.
//!
//! The subset is deliberately small - the set this repository's content actually uses:
//! fenced code, hr, `#`-`####` headings, tables, blockquotes, lists (with task items), and
//! inline code / strong / em / links. Everything else is emitted as a plain paragraph, so
//! unexpected syntax shows up as text instead of silently disappearing.
//!
//! Both readers of Markdown go through here: the desktop app (through a Tauri command)
//! and `sop preview` (which renders server-side). There is no second implementation to
//! keep in step, and no rules in the UI.

use std::sync::LazyLock;

use regex::Regex;

static FENCE_START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*```").unwrap());
static FENCE_TICKS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(`+)").unwrap());
static HR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(---+|\*\*\*+)\s*$").unwrap());
static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(#{1,4})\s+(.*)$").unwrap());
static TABLE_ROW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\|").unwrap());
static TABLE_SEPARATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\|[\s:|-]+\|\s*$").unwrap());
static QUOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*>\s?").unwrap());
static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)[-*+]\s+(.*)$").unwrap());
static NUMBERED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)\d+[.)]\s+(.*)$").unwrap());
static BULLET_ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*[-*+]\s+(.*)$").unwrap());
static NUMBERED_ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\d+[.)]\s+(.*)$").unwrap());
static TASK_ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[([ xX])\]\s+(.*)$").unwrap());
static PARAGRAPH_STOP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(```|>|[-*+]\s|\d+[.)]\s|\||#{1,4}\s)").unwrap()
});

static CODE_SPAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`]+)`").unwrap());
static STRONG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*\*([^*]+)\*\*").unwrap());
static EMPHASIS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[\s(])\*([^*\n]+)\*").unwrap());
static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\(([^)\s]+)\)").unwrap());

/// Render Markdown to HTML with the subset above.
pub fn render(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut html: Vec<String> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if FENCE_START.is_match(line) {
            let ticks = FENCE_TICKS.captures(line).map_or(0, |caps| caps[1].len());
            let closing = Regex::new(&format!(r"^\s*`{{{ticks},}}\s*$")).unwrap();
            let mut body: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() && !closing.is_match(lines[i]) {
                body.push(lines[i]);
                i += 1;
            }
            i += 1;
            html.push(format!(
                "<pre><code>{}</code></pre>",
                escape_html(&body.join("\n"))
            ));
            continue;
        }

        if HR.is_match(line) {
            html.push("<hr>".to_owned());
            i += 1;
            continue;
        }

        if let Some(caps) = HEADING.captures(line) {
            let level = caps[1].len();
            html.push(format!("<h{level}>{}</h{level}>", inline(&caps[2])));
            i += 1;
            continue;
        }

        if TABLE_ROW.is_match(line)
            && lines.get(i + 1).is_some_and(|next| TABLE_SEPARATOR.is_match(next))
        {
            let head = cells(line);
            i += 2;
            let mut body: Vec<Vec<String>> = Vec::new();
            while i < lines.len() && TABLE_ROW.is_match(lines[i]) {
                body.push(cells(lines[i]));
                i += 1;
            }
            let head_html: String = head
                .iter()
                .map(|cell| format!("<th>{}</th>", inline(cell)))
                .collect();
            let body_html: String = body
                .iter()
                .map(|row| {
                    let cells: String = row
                        .iter()
                        .map(|cell| format!("<td>{}</td>", inline(cell)))
                        .collect();
                    format!("<tr>{cells}</tr>")
                })
                .collect();
            html.push(format!(
                "<table><thead><tr>{head_html}</tr></thead><tbody>{body_html}</tbody></table>"
            ));
            continue;
        }

        if QUOTE.is_match(line) {
            let mut body: Vec<String> = Vec::new();
            while i < lines.len() && QUOTE.is_match(lines[i]) {
                body.push(QUOTE.replace(lines[i], "").into_owned());
                i += 1;
            }
            html.push(format!(
                "<blockquote>{}</blockquote>",
                render(&body.join("\n"))
            ));
            continue;
        }

        let bullet = BULLET.is_match(line);
        let numbered = NUMBERED.is_match(line);
        if bullet || numbered {
            let mut items: Vec<String> = Vec::new();
            while i < lines.len() {
                let item = if numbered {
                    NUMBERED_ITEM.captures(lines[i])
                } else {
                    BULLET_ITEM.captures(lines[i])
                };
                let Some(caps) = item else { break };
                let text = caps[1].to_owned();
                items.push(match TASK_ITEM.captures(&text) {
                    Some(task) => {
                        let checked = if &task[1] == " " { "" } else { " checked" };
                        format!(
                            "<li class=\"task\"><input type=\"checkbox\" disabled{checked}> {}</li>",
                            inline(&task[2])
                        )
                    }
                    None => format!("<li>{}</li>", inline(&text)),
                });
                i += 1;
            }
            let tag = if numbered { "ol" } else { "ul" };
            html.push(format!("<{tag}>{}</{tag}>", items.join("")));
            continue;
        }

        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        let mut paragraph: Vec<&str> = Vec::new();
        while i < lines.len()
            && !lines[i].trim().is_empty()
            && !PARAGRAPH_STOP.is_match(lines[i])
        {
            paragraph.push(lines[i]);
            i += 1;
        }
        // Every stop pattern is handled above, so this cannot normally be reached. It is
        // here so an unforeseen line is emitted rather than looping forever.
        if paragraph.is_empty() {
            paragraph.push(lines[i]);
            i += 1;
        }
        html.push(format!("<p>{}</p>", inline(&paragraph.join(" "))));
    }

    html.join("\n")
}

/// The inline spans, applied in the order the content expects: escape first, then code,
/// strong, emphasis, and links.
fn inline(text: &str) -> String {
    let out = escape_html(text);
    let out = CODE_SPAN.replace_all(&out, "<code>${1}</code>");
    let out = STRONG.replace_all(&out, "<strong>${1}</strong>");
    let out = EMPHASIS.replace_all(&out, "${1}<em>${2}</em>");
    let out = LINK.replace_all(&out, "<a href=\"${2}\">${1}</a>");
    out.into_owned()
}

/// Escape the five characters that would otherwise become markup. A note is free text an
/// operator typed, so this is what keeps a `<script>` in a note from being one.
fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

/// One table row split into its cells, without the outer pipes.
fn cells(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    trimmed.split('|').map(|cell| cell.trim().to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_from_one_to_four_render() {
        assert_eq!(render("# test"), "<h1>test</h1>");
        assert_eq!(render("## keyi"), "<h2>keyi</h2>");
        assert_eq!(render("### three"), "<h3>three</h3>");
        assert_eq!(render("#### four"), "<h4>four</h4>");
    }

    #[test]
    fn text_that_only_looks_like_a_heading_stays_a_paragraph() {
        assert_eq!(render("####### seven"), "<p>####### seven</p>");
        assert_eq!(render("#nospace"), "<p>#nospace</p>");
    }

    #[test]
    fn a_heading_after_a_paragraph_is_not_swallowed_by_it() {
        assert_eq!(render("text\n# after"), "<p>text</p>\n<h1>after</h1>");
    }

    #[test]
    fn a_fence_keeps_its_body_verbatim_and_escaped() {
        assert_eq!(
            render("```\n# not a heading\n<b>\n```"),
            "<pre><code># not a heading\n&lt;b&gt;</code></pre>"
        );
    }

    #[test]
    fn paragraph_lines_join_with_a_space() {
        assert_eq!(render("one\ntwo\n\nthree"), "<p>one two</p>\n<p>three</p>");
    }

    #[test]
    fn a_table_renders_a_head_and_a_body() {
        assert_eq!(
            render("| a | b |\n|---|---|\n| 1 | 2 |"),
            "<table><thead><tr><th>a</th><th>b</th></tr></thead>\
             <tbody><tr><td>1</td><td>2</td></tr></tbody></table>"
        );
    }

    #[test]
    fn task_items_render_as_disabled_checkboxes() {
        assert_eq!(
            render("- [ ] first\n- [x] second"),
            "<ul><li class=\"task\"><input type=\"checkbox\" disabled> first</li>\
             <li class=\"task\"><input type=\"checkbox\" disabled checked> second</li></ul>"
        );
    }

    #[test]
    fn a_numbered_list_opens_and_closes_as_an_ordered_list() {
        assert_eq!(render("1. one\n2. two"), "<ol><li>one</li><li>two</li></ol>");
    }

    #[test]
    fn a_blockquote_renders_its_lines_as_one_block() {
        assert_eq!(
            render("> note: first line\n>\n> - a list item\n> second line"),
            "<blockquote><p>note: first line</p>\n<ul><li>a list item</li></ul>\n\
             <p>second line</p></blockquote>"
        );
    }

    #[test]
    fn inline_spans_render_in_the_documented_order() {
        assert_eq!(
            render("a `code` and **bold** and *em* and [link](x.md)"),
            "<p>a <code>code</code> and <strong>bold</strong> and <em>em</em> and \
             <a href=\"x.md\">link</a></p>"
        );
    }

    #[test]
    fn markup_in_an_operators_text_is_escaped() {
        assert_eq!(
            render("# <script>alert(\"x\")</script>"),
            "<h1>&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;</h1>"
        );
        assert!(!render("# <script>alert(1)</script>").contains("<script>"));
    }

    #[test]
    fn a_horizontal_rule_renders() {
        assert_eq!(render("---"), "<hr>");
    }
}
