---
kind: help
help_id: app-basics
title: Running a checklist in the app
section: Using the app
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

## Notes

Every step takes free-text notes, and the run as a whole takes them too. Notes are
timestamped and kept alongside the structured captures. When you find yourself writing
the same note for the third campaign in a row, it belongs in a procedure - see
`help/authoring.md`.

## When the app closes unexpectedly

Reopen it. The run log is written as each action happens, so the session reopens in its
last consistent state. Nothing you had already recorded is lost, and nothing is
invented.
