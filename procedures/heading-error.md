---
kind: procedure
procedure_id: heading-error
title: Heading Error Test
version: 1
updated: 2026-09-24
applies_to: [calibration, uav-survey]
tags: [heading, orientation]
---

Total-field sensors do not read identically in all orientations. The reading changes
as the sensor rotates about the vertical axis, and on a moving platform that error
becomes a false anomaly correlated with direction of travel. For a walking survey it
is small; for an airborne or vehicle-mounted sensor it is often the largest single
error term.

## Prepare a rotation fixture

```yaml step
id: heading-setup
kind: check
severity: critical
```

The sensor must be rotated about its vertical axis without tilting and without the
operator moving relative to it. A non-magnetic turntable, or a marked plate the sensor
is re-seated on, both work. Marking azimuths on the ground and walking the sensor
between them does not, because the operator moves too.

- [ ] Rotation fixture is non-magnetic and does not tilt
- [ ] Azimuth reference marked, traceable to north
- [ ] Operator position kept clear and constant

## Sweep azimuths

```yaml step
id: heading-sweep
kind: measure
severity: critical
captures:
  - key: azimuth_step_deg
    label: Azimuth increment
    type: number
    unit: deg
    required: true
  - key: heading_error_max_nt
    label: Maximum spread across azimuths
    type: number
    unit: nT
    required: true
```

Take a reading at eight azimuths, 45 degrees apart, allowing the reading to settle at
each position before recording it. The quantity of interest is the spread between the
highest and lowest reading, not the absolute value.

As a rough guide, a well-behaved caesium or Overhauser sensor in a fixed mounting
shows a spread of a few nanotesla. A spread above roughly 10 nT normally means the
mount, the cabling, or an object near the sensor is contributing, and it should be
resolved before flying or driving with that installation.

- [ ] Eight azimuths taken at equal increments
- [ ] Reading given time to settle at each position
- [ ] All eight readings recorded, not just the extremes

## Record the azimuth table

```yaml step
id: heading-record
kind: note
severity: normal
```

Put the per-azimuth readings in the run record as a table. The shape of the curve
identifies the cause - a single offset lobe usually means a fixed ferrous object near
the sensor, while a smooth two-cycle variation is characteristic of the sensor itself.
The maximum spread alone cannot distinguish these.
