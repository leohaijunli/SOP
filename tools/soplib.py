"""Shared parsing helpers for the field-sop content format.

Implements the parsing half of SPEC.md. `validate.py` and `build_index.py` both
depend on this module; neither re-implements the format.
"""

from __future__ import annotations

import os
import re

import yaml

TYPES = ("procedure", "checklist", "run")

APPLIES_TO = {"calibration", "ground-survey", "interference", "navigation", "uav-survey"}
SEVERITY = {"info", "normal", "critical"}
STEP_KIND = {"check", "measure", "select", "note", "gate"}
CAPTURE_TYPE = {"text", "number", "integer", "bool", "select", "datetime", "duration", "attach"}
EXPECTED_TYPE = {"number", "integer", "select", "bool"}
SUPPORTED_SCHEMA = 1

CHECKBOX_RE = re.compile(r"^\s*[-*+]\s+\[([ xX])\]\s")

FRONT_KEYS = {
    "common": {"kind", "title", "version", "updated", "applies_to", "schema", "tags", "notes"},
    "procedure": {"procedure_id"},
    "checklist": {"sop_id", "status", "equipment"},
    "run": {
        "run_id", "sop", "sop_version", "sop_commit", "operator", "site", "started",
        "ended", "status", "sensor", "hardware", "conditions", "logs",
        "deviations_count",
    },
}

STEP_KEYS = {"id", "kind", "severity", "deprecated", "captures"}
CAPTURE_KEYS = {"key", "label", "type", "unit", "required", "options", "expected", "accept"}
RESULT_KEYS = {"step", "status", "reason", "captures"}
CHECKLIST_STATUS = {"draft", "active", "retired"}
RUN_STATUS = {"complete", "partial", "aborted"}
RESULT_STATUS = {"done", "skipped", "deviated"}

ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
CAPTURE_KEY_RE = re.compile(r"^[a-z0-9][a-z0-9_]*$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
INCLUDE_RE = re.compile(r"^<!--\s*include:\s*(\S+?)\s*-->\s*$")
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
SCHEME_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


class SpecError(Exception):
    """A structural problem that makes the file unparseable."""

    def __init__(self, message, line=None):
        super().__init__(message)
        self.line = line


def read_text(path):
    with open(path, "rb") as fh:
        raw = fh.read()
    if raw.startswith(b"\xef\xbb\xbf"):
        raise SpecError("file starts with a UTF-8 BOM")
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise SpecError(f"file is not valid UTF-8: {exc}") from exc


def split_front_matter(text):
    """Return (front_matter_dict, body, body_start_line)."""
    lines = text.split("\n")
    if not lines or lines[0].strip() != "---":
        raise SpecError("file does not start with a '---' front matter delimiter", 1)
    for index in range(1, len(lines)):
        if lines[index].strip() == "---":
            raw = "\n".join(lines[1:index])
            try:
                data = yaml.safe_load(raw)
            except yaml.YAMLError as exc:
                raise SpecError(f"front matter is not valid YAML: {exc}") from exc
            if data is None:
                data = {}
            if not isinstance(data, dict):
                raise SpecError("front matter must be a YAML mapping")
            return data, "\n".join(lines[index + 1:]), index + 2
    raise SpecError("front matter is not closed by a '---' delimiter")


def extract_fences(body, body_start_line):
    """Return every fenced block that starts at column 0.

    Each entry is {info, content, start_line, end_line}.
    """
    lines = body.split("\n")
    blocks = []
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("```"):
            info = line[3:].strip()
            close = index + 1
            while close < len(lines) and lines[close].strip() != "```":
                close += 1
            if close >= len(lines):
                raise SpecError("unclosed code fence", body_start_line + index)
            blocks.append({
                "info": info,
                "content": "\n".join(lines[index + 1:close]),
                "start_line": body_start_line + index,
                "end_line": body_start_line + close,
            })
            index = close + 1
        else:
            index += 1
    return blocks


def parse_yaml_block(block, expect):
    try:
        data = yaml.safe_load(block["content"])
    except yaml.YAMLError as exc:
        raise SpecError(
            f"'{expect}' block is not valid YAML: {exc}", block["start_line"]
        ) from exc
    if data is None:
        data = {}
    if not isinstance(data, dict):
        raise SpecError(f"'{expect}' block must be a YAML mapping", block["start_line"])
    return data


def headings(body, body_start_line):
    """Return [(line_number, level, title)] for ATX headings outside code fences."""
    lines = body.split("\n")
    fenced = False
    out = []
    for offset, line in enumerate(lines):
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        match = re.match(r"^(#{2,6})\s+(.*?)\s*$", line)
        if match:
            out.append((body_start_line + offset, len(match.group(1)), match.group(2)))
    return out


def include_markers(body, body_start_line):
    """Return [(line_number, path)] for include markers outside code fences."""
    lines = body.split("\n")
    fenced = False
    out = []
    for offset, line in enumerate(lines):
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        match = INCLUDE_RE.match(line)
        if match:
            out.append((body_start_line + offset, match.group(1)))
    return out


def parse_document(path, repo_root):
    """Parse one content file into a dict of its parts."""
    text = read_text(path)
    front, body, body_start_line = split_front_matter(text)
    fences = extract_fences(body, body_start_line)

    steps = []
    results = []
    head_list = headings(body, body_start_line)

    for block in fences:
        if block["info"] == "yaml step":
            step = parse_yaml_block(block, "yaml step")
            title = None
            for line_number, level, text_title in head_list:
                if level == 2 and line_number < block["start_line"]:
                    title = text_title
                elif line_number > block["start_line"]:
                    break
            step["_line"] = block["start_line"]
            step["_title"] = title
            steps.append(step)
        elif block["info"] == "yaml result":
            result = parse_yaml_block(block, "yaml result")
            result["_line"] = block["start_line"]
            results.append(result)

    return {
        "path": path,
        "relpath": os.path.relpath(path, repo_root),
        "text": text,
        "front": front,
        "body": body,
        "body_start_line": body_start_line,
        "steps": steps,
        "results": results,
        "includes": include_markers(body, body_start_line),
        "headings": head_list,
    }


def load_procedure(path, repo_root):
    doc = parse_document(path, repo_root)
    steps = []
    for step in doc["steps"]:
        entry = dict(step)
        entry["_source"] = doc["relpath"]
        steps.append(entry)
    return steps, doc


def resolve_checklist(path, repo_root):
    """Expand include markers in place, returning (ordered_steps, doc, problems).

    problems is a list of (line, message) for markers that could not be resolved.
    """
    doc = parse_document(path, repo_root)
    items = []
    for line, target in doc["includes"]:
        items.append((line, "include", target))
    for step in doc["steps"]:
        items.append((step["_line"], "step", step))
    items.sort(key=lambda item: item[0])

    ordered = []
    problems = []
    procedures_root = os.path.normpath(os.path.join(repo_root, "procedures"))
    for line, kind, payload in items:
        if kind == "step":
            entry = dict(payload)
            entry["_source"] = doc["relpath"]
            ordered.append(entry)
            continue
        target = payload
        resolved = os.path.normpath(os.path.join(repo_root, target))
        if not os.path.isfile(resolved):
            problems.append((line, f"include target does not exist: {target}"))
            continue
        if not resolved.startswith(procedures_root + os.sep):
            problems.append((line, f"include target is not a procedure: {target}"))
            continue
        try:
            steps, _ = load_procedure(resolved, repo_root)
        except SpecError as exc:
            where = f" line {exc.line}" if exc.line else ""
            problems.append((line, f"included procedure {target} is unreadable at{where}: {exc}"))
            continue
        ordered.extend(steps)
    return ordered, doc, problems


def discover(repo_root):
    """Return (procedures, checklists, runs, inbox) lists of absolute paths."""
    def glob_dir(*parts):
        directory = os.path.join(repo_root, *parts)
        if not os.path.isdir(directory):
            return []
        return sorted(
            os.path.join(directory, name)
            for name in os.listdir(directory)
            if name.endswith(".md") and name != "README.md"
        )

    runs = []
    runs_dir = os.path.join(repo_root, "runs")
    if os.path.isdir(runs_dir):
        for name in sorted(os.listdir(runs_dir)):
            if name == "_inbox":
                continue
            runs.extend(glob_dir("runs", name))

    return (
        glob_dir("procedures"),
        glob_dir("checklists"),
        runs,
        glob_dir("runs", "_inbox"),
    )


def find_run_records(repo_root):
    """Map run_id -> path for every run record in the repository."""
    _, _, runs, _ = discover(repo_root)
    return {os.path.splitext(os.path.basename(p))[0]: p for p in runs}


def as_list(value):
    if value is None:
        return []
    if isinstance(value, (list, tuple)):
        return list(value)
    return [value]
