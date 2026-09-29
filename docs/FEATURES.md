# Expected Features

Priorities refer to the phases in `DESIGN.md` section 8.

Legend: **P0-a** spec, **P1** core parsing, **P2** core events, **P3** CLI, **P4** GUI,
**P5** knowledge loop, **no** deliberately excluded.

## A. Content authoring

| Feature | Notes | Priority |
|---|---|---|
| Procedure fragments with stable step ids | The atomic unit; referenced by checklists and cited by run records | done |
| Checklists assembled from inline include markers | Order-precise, renders invisibly on GitHub | done |
| Typed captures | `text`, `number`, `integer`, `bool`, `select`, `datetime`, `duration` | done |
| Optional `expected` on captures | Highlighted in the UI, acknowledged by the operator, never auto-judged | done |
| Step-level `attach` capture | A specific step can require its own log or photo | done |
| Explicit checkbox vs prose-list semantics | Removes the silent "mixed list degrades to prose" trap other tools have | done |
| Optional `traces` | Step or procedure traced to a datasheet section, requirement, or calibration record | P1 |
| Deprecating a step without breaking history | `deprecated: true`; ids are never reused or renumbered | P1 |
| Authoring templates | Starting points for procedure, checklist, run | done |
| The app edits one step at a time | A step list plus the selected step's fields, so the screen scales with the checklist | done |
| An edit is written to the file that defines the step | A step pulled in by an include is edited in its procedure, not in the checklist that includes it | done |
| Schema compatibility policy | Written down in `SPEC-COMPAT.md`, so format evolution is a decision and not an accident | done |
| One small Markdown renderer for prose, help, and notes | `h1`-`h4`, tables, lists (with task items), quotes, fenced code, links; one implementation (`sop_core::md`), used by the window through a command and by `sop preview` server-side (D22) | done |

## B. Execution on site

| Feature | Notes | Priority |
|---|---|---|
| Start a run from a checklist | Freezes `snapshot.md`, opens `events.jsonl` | done |
| Pick the instrument from a list | The `sensors` setting (`model: serial, serial; model`) fills the model and serial menus; a serial entered by hand is added to it, and a sensor the checklist declares is preselected | done |
| Equipment ticked off, not retyped | The checklist's `equipment:` front matter appears as checkboxes; only what the run used reaches the record | done |
| A run id that reads back its own run | `2026-09-28-uas-mag-1001` from the date, model and serial, with `-2` on a repeat; shown as a preview and editable by hand | done |
| Ordered step list with instructions inline | Prose, tables, warnings, and acceptance criteria next to the widgets | done |
| Interactive checkboxes | Every toggle is an event | done |
| Capture entry with unit and expected-range highlight | Operator acknowledges out-of-range values | done |
| Skips and deviations require a reason | Non-negotiable; this is the audit trail | done |
| Notes per step and per run | Markdown, timestamped; the editor previews and the run screen renders them (D21) | done |
| Attach a log to a step, or a set to the run | Copied into the run's `logs/` under its own name, hashed, and listed in the record | done |
| Block starting a `draft` checklist | Override is possible and recorded as an event with a reason | done |
| Fully offline | The app makes no network calls of its own; publishing hands the push to `git` (D17) | done |
| Checklist items shown as the author wrote them | The run screen ticks the words in the file, not `Item 1` | done |
| Keyboard-first operation | Field use is often one-handed and gloved; the run screen binds `space`, `s`, `d`, `n`, `j`, `k`, `enter`, and `ctrl+enter` (which asks to end the run and does not fire inside a field) | done |
| Ending a run is confirmed | `ctrl+enter` and the End buttons only arm a confirmation bar; an ended run cannot be reopened, and a mis-press must not end one | done |
| Reasons are collected in the app | Skip, deviate, acknowledge, and the empty-required-capture reminder use `PromptModal` with one-tap reasons, not the browser's blocking `prompt` | done |
| The window never blocks on a command | Every Tauri command runs on the blocking thread pool; `git` has a kill-deadline, and startup skips the pull when offline | done |
| Progress and navigation | A "N / M done" bar; marking done advances to the next open step; a step focuses its first empty capture; the complete-run blocker is named with a jump button | done |
| Unfinished runs are visible everywhere | The start form lists every in-progress run in the working copy, across checklists, and resuming one switches to its checklist | done |
| The start form asks only what is required | Run id, operator, and site are visible; the instrument, equipment, and conditions fold into a "Same as last run: …" line | done |
| Field-safe navigation | The window lands on Test Plans, orders the bar plan / run / history / browse, and keeps Edit, Project, and Settings behind a Manage dropdown | done |
| Expected range shown before entry | A capture with an `expected` value shows "expected -5 … +5 nT" while it is still empty | done |
| Numeric captures are typed | `number`/`integer` captures render a numeric input; a value that is not a number is called out, not silently accepted | done |
| Required captures are a reminder | Marking a step done counts the empty `required` captures, asks for a reason, and records it; the operator can always continue | done |
| The event log is append-only | `events.jsonl` is appended with `O_APPEND`, fsynced per event, and locked per run, so a crash or a second process cannot lose earlier events | done |
| Dates show in local time | History and Test Plans render the operator's local day/time (the run id uses the local date); the raw UTC stays in a tooltip | done |
| Resume after a crash | Replay the log; the run reopens in its last consistent state | done |
| Run a whole test plan | **Run plan** walks the plan's cases in `order`, carries the start configuration from one case to the next, records `plan`/`case`, and offers **Next case**; the plans view shows each case's last run and a **Run next** | done |
| The start form remembers the last run | Operator, site, sensor, and conditions are seeded from the previous run of the checklist; a value the operator typed is never overwritten | done |
| Typed fields for the conditions a checklist declares | `conditions:` (a key, a list of keys, or key to hint) becomes one field per key; the free-text box stays as the fallback | done |
| Site picked, not retyped | Candidates come from `project.md` `sites:` and from earlier runs | done |
| The `sensors` setting has a structured editor | Add and remove models and serials instead of editing `model: serial, serial; model` by hand | done |
| The start form refuses to start on a mistake | Run-id collision, missing fields, and a `draft` without an override are shown before **Start** | done |
| Notes are multi-line Markdown | A step note and a run note each have an editor with a live preview; saved notes render under the step (D21) | done |
| Reopen a mis-tapped outcome | Done/skip/deviate can be corrected with a compensating event; the log stays append-only and the correction is visible | done |

## C. Records and audit

| Feature | Notes | Priority |
|---|---|---|
| Append-only JSONL event log | Source of truth | done |
| Rendered `record.md` | Human review via GitHub diff | done |
| Template snapshot per run | Record survives repo moves and history rewrites | done |
| Which revision was executed | `sop_version` + `sop_commit` + `snapshot_sha256` | done |
| Immutable change history | Changing a value appends a typed event with a reason | done |
| SHA-256 on every attachment | Verified by the validator, not just recorded | done |
| Run metadata | Operator, site, sensor model/serial/firmware, hardware, and conditions entered at start; restated in the record body (D19) | done |
| Step-level coverage check | A `complete` run with an unfilled or skipped step is a validation error | done |
| Checkbox items rendered as the operator left them | The record shows `- [x]` and the item's text, not the template's `- [ ]` (D16) | done |
| A note survives a step left without an outcome | An unfinished step keeps its notes and attachments; only its result block is absent | done |
| Captures recorded on a step with no outcome reach the record | A `yaml result` block may omit `status`: the step has data but no outcome, and a `complete` run may not contain one (D19) | done |
| Where a run came from | `plan` and `case` on the record and on `RunStarted`, written when the run was started from a test case | done |
| An acknowledgement is part of the record | `acknowledged:` lists the captures the operator accepted despite an out-of-range expectation, and `CaptureAcknowledged` is the event behind it | done |
| A note is Markdown in the record too | A multi-line note keeps its block: every step-note line is prefixed, run-note continuation lines are indented (D21) | done |
| Drift between the snapshot and the working copy | The run screen asks `run_drift` and says when the checklist changed after the run started | done |
| A run in progress is a valid record | `status`/`ended` are written together by `sop run end`; an open run is not a validation error (D20) | done |

## D. Validation

| Feature | Notes | Priority |
|---|---|---|
| Front matter schema per file kind | Required keys, types, controlled vocabularies | done |
| Id well-formedness and uniqueness | Including after include resolution | done |
| Dangling include detection | | done |
| Attachment hash and size verification | | done |
| Deviation count consistency | Declared count must match the log and the record | done |
| Relative link resolution | | done |
| Formatting rules | UTF-8, LF, no BOM, single trailing newline, no tab indentation | done |
| Every `##` chapter carries a `yaml step` block | A chapter without one is prose, and the validator names it rather than reporting only `no steps defined` (D18) | done |
| An id written as a bare number is explained | `id: 7` is YAML for an integer, so the message says to quote it | done |
| Cross-check `record.md` against `events.jsonl` | Catches hand-edited records | done |
| Markdown structural lint | Malformed fences, headings inside steps, mixed-list traps | P1 |
| Validation in CI on every push and PR | Includes 22 negative tests that must fail | done |
| Validation available offline in the CLI and the app | So content can be checked at the desk, not only in CI | done |
| A record on disk mid-run validates | Only an ended record is judged on results, and `status` without `ended` is an error rather than silence (D20) | done |
| A checklist's `conditions:` is well-formed | A bad shape is a warning, because a run still works and an older reader ignores the key | done |

## E. Sync and operations

| Feature | Notes | Priority |
|---|---|---|
| Plain files in a git working copy | The app edits the working copy; git syncs | done |
| Atomic writes | Temp file plus rename; never a partially written file | done |
| No credentials in the app | No tokens, no API keys; the app stores no credential and runs `git` with `GIT_TERMINAL_PROMPT=0` (D17) | done |
| Publish the working copy from the app | **Push repo** runs `git add -A`, commits with the operator's message, and pushes `HEAD`; a commit that succeeds and a push that fails leaves the commit (D17) | done |
| Record and attachments committed together | The record is never separated from its data, and one button commits them together (D17) | done |
| Large-log guidance and optional Git LFS | Policy in `docs/LOGS.md` | done |
| Manifest generation for fast app loading | `dist/manifest.json` | done |
| Packaged `.deb` and AppImage | Installed deliberately per laptop, no silent updates | done |
| Header shows git and validation state | Dirty/ahead/behind, the validation counts, and a persistent last-saved time, so the risk is visible before **Export** / **Push** | done |

## F. Knowledge loop

| Feature | Notes | Priority |
|---|---|---|
| `runs/_inbox/` for unproven observations | Evidence lives separately from the conclusion drawn from it | done |
| Inbox entry schema and validation | Created, author, observed_in, target, confidence | done |
| "Promote to procedure" flow | Copies an observation into a proposed procedure edit | P5 |
| Deviation roll-up across runs | "What keeps going wrong, and where" | done |
| Guide for opening the resulting PR | Review is the point, not the automation | P5 |

## G. Analysis across runs

| Feature | Notes | Priority |
|---|---|---|
| Stable step ids comparable across runs | The reason ids are authored rather than execution-assigned | done |
| Run history by test plan and case | Grouped under the plan and case each run was started from; filter by site, operator, outcome; cases with no runs are shown | done |
| Summary across every test | One markdown document: a coverage row for every case (run or not, with outcome counts) and one row per run with the attached file names | done |
| Remove a run, or the whole history | **Delete** on a row and **Delete all runs** in the toolbar; the inbox is never touched | done |
| Capture trend across runs | e.g. noise floor over successive calibrations | P4 |
| Export run data as CSV | For the processing pipelines in `mag_gcs` and `geomag-uav-survey` | done |
| Export one run's record as a self-contained document | `sop run export --format markdown --run <id>`, and **Export record** in the app (D16) | done |
| Site and sensor comparison | The primary axis of a multi-site campaign | P5 |
| Case-level start from the plan table | Each case's last run (outcome, operator, date, steps) and a **Run next** for the first case that is not complete | done |

## H. In-app help

| Feature | Notes | Priority |
|---|---|---|
| Help panel on the right, toggled with `F1` | Answers "how do I drive this?" without leaving the run | done |
| Help content as a validated file kind | `help/*.md`; same review, same offline story as procedures | done |
| Sections and ordering declared in the content | A global `order`; adding a section needs no code change | done |
| Help published in the manifest | Panel needs one read and no extra file access | done |
| Contextual entry point | Open the panel at the page for the screen or message in front of you | P4 |
| Search across help pages | `/` opens the panel, then filters title, summary, section, body and tags; no index needed | done |
| Help pages double as the UX contract | A page describing a shortcut is the specification of it | done |
| Cross-links from a validation message | The message names the page that explains it | P5 |

## Explicitly excluded

| Not building | Why |
|---|---|
| Automatic pass/fail verdicts | `DESIGN.md` 6.4 - the criterion belongs to the author, the judgement to the operator |
| Real-time multi-user editing | One operator per run; parallel runs are fine |
| Photo/QR upload relay | Attachment volume does not justify the complexity |
| Database or server component | The filesystem is the storage engine |
| In-app GitHub API access | `git` handles sync; see `DECISIONS.md` D5 |
| Windows and macOS packaging | Field target is Ubuntu; add only if a real need appears |
