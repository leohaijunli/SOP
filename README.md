# field-sop

Digitized field-test SOPs for geomagnetic survey work: **checklists that carry their
own instructions**, plus execution records that can be reviewed and fed back into the
SOPs themselves.

Every customizable part of this repo is a plain Markdown file. No build step is
required to read or edit it; a human with a text editor, a code review, and the desktop
app are all first-class users.

**Status:** content and format are validated; the CLI, the event-sourced run recorder
(P2/P3), and the desktop app (P4, execution + history views) are implemented. Start with
`docs/DESIGN.md`.

What works today: `sop validate`, `sop index`, `sop preview`, `sop status`,
`sop project`, `sop remote`, `sop settings`, `sop export`, and
`sop run start|record|recover|attach|end` (plus `sop run deviations`, `sop run export`,
and `sop run delete`). The desktop app (`field-sop`) renders Run / History / Browse /
Edit / Project / Settings views, attaches logs/photos/files to a run (during or after
it), exports a run's record, packages a summary with every run into one folder, and
publishes the working copy with a push, and is packaged as a `.deb` and AppImage on tag
(`.github/workflows/validate.yml`).

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
runs/             Execution records and their data, one directory per run. Never overwritten.
  _inbox/         Field observations that are candidates for promotion into procedures/.
  <sop_id>/<run_id>/
                  events.jsonl, snapshot.md, record.md, and logs/ - the attached files.
help/             In-app documentation, shown in the app's help panel. Never executed.
project.md        The repository's identity: which campaign this is, and who leads it.
templates/        Starting points for authoring new files by hand.
crates/           The Rust domain, repository access, and `sop` CLI.
docs/             Design, features, scenarios, decisions, log policy.
SPEC.md           The authoritative format specification. Read this first.
```

Attached files live in `logs/` inside the run directory, so one run is one
self-contained directory; see `docs/DESIGN.md` section 5 and `docs/LOGS.md`.

## Documentation

| Document | Contents |
|---|---|
| `SPEC.md` | The authoritative content format. Read this first. |
| `docs/DESIGN.md` | Architecture, crate layout, stack choices, phases, risks |
| `docs/FEATURES.md` | Expected feature list with priorities |
| `docs/SCENARIOS.md` | Six typical situations the tool is built for |
| `docs/DECISIONS.md` | Decisions that are expensive to revisit |
| `docs/LOGS.md` | Attachment naming, hashing, and size policy |
| `docs/IMPROVEMENTS.md` | Review notes: improvements for field engineers, ordered by value |
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
./run-app.sh --repo /path/to/working-copy --release
```

`run-app.sh` installs the frontend deps, builds the renderer if missing, builds the Rust
binary, and starts the window. Add `--no-isolate` to use the machine's real settings
instead of a throwaway XDG directory.

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
| `sop remote [url]` | Show or set the git remote URL, through `git` itself. |
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

The app's History screen groups runs under the test plan and case they were started
from, with anything that does not match a case kept under "Runs outside a test plan".
Every table shares one column layout (`run id / started / site / sensor / operator /
outcome / conclusion / dev / files / ▸`), so the cases line up, and the `files` column
shows the log / photo / file counts with a hint when an ended run has no logs yet.

Each row opens a detail drawer. There the operator can **Add log…**, **Add photos…**, or
**Add files…** (each multi-select; a file over 50 MB asks first), **Add note…**, see the
run's files and notes with their size, time, and an `after run` tag, **Open folder** (the
run's own directory, in the desktop file manager), **Export record**, and **Delete** (that
one run, after asking). Attaching works whether the run is live or already ended; what was
added after the end is listed separately in the record, and the seal over the field record
does not move (`SPEC.md` section 8, `docs/DECISIONS.md` D29).

**Export summary** packages a report and every run into one self-contained folder,
`<destination>/export_<local time>/`: `summary.md`, `summary.csv`, each run's record and
whole run directory, and a `MANIFEST.sha256`. The folder picker starts in `exports/`
(ignored by `git`) and the result is reported as runs / files / size, with a link to open
the folder. The markdown and CSV tables carry the same columns as History. A failed export
leaves nothing behind, and a folder that already exists gets a `-2` suffix
(`docs/DECISIONS.md` D30).

The toolbar also has **Push repo** (`git add -A`, commit with the message you give it,
push to the configured remote) and **Delete all runs** (the whole history in this working
copy; `runs/_inbox/` is left alone). Deletion cannot be undone. The app holds no
credential and runs `git` with `GIT_TERMINAL_PROMPT=0`. See `docs/DECISIONS.md` D17.

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
