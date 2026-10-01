<script lang="ts">
  // The operator sign-in. field-sop records who did the run, so whoever is at the
  // keyboard signs in once and every run picks that name up automatically. Choosing a
  // name from the list is one tap; a gloved hand can manage that. A free box covers the
  // operator who is not on the machine yet.
  import { onMount } from "svelte";

  let {
    knownOperators,
    onLogin,
    onRemove,
  }: {
    /// Names this machine has seen before, newest first.
    knownOperators: string[];
    onLogin: (name: string) => void;
    onRemove: (name: string) => void;
  } = $props();

  let box = $state<HTMLInputElement | null>(null);
  let name = $state("");

  onMount(() => box?.focus());

  function choose(value: string): void {
    const trimmed = value.trim();
    if (!trimmed) return;
    onLogin(trimmed);
  }
</script>

<div class="scrim">
  <div class="card">
    <img class="mark" src="/condor.svg" alt="" aria-hidden="true" />
    <h1>Sign in</h1>
    <p class="muted">Who is running the procedures today?</p>

    {#if knownOperators.length}
      <div class="known">
        {#each knownOperators as op (op)}
          <span class="known-item">
            <button type="button" onclick={() => choose(op)}>{op}</button>
            <button
              type="button"
              class="remove"
              title={`Remove ${op} from this list`}
              onclick={() => onRemove(op)}
            >&times;</button>
          </span>
        {/each}
      </div>
      <div class="or"><span>or</span></div>
    {/if}

    <form
      onsubmit={(event) => {
        event.preventDefault();
        choose(name);
      }}
    >
      <input
        bind:this={box}
        bind:value={name}
        placeholder="your name"
        autocomplete="off"
      />
      <button type="submit" class="primary" disabled={!name.trim()}>Start</button>
    </form>
  </div>
</div>

<style>
  .scrim {
    position: fixed; inset: 0; z-index: 50;
    display: grid; place-items: center;
    background: var(--bg-darker);
  }
  .card {
    width: min(400px, 90vw);
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 24px;
    text-align: center;
    box-shadow: 0 12px 32px var(--shadow);
  }
  .mark { width: 40px; height: 40px; border-radius: 9px; margin-bottom: 10px; }
  h1 { margin: 0 0 4px; font-size: 20px; color: var(--ink-bright); }
  .muted { margin: 0 0 16px; font-size: 13px; }
  .known { display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin-bottom: 6px; }
  .known-item { display: inline-flex; align-items: center; gap: 2px; }
  .known-item button { font-size: 14px; padding: 6px 8px; }
  .known-item .remove { padding: 6px 8px; color: var(--muted); }
  .or { color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: .08em; margin: 10px 0 4px; }
  form { display: flex; gap: 8px; margin-top: 8px; }
  form input { flex: 1; text-align: center; }
</style>