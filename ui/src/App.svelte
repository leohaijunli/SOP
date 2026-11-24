<script lang="ts">
  import { onMount, tick } from "svelte";
  import Header from "./components/AppHeader.svelte";
  import StepsNav from "./components/StepsNav.svelte";
  import Detail from "./components/Detail.svelte";
  import HelpPanel from "./components/HelpPanel.svelte";
  import ManagePanel from "./components/ManagePanel.svelte";
  import AuthoringPanel from "./components/AuthoringPanel.svelte";
  import ExecutionView from "./components/ExecutionView.svelte";
  import RecordsView from "./components/RecordsView.svelte";
  import TestPlansView from "./components/TestPlansView.svelte";
  import WorkingDirectoryPicker from "./components/WorkingDirectoryPicker.svelte";
  import * as api from "./lib/api";
  import { getOperator, setOperator, clearOperator } from "./lib/operator";
  import type { ChecklistEntry, Manifest, Status, TestCase, TestPlan, WorkingDirectory } from "./lib/types";

  // Which screen the window is showing.
  type View = "notes" | "logs" | "search" | "browse" | "testplans" | "run" | "settings" | "authoring";

  let manifest: Manifest | null = $state(null);
  let status: Status | null = $state(null);
  let error: string | null = $state(null);
  let notice = $state("");
  let directory = $state<WorkingDirectory | null>(null);
  let selectingDirectory = $state(false);
  let initializing = $state(true);
  let recordsView = $state<{ focusSearch: () => void; newNote: () => void } | null>(null);
  let refreshTicket = 0;
  let pendingOperations = $state(0);
  const operationChanged = (started: boolean): void => {
    pendingOperations = Math.max(0, pendingOperations + (started ? 1 : -1));
  };
  const chooseDirectory = (): void => { if (pendingOperations === 0) selectingDirectory = true; };
  // Checklists the operator loaded from local markdown files via "Open file" or "Start".
  // They are not part of the repository, so they are merged back in after every manifest
  // refresh and would otherwise be lost whenever the view changes.
  let externalChecklists: ChecklistEntry[] = $state([]);
  // Notebook first. Plans and the full SOP workflow are optional management tools.
  let view: View = $state("notes");
  let helpOpen = $state(false);
  // Steps are always enabled on the Browse view; there is no toggle for them.
  const showSteps = true;

  // Which checklist and step are selected, as indexes into the (sorted) manifest.
  let checklist = $state(0);
  let step = $state(0);
  let help = $state<string | null>(null);
  let helpQuery = $state("");
  let stepQuery = $state("");
  let currentOperator = $state(getOperator());
  // A run the Execution view asked to resume: the shell switches the checklist, then the
  // view loads it. Cleared by the view once it has the run.
  let resumeRunId = $state<string | null>(null);

  // A plan run walks the cases of one test plan in `order`, carrying the start
  // configuration from one case to the next. `index` is the case now loaded.
  type PlanCase = { path: string; id: string; title: string; sopId: string };
  type PlanRun = { planId: string; planTitle: string; cases: PlanCase[]; index: number };
  let planRun: PlanRun | null = $state(null);

  // What the Execution view needs to know about the case it is starting, or null when
  // the checklist was opened directly. This is what the run records as its provenance.
  const caseContext = $derived.by(() => {
    if (!planRun) return null;
    const c = planRun.cases[planRun.index];
    if (!c) return null;
    const hasNext = planRun.index < planRun.cases.length - 1;
    return {
      planId: planRun.planId,
      planTitle: planRun.planTitle,
      caseId: c.id,
      caseTitle: c.title,
      index: planRun.index,
      total: planRun.cases.length,
      hasNext,
      // The name of the case the run will advance to, not the one now ending, so the
      // ended-run banner reads "Next case: <the next name>" instead of repeating the
      // case the operator just finished.
      nextCaseTitle: hasNext ? planRun.cases[planRun.index + 1].title : null,
    };
  });

  const counts = (): string => {
    const m = manifest;
    if (!m) return "";
    const logs = m.runs.reduce((count, run) => count + run.log_count, 0) + (m.workspace_files?.length ?? 0);
    return `${m.runs.length} records · ${logs} logs${m.testplans.length ? ` · ${m.testplans.length} plans` : ""}`;
  };

  const openView = (next: string): void => {
    if (
      next === "notes" || next === "logs" || next === "search" || next === "browse" || next === "testplans" || next === "run" ||
      next === "settings" || next === "authoring"
    ) {
      view = next as View;
    }
  };

  // Start a test case: load its markdown as the active checklist and open the Run view.
  // `context` carries the plan queue when the case is part of a plan run, so the run can
  // record its plan/case and offer the next one; a direct start clears it.
  const startCase = async (path: string, context: PlanRun | null = null): Promise<void> => {
    try {
      const entry = await api.loadExternalMd(path);
      if (!entry || !manifest) return;
      planRun = context;
      externalChecklists = [
        ...externalChecklists.filter((c) => api.text(c.sop_id) !== api.text(entry.sop_id)),
        entry,
      ];
      manifest = {
        ...manifest,
        checklists: [
          ...manifest.checklists.filter((c) => api.text(c.sop_id) !== api.text(entry.sop_id)),
          entry,
        ],
      };
      checklist = manifest.checklists.findIndex((c) => api.text(c.sop_id) === api.text(entry.sop_id));
      view = "run";
    } catch (e) {
      notice = `Could not start the test: ${e}`;
    }
  };

  // Run a whole plan: load its first case, and let the ended-run banner advance through
  // the rest in order via `nextCase`.
  const planCase = (c: { path: string; id: string; title: unknown; sop_id: string }): PlanCase => ({
    path: c.path,
    id: c.id,
    title: api.text(c.title) || c.id,
    sopId: c.sop_id,
  });

  // A one-case queue, so starting a single case from the plan table still records which
  // plan and case it came from, but offers no "next case" at the end.
  const startPlanCase = (plan: TestPlan, c: TestCase): void => {
    const queue: PlanRun = {
      planId: plan.id,
      planTitle: api.text(plan.title) || plan.id,
      cases: [planCase(c)],
      index: 0,
    };
    void startCase(c.path, queue);
  };

  const startPlan = (plan: TestPlan): void => {
    const cases: PlanCase[] = plan.cases.map(planCase);
    if (cases.length === 0) return;
    const queue: PlanRun = {
      planId: plan.id,
      planTitle: api.text(plan.title) || plan.id,
      cases,
      index: 0,
    };
    void startCase(cases[0].path, queue);
  };

  // Advance a plan run to its next case after the current one has ended.
  const nextCase = (): void => {
    if (!planRun) return;
    const next = planRun.index + 1;
    if (next >= planRun.cases.length) return;
    void startCase(planRun.cases[next].path, { ...planRun, index: next });
  };

  const refresh = async (reloadExternal = false): Promise<void> => {
    const ticket = ++refreshTicket;
    const currentPath = directory?.path;
    if (!currentPath) return;
    try {
      const activeSop = api.text(manifest?.checklists[checklist]?.sop_id);
      const [base, freshStatus] = await Promise.all([api.manifest(), api.status()]);
      // Re-read external files from disk so edits written to them by the Authoring page
      // show up in Browse/Run; otherwise the in-memory copy stays stale.
      const reloaded: ChecklistEntry[] = [];
      for (const entry of externalChecklists) {
        if (!reloadExternal) { reloaded.push(entry); continue; }
        try {
          const fresh = await api.loadExternalMd(entry.path);
          if (fresh) reloaded.push(fresh);
          else reloaded.push(entry);
        } catch {
          reloaded.push(entry);
        }
      }
      if (ticket !== refreshTicket || currentPath !== directory?.path) return;
      externalChecklists = reloaded;
      manifest = {
        ...base,
        checklists: [
          ...base.checklists.filter(
            (c) => !externalChecklists.some((e) => api.text(e.sop_id) === api.text(c.sop_id))
          ),
          ...externalChecklists,
        ],
      };
      const activeIndex = manifest.checklists.findIndex((entry) => api.text(entry.sop_id) === activeSop);
      checklist = activeIndex >= 0 ? activeIndex : 0;
      step = Math.min(step, Math.max(0, (manifest.checklists[checklist]?.steps.length ?? 0) - 1));
      status = freshStatus;
      error = null;
    } catch (e) {
      if (ticket === refreshTicket) error = String(e);
    }
  };

  const refreshRuns = async (): Promise<void> => {
    await refresh();
  };

  // Open another checklist's unfinished run: the Execution view cannot change the
  // selection itself, so it hands the run up and the shell points the view at it.
  const resumeRun = (sopId: string, runId: string): void => {
    const index = (manifest?.checklists ?? []).findIndex((c) => api.text(c.sop_id) === sopId);
    if (index < 0) {
      notice = `Cannot resume ${runId}: checklist ${sopId} is not loaded. Open its Markdown file in Manage → Run Checklist first.`;
      return;
    }
    checklist = index;
    step = 0;
    planRun = null;
    resumeRunId = runId;
    view = "run";
  };

  const openDirectory = async (path: string): Promise<void> => {
    if (pendingOperations > 0) throw new Error("Finish saving or adding files before switching working directories.");
    const opened = await api.openWorkingDirectory(path);
    refreshTicket++;
    directory = opened;
    status = null;
    manifest = null; externalChecklists = []; planRun = null; resumeRunId = null;
    checklist = 0; step = 0; view = "notes"; error = null; notice = "";
    await refresh();
    selectingDirectory = false;
  };

  onMount(() => {
    void (async () => {
      try {
        directory = await api.workingDirectory();
        if (directory.path) await refresh();
        else selectingDirectory = true;
      } catch (e) { error = String(e); selectingDirectory = true; }
      finally { initializing = false; }
    })();
  });

  // Deep links and keyboard shortcuts mirror the preview server.
  const readHash = (): void => {
    if (!manifest) return;
    const [sopId, stepRef, helpId] = location.hash.replace(/^#/, "").split("/");
    if (sopId) {
      const index = manifest.checklists.findIndex((c) => api.text(c.sop_id) === sopId);
      if (index >= 0) checklist = index;
    }
    const steps = manifest.checklists[checklist]?.steps ?? [];
    if (stepRef !== undefined && stepRef !== "") {
      const byId = steps.findIndex((candidate) => api.text(candidate.id) === stepRef);
      const idx = Number(stepRef);
      if (byId >= 0) step = byId;
      else if (Number.isInteger(idx) && idx >= 0 && idx < steps.length) step = idx;
    }
    if (helpId) help = helpId;
  };

  const writeHash = (): void => {
    if (!manifest) return;
    const c = manifest.checklists[checklist];
    if (!c) return;
    const parts = [api.text(c.sop_id), String(step)];
    if (help) parts.push(help);
    history.replaceState(null, "", `#${parts.join("/")}`);
  };

  $effect(() => {
    if (view !== "browse") return;
    writeHash();
  });

  onMount(() => {
    window.addEventListener("hashchange", readHash);
    return () => window.removeEventListener("hashchange", readHash);
  });

  const onKey = (event: KeyboardEvent): void => {
    if (selectingDirectory || document.querySelector("dialog[open]")) return;
    const command = event.ctrlKey || event.metaKey;
    if (command && (event.key.toLowerCase() === "k" || event.key.toLowerCase() === "n")) {
      event.preventDefault();
      const creating = event.key.toLowerCase() === "n";
      view = creating ? "notes" : "search";
      void tick().then(() => creating ? recordsView?.newNote() : recordsView?.focusSearch());
      return;
    }
    if ((event.target as HTMLElement).matches("input, select, textarea")) {
      if (event.key === "Escape") (event.target as HTMLElement).blur();
      return;
    }
    const steps = manifest?.checklists[checklist]?.steps ?? [];
    if (event.key === "F1" && (view === "browse" || view === "run")) { event.preventDefault(); helpOpen = !helpOpen; }
    else if (event.key === "/") { event.preventDefault(); view = "search"; void tick().then(() => recordsView?.focusSearch()); }
    else if (view !== "browse") { /* the active view handles its own keys */ }
    else if (event.key === "j" && step < steps.length - 1) { step++; }
    else if (event.key === "k" && step > 0) { step--; }
  };

  const updateOperator = (name: string): void => {
    currentOperator = name.trim();
    if (currentOperator) setOperator(currentOperator);
    else clearOperator();
  };
</script>

<svelte:window onkeydown={onKey} />

  <Header
    {counts}
    view={view}
    onView={openView}
    title={manifest?.project?.title ?? null}
    workingCopy={directory?.path ?? ""}
    {helpOpen}
    onToggleHelp={() => (helpOpen = !helpOpen)}
    operator={currentOperator}
    directoryLocked={pendingOperations > 0}
    onDirectory={chooseDirectory}
  />

  {#if error}<div class="app-message err" role="alert"><span>Could not read the working directory: {error}</span><button onclick={() => void refresh()}>Retry</button><button onclick={() => selectingDirectory = true}>Choose directory</button></div>{/if}
  {#if notice}<div class="app-message" role="status"><span>{notice}</span><button onclick={() => notice = ""}>Dismiss</button></div>{/if}

  <main
    class:with-steps={showSteps && view === "browse"}
    class:with-help={helpOpen && (view === "browse" || view === "run")}
  >
    {#if initializing || !directory?.path}
      <article class="setup"><h1>{initializing ? "Loading working directory…" : "Choose a working directory"}</h1><p class="muted">Select the folder containing your logs and test plans to begin.</p>{#if !initializing}<button class="primary" onclick={() => selectingDirectory = true}>Choose working directory…</button>{/if}</article>
    {:else if view === "notes" || view === "logs" || view === "search"}
      {#key directory.path}<RecordsView bind:this={recordsView} {manifest} workingCopy={directory.path} mode={view} signedInOperator={currentOperator} onMode={openView} onChanged={refreshRuns} onResumeRun={resumeRun} onBusy={operationChanged} />{/key}
    {:else if view === "browse"}
      {#if showSteps}
        <StepsNav
          manifest={manifest}
          {checklist}
          {step}
          {stepQuery}
          onChecklist={(i) => { checklist = i; step = 0; }}
          onStep={(i) => (step = i)}
          onStepQuery={(q) => (stepQuery = q)}
        />
      {/if}
      <Detail manifest={manifest} {checklist} {step} onEdit={() => (view = "authoring")} />
      {#if helpOpen}
        <HelpPanel manifest={manifest} {help} {helpQuery} onHelp={(h) => (help = h)} onQuery={(q) => (helpQuery = q)} />
      {/if}
    {:else if view === "testplans"}
      <TestPlansView
        {manifest}
        onStartCase={startPlanCase}
        onStartPlan={startPlan}
        onChanged={refreshRuns}
      />
    {:else if view === "run"}
      <ExecutionView
        {manifest}
        {checklist}
        {caseContext}
        onNextCase={nextCase}
        onRunUpdate={() => void refreshRuns()}
        onResumeRun={resumeRun}
        {resumeRunId}
        onResumed={() => (resumeRunId = null)}
        signedInOperator={currentOperator}
      />
      {#if helpOpen}
        <HelpPanel manifest={manifest} {help} {helpQuery} onHelp={(h) => (help = h)} onQuery={(q) => (helpQuery = q)} />
      {/if}
    {:else if view === "settings"}
      <ManagePanel {manifest} workingCopy={status?.workingCopy ?? directory.path} operator={currentOperator} onOperator={updateOperator} onDirectory={chooseDirectory} onChanged={refreshRuns} />
    {:else}
      <AuthoringPanel manifest={manifest} {checklist} onDone={() => { void refresh(true); view = "browse"; }} />
    {/if}
  </main>
  <WorkingDirectoryPicker open={selectingDirectory} {directory} onSelect={openDirectory} onCancel={() => selectingDirectory = false} />
<style>
  .app-message { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; padding: 8px 20px; background: var(--panel); border-bottom: 1px solid var(--line); font-size: 12px; }
  .app-message span { flex: 1; overflow-wrap: anywhere; }
  .app-message button { font-size: 12px; }
  .setup { padding: 32px; }
</style>
