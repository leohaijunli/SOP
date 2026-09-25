# field-sop Format Specification

Version 1 - 2026-09-24

This document is authoritative. `tools/validate.py` implements the checks described
here; when the two disagree, this document is wrong and should be fixed.

## 1. Scope

Five file kinds are defined:

| Kind | Lives in | Role |
|---|---|---|
| `procedure` | `procedures/` | A reusable block of steps. Not directly executable on its own. |
| `checklist` | `checklists/` | One experiment type. Includes procedures and adds local steps. This is what an operator runs. |
| `run` | `runs/` | The record of one execution of one checklist. |
| `help` | `help/` | Documentation read while working. Never executed. |
| `project` | `project.md` | The repository's identity: what campaign this is, and who is responsible for it. See section 14. |

A checklist does not copy procedure text. It references it, so fixing one procedure
fixes every checklist that uses it. This matters because the experiment matrix
(experiment type x sensor x site x software/hardware configuration) would otherwise
require dozens of near-identical documents that drift apart.

## 2. Controlled vocabulary

`applies_to` values are drawn from this list. Adding a value is a spec change.

`calibration`, `ground-survey`, `interference`, `navigation`, `uav-survey`

`severity` is one of `info`, `normal`, `critical`. `critical` means a failed or
skipped step invalidates the run.

`kind` (on a step) is one of `check`, `measure`, `select`, `note`, `gate`.

`status` (on a run) is one of `complete`, `partial`, `aborted`.

`status` (on a step result) is one of `done`, `skipped`, `deviated`.

`audience` (on a help page) is one of `operator`, `author`, `maintainer`.

## 3. Formatting rules

Applies to every file:

- UTF-8, no BOM.
- LF line endings.
- Exactly one trailing newline.
- Front matter starts on line 1 with `---` and ends with a closing `---`.
- Tabs are not used for indentation.

Any file may optionally declare the content schema it was written against:

```yaml
schema: 1
```

Absence means schema 1. `SPEC-COMPAT.md` defines what may change between versions and
what the tooling does when it meets a file from the future.

## 4. Identifiers

- Must match `^[a-z0-9][a-z0-9-]*$`.
- Must be unique within the file that defines them.
- After resolving include markers, all step ids in a checklist must be unique.
- Ids are permanent. Renaming an id breaks every run record that cites it. To retire
  a step, set `deprecated: true` and stop using it.

Step ids are referenced by run records, so they are part of the data contract, not a
display detail.

## 5. Procedures (`procedures/*.md`)

````markdown
---
kind: procedure
procedure_id: magnetic-hygiene
title: Personnel and Equipment Magnetic Hygiene
version: 1
updated: 2026-09-24
applies_to: [calibration, ground-survey]
tags: [safety, hygiene]
---

Introductory prose for the whole block. Optional.

## Remove magnetisable items from the operator

```yaml step
id: hygiene-person
kind: check
severity: critical
```

Any ferrous object on the operator shows up as an anomaly.

- [ ] Watch, phone, keys, coins removed
- [ ] Belt buckle or steel-toed boots replaced
````

Required front matter: `kind`, `procedure_id`, `title`, `version`, `updated`,
`applies_to`.

Optional: `tags`, `notes`.

`version` is an integer, incremented when the procedure changes in a way that would
change how a step is executed or evaluated. Fixing a typo does not require a bump.

## 6. Steps

A step is a `##` heading followed by optional structured metadata and then prose.

The `yaml step` block:

```yaml
id: static-noise            # required, unique
kind: measure               # required
severity: critical          # optional, default normal
deprecated: false           # optional
captures:                   # optional list of values the operator records
  - key: sigma_nt
    label: Static noise (1 sigma)
    type: number
    unit: nT
    required: true
```

Capture `type` is one of `text`, `number`, `integer`, `bool`, `select`, `datetime`,
`duration`, `attach`.

`select` requires an `options` list. `number` and `integer` may carry `unit`. `attach`
may carry `accept`, a list of file extensions used only as a filter hint in the UI.

The prose body is unrestricted CommonMark: tables, images, links, code blocks. Put
the instruction, the acceptance criterion, and the warning here - in that order is a
good convention, but only the text matters to tooling.

### Expected values

Captures of type `number`, `integer`, `select`, and `bool` may declare an `expected`
value or range:

```yaml
captures:
  - key: sigma_nt
    label: Static noise (1 sigma)
    type: number
    unit: nT
    expected:
      min: 0
      max: 0.5
  - key: self_test_result
    label: Self-test result as reported
    type: select
    options: [ok, warning, error]
    expected: ok
```

`expected` is **presentation only**. The app highlights a value outside it and requires
the operator to acknowledge that field explicitly. The tooling never marks a step pass
or fail, and never writes a verdict - see `DECISIONS.md` D4.

Declaring it is still worth doing. It moves the criterion out of prose and in front of
the operator at the moment of entry, which is when it is still actionable.

`expected` is valid only on those four types. Declaring it on `text`, `datetime`,
`duration`, or `attach` is a validation error, because there is nothing meaningful to
compare against.

### Checkbox and prose list semantics

List items are interpreted one at a time, by their own syntax. There is no whole-list
rule:

- An item beginning `- [ ]` is an interactive checkbox.
- An item beginning `- [x]` is a **validation error** in a procedure or checklist.
  Templates are executed repeatedly; a pre-checked item cannot be. It nearly always
  means a run record was copied into a template.
- Any other list item is prose.

This deliberately differs from tools that treat a list containing a mix of checkbox and
plain items as entirely prose. That rule is a silent trap: one stray bullet silently
changes the meaning of every other item in the list, with no error and no visible
difference in a Markdown preview.

Checkbox identity is the 0-based index of the item within its step. The index is stable
for the life of a run because the checklist is snapshotted when the run starts, and a
run record must never be interpreted against a different revision of its checklist.

A step with no checkbox items is allowed and is treated as prose-only guidance.

### Writing acceptance criteria

State the criterion as fact, not as a logged judgement, and record the measured value
separately.

```markdown
Acceptance: static noise 1 sigma below 0.1 nT over 300 s.

- [ ] Sensor undisturbed for the full window
- [ ] Reading recorded
```

Where the criterion is numeric or a known-good value, put it in `expected` as well, so
it is in front of the operator while entering the value rather than only on read-back.

## 7. Checklists (`checklists/*.md`)

````markdown
---
kind: checklist
sop_id: ground-walk-survey
title: Ground Magnetic Walk Survey
version: 1
updated: 2026-09-24
status: draft
applies_to: [ground-survey]
equipment:
  - Total-field magnetometer with 1 Hz logging
  - Non-magnetic tripod
  - GNSS receiver
---

Prose shown at the top of the checklist. Scope, prerequisites, what "done" means.

## Team briefing

```yaml step
id: walk-briefing
kind: check
```

A step defined directly in the checklist, for something not reusable enough to live
in `procedures/`.

<!-- include: procedures/power-on.md -->

<!-- include: procedures/magnetic-hygiene.md -->

## Session close-out

```yaml step
id: walk-session-closeout
kind: note
```

Another local step, after the included ones.
````

Required front matter: `kind`, `sop_id`, `title`, `version`, `updated`, `status`,
`applies_to`.

`status` is `draft`, `active`, or `retired`. Only `active` checklists should be used
for real data collection.

### Include markers

Procedure steps are pulled in at a specific document position with an HTML comment:

```markdown
<!-- include: procedures/power-on.md -->
```

An include marker must be on its own line, and the path is relative to the repository
root. The marker renders as nothing in GitHub and in any CommonMark viewer, so a
checklist stays readable as ordinary Markdown.

Step order is document order, with each marker replaced in place by the steps of the
procedure it names. This is why ordering is expressed inline rather than in front
matter: a checklist needs its briefing before the shared startup steps and its
close-out after them, and a front-matter list cannot express that.

Including the same procedure twice in one checklist is an error, because the step ids
would then collide.

## 8. Run records (`runs/<sop_id>/<run_id>.md`)

One file per execution. The file is created once and then only appended to.

````markdown
---
kind: run
run_id: 2026-09-24-renfrew-walk01
sop: ground-walk-survey
sop_version: 1
sop_commit: 8f3c1a2
operator: Feng
site: Renfrew 395
started: 2026-09-24T16:10:00Z
ended: 2026-09-24T20:05:00Z
status: complete
sensor:
  model: GEM GSM-19
  serial: "4451233"
  firmware: "7.0"
hardware:
  - mag_gcs v0.3.1
conditions:
  weather: clear
  temp_c: 12
logs:
  - path: logs/2026-09-24-renfrew-walk01/mag_raw.csv
    sha256: 0f2a...
    size: 1048576
    description: Raw magnetometer log
---

## Static noise test

```yaml result
step: static-noise
status: done
captures:
  sigma_nt: 0.08
  duration_s: 300
```

Operator prose. Anything unusual goes here.
````

Required front matter: `kind`, `run_id`, `sop`, `sop_version`, `operator`, `site`,
`started`, `status`, `deviations_count`.

`run_id` must equal the filename stem, and must be unique repo-wide. Recommended
form: `<YYYY-MM-DD>-<site-slug>-<sop-slug><NN>`.

Timestamps are ISO 8601 in UTC with a `Z` suffix. Local time with an offset is
accepted but UTC is preferred, because it must line up with instrument logs.

`sop_version` is the version of the checklist that was actually executed.
`sop_commit` is the git commit it was read from. Both are required for the record to
be reviewable later - without them, nobody can tell which revision produced the data.

Each executed step gets one `yaml result` block. `step` must match an id that exists
in the referenced checklist after include markers are resolved.

A result whose `status` is `skipped` or `deviated` requires a `reason` string. Both
are equally useful signals:

- `skipped` - a required step did not happen. Bound the dataset's validity.
- `deviated` - the step happened but not as written. This is the highest-value output
  of the whole system, because it is the raw material for improving the procedures.

`deviations_count` counts `deviated` results only, not `skipped` ones.

## 9. Log attachments

Logs are attached manually by the operator at the end of a run. The field app does not
discover them.

Flow:

1. Operator picks the log files for the run in the app.
2. The app copies them to `logs/<run_id>/`.
3. The app computes `sha256` and `size` for each and writes the `logs:` entries into
   the run record.
4. The app commits the run record and the logs together, so the record and its data
   are never separated in history.

`sha256` is what makes a record trustworthy later: it proves the analysed file is the
one that was collected. See `docs/LOGS.md` for naming, size limits, and the Git LFS
policy.

## 10. Write-back rules

These bind the field app, which edits these files under a single-writer assumption
(one operator drives the app; a team may run experiments in parallel, but never two
people on the same file).

1. The app may append a step block to a section, or replace exactly one block.
2. The app must not rewrite any line outside the block it is targeting. Hand-written
   prose, comments, unusual formatting, and unknown syntax are preserved verbatim.
3. When the app creates content, it uses the canonical form in this spec so that
   `git diff` stays readable.
4. The app writes run records as new files. It must not modify an existing run record
   except by appending to a trailing section.
5. On load, the app records the file's `sha256` and mtime. If either changed before a
   write, it reloads and surfaces the change rather than overwriting.

Rule 2 is the important one. Everything else is convention.

## 11. Promotion of field knowledge

Observations that are not yet proven do not belong in a procedure. Put them in
`runs/_inbox/` as a short Markdown file, then promote them deliberately:

1. During a run, note the observation in the run record.
2. At review time, copy the reusable part into `runs/_inbox/`.
3. When the observation has held up more than once, open a pull request that edits
   the relevant procedure and deletes the inbox entry.

The inbox is deliberately not a checklist and is never executed. It exists so that
"we should add this to the SOP" has somewhere to live other than someone's memory.

## 12. Help pages (`help/*.md`)

Help is what an operator reads while working: how to drive the app, the commands worth
memorising, how to extend the content, and what the validation messages mean. It lives in
the repository, in the same format as everything else, so it can be extended without
touching the application and it reviews like any other change.

````markdown
---
kind: help
help_id: attaching-logs
title: Attaching logs and photos
section: Using the app
order: 40
audience: operator
summary: How files are attached to a run, and why they are hashed.
updated: 2026-09-24
tags: [logs, attachments]
---

## What counts as a log

Any file that a run produced or that a run depends on.
````

Required front matter: `kind`, `help_id`, `title`, `section`, `updated`.

Optional: `order`, `audience`, `summary`, `tags`.

- `help_id` must match the filename.
- `section` is free text and groups the page in the panel. It is deliberately not a
  controlled vocabulary, because adding a section must not require a spec change.
- `order` is an integer, default `100`, and is **global across help pages**, not per
  section: a section appears in the panel wherever its lowest `order` appears. Leave gaps
  in the numbering rather than using consecutive integers.
- `audience` is drawn from the vocabulary in section 2.

Help pages carry no `version` and no `applies_to`. There is nothing to reproduce later,
so there is nothing to freeze, and a page is simply edited in place.

The body is never executed. `##` headings are only headings, and a `yaml step` block in a
help page is a quoted example rather than a step. A help page may therefore show a
pre-checked `- [x]` item, which procedures and checklists may not.

The generated manifest carries every page with its section and body, so the panel needs
one read and no extra file access:

```json
{
  "help": [
    {
      "help_id": "overview",
      "title": "What this tool is for",
      "section": "Getting started",
      "order": 10,
      "audience": "operator",
      "path": "help/overview.md",
      "body": "A geomagnetic survey produces two things ..."
    }
  ],
  "help_sections": [ { "title": "Getting started", "order": 10 } ]
}
```

## 13. Validator checks

`sop validate` enforces:

- front matter parses, and required keys per kind are present;
- ids are well-formed, and unique within a file and after include resolution;
- include markers resolve to existing procedures, with no duplicate inclusion;
- `applies_to` values come from the controlled vocabulary;
- capture `expected` is only declared on `number`, `integer`, `select`, or `bool`;
- `expected` ranges are ordered, and a `select` expectation is one of its own options;
- no `- [x]` pre-checked items appear in a procedure or checklist;
- a declared `schema` is not newer than the tooling supports;
- relative Markdown links resolve to files or anchors that exist;
- run records reference an existing checklist, and their `step` ids exist in it;
- `skipped` and `deviated` results both carry a `reason`;
- for `complete` runs, every resolved step has a result and none is `skipped`;
- `run_id` matches the filename, and `deviations_count` matches reality;
- `logs[].path` exists and its `sha256`/`size` match the file on disk;
- help pages carry the required keys, a valid `help_id` matching the filename, and a
  non-empty `section`; two help pages claiming the same `order` are a warning;
- a `project.md`, when present, declares `kind: project`, carries `project_id`, `title`,
  and `updated`, and any `updated` or `started` it declares is a `YYYY-MM-DD` date;
- formatting rules from section 3.

Exit code is non-zero if any check fails.

## 14. Project identity (`project.md`)

One file at the repository root names the work. It is what the application shows in its
title bar, what `sop status` prints, and what a reader of a run record two years from now
reads to learn which campaign the run belonged to.

````markdown
---
kind: project
project_id: uvic-geomag-survey
title: Geomagnetic Survey Field Work
institution: University of Victoria
lead: leo
started: 2026-09-01
updated: 2026-09-25
summary: Sensor calibration, ground walk surveys, interference and navigation tests.
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [campaign]
---

Prose about the campaign.
````

Required front matter: `kind`, `project_id`, `title`, `updated`.

Optional: `institution`, `lead`, `started`, `summary`, `contact`, and the common
`applies_to`, `tags`, `schema`.

### This is content, not a preference

The project's name is **not** an application setting. It is committed, so every operator
and every historical record agree about which campaign the work belongs to. If it lived
in each operator's own configuration, two laptops would disagree, both would look
correct, and the disagreement would never be reviewed. The application's settings hold
the *path* to the working copy and the *name* of the git remote, and nothing about the
project itself.

### Editing

Edit the file in place. `sop project` shows the identity, and `sop project set <field>
<value>` rewrites one field:

```console
$ sop project set title "Renfrew Walk 2026"
project.md: title = Renfrew Walk 2026
```

Every other byte of the file is left alone: the prose, the key order, the comments, and
any key this build does not know about. A key that is absent is added inside the front
matter rather than at the end of the file.

The result is parsed and checked *before* it is written, so a command that reports
success cannot have left a project file the validator rejects. A value that would fail -
a `project_id` with a space in it, an `updated` that is not a date - is refused, and the
file is untouched.

`project_id` is stable. Changing it is a rewrite of the project's name, not the creation
of a new project, and nothing in the format detects the difference after the fact, so
prefer not to change it. Adding a *field* is additive and needs no schema bump (see
`SPEC-COMPAT.md`); the fields above are all optional except the four required ones.
