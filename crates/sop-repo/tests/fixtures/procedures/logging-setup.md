---
kind: procedure
procedure_id: logging-setup
title: Data Logging Setup and Write Test
version: 1
updated: 2026-09-24
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [logging, data]
---

Recording settings are the one part of a survey that cannot be reconstructed after
the fact. A wrong sample rate or a unit mismatch invalidates a session that otherwise
went perfectly.

## Configuration

```yaml step
id: log-config
kind: check
severity: critical
captures:
  - key: log_path
    label: Log file path on the recording device
    type: text
    required: true
  - key: sample_rate_hz
    label: Sample rate
    type: number
    unit: Hz
    required: true
  - key: units
    label: Recorded units
    type: select
    options: [nT, uT, mG, counts, other]
    required: true
  - key: timestamp_utc
    label: Timestamps are UTC
    type: bool
    expected: true
    required: true
```

Choose a sample rate that resolves the smallest feature the survey targets, not the
largest your storage allows. For a walking survey, 1 Hz is typical; a moving platform
needs more.

- [ ] Sample rate set for the target resolution
- [ ] Units confirmed against what the processing tools expect
- [ ] Timestamp mode checked, and it is UTC
- [ ] Log path recorded

## Write test

```yaml step
id: log-write-test
kind: check
severity: critical
captures:
  - key: test_file_bytes
    label: Size of the test file
    type: integer
    unit: B
    required: true
```

Start a recording, leave it for at least 30 seconds, stop it, and confirm the file
exists and is not empty. Do this on every session. The failure mode this prevents is
starting a two-hour survey and discovering at pack-down that the card was full or the
directory was not writable.

- [ ] Test recording started and stopped
- [ ] File confirmed present and non-empty

## Storage headroom

```yaml step
id: log-disk-space
kind: check
severity: normal
captures:
  - key: free_disk_gb
    label: Free space on the recording device
    type: number
    unit: GB
    required: true
```

Estimate from the test file size and the planned duration, then add a wide margin.

- [ ] Free space checked against a computed estimate
