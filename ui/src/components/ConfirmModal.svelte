<script lang="ts">
  let { open, title, message, confirmLabel = "Confirm", onConfirm, onCancel }: {
    open: boolean; title: string; message: string; confirmLabel?: string;
    onConfirm: () => void; onCancel: () => void;
  } = $props();
  let dialog = $state<HTMLDialogElement | null>(null);
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} aria-label={title} oncancel={(event) => { event.preventDefault(); onCancel(); }}>
  <h2>{title}</h2>
  <p>{message}</p>
  <div class="actions">
    <button type="button" onclick={onCancel}>Cancel</button>
    <button type="button" class="primary" onclick={onConfirm}>{confirmLabel}</button>
  </div>
</dialog>

<style>
  dialog { width: min(520px, 90vw); max-height: 85vh; overflow: auto; padding: 20px; border: 1px solid var(--line); border-radius: 10px; background: var(--panel); color: var(--ink); }
  dialog::backdrop { background: var(--overlay); }
  h2 { margin: 0 0 10px; font-size: 17px; }
  p { font-size: 13px; white-space: pre-wrap; overflow-wrap: anywhere; }
  .actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
</style>
