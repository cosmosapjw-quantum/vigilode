#!/usr/bin/env python3
"""Validate the component-triage manifests against `rodas5p triage-list`; dry-run cell counts; static reach.

Validation (always; exit 1 on any error):
  * COMPONENTS.json / CORPUS.json schemas, duplicate component or problem ids;
  * every arm id in COMPONENTS `arms` is a triage-list arm or a registered external arm
    (tools/triage/triage_external.py::EXTERNAL_ARMS); an internal arm listed for c must contain c; every
    optional `external_peers` id is a registered external arm;
  * every component of a triage-list arm and every `blocked_by` id is a COMPONENTS id;
  * every CORPUS problem is a triage-list problem;
  * coverage: a component with reach D or O needs a wired triage-list arm containing it with at least one
    applicable CORPUS problem (contract applicability, tools/triage/triage_common.py::applicable); otherwise it
    is a coverage gap.
The report (JSON) goes to stdout or `--output`.

`--dry-run` prints the applicability matrix (arm x problem: `x` applicable, `.` not) and the cell counts of the
three campaigns: runs (applicable problems x sweep rtols x grids x h0_scales), profile and alloc (profile
subset: profile rtols, dimension <= max_dimension, grids, h0 = 1). `--seconds-per-cell runs=S,profile=S,alloc=S`
adds a rough time budget (sequential seconds; divide by -j yourself).

`--static-reach OUT` writes STATIC_REACH.json: for every module (source file) of crates/rodas5p-integrators/src,
crates/rodas5p-core/src and crates/rodas5p-krylov/src, its non-test caller files and whether it is reachable
from the entry modules of the wired arms (the `files` of the components of wired arms, plus `--reach-root`).
Approximation (documented in the output): references are found by a token scan of the `git ls-files` set
(tracked and untracked files under crates/*/src, outside tests/, examples/, benches/, comment lines and the
part of a file after its first `#[cfg(test)]` line; a `git grep -w -F` with ~2000 patterns was too slow) by
name, not by type resolution: a file references a module when it names one of the module's `pub`
items whose name is defined in exactly one scanned module, or the module path (`<crate>::<path>`,
`crate::<path>`, `super::<name>`, `self::<name>`); `mod <name>;` declarations do not count, re-export lines do.
Trait dispatch, macros and glob imports of non-unique names are invisible, so reachability is approximate.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import defaultdict, deque
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402

REACH_SCHEMA = "vigilode-triage-static-reach-v1"
REPORT_SCHEMA = "vigilode-triage-manifest-report-v1"
REACH_CRATES = ("rodas5p-integrators", "rodas5p-core", "rodas5p-krylov")
COVERAGE_REACH = ("D", "O")
PUB_ITEM = re.compile(r"^\s*pub(?:\([^)]*\))?\s+(?:unsafe\s+|const\s+|async\s+|extern\s+\"C\"\s+)*"
                      r"(?:fn|struct|enum|trait|type|const|static|union)\s+([A-Za-z_][A-Za-z0-9_]*)")
MOD_DECL = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+[A-Za-z_][A-Za-z0-9_]*\s*;")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
PATH_CHAIN = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+")
EXCLUDED_PARTS = {"tests", "examples", "benches"}


def external_arms() -> tuple:
    try:
        import triage_external
        return triage_external.EXTERNAL_ARMS
    except Exception:  # noqa: BLE001 - only the arm id list is needed; fall back to the prefix rule
        return ()


# ------------------------------------------------------------------------------------------ validate
def validate(components: dict, corpus: dict, list_doc: dict) -> dict:
    errors, warnings = [], []
    if components.get("schema") != tc.COMPONENTS_SCHEMA:
        errors.append(f"COMPONENTS: schema is not {tc.COMPONENTS_SCHEMA!r}")
    if corpus.get("schema") != tc.CORPUS_SCHEMA:
        errors.append(f"CORPUS: schema is not {tc.CORPUS_SCHEMA!r}")
    comps: dict[str, dict] = {}
    for c in components.get("components", []):
        if c.get("id") in comps:
            errors.append(f"COMPONENTS: duplicate component id {c.get('id')!r}")
        comps[c.get("id")] = c
    try:
        problems = tc.corpus_problems(corpus)
    except tc.TriageError as exc:
        errors.extend(exc.reasons)
        problems = {}
    arms = {}
    for a in list_doc.get("arms", []):
        if a["id"] in arms:
            errors.append(f"triage-list: duplicate arm id {a['id']!r}")
        arms[a["id"]] = a
    list_problems = {p["id"]: p for p in list_doc.get("problems", [])}
    ext = set(external_arms())
    for cid, c in sorted(comps.items()):
        for arm in c.get("arms", []):
            if arm in arms:
                if cid not in arms[arm].get("components", []):
                    errors.append(f"component {cid}: arm {arm!r} does not contain it")
            elif tc.is_external(arm):
                if ext and arm not in ext:
                    errors.append(f"component {cid}: unknown external arm id {arm!r}")
            else:
                errors.append(f"component {cid}: unknown arm id {arm!r}")
        for arm in c.get("external_peers", []) or []:
            if not tc.is_external(arm) or (ext and arm not in ext):
                errors.append(f"component {cid}: unknown external peer {arm!r}")
        for b in c.get("blocked_by", []):
            if b not in comps:
                errors.append(f"component {cid}: blocked_by unknown component {b!r}")
    for arm_id, a in sorted(arms.items()):
        for c in a.get("components", []):
            if c not in comps:
                errors.append(f"triage-list arm {arm_id!r}: component {c!r} not in COMPONENTS")
        if not a.get("wired"):
            warnings.append(f"arm {arm_id} unwired: {a.get('unwired_reason')}")
    for pid in sorted(problems):
        if pid not in list_problems:
            errors.append(f"CORPUS problem {pid!r} is not a triage-list problem")
    matrix = {}
    for arm_id, a in sorted(arms.items()):
        row = {}
        for pid, cp in sorted(problems.items()):
            if pid not in list_problems:
                continue
            ok, why = tc.applicable(a, tc.merged_problem(cp, list_problems[pid]))
            row[pid] = {"applicable": ok, "reason": why}
        matrix[arm_id] = row
    coverage = {}
    for cid, c in sorted(comps.items()):
        wired = sorted(a for a, spec in arms.items() if spec.get("wired") and cid in spec.get("components", []))
        reach = [a for a in wired if any(v["applicable"] for v in matrix.get(a, {}).values())]
        coverage[cid] = {"reach": c.get("reach"), "wired_arms": wired, "arms_with_applicable_problem": reach}
        if c.get("reach") in COVERAGE_REACH and not reach:
            errors.append(f"coverage gap: component {cid} (reach {c.get('reach')}) has no wired arm with an "
                          f"applicable CORPUS problem (wired arms: {wired})")
    return {"schema": REPORT_SCHEMA, "errors": errors, "warnings": warnings, "coverage": coverage,
            "applicability": matrix}


def dry_run(list_doc: dict, corpus: dict, report: dict, budget: dict[str, float]) -> list[str]:
    lines = []
    matrix = report["applicability"]
    problems = sorted({p for row in matrix.values() for p in row})
    width = max([len(a) for a in matrix] + [4])
    lines.append("applicability (x applicable, . not):")
    for i, pid in enumerate(problems):
        lines.append(f"{'':{width}}  {'|' * i}{pid}")
    for arm in sorted(matrix):
        marks = "".join("x" if matrix[arm][p]["applicable"] else "." for p in problems)
        wired = next((a.get("wired") for a in list_doc["arms"] if a["id"] == arm), False)
        lines.append(f"{arm:{width}}  {marks}{'' if wired else '  (unwired)'}")
    total = 0.0
    for kind in tc.PLAN_KINDS:
        cells, _ = tc.plan_cells(list_doc, corpus, kind)
        seconds = budget.get(kind)
        text = f"{kind}: {len(cells)} cells"
        if seconds is not None:
            total += seconds * len(cells)
            text += f", ~{seconds * len(cells) / 3600.0:.2f} h at {seconds:g} s/cell"
        lines.append(text)
    if budget:
        lines.append(f"total sequential budget ~{total / 3600.0:.2f} h")
    return lines


# --------------------------------------------------------------------------------------- static reach
def module_of(path: str) -> tuple[str, str]:
    parts = path.split("/")
    crate = parts[1].replace("-", "_")
    rel = parts[3:]
    if rel[-1] in ("lib.rs", "main.rs"):
        rel = rel[:-1]
    elif rel[-1] == "mod.rs":
        rel = rel[:-1]
    else:
        rel[-1] = rel[-1][:-3]
    return crate, "::".join(rel)


def excluded(path: str) -> bool:
    parts = path.split("/")
    return bool(EXCLUDED_PARTS & set(parts)) or parts[-1].endswith("_tests.rs") or parts[-1] == "tests.rs"


def source_files(repo: Path) -> list[str]:
    out = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "--", "crates"],
                         cwd=repo, capture_output=True, text=True, check=True).stdout.split()
    return sorted({f for f in out if re.match(r"^crates/[^/]+/src/.+\.rs$", f) and not excluded(f)})


def test_cutoff(text: str) -> int:
    for number, line in enumerate(text.splitlines(), 1):
        if line.strip().startswith("#[cfg(test)]"):
            return number
    return 10 ** 9


def scan_line(path: str, content: str, symbol_module: dict, by_full: dict, by_last: dict, edges: dict) -> None:
    """Modules referenced by one source line: unique pub item names and module paths."""
    targets = {symbol_module[i] for i in set(IDENT.findall(content)) if i in symbol_module}
    if "::" in content:
        crate_here = path.split("/")[1].replace("-", "_")
        for chain in PATH_CHAIN.findall(content):
            segs = chain.split("::")
            head = crate_here if segs[0] == "crate" else segs[0]
            for k in range(2, len(segs) + 1):
                m = by_full.get("::".join([head] + segs[1:k]))
                if m is not None:
                    targets.add(m)
            if segs[0] in ("super", "self"):
                targets |= by_last.get((crate_here, segs[1]), set())
    targets.discard(path)
    edges[path] |= targets


def static_reach(repo: Path, components: dict, list_doc: dict, extra_roots: list[str]) -> dict:
    files = source_files(repo)
    cutoff = {}
    texts = {}
    for f in files:
        try:
            texts[f] = (repo / f).read_text(errors="replace")
        except OSError:
            continue
        cutoff[f] = test_cutoff(texts[f])
    modules = [f for f in files if f.split("/")[1] in REACH_CRATES and f in texts]
    defined: dict[str, set] = defaultdict(set)
    for m in modules:
        for number, line in enumerate(texts[m].splitlines(), 1):
            if number >= cutoff[m]:
                break
            match = PUB_ITEM.match(line)
            if match:
                defined[match.group(1)].add(m)
    symbol_module = {name: next(iter(ms)) for name, ms in defined.items() if len(ms) == 1}
    names = {m: module_of(m) for m in modules}
    by_full: dict[str, str] = {}
    by_last: dict[tuple[str, str], set] = defaultdict(set)
    for m, (crate, modpath) in names.items():
        if modpath:
            by_full[f"{crate}::{modpath}"] = m
            by_last[(crate, modpath.split("::")[-1])].add(m)
    edges: dict[str, set] = defaultdict(set)
    for path in sorted(texts):
        for number, content in enumerate(texts[path].splitlines(), 1):
            if number >= cutoff[path]:
                break
            if MOD_DECL.match(content) or content.lstrip().startswith("//"):
                continue
            scan_line(path, content, symbol_module, by_full, by_last, edges)
    callers: dict[str, set] = defaultdict(set)
    for src, targets in edges.items():
        for t in targets:
            callers[t].add(src)
    comps = {c["id"]: c for c in components.get("components", [])}
    wired = [a for a in list_doc.get("arms", []) if a.get("wired") and not tc.is_external(a["id"])]
    roots = sorted({f for a in wired for cid in a.get("components", []) for f in comps.get(cid, {}).get("files", [])
                    if f in texts} | {f for f in extra_roots if f in texts})
    parent: dict[str, str | None] = {r: None for r in roots}
    queue = deque(roots)
    while queue:
        src = queue.popleft()
        for t in sorted(edges.get(src, ())):
            if t not in parent:
                parent[t] = src
                queue.append(t)
    entries = []
    for m in modules:
        crate, modpath = names[m]
        entries.append({"file": m, "crate": crate, "module": f"{crate}::{modpath}" if modpath else crate,
                        "non_test_callers": sorted(callers.get(m, ())), "reachable_from_wired_arm": m in parent,
                        "via": parent.get(m)})
    commit = tc.git_provenance(repo)
    return {"schema": REACH_SCHEMA, "base_commit": commit.get("commit"), "tree_clean": commit.get("tree_clean"),
            "approximation": "name-based token scan of the git ls-files set under crates/*/src (tracked and untracked, "
                             "outside tests/examples/benches, comment lines and in-file #[cfg(test)] tails): a "
                             "reference is a unique pub item name of the module or its path; no type resolution, "
                             "trait dispatch or macro expansion",
            "roots": roots, "unresolved_component_files": sorted(
                {f for c in comps.values() for f in c.get("files", []) if f.endswith(".rs") and f not in texts}),
            "modules": entries}


def parse_budget(text: str | None) -> dict[str, float]:
    if not text:
        return {}
    out = {}
    for part in text.split(","):
        key, _, value = part.partition("=")
        if key not in tc.PLAN_KINDS:
            raise SystemExit(f"--seconds-per-cell: unknown kind {key!r} (use {', '.join(tc.PLAN_KINDS)})")
        out[key] = float(value)
    return out


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    node = tc.DEFAULT_NODE
    parser.add_argument("--components", type=Path, default=node / "COMPONENTS.json")
    parser.add_argument("--corpus", type=Path, default=node / "CORPUS.json")
    parser.add_argument("--list", type=Path, help="triage-list JSON")
    parser.add_argument("--binary", help="rodas5p binary (runs `triage-list` when --list is absent)")
    parser.add_argument("--output", type=Path, help="write the validation report here instead of stdout")
    parser.add_argument("--dry-run", action="store_true", help="print the applicability matrix and cell counts")
    parser.add_argument("--seconds-per-cell", help="budget estimates, e.g. runs=0.5,profile=40,alloc=3")
    parser.add_argument("--static-reach", type=Path, help="write STATIC_REACH.json here")
    parser.add_argument("--reach-root", action="append", default=[], help="extra root file for --static-reach")
    parser.add_argument("--repo", type=Path, default=tc.REPO)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)
    try:
        list_doc, list_sha = tc.list_document(args.list, args.binary)
        components = tc.load_json(args.components)
        corpus = tc.load_json(args.corpus)
    except tc.TriageError as exc:
        for reason in exc.reasons:
            print("error:", reason, file=sys.stderr)
        return 2
    report = validate(components, corpus, list_doc)
    report["inputs"] = {"components_sha256": tc.sha256_file(args.components),
                        "corpus_sha256": tc.sha256_file(args.corpus), "list_sha256": list_sha}
    text = tc.dumps(report) + "\n"
    if args.output:
        tc.write_text(args.output, text, args.force)
    else:
        sys.stdout.write(text)
    for e in report["errors"]:
        print("error:", e, file=sys.stderr)
    for w in report["warnings"]:
        print("warning:", w, file=sys.stderr)
    if args.dry_run:
        for line in dry_run(list_doc, corpus, report, parse_budget(args.seconds_per_cell)):
            print(line, file=sys.stderr)
    if args.static_reach:
        tc.refuse_existing(args.static_reach, args.force)
        try:
            reach = static_reach(args.repo, components, list_doc, args.reach_root)
        except (tc.TriageError, subprocess.CalledProcessError) as exc:
            print("error: static reach:", exc, file=sys.stderr)
            return 2
        tc.write_text(args.static_reach, tc.dumps(reach) + "\n", args.force)
        unreached = sum(1 for m in reach["modules"] if not m["reachable_from_wired_arm"])
        print(f"wrote {args.static_reach}: {len(reach['modules'])} modules, {unreached} not reachable from wired arms",
              file=sys.stderr)
    return 1 if report["errors"] else 0


if __name__ == "__main__":
    sys.exit(main())
