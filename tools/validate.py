#!/usr/bin/env python3
"""Validate field-sop content against SPEC.md.

Usage:
    python3 tools/validate.py [--repo PATH] [FILE ...]

Exits non-zero if any error is found. Warnings do not fail the run.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import os
import re
import sys

import soplib as lib


class Report:
    def __init__(self):
        self.items = []

    def error(self, path, message, line=None):
        self.items.append(("error", path, message, line))

    def warn(self, path, message, line=None):
        self.items.append(("warning", path, message, line))

    @property
    def errors(self):
        return [item for item in self.items if item[0] == "error"]

    def render(self, repo_root):
        by_path = {}
        for severity, path, message, line in self.items:
            by_path.setdefault(path, []).append((severity, message, line))
        for path in sorted(by_path):
            rel = os.path.relpath(path, repo_root) if path else "<repo>"
            print(rel)
            for severity, message, line in sorted(by_path[path], key=lambda i: i[2] or 0):
                where = f"line {line}: " if line else ""
                print(f"  {severity}: {where}{message}")
        counts = {}
        for severity, _, _, _ in self.items:
            counts[severity] = counts.get(severity, 0) + 1
        print()
        print(f"{counts.get('error', 0)} error(s), {counts.get('warning', 0)} warning(s)")


def check_formatting(path, text, report):
    if "\r" in text:
        report.error(path, "file contains CR characters; use LF line endings")
    if not text.endswith("\n"):
        report.error(path, "file does not end with a newline")
    elif text.endswith("\n\n"):
        report.error(path, "file ends with more than one newline")
    for index, line in enumerate(text.split("\n"), start=1):
        if line.startswith("\t") or re.match(r"^ +\t", line):
            report.error(path, "indented with a tab", index)


def check_common_front_matter(path, front, kind, report):
    schema = front.get("schema", 1)
    if not isinstance(schema, int) or isinstance(schema, bool):
        report.error(path, "'schema' must be an integer")
    elif schema > lib.SUPPORTED_SCHEMA:
        report.error(
            path,
            f"file declares schema {schema}, but this tooling supports at most "
            f"{lib.SUPPORTED_SCHEMA}; upgrade the tooling (see SPEC-COMPAT.md)",
        )
    known = lib.FRONT_KEYS["common"] | lib.FRONT_KEYS.get(kind, set())
    for extra in sorted(set(front) - known):
        report.warn(
            path, f"unknown front matter key {extra!r}; ignored and preserved (SPEC-COMPAT.md)"
        )
    if front.get("kind") != kind:
        report.error(path, f"front matter 'kind' must be '{kind}', found {front.get('kind')!r}")
    for key in ("title", "version", "updated", "applies_to"):
        if key not in front:
            report.error(path, f"front matter is missing required key '{key}'")
    version = front.get("version")
    if not isinstance(version, int) or isinstance(version, bool):
        report.error(path, "'version' must be an integer")
    updated = front.get("updated")
    is_date = isinstance(updated, (dt.date, dt.datetime)) or (
        isinstance(updated, str) and lib.DATE_RE.match(updated)
    )
    if not is_date:
        report.error(path, "'updated' must be a YYYY-MM-DD date")
    applies_to = lib.as_list(front.get("applies_to"))
    if not applies_to:
        report.error(path, "'applies_to' must list at least one experiment type")
    for value in applies_to:
        if value not in lib.APPLIES_TO:
            report.error(
                path,
                f"'applies_to' value {value!r} is not in the controlled vocabulary "
                f"({', '.join(sorted(lib.APPLIES_TO))})",
            )


def check_captures(path, step, report):
    captures = step.get("captures")
    if captures is None:
        return
    step_id = step.get("id")
    line = step.get("_line")
    if not isinstance(captures, list):
        report.error(path, f"step {step_id!r}: 'captures' must be a list", line)
        return
    seen = set()
    for capture in captures:
        if not isinstance(capture, dict):
            report.error(path, f"step {step_id!r}: each capture must be a mapping", line)
            continue
        key = capture.get("key")
        if not isinstance(key, str) or not lib.CAPTURE_KEY_RE.match(key):
            report.error(path, f"step {step_id!r}: bad capture key {key!r}", line)
        elif key in seen:
            report.error(path, f"step {step_id!r}: duplicate capture key {key!r}", line)
        else:
            seen.add(key)
        if "label" not in capture:
            report.error(path, f"step {step_id!r}: capture {key!r} has no 'label'", line)
        ctype = capture.get("type")
        if ctype not in lib.CAPTURE_TYPE:
            report.error(
                path,
                f"step {step_id!r}: capture {key!r} type {ctype!r} is not one of "
                f"{', '.join(sorted(lib.CAPTURE_TYPE))}",
                line,
            )
        if ctype == "select" and not lib.as_list(capture.get("options")):
            report.error(
                path,
                f"step {step_id!r}: capture {key!r} is a select but declares no 'options'",
                line,
            )
        for extra in sorted(set(capture) - lib.CAPTURE_KEYS):
            report.warn(
                path,
                f"step {step_id!r}: capture {key!r} has unknown key {extra!r}; "
                f"ignored and preserved (SPEC-COMPAT.md)",
                line,
            )
        if ctype == "attach":
            for meaningless in ("unit", "options"):
                if meaningless in capture:
                    report.warn(
                        path,
                        f"step {step_id!r}: capture {key!r} is an attach but declares "
                        f"'{meaningless}', which has no meaning for a file",
                        line,
                    )
        elif "accept" in capture:
            report.warn(
                path,
                f"step {step_id!r}: capture {key!r} declares 'accept' but is not an attach",
                line,
            )
        check_expected(path, step_id, key, capture, line, report)


def is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def check_expected(path, step_id, key, capture, line, report):
    if "expected" not in capture:
        return
    ctype = capture.get("type")
    expected = capture.get("expected")
    where = f"step {step_id!r}: capture {key!r}"
    if ctype not in lib.EXPECTED_TYPE:
        report.error(
            path,
            f"{where} declares 'expected', which is valid only on "
            f"{', '.join(sorted(lib.EXPECTED_TYPE))} (see SPEC.md)",
            line,
        )
        return
    if ctype in ("number", "integer"):
        if not isinstance(expected, dict):
            report.error(
                path,
                f"{where}: 'expected' must be a mapping with 'min' and/or 'max'",
                line,
            )
            return
        for bound in sorted(set(expected) - {"min", "max"}):
            report.error(path, f"{where}: unexpected 'expected' key {bound!r}", line)
        for bound in ("min", "max"):
            if bound in expected and not is_number(expected[bound]):
                report.error(path, f"{where}: 'expected.{bound}' must be a number", line)
        low, high = expected.get("min"), expected.get("max")
        if is_number(low) and is_number(high) and low > high:
            report.error(path, f"{where}: 'expected.min' is greater than 'expected.max'", line)
    elif ctype == "bool":
        if not isinstance(expected, bool):
            report.error(path, f"{where}: 'expected' must be true or false", line)
    elif ctype == "select":
        if expected not in lib.as_list(capture.get("options")):
            report.error(
                path,
                f"{where}: 'expected' {expected!r} is not one of its own options",
                line,
            )


def check_steps(path, steps, report, require_steps=True):
    if not steps and require_steps:
        report.error(path, "no steps defined")
    seen = set()
    for step in steps:
        line = step.get("_line")
        step_id = step.get("id")
        if not isinstance(step_id, str) or not lib.ID_RE.match(step_id):
            report.error(path, f"step id {step_id!r} is not a valid id", line)
        elif step_id in seen:
            report.error(path, f"duplicate step id {step_id!r}", line)
        else:
            seen.add(step_id)
        kind = step.get("kind")
        if kind not in lib.STEP_KIND:
            report.error(
                path,
                f"step {step_id!r}: kind {kind!r} is not one of "
                f"{', '.join(sorted(lib.STEP_KIND))}",
                line,
            )
        severity = step.get("severity", "normal")
        if severity not in lib.SEVERITY:
            report.error(path, f"step {step_id!r}: severity {severity!r} is invalid", line)
        for extra in sorted(set(step) - lib.STEP_KEYS - {"_line", "_title", "_source"}):
            report.warn(
                path,
                f"step {step_id!r}: unknown key {extra!r}; ignored and preserved "
                f"(SPEC-COMPAT.md)",
                line,
            )
        check_captures(path, step, report)
        if not step.get("_title"):
            report.error(path, f"step {step_id!r} has no preceding '##' heading", line)


def check_links(path, doc, repo_root, report):
    lines = doc["body"].split("\n")
    fenced = False
    for offset, line in enumerate(lines):
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        for target in lib.LINK_RE.findall(line):
            if target.startswith("#") or lib.SCHEME_RE.match(target):
                continue
            candidate = target.split("#", 1)[0]
            if not candidate:
                continue
            for base in (os.path.dirname(path), repo_root):
                if os.path.exists(os.path.normpath(os.path.join(base, candidate))):
                    break
            else:
                report.error(
                    path,
                    f"link target does not resolve: {target}",
                    doc["body_start_line"] + offset,
                )


def check_no_prechecked(path, doc, report):
    lines = doc["body"].split("\n")
    fenced = False
    for offset, line in enumerate(lines):
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        match = lib.CHECKBOX_RE.match(line)
        if match and match.group(1) in ("x", "X"):
            report.error(
                path,
                "pre-checked item '- [x]' in a template; use '- [ ]' "
                "(see SPEC.md, checkbox and prose list semantics)",
                doc["body_start_line"] + offset,
            )


def check_procedure(path, repo_root, report):
    doc = lib.parse_document(path, repo_root)
    front = doc["front"]
    check_common_front_matter(path, front, "procedure", report)
    procedure_id = front.get("procedure_id")
    stem = os.path.splitext(os.path.basename(path))[0]
    if procedure_id != stem:
        report.error(path, f"'procedure_id' {procedure_id!r} must match filename {stem!r}")
    if not isinstance(procedure_id, str) or not lib.ID_RE.match(procedure_id):
        report.error(path, f"'procedure_id' {procedure_id!r} is not a valid id")
    check_steps(path, doc["steps"], report)
    if doc["includes"]:
        report.error(path, "a procedure must not contain include markers")
    check_no_prechecked(path, doc, report)
    check_links(path, doc, repo_root, report)


def check_checklist(path, repo_root, report):
    ordered, doc, problems = lib.resolve_checklist(path, repo_root)
    front = doc["front"]
    check_common_front_matter(path, front, "checklist", report)
    sop_id = front.get("sop_id")
    stem = os.path.splitext(os.path.basename(path))[0]
    if sop_id != stem:
        report.error(path, f"'sop_id' {sop_id!r} must match filename {stem!r}")
    if not isinstance(sop_id, str) or not lib.ID_RE.match(sop_id):
        report.error(path, f"'sop_id' {sop_id!r} is not a valid id")
    status = front.get("status")
    if status not in lib.CHECKLIST_STATUS:
        report.error(
            path,
            f"'status' {status!r} must be one of {', '.join(sorted(lib.CHECKLIST_STATUS))}",
        )
    if not lib.as_list(front.get("equipment")):
        report.warn(path, "'equipment' is empty; operators have no packing list")
    for line, message in problems:
        report.error(path, message, line or None)
    check_steps(path, doc["steps"], report, require_steps=False)
    seen = {}
    for step in ordered:
        step_id = step.get("id")
        if step_id in seen:
            report.error(
                path,
                f"step id {step_id!r} from {step.get('_source')} collides with "
                f"{seen[step_id]}; included procedures must not share step ids",
            )
        else:
            seen[step_id] = step.get("_source")
    included = [target for _, target in doc["includes"]]
    for target in sorted({t for t in included if included.count(t) > 1}):
        report.error(path, f"procedure included more than once: {target}")
    if not ordered:
        report.error(path, "checklist resolves to zero steps")
    check_no_prechecked(path, doc, report)
    check_links(path, doc, repo_root, report)
    return ordered


def check_run(path, repo_root, report, run_ids):
    doc = lib.parse_document(path, repo_root)
    front = doc["front"]
    if front.get("kind") != "run":
        report.error(path, "front matter 'kind' must be 'run'")
    required = (
        "run_id", "sop", "sop_version", "operator", "site", "started", "status",
        "deviations_count",
    )
    for key in required:
        if key not in front:
            report.error(path, f"front matter is missing required key '{key}'")

    run_id = front.get("run_id")
    stem = os.path.splitext(os.path.basename(path))[0]
    if run_id != stem:
        report.error(path, f"'run_id' {run_id!r} must match filename {stem!r}")
    if isinstance(run_id, str) and run_ids.get(run_id, path) != path:
        report.error(path, f"'run_id' {run_id!r} is used by more than one run record")

    status = front.get("status")
    if status not in lib.RUN_STATUS:
        report.error(path, f"'status' {status!r} must be one of {', '.join(sorted(lib.RUN_STATUS))}")

    sop = front.get("sop")
    sop_path = os.path.join(repo_root, "checklists", f"{sop}.md")
    checklist_steps = None
    if not isinstance(sop, str) or not os.path.isfile(sop_path):
        report.error(path, f"'sop' {sop!r} does not name an existing checklist")
    else:
        checklist_steps, _, _ = lib.resolve_checklist(sop_path, repo_root)

    sop_version = front.get("sop_version")
    if not isinstance(sop_version, int) or isinstance(sop_version, bool):
        report.error(path, "'sop_version' must be an integer")

    expected_place = os.path.join("runs", str(sop))
    if os.path.dirname(doc["relpath"]) != expected_place:
        report.error(path, f"run record must live in {expected_place}/")

    known_ids = {step.get("id") for step in checklist_steps or []}
    results = doc["results"]
    deviations = 0
    seen_results = set()
    for result in results:
        line = result.get("_line")
        step_id = result.get("step")
        if step_id not in known_ids:
            report.error(
                path,
                f"result references step {step_id!r}, which is not in checklist {sop!r}",
                line,
            )
        elif step_id in seen_results:
            report.error(path, f"more than one result for step {step_id!r}", line)
        else:
            seen_results.add(step_id)
        result_status = result.get("status")
        if result_status not in lib.RESULT_STATUS:
            report.error(
                path,
                f"result {step_id!r}: status {result_status!r} must be one of "
                f"{', '.join(sorted(lib.RESULT_STATUS))}",
                line,
            )
        if result_status == "deviated":
            deviations += 1
        if result_status in ("skipped", "deviated") and not result.get("reason"):
            report.error(path, f"result {step_id!r} is {result_status} but has no 'reason'", line)

    declared = front.get("deviations_count")
    if isinstance(declared, int) and not isinstance(declared, bool) and declared != deviations:
        report.error(
            path,
            f"'deviations_count' is {declared} but {deviations} result(s) are deviated",
        )

    if status == "complete" and checklist_steps is not None:
        for step in checklist_steps:
            if step.get("id") not in seen_results:
                report.error(
                    path, f"run is 'complete' but step {step.get('id')!r} has no result"
                )
        for result in results:
            if result.get("status") == "skipped":
                report.error(
                    path, f"run is 'complete' but step {result.get('step')!r} was skipped"
                )

    if not results:
        report.warn(path, "run record has no step results")

    for entry in lib.as_list(front.get("logs")):
        check_log_entry(path, repo_root, run_id, entry, report)

    check_links(path, doc, repo_root, report)


def check_log_entry(path, repo_root, run_id, entry, report):
    if not isinstance(entry, dict):
        report.error(path, "each 'logs' entry must be a mapping")
        return
    log_path = entry.get("path")
    if not isinstance(log_path, str):
        report.error(path, "'logs' entry has no 'path'")
        return
    expected_prefix = os.path.join("logs", str(run_id))
    if not log_path.startswith(expected_prefix + os.sep):
        report.warn(
            path, f"log {log_path!r} is not under {expected_prefix}{os.sep} (see docs/LOGS.md)"
        )
    digest = entry.get("sha256")
    if not isinstance(digest, str):
        report.warn(path, f"log {log_path!r}: quote 'sha256' so leading zeros survive YAML")
    if entry.get("external") is True:
        if not digest:
            report.error(path, f"log {log_path!r} is marked external but has no 'sha256'")
        return
    absolute = os.path.normpath(os.path.join(repo_root, log_path))
    if not os.path.isfile(absolute):
        report.error(path, f"attached log does not exist: {log_path}")
        return
    with open(absolute, "rb") as fh:
        data = fh.read()
    actual_digest = hashlib.sha256(data).hexdigest()
    if str(digest).lower() != actual_digest:
        report.error(
            path,
            f"log {log_path!r}: sha256 in the record does not match the file "
            f"({digest} != {actual_digest})",
        )
    size = entry.get("size")
    if isinstance(size, int) and not isinstance(size, bool) and size != len(data):
        report.error(
            path,
            f"log {log_path!r}: recorded size is {size} but the file is {len(data)} bytes",
        )


def check_log_directories(repo_root, run_ids, report):
    logs_root = os.path.join(repo_root, "logs")
    if not os.path.isdir(logs_root):
        return
    for name in sorted(os.listdir(logs_root)):
        if not os.path.isdir(os.path.join(logs_root, name)):
            continue
        if name not in run_ids:
            report.error(
                os.path.join(logs_root, name),
                f"log directory {name!r} has no matching run record in runs/",
            )


def check_inbox(path, repo_root, report):
    doc = lib.parse_document(path, repo_root)
    front = doc["front"]
    for key in ("created", "author", "observed_in", "target", "confidence"):
        if key not in front:
            report.error(path, f"inbox entry is missing '{key}'")
    confidence = front.get("confidence")
    if confidence not in ("low", "medium", "high"):
        report.error(path, f"'confidence' {confidence!r} must be low, medium, or high")
    for key in ("observed_in", "target"):
        value = front.get(key)
        if isinstance(value, str) and not os.path.isfile(
            os.path.normpath(os.path.join(repo_root, value))
        ):
            report.error(path, f"'{key}' does not exist: {value}")
    check_links(path, doc, repo_root, report)


def classify(paths, repo_root):
    groups = {"procedure": [], "checklist": [], "run": [], "inbox": []}
    for path in paths:
        rel = os.path.relpath(path, repo_root)
        if rel.startswith("procedures" + os.sep):
            groups["procedure"].append(path)
        elif rel.startswith("checklists" + os.sep):
            groups["checklist"].append(path)
        elif rel.startswith(os.path.join("runs", "_inbox") + os.sep):
            groups["inbox"].append(path)
        elif rel.startswith("runs" + os.sep):
            groups["run"].append(path)
    return groups


def main(argv=None):
    parser = argparse.ArgumentParser(description="Validate field-sop content.")
    parser.add_argument("files", nargs="*", help="specific files to validate")
    parser.add_argument("--repo", default=None, help="repository root")
    args = parser.parse_args(argv)

    repo_root = args.repo or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    repo_root = os.path.abspath(repo_root)
    report = Report()
    run_ids = lib.find_run_records(repo_root)

    if args.files:
        groups = classify([os.path.abspath(name) for name in args.files], repo_root)
        procedures = groups["procedure"]
        checklists = groups["checklist"]
        runs = groups["run"]
        inbox = groups["inbox"]
    else:
        procedures, checklists, runs, inbox = lib.discover(repo_root)

    for path in procedures + checklists + runs + inbox:
        try:
            check_formatting(path, lib.read_text(path), report)
        except lib.SpecError as exc:
            report.error(path, str(exc), exc.line)

    for path, checker in (
        [(p, check_procedure) for p in procedures]
        + [(p, check_checklist) for p in checklists]
        + [(p, lambda a, b, c: check_run(a, b, c, run_ids)) for p in runs]
        + [(p, check_inbox) for p in inbox]
    ):
        try:
            checker(path, repo_root, report)
        except lib.SpecError as exc:
            report.error(path, str(exc), exc.line)

    if not args.files:
        check_log_directories(repo_root, run_ids, report)

    report.render(repo_root)
    return 1 if report.errors else 0


if __name__ == "__main__":
    sys.exit(main())
