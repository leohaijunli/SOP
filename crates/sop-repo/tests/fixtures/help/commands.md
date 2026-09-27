---
kind: help
help_id: commands
title: Common commands
section: PX4 Operations
order: 50
audience: operator
summary: The handful of commands worth memorising, and what each one is for.
updated: 2026-09-25
tags: [cli, git]
---

Everything below is run from the repository root.

## Content

Check every procedure, checklist, run record, and help page:

```bash
sop validate
```

Check one file, or a few, while you are editing:

```bash
sop validate procedures/base-station.md
```

Point at a repository somewhere else:

```bash
sop validate --repo /path/to/field-sop
```

Rebuild the file the app loads at startup. Run this after editing content by hand:

```bash
sop index
```

Write the manifest somewhere else:

```bash
sop index --out /tmp/manifest.json
```

## The project, and this app's own settings

Show which campaign the repository is for, and the fields that can be changed:

```bash
sop project
```

Rename it. Only that one field is rewritten: the prose, the comments, and the key order
all stay as they were, and the change is refused if it would leave an invalid file.

```bash
sop project set title "Renfrew Walk 2026"
sop project set updated 2026-09-25
```

The project's name lives in `project.md` in the repository, not in the app, so everybody
who opens the repository sees the same one. What the app keeps for itself is where the
working copy is and which remote to use:

```bash
sop settings                                  # what it holds, and where the file is
sop settings set repository ~/field-sop       # open this working copy by default
sop settings set remote upstream              # use a remote other than origin
sop settings unset branch                     # back to the default
```

The remote **URL** is not a setting, and the app will not store one: it belongs to the
repository, where `git` keeps it. Set it through `git`'s own configuration:

```bash
sop remote                                    # show the URL git holds
sop remote https://github.com/leo/field-sop.git
```

## Looking at the content without the app

When the desktop app is not to hand, or while editing, this shows the same steps and help
pages in a browser. It serves only on this machine, and it never writes anything:

```bash
sop preview --port 8731
```

`sop status` is the one command worth running at the start of a field day. It prints the
working copy, the project, the branch, whether anything is uncommitted, and whether the
copy is ahead of or behind the others:

```bash
sop status
```

## Reading a validation report

```text
procedures/base-station.md
  error: line 42: step 'base-mark': capture 'mark_id' has no 'label'

1 error(s), 0 warning(s)
```

Errors fail the command; warnings do not. Both are grouped by file, in line order. Each
message names the file, the line, and the thing that is wrong, because the alternative is
hunting.

## A run, end to end

```bash
sop validate                                  # nothing broken before you start
git pull --rebase                             # start from what the others have
# ... record the run in the app ...
sop validate                                  # the new record is consistent
git add runs/ logs/
git commit -m "run: ground-walk-survey at Renfrew, 2026-09-24"
git push
```

The History screen does the last three steps for you: **Export record** writes the
record on its own, and **Push repo** runs `git add -A`, commits, and pushes. Use the
commands when you want to choose exactly what goes into the commit.

## Getting the record out

`sop run end` writes the record into the repository. The same document can be exported
on its own - it is self-contained, so it reads as the experiment record without the
repository beside it:

```bash
sop run export ground-walk-survey --format markdown --run 2026-09-24-renfrew-walk01
sop run export ground-walk-survey --format markdown --run 2026-09-24-renfrew-walk01 \
  --out ~/records/renfrew-walk01.md
```

Every step is one section: its instructions, its checkbox items exactly as the operator
left them, and the values recorded against it. Captures for the processing pipelines
come out as CSV instead, one row per value across every run of the checklist.

The head of the record restates the operator, the site, the sensor, the hardware, and the
conditions, so the exported file answers "with what, and in what weather" without a YAML
reader. Those three instrument blocks are optional and are entered in the app's start
panel. A step that has data but no outcome keeps its `yaml result` block with the
`status` line left out; only a step with nothing recorded at all has no block.

The same document is one button away in the app: **Export record** on a History row
writes it to `exports/<sop_id>-<run_id>.md` in the working copy. A record written before
runs carried their own directory is exported from the record file itself, so runs
recorded by an older build still export.

A run whose record was written by hand, or before runs carried their own directory, has
no event log to re-render from. `export` returns that file as it stands rather than
failing.

## Removing a run

Deleting a run is not editing it, and `git` is the only undo, so the command is a dry
run by default:

```bash
sop run delete ground-walk-survey 2026-09-25-typo        # lists what would go
sop run delete ground-walk-survey 2026-09-25-typo --yes  # removes it
```

It removes the record, the run directory, and the run's log directory - the three
places a run occupies, and nothing else. Anything already committed is recoverable
with `git restore`.

## Repository

See what a session changed, including the data:

```bash
git status
git diff --stat
```

Look at what somebody else changed in a procedure, and why:

```bash
git log -p procedures/base-station.md
```

Bring your copy up to date before a field day, not during one:

```bash
git pull --rebase
```

Undo an edit to one file, before committing:

```bash
git restore checklists/ground-walk-survey.md
```

## Maintenance

Run the validator's own tests, after changing the tooling rather than the content:

```bash
cargo test
```

Check formatting and lints before opening a pull request:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## Undoing a bad attachment

The digest makes a wrong file obvious, and `git` makes it reversible. Restore the record
and the log together - restoring only one of them leaves the hash check failing on the
next run:

```bash
git restore runs/<sop_id>/<run_id>.md logs/<run_id>/
```
