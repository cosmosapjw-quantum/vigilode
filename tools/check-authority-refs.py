#!/usr/bin/env python3
"""Check typed research authority without conflating repository or Git object identity.

Tracked Markdown/JSON/JSONL/TOML under research/ and docs/ retain the legacy
40-hex/authority-keyword extraction rule. Opaque JSON body/content/diagnostic
payloads and Julia Manifest git-tree-sha1 package hashes do not declare local
commit authority. Structured source_commit fields remain checked.

Each citation records repository, object kind, identifier and required relation.
Explicit repo/repository fields, GitHub URLs and repository-prefixed dependency
fields qualify external objects. Use --repository owner/name=/checkout to resolve
an external repository offline; its origin identity must match. Missing external
checkouts fail rather than silently passing or consulting VigilODE objects.

Commits require remote reachability. Tree/blob citations require an object of the
right kind; a missing local tree remains a failure. Legacy ambiguous object
citations retain the non-commit classification. Explicit object_kind/identifier/
required_relation records support published-source, remote-reachable and
object-exists. The research ledger's commit always requires remote-reachable.

A missing historical source may report tree-equivalent only through a recognized
publication-map schema (or SOURCE_IDENTITY.json v1 local/remote/tree tuple), whose
exact bytes occur in remote branch history. The target must be a reachable commit
with the recorded whole tree; an available local commit must match that tree too.
This supports source reuse and retains the original commit identity. It does not
satisfy a remote-reachable relation. Invalid or unpublished mappings fail closed.

The existing reasoned allowlist applies only to missing/unreachable local commit
citations. It cannot exempt external repositories, missing trees, kind mismatches
or invalid mappings. CI needs complete remote branch history (fetch-depth: 0).
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass, replace
from pathlib import Path
from urllib.parse import urlsplit

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
GITHUB_RE = re.compile(r"https://(?:api\.)?github\.com/(?:repos/)?([\w.-]+/[\w.-]+)(?:/|$)")
GITHUB_OBJECT_RE = re.compile(
    r"https://(?:api\.)?github\.com/(?:repos/)?([\w.-]+/[\w.-]+)/"
    r"(?:commit|commits|tree|blob)/([0-9a-f]{40})(?![0-9a-fA-F])"
)
ALLOW_LINE_RE = re.compile(r"^([0-9a-f]{40})\s+(\S.*)$")


@dataclass(frozen=True)
class Citation:
    sha: str
    path: str
    line: int
    context: str
    repository: str = "local"
    object_kind: str = "git-object"
    required_relation: str = "published-source"


def repository_in_object_url(value: str, sha_start: int, sha_end: int) -> str | None:
    """Bind only the identifier that occurs inside an explicit GitHub object URL."""
    for match in GITHUB_OBJECT_RE.finditer(value):
        if match.start(2) == sha_start and match.end(2) == sha_end:
            return match.group(1)
    return None


def canonical_repository(value: str) -> str | None:
    if value.startswith("https://"):
        parsed = urlsplit(value)
        if (parsed.scheme != "https" or parsed.netloc != "github.com"
                or parsed.query or parsed.fragment):
            return None
        name = parsed.path.lstrip("/")
    elif value.startswith("git@github.com:"):
        name = value[len("git@github.com:"):]
    else:
        name = value
    if name.endswith(".git"):
        name = name[:-4]
    return name if re.fullmatch(r"[\w.-]+/[\w.-]+", name) else None


@dataclass(frozen=True)
class SourceMapping:
    repository: str
    source: str
    target: str
    tree: str
    path: str


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
        # Julia package content hashes are not VigilODE commit authorities.
        if Path(path).name == "Manifest.toml" and re.match(r'\s*git-tree-sha1\s*=', line):
            continue
        if not SHA_RE.search(line):
            continue
        prev = lines[idx - 1] if idx > 0 else ""
        if not (has_keyword(line) or has_keyword(prev)):
            continue
        for match in SHA_RE.finditer(line):
            prefix = line[:match.start()]
            clause = re.split(r"[.;]", prefix)[-1]
            kind = "tree" if re.search(r"\b(?:source[_ -]?)?tree\s*[:=]?\s*[`\"]?$", clause, re.I) else "git-object"
            if kind == "git-object" and re.search(r"\b(?:tree[_ -]?)?(?:commit|ref)\s*[:=]?\s*[`\"]?$", clause, re.I):
                kind = "commit"
            named = repository_in_object_url(line, match.start(1), match.end(1)) or "local"
            out.append(Citation(match.group(1), path, idx + 1, line.strip()[:120], named, kind))
    return out


def _line_of(text: str, sha: str) -> int:
    pos = text.find(sha)
    return text.count("\n", 0, pos) + 1 if pos >= 0 else 0


def json_value_citations(
    path: str, value: object, key_path: list[str], text: str, line_offset: int,
    repository: str = "local", object_kind: str = "git-object",
    required_relation: str = "published-source", typed_record: bool = False,
    github_capture: bool = False,
) -> list[Citation]:
    out: list[Citation] = []
    if isinstance(value, dict):
        if not key_path:
            github_capture = "pulls" in value or (
                Path(path).name.lower().startswith("github") and "structuredContent" in value
            )
        if "object_kind" in value or "required_relation" in value:
            # A typed declaration is atomic.  Partial or malformed declarations
            # must be reported, never silently interpreted as legacy prose.
            identifier = value.get("identifier")
            named_repo = value.get("repository")
            kind = value.get("object_kind")
            relation = value.get("required_relation")
            dotted = ".".join(key_path + ["identifier"]) or "identifier"
            if path == "research/LEDGER.jsonl" and key_path in ([], ["commit"]) and relation != "remote-reachable":
                relation = "<invalid-ledger-relation>"
            primary = Citation(
                identifier if isinstance(identifier, str) else "<missing-identifier>",
                path,
                line_offset + _line_of(text, identifier if isinstance(identifier, str) else '"object_kind"'),
                dotted[:120],
                canonical_repository(named_repo) if isinstance(named_repo, str) and canonical_repository(named_repo) else "<invalid-repository>",
                kind if isinstance(kind, str) else "<invalid-kind>",
                relation if isinstance(relation, str) else "<invalid-relation>",
            )
            out.append(primary)
            for key, sub in value.items():
                if key in {"repository", "object_kind", "identifier", "required_relation"}:
                    continue
                # A second authority is a second obligation.  It must not
                # inherit a weaker relation such as object-exists.
                out.extend(json_value_citations(
                    path, sub, key_path + [str(key)], text, line_offset,
                    primary.repository, "git-object", "published-source", False,
                    github_capture,
                ))
            return out
        named_repo = value.get("repository", value.get("repo", value.get("source_repository")))
        if isinstance(named_repo, str):
            repository = canonical_repository(named_repo) or "<invalid-repository>"
        for key, sub in value.items():
            out.extend(json_value_citations(path, sub, key_path + [str(key)], text, line_offset,
                                            repository, object_kind, required_relation, typed_record, github_capture))
    elif isinstance(value, list):
        for sub in value:
            out.extend(json_value_citations(path, sub, key_path, text, line_offset,
                                            repository, object_kind, required_relation, typed_record, github_capture))
    elif isinstance(value, str) and (SHA_RE.search(value) or (typed_record and key_path and key_path[-1] == "identifier")):
        # Quoted PR prose, serialized API responses and captured diagnostics are
        # evidence payloads, not declarations that their substrings are authority.
        if key_path and key_path[-1] in {"body", "content"} and github_capture and (
            "pulls" in key_path or "structuredContent" in key_path
        ):
            return out
        key_words: set[str] = set()
        for key in key_path:
            key_words |= words(key)
        if key_words & KEYWORDS or has_keyword(value) or (key_path and key_path[-1] == "identifier"):
            dotted = ".".join(key_path) or "<root>"
            leaf = words(key_path[-1]) if key_path else set()
            kind = object_kind
            if kind == "git-object":
                if "tree" in leaf and not (leaf & {"commit", "ref"}):
                    kind = "tree"
                    if not SHA_RE.fullmatch(value) and re.search(r"\b(?:base|commit)\s+[0-9a-f]{40}\b", value):
                        kind = "commit"
                elif "blob" in leaf:
                    kind = "blob"
                elif leaf & {"commit", "head", "parent", "revision", "ref"}:
                    kind = "commit"
            relation = required_relation
            if path == "research/LEDGER.jsonl" and key_path == ["commit"]:
                relation = "remote-reachable"
            for match in SHA_RE.finditer(value):
                sha = match.group(1)
                named = repository_in_object_url(value, match.start(1), match.end(1)) or repository
                out.append(Citation(sha, path, line_offset + _line_of(text, sha), dotted[:120],
                                    named, kind, relation))
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
                out.extend(replace(c, path=path, line=idx + 1) for c in text_citations(path, line))
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


def repository_identity(repo: Path) -> str:
    remote = git(repo, "remote", "get-url", "origin", check=False)
    if remote.returncode != 0:
        return "local"
    value = remote.stdout.strip()
    if not (value.startswith("https://") or value.startswith("git@github.com:")):
        return "local"
    return canonical_repository(value) or "local"


def published_file(repo: Path, path: str) -> bool:
    """Require this exact mapping blob at this path in a remote branch history."""
    blob = git(repo, "hash-object", "--", path).stdout.strip()
    history = git(repo, "log", "--remotes", "--format=%H", "--", path).stdout.splitlines()
    return any(git(repo, "rev-parse", f"{commit}:{path}", check=False).stdout.strip() == blob
               for commit in history)


def load_mappings(repo: Path, documents: dict[str, str], local_name: str) -> tuple[dict[tuple[str, str], SourceMapping], list[str]]:
    mappings: dict[tuple[str, str], SourceMapping] = {}
    errors: list[str] = []
    for path, text in documents.items():
        if Path(path).suffix != ".json":
            continue
        try:
            data = json.loads(text)
        except ValueError:
            continue
        if not isinstance(data, dict):
            continue
        schema = data.get("schema")
        if schema == "vigilode.git-publication-map.v1":
            entries = data.get("mappings")
            fields = ("local_evidence_commit", "remote_equivalent_commit", "identical_tree_sha")
        elif schema == "vigilode.frontier.publication-map.v1":
            entries = data.get("source_equivalence")
            fields = ("local_commit", "github_commit", "whole_tree")
        elif Path(path).name == "SOURCE_IDENTITY.json" and data.get("schema_version") == 1 and any(
            key in data for key in ("source_local_commit", "source_remote_commit")
        ):
            entries = [data]
            fields = ("source_local_commit", "source_remote_commit", "source_tree")
        else:
            continue
        if data.get("repository", local_name) != local_name:
            errors.append(f"{path}: mapping repository does not match {local_name}")
            continue
        if not isinstance(entries, list) or not entries:
            errors.append(f"{path}: malformed publication mapping list")
            continue
        if not published_file(repo, path):
            errors.append(f"{path}: mapping is not published unchanged")
            continue
        for entry in entries:
            if not isinstance(entry, dict) or any(
                not isinstance(entry.get(field), str) or not SHA_RE.fullmatch(entry[field]) for field in fields
            ):
                errors.append(f"{path}: malformed mapping identifiers")
                continue
            source, target, tree = (entry[field] for field in fields)
            if resolve(repo, target) != "reachable":
                errors.append(f"{path}: mapping target is not a remote-reachable commit: {target}")
                continue
            if git(repo, "rev-parse", f"{target}^{{tree}}").stdout.strip() != tree:
                errors.append(f"{path}: mapping target tree mismatch: {target}")
                continue
            local_kind = git(repo, "cat-file", "-t", source, check=False)
            if local_kind.returncode == 0 and (
                local_kind.stdout.strip() != "commit" or
                git(repo, "rev-parse", f"{source}^{{tree}}", check=False).stdout.strip() != tree
            ):
                errors.append(f"{path}: mapping local tree mismatch: {source}")
                continue
            item = SourceMapping(local_name, source, target, tree, path)
            key = (local_name, source)
            previous = mappings.get(key)
            if previous and (previous.target, previous.tree) != (target, tree):
                errors.append(f"{path}: conflicting publication mapping: {source}")
                mappings.pop(key, None)
                continue
            mappings[key] = item
    return mappings, errors


def resolve_citation(citation: Citation, repositories: dict[str, Path], mappings: dict[tuple[str, str], SourceMapping]) -> tuple[str, str]:
    if not SHA_RE.fullmatch(citation.sha):
        return "invalid-identifier", "expected a 40-hex Git object identifier"
    if citation.object_kind not in {"git-object", "commit", "tree", "blob"}:
        return "invalid-kind", citation.object_kind
    if citation.required_relation == "<invalid-ledger-relation>":
        return "invalid-ledger-relation", "ledger commit requires remote-reachable"
    if citation.required_relation not in {"published-source", "remote-reachable", "object-exists"}:
        return "invalid-relation", citation.required_relation
    if citation.repository == "<invalid-repository>":
        return "invalid-repository", "expected explicit owner/name"
    repo = repositories.get(citation.repository)
    if repo is None:
        return "repository-unavailable", "supply --repository NAME=PATH with matching origin identity"
    kind = git(repo, "cat-file", "-t", citation.sha, check=False)
    actual = kind.stdout.strip() if kind.returncode == 0 else None
    if actual and citation.object_kind not in {"git-object", actual}:
        return "object-kind-mismatch", f"expected {citation.object_kind}, found {actual}"
    if actual in {"tree", "blob"}:
        if citation.required_relation == "remote-reachable":
            return "object-kind-mismatch", "remote-reachable requires a commit"
        return "non-commit", f"verified {actual} object exists"
    state = resolve(repo, citation.sha)
    if state in {"missing", "unreachable"} and citation.object_kind in {"git-object", "commit"} and citation.required_relation == "published-source":
        mapping = mappings.get((citation.repository, citation.sha))
        if mapping:
            return "tree-equivalent", f"{mapping.path}: target={mapping.target} tree={mapping.tree}; commit identity retained"
    if actual and citation.required_relation == "object-exists":
        return "object-present", ""
    return state, "no remote branch contains it" if state in {"missing", "unreachable"} else ""


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo", type=Path, default=None, help="repository root (default: current toplevel)")
    parser.add_argument("--paths", nargs="+", default=list(DEFAULT_PATHS), help="paths to scan (default: research docs)")
    parser.add_argument("--allowlist", default=DEFAULT_ALLOWLIST, help="allowlist file relative to the repo root")
    parser.add_argument("--repository", action="append", default=[], metavar="NAME=PATH",
                        help="external GitHub repository identity and local checkout; never fetched automatically")
    parser.add_argument("--list", action="store_true", help="print every citation and its status")
    args = parser.parse_args(argv)

    try:
        repo = args.repo or Path(git(Path.cwd(), "rev-parse", "--show-toplevel").stdout.strip())
        tracked = git(repo, "ls-files", "-z", "--", *args.paths).stdout.split("\0")
    except subprocess.CalledProcessError as exc:
        print(f"check-authority-refs: git failed: {exc.stderr.strip()}", file=sys.stderr)
        return 2

    allow, allow_errors = load_allowlist(repo / args.allowlist)
    local_name = repository_identity(repo)
    repositories = {local_name: repo, "local": repo}
    for supplied in args.repository:
        name, separator, location = supplied.partition("=")
        if not separator or not name or not location:
            print(f"check-authority-refs: expected --repository NAME=PATH: {supplied}", file=sys.stderr)
            return 2
        checkout = Path(location)
        actual_name = repository_identity(checkout)
        if actual_name != name or name in repositories:
            print(f"check-authority-refs: repository identity mismatch or duplicate: {name} != {actual_name}", file=sys.stderr)
            return 2
        repositories[name] = checkout
    citations: list[Citation] = []
    documents: dict[str, str] = {}
    for rel in sorted(p for p in tracked if p and Path(p).suffix in SUFFIXES):
        file_path = repo / rel
        if not file_path.is_file():
            continue
        text = file_path.read_text(encoding="utf-8", errors="replace")
        documents[rel] = text
        citations.extend(file_citations(rel, text))

    citations = [replace(c, repository=local_name) if c.repository == "local" else c for c in citations]
    mappings, mapping_errors = load_mappings(repo, documents, local_name)
    status: dict[tuple[str, str, str, str], tuple[str, str]] = {}
    def identity(cit: Citation) -> tuple[str, str, str, str]:
        return cit.repository, cit.object_kind, cit.sha, cit.required_relation
    for cit in citations:
        key = identity(cit)
        if key not in status:
            status[key] = resolve_citation(cit, repositories, mappings)

    failures: list[str] = list(allow_errors) + mapping_errors
    for cit in citations:
        state, detail = status[identity(cit)]
        allowed = cit.repository == local_name and cit.object_kind in {"git-object", "commit"} and cit.sha in allow
        label = state
        if state not in {"reachable", "non-commit", "tree-equivalent", "object-present"}:
            if allowed and state in {"missing", "unreachable"}:
                label = f"{state}+allowlisted"
            else:
                failures.append(
                    f"{cit.path}:{cit.line}: {cit.sha} is {state} "
                    f"({detail}) [{cit.context}; repository={cit.repository}; "
                    f"object_kind={cit.object_kind}; required_relation={cit.required_relation}]"
                )
        if args.list:
            print(f"{label:24} {cit.sha} {cit.path}:{cit.line} [{cit.context}; "
                  f"repository={cit.repository}; object_kind={cit.object_kind}; "
                  f"required_relation={cit.required_relation}]" + (f" {detail}" if detail else ""))

    if args.list:
        cited = {c.sha for c in citations if c.repository == local_name}
        for sha, reason in sorted(allow.items()):
            if sha not in cited:
                print(f"{'allowlist-not-cited':24} {sha} (in scanned paths) {reason}")
            elif resolve(repo, sha) == "reachable":
                print(f"{'allowlist-now-reachable':24} {sha} {reason}")

    counts: dict[str, int] = {}
    for (name, kind, sha, relation), (state, detail) in status.items():
        allowed = name == local_name and kind in {"git-object", "commit"} and sha in allow
        key = f"{state}+allowlisted" if state in ("missing", "unreachable") and allowed else state
        counts[key] = counts.get(key, 0) + 1
    summary = ", ".join(f"{k}={v}" for k, v in sorted(counts.items())) or "none"
    for failure in failures:
        print(f"FAIL: {failure}")
    verdict = "FAIL" if failures else "PASS"
    print(
        f"check-authority-refs: {verdict} ({len(citations)} citations, "
        f"{len(status)} distinct authority identities: {summary})"
    )
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
