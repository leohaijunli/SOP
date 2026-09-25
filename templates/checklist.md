---
kind: checklist
sop_id: replace-me
title: Replace Me Checklist
version: 1
updated: 2026-01-01
status: draft
applies_to: [calibration]
equipment:
  - Item one
  - Item two
---

Scope of this checklist: what experiment it covers, what it does not cover, and what
must be true before the first step is useful.

## Local step title

```yaml step
id: replace-me-local
kind: check
severity: normal
captures:
  - key: example_value
    label: Example value
    type: text
    required: false
```

Only define a step here if it is specific to this experiment and too narrow to reuse.
Otherwise add it to a procedure in `procedures/` and include it below.

- [ ] Detail that is specific to this experiment

<!-- include: procedures/power-on.md -->

## Closing step title

```yaml step
id: replace-me-closing
kind: note
```

Steps after the include markers run after the included procedure's steps. Position in
the document is the step order.
