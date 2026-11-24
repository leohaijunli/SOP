<script lang="ts">
  import { text } from "../lib/text";
  import { cycleTheme, loadTheme, type Theme } from "../lib/theme";

  let { title, counts, view, workingCopy, operator, helpOpen, directoryLocked, onView, onToggleHelp, onDirectory }: {
    title: unknown; counts: () => string; view: string; workingCopy: string; operator: string; helpOpen: boolean;
    onView: (view: string) => void; onToggleHelp: () => void; onDirectory: () => void;
    directoryLocked: boolean;
  } = $props();
  let theme = $state<Theme>(loadTheme());
  let manageOpen = $state(false);
  const advanced = ["settings", "testplans", "browse", "authoring", "run"];
  const displayPath = $derived(workingCopy.replace("\\\\?\\UNC\\", "\\\\").replace("\\\\?\\", ""));
  function choose(next: string): void { manageOpen = false; onView(next); }
</script>

<header>
  <div class="identity">
    <img class="mark" src="/condor.svg" alt="" aria-hidden="true" />
    <h1>{text(title) || "Experiment Notebook"}</h1>
    <span class="local">Local data</span>
    <span class="counts">{counts()}</span>
    <span class="spacer"></span>
    {#if operator}<span class="local">{operator}</span>{/if}
    <button class="small-button" onclick={() => theme = cycleTheme(theme)} title="Theme: system / light / dark">{theme === "dark" ? "Dark" : theme === "light" ? "Light" : "System"}</button>
  </div>
  <div class="navigation">
    <nav aria-label="Main navigation">
      <button class:primary={view === "notes"} onclick={() => choose("notes")}>Test Notes</button>
      <button class:primary={view === "logs"} onclick={() => choose("logs")}>Logs</button>
      <button class:primary={view === "search"} onclick={() => choose("search")} title="Search (Ctrl + K)">Search</button>
      <details class="manage" bind:open={manageOpen}>
        <summary class:active={advanced.includes(view)}>Manage</summary>
        <div class="menu">
          <button onclick={() => choose("settings")}>Working Directory &amp; Settings</button>
          <button onclick={() => choose("testplans")}>Test Plans</button>
          <button onclick={() => choose("run")}>Run Checklist</button>
          <button onclick={() => choose("browse")}>Browse SOP</button>
          <button onclick={() => choose("authoring")}>Edit SOP</button>
        </div>
      </details>
      {#if view === "browse" || view === "run"}<button class:active={helpOpen} onclick={onToggleHelp} title="Show help (F1)">Help</button>{/if}
    </nav>
    <div class="directory"><span>Working directory</span><button disabled={directoryLocked} onclick={onDirectory} title={directoryLocked ? "Switch directories after the file operation finishes" : workingCopy ? `${workingCopy}\nClick to switch working directory` : "Choose the folder containing logs and test plans"}>{displayPath || "Not selected — choose a folder…"}</button></div>
  </div>
</header>

<style>
  header { display: block; padding: 0; background: var(--bg-dark); border-bottom: 1px solid var(--line); }
  .identity { display: flex; align-items: center; gap: 12px; padding: 10px 20px 8px; flex-wrap: wrap; }
  .mark { width: 25px; height: 25px; border-radius: 6px; }
  h1 { font-size: 15px; margin: 0; }
  .local { font-size: 11px; color: var(--muted); }
  .counts { font-size: 12px; }
  .small-button { font-size: 11px; padding: 3px 8px; }
  .spacer { flex: 1; }
  .navigation { display: flex; gap: 16px; align-items: center; justify-content: space-between; padding: 0 20px 10px; flex-wrap: wrap; }
  nav { display: flex; gap: 6px; align-items: center; overflow: visible; padding: 0; background: none; border: 0; }
  nav button { padding: 5px 14px; font-size: 13px; }
  .manage { position: relative; }
  summary { list-style: none; cursor: pointer; padding: 5px 14px; font-size: 13px; border: 1px solid var(--border); border-radius: 6px; }
  summary::-webkit-details-marker { display: none; }
  summary::after { content: " ▾"; color: var(--muted); }
  summary.active, details[open] summary, button.active { border-color: var(--accent); }
  .menu { position: absolute; left: 0; top: calc(100% + 6px); z-index: 20; min-width: 160px; padding: 6px; display: flex; flex-direction: column; gap: 4px; background: var(--panel); border: 1px solid var(--line); border-radius: 8px; box-shadow: 0 6px 18px var(--shadow); }
  .menu button { text-align: left; white-space: nowrap; font-size: 12px; }
  .directory { display: flex; align-items: center; gap: 8px; min-width: 0; max-width: 55%; font-size: 11px; color: var(--muted); }
  .directory > span { flex-shrink: 0; }
  .directory button { padding: 0; background: none; border: 0; color: var(--accent); font-family: var(--mono); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (max-width: 960px) { .directory { max-width: 100%; width: 100%; } .identity, .navigation { padding-left: 14px; padding-right: 14px; } }
</style>
