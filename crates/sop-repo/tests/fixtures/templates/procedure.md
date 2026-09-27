---
kind: procedure
procedure_id: replace-me
title: Replace Me
version: 1
updated: 2026-01-01
applies_to: [calibration]
tags: []
---

One or two sentences on what this procedure covers and when it applies. Delete if
obvious from the title.

## Step title

```yaml step
id: replace-me-step
kind: check
severity: normal
captures:
  - key: example_value
    label: Example value
    type: number
    unit: nT
    expected:
      min: 0
      max: 1
    required: false
  - key: example_result
    label: Example selection
    type: select
    options: [ok, warning, error]
    expected: ok
    required: true
  - key: example_log
    label: Log file produced by this step
    type: attach
    accept: [".csv"]
    required: false
```

What the operator does, and why it matters.

Acceptance: state the criterion as fact, not as a judgement.

- [ ] First observable done
- [ ] Value recorded

## Second step title

```yaml step
id: replace-me-step2
kind: note
```

Prose-only guidance. No task list means no ticks; the step is a prompt to write.
