<script lang="ts">
  import { text } from "../lib/text";
  import type { Manifest } from "../lib/types";

  let {
    manifest,
    checklist,
    step,
    stepQuery,
    onChecklist,
    onStep,
    onStepQuery,
  }: {
    manifest: Manifest | null;
    checklist: number;
    step: number;
    stepQuery: string;
    onChecklist: (index: number) => void;
    onStep: (index: number) => void;
    onStepQuery: (query: string) => void;
  } = $props();

  let visibleSteps = $derived(
    (() => {
      if (!manifest || !manifest.checklists[checklist]) return [];
      const steps = manifest.checklists[checklist].steps;
      const query = stepQuery.trim().toLowerCase();
      if (!query) return steps.map((s, i) => ({ s, i }));
      return steps
        .map((s, i) => ({ s, i }))
        .filter(({ s }) =>
          [s.id, s.title, s.kind, s.severity].some((f) => text(f).toLowerCase().includes(query))
        );
    })()
  );
</script>

<nav>
  {#if manifest}
    <label for="checklist">Checklist</label>
    <select
      id="checklist"
      value={checklist}
      onchange={(e) => onChecklist(Number((e.currentTarget as HTMLSelectElement).value))}
    >
      {#each manifest.checklists as c, i (text(c.sop_id))}
        <option value={i}>{text(c.title)} ({text(c.sop_id)}) &middot; {c.step_count} steps</option>
      {/each}
    </select>
    <label for="stepfilter" style="display:block;margin-top:12px">Filter steps</label>
    <input
      id="stepfilter"
      type="search"
      placeholder="id, title, or kind"
      value={stepQuery}
      oninput={(e) => onStepQuery((e.currentTarget as HTMLInputElement).value)}
    />
  {/if}
  <h2>Steps</h2>
  <ol class="steps">
    {#if !visibleSteps.length}
      <li class="empty">no matching steps</li>
    {:else}
      {#each visibleSteps as { s, i } (text(s.id))}
        <li data-index={i} aria-current={i === step}>
          <button type="button" onclick={() => onStep(i)}>
            <span class="title">
              <span class="dot {text(s.severity)}"></span>
              <span class="no">{i + 1}</span>{text(s.title)}
            </span>
            <span class="meta">
              {text(s.kind)}{s.deprecated ? " \u00b7 deprecated" : ""} &middot; {text(s.id)}
            </span>
          </button>
        </li>
      {/each}
    {/if}
  </ol>
</nav>
<style>
  li > button { display: block; width: 100%; text-align: left; background: none; border: none; padding: 0; font: inherit; cursor: pointer; }
</style>
