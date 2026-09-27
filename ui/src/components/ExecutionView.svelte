<script lang="ts">
  import { onMount } from "svelte";
  import { markdown, text } from "../lib/markdown";
  import * as api from "../lib/api";
  import type { Manifest, RunStepView, RunView } from "../lib/types";

  let {
    manifest,
    checklist,
    onRunUpdate,
  }: {
    manifest: Manifest | null;
    checklist: number;
    onRunUpdate: () => void;
  } = $props();

  const sop = $derived(
    manifest?.checklists[checklist] ? api.text(manifest.checklists[checklist].sop_id) : ""
  );
  // External files loaded via "Open file" live anywhere, so the backend has to be given
  // the file path to find the checklist; repo checklists are found by sop_id. `run.sop`
  // returned by the backend is always the checklist id, which is what record/end use.
  const runSop = $derived(
    (() => {
      const entry = manifest?.checklists[checklist];
      if (!entry) return "";
      const p = api.text(entry.path);
      const external = p.startsWith("/") || /^[A-Za-z]:[\\/]/.test(p);
      return external ? p : api.text(entry.sop_id);
    })()
  );
  const isDraft = $derived(api.text(manifest?.checklists[checklist]?.status) === "draft");

  let runId = $state("");
  let operator = $state("");
  let site = $state("");
  let overrideReason = $state("");
  let sensorModel = $state("");
  let sensorSerial = $state("");
  let sensorFirmware = $state("");
  let hardwareText = $state("");
  let conditionsText = $state("");
  let message = $state("");
  let isError = $state(false);

  let run = $state<RunView | null>(null);
  let currentStep = $state(0);
  let stepQuery = $state("");
  let noteText = $state("");
  let noteInput = $state<HTMLInputElement | null>(null);
  let capturesBox = $state<HTMLElement | null>(null);
  // Set when the run has ended, so a prominent confirmation is shown over the finished
  // run instead of a small line of text the operator may miss.
  let ended = $state<{ status: string; runId: string } | null>(null);

  // Clear the finished-run confirmation and go back to the start-a-run form.
  const newRun = (): void => {
    run = null;
    ended = null;
    runId = "";
    site = "";
    currentStep = 0;
  };

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  // Slugify a site name into the id-friendly form used by run ids.
  const slug = (value: string): string =>
    value
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "site";

  const today = (): string => {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  };

  // Suggest `YYYY-MM-DD-<site>-NN`, skipping the number suffixes this checklist has
  // already used, so the operator rarely has to think of an id and never collides with
  // an existing run. An id the operator typed themselves is always kept.
  $effect(() => {
    if (run || runId.trim()) return;
    const base = `${today()}-${slug(site)}`;
    const existing = new Set(
      (manifest?.runs ?? [])
        .filter((r) => api.text(r.sop) === sop)
        .map((r) => api.text(r.run_id))
    );
    let candidate = `${base}-01`;
    for (let i = 2; existing.has(candidate); i++) {
      candidate = `${base}-${String(i).padStart(2, "0")}`;
    }
    runId = candidate;
  });

  // One item per line; blank lines and surrounding space are dropped.
  const parseLines = (value: string): string[] =>
    value
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line !== "");

  // `key: value` per line, so a checklist can add a dimension without a form change.
  const parseConditions = (value: string): Record<string, string> => {
    const out: Record<string, string> = {};
    for (const line of value.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      const separator = trimmed.indexOf(":");
      if (separator < 0) continue;
      const key = trimmed.slice(0, separator).trim();
      const item = trimmed.slice(separator + 1).trim();
      if (key && item) out[key] = item;
    }
    return out;
  };

  const start = async (): Promise<void> => {
    try {
      run = await api.runStart(runSop, runId.trim(), operator.trim(), site.trim(), overrideReason.trim() || null, {
        sensorModel: sensorModel.trim() || null,
        sensorSerial: sensorSerial.trim() || null,
        sensorFirmware: sensorFirmware.trim() || null,
        hardware: parseLines(hardwareText),
        conditions: parseConditions(conditionsText),
      });
      currentStep = 0;
      message = `started ${runId.trim()}`;
      isError = false;
      onRunUpdate();
    } catch (e) {
      say(String(e), true);
    }
  };

  // One event at a time. `enter` on the last capture records a value and marks the step
  // done in the same keystroke, and two concurrent `run_record` calls could otherwise
  // interleave their appends to the log and leave the view showing the older response.
  let queue: Promise<void> = Promise.resolve();
  const emit = (event: any): Promise<void> => {
    queue = queue.then(async () => {
      if (!run) return;
      try {
        run = await api.runRecord(run.sop, run.runId, { at: new Date().toISOString(), ...event });
        message = "saved";
        isError = false;
      } catch (e) {
        say(String(e), true);
      }
    });
    return queue;
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

  // Attach a data file or photo to the run. The shell copies it into the run's logs,
  // hashes it, and records a `logs:` entry in the record, so the experiment and its data
  // travel together.
  const attach = async (step: RunStepView | null): Promise<void> => {
    if (!run) return;
    const path = await api.pickDataFile();
    if (!path) return;
    try {
      run = await api.runAttach(run.sop, run.runId, step?.id ?? null, path);
      message = `attached ${path.split(/[\\/]/).pop()}`;
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  const end = (status: string): Promise<void> => {
    // A `complete` run must account for every step; ending a run with open steps as
    // complete writes an invalid record that then blocks every content edit. Require
    // the operator to finish the steps, or mark the run partial/aborted instead.
    if (status === "complete" && run) {
      const open = run.steps.filter((s) => !["done", "skipped", "deviated"].includes(s.status));
      if (open.length > 0) {
        say(`${open.length} step(s) have no outcome yet; finish or skip them, or end as partial/aborted`, true);
        return Promise.resolve();
      }
    }
    // Queued with the events, so ending cannot overtake a capture still being written.
    queue = queue.then(async () => {
      if (!run) return;
      try {
        run = await api.runEnd(run.sop, run.runId, status);
        ended = { status, runId: run.runId };
        message = `ended ${run.runId} (${status})`;
        isError = false;
        onRunUpdate();
      } catch (e) {
        say(String(e), true);
      }
    });
    return queue;
  };

  // The keyboard contract in `help/app-basics.md`: field use is one-handed, so every
  // action on the run screen has a key. `F1` and `/` belong to the shell, so they are
  // left alone here; `ctrl+enter` finishes the run from inside a field, where the
  // operator's hands usually are.
  const onKey = (event: KeyboardEvent): void => {
    const target = event.target as HTMLElement;
    const inField = target.matches("input, select, textarea");
    if (event.ctrlKey && event.key === "Enter") {
      event.preventDefault();
      void end("complete");
      return;
    }
    if (inField || event.key === "F1" || event.key === "/" || !run || !current) return;
    const index = run.steps.indexOf(current);
    if (event.key === " ") {
      event.preventDefault();
      void setStatus(current, "done");
    } else if (event.key === "j" && index < run.steps.length - 1) {
      currentStep = index + 1;
    } else if (event.key === "k" && index > 0) {
      currentStep = index - 1;
    } else if (event.key === "s") {
      void setStatus(current, "skipped");
    } else if (event.key === "d") {
      void setStatus(current, "deviated");
    } else if (event.key === "n") {
      event.preventDefault();
      noteInput?.focus();
    }
  };

  // `enter` walks the captures of the current step and finishes the step after the last
  // one (`help/app-basics.md`). The default is left alone so the browser still fires
  // `change`, which is what records the value the operator just typed.
  const onCaptureKey = (event: KeyboardEvent): void => {
    if (event.key !== "Enter") return;
    const target = event.target as HTMLElement;
    if (!target.matches("input, select")) return;
    const fields = Array.from(capturesBox?.querySelectorAll<HTMLElement>("input, select") ?? []);
    const next = fields[fields.indexOf(target) + 1];
    if (next) {
      next.focus();
    } else if (current) {
      void setStatus(current, "done");
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
      const runs = m.runs
        .filter((r) => api.text(r.sop) === sop)
        .sort((a, b) => api.text(b.started).localeCompare(api.text(a.started)));
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

<svelte:window onkeydown={onKey} />

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

  {#if ended}
    <div class="ended-banner" role="status">
      <div class="ended-main">
        <span class="ended-check">&#10003;</span>
        <div>
          <strong>Run {ended.status.toUpperCase()}</strong>
          <span class="ended-id">{ended.runId}</span>
        </div>
      </div>
      <button class="primary" onclick={() => void newRun()}>Start new run</button>
    </div>
  {/if}

  {#if run && (run.sensor || run.hardware.length || Object.keys(run.conditions).length)}
    <p class="muted run-meta">
      {#if run.sensor}
        <span>
          sensor: {[run.sensor.model, run.sensor.serial && `serial ${run.sensor.serial}`, run.sensor.firmware && `firmware ${run.sensor.firmware}`]
            .filter(Boolean)
            .join(", ")}
        </span>
      {/if}
      {#if run.hardware.length}<span>hardware: {run.hardware.join(", ")}</span>{/if}
      {#if Object.keys(run.conditions).length}
        <span>
          conditions: {Object.entries(run.conditions)
            .map(([key, value]) => `${key}=${value}`)
            .join(", ")}
        </span>
      {/if}
    </p>
  {/if}

  {#if !run}
    <section class="start-panel">
      <h2>Start a run</h2>
      {#if isDraft}
        <p class="warn">This checklist is a draft. It cannot start without an override reason.</p>
      {/if}
      <div class="field-row">
        <div class="field">
          <label for="run-id">run id</label>
          <input id="run-id" placeholder={`${today()}-${slug(site)}-01`} bind:value={runId} />
          <span class="desc">Auto-suggested from today's date and the site; a suffix avoids a duplicate.</span>
        </div>
        <div class="field"><label for="operator">operator</label><input id="operator" placeholder="your name" bind:value={operator} /></div>
        <div class="field"><label for="site">site</label><input id="site" placeholder="Renfrew 395" bind:value={site} /></div>
      </div>
      {#if isDraft}
        <div class="field">
          <label for="override-reason">override reason (required to start a draft)</label>
          <input id="override-reason" bind:value={overrideReason} />
        </div>
      {/if}
      <h3>Instrument and conditions (optional)</h3>
      <div class="field-row">
        <div class="field"><label for="sensor-model">sensor model</label><input id="sensor-model" placeholder="GEM GSM-19" bind:value={sensorModel} /></div>
        <div class="field"><label for="sensor-serial">serial</label><input id="sensor-serial" placeholder="4451233" bind:value={sensorSerial} /></div>
        <div class="field"><label for="sensor-firmware">firmware</label><input id="sensor-firmware" placeholder="7.0" bind:value={sensorFirmware} /></div>
      </div>
      <div class="field">
        <label for="hardware">hardware (one per line)</label>
        <textarea id="hardware" rows="2" placeholder="mag_gcs v0.3.1" bind:value={hardwareText}></textarea>
      </div>
      <div class="field">
        <label for="conditions">conditions (one <span class="mono">key: value</span> per line)</label>
        <textarea id="conditions" rows="2" placeholder="weather: clear&#10;temp_c: 12" bind:value={conditionsText}></textarea>
      </div>
      <button
        class="primary"
        onclick={() => void start()}
        disabled={!runId.trim() || !operator.trim() || !site.trim() || (isDraft && !overrideReason.trim())}
      >
        Start run
      </button>
      <p class="muted" style="margin-top:8px">
        A snapshot of the checklist is frozen and an event log is opened. The run can be
        resumed after a crash by replaying the log. The instrument and conditions are
        written into the record, so two runs at the same site can be told apart.
      </p>
    </section>
  {:else}
    <div class="exec-layout">
      <nav>
        <label for="stepfilter">Filter steps</label>
        <input id="stepfilter" type="search" bind:value={stepQuery} />
        <ol class="steps">
          {#each visibleSteps as s, i (s.id)}
            <li
              data-state={s.status}
              aria-current={run!.steps.indexOf(s) === currentStep}
            >
              <button type="button" onclick={() => (currentStep = run!.steps.indexOf(s))}>
                <span class="title">
                  <span class="dot {text(s.severity)}"></span>
                  <span class="no">{i + 1}</span>{s.title}
                </span>
                <span class="meta">
                  <span class="state {s.status}">{s.status}</span>
                  {#if s.reason}<span class="reason">{s.reason}</span>{/if}
                </span>
              </button>
            </li>
          {/each}
        </ol>
        <hr class="divider" />
        <button
          disabled={run!.steps.some((s) => !["done", "skipped", "deviated"].includes(s.status))}
          title="All steps need an outcome before a run can be marked complete"
          onclick={() => void end("complete")}
        >End: complete</button>
        <button onclick={() => void end("partial")}>End: partial</button>
        <button onclick={() => void end("aborted")}>End: aborted</button>
      </nav>

      {#if current}
        <article class="step-detail">
          <div class="badges">
            <span class="badge {text(current.severity)}">{text(current.severity)}</span>
            <span class="badge">{text(current.kind)}</span>
            <span class="badge {current.status}">{current.status}</span>
            <span class="badge">{current.id}</span>
          </div>
          <h2>{current.title}</h2>
          <div class="prose">{@html markdown(current.prose)}</div>

          {#if current.captures.length}
            <div class="captures" bind:this={capturesBox}>
              <h3>Captures</h3>
              {#each current.captures as cap (cap.key)}
                <div class="capture">
                  <label for={cap.key}>{cap.label ?? cap.key}{#if cap.required}<span class="req">*</span>{/if}</label>
                  <div class="hint mono">{cap.key} &middot; {cap.type}{#if cap.unit} {cap.unit}{/if}</div>
                  <div class="field-row">
                    {#if cap.type === "select" && cap.options?.length}
                      <select id={cap.key} onkeydown={onCaptureKey} value={cap.value ?? ""} onchange={(e) => void recordCapture(current, cap.key, (e.currentTarget as HTMLSelectElement).value)}>
                        <option value="">(unset)</option>
                        {#each cap.options as option (option)}
                          <option value={option}>{option}</option>
                        {/each}
                      </select>
                    {:else if cap.type === "bool"}
                      <select id={cap.key} onkeydown={onCaptureKey} value={cap.value ?? ""} onchange={(e) => void recordCapture(current, cap.key, (e.currentTarget as HTMLSelectElement).value)}>
                        <option value="">(unset)</option>
                        <option value="true">true</option>
                        <option value="false">false</option>
                      </select>
                    {:else}
                      <input
                        id={cap.key}
                        onkeydown={onCaptureKey}
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
            </div>
          {/if}

          {#if current.checklist.length}
            <section class="captures">
              <h3>Checklist</h3>
              {#each current.checklist as item, index (index)}
                <label class="task">
                  <input type="checkbox" checked={item.checked} onchange={(e) => void toggleCheckbox(current, index, (e.currentTarget as HTMLInputElement).checked)} />
                  {item.text}
                </label>
              {/each}
            </section>
          {/if}

          <div class="toolbar">
            <button onclick={() => void setStatus(current, "done")}>Mark done</button>
            <button onclick={() => void setStatus(current, "skipped")}>Skip</button>
            <button onclick={() => void setStatus(current, "deviated")}>Deviate</button>
            <span class="divider-btn"></span>
            <button onclick={() => void attach(current)}>Attach data / photo</button>
          </div>
          <p class="muted">Skips and deviations require a reason; it is written into the record.</p>

          {#if run.runAttachments.length}
            <section class="attachments">
              <h3>Attached data ({run.runAttachments.length})</h3>
              {#each run.runAttachments as att (att.sha256)}
                <div class="attachment">
                  <span class="name">{att.path.split(/[\\/]/).pop()}</span>
                  <span class="mono muted">{att.size} bytes &middot; sha256 {att.sha256.slice(0, 12)}&hellip;</span>
                </div>
              {/each}
            </section>
          {/if}

          <div class="field">
            <label for="step-note">Note for this step</label>
            <div class="field-row">
              <input id="step-note" bind:this={noteInput} bind:value={noteText} onkeydown={(e) => { if (e.key === "Enter") void addNote(current); }} />
              <button onclick={() => void addNote(current)}>Add note</button>
            </div>
          </div>
        </article>
      {/if}
    </div>
  {/if}
</article>

<style>
  /* The app shell lays this view beside the help panel, so the run screen ends up with
     the same three columns as Browse: step list, instructions, help. This view owns the
     first two and lets each scroll on its own instead of scrolling as one block. */
  .exec { display: flex; flex-direction: column; padding: 0; overflow: hidden; }
  .exec > .toolbar { padding: 16px 16px 0; margin-bottom: 0; }
  .exec > .err, .exec > .muted { padding: 0 16px; margin: 0 0 8px; }
  .run-meta { display: flex; gap: 16px; flex-wrap: wrap; }
  .spacer { flex: 1; }
  .toolbar { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }
  /* The form is only four fields, but the window is a full field laptop: let the panel
     use it. Only the explanatory prose keeps a readable measure. */
  .start-panel { flex: 1; min-height: 0; padding: 16px; overflow: auto; }
  .start-panel p { max-width: 68ch; }
  .start-panel .field { margin-bottom: 8px; }
  .exec-layout { flex: 1; min-height: 0; display: grid; grid-template-columns: 300px minmax(0, 1fr); }
  .exec-layout > nav { overflow: auto; border-right: 1px solid var(--line); }
  .exec-layout > nav button { width: 100%; margin-top: 4px; font-size: 12px; }
  .exec-layout > .step-detail { overflow: auto; padding: 16px; }
  .warn { color: var(--warn); }
  .step-detail .captures h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); }
  .task { display: block; padding: 2px 0; }
  .expected { color: var(--warn); font-size: 12px; margin-top: 6px; }
  .divider-btn { width: 1px; height: 20px; background: var(--line); margin: 0 4px; }
  .attachments h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); }
  .attachment { display: flex; gap: 8px; align-items: baseline; padding: 2px 0; font-size: 13px; }
  .attachment .name { font-family: var(--mono); font-size: 12px; }
  .ended-banner {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    margin: 0 16px 12px; padding: 12px 16px; border-radius: 8px;
    background: color-mix(in srgb, var(--ok) 16%, transparent);
    border: 1px solid var(--ok);
  }
  .ended-main { display: flex; align-items: center; gap: 12px; }
  .ended-check {
    width: 34px; height: 34px; border-radius: 50%;
    display: grid; place-items: center;
    background: var(--ok); color: #fff; font-size: 20px; line-height: 1;
  }
  .ended-main strong { font-size: 16px; }
  .ended-id { display: block; font-family: var(--mono); font-size: 13px; color: var(--muted); }
</style>
