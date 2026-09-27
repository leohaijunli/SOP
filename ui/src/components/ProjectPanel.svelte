<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import type { ProjectField } from "../lib/types";

  let fields: ProjectField[] = $state([]);
  let path = $state("");
  let message = $state("");
  let isError = $state(false);
  let saving = $state(false);

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
      const project = parsed.project ?? {};
      path = parsed.path ?? "";
      fields = FIELD_KEYS.map((key) => ({
        key,
        value: project[key] ?? "",
        description: DESCRIPTION[key] ?? "",
      }));
    } catch (e) {
      say(String(e), true);
    }
  };

  const confirm = async (): Promise<void> => {
    saving = true;
    try {
      const existing = JSON.parse(await api.configLoad());
      const config = { ...existing, project: { ...values } };
      path = await api.configSave(JSON.stringify(config, null, 2));
      say(`saved ${path}`);
      isError = false;
    } catch (e) {
      say(String(e), true);
    } finally {
      saving = false;
    }
  };

  onMount(() => {
    void reload().catch((e) => say(String(e), true));
  });
</script>

<article class="settings">
  <div class="toolbar">
    <h1 style="margin:0">Project</h1>
    <span class="spacer"></span>
    <button class="primary" onclick={() => void confirm()} disabled={saving}>
      {saving ? "Saving…" : "Confirm"}
    </button>
  </div>
  {#if path}
    <p class="muted mono">{path}</p>
  {/if}
  <p class="desc">The project's name is content, not an application preference. Edit it
  here and press Confirm: the values are written together to a JSON config file and
  applied to the repository.</p>

  {#each fields as field (field.key)}
    <div class="field">
      <label>{field.key}</label>
      <input
        type="text"
        value={field.value ?? ""}
        placeholder="(unset)"
        onchange={(e) => { field.value = (e.currentTarget as HTMLInputElement).value; }}
      />
      <span class="desc">{DESCRIPTION[field.key]}</span>
    </div>
  {/each}

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}
</article>