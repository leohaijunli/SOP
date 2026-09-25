# Design Decisions

Short records of decisions that are expensive to revisit. Numbered, newest last.

## D1 - Content is a standalone artifact; the app now lives in this repo

Decision: the content stays plain Markdown that anything can consume - `git`, a text
editor, a code review, a printed PDF, or a viewer we have not written yet. The
application lives in this same repository, under `crates/`.

Rationale: the original version of this decision kept the app out of the repository
entirely, on the grounds that coupling content to one binary makes the content less
useful. That reasoning still holds and is still enforced - nothing in `procedures/`,
`checklists/`, or `runs/` may require the app to be readable or writable.

What changed is the cost of the alternative. Splitting the app into a second repository
would mean two implementations of the format (one for the app, one for CI validation) or
a published parser crate that the content repo depends on. Both reintroduce exactly the
divergence the format is meant to prevent. Co-location keeps one implementation, and the
content remains fully usable without ever running it.

Amended 2026-09-24. See D9.

## D2 - Markdown with YAML front matter, not JSON

Decision: procedures, checklists, and run records are Markdown. Structured fields go
in YAML front matter or in fenced `yaml step` / `yaml result` blocks.

Rationale: an earlier draft used JSON for SOP definitions. JSON cannot carry the
instructions that make a checklist a guide rather than a form, and it diffs badly in
review. Markdown keeps prose, tables, images, and datasheet links usable next to the
machine-readable fields.

Consequence: the app must handle round-tripping Markdown, which is harder than
parsing JSON. That cost is contained by section 10 of `SPEC.md`.

## D3 - Modular procedures instead of one checklist per experiment

Decision: reusable steps live in `procedures/` and are pulled into a checklist by
inline `<!-- include: ... -->` markers, which fix their position in the step order.

Rationale: the experiment matrix is experiment type x sensor x site x
software/hardware configuration. Copying full checklists per combination produces
documents that diverge, and a fix applied to one combination silently misses the
others.

## D4 - The app records, it does not judge

Decision: no automatic pass/fail evaluation. Numeric captures are stored with their
unit; the acceptance criterion is stated in prose.

Rationale: automatic evaluation implies the tooling owns the threshold, which pushes
experiment-specific judgement into the format. Recording the raw value keeps the data
useful when criteria change, and avoids a checklist silently "passing" against a
stale threshold.

## D5 - Write mode A: the app edits a local git working copy

Decision: the app reads and writes a local working copy of this repo via the File
System Access API. Synchronisation is `git`'s job, not the app's.

Rationale: keeps a personal access token out of browser storage, works offline
without a queue-and-retry layer, and lets `git` handle conflicts. Committing through
the GitHub Contents API remains possible later, but is not the primary path.

## D6 - Logs are attached manually, and travel with the record

Decision: the operator selects log files explicitly at the end of a run. The app
copies them under `logs/<run_id>/`, records `sha256` and size, and commits them
together with the run record.

Rationale: automatic log discovery would couple the app to each instrument's file
layout and fail unpredictably in the field. Manual selection is one action, and
hashing at attach time makes the provenance claim verifiable later.

## D7 - English content and UI

Decision: procedures, checklists, records, and the app UI are English.

Rationale: consistent with the product language already used in `geomag-uav-survey`
and `mag_gcs`. Planning documents may remain Chinese.

## D8 - One operator per run, many runs in parallel

Decision: a team may run several experiments at once, but exactly one person drives
the app for a given run, and run files are one-per-run.

Rationale: removes the need for concurrent-edit merging in the format. Conflicts can
only occur at the git level between different run files, which git handles naturally.

## D9 - Pure Rust application, Tauri 2 shell

Decision: the application is Rust, with `sop-core` as a pure domain crate and a Tauri 2
desktop shell. No Python in the product. The TypeScript frontend is confined to the
shell.

Rationale: the deciding factor was implementation count. A Python core with a Rust shell
would mean two parsers for one format, which drift silently and surface as "CI passes but
the app reads it differently". One language, one parser, one set of tests.

Tauri over a pure-Rust GUI toolkit (`egui`, `iced`, `slint`) is a separate decision: the
central UI task is rendering CommonMark with interactive widgets inline and in document
order. Web engines do this natively; immediate-mode Rust toolkits would require writing
and maintaining a Markdown-to-widgets layer, a larger and more permanent cost than the
frontend toolchain.

Supersedes the earlier `Chrome --app` direction from the handoff document, and the
`Python + PyQt6` proposal considered on 2026-09-24.

## D10 - One run is one self-contained directory

Decision: `runs/<sop_id>/<run_id>/` contains `events.jsonl` (source of truth),
`snapshot.md`, `record.md`, and `attachments/`. The separate `logs/` tree is folded into
the run directory.

Rationale: a run can then be archived, moved, size-capped with Git LFS, or deleted as a
unit. The template snapshot makes the record readable without the repository history,
which matters because history can be squashed or the repository re-homed.

`record.md` is rendered from the event log but is committed deliberately: a reviewable
diff on GitHub is the point of using Markdown at all.

## D11 - The Python tooling is transitional and will be deleted

Decision: `tools/soplib.py`, `tools/validate.py`, and `tools/build_index.py` remain until
the Rust CLI reaches parity, and are then removed.

Rationale: their value is to keep content validated in the interim and to serve as
executable reference behaviour for the Rust port. Their cost is a second implementation
of the format, which is precisely what D9 rejects. Keeping them permanently would
contradict D9 for no benefit.

Deletion is gated on parity: the Rust validator must pass on the existing content and on
the negative test cases before the Python one is removed.

## D12 - Svelte 5 + Vite for the frontend, not SvelteKit, not React

Decision: plain Svelte 5 with Vite and TypeScript.

Chosen purely on implementation complexity and risk, with no weight given to consistency
with any other project in the lab.

Reasons:

- This application consumes no third-party component ecosystem. Every widget is ours and
  the document tree arrives from Rust, so there are no virtualised grids, maps, or heavy
  component kits to pick from. React's principal advantage - the breadth of its
  ecosystem - is therefore worth close to nothing here.
- Svelte removes the specific complexity that produces subtle bugs in stateful forms: no
  dependency arrays, no stale closures in effects, no memoisation, no immutable update
  ceremony, and two-way binding for inputs.
- Svelte 5 has been stable since 2024-10-19; at the time of writing it is at 5.57.1 with
  342 stable 5.x releases. Maturity is not a concern.
- `ut-issl/procnote` is a working application with nearly identical UI requirements on
  this same stack.

Not SvelteKit: a Tauri application has one window and no server. SSR, routing, and
adapters are overhead with no corresponding benefit, and plain Svelte plus Vite already
emits the static bundle Tauri consumes.

Dismissal condition: if the person who maintains this frontend has only React
experience, this decision should be revisited and probably reversed. Familiarity would
outweigh the code-volume difference.

This is deliberately a low-stakes decision. Section 6.6 of `DESIGN.md` keeps the
frontend a pure renderer, so replacing the framework later does not touch any rule.

## D13 - Help content is a validated file kind in the repository, not application code

Decision: the in-app help the operator reads while working lives in `help/*.md`, as a
fourth content kind with its own front matter and validation, and is published in
`dist/manifest.json`.

Reasons:

- The people who know what needs explaining are the people running the experiments. If
  help is application code, they need a developer to change a sentence. If it is
  Markdown, they need a text editor.
- Help that contradicts a procedure is a real failure mode, and a contradiction that
  appears in a diff is a review comment. A contradiction buried in a UI string is a
  mystery.
- It works offline, in a text editor, and needs no build step - the same properties the
  rest of the content has.
- Adding a section must not require a code change, so `section` is free text and panel
  order comes from a global `order` integer. The frontend renders the order; it does not
  compute it (`DESIGN.md` section 6.6).

Cost: a fourth file kind to keep validated, and a second place documentation can live
alongside `docs/`. The split is deliberate and worth stating: `docs/` is written for
maintainers of the tool, `help/` for people using it. `docs/` is not shipped to the app.

Dismissal condition: if the help pages and `docs/` start duplicating each other, that is
the signal that the split is wrong - merge them into `help/` rather than maintaining both.

Consequence accepted: the help pages describing the app are written before the app
exists, so they are a specification until then. Where a page and the implementation
disagree, one of them is a bug; neither is authoritative by default.

## D14 - The project's name is content, and the app's settings never hold a URL

Decision: the campaign's name, id, institution, and lead live in `project.md` at the
repository root, validated like any other file kind. The application's own settings file
holds the path to the working copy, the *name* of the git remote, and window state, and
nothing else. The remote URL is left where `git` put it, in the repository's
configuration.

Reasons:

- A name that lives on a laptop disagrees with a name that lives on another laptop, and
  both look right. The disagreement is only visible if somebody compares two records two
  years later, which is exactly when nobody can reconstruct which was intended. In the
  repository, the disagreement is a merge conflict, and a merge conflict gets resolved.
- The same reasoning applies to the remote URL for a sharper reason: a settings file that
  can hold a URL is a settings file somebody will paste a token into. An application that
  cannot store a credential cannot leak one, and a repository copied to a colleague, a
  USB stick, or a backup carries no secret with it. `sop settings set remote
  https://...` is refused by name, and `sop remote <url>` writes to `.git/config` instead.
- Records are read long after they are written. A run record that names its campaign by a
  value in a shared, reviewed, versioned file stays interpretable; one that names it by
  whatever a laptop's preferences said at the time does not.

Cost: two places to look when something about the project is wrong - the repository for
content, the settings file for behaviour - and a command (`sop project`) that exists only
because a text editor was not the answer at that moment. The split is worth it because the
two have different review paths: content is reviewed by the people who run the experiments,
settings are personal and reviewed by nobody.

Two consequences accepted:

- `project.md` is a file kind, so the validator owns it (`SPEC.md` section 14). A field
  that must be right for the record to be interpretable - `project_id`, `title`,
  `updated` - is required rather than optional.
- `sop project set` refuses to write a file that would not validate, rather than writing
  it and reporting the problem afterwards. The command either leaves a valid file or
  leaves the file alone; it can not leave a broken one. Editing by hand is still allowed
  and is still checked by `sop validate`, because a rule that only holds for the tool is
  a rule that does not hold.

Dismissal condition: if operators are forever editing `project.md` by hand because the
command does not cover a field they need, the answer is to add the field to `FIELDS`, not
to move the file into settings.
