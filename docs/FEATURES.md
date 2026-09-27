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

## B. Execution on site

| Feature | Notes | Priority |
|---|---|---|
| Start a run from a checklist | Freezes `snapshot.md`, opens `events.jsonl` | done |
| Ordered step list with instructions inline | Prose, tables, warnings, and acceptance criteria next to the widgets | done |
| Interactive checkboxes | Every toggle is an event | done |
| Capture entry with unit and expected-range highlight | Operator acknowledges out-of-range values | done |
| Skips and deviations require a reason | Non-negotiable; this is the audit trail | done |
| Notes per step and per run | Free text, timestamped | done |
| Attach a log to a step, or a set to the run | Copied, hashed, filename prefixed with the hash | done |
| Block starting a `draft` checklist | Override is possible and recorded as an event with a reason | done |
| Fully offline | The app makes no network calls of its own; publishing hands the push to `git` (D17) | done |
| Checklist items shown as the author wrote them | The run screen ticks the words in the file, not `Item 1` | done |
| Keyboard-first operation | Field use is often one-handed and gloved; the run screen binds every key `help/app-basics.md` documents (`space`, `s`, `d`, `n`, `j`, `k`, `enter`, `ctrl+enter`) | done |
| Resume after a crash | Replay the log; the run reopens in its last consistent state | done |

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
| Run history per checklist | Filter by site, sensor, operator, outcome | done |
| Capture trend across runs | e.g. noise floor over successive calibrations | P4 |
| Export run data as CSV | For the processing pipelines in `mag_gcs` and `geomag-uav-survey` | done |
| Export one run's record as a self-contained document | `sop run export --format markdown --run <id>`, and **Export record** in the app (D16) | done |
| Site and sensor comparison | The primary axis of a multi-site campaign | P5 |

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
