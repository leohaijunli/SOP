---
kind: help
help_id: troubleshooting
title: When validation fails
section: Troubleshooting
order: 70
audience: author
summary: The validator messages people actually hit, and what each one is asking for.
updated: 2026-09-24
tags: [validation]
---

Validation messages name the file, the line, and the problem. This page covers the ones
that come up repeatedly.

## `front matter is missing required key 'x'`

You are looking at the top of a file, between the two `---` lines. Every procedure and
checklist needs `title`, `version`, `updated`, and `applies_to`; every run record needs
`run_id`, `sop`, `sop_version`, `operator`, `site`, `started`, `status`, and
`deviations_count`.

`updated` is a date in `YYYY-MM-DD` form. Quote nothing, and do not write "today".

## `'procedure_id' 'x' must match filename 'y'`

The id in the front matter and the filename have to agree, because the filename is how
everything else finds the file. Rename one or the other, or change the id, and re-run.

## `include target does not exist: procedures/x.md`

A checklist includes a procedure that is not there. Either the filename is misspelled or
the file was never added to the commit. `git status` will tell you which.

## `step id 'x' from a.md collides with b.md`

Two included procedures define the same step id. Ids have to be unique across the whole
resolved checklist, not only within each file. Rename one of them - and remember that a
step id cited by an existing run record cannot be renamed without breaking that record.

## `step 'x': capture 'y' type 'z' is not one of ...`

The capture `type` is misspelled or is not a type this build knows. The valid set is
`text`, `number`, `integer`, `bool`, `select`, `datetime`, `duration`, `attach`. Guessing
is deliberately forbidden - an unknown type is an error rather than being treated as
`text`, because a typo would otherwise go unnoticed forever.

## `'expected' is valid only on ...`

`expected` says what the author expects a value to look like, for highlighting. It is
meaningful on `number`, `integer`, `select`, and `bool`, and nowhere else. For a range:

```yaml
expected:
  min: -1.0
  max: 1.0
```

For a selection, the expected value must be one of the capture's own `options`.

## `log 'x': sha256 in the record does not match the file`

The attached file was edited, replaced, or truncated after the record was written. Find
out which happened before you do anything else: a file that changed on its own is a much
more interesting problem than a stale hash. If the file is correct and the record is not,
regenerate the record's hash. If the file is wrong, restore it from git.

## `log directory 'x' has no matching run record in runs/`

There is a directory under `logs/` for a run that has no record. Usually a run was
started, logs were copied in, and the record was never committed. Either add the record
or remove the directory.

## `attached log does not exist: logs/x/y.csv`

The record cites a file that is not in the working copy. You may have forgotten
`git add`. Note that `git add` alone is not enough - the file has to be present on disk
for the hash to be checked.

## `run is 'complete' but step 'x' has no result`

A run marked `complete` must account for every step in the checklist. Either record the
missing step, or mark the run `partial`. `partial` is not a failure - an honest partial
run is worth more than a complete one that is not true.

## `file contains CR characters; use LF line endings`

An editor saved the file with Windows line endings. In most editors, change the line
ending to LF and save again. `.gitattributes` sets LF for Markdown, so this usually comes
from editing outside the repository.

## The app shows a checklist as `draft`

A `draft` checklist is still being written. Starting one is possible, and the override is
recorded as an event with a reason, so the resulting record says plainly that it was run
against unfinished content.

## Nothing above matches

Read the message as a sentence: it names the file, the line, and the thing that is wrong.
If the message is wrong or unclear, that is a bug in the validator and worth reporting -
a message that sends people in the wrong direction is worse than no check at all.
