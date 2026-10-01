<script lang="ts">
  import { text } from "../lib/api";
  import type { Status } from "../lib/types";
  import { cycleTheme, loadTheme } from "../lib/theme";
  import type { Theme } from "../lib/theme";

  let {
    title,
    projectId,
    counts,
    status,
    view,
    helpOpen,
    onView,
    onToggleHelp,
    operator,
    onSwitchOperator,
  }: {
    title: unknown;
    projectId: unknown;
    counts: () => string;
    /// Git and validation state, or null before the first load. Shown as badges so the
    /// operator can see whether the working copy is committed and the content is clean.
    status: Status | null;
    view: string;
    helpOpen: boolean;
    onView: (v: string) => void;
    onToggleHelp: () => void;
    operator: string;
    onSwitchOperator: () => void;
  } = $props();

  let theme: Theme = $state(loadTheme());

  const toggleTheme = (): void => {
    theme = cycleTheme(theme);
  };

  // The manage views are behind one more tap on purpose: none of them should be reachable
  // by a mis-tap on a run screen held in a gloved hand.
  let manageOpen = $state(false);
  const manageViews = ["authoring", "settings"];

  const choose = (next: string): void => {
    manageOpen = false;
    onView(next);
  };
</script>

<header>
  <div class="header-top">
    <img class="mark" src="/condor.svg" alt="" aria-hidden="true" />
    <h1 id="projectname" title={text(projectId)}>{text(title) || "field-sop"}</h1>
    <span class="tag">desktop</span>
    <span class="counts">{counts()}</span>
    {#if status}
      {#if status.git.isRepository}
        <span
          class="badge git"
          class:warn={status.git.dirty}
          title={`branch ${status.git.branch ?? "detached"} · remote ${status.git.remote}`}
        >
          {status.git.branch ?? "detached"}
          {#if status.git.dirty}&middot; uncommitted{/if}
          {#if status.git.ahead}&middot; {status.git.ahead} ahead{/if}
          {#if status.git.behind}&middot; {status.git.behind} behind{/if}
        </span>
      {/if}
      <span
        class="badge"
        class:bad={status.validation.errors > 0}
        class:warn={status.validation.errors === 0 && status.validation.warnings > 0}
        title="problems reported by a full content validation"
      >
        {#if status.validation.errors > 0}
          {status.validation.errors} error(s)
        {:else if status.validation.warnings > 0}
          {status.validation.warnings} warning(s)
        {:else}
          content clean
        {/if}
      </span>
    {/if}
    <span class="spacer"></span>
    {#if operator}
      <button class="operator" onclick={onSwitchOperator} title="Switch operator">
        <span class="mono">{operator}</span>
      </button>
    {/if}
    <button class="theme-btn" onclick={toggleTheme} title="Switch theme (auto / light / dark)">
      {theme === "dark" ? "Dark" : theme === "light" ? "Light" : "Auto"}
    </button>
    <span class="toolbar nav">
      <button class:primary={view === "testplans"} onclick={() => onView("testplans")}>Test Plans</button>
      <button class:primary={view === "run"} onclick={() => onView("run")}>Run</button>
      <button class:primary={view === "history"} onclick={() => onView("history")}>History</button>
      <button class:primary={view === "browse"} onclick={() => onView("browse")}>Browse</button>
      <details class="manage" bind:open={manageOpen}>
        <summary class:active={manageViews.includes(view)}>Manage</summary>
        <div class="menu">
          <button class:primary={view === "authoring"} onclick={() => choose("authoring")}>Edit SOP</button>
          <button class:primary={view === "settings"} onclick={() => choose("settings")}>Settings</button>
        </div>
      </details>
    </span>
  </div>
  <div class="header-tools">
    {#if view === "browse"}
      <span class="tools-label">Browse display</span>
      <button title="Toggle help panel (F1)" onclick={onToggleHelp} class:active={helpOpen}>Help</button>
    {/if}
  </div>
</header>

<style>
  header { display: block; padding: 0; background: var(--bg-dark); border-bottom: 1px solid var(--line); }
  .header-top { display: flex; align-items: baseline; gap: 14px; flex-wrap: wrap; padding: 10px 16px 6px; }
  .mark {
    width: 26px; height: 26px; border-radius: 7px; align-self: center;
    box-shadow: 0 1px 3px var(--shadow);
  }
  .header-tools {
    display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
    padding: 2px 16px 8px;
  }
  .tools-label {
    font-size: 11px; text-transform: uppercase; letter-spacing: .08em;
    color: var(--muted); margin-right: 4px;
  }
  .theme-btn { font-size: 11px; padding: 2px 10px; }
  .operator { font-size: 12px; padding: 2px 10px; }
  .operator .mono { font-size: 12px; }
  .toolbar { display: flex; gap: 6px; align-items: center; }
  .toolbar button { font-size: 12px; padding: 4px 10px; }
  .manage { position: relative; }
  .manage summary {
    list-style: none; cursor: pointer; font-size: 12px; padding: 4px 10px;
    border: 1px solid var(--line); border-radius: 4px; color: var(--ink); user-select: none;
  }
  .manage summary::-webkit-details-marker { display: none; }
  .manage summary::after { content: " \25be"; color: var(--muted); }
  .manage[open] summary, .manage summary.active { border-color: var(--accent); }
  .manage .menu {
    position: absolute; right: 0; top: calc(100% + 4px); z-index: 10;
    display: flex; flex-direction: column; gap: 4px; padding: 6px;
    background: var(--panel); border: 1px solid var(--line); border-radius: 6px;
    box-shadow: 0 6px 18px var(--shadow);
  }
  .manage .menu button { text-align: left; white-space: nowrap; }
  button.active { border-color: var(--accent); }
  .badge {
    font-size: 11px; font-family: var(--mono); padding: 2px 8px; border-radius: 10px;
    border: 1px solid var(--line); color: var(--muted); white-space: nowrap;
  }
  .badge.git { color: var(--muted); }
  .badge.warn { color: var(--warn); border-color: var(--warn); }
  .badge.bad { color: var(--critical); border-color: var(--critical); }
</style>
