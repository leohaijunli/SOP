# field-sop

Digitized field-test SOPs for geomagnetic survey work: **checklists that carry their
own instructions**, plus execution records that can be reviewed and fed back into the
SOPs themselves.

Every customizable part of this repo is a plain Markdown file. No build step is
required to read or edit it; a human with a text editor, a code review, and the desktop
app are all first-class users.

**Status:** content and format are in place and validated; the application is designed
but not yet built. Start with `docs/DESIGN.md`.

What works today: `sop validate`, `sop index`, `sop preview`, `sop status`,
`sop project`, `sop remote`, and `sop settings`. What does not exist yet: the desktop
application, the run-record writer, and attachments. See `docs/DESIGN.md` section 8.

## Why this exists

Field procedures were previously paper or verbal, which meant:

- nobody could tell whether the printed copy was the current revision;
- execution records were transcribed by hand, so steps got skipped or back-filled;
- nothing was traceable, so a bad dataset could not be traced back to a step, a
  sensor, or a site.

This repo holds the procedures under version control and gives the field app a place
to write records back into.

## Layout

```
procedures/       Reusable step blocks. The atomic unit of the whole system.
checklists/       One experiment type = procedures to include + local steps.
runs/             Execution records, one directory per run. Never overwritten.
  _inbox/         Field observations that are candidates for promotion into procedures/.
help/             In-app documentation, shown in the app's help panel. Never executed.
project.md        The repository's identity: which campaign this is, and who leads it.
templates/        Starting points for authoring new files by hand.
logs/             Raw instrument logs, one directory per run.
crates/           The Rust domain, repository access, and `sop` CLI.
tools/            Transitional Python validator and manifest builder (being deleted).
docs/             Design, features, scenarios, decisions, log policy.
SPEC.md           The authoritative format specification. Read this first.
```

`logs/` is planned to move inside each run directory so that one run is one
self-contained directory; see `docs/DESIGN.md` section 5.

## Documentation

| Document | Contents |
|---|---|
| `SPEC.md` | The authoritative content format. Read this first. |
| `docs/DESIGN.md` | Architecture, crate layout, stack choices, phases, risks |
| `docs/FEATURES.md` | Expected feature list with priorities |
| `docs/SCENARIOS.md` | Six typical situations the tool is built for |
| `docs/DECISIONS.md` | Decisions that are expensive to revisit |
| `docs/LOGS.md` | Attachment naming, hashing, and size policy |
| `docs/PARITY.md` | What the Rust port matches in the Python tooling, and what differs |
| `help/*.md` | Documentation for people using the tool, shown in the app's help panel |

## Current content

| Checklist | Status | Purpose |
|---|---|---|
| `checklists/mag-sensor-calibration.md` | draft | Sensor-level calibration of a geomagnetic sensor |
| `checklists/ground-walk-survey.md` | draft | Hand-carried ground magnetic walk survey |

Planned: interference test, navigation test, UAV-mounted survey.

## Working with it

Build once, then validate after editing:

```bash
cargo build --release
./target/release/sop validate
```

Every command takes `--repo <path>`; the working copy configured in the settings is used
when it is left out, and the current directory after that.

| Command | What it does |
|---|---|
| `sop validate [files...]` | Check the content against `SPEC.md`. Non-zero exit on any error. |
| `sop index` | Rebuild `dist/manifest.json`, the one file the field app loads. |
| `sop preview --port 8731` | Look at the layout, the steps, and the help panel in a browser. |
| `sop project` | Show the project's identity and the fields that can be set. |
| `sop project set title "..."` | Rewrite one field of `project.md`, checking first. |
| `sop status` | Working copy, project, git state, and what content is present. |
| `sop remote [url]` | Show or set the git remote URL, through `git` itself. |
| `sop settings [set KEY VALUE]` | Show or change this application's own preferences. |

```bash
./target/release/sop index
./target/release/sop project set title "Geomagnetic Survey Field Work"
./target/release/sop remote https://github.com/<you>/field-sop.git
```

The project's name is repository content, not a preference: it is committed, so every
operator and every old record agrees about which campaign the work belongs to. The
settings file holds the path to the working copy and the *name* of the remote, and never
a URL or a token - `git` keeps the URL, where it belongs. See `docs/DECISIONS.md` D14.

```bash
./target/release/sop settings            # where the settings file is, and what it holds
./target/release/sop status              # what the app would open, and the git state
```

Run the tooling's own tests, which include a negative case for every rule:

```bash
cargo test
```

The Python tools under `tools/` are **transitional**. They kept content validated while
the Rust implementation was written, and they are the reference behaviour the port was
tested against. `sop validate` and `sop index` now produce the same results, so the
Python tooling is being removed; see `docs/DECISIONS.md` D11.

All of it runs in CI on every push and pull request
(`.github/workflows/validate.yml`).
Procedures and checklists are intended to change through pull requests rather than
direct commits to `main`, so that the revision used in the field has actually been
reviewed.

## Editing rules that keep this repo usable

- One step = one `##` heading + an optional `yaml step` block + free Markdown.
- Steps carry a stable `id`. Never renumber or reuse an id; records reference it.
- Add new content by appending. Do not reflow existing prose in bulk.
- Files are UTF-8, LF line endings, no BOM, trailing newline.

Full details, including the run-record schema and the rules the field app must follow
when writing back to these files, are in `SPEC.md`.

## Related repositories

This repo is self-contained: content, specification, validator, and application. It does
not depend on the software that consumes its data:

- `mag_gcs` - magnetometer monitor and calibration GUI
- `geomag-uav-survey` - survey processing and visualization platform

Records produced here are designed to be parsed into the processing pipelines in
those repositories.
