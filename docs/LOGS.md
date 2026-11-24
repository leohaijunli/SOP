# Log Handling Policy

## Where logs live

The desktop requires an explicitly selected data directory. Independent logs may live
under its `logs/` folder (including subfolders) without a test record. Logs attached to
a test remain inside that record's own directory as below.

```
runs/<sop_id>/<run_id>/
  logs/
    raw.csv
    base_station.csv
    notes.txt
  photos/
    setup.jpg
    site_wide.jpg
  attachments/
    calibration.pdf
    reading.xlsx
```

Logs live inside the run directory, so one run is one folder that can be archived,
moved, or deleted as a unit (`docs/DECISIONS.md` D10). Each attached file keeps its own
name; a second file with the same name gets a `-2`, `-3` ... suffix. Picking a file the
run already holds again adds nothing rather than a second copy.

An attachment has a kind, and the kind decides the subdirectory:

- `logs/` - instrument and console logs, text or CSV;
- `photos/` - photographs of the setup, the site, or a reading;
- `attachments/` - anything else: PDF, spreadsheet, exported screenshot.

The subdirectories are created only when something is filed under them, so a run with no
photographs has no empty `photos/`. A file can be attached while the run is live or after
it has ended; the run record and `record.md` list what arrived after the end separately
(`SPEC.md` section 8.2, `docs/DECISIONS.md` D29). What the seal protects is the field
record, not the later uploads.

`logs/<run_id>/` at the working-directory root is also readable for older records.
Unassociated log folders are supported; matching a record is not a requirement.

## Why hash on attach

The app records `sha256` and `size` for every attached file. This is what turns "the
run used the Renfrew dataset" into a checkable claim: if a log is later edited,
truncated, or replaced, the mismatch is detectable. Attach-time hashing is cheap; a
dataset whose provenance is unknown is not.

The record carries the hash in its `logs:` front matter as well as restating the file in
its body, so the validator re-checks the file's presence, size, and hash on every pass.

## Naming

Recommended, but not enforced:

```
<site-slug>_<yyyymmdd>_<sensor>_<stream>.csv
renfrew_20260924_gsm19_raw.csv
renfrew_20260924_gsm19_base.csv
```

Imports preserve the original file name, including spaces and Unicode. Same-name files
with different contents receive a suffix; no source file is renamed or modified.

## Chronological history, remarks, and tags

The log list defaults to newest first; it can be reversed or sorted by import time.
For recorded CSV logs with a recognized time column, the data timestamp is used.
Otherwise file modification time is used and labeled. Import time is kept separate,
and a source file's modification time is preserved when copying it.

Every file can carry a free-form remark and multiple tags. Tags are displayed in the
list, clickable for filtering, and searchable along with names and remarks. They never
change the log bytes. Attached-file metadata is an append-only event, including changes
after the run ends. Loose-log metadata lives in `.field-sop/log-remarks.json`.

## Size and backup

Files over 50 MB ask for confirmation before copying. Independent log imports are
streamed rather than loaded into memory as one large buffer. Search reads indexed
metadata and notes, not raw log bodies; full content validation is a manual operation.

Use **Export working directory** for a self-contained backup. It contains all records and attachments,
independent logs, local test plans, the project file, remark/tag metadata and a checksum
manifest. If copying a folder manually, include `.field-sop/log-remarks.json` as well.
There is no git synchronization or sign-in requirement.
