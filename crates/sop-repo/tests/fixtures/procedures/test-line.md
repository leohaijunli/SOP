---
kind: procedure
procedure_id: test-line
title: Test Line and Repeatability Check
version: 1
updated: 2026-09-24
applies_to: [ground-survey, uav-survey]
tags: [qc, repeatability]
---

The only step that can distinguish "the ground is quiet" from "the instrument stopped
recording". A test line walked at the start and repeated at the end also bounds the
drift across the whole session.

## Choose the test line

```yaml step
id: testline-select
kind: check
severity: critical
captures:
  - key: test_line_id
    label: Test line identifier
    type: text
    required: true
```

Pick a line that crosses representative ground and is easy to walk identically twice.
It should include at least one real feature, because a line over uniformly flat ground
cannot show whether the instrument is recording the field at all.

- [ ] Line crosses representative ground and includes a real feature
- [ ] Both end points are physically identifiable for the repeat
- [ ] Line identifier recorded on the site sketch

## First pass

```yaml step
id: testline-before
kind: measure
severity: critical
captures:
  - key: before_mean_nt
    label: Mean value over the test line, first pass
    type: number
    unit: nT
    required: true
  - key: before_sigma_nt
    label: Standard deviation, first pass
    type: number
    unit: nT
    required: false
  - key: testline_log
    label: Test line log file
    type: attach
    accept: [".csv", ".txt", ".h5"]
    required: false
```

- [ ] Test line walked at the start of the session
- [ ] Mean and spread computed and recorded
- [ ] Any feature visible in the trace noted

## Repeat at end of session

```yaml step
id: testline-after
kind: measure
severity: critical
captures:
  - key: after_mean_nt
    label: Mean value over the test line, repeat pass
    type: number
    unit: nT
    required: true
  - key: difference_nt
    label: Difference between passes
    type: number
    unit: nT
    required: true
```

Walk the line again the same way, at the end of the session, before packing up. Do
this even if the session ran short or went badly - especially then.

Compare the two traces shape-to-shape as well as mean-to-mean. A small offset with
matching shape is normal and correctable; a difference in shape means the geometry
changed, and the survey lines walked in between inherit that problem.

- [ ] Repeat walked at the end of the session
- [ ] Difference recorded
- [ ] Traces compared for shape, not only for mean value

## Trailing note

```yaml step
id: testline-note
kind: note
severity: info
```

Describe what you saw in the two traces while it is still on screen. The numbers alone
rarely capture it, and the person processing the data was not standing in the field.
