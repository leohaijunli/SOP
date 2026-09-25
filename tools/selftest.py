#!/usr/bin/env python3
"""Negative tests for the validator.

For each case: copy the repository to a temporary directory, break exactly one thing,
run the validator, and require that it fails with a recognisable message. A clean copy
must still pass, so the harness cannot pass by failing everything.

Usage:
    python3 tools/selftest.py [--repo PATH] [--keep]
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile

IGNORE = shutil.ignore_patterns(".git", "dist", "__pycache__", "*.pyc", ".venv")


def read(root, rel):
    with open(os.path.join(root, rel), encoding="utf-8") as fh:
        return fh.read()


def write(root, rel, text):
    with open(os.path.join(root, rel), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)


def replace_once(root, rel, old, new):
    text = read(root, rel)
    if old not in text:
        raise AssertionError(f"selftest fixture drifted: {old!r} not found in {rel}")
    write(root, rel, text.replace(old, new, 1))


def append(root, rel, extra):
    write(root, rel, read(root, rel) + extra)


PROC = "procedures/power-on.md"
CAL_SOP = "checklists/mag-sensor-calibration.md"
WALK_SOP = "checklists/ground-walk-survey.md"
CAL_RUN = "runs/mag-sensor-calibration/2026-09-22-bench-cal01.md"
WALK_RUN = "runs/ground-walk-survey/2026-09-24-renfrew-walk01.md"
TESTLINE = "procedures/test-line.md"


def case_missing_key(root):
    replace_once(root, PROC, "\nversion: 1\n", "\n")


def case_duplicate_step_id(root):
    replace_once(root, PROC, "id: poweron-clock", "id: poweron-supply")


def case_bad_step_id(root):
    replace_once(root, PROC, "id: poweron-clock", "id: PowerOn-Clock")


def case_dangling_include(root):
    replace_once(root, WALK_SOP, "<!-- include: procedures/walk-line.md -->",
                 "<!-- include: procedures/walk-lines.md -->")


def case_colliding_include_ids(root):
    replace_once(root, TESTLINE, "id: testline-select", "id: recon-access")


def case_unknown_capture_type(root):
    replace_once(root, PROC, "    type: number\n    unit: V", "    type: numeric\n    unit: V")


def case_expected_on_text(root):
    replace_once(
        root, TESTLINE,
        "    label: Test line identifier\n    type: text\n",
        "    label: Test line identifier\n    type: text\n    expected: L1\n",
    )


def case_expected_not_an_option(root):
    replace_once(root, PROC, "    expected: ok", "    expected: passed")


def case_expected_range_inverted(root):
    replace_once(root, "procedures/warm-up.md", "min: -1.0", "min: 5.0")


def case_prechecked_item(root):
    append(root, PROC, "\n- [x] this should not be pre-checked\n")


def case_schema_from_the_future(root):
    replace_once(root, PROC, "kind: procedure", "kind: procedure\nschema: 99")


def case_corrupt_log_hash(root):
    text = read(root, CAL_RUN)
    old = next(line for line in text.splitlines() if line.strip().startswith("sha256:"))
    write(root, CAL_RUN, text.replace(old, "    sha256: " + "0" * 64, 1))


def case_wrong_deviations_count(root):
    replace_once(root, CAL_RUN, "deviations_count: 1", "deviations_count: 0")


def case_deviated_without_reason(root):
    replace_once(root, CAL_RUN, "reason: Spread of 12.0 nT", "note: Spread of 12.0 nT")


def case_result_step_not_in_checklist(root):
    replace_once(root, CAL_RUN, "step: noise-verdict", "step: noise-summary")


def case_complete_run_with_skipped_step(root):
    replace_once(root, WALK_RUN, "status: partial", "status: complete")
    replace_once(root, WALK_RUN, "deviations_count: 0", "deviations_count: 0")


def case_complete_run_missing_result(root):
    text = read(root, CAL_RUN)
    block = "## hygiene-person\n\n```yaml result\nstep: hygiene-person\nstatus: done\n```\n\n"
    if block not in text:
        raise AssertionError("selftest fixture drifted: hygiene-person result block")
    write(root, CAL_RUN, text.replace(block, "", 1))


def case_missing_log_file(root):
    os.remove(os.path.join(root, "logs/2026-09-22-bench-cal01/heading_sweep.csv"))


def case_orphan_log_directory(root):
    os.makedirs(os.path.join(root, "logs/2026-09-30-nowhere-walk01"), exist_ok=True)


def case_unresolved_link(root):
    append(root, PROC, "\nSee [the missing doc](../docs/nope.md).\n")


def case_crlf(root):
    text = read(root, PROC).replace("\n", "\r\n")
    with open(os.path.join(root, PROC), "w", encoding="utf-8", newline="") as fh:
        fh.write(text)


CASES = [
    ("missing required front matter key", case_missing_key, "missing required key 'version'"),
    ("duplicate step id", case_duplicate_step_id, "duplicate step id 'poweron-supply'"),
    ("invalid step id", case_bad_step_id, "is not a valid id"),
    ("dangling include target", case_dangling_include, "include target does not exist"),
    ("colliding step ids across includes", case_colliding_include_ids, "collides with"),
    ("unknown capture type", case_unknown_capture_type, "is not one of"),
    ("expected on a text capture", case_expected_on_text, "valid only on"),
    ("expected not among its own options", case_expected_not_an_option, "not one of its own options"),
    ("inverted expected range", case_expected_range_inverted, "greater than 'expected.max'"),
    ("pre-checked item in a template", case_prechecked_item, "pre-checked item"),
    ("schema from the future", case_schema_from_the_future, "supports at most"),
    ("attachment hash mismatch", case_corrupt_log_hash, "does not match the file"),
    ("deviations_count disagrees", case_wrong_deviations_count, "result(s) are deviated"),
    ("deviated without a reason", case_deviated_without_reason, "has no 'reason'"),
    ("result cites an unknown step", case_result_step_not_in_checklist, "not in checklist"),
    ("complete run with a skipped step", case_complete_run_with_skipped_step, "was skipped"),
    ("complete run missing a result", case_complete_run_missing_result, "has no result"),
    ("missing attached log", case_missing_log_file, "attached log does not exist"),
    ("orphan log directory", case_orphan_log_directory, "has no matching run record"),
    ("unresolved relative link", case_unresolved_link, "link target does not resolve"),
    ("CRLF line endings", case_crlf, "CR characters"),
]


def run_validator(root):
    proc = subprocess.run(
        [sys.executable, os.path.join(root, "tools", "validate.py"), "--repo", root],
        capture_output=True, text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


def main(argv=None):
    parser = argparse.ArgumentParser(description="Validator negative tests.")
    parser.add_argument("--repo", default=None, help="repository root")
    parser.add_argument("--keep", action="store_true", help="keep temporary copies")
    args = parser.parse_args(argv)

    repo_root = args.repo or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    repo_root = os.path.abspath(repo_root)
    failures = []
    scratch = tempfile.mkdtemp(prefix="field-sop-selftest-")

    try:
        clean = os.path.join(scratch, "clean")
        shutil.copytree(repo_root, clean, ignore=IGNORE)
        code, output = run_validator(clean)
        if code != 0:
            print("FAIL  control: a clean copy must validate")
            print(output)
            failures.append("control")
        else:
            print("ok    control: clean copy validates")

        for index, (name, mutate, expected) in enumerate(CASES):
            root = os.path.join(scratch, f"case{index:02d}")
            shutil.copytree(repo_root, root, ignore=IGNORE)
            try:
                mutate(root)
            except Exception as exc:
                print(f"FAIL  {name}: could not apply mutation: {exc}")
                failures.append(name)
                continue
            code, output = run_validator(root)
            if code == 0:
                print(f"FAIL  {name}: validator accepted the broken content")
                failures.append(name)
            elif expected not in output:
                print(f"FAIL  {name}: failed for the wrong reason")
                print(f"      expected message containing: {expected!r}")
                print("      " + output.strip().replace("\n", "\n      "))
                failures.append(name)
            else:
                print(f"ok    {name}")
            if not args.keep:
                shutil.rmtree(root, ignore_errors=True)
    finally:
        if args.keep:
            print(f"\ntemporary copies kept in {scratch}")
        else:
            shutil.rmtree(scratch, ignore_errors=True)

    print()
    total = len(CASES) + 1
    print(f"{total - len(failures)} of {total} checks passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
