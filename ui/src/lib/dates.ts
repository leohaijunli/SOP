// Run timestamps are stored in UTC (ISO 8601). The window shows them in the operator's
// local time: an evening run west of UTC otherwise reads as the next day, which will not
// match the run id, and the run id is built from the local date.
function parse(value: unknown): Date | null {
  if (value === null || value === undefined) return null;
  const raw = String(value).trim();
  if (!raw) return null;
  const date = new Date(raw);
  return Number.isNaN(date.getTime()) ? null : date;
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/// The local calendar day, `YYYY-MM-DD`. A value that is not a parseable timestamp falls
/// back to its own first ten characters, so a hand-written date still shows.
export function localDay(value: unknown): string {
  const date = parse(value);
  if (!date) return String(value ?? "").slice(0, 10);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/// The local date and time, `YYYY-MM-DD HH:MM`.
export function localDateTime(value: unknown): string {
  const date = parse(value);
  if (!date) return String(value ?? "");
  return `${localDay(value)} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
