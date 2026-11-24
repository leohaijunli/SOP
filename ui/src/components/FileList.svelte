<script lang="ts">
  import { localDateTime } from "../lib/dates";
  import { fileTime, formatSize, parseTags, type ManagedFile, type TimeBasis } from "../lib/records";
  import { readDraft, writeDraft } from "../lib/drafts";

  let { rows, workspace, busy, timeBasis = "file", showRecord = true, onSave, onOpen, onRecord, onTag }: {
    rows: ManagedFile[]; workspace: string; busy: boolean; timeBasis?: TimeBasis; showRecord?: boolean;
    onSave: (row: ManagedFile, remark: string, tags: string[]) => Promise<boolean>;
    onOpen: (row: ManagedFile) => void; onRecord: (row: ManagedFile) => void; onTag: (tag: string) => void;
  } = $props();
  let editing = $state<string | null>(null);
  let remark = $state("");
  let tags = $state("");

  const draftKey = (row: ManagedFile): string => `${workspace}::file::${row.key}`;
  function edit(row: ManagedFile): void {
    editing = row.key;
    let draft: { remark?: string; tags?: string } = {};
    try { draft = JSON.parse(readDraft(draftKey(row)) || "{}"); } catch { /* use the current file */ }
    remark = draft.remark ?? row.file.remark ?? "";
    tags = draft.tags ?? (row.file.tags ?? []).join(", ");
  }
  function remember(row: ManagedFile): void {
    writeDraft(draftKey(row), JSON.stringify({ remark, tags }));
  }
  async function save(row: ManagedFile): Promise<void> {
    if (busy) return;
    if (await onSave(row, remark.trim(), parseTags(tags))) {
      writeDraft(draftKey(row), "");
      editing = null;
    }
  }
</script>

<div class="table-wrap">
  <table>
    <thead><tr><th>{timeBasis === "added" ? "Import time" : "Log time"}</th><th>File</th>{#if showRecord}<th>Linked record</th>{/if}<th>Remarks</th><th>Tags</th><th>Actions</th></tr></thead>
    <tbody>
      {#each rows as row (row.key)}
        {@const file = row.file}
        <tr class:missing={file.missing}>
          <td class="time">
            <time title={fileTime(file, timeBasis)}>{localDateTime(fileTime(file, timeBasis)) || "Time not recorded"}</time>
            <span class="small muted">{timeBasis === "added" && file.addedAt ? "Added to record" : file.timeSource === "data" ? "Log data timestamp" : file.timeSource === "added" ? "Added to record" : file.timeSource === "record" ? "Record time (legacy)" : "File modification time"}</span>
            {#if timeBasis === "file" && file.addedAt}<span class="small muted">Imported {localDateTime(file.addedAt)}</span>{/if}
          </td>
          <td class="filename">
            <span class="mono" title={file.path}>{file.name}</span>
            <span class="small muted">{formatSize(file.size)} · {file.kind === "photo" ? "photo" : file.kind === "file" ? "attachment" : "log"}</span>
            {#if file.missing}<span class="small err">File missing — check the working directory</span>{/if}
          </td>
          {#if showRecord}<td class="record">{#if row.run}<button class="link" onclick={() => onRecord(row)} title={row.label}>{row.label}</button>{:else}<span class="muted">Unlinked</span>{/if}</td>{/if}
          <td class="remark">{file.remark || "—"}</td>
          <td class="tags">
            {#each file.tags ?? [] as tag (tag)}<button type="button" class="tag-chip" onclick={() => onTag(tag)} title={`Filter by tag: ${tag}`}>{tag}</button>{/each}
            {#if !(file.tags ?? []).length}<span class="muted">—</span>{/if}
          </td>
          <td class="actions">
            <button type="button" disabled={busy || !row.editable} onclick={() => edit(row)}>Remarks / Tags</button>
            <button type="button" disabled={busy || file.missing} onclick={() => onOpen(row)} title="Open the file's containing folder in the file manager">Show in folder</button>
          </td>
        </tr>
        {#if editing === row.key}
          <tr class="edit-row"><td colspan={showRecord ? 6 : 5}>
            <form onsubmit={(event) => { event.preventDefault(); void save(row); }}>
              <div class="field"><label for={`remark-${row.key}`}>File remarks</label><textarea id={`remark-${row.key}`} rows="3" value={remark} oninput={(event) => { remark = event.currentTarget.value; remember(row); }} disabled={busy} placeholder="e.g. First 10 seconds are warmup; segment 3 contains power interference."></textarea></div>
              <div class="field"><label for={`tags-${row.key}`}>Tags (comma-separated)</label><input id={`tags-${row.key}`} value={tags} oninput={(event) => { tags = event.currentTarget.value; remember(row); }} disabled={busy} placeholder="temperature-drift, needs-review, retest" /></div>
              <div class="edit-actions"><span class="small muted">Remarks and tags do not modify the log file itself</span><button type="button" disabled={busy} onclick={() => { writeDraft(draftKey(row), ""); editing = null; }}>Cancel</button><button type="submit" class="primary" disabled={busy}>Save</button></div>
            </form>
          </td></tr>
        {/if}
      {/each}
    </tbody>
  </table>
</div>

<style>
  .table-wrap { overflow-x: auto; }
  table { width: 100%; min-width: 720px; border-collapse: collapse; font-size: 13px; }
  th, td { text-align: left; padding: 10px; border-bottom: 1px solid var(--line); vertical-align: top; }
  th { background: var(--panel); font-size: 12px; color: var(--muted); white-space: nowrap; }
  .time { width: 180px; white-space: nowrap; font-size: 12px; }
  .small { display: block; font-size: 11px; margin-top: 3px; }
  .filename { max-width: 260px; overflow-wrap: anywhere; }
  .record { max-width: 160px; overflow-wrap: anywhere; }
  .remark { min-width: 120px; max-width: 320px; white-space: pre-wrap; overflow-wrap: anywhere; }
  .tags { min-width: 90px; max-width: 200px; }
  .actions { min-width: 104px; }
  .actions button { font-size: 12px; display: block; margin: 0 0 5px; }
  .link { border: 0; background: none; padding: 0; color: var(--accent); text-align: left; }
  .missing { background: var(--deviate); }
  .edit-row td { background: var(--panel); padding: 14px; }
  .field { margin-bottom: 10px; }
  .edit-actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .edit-actions span { flex: 1; }
</style>
