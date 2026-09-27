---
kind: procedure
procedure_id: magnetic-hygiene
title: Personnel and Equipment Magnetic Hygiene
version: 1
updated: 2026-09-24
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [hygiene, safety]
---

A magnetometer measures everything magnetic in its vicinity, including the operator.
Most anomalies that turn out to be "instrument problems" are actually carried items,
and they are far cheaper to prevent than to post-process out.

## Operator

```yaml step
id: hygiene-person
kind: check
severity: critical
```

Any ferrous object within roughly a metre of the sensor shows up as an anomaly. This
includes items that do not obviously contain iron.

- [ ] Watch removed
- [ ] Phone removed, or powered off and left behind
- [ ] Keys, coins, and loose metal removed from pockets
- [ ] Belt buckle checked; no steel-toed boots
- [ ] Steel-rimmed glasses or earphone cables checked
- [ ] Clothing checked for zips, studs, and metal buttons

## Equipment

```yaml step
id: hygiene-equipment
kind: check
severity: critical
```

- [ ] Sensor mounted on a non-magnetic tripod or staff (aluminium, fibreglass, or wood)
- [ ] No steel tools, clipboards, or cases within the measurement radius
- [ ] Cable routing checked: no loops that can flex during a line
- [ ] Reference marker stakes are non-magnetic

## Separation distances

```yaml step
id: hygiene-separation
kind: check
severity: critical
captures:
  - key: nearest_vehicle_m
    label: Distance to nearest vehicle
    type: number
    unit: m
    required: false
  - key: nearest_structure_m
    label: Distance to nearest structure or fence
    type: number
    unit: m
    required: false
```

Vehicles, generators, and steel structures are the dominant interference near a
survey area. Record the distances you actually achieved; they are needed later to
decide which lines are trustworthy.

- [ ] Vehicle parked well clear of the working area
- [ ] Generators and power equipment shut down or moved clear
- [ ] Fences, gates, and buried services noted

## Record the moving parts

```yaml step
id: hygiene-record
kind: note
severity: info
```

Write down where people, vehicles, and equipment were during the survey, with times.
These sources move, so they leave anomalies that are present on some lines and not
others. Having the position and time on record is what makes it possible to exclude
them instead of discarding the affected lines entirely.
