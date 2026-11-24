---
kind: help
help_id: ubuntu-operations
title: Ubuntu Operations
section: Ubuntu Operations
order: 20
audience: operator
updated: 2026-11-24
summary: Desktop, terminal, and data-handling procedures for the Ubuntu field laptop.
---

Help and reference for the Ubuntu field laptop that runs field-sop.

## Setting up

- Select the data directory containing logs and test plans. A plain folder is sufficient.
- Check the working directory shown at the top of the window before recording.
- No sign-in, git remote, pull, commit, or push is required or performed by the desktop app.

## Terminal basics

- `sop index` rebuilds the manifest the app reads.
- `sop status` shows the working copy, the project, and the content counts.
- `sop run export <sop> --format markdown --run <run_id>` writes one run record as
  markdown.
- `sop run attach <sop> <run_id> --kind log|photo|file <file>` copies a file into the
  run's `logs/`, `photos/`, or `attachments/` directory.
- `sop export --out <dir>` packages a summary and every run into one folder, the same
  thing the app's **Export working directory** does.

## Handling data

- Copy logs to the field laptop with the run id before the transfer can be lost. The app
  files logs under `logs/`, photographs under `photos/`, and anything else under
  `attachments/`, all inside the run's own directory. A file can be added after the run
  has ended; the record lists what arrived late without moving the seal over the field
  record.
- Files in the working directory's `logs/` can be managed independently of a run.
- Use per-file remarks and tags for context; keep `.field-sop/log-remarks.json` with loose logs when copying by hand.
- Checksum raw data before leaving the site.
- Before leaving the field, **Export working directory** to a folder on the laptop or a USB stick:
  it holds the summary plus every run's record and material, so the day's work is in one
  place without depending on a network or repository synchronization.

See the individual pages under this category for the step-by-step details.
