# Design

Status: agreed direction, not yet implemented.

This supersedes the earlier `Chrome --app` and `Python + PyQt6` directions. See
`DECISIONS.md` D9 for why the change happened.

## 1. Goal

A field tool that (a) guides an operator through a procedure step by step, (b) produces
a record that is trustworthy years later, and (c) lets field experience flow back into
the procedures themselves.

The hard requirements, in order:

1. **No data loss.** A field session cannot be repeated. A crash must cost at most the
   current keystroke.
2. **Auditability.** For any record, answer: which revision of which procedure, run by
   whom, when, with what sensor at what site, and what was changed afterwards.
3. **Reuse.** The experiment matrix (experiment type x sensor x site x configuration)
   must not be satisfied by copying templates.
4. **Offline.** The core loop works with no network at all.

## 2. Non-goals

- Multi-user real-time collaboration. Parallel runs, yes; two writers on one run, no.
- A general-purpose procedure tool. `ut-issl/procnote` already exists and is better at
  being general. Our differentiators are composition, field metadata, and the knowledge
  loop - see section 10.
- Automatic pass/fail judgement. See `DECISIONS.md` D4 and section 6.4.
- A database. The filesystem is the storage engine.
- The app talking to GitHub. `git` does that, not us. See `DECISIONS.md` D5.

## 3. Stack

| Layer | Choice | Why |
|---|---|---|
| Domain logic | Rust | Single implementation of the format; no second parser to drift |
| CLI | Rust (`clap`) | Validation and rendering in CI without a GUI |
| Desktop shell | Tauri 2 | Native installers, small binaries, proven in this domain by procnote |
| Frontend | Svelte 5 + Vite + TypeScript | Chosen for lowest complexity and risk; see D12 |
| Tests | `cargo test` | Unit and golden-fixture tests |

### Why a web frontend inside a Rust app

The central UI task is: render CommonMark prose and place interactive widgets inside it,
in document order. Web engines do this natively. `egui`/`iced`/`slint` would require
writing and maintaining a Markdown-to-widgets layer, which is a larger and more permanent
cost than the frontend toolchain.

"Pure Rust" here means **no Python in the product**. It does not mean no TypeScript.

Svelte 5 with Vite, and specifically **not** SvelteKit: a Tauri application has a single
window and no server, so SSR, routing, and adapters would be pure overhead. Plain Svelte
plus Vite already emits the static bundle Tauri consumes, with no adapter to configure.
`DECISIONS.md` D12 records why Svelte rather than React.

### Known platform cost

Tauri on Linux links WebKitGTK (`webkit2gtk-4.1`), which must be present on every field
laptop and in the packaging environment. See section 7 for the consequences and the
current verification status.

## 4. Workspace layout

```
field-sop/
├── Cargo.toml                  # workspace
├── crates/
│   ├── sop-core/               # pure domain. No filesystem, no Tauri, no async.
│   ├── sop-repo/               # filesystem: discover, load, validate, write
│   ├── sop-cli/                # binary `sop`
│   └── sop-desktop/            # Tauri 2 shell + frontend
├── procedures/                 # fragment library
├── checklists/                 # assembled procedures
├── runs/                       # execution records
├── help/                       # in-app documentation, shown in the side panel
├── templates/
├── docs/
└── SPEC.md
```

Dependency direction is strict and enforced in review:

```
sop-desktop ─┐
             ├─→ sop-repo ─→ sop-core
sop-cli    ──┘
```

`sop-core` must not read a file, know a path, or import Tauri. This is what makes the
state machine testable without a filesystem or a display.

### `sop-core`

Pure functions over parsed documents and event logs:

| Module | Responsibility |
|---|---|
| `template` | Parse a procedure or checklist into a span-preserving AST |
| `resolve` | Expand include markers into an ordered step list |
| `event` | Event enum, JSONL encode/decode, schema versioning |
| `state` | Apply events to derive execution state; replay determinism |
| `render` | Render execution state back to Markdown (`record.md`) |
| `check` | Validation rules over parsed documents |

### `sop-repo`

Everything that touches the disk: discovering procedures and runs, loading content,
resolving include paths, verifying hashes, writing atomically (temp file + rename),
taking template snapshots, managing attachments, and shelling out to `git`.

### `sop-cli`

```
sop validate [PATH]        # same checks as CI, exit non-zero on error
sop index                  # write dist/manifest.json for the app
sop render <RUN_DIR>       # regenerate record.md from events.jsonl
sop status                 # overview: checklists, runs, pending inbox items
sop init <SOP_ID>          # start a run, snapshot the checklist
sop append <RUN_DIR>       # append one event (used by scripts and tests)
```

## 5. Storage layout

One run is one directory, self-contained:

```
runs/ground-walk-survey/2026-09-24-renfrew-walk01/
├── events.jsonl        # append-only source of truth
├── snapshot.md         # the checklist exactly as executed
├── record.md           # rendered from events.jsonl; committed for human review
└── attachments/        # log files, filename prefixed with a sha256 short form
```

`snapshot.md` is why the record survives without git history: the run stays readable even
if the repository is squashed, moved, or re-homed. `sop_commit` is retained but is now a
convenience, not a dependency.

`record.md` is a build artifact in spirit but is committed deliberately, because the
reviewable diff on GitHub is the whole point of the format.

## 6. Core design points

### 6.1 Span-preserving AST

The parser records a byte range for every block. Writing back replaces exactly one block
range and copies everything else verbatim. This is what makes the writer safe: hand-written
prose, comments, unusual formatting, and syntax the parser does not understand are never
touched.

This is the single hardest part of the implementation. It is protected by round-trip
golden tests: parse a file, write it back unchanged, assert byte equality.

### 6.2 Event sourcing

Operator actions are immutable typed events. State is never stored; it is derived by
replaying the log. There is no in-memory-only state, so a crash loses at most an event
that was never written.

```jsonl
{"type":"LogMeta","schema":1,"tool_version":"0.1.0","at":"2026-09-24T15:58:00Z"}
{"type":"RunStarted","at":"...","run_id":"...","sop":"ground-walk-survey","sop_version":1,"sop_commit":"8f3c1a2","snapshot_sha256":"...","operator":"leo","site":"Renfrew 395"}
{"type":"CheckboxToggled","at":"...","step":"hygiene-person","item":3,"checked":true}
{"type":"CaptureRecorded","at":"...","step":"cond-environment","key":"ambient_c","value":12,"unit":"C"}
{"type":"CaptureCleared","at":"...","step":"cond-environment","key":"ambient_c","reason":"probe replaced"}
{"type":"StepSkipped","at":"...","step":"testline-after","reason":"session ended early"}
{"type":"AttachmentAdded","at":"...","step":"testline-before","path":"attachments/a1b2c3d-mag_raw.csv","sha256":"...","size":18623}
{"type":"NoteAdded","at":"...","step":"testline-note","text":"..."}
{"type":"RunCompleted","at":"...","status":"partial"}
```

Two rules, both learned from procnote:

- Changing a recorded value is its own typed event (`CaptureCleared`) that carries a
  reason. The original event is never deleted, so "what was it before, who changed it,
  and why" is always answerable.
- Reversal is modelled as domain actions, not as log editing.

### 6.3 Compatibility policy

- No forward compatibility. A log written by a newer schema is rejected with a clear
  message rather than guessed at.
- Within a schema major version, only additive changes are allowed.
- Schema version lives in `LogMeta` and is checked on load.

### 6.4 Recording, not judging

Captures may declare an `expected` range or value. The UI **highlights** a value outside
it and requires the operator to acknowledge the field explicitly. The tool never marks a
step pass or fail, and never writes a verdict. The criterion belongs to the procedure
author; the judgement belongs to the operator on site.

### 6.5 Draft checklists

A checklist with `status: draft` cannot start a run without an explicit override that is
recorded as an event with a reason. Only `active` checklists are meant to produce real
data.

### 6.6 The frontend is a renderer, not a decision maker

`sop-core` sends the frontend a typed step view-model: an ordered tree of prose nodes,
checkbox items, capture fields, and warnings, already resolved against the run's
snapshot.

The frontend has no Markdown parser, no include resolution, no validation rules, and
generates no identifiers. It draws the tree and emits events back. That is its whole job.

This is what caps the risk of the framework choice. If the frontend also decided what a
step means, we would have two implementations of that question, diverging exactly as two
parsers would (see D9). Keeping the frontend thin also means it can be replaced without
touching a single rule, which is why D12 is deliberately a low-stakes decision.

### 6.7 The help panel

The app has a third column: a help panel on the right, toggled with `F1`. It answers
"how do I drive this?" and "what does that message mean?" without leaving the run.

The content is `help/*.md` - a fourth validated file kind, specified in `SPEC.md` section
12. It is Markdown in the repository for the same reason the procedures are:

- it can be extended without touching the application, which matters because the person
  who knows what needs explaining is the person running the experiments;
- it changes through the same review as everything else, so help that contradicts a
  procedure is a review comment and not a mystery;
- it works offline and in a text editor.

The manifest carries every page with its body, so the panel is built from the artifact
the app already loads and needs no second read. Panel order comes from a global `order`
integer: pages sort by it, and a section appears wherever its first page appears. The
frontend does not decide the order, only renders it, which is section 6.6 applied to
navigation.

Two properties are deliberate:

- **The help pages double as the UX contract.** A page describing a keyboard shortcut is
  a specification of that shortcut. Where the two disagree, one of them is a bug, and
  the disagreement is visible in the repository rather than in someone's memory.
- **Help is not versioned per page.** There is no `version` and no `applies_to`. A help
  page describes the system as it is now, and is edited in place. Freezing help would
  mean maintaining a history that nobody ever reconstructs.

Deliberately not built: a search index, a documentation generator, and any notion of help
being generated from code. Operators search with `/` over a few dozen pages; anything
more elaborate would need to be maintained, and the pages are the thing that has to stay
true.

## 7. Distribution

Linux only. The field target is Ubuntu laptops; Windows and macOS are out of scope until
a real need appears.

| Artifact | Purpose |
|---|---|
| `.deb` | Normal install. Brings the `.desktop` entry and icon, so menu launch and Dock pinning come for free. |
| AppImage | Single file, no install. For a spare laptop or a quick trial. |
| Plain binary (`--no-bundle`) | Development and CI smoke testing. |

Builds are produced by CI on tag and installed deliberately. There is no auto-update: a
field laptop must never change behaviour mid-campaign. The version is visible in the UI
and recorded in every run.

### Build on Ubuntu, not on the development machine

The development machine is Arch-based (Omarchy) with glibc 2.44. Ubuntu 24.04 has glibc
2.39 and 22.04 has 2.35, so a binary linked on the development machine will not start on
either. Release artifacts must therefore be built on a GitHub Actions Ubuntu runner, or
in a container of the target release. Local development and `cargo run` are unaffected.

Build against the **oldest** Ubuntu release that must be supported: a binary built on
22.04 also runs on 24.04, but not the reverse. Which release that is depends on the field
laptops and is not yet confirmed.

Verification status:

| Requirement | Status |
|---|---|
| `webkit2gtk-4.1` on the development machine | Present, 2.52.6, with `.so` and `.pc` |
| `gtk3` on the development machine | Present, 3.24.52 |
| Package manager on the development machine | `pacman` (Arch); no `dpkg` or `apt` |
| Rust toolchain | Present, 1.98.1, via `rustup` |
| `node` / `npm` | Present, 26.7.0 / 11.19.0, via `mise` |
| Tauri crates in the local registry | **Not cached** - the first build needs network |
| `webkit2gtk-4.1` on the field image | **Unverified** - check before the first deployment |

### WebKitGTK is a runtime dependency, not a bundled one

Tauri does not bundle a browser engine on Linux; it links the system WebKitGTK. The app is
therefore self-contained in its own code but not in its rendering engine, unlike Electron,
which ships an entire Chromium.

The trade is deliberate: installers measured in megabytes rather than hundreds of them,
at the cost of one declared system dependency. A `.deb` declares it, so a normal install
pulls it in. An AppImage cannot, so an AppImage on a machine without WebKitGTK simply
will not start. That is why the `.deb` is the primary artifact and the AppImage is a
convenience.

## 8. Phases

| Phase | Deliverable | Verified by |
|---|---|---|
| P0-a | Spec gaps closed: checkbox semantics, `expected`, step-level `attach`, compat policy | Python validator still 0 errors |
| P0-b | CI workflow, validator negative tests, golden fixtures | Deliberately broken content fails CI |
| P1 | `sop-core`: template AST + resolve + check, ported from the Python validator | `cargo test`, parity with Python on the existing content |
| P2 | `sop-core`: events, state machine, replay, render | Golden JSONL fixtures; replay determinism; MD round-trip |
| P3 | `sop-repo` + `sop-cli`: full CLI, atomic writes, snapshots, attachments | A run can be executed and recovered with no GUI |
| P4 | `sop-desktop`: Tauri shell, execution view, history view | Field trial on one campaign |
| P5 | Knowledge loop: inbox triage and promote-to-procedure flow | First procedure change originates from a real run |

P3 is the first genuinely useful milestone. The GUI is not on the critical path for
correctness, only for usability.

## 9. Migration from the Python tooling

`tools/soplib.py`, `tools/validate.py`, and `tools/build_index.py` are **transitional**.
They keep content validated until the Rust CLI reaches parity at P1/P3, and they are the
reference behaviour the Rust port is tested against. They are deleted once parity is
proven, so that only one implementation of the format exists.

## 10. What we deliberately do differently from procnote

| Axis | procnote | field-sop |
|---|---|---|
| Composition | Monolithic templates | Reusable procedures via inline include markers |
| Step identity | Assigned at execution start | Authored, permanent, comparable across runs |
| Domain metadata | `equipment` | site, sensor+serial, firmware, conditions |
| Knowledge loop | Notes only | `runs/_inbox/` -> review -> procedure PR |
| Content validation | Markdown lint | Full schema, hash, and coverage validation |

## 11. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Span-preserving round-trip is genuinely hard | Corrupted content on write | Round-trip golden tests; writer refuses to write if a range check fails |
| `serde_yaml` is archived upstream | Dependency rot | Use a maintained fork (`serde_yaml_ng`) or `saphyr`; pin the choice in `Cargo.toml` |
| WebKitGTK missing on field laptops | App will not start | Verify on the target Ubuntu image before P4 |
| Format churn after real content exists | Costly migrations | Spec gaps closed in P0-a, before the Rust core is written |
| Scope creep toward a general tool | Loses the differentiators | Section 2 non-goals; section 10 comparison |
