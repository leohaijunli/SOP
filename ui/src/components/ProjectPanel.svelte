<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import type { ProjectView } from "../lib/types";

  let view: ProjectView | null = $state(null);
  let message = $state("");
  let isError = $state(false);

  const reload = async (): Promise<void> => {
    view = await api.projectFields();
  };

  const save = async (key: string, value: string): Promise<void> => {
    try {
      view = await api.projectSet(key, value);
      message = `${key} saved`;
      isError = false;
    } catch (e) {
      message = String(e);
      isError = true;
    }
  };

  onMount(() => {
    void reload().catch((e) => {
      message = String(e);
      isError = true;
    });
  });
</script>

<article class="settings">
  <h1>Project</h1>
  {#if view}
    <p class="muted mono">{view.path}</p>
    <p class="desc">The project's name is content in the repository, not an application
    preference: it lives in project.md and travels with the records, so two operators
    editing the same records always see the same name.</p>

    {#each view.fields as field (field.key)}
      <div class="field">
        <label>{field.key}</label>
        <div class="field-row">
          <input
            type="text"
            value={field.value ?? ""}
            placeholder="(unset)"
            onchange={(e) => void save(field.key, (e.currentTarget as HTMLInputElement).value)}
          />
          <button onclick={() => void save(field.key, "")}>Save</button>
        </div>
        <span class="desc">{field.description}</span>
      </div>
    {/each}
  {/if}

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}
</article>