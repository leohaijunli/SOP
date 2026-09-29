<script lang="ts">
  import { htmlOf } from "../lib/md.svelte";
  import { text } from "../lib/text";
  import type { Manifest } from "../lib/types";

  let {
    manifest,
    help,
    helpQuery,
    onHelp,
    onQuery,
  }: {
    manifest: Manifest | null;
    help: string | null;
    helpQuery: string;
    onHelp: (help: string) => void;
    onQuery: (query: string) => void;
  } = $props();

let pages = $derived(manifest?.help ?? []);

let matches = $derived(
  (() => {
    const query = helpQuery.trim().toLowerCase();
    if (!query) return pages;
    return pages.filter((page) =>
      [page.title, page.summary, page.section, page.body, (page.tags ?? []).join(" ")]
        .some((f) => text(f).toLowerCase().includes(query))
    );
  })()
);

let selected = $derived(matches.find((p) => text(p.help_id) === help) ?? matches[0]);

let sections = $derived(
  (() => {
    const out: { title: string; pages: typeof pages }[] = [];
    for (const page of matches) {
      const title = text(page.section);
      const last = out[out.length - 1];
      if (last && last.title === title) last.pages.push(page);
      else out.push({ title, pages: [page] });
    }
    return out;
  })()
);
</script>

<aside>
  <input
    type="search"
    placeholder="Search help (press /)"
    value={helpQuery}
    oninput={(e) => onQuery((e.currentTarget as HTMLInputElement).value)}
  />
  {#if !pages.length}
    <p class="empty">no help pages</p>
  {:else if !matches.length}
    <p class="empty">no matching pages</p>
  {:else}
    <h2>Pages</h2>
    <ul class="help-index">
      {#each sections as section}
        <li class="empty">{section.title}</li>
        {#each section.pages as page (text(page.help_id))}
          <li data-help={text(page.help_id)} aria-current={selected && text(page.help_id) === text(selected.help_id)}>
            <button type="button" onclick={() => onHelp(text(page.help_id))}>{text(page.title)}</button>
          </li>
        {/each}
      {/each}
    </ul>
    {#if selected}
      <h2 id="helptitle">{text(selected.title)}</h2>
      <div class="prose">{@html htmlOf(selected.body)}</div>
    {/if}
  {/if}
</aside>
<style>
  li > button { display: block; width: 100%; text-align: left; background: none; border: none; padding: 0; font: inherit; cursor: pointer; }
</style>
