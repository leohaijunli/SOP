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

Progress: **batches 1 and 2 are merged** - 0.2, 0.3, 0.4, 1.1-1.4, and 2.1-2.6 are done
(they carry a `done` mark below). 0.1 has no code to change but still needs a real Ubuntu
machine, so it stays open. Next is P3, starting with 3.1 (a run conclusion).

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
| 2.1 `done` | **In-app modals instead of `prompt`/`confirm`** | `PromptModal.svelte`; skip, deviate, acknowledge, and the required-capture reminder each offer one-tap reasons plus a text box. |
| 2.2 `done` | **Commands must not block the window** `[verify]` | Every command carries `#[tauri::command(async)]` (Tauri's blocking thread pool); git commands have a kill-deadline (15s local, 30s network); startup skips the pull when `navigator.onLine` is false. |
| 2.3 `done` | **Progress and navigation** | A "N / M done" bar; marking done advances to the next open step; opening a step focuses its first empty capture; the End:complete blocker is spelled out with a jump button. |
| 2.4 `done` | **A global list of unfinished runs** | The start form lists every in-progress run across checklists; resuming one on another checklist switches to it through the shell. |
| 2.5 `done` | **Collapse the start form** | Run id / operator / site stay visible; instrument, equipment, and conditions fold under "Same as last run: …" (re-seeded per checklist, opened when there is no history). |
| 2.6 `done` | **Reorder navigation** | Lands on Test Plans; the bar is Test Plans / Run / History / Browse; Edit / Project / Settings sit behind a "Manage" dropdown. |
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
