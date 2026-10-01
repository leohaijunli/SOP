<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { text } from "../lib/api";
  import { localDateTime } from "../lib/dates";
  import type { AttachResult, AttachmentKind, Manifest, RunEntry, RunFiles, TestPlan } from "../lib/types";
  import DoubleConfirm from "./DoubleConfirm.svelte";

  let {
    manifest,
    workingCopy,
    onChanged,
  }: {
    manifest: Manifest | null;
    /// The working copy root, so the export dialog can start in its `exports/` folder.
    workingCopy: string | null;
    /// Called after a run is removed, so the manifest (and this view) is rebuilt from disk.
    onChanged: () => void;
  } = $props();

  // The plans come from the testcase repository - the same source the Test plans view
  // reads - so a run is grouped under the plan and case it was actually started from.
  let plans: TestPlan[] = $state([]);
  // Until the plans arrive, every run would look like it belongs to no case; wait for
  // them rather than showing a grouping that is about to change.
  let plansLoaded = $state(false);
  let siteFilter = $state("");
  let operatorFilter = $state("");
  let statusFilter = $state("");
  let sensorFilter = $state("");
  let hasDevFilter = $state(false);
  let conclusionFilter = $state("");
  let message = $state("");
  let isError = $state(false);
  let busy = $state("");
  let confirmOpen = $state(false);
  let confirmMessage = $state("");
  let confirmKind = $state<"run" | "all">("run");
  let confirmRun = $state<RunEntry | null>(null);

  // The History table is one shared column layout, so every case's table lines up. The
  // widths live here, once, and are rendered as a <colgroup> in each table.
  const COLUMNS = [
    { key: "run", label: "run id", width: "19%" },
    { key: "started", label: "started", width: "12%" },
    { key: "site", label: "site", width: "10%" },
    { key: "sensor", label: "sensor", width: "9%" },
    { key: "operator", label: "operator", width: "8%" },
    { key: "outcome", label: "outcome", width: "10%" },
    { key: "conclusion", label: "conclusion", width: "9%" },
    { key: "dev", label: "dev", width: "4%" },
    { key: "files", label: "files", width: "9%" },
    { key: "open", label: "", width: "10%" },
  ] as const;

  // Which run's detail drawer is open (keyed by checklist::run), the files each opened
  // run has, the note being typed, and the drawer's own busy flag.
  let openRun = $state<string | null>(null);
  let filesByRun = $state<Record<string, RunFiles>>({});
  let noteDraft = $state<Record<string, string>>({});
  let drawerBusy = $state("");
  // The folder the most recent export wrote, so the window can offer to open it.
  let exportDir = $state<string | null>(null);

  const say = (msg: string, bad = false): void => {
    message = msg;
    isError = bad;
  };

  onMount(() => {
    void api
      .testPlans()
      .then((fresh) => (plans = fresh))
      .catch((e) => say(String(e), true))
      .finally(() => (plansLoaded = true));
  });

  const filtering = $derived(
    Boolean(
      siteFilter || operatorFilter || statusFilter || sensorFilter || hasDevFilter || conclusionFilter
    )
  );

  const sensorOf = (r: RunEntry): string => {
    const serial = r.sensor?.serial;
    if (serial !== undefined && serial !== null && String(serial).trim() !== "") return String(serial);
    const model = r.sensor?.model;
    if (model !== undefined && model !== null && String(model).trim() !== "") return String(model);
    return "";
  };

  const filtered: RunEntry[] = $derived(
    (manifest?.runs ?? [])
      .filter((r) => {
        if (siteFilter && text(r.site) !== siteFilter) return false;
        if (operatorFilter && text(r.operator) !== operatorFilter) return false;
        if (statusFilter && text(r.status) !== statusFilter) return false;
        if (sensorFilter && sensorOf(r) !== sensorFilter) return false;
        if (conclusionFilter && String(r.conclusion ?? "") !== conclusionFilter) return false;
        if (hasDevFilter && Number(r.deviations_count) === 0) return false;
        return true;
      })
      .sort((a, b) => text(b.started).localeCompare(text(a.started)))
  );

  // Filter choices come from every run, not only the ones a case claims, so the list of
  // sites does not shift as runs are grouped.
  const options = (pick: (run: RunEntry) => string): string[] => {
    const seen = new Set<string>();
    for (const run of manifest?.runs ?? []) {
      const value = pick(run);
      if (value) seen.add(value);
    }
    return [...seen].sort();
  };

  const sites: string[] = $derived(options((r) => text(r.site)));
  const operators: string[] = $derived(options((r) => text(r.operator)));
  const sensors: string[] = $derived(options(sensorOf));

  type CaseGroup = { key: string; title: string; sop: string; runs: RunEntry[] };
  type PlanGroup = { id: string; title: string; cases: CaseGroup[] };

  // Runs are matched to a case by the checklist id they recorded, which is the case's
  // front-matter `sop_id`. Anything that matches no case - a checklist opened by path, or
  // content this testcase repository does not describe - is kept rather than dropped.
  const grouped = $derived.by(() => {
    const claimed = new Set<string>();
    const out: PlanGroup[] = [];
    for (const plan of plans) {
      const cases: CaseGroup[] = [];
      for (const c of plan.cases) {
        claimed.add(c.sop_id);
        cases.push({
          key: `${plan.id}/${c.id}`,
          title: text(c.title) || c.id,
          sop: c.sop_id,
          runs: filtered.filter((r) => text(r.sop) === c.sop_id),
        });
      }
      // With a filter on, a plan with nothing to show is noise, not information.
      if (filtering && cases.every((c) => c.runs.length === 0)) continue;
      out.push({ id: plan.id, title: text(plan.title) || plan.id, cases });
    }

    const outside = new Map<string, RunEntry[]>();
    for (const run of filtered) {
      const sop = text(run.sop);
      if (claimed.has(sop)) continue;
      const rows = outside.get(sop) ?? [];
      rows.push(run);
      outside.set(sop, rows);
    }
    return {
      plans: out,
      outside: [...outside].map(([sop, runs]) => ({ sop, runs })),
    };
  });

  const runKey = (run: RunEntry): string => `${text(run.sop)}::${text(run.run_id)}`;

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  };

  // Read a run's files and notes into the drawer. Called when a drawer opens and after an
  // addition, so the list always reflects what is on disk.
  const loadFiles = async (run: RunEntry): Promise<void> => {
    try {
      const files = await api.runFiles(text(run.sop), text(run.run_id));
      filesByRun = { ...filesByRun, [runKey(run)]: files };
    } catch (e) {
      say(String(e), true);
    }
  };

  const toggleRun = async (run: RunEntry): Promise<void> => {
    const key = runKey(run);
    if (openRun === key) {
      openRun = null;
      return;
    }
    openRun = key;
    await loadFiles(run);
  };

  // Upload logs / photos / files in one picker. A big file is confirmed first, because it
// is copied into the repository. Each file is sorted by extension into the kind's folder
// (photos under `photos/`, `.log/.csv/.dat/.txt/.json` under `logs/`, everything else
// under `files/`), so the record's per-kind tallies still add up.
  const addFiles = async (run: RunEntry): Promise<void> => {
    const key = runKey(run);
    const paths = await api.pickFilesAny();
    if (!paths.length) return;
    try {
      const sizes = await api.fileSizes(paths);
      const big = paths.filter((_, i) => (sizes[i] ?? 0) > 50 * 1024 * 1024);
      if (big.length) {
        const ok = window.confirm(
          `${big.length} file(s) are larger than 50 MB and will be copied into the repository:\n\n` +
            big.join("\n") +
            "\n\nContinue?"
        );
        if (!ok) return;
      }
      drawerBusy = `upload:${key}`;
      say("");

      const PHOTO_EXT = ["jpg", "jpeg", "png", "heic", "webp", "tif", "tiff"];
      const LOG_EXT = ["log", "csv", "dat", "txt", "json"];
      const groups: Record<AttachmentKind, string[]> = { log: [], photo: [], file: [] };
      for (const path of paths) {
        const ext = path.split(".").pop()?.toLowerCase() ?? "";
        if (PHOTO_EXT.includes(ext)) groups.photo.push(path);
        else if (LOG_EXT.includes(ext)) groups.log.push(path);
        else groups.file.push(path);
      }

      let added = 0;
      const failed: AttachResult[] = [];
      for (const kind of ["log", "photo", "file"] as const) {
        const files = groups[kind];
        if (!files.length) continue;
        const results = await api.runAttachMany(text(run.sop), text(run.run_id), kind, files);
        added += results.filter((r) => r.error === null).length;
        failed.push(...results.filter((r) => r.error !== null));
      }
      if (failed.length === 0) {
        say(`added ${added} file(s)`);
      } else {
        for (const f of failed) say(`${f.source}: ${f.error}`, true);
        say(`added ${added}; ${failed.length} failed`, true);
      }
      await loadFiles(run);
      onChanged();
    } catch (e) {
      say(String(e), true);
    } finally {
      drawerBusy = "";
    }
  };

  const addNote = async (run: RunEntry): Promise<void> => {
    const key = runKey(run);
    const body = (noteDraft[key] ?? "").trim();
    if (!body) {
      say("a note cannot be empty", true);
      return;
    }
    drawerBusy = `note:${key}`;
    try {
      const files = await api.runNoteAdd(text(run.sop), text(run.run_id), body);
      filesByRun = { ...filesByRun, [key]: files };
      noteDraft = { ...noteDraft, [key]: "" };
      say("note added");
      onChanged();
    } catch (e) {
      say(String(e), true);
    } finally {
      drawerBusy = "";
    }
  };

  // Open the run's folder in the file manager. If it fails, the shell's message carries
  // the full path, so the operator can open it by hand.
  const openFolder = async (run: RunEntry): Promise<void> => {
    try {
      const path = await api.openRunFolder(text(run.sop), text(run.run_id));
      say(`opened ${path}`);
    } catch (e) {
      say(String(e), true);
    }
  };

  // Export one run's record. The native save dialog comes from the shell, so the window
  // just reports where the file landed. The run's own checklist id names the export.
  const exportRun = async (run: RunEntry): Promise<void> => {
    const runId = text(run.run_id);
    busy = `export:${runId}`;
    try {
      const result = await api.runExportDialog(text(run.sop), runId);
      if (!result) {
        say("export cancelled");
        return;
      }
      say(`wrote ${result.path} (${result.bytes} bytes)`);
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Remove one run: its record file, its run directory, and its log directory. Deleting a
  // record is not something to do by accident, so it asks first (twice) and says what goes.
  const askDeleteRun = (run: RunEntry): void => {
    confirmKind = "run";
    confirmRun = run;
    confirmMessage = `Delete run "${text(run.run_id)}" of "${text(run.sop)}"?`;
    confirmOpen = true;
  };

  const doDeleteRun = async (run: RunEntry): Promise<void> => {
    const runId = text(run.run_id);
    const sop = text(run.sop);
    busy = `delete:${runId}`;
    try {
      await api.runDelete(sop, runId);
      say(`deleted run ${runId}`);
      onChanged();
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Remove every run in the working copy. The filters do not narrow this: "all" means all,
  // and the confirmation says how many that is before anything is removed.
  const askDeleteAll = (): void => {
    confirmKind = "all";
    confirmMessage = `Delete all ${manifest?.runs.length ?? 0} run(s) in this working copy?`;
    confirmOpen = true;
  };

  const doDeleteAll = async (): Promise<void> => {
    busy = "delete-all";
    try {
      const removed = await api.runDeleteAll();
      say(`deleted ${removed} run(s)`);
      onChanged();
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  const doConfirm = (): void => {
    confirmOpen = false;
    if (confirmKind === "run" && confirmRun) void doDeleteRun(confirmRun);
    else if (confirmKind === "all") void doDeleteAll();
  };

  // Export a summary and every run into one self-contained folder. The operator picks
  // the destination; the package lands in `<dest>/export_<local time>/`.
  const exportSummary = async (): Promise<void> => {
    busy = "summary";
    try {
      const defaultDir = workingCopy ? `${workingCopy}/exports` : null;
      const report = await api.runExportPackageDialog(defaultDir);
      if (!report) {
        say("export cancelled");
        return;
      }
      exportDir = report.dir;
      const mb = (report.bytes / (1024 * 1024)).toFixed(1);
      const skipped = report.skipped.length > 0 ? `, ${report.skipped.length} skipped` : "";
      say(`exported ${report.runs} run(s) / ${report.files} file(s) / ${mb} MB → ${report.dir}${skipped}`);
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Open the folder the last export wrote, in the desktop's file manager.
  const openExport = async (): Promise<void> => {
    if (!exportDir) return;
    busy = "open-export";
    try {
      await api.openExportFolder(exportDir);
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };

  // Publish the whole working copy: the records, the logs, and every content edit that
  // has not been committed yet. The commit message is the operator's; blank takes the
  // shell's default.
  const pushRepo = async (): Promise<void> => {
    const answer = window.prompt("Commit message (blank for the default):", "");
    if (answer === null) return;
    busy = "push";
    try {
      const result = await api.repoPush(answer);
      say(result.log.join("; ") || `pushed to ${result.remote}`);
    } catch (e) {
      say(String(e), true);
    } finally {
      busy = "";
    }
  };
</script>

{#snippet runTable(rows: RunEntry[])}
  <div class="table-wrap">
    <table>
      <colgroup>
        {#each COLUMNS as col (col.key)}
          <col style={`width:${col.width}`} />
        {/each}
      </colgroup>
      <thead>
        <tr>
          {#each COLUMNS as col (col.key)}
            <th>{col.label}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as r (text(r.run_id))}
          {@const key = runKey(r)}
          <tr>
            <td class="mono ellipsis" title={text(r.run_id)}>{text(r.run_id)}</td>
            <td class="ellipsis" title={text(r.started)}>{localDateTime(r.started)}</td>
            <td class="ellipsis" title={text(r.site)}>{text(r.site)}</td>
            <td class="mono ellipsis" title={sensorOf(r)}>{sensorOf(r) || "—"}</td>
            <td class="ellipsis" title={text(r.operator)}>{text(r.operator)}</td>
            <td>
              <span class="badge {text(r.status)}">{text(r.status) || "in progress"}</span>
            </td>
            <td class="ellipsis" title={String(r.conclusion ?? "")}>
              {#if r.conclusion}
                <span class="badge {String(r.conclusion)}">{String(r.conclusion)}</span>
              {:else}
                <span class="muted">—</span>
              {/if}
            </td>
            <td>{text(r.deviations_count)}</td>
            <td class="files" title="logs · photos · files">
              {#if r.log_count + r.photo_count + r.file_count === 0}
                {#if text(r.status)}
                  <span class="muted pending" title="attach logs in the details">no logs yet</span>
                {:else}
                  <span class="muted">—</span>
                {/if}
              {:else}
                <span class="mono">{r.log_count}·{r.photo_count}·{r.file_count}</span>
              {/if}
            </td>
            <td class="actions">
              <button class="details" disabled={busy !== ""} onclick={() => void toggleRun(r)}>
                {openRun === key ? "Details ▾" : "Details ▸"}
              </button>
            </td>
          </tr>
          {#if openRun === key}
            <tr class="detail">
              <td colspan={COLUMNS.length}>
                <div class="drawer">
                  <div class="drawer-actions">
                    <button disabled={drawerBusy !== ""} onclick={() => void addFiles(r)}>
                      Add files…
                    </button>
                    <button disabled={drawerBusy !== ""} onclick={() => void openFolder(r)}>
                      Open folder
                    </button>
                    <button disabled={busy !== ""} onclick={() => void exportRun(r)}>
                      Export record
                    </button>
                    <button class="danger" disabled={busy !== ""} onclick={() => askDeleteRun(r)}>
                      Delete
                    </button>
                    {#if drawerBusy}<span class="muted">{drawerBusy}</span>{/if}
                  </div>

                  <div class="note-form">
                    <textarea
                      rows="2"
                      placeholder="Add a note (Markdown)…"
                      bind:value={noteDraft[key]}
                    ></textarea>
                    <button disabled={drawerBusy !== ""} onclick={() => void addNote(r)}>
                      Add note
                    </button>
                  </div>

                  {#if filesByRun[key]}
                    {@const data = filesByRun[key]}
                    <div class="drawer-lists">
                      <div>
                        <h4>Files — {data.logCount} log · {data.photoCount} photo · {data.fileCount} file</h4>
                        {#if data.files.length === 0}
                          <p class="muted">no files yet</p>
                        {:else}
                          <ul>
                            {#each data.files as f (f.path + f.addedAt)}
                              <li title={f.path}>
                                <span class="tag">{f.kind}</span>
                                <span class="att-name">{f.name}</span>
                                <span class="muted">{formatSize(f.size)} · {localDateTime(f.addedAt)}</span>
                                {#if f.postRun}<span class="tag after">after run</span>{/if}
                              </li>
                            {/each}
                          </ul>
                        {/if}
                      </div>
                      <div>
                        <h4>Notes</h4>
                        {#if data.notes.length === 0}
                          <p class="muted">no notes yet</p>
                        {:else}
                          <ul>
                            {#each data.notes as n, i (i)}
                              <li>
                                <span class="muted">{localDateTime(n.addedAt)}</span>
                                {#if n.postRun}<span class="tag after">after run</span>{/if}
                                <div class="note-body">{n.text}</div>
                              </li>
                            {/each}
                          </ul>
                        {/if}
                      </div>
                    </div>
                  {:else}
                    <p class="muted">loading files…</p>
                  {/if}
                </div>
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
{/snippet}

<article class="history">
  <div class="toolbar">
    <h1 style="margin:0">History</h1>
    <span class="muted">{filtered.length} run(s)</span>
    <span class="spacer"></span>
    <button disabled={busy !== ""} onclick={() => void exportSummary()}>
      {busy === "summary" ? "Exporting…" : "Export summary"}
    </button>
    <button disabled={busy !== ""} onclick={() => void pushRepo()}>
      {busy === "push" ? "Pushing…" : "Push repo"}
    </button>
    <button class="danger" disabled={busy !== ""} onclick={() => askDeleteAll()}>
      {busy === "delete-all" ? "Deleting…" : "Delete all runs"}
    </button>
  </div>

  {#if message}
    <p class={isError ? "err" : "muted"}>{message}</p>
  {/if}

  {#if exportDir}
    <p class="muted">
      Export folder:
      <button class="link" disabled={busy !== ""} onclick={() => void openExport()}>
        {busy === "open-export" ? "Opening…" : exportDir}
      </button>
    </p>
  {/if}

  <div class="filters">
    <select bind:value={siteFilter}>
      <option value="">All sites</option>
      {#each sites as site (site)}
        <option value={site}>{site}</option>
      {/each}
    </select>
    <select bind:value={operatorFilter}>
      <option value="">All operators</option>
      {#each operators as op (op)}
        <option value={op}>{op}</option>
      {/each}
    </select>
    <select bind:value={statusFilter}>
      <option value="">All outcomes</option>
      <option value="complete">complete</option>
      <option value="partial">partial</option>
      <option value="aborted">aborted</option>
    </select>
    <select bind:value={sensorFilter}>
      <option value="">All sensors</option>
      {#each sensors as s (s)}
        <option value={s}>{s}</option>
      {/each}
    </select>
    <select bind:value={conclusionFilter}>
      <option value="">Any conclusion</option>
      <option value="pass">pass</option>
      <option value="fail">fail</option>
      <option value="inconclusive">inconclusive</option>
    </select>
    <label class="check"><input type="checkbox" bind:checked={hasDevFilter} /> has deviations</label>
  </div>

  {#if !filtered.length}
    <p class="empty">no runs</p>
  {:else if !plansLoaded}
    <p class="muted">loading test plans…</p>
  {:else}
    {#each grouped.plans as plan (plan.id)}
      <section class="plan">
        <h2>{plan.title}</h2>
        {#each plan.cases as c (c.key)}
          {#if c.runs.length || !filtering}
            <section class="case">
              <h3>
                {c.title}
                <span class="mono muted">{c.sop}</span>
                <span class="count">{c.runs.length} run(s)</span>
              </h3>
              {#if c.runs.length}
                {@render runTable(c.runs)}
              {:else}
                <p class="empty">no runs yet</p>
              {/if}
            </section>
          {/if}
        {/each}
      </section>
    {/each}

    {#if grouped.outside.length}
      <section class="plan">
        <h2>Runs outside a test plan</h2>
        {#each grouped.outside as group (group.sop)}
          <section class="case">
            <h3>
              {group.sop}
              <span class="count">{group.runs.length} run(s)</span>
            </h3>
            {@render runTable(group.runs)}
          </section>
        {/each}
      </section>
    {/if}
  {/if}
</article>

<DoubleConfirm
  open={confirmOpen}
  message={confirmMessage}
  onConfirm={doConfirm}
  onCancel={() => (confirmOpen = false)}
/>

<style>
  .toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 12px; }
  .spacer { flex: 1; }
  .filters { display: flex; gap: 8px; margin-bottom: 12px; }
  .plan { margin: 0 0 24px; }
  .plan h2 { margin: 0 0 8px; font-size: 15px; letter-spacing: .02em; }
  .case { margin: 0 0 16px; padding-left: 12px; border-left: 2px solid var(--line); }
  .case h3 { margin: 0 0 6px; font-size: 13px; font-weight: 600; display: flex; gap: 8px; align-items: baseline; }
  .case h3 .count { font-weight: 400; font-size: 12px; color: var(--muted); }
  .table-wrap { overflow-x: auto; }
  table { width: 100%; table-layout: fixed; border-collapse: collapse; }
  th, td { border: 1px solid var(--line); padding: 6px 10px; text-align: left; overflow: hidden; text-overflow: ellipsis; }
  th { background: var(--quote); font-size: 12px; text-transform: uppercase; letter-spacing: .06em; white-space: nowrap; }
  td { white-space: nowrap; }
  td.mono { font-family: var(--mono); font-size: 12px; }
  td.actions { white-space: nowrap; }
  td button { font-size: 12px; white-space: nowrap; }
  td button + button { margin-left: 6px; }
  td.files .pending { font-size: 11px; }
  tr.detail > td { background: var(--quote); white-space: normal; padding: 12px; }
  .drawer { display: flex; flex-direction: column; gap: 10px; }
  .drawer-actions { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  .drawer-actions button { margin: 0; }
  .note-form { display: flex; gap: 8px; align-items: flex-start; }
  .note-form textarea { flex: 1; font-family: inherit; font-size: 13px; }
  .drawer-lists { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .drawer-lists h4 { margin: 0 0 4px; font-size: 12px; text-transform: uppercase; letter-spacing: .06em; color: var(--muted); }
  .drawer-lists ul { margin: 0; padding-left: 16px; }
  .drawer-lists li { margin: 2px 0; }
  .drawer-lists .att-name {
    color: var(--accent); font-family: var(--mono); font-size: 12px;
  }
  .note-body { white-space: pre-wrap; }
  .tag { display: inline-block; font-size: 11px; padding: 0 4px; border: 1px solid var(--line); border-radius: 4px; text-transform: uppercase; }
  .tag.after { color: var(--warn); border-color: var(--warn); }
  button.danger { color: var(--critical); }
  button.link { background: none; border: none; padding: 0; color: var(--accent, inherit); text-decoration: underline; font: inherit; cursor: pointer; }
  .badge.complete { color: var(--ok); }
  .badge.partial { color: var(--warn); }
  .badge.aborted { color: var(--critical); }
  .badge.pass { color: var(--ok); }
  .badge.fail { color: var(--critical); }
  .badge.inconclusive { color: var(--warn); }
  .filters .check { display: inline-flex; align-items: center; gap: 4px; font-size: 13px; }
</style>
