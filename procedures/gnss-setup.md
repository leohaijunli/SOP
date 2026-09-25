---
kind: procedure
procedure_id: gnss-setup
title: GNSS and Positioning Setup
version: 1
updated: 2026-09-24
applies_to: [ground-survey, uav-survey, navigation]
tags: [gnss, positioning]
---

Establishes the coordinate frame the magnetic data will be interpreted in. Datum and
height mistakes are silent: the survey processes normally and the anomalies end up in
the wrong place.

## Coordinate frame

```yaml step
id: gnss-datum
kind: check
severity: critical
captures:
  - key: datum
    label: Datum and coordinate frame
    type: text
    required: true
  - key: height_reference
    label: Height reference
    type: select
    options: [ellipsoidal, geoid, msl, unknown]
    required: true
```

Record the datum and the height reference explicitly. "WGS84" alone is ambiguous for
height, and the distinction between ellipsoidal and orthometric height matters as soon
as survey lines are compared with a terrain model.

- [ ] Datum recorded
- [ ] Height reference recorded
- [ ] Same frame used for base station, rover, and any existing maps

## Correction service and fix quality

```yaml step
id: gnss-fix
kind: measure
severity: critical
captures:
  - key: fix_type
    label: Fix type during the survey
    type: select
    options: [rtk-fixed, rtk-float, dgps, single, other]
    expected: rtk-fixed
    required: true
  - key: satellites
    label: Satellites tracked
    type: integer
    required: false
  - key: hdop
    label: Horizontal dilution of precision
    type: number
    required: false
```

Confirm the correction source is actually working before the survey starts, and note
the fix type that is expected to hold. A survey that quietly falls back to autonomous
positions has metre-level errors in it, which is often the same order as the anomalies
being mapped.

- [ ] Correction source connected and confirmed
- [ ] Fix type observed and recorded before the first line
- [ ] Fallback behaviour understood: what happens if corrections drop out

## Antenna height and geometry

```yaml step
id: gnss-height
kind: measure
severity: critical
captures:
  - key: antenna_height_m
    label: Antenna height above ground
    type: number
    unit: m
    required: true
```

Measure the pole or mast height physically. This is independent of the sensor-to-antenna
offset recorded by the lever arm procedure: both are needed, and they are different
quantities.

- [ ] Antenna height measured with a tape
- [ ] Height recorded against the same reference recorded above
- [ ] Antenna mounting checked for rigidity during movement
