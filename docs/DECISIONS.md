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

Executed 2026-09-26: `sop validate` reports 0 errors and 0 warnings on the content, the
Rust negative suite covers the cases `tools/selftest.py` did, and the files are deleted.

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

## D15 - A run record is read against its own revision, never the current checklist

Decision: the step ids a run record cites are checked against the `snapshot.md` frozen
when the run started, not against the checklist as it stands now. A record that has no
snapshot - which is only true of records written before runs became self-contained
directories - is checked against the current checklist and any difference is a warning,
not an error.

Reasons:

- `SPEC.md` section 8 already says a run record must never be interpreted against a
  different revision of its checklist. The validator was resolving the working copy
  instead, so the rule held on paper and not in the tool.
- The consequence was not subtle. Removing an include marker orphaned every record that
  had cited a step from that procedure, the edit was undone, and the operator was told
  their change "broke something else in the repository". The one action the editor exists
  for - changing an SOP in the light of a run - was the one it refused.
- The alternative, keeping step ids permanently undeletable, was rejected in
  `SPEC.md` section 4 anyway: it cannot be enforced for a step whose whole procedure is
  removed from a checklist, and a rule that cannot be enforced is a rule that gets
  worked around.
- History is evidence. A record may be out of date with respect to the current SOP and
  still be exactly right about what happened, so drift is worth reporting and is not a
  reason to reject a repository or to refuse an edit.

Cost: two severities for one finding, distinguished by whether the record has a snapshot,
and a stale record can persist without anyone being forced to migrate it. That is
accepted: the warning is visible in every `sop validate`, and forcing a migration would
mean inventing the revision a legacy record was written against.

Dismissal condition: if records without snapshots become common rather than a one-off
migration artifact, migrate them and delete the advisory path, rather than letting two
severities for the same rule become permanent.

## D16 - The record is the exportable document, and deleting one is a dry run by default

Decision: `sop run export <sop> --format markdown --run <id>` prints the same document
`sop run end` commits - front matter, then one section per step holding its instructions,
its checkbox items as the operator left them, and what was recorded against it. Deleting
a run removes only the three paths a run can occupy (the record, the run directory, the
log directory), and without `--yes` it only lists them.

Reasons:

- The record has to be readable on its own. A session is reviewed months later, often by
  somebody who does not have the repository checked out, and a document that cites step
  ids the reader cannot resolve is not an experiment record.
- Rendering the checklist items was a real defect, not a nicety: the record printed the
  template's `- [ ]` for every item, so a run where everything was ticked was
  indistinguishable from one where nothing was. A record that does not say what was
  checked is not evidence of anything.
- Deleting is the one operation the "a record is evidence, never edited" rule cannot
  cover by refusing, because the first thing anybody records is a typo. Making it narrow
  and two-step keeps the rule's intent - a record never changes - while leaving a way to
  remove a run that should never have existed.
- `--out` and stdout both exist because the two uses differ: a pipeline wants the bytes,
  a person wants a file. The summary goes to standard error so the data stream stays pure.

Cost: a fourth export path to keep working, and a delete command that a careful operator
must confirm with `--yes`. Accepted, because the alternative - no export and no delete,
with `rm` and hand-copied Markdown as the workaround - is exactly what was happening.

Dismissal condition: if exporting to a file is what people actually do and printing to
standard output is never used, drop stdout and require `--out`.

## D17 - The app may publish, but `git` does the publishing

Decision: the History screen carries two actions. **Export record** writes one run's
record to `exports/<sop_id>-<run_id>.md` inside the working copy. **Push repo** runs
`git add -A`, commits with a message the operator supplies (blank takes a generated
default), and pushes `HEAD` to the configured remote. The app holds no credential, and
every `git` invocation it makes for this runs with `GIT_TERMINAL_PROMPT=0`.

Reasons:

- Until now the last step of every session was a hand-typed `git add` / `git commit` /
  `git push`, and the failure mode of forgetting it is silent: the run looks complete on
  the laptop and does not exist anywhere else. `SPEC.md` section 9 step 4 already says
  the app commits the record and the logs together, so the app was always meant to do
  this; only the button was missing.
- The push names `HEAD` rather than a configured upstream, so a repository that has never
  been pushed works, and it cannot accidentally push the wrong branch.
- `GIT_TERMINAL_PROMPT=0` is what makes "the app holds no credential" enforceable rather
  than a promise. Without a terminal, `git` cannot prompt, so a missing credential becomes
  an error the window can show instead of a process hanging with no window to type into.
  D14's reasoning applies unchanged: an application that cannot store a credential cannot
  leak one.
- A commit that succeeds and a push that fails leaves the commit in place. That is the
  right way round - the work is local, recoverable, and can be pushed again - and the
  report the window shows says which half happened.
- `exports/` is ignored by `git` because the record is already committed under `runs/`.
  An export is a copy for handing to somebody, and committing a second copy of a record
  would create two documents that can disagree.
- Exporting falls back to the record file when a run has no directory of its own, which
  `SPEC.md` section 8 says to expect from records written before runs were self-contained.
  Refusing to export exactly the records that are hardest to re-derive would invert the
  purpose of the command.

Cost: the app now reaches the network, through `git`, on one button. That is a real
change in what the binary does - it was previously readable as never touching the
network - so the module documentation, this record, and `help/commands.md` all say it.
Accepted, because the alternative is the silent failure of an un-pushed session.

## D18 - A chapter is a step only with its block, and the validator says so

Decision: a `##` chapter becomes a step only when a `yaml step` block gives it an `id`.
A chapter without one is prose, and `sop validate` names the chapter and its line rather
than reporting only that the file defines no steps. `SPEC.md` sections 6 and 13 now say
this outright; section 6 previously called the block "optional metadata", which read as
though a heading alone were enough.

Reasons:

- Deriving an `id` from the heading was the alternative, and it is ruled out by section 4:
  a step id is part of the data contract and a heading is display text. Renaming a
  chapter would silently change the id every later run record cites, which is precisely
  the failure the permanence rule exists to prevent.
- The cost of the block is three lines. The cost of the confusion was real: a document of
  chapters and checklists written without blocks validated as "no steps defined", with no
  hint about which chapter the tool had ignored or what to add.
- Naming the chapter turns a rule into an instruction. The message says what is missing
  and where, which is the difference between a validator that blocks and a validator that
  teaches.

Cost: one more error a hand-written procedure can trip, and an author who wants a `##`
section of pure prose inside a checklist has to accept it being reported as an error
rather than being inferred. The current content has no such section; if one is ever
wanted, the escape is a step whose `kind` is `note`.

## D19 - Instrument, conditions, and a result with no outcome

Decision: a run record gains three optional front-matter blocks - `sensor` (a `model` /
`serial` / `firmware` mapping), `hardware` (a list), and `conditions` (a free-form
mapping). The operator enters them in the app's start panel, they are optional fields on
the `RunStarted` event, and the recording reads them back from the event log. The record
restates them in its body as well as its front matter. In a `yaml result` block, `status`
may now be omitted: the step happened and its data is recorded, but the operator gave no
outcome for it. A `complete` run may not contain such a result.

Reasons:

- The record exists to answer "what happened", and the checklist cannot answer which
  instrument, which firmware, or what the weather was. Without those, two runs at the same
  site are indistinguishable when they are read a year later, which is the comparison the
  whole repository is for. Section 8 already showed `sensor`, `hardware`, and `conditions`
  in its example; nothing wrote them, so the example was documenting a wish.
- Restating them in the body is deliberate duplication. Front matter is machine-readable
  and easy to scroll past; the exported record is what a person reads, and it should carry
  the facts they need without a YAML reader.
- The fields are additive to the event log: they default when absent, so a log written
  before today replays unchanged, and a log written today still replays in a build that
  does not know the fields. That is why this does not bump the schema (`SPEC-COMPAT.md`).
- Omitting `status` is the honest encoding of a state the format had no way to express.
  One option was to refuse to end a run that had captures on an open step; that would have
  made the operator invent a `done` they do not mean, which is worse than silence. The
  other was to accept `open` as a status value, which would have put four values in a
  three-value vocabulary. An absent key costs one line of prose and is unambiguous.
- Forbidding the absent status on a `complete` run keeps the two levels of the format
  consistent: `complete` already means every step has a result, and a result with no
  outcome is not a result. The check is `citation.report`, so a record with no snapshot
  warns rather than fails, like the coverage rule it sits beside.

Cost: three more optional keys and one new invariant to remember, and a record can now be
"valid but unanswered", which a reader has to notice. Accepted, because the alternative is
silently dropping the measurements an operator did record - the failure this change fixes.

## D20 - A run record on disk is valid while the run is still open

Decision: `status` and `ended` are written together by `sop run end`, and both are absent
while the run is in progress. `run_front_matter` requires `status` only when `ended` is
present, and reports a record that carries one of the two without the other. The
"run record has no step results" warning applies only to a run that has ended.

Reasons:

- The record file is written when the run starts, because it is what crash recovery
  replays (`SPEC.md` §8). A rule that made the file invalid the moment it was created
  turned every live run into a repository-wide validation error: the app's header showed
  `1 ERROR(S)` from the first second of every run, which trains the operator to ignore
  the badge that exists to be trusted (review Phase 3).
- Requiring the pair, rather than dropping the rule, keeps the finished-record contract
  unchanged: a finished run still declares one of `complete` / `partial` / `aborted`, and
  a hand-edited record with a status but no end time is still reported.
- Nothing else keys off the absence. `complete_run_coverage` and its neighbours already
  guard on `status == complete`, so an unfinished record simply has no complete-only
  rules applied to it.

Cost: the validator now accepts a record that is genuinely unfinished, so "is this run
still open?" is answered by `ended` and not by the file's existence. The app already reads
it that way through `run_state` / `started`. The alternative - writing the record only at
the end - would give up the recovery story that the log exists for.

## D21 - A note is Markdown, and it stays inside its own block in the record

Decision: notes are stored verbatim, may contain newlines, and are rendered with the same
small renderer as prose. A step note is written as a blockquote with every line prefixed
(`> note: first`, then `> second`); a run note is a list item whose continuation lines are
indented. The editor is a multi-line textarea with a live preview, where `Enter` inserts a
newline and `Ctrl/Cmd+Enter` saves. The renderer accepts headings `#` through `####`.

Reasons:

- A field note is where the interesting information lands: a few readings, a list of what
  was changed, a link. One line pushed that detail into the prose or out of the record.
- The record has to stay valid Markdown for a reader who never opens the app (D1), so the
  renderer cannot be what holds a multi-line note together. The writer prefixes each line,
  which is why `a_note_between_steps_leaves_exactly_one_blank_line` still holds and a note
  cannot leak into the step's prose.
- `Enter` saves was wrong once notes could be multi-line: a note is typed in paragraphs,
  and the save key moved to `Ctrl/Cmd+Enter`, matching the run screen's own bindings.
- `#` was left out of the renderer when only the content tree used it (a file's title is
  front matter, a step is `##`). Notes are authored text, and `#` is the first thing
  anyone types. It now renders, and `.prose h1` is sized down so a heading inside a note
  does not outrank the step title above it.

Cost: a note written by hand (not through the app) has to follow the same per-line rule to
read back as one block. The renderer itself is one implementation now - see D22.

## D22 - Markdown is rendered in Rust, not in the window

Decision: `sop_core::md::render` is the only Markdown implementation. The window asks for
HTML through the `render_markdown` command, with a small cache keyed by the source text.
`sop preview` renders every body server-side into the manifest it serves, so its page ships
no renderer at all.

Reasons:

- The window's own description is "No rules live here", and a renderer is a rule: what a
  `#` means, whether a table is a table, and what a task item looks like are as much a
  format decision as front matter is. Two copies (the window's TypeScript and the JS
  embedded in the preview page) had to be changed by hand in step, and a divergence would
  show the operator one document and a reviewer another.
- Rust is already where the format lives, so the next consumer - the PDF/HTML report in
  Phase 6 - gets the renderer for free, and `md::render` is tested next to `front`,
  `check`, and `run` instead of in a toolchain the repository does not otherwise use.
- Removing the page's renderer leaves the page what it is: a viewer. It also removes the
  only two places where content markup was decided outside `sop-core`.

Cost: rendering as the operator types is a Tauri round trip (local, and cached by source
text), and the preview page depends on the manifest carrying HTML. Both are cheaper than
two implementations of the same rule. The port also fixed a real bug the JS copy had: an
ordered list was closed with `</ul>`.

## D23 - The event log is appended, never rewritten

Decision: `run::record` appends one line to `events.jsonl` with `O_APPEND`, holds a
per-run advisory lock (`File::lock`) while it writes, and calls `sync_data` before
returning. The whole-file rewrite through `atomic::write` is gone from this path; the
rendered `record.md` is still regenerated for a human to read, but it is never the truth.

Reasons:

- The log is the source of truth (DESIGN 6.2), so the one thing a crash must not be able
  to do is truncate it. A read-all / append / rewrite is an obvious way to lose every
  earlier event if the machine dies mid-write; append-only means a crash can cost only the
  event in flight, and replay already ignores a torn final line.
- The app and the CLI can both be pointed at the same run - and two windows can. On POSIX
  a small `O_APPEND` write is atomic, but the lock makes the read-lock-append step one
  critical section, so a value the operator just typed and a CLI event cannot interleave.
- `sync_data` per event is the price of "written" meaning on the medium. Events are small
  and one happens per operator action, so the cost is invisible next to the keypress.

Cost: a run's `events.jsonl` can no longer be edited by hand and re-saved as a whole; a
correction is a new event (the `Reopen` path already works this way). This bumped the
declared Rust floor to 1.89 for `File::lock`.

Writing the concurrency test paid for itself: `atomic::write` used a fixed `path.tmp`
sibling, so two writers of the same `record.md` shared one temporary and the second
`rename` failed with "no such file". The temporary now carries the process id and a
per-process counter. The event log itself was never affected - it is appended, not
rewritten - but the rendered `record.md` was racing.

## D24 - Ending a run asks first, and a completeness gap is a reminder

Decision: ending a run goes through a confirmation bar (`requestEnd` / `confirmEnd`).
`ctrl+enter` only arms it and only when the focus is not in a field, so it cannot fire
from the note editor, where `ctrl+enter` means "save note". Marking a step done with
empty `required` captures asks for a reason and writes it with the status; the operator
can always continue.

Reasons:

- An ended run cannot be reopened, and the run screen is used one-handed with gloves; a
  single stray chord must not be able to finish a session's work.
- `required` was only an asterisk. A nudge is a completeness check, not a judgement about
  the data, so it does not cross the "the tool never judges" line (D4) - the reason is
  recorded and the operator decides.
- Showing the `expected` range before a value is typed, and rejecting a non-number for a
  `number` capture, moves the check to the moment the operator can act on it. The
  judgement on an out-of-range value is still theirs (the `Acknowledge` path, D3/D4).

## D25 - Commands run off the window's thread, and git is bounded

Decision: every Tauri command is `#[tauri::command(async)]` even though the functions are
synchronous, `git` subprocesses have a kill-deadline (15s for a local query, 30s for one
that reaches the network), and startup skips the pull when `navigator.onLine` is false.

Reasons:

- A synchronous Tauri command runs on the window's main thread. One slow command - a
  `git pull` at an unreachable remote, a walk over a large repository, a big attachment
  copy - freezes the whole window. Tauri maps a `(async)` sync fn onto its blocking
  thread pool, which is the `spawn_blocking` this would otherwise spell out by hand, and
  the function stays synchronous so Rust can still call it directly.
- `Command::output()` waits forever, so a dead network hangs the thread as long as the
  kernel lets it. The helper drains stdout/stderr on side threads (so a chatty command
  cannot fill its pipe and deadlock), polls to a deadline, and kills the child.
- A field laptop is often offline, so the startup pull is skipped outright when the
  system already knows there is no connection. The timeout is the backstop for when that
  signal is wrong.

Cost: an async command cannot borrow non-`'static` data across an await, so the bodies
stay synchronous (no awaits). A killed `git` leaves the repository as it was - the pull
either completed or did not.

## D26 - Reasons are collected in the app, and finishing a step moves you on

Decision: skip/deviate reasons, acknowledgement notes, and the required-capture reason
are collected by `PromptModal.svelte` with one-tap buttons for the common answers. A
reason prompt is `await`ed like a function. Marking a step done advances to the next step
with an outcome still missing; opening a step focuses its first empty capture.

Reasons:

- `window.prompt`/`confirm` block the webview, look like the browser rather than the app,
  and may not render at all in a packaged build (this is one of the three things ROADMAP
  0.1 has to confirm on a real machine). A field operator is one-handed and often gloved,
  so the answer they give most of the time should be one tap, not a keyboard.
- The modal never decides anything: it collects the operator's words, which is exactly
  what the event records. The tool still does not judge (D4).
- After a step is finished the next thing the operator wants is the next step, and the
  cursor belongs in the first field that still needs a value. Both are navigation, not
  policy.

Cost: a modal is more code than `prompt`, and auto-advance is a small surprise the first
time (a jump to another checklist, or wrapping to the first open step). The alternative -
leaving the operator to hunt for their place on a glare-washed screen - is worse.

## D27 - The start form folds, and the bar lands on the day's work

Decision: the start form keeps only run id, operator, and site visible; the instrument,
equipment, and conditions fold into a `<details>` whose summary reads "Same as last run:
…". The window opens on Test Plans, the navigation bar is Test Plans / Run / History /
Browse, and Edit / Project / Settings live behind a "Manage" dropdown.

Reasons:

- A run's identity is who, where, and which run. The instrument and conditions are almost
  always the same as the last run of the same checklist, so they are a review, not a
  form to fill. Folding them removes the fields a gloved hand can change by accident, and
  the one-line summary still shows what is being carried. It opens by itself when there
  is nothing to inherit.
- The first thing a session does is choose what to run, not edit a checklist. Landing on
  Test Plans means the run screen and the content editor are both one deliberate tap
  away, and the editor cannot be reached by a mis-tap while holding the laptop.

Cost: a setting that genuinely needs changing every run is one tap further. The Manage
group hides three views the operator rarely needs, and none of them had a one-key shortcut
to lose.

## D28 - A run carries a verdict, separate from its status

Decision: ending a run records the operator's own verdict - `pass`, `fail`, or
`inconclusive` plus a one-sentence note - as `conclusion` / `conclusion_note` in the run
record and a `RunConcluded` event. `status` (`complete` / `partial` / `aborted`) keeps
meaning "the run was executed"; `conclusion` is the human judgement)Skip. The two are
never derived from each other. The verdict is offered, never forced: a run can be ended
without one.

Reasons:

- `complete` answers "did the run happen", not "did it go well". A completed run can be a
  failure (all steps executed, the instrument out of spec). Before this, the only place
  that distinction lived was a note nobody was forced to write. The coverage matrix
  (3.2) and History filters (3.3) now read a cell the operator actually committed to.
- Keeping it human is D4 applied to the run as a whole: the tool records `pass` /
  `fail` / `inconclusive` because the operator says so, and never computes it from the
  step results. A verdict derived from the steps would inherit every threshold the tool
  does not own.
- It is additive (a new event type, two optional front-matter keys), so a record written
  before it exists reads unchanged and a run ended without one is still a valid record.
  `conclusion` and `conclusion_note` are written together, like `status` and `ended`.

Cost: ending a run now asks two more questions (verdict, then one sentence). The verdict
is a one-tap choice; the sentence can be skipped by backing out, and skipping ends the run
exactly as before. The alternative - a verdict computed from step status - is the D4 line
the whole repository exists not to cross.
