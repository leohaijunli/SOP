<script lang="ts">
  import { onMount } from "svelte";
  import { markdown, text } from "../lib/markdown";
  import * as api from "../lib/api";
  import type { Manifest, RunStepView, RunView } from "../lib/types";

  let {
    manifest,
    checklist,
  }: {
    manifest: Manifest | null;
    checklist: number;
  } = $props();

  const sop = $derived(
    manifest?.checklists[checklist] ? api.text(manifest.checklists[checklist].sop_id) : ""
  );
  const isDraft = $derived(api.text(manifest?.checklists[checklist]?.status) === "draft");

  let runId = $state("");
  let operator = $state("");
  let site = $state("");
  let overrideReason = $state("");
  let message = $state("");
  let isError = $state(false);

  let run = $state<RunView | null>(null);
  let currentStep = $state(0);
  let stepQuery = $state("");
  let noteText = $state("");

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  const start = async (): Promise<void> => {
    try {
      run = await api.runStart(sop, runId.trim(), operator.trim(), site.trim(), overrideReason.trim() || null);
      currentStep = 0;
      message = `started ${runId.trim()}`;
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  const emit = async (event: any): Promise<void> => {
    if (!run) return;
    try {
      run = await api.runRecord(run.sop, run.runId, { at: new Date().toISOString(), ...event });
      message = "saved";
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  const setStatus = async (step: RunStepView, status: string): Promise<void> => {
    if (status === "skipped" || status === "deviated") {
      const reason = prompt(`Reason for ${status}:`);
      if (!reason) {
        say("a reason is required", true);
        return;
      }
      await emit({ type: "StepStatusChanged", step: step.id, status, reason });
    } else {
      await emit({ type: "StepStatusChanged", step: step.id, status, reason: null });
    }
  };

  const toggleCheckbox = async (step: RunStepView, index: number, checked: boolean): Promise<void> => {
    await emit({ type: "CheckboxToggled", step: step.id, index, checked });
  };

  const recordCapture = async (step: RunStepView, key: string, value: string): Promise<void> => {
    if (value.trim() === "") {
      await emit({ type: "CaptureCleared", step: step.id, key, reason: "cleared" });
      return;
    }
    const unit = step.captures.find((c) => c.key === key)?.unit ?? null;
    await emit({ type: "CaptureRecorded", step: step.id, key, value: value.trim(), unit });
  };

  const addNote = async (step: RunStepView | null): Promise<void> => {
    const t = noteText.trim();
    if (!t) return;
    await emit({ type: "NoteAdded", step: step?.id ?? null, text: t });
    noteText = "";
  };

  const end = async (status: string): Promise<void> => {
    if (!run) return;
    try {
      run = await api.runEnd(run.sop, run.runId, status);
      message = `ended ${run.runId} (${status})`;
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  const visibleSteps: RunStepView[] = $derived(
    (() => {
      const r: RunView | null = run;
      if (!r) return [];
      const q = stepQuery.trim().toLowerCase();
      if (!q) return r.steps;
      return r.steps.filter((s: RunStepView) =>
        [s.id, s.title, s.kind, s.severity, s.status].some((f) => text(f).toLowerCase().includes(q))
      );
    })()
  );

  const current: RunStepView | null = $derived(
    (() => {
      const r: RunView | null = run;
      return r ? r.steps[currentStep] ?? null : null;
    })()
  );

  function expectedOk(value: string, expected: unknown): boolean {
    if (expected === null || expected === undefined) return true;
    if (typeof expected === "object" && ("min" in (expected as object) || "max" in (expected as object))) {
      const n = Number(value);
      if (Number.isNaN(n)) return true;
      const e = expected as { min?: number; max?: number };
      if (e.min !== undefined && n < e.min) return false;
      if (e.max !== undefined && n > e.max) return false;
      return true;
    }
    return text(expected) === value;
  }

  onMount(() => {
    // Resume the most recent unfinished run of this checklist, if any.
    void api.manifest().then(async (m) => {
      const runs = m.runs.filter((r) => api.text(r.sop) === sop).sort().reverse();
      for (const r of runs.slice(0, 1)) {
        const id = String(api.text(r.run_id));
        try {
          const loaded = await api.runState(sop, id);
          if (!loaded.ended) {
            run = loaded;
            message = `resumed ${loaded.runId}`;
          }
        } catch {
          /* not resumable */
        }
      }
    });
  });
</script>

<article class="exec">
  <div class="toolbar">
    <h1 style="margin:0">Run</h1>
    <span class="muted mono">{sop}</span>
    <span class="spacer"></span>
    {#if run}
      <span class="muted">
        {run!.runId} &middot; {run!.operator ?? "?"} &middot; {run!.site ?? "?"}{#if run!.runStatus} &middot; {run!.runStatus}{/if}
      </span>
    {/if}
  </div>

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}

  {#if !run}
    <section class="start-panel">
      <h2>Start a run</h2>
      {#if isDraft}
        <p class="warn">This checklist is a draft. It cannot start without an override reason.</p>
      {/if}
      <div class="field-row">
        <div class="field"><label>run id</label><input placeholder="2026-09-25-site-01" bind:value={runId} /></div>
        <div class="field"><label>operator</label><input placeholder="your name" bind:value={operator} /></div>
        <div class="field"><label>site</label><input placeholder="Renfrew 395" bind:value={site} /></div>
      </div>
      {#if isDraft}
        <div class="field">
          <label>override reason (required to start a draft)</label>
          <input bind:value={overrideReason} />
        </div>
      {/if}
      <button
        class="primary"
        onclick={() => void start()}
        disabled={!runId.trim() || !operator.trim() || !site.trim() || (isDraft && !overrideReason.trim())}
      >
        Start run
      </button>
      <p class="muted" style="margin-top:8px">
        A snapshot of the checklist is frozen and an event log is opened. The run can be
        resumed after a crash by replaying the log.
      </p>
    </section>
  {:else}
    <div class="exec-layout">
      <nav>
        <label for="stepfilter">Filter steps</label>
        <input id="stepfilter" type="search" bind:value={stepQuery} />
        <ol class="steps">
          {#each visibleSteps as s, i (s.id)}
            <li aria-current={run!.steps.indexOf(s) === currentStep} onclick={() => (currentStep = run!.steps.indexOf(s))}>
              <span class="title">
                <span class="dot {text(s.severity)}"></span>
                <span class="no">{i + 1}</span>{s.title}
              </span>
              <span class="meta">{s.status}{s.reason ? ` \u2014 ${s.reason}` : ""}</span>
            </li>
          {/each}
        </ol>
        <hr class="divider" />
        <button onclick={() => void end("complete")}>End: complete</button>
        <button onclick={() => void end("partial")}>End: partial</button>
        <button onclick={() => void end("aborted")}>End: aborted</button>
      </nav>

      {#if current}
        <article class="step-detail">
          <div class="badges">
            <span class="badge {text(current.severity)}">{text(current.severity)}</span>
            <span class="badge">{text(current.kind)}</span>
            <span class="badge">{current.status}</span>
            <span class="badge">{current.id}</span>
          </div>
          <h2>{current.title}</h2>
          <div class="prose">{@html markdown(current.prose)}</div>

          {#if current.captures.length}
            <section class="captures">
              <h3>Captures</h3>
              {#each current.captures as cap (cap.key)}
                <div class="capture">
                  <label>{cap.label ?? cap.key}{#if cap.required}<span class="req">*</span>{/if}</label>
                  <div class="hint mono">{cap.key} &middot; {cap.type}{#if cap.unit} {cap.unit}{/if}</div>
                  <div class="field-row">
                    {#if cap.type === "select" && cap.options?.length}
                      <select value={cap.value ?? ""} onchange={(e) => void recordCapture(current, cap.key, (e.currentTarget as HTMLSelectElement).value)}>
                        <option value="">(unset)</option>
                        {#each cap.options as option (option)}
                          <option value={option}>{option}</option>
                        {/each}
                      </select>
                    {:else if cap.type === "bool"}
                      <select value={cap.value ?? ""} onchange={(e) => void recordCapture(current, cap.key, (e.currentTarget as HTMLSelectElement).value)}>
                        <option value="">(unset)</option>
                        <option value="true">true</option>
                        <option value="false">false</option>
                      </select>
                    {:else}
                      <input
                        type="text"
                        placeholder={cap.value ?? "enter value"}
                        onchange={(e) => void recordCapture(current, cap.key, (e.currentTarget as HTMLInputElement).value)}
                      />
                    {/if}
                  </div>
                  {#if cap.value}
                    <div class="muted">recorded: {cap.value}</div>
                  {/if}
                  {#if cap.value && cap.expected !== null && cap.expected !== undefined && !expectedOk(cap.value, cap.expected)}
                    <div class="expected">outside the expected range &mdash; acknowledge it</div>
                  {/if}
                </div>
              {/each}
            </section>
          {/if}

          {#if current.checkboxes.length}
            <section class="captures">
              <h3>Checklist</h3>
              {#each current.checkboxes as checked, index (index)}
                <label class="task">
                  <input type="checkbox" checked={checked} onchange={(e) => void toggleCheckbox(current, index, (e.currentTarget as HTMLInputElement).checked)} />
                  Item {index + 1}
                </label>
              {/each}
            </section>
          {/if}

          <div class="toolbar">
            <button onclick={() => void setStatus(current, "done")}>Mark done</button>
            <button onclick={() => void setStatus(current, "skipped")}>Skip</button>
            <button onclick={() => void setStatus(current, "deviated")}>Deviate</button>
          </div>
          <p class="muted">Skips and deviations require a reason; it is written into the record.</p>

          <div class="field">
            <label>Note for this step</label>
            <div class="field-row">
              <input bind:value={noteText} onkeydown={(e) => { if (e.key === "Enter") void addNote(current); }} />
              <button onclick={() => void addNote(current)}>Add note</button>
            </div>
          </div>
        </article>
      {/if}
    </div>
  {/if}
</article>

<style>
  .spacer { flex: 1; }
  .toolbar { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }
  .start-panel { max-width: 640px; }
  .start-panel .field { margin-bottom: 8px; }
  .exec-layout { display: grid; grid-template-columns: 280px minmax(0, 1fr); gap: 16px; }
  .exec-layout nav { border-right: 1px solid var(--line); padding-right: 12px; }
  .exec-layout nav button { width: 100%; margin-top: 4px; font-size: 12px; }
  .warn { color: var(--warn); }
  .step-detail .captures h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); }
  .task { display: block; padding: 2px 0; }
  .expected { color: var(--warn); font-size: 12px; margin-top: 6px; }
</style>