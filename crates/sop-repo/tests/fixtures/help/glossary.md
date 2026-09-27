---
kind: help
help_id: glossary
title: Field and format vocabulary
section: PX4 Operations
order: 80
audience: operator
summary: The terms that appear in procedures, checklists, and validation messages.
updated: 2026-09-24
tags: [reference]
---

## Magnetometry

**Total field** - the magnitude of the magnetic field vector, measured in nanotesla (nT),
without direction. What a scalar magnetometer such as a proton precession or Overhauser
instrument reports.

**Base station** - a magnetometer left recording at one fixed place for the whole survey.
Its record is used to remove the diurnal variation that the walking instrument also saw.

**Diurnal variation** - the natural daily drift of the field, from ionospheric currents,
typically tens of nT over a day. Removed by subtracting the base station record.

**Heading error** - the change in a magnetometer's reading caused by rotating the sensor
about the vertical axis. Caused by the sensor not being perfectly aligned, and by
magnetic material near it. Measured by a heading sweep, quoted as a peak-to-peak spread.

**Lever arm** - the distance between the sensor and anything that moves relative to it -
the GNSS antenna, a backpack, the operator. Its effect grows as the distance shrinks, and
it is why the sensor is carried on a staff rather than a belt.

**Noise floor** - the spread of readings when nothing is moving and nothing is changing.
Sets the smallest anomaly that can be believed.

**Anomaly** - a local deviation from the expected field, and the thing a survey is
usually looking for.

**Magnetic hygiene** - keeping ferrous and magnetised objects away from the sensor.
Watches, phones, keys, steel-toed boots, belt buckles, and vehicle bodies all count.

**Static noise test** - recording with the sensor stationary for a fixed window, to
measure the noise floor before a survey. A precondition for trusting anything that
follows.

## Format

**Procedure** - a reusable block of steps. `procedures/*.md`.

**Checklist** - one experiment type. Includes procedures and adds local steps.
`checklists/*.md`.

**Run record** - the record of one execution of one checklist. `runs/<sop_id>/<run_id>.md`.

**Step** - a `##` heading plus a `yaml step` block plus prose.

**Step id** - the permanent identifier in a step's `yaml step` block. Run records cite
it, so it is never renamed and never reused.

**Capture** - one value recorded during a step: a number, a choice, a file.

**`expected`** - the range or value the author expects for a capture. Presentation only,
never a pass or a fail.

**Include marker** - a comment such as `<!-- include: procedures/warm-up.md -->` that
places a procedure's steps into a checklist at that position in the document.

**Snapshot** - the copy of the checklist as it was when a run started. It is what makes a
record interpretable after the checklist has changed underneath it.

**Event log** - `events.jsonl`, the append-only record of what the operator did during a
run. The run record is rendered from it.

**Deviation** - a step carried out differently from the procedure. Always carries a
reason.

**`runs/_inbox/`** - observations that are not yet proven and not yet part of any
procedure. Somewhere for "we should add this to the SOP" to live.

**`applies_to`** - which experiment types a procedure or checklist is written for:
`calibration`, `ground-survey`, `interference`, `navigation`, `uav-survey`.
