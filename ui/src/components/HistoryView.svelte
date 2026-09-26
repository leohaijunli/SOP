<script lang="ts">
  import { text } from "../lib/api";
  import type { Manifest, RunEntry } from "../lib/types";

  let {
    manifest,
    checklist,
  }: {
    manifest: Manifest | null;
    checklist: number;
  } = $props();

  let siteFilter = $state("");
  let operatorFilter = $state("");
  let statusFilter = $state("");

  const sop = $derived(manifest?.checklists[checklist] ? text(manifest.checklists[checklist].sop_id) : "");

  const runs: RunEntry[] = $derived(
    (() => {
      if (!manifest) return [];
      return manifest.runs
        .filter((r) => text(r.sop) === sop)
        .sort((a, b) => text(b.started).localeCompare(text(a.started)));
    })()
  );

  const sites: string[] = $derived(
    (() => {
      const seen = new Set<string>();
      for (const r of runs) if (text(r.site)) seen.add(text(r.site));
      return [...seen].sort();
    })()
  );

  const operators: string[] = $derived(
    (() => {
      const seen = new Set<string>();
      for (const r of runs) if (text(r.operator)) seen.add(text(r.operator));
      return [...seen].sort();
    })()
  );

  const filtered: RunEntry[] = $derived(
    (() => {
      return runs.filter((r) => {
        if (siteFilter && text(r.site) !== siteFilter) return false;
        if (operatorFilter && text(r.operator) !== operatorFilter) return false;
        if (statusFilter && text(r.status) !== statusFilter) return false;
        return true;
      });
    })()
  );
</script>

<article class="history">
  <div class="toolbar">
    <h1 style="margin:0">History</h1>
    <span class="muted mono">{sop}</span>
  </div>

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
    <p class="empty">no runs for this checklist</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>run id</th>
          <th>started</th>
          <th>site</th>
          <th>operator</th>
          <th>outcome</th>
          <th>deviations</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as r (text(r.run_id))}
          <tr>
            <td class="mono">{text(r.run_id)}</td>
            <td>{text(r.started)}</td>
            <td>{text(r.site)}</td>
            <td>{text(r.operator)}</td>
            <td><span class="badge {text(r.status)}">{text(r.status)}</span></td>
            <td>{text(r.deviations_count)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</article>

<style>
  .toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 12px; }
  .filters { display: flex; gap: 8px; margin-bottom: 12px; }
  table { width: 100%; border-collapse: collapse; }
  th, td { border: 1px solid var(--line); padding: 6px 10px; text-align: left; }
  th { background: var(--quote); font-size: 12px; text-transform: uppercase; letter-spacing: .06em; }
  td.mono { font-family: var(--mono); font-size: 12px; }
  .badge.complete { color: var(--ok); }
  .badge.partial { color: var(--warn); }
  .badge.aborted { color: var(--critical); }
</style>