<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";

  // All settings are held locally and written back as one JSON document on Confirm.
  let settings: Record<string, string> = $state({});
  let repoPath = $state("");
  let remote = $state("");
  let message = $state("");
  let isError = $state(false);
  let saving = $state(false);

  const KEY_DESC: Record<string, string> = {
    repository: "Working copy to open. A path, or a path to create.",
    remote: "Name of the git remote to use, for example origin. Not a URL.",
    branch: "Branch runs are recorded on. Absent means the current branch.",
    "testcase-repo": "Separate repository holding the testplan/ tree (e.g. a clone of this repo on the testcases branch). Absent uses the working copy.",
    "help-open": "Whether the help panel starts open: true or false.",
    sensors: "Sensor models and their serial numbers, as model: serial, serial; model (for example UAS-MAG: 1001; RM3100). Fills the picker in the start-a-run form.",
    "recent-repositories": "Working copies opened before, most recent first.",
  };

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
    } catch (e) {
      say(String(e), true);
    }
  };

  const confirm = async (): Promise<void> => {
    saving = true;
    try {
      settings = { ...settings, repository: repoPath.trim(), remote: remote.trim() };
      const existing = JSON.parse(await api.configLoad());
      const config = { ...existing, settings: { ...settings } };
      const written = await api.configSave(JSON.stringify(config, null, 2));
      say(`saved ${written}`);
      isError = false;
    } catch (e) {
      say(String(e), true);
    } finally {
      saving = false;
    }
  };

  const openRepo = async (): Promise<void> => {
    try {
      await api.openRepository(repoPath.trim());
      say("working copy changed");
      isError = false;
      await reload();
    } catch (e) {
      say(String(e), true);
    }
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

  <h2>Working copy</h2>
  <div class="field">
    <div class="field-row">
      <input type="text" placeholder="/path/to/working-copy" bind:value={repoPath} />
      <button onclick={() => void openRepo()}>Open</button>
    </div>
    <span class="desc">Open another field-sop repository. The choice is remembered for next launch.</span>
  </div>

  <h2>Git remote</h2>
  <div class="field">
    <div class="field-row">
      <input type="text" placeholder="git@host:org/repo.git" bind:value={remote} />
      <button onclick={() => void setRemote()}>Set remote</button>
    </div>
    <span class="desc">The URL is stored in git config, not in the app's settings.</span>
  </div>

  <hr class="divider" />

  <h2>Application settings</h2>
  <p class="muted">
    Settings marked <span class="scope">repository</span> are kept in the working copy, so
    they travel with it. The rest stay on this machine. Press Confirm to write everything
    to the JSON config file and apply it.
  </p>
  {#each Object.keys(KEY_DESC) as key (key)}
    <div class="field">
      <label for={key}>{key}</label>
      <input
        id={key}
        type="text"
        value={settings[key] ?? ""}
        placeholder="(unset)"
        onchange={(e) => { settings[key] = (e.currentTarget as HTMLInputElement).value; }}
      />
      <span class="desc">{KEY_DESC[key]}</span>
    </div>
  {/each}

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}
</article>