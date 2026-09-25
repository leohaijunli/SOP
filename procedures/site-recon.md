---
kind: procedure
procedure_id: site-recon
title: Site Reconnaissance and Grid Design
version: 1
updated: 2026-09-24
applies_to: [ground-survey, interference, navigation]
tags: [planning, access]
---

Half an hour of reconnaissance prevents the most expensive failure mode in a walking
survey: walking a clean grid over ground whose magnetic signal is dominated by
something you did not notice.

## Access and permissions

```yaml step
id: recon-access
kind: check
severity: normal
captures:
  - key: landowner_contact
    label: Landowner or site contact
    type: text
    required: false
  - key: access_notes
    label: Access notes
    type: text
    required: false
```

Record who gave permission and how to reach them. A survey that is interrupted
part-way by an access question is rarely resumable on the same day.

- [ ] Permission confirmed for the area and the time window
- [ ] Contact details recorded for the session

## Hazards and buried services

```yaml step
id: recon-hazards
kind: check
severity: critical
captures:
  - key: hazards
    label: Hazards identified
    type: text
    required: true
```

Walk the area and note power lines, pipelines, fences, gates, buried services,
steep ground, and water. These are both a safety matter and a data matter: a buried
steel pipe will dominate every line that crosses it.

Locate buried services before driving stakes anywhere.

- [ ] Overhead and buried services identified and marked on the site sketch
- [ ] Terrain hazards noted
- [ ] Safety word for the session agreed, if working alone

## Magnetic cleanliness walk

```yaml step
id: recon-cleanliness
kind: measure
severity: critical
captures:
  - key: worst_gradient_nt
    label: Worst gradient seen during reconnaissance
    type: number
    unit: nT
    required: true
  - key: worst_gradient_location
    label: Where the worst gradient was
    type: text
    required: true
```

Walk a loop around the intended area with the instrument running. On flat ground the
field should vary smoothly; isolated large jumps mean a discrete source - a fence post,
a buried drum, a piece of machinery.

Mark the locations of the worst areas. Lines planned through them can still be
surveyed, but they will need to be excluded or masked in processing, and it is far
easier to record that now than to work it out from the data afterwards.

- [ ] Loop walked with the instrument recording
- [ ] Discrete sources located and marked on the site sketch
- [ ] Worst gradient and its location recorded

## Grid design

```yaml step
id: recon-grid
kind: measure
severity: critical
captures:
  - key: line_spacing_m
    label: Planned line spacing
    type: number
    unit: m
    required: true
  - key: station_spacing_m
    label: Planned station spacing
    type: number
    unit: m
    required: true
  - key: line_azimuth_deg
    label: Survey line azimuth
    type: number
    unit: deg
    required: true
  - key: tie_line_spacing_m
    label: Tie line spacing
    type: number
    unit: m
    required: false
```

Line spacing sets the smallest target that can be resolved, so choose it from the
target, not from how much time you have. Station spacing along the line follows from
the same reasoning.

Run lines perpendicular to the strike of the features of interest. Add tie lines
perpendicular to the main lines: they are what makes levelling between lines possible,
and they cannot be added afterwards.

- [ ] Spacing chosen from the target size
- [ ] Line azimuth set perpendicular to the features of interest
- [ ] Tie lines planned and their spacing recorded
- [ ] Grid marked on a site sketch with north indicated

## Baseline and reference stations

```yaml step
id: recon-reference
kind: check
severity: normal
captures:
  - key: reference_station_id
    label: Reference station identifier
    type: text
    required: true
```

Choose a small number of permanently identifiable points to re-occupy during and
after the survey. Usable ones are a nail in a post, a marked rock, or a painted stake -
something that will still be findable next month. Record coordinates with GNSS before
leaving.

- [ ] Reference station marked physically
- [ ] GNSS coordinates recorded for each reference station
- [ ] Station described well enough for someone else to find it
