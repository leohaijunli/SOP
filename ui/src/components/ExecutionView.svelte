<script lang="ts">
  import { onMount } from "svelte";
  import { htmlOf } from "../lib/md.svelte";
  import { text } from "../lib/text";
  import { expectsNumber, expectedHint, expectedOk, formatProblem, missingRequired } from "../lib/capture";
  import MarkdownEditor from "./MarkdownEditor.svelte";
  import * as api from "../lib/api";
  import type { Drift, Manifest, RunEntry, RunStepView, RunView } from "../lib/types";

  let {
    manifest,
    checklist,
    caseContext,
    onNextCase,
    onRunUpdate,
  }: {
    manifest: Manifest | null;
    checklist: number;
    /// The plan/case this run was started from, or null for a directly-opened checklist.
    caseContext: {
      planId: string;
      planTitle: string;
      caseId: string;
      caseTitle: string;
      index: number;
      total: number;
      hasNext: boolean;
    } | null;
    onNextCase: () => void;
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
  let runIdTouched = $state(false);
  let editingRunId = $state(false);
  let operator = $state("");
  let site = $state("");
  let overrideReason = $state("");
  // The instrument is picked from a list rather than typed: `sensors` is a machine
  // setting, seeded with the models the lab owns, so a run id and a record only ever
  // carry a name the operator chose from the same menu.
  let sensorsText = $state("UAS-MAG; RM3100");
  let sensorModel = $state("");
  let sensorSerial = $state("");
  let customModel = $state("");
  let customSerial = $state("");
  let sensorFirmware = $state("");
  let sensorTouched = $state(false);
  let equipmentChecked: string[] = $state([]);
  let extraHardware = $state("");
  let conditionsText = $state("");
  // Values for the conditions this checklist declares, keyed by the declared key. Kept
  // apart from the free-form box so a declared field and an extra never collide.
  let declaredValues: Record<string, string> = $state({});
  let message = $state("");
  let isError = $state(false);
  // A persistent "saved HH:MM:SS", so an operator can tell at a glance that the last
  // event landed rather than trusting a transient word.
  let lastSaved = $state<string | null>(null);
  // Provenance: whether the checklist has changed since this run's snapshot.
  let drift: Drift | null = $state(null);

  // The conditions the checklist asks for, in file order.
  const declaredConditions = $derived(
    (manifest?.checklists[checklist]?.conditions ?? []) as { key: string; hint?: string | null }[]
  );
  // Sites to offer: the ones project.md names, plus every site a run was recorded at.
  const siteOptions = $derived.by(() => {
    const seen = new Set<string>();
    for (const value of manifest?.project?.sites ?? []) {
      const site = api.text(value);
      if (site) seen.add(site);
    }
    for (const entry of manifest?.runs ?? []) {
      const site = api.text(entry.site);
      if (site) seen.add(site);
    }
    return [...seen].sort();
  });
  let run = $state<RunView | null>(null);
  let currentStep = $state(0);
  let stepQuery = $state("");
  let noteText = $state("");
  let runNoteText = $state("");
  // The note editor is a component, so the `n` shortcut holds its instance rather than a
  // raw textarea: `focus()` is the one method the shortcut needs.
  let noteInput = $state<{ focus: () => void } | null>(null);
  let capturesBox = $state<HTMLElement | null>(null);
  // Set when the run has ended, so a prominent confirmation is shown over the finished
  // run instead of a small line of text the operator may miss.
  let ended = $state<{ status: string; runId: string } | null>(null);
  // A pending request to end the run. Ending is not undoable, so the first press only
  // arms this and the second confirms.
  let endPending = $state<string | null>(null);

  // Clear the finished-run confirmation and go back to the start-a-run form, re-seeded
  // from the checklist rather than left as the finished run left it.
  const newRun = (): void => {
    run = null;
    ended = null;
    runId = "";
    runIdTouched = false;
    editingRunId = false;
    site = "";
    currentStep = 0;
    seeded = -1;
    equipmentChecked = [];
  };

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  // Slugify a name into the id-friendly form used by run ids.
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

  // ---- instruments ----------------------------------------------------------
  //
  // `sensors` is `model: serial, serial; model`. The picker reads it, and a serial typed
  // by hand is written back, so the list grows from the runs actually taken instead of
  // having to be filled in up front.
  type SensorStock = { model: string; serials: string[] };

  const parseSensors = (value: string): SensorStock[] => {
    const out: SensorStock[] = [];
    for (const entry of value.split(";")) {
      const trimmed = entry.trim();
      if (!trimmed) continue;
      const colon = trimmed.indexOf(":");
      const model = (colon < 0 ? trimmed : trimmed.slice(0, colon)).trim();
      if (!model) continue;
      const serials = (colon < 0 ? "" : trimmed.slice(colon + 1))
        .split(",")
        .map((serial) => serial.trim())
        .filter((serial) => serial !== "");
      const known = out.find((stock) => stock.model === model);
      if (known) {
        for (const serial of serials) if (!known.serials.includes(serial)) known.serials.push(serial);
      } else {
        out.push({ model, serials });
      }
    }
    return out;
  };

  const formatSensors = (stocks: SensorStock[]): string =>
    stocks
      .map((stock) => (stock.serials.length ? `${stock.model}: ${stock.serials.join(", ")}` : stock.model))
      .join("; ");

  const sensorOptions: SensorStock[] = $derived(parseSensors(sensorsText));
  const serialsFor = (model: string): string[] =>
    sensorOptions.find((stock) => stock.model === model)?.serials ?? [];
  const usingOtherModel = (): boolean => sensorModel === "__other__";
  const effectiveModel = (): string => (usingOtherModel() ? customModel.trim() : sensorModel);
  const effectiveSerial = (): string =>
    usingOtherModel() || sensorSerial === "__new__" ? customSerial.trim() : sensorSerial;

  const pickModel = (): void => {
    sensorTouched = true;
    sensorSerial = "";
    customSerial = "";
  };

  // The checklist already names the equipment it needs, so the form ticks it rather than
  // asking the operator to retype it. The sensor, if the list has it, is preselected.
  const equipmentOf = (index: number): string[] =>
    ((manifest?.checklists[index]?.equipment ?? []) as unknown[])
      .map((item) => text(item))
      .filter((item) => item !== "");
  const equipment: string[] = $derived(equipmentOf(checklist));

  const toggleEquipment = (item: string, checked: boolean): void => {
    equipmentChecked = checked
      ? [...equipmentChecked, item]
      : equipmentChecked.filter((name) => name !== item);
  };

  const existingRunIds = (): Set<string> =>
    new Set(
      (manifest?.runs ?? [])
        .filter((r) => api.text(r.sop) === sop)
        .map((r) => api.text(r.run_id))
    );

  // Start-button state, computed here so the reason it is disabled is visible rather
  // than discovered by pressing it and reading an error.
  const runIdConflict = $derived(runId.trim() !== "" && existingRunIds().has(runId.trim()));
  const missingFields = $derived(
    [
      runId.trim() ? "" : "run id",
      operator.trim() ? "" : "operator",
      site.trim() ? "" : "site",
      isDraft && !overrideReason.trim() ? "override reason" : "",
    ].filter((field) => field !== "")
  );
  const canStart = $derived(missingFields.length === 0 && !runIdConflict);

  // The id a run is filed under. `2026-09-28-uas-mag-1001` reads back the date, the model
  // and the unit. With no instrument it falls back to the site, and either way a repeat
  // gets `-2`, `-3` so it never collides with an existing run.
  const suggestRunId = (): string => {
    const existing = existingRunIds();
    const model = effectiveModel();
    const serial = effectiveSerial();
    // A case started from a plan carries the case in the id, so two cases of one plan on
    // the same instrument the same day never read as the same run.
    const caseId = caseContext ? slug(caseContext.caseId) : "";
    if (caseId) {
      const base = [today(), caseId, model ? slug(model) : "", serial ? slug(serial) : ""]
        .filter((part) => part !== "")
        .join("-");
      let candidate = base;
      for (let i = 2; existing.has(candidate); i++) candidate = `${base}-${i}`;
      return candidate;
    }
    if (model || serial) {
      const base = [today(), model ? slug(model) : "", serial ? slug(serial) : ""]
        .filter((part) => part !== "")
        .join("-");
      let candidate = base;
      for (let i = 2; existing.has(candidate); i++) candidate = `${base}-${i}`;
      return candidate;
    }
    const base = `${today()}-${slug(site)}`;
    let candidate = `${base}-01`;
    for (let i = 2; existing.has(candidate); i++) {
      candidate = `${base}-${String(i).padStart(2, "0")}`;
    }
    return candidate;
  };

  // Seed the form from the checklist that was opened, and again after "Start new run".
  let seeded = -1;
  $effect(() => {
    if (checklist === seeded) return;
    seeded = checklist;
    equipmentChecked = equipment;
    declaredValues = {};
    extraHardware = "";
    sensorModel = "";
    sensorSerial = "";
    customModel = "";
    customSerial = "";
    sensorTouched = false;
    runIdTouched = false;
    editingRunId = false;
  });

  // When the plan advances to the next case the checklist changes while this component
  // stays mounted, so a run from the previous case must be dropped rather than shown
  // against the new one.
  $effect(() => {
    if (run && run.sop && run.sop !== sop) {
      run = null;
      ended = null;
    }
  });

  // Re-check drift whenever the active run changes, so a checklist edited while the run
  // is open shows as changed rather than silently disagreeing with the snapshot.
  let driftFor = "";
  $effect(() => {
    const id = run?.runId ?? "";
    if (id === driftFor) return;
    driftFor = id;
    drift = null;
    if (run && id) {
      void api
        .runDrift(run.sop, id)
        .then((next) => (drift = next))
        .catch(() => (drift = null));
    }
  });

  // Preselect the sensor the checklist names, once the setting has loaded and unless the
  // operator has chosen one themselves.
  $effect(() => {
    if (sensorTouched || run) return;
    const names = equipment.map((name) => name.toLowerCase());
    const hit = sensorOptions.find((stock) =>
      names.some((name) => name.includes(stock.model.toLowerCase()))
    );
    if (hit && hit.model !== sensorModel) sensorModel = hit.model;
  });

  // Keep the id in step with the date, the instrument and the site until it is edited by
  // hand, at which point the operator's name wins.
  $effect(() => {
    if (run || editingRunId || runIdTouched) return;
    runId = suggestRunId();
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

  // A serial typed by hand joins the machine's sensor list, so the next run of the same
  // unit picks it from the menu. The run has already started, so a failure here must not
  // put an error banner over it.
  const rememberSerial = async (model: string, serial: string): Promise<void> => {
    if (!model || !serial) return;
    const stocks = parseSensors(sensorsText);
    let stock = stocks.find((item) => item.model === model);
    if (!stock) {
      stock = { model, serials: [] };
      stocks.push(stock);
    }
    if (stock.serials.includes(serial)) return;
    stock.serials.push(serial);
    try {
      const rows = await api.settingsSet("sensors", formatSensors(stocks));
      const saved = rows.find((row) => row.key === "sensors")?.value;
      if (saved) sensorsText = saved;
    } catch {
      /* keeping the list is a convenience, not part of the run */
    }
  };

  const start = async (): Promise<void> => {
    const model = effectiveModel();
    const serial = effectiveSerial();
    try {
      run = await api.runStart(runSop, runId.trim(), operator.trim(), site.trim(), overrideReason.trim() || null, {
        sensorModel: model || null,
        sensorSerial: serial || null,
        sensorFirmware: sensorFirmware.trim() || null,
        hardware: [...new Set([...equipmentChecked, ...parseLines(extraHardware)])],
        // A declared field wins over the same key typed into the free-form box: the
        // checklist asked for that field explicitly.
        conditions: {
          ...parseConditions(conditionsText),
          ...Object.fromEntries(
            Object.entries(declaredValues).filter(([, value]) => value.trim() !== "")
          ),
        },
        plan: caseContext ? caseContext.planId : null,
        case: caseContext ? caseContext.caseId : null,
      });
      currentStep = 0;
      message = `started ${runId.trim()}`;
      isError = false;
      void rememberSerial(model, serial);
      onRunUpdate();
    } catch (e) {
      say(String(e), true);
    }
  };

  // One event at a time. `enter` on the last capture records a value and marks the step
  // done in the same keystroke, and two concurrent `run_record` calls could otherwise
  // interleave their appends to the log and leave the view showing the older response.
  let queue: Promise<void> = Promise.resolve();
  // Recording an event rewrites the whole record from the replayed log, so the caller gets
  // back whether it landed: the note editor keeps the operator's text on a failure rather
  // than clearing a note that was never written.
  const emit = (event: any): Promise<boolean> => {
    let saved = false;
    queue = queue.then(async () => {
      if (!run) return;
      try {
        run = await api.runRecord(run.sop, run.runId, { at: new Date().toISOString(), ...event });
        message = "";
        lastSaved = new Date().toLocaleTimeString();
        isError = false;
        saved = true;
      } catch (e) {
        say(String(e), true);
      }
    });
    return queue.then(() => saved);
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

  // The recorded judgement for a value outside the expected range. The tool never judges;
  // the operator does, and this event is what survives review.
  const acknowledge = async (step: RunStepView, key: string): Promise<void> => {
    const reason = prompt("Why is this value acceptable? (optional, recorded)");
    if (reason === null) return;
    await emit({ type: "CaptureAcknowledged", step: step.id, key, reason: reason.trim() || null });
  };

  // Reopen a step that was marked done, skipped, or deviated by mistake. The log is
  // append-only, so this is a compensating event, not a deletion.
  const reopen = async (step: RunStepView): Promise<void> => {
    await emit({
      type: "StepStatusChanged",
      step: step.id,
      status: "open",
      reason: "reopened after a mis-tap",
    });
  };

  // Notes are stored verbatim, so the editor's text goes straight through. A step note
  // is filed under that step; the run-level note uses `step: null`.
  const addNote = async (step: RunStepView | null, text: string): Promise<boolean> => {
    const t = text.trim();
    if (!t) return false;
    return emit({ type: "NoteAdded", step: step?.id ?? null, text: t });
  };

  // Attach a data file or photo to the run. The shell copies it into the run's logs,
  // hashes it, and records a `logs:` entry in the record, so the experiment and its data
  // travel together.
  const attach = async (step: RunStepView | null): Promise<void> => {
    if (!run) return;
    const path = await api.pickDataFile();
    if (!path) return;
    // The stored name can differ from the picked one (a `-2` suffix on a name clash), so
    // report the copy that actually landed rather than the source path.
    const before = new Set(
      [...run.steps.flatMap((s) => s.attachments), ...run.runAttachments].map((a) => a.sha256)
    );
    try {
      const next = await api.runAttach(run.sop, run.runId, step?.id ?? null, path);
      const added = [...next.steps.flatMap((s) => s.attachments), ...next.runAttachments].find(
        (a) => !before.has(a.sha256)
      );
      run = next;
      message = added
        ? `saved ${added.path}`
        : `${path.split(/[\\/]/).pop()} is already attached to this step`;
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  // The file name part of a stored path, for a compact label next to the full path.
  const fileName = (path: string): string => path.split(/[\\/]/).pop() ?? path;

  // A short, stable form of a hash or commit for a tooltip or an inline label.
  const short = (value: string | null): string => (value ? value.slice(0, 8) : "?");

  // Ending cannot be undone, so a press first arms a confirmation bar and only a second
  // press writes the event. `blockedReason` is the `complete` coverage check, kept so the
  // confirmation bar can explain why that status is not offered.
  const blockedReason = $derived(
    (() => {
      if (!run) return null;
      const open = run.steps.filter((s) => !["done", "deviated"].includes(s.status));
      if (open.length === 0) return null;
      return `${open.length} step(s) are skipped or have no outcome; a complete run cannot ` +
        `contain those. Finish or deviate them, or end as partial/aborted`;
    })()
  );

  const requestEnd = (status: string): void => {
    // A `complete` run must account for every step and cannot contain a skipped one:
    // both are validation errors (`check::complete_run_coverage`), so ending a run this
    // way would write a record that then blocks every content edit. A deviation is
    // allowed - the step happened, just not as written.
    if (status === "complete" && blockedReason) {
      say(blockedReason, true);
      return;
    }
    endPending = status;
  };

  const cancelEnd = (): void => {
    endPending = null;
  };

  const confirmEnd = (): Promise<void> => {
    const status = endPending;
    endPending = null;
    if (!status) return Promise.resolve();
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

  // Finishing a step. Done is not a verdict on the data, so a required capture that is
  // still empty is a reminder, not a block: the operator confirms and the reason (the
  // list of empty fields) is written with the status. A manual entry still wins.
  const markDone = async (step: RunStepView): Promise<void> => {
    const missing = missingRequired(step.captures);
    let reason: string | null = null;
    if (missing.length) {
      const typed = prompt(
        `${missing.length} required capture(s) are empty: ${missing.join(", ")}. ` +
          `Finish anyway? Add a reason to record (blank cancels):`
      );
      if (!typed) return;
      reason = `done with ${missing.length} required capture(s) empty (${missing.join(", ")}): ${typed.trim()}`;
    }
    await emit({ type: "StepStatusChanged", step: step.id, status: "done", reason });
  };

  // The keyboard contract in `help/app-basics.md`: field use is one-handed, so every
  // action on the run screen has a key. `F1` and `/` belong to the shell, so they are
  // left alone here. `ctrl+enter` asks to finish the run, but only when the focus is not
  // in a field - inside the note editor it must insert a newline, and a stray chord must
  // not end a run that cannot be reopened.
  const onKey = (event: KeyboardEvent): void => {
    const target = event.target as HTMLElement;
    const inField = target.matches("input, select, textarea");
    if (endPending) {
      event.preventDefault();
      if (event.key === "Escape") cancelEnd();
      return;
    }
    if (event.ctrlKey && event.key === "Enter") {
      if (inField) return;
      event.preventDefault();
      requestEnd("complete");
      return;
    }
    if (inField || event.key === "F1" || event.key === "/" || !run || !current) return;
    const index = run.steps.indexOf(current);
    if (event.key === " ") {
      event.preventDefault();
      void markDone(current);
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
      void markDone(current);
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

  // Seed the start form from a run the same lab took before. Values are only filled
  // where the operator has not typed, and a declared condition's value goes to its own
  // field rather than the free-form box.
  function seedFromRun(previous: RunEntry, m: Manifest): void {
    if (!operator && text(previous.operator)) operator = text(previous.operator);
    if (!site && text(previous.site)) site = text(previous.site);

    // An external case is not in the repository manifest's checklists, so the current
    // checklist's own declarations are merged with the manifest's.
    const declared = new Set<string>();
    for (const c of m.checklists.find((c) => text(c.sop_id) === sop)?.conditions ?? []) {
      declared.add(c.key);
    }
    for (const c of declaredConditions) declared.add(c.key);
    const values = (previous.conditions ?? {}) as Record<string, unknown>;
    const extra: string[] = [];
    const fields: Record<string, string> = { ...declaredValues };
    for (const [key, value] of Object.entries(values)) {
      const item = text(value);
      if (!item) continue;
      if (declared.has(key)) fields[key] = item;
      else extra.push(`${key}: ${item}`);
    }
    declaredValues = fields;
    if (extra.length && !conditionsText.trim()) conditionsText = extra.join("\n");

    const sensor = previous.sensor;
    if (sensor && !sensorTouched) {
      const model = text(sensor.model);
      const serial = text(sensor.serial);
      if (model) {
        sensorModel = model;
        sensorTouched = true;
        // A run's instrument joins the picker list, the same way a hand-typed serial
        // does, so the next run finds it in the menu.
        void rememberSerial(model, serial);
      }
      if (serial) sensorSerial = serial;
      if (text(sensor.firmware)) sensorFirmware = text(sensor.firmware);
    }
  }

  onMount(() => {
    // The sensor picker reads a machine setting; a failure leaves the built-in default.
    void api
      .settingsRows()
      .then((rows) => {
        const value = rows.find((row) => row.key === "sensors")?.value;
        if (value) sensorsText = value;
      })
      .catch(() => {});
    // Seed the form from the last run, then resume the most recent unfinished run of
    // this checklist if there is one. Seeding is why a campaign's operator, site, and
    // conditions are typed once instead of every run.
    void api.manifest().then(async (m) => {
      const newestFirst = m.runs
        .slice()
        .sort((a, b) => api.text(b.started).localeCompare(api.text(a.started)));
      const last = newestFirst.find((r) => api.text(r.sop) === sop) ?? newestFirst[0];
      if (last && !run) seedFromRun(last, m);

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
    {#if caseContext}
      <span class="breadcrumb">
        {caseContext.planTitle} &middot; case {caseContext.index + 1}/{caseContext.total}: {caseContext.caseTitle}
      </span>
    {/if}
    <span class="spacer"></span>
    {#if run}
      <span class="muted">
        {run!.runId} &middot; {run!.operator ?? "?"} &middot; {run!.site ?? "?"}{#if run!.runStatus} &middot; {run!.runStatus}{/if}
      </span>
      {#if lastSaved}<span class="saved">saved {lastSaved}</span>{/if}
    {/if}
  </div>

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}

  {#if endPending}
    <div class="end-confirm" role="alertdialog" aria-label="Confirm ending the run">
      <span>
        End the run as <strong>{endPending}</strong>? A run that has ended cannot be
        reopened.
      </span>
      <button class="primary" onclick={() => void confirmEnd()}>Yes, end {endPending}</button>
      <button onclick={cancelEnd}>Cancel (Esc)</button>
    </div>
  {/if}

  {#if ended}
    <div class="ended-banner" role="status">
      <div class="ended-main">
        <span class="ended-check">&#10003;</span>
        <div>
          <strong>Run {ended.status.toUpperCase()}</strong>
          <span class="ended-id">{ended.runId}</span>
          {#if caseContext}
            <span class="ended-id">plan {caseContext.planId} &middot; case {caseContext.index + 1} of {caseContext.total}</span>
          {/if}
        </div>
      </div>
      {#if caseContext?.hasNext}
        <button class="primary" onclick={() => onNextCase()}>
          Next case: {caseContext.caseTitle} &rarr;
        </button>
        <button onclick={() => void newRun()}>Start new run</button>
      {:else}
        <button class="primary" onclick={() => void newRun()}>Start new run</button>
      {/if}
    </div>
  {/if}

  {#if run && (run.plan || run.sopVersion || run.sensor || run.hardware.length || Object.keys(run.conditions).length)}
    <p class="muted run-meta">
      {#if run.plan}<span>plan: {run.plan}{#if run.case} / {run.case}{/if}</span>{/if}
      {#if run.sopVersion}<span>checklist v{run.sopVersion}{#if run.sopCommit} ({short(run.sopCommit)}){/if}</span>{/if}
      {#if drift && drift.currentSha256}
        {#if drift.drifted}
          <span class="drift-warn" title={`snapshot ${short(drift.snapshotSha256)}; now ${short(drift.currentSha256)}`}>
            changed since this run started{#if drift.currentVersion} (now v{drift.currentVersion}){/if}
          </span>
        {:else}
          <span title={`snapshot ${short(drift.snapshotSha256)}`}>&#10003; unchanged since start</span>
        {/if}
      {/if}
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
      {#if caseContext}
        <p class="breadcrumb-plan">
          Plan <strong>{caseContext.planTitle}</strong> &mdash;
          case {caseContext.index + 1} of {caseContext.total}:
          <strong>{caseContext.caseTitle}</strong>
        </p>
      {/if}
      {#if isDraft}
        <p class="warn">This checklist is a draft. It cannot start without an override reason.</p>
      {/if}
      <div class="field-row">
        <div class="field"><label for="operator">operator</label><input id="operator" placeholder="your name" bind:value={operator} /></div>
        <div class="field">
          <label for="site">site</label>
          <input id="site" list="site-options" placeholder="Renfrew 395" bind:value={site} />
          <datalist id="site-options">
            {#each siteOptions as option (option)}
              <option value={option}></option>
            {/each}
          </datalist>
        </div>
      </div>

      <h3>Instrument</h3>
      <div class="field-row">
        <div class="field">
          <label for="sensor-model">sensor model</label>
          <select id="sensor-model" bind:value={sensorModel} onchange={pickModel}>
            <option value="">(none)</option>
            {#each sensorOptions as stock (stock.model)}
              <option value={stock.model}>{stock.model}</option>
            {/each}
            <option value="__other__">Other model&hellip;</option>
          </select>
        </div>
        {#if usingOtherModel()}
          <div class="field">
            <label for="sensor-model-other">model name</label>
            <input id="sensor-model-other" placeholder="RM3100" bind:value={customModel} oninput={() => (sensorTouched = true)} />
          </div>
        {/if}
        <div class="field">
          <label for="sensor-serial">serial number</label>
          {#if usingOtherModel() || sensorSerial === "__new__"}
            <input id="sensor-serial" placeholder="1001" bind:value={customSerial} />
          {:else}
            <select id="sensor-serial" bind:value={sensorSerial}>
              <option value="">(none)</option>
              {#each serialsFor(sensorModel) as serial (serial)}
                <option value={serial}>{serial}</option>
              {/each}
              <option value="__new__">New serial&hellip;</option>
            </select>
          {/if}
        </div>
        <div class="field"><label for="sensor-firmware">firmware</label><input id="sensor-firmware" placeholder="optional" bind:value={sensorFirmware} /></div>
      </div>
      <span class="desc">The list comes from the <span class="mono">sensors</span> setting, and a serial entered here is added to it.</span>

      <div class="field">
        <label for="run-id">run id</label>
        {#if editingRunId}
          <div class="field-row">
            <input id="run-id" bind:value={runId} oninput={() => (runIdTouched = true)} />
            {#if runIdTouched}
              <button type="button" onclick={() => { runIdTouched = false; editingRunId = false; }}>Use suggested</button>
            {/if}
          </div>
        {:else}
          <div class="field-row">
            <code class="id-preview">{runId || "…"}</code>
            <button type="button" onclick={() => (editingRunId = true)}>Change</button>
          </div>
        {/if}
        <span class="desc">
          Filed as <span class="mono">runs/{sop}/{runId || "…"}/</span>, beside this
          checklist's other runs. The name is built from the date and the instrument so a
          run reads back what it was taken with; change it only if you want your own.
        </span>
      </div>

      {#if isDraft}
        <div class="field">
          <label for="override-reason">override reason (required to start a draft)</label>
          <input id="override-reason" bind:value={overrideReason} />
        </div>
      {/if}

      <h3>Equipment and conditions</h3>
      {#if equipment.length}
        <div class="field">
          <span class="group-label">equipment this checklist declares</span>
          <div class="equipment">
            {#each equipment as item (item)}
              <label class="task">
                <input
                  type="checkbox"
                  checked={equipmentChecked.includes(item)}
                  onchange={(e) => toggleEquipment(item, (e.currentTarget as HTMLInputElement).checked)}
                />
                {item}
              </label>
            {/each}
          </div>
          <span class="desc">Everything not ticked is left out of the record, so drop what the run did not use.</span>
        </div>
      {/if}
      <div class="field">
        <label for="extra-hardware">additional hardware (one per line)</label>
        <textarea id="extra-hardware" rows="2" placeholder="mag_gcs v0.3.1" bind:value={extraHardware}></textarea>
      </div>
      {#if declaredConditions.length}
        <div class="field">
          <span class="group-label">conditions this checklist records</span>
          {#each declaredConditions as cond (cond.key)}
            <div class="condition-field">
              <label for={`cond-${cond.key}`}>
                {cond.key}{#if cond.hint}<span class="muted"> &mdash; {cond.hint}</span>{/if}
              </label>
              <input
                id={`cond-${cond.key}`}
                placeholder={cond.hint ?? "enter value"}
                value={declaredValues[cond.key] ?? ""}
                onchange={(e) =>
                  (declaredValues = {
                    ...declaredValues,
                    [cond.key]: (e.currentTarget as HTMLInputElement).value,
                  })}
              />
            </div>
          {/each}
        </div>
      {/if}
      <div class="field">
        <label for="conditions">
          {declaredConditions.length ? "other conditions" : "conditions"}
          (one <span class="mono">key: value</span> per line)
        </label>
        <textarea id="conditions" rows="2" placeholder="weather: clear&#10;temp_c: 12" bind:value={conditionsText}></textarea>
      </div>
      {#if runIdConflict}
        <p class="err">
          a run named <span class="mono">{runId.trim()}</span> already exists for this
          checklist &mdash; change the run id
        </p>
      {:else if missingFields.length}
        <p class="muted">still needed: {missingFields.join(", ")}</p>
      {/if}
      <button
        class="primary"
        onclick={() => void start()}
        disabled={!canStart}
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
          disabled={run!.steps.some((s) => !["done", "deviated"].includes(s.status))}
          title={blockedReason ?? "End the run as complete"}
          onclick={() => requestEnd("complete")}
        >End: complete</button>
        <button onclick={() => requestEnd("partial")}>End: partial</button>
        <button onclick={() => requestEnd("aborted")}>End: aborted</button>
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
          <div class="prose">{@html htmlOf(current.prose)}</div>

          {#if current.captures.length}
            <div class="captures" bind:this={capturesBox}>
              <h3>Captures</h3>
              {#each current.captures as cap (cap.key)}
                <div class="capture">
                  <label for={cap.key}>{cap.label ?? cap.key}{#if cap.required}<span class="req">*</span>{/if}</label>
                  <div class="hint mono">{cap.key} &middot; {cap.type}{#if cap.unit} {cap.unit}{/if}</div>
                  {#if expectedHint(cap)}
                    <div class="expected-hint">expected: {expectedHint(cap)}</div>
                  {/if}
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
                    {:else if expectsNumber(cap.type)}
                      <input
                        id={cap.key}
                        onkeydown={onCaptureKey}
                        type="number"
                        inputmode="decimal"
                        step={cap.type === "integer" ? "1" : "any"}
                        placeholder={cap.value ?? "enter value"}
                        onchange={(e) => void recordCapture(current, cap.key, (e.currentTarget as HTMLInputElement).value)}
                      />
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
                  {#if formatProblem(cap.value, cap.type)}
                    <div class="expected">&#9888; {formatProblem(cap.value, cap.type)} &mdash; retype and press Enter</div>
                  {/if}
                  {#if cap.value}
                    <div class="muted">recorded: {cap.value}</div>
                  {/if}
                  {#if cap.value && cap.expected !== null && cap.expected !== undefined && !expectedOk(cap.value, cap.expected) && !cap.acknowledged}
                    <div class="expected">
                      outside the expected range &mdash;
                      <button type="button" onclick={() => void acknowledge(current, cap.key)}>Acknowledge</button>
                    </div>
                  {/if}
                  {#if cap.acknowledged}
                    <div class="muted">&#10003; acknowledged by the operator</div>
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
            <button onclick={() => void markDone(current)}>Mark done</button>
            <button onclick={() => void setStatus(current, "skipped")}>Skip</button>
            <button onclick={() => void setStatus(current, "deviated")}>Deviate</button>
            {#if current.status !== "open"}
              <button onclick={() => void reopen(current)} title="Undo a mis-tapped outcome (recorded as a correction)">Reopen</button>
            {/if}
            <span class="divider-btn"></span>
            <button onclick={() => void attach(current)}>Attach data / photo</button>
          </div>
          <p class="muted">Skips and deviations require a reason; it is written into the record.</p>

          {#if current.attachments.length}
            <section class="attachments">
              <h3>Data saved for this step ({current.attachments.length})</h3>
              {#each current.attachments as att (att.sha256)}
                <div class="attachment">
                  <span class="name">{fileName(att.path)}</span>
                  <span class="mono muted">{att.size} bytes &middot; sha256 {att.sha256.slice(0, 12)}&hellip;</span>
                  <span class="where muted">{att.path}</span>
                </div>
              {/each}
            </section>
          {/if}

          {#if run.runAttachments.length}
            <section class="attachments">
              <h3>Data saved for the run ({run.runAttachments.length})</h3>
              {#each run.runAttachments as att (att.sha256)}
                <div class="attachment">
                  <span class="name">{fileName(att.path)}</span>
                  <span class="mono muted">{att.size} bytes &middot; sha256 {att.sha256.slice(0, 12)}&hellip;</span>
                  <span class="where muted">{att.path}</span>
                </div>
              {/each}
            </section>
          {/if}

          {#if current.notes.length}
            <section class="notes">
              <h3>Notes on this step ({current.notes.length})</h3>
              {#each current.notes as note, i (i)}
                <div class="prose note-body">{@html htmlOf(note)}</div>
              {/each}
            </section>
          {/if}

          <div class="field note-editor">
            <label for="step-note">Add a note to this step</label>
            <MarkdownEditor
              bind:this={noteInput}
              value={noteText}
              placeholder="What did you see or change? Markdown is fine."
              submitLabel="Add step note"
              onChange={(next) => (noteText = next)}
              onSubmit={() => {
                void addNote(current, noteText).then((saved) => {
                  if (saved) noteText = "";
                });
              }}
            />
          </div>

          <details class="run-notes" open={run.runNotes.length > 0}>
            <summary>Run notes ({run.runNotes.length})</summary>
            {#each run.runNotes as note, i (i)}
              <div class="prose note-body">{@html htmlOf(note)}</div>
            {/each}
            <MarkdownEditor
              value={runNoteText}
              rows={4}
              placeholder="A note about the whole run: site conditions, anything out of the ordinary…"
              submitLabel="Add run note"
              onChange={(next) => (runNoteText = next)}
              onSubmit={() => {
                void addNote(null, runNoteText).then((saved) => {
                  if (saved) runNoteText = "";
                });
              }}
            />
          </details>
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
  .breadcrumb { font-size: 12px; color: var(--muted); }
  .breadcrumb-plan { color: var(--muted); }
  .saved { font-family: var(--mono); font-size: 11px; color: var(--ok); }
  .drift-warn { color: var(--warn); font-weight: 600; }
  .spacer { flex: 1; }
  .toolbar { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }
  /* The form is only four fields, but the window is a full field laptop: let the panel
     use it. Only the explanatory prose keeps a readable measure. */
  .start-panel { flex: 1; min-height: 0; padding: 16px; overflow: auto; }
  .start-panel p { max-width: 68ch; }
  .start-panel .field { margin-bottom: 8px; }
  .start-panel .field-row { align-items: flex-end; }
  .id-preview {
    font-family: var(--mono); font-size: 13px; padding: 5px 10px; border-radius: 4px;
    background: var(--quote); border: 1px solid var(--line); flex: 1;
  }
  .equipment { display: flex; flex-wrap: wrap; gap: 4px 16px; }
  .start-panel .group-label { font-size: 12px; color: var(--muted); }
  .condition-field { display: flex; flex-direction: column; gap: 2px; margin-top: 6px; }
  .condition-field input { max-width: 28ch; }
  .exec-layout { flex: 1; min-height: 0; display: grid; grid-template-columns: 300px minmax(0, 1fr); }
  .exec-layout > nav { overflow: auto; border-right: 1px solid var(--line); }
  .exec-layout > nav button { width: 100%; margin-top: 4px; font-size: 12px; }
  .exec-layout > .step-detail { overflow: auto; padding: 16px; }
  .warn { color: var(--warn); }
  .step-detail .captures h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); }
  .task { display: block; padding: 2px 0; }
  .expected-hint { color: var(--muted); font-size: 12px; margin-top: 2px; }
  .expected { color: var(--warn); font-size: 12px; margin-top: 6px; }
  .end-confirm {
    display: flex; gap: 12px; align-items: center; flex-wrap: wrap;
    margin: 0 16px 8px; padding: 10px 12px; border-radius: 6px;
    background: var(--quote); border: 1px solid var(--warn); color: var(--warn);
  }
  .end-confirm strong { text-transform: uppercase; }
  .divider-btn { width: 1px; height: 20px; background: var(--line); margin: 0 4px; }
  .notes h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); }
  .note-body {
    background: var(--quote); border-left: 3px solid var(--line); border-radius: 6px;
    padding: 4px 12px; margin: 6px 0;
  }
  .note-editor { margin-top: 12px; }
  .run-notes { margin-top: 16px; }
  .run-notes summary {
    cursor: pointer; font-size: 12px; text-transform: uppercase;
    letter-spacing: .08em; color: var(--muted); margin-bottom: 6px;
  }
  .attachments h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); }
  .attachment { display: flex; flex-wrap: wrap; gap: 2px 8px; align-items: baseline; padding: 2px 0; font-size: 13px; }
  .attachment .name { font-family: var(--mono); font-size: 12px; }
  .attachment .where { flex-basis: 100%; font-family: var(--mono); font-size: 11px; word-break: break-all; }
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
