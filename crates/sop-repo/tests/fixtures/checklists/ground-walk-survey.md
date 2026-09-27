---
kind: checklist
sop_id: ground-walk-survey
title: Ground Magnetic Walk Survey
version: 1
updated: 2026-09-24
status: draft
applies_to: [ground-survey]
equipment:
  - Total-field magnetometer with 1 Hz or better logging
  - Second magnetometer for base station, or a spare channel
  - Non-magnetic staff or harness at fixed sensor height
  - GNSS receiver with RTK correction
  - Non-magnetic stakes and marking tape
  - Site sketch and line list
---

A hand-carried magnetic survey over a defined grid, with a base station running
throughout to correct for diurnal variation.

One operator drives this checklist. A team may be larger, and several runs may be in
progress on the same day; the constraint is that one person is responsible for this
record and this dataset.

**Before starting:** the sensor must have a current calibration record. Run
`mag-sensor-calibration` first if it does not.

<!-- include: procedures/conditions-log.md -->

## Team briefing

```yaml step
id: walk-briefing
kind: check
severity: normal
captures:
  - key: team_size
    label: Number of people on site
    type: integer
    required: true
  - key: roles
    label: Roles assigned
    type: text
    required: false
```

Brief everyone before the first line, including anyone who is not operating an
instrument. People who do not know that a vehicle is a magnetic source will park it
next to the grid, and the resulting anomaly will look geological.

- [ ] Everyone briefed on magnetic hygiene, including non-operators
- [ ] Operator, base-station owner, and GNSS owner identified
- [ ] Break schedule agreed, so the mid-survey check happens on time

## Line plan for the session

```yaml step
id: walk-line-plan
kind: check
severity: critical
captures:
  - key: lines_planned
    label: Lines planned for this session
    type: integer
    required: true
  - key: lines_completed
    label: Lines completed
    type: integer
    required: false
  - key: session_goal
    label: Session goal
    type: text
    required: false
```

State the lines intended for this session before starting, and the order they will be
walked. Partial sessions are normal; an explicit plan is what makes it possible to
resume cleanly on a later visit instead of re-walking ground already covered.

- [ ] Line list and order written down before the first line
- [ ] Resume point clear for a session that ends early

<!-- include: procedures/power-on.md -->

<!-- include: procedures/magnetic-hygiene.md -->

<!-- include: procedures/site-recon.md -->

<!-- include: procedures/base-station.md -->

<!-- include: procedures/gnss-setup.md -->

<!-- include: procedures/warm-up.md -->

<!-- include: procedures/logging-setup.md -->

<!-- include: procedures/test-line.md -->

<!-- include: procedures/walk-line.md -->

<!-- include: procedures/mid-survey-check.md -->

<!-- include: procedures/pack-down.md -->

## Session close-out

```yaml step
id: walk-session-closeout
kind: note
severity: normal
```

Summarise the day before committing: lines completed, lines skipped and why, whether
the test line repeated acceptably, and anything that should change in this checklist
next time.

Then move anything reusable into `runs/_inbox/`, where it can be promoted into a
procedure through a pull request.

- [ ] Lines completed recorded against `walk-line-plan`
- [ ] Skipped lines and the reason recorded
- [ ] Candidate procedure improvements copied to `runs/_inbox/`

## Sensor and staff check

```yaml step
id: walk-sensor-check
kind: measure
severity: critical
captures:
  - key: staff_height
    label: Sensor height above ground
    type: number
    unit: m
    expected:
      min: 0.5
      max: 2.0
    required: true
  - key: level_bubble
    label: Level bubble centred
    type: bool
    expected: true
    required: true
  - key: sensor_choice
    label: Sensor in use
    type: select
    options: [cal01, cal02, cal03]
    required: true
```

A sensor held at a different height than the one it was calibrated at produces a height
error that looks like a gradient, so this is checked before every line.

- [ ] Bubble centred
- [ ] Height recorded against the line

<!-- include: procedures/absolute-value-check.md -->
