<script lang="ts">
  import { onMount } from "svelte";
  import Header from "./components/Header.svelte";
  import StepsNav from "./components/StepsNav.svelte";
  import Detail from "./components/Detail.svelte";
  import HelpPanel from "./components/HelpPanel.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import ProjectPanel from "./components/ProjectPanel.svelte";
  import AuthoringPanel from "./components/AuthoringPanel.svelte";
  import ExecutionView from "./components/ExecutionView.svelte";
  import HistoryView from "./components/HistoryView.svelte";
  import TestPlansView from "./components/TestPlansView.svelte";
  import * as api from "./lib/api";
  import type { ChecklistEntry, Manifest, Status, TestCase, TestPlan } from "./lib/types";

  // Which screen the window is showing.
  type View = "browse" | "testplans" | "run" | "history" | "settings" | "project" | "authoring";

  let manifest: Manifest | null = $state(null);
  // Git and validation state, shown in the header so the operator sees whether the
  // working copy is committed and whether the content is clean.
  let status: Status | null = $state(null);
  let error: string | null = $state(null);
  // Sync status: pull/push result shown as a non-intrusive banner.
  let sync = $state("");
  let syncBad = $state(false);
  // Checklists the operator loaded from local markdown files via "Open file" or "Start".
  // They are not part of the repository, so they are merged back in after every manifest
  // refresh and would otherwise be lost whenever the view changes.
  let externalChecklists: ChecklistEntry[] = $state([]);
  // Land on Test Plans: the morning starts by picking what to run, not by editing content.
  let view: View = $state("testplans");
  let helpOpen = $state(true);
  // Steps are always enabled on the Browse view; there is no toggle for them.
  const showSteps = true;

  // Which checklist and step are selected, as indexes into the (sorted) manifest.
  let checklist = $state(0);
  let step = $state(0);
  let help = $state<string | null>(null);
  let helpQuery = $state("");
  let stepQuery = $state("");
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
    return {
      planId: planRun.planId,
      planTitle: planRun.planTitle,
      caseId: c.id,
      caseTitle: c.title,
      index: planRun.index,
      total: planRun.cases.length,
      hasNext: planRun.index < planRun.cases.length - 1,
    };
  });

  const counts = (): string => {
    const m = manifest;
    if (!m) return "";
    const planCount = m.testplans.length;
    return `${planCount} test plan(s) \u00b7 ${m.runs.length} run(s) \u00b7 ${m.checklists.length} checklist(s) \u00b7 ${m.help.length} help page(s)`;
  };

  const openView = (next: string): void => {
    if (
      next === "browse" || next === "testplans" || next === "run" ||
      next === "history" || next === "settings" || next === "project" || next === "authoring"
    ) {
      view = next as View;
    }
    // Refresh the manifest whenever the operator moves to a screen that reads it, so the
    // counts in the header and the History table match the working copy on disk.
    if (next === "browse" || next === "testplans" || next === "run" || next === "history") {
      void refreshRuns();
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
      console.warn("start case:", e);
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

  const refresh = async (): Promise<void> => {
    try {
      const base = await api.manifest();
      // Re-read external files from disk so edits written to them by the Authoring page
      // show up in Browse/Run; otherwise the in-memory copy stays stale.
      const reloaded: ChecklistEntry[] = [];
      for (const entry of externalChecklists) {
        try {
          const fresh = await api.loadExternalMd(entry.path);
          if (fresh) reloaded.push(fresh);
          else reloaded.push(entry);
        } catch {
          reloaded.push(entry);
        }
      }
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
      status = await api.status();
      error = null;
    } catch (e) {
      error = String(e);
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
      sync = `cannot resume ${runId}: checklist ${sopId} is not loaded. Open it with "Open file" first.`;
      syncBad = true;
      return;
    }
    checklist = index;
    step = 0;
    planRun = null;
    resumeRunId = runId;
    view = "run";
  };

  // Pull the working copy and the testcase repo, and report what happened. Called on
  // start and after edits so all machines stay in step; the result is shown as a banner.
  const doSync = async (): Promise<void> => {
    // A field laptop is often offline. `git pull` at an unreachable remote sits in DNS
    // and TCP until it times out, so skip it outright when the system already knows there
    // is no connection; the git command has its own bound as a backstop when this is
    // wrong. Sync stays one tap away for when the network comes back.
    if (!navigator.onLine) {
      sync = "offline - skipped sync";
      syncBad = false;
      return;
    }
    try {
      const report = await api.syncPull();
      sync = report;
      syncBad = report.toLowerCase().includes("error") || report.includes("diverged");
    } catch (e) {
      sync = String(e);
      syncBad = true;
    }
  };

  onMount(() => {
    void refresh();
    void doSync();
  });

  // Deep links and keyboard shortcuts mirror the preview server.
  const readHash = (): void => {
    if (!manifest) return;
    const [sopId, stepRef, helpId] = location.hash.replace(/^#/, "").split("/");
    if (sopId) {
      const index = manifest.checklists.findIndex((c) => api.text(c.sop_id) === sopId);
      if (index >= 0) checklist = index;
    }
    const steps = manifest.checklists[checklist].steps;
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
    if ((event.target as HTMLElement).matches("input, select, textarea")) {
      if (event.key === "Escape") (event.target as HTMLElement).blur();
      return;
    }
    const steps = manifest?.checklists[checklist]?.steps ?? [];
    if (event.key === "F1") { event.preventDefault(); helpOpen = !helpOpen; }
    else if (event.key === "/") { event.preventDefault(); helpOpen = true; }
    else if (view !== "browse") { /* the active view handles its own keys */ }
    else if (event.key === "j" && step < steps.length - 1) { step++; }
    else if (event.key === "k" && step > 0) { step--; }
  };
</script>

<svelte:window onkeydown={onKey} />

{#if error}
  <main class="fatal">
    <p class="err">Could not load the manifest: {error}</p>
  </main>
{:else}
  <Header
    {counts}
    {status}
    view={view}
    onView={openView}
    title={manifest?.project?.title ?? null}
    projectId={manifest?.project?.project_id ?? null}
    {helpOpen}
    onToggleHelp={() => (helpOpen = !helpOpen)}
  />

  {#if sync}
    <div class:syncbad={syncBad} class="syncbar">
      <span>{sync}</span>
      <button onclick={() => void doSync()} title="Sync now">Sync</button>
    </div>
  {/if}

  <main
    class:with-steps={showSteps && view === "browse"}
    class:with-help={helpOpen && (view === "browse" || view === "run")}
  >
    {#if view === "browse"}
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
        onChanged={() => void refresh()}
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
      />
      {#if helpOpen}
        <HelpPanel manifest={manifest} {help} {helpQuery} onHelp={(h) => (help = h)} onQuery={(q) => (helpQuery = q)} />
      {/if}
    {:else if view === "history"}
      <HistoryView {manifest} onChanged={() => void refreshRuns()} />
    {:else if view === "settings"}
      <SettingsPanel />
    {:else if view === "project"}
      <ProjectPanel />
    {:else}
      <AuthoringPanel manifest={manifest} {checklist} onDone={() => { void refresh(); view = "browse"; }} />
    {/if}
  </main>
{/if}
<style>
  .syncbar {
    display: flex; align-items: center; gap: 8px; padding: 4px 16px;
    background: var(--quote); border-bottom: 1px solid var(--line);
    font-size: 12px; color: var(--muted);
  }
  .syncbar.syncbad { color: var(--critical); }
  .syncbar button { font-size: 12px; margin-left: auto; }
</style>
