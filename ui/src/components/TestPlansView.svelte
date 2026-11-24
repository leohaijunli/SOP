<script lang="ts">
  import { onMount, untrack } from "svelte";
  import * as api from "../lib/api";
  import { text } from "../lib/api";
  import { localDay } from "../lib/dates";
  import type { Manifest, RunEntry, TestPlan, TestCase } from "../lib/types";
  import DoubleConfirm from "./DoubleConfirm.svelte";

  let {
    manifest,
    onStartCase,
    onStartPlan,
    onChanged,
  }: {
    manifest: Manifest | null;
    onStartCase: (plan: TestPlan, c: TestCase) => void;
    onStartPlan: (plan: TestPlan) => void;
    onChanged: () => Promise<void>;
  } = $props();

  let planIndex = $state(0);
  let message = $state("");
  let isError = $state(false);
  let plans: TestPlan[] = $state(untrack(() => manifest?.testplans ?? []));
  let confirmOpen = $state(false);
  let confirmMessage = $state("");
  let confirmCase: TestCase | null = $state(null);
  let creatingPlan = $state(false);
  let planTitle = $state("");
  let busy = $state(false);
  let query = $state("");

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  // Plans are local Markdown files in the selected working directory's testplan/ tree.
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

  const visibleCases = $derived((plan?.cases ?? []).filter((entry) => `${text(entry.title)} ${entry.id}`.toLowerCase().includes(query.trim().toLowerCase())));

  onMount(() => {
    void reload();
  });

  const saved = async (note: string): Promise<void> => {
    await reload();
    say(note);
    await onChanged();
  };
  const refreshPlans = async (): Promise<void> => {
    if (busy) return;
    busy = true;
    try { await reload(); await onChanged(); }
    finally { busy = false; }
  };

  const start = (c: TestCase): void => {
    if (plan) onStartCase(plan, c);
  };

  const duplicate = async (c: TestCase): Promise<void> => {
    try {
      const newPath = await api.duplicateTestCase(c.path);
      await saved(`Case duplicated: ${newPath.split(/[\\/]/).pop()}`);
    } catch (e) {
      say(String(e), true);
    }
  };

  const askRemove = (c: TestCase): void => {
    confirmCase = c;
    confirmMessage = `Delete test case "${text(c.title) || c.id}"?`;
    confirmOpen = true;
  };

  const remove = async (): Promise<void> => {
    if (!confirmCase) return;
    const c = confirmCase;
    try {
      await api.deleteTestCase(c.path);
      await saved(`Case deleted: ${c.id}`);
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
      await saved(`Case imported: ${newPath.split(/[\\/]/).pop()}`);
    } catch (e) {
      say(String(e), true);
    }
  };

  const newPlan = async (): Promise<void> => {
    if (busy || !planTitle.trim()) return;
    busy = true;
    try {
      await api.createTestPlan("", planTitle.trim());
      creatingPlan = false;
      await saved(`Plan created: ${planTitle.trim()}`);
      planTitle = "";
    } catch (e) {
      say(String(e), true);
    } finally { busy = false; }
  };
</script>

<article class="testplans">
  <div class="toolbar">
    <h1 style="margin:0">Test Plans</h1>
    <span class="spacer"></span>
    <button onclick={() => plan && onStartPlan(plan)} disabled={busy || !plan || !plan.cases.length} title="Run this plan's cases in order">Run plan</button>
    <button onclick={() => creatingPlan = !creatingPlan} disabled={busy}>New plan</button>
    <button onclick={() => void addCase()} disabled={busy || !plan}>Import case…</button>
    <button onclick={() => void refreshPlans()} disabled={busy}>Refresh local plans</button>
  </div>
  <p class="muted">Local Markdown plans and cases are loaded automatically from the working directory's testplan/ folder. Plain MD without front matter is supported. No repo synchronization is needed; use Test Notes for a lightweight experiment.</p>
  {#if creatingPlan}<form class="toolbar" onsubmit={(event) => { event.preventDefault(); void newPlan(); }}><input aria-label="Plan name" bind:value={planTitle} placeholder="Plan name (ID generated automatically)" disabled={busy} /><button class="primary" disabled={busy || !planTitle.trim()}>Create plan</button><button type="button" onclick={() => creatingPlan = false}>Cancel</button></form>{/if}
  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}

  {#if !plans.length}
    <p class="empty">No test plans yet. Create a plan and import Markdown cases, or add a plan folder under testplan/.</p>
  {:else}
    <div class="layout">
      <nav class="plan-list">
        {#each plans as p, i (p.id)}
          <button class:current={i === planIndex} onclick={() => (planIndex = i)}>
            <span class="title">{text(p.title) || p.id}</span>
            <span class="mono muted">{p.cases.length} cases</span>
          </button>
        {/each}
      </nav>

      <section class="cases">
        {#if !plan}
          <p class="empty">Select a test plan</p>
        {:else}
          <h2>{text(plan.title) || plan.id}</h2>
          <input class="case-search" type="search" aria-label="Search test cases" bind:value={query} placeholder="Search test cases…" />
          {#if !plan.cases.length}
            <p class="empty">This plan has no cases yet. Use Import case to add one.</p>
          {:else}
            <table class="matrix">
              <thead>
                <tr>
                  <th>Test case</th>
                  <th>Latest record</th>
                  <th>Logs</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {#each visibleCases as c (c.path)}
                  {@const runs = runsFor(c.sop_id)}
                  {@const last = runs[0] ?? null}
                  <tr>
                    <td>
                      <span class="title">{text(c.title) || c.id}</span>
                      <span class="mono muted">{c.id}</span>
                    </td>
                    <td class="mono" title={last ? text(last.started) : ""}>
                      {last ? localDay(last.started) : "\u2014"}
                      <span class="muted">{runs.length} records{last?.conclusion ? ` · ${text(last.conclusion)}` : ""}</span>
                    </td>
                    <td>{last?.log_count ?? 0}</td>
                    <td class="actions">
                      <button class="primary" onclick={() => start(c)}>Start test</button>
                      <details><summary>Manage case</summary><button onclick={() => void duplicate(c)}>Duplicate</button><button onclick={() => askRemove(c)}>Delete…</button></details>
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

  <DoubleConfirm
    open={confirmOpen}
    message={confirmMessage}
    onConfirm={() => { confirmOpen = false; void remove(); }}
    onCancel={() => (confirmOpen = false)}
  />
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
  .actions { white-space: nowrap; }
  .actions button { font-size: 12px; margin-right: 4px; }
  .actions details { display: inline-block; font-size: 12px; }
  .actions summary { cursor: pointer; color: var(--muted); }
  .case-search { width: 100%; margin-bottom: 12px; }
  @media (max-width: 900px) { .layout { grid-template-columns: 1fr; } }
</style>
