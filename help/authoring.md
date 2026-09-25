---
kind: help
help_id: authoring
title: Adding to the content
section: Authoring
order: 60
audience: author
summary: How to add a procedure, a checklist, a help page, or a step without breaking history.
updated: 2026-09-24
tags: [authoring, format]
---

Everything in this list is an edit to a Markdown file. There is no import step and no
database.

## Adding a step to a procedure

A step is a `##` heading followed by an optional `yaml step` block and then the prose
that the operator reads.

````markdown
## Compare against a reference

```yaml step
id: abs-reference
kind: measure
severity: normal
captures:
  - key: reference_value_nt
    label: Reference value
    type: number
    unit: nT
    required: true
```

Take the reference reading twice, at least five minutes apart.
````

Rules that matter:

- The `id` is **permanent**. Run records cite it. To retire a step, set
  `deprecated: true` and stop using it; never renumber and never reuse.
- The `##` heading is mandatory and becomes the step's title.
- `kind` is one of `check`, `measure`, `select`, `note`, `gate`.
- `severity` is `info`, `normal`, or `critical`. `critical` means a failed or skipped
  step invalidates the run.
- A capture needs a `key`, a `label`, and a `type`. A `select` also needs `options`.

## Adding a procedure

Create `procedures/<procedure_id>.md`. The filename and `procedure_id` must match.

```yaml
---
kind: procedure
procedure_id: base-station
title: Base Station Setup
version: 1
updated: 2026-09-24
applies_to: [ground-survey, interference]
tags: [reference]
---
```

Nothing executes a procedure on its own. It becomes reachable when a checklist includes
it.

## Adding a checklist

Create `checklists/<sop_id>.md`, then place include markers where the procedures belong.
Order is decided by the marker's position, so the file reads in execution order:

````markdown
<!-- include: procedures/site-recon.md -->
<!-- include: procedures/power-on.md -->
<!-- include: procedures/base-station.md -->

## Team briefing

```yaml step
id: briefing-risks
kind: note
severity: normal
```
````

A procedure may appear only once in a checklist. Step ids must be unique across the whole
resolved checklist, not just within each procedure - `sop validate` checks this and names
both files when it fails.

## Adding a help page

Create `help/<help_id>.md`:

```yaml
---
kind: help
help_id: commands
title: Common commands
section: Commands
order: 10
audience: operator
summary: One line, shown in the panel's list.
updated: 2026-09-24
tags: [cli]
---
```

- `section` is free text and groups the page in the panel.
- `order` positions the page in the panel, lower first. It is global across help pages,
  not per section, so a section appears wherever its lowest page appears. Leave gaps
  between the numbers you use.
- `audience` is `operator`, `author`, or `maintainer`.

A new section needs no change anywhere else: the panel's sections are derived from the
pages.

## Turning an observation into a procedure change

Observations that are not yet proven go to `runs/_inbox/`, not into a procedure:

```yaml
---
created: 2026-09-24
author: leo
observed_in: runs/ground-walk-survey/2026-09-24-renfrew-walk01.md
target: procedures/walk-line.md
confidence: medium
---
```

When the same observation has held up more than once, edit the procedure and delete the
inbox entry in the same commit. The inbox is where "we should add this to the SOP" lives
until it earns its place.

## Before you commit

```bash
sop validate
```

Errors block; warnings are worth a look. If you are changing a procedure that a run
record currently cites, remember that the ids, not the wording, are the contract.
