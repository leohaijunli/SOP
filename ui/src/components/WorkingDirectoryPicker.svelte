<script lang="ts">
  import * as api from "../lib/api";
  import type { WorkingDirectory } from "../lib/types";

  let { open, directory, onSelect, onCancel }: {
    open: boolean;
    directory: WorkingDirectory | null;
    onSelect: (path: string) => Promise<void>;
    onCancel: () => void;
  } = $props();

  let dialog = $state<HTMLDialogElement | null>(null);
  let field = $state<HTMLInputElement | null>(null);
  let path = $state("");
  let busy = $state(false);
  let error = $state("");

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      path = directory?.path ?? directory?.configuredPath ?? "";
      error = "";
      dialog.showModal();
      field?.focus();
    } else if (!open && dialog.open) dialog.close();
  });

  async function select(value: string): Promise<void> {
    if (busy || !value.trim()) return;
    busy = true;
    error = "";
    try { await onSelect(value.trim()); }
    catch (e) { error = String(e); }
    finally { busy = false; }
  }

  async function browse(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const chosen = await api.pickDirectory();
      busy = false;
      if (chosen) { path = chosen; await select(chosen); }
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="directory-title" oncancel={(event) => {
  event.preventDefault();
  if (directory?.path && !busy) onCancel();
}}>
  <h2 id="directory-title">Choose Working Directory</h2>
  <p>Select the folder containing your <strong>logs, test plans, and experiment records</strong>, not the application source code.</p>
  <div class="layout-hint mono">
    working-directory/<br />
    &nbsp; logs/ &nbsp; Independent log files<br />
    &nbsp; testplan/ &nbsp; Test plans (optional)<br />
    &nbsp; runs/ &nbsp; Test notes and existing run records
  </div>
  <p class="muted">An existing data folder or an empty folder is fine. No Git repository is required. Your selection is remembered on this machine.</p>
  {#if directory?.configuredPath && !directory.path}
    <p class="err">The previously selected directory is unavailable. Please choose another folder.</p>
  {/if}
  <form onsubmit={(event) => { event.preventDefault(); void select(path); }}>
    <label for="working-directory">Working directory path</label>
    <div class="field-row">
      <input id="working-directory" bind:this={field} bind:value={path} placeholder="D:\\TestData or \\\\server\\test-data" disabled={busy} />
      <button type="button" onclick={() => void browse()} disabled={busy}>Browse…</button>
    </div>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
    <div class="actions">
      {#if directory?.path}<button type="button" onclick={onCancel} disabled={busy}>Cancel</button>{/if}
      <button type="submit" class="primary" disabled={busy || !path.trim()}>{busy ? "Opening…" : "Use this directory"}</button>
    </div>
  </form>
  {#if directory?.recent.length}
    <details>
      <summary>Recent directories</summary>
      <div class="recent">
        {#each directory.recent as entry (entry)}
          <button type="button" disabled={busy} onclick={() => void select(entry)} title={entry}>{entry}</button>
        {/each}
      </div>
    </details>
  {/if}
</dialog>

<style>
  dialog { width: min(680px, 92vw); max-height: 88vh; overflow: auto; background: var(--panel); color: var(--ink); border: 1px solid var(--line); border-radius: 12px; padding: 24px; }
  dialog::backdrop { background: var(--overlay); }
  h2 { margin: 0 0 12px; font-size: 20px; }
  p { font-size: 13px; }
  .layout-hint { padding: 12px; background: var(--bg-dark); border-radius: 6px; line-height: 1.8; }
  label { display: block; margin-bottom: 6px; }
  input { flex: 1; min-width: 0; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
  details { margin-top: 16px; font-size: 13px; }
  summary { cursor: pointer; }
  .recent { display: flex; flex-direction: column; gap: 6px; margin-top: 8px; }
  .recent button { text-align: left; overflow-wrap: anywhere; font-size: 12px; }
</style>
