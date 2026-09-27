---
kind: procedure
procedure_id: static-noise-test
title: Static Noise Test
version: 1
updated: 2026-09-24
applies_to: [calibration]
tags: [noise, repeatability]
---

The floor of what the sensor can resolve. Everything downstream - anomaly detection,
gradient calculation, tie-line levelling - is limited by this number, so it is worth
measuring properly rather than assuming the datasheet value.

## Install the sensor for a static measurement

```yaml step
id: noise-setup
kind: check
severity: critical
captures:
  - key: distance_from_structures_m
    label: Distance to nearest structure or vehicle
    type: number
    unit: m
    required: true
```

Mount the sensor on a rigid non-magnetic stand at the height it will be used at during
the survey. A sensor tested at a different height or orientation is not testing the
same configuration that will fly or be carried.

Choose a location where the ambient field is flat. If the reading wanders while you
are standing still, the site is the problem, not the sensor.

- [ ] Non-magnetic stand used, at the operational height
- [ ] Cables dressed so they cannot move or flex
- [ ] Location verified quiet before starting

## Record the noise window

```yaml step
id: noise-sample
kind: measure
severity: critical
captures:
  - key: duration_s
    label: Observation window
    type: number
    unit: s
    required: true
  - key: sigma_nt
    label: Standard deviation (1 sigma)
    type: number
    unit: nT
    required: true
  - key: peak_to_peak_nt
    label: Peak-to-peak excursion
    type: number
    unit: nT
    required: true
```

Leave the sensor completely undisturbed for at least 300 seconds and compute the
statistics over that window. Nobody should be walking near the sensor while it runs.

For guidance, a quiet caesium or Overhauser total-field sensor typically shows 1 sigma
well below 0.1 nT; a fluxgate is one to two orders of magnitude noisier. Treat a
result far outside the expected range as a reason to re-test, not as a pass or fail
decided by this checklist.

- [ ] Sensor undisturbed for the full window
- [ ] Statistics computed from the raw data, not from a display average
- [ ] Window and statistics recorded in the notes

## Repeat for confidence

```yaml step
id: noise-repeat
kind: check
severity: normal
captures:
  - key: runs_completed
    label: Number of completed windows
    type: integer
    required: true
```

A single window can look good by chance. Run at least three, and treat a wide spread
between them as a finding in its own right.

- [ ] At least three windows completed
- [ ] Spread between windows noted

## Noise floor check

```yaml step
id: noise-verdict
kind: note
severity: normal
```

Write down the measured floor next to the anomaly amplitude the survey needs to
detect. A sensor whose noise floor is comparable to the target anomalies will not
resolve them, no matter how the data is processed later. Recording both numbers
side by side is what makes that visible early.
