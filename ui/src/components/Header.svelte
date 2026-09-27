<script lang="ts">
  import { text } from "../lib/api";

  let {
    title,
    projectId,
    counts,
    view,
    helpOpen,
    onView,
    onToggleHelp,
  }: {
    title: unknown;
    projectId: unknown;
    counts: () => string;
    view: string;
    helpOpen: boolean;
    onView: (v: string) => void;
    onToggleHelp: () => void;
  } = $props();
</script>

<header>
  <div class="header-top">
    <h1 id="projectname" title={text(projectId)}>{text(title) || "field-sop"}</h1>
    <span class="tag">desktop</span>
    <span class="counts">{counts()}</span>
    <span class="spacer"></span>
    <span class="toolbar nav">
      <button class:primary={view === "run"} onclick={() => onView("run")}>Run</button>
      <button class:primary={view === "history"} onclick={() => onView("history")}>History</button>
      <button class:primary={view === "browse"} onclick={() => onView("browse")}>Browse</button>
        <button class:primary={view === "testplans"} onclick={() => onView("testplans")}>Test Plans</button>
      <button class:primary={view === "authoring"} onclick={() => onView("authoring")}>Edit</button>
      <button class:primary={view === "project"} onclick={() => onView("project")}>Project</button>
      <button class:primary={view === "settings"} onclick={() => onView("settings")}>Settings</button>
      <span class="divider-btn"></span>
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
  .header-tools {
    display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
    padding: 2px 16px 8px;
  }
  .tools-label {
    font-size: 11px; text-transform: uppercase; letter-spacing: .08em;
    color: var(--muted); margin-right: 4px;
  }
  .toolbar { display: flex; gap: 6px; align-items: center; }
  .toolbar button { font-size: 12px; padding: 4px 10px; }
  .divider-btn { width: 1px; height: 20px; background: var(--line); margin: 0 4px; }
  button.active { border-color: var(--accent); }
</style>