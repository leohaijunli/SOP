# Log Handling Policy

## Where logs live

```
logs/<run_id>/
  raw.csv
  base_station.csv
  notes.txt
```

One directory per run, named exactly after the run record's `run_id`. A log directory
without a matching `runs/<sop_id>/<run_id>.md` is a validator error, because a log
with no record has no context.

## Why hash on attach

The app records `sha256` and `size` for every attached file. This is what turns "the
run used the Renfrew dataset" into a checkable claim: if a log is later edited,
truncated, or replaced, the mismatch is detectable. Attach-time hashing is cheap; a
dataset whose provenance is unknown is not.

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
git lfs track "logs/**/*.h5" "logs/**/*.bin" "logs/**/*.tif"
git add .gitattributes
```

## If a log is too large or cannot be committed

Keep the record and the manifest in git, and store the payload elsewhere:

1. Still write the `logs:` entry, with `sha256` and `size`.
2. Add `external: true` and a `location:` string naming the archive.
3. Commit only the run record.

The record stays valid because the hash is what makes it verifiable, not the file's
presence in this repository.
