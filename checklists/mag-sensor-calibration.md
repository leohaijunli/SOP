---
kind: checklist
sop_id: mag-sensor-calibration
title: Geomagnetic Sensor Calibration
version: 1
updated: 2026-09-24
status: draft
applies_to: [calibration]
equipment:
  - Total-field magnetometer with 1 Hz or better logging
  - Non-magnetic tripod or rigid stand, at the operational height
  - Non-magnetic rotation fixture with marked azimuths
  - Bubble level
  - Tape measure
  - GNSS receiver
---

Sensor-level calibration of a geomagnetic sensor: what the sensor can resolve, how it
behaves as it is rotated and tilted, whether its absolute values are right, and where
the measurement point sits relative to the GNSS antenna.

Run this on a sensor before it is used for survey work, after any change to its
mounting or its electronics, and on a periodic schedule thereafter. Results are only
comparable within a firmware version, which is why the boot sequence captures it.

**Not covered here:** survey-level checks such as diurnal correction and tie-line
levelling. Those belong to the survey checklists.

**Output:** a set of sensor parameters - noise floor, heading error, tilt sensitivity,
absolute offset, and lever arm - that should be applied when this sensor's data is
processed.

<!-- include: procedures/conditions-log.md -->

<!-- include: procedures/power-on.md -->

<!-- include: procedures/magnetic-hygiene.md -->

<!-- include: procedures/warm-up.md -->

<!-- include: procedures/logging-setup.md -->

<!-- include: procedures/static-noise-test.md -->

<!-- include: procedures/heading-error.md -->

<!-- include: procedures/tilt-orientation.md -->

<!-- include: procedures/absolute-value-check.md -->

<!-- include: procedures/lever-arm.md -->

<!-- include: procedures/pack-down.md -->

## Sign-off

```yaml step
id: cal-signoff
kind: note
severity: normal
captures:
  - key: fit_for_use
    label: Sensor judged fit for the intended survey
    type: select
    options: ["yes", "yes-with-limits", "no"]
    required: true
  - key: limits_or_conditions
    label: Limits, conditions, or parameters to apply
    type: text
    required: false
```

Write a short verdict paragraph: what was measured, whether the sensor is fit for the
intended survey, and which of the measured parameters should be applied to its data.
The individual numbers are in the step results above; this paragraph is what the next
person reads first.

If any step was skipped or deviated from, say which and why. A calibration record with
an unexplained gap is worse than one that states its limitation plainly.

- [ ] Verdict written
- [ ] Any parameters to apply to the data listed explicitly
- [ ] Skipped or deviated steps named
