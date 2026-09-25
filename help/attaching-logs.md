---
kind: help
help_id: attaching-logs
title: Attaching logs and photos
section: Using the app
order: 40
audience: operator
summary: How files are attached to a run, why they are hashed, and where they end up.
updated: 2026-09-24
tags: [logs, attachments]
---

## What counts as a log

Any file that a run produced or that a run depends on:

- magnetometer output, base station output, GNSS output
- a notes file exported from an instrument
- a photograph of the setup, the site, or an anomaly
- a configuration dump from the software that was driving the sensor

Files are attached **manually**. Nothing is discovered from a directory on your behalf,
because the point of the attachment list is that a person decided each entry belongs to
this run.

## How to attach

1. Attach at the run level for files that belong to the whole session.
2. Attach at the step level when a file is evidence for one step - a photo of the sensor
   orientation, for instance.
3. Write a `description` for each one. "Static noise window, 300 s at 1 Hz" is useful in
   two years; `run1.csv` is not.

## What the app records

Attaching a file copies it into the run's log directory and records its size and its
SHA-256 digest in the run record:

```yaml
logs:
  - path: logs/2026-09-22-bench-cal01/mag_static.csv
    sha256: 0793944de2a64aa1cc4bf7a972b9eed22a587d381040290d1c1e6dde1da957db
    size: 9323
    description: Static noise window, 300 s at 1 Hz
```

The digest is not decoration. `sop validate` recomputes it and fails if the file changed
after the record was written. That is what turns "the data is in the repository" into
"the data in the repository is the data from that run".

Keep the digest quoted in the YAML. An unquoted digest that starts with a zero would lose
it, and a changed hash is worse than no hash.

## Where the file ends up

```text
logs/<run_id>/<filename>
```

One directory per run, matching `runs/<sop_id>/<run_id>.md`. `sop validate` reports a log
directory that has no matching run record, and a run record that cites a file which is
not there.

## Large files

Attach the processed or decimated product, not the raw 50 Hz stream, unless the raw
stream is itself the evidence. Raw streams that must be kept belong in git LFS or in the
project data store; see `docs/LOGS.md` for the policy and the size thresholds.

## If a file must stay outside the repository

Mark the entry `external: true` and give it the correct `sha256`. The validator will
check the digest but not require the file to be present. Use this deliberately - an
external file that nobody can find is not evidence.
