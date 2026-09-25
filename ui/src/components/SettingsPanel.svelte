<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import type { SettingRow } from "../lib/types";

  let rows: SettingRow[] = $state([]);
  let path = $state("");
  let remote = $state("");
  let repoPath = $state("");
  let message = $state("");
  let isError = $state(false);

  const reload = async (): Promise<void> => {
    rows = await api.settingsRows();
    path = await api.settingsPath();
    remote = (await api.remoteUrl()) ?? "";
  };

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  const save = async (key: string, value: string, unset = false): Promise<void> => {
    try {
      rows = unset ? await api.settingsUnset(key) : await api.settingsSet(key, value);
      say(unset ? `${key} cleared` : `${key} saved`);
      isError = false;
    } catch (e) {
      say(String(e), true);
    }
  };

  const setRemote = async (): Promise<void> => {
    try {
      const url = remote.trim();
      if (!url) return;
      remote = await api.remoteSet(url);
      say("remote set");
      isError = false;
    } catch (e) {
      say(String(e), true);
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

  onMount(() => {
    void reload().catch((e) => say(String(e), true));
  });
</script>

<article class="settings">
  <h1>Settings</h1>
  <p class="muted mono">{path}</p>

  <h2>Working copy</h2>
  <div class="field">
    <div class="field-row">
      <input
        type="text"
        placeholder="/path/to/working-copy"
        bind:value={repoPath}
        onkeydown={(e) => { if (e.key === "Enter") void openRepo(); }}
      />
      <button onclick={() => void openRepo()}>Open</button>
    </div>
    <span class="desc">Open another field-sop repository. The choice is remembered for next launch.</span>
  </div>

  <h2>Git remote</h2>
  <div class="field">
    <div class="field-row">
      <input
        type="text"
        placeholder="git@host:org/repo.git"
        bind:value={remote}
        onkeydown={(e) => { if (e.key === "Enter") void setRemote(); }}
      />
      <button onclick={() => void setRemote()}>Set remote</button>
    </div>
    <span class="desc">The URL is stored in git config, not in the app's settings.</span>
  </div>

  <hr class="divider" />

  <h2>Application settings</h2>
  {#each rows as row (row.key)}
    <div class="field">
      <label>{row.key}</label>
      <div class="field-row">
        <input
          type="text"
          value={row.value ?? ""}
          placeholder="(unset)"
          onchange={(e) => void save(row.key, (e.currentTarget as HTMLInputElement).value)}
        />
        {#if row.value !== null}
          <button onclick={() => void save(row.key, "", true)}>Clear</button>
        {/if}
      </div>
      <span class="desc">{row.description}</span>
    </div>
  {/each}

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}
</article>