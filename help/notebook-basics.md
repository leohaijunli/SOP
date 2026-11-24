---
kind: help
help_id: notebook-basics
title: Test Notes and Log Management
section: Using the app
order: 1
audience: operator
updated: 2026-11-24
summary: Choose a working directory, record lightweight experiments, and manage logs by time and tags.
---

## Working directory

Choose the data folder containing logs and test plans on first use. `logs/` holds
independent logs, `testplan/` holds optional plans, and `runs/` holds notes and execution
records. The current directory is shown in the header and can be changed there.
No Git repository or sign-in is required.

## Test notes

Only a title is required. Operator, location, equipment, and body are optional.
Add Markdown notes in the detail pane and save with Ctrl + Enter. Add multiple logs,
photos, or attachments in one selection.

## Log history, remarks, and tags

Logs default to newest first; change the direction or time basis as needed.
Remarks / Tags edits a row inline. Separate multiple tags with commas; tags are
displayed and clickable. Remarks do not change log bytes. Independent-log metadata
is stored in `.field-sop/log-remarks.json`.

## Search and backup

Ctrl + K opens Search; Ctrl + N creates a note. Search covers notes and file remarks /
tags, not large log bodies. Export working directory backs up records, logs, plans,
and remark/tag metadata without repository synchronization.
