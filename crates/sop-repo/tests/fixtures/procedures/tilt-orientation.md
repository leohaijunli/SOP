---
kind: procedure
procedure_id: tilt-orientation
title: Tilt and Orientation Sensitivity
version: 1
updated: 2026-09-24
applies_to: [calibration, uav-survey]
tags: [tilt, orientation]
---

Companion to the heading test. A sensor that is sensitive to tilt will show an error
whenever the platform pitches or rolls, which happens continuously on a walking
operator and on any airborne platform.

## Establish a level reference

```yaml step
id: tilt-baseline
kind: measure
severity: critical
captures:
  - key: level_reading_nt
    label: Reading at level orientation
    type: number
    unit: nT
    required: true
```

Record the reading with the sensor levelled, using the same fixture as the heading
test. This is the reference the tilted readings are compared against.

- [ ] Sensor levelled using a bubble level or equivalent
- [ ] Reference reading recorded with its timestamp

## Tilt in two axes

```yaml step
id: tilt-sweep
kind: measure
severity: critical
captures:
  - key: tilt_deg
    label: Tilt applied
    type: number
    unit: deg
    required: true
  - key: deviation_nt
    label: Largest deviation from the level reading
    type: number
    unit: nT
    required: true
```

Repeat at ten degrees in pitch and in roll, in both directions, returning to level
between positions to confirm the reference has not drifted. Record the largest
deviation from the level reading.

In practice this sets the mounting tolerance: if ten degrees of tilt moves the reading
by more than the anomalies of interest, the platform needs a gimbal or a rigid mount,
and the recorded tilt should be available to correct the data later.

- [ ] Pitch and roll both tested, in both directions
- [ ] Level reference re-checked between positions
- [ ] Largest deviation recorded
