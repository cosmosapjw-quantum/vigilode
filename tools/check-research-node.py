#!/usr/bin/env python3
"""Enforce the research-node rule and the append-only research ledger (audit F-024).

A research node is a new top-level directory under ``research/``. It consists of:

* ``PREREGISTRATION.md``, written before the run, with the results appended below it;
* raw outputs, including at least one results file with numeric rows;
* at least one row in ``research/LEDGER.jsonl`` whose ``outputs_sha256`` covers files in
  that directory.

Checks:

(a) with ``--base REF``: every top-level ``research/<dir>`` tracked now but absent at
    ``merge-base(REF, HEAD)`` must have ``PREREGISTRATION.md``, a numeric results file and
    a ledger row covering a numeric results file in the directory.  Directories already
    present in the tree of the commit that first added ``research/LEDGER.jsonl`` are
    frozen history and exempt; their receipt families are not migrated.  A node listed in
    the exemption file (default ``tools/research_node_exemptions.txt``, one
    ``<node> <reason>`` per line, reason mandatory) is exempt from the
    ``PREREGISTRATION.md`` requirement only; it still needs a numeric results file and a
    covering ledger row.  An entry naming no tracked node is an error;
(b) with ``--base REF``: the ledger is append-only, so the ledger lines present at the
    merge base must be an exact line-by-line prefix of the current ledger;
(c) always: every ledger row validates against the single row schema, and every
    ``outputs_sha256`` entry matches the SHA-256 of the file in the working tree;
(d) always: the stall rule.  Nodes are ordered by their first ledger row, using
    (``date``, line order).  A node "has a numeric result" when a ledger row that no later
    row supersedes gives it verdict PASS or FAIL and covers a numeric results file in it.
    After two consecutive nodes without a numeric result, any further node without one
    is blocked; such a node is reported as a failure.  The stall ends when a node, new or
    earlier, gets a PASS/FAIL row on a numeric result.  With ``--base``, only nodes whose
    first ledger row is new relative to the merge base are reported.

Exit status: 0 on pass, 1 on a check failure, 2 on a usage or git error.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import io
import json
import math
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

LEDGER = "research/LEDGER.jsonl"
PREREG = "PREREGISTRATION.md"
FIELDS = (
    "id",
    "date",
    "commit",
    "command",
    "profile",
    "inputs_sha256",
    "outputs_sha256",
    "claim",
    "verdict",
    "supersedes",
)
VERDICTS = ("PASS", "FAIL", "INCONCLUSIVE", "INVALID")
NUMERIC_VERDICTS = ("PASS", "FAIL")
RESULT_SUFFIXES = (".json", ".jsonl", ".csv", ".tsv", ".txt")
ID_RE = re.compile(r"^L-[0-9]{4,}$")
DATE_RE = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
NUMBER_RE = re.compile(r"(?<![\w.])[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?(?![\w.])")
MAX_CLAIM_CHARS = 600
DEFAULT_EXEMPTIONS = "tools/research_node_exemptions.txt"
EXEMPT_LINE_RE = re.compile(r"^([A-Za-z0-9_.-]+)\s+(\S.*)$")


class GitError(Exception):
    pass


def git(repo: Path, *args: str) -> str:
    proc = subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True)
    if proc.returncode != 0:
        raise GitError(f"git {' '.join(args)}: {proc.stderr.strip()}")
    return proc.stdout


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def _json_has_number(value: object) -> bool:
    if isinstance(value, bool):
        return False
    if isinstance(value, (int, float)):
        return True
    if isinstance(value, dict):
        return any(_json_has_number(v) for v in value.values())
    if isinstance(value, list):
        return any(_json_has_number(v) for v in value)
    return False


def _is_number(cell: str) -> bool:
    try:
        return math.isfinite(float(cell.strip()))
    except ValueError:
        return False


def is_numeric_results_file(path: Path) -> bool:
    """True for a json/jsonl/csv/tsv/txt file that carries at least one numeric value."""
    if path.suffix not in RESULT_SUFFIXES or not path.is_file():
        return False
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return False
    if path.suffix == ".json":
        try:
            return _json_has_number(json.loads(text))
        except ValueError:
            return False
    if path.suffix == ".jsonl":
        for line in text.splitlines():
            try:
                if line.strip() and _json_has_number(json.loads(line)):
                    return True
            except ValueError:
                return False
        return False
    if path.suffix in (".csv", ".tsv"):
        delimiter = "\t" if path.suffix == ".tsv" else ","
        return any(
            any(_is_number(cell) for cell in row)
            for row in csv.reader(io.StringIO(text), delimiter=delimiter)
        )
    return any(NUMBER_RE.search(line) for line in text.splitlines())


def node_of(path: str) -> str | None:
    parts = PurePosixPath(path).parts
    if len(parts) >= 3 and parts[0] == "research":
        return parts[1]
    return None


def _valid_rel_path(path: object) -> bool:
    if not isinstance(path, str) or not path or path.startswith("/") or "\\" in path:
        return False
    return ".." not in PurePosixPath(path).parts


def validate_row(row: object, number: int, seen_ids: list[str]) -> list[str]:
    where = f"{LEDGER}:{number}"
    if not isinstance(row, dict):
        return [f"{where}: row is not a JSON object"]
    errors: list[str] = []
    keys = set(row)
    missing = [f for f in FIELDS if f not in keys]
    extra = sorted(keys - set(FIELDS))
    if missing:
        errors.append(f"{where}: missing fields {missing}")
    if extra:
        errors.append(f"{where}: unknown fields {extra}")
    rid = row.get("id")
    if not isinstance(rid, str) or not ID_RE.match(rid):
        errors.append(f"{where}: id must match L-NNNN")
    elif rid in seen_ids:
        errors.append(f"{where}: duplicate id {rid}")
    date = row.get("date")
    try:
        if not isinstance(date, str) or not DATE_RE.match(date):
            raise ValueError
        dt.date.fromisoformat(date)
    except ValueError:
        errors.append(f"{where}: date must be YYYY-MM-DD")
    if not isinstance(row.get("commit"), str) or not COMMIT_RE.match(row["commit"]):
        errors.append(f"{where}: commit must be a full 40-hex lowercase SHA")
    for field in ("command", "profile"):
        if not isinstance(row.get(field), str) or not row[field].strip():
            errors.append(f"{where}: {field} must be a non-empty string")
    for field in ("inputs_sha256", "outputs_sha256"):
        value = row.get(field)
        if not isinstance(value, dict):
            errors.append(f"{where}: {field} must be an object mapping path to sha256")
            continue
        if field == "outputs_sha256" and not value:
            errors.append(f"{where}: outputs_sha256 must name at least one file")
        for path, digest in value.items():
            if not _valid_rel_path(path):
                errors.append(f"{where}: {field} has an invalid repository path {path!r}")
            if not isinstance(digest, str) or not SHA256_RE.match(digest):
                errors.append(f"{where}: {field}[{path!r}] must be 64-hex lowercase")
    claim = row.get("claim")
    if not isinstance(claim, str) or not claim.strip():
        errors.append(f"{where}: claim must be a non-empty string")
    elif "\n" in claim or len(claim) > MAX_CLAIM_CHARS:
        errors.append(f"{where}: claim must be one sentence on one line (<= {MAX_CLAIM_CHARS} chars)")
    if row.get("verdict") not in VERDICTS:
        errors.append(f"{where}: verdict must be one of {'|'.join(VERDICTS)}")
    elif (
        isinstance(row.get("command"), str)
        and row["command"].startswith("UNRECORDED")
        and row["verdict"] not in ("INCONCLUSIVE", "INVALID")
    ):
        errors.append(f"{where}: an UNRECORDED command requires verdict INCONCLUSIVE or INVALID")
    sup = row.get("supersedes")
    if sup is not None and (not isinstance(sup, str) or sup not in seen_ids):
        errors.append(f"{where}: supersedes must be null or the id of an earlier row")
    return errors


def parse_ledger(lines: list[str]) -> tuple[list[tuple[int, dict]], list[str]]:
    rows: list[tuple[int, dict]] = []
    errors: list[str] = []
    seen: list[str] = []
    for number, line in enumerate(lines, 1):
        if not line.strip():
            errors.append(f"{LEDGER}:{number}: blank line (one JSON object per line)")
            continue
        try:
            row = json.loads(line)
        except ValueError as exc:
            errors.append(f"{LEDGER}:{number}: invalid JSON: {exc}")
            continue
        row_errors = validate_row(row, number, seen)
        errors.extend(row_errors)
        if isinstance(row, dict):
            if isinstance(row.get("id"), str):
                seen.append(row["id"])
            if not row_errors:
                rows.append((number, row))
    return rows, errors


def check_outputs(repo: Path, rows: list[tuple[int, dict]]) -> list[str]:
    errors: list[str] = []
    for number, row in rows:
        for path, digest in row["outputs_sha256"].items():
            file_path = repo / path
            if not file_path.is_file():
                errors.append(f"{LEDGER}:{number}: {row['id']} output {path} does not exist")
            elif sha256_file(file_path) != digest:
                errors.append(f"{LEDGER}:{number}: {row['id']} output {path} sha256 mismatch")
    return errors


def numeric_nodes_of_row(repo: Path, row: dict) -> set[str]:
    nodes: set[str] = set()
    for path in row["outputs_sha256"]:
        node = node_of(path)
        if node and is_numeric_results_file(repo / path):
            nodes.add(node)
    return nodes


def stall_violations(repo: Path, rows: list[tuple[int, dict]], new_ids: set[str] | None) -> list[str]:
    superseded = {row["supersedes"] for _, row in rows if row["supersedes"]}
    ordered = sorted(rows, key=lambda item: (item[1]["date"], item[0]))
    first_row: dict[str, dict] = {}
    order: list[str] = []
    numeric: dict[str, bool] = {}
    for _, row in ordered:
        row_nodes = sorted({n for n in map(node_of, row["outputs_sha256"]) if n})
        for node in row_nodes:
            if node not in first_row:
                first_row[node] = row
                order.append(node)
                numeric[node] = False
        if row["id"] in superseded or row["verdict"] not in NUMERIC_VERDICTS:
            continue
        for node in numeric_nodes_of_row(repo, row):
            numeric[node] = True
    errors: list[str] = []
    streak: list[str] = []
    for node in order:
        if numeric[node]:
            streak = []
            continue
        if len(streak) >= 2 and (new_ids is None or first_row[node]["id"] in new_ids):
            errors.append(
                f"stall rule: node research/{node} (first row {first_row[node]['id']}) has no "
                f"PASS/FAIL numeric result after two consecutive nodes without one "
                f"({', '.join('research/' + n for n in streak[-2:])}); "
                "produce a numeric result before opening another node"
            )
        streak.append(node)
    return errors


def tracked_research_files(repo: Path) -> list[str]:
    return [p for p in git(repo, "ls-files", "-z", "--", "research").split("\0") if p]


def frozen_dirs(repo: Path) -> set[str]:
    """Top-level research dirs present when the ledger was introduced (exempt history)."""
    adds = git(repo, "log", "--diff-filter=A", "--format=%H", "HEAD", "--", LEDGER).split()
    rev = adds[-1] if adds else "HEAD"
    if not _has_path(repo, rev, "research"):
        return set()
    names = git(repo, "ls-tree", "-d", "--name-only", f"{rev}:research")
    return {n for n in names.splitlines() if n}


def base_state(repo: Path, base: str) -> tuple[str, set[str], list[str]]:
    merge_base = git(repo, "merge-base", base, "HEAD").strip()
    names = git(repo, "ls-tree", "-d", "--name-only", f"{merge_base}:research") if _has_path(
        repo, merge_base, "research"
    ) else ""
    base_dirs = {n for n in names.splitlines() if n}
    ledger_lines: list[str] = []
    if _has_path(repo, merge_base, LEDGER):
        ledger_lines = git(repo, "show", f"{merge_base}:{LEDGER}").splitlines()
    return merge_base, base_dirs, ledger_lines


def _has_path(repo: Path, rev: str, path: str) -> bool:
    proc = subprocess.run(
        ["git", "cat-file", "-e", f"{rev}:{path}"], cwd=repo, capture_output=True
    )
    return proc.returncode == 0


def load_exemptions(path: Path) -> tuple[dict[str, str], list[str]]:
    """Nodes exempt from the pre-registration requirement, each with its reason."""
    entries: dict[str, str] = {}
    errors: list[str] = []
    if not path.exists():
        return entries, errors
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        match = EXEMPT_LINE_RE.match(line)
        if not match:
            errors.append(f"{path.name}:{number}: expected '<node> <reason>'")
            continue
        node, reason = match.groups()
        if node in entries:
            errors.append(f"{path.name}:{number}: duplicate exemption {node}")
        entries[node] = reason
    return entries, errors


def append_only_errors(base_lines: list[str], current_lines: list[str]) -> list[str]:
    errors: list[str] = []
    for index, base_line in enumerate(base_lines):
        if index >= len(current_lines):
            errors.append(
                f"{LEDGER}: append-only violation: {len(base_lines) - index} row(s) present at "
                f"the merge base were removed (from base line {index + 1})"
            )
            break
        if current_lines[index] != base_line:
            errors.append(
                f"{LEDGER}:{index + 1}: append-only violation: row present at the merge base "
                "was edited, removed or reordered"
            )
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--base", help="base ref; enables new-node and append-only checks")
    parser.add_argument("--repo", type=Path, default=None, help="repository root (default: current toplevel)")
    parser.add_argument(
        "--exemptions", default=DEFAULT_EXEMPTIONS, help="pre-registration exemption file relative to the repo root"
    )
    args = parser.parse_args(argv)

    try:
        repo = args.repo or Path(git(Path.cwd(), "rev-parse", "--show-toplevel").strip())
        ledger_path = repo / LEDGER
        current_lines = (
            ledger_path.read_text(encoding="utf-8").splitlines() if ledger_path.exists() else []
        )
        rows, errors = parse_ledger(current_lines)
        errors.extend(check_outputs(repo, rows))
        exemptions, exemption_errors = load_exemptions(repo / args.exemptions)
        errors.extend(exemption_errors)
        tracked_nodes = {node for node in map(node_of, tracked_research_files(repo)) if node}
        for node in sorted(set(exemptions) - tracked_nodes):
            errors.append(f"{args.exemptions}: exemption for research/{node}/, which is not a tracked node")

        new_ids: set[str] | None = None
        new_nodes: list[str] = []
        if args.base:
            merge_base, base_dirs, base_lines = base_state(repo, args.base)
            errors.extend(append_only_errors(base_lines, current_lines))
            base_ids = set()
            for line in base_lines:
                try:
                    base_ids.add(json.loads(line).get("id"))
                except (ValueError, AttributeError):
                    pass
            new_ids = {row["id"] for _, row in rows} - base_ids
            tracked = tracked_research_files(repo)
            current_dirs: dict[str, list[str]] = {}
            for path in tracked:
                node = node_of(path)
                if node:
                    current_dirs.setdefault(node, []).append(path)
            new_nodes = sorted(set(current_dirs) - base_dirs - frozen_dirs(repo))
            for node in new_nodes:
                files = current_dirs[node]
                prefix = f"research/{node}/"
                if f"{prefix}{PREREG}" not in files and node not in exemptions:
                    errors.append(f"new node {prefix}: missing {PREREG} (pre-registration)")
                numeric_files = {p for p in files if is_numeric_results_file(repo / p)}
                if not numeric_files:
                    errors.append(
                        f"new node {prefix}: no results file with numeric rows "
                        f"({'/'.join(RESULT_SUFFIXES)})"
                    )
                covering = [
                    row for _, row in rows if any(p.startswith(prefix) for p in row["outputs_sha256"])
                ]
                if not covering:
                    errors.append(f"new node {prefix}: no {LEDGER} row whose outputs_sha256 covers it")
                elif not any(p in numeric_files for row in covering for p in row["outputs_sha256"]):
                    errors.append(
                        f"new node {prefix}: ledger rows cover it but no covered output is a "
                        "numeric results file"
                    )
        errors.extend(stall_violations(repo, rows, new_ids))
    except GitError as exc:
        print(f"check-research-node: {exc}", file=sys.stderr)
        return 2

    for error in errors:
        print(f"FAIL: {error}")
    verdict = "FAIL" if errors else "PASS"
    scope = f", base={args.base}, new nodes={new_nodes or 'none'}" if args.base else ""
    print(f"check-research-node: {verdict} ({len(current_lines)} ledger rows{scope})")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
