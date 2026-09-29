# Engineer Workflow Review and Improvement Plan

Status: review complete; Phases 1-3 implemented, plus a note-editor pass and a validator
fix (2026-09-28). Phase 4-5 pending; Phase 6-7 proposed.

Scope: how well the app serves a field engineer through the daily loop - pick a test case,
configure a run, execute it, and trust the record afterwards. This extends
`docs/IMPROVEMENTS.md`, which covers the execution screen and provenance in detail. This
document adds the test-case-start and configuration axes and orders the work.

## What the design already gets right

- Layering is clean: `sop-core` (pure) -> `sop-repo` (filesystem) -> `sop-cli` / `sop-app`.
  The event log, the frozen `snapshot.md`, and the rendered `record.md` are a coherent
  story.
- Composition through include markers, authored step ids, deviations that require a
  reason, hashed attachments, and a `testplan/` tree of test cases.
- History already groups runs by plan and case, matched on the case's `sop_id`
  (`ui/src/components/HistoryView.svelte`).

The friction is concentrated in three places: starting a case loses its *plan* context,
configuration is retyped every run, and the trust signals the model already stores never
reach the screen.

## Findings

### A. Test-case start

- A test case is loaded as an external file, not as repository content. `TestPlansView`
  Start calls `startCase()` (`ui/src/App.svelte`), which calls `loadExternalMd`
  (`crates/sop-app/src/commands.rs`). That path hardcodes `unresolved_includes: []` and
  records no per-file provenance, so an in-repo case runs like an ad-hoc "Open file"
  checklist.
- A plan's `order` is used only for sorting (`crates/sop-repo/src/testplan.rs`). There is
  no "run this plan in order"; engineers start cases one at a time and re-enter context.
- The suggested run id is `date + model + serial`, so two cases of one plan on the same
  instrument the same day differ only by `-2`. The run does not say which case it was.
- The plan table shows step counts, not coverage. There is no view of which cases have a
  run, its status, or its operator.
- Ending a run returns to the plan list; there is no "next case".

### B. Configuration

- Operator, site, sensor, hardware, and conditions are retyped every run
  (`ui/src/components/ExecutionView.svelte`); nothing remembers the last values, and site
  is free text, so typos fork grouping and analysis.
- `sensors` is an opaque string (`model: serial, serial; model`) edited as raw text in
  Settings. Serials are added implicitly on start but there is no add/remove editor.
- Settings persist only on Confirm, as free text, with no validation of paths or refs.
- Conditions are free-form `key: value` lines; checklists declare `equipment:` but not the
  conditions they expect, so the form cannot render typed fields.
- Start-form errors (run-id collision, draft override) surface only after clicking Start.

### C. Trust signals and provenance

- The header shows no git `dirty` / `ahead` / `behind`, no validation count, and no
  "last saved", although `Status.git` and `validation_report` already exist.
- An out-of-range `expected` value is labelled but never recorded as an acknowledgement.
- No compensating-event undo for gloved mis-taps.
- Attachments are one file per dialog, with no drag-and-drop and no size budget warning.

### D. Platform and robustness

- `prompt()` / `confirm()` drive skip/deviate reasons and plan/case management; blocking
  browser dialogs are poor for gloved, one-handed field use.
- Tauri commands are synchronous, so large attachment copies block the UI thread.

## Improvement plan

### Phase 1 - Make test-case start first-class

- Record `plan` and `case` on the run, so a run self-describes where it came from.
- Encode plan and case into the suggested run id.
- Add "Run plan": one shared configuration, then walk the cases in `order`, offering the
  next case at the end of each run.
- Turn the plan table into a coverage board: last run status, operator, date, step count,
  and a "Run next" action.

### Phase 2 - Configuration ergonomics

- Saved field profile: remember the last operator, site, sensor, and conditions; seed a
  new run from the previous one.
- Site picker seeded from `project.md` and prior runs; new sites added explicitly.
- Structured sensors editor (add/remove model and serials); validated settings.
- Let a checklist declare expected `conditions:` and render typed fields; keep free-form
  as fallback.
- Live start-form validation (run-id uniqueness, required fields, draft override).

### Phase 3 - Trust signals and provenance

- Header badges: validation count, git dirty/ahead/behind, last-saved time; warn on
  Export/Push while invalid.
- Record an acknowledgement event for out-of-range expected values.
- Per-file provenance for the checklist (blob hash / commit) and a drift indicator.
- Compensating-event undo for `done` and checkbox mis-taps.

### Phase 4 - Data capture and analysis

- Multi-file and folder attach with drag-and-drop and a per-run size budget warning.
- Typed capture inputs per `type`; declared `outputs:` with tick-off, so `complete` means
  *collected*.
- Compare runs of the same case side by side; "export all" across a plan.

### Phase 5 - Platform polish

- Native Tauri dialogs in place of `prompt` / `confirm`; move large file copies off the UI
  thread; tests around plan run and configuration persistence.

### Phase 6 - Delivery and collaboration (proposed)

- Export a run, a case, or a whole plan as a PDF or HTML report for a customer or a review.
- Run handover: a second operator continues an unfinished run, with the change of hands
  recorded rather than the `operator` silently overwritten.
- Read-only share view of a run or a plan for someone who does not have the app.
- Conflict handling when the same repository is pushed from both the field laptop and the
  office.

### Phase 7 - Packaging and operations (proposed)

- Signed installers and auto-update, installable without network access.
- Long work as a background task with progress: large attachment copies, manifest rebuild.
- Scheduled backup / auto-commit of the working copy.
- UI language switch (zh/en).
- CI: `cargo fmt --check` (as its own formatting commit), `clippy`, and front-end e2e
  (tauri-driver or Playwright) over the Run-plan -> note -> export path.

## If only three things land first

1. Run-a-plan with run -> case provenance (Phase 1).
2. Saved field profile plus structured sensors and site (Phase 2).
3. Git and validation in the header (Phase 3) - the one that prevents losing a survey the
   rest cannot recover.
