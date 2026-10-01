#!/usr/bin/env bash
# Virtual-user end-to-end traversal of the field-sop CLI. Drives a disposable
# working-copy repository through every command, the way an operator would, and
# reports each step PASS/FAIL so a change that breaks any path shows up as a red
# line rather than a silent surprise.
#
#   runs/virtual-user-e2e.sh [--repo PATH] [--keep]
#
# The repo is rebuilt from the source tree if it does not exist, so a run never
# touches the live working copy. --keep leaves the copy in place for inspection.

set -u

SOP="${SOP:-$(pwd)/target/debug/sop}"
REPO="/tmp/vu-repo"
KEEP=0
PASS=0
FAIL=0
STEP=0

for arg in "$@"; do
  case "$arg" in
    --repo) REPO="$2"; shift 2 ;;
    --keep) KEEP=1; shift ;;
  esac
done

pass() { PASS=$((PASS + 1)); printf 'PASS %02d  %s\n' "$STEP" "$1"; }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %02d  %s\n' "$STEP" "$1"; }
step() { STEP=$((STEP + 1)); }
ok() { step; if [ "$2" -eq 0 ]; then pass "$1"; else fail "$1"; fi }

echo "== field-sop virtual-user traversal =="
echo "repo: $REPO   sop: $SOP"

if [ ! -x "$SOP" ]; then
  echo "sop binary not found at $SOP; run: cargo build -p sop-cli"
  exit 2
fi

# A fresh, self-contained working copy to avoid polluting the live one.
if [ ! -d "$REPO/.git" ]; then
  SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  mkdir -p "$REPO"
  rsync -a --exclude=target --exclude=node_modules --exclude=.git --exclude=dist "$SRC/" "$REPO/"
  (cd "$REPO" && git init -q && git add -A && git -c user.email=vu@test -c user.name=vu commit -qm init)
fi
cd "$REPO"

# ---- static inspection -----------------------------------------------
# `settings` and `project` take --repo at the command level, before the subcommand;
# `status`/`validate`/`index` take it at the end.
if [ ! -f "$REPO/project.md" ]; then
  cat > "$REPO/project.md" <<'MD'
---
kind: project
title: Virtual User Project
project_id: vu-2026
institution: Test Lab
lead: Virtual User
started: 2026-09-26
updated: 2026-09-26
summary: A project created for the virtual-user end-to-end traversal.
contact: vu@test
---
MD
fi

# Seed the test plans the traversal runs against, so a rebuilt copy always has them.
seed_plan() { # seed_plan <dir> <title> <order>
  mkdir -p "$REPO/testplan/$1"
  [ -f "$REPO/testplan/$1/plan.md" ] || cat > "$REPO/testplan/$1/plan.md" <<MD
---
title: $2
order: $3
version: 1
updated: 2026-09-26
---

$4
MD
}
seed_plan mag-scan "Magnetic Line Scan" 10 "Ground magnetic line scans."
cat > "$REPO/testplan/mag-scan/line-scan.md" <<'MD'
---
kind: checklist
sop_id: mag-scan-line
title: Walk a Survey Line
order: 10
version: 1
status: active
equipment:
  - Fluxgate magnetometer
---

## Setup

- [ ] Baseline taken
- [ ] Staff levelled

## Walk

- [ ] Start marked
- [ ] Line walked

## Wrap

- [ ] Baseline repeated
MD
seed_plan sensor-bench "Sensor Bench" 20 "Bench checks for the lab magnetometer."
cat > "$REPO/testplan/sensor-bench/offset.md" <<'MD'
---
kind: checklist
sop_id: sensor-bench-offset
title: Offset Correction
order: 10
version: 1
status: active
equipment:
  - Lab magnetometer
---

## Procedure

- [ ] Enclosure at zero
- [ ] Reading recorded
MD
seed_plan preflight "Preflight Check" 30 "Go/no-go checks before flight."
cat > "$REPO/testplan/preflight/checks.md" <<'MD'
---
kind: checklist
sop_id: preflight-go
title: Go/No-Go
order: 10
version: 1
status: active
equipment:
  - UAV
  - Flight battery
---

## Battery

- [ ] Charge above threshold
- [ ] Voltage recorded
MD

(cd "$REPO" && git add -A && git -c user.email=vu@test -c user.name=vu commit -qm "seed project" >/dev/null)

$SOP status --repo "$REPO" >/dev/null 2>&1; ok "status" "$?"
$SOP project --repo "$REPO" >/dev/null 2>&1; ok "project (show)" "$?"
$SOP project --repo "$REPO" set title "Virtual User Test 2" >/dev/null 2>&1; ok "project set title" "$?"
$SOP settings --repo "$REPO" >/dev/null 2>&1; ok "settings (show)" "$?"
$SOP settings --repo "$REPO" path >/dev/null 2>&1; ok "settings path" "$?"
$SOP settings --repo "$REPO" set sensors "UAS-MAG: 1001; RM3100" >/dev/null 2>&1; ok "settings set sensors" "$?"
$SOP settings --repo "$REPO" unset sensors >/dev/null 2>&1; ok "settings unset sensors" "$?"
$SOP validate --repo "$REPO" >/dev/null 2>&1; ok "validate" "$?"
$SOP index --repo "$REPO" >/dev/null 2>&1; ok "index" "$?"

# ---- run A: full event coverage on mag-scan-line ----------------------
A=vur-a-line
$SOP run start mag-scan-line "$A" "ops-sim" "site-alpha" --repo "$REPO" >/dev/null 2>&1; ok "run A start" "$?"

echo "sensor log line 1" > /tmp/vur-sensor.log
echo "IMG_DUMMY" > /tmp/vur-photo.jpg

$SOP run record mag-scan-line "$A" done setup --repo "$REPO" >/dev/null 2>&1; ok "record done" "$?"
$SOP run record mag-scan-line "$A" skip walk "wind too high" --repo "$REPO" >/dev/null 2>&1; ok "record skip" "$?"
$SOP run record mag-scan-line "$A" deviate wrap "end marker lost" --repo "$REPO" >/dev/null 2>&1; ok "record deviate" "$?"
$SOP run record mag-scan-line "$A" capture setup reading 49432 --unit nT --repo "$REPO" >/dev/null 2>&1; ok "record capture" "$?"
$SOP run record mag-scan-line "$A" clear setup reading "retyped value" --repo "$REPO" >/dev/null 2>&1; ok "record clear" "$?"
$SOP run record mag-scan-line "$A" checkbox setup 0 true --repo "$REPO" >/dev/null 2>&1; ok "record checkbox" "$?"
$SOP run record mag-scan-line "$A" note --step setup "noted during line" --repo "$REPO" >/dev/null 2>&1; ok "record step note" "$?"
$SOP run record mag-scan-line "$A" note "run-level note" --repo "$REPO" >/dev/null 2>&1; ok "record run note" "$?"
$SOP run attach mag-scan-line "$A" --kind log /tmp/vur-sensor.log --repo "$REPO" >/dev/null 2>&1; ok "attach log" "$?"
$SOP run attach mag-scan-line "$A" --kind photo /tmp/vur-photo.jpg --repo "$REPO" >/dev/null 2>&1; ok "attach photo" "$?"

$SOP run recover mag-scan-line "$A" --repo "$REPO" >/dev/null 2>&1; ok "recover A" "$?"
$SOP run end mag-scan-line "$A" complete --conclusion pass --note "clean line" --repo "$REPO" >/dev/null 2>&1; ok "run A end" "$?"

# ---- run B: partial, fail --------------------------------------------
B=vur-b-offset
$SOP run start sensor-bench-offset "$B" "ops-sim" "lab-1" --repo "$REPO" >/dev/null 2>&1; ok "run B start" "$?"
$SOP run record sensor-bench-offset "$B" done procedure --repo "$REPO" >/dev/null 2>&1; ok "run B done" "$?"
$SOP run end sensor-bench-offset "$B" partial --conclusion fail --note "offset out of spec" --repo "$REPO" >/dev/null 2>&1; ok "run B end (partial/fail)" "$?"

# ---- run C: aborted, inconclusive ------------------------------------
C=vur-c-preflight
$SOP run start preflight-go "$C" "ops-sim" "pad-3" --repo "$REPO" >/dev/null 2>&1; ok "run C start" "$?"
$SOP run record preflight-go "$C" deviate battery "voltage low" --repo "$REPO" >/dev/null 2>&1; ok "run C deviate" "$?"
$SOP run end preflight-go "$C" aborted --conclusion inconclusive --note "aborted at gate" --repo "$REPO" >/dev/null 2>&1; ok "run C end (aborted)" "$?"

# ---- analysis --------------------------------------------------------
$SOP run deviations mag-scan-line --repo "$REPO" >/dev/null 2>&1; ok "run deviations" "$?"
$SOP run timeline mag-scan-line "$A" --repo "$REPO" >/dev/null 2>&1; ok "run timeline" "$?"
$SOP run export mag-scan-line --format csv --repo "$REPO" >/dev/null 2>&1; ok "run export csv" "$?"
$SOP run export mag-scan-line --format markdown --run "$A" --out /tmp/vur-record.md --repo "$REPO" >/dev/null 2>&1; ok "run export markdown" "$?"
[ -s /tmp/vur-record.md ] && { step; pass "markdown record produced"; } || { step; fail "markdown record produced"; }

$SOP export --out /tmp --label vure2e --repo "$REPO" >/dev/null 2>&1; ok "package export" "$?"
[ -d /tmp/export_vure2e ] && { step; pass "package folder produced"; } || { step; fail "package folder produced"; }

# ---- cleanup ---------------------------------------------------------
$SOP run delete mag-scan-line "$A" --repo "$REPO" >/dev/null 2>&1; ok "run delete (dry run)" "$?"
$SOP run delete mag-scan-line "$A" --yes --repo "$REPO" >/dev/null 2>&1; ok "run delete A" "$?"
$SOP run delete sensor-bench-offset "$B" --yes --repo "$REPO" >/dev/null 2>&1; ok "run delete B" "$?"
$SOP run delete preflight-go "$C" --yes --repo "$REPO" >/dev/null 2>&1; ok "run delete C" "$?"

echo "== result: $PASS pass, $FAIL fail =="
[ "$FAIL" -eq 0 ]