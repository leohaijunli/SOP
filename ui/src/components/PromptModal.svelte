<script lang="ts">
  // A reason prompt that belongs to the app rather than to the browser.
  //
  // `window.prompt` and `window.confirm` are blocking, are drawn by the webview (so they
  // do not look like the app, and may not appear at all in a packaged build), and give
  // the operator one empty box. A field operator is one-handed and often gloved, so this
  // offers the common reasons as buttons: one tap answers, and a free-text box is there
  // for the rest. It never judges the answer, it only collects it.
  let {
    open,
    title,
    message = null,
    label = "Reason",
    choices = [],
    optional = false,
    submitLabel = "Save",
    placeholder = "",
    onResolve,
  }: {
    open: boolean;
    title: string;
    message?: string | null;
    label?: string;
    /// Common answers, offered as single-tap buttons.
    choices?: string[];
    /// When true, an empty box is a valid answer (the event records no reason).
    optional?: boolean;
    submitLabel?: string;
    placeholder?: string;
    /// The typed text, or null when the operator cancelled.
    onResolve: (value: string | null) => void;
  } = $props();

  let dialog = $state<HTMLDialogElement | null>(null);
  let box = $state<HTMLTextAreaElement | null>(null);
  let text = $state("");

  // `showModal` gives the native focus trap and Escape handling; the effect keeps the
  // element in step with `open`.
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      text = "";
      dialog.showModal();
      box?.focus();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function submit(): void {
    const value = text.trim();
    if (!value && !optional) return;
    onResolve(value);
  }
</script>

<dialog
  bind:this={dialog}
  aria-label={title}
  oncancel={(event) => {
    event.preventDefault();
    onResolve(null);
  }}
>
  <h2>{title}</h2>
  {#if message}<p class="muted">{message}</p>{/if}
  {#if choices.length}
    <div class="choices">
      {#each choices as choice (choice)}
        <button type="button" onclick={() => onResolve(choice)}>{choice}</button>
      {/each}
    </div>
  {/if}
  <label for="prompt-reason">
    {label}{#if optional}<span class="muted"> (optional)</span>{/if}
  </label>
  <textarea
    id="prompt-reason"
    bind:this={box}
    bind:value={text}
    rows="2"
    {placeholder}
    onkeydown={(event) => {
      // Enter submits; Shift+Enter keeps a newline for a longer reason.
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        submit();
      }
    }}
  ></textarea>
  <div class="actions">
    <button type="button" class="primary" onclick={submit}>{submitLabel}</button>
    <button type="button" onclick={() => onResolve(null)}>Cancel (Esc)</button>
  </div>
</dialog>

<style>
  dialog {
    min-width: min(520px, 90vw);
    max-width: 90vw;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--ink);
  }
  dialog::backdrop { background: var(--overlay); }
  h2 { margin: 0 0 6px; font-size: 16px; color: var(--ink-bright); }
  p { margin: 0 0 10px; }
  .choices { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 12px; }
  .choices button { font-size: 13px; }
  label { display: block; font-size: 12px; color: var(--muted); margin-bottom: 4px; }
  textarea { width: 100%; box-sizing: border-box; }
  .actions { display: flex; gap: 8px; margin-top: 12px; }
</style>
