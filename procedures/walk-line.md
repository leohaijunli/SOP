---
kind: procedure
procedure_id: walk-line
title: Walking a Survey Line
version: 1
updated: 2026-09-24
applies_to: [ground-survey]
tags: [acquisition, technique]
---

The acquisition technique is a data quality parameter, not a matter of style. Most
noise in a hand-carried magnetic survey comes from the operator's own variation in
pace, height, and azimuth, not from the instrument.

## Pace

```yaml step
id: walk-pace
kind: measure
severity: critical
captures:
  - key: target_pace_mps
    label: Target pace
    type: number
    unit: m/s
    required: true
  - key: observed_pace_mps
    label: Observed pace on a test line
    type: number
    unit: m/s
    required: false
```

Walk at a constant pace, slow enough that the sample spacing along the line matches
the planned station spacing. For a 1 Hz instrument at 0.5 m station spacing, that is
0.5 m/s - which is much slower than normal walking. Measure it once on a test line
rather than guessing.

Starting and stopping within a line is worse than a slightly wrong constant speed. If
you must stop, note the position; a stationary sensor records the same spot many times
and produces an artificial spike in any interpolated grid.

- [ ] Pace practised on a test line and measured
- [ ] Sample spacing on the line checked against the plan
- [ ] No stopping except to note a hazard or a problem

## Sensor height

```yaml step
id: walk-height
kind: measure
severity: critical
captures:
  - key: sensor_height_m
    label: Sensor height above ground
    type: number
    unit: m
    required: true
```

Hold the sensor at a constant height above the ground surface. Small variations
directly modulate the reading, because the field falls off quickly with distance from
a near-surface source. On rough ground, use a fixed-length staff or harness rather
than trying to hold the sensor by hand.

- [ ] Height held by a fixed-length staff or harness, not by hand
- [ ] Height measured and recorded
- [ ] Height kept constant over uneven ground

## Azimuth

```yaml step
id: walk-direction
kind: check
severity: critical
captures:
  - key: line_azimuth_deg
    label: Line azimuth as walked
    type: number
    unit: deg
    required: true
```

Keep the sensor orientation constant along the line, and walk each line in the same
direction. Walking alternate lines in opposite directions turns any heading error into
a line-to-line pattern that looks exactly like a geological fabric.

- [ ] Sensor orientation constant along the line
- [ ] All lines walked in the same direction
- [ ] Azimuth checked against the planned value

## Off-line departure

```yaml step
id: walk-offline
kind: check
severity: critical
```

At the end of a line, turn off the line and continue walking away before returning to
the start of the next one. Walking back along the line you just measured adds a second
pass at a different speed and height into the same dataset.

Do not walk back along a line that has not yet been measured, either - the operator
carries a magnetic signature of their own, and it is recorded.

- [ ] Departed the line end laterally before returning
- [ ] Return path clear of unmeasured lines
- [ ] Line end waypoint marked

## Per-line record

```yaml step
id: walk-log
kind: check
severity: critical
captures:
  - key: line_id
    label: Line identifier
    type: text
    required: true
  - key: start_station
    label: Start station
    type: text
    required: true
  - key: end_station
    label: End station
    type: text
    required: true
```

Record every line as it is completed, with its identifiers and any deviation, rather
than reconstructing the day's work at pack-down. Where a line was cut short, say so
and say why: a partial line that is not marked as partial will be interpolated across
as if it were complete.

- [ ] Line identifier, start, and end recorded for each line
- [ ] Partial lines marked as partial with the reason
