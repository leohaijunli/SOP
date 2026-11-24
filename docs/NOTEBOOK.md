# Test Notes and Log Management

## 1. Choose a working directory

Choose the data folder on first launch; your selection is remembered locally. This
folder contains logs, test plans, and records, not the application source code.
An existing data directory or an empty folder is fine. No Git repository or sign-in is needed.

```text
working-directory/
  logs/                       Independent logs, including subfolders
  testplan/                   Test plans (optional)
  runs/                       Test notes and existing execution records
  .field-sop/log-remarks.json  Remarks and tags for independent logs
```

Click the directory in the header, or use **Manage → Working Directory & Settings →
Switch working directory**. Switching does not move or delete the source data. If the
previous folder is unavailable, the app asks you to choose again rather than falling
back to the program directory.

## 2. Record a lightweight experiment

1. Select **New note** and enter a title.
2. Optionally record the purpose, actions, observations, or conclusion. No test plan or full report is required.
3. Operator, location, and equipment are optional. The date and ID are generated automatically.
4. Add more Markdown notes in the detail pane; **Ctrl + Enter** saves.
5. Use **Add log files** or **Add photos / attachments** to select multiple files.

Unsubmitted drafts are saved locally and isolated by working directory. Saved content
is authoritative in `events.jsonl`; `record.md` and `notes.md` are readable generated
files. Do not edit the event log directly.

**Complete record** marks the record as complete; it does not automatically judge the test as passed.
You can still add notes, attachments, file remarks, and tags afterward. No operator
sign-in is required; leave the name blank or set a machine-local default in Manage.

## 3. Manage logs chronologically

Logs lists both attached files and independent files under `logs/`. **Add log files**
supports multiple selection and defaults to independent storage. Optionally choose a
record under **Link to experiment**. Originals are kept; same-name conflicts with
different contents get `-2`, `-3` suffixes, while identical files are reused.

- Default order is **Newest first**; **Oldest first** reverses it.
- Recorded CSV logs with a recognized time column use the data timestamp; otherwise file modification time is used and labeled.
- You can sort by import time. A missing legacy import time is never fabricated.
- **More filters** provides date ranges and linked-record filtering.
- **Show in folder** opens the containing folder without running or opening the log itself.

## 4. File remarks and tags

Use **Remarks / Tags** on a file row to edit inline. Remarks can explain the data's
purpose, anomalous intervals, or processing notes. Multiple tags are comma-separated,
for example `temperature-drift, retest, needs-review`. Tags appear directly in the
list, are clickable for filtering, and are also available in the tag dropdown.

Attached-file remarks and tags are append-only events. They do not modify the raw
file or the sealed prefix. Independent-log metadata lives in
`.field-sop/log-remarks.json`; include this file when copying data manually.

## 5. Search

**Ctrl + K** opens Search; **Ctrl + N** creates a note. Keywords cover titles, notes,
file names, remarks, tags, locations, and equipment. Combine terms with spaces, such
as `GPS warmup`. Search does not read large log bodies, keeping the list responsive.

## 6. Management and backup

Checklist execution, SOP browsing/editing, test plans, and equipment shortcuts are
optional tools under Manage. Markdown plans and cases in `testplan/` are indexed
automatically. **Manage → Test Plans** loads them and offers **Refresh local plans**.
Plain MD cases do not need front matter; second-level headings become steps.
The home screen does not show branches, remotes, synchronization status, or the full run-start form.

**Export record** writes one Markdown record. **Export working directory** creates a
timestamped, self-contained backup containing records, attachments, independent logs,
test plans, the project file, remark/tag metadata, and a checksum manifest. The export
destination cannot be inside source `logs/`, `runs/`, or `testplan/`, preventing recursive copies.

Deletion is under **More** in record details, states the exact scope, and requires two
confirmations. Independent logs and original imported files are unaffected. The app
does not provide repository pulls, commits, pushes, or a sign-in window.

The interface and current guide are English. User-authored notes, file names, and tags
are preserved in their original language.
