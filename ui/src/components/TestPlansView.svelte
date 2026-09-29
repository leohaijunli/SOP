<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { text } from "../lib/api";
  import type { Manifest, RunEntry, TestPlan, TestCase } from "../lib/types";

  let {
    manifest,
    onStartCase,
    onStartPlan,
    onChanged,
  }: {
    manifest: Manifest | null;
    onStartCase: (plan: TestPlan, c: TestCase) => void;
    onStartPlan: (plan: TestPlan) => void;
    onChanged: () => void;
  } = $props();

  let planIndex = $state(0);
  let message = $state("");
  let isError = $state(false);
  let plans: TestPlan[] = $state([]);

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  // The plans come from the testcase repository (the `testcases` branch), not the
  // working copy, so they are fetched fresh each time the view opens or a change lands.
  const reload = async (): Promise<void> => {
    try {
      plans = await api.testPlans();
      if (planIndex >= plans.length) planIndex = plans.length > 0 ? 0 : 0;
    } catch (e) {
      say(String(e), true);
    }
  };

  const plan: TestPlan | null = $derived(plans[planIndex] ?? null);

  // Runs of one case, newest first. A case is matched by the checklist id it records,
  // which is the case's `sop_id`, the same key History groups on.
  const runsFor = (sopId: string): RunEntry[] =>
    (manifest?.runs ?? [])
      .filter((r) => text(r.sop) === sopId)
      .sort((a, b) => text(b.started).localeCompare(text(a.started)));

  const lastRun = (sopId: string): RunEntry | null => runsFor(sopId)[0] ?? null;

  const runLabel = (r: RunEntry | null): string => {
    if (!r) return "not run";
    return text(r.status) || "in progress";
  };

  // The case a "Run next" should open: the first one whose latest run is not complete.
  const nextCase = (p: TestPlan): TestCase | null =>
    p.cases.find((c) => {
      const last = lastRun(c.sop_id);
      return !last || text(last.status) !== "complete";
    }) ?? null;

  onMount(() => {
    void reload();
  });

  // After a change to the testcase repo, commit + push it so other machines pick it up,
  // then refresh the view.
  const synced = async (note: string): Promise<void> => {
    let pushed = true;
    try {
      await api.testcasePush(note);
    } catch (e) {
      say(`saved locally, but push failed: ${e}`, true);
      pushed = false;
    }
    // Always reload the local view: the file change already landed on disk, and the list
    // must reflect it immediately even when the remote push failed.
    await reload();
    if (pushed) {
      say(note);
      isError = false;
    }
    onChanged();
  };

  const start = (c: TestCase): void => {
    if (plan) onStartCase(plan, c);
  };

  const duplicate = async (c: TestCase): Promise<void> => {
    try {
      const newPath = await api.duplicateTestCase(c.path);
      await synced(`duplicate case ${newPath.split(/[\\/]/).pop()}`);
    } catch (e) {
      say(String(e), true);
    }
  };

  const remove = async (c: TestCase): Promise<void> => {
    if (!confirm(`Delete test case "${text(c.title) || c.id}"?`)) return;
    try {
      await api.deleteTestCase(c.path);
      await synced(`delete case ${c.id}`);
    } catch (e) {
      say(String(e), true);
    }
  };

  const addCase = async (): Promise<void> => {
    if (!plan) return;
    const source = await api.pickDataFile();
    if (!source) return;
    try {
      const newPath = await api.importTestCase(plan.path, source);
      await synced(`import case ${newPath.split(/[\\/]/).pop()}`);
    } catch (e) {
      say(String(e), true);
    }
  };

  const newPlan = async (): Promise<void> => {
    const name = prompt("New plan id (lowercase, hyphens ok):");
    if (!name) return;
    const title = prompt("Plan title (blank uses the id):") ?? "";
    try {
      await api.createTestPlan(name.trim(), title);
      await synced(`create plan ${name.trim()}`);
    } catch (e) {
      say(String(e), true);
    }
  };
</script>

<article class="testplans">
  <div class="toolbar">
    <h1 style="margin:0">Test Plans</h1>
    <span class="spacer"></span>
    <button onclick={() => plan && onStartPlan(plan)} disabled={!plan || !plan.cases.length} title="Run every case in this plan in order">Run plan</button>
    {#if plan}
      {@const next = nextCase(plan)}
      <button onclick={() => next && start(next)} disabled={!next} title="Run the first case whose latest run is not complete">Run next</button>
    {/if}
    <button onclick={() => void newPlan()}>New plan</button>
    <button onclick={() => void addCase()} disabled={!plan}>Add case</button>
  </div>
  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}

  {#if !plans.length}
    <p class="empty">No test plans yet. Use "New plan" to create one, or add folders under testplan/ with a case md in each.</p>
  {:else}
    <div class="layout">
      <nav class="plan-list">
        {#each plans as p, i (p.id)}
          <button class:current={i === planIndex} onclick={() => (planIndex = i)}>
            <span class="title">{text(p.title) || p.id}</span>
            <span class="mono muted">{p.cases.length} case(s) &middot; {p.id}</span>
          </button>
        {/each}
      </nav>

      <section class="cases">
        {#if !plan}
          <p class="empty">select a plan</p>
        {:else}
          <h2>{text(plan.title) || plan.id}</h2>
          {#if !plan.cases.length}
            <p class="empty">no test cases in this plan yet &mdash; use "Add case"</p>
          {:else}
            <table>
              <thead>
                <tr><th>order</th><th>case</th><th>steps</th><th>runs</th><th>last run</th><th>status</th><th></th></tr>
              </thead>
              <tbody>
                {#each plan.cases as c (c.path)}
                  {@const runs = runsFor(c.sop_id)}
                  {@const last = runs[0] ?? null}
                  <tr>
                    <td class="mono">{c.order}</td>
                    <td>
                      <span class="title">{text(c.title) || c.id}</span>
                      <span class="mono muted">{c.id}</span>
                    </td>
                    <td class="mono">{c.step_count}</td>
                    <td class="mono">{runs.length}</td>
                    <td class="mono">{last ? text(last.started).slice(0, 10) : "\u2014"}</td>
                    <td>
                      <span class="state {last ? (text(last.status) || "open") : "none"}">{runLabel(last)}</span>
                      {#if last && text(last.operator)}<span class="muted">{text(last.operator)}</span>{/if}
                    </td>
                    <td class="actions">
                      <button class="primary" onclick={() => start(c)}>Start</button>
                      <button onclick={() => void duplicate(c)} title="Duplicate case">Duplicate</button>
                      <button onclick={() => void remove(c)} title="Delete case">Delete</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        {/if}
      </section>
    </div>
  {/if}
</article>

<style>
  .toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 12px; }
  .spacer { flex: 1; }
  .layout { display: grid; grid-template-columns: 240px minmax(0, 1fr); gap: 16px; align-items: start; }
  .plan-list { border-right: 1px solid var(--line); padding-right: 8px; display: flex; flex-direction: column; gap: 4px; }
  .plan-list button { text-align: left; padding: 6px 8px; background: none; border: 1px solid transparent; }
  .plan-list button.current { border-color: var(--line); background: var(--quote); }
  .plan-list .title { display: block; font-weight: 600; }
  .plan-list .muted { display: block; font-size: 11px; }
  table { width: 100%; border-collapse: collapse; }
  th, td { border: 1px solid var(--line); padding: 6px 10px; text-align: left; }
  th { background: var(--quote); font-size: 12px; text-transform: uppercase; letter-spacing: .06em; }
  td.mono { font-family: var(--mono); font-size: 12px; }
  .title { display: block; }
  .muted { display: block; font-size: 11px; }
  .state {
    display: inline-block; padding: 1px 8px; border-radius: 10px; font-size: 11px;
    font-family: var(--mono); border: 1px solid var(--line); color: var(--muted);
  }
  .state.complete { color: var(--ok); border-color: var(--ok); }
  .state.partial { color: var(--warn); border-color: var(--warn); }
  .state.aborted { color: var(--critical); border-color: var(--critical); }
  .actions { white-space: nowrap; }
  .actions button { font-size: 12px; margin-right: 4px; }
</style>
