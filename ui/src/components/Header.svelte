<script lang="ts">
  import { text } from "../lib/api";

  let {
    title,
    projectId,
    counts,
    view,
    helpOpen,
    showSteps,
    onView,
    onToggleHelp,
    onToggleSteps,
  }: {
    title: unknown;
    projectId: unknown;
    counts: () => string;
    view: string;
    helpOpen: boolean;
    showSteps: boolean;
    onView: (v: string) => void;
    onToggleHelp: () => void;
    onToggleSteps: () => void;
  } = $props();
</script>

<header>
  <h1 id="projectname" title={text(projectId)}>{text(title) || "field-sop"}</h1>
  <span class="tag">desktop</span>
  <span class="counts">{counts()}</span>
  <span class="spacer"></span>
  <span class="toolbar">
    <button class:primary={view === "run"} onclick={() => onView("run")}>Run</button>
    <button class:primary={view === "history"} onclick={() => onView("history")}>History</button>
    <button class:primary={view === "browse"} onclick={() => onView("browse")}>Browse</button>
    <button class:primary={view === "authoring"} onclick={() => onView("authoring")}>Edit</button>
    <button class:primary={view === "project"} onclick={() => onView("project")}>Project</button>
    <button class:primary={view === "settings"} onclick={() => onView("settings")}>Settings</button>
    <span class="divider-btn"></span>
    <button title="Toggle step list" onclick={onToggleSteps} class:active={showSteps}>Steps</button>
    <button title="Toggle help panel (F1)" onclick={onToggleHelp} class:active={helpOpen}>Help</button>
  </span>
</header>

<style>
  .toolbar { display: flex; gap: 6px; align-items: center; }
  .toolbar button { font-size: 12px; padding: 4px 10px; }
  .divider-btn { width: 1px; height: 20px; background: var(--line); margin: 0 4px; }
  button.active { border-color: var(--accent); }
</style>