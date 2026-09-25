<script lang="ts">
  import { onMount } from "svelte";
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

  // "add a step" form
  let draft = $state({
    id: "",
    title: "",
    kind: "check",
    severity: "normal",
    prose: "",
    captures: [] as { key: string; label: string; type: string; required: boolean }[],
  });

  // a capture being edited, keyed by step id
  let captureDraft = $state<Record<string, { key: string; label: string; type: string; required: boolean }>>({});

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  const loadProcedures = async (): Promise<void> => {
    try {
      procedures = await api.procedureChoices(file);
    } catch (e) {
      procedures = [];
    }
  };

  onMount(() => {
    file = manifest?.checklists[checklist]?.path ?? null;
    steps = manifest?.checklists[checklist]?.steps ?? [];
    void loadProcedures();
  });

  const currentFile = (): string => {
    if (file) return file;
    throw new Error("no checklist selected");
  };

  const refreshSteps = async (): Promise<void> => {
    steps = (await api.manifest()).checklists.find((c) => c.path === file)?.steps ?? [];
    await loadProcedures();
  };

  const addStep = async (): Promise<void> => {
    try {
      const out = await api.stepAdd(
        currentFile(),
        {
          id: draft.id.trim(),
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
      say(`added ${out}`);
      draft = { id: "", title: "", kind: "check", severity: "normal", prose: "", captures: [] };
      await refreshSteps();
    } catch (e) {
      say(String(e), true);
    }
  };

  const patch = async (id: string, field: "title" | "kind" | "severity" | "prose", value: string): Promise<void> => {
    try {
      await api.stepUpdate(currentFile(), id, { [field]: value });
      say(`${id} updated`);
      await refreshSteps();
    } catch (e) {
      say(String(e), true);
    }
  };

  const move = async (id: string, up: boolean): Promise<void> => {
    try {
      await api.itemMove(currentFile(), "step", id, up);
      await refreshSteps();
    } catch (e) {
      say(String(e), true);
    }
  };

  const removeStep = async (id: string): Promise<void> => {
    if (!confirm(`Remove step "${id}"?`)) return;
    try {
      await api.stepRemove(currentFile(), id);
      say(`removed ${id}`);
      await refreshSteps();
    } catch (e) {
      say(String(e), true);
    }
  };

  const addCapture = async (step: string): Promise<void> => {
    const d = captureDraft[step] ?? { key: "", label: "", type: "text", required: false };
    if (!d.key.trim()) { say("capture needs a key", true); return; }
    try {
      await api.captureSet(currentFile(), step, {
        key: d.key.trim(),
        label: d.label.trim() || d.key.trim(),
        type: d.type,
        unit: null,
        required: d.required,
        options: [],
        expected: null,
      });
      captureDraft[step] = { key: "", label: "", type: "text", required: false };
      say(`added capture to ${step}`);
      await refreshSteps();
    } catch (e) {
      say(String(e), true);
    }
  };

  const removeCapture = async (step: string, key: string): Promise<void> => {
    try {
      await api.captureRemove(currentFile(), step, key);
      say(`removed capture ${key}`);
      await refreshSteps();
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

  const captureDraftFor = (step: string) => (captureDraft[step] ??= { key: "", label: "", type: "text", required: false });
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

  <hr class="divider" />

  <section>
    <h2>Procedures to include</h2>
    <p class="desc">An include expands the procedure's steps into this checklist at the
    marker's position. Toggle to add or remove.</p>
    {#each procedures as proc (proc.path)}
      <label class="include">
        <input type="checkbox" checked={proc.included} onchange={() => void toggleInclude(proc.path)} />
        <span>{text(proc.title) || proc.path}</span>
        <span class="muted mono"> ({proc.path})</span>
      </label>
    {/each}
  </section>

  <hr class="divider" />

  <section>
    <h2>Add a step</h2>
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
  </section>

  <hr class="divider" />

  <section>
    <h2>Steps in this checklist</h2>
    {#each steps as step, index (text(step.id))}
      <div class="step-card">
        <div class="field-row">
          <span class="no muted">{index + 1}</span>
          <strong>{text(step.title)}</strong>
          <span class="muted">({text(step.id)})</span>
          <span class="spacer"></span>
          <button title="move up" onclick={() => void move(text(step.id), true)} disabled={index === 0}>&uarr;</button>
          <button title="move down" onclick={() => void move(text(step.id), false)} disabled={index === steps.length - 1}>&darr;</button>
          <button title="remove" onclick={() => void removeStep(text(step.id))}>&times;</button>
        </div>
        <div class="field">
          <label>title</label>
          <input value={text(step.title)} onchange={(e) => void patch(text(step.id), "title", (e.currentTarget as HTMLInputElement).value)} />
        </div>
        <div class="field-row">
          <div class="field">
            <label>kind</label>
            <select value={text(step.kind)} onchange={(e) => void patch(text(step.id), "kind", (e.currentTarget as HTMLSelectElement).value)}>
              <option value="check">check</option>
              <option value="measure">measure</option>
              <option value="select">select</option>
              <option value="note">note</option>
              <option value="gate">gate</option>
            </select>
          </div>
          <div class="field">
            <label>severity</label>
            <select value={text(step.severity)} onchange={(e) => void patch(text(step.id), "severity", (e.currentTarget as HTMLSelectElement).value)}>
              <option value="info">info</option>
              <option value="normal">normal</option>
              <option value="critical">critical</option>
            </select>
          </div>
        </div>
        <div class="field">
          <label>prose</label>
          <textarea rows={3} value={text(step.body)} onchange={(e) => void patch(text(step.id), "prose", (e.currentTarget as HTMLTextAreaElement).value)}></textarea>
        </div>

        <div class="captures">
          <label>captures</label>
          {#each step.captures ?? [] as capture (text(capture.key))}
            <div class="capture-row">
              <span class="mono">{text(capture.key)}</span>
              <span>&mdash; {text(capture.label)} ({text(capture.type)})</span>
              <span class="spacer"></span>
              <button onclick={() => void removeCapture(text(step.id), text(capture.key))}>&times;</button>
            </div>
          {/each}
          <div class="field-row">
            <input
              placeholder="key"
              value={captureDraftFor(text(step.id)).key}
              oninput={(e) => (captureDraftFor(text(step.id)).key = (e.currentTarget as HTMLInputElement).value)}
            />
            <input
              placeholder="label"
              value={captureDraftFor(text(step.id)).label}
              oninput={(e) => (captureDraftFor(text(step.id)).label = (e.currentTarget as HTMLInputElement).value)}
            />
            <select
              value={captureDraftFor(text(step.id)).type}
              onchange={(e) => (captureDraftFor(text(step.id)).type = (e.currentTarget as HTMLSelectElement).value)}
            >
              <option value="text">text</option>
              <option value="number">number</option>
              <option value="integer">integer</option>
              <option value="bool">bool</option>
              <option value="select">select</option>
              <option value="datetime">datetime</option>
              <option value="duration">duration</option>
              <option value="attach">attach</option>
            </select>
            <button onclick={() => void addCapture(text(step.id))}>Add</button>
          </div>
        </div>
      </div>
    {/each}
  </section>
</article>

<style>
  .spacer { flex: 1; }
  .include { display: flex; align-items: center; gap: 6px; padding: 2px 0; }
  .step-card {
    border: 1px solid var(--line); border-radius: 8px;
    padding: 10px 12px; margin-bottom: 12px; display: flex; flex-direction: column; gap: 8px;
  }
  .capture-row { display: flex; align-items: center; gap: 6px; padding: 2px 0; }
  .no { font-variant-numeric: tabular-nums; }
  .field-row input, .field-row select { flex: 1; }
  textarea { width: 100%; resize: vertical; }
  button { white-space: nowrap; }
</style>