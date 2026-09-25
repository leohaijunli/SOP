<script lang="ts">
  import { onMount } from "svelte";
  import Header from "./components/Header.svelte";
  import StepsNav from "./components/StepsNav.svelte";
  import Detail from "./components/Detail.svelte";
  import HelpPanel from "./components/HelpPanel.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import ProjectPanel from "./components/ProjectPanel.svelte";
  import AuthoringPanel from "./components/AuthoringPanel.svelte";
  import * as api from "./lib/api";
  import type { Manifest } from "./lib/types";

  // Which screen the window is showing. Browse is the run view; the others are
  // configuration and content editing, reached from the toolbar.
  type View = "browse" | "settings" | "project" | "authoring";

  let manifest: Manifest | null = $state(null);
  let error: string | null = $state(null);
  let view: View = $state("browse");
  let helpOpen = $state(true);
  let showSteps = $state(true);

  // Which checklist and step are selected, as indexes into the (sorted) manifest.
  let checklist = $state(0);
  let step = $state(0);
  let help = $state<string | null>(null);
  let helpQuery = $state("");
  let stepQuery = $state("");

  const counts = (): string => {
    const m = manifest;
    if (!m) return "";
    return `${m.procedures.length} procedures \u00b7 ${m.checklists.length} checklists \u00b7 ${m.runs.length} runs \u00b7 ${m.help.length} help pages`;
  };

  const openView = (next: string): void => {
    if (next === "browse" || next === "settings" || next === "project" || next === "authoring") {
      view = next;
    }
    // The authoring screen edits whatever checklist is current.
  };

  const refresh = async (): Promise<void> => {
    try {
      manifest = await api.manifest();
      error = null;
    } catch (e) {
      error = String(e);
    }
  };

  onMount(() => {
    void refresh();
  });

  // Deep links and keyboard shortcuts mirror the preview server, so the desktop app
  // behaves the same way the browser did: #<sop_id>/<step-id>/<help-id>.
  const readHash = (): void => {
    if (!manifest) return;
    const [sopId, stepRef, helpId] = location.hash.replace(/^#/, "").split("/");
    if (sopId) {
      const index = manifest.checklists.findIndex((c) => api.text(c.sop_id) === sopId);
      if (index >= 0) checklist = index;
    }
    const steps = manifest.checklists[checklist].steps;
    if (stepRef !== undefined && stepRef !== "") {
      const byId = steps.findIndex((candidate) => api.text(candidate.id) === stepRef);
      const idx = Number(stepRef);
      if (byId >= 0) step = byId;
      else if (Number.isInteger(idx) && idx >= 0 && idx < steps.length) step = idx;
    }
    if (helpId) help = helpId;
  };

  const writeHash = (): void => {
    if (!manifest) return;
    const c = manifest.checklists[checklist];
    const parts = [api.text(c.sop_id), String(step)];
    if (help) parts.push(help);
    history.replaceState(null, "", `#${parts.join("/")}`);
  };

  $effect(() => {
    if (view !== "browse") return;
    writeHash();
  });

  onMount(() => {
    window.addEventListener("hashchange", readHash);
    return () => window.removeEventListener("hashchange", readHash);
  });

  const onKey = (event: KeyboardEvent): void => {
    if ((event.target as HTMLElement).matches("input, select, textarea")) {
      if (event.key === "Escape") (event.target as HTMLElement).blur();
      return;
    }
    const steps = manifest?.checklists[checklist]?.steps ?? [];
    if (event.key === "F1") { event.preventDefault(); helpOpen = !helpOpen; }
    else if (event.key === "/") { event.preventDefault(); helpOpen = true; }
    else if (event.key === "j" && step < steps.length - 1) { step++; }
    else if (event.key === "k" && step > 0) { step--; }
  };
</script>

<svelte:window onkeydown={onKey} />

{#if error}
  <main class="fatal">
    <p class="err">Could not load the manifest: {error}</p>
  </main>
{:else}
  <Header
    {counts}
    view={view}
    onView={openView}
    title={manifest?.project?.title ?? null}
    projectId={manifest?.project?.project_id ?? null}
    {helpOpen}
    onToggleHelp={() => (helpOpen = !helpOpen)}
    {showSteps}
    onToggleSteps={() => (showSteps = !showSteps)}
  />

  <main class:with-help={helpOpen && view === "browse"} class:with-steps={showSteps && view === "browse"}>
    {#if view === "browse"}
      {#if showSteps}
        <StepsNav
          manifest={manifest}
          {checklist}
          {step}
          {stepQuery}
          onChecklist={(i) => { checklist = i; step = 0; }}
          onStep={(i) => (step = i)}
          onStepQuery={(q) => (stepQuery = q)}
        />
      {/if}
      <Detail manifest={manifest} {checklist} {step} onEdit={() => (view = "authoring")} />
      {#if helpOpen}
        <HelpPanel manifest={manifest} {help} {helpQuery} onHelp={(h) => (help = h)} onQuery={(q) => (helpQuery = q)} />
      {/if}
    {:else if view === "settings"}
      <SettingsPanel />
    {:else if view === "project"}
      <ProjectPanel />
    {:else}
      <AuthoringPanel manifest={manifest} {checklist} onDone={() => { void refresh(); view = "browse"; }} />
    {/if}
  </main>
{/if}