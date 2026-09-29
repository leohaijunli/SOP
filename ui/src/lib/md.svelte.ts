// Markdown rendering for the window: the rules live in Rust (`sop_core::md`, reached
// through the `render_markdown` command), and this is a cache of the answers.
//
// `htmlOf` is called from templates, so it has to be synchronous. It returns the cached
// HTML when it has it and asks Rust once otherwise; reading `version` is what makes the
// calling component re-render when the answer arrives.
import { renderMarkdown } from "./api";
import { text } from "./text";

let version = $state(0);
const cache = new Map<string, string>();
const inFlight = new Set<string>();

const escapeHtml = (value: string): string =>
  value.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] ?? c);

export function htmlOf(source: unknown): string {
  const key = text(source);
  if (!key.trim()) return "";
  version;
  const cached = cache.get(key);
  if (cached !== undefined) return cached;
  if (!inFlight.has(key)) {
    inFlight.add(key);
    void renderMarkdown(key)
      .then((html) => cache.set(key, html))
      // A renderer that is unreachable still shows the operator their own words.
      .catch(() => cache.set(key, `<p>${escapeHtml(key)}</p>`))
      .finally(() => {
        inFlight.delete(key);
        version++;
      });
  }
  return "";
}
