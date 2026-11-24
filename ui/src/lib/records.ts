// Small presentation helpers shared by notebook, log history, and global search.
// Log bytes are never loaded by a search: only indexed metadata and recorded notes.
import { text } from "./text.ts";
import type { Manifest, RunEntry, RunFileView } from "./types.ts";

export type RecordMode = "notes" | "logs" | "search";
export type TimeOrder = "newest" | "oldest";
export type TimeBasis = "file" | "added";

export interface ManagedFile {
  key: string;
  file: RunFileView;
  run: RunEntry | null;
  label: string;
  editable: boolean;
}

export const runKey = (run: RunEntry): string => `${text(run.sop)}::${text(run.run_id)}`;

export function runTitle(run: RunEntry, manifest: Manifest | null): string {
  if (text(run.title)) return text(run.title);
  const sop = text(run.sop);
  for (const plan of manifest?.testplans ?? []) {
    const entry = plan.cases.find((candidate) => candidate.sop_id === sop);
    if (entry) return text(entry.title) || entry.id;
  }
  return text(manifest?.checklists.find((candidate) => text(candidate.sop_id) === sop)?.title) || text(run.run_id);
}

const normalized = (value: string): string => value.normalize("NFKC").toLocaleLowerCase();

export function matchesQuery(query: string, values: unknown[]): boolean {
  const haystack = normalized(values.map(text).join("\n"));
  return normalized(query).trim().split(/\s+/).filter(Boolean).every((term) => haystack.includes(term));
}

export function recordMatches(run: RunEntry, manifest: Manifest | null, query: string): boolean {
  return matchesQuery(query, [runTitle(run, manifest), run.run_id, run.sop, run.operator, run.site,
    run.plan, run.case, run.started, run.status, run.conclusion, run.conclusion_note,
    run.sensor?.model, run.sensor?.serial,
    ...(run.notes ?? []).map((note) => note.text),
    ...(run.files ?? []).flatMap((file) => [file.name, file.path, file.remark, ...(file.tags ?? [])]),
  ]);
}

export function collectFiles(manifest: Manifest | null): ManagedFile[] {
  const out: ManagedFile[] = [];
  const seen = new Set<string>();
  for (const run of manifest?.runs ?? []) {
    for (const file of run.files ?? []) {
      const key = `${runKey(run)}::${file.path}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push({ key, file, run, label: runTitle(run, manifest), editable: run.editable });
    }
  }
  for (const file of manifest?.workspace_files ?? []) {
    out.push({ key: `workspace::${file.path}`, file, run: null, label: "Unlinked log", editable: true });
  }
  return out;
}

export function fileMatches(row: ManagedFile, query: string): boolean {
  const { file, run } = row;
  return matchesQuery(query, [file.name, file.path, file.remark, file.fileTime, file.addedAt,
    ...(file.tags ?? []), row.label, run?.operator, run?.site, run?.run_id, run?.sensor?.model, run?.sensor?.serial]);
}

export const fileTime = (file: RunFileView, basis: TimeBasis): string =>
  basis === "added" ? file.addedAt || file.fileTime : file.fileTime || file.addedAt;

export function sortFiles(rows: ManagedFile[], order: TimeOrder, basis: TimeBasis): ManagedFile[] {
  const direction = order === "newest" ? -1 : 1;
  return [...rows].sort((a, b) => {
    const first = Date.parse(fileTime(a.file, basis));
    const second = Date.parse(fileTime(b.file, basis));
    if (Number.isNaN(first) !== Number.isNaN(second)) return Number.isNaN(first) ? 1 : -1;
    if (!Number.isNaN(first) && first !== second) return direction * (first - second);
    return a.key.localeCompare(b.key);
  });
}

export function parseTags(value: string): string[] {
  const seen = new Set<string>();
  return value.split(/[,，;；\n]/).map((tag) => tag.trim()).filter((tag) => {
    const key = normalized(tag);
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

export const hasTag = (file: RunFileView, tag: string): boolean =>
  !tag || (file.tags ?? []).some((candidate) => normalized(candidate) === normalized(tag));

export function tagChoices(rows: ManagedFile[]): string[] {
  const seen = new Set<string>();
  return rows.flatMap((row) => row.file.tags ?? []).filter((tag) => {
    const key = normalized(tag.trim());
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  }).sort((a, b) => a.localeCompare(b, "en", { numeric: true }));
}

export function notePreview(run: RunEntry, query = ""): string {
  const notes = run.notes ?? [];
  const match = query.trim() ? notes.find((note) => matchesQuery(query, [note.text])) : null;
  return (match?.text || notes.at(-1)?.text || text(run.conclusion_note)).replace(/\s+/g, " ").slice(0, 140);
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}
