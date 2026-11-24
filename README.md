# field-sop

A lightweight local experiment notebook focused on **test notes, log remarks and tags,
search, and file management**. No sign-in or Git repository is required. Repository
synchronization, automatic pulls, and pushes have been removed.

The main navigation is **Test Notes / Logs / Search**, with test plans and the full SOP
workflow under **Manage**. A title is enough to create a note; operator, location,
equipment, and experiment details are optional. Existing SOP records remain readable,
can receive notes and logs, and can be exported.

On first use, explicitly select the **data directory containing logs and test plans**.
The app never treats its launch directory as the data directory by default. Your choice
is remembered locally and can be changed from the header or **Manage → Working Directory & Settings**.

Log history defaults to newest first and can be reversed, using log time or import time.
Each file supports remarks and multiple visible, clickable tags. Search covers titles,
notes, file names, remarks, tags, locations, and equipment without reading large log bodies.

Quick start: [`docs/NOTEBOOK.md`](docs/NOTEBOOK.md).
CLI validation, indexing, preview, export, and the existing SOP workflow remain available
for compatibility with your data and scripts. All application-owned interface text is English;
existing note text, file names, and tags are kept exactly as stored.

## Why this exists

Field procedures were previously paper or verbal, which meant:

- nobody could tell whether the printed copy was the current revision;
- execution records were transcribed by hand, so steps got skipped or back-filled;
- nothing was traceable, so a bad dataset could not be traced back to a step, a
  sensor, or a site.

A full SOP is useful when step-by-step traceability is needed. An ordinary experiment
does not require a checklist: a title, a few notes, and log files are enough.

## Layout

```
working-directory/
  logs/           Independent logs, optionally grouped by date or equipment.
  testplan/       Test plans and Markdown cases (optional).
  runs/           Test notes and existing execution records, one directory per record.
    [project_id/]<sop_id>/<run_id>/
                  events.jsonl, snapshot.md, record.md, notes.md, and attachment folders.
  .field-sop/log-remarks.json
                  Remarks and tags for independent logs; include this in backups.
  exports/        Exported records and complete working-directory backups.

application-source/
  ui/             Svelte desktop interface.
  crates/         Rust recorder, file access, CLI, and Tauri shell.
  help/           In-app help content.
  docs/           Usage, design decisions, and log policy.
  SPEC.md         Content format specification.
```

Attached files live in `logs/` inside the run directory, so one run is one
self-contained directory; see `docs/DESIGN.md` section 5 and `docs/LOGS.md`.

## Documentation

| Document | Contents |
|---|---|
| `SPEC.md` | The authoritative content format. Read this first. |
| `docs/NOTEBOOK.md` | Working directories, notes, logs, tags, search, and backups |
| `docs/DECISIONS.md` | Decisions that are expensive to revisit |
| `docs/LOGS.md` | Attachment naming, hashing, and size policy |
| `docs/USAGE-zh.md` | Archived legacy CLI examples; use the current notebook guide for the desktop UI |
| `help/*.md` | Documentation for people using the tool, shown in the app's help panel |

## Current content

Demo checklists are not required. Plans load from the selected working directory's
`testplan/` folder, and you can open your own Markdown checklists. Ordinary test notes
do not need a checklist.

## Working with it

Build once, then validate after editing:

```bash
cargo build --release
./target/release/sop validate
```

## Building and deploying on Ubuntu

The desktop app is a Tauri shell over the Svelte renderer, so building it needs the
WebKit/GTK system libraries plus Rust and Node.

**1. Install the system dependencies** (Debian/Ubuntu):

```bash
sudo apt-get install build-essential curl wget file libssl-dev libgtk-3-dev \
  libwebkit2gtk-4.1-dev librsvg2-dev patchelf libayatana-appindicator3-dev \
  libsoup-3.0-dev javascriptcoregtk-4.1-dev
```

**2. Install the toolchains:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
# Node 18+ (or via mise/nvm); npm is used to build the renderer.
node --version && npm --version
```

**3. Build the renderer and the app** (from the repository root):

```bash
cd ui && npm install && npm run build && cd ..
cargo build -p sop-app --release
```

**4. Run the desktop app:**

```bash
./run-app.sh --release
# or point it at the working copy explicitly:
./run-app.sh --workdir /path/to/test-data --release
```

`run-app.sh` installs the frontend deps, builds the renderer if missing, builds the Rust
binary, and starts the window. `--workdir` explicitly selects and remembers the data
directory; the old `--repo` spelling remains an alias. Without a flag, the app restores
the selected folder or opens the folder picker, never the code directory. Add
`--no-isolate` to use the machine's real settings instead of a throwaway XDG directory.

**5. Package installers (`.deb` and AppImage):**

```bash
cargo tauri build --bundles deb,appimage
```

The installers are written to `target/release/bundle/`. Installing the `.deb`:

```bash
sudo dpkg -i target/release/bundle/deb/field-sop_*.deb
# or apt so missing dependencies are fetched
sudo apt install ./target/release/bundle/deb/field-sop_*.deb
```

Everything runs in CI on `ubuntu-latest` (`.github/workflows/validate.yml`): formatting,
clippy, the full test suite, content validation, and the manifest build.

Every command takes `--repo <path>`; the working copy configured in the settings is used
when it is left out, and the current directory after that.

| Command | What it does |
|---|---|
| `sop validate [files...]` | Check the content against `SPEC.md`. Non-zero exit on any error. |
| `sop index` | Rebuild `dist/manifest.json`, the one file the field app loads. |
| `sop preview --port 8731` | Look at the layout, the steps, and the help panel in a browser. |
| `sop export --out DIR [--label L]` | Package a summary and every run into `DIR/export_<label>/`. |
| `sop project` | Show the project's identity and the fields that can be set. |
| `sop project set title "..."` | Rewrite one field of `project.md`, checking first. |
| `sop status` | Working copy, project, git state, and what content is present. |
| `sop remote [url]` | Legacy local git-configuration helper; does not synchronize or publish. |
| `sop settings [set KEY VALUE]` | Show or change this application's own preferences. |
| `sop run start <sop> <id> <op> <site> [--override REASON]` | Start a run: freeze a snapshot, open the event log. |
| `sop run record <sop> <id> <event...>` | Append one event (capture / checkbox / done / skip / deviate / note). |
| `sop run recover <sop> <id>` | Replay the event log to the run's latest state. |
| `sop run attach <sop> <id> <file> [--kind log\|photo\|file]` | Copy a file in, hashed, and record an attachment event. The kind picks `logs/`, `photos/`, or `attachments/`. |
| `sop run end <sop> <id> <status>` | End the run and write the record file. |
| `sop run deviations <sop>` | Roll up every deviation across the checklist's runs. |
| `sop run export <sop> --format csv` | Export run captures as CSV for the processing pipelines. |
| `sop run export <sop> --format markdown --run <id> [--out PATH]` | Export one run as a self-contained record document. |
| `sop run delete <sop> <id> [--yes]` | List what a run occupies; with `--yes`, remove it. |

Starting a run picks the instrument from a list instead of a text box: the `sensors`
setting holds `model: serial, serial; model` (default `UAS-MAG; RM3100`), the checklist's
own `equipment:` front matter arrives ticked, and a serial typed by hand joins the list.
The run id is built from the date, model and serial (`2026-09-28-uas-mag-1001`) and can
still be changed by hand.

The `devices` setting is for everything that is not the instrument - GNSS receiver, base
station, drone, battery, ground station, radio. It holds the same shape,
`kind/name: serial, serial; kind/name`, and the start form adds a ticked group for each
device, written into the record's `hardware` list as `kind: name serial`. `devices` and
`sensors` coexist: `sensors` still decides the run id, the sensor filter, and the record's
`sensor` block; `devices` only adds to `hardware`.

Test Notes uses a chronological record list and a detail pane instead of a ten-column
history table. Add Markdown notes, multiple logs/photos/attachments, open folders, or
export from the selected record. Logs lists both attached files and independent files
under `logs/`, and supports multi-file imports without first creating an experiment.

Imports keep the originals and suffix same-name conflicts with different contents.
Edit remarks and tags inline without changing log bytes. Files larger than 50 MB ask
before copying. Notes and logs can be added after completion without moving the event
seal. Single-record deletion is under More and requires two confirmations; there is
no home-page delete-all action.

**Export working directory** packages a report and every run into one self-contained folder,
`<destination>/export_<local time>/`: `summary.md`, `summary.csv`, each run's record and
whole run directory, and a `MANIFEST.sha256`. The folder picker starts in `exports/`
(ignored by `git`) and the result is reported as runs / files / size, with a link to open
the folder. The Markdown and CSV tables provide a record overview. A failed export
leaves nothing behind, and a folder that already exists gets a `-2` suffix
(`docs/DECISIONS.md` D30).

A full export also includes independent `logs/`, `testplan/`, the project file, and
remark/tag metadata. The app does not pull, commit, or push data. Backups do not depend
on a network or Git. D31 supersedes the publishing behavior from D17.

```bash
./target/release/sop index
./target/release/sop project set title "Geomagnetic Survey Field Work"
```

The project name can still live in `project.md`, but is not required to create notes.
Legacy CLI local git metadata/configuration remains available; the desktop no longer
exposes remote or branch settings.

```bash
./target/release/sop settings            # where the settings file is, and what it holds
./target/release/sop status              # what the app would open, and the git state
```

Run the tooling's own tests, which include a negative case for every rule:

```bash
cargo test
```

Frontend checks and helper tests (the test scripts use Node 24's TypeScript support):

```bash
cd ui
npm ci
npm run check
npm run test:records
npm run test:devices
npm run test:api
npm run test:language
npm run build
```

The Rust implementation is the only implementation of the format. The transitional
Python validator and manifest builder were removed once `sop validate` and `sop index`
reached parity with them; see `docs/DECISIONS.md` D11.

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
