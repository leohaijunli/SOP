---
kind: procedure
procedure_id: warm-up
title: Sensor Warm-Up and Drift Check
version: 1
updated: 2026-09-24
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [startup, drift]
---

Sensor output drifts as the electronics reach thermal equilibrium. Starting a
calibration or a survey before that settles puts a slow ramp into the data that is
easy to mistake for a real gradient.

## Thermal soak

```yaml step
id: warmup-thermal
kind: check
severity: critical
captures:
  - key: warmup_min
    label: Actual warm-up time
    type: number
    unit: min
    required: true
  - key: ambient_c
    label: Ambient temperature
    type: number
    unit: C
    required: false
```

Power the sensor on and let it run undisturbed for the interval the manufacturer
specifies, normally 10 to 30 minutes for a total-field instrument. Time this on a
clock rather than estimating it. Operating near the published temperature limits
warrants a longer soak, not the same one.

- [ ] Sensor powered on and left undisturbed for the full interval
- [ ] Interval timed, not estimated
- [ ] Ambient temperature recorded

## Drift check

```yaml step
id: warmup-stability
kind: measure
severity: critical
captures:
  - key: drift_nt_per_min
    label: Residual drift after warm-up
    type: number
    unit: nT/min
    expected:
      min: -1.0
      max: 1.0
    required: true
  - key: stability_min
    label: Observation window
    type: number
    unit: min
    required: true
```

With the sensor stationary in a quiet location, watch the reading for several minutes.
After warm-up the residual trend should be small compared with the anomalies you are
chasing; for a walking survey that means well under 1 nT/min.

A trend that does not settle, or that reverses sign, points at the environment rather
than the sensor. Rule out moving traffic and nearby equipment before condemning the
instrument.

- [ ] Stationary reading observed for the full window
- [ ] Residual drift recorded
