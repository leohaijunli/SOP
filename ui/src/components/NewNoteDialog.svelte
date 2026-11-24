<script lang="ts">
  import { onMount, untrack } from "svelte";
  import * as api from "../lib/api";
  import { readDraft, writeDraft } from "../lib/drafts";
  import type { RunView } from "../lib/types";

  let { open, workspace, operator, sites, onCreated, onCancel }: {
    open: boolean; workspace: string; operator: string; sites: string[];
    onCreated: (run: RunView) => Promise<void>; onCancel: () => void;
  } = $props();
  let dialog = $state<HTMLDialogElement | null>(null);
  let field = $state<HTMLInputElement | null>(null);
  let title = $state("");
  let note = $state("");
  let site = $state("");
  let author = $state(untrack(() => operator));
  let model = $state("");
  let serial = $state("");
  let busy = $state(false);
  let error = $state("");
  let loaded = $state(false);

  onMount(() => {
    try {
      const draft = JSON.parse(readDraft(`${workspace}::new-note`) || "{}");
      title = draft.title ?? ""; note = draft.note ?? ""; site = draft.site ?? "";
      model = draft.model ?? ""; serial = draft.serial ?? "";
    } catch { /* A malformed local draft does not prevent a new record. */ }
    loaded = true;
  });

  $effect(() => {
    if (loaded) writeDraft(`${workspace}::new-note`, title || note || site || model || serial
      ? JSON.stringify({ title, note, site, model, serial }) : "");
  });
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      author = operator;
      error = "";
      dialog.showModal();
      field?.focus();
    } else if (!open && dialog.open) dialog.close();
  });

  async function create(): Promise<void> {
    if (busy || !title.trim()) return;
    busy = true;
    error = "";
    try {
      const run = await api.runNoteCreate(title.trim(), note.trim(), author.trim(), site.trim(), {
        sensorModel: model.trim() || null, sensorSerial: serial.trim() || null,
        sensorFirmware: null, hardware: [], conditions: {},
      });
      // Once creation succeeds, do not retry it because a subsequent refresh failed.
      title = ""; note = ""; site = ""; model = ""; serial = "";
      writeDraft(`${workspace}::new-note`, "");
      await onCreated(run);
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="new-note-title" oncancel={(event) => {
  event.preventDefault(); if (!busy) onCancel();
}}>
  <form onsubmit={(event) => { event.preventDefault(); void create(); }}>
    <h2 id="new-note-title">New Test Note</h2>
    <p class="muted">Only a title is required. The date and ID are generated automatically. Add more notes and logs whenever you need them.</p>
    <div class="field">
      <label for="note-title">Title *</label>
      <input id="note-title" bind:this={field} bind:value={title} placeholder="e.g. RM3100 temperature drift retest" disabled={busy} required />
    </div>
    <div class="field">
      <label for="note-content">Experiment notes (optional, Markdown supported)</label>
      <textarea id="note-content" rows="6" bind:value={note} disabled={busy} placeholder="Briefly record the purpose, actions, observations, or conclusion. No full report is required."></textarea>
    </div>
    <details>
      <summary>Location, operator, or equipment (optional)</summary>
      <div class="optional">
        <div class="field"><label for="note-site">Location</label><input id="note-site" list="note-sites" bind:value={site} disabled={busy} /></div>
        <div class="field"><label for="note-author">Operator</label><input id="note-author" bind:value={author} disabled={busy} /></div>
        <div class="field"><label for="note-model">Equipment / model</label><input id="note-model" bind:value={model} disabled={busy} /></div>
        <div class="field"><label for="note-serial">Serial number</label><input id="note-serial" bind:value={serial} disabled={busy} /></div>
      </div>
    </details>
    <datalist id="note-sites">{#each sites as option (option)}<option value={option}></option>{/each}</datalist>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    <div class="actions">
      <span class="muted">Unsubmitted drafts are saved on this machine</span>
      <button type="button" onclick={onCancel} disabled={busy}>Continue later</button>
      <button type="submit" class="primary" disabled={busy || !title.trim()}>{busy ? "Creating…" : "Create note"}</button>
    </div>
  </form>
</dialog>

<style>
  dialog { width: min(680px, 92vw); max-height: 88vh; overflow: auto; padding: 24px; border: 1px solid var(--line); border-radius: 12px; background: var(--panel); color: var(--ink); }
  dialog::backdrop { background: var(--overlay); }
  h2 { margin: 0 0 8px; font-size: 20px; }
  p { font-size: 13px; }
  .field { margin-bottom: 14px; }
  textarea { resize: vertical; font-size: 14px; }
  summary { cursor: pointer; color: var(--muted); font-size: 13px; }
  .optional { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 12px; }
  .optional .field { margin: 0; }
  .actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 20px; }
  .actions span { flex: 1; font-size: 11px; }
  @media (max-width: 540px) { .optional { grid-template-columns: 1fr; } }
</style>
