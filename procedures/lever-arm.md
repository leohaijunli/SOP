---
kind: procedure
procedure_id: lever-arm
title: Sensor Geometry and Lever Arm Survey
version: 1
updated: 2026-09-24
applies_to: [calibration, uav-survey, navigation]
tags: [geometry, lever-arm]
---

Establishes where the magnetic measurement actually happened. Without this, a
position from GNSS is attributed to a point up to a metre away from the sensor, and
the resulting anomaly is smeared along the flight or walk direction.

## Sensor to GNSS antenna offset

```yaml step
id: lever-antenna
kind: measure
severity: critical
captures:
  - key: offset_x_m
    label: Offset along the forward axis
    type: number
    unit: m
    required: true
  - key: offset_y_m
    label: Offset along the right axis
    type: number
    unit: m
    required: true
  - key: offset_z_m
    label: Offset along the down axis
    type: number
    unit: m
    required: true
```

Measure as a vector in the platform's own frame, with the sign convention stated in
the notes. Measure with a tape on the physical hardware rather than reading a drawing,
and state which axes are which - an unlabelled offset is worse than none, because it
will be applied with the wrong sign.

- [ ] All three axes measured on the physical hardware
- [ ] Sign convention written down in the notes
- [ ] Reference point on the sensor and on the antenna named

## Separation from interfering hardware

```yaml step
id: lever-separation
kind: check
severity: critical
captures:
  - key: distance_to_motors_m
    label: Distance from sensor to nearest motor
    type: number
    unit: m
    required: false
  - key: distance_to_electronics_m
    label: Distance from sensor to nearest electronics enclosure
    type: number
    unit: m
    required: false
```

Magnetic cleanliness falls off with distance, but slowly. Record the actual
separation; it sets the expectation for what the interference baseline will look
like, and whether a boom extension is worth building.

- [ ] Distances measured, not estimated
- [ ] Cable route noted: no current-carrying conductor closer than necessary to the sensor
