<script lang="ts">
  import { htmlOf } from "../lib/md.svelte";
  import { text } from "../lib/text";
  import type { Capture, Manifest } from "../lib/types";

  let {
    manifest,
    checklist,
    step,
    onEdit,
  }: {
    manifest: Manifest | null;
    checklist: number;
    step: number;
    onEdit: () => void;
  } = $props();

  let stepEntry = $derived(
    (() => {
      if (!manifest || !manifest.checklists[checklist]) return null;
      return manifest.checklists[checklist].steps[step] ?? null;
    })()
  );

  const expectedText = (c: Capture): string => {
    const e = c.expected;
    if (e === null || e === undefined) return "";
    if (typeof e === "object" && ("min" in e || "max" in e)) {
      const ev = e as { min?: number; max?: number };
      const min = ev.min === undefined ? "\u2212\u221e" : String(ev.min);
      const max = ev.max === undefined ? "\u221e" : String(ev.max);
      return `expected between ${min} and ${max}`;
    }
    return `expected ${text(e)}`;
  };

  const hintOf = (c: Capture): string => {
    const parts = [text(c.type), text(c.unit)];
    if (c.options && c.options.length) parts.push(`one of ${c.options.map((o) => JSON.stringify(o)).join(", ")}`);
    return parts.filter(Boolean).join(" \u00b7 ");
  };
</script>

<article>
  {#if !stepEntry}
    <p class="empty">This checklist has no steps.</p>
  {:else}
    <div class="notice">
      Recording arrives with the app's record mode. This view renders the checklist
      exactly as an operator reads it.
      <button class="link" onclick={onEdit}>Edit this step</button>
    </div>
    <h1>{text(stepEntry.title)}</h1>
    <div class="badges">
      <span class="badge {text(stepEntry.severity)}">{text(stepEntry.severity)}</span>
      <span class="badge">{text(stepEntry.kind)}</span>
      {#if stepEntry.deprecated}<span class="badge">deprecated</span>{/if}
      <span class="badge">{text(stepEntry.id)}</span>
      <span class="badge">from {text(stepEntry.source)}</span>
    </div>
    {#if stepEntry.captures?.length}
      <section class="captures">
        <h2>Captures</h2>
        {#each stepEntry.captures as capture (text(capture.key))}
          <div class="capture">
            <div class="label">{text(capture.label)}{#if capture.required}<span class="req">*</span>{/if}</div>
            <div class="hint">{text(capture.key)} &middot; {hintOf(capture)}</div>
            <div class="control">recording arrives with the app</div>
            {#if expectedText(capture)}
              <div class="expected">{expectedText(capture)} &mdash; highlighted, never judged</div>
            {/if}
          </div>
        {/each}
      </section>
    {/if}
    <div class="prose">{@html htmlOf(stepEntry.body)}</div>
  {/if}
</article>

<style>
  button.link {
    background: none; border: none; color: var(--accent);
    padding: 0; font-size: 13px; cursor: pointer; margin-left: 8px;
  }
</style>