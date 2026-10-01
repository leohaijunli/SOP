# Log Handling Policy

## Where logs live

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

`logs/<run_id>/` at the repository root is the older location, kept readable for
records written before the run directory became self-contained. A directory there
without a matching `runs/<sop_id>/<run_id>.md` is a validator error, because a log with
no record has no context.

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

Avoid spaces and non-ASCII characters in log filenames. Instrument-generated names
that violate this should be renamed on attach, not left as-is.

## Size and Git LFS

Text logs from a day of walking survey are typically single-digit megabytes and are
fine to commit directly.

Switch to Git LFS before the repository exceeds roughly 50 MB, or as soon as binary
artifacts (HDF5, imagery, raw instrument binaries) appear. This repo intentionally
does not enable LFS by default, because an active `filter=lfs` attribute in a clone
without LFS installed breaks checkout. To opt in:

```bash
git lfs install
git lfs track "runs/**/logs/**/*.h5" "runs/**/logs/**/*.bin" "runs/**/logs/**/*.tif"
git add .gitattributes
```

Photographs are the usual reason to cross the threshold: a phone photo is a few
megabytes and a run may collect a dozen. If photographs are being committed, track them
too:

```bash
git lfs track "runs/**/photos/**"
git add .gitattributes
```

LFS is still opt-in. Enabling it in this repository would break a clone on a machine
without the LFS filter installed, so the decision stays with the operator, and this is
the line to run if they make it.

## If a log is too large or cannot be committed

Keep the record and the manifest in git, and store the payload elsewhere:

1. Still write the `logs:` entry, with `sha256` and `size`.
2. Add `external: true` and a `location:` string naming the archive.
3. Commit only the run record.

The record stays valid because the hash is what makes it verifiable, not the file's
presence in this repository.
