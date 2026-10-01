# Virtual-user functional traversal

A disposable working-copy repository driven through the full field-sop CLI, the
way an operator would work a survey day. The run is repeatable and never touches
the live working copy.

## How to run

```sh
cargo build -p sop-cli          # once
bash runs/virtual-user-e2e.sh   # rebuilds a throwaway repo under /tmp/vu-repo
```

`--repo PATH` and `--keep` adjust the target. The script exits non-zero if any
step fails.

## Test plans seeded by the traversal

Three plans (each with a plan.md and a checklist case) are generated into the
throwaway repo so the run commands have targets:

| Plan        | Checklist        | sop_id             |
|-------------|------------------|--------------------|
| mag-scan    | Walk a Survey Line | `mag-scan-line`   |
| sensor-bench| Offset Correction  | `sensor-bench-offset` |
| preflight   | Go/No-Go           | `preflight-go`    |

## Coverage and result

**CLI end-to-end: 39 / 39 PASS** — every command below executed and its exit code
checked against the throwaway repo.

| Area | Commands exercised |
|------|--------------------|
| Inspection | status, project (show/set), settings (show/path/set/unset), validate, index |
| Run lifecycle | run start / recover / end (complete+pass, partial+fail, aborted+inconclusive) |
| Recording | done, skip, deviate, capture (with unit), clear, checkbox, step note, run note |
| Attachments | attach log, attach photo |
| Analysis | run deviations, run timeline, run export (csv, markdown) |
| Packaging | `sop export` package folder + produced artifact check |
| Cleanup | run delete (dry run + `--yes`) |

**Backend unit/integration: all pass** — `cargo test --workspace` (200+ tests,
0 failed): authoring, devices vectors, format, atomic, git, manifest, negative,
project, run, run_amend, settings, summary, testplan, export, plus sop-app and
sop-cli unit tests.

**Frontend: pass** — `npm run test:devices` (9/9 vectors) and `svelte-check`
(0 errors, 0 warnings).

## GUI views (need a human at the window)

The desktop app renders Run / History / Browse / Edit / Settings / Test Plans
views that cannot be driven headlessly here. The CLI traversal covers the same
backend commands those views call; the view-specific wiring (layout, the run
form, history filters, the settings editors) is the part to click through by hand.

Artifacts left for inspection: `/tmp/vu-repo` (the throwaway working copy with a
committed run history), `/tmp/export_vure2e` (the export package).