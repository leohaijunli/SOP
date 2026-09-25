---
kind: procedure
procedure_id: mid-survey-check
title: Mid-Survey Check
version: 1
updated: 2026-09-24
applies_to: [ground-survey, uav-survey, interference, navigation]
tags: [qc, monitoring]
---

Performed periodically through a session, and always before packing up. Its purpose is
to catch slow failures - a base station that stopped logging, a GNSS that dropped to
float, a pack that is draining faster than expected - while there is still time to do
something about them.

Suggested interval: every two hours, and after any long break.

## Re-occupy a reference station

```yaml step
id: midcheck-reference
kind: measure
severity: critical
captures:
  - key: reference_station_id
    label: Reference station identifier
    type: text
    required: true
  - key: reference_value_nt
    label: Reading at the reference station
    type: number
    unit: nT
    required: true
  - key: reference_time
    label: Time of the reading (UTC)
    type: datetime
    required: true
```

Return to the same marked station and take a reading in the same way as before. The
change since the previous occupation is the quantity of interest; it should track the
base station's own variation.

Take the reading the same way each time. A different height or orientation at the
reference station introduces a change that is not in the base record and cannot be
corrected.

- [ ] Same station re-occupied, same height and orientation as before
- [ ] Reading and UTC time recorded
- [ ] Change since the previous occupation noted

## Base station health

```yaml step
id: midcheck-base
kind: check
severity: critical
captures:
  - key: base_gap_count
    label: Recording gaps observed
    type: integer
    required: true
  - key: base_still_running
    label: Base station still logging
    type: bool
    expected: true
    required: true
```

Gaps in a base record are the most common cause of an uncorrectable survey, and they
are almost always invisible until processing. Check the file is still growing rather
than only checking the instrument is powered on.

- [ ] Base file confirmed still growing
- [ ] Gap count checked as far as the instrument allows mid-session
- [ ] Any gap noted with its time, so the affected survey lines can be identified

## Power and storage

```yaml step
id: midcheck-power
kind: check
severity: normal
captures:
  - key: battery_v
    label: Remaining supply voltage
    type: number
    unit: V
    required: false
  - key: free_disk_gb
    label: Remaining free space
    type: number
    unit: GB
    required: false
```

Compare against the consumption so far and project it to the end of the planned
session. It is much cheaper to swap a pack at a scheduled break than to lose the last
hour of a survey.

- [ ] Remaining power projected to end of session
- [ ] Free space projected to end of session

## GNSS quality

```yaml step
id: midcheck-gnss
kind: check
severity: critical
captures:
  - key: fix_type
    label: Current fix type
    type: select
    options: [rtk-fixed, rtk-float, dgps, single, other]
    expected: rtk-fixed
    required: true
```

A fix that has degraded quietly is worth catching here. Note when it degraded, because
every line walked during the degraded interval has to be treated as lower precision.

- [ ] Fix type checked and recorded
- [ ] Degradation, if any, noted with the time it started
- [ ] Affected lines identified by identifier
