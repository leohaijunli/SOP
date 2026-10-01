<script lang="ts">
  // A two-step destructive confirmation. Deleting something cannot be undone, so the
  // operator has to confirm twice, and the second dialog appears in a different place
  // than the first. That stops a gloved thumb from walking the same "confirm" muscle
  // memory across a second tap: the dialog is somewhere else, so the tap misses it.
  //
  // Stage one asks whether to proceed; stage two moves and states the real cost.
  let {
    open,
    message = null,
    confirmLabel = "Delete",
    onConfirm,
    onCancel,
  }: {
    open: boolean;
    message?: string | null;
    confirmLabel?: string;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  let dialog = $state<HTMLDialogElement | null>(null);
  // 1 = centered "are you sure"; 2 = moved "really delete". Reset whenever reopened.
  let stage = $state(1);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      stage = 1;
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function proceed(): void {
    stage = 2;
  }

  function finish(): void {
    onConfirm();
  }

  function abort(): void {
    onCancel();
  }
</script>

<dialog
  bind:this={dialog}
  class:stage2={stage === 2}
  aria-label={stage === 1 ? "Confirm action" : "Confirm again"}
  oncancel={(event) => {
    event.preventDefault();
    abort();
  }}
>
  {#if stage === 1}
    <h2>Hold on…</h2>
    <p class="muted">{message ?? "This cannot be undone. Probably. Maybe. Definitely."}</p>
    <p class="muted">Are you absolutely, positively, no-take-backs sure?</p>
    <div class="actions">
      <button type="button" class="primary" onclick={proceed}>Yeah, I guess</button>
      <button type="button" onclick={abort}>Cancel (Esc)</button>
    </div>
  {:else}
    <h2 class="danger">Really {confirmLabel.toLowerCase()}?</h2>
    <p class="muted">{message ?? "This cannot be undone. Probably. Maybe. Definitely."}</p>
    <p class="warn">It's gone forever. No recycle bin. No time machine. No crying.</p>
    <div class="actions">
      <button type="button" class="danger" onclick={finish}>Yes, delete it all</button>
      <button type="button" onclick={abort}>Go back (Esc)</button>
    </div>
  {/if}
</dialog>

<style>
  dialog {
    min-width: min(460px, 90vw);
    max-width: 90vw;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--ink);
    transition: transform 0.16s ease;
  }
  dialog::backdrop { background: var(--overlay); }
  /* Second pass appears somewhere else, so a rehearsed tap in the old spot misses. */
  dialog.stage2 { transform: translate(26vw, 8vh); }
  h2 { margin: 0 0 6px; font-size: 16px; color: var(--ink-bright); }
  h2.danger { color: var(--critical); }
  p { margin: 0 0 8px; }
  .warn { color: var(--warn); font-size: 13px; }
  .actions { display: flex; gap: 8px; margin-top: 12px; justify-content: flex-end; }
  button.danger {
    background: var(--critical); border-color: var(--critical);
    color: var(--bg-darker); font-weight: 600;
  }
</style>