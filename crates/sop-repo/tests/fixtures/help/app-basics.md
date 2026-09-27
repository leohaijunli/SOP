---
kind: help
help_id: app-basics
title: Running a checklist in the app
section: PX4 Operations
order: 30
audience: operator
summary: The main screen, how a step is recorded, and how to get through a session without a mouse.
updated: 2026-09-24
tags: [app, keyboard]
---

## The layout

Three columns, left to right:

1. **Step list** - every step in the checklist, in execution order, with its severity.
   The current step is highlighted; steps already recorded are marked.
2. **Step detail** - the instruction as written: prose, tables, warnings, and checklist
   items, followed by the captures for that step.
3. **Help panel** - these pages. Toggle it with `F1`. It remembers where you were.

## Recording a step

A step ends in one of three states, and the state is always explicit:

| State | Means | Requires |
|---|---|---|
| `done` | Carried out as written | the required captures |
| `skipped` | Deliberately not carried out | a reason, always |
| `deviated` | Carried out differently | a reason, always |

The reason is not bureaucracy. It is the only part of a run record that a reader in two
years cannot reconstruct from the data, and it is what makes the record trustworthy.

## Captures

Each capture is typed, and the type decides the widget:

- `text` - free text
- `number`, `integer` - numeric entry, with the unit shown next to it
- `bool` - a two-state control
- `select` - a fixed list of options from the procedure
- `datetime`, `duration` - date/time and elapsed time
- `attach` - a file that belongs to this step

If a procedure declares an `expected` range or value, the widget shows it. Values outside
the range are highlighted and you will be asked to acknowledge them. **Nothing is
recorded as a pass or a fail** - the acknowledgement is the record.

## Keyboard

Field use is one-handed and often gloved, so every action has a key.

| Key | Action |
|---|---|
| `F1` | Show or hide the help panel |
| `j` / `k` | Next / previous step |
| `space` | Mark the current step `done` |
| `s` | Mark `skipped` and ask for a reason |
| `d` | Mark `deviated` and ask for a reason |
| `n` | Add a note to the current step |
| `enter` | Move to the next capture; from the last one, complete the step |
| `ctrl` + `enter` | Finish the run |
| `/` | Search the help panel |

The single-letter keys are read only when the cursor is not in a text field, so typing
`s` into a note types an `s` rather than skipping the step. `ctrl` + `enter` works from
inside a field, because that is where your hands usually are when you finish.

## Notes

Every step takes free-text notes, and the run as a whole takes them too. Notes are
timestamped and kept alongside the structured captures. When you find yourself writing
the same note for the third campaign in a row, it belongs in a procedure - see
`help/authoring.md`.

## History: exporting and publishing

The History view lists the runs of the current checklist. Two buttons there act on the
repository rather than on one run:

- **Export record**, on a row, writes that run's record to `exports/<sop_id>-<run_id>.md`
  inside the working copy. It is the same document `sop run end` committed and
  `sop run export --format markdown` prints: self-contained, so it reads as the
  experiment record with no repository beside it. `exports/` is ignored by `git`, because
  the record is already committed under `runs/`.
- **Push repo** publishes the whole working copy: `git add -A`, then a commit, then a
  push to the configured remote. It asks for a commit message first, and a blank one
  takes a generated default.

Publishing is done by `git`, with your identity and your credentials. The app stores no
credential and runs `git` with `GIT_TERMINAL_PROMPT=0`, so a missing credential is an
error it can show you rather than a prompt it cannot answer. A commit that succeeds and
a push that fails leaves the commit in place, and the message says so.

## When the app closes unexpectedly

Reopen it. The run log is written as each action happens, so the session reopens in its
last consistent state. Nothing you had already recorded is lost, and nothing is
invented.
