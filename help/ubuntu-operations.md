---
kind: help
help_id: ubuntu-operations
title: Ubuntu Operations
section: Ubuntu Operations
order: 20
audience: operator
updated: 2026-09-29
summary: Desktop, terminal, and data-handling procedures for the Ubuntu field laptop.
---

Help and reference for the Ubuntu field laptop that runs field-sop.

## Setting up

- Clone the repository and confirm it validates before recording anything.
- Check which working copy the app is using on the Settings page.
- Keep the git remote set so a finished run can be pushed before leaving the field.

## Terminal basics

- `sop index` rebuilds the manifest the app reads.
- `sop status` shows the working copy, the project, and the content counts.
- `sop run export <sop> --format markdown --run <run_id>` writes one run record as
  markdown.
- `sop run attach <sop> <run_id> --kind log|photo|file <file>` copies a file into the
  run's `logs/`, `photos/`, or `attachments/` directory.
- `sop export --out <dir>` packages a summary and every run into one folder, the same
  thing the app's **Export summary** does.

## Handling data

- Copy logs to the field laptop with the run id before the transfer can be lost. The app
  files logs under `logs/`, photographs under `photos/`, and anything else under
  `attachments/`, all inside the run's own directory. A file can be added after the run
  has ended; the record lists what arrived late without moving the seal over the field
  record.
- Photographs are the largest thing a run collects. Text logs are fine to commit; commit
  photographs only with Git LFS set up (`git lfs track "runs/**/photos/**"`).
- Checksum raw data before leaving the site.
- Before leaving the field, **Export summary** to a folder on the laptop or a USB stick:
  it holds the summary plus every run's record and material, so the day's work is in one
  place even if the push cannot happen until later.

See the individual pages under this category for the step-by-step details.
