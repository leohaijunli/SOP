---
kind: procedure
procedure_id: base-station
title: Base Station Setup for Diurnal Monitoring
version: 1
updated: 2026-09-24
applies_to: [ground-survey, uav-survey, interference, navigation]
tags: [diurnal, base-station]
---

The Earth's field varies continuously through the day. A base station logging
throughout the survey is what allows that variation to be removed; without it, a
morning-to-afternoon drift is indistinguishable from a real spatial gradient along
the survey lines.

## Site selection

```yaml step
id: base-site
kind: check
severity: critical
captures:
  - key: base_distance_to_road_m
    label: Distance to nearest road or track
    type: number
    unit: m
    required: true
  - key: base_lat_deg
    label: Latitude
    type: number
    required: true
  - key: base_lon_deg
    label: Longitude
    type: number
    required: true
```

The base station must be magnetically quiet and stay quiet: away from roads, tracks,
footpaths, gates, and parked vehicles. It measures the diurnal variation only if
nothing else changes near it.

Record the position properly. If the base log has to be re-processed later, the
coordinates are what connect it to the survey area.

- [ ] Location chosen away from traffic and footpaths
- [ ] Scene checked for sources that will move during the day
- [ ] Position recorded from GNSS

## Fixed installation

```yaml step
id: base-install
kind: check
severity: critical
captures:
  - key: sensor_height_m
    label: Sensor height above ground
    type: number
    unit: m
    required: true
```

Install on a rigid non-magnetic stand at the same height used by the moving sensor.
The base station is a reference, so its geometry should match the survey where
practical, and in any case should be recorded.

Secure everything against wind. A sensor that changes orientation during the day adds
a heading error to the diurnal record, which is worse than no base station at all,
because it looks plausible.

- [ ] Non-magnetic stand, rigid, wind-secure
- [ ] Sensor height measured and recorded
- [ ] Cables secured so they cannot move

## Start logging before the first line

```yaml step
id: base-logging
kind: check
severity: critical
captures:
  - key: base_sample_rate_hz
    label: Base sample rate
    type: number
    unit: Hz
    required: true
  - key: base_start_time
    label: Base logging start time (UTC)
    type: datetime
    required: true
```

Start the base station before any survey line is walked, and leave it running until
after the last one. A base log that starts late leaves the first lines uncorrectable.

One hertz is the usual choice; some instruments will do better. It must at minimum
resolve the diurnal variation, which is slow.

- [ ] Base logging started before the first survey line
- [ ] Start time recorded in UTC
- [ ] Sample rate recorded

## Stability check

```yaml step
id: base-verify
kind: measure
severity: critical
captures:
  - key: first_min_sigma_nt
    label: Standard deviation over the first minutes
    type: number
    unit: nT
    required: true
```

Watch the first several minutes. A stable low-noise trace means the base station is
doing its job; drift, spikes, or a wandering baseline means the location is not quiet
enough and should be moved before the survey starts.

- [ ] First minutes observed before starting the survey
- [ ] Stability figure recorded
- [ ] Any spikes identified and their cause determined
