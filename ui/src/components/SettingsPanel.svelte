<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import type { ProjectField } from "../lib/types";
  import SensorsEditor from "./SensorsEditor.svelte";
  import DevicesEditor from "./DevicesEditor.svelte";

  // All settings are held locally and written back as one JSON document on Confirm.
  let settings: Record<string, string> = $state({});
  let repoPath = $state("");
  let remote = $state("");
  let message = $state("");
  let isError = $state(false);
  let saving = $state(false);

  // Project metadata lives in the same JSON document, under `project`.
  let fields: ProjectField[] = $state([]);
  let configPath = $state("");

  const KEY_DESC: Record<string, string> = {
    repository: "The repository to open. A path, or a path to create.",
    remote: "Name of the git remote to use, for example origin. Not a URL.",
    branch: "Branch runs are recorded on. Absent means the current branch.",
    "help-open": "Whether the help panel starts open: true or false.",
    sensors: "Sensor models and their serial numbers, as model: serial, serial; model (for example UAS-MAG: 1001; RM3100). Fills the picker in the start-a-run form.",
    devices: "Other equipment and their serial numbers, as kind/name: serial, serial; name (for example GNSS receiver/Trimble R10: 123; UAV). Fills the Equipment used picker in the start-a-run form.",
    "recent-repositories": "Working copies opened before, most recent first.",
  };

  const FIELD_KEYS = [
    "title",
    "project_id",
    "institution",
    "lead",
    "started",
    "updated",
    "summary",
    "contact",
  ];

  const DESCRIPTION: Record<string, string> = {
    title: "The name the app shows, and the name a reader of an old record sees.",
    project_id: "Stable id. Changing it is a rename, not a new project; prefer not to.",
    institution: "Who is responsible for the work.",
    lead: "Who to ask about the content.",
    started: "When the work started, as YYYY-MM-DD.",
    updated: "When the project file last changed, as YYYY-MM-DD.",
    summary: "One sentence describing the work. Carried into the manifest.",
    contact: "Where to write about the project.",
  };

  const values = $derived.by(() => {
    const out: Record<string, string> = {};
    for (const f of fields) out[f.key] = f.value ?? "";
    return out;
  });

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  const reload = async (): Promise<void> => {
    try {
      const json = await api.configLoad();
      const parsed = JSON.parse(json);
      settings = parsed.settings ?? {};
      repoPath = settings.repository ?? "";
      remote = settings.remote ?? "";
      const project = parsed.project ?? {};
      configPath = parsed.path ?? "";
      fields = FIELD_KEYS.map((key) => ({
        key,
        value: project[key] ?? "",
        description: DESCRIPTION[key] ?? "",
      }));
    } catch (e) {
      say(String(e), true);
    }
  };

  // Every edit is written back to the config file as it happens, so a field operator who
  // leaves the page is not surprised to find an added device missing on the way back. The
  // Confirm button is still there to force a save and report the path.
  const persist = async (): Promise<void> => {
    try {
      const next = { ...settings, repository: repoPath.trim(), remote: remote.trim() };
      const existing = JSON.parse(await api.configLoad());
      const config = { ...existing, settings: { ...next }, project: { ...values } };
      configPath = await api.configSave(JSON.stringify(config, null, 2));
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  const confirm = async (): Promise<void> => {
    saving = true;
    await persist();
    saving = false;
    say(`saved ${configPath}`);
  };

  const openRepo = async (): Promise<void> => {
    try {
      if (!repoPath.trim()) {
        say("type a repository path first", true);
        return;
      }
      await api.openRepository(repoPath.trim());
      say("repository changed");
      isError = false;
      await reload();
    } catch (e) {
      say(String(e), true);
    }
  };

  const browseRepo = async (): Promise<void> => {
    const picked = await api.pickDirectory();
    if (!picked) return;
    repoPath = picked;
    // Remember the choice first, so a folder that fails the repository check does not
    // silently revert the field to its old value on the next load.
    await persist();
    await openRepo();
  };

  const setRemote = async (): Promise<void> => {
    try {
      if (!remote.trim()) return;
      await api.remoteSet(remote.trim());
      say("remote set");
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  onMount(() => {
    void reload().catch((e) => say(String(e), true));
  });
</script>

<article class="settings">
  <div class="toolbar">
    <h1 style="margin:0">Settings</h1>
    <span class="spacer"></span>
    <button class="primary" onclick={() => void confirm()} disabled={saving}>
      {saving ? "Saving…" : "Confirm"}
    </button>
  </div>
  {#if configPath}
    <p class="muted mono config-path">{configPath}</p>
  {/if}

  <section class="sgroup">
    <h2>Repository</h2>
    <div class="field">
      <div class="field-row">
        <input type="text" placeholder="/path/to/your-repo" bind:value={repoPath} onchange={() => void persist()} />
        <button onclick={() => void browseRepo()}>Browse</button>
        <button onclick={() => void openRepo()}>Open</button>
      </div>
      <span class="desc">Open another field-sop repository. The choice is remembered for next launch.</span>
    </div>
  </section>

  <section class="sgroup">
    <h2>Git remote</h2>
    <div class="field">
      <div class="field-row">
        <input type="text" placeholder="git@host:org/repo.git" bind:value={remote} onchange={() => void persist()} />
        <button onclick={() => void setRemote()}>Set remote</button>
      </div>
      <span class="desc">The URL is stored in git config, not in the app's settings.</span>
    </div>
  </section>

  <section class="sgroup">
    <h2>Project</h2>
    <p class="desc">The project's name is content, not an application preference. Edit it
    here and press Confirm: the values are written together to the JSON config file and
    applied to the repository.</p>
    <div class="grid">
      {#each fields as field (field.key)}
        <div class="field">
          <label for={field.key}>{field.key}</label>
          <input
            id={field.key}
            type="text"
            value={field.value ?? ""}
            placeholder="(unset)"
            onchange={(e) => { field.value = (e.currentTarget as HTMLInputElement).value; void persist(); }}
          />
          <span class="desc">{DESCRIPTION[field.key]}</span>
        </div>
      {/each}
    </div>
  </section>

  <hr class="divider" />

  <section class="sgroup">
    <h2>Application settings</h2>
    <p class="muted">
      Settings marked <span class="scope">repository</span> are kept in the repository, so
      they travel with it. The rest stay on this machine. Press Confirm to write everything
      to the JSON config file and apply it.
    </p>
    <div class="grid">
      {#each Object.keys(KEY_DESC) as key (key)}
        <div class="field">
          <label for={key}>{key}</label>
          {#if key === "sensors"}
            <SensorsEditor value={settings[key] ?? ""} onChange={(next) => { settings[key] = next; void persist(); }} />
          {:else if key === "devices"}
            <DevicesEditor value={settings[key] ?? ""} onChange={(next) => { settings[key] = next; void persist(); }} />
          {:else}
            <input
              id={key}
              type="text"
              value={settings[key] ?? ""}
              placeholder="(unset)"
              onchange={(e) => { settings[key] = (e.currentTarget as HTMLInputElement).value; void persist(); }}
            />
          {/if}
          <span class="desc">{KEY_DESC[key]}</span>
        </div>
      {/each}
    </div>
  </section>

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}
</article>

<style>
  .sgroup { display: flex; flex-direction: column; gap: 12px; }
  .sgroup h2 {
    margin: 0; font-size: 14px; letter-spacing: .02em; color: var(--ink-bright);
    border-bottom: 1px solid var(--line); padding-bottom: 6px;
  }
  .config-path { margin: 0 0 12px; }
  /* Fields sit side by side where the window is wide enough, and wrap to one column
     on a narrow one. Text fields never stretch across the whole window. */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(340px, 100%), 1fr));
    gap: 4px 24px;
    align-items: start;
  }
  .grid .field { min-width: 0; }
  .grid input[type="text"] { width: 100%; max-width: 480px; }
  .field-row { max-width: 640px; }
  .field-row input { flex: 1; min-width: 0; }
</style>