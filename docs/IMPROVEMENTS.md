# Improvements for field engineers

Status: review notes, not scheduled. Nothing here is decided; it is a list to argue with.

These are observations from walking the current build against how a run actually happens
in the field: one hand, gloves, a sensor that has to be watched, and a laptop that has no
network. They assume the existing design - an append-only event log as the source of
truth, a snapshot per run, validation in the domain, and no network calls from the app -
and try to spend it better rather than replace it.

Order is by expected value per unit of work, not by size.

## 1. The run screen is a cockpit, not a form

The execution view is where the tool is actually used, and today it asks the operator to
do work the app could do.

- **Advance to the next step on completion.** `setStatus` records the outcome and leaves
  the selection where it was; only `j` and `k` move (`ui/src/components/ExecutionView.svelte`).
  Selecting the next step *with no outcome* after a `done` saves one deliberate action per
  step - sixty of them on a long checklist - and removes the "which one was I on" pause.
- **Focus the first unset capture when a step opens**, and give numeric captures
  `inputmode="decimal"`. A numeric keypad instead of a full keyboard is most of the
  difference on a tablet.
- **Emit `StepOpened`.** The event exists (`crates/sop-core/src/run.rs`) but the UI never
  writes it. It costs one line and buys per-step dwell time, which answers "how long does
  this test really take" and makes a run's timeline readable in the log.
- **Collapse finished steps** or offer a "remaining only" filter. The step list plus the
  text filter is the right shape; it just is not oriented around what is left to do.

## 2. Show the trust signals already in the model

Everything needed for these is stored already; none of it reaches the screen.

- **Validation.** `validation_report` is implemented (`crates/sop-app/src/commands.rs`) and
  wrapped (`ui/src/lib/api.ts`) but no view calls it. A header badge ("3 problems") and a
  panel would move the `sop validate` check to where it can still change what happens, and
  Export/Push should warn while a run is invalid.
- **Git state.** `Status.git` already carries `dirty`, `ahead`, and `behind`, and the header
  shows none of it. "7 runs not pushed" is the difference between a survey and a lost
  survey when the laptop is the only copy.
- **Drift.** A run records `sop_commit` and `snapshot_sha256`. "Started against v3, the file
  is now v5" is display-only work, and it is what makes keeping the snapshot worth it.
- **Last write.** A persistent "saved 12:04:31" in place of a transient word, so an operator
  can tell at a glance that the last event landed.

## 3. Close the run -> deviation -> edit loop

The knowledge loop is the differentiator, and it currently depends on the operator
remembering to transcribe.

- A **"this step needs work"** action on the run screen that opens that exact step in the
  authoring view with the deviation reason pre-filled. Run and Edit are separate screens
  today, so the loop crosses a gap only a person can carry.
- Next to it, "3 deviations on this step across 12 runs" - the roll-up `sop run deviations`
  already computes.

## 4. Attachments should be a session, not sixty dialogs

- `pickDataFile` opens the picker with `multiple: false` (`ui/src/lib/api.ts`). Multi-select
  and drag-and-drop onto a step, attached as one batch, matches how a session's files
  come off a device.
- **Show the weight.** Attached data is committed to `git`; a 2 GB video fails the push
  minutes later, not now. A per-run total and a warning above a threshold turn that into
  a decision at the right time (`docs/LOGS.md` has the policy; the screen does not show it).
- **Expected outputs in the checklist.** Let a step declare `outputs: [mag_raw.csv]` and
  have the app tick them off. The validator can then flag a `complete` run whose declared
  data is missing, which is the single biggest "did we get everything" win available: it
  makes `complete` mean *collected*, not just *ticked*.
- **Import from device.** Choose a folder or SD card once and file the matching files into
  the current step, instead of one file per dialog.

## 5. Provenance a reviewer will actually read

- **Local time.** Timestamps are UTC with a `Z` (`crates/sop-core/src/timestamp.rs`), which
  is right for the record and awkward for the operator. Display local time with the offset,
  and store the run's UTC offset at start: two runs at "09:00" across a DST change are
  ambiguous to a reader today.
- **Record the acknowledgement.** "outside the expected range - acknowledge it" is a label
  with no recorded decision (`ExecutionView.svelte`). An acknowledgement event with a reason
  turns a red line into a judgement that survives review.
- **Undo as a compensating event.** Gloves and one hand produce mis-taps on `done` and on
  checkboxes. An append-only log cannot delete, but it can record "corrected to not done",
  which keeps the audit trail honest and the operator unblocked.
- **Snapshot the device configuration.** `sensor: model/serial/firmware` is entered once at
  start; the sensor's own config file could be attached automatically in the same step,
  the way the checklist snapshot is taken.

## 6. The analysis end of the loop

- **Compare runs of the same case.** Captures side by side by site and serial, with filters
  for sensor, serial, and operator. `sensors` just made serial a first-class key, so this is
  cheaper now than it was. `FEATURES.md` already lists it as P5 "site and sensor
  comparison".
- **Export all.** One CSV or workbook across runs, alongside the per-sop `run export` and
  the markdown summary.
- **Search the records.** Full-text across run records, not just the History table's filter.

## If three things happen first

1. **Advance and focus on completion.** Contained in `ExecutionView.svelte`, effects every
   step of every run.
2. **Git and validation in the header.** Contained in `Header.svelte` and the existing
   commands; prevents the loss the rest of this list cannot recover.
3. **`outputs:` in checklists with tick-off.** A format change, so it touches `SPEC.md`,
   the validator, and the record - but it is what makes a finished run trustworthy without
   re-reading it.
