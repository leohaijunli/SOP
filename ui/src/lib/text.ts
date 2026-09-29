// A value from any source, as display text. This is the only helper the window needs from
// the old Markdown module: rendering itself lives in Rust (`sop_core::md`), reached
// through `htmlOf` in `md.svelte.ts`.
export const text = (value: unknown): string =>
  value === null || value === undefined ? "" : String(value);
