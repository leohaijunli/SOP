#!/usr/bin/env python3
"""Build dist/manifest.json from the Markdown content.

The field app loads one file instead of walking the repository, so it can work from a
single fetch or a local directory copy. `dist/` is generated, never edited by hand.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import sys

import soplib as lib


def json_safe(value):
    if isinstance(value, (dt.date, dt.datetime)):
        return value.isoformat()
    if isinstance(value, dict):
        return {
            str(key): json_safe(item)
            for key, item in value.items()
            if not str(key).startswith("_")
        }
    if isinstance(value, (list, tuple)):
        return [json_safe(item) for item in value]
    return value


def public_step(step):
    return {
        "id": step.get("id"),
        "title": step.get("_title"),
        "kind": step.get("kind"),
        "severity": step.get("severity", "normal"),
        "deprecated": bool(step.get("deprecated", False)),
        "captures": json_safe(step.get("captures", [])),
        "source": step.get("_source"),
    }


def build(repo_root):
    procedures, checklists, runs, _ = lib.discover(repo_root)
    manifest = {
        "schema": 1,
        "procedures": [],
        "checklists": [],
        "runs": [],
    }

    for path in procedures:
        doc = lib.parse_document(path, repo_root)
        front = doc["front"]
        manifest["procedures"].append({
            "procedure_id": front.get("procedure_id"),
            "title": front.get("title"),
            "version": front.get("version"),
            "updated": json_safe(front.get("updated")),
            "applies_to": lib.as_list(front.get("applies_to")),
            "tags": lib.as_list(front.get("tags")),
            "path": doc["relpath"],
            "steps": [public_step(step) for step in doc["steps"]],
        })

    for path in checklists:
        ordered, doc, problems = lib.resolve_checklist(path, repo_root)
        front = doc["front"]
        manifest["checklists"].append({
            "sop_id": front.get("sop_id"),
            "title": front.get("title"),
            "version": front.get("version"),
            "updated": json_safe(front.get("updated")),
            "status": front.get("status"),
            "applies_to": lib.as_list(front.get("applies_to")),
            "equipment": lib.as_list(front.get("equipment")),
            "path": doc["relpath"],
            "step_count": len(ordered),
            "unresolved_includes": [message for _, message in problems],
            "steps": [public_step(step) for step in ordered],
        })

    for path in runs:
        doc = lib.parse_document(path, repo_root)
        front = doc["front"]
        manifest["runs"].append({
            "run_id": front.get("run_id"),
            "sop": front.get("sop"),
            "sop_version": front.get("sop_version"),
            "operator": front.get("operator"),
            "site": front.get("site"),
            "started": json_safe(front.get("started")),
            "status": front.get("status"),
            "deviations_count": front.get("deviations_count", 0),
            "path": doc["relpath"],
            "step_count": len(doc["results"]),
        })

    manifest["procedures"].sort(key=lambda item: str(item["procedure_id"]))
    manifest["checklists"].sort(key=lambda item: str(item["sop_id"]))
    manifest["runs"].sort(key=lambda item: str(item["run_id"]))
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description="Build dist/manifest.json.")
    parser.add_argument("--repo", default=None, help="repository root")
    parser.add_argument("--out", default=None, help="output path")
    args = parser.parse_args(argv)

    repo_root = args.repo or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    repo_root = os.path.abspath(repo_root)
    out = args.out or os.path.join(repo_root, "dist", "manifest.json")

    manifest = build(repo_root)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(manifest, fh, indent=2, ensure_ascii=False)
        fh.write("\n")

    print(
        f"wrote {os.path.relpath(out, repo_root)}: "
        f"{len(manifest['procedures'])} procedure(s), "
        f"{len(manifest['checklists'])} checklist(s), "
        f"{len(manifest['runs'])} run record(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
