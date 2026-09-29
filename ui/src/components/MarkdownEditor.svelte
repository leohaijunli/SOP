<script lang="ts">
  // A note editor with a live Markdown preview. Notes are stored verbatim in the run
  // record, so what the operator previews here is what the record will render later; the
  // renderer is the same one the run screen and the preview server use.
  //
  // The parent owns the text (matching SensorsEditor): this component renders it and
  // reports edits, and exposes `focus()` so a keyboard shortcut in the parent can reach
  // the textarea.
  import { htmlOf } from "../lib/md.svelte";

  let {
    value,
    onChange,
    onSubmit,
    rows = 6,
    placeholder = "Write a note…",
    submitLabel = "Add note",
    hint = "Markdown: **bold**, *italic*, `code`, - lists, > quotes, links, tables.",
  }: {
    value: string;
    onChange: (next: string) => void;
    onSubmit: () => void;
    rows?: number;
    placeholder?: string;
    submitLabel?: string;
    hint?: string;
  } = $props();

  let field = $state<HTMLTextAreaElement | null>(null);

  const filled = $derived(value.trim().length > 0);

  export function focus(): void {
    field?.focus();
  }

  // Enter inserts a newline, because notes are Markdown and may be multi-line. Ctrl/Cmd
  // + Enter is the shortcut for the whole field, matching the run screen's own bindings.
  const keydown = (event: KeyboardEvent): void => {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      onSubmit();
    }
  };
</script>

<div class="md-editor">
  <textarea
    bind:this={field}
    rows={rows}
    {placeholder}
    value={value}
    oninput={(e) => onChange((e.currentTarget as HTMLTextAreaElement).value)}
    onkeydown={keydown}
  ></textarea>
  <div class="md-actions">
    <span class="muted">{hint} <kbd>Ctrl</kbd>+<kbd>Enter</kbd> to save.</span>
    <button class="primary" disabled={!filled} onclick={() => onSubmit()}>{submitLabel}</button>
  </div>
  {#if filled}
    <div class="md-preview">
      <div class="md-preview-label">Preview</div>
      <div class="prose">{@html htmlOf(value)}</div>
    </div>
  {/if}
</div>

<style>
  .md-editor { display: flex; flex-direction: column; gap: 6px; }
  textarea {
    width: 100%; min-height: 96px; resize: vertical;
    font-family: var(--mono); font-size: 13px; line-height: 1.5;
  }
  .md-actions { display: flex; gap: 8px; align-items: center; justify-content: space-between; flex-wrap: wrap; }
  .md-actions .muted { font-size: 11px; }
  kbd {
    font-family: var(--mono); font-size: 10px; border: 1px solid var(--line);
    border-radius: 4px; padding: 0 4px; background: var(--bg-dark);
  }
  .md-preview { border: 1px dashed var(--line); border-radius: 6px; padding: 6px 12px; background: var(--quote); }
  .md-preview-label { font-size: 10px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); }
  .md-preview :global(p:first-child) { margin-top: 4px; }
  .md-preview :global(p:last-child) { margin-bottom: 4px; }
</style>
