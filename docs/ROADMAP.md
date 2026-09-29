# Roadmap

Ordered by "what breaks if we skip it", not by effort:

1. **P0 - data safety.** Work that cannot be recovered afterwards.
2. **P1 - capture correctness.** Small changes with the highest value per line.
3. **P2 - usability in the field.** Gloves, one hand, bad light, no network.
4. **P3 - tracking value.** Turns the record store into a tool for tracking a campaign.
5. **P4 - structure and engineering health.** Interleaved with the rest.

`[verify]` marks something inferred by reading the code. It needs a real Ubuntu machine
before it is believed. This document merges the architecture review
(`docs/REVIEW-engineer-workflow.md`, phases 1-3 done) with the operations review, and
supersedes both as the list of what is left.

Invariants: no database, no change of UI framework, and Markdown plus event sourcing stays.

Progress: **batch 1 is merged** - 0.2, 0.3, 0.4, and 1.1-1.4 are done (they carry a
`done` mark below). 0.1 found no code to change but still needs a real Ubuntu machine, so
it stays open. Batch 2 (P2) is next.

## P0 - Data safety

| # | Item | Where | Acceptance |
|---|---|---|---|
| 0.1 | **Falsify three assumptions on a real machine** `[verify]`: `prompt`/`confirm` inside the `.deb`; whether startup freezes without a network; whether a run recovers after `kill -9` and after power loss | no code; write the findings into `docs/` | all three answered |
| 0.2 `done` | **The event log is really appended**: `O_APPEND` plus `sync_data` per event, no whole-file rewrite; a per-run file lock so the app and the CLI cannot drop each other's events | `crates/sop-repo/src/run.rs` | replay of a log truncated at any byte yields a valid prefix; two processes writing concurrently lose nothing |
| 0.3 `done` | **Atomic write also fsyncs** (the file and its directory), and `*.tmp` is ignored | `crates/sop-repo/src/atomic.rs`, `.gitignore` | survives a power cut |
| 0.4 `done` | **Ending a run asks first**: today Ctrl+Enter ends the run even from inside a field - including the note box, where it is advertised as "save note" - and an ended run cannot be undone | `ui/src/components/ExecutionView.svelte` | a mis-press cannot end a run |

0.2 and 0.3 are one batch of work. What 0.1 finds sets the urgency of 2.1 and 2.2.

## P1 - Capture correctness

| # | Item | Notes |
|---|---|---|
| 1.1 `done` | **Show the expected range before entry** | The field now reads "expected -5 … +5 nT" while it is empty (`ui/src/lib/capture.ts`). |
| 1.2 `done` | **Type checking for numeric captures** | `number`/`integer` use a numeric input with `inputmode="decimal"`; `expectedOk` no longer accepts `NaN`, and a bad value shows a format warning. |
| 1.3 `done` | **Required-capture reminder** | Marking done counts the empty required captures, asks for a reason, and writes it with the status. A completeness prompt, not a verdict (D4). |
| 1.4 `done` | **Local dates** | History and Test Plans render the operator's local date/time (`ui/src/lib/dates.ts`), with the raw UTC in a tooltip. |

1.1-1.3 touch only `ui/src/components/ExecutionView.svelte`, change no format, and ship
together.

## P2 - Usability in the field

| # | Item | Notes |
|---|---|---|
| 2.1 | **In-app modals instead of `prompt`/`confirm`** | Skip, deviate, and acknowledging an out-of-range value use blocking browser dialogs; offer shortcut buttons for the common reasons. |
| 2.2 | **Commands must not block the window** `[verify]` | Every Tauri command is a synchronous `fn`, and startup runs `git pull` unconditionally with no timeout. Make them `async` with `spawn_blocking`, give git a timeout, and skip the sync when there is no network. |
| 2.3 | **Progress and navigation** | "12 of 38 done"; advance to the next step after marking done; focus the first empty capture when a step opens; when **End: complete** is disabled, say which steps block it and offer to jump there. |
| 2.4 | **A global list of unfinished runs** | Today only the newest run of the open checklist is offered for resume; runs of other checklists, and older open runs, are invisible. |
| 2.5 | **Collapse the start form** | Only run id, operator, and site are required; the rest folds into a "same as last time: …" line. |
| 2.6 | **Reorder navigation** | Land on Test Plans; order plan / run / history; move Edit, Project, and Settings under "Manage" so a field tap cannot change the SOP. |
| 2.7 | **Multi-file attachments and a size warning** | One file per dialog today; add multi-select, drag and drop, and a total-size warning per run. |
| 2.8 | **Emit `StepOpened`** | The event type exists and the window never writes it. One line buys per-step timing. |

## P3 - Tracking value

| # | Item | Notes |
|---|---|---|
| 3.1 | **A run conclusion** | At the end of a run the operator records pass / fail / inconclusive plus one sentence. `complete` means executed, not passed; the judgement stays human (D4). |
| 3.2 | **Coverage matrix** | Test Plans shows one "latest run" per case; it should be case × sensor serial (× site) with the latest conclusion in each cell. |
| 3.3 | **History filters and comparison** | Filter by sensor/serial, date range, has deviations, has acknowledgements; compare runs of one case side by side; capture trends over time. |
| 3.4 | **Deviation loop** | On a step: "deviated in M of the last N runs", with a jump into Edit and a prefilled reason. |
| 3.5 | **`outputs:` declaration** | A checklist declares the files it should produce, so `complete` means the data was collected. Touches `SPEC.md`, the validator, and the record. |

3.1 and 3.5 change the record format and must follow the additive-only rule in
`SPEC-COMPAT.md`. They land after P1 and P2 settle.

## P4 - Structure and engineering health

| # | Item | Notes |
|---|---|---|
| 4.1 | **Move business rules down** | Range checks, completeness, run-id generation, and `sensors`/`conditions` parsing live in the window and are partly duplicated in the core (against "the window is only a renderer"); event timestamps should be stamped by Rust. |
| 4.2 | **Load plan cases through the repository loader** | `load_external_md` keys on an absolute path, has no file-level provenance, and hardcodes `unresolved_includes: []`. |
| 4.3 | **Render `record.md` less often** | Every event re-renders the whole document today; the log is the truth, so render at the end or when idle. |
| 4.4 | **Align the docs** | `DESIGN.md` still says "not yet implemented" and names old crates and commands; D10 and `DESIGN.md` disagree about `attachments/` versus `logs/`; the two review documents merge into this one. |
| 4.5 | **Front-end tests** | There are none. Start with vitest over the pure functions (range checks, run id), then one smoke test: pick a plan, capture, export. |

## Pace

1. **Batch 1:** 0.1, 0.4, 1.1-1.3, and 0.2 + 0.3 started.
2. **Batch 2:** 2.1-2.4, then 1.4, 2.5, 2.6.
3. **Batch 3:** 3.1-3.3, alongside 4.1.
4. **Batch 4:** 3.4, 3.5, 4.2-4.5.
