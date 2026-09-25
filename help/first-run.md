---
kind: help
help_id: first-run
title: Your first fifteen minutes
section: Getting started
order: 20
audience: operator
summary: Get the repository, check it, and know where your run will be written.
updated: 2026-09-24
tags: [setup]
---

## 1. Get the repository

```bash
git clone <repository-url> field-sop
cd field-sop
```

The clone is your working copy. Everything you record goes here first; syncing happens
whenever you decide to `git push`.

## 2. Check that the content is intact

```bash
sop validate
```

You are looking for `0 error(s)`. Warnings are worth reading but do not stop you: they
mark things that are odd rather than wrong.

If this reports `no content found`, you are in the wrong directory - run it from the
repository root, or pass `--repo /path/to/field-sop`.

## 3. Know which checklist you are running

```bash
ls checklists/
```

A checklist is one experiment type. Pick the one that matches the session you are about
to start, and open it in the app.

## 4. Read the checklist before you are on site

The step list is meant to be read at a desk first. Anything you do not understand there
will be worse at a field site in the rain. `help/glossary.md` covers the terminology, and
`help/troubleshooting.md` covers the failures people hit most often.

## 5. Before you leave the site

- Every capture you filled in is still there after closing and reopening the app.
- Your logs are attached to the run, not merely copied somewhere.
- The run record is committed:

```bash
git add runs/ logs/
git commit -m "run: ground-walk-survey at Renfrew, 2026-09-24"
```

Attachments are committed with the record. A record that arrives without its data is not
evidence of anything.

## Where the run was written

One run is one directory under `runs/<sop_id>/`. The run record and the logs that belong
to it stay together.
