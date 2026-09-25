---
kind: help
help_id: overview
title: What this tool is for
section: Getting started
order: 10
audience: operator
summary: Why the procedures live in files, and what the app is and is not allowed to decide.
updated: 2026-09-24
tags: [concepts]
---

A geomagnetic survey produces two things that have to survive the campaign: the data, and
the evidence that the data was collected the way it was supposed to be. The second one is
what this tool keeps.

## The three documents

| Document | Answers | Example |
|---|---|---|
| Procedure | "How is this done?" | `procedures/base-station.md` |
| Checklist | "What does this experiment consist of, in what order?" | `checklists/ground-walk-survey.md` |
| Run record | "What actually happened on the day?" | `runs/ground-walk-survey/2026-09-24-renfrew-walk01.md` |

A checklist does not copy the text of a procedure. It references it, so correcting one
procedure corrects every checklist that uses it. That is the whole reason the split
exists: the experiment matrix is experiment type x sensor x site x software x hardware,
and copying text across dozens of documents guarantees that they drift apart.

## What the app will not do

- **It never decides whether a measurement passed.** A capture may carry an `expected`
  range. That range is highlighted for the operator and acknowledged by them; it is
  never turned into a verdict. The criterion belongs to the author of the procedure, and
  the judgement belongs to the person standing in the field. See `docs/DESIGN.md`
  section 6.4.
- **It never syncs anything.** It edits a local git working copy. `git` does the rest.
  There are no tokens, no accounts, and no network calls in the app.
- **It never renumbers a step.** Step ids are authored, permanent, and cited by run
  records. Retiring a step means `deprecated: true`, never reuse of an id.

## Why the content is Markdown in a repository

Everything that a person might want to change - the procedures, the checklists, the help
pages you are reading - is a plain Markdown file. It diffs in review, it works offline,
and it can be edited in any text editor when the app is not the right tool for the job.

This help page is itself one of those files: `help/overview.md`.
