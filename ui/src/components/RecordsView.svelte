<script lang="ts">
  import * as api from "../lib/api";
  import { text } from "../lib/text";
  import { htmlOf } from "../lib/md.svelte";
  import { localDateTime, localDay } from "../lib/dates";
  import { readDraft, writeDraft } from "../lib/drafts";
  import { collectFiles, fileMatches, fileTime, hasTag, notePreview, recordMatches, runKey, runTitle,
    sortFiles, tagChoices, type ManagedFile, type RecordMode, type TimeBasis, type TimeOrder } from "../lib/records";
  import type { AttachResult, AttachmentKind, Manifest, RunEntry, RunView } from "../lib/types";
  import FileList from "./FileList.svelte";
  import NewNoteDialog from "./NewNoteDialog.svelte";
  import ConfirmModal from "./ConfirmModal.svelte";
  import DoubleConfirm from "./DoubleConfirm.svelte";

  let { manifest, workingCopy, mode, signedInOperator = "", onMode, onChanged, onResumeRun, onBusy }: {
    manifest: Manifest | null; workingCopy: string; mode: RecordMode; signedInOperator?: string;
    onMode: (mode: RecordMode) => void; onChanged: () => Promise<void>;
    onResumeRun: (sop: string, runId: string) => void;
    onBusy: (started: boolean) => void;
  } = $props();

  let queries = $state<Record<RecordMode, string>>({ notes: "", logs: "", search: "" });
  const query = $derived(queries[mode]);
  let queryInput = $state<HTMLInputElement | null>(null);
  let selectedKey = $state("");
  let noteDrafts = $state<Record<string, string>>({});
  let newNoteOpen = $state(false);
  let busy = $state("");
  let message = $state("");
  let isError = $state(false);
  let tagFilter = $state("");
  let kindFilter = $state("log");
  let statusFilter = $state("");
  let recordFilter = $state("");
  let importRecord = $state("");
  let timeOrder = $state<TimeOrder>("newest");
  let timeBasis = $state<TimeBasis>("file");
  let fromDate = $state("");
  let toDate = $state("");
  let fileLimit = $state(100);
  let recordLimit = $state(100);
  let editingTitle = $state<string | null>(null);
  let titleDraft = $state("");
  let deleteRun = $state<RunEntry | null>(null);
  let exportDir = $state("");
  let confirmation = $state<{ title: string; message: string; confirmLabel: string } | null>(null);
  let resolveConfirmation: ((answer: boolean) => void) | null = null;

  const allFiles = $derived(collectFiles(manifest));
  const tags = $derived(tagChoices(allFiles));
  const allRuns = $derived([...(manifest?.runs ?? [])].sort((a, b) => text(b.started).localeCompare(text(a.started))));
  const filteredRuns = $derived(allRuns.filter((run) => recordMatches(run, manifest, query)
    && (mode !== "notes" || !statusFilter || (statusFilter === "ongoing" ? !text(run.status) : Boolean(text(run.status))))
    && (!tagFilter || (run.files ?? []).some((file) => hasTag(file, tagFilter)))));
  const selected = $derived(filteredRuns.find((run) => runKey(run) === selectedKey) ?? filteredRuns[0] ?? null);
  const selectedFiles = $derived(sortFiles(allFiles.filter((row) => row.run && selected && runKey(row.run) === runKey(selected)), timeOrder, timeBasis));
  const filteredFiles = $derived(sortFiles(allFiles.filter((row) => {
    if (!fileMatches(row, query) || !hasTag(row.file, tagFilter)) return false;
    if (mode === "logs" && kindFilter && row.file.kind !== kindFilter) return false;
    if (recordFilter && (row.run ? runKey(row.run) : "unassigned") !== recordFilter) return false;
    const day = localDay(fileTime(row.file, timeBasis));
    if (fromDate && (!day || day < fromDate)) return false;
    if (toDate && (!day || day > toDate)) return false;
    return true;
  }), timeOrder, timeBasis));
  const sites = $derived([...new Set(allRuns.map((run) => text(run.site)).filter(Boolean))].sort());
  const activeDraft = $derived(selected ? noteDrafts[runKey(selected)] ?? readDraft(`${workingCopy}::${runKey(selected)}`) : "");

  export function focusSearch(): void { queryInput?.focus(); queryInput?.select(); }
  export function newNote(): void { newNoteOpen = true; }

  function say(value: string, bad = false): void { message = value; isError = bad; }
  function beginAction(value: string): void { busy = value; onBusy(true); }
  function endAction(): void { busy = ""; onBusy(false); }
  function setQuery(value: string): void { queries[mode] = value; fileLimit = 100; recordLimit = 100; }
  function resetFilters(): void {
    queries[mode] = ""; tagFilter = ""; statusFilter = ""; recordFilter = ""; fromDate = ""; toDate = "";
    fileLimit = 100; recordLimit = 100;
  }
  function chooseTag(tag: string): void { tagFilter = tag; kindFilter = ""; onMode("logs"); }
  function showRecord(row: ManagedFile): void {
    if (!row.run) return;
    selectedKey = runKey(row.run); queries.notes = ""; tagFilter = ""; statusFilter = "";
    onMode("notes");
  }
  function rememberNote(run: RunEntry, value: string): void {
    noteDrafts[runKey(run)] = value;
    writeDraft(`${workingCopy}::${runKey(run)}`, value);
  }
  function ask(title: string, message: string, confirmLabel = "Confirm"): Promise<boolean> {
    confirmation = { title, message, confirmLabel };
    return new Promise((resolve) => { resolveConfirmation = resolve; });
  }
  function settle(answer: boolean): void {
    const resolve = resolveConfirmation; resolveConfirmation = null; confirmation = null; resolve?.(answer);
  }

  async function changed(): Promise<void> { await onChanged(); }
  async function created(run: RunView): Promise<void> {
    newNoteOpen = false; selectedKey = `${run.sop}::${run.runId}`; queries.notes = ""; statusFilter = ""; tagFilter = "";
    onMode("notes"); say("Test note created. You can continue recording or add log files.");
    await changed();
  }
  async function addNote(run: RunEntry): Promise<void> {
    const key = runKey(run);
    const body = (noteDrafts[key] ?? readDraft(`${workingCopy}::${key}`)).trim();
    if (busy || !body) return;
    beginAction("note");
    try {
      await api.runNoteAdd(text(run.sop), text(run.run_id), body);
      rememberNote(run, ""); say("Note saved"); await changed();
    } catch (e) { say(String(e), true); }
    finally { endAction(); }
  }
  async function rename(run: RunEntry): Promise<void> {
    if (busy || !titleDraft.trim()) return;
    beginAction("title");
    try {
      await api.runRecord(text(run.sop), text(run.run_id), { type: "RunTitleSet", at: "", title: titleDraft.trim() });
      editingTitle = null; say("Title saved"); await changed();
    } catch (e) { say(String(e), true); }
    finally { endAction(); }
  }
  async function finish(run: RunEntry): Promise<void> {
    if (busy) return;
    const confirmed = await ask("Complete record", "Mark this experiment as complete. You can still add notes, logs, remarks, and tags afterward.", "Complete record");
    if (!confirmed) return;
    beginAction("finish");
    try { await api.runEnd(text(run.sop), text(run.run_id), "complete"); say("Record completed"); await changed(); }
    catch (e) { say(String(e), true); }
    finally { endAction(); }
  }
  async function saveFile(row: ManagedFile, remark: string, tags: string[]): Promise<boolean> {
    if (busy) return false;
    beginAction("metadata");
    try {
      if (row.run) await api.runFileUpdate(text(row.run.sop), text(row.run.run_id), row.file.path, remark, tags);
      else await api.workspaceLogUpdate(row.file.path, remark, tags);
      say("File remarks and tags saved"); await changed(); return true;
    } catch (e) { say(String(e), true); return false; }
    finally { endAction(); }
  }
  async function openFile(row: ManagedFile): Promise<void> {
    try { await api.openFileFolder(row.file.path, row.run ? text(row.run.sop) : null, row.run ? text(row.run.run_id) : null); }
    catch (e) { say(String(e), true); }
  }
  async function addFiles(run: RunEntry | null, kind: AttachmentKind): Promise<void> {
    if (busy) return;
    beginAction("import");
    try {
      const paths = await api.pickFiles(kind);
      if (!paths.length) return;
      const sizes = await api.fileSizes(paths);
      const large = paths.filter((_, i) => (sizes[i] ?? 0) > 50 * 1024 ** 2);
      if (large.length && !await ask("Add large files", `${large.length} files exceed 50 MB and will be copied into the working directory. The originals will be kept:\n\n${large.join("\n")}`, "Continue import")) return;
      let results: AttachResult[] = [];
      if (!run) results = await api.workspaceLogImport(paths);
      else if (kind === "file") {
        const photos = paths.filter((path) => /\.(jpg|jpeg|png|heic|webp|tif|tiff)$/i.test(path));
        const other = paths.filter((path) => !photos.includes(path));
        if (photos.length) results.push(...await api.runAttachMany(text(run.sop), text(run.run_id), "photo", photos));
        if (other.length) results.push(...await api.runAttachMany(text(run.sop), text(run.run_id), "file", other));
      } else results = await api.runAttachMany(text(run.sop), text(run.run_id), kind, paths);
      const failed = results.filter((result) => result.error);
      say(`Processed ${results.length - failed.length} files${failed.length ? `; ${failed.length} failed:\n${failed.map((result) => `${result.source}: ${result.error}`).join("\n")}` : " (identical files are not saved twice)"}`, failed.length > 0);
      await changed();
    } catch (e) { say(String(e), true); }
    finally { endAction(); }
  }
  async function remove(): Promise<void> {
    const run = deleteRun; deleteRun = null;
    if (!run || busy) return;
    beginAction("delete");
    try { await api.runDelete(text(run.sop), text(run.run_id)); selectedKey = ""; say("Record deleted"); await changed(); }
    catch (e) { say(String(e), true); }
    finally { endAction(); }
  }
  async function exportRecord(run: RunEntry): Promise<void> {
    if (busy) return;
    beginAction("export");
    try { const result = await api.runExportDialog(text(run.sop), text(run.run_id)); if (result) say(`Record exported: ${result.path}`); }
    catch (e) { say(String(e), true); }
    finally { endAction(); }
  }
  async function exportWorkspace(): Promise<void> {
    if (busy) return;
    beginAction("export-workspace");
    try {
      const result = await api.runExportPackageDialog(`${workingCopy}/exports`);
      if (result) {
        exportDir = result.dir;
        say(`Exported ${result.runs} records and ${result.files} files: ${result.dir}${result.skipped.length ? `; skipped ${result.skipped.length} links or special files` : ""}${result.failed.length ? `; ${result.failed.length} records not found` : ""}`, result.failed.length > 0);
      }
    } catch (e) { say(String(e), true); }
    finally { endAction(); }
  }
  async function openFolder(run: RunEntry): Promise<void> {
    try { await api.openRunFolder(text(run.sop), text(run.run_id)); }
    catch (e) { say(String(e), true); }
  }
  const statusLabel = (run: RunEntry): string => ({ complete: "Completed", partial: "Partial", aborted: "Aborted" }[text(run.status)] ?? "In progress");
  const conclusionLabel = (run: RunEntry): string => ({ pass: "Pass", fail: "Fail", inconclusive: "Inconclusive" }[text(run.conclusion)] ?? text(run.conclusion));
</script>

<article class="records">
  <div class="toolbar">
    <h1>{mode === "logs" ? "Log History" : mode === "search" ? "Search" : "Test Notes"}</h1>
    <span class="muted count">{mode === "logs" ? `${filteredFiles.length} files` : `${filteredRuns.length} records`}</span>
    <span class="spacer"></span>
    {#if mode === "logs"}<button class="primary" disabled={Boolean(busy)} onclick={() => void addFiles(allRuns.find((run) => runKey(run) === importRecord) ?? null, "log")}>{busy === "import" ? "Adding…" : "Add log files…"}</button>{/if}
    <button class:primary={mode !== "logs"} disabled={Boolean(busy)} onclick={newNote}>New note</button>
    <button disabled={Boolean(busy)} onclick={() => void changed()}>Refresh</button>
    <button disabled={Boolean(busy)} onclick={() => void exportWorkspace()}>{busy === "export-workspace" ? "Exporting…" : "Export working directory"}</button>
  </div>
  <div class="search-row">
    <input id="record-search" bind:this={queryInput} type="search" value={query} oninput={(event) => setQuery(event.currentTarget.value)} aria-label="Search records and logs" placeholder={mode === "logs" ? "Search file names, remarks, tags, or linked records…" : "Search titles, notes, file names, remarks, tags, locations, or equipment…"} />
    {#if mode === "notes"}<select aria-label="Record status" bind:value={statusFilter}><option value="">All records</option><option value="ongoing">In progress</option><option value="ended">Ended</option></select>{/if}
    <button onclick={resetFilters} title="Clear search and filters">Clear</button>
  </div>
  {#if message}<p class:err={isError} class="feedback" role="status">{message}</p>{/if}
  {#if exportDir}<p class="muted small">Export folder: <button class="link" onclick={() => void api.openExportFolder(exportDir).catch((e) => say(String(e), true))}>{exportDir}</button></p>{/if}
  {#if mode === "search"}<p class="muted small">Search notes and file remarks / tags together, without reading large log bodies. <kbd>Ctrl</kbd>+<kbd>K</kbd> to search; <kbd>Ctrl</kbd>+<kbd>N</kbd> for a new note.</p>{/if}

  {#if mode === "logs" || mode === "search" && query.trim()}
    <div class="log-filters">
      {#if mode === "logs"}<select aria-label="File type" bind:value={kindFilter}><option value="log">Logs</option><option value="photo">Photos</option><option value="file">Other attachments</option><option value="">All files</option></select>{/if}
      <select aria-label="Filter by tag" bind:value={tagFilter}><option value="">All tags</option>{#each tags as tag (tag)}<option value={tag}>{tag}</option>{/each}</select>
      <select aria-label="Time order" bind:value={timeOrder}><option value="newest">Newest first</option><option value="oldest">Oldest first</option></select>
      <select aria-label="Sort time basis" bind:value={timeBasis}><option value="file">By log time</option><option value="added">By import time</option></select>
      <details class="more-filters"><summary>More filters</summary><div>
        <label>From date<input type="date" bind:value={fromDate} /></label><label>To date<input type="date" bind:value={toDate} /></label>
        <select aria-label="Filter by linked record" bind:value={recordFilter}><option value="">All linked records</option><option value="unassigned">Unlinked logs</option>{#each allRuns as run (runKey(run))}<option value={runKey(run)}>{runTitle(run, manifest)} · {localDay(run.started)}</option>{/each}</select>
      </div></details>
    </div>
  {/if}
  {#if tagFilter}<p class="small muted">Active tag: <button class="tag-chip" onclick={() => tagFilter = ""}>{tagFilter} ×</button></p>{/if}

  {#if !manifest}<p class="empty">Reading working directory…</p>
  {:else if mode === "search" && !query.trim()}<div class="empty-card"><h2>Find experiment records and logs</h2><p>Combine keywords, such as "temperature retest" or "GPS warmup".</p></div>
  {:else}
    {#if mode !== "logs"}
      <div class="record-layout">
        <nav class="record-list" aria-label="Test notes list">
          {#each filteredRuns.slice(0, recordLimit) as run (runKey(run))}
            <button class:current={selected && runKey(run) === runKey(selected)} onclick={() => selectedKey = runKey(run)}>
              <strong>{runTitle(run, manifest)}</strong>
              <span class="small muted">{localDateTime(run.started)} · {statusLabel(run)}</span>
              {#if notePreview(run, query)}<span class="preview">{notePreview(run, query)}</span>{/if}
              <span class="small muted">{run.notes?.length ?? 0} notes · {run.log_count} logs{run.site ? ` · ${text(run.site)}` : ""}</span>
            </button>
          {/each}
          {#if filteredRuns.length > recordLimit}<button onclick={() => recordLimit += 100}>Show more records</button>{/if}
          {#if !filteredRuns.length}<p class="empty">{query || tagFilter || statusFilter ? "No matching records" : "No records yet. Use New note to get started."}</p>{/if}
        </nav>
        <section class="record-detail">
          {#if selected}
            {@const run = selected}
            {#if editingTitle === runKey(run)}<form class="title-form" onsubmit={(event) => { event.preventDefault(); void rename(run); }}><input aria-label="Record title" bind:value={titleDraft} disabled={Boolean(busy)} /><button type="submit" class="primary" disabled={Boolean(busy) || !titleDraft.trim()}>Save title</button><button type="button" onclick={() => editingTitle = null}>Cancel</button></form>
            {:else}<div class="title-row"><h2>{runTitle(run, manifest)}</h2>{#if run.editable}<button class="link small" disabled={Boolean(busy)} onclick={() => { editingTitle = runKey(run); titleDraft = runTitle(run, manifest); }}>Edit title</button>{/if}</div>{/if}
            <p class="record-meta"><span class="badge">{statusLabel(run)}</span>{#if run.conclusion}<span class="badge">{conclusionLabel(run)}</span>{/if}<span>{localDateTime(run.started)}</span>{#if run.operator}<span>{text(run.operator)}</span>{/if}{#if run.site}<span>{text(run.site)}</span>{/if}{#if run.sensor}<span>{text(run.sensor.model)} {text(run.sensor.serial)}</span>{/if}</p>
            <div class="detail-actions">
              <button disabled={Boolean(busy) || !run.editable} onclick={() => void addFiles(run, "log")}>Add log files…</button>
              <button disabled={Boolean(busy) || !run.editable} onclick={() => void addFiles(run, "file")}>Add photos / attachments…</button>
              <button disabled={Boolean(busy)} onclick={() => void openFolder(run)}>Open record folder</button>
              <button disabled={Boolean(busy)} onclick={() => void exportRecord(run)}>Export record</button>
              {#if run.notebook && !run.status}<button disabled={Boolean(busy)} onclick={() => void finish(run)}>Complete record</button>{/if}
              {#if !run.notebook && !run.status && run.editable}<button disabled={Boolean(busy)} onclick={() => onResumeRun(text(run.sop), text(run.run_id))}>Resume checklist run</button>{/if}
              <details class="record-more"><summary>More</summary><button class="danger" disabled={Boolean(busy)} onclick={() => deleteRun = run}>Delete this record…</button></details>
            </div>
            {#if run.editable}
              <form class="note-form" onsubmit={(event) => { event.preventDefault(); void addNote(run); }}>
                <label for="record-note">Add experiment notes</label><textarea id="record-note" rows="4" value={activeDraft} oninput={(event) => rememberNote(run, event.currentTarget.value)} disabled={Boolean(busy)} placeholder="Record actions, observations, anomalies, or conclusions…" onkeydown={(event) => { if ((event.ctrlKey || event.metaKey) && event.key === "Enter") { event.preventDefault(); void addNote(run); } }}></textarea>
                <div><span class="small muted">Markdown supported. Drafts are saved locally. Ctrl + Enter to save.</span><button class="primary" disabled={Boolean(busy) || !activeDraft.trim()}>{busy === "note" ? "Saving…" : "Save note"}</button></div>
              </form>
            {:else}<p class="muted small">This legacy record has no event log and can be read or exported. Create a new note to continue recording.</p>{/if}
            <h3>Notes</h3>
            {#each [...(run.notes ?? [])].reverse() as note, i (i)}<div class="note"><div class="small muted">{localDateTime(note.addedAt)}{note.step ? ` · step ${note.step}` : ""}{note.postRun ? " · added after the experiment" : ""}</div><div class="prose">{@html htmlOf(note.text)}</div></div>{/each}
            {#if !(run.notes ?? []).length}<p class="empty">No notes yet. A brief record of this test is enough.</p>{/if}
            {#if run.conclusion_note}<p class="conclusion">Conclusion: {text(run.conclusion_note)}</p>{/if}
            <h3>Logs and attachments <span class="muted small inline">{selectedFiles.length} files</span></h3>
            {#if selectedFiles.length}<FileList rows={selectedFiles} workspace={workingCopy} busy={Boolean(busy)} {timeBasis} showRecord={false} onSave={saveFile} onOpen={(row) => void openFile(row)} onRecord={showRecord} onTag={chooseTag} />{:else}<p class="empty">Add multiple logs, photos, or attachments, then give each file remarks and tags.</p>{/if}
            <details class="record-path"><summary>Record location and ID</summary><p class="mono">{text(run.run_id)}<br />{run.path}</p></details>
          {:else}<div class="empty-card"><h2>A lightweight record, no test plan required</h2><p>A title, a few notes, and log files are enough to retain the key details of an experiment.</p><button class="primary" onclick={newNote}>New test note</button></div>{/if}
        </section>
      </div>
    {/if}
    {#if mode === "logs" || mode === "search"}
      {#if mode === "logs"}<div class="import-row"><label for="import-record">Link to experiment (optional)</label><select id="import-record" bind:value={importRecord} disabled={Boolean(busy)}><option value="">Unlinked (save to logs/)</option>{#each allRuns.filter((run) => run.editable) as run (runKey(run))}<option value={runKey(run)}>{runTitle(run, manifest)} · {localDay(run.started)}</option>{/each}</select><span class="small muted">Add log files supports multiple selection and keeps the originals.</span></div>
      {:else}<h2 class="file-results">Matching files <span class="muted small inline">{filteredFiles.length}</span></h2>{/if}
      {#if manifest.workspace_error}<p class="err" role="alert">Could not read independent logs: {manifest.workspace_error}</p>{/if}
      {#if fromDate && toDate && fromDate > toDate}<p class="err">The start date cannot be later than the end date.</p>{/if}
      {#if filteredFiles.length}<FileList rows={filteredFiles.slice(0, fileLimit)} workspace={workingCopy} busy={Boolean(busy)} {timeBasis} onSave={saveFile} onOpen={(row) => void openFile(row)} onRecord={showRecord} onTag={chooseTag} />{#if filteredFiles.length > fileLimit}<button class="load-more" onclick={() => fileLimit += 100}>Show more ({filteredFiles.length - fileLimit} files remaining)</button>{/if}
      {:else}<p class="empty">No matching log files. {mode === "logs" ? "Add files, or place them in the working directory's logs/ folder and select Refresh." : "Try a file name, remark, or tag."}</p>{/if}
    {/if}
  {/if}
</article>

<NewNoteDialog open={newNoteOpen} workspace={workingCopy} operator={signedInOperator} {sites} onCreated={created} onCancel={() => newNoteOpen = false} />
<ConfirmModal open={Boolean(confirmation)} title={confirmation?.title ?? "Confirm"} message={confirmation?.message ?? ""} confirmLabel={confirmation?.confirmLabel ?? "Confirm"} onConfirm={() => settle(true)} onCancel={() => settle(false)} />
<DoubleConfirm open={Boolean(deleteRun)} message={deleteRun ? `Delete "${runTitle(deleteRun, manifest)}" and the logs, photos, and attachments stored inside this record?\nIndependent files in logs/ and the original imported files will not be affected.` : ""} onConfirm={() => void remove()} onCancel={() => deleteRun = null} />

<style>
  .records { padding: 20px 24px; }
  .toolbar { align-items: center; }
  .toolbar h1 { margin: 0; }
  .spacer { flex: 1; }
  .count { font-size: 12px; }
  .search-row, .log-filters, .import-row, .detail-actions, .title-form { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .search-row { margin-bottom: 14px; }
  .search-row input { flex: 1; min-width: 220px; }
  .feedback { font-size: 13px; white-space: pre-wrap; overflow-wrap: anywhere; padding: 8px 12px; border-left: 3px solid var(--accent); background: var(--panel); }
  .feedback.err { border-color: var(--critical); }
  .small { font-size: 12px; }
  .inline { display: inline; font-weight: 400; }
  .log-filters { margin-bottom: 12px; font-size: 13px; }
  .more-filters summary, .record-more summary, .record-path summary { cursor: pointer; font-size: 12px; color: var(--muted); }
  .more-filters > div { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px; }
  .more-filters input { margin-left: 4px; }
  .record-layout { display: grid; grid-template-columns: minmax(230px, 290px) minmax(0, 1fr); gap: 24px; }
  .record-list { padding: 0; background: transparent; border: 0; }
  .record-list > button { width: 100%; text-align: left; padding: 12px; margin-bottom: 8px; background: var(--panel); border-color: var(--line); }
  .record-list > button.current { border-color: var(--accent); background: var(--selection); }
  .record-list strong, .record-list span { display: block; }
  .record-list strong { font-size: 14px; overflow-wrap: anywhere; }
  .record-list .small { font-size: 11px; margin-top: 5px; }
  .preview { font-size: 12px; color: var(--ink-light); margin-top: 8px; overflow: hidden; display: -webkit-box !important; line-clamp: 2; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
  .record-detail { min-width: 0; }
  .title-row { display: flex; gap: 12px; align-items: baseline; }
  h2 { font-size: 19px; margin: 0 0 8px; overflow-wrap: anywhere; }
  h3 { font-size: 14px; margin: 22px 0 10px; }
  .record-meta { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; font-size: 12px; color: var(--muted); margin: 0 0 14px; }
  .detail-actions { margin-bottom: 16px; }
  .detail-actions button { font-size: 12px; }
  .record-more { margin-left: auto; }
  .record-more button { margin-top: 6px; }
  .note-form { padding: 14px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel); }
  .note-form label { display: block; margin-bottom: 6px; }
  .note-form textarea { width: 100%; resize: vertical; font-size: 14px; }
  .note-form > div { display: flex; gap: 8px; justify-content: space-between; align-items: center; flex-wrap: wrap; margin-top: 8px; }
  .note { padding: 12px 0; border-bottom: 1px solid var(--line); }
  .note :global(.prose > :first-child) { margin-top: 8px; }
  .note :global(.prose > :last-child) { margin-bottom: 0; }
  .empty-card { background: var(--panel); border: 1px dashed var(--line); border-radius: 10px; padding: 28px; color: var(--muted); }
  .empty-card h2 { font-size: 17px; color: var(--ink); }
  .empty-card p { font-size: 13px; }
  .import-row { margin: 14px 0; }
  .import-row select { max-width: 420px; }
  .load-more { margin-top: 12px; }
  .file-results { margin-top: 28px; }
  .title-form input { flex: 1; min-width: 180px; }
  .record-path { margin-top: 18px; }
  .record-path p { font-size: 11px; }
  .link { color: var(--accent); border: 0; background: none; padding: 0; text-align: left; overflow-wrap: anywhere; }
  .danger { color: var(--critical); }
  @media (max-width: 900px) { .records { padding: 14px; } .record-layout { grid-template-columns: 1fr; } .record-list { max-height: 280px; } }
</style>
