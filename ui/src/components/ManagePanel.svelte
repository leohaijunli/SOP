<script lang="ts">
  import { onMount, untrack } from "svelte";
  import * as api from "../lib/api";
  import { text } from "../lib/text";
  import type { Manifest } from "../lib/types";
  import SensorsEditor from "./SensorsEditor.svelte";
  import DevicesEditor from "./DevicesEditor.svelte";

  let { manifest, workingCopy, operator, onOperator, onDirectory, onChanged }: {
    manifest: Manifest | null; workingCopy: string; operator: string; onOperator: (name: string) => void;
    onDirectory: () => void; onChanged: () => Promise<void>;
  } = $props();
  let sensors = $state("");
  let devices = $state("");
  let projectTitle = $state(untrack(() => text(manifest?.project?.title)));
  let message = $state("");
  let bad = $state(false);
  let pending = $state(0);
  let report = $state("");
  let checking = $state(false);
  let queue: Promise<void> = Promise.resolve();

  onMount(() => {
    void api.settingsRows().then((rows) => {
      sensors = rows.find((row) => row.key === "sensors")?.value ?? "";
      devices = rows.find((row) => row.key === "devices")?.value ?? "";
    }).catch((e) => { message = String(e); bad = true; });
  });

  function saveSetting(key: string, value: string): void {
    pending++;
    queue = queue.then(async () => {
      try { await api.settingsSet(key, value); message = "Settings saved"; bad = false; }
      catch (e) { message = String(e); bad = true; }
      finally { pending--; }
    });
  }
  async function saveTitle(): Promise<void> {
    pending++;
    try { await api.projectSet("title", projectTitle.trim()); await onChanged(); message = "Project name saved"; bad = false; }
    catch (e) { message = String(e); bad = true; }
    finally { pending--; }
  }
  async function validate(): Promise<void> {
    checking = true;
    try { report = await api.validationReport(); }
    catch (e) { report = String(e); }
    finally { checking = false; }
  }
</script>

<article class="management">
  <h1>Working Directory &amp; Settings</h1>
  <section class="workspace-card">
    <h2>Current working directory</h2><p class="mono">{workingCopy}</p>
    <p class="muted">logs/ contains independent logs, testplan/ contains test plans, and runs/ contains test notes and execution records.</p>
    <button class="primary" disabled={pending > 0} onclick={onDirectory}>Switch working directory…</button>
    <p class="small muted">All data stays in the selected folder. No Git repository or automatic synchronization is required. Switching folders does not move or delete data.</p>
  </section>
  <section class="field"><label for="default-operator">Default operator (optional)</label><input id="default-operator" value={operator} onchange={(event) => onOperator(event.currentTarget.value)} placeholder="Leave blank — no sign-in required" /><span class="small muted">The name is remembered only on this machine. Each note can use a different name or leave it blank.</span></section>
  {#if manifest?.project}
    <section><h2>Project name</h2><form class="field-row" onsubmit={(event) => { event.preventDefault(); void saveTitle(); }}><input aria-label="Project name" bind:value={projectTitle} disabled={pending > 0} /><button disabled={pending > 0 || !projectTitle.trim()}>Save name</button></form></section>
  {/if}
  <details class="equipment"><summary>Equipment shortcuts (optional)</summary>
    <p class="small muted">Quick selections for checklist runs only; lightweight notes do not depend on them. Changes are saved automatically.</p>
    <div class="equipment-grid"><section><h2>Sensors / instruments</h2><SensorsEditor value={sensors} onChange={(value) => { sensors = value; saveSetting("sensors", value); }} /></section><section><h2>Other equipment</h2><DevicesEditor value={devices} onChange={(value) => { devices = value; saveSetting("devices", value); }} /></section></div>
  </details>
  <details><summary>Content validation (advanced)</summary><p class="small muted">Validate SOPs and records only when needed, rather than scanning every log each time a note is saved.</p><button disabled={checking} onclick={() => void validate()}>{checking ? "Checking…" : "Validate content"}</button>{#if report}<pre>{report}</pre>{/if}</details>
  {#if pending > 0}<p class="muted" role="status">Saving…</p>{:else if message}<p class:err={bad} role="status">{message}</p>{/if}
</article>

<style>
  .management { padding: 24px; display: flex; flex-direction: column; gap: 20px; }
  h1 { margin: 0; }
  h2 { font-size: 14px; margin: 0 0 10px; }
  p { font-size: 13px; }
  .workspace-card { padding: 18px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel); }
  .workspace-card .mono { overflow-wrap: anywhere; }
  .small { font-size: 12px; }
  summary { cursor: pointer; font-size: 14px; color: var(--muted); }
  .equipment-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 24px; margin-top: 12px; }
  .field-row { max-width: 650px; }
  .field-row input { flex: 1; min-width: 0; }
  pre { overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; font: 12px var(--mono); background: var(--panel); padding: 12px; border-radius: 6px; }
</style>
