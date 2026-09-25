---
kind: procedure
procedure_id: conditions-log
title: Session Location, Environment, and Reference Instrument
version: 1
updated: 2026-09-24
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [metadata, conditions]
---

The context a session was run in. None of it changes the measurement directly, and all
of it is needed to interpret the measurement later: results taken at 5 C are not
directly comparable with results taken at 30 C, and a reference comparison is only as
good as the reference.

## Location and scene

```yaml step
id: cond-location
kind: check
severity: normal
captures:
  - key: session_location
    label: Where the session took place
    type: text
    required: true
  - key: nearby_sources
    label: Known magnetic sources nearby
    type: text
    required: false
```

Name the location precisely enough to return to it, and note anything magnetic in the
vicinity that is not part of the experiment.

- [ ] Location described well enough to find again
- [ ] Nearby sources noted, or explicitly recorded as none known

## Environment

```yaml step
id: cond-environment
kind: check
severity: normal
captures:
  - key: ambient_c
    label: Ambient temperature
    type: number
    unit: C
    required: true
  - key: wind_mps
    label: Wind speed
    type: number
    unit: m/s
    required: false
  - key: humidity_pct
    label: Relative humidity
    type: number
    unit: '%'
    required: false
```

Temperature drives sensor electronics drift. Wind matters more than it appears to in
a walking survey: a staff that moves in the wind modulates the reading, and the
resulting oscillation looks like instrument noise.

- [ ] Temperature recorded
- [ ] Wind recorded, since it affects how steadily the sensor can be held

## Reference instrument

```yaml step
id: cond-reference-instrument
kind: check
severity: normal
captures:
  - key: reference_instrument
    label: Reference instrument, if used
    type: text
    required: false
  - key: reference_last_calibrated
    label: Reference instrument last calibrated
    type: text
    required: false
```

Any absolute comparison is made against something. If that something is a second
instrument, its own calibration currency becomes part of the result; a comparison
against an uncalibrated reference propagates the reference's error into the sensor
under test.

- [ ] Reference instrument's calibration currency confirmed, or its absence recorded
