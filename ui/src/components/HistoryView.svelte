<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { text } from "../lib/api";
  import type { Manifest, RunEntry, TestPlan } from "../lib/types";

  let {
    manifest,
    onChanged,
  }: {
    manifest: Manifest | null;
    /// Called after a run is removed, so the manifest (and this view) is rebuilt from disk.
    onChanged: () => void;
  } = $props();

  // The plans come from the testcase repository - the same source the Test plans view
  // reads - so a run is grouped under the plan and case it was actually started from.
  let plans: TestPlan[] = $state([]);
  // Until the plans arrive, every run would look like it belongs to no case; wait for
  // them rather than showing a grouping that is about to change.
  let plansLoaded = $state(false);
  let siteFilter = $state("");
  let operatorFilter = $state("");
  let statusFilter = $state("");
  let message = $state("");
  let isError = $state(false);
  let busy = $state("");

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  onMount(() => {
    void api
      .testPlans()
      .then((fresh) => (plans = fresh))
      .catch((e) => say(String(e), true))
      .finally(() => (plansLoaded = true));
  });

  const filtering = $derived(Boolean(siteFilter || operatorFilter || statusFilter));

  const filtered: RunEntry[] = $derived(
    (manifest?.runs ?? [])
      .filter((r) => {
        if (siteFilter && text(r.site) !== siteFilter) return false;
        if (operatorFilter && text(r.operator) !== operatorFilter) return false;
        if (statusFilter && text(r.status) !== statusFilter) return false;
        return true;
      })
      .sort((a, b) => text(b.started).localeCompare(text(a.started)))
  );

  // Filter choices come from every run, not only the ones a case claims, so the list of
  // sites does not shift as runs are grouped.
  const options = (pick: (run: RunEntry) => string): string[] => {
    const seen = new Set<string>();
    for (const run of manifest?.runs ?? []) {
      const value = pick(run);
      if (value) seen.add(value);
    }
    return [...seen].sort();
  };

  const sites: string[] = $derived(options((r) => text(r.site)));
  const operators: string[] = $derived(options((r) => text(r.operator)));

  type CaseGroup = { key: string; title: string; sop: string; runs: RunEntry[] };
  type PlanGroup = { id: string; title: string; cases: CaseGroup[] };

  // Runs are matched to a case by the checklist id they recorded, which is the case's
  // front-matter `sop_id`. Anything that matches no case - a checklist opened by path, or
  // content this testcase repository does not describe - is kept rather than dropped.
  const grouped = $derived.by(() => {
    const claimed = new Set<string>();
    const out: PlanGroup[] = [];
    for (const plan of plans) {
      const cases: CaseGroup[] = [];
      for (const c of plan.cases) {
        claimed.add(c.sop_id);
        cases.push({
          key: `${plan.id}/${c.id}`,
          title: text(c.title) || c.id,
          sop: c.sop_id,
          runs: filtered.filter((r) => text(r.sop) === c.sop_id),
        });
      }
      // With a filter on, a plan with nothing to show is noise, not information.
      if (filtering && cases.every((c) => c.runs.length === 0)) continue;
      out.push({ id: plan.id, title: text(plan.title) || plan.id, cases });
    }

    const outside = new Map<string, RunEntry[]>();
    for (const run of filtered) {
      const sop = text(run.sop);
      if (claimed.has(sop)) continue;
      const rows = outside.get(sop) ?? [];
      rows.push(run);
      outside.set(sop, rows);
    }
    return {
      plans: out,
      outside: [...outside].map(([sop, runs]) => ({ sop, runs })),
    };
  });

  // Export one run's record. The native save dialog comes from the shell, so the window
  // just reports where the file landed. The run's own checklist id names the export.
  const exportRun = async (run: RunEntry): Promise<void> => {
    const runId = text(run.run_id);
    busy = `export:${runId}`;
    try {
      const result = await api.runExportDialog(text(run.sop), runId);
      if (!result) {
        say("export cancelled");
        return;
      }
      say(`wrote ${result.path} (${result.bytes} bytes)`);
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Remove one run: its record file, its run directory, and its log directory. Deleting a
  // record is not something to do by accident, so it asks first and says what goes.
  const deleteRun = async (run: RunEntry): Promise<void> => {
    const runId = text(run.run_id);
    const sop = text(run.sop);
    const ok = window.confirm(
      `Delete run "${runId}" of "${sop}"?\n\n` +
        "This removes its record, its run directory, and its attached data. It cannot be undone."
    );
    if (!ok) return;
    busy = `delete:${runId}`;
    try {
      await api.runDelete(sop, runId);
      say(`deleted run ${runId}`);
      onChanged();
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Remove every run in the working copy. The filters do not narrow this: "all" means all,
  // and the confirmation says how many that is before anything is removed.
  const deleteAll = async (): Promise<void> => {
    const total = manifest?.runs.length ?? 0;
    const ok = window.confirm(
      `Delete all ${total} run(s) in this working copy?\n\n` +
        "This removes every run record, run directory, and attached data. It cannot be undone."
    );
    if (!ok) return;
    busy = "delete-all";
    try {
      const removed = await api.runDeleteAll();
      say(`deleted ${removed} run(s)`);
      onChanged();
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Export a summary of every run, grouped by test plan and case.
  const exportSummary = async (): Promise<void> => {
    busy = "summary";
    try {
      const path = await api.runSummaryDialog();
      if (!path) {
        say("export cancelled");
        return;
      }
      say(`wrote summary ${path}`);
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Publish the whole working copy: the records, the logs, and every content edit that
  // has not been committed yet. The commit message is the operator's; blank takes the
  // shell's default.
  const pushRepo = async (): Promise<void> => {
    const answer = window.prompt("Commit message (blank for the default):", "");
    if (answer === null) return;
    busy = "push";
    try {
      const result = await api.repoPush(answer);
      say(result.log.join("; ") || `pushed to ${result.remote}`);
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };
</script>

{#snippet runTable(rows: RunEntry[])}
  <table>
    <thead>
      <tr>
        <th>run id</th>
        <th>started</th>
        <th>site</th>
        <th>operator</th>
        <th>outcome</th>
        <th>deviations</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as r (text(r.run_id))}
        <tr>
          <td class="mono">{text(r.run_id)}</td>
          <td>{text(r.started)}</td>
          <td>{text(r.site)}</td>
          <td>{text(r.operator)}</td>
          <td><span class="badge {text(r.status)}">{text(r.status) || "in progress"}</span></td>
          <td>{text(r.deviations_count)}</td>
          <td class="actions">
            <button disabled={busy !== ""} onclick={() => void exportRun(r)}>
              {busy === `export:${text(r.run_id)}` ? "Exporting…" : "Export record"}
            </button>
            <button class="danger" disabled={busy !== ""} onclick={() => void deleteRun(r)}>
              {busy === `delete:${text(r.run_id)}` ? "Deleting…" : "Delete"}
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/snippet}

<article class="history">
  <div class="toolbar">
    <h1 style="margin:0">History</h1>
    <span class="muted">{filtered.length} run(s)</span>
    <span class="spacer"></span>
    <button disabled={busy !== ""} onclick={() => void exportSummary()}>
      {busy === "summary" ? "Exporting…" : "Export summary"}
    </button>
    <button disabled={busy !== ""} onclick={() => void pushRepo()}>
      {busy === "push" ? "Pushing…" : "Push repo"}
    </button>
    <button class="danger" disabled={busy !== ""} onclick={() => void deleteAll()}>
      {busy === "delete-all" ? "Deleting…" : "Delete all runs"}
    </button>
  </div>

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}

  <div class="filters">
    <select bind:value={siteFilter}>
      <option value="">All sites</option>
      {#each sites as site (site)}
        <option value={site}>{site}</option>
      {/each}
    </select>
    <select bind:value={operatorFilter}>
      <option value="">All operators</option>
      {#each operators as op (op)}
        <option value={op}>{op}</option>
      {/each}
    </select>
    <select bind:value={statusFilter}>
      <option value="">All outcomes</option>
      <option value="complete">complete</option>
      <option value="partial">partial</option>
      <option value="aborted">aborted</option>
    </select>
  </div>

  {#if !filtered.length}
    <p class="empty">no runs</p>
  {:else if !plansLoaded}
    <p class="muted">loading test plans…</p>
  {:else}
    {#each grouped.plans as plan (plan.id)}
      <section class="plan">
        <h2>{plan.title}</h2>
        {#each plan.cases as c (c.key)}
          {#if c.runs.length || !filtering}
            <section class="case">
              <h3>
                {c.title}
                <span class="mono muted">{c.sop}</span>
                <span class="count">{c.runs.length} run(s)</span>
              </h3>
              {#if c.runs.length}
                {@render runTable(c.runs)}
              {:else}
                <p class="empty">no runs yet</p>
              {/if}
            </section>
          {/if}
        {/each}
      </section>
    {/each}

    {#if grouped.outside.length}
      <section class="plan">
        <h2>Runs outside a test plan</h2>
        {#each grouped.outside as group (group.sop)}
          <section class="case">
            <h3>
              {group.sop}
              <span class="count">{group.runs.length} run(s)</span>
            </h3>
            {@render runTable(group.runs)}
          </section>
        {/each}
      </section>
    {/if}
  {/if}
</article>

<style>
  .toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 12px; }
  .spacer { flex: 1; }
  .filters { display: flex; gap: 8px; margin-bottom: 12px; }
  .plan { margin: 0 0 24px; }
  .plan h2 { margin: 0 0 8px; font-size: 15px; letter-spacing: .02em; }
  .case { margin: 0 0 16px; padding-left: 12px; border-left: 2px solid var(--line); }
  .case h3 { margin: 0 0 6px; font-size: 13px; font-weight: 600; display: flex; gap: 8px; align-items: baseline; }
  .case h3 .count { font-weight: 400; font-size: 12px; color: var(--muted); }
  table { width: 100%; border-collapse: collapse; }
  th, td { border: 1px solid var(--line); padding: 6px 10px; text-align: left; }
  th { background: var(--quote); font-size: 12px; text-transform: uppercase; letter-spacing: .06em; }
  td.mono { font-family: var(--mono); font-size: 12px; }
  td.actions { white-space: nowrap; }
  td button { font-size: 12px; white-space: nowrap; }
  td button + button { margin-left: 6px; }
  button.danger { color: var(--critical); }
  .badge.complete { color: var(--ok); }
  .badge.partial { color: var(--warn); }
  .badge.aborted { color: var(--critical); }
</style>
