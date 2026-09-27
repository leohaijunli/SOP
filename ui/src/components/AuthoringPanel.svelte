<script lang="ts">
  import * as api from "../lib/api";
  import { text } from "../lib/markdown";
  import type { Manifest, ProcedureChoice, StepEntry } from "../lib/types";

  let {
    manifest,
    checklist,
    onDone,
  }: {
    manifest: Manifest | null;
    checklist: number;
    onDone: () => void;
  } = $props();

  let file = $state<string | null>(null);
  let steps: StepEntry[] = $state([]);
  let procedures: ProcedureChoice[] = $state([]);
  let message = $state("");
  let isError = $state(false);
  // One step at a time: the list picks, the editor edits.
  let selected = $state(0);

  // "add a step" form
  let draft = $state({
    id: "",
    title: "",
    kind: "check",
    severity: "normal",
    prose: "",
    captures: [] as { key: string; label: string; type: string; required: boolean }[],
  });

  // The capture being added to the selected step. One step is on screen at a time, so
  // one draft is enough, and it starts empty whenever the selection moves.
  const emptyCapture = () => ({ key: "", label: "", type: "text", required: false });
  let captureDraft = $state(emptyCapture());

  const selectStep = (index: number): void => {
    selected = index;
    captureDraft = emptyCapture();
  };

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  const loadProcedures = async (target: string | null = file): Promise<void> => {
    try {
      procedures = await api.procedureChoices(target);
    } catch (e) {
      procedures = [];
    }
  };

  // The app shell loads the manifest after the first render, so this panel can mount
  // while it is still null. Follow the props instead of reading them once, otherwise
  // opening Edit early leaves it empty with no file, no steps, and no error.
  $effect(() => {
    const entry = manifest?.checklists[checklist];
    const path = entry?.path ?? null;
    file = path;
    steps = entry?.steps ?? [];
    selected = 0;
    captureDraft = emptyCapture();
    void loadProcedures(path);
  });

  const currentFile = (): string => {
    if (file) return file;
    throw new Error("no checklist selected");
  };

  // A step's own file, which is not always the checklist: steps pulled in by an include
  // live in their procedure, and that is the file an edit has to be written to.
  const sourceOf = (step: StepEntry): string => text(step.source) || file || "";

  // True when the checklist itself defines the step, rather than an included procedure.
  const isLocal = (step: StepEntry): boolean => text(step.source) === "" || text(step.source) === file;

  const refreshSteps = async (select?: string): Promise<void> => {
    // The repository manifest only knows files under checklists/; a file loaded via
    // "Open file" lives anywhere, so re-parse it from disk to pick up the edit.
    const repoManifest = await api.manifest();
    const entry = repoManifest.checklists.find((c) => c.path === file);
    let next = entry?.steps ?? [];
    if (!entry && file) {
      try {
        const ext = await api.loadExternalMd(file);
        next = ext?.steps ?? [];
      } catch {
        /* keep current */
      }
    }
    steps = next;
    const wanted = select ? steps.findIndex((s) => text(s.id) === select) : selected;
    selected = Math.max(0, Math.min(wanted, steps.length - 1));
    await loadProcedures();
  };

  const addStep = async (): Promise<void> => {
    const id = draft.id.trim();
    try {
      await api.stepAdd(
        currentFile(),
        {
          id,
          title: draft.title.trim(),
          kind: draft.kind,
          severity: draft.severity,
          deprecated: false,
          prose: draft.prose,
          captures: draft.captures.map((c) => ({
            key: c.key.trim(),
            label: c.label.trim(),
            type: c.type,
            unit: null,
            required: c.required,
            options: [],
            expected: null,
          })),
        },
        null
      );
      say(`added ${id}`);
      draft = { id: "", title: "", kind: "check", severity: "normal", prose: "", captures: [] };
      await refreshSteps(id);
    } catch (e) {
      say(String(e), true);
    }
  };

  const patch = async (step: StepEntry, field: "title" | "kind" | "severity" | "prose", value: string): Promise<void> => {
    const id = text(step.id);
    try {
      await api.stepUpdate(sourceOf(step), id, { [field]: value });
      say(`${id} updated in ${sourceOf(step)}`);
      await refreshSteps(id);
    } catch (e) {
      say(String(e), true);
    }
  };

  // Only a step the checklist defines can be moved here: a step out of a procedure moves
  // with the include marker that pulled it in, which is the checklist's own item to move.
  const move = async (step: StepEntry, up: boolean): Promise<void> => {
    try {
      await api.itemMove(currentFile(), "step", text(step.id), up);
      await refreshSteps(text(step.id));
    } catch (e) {
      say(String(e), true);
    }
  };

  const removeStep = async (step: StepEntry): Promise<void> => {
    const id = text(step.id);
    if (!confirm(`Remove step "${id}" from ${sourceOf(step)}?`)) return;
    try {
      await api.stepRemove(sourceOf(step), id);
      say(`removed ${id}`);
      await refreshSteps();
    } catch (e) {
      say(String(e), true);
    }
  };

  const addCapture = async (step: StepEntry): Promise<void> => {
    const id = text(step.id);
    const d = captureDraft;
    if (!d.key.trim()) { say("capture needs a key", true); return; }
    try {
      await api.captureSet(sourceOf(step), id, {
        key: d.key.trim(),
        label: d.label.trim() || d.key.trim(),
        type: d.type,
        unit: null,
        required: d.required,
        options: [],
        expected: null,
      });
      say(`added capture to ${id}`);
      captureDraft = emptyCapture();
      await refreshSteps(id);
    } catch (e) {
      say(String(e), true);
    }
  };

  const removeCapture = async (step: StepEntry, key: string): Promise<void> => {
    try {
      await api.captureRemove(sourceOf(step), text(step.id), key);
      say(`removed capture ${key}`);
      await refreshSteps(text(step.id));
    } catch (e) {
      say(String(e), true);
    }
  };

  const toggleInclude = async (target: string): Promise<void> => {
    try {
      const proc = procedures.find((p) => p.path === target);
      if (proc?.included) {
        await api.includeRemove(currentFile(), target);
      } else {
        await api.includeAdd(currentFile(), target);
      }
      await refreshSteps();
    } catch (e) {
      say(String(e), true);
    }
  };

  const current: StepEntry | null = $derived(steps[selected] ?? null);
  const includedCount: number = $derived(procedures.filter((p) => p.included).length);
</script>

<article class="authoring">
  <div class="toolbar">
    <h1 style="margin:0">Authoring</h1>
    <span class="spacer"></span>
    <button onclick={onDone}>Back to run</button>
  </div>
  <p class="muted mono">{file}</p>
  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}

  <details class="fold">
    <summary>Procedures to include ({includedCount} of {procedures.length})</summary>
    <p class="desc">An include expands the procedure's steps into this checklist at the
    marker's position. Toggle to add or remove.</p>
    {#each procedures as proc (proc.path)}
      <label class="include">
        <input type="checkbox" checked={proc.included} onchange={() => void toggleInclude(proc.path)} />
        <span>{text(proc.title) || proc.path}</span>
        <span class="muted mono"> ({proc.path})</span>
      </label>
    {/each}
  </details>

  <details class="fold">
    <summary>Add a step</summary>
    <div class="field">
      <label>id</label>
      <input bind:value={draft.id} placeholder="stable id, never reused" />
    </div>
    <div class="field">
      <label>title</label>
      <input bind:value={draft.title} />
    </div>
    <div class="field-row">
      <div class="field">
        <label>kind</label>
        <select bind:value={draft.kind}>
          <option value="check">check</option>
          <option value="measure">measure</option>
          <option value="select">select</option>
          <option value="note">note</option>
          <option value="gate">gate</option>
        </select>
      </div>
      <div class="field">
        <label>severity</label>
        <select bind:value={draft.severity}>
          <option value="info">info</option>
          <option value="normal">normal</option>
          <option value="critical">critical</option>
        </select>
      </div>
    </div>
    <div class="field">
      <label>prose (Markdown)</label>
      <textarea bind:value={draft.prose} rows={5}></textarea>
    </div>
    <button class="primary" onclick={() => void addStep()} disabled={!draft.id.trim()}>Add step</button>
  </details>

  <hr class="divider" />

  <section>
    <h2>Steps in this checklist ({steps.length})</h2>
    {#if !steps.length}
      <p class="empty">No steps resolved for this checklist yet.</p>
    {:else}
      <div class="step-layout">
        <nav class="step-nav">
          <ol>
            {#each steps as step, index (text(step.id))}
              <li>
                <button class:current={index === selected} onclick={() => selectStep(index)}>
                  <span class="label"><span class="no muted">{index + 1}</span> {text(step.title)}</span>
                  <span class="id mono">{text(step.id)}{#if !isLocal(step)} &middot; {text(step.source)}{/if}</span>
                </button>
              </li>
            {/each}
          </ol>
        </nav>

        {#if current}
          <div class="step-editor">
            <div class="field-row">
              <strong>{text(current.title)}</strong>
              <span class="muted mono">{text(current.id)}</span>
              <span class="spacer"></span>
              {#if isLocal(current)}
                <button title="move up" onclick={() => void move(current, true)} disabled={selected === 0}>&uarr;</button>
                <button title="move down" onclick={() => void move(current, false)} disabled={selected === steps.length - 1}>&darr;</button>
                <button title="remove" onclick={() => void removeStep(current)}>&times;</button>
              {/if}
            </div>

            <p class="desc">
              {#if isLocal(current)}
                Defined by this checklist (<span class="mono">{file}</span>).
              {:else}
                Defined by <span class="mono">{text(current.source)}</span>. Edits here change
                that procedure for every checklist that includes it. Order is set by the
                include marker, not by this step.
              {/if}
            </p>

            <div class="field">
              <label>title</label>
              <input value={text(current.title)} onchange={(e) => void patch(current!, "title", (e.currentTarget as HTMLInputElement).value)} />
            </div>
            <div class="field-row">
              <div class="field">
                <label>kind</label>
                <select value={text(current.kind)} onchange={(e) => void patch(current!, "kind", (e.currentTarget as HTMLSelectElement).value)}>
                  <option value="check">check</option>
                  <option value="measure">measure</option>
                  <option value="select">select</option>
                  <option value="note">note</option>
                  <option value="gate">gate</option>
                </select>
              </div>
              <div class="field">
                <label>severity</label>
                <select value={text(current.severity)} onchange={(e) => void patch(current!, "severity", (e.currentTarget as HTMLSelectElement).value)}>
                  <option value="info">info</option>
                  <option value="normal">normal</option>
                  <option value="critical">critical</option>
                </select>
              </div>
            </div>
            <div class="field">
              <label>prose (Markdown, including any `- [ ]` items)</label>
              <textarea rows={10} value={text(current.body)} onchange={(e) => void patch(current!, "prose", (e.currentTarget as HTMLTextAreaElement).value)}></textarea>
            </div>

            <div class="captures">
              <label>captures</label>
              {#each current.captures ?? [] as capture (text(capture.key))}
                <div class="capture-row">
                  <span class="mono">{text(capture.key)}</span>
                  <span>&mdash; {text(capture.label)} ({text(capture.type)})</span>
                  <span class="spacer"></span>
                  <button onclick={() => void removeCapture(current!, text(capture.key))}>&times;</button>
                </div>
              {/each}
              <div class="field-row">
                <input
                  placeholder="key"
                  bind:value={captureDraft.key}
                />
                <input
                  placeholder="label"
                  bind:value={captureDraft.label}
                />
                <select bind:value={captureDraft.type}>
                  <option value="text">text</option>
                  <option value="number">number</option>
                  <option value="integer">integer</option>
                  <option value="bool">bool</option>
                  <option value="select">select</option>
                  <option value="datetime">datetime</option>
                  <option value="duration">duration</option>
                  <option value="attach">attach</option>
                </select>
                <button onclick={() => void addCapture(current!)}>Add</button>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </section>
</article>

<style>
  .spacer { flex: 1; }
  .fold { margin-bottom: 8px; }
  summary { cursor: pointer; font-weight: 600; padding: 2px 0; }
  .include { display: flex; align-items: center; gap: 6px; padding: 2px 0; }
  .step-layout {
    display: grid; grid-template-columns: 260px minmax(0, 1fr); gap: 12px; align-items: start;
  }
  .step-nav { border-right: 1px solid var(--line); padding-right: 8px; max-height: 72vh; overflow: auto; }
  .step-nav ol { list-style: none; margin: 0; padding: 0; }
  .step-nav button {
    display: block; width: 100%; text-align: left; margin: 0 0 2px; padding: 4px 6px;
    background: none; border: 1px solid transparent;
  }
  .step-nav button.current { border-color: var(--line); background: var(--quote); }
  .step-nav .label { display: block; }
  .step-nav .id { display: block; font-size: 11px; color: var(--muted); }
  .step-editor { display: flex; flex-direction: column; gap: 8px; }
  .capture-row { display: flex; align-items: center; gap: 6px; padding: 2px 0; }
  .no { font-variant-numeric: tabular-nums; }
  .field-row input, .field-row select { flex: 1; }
  textarea { width: 100%; resize: vertical; font-family: var(--mono); font-size: 12px; }
  button { white-space: nowrap; }
</style>
