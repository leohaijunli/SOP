---
kind: procedure
procedure_id: pack-down
title: Pack-Down, Log Backup, and Record Close-Out
version: 1
updated: 2026-09-24
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [teardown, backup, records]
---

The session is not finished when the last line is walked. An unbacked-up log and an
unfinished record are the two ways a successful survey turns into a lost one.

## Stop and flush

```yaml step
id: packdown-stop
kind: check
severity: critical
```

Let the instrument finish writing before removing power or the storage card. Cutting
power mid-write is the usual cause of a truncated final log.

- [ ] Recording stopped through the instrument's normal procedure
- [ ] Instrument powered down cleanly
- [ ] Storage unmounted or card ejected properly

## Back up to two locations

```yaml step
id: packdown-backup
kind: check
severity: critical
captures:
  - key: backup_primary
    label: Primary backup location
    type: text
    required: true
  - key: backup_secondary
    label: Secondary backup location
    type: text
    required: true
```

Two independent copies, on different devices. A single copy on the instrument's
internal storage does not count; neither does a copy that only exists on the machine
you are about to carry across a field site.

- [ ] Primary copy confirmed readable
- [ ] Secondary copy on a different device confirmed readable

## Attach logs to the run record

```yaml step
id: packdown-attach
kind: check
severity: critical
captures:
  - key: logs_attached
    label: Number of log files attached
    type: integer
    required: true
  - key: hash_mismatch
    label: Any hash mismatch reported by the app
    type: bool
    required: true
```

Attach the logs in the app so they are copied under `logs/<run_id>/` and hashed into
the record. A hash mismatch means the file changed between attach and verification;
stop and find out why before the record is committed.

- [ ] Every log the run produced is attached
- [ ] Base-station and GNSS logs attached alongside the survey log
- [ ] No hash mismatches, or each one explained in the notes

## Close out

```yaml step
id: packdown-closeout
kind: note
severity: normal
captures:
  - key: run_status
    label: Run status
    type: select
    options: [complete, partial, aborted]
    required: true
```

Fill in every step result, record each deviation with its reason, then commit the run
record and its logs together. Keep the record and the data in the same commit so
history never holds one without the other.

Before committing, move anything reusable and not yet proven into `runs/_inbox/`.
That is where "we should add this to the procedure" goes, while it is still fresh.

- [ ] All step results complete
- [ ] Deviations written up with reasons
- [ ] Candidate procedure improvements copied to `runs/_inbox/`
- [ ] Run record and logs committed together
