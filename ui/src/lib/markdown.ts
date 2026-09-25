// A deliberately small Markdown renderer: enough for the content in this repository and
// no more. The set of constructs it supports is the set the content actually uses:
// fenced code, hr, h2-h4, tables, blockquotes, lists (with task items), and inline
// code/strong/em/links. Everything else is emitted as plain paragraphs.
//
// This is the same renderer the preview server uses, lifted out of the HTML so the
// desktop app and the preview server stay on one implementation.

const escapeHtml = (s: string): string =>
  s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] ?? c);

function inline(text: string): string {
  let out = escapeHtml(text);
  out = out.replace(/`([^`]+)`/g, (_, code) => `<code>${code}</code>`);
  out = out.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  out = out.replace(/(^|[\s(])\*([^*\n]+)\*/g, "$1<em>$2</em>");
  out = out.replace(/\[([^\]]*)\]\(([^)\s]+)\)/g, (_, label, href) => `<a href="${href}">${label}</a>`);
  return out;
}

export function markdown(source: string): string {
  const lines = source.split("\n");
  const html: string[] = [];
  let index = 0;

  const flushParagraph = (buffer: string[]): void => {
    if (buffer.length) html.push(`<p>${inline(buffer.join(" "))}</p>`);
    buffer.length = 0;
  };

  while (index < lines.length) {
    const line = lines[index];

    if (/^\s*```/.test(line)) {
      const ticks = (line.match(/^\s*`+/) ?? [""])[0].trim().length;
      const body: string[] = [];
      index++;
      while (index < lines.length && !new RegExp(`^\\s*\`{${ticks},}\\s*$`).test(lines[index])) body.push(lines[index++]);
      index++;
      html.push(`<pre><code>${escapeHtml(body.join("\n"))}</code></pre>`);
      continue;
    }

    if (/^\s*(---+|\*\*\*+)\s*$/.test(line)) { html.push("<hr>"); index++; continue; }

    const heading = line.match(/^(#{2,4})\s+(.*)$/);
    if (heading) { html.push(`<h${heading[1].length}>${inline(heading[2])}</h${heading[1].length}>`); index++; continue; }

    if (/^\s*\|/.test(line) && /^\s*\|[\s:|-]+\|\s*$/.test(lines[index + 1] || "")) {
      const cells = (row: string): string[] => row.trim().replace(/^\||\|$/g, "").split("|").map((c) => c.trim());
      const head = cells(line);
      index += 2;
      const body: string[][] = [];
      while (index < lines.length && /^\s*\|/.test(lines[index])) body.push(cells(lines[index++]));
      html.push(
        `<table><thead><tr>${head.map((c) => `<th>${inline(c)}</th>`).join("")}</tr></thead>` +
        `<tbody>${body.map((r) => `<tr>${r.map((c) => `<td>${inline(c)}</td>`).join("")}</tr>`).join("")}</tbody></table>`
      );
      continue;
    }

    if (/^\s*>\s?/.test(line)) {
      const body: string[] = [];
      while (index < lines.length && /^\s*>\s?/.test(lines[index])) body.push(lines[index++].replace(/^\s*>\s?/, ""));
      html.push(`<blockquote>${markdown(body.join("\n"))}</blockquote>`);
      continue;
    }

    const bullet = line.match(/^(\s*)[-*+]\s+(.*)$/);
    const numbered = line.match(/^(\s*)\d+[.)]\s+(.*)$/);
    if (bullet || numbered) {
      const ordered = Boolean(numbered);
      const items: string[] = [];
      while (index < lines.length) {
        const m = lines[index].match(ordered ? /^\s*\d+[.)]\s+(.*)$/ : /^\s*[-*+]\s+(.*)$/);
        if (!m) break;
        const task = m[1].match(/^\[([ xX])\]\s+(.*)$/);
        items.push(task
          ? `<li class="task"><input type="checkbox" disabled${task[1] === " " ? "" : " checked"}> ${inline(task[2])}</li>`
          : `<li>${inline(m[1])}</li>`);
        index++;
      }
      html.push(`<${ordered ? "ol" : "ul"}>${items.join("")}</${ordered ? "ul" : "ol"}>`);
      continue;
    }

    if (!line.trim()) { index++; continue; }

    const paragraph: string[] = [];
    while (index < lines.length && lines[index].trim() && !/^\s*(```|>|[-*+]\s|\d+[.)]\s|\||#{2,4}\s)/.test(lines[index])) {
      paragraph.push(lines[index++]);
    }
    flushParagraph(paragraph);
  }

  return html.join("\n");
}

export const text = (value: unknown): string =>
  value === null || value === undefined ? "" : String(value);