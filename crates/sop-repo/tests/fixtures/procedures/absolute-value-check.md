---
kind: procedure
procedure_id: absolute-value-check
title: Absolute Value and Repeatability Check
version: 1
updated: 2026-09-24
applies_to: [calibration]
tags: [accuracy, reference]
---

Sensor noise and heading error say nothing about whether the values are right. This
step anchors the readings to something outside the instrument.

## Compare against a reference

```yaml step
id: abs-reference
kind: measure
severity: critical
captures:
  - key: reference_source
    label: Source of the reference value
    type: text
    required: true
  - key: reference_value_nt
    label: Reference value
    type: number
    unit: nT
    required: true
  - key: measured_value_nt
    label: Value measured by this sensor
    type: number
    unit: nT
    required: true
  - key: difference_nt
    label: Difference
    type: number
    unit: nT
    required: true
```

A usable reference is a nearby observatory value reduced to the same epoch and
location, a second instrument already trusted in the same place at the same time, or
a permanently marked reference station measured repeatedly across sessions. A
datasheet absolute figure is not a reference for this purpose.

Compare values taken at the same time. The Earth's field varies enough over hours that
a comparison against a value measured this morning mostly measures the diurnal
variation, not the sensor.

- [ ] Reference is traceable and measured at the same epoch
- [ ] Both values and the difference recorded
- [ ] Reference source named precisely enough to be re-checked

## Repeat across sessions

```yaml step
id: abs-repeat
kind: check
severity: critical
captures:
  - key: sessions_completed
    label: Number of sessions compared
    type: integer
    required: true
  - key: session_spread_nt
    label: Spread of the offset across sessions
    type: number
    unit: nT
    required: true
```

Re-occupy the same reference on separate days. A stable offset that repeats is a
calibration constant that can be applied. An offset that changes between sessions
means something in the installation is not repeatable, and the earlier sessions
cannot be corrected retrospectively.

- [ ] At least three sessions compared
- [ ] Same reference location re-occupied, documented well enough to find again
- [ ] Spread across sessions recorded
