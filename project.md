---
kind: project
project_id: p
title: T
institution: University of Victoria
lead: leo
updated: 2026-09-26
summary: Sensor calibration, ground walk surveys, interference and navigation tests, and UAV-mounted survey work.
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [campaign]
---

The campaign this repository serves: what it is measuring, and with what.

## Scope

Magnetic field surveying with a mix of sensors, sites, software, and platforms. The work
runs in stages, and each stage has its own checklist:

| Stage | Checklist | Status |
|---|---|---|
| Sensor calibration | `checklists/mag-sensor-calibration.md` | draft |
| Ground walk survey | `checklists/ground-walk-survey.md` | draft |
| Interference test | not written yet | planned |
| Navigation test | not written yet | planned |
| UAV-mounted survey | not written yet | planned |

## Why this file exists

The title here is the name the app shows, and the name a reader of a run record two
years from now will use to know which campaign the run belonged to. It is repository
content rather than an application setting on purpose: if it lived on each laptop, two
operators would disagree, and the disagreement would never be reviewed.

Edit it in place. Changing `project_id` is a rename, not a new project, so prefer not to.
