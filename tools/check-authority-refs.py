#!/usr/bin/env python3
"""Fail when research/docs cite a commit SHA as authority that no remote ref contains.

Audit finding F-025 (research/adversarial_audit_20260927): any document that names a
commit as source authority must cite a commit reachable from a remote ref, so that an
independent auditor can rebuild the state it describes from a fresh clone.

Extraction rule (documented in docs/RESEARCH_LEDGER.md):

* Scope: tracked files under the scanned paths (default ``research`` and ``docs``) whose
  suffix is ``.md``, ``.json``, ``.jsonl`` or ``.toml``.
* A candidate is a 40-character lowercase hex token that is not part of a longer hex run
  (so SHA-256 digests are never mistaken for Git object ids).
* JSON/JSONL: a candidate inside a string value is an authority citation when a token of
  the key path (keys split on ``_``, ``-``, ``.``, spaces and camelCase) or a word of the
  string itself is an authority keyword.  Unparseable JSON falls back to the text rule.
* Markdown/TOML/other text: a candidate is an authority citation when its own line or the
  line before it contains an authority keyword as a word.
* Authority keywords: authority, commit, head, parent, base, revision, rev, checkpoint,
  merge, sealed, source, tip, sha, ref, anchor, main, branch.

Each citation is then resolved in the local object database:

* object is a tree or blob: not a commit, reported as ``non-commit`` and skipped;
* object is a commit and ``git branch -r --contains <sha>`` is non-empty: ``reachable``;
* otherwise (commit with no remote branch, or object absent): ``unreachable`` / ``missing``,
  which fails unless the SHA is listed in the allowlist.

Allowlist (default ``tools/authority_refs_allowlist.txt``): one ``<40-hex sha> <reason>``
per line; ``#`` starts a comment.  Every entry must carry a non-empty reason.  The
allowlist freezes historical citations that are known to be unreachable, so the check
passes on the current tree and fails on any new unreachable citation.

CI must check out full history of all branches (``fetch-depth: 0``) so that remote refs
exist locally.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

DEFAULT_PATHS = ("research", "docs")
SUFFIXES = {".md", ".json", ".jsonl", ".toml"}
DEFAULT_ALLOWLIST = "tools/authority_refs_allowlist.txt"
KEYWORDS = frozenset(
    {
        "authority",
        "commit",
        "head",
        "parent",
        "base",
        "revision",
        "rev",
        "checkpoint",
        "merge",
        "sealed",
        "source",
        "tip",
        "sha",
        "ref",
        "anchor",
        "main",
        "branch",
    }
)
SHA_RE = re.compile(r"(?<![0-9a-fA-F])([0-9a-f]{40})(?![0-9a-fA-F])")
ALLOW_LINE_RE = re.compile(r"^([0-9a-f]{40})\s+(\S.*)$")


@dataclass(frozen=True)
class Citation:
    sha: str
    path: str
    line: int
    context: str


def words(text: str) -> set[str]:
    spaced = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", text)
    return {w for w in re.split(r"[^a-z0-9]+", spaced.lower()) if w}


def has_keyword(text: str) -> bool:
    return bool(words(SHA_RE.sub(" ", text)) & KEYWORDS)


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args], cwd=repo, capture_output=True, text=True, check=check
    )


def text_citations(path: str, text: str) -> list[Citation]:
    out: list[Citation] = []
    lines = text.splitlines()
    for idx, line in enumerate(lines):
        if not SHA_RE.search(line):
            continue
        prev = lines[idx - 1] if idx > 0 else ""
        if not (has_keyword(line) or has_keyword(prev)):
            continue
        for match in SHA_RE.finditer(line):
            out.append(Citation(match.group(1), path, idx + 1, line.strip()[:120]))
    return out


def _line_of(text: str, sha: str) -> int:
    pos = text.find(sha)
    return text.count("\n", 0, pos) + 1 if pos >= 0 else 0


def json_value_citations(
    path: str, value: object, key_path: list[str], text: str, line_offset: int
) -> list[Citation]:
    out: list[Citation] = []
    if isinstance(value, dict):
        for key, sub in value.items():
            out.extend(json_value_citations(path, sub, key_path + [str(key)], text, line_offset))
    elif isinstance(value, list):
        for sub in value:
            out.extend(json_value_citations(path, sub, key_path, text, line_offset))
    elif isinstance(value, str) and SHA_RE.search(value):
        key_words: set[str] = set()
        for key in key_path:
            key_words |= words(key)
        if key_words & KEYWORDS or has_keyword(value):
            dotted = ".".join(key_path) or "<root>"
            for match in SHA_RE.finditer(value):
                sha = match.group(1)
                out.append(Citation(sha, path, line_offset + _line_of(text, sha), dotted[:120]))
    return out


def file_citations(path: str, text: str) -> list[Citation]:
    suffix = Path(path).suffix
    if suffix == ".json":
        try:
            return json_value_citations(path, json.loads(text), [], text, 0)
        except ValueError:
            return text_citations(path, text)
    if suffix == ".jsonl":
        out: list[Citation] = []
        for idx, line in enumerate(text.splitlines()):
            if not line.strip():
                continue
            try:
                out.extend(json_value_citations(path, json.loads(line), [], line, idx))
            except ValueError:
                out.extend(Citation(c.sha, path, idx + 1, c.context) for c in text_citations(path, line))
        return out
    return text_citations(path, text)


def load_allowlist(path: Path) -> tuple[dict[str, str], list[str]]:
    entries: dict[str, str] = {}
    errors: list[str] = []
    if not path.exists():
        return entries, errors
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        match = ALLOW_LINE_RE.match(line)
        if not match:
            errors.append(f"{path}:{number}: expected '<40-hex sha> <reason>'")
            continue
        sha, reason = match.groups()
        if sha in entries:
            errors.append(f"{path}:{number}: duplicate allowlist entry {sha}")
        entries[sha] = reason
    return entries, errors


def resolve(repo: Path, sha: str) -> str:
    kind = git(repo, "cat-file", "-t", sha, check=False)
    if kind.returncode != 0:
        return "missing"
    kind_name = kind.stdout.strip()
    if kind_name != "commit":
        return "non-commit"
    remotes = git(repo, "branch", "-r", "--contains", sha, check=False)
    if remotes.returncode == 0 and remotes.stdout.strip():
        return "reachable"
    return "unreachable"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo", type=Path, default=None, help="repository root (default: current toplevel)")
    parser.add_argument("--paths", nargs="+", default=list(DEFAULT_PATHS), help="paths to scan (default: research docs)")
    parser.add_argument("--allowlist", default=DEFAULT_ALLOWLIST, help="allowlist file relative to the repo root")
    parser.add_argument("--list", action="store_true", help="print every citation and its status")
    args = parser.parse_args(argv)

    try:
        repo = args.repo or Path(git(Path.cwd(), "rev-parse", "--show-toplevel").stdout.strip())
        tracked = git(repo, "ls-files", "-z", "--", *args.paths).stdout.split("\0")
    except subprocess.CalledProcessError as exc:
        print(f"check-authority-refs: git failed: {exc.stderr.strip()}", file=sys.stderr)
        return 2

    allow, allow_errors = load_allowlist(repo / args.allowlist)
    citations: list[Citation] = []
    for rel in sorted(p for p in tracked if p and Path(p).suffix in SUFFIXES):
        file_path = repo / rel
        if not file_path.is_file():
            continue
        text = file_path.read_text(encoding="utf-8", errors="replace")
        citations.extend(file_citations(rel, text))

    status: dict[str, str] = {}
    for sha in sorted({c.sha for c in citations}):
        status[sha] = resolve(repo, sha)

    failures: list[str] = list(allow_errors)
    for cit in citations:
        state = status[cit.sha]
        allowed = cit.sha in allow
        label = state
        if state in ("missing", "unreachable"):
            if allowed:
                label = f"{state}+allowlisted"
            else:
                failures.append(
                    f"{cit.path}:{cit.line}: {cit.sha} is {state} "
                    f"(no remote branch contains it) [{cit.context}]"
                )
        if args.list:
            print(f"{label:24} {cit.sha} {cit.path}:{cit.line} [{cit.context}]")

    if args.list:
        cited = set(status)
        for sha, reason in sorted(allow.items()):
            if sha not in cited:
                print(f"{'allowlist-not-cited':24} {sha} (in scanned paths) {reason}")
            elif status[sha] == "reachable":
                print(f"{'allowlist-now-reachable':24} {sha} {reason}")

    counts: dict[str, int] = {}
    for sha, state in status.items():
        key = f"{state}+allowlisted" if state in ("missing", "unreachable") and sha in allow else state
        counts[key] = counts.get(key, 0) + 1
    summary = ", ".join(f"{k}={v}" for k, v in sorted(counts.items())) or "none"
    for failure in failures:
        print(f"FAIL: {failure}")
    verdict = "FAIL" if failures else "PASS"
    print(
        f"check-authority-refs: {verdict} ({len(citations)} citations, "
        f"{len(status)} distinct SHAs: {summary})"
    )
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
