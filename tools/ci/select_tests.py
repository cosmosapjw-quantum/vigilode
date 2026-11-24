#!/usr/bin/env python3
"""Change-based selection of the tests that the CI test jobs run (owner decisions OD-L1-19, OD-L1-20).

A test job does not re-run the whole suite at every push. It runs the tests that the changes
since its baseline can affect, the tests that are new, and the tests that failed in the baseline
run. The whole suite runs when it is asked for (workflow_dispatch with full), on the main branch,
when no baseline is known, and when a file changed that every test depends on. The owner runs the
whole suite before a release.

Baseline: the head of the most recent earlier run of the same job, on the same branch, whose
outcome file says that its selection ran to the end ("complete"). A cancelled run or a run that
broke before its tests ran leaves no complete outcome, so its changes stay in the next difference.
The baseline run also hands over its coverage map (below).

Coverage map (OD-L1-20): every run executes its selected test targets with coverage
instrumentation and records, per target, the workspace source files that the target executed.
The entry of a target is replaced whenever the target runs; entries of targets that did not run
are carried over from the baseline. A target that ran without leaving a profile loses its entry.

Selection rules, in this order; none of them has a tunable number:
1. FULL when asked, when there is no baseline, when the baseline is not an ancestor of the head,
   or when a global file changed: the workspace manifest and lock file, the toolchain file,
   .cargo/, this tool, and the job's own workflow file.
2. A changed file inside a workspace package: src/**/*.rs, build.rs and Cargo.toml are package
   sources; examples/ and benches/ change the package without its dependents; tests/**/*.rs are
   test sources of the package; any other file is a data file.
3. A modified package source file is classified with a Rust tokenizer: unchanged tokens (only
   whitespace or comments that are not doc comments changed) have no effect; if every changed line
   (old and new side) lies inside the body of a function that is not a const fn, it is a body
   change; anything else is a structural change (items, signatures, consts, statics, types,
   traits, macros, attributes, doc comments, const fn bodies). An added, deleted or renamed
   source file, Cargo.toml and build.rs are structural.
4. A data file: a test can read a file at run time only through a path written in a string
   literal. Every path-like string literal of the Rust sources of the packages is compared with
   the changed file and its parent directories (equal, or a suffix after a '/'). A matching
   literal in a package source makes a structural change of that package; a matching literal in
   tests/ changes that test source. Without a match no test reads the file.
5. A structural change: the package and every package that depends on it (normal, dev or build
   dependency, transitively) run all their tests (unit, integration, binary and doc tests).
6. A body change of a package P, with a coverage map: the targets whose map entry lists a changed
   file run, and so does every target of P and of its dependents that has no map entry; the doc
   tests of P and its dependents run (doc tests have no coverage on the stable toolchain).
   Without a coverage map a body change is treated as structural.
7. A changed test source of a package that does not run all its tests: the test targets whose
   dep-info (written by cargo when it builds the test binaries) lists the file run; if no target
   lists it, every test target of the package runs.
8. Targets that failed in the baseline run are added.
9. Only packages in the job's scope are selected.

Subcommands:
  baseline  --workflow <file> --artifact <name> --branch <b> --run-id <id> --out <json>
  select    --baseline <json> --head <sha> --scope <pkg ...> --workflow-file <path>
            [--full <true|false>] [--profile <p>] --out <json>
  run       --selection <json> --mode normal|ignored [--profile <p>] [--skip <test> ...]
            --outcome <json> [--coverage --baseline <json> --map-out <json>]
  has       --selection <json> --target <package>:<target>   (prints yes or no)
  self-test
"""

import argparse
import concurrent.futures
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

SCHEMA_SELECTION = "vigilode.ci.test-selection.v2"
SCHEMA_OUTCOME = "vigilode.ci.test-outcome.v1"
SCHEMA_MAP = "vigilode.ci.coverage-map.v1"
GLOBAL_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rust-toolchain")
GLOBAL_PREFIXES = (".cargo/", "tools/ci/")
RERUN = re.compile(r"`-p (\S+) (--lib|--doc|--test (\S+)|--bin (\S+))`")
ORDER = ("rodas5p-core", "rodas5p-krylov", "rodas5p-integrators", "rodas5p-fair-ab", "rodas5p-cli",
         "vigilode", "vigilode-coupled")


def sh(args, cwd=None, check=True, env=None):
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True, env=env)
    if check and result.returncode != 0:
        raise SystemExit(f"command failed ({result.returncode}): {' '.join(args)}\n{result.stderr}")
    return result


def repo_root():
    return sh(["git", "rev-parse", "--show-toplevel"]).stdout.strip()


# ---------------------------------------------------------------------------- workspace


def workspace(root):
    """Workspace packages: name -> dir (relative), workspace dependencies, runnable targets."""
    meta = json.loads(sh(["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
                         cwd=root).stdout)
    names = {p["name"] for p in meta["packages"]}
    packages = {}
    for p in meta["packages"]:
        targets = []
        for t in p["targets"]:
            if not t.get("test", True):
                continue
            if "test" in t["kind"]:
                targets.append(t["name"])
            elif "bin" in t["kind"]:
                targets.append("bin:" + t["name"])
            elif any(k in ("lib", "rlib", "dylib", "cdylib", "staticlib") for k in t["kind"]):
                targets.append("lib")
        packages[p["name"]] = {
            "dir": os.path.relpath(os.path.dirname(p["manifest_path"]), root),
            "deps": sorted({d["name"] for d in p["dependencies"] if d["name"] in names}),
            "tests": sorted(t["name"] for t in p["targets"] if "test" in t["kind"]),
            "targets": sorted(set(targets)),
        }
    return packages


def dependents_closure(changed, packages):
    """Every package that is changed or depends, transitively, on a changed package."""
    affected = set(changed)
    grew = True
    while grew:
        grew = False
        for name, info in packages.items():
            if name not in affected and affected.intersection(info["deps"]):
                affected.add(name)
                grew = True
    return affected


def dependencies_closure(name, packages):
    """Every package that `name` depends on, transitively."""
    seen, stack = set(), [name]
    while stack:
        for dep in packages[stack.pop()]["deps"]:
            if dep not in seen:
                seen.add(dep)
                stack.append(dep)
    return seen


def package_of(path, packages):
    best = None
    for name, info in packages.items():
        prefix = info["dir"].rstrip("/") + "/"
        if path.startswith(prefix) and (best is None or len(info["dir"]) > len(packages[best]["dir"])):
            best = name
    return best


def classify(path, packages):
    """('global'|'package'|'package_local'|'test_source'|'data'|'other_rust', package or None)."""
    if path in GLOBAL_FILES or path.startswith(GLOBAL_PREFIXES):
        return "global", None
    name = package_of(path, packages)
    if name is None:
        return ("other_rust" if path.endswith(".rs") else "data"), None
    inner = path[len(packages[name]["dir"].rstrip("/")) + 1:]
    if inner in ("Cargo.toml", "build.rs") or (inner.startswith("src/") and inner.endswith(".rs")):
        return "package", name
    if inner.startswith(("examples/", "benches/")):
        return "package_local", name
    if inner.startswith("tests/") and inner.endswith(".rs"):
        return "test_source", name
    return "data", name


# ---------------------------------------------------------------------------- Rust tokens


def rust_tokens(text):
    """Tokens (kind, value, line) of a Rust source. Whitespace and comments that are not doc
    comments are dropped; doc comments are tokens of kind 'doc'; literals are kind 'lit'."""
    toks, i, n, line = [], 0, len(text), 1
    while i < n:
        c = text[i]
        if c == "\n":
            line, i = line + 1, i + 1
            continue
        if c.isspace():
            i += 1
            continue
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            body = text[i:j]
            if (body.startswith("///") and not body.startswith("////")) or body.startswith("//!"):
                toks.append(("doc", body.rstrip(), line))
            i = j
            continue
        if text.startswith("/*", i):
            start, depth, k = line, 1, i + 2
            while k < n and depth:
                if text.startswith("/*", k):
                    depth, k = depth + 1, k + 2
                elif text.startswith("*/", k):
                    depth, k = depth - 1, k + 2
                else:
                    if text[k] == "\n":
                        line += 1
                    k += 1
            body = text[i:k]
            if (body.startswith("/**") and not body.startswith("/***") and body != "/**/") or body.startswith("/*!"):
                toks.append(("doc", body, start))
            i = k
            continue
        word_start = i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")
        raw = re.match(r'(?:b|c)?r(#*)"', text[i:i + 300]) if c in "bcr" and word_start else None
        if raw:
            hashes = raw.group(1)
            end = text.find('"' + hashes, i + raw.end())
            end = n - 1 if end < 0 else end + len(hashes)
            lit = text[i:end + 1]
            toks.append(("lit", lit, line))
            line += lit.count("\n")
            i = end + 1
            continue
        if c == '"' or (c in "bc" and word_start and text.startswith('"', i + 1)):
            k = i + (1 if c == '"' else 2)
            while k < n and text[k] != '"':
                k += 2 if text[k] == "\\" else 1
            lit = text[i:k + 1]
            toks.append(("lit", lit, line))
            line += lit.count("\n")
            i = k + 1
            continue
        if c == "'":
            m = re.match(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'", text[i:i + 12])
            if m:
                toks.append(("lit", m.group(0), line))
                i += m.end()
                continue
            m = re.match(r"'[A-Za-z_][A-Za-z0-9_]*", text[i:i + 200])
            if m:
                toks.append(("id", m.group(0), line))
                i += m.end()
                continue
        if c.isalpha() or c == "_":
            m = re.match(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", text[i:i + 300])
            toks.append(("id", m.group(0), line))
            i += m.end()
            continue
        if c.isdigit():
            m = re.match(r"[0-9][0-9A-Za-z_]*(?:\.[0-9][0-9A-Za-z_]*)?", text[i:i + 100])
            toks.append(("num", m.group(0), line))
            i += m.end()
            continue
        toks.append(("p", c, line))
        i += 1
    return toks


def fn_body_spans(toks):
    """(line of the opening brace, line of the closing brace, const fn) of every function body."""
    spans, n = [], len(toks)
    for idx, (kind, value, _) in enumerate(toks):
        if kind != "id" or value != "fn" or idx + 1 >= n or toks[idx + 1][0] != "id":
            continue
        j, is_const = idx - 1, False
        while j >= 0 and not (toks[j][0] == "p" and toks[j][1] in ";{}]"):
            if toks[j][0] == "id" and toks[j][1] == "const":
                is_const = True
            j -= 1
        depth, m, opening = 0, idx + 2, None
        while m < n:
            k, v, _ = toks[m]
            if k == "p" and v in "([":
                depth += 1
            elif k == "p" and v in ")]":
                depth -= 1
            elif k == "p" and v == ";" and depth == 0:
                break
            elif k == "p" and v == "{" and depth == 0:
                opening = m
                break
            m += 1
        if opening is None:
            continue
        d, e = 0, opening
        while e < n:
            if toks[e][0] == "p" and toks[e][1] == "{":
                d += 1
            elif toks[e][0] == "p" and toks[e][1] == "}":
                d -= 1
                if d == 0:
                    break
            e += 1
        spans.append((toks[opening][2], toks[e][2] if e < n else 10 ** 9, is_const))
    return spans


def inside_body(line, spans):
    return any(start < line < end and not const for start, end, const in spans)


def hunk_lines(diff_text):
    """Old and new line numbers of the changed lines of a unified diff with zero context."""
    old, new = set(), set()
    for m in re.finditer(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", diff_text, re.M):
        a, b = int(m.group(1)), int(m.group(2) if m.group(2) is not None else 1)
        c, d = int(m.group(3)), int(m.group(4) if m.group(4) is not None else 1)
        old.update(range(a, a + b))
        new.update(range(c, c + d))
    return old, new


def classify_texts(old_text, new_text, diff_text):
    """'noop', 'body' or 'structural' for one modified Rust source."""
    old_toks, new_toks = rust_tokens(old_text), rust_tokens(new_text)
    if [(k, v) for k, v, _ in old_toks] == [(k, v) for k, v, _ in new_toks]:
        return "noop"
    old_lines, new_lines = hunk_lines(diff_text)
    old_spans, new_spans = fn_body_spans(old_toks), fn_body_spans(new_toks)
    if all(inside_body(x, old_spans) for x in old_lines) and all(inside_body(x, new_spans) for x in new_lines):
        return "body"
    return "structural"


def classify_source_change(root, base, head, path):
    old = sh(["git", "show", f"{base}:{path}"], cwd=root, check=False)
    new = sh(["git", "show", f"{head}:{path}"], cwd=root, check=False)
    if old.returncode != 0 or new.returncode != 0:
        return "structural"
    diff = sh(["git", "diff", "-U0", "--no-color", base, head, "--", path], cwd=root).stdout
    return classify_texts(old.stdout, new.stdout, diff)


# ---------------------------------------------------------------------------- string literals


def rust_string_literals(text):
    """The contents of the string literals of a Rust source (comments and char literals skipped)."""
    out = []
    for kind, value, _ in rust_tokens(text):
        if kind != "lit" or value.startswith("'"):
            continue
        m = re.match(r'^(?:b|c)?(?:r(#*))?"(.*)"(#*)$', value, re.S)
        if m:
            out.append(m.group(2))
    return out


def normalise_literal(literal):
    """A path-like literal as a relative path without ./, ../, leading or trailing '/'."""
    cut = literal.split("{", 1)[0]
    if "/" not in cut and "." not in cut:
        return None
    parts = [p for p in cut.replace("\\", "/").split("/") if p not in ("", ".", "..")]
    return "/".join(parts) or None


def literal_matches(changed_path, literal):
    norm = normalise_literal(literal)
    if not norm:
        return False
    candidates = [changed_path]
    head = changed_path
    while "/" in head:
        head = head.rsplit("/", 1)[0]
        candidates.append(head)
    return any(c == norm or c.endswith("/" + norm) for c in candidates)


def rust_sources(root, packages):
    files = []
    for info in packages.values():
        base = os.path.join(root, info["dir"])
        for directory, dirs, names in os.walk(base):
            dirs[:] = [d for d in dirs if d != "target"]
            files += [os.path.relpath(os.path.join(directory, f), root) for f in names if f.endswith(".rs")]
    return sorted(files)


# ---------------------------------------------------------------------------- cargo artifacts


def parse_dep_info(text):
    """The prerequisites of the first rule of a make-style dep-info file."""
    line = text.replace("\\\n", " ").split("\n", 1)[0]
    if ":" not in line:
        return []
    deps = line.split(":", 1)[1]
    out, buf, i = [], [], 0
    while i < len(deps):
        ch = deps[i]
        if ch == "\\" and i + 1 < len(deps) and deps[i + 1] == " ":
            buf.append(" ")
            i += 2
            continue
        if ch.isspace():
            if buf:
                out.append("".join(buf))
                buf = []
        else:
            buf.append(ch)
        i += 1
    if buf:
        out.append("".join(buf))
    return out


def target_key(target):
    """'lib', 'bin:<name>' or the test name, from a cargo artifact target."""
    kinds = target["kind"]
    if "test" in kinds:
        return target["name"]
    if "bin" in kinds:
        return "bin:" + target["name"]
    return "lib"


def build_artifacts(root, package, profile, filters, env=None):
    """Build the test executables of a package (with target filters) and return
    ({key: (executable, dep-info sources)}, [non-test binaries], exit code)."""
    args = ["cargo", "test", "--locked", "--no-run", "--message-format=json", "-p", package]
    if profile and profile != "dev":
        args += ["--profile", profile]
    args += filters
    result = sh(args, cwd=root, check=False, env=env)
    tests, bins = {}, []
    for line in result.stdout.splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-artifact":
            continue
        exe = msg.get("executable")
        if not exe:
            continue
        if msg.get("profile", {}).get("test"):
            deps = []
            if os.path.exists(exe + ".d"):
                with open(exe + ".d") as f:
                    deps = sorted({os.path.relpath(d, root) for d in parse_dep_info(f.read())})
            tests[target_key(msg["target"])] = (exe, deps)
        elif "bin" in msg["target"]["kind"]:
            bins.append(exe)
    return tests, sorted(set(bins)), result.returncode


# ---------------------------------------------------------------------------- select


def changed_entries(root, base, head):
    """(status letter, path) for every changed path; both sides of a rename."""
    out = sh(["git", "diff", "--name-status", "-M", base, head], cwd=root).stdout
    entries = []
    for line in out.splitlines():
        parts = line.split("\t")
        entries += [(parts[0][0], p) for p in parts[1:]]
    return sorted(set(entries), key=lambda e: e[1])


def load_map(path):
    if not path or not os.path.exists(path):
        return None
    with open(path) as f:
        data = json.load(f)
    return data.get("targets") if data.get("schema") == SCHEMA_MAP else None


def entry_of(selection, name):
    return selection["packages"].setdefault(
        name, {"all": False, "targets": [], "lib": False, "doc": False, "why": {}})


def select(args):
    root = repo_root()
    with open(args.baseline) as f:
        baseline = json.load(f)
    head, base, scope = args.head, baseline.get("base"), list(args.scope)
    reasons, full = [], False
    if str(args.full).lower() == "true":
        full, reasons = True, ["the whole suite was asked for"]
    elif not base:
        full, reasons = True, [baseline.get("note") or "no baseline run with a complete outcome"]
    elif sh(["git", "merge-base", "--is-ancestor", base, head], cwd=root, check=False).returncode != 0:
        full, reasons = True, [f"the baseline {base} is not an ancestor of the head {head}"]
    packages = workspace(root)
    cmap = load_map(baseline.get("map_path"))
    selection = {"schema": SCHEMA_SELECTION, "head": head, "base": base, "scope": scope,
                 "baseline_run": baseline.get("run_id"), "mode": "full" if full else "selected",
                 "reasons": reasons, "changed_files": [], "packages": {}, "notes": [],
                 "classified": {}, "coverage_map_targets": None if cmap is None else len(cmap),
                 "baseline_failures_added": []}
    entries = []
    if not full:
        entries = changed_entries(root, base, head)
        selection["changed_files"] = sorted({p for _, p in entries})
        workflow = os.path.normpath(args.workflow_file)
        globals_changed = sorted({p for _, p in entries if classify(p, packages)[0] == "global" or p == workflow})
        if globals_changed:
            full = True
            selection["mode"] = "full"
            selection["reasons"] = [f"global file changed: {p}" for p in globals_changed]
    if full:
        for name in scope:
            selection["packages"][name] = {"all": True, "targets": [], "lib": True, "doc": True,
                                           "why": selection["reasons"]}
        return finish(selection, args.out)

    why_pkg = {}
    structural, local_pkgs, body_files, test_sources, data = set(), set(), {}, {}, []
    for status, path in entries:
        kind, name = classify(path, packages)
        if kind == "package":
            cls = "structural"
            if status == "M" and path.endswith(".rs"):
                cls = classify_source_change(root, base, head, path)
            selection["classified"][path] = cls
            if cls == "structural":
                structural.add(name)
                why_pkg.setdefault(name, []).append(f"structural change: {path}")
            elif cls == "body":
                body_files.setdefault(name, set()).add(path)
        elif kind == "package_local":
            local_pkgs.add(name)
            why_pkg.setdefault(name, []).append(f"example or bench changed: {path}")
        elif kind == "test_source":
            test_sources.setdefault(name, {}).setdefault(path, []).append("changed")
        elif kind == "data":
            data.append(path)
    if data:
        literals = {}
        for src in rust_sources(root, packages):
            with open(os.path.join(root, src), encoding="utf-8", errors="replace") as f:
                literals[src] = rust_string_literals(f.read())
        for path in data:
            hits = sorted(src for src, lits in literals.items() if any(literal_matches(path, x) for x in lits))
            if not hits:
                selection["notes"].append(f"read by no test: {path}")
            for src in hits:
                kind, name = classify(src, packages)
                if kind == "test_source":
                    test_sources.setdefault(name, {}).setdefault(src, []).append(f"reads {path}")
                elif kind in ("package", "package_local"):
                    (structural if kind == "package" else local_pkgs).add(name)
                    why_pkg.setdefault(name, []).append(f"{src} reads {path}")
    if body_files and cmap is None:
        for name, files in body_files.items():
            structural.add(name)
            why_pkg.setdefault(name, []).append(
                "body change without a coverage map: " + ", ".join(sorted(files)))
        selection["notes"].append("no coverage map from the baseline: body changes use the package rule")
        body_files = {}
    affected = dependents_closure(structural, packages) | local_pkgs
    for name in sorted(affected):
        if name in scope:
            reasons_here = why_pkg.get(name) or ["depends on changed package(s): " + ", ".join(
                sorted(structural & dependencies_closure(name, packages)))]
            selection["packages"][name] = {"all": True, "targets": [], "lib": True, "doc": True,
                                           "why": reasons_here}
    for name, files in sorted(body_files.items()):
        reach = dependents_closure({name}, packages)
        for key, item in sorted(cmap.items()):
            pkg, target = key.split("::", 1)
            if pkg not in scope or pkg in affected or pkg not in reach:
                continue
            hit = sorted(set(item.get("files", [])) & files)
            if hit:
                entry = entry_of(selection, pkg)
                if target == "lib":
                    entry["lib"] = True
                else:
                    entry["targets"].append(target)
                entry["why"].setdefault(target, []).append("executed a changed file: " + ", ".join(hit))
        for pkg in sorted(reach):
            if pkg not in scope or pkg in affected:
                continue
            entry = entry_of(selection, pkg)
            entry["doc"] = True
            entry["why"].setdefault("doc", []).append(f"doc tests; body change in {name}")
            for target in packages[pkg]["targets"]:
                if f"{pkg}::{target}" not in cmap:
                    if target == "lib":
                        entry["lib"] = True
                    else:
                        entry["targets"].append(target)
                    entry["why"].setdefault(target, []).append("no coverage entry yet")
    build_env = dict(os.environ, LLVM_PROFILE_FILE=os.path.join(tempfile.mkdtemp(prefix="ci-build-"), "%p-%m.profraw"))
    for name, files in sorted(test_sources.items()):
        if name in affected or name not in scope:
            continue
        built, _, code = build_artifacts(root, name, args.profile, [], env=build_env)
        targets, unlisted = {}, []
        for path, how in sorted(files.items()):
            listing = [t for t, (_, deps) in built.items() if path in deps]
            for t in listing:
                targets.setdefault(t, []).append(f"{path}: {', '.join(how)}")
            if not listing:
                unlisted.append(path)
        if unlisted or code != 0:
            for t in packages[name]["tests"]:
                targets.setdefault(t, []).append(
                    "every test target: " + "; ".join([f"listed by no target: {p}" for p in unlisted]
                                                      + ([f"cargo test --no-run exited {code}"] if code else [])))
        entry = entry_of(selection, name)
        for t, why in targets.items():
            if t == "lib":
                entry["lib"] = True
            else:
                entry["targets"].append(t)
            entry["why"].setdefault(t, []).extend(why)
    for item in baseline.get("failed", []):
        name, target = item.get("package"), item.get("target")
        if name not in scope or name not in packages:
            continue
        entry = entry_of(selection, name)
        if entry["all"]:
            continue
        if target == "lib":
            entry["lib"] = True
        elif target == "doc":
            entry["doc"] = True
        else:
            entry["targets"].append(target)
        if isinstance(entry["why"], dict):
            entry["why"].setdefault(target, []).append("failed in the baseline run")
        selection["baseline_failures_added"].append(f"{name}:{target}")
    for entry in selection["packages"].values():
        entry["targets"] = sorted(set(entry["targets"]))
    return finish(selection, args.out)


def finish(selection, out):
    with open(out, "w") as f:
        json.dump(selection, f, indent=2)
        f.write("\n")
    lines = [f"### Test selection: {selection['mode']}",
             f"base `{selection['base']}`, head `{selection['head']}`, "
             f"{len(selection['changed_files'])} changed files, coverage map: "
             f"{selection.get('coverage_map_targets')} targets"]
    lines += [f"- {r}" for r in selection["reasons"]]
    if not selection["packages"]:
        lines.append("- no test is affected by the changes")
    for name, entry in sorted(selection["packages"].items()):
        if entry["all"]:
            lines.append(f"- `{name}`: every test")
        else:
            parts = (["lib"] if entry["lib"] else []) + entry["targets"] + (["doc"] if entry.get("doc") else [])
            lines.append(f"- `{name}`: {', '.join(parts) if parts else 'nothing'}")
    lines += [f"- {path}: {cls}" for path, cls in sorted(selection.get("classified", {}).items())]
    lines += [f"- added from the baseline failures: {x}" for x in selection["baseline_failures_added"]]
    lines += [f"- note: {n}" for n in selection["notes"]]
    text = "\n".join(lines) + "\n"
    print(text)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as f:
            f.write(text)
    return 0


# ---------------------------------------------------------------------------- run


def target_args(target):
    if target == "lib":
        return ["--lib"]
    if target == "doc":
        return ["--doc"]
    if target.startswith("bin:"):
        return ["--bin", target[4:]]
    return ["--test", target]


def harness_args(mode, skips):
    if mode != "ignored":
        return []
    out = ["--", "--ignored"]
    for s in skips:
        out += ["--skip", s]
    return out


def ordered(names):
    return sorted(names, key=lambda n: (ORDER.index(n) if n in ORDER else len(ORDER), n))


def commands(selection, mode, profile, skips):
    """One cargo test per package (without coverage)."""
    cmds = []
    for name in ordered(selection["packages"]):
        entry = selection["packages"][name]
        base = ["cargo", "test", "--locked", "--no-fail-fast"] + (["--profile", profile] if profile and profile != "dev" else [])
        if entry["all"]:
            cmds.append(base + ["-p", name] + harness_args(mode, skips))
            continue
        filters = (["--lib"] if entry["lib"] else []) + [a for t in entry["targets"] for a in target_args(t)]
        if filters:
            cmds.append(base + ["-p", name] + filters + harness_args(mode, skips))
        if entry.get("doc"):
            cmds.append(base + ["-p", name, "--doc"] + harness_args(mode, skips))
    return cmds


def units(selection, packages):
    """(package, target) for every unit that runs; 'doc' for doc tests."""
    out = []
    for name in ordered(selection["packages"]):
        entry = selection["packages"][name]
        if entry["all"]:
            out += [(name, t) for t in packages[name]["targets"]] + [(name, "doc")]
            continue
        out += ([(name, "lib")] if entry["lib"] else []) + [(name, t) for t in entry["targets"]]
        if entry.get("doc"):
            out.append((name, "doc"))
    return out


def llvm_bin():
    override = os.environ.get("VIGILODE_LLVM_BIN")
    if override:
        return override
    sysroot = sh(["rustc", "--print", "sysroot"]).stdout.strip()
    host = re.search(r"^host: (\S+)", sh(["rustc", "-vV"]).stdout, re.M).group(1)
    return os.path.join(sysroot, "lib", "rustlib", host, "bin")


def covered_files(root, llvm, profraws, exe, objects, workdir):
    """Workspace files with an executed region in the given profiles, or None on failure."""
    merged = os.path.join(workdir, "merged.profdata")
    r = sh([os.path.join(llvm, "llvm-profdata"), "merge", "-sparse", "-o", merged] + profraws, check=False)
    if r.returncode != 0:
        return None
    args = [os.path.join(llvm, "llvm-cov"), "export", "-summary-only", "-instr-profile", merged,
            "-ignore-filename-regex", r"(/\.cargo/registry/|/\.rustup/|^/rustc/)", exe]
    for o in objects:
        args += ["-object", o]
    r = sh(args, check=False)
    if r.returncode != 0:
        return None
    files = set()
    for block in json.loads(r.stdout).get("data", []):
        for f in block.get("files", []):
            if f.get("summary", {}).get("regions", {}).get("covered", 0) > 0:
                rel = os.path.relpath(f["filename"], root)
                if not rel.startswith(".."):
                    files.add(rel)
    return sorted(files)


def run(args):
    root = repo_root()
    with open(args.selection) as f:
        selection = json.load(f)
    outcome = {"schema": SCHEMA_OUTCOME, "head": selection["head"], "mode": args.mode,
               "selection_mode": selection["mode"], "complete": True, "commands": [], "failed": [],
               "coverage": None}
    status = 0

    def execute(cmd, env=None):
        nonlocal status
        print("+ " + " ".join(cmd), flush=True)
        proc = subprocess.Popen(cmd, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=env)
        lines = []
        for line in proc.stdout:
            sys.stdout.write(line)
            lines.append(line)
        code = proc.wait()
        failed = set()
        for m in RERUN.finditer("".join(lines)):
            target = "lib" if m.group(2) == "--lib" else "doc" if m.group(2) == "--doc" else (
                m.group(3) or "bin:" + m.group(4))
            failed.add((m.group(1), target))
        broken = code != 0 and not failed
        outcome["commands"].append({"command": " ".join(cmd), "exit_code": code,
                                    "failed_targets": sorted(f"{p}:{t}" for p, t in failed),
                                    "did_not_run_its_tests": broken})
        outcome["failed"] += [{"package": p, "target": t} for p, t in sorted(failed)]
        if broken:
            outcome["complete"] = False
        if code != 0:
            status = 1
        return code

    if not args.coverage:
        for cmd in commands(selection, args.mode, args.profile, args.skip):
            execute(cmd)
    else:
        packages = workspace(root)
        llvm = llvm_bin()
        base_map = {}
        if args.baseline and selection["mode"] != "full":
            with open(args.baseline) as f:
                base_map = load_map(json.load(f).get("map_path")) or {}
        cmap = dict(base_map)
        covdir = args.coverage_dir or tempfile.mkdtemp(prefix="ci-coverage-")
        junk = os.path.join(covdir, "_build", "%p-%m.profraw")
        env = dict(os.environ, LLVM_PROFILE_FILE=junk)
        prof = ["--profile", args.profile] if args.profile and args.profile != "dev" else []
        plan = units(selection, packages)
        executables, bins = {}, {}
        for name in ordered({p for p, _ in plan}):
            filters = [] if selection["packages"][name]["all"] else [
                a for p, t in plan if p == name and t != "doc" for a in target_args(t)]
            if not filters and not selection["packages"][name]["all"]:
                continue
            built, pkg_bins, code = build_artifacts(root, name, args.profile, filters, env=env)
            for key, (exe, _) in built.items():
                executables[(name, key)] = exe
            bins[name] = pkg_bins
            if code != 0:
                execute(["cargo", "test", "--locked", "--no-run"] + prof + ["-p", name] + filters, env=env)
        jobs = []
        for name, target in plan:
            key = f"{name}::{target}"
            workdir = os.path.join(covdir, re.sub(r"[^A-Za-z0-9_.-]", "_", key))
            shutil.rmtree(workdir, ignore_errors=True)
            os.makedirs(workdir)
            run_env = dict(os.environ, LLVM_PROFILE_FILE=junk if target == "doc"
                           else os.path.join(workdir, "%p-%m.profraw"))
            execute(["cargo", "test", "--locked", "--no-fail-fast"] + prof + ["-p", name]
                    + target_args(target) + harness_args(args.mode, args.skip), env=run_env)
            if target != "doc":
                jobs.append((key, workdir, executables.get((name, target)), bins.get(name, [])))

        def export(job):
            key, workdir, exe, objects = job
            profraws = sorted(glob.glob(os.path.join(workdir, "*.profraw")))
            files = covered_files(root, llvm, profraws, exe, objects, workdir) if profraws and exe else None
            shutil.rmtree(workdir, ignore_errors=True)
            return key, files

        # The exports are independent; they run after the tests, one per processor.
        refreshed, lost = [], []
        with concurrent.futures.ThreadPoolExecutor(max_workers=os.cpu_count() or 1) as pool:
            for key, files in pool.map(export, jobs):
                if files is None:
                    cmap.pop(key, None)
                    lost.append(key)
                else:
                    cmap[key] = {"files": files, "head": selection["head"]}
                    refreshed.append(key)
        outcome["coverage"] = {"refreshed": len(refreshed), "lost": lost, "map_targets": len(cmap),
                               "llvm_bin": llvm}
        with open(args.map_out, "w") as f:
            json.dump({"schema": SCHEMA_MAP, "head": selection["head"], "mode": args.mode, "targets": cmap}, f)
            f.write("\n")
    with open(args.outcome, "w") as f:
        json.dump(outcome, f, indent=2)
        f.write("\n")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as f:
            f.write(f"### Test outcome ({args.mode}): {len(outcome['commands'])} commands, "
                    f"{len(outcome['failed'])} failing targets, complete: {outcome['complete']}\n")
            if outcome["coverage"]:
                f.write(f"- coverage map: {outcome['coverage']['refreshed']} entries refreshed, "
                        f"{len(outcome['coverage']['lost'])} lost, {outcome['coverage']['map_targets']} in all\n")
            f.writelines(f"- failed `{x['package']}` `{x['target']}`\n" for x in outcome["failed"])
    return status


def has(args):
    with open(args.selection) as f:
        selection = json.load(f)
    package, target = args.target.split(":", 1)
    entry = selection["packages"].get(package)
    print("yes" if entry and (entry["all"] or target in entry["targets"]) else "no")
    return 0


# ---------------------------------------------------------------------------- baseline


def baseline(args):
    repo = os.environ.get("GITHUB_REPOSITORY")
    out = {"base": None, "run_id": None, "failed": [], "map_path": None, "note": ""}
    if not repo or not shutil.which("gh"):
        out["note"] = "no GitHub repository or gh in this environment"
    else:
        api = (f"repos/{repo}/actions/workflows/{args.workflow}/runs"
               f"?branch={args.branch}&status=completed&per_page=100")
        listing = sh(["gh", "api", api], check=False)
        runs = json.loads(listing.stdout).get("workflow_runs", []) if listing.returncode == 0 else []
        runs = sorted((r for r in runs if str(r["id"]) != str(args.run_id)),
                      key=lambda r: r["created_at"], reverse=True)
        for r in runs:
            tmp = tempfile.mkdtemp()
            got = sh(["gh", "run", "download", str(r["id"]), "-R", repo, "-n", args.artifact, "-D", tmp],
                     check=False)
            path = os.path.join(tmp, "outcome.json")
            if got.returncode != 0 or not os.path.exists(path):
                continue
            with open(path) as f:
                outcome = json.load(f)
            if outcome.get("schema") != SCHEMA_OUTCOME or not outcome.get("complete"):
                continue
            map_path = os.path.join(tmp, "map.json")
            if os.path.exists(map_path):
                keep = os.path.join(os.path.dirname(os.path.abspath(args.out)), "baseline_map.json")
                shutil.copyfile(map_path, keep)
                out["map_path"] = keep
            out.update(base=outcome["head"], run_id=r["id"], failed=outcome.get("failed", []),
                       note=f"run {r['id']} ran its selection to the end at {outcome['head'][:8]}")
            break
        else:
            out["note"] = "no earlier run of this job on this branch has a complete outcome"
    with open(args.out, "w") as f:
        json.dump(out, f, indent=2)
        f.write("\n")
    print(json.dumps(out, indent=2))
    return 0


# ---------------------------------------------------------------------------- self-test


def self_test():
    src = r'''
    // "tests/golden/in_a_comment.json"
    /* "fixtures/in_a_block.json" /* nested */ */
    let a = "tests/golden/default_path";
    let b = r#"research/x_2026/RESULTS.json"#;
    let c = '"';
    let d = b"fixtures/bytes.bin";
    fn f<'a>(x: &'a str) -> &'a str { x }
    let e = "escaped \" quote/inside.txt";
    let g = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/rodas5p_coefficients_snapshot.json");
    let h = format!("research/{}/cases.json", name);
    '''
    lits = rust_string_literals(src)
    assert "tests/golden/default_path" in lits, lits
    assert "research/x_2026/RESULTS.json" in lits, lits
    assert "fixtures/bytes.bin" in lits, lits
    assert not any("in_a_comment" in s or "in_a_block" in s for s in lits), lits
    assert any(s.endswith("rodas5p_coefficients_snapshot.json") for s in lits), lits
    assert literal_matches("crates/vigilode/tests/golden/default_path/dopri5.txt", "tests/golden/default_path")
    assert literal_matches("fixtures/rodas5p_coefficients_snapshot.json",
                           "/../../fixtures/rodas5p_coefficients_snapshot.json")
    assert literal_matches("research/abc/cases.json", "research/{}/cases.json")
    assert not literal_matches("research/production_loop_20261116/PROGRESS.md",
                               "research/production_loop_20261116/mf_adaptive")
    assert literal_matches("research/production_loop_20261116/mf_adaptive/RECORDING_A.json",
                           "research/production_loop_20261116/mf_adaptive")
    assert not literal_matches("docs/x.md", "hello world")
    deps = parse_dep_info("/t/deps/a-1: /r/tests/a.rs /r/tests/common/mod.rs /r/tests/my\\ file.rs\n\n"
                          "/r/tests/a.rs:\n")
    assert deps == ["/r/tests/a.rs", "/r/tests/common/mod.rs", "/r/tests/my file.rs"], deps
    pk = {"core": {"deps": []}, "kry": {"deps": ["core"]}, "int": {"deps": ["core", "kry"]},
          "vig": {"deps": ["core", "int"]}, "cpl": {"deps": ["core", "int"]}}
    assert dependents_closure({"int"}, pk) == {"int", "vig", "cpl"}
    assert dependents_closure({"vig"}, pk) == {"vig"}
    assert dependents_closure({"core"}, pk) == set(pk)
    assert dependencies_closure("vig", pk) == {"core", "int", "kry"}
    pk2 = {"vigilode": {"dir": "crates/vigilode", "deps": []},
           "rodas5p-core": {"dir": "crates/rodas5p-core", "deps": []}}
    assert classify("crates/vigilode/src/solve.rs", pk2) == ("package", "vigilode")
    assert classify("crates/vigilode/Cargo.toml", pk2) == ("package", "vigilode")
    assert classify("crates/vigilode/tests/a.rs", pk2) == ("test_source", "vigilode")
    assert classify("crates/vigilode/tests/golden/x.json", pk2) == ("data", "vigilode")
    assert classify("crates/vigilode/src/data.json", pk2) == ("data", "vigilode")
    assert classify("crates/vigilode/examples/e.rs", pk2) == ("package_local", "vigilode")
    assert classify("Cargo.lock", pk2) == ("global", None)
    assert classify("tools/ci/select_tests.py", pk2) == ("global", None)
    assert classify("research/x/RESULTS.json", pk2) == ("data", None)
    assert classify("research/x/runner/src/main.rs", pk2) == ("other_rust", None)
    m = RERUN.findall("error: test failed, to rerun pass `-p rodas5p-integrators --test stage_residual_telemetry`\n"
                      "    `-p vigilode --lib`\n    `-p vigilode --doc`")
    assert [x[0] for x in m] == ["rodas5p-integrators", "vigilode", "vigilode"], m
    sel = {"packages": {"vigilode": {"all": True, "targets": [], "lib": True, "doc": True},
                        "rodas5p-core": {"all": False, "targets": ["a", "b"], "lib": False, "doc": True},
                        "rodas5p-krylov": {"all": False, "targets": [], "lib": False, "doc": False}}}
    cmds = commands(sel, "ignored", "measurement", ["kill"])
    assert len(cmds) == 3, cmds
    assert cmds[0] == ["cargo", "test", "--locked", "--no-fail-fast", "--profile", "measurement", "-p",
                       "rodas5p-core", "--test", "a", "--test", "b", "--", "--ignored", "--skip", "kill"], cmds
    assert cmds[1][-6:] == ["rodas5p-core", "--doc", "--", "--ignored", "--skip", "kill"], cmds
    assert cmds[2][-5:] == ["vigilode", "--", "--ignored", "--skip", "kill"], cmds
    # change classification
    old = """use x::y;

/// Doc of f.
pub fn f(a: f64) -> f64 {
    let b = a * 2.0; // twice
    b + 1.0
}

const LIMIT: usize = 4;

pub const fn g() -> usize {
    3
}

impl S {
    fn h(&self) -> u8 {
        self.v
    }
}
"""
    def diff_of(a, b):
        with tempfile.TemporaryDirectory() as d:
            pa, pb = os.path.join(d, "a.rs"), os.path.join(d, "b.rs")
            open(pa, "w").write(a)
            open(pb, "w").write(b)
            return subprocess.run(["git", "diff", "--no-index", "-U0", "--no-color", pa, pb],
                                  capture_output=True, text=True).stdout
    cases = [
        (old.replace("// twice", "// two times"), "noop"),
        (old.replace("a * 2.0", "a * 3.0"), "body"),
        (old.replace("self.v", "self.v + 1"), "body"),
        (old.replace("LIMIT: usize = 4", "LIMIT: usize = 5"), "structural"),
        (old.replace("/// Doc of f.", "/// Doc of the function f."), "structural"),
        (old.replace("pub fn f(a: f64)", "pub fn f(a: f32)"), "structural"),
        (old.replace("    3\n", "    4\n"), "structural"),
        (old.replace("use x::y;", "use x::z;"), "structural"),
        (old.replace("    b + 1.0\n", "    b + 1.0\n    // note\n"), "noop"),
    ]
    for new, want in cases:
        got = classify_texts(old, new, diff_of(old, new))
        assert got == want, (want, got, new)
    spans = fn_body_spans(rust_tokens(old))
    assert [(s, e, c) for s, e, c in spans] == [(4, 7, False), (11, 13, True), (16, 18, False)], spans
    toks = rust_tokens("let s = r#\"a \"quoted\" b\"#; let c = b'x'; fn q<'a>() {}")
    assert ("lit", 'r#"a "quoted" b"#', 1) in toks, toks
    print("self-test passed")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("baseline")
    b.add_argument("--workflow", required=True)
    b.add_argument("--artifact", required=True)
    b.add_argument("--branch", required=True)
    b.add_argument("--run-id", required=True)
    b.add_argument("--out", required=True)
    s = sub.add_parser("select")
    s.add_argument("--baseline", required=True)
    s.add_argument("--head", required=True)
    s.add_argument("--scope", nargs="+", required=True)
    s.add_argument("--workflow-file", required=True)
    s.add_argument("--full", default="false")
    s.add_argument("--profile", default="dev")
    s.add_argument("--out", required=True)
    r = sub.add_parser("run")
    r.add_argument("--selection", required=True)
    r.add_argument("--mode", choices=("normal", "ignored"), required=True)
    r.add_argument("--profile", default="dev")
    r.add_argument("--skip", action="append", default=[])
    r.add_argument("--outcome", required=True)
    r.add_argument("--coverage", action="store_true")
    r.add_argument("--baseline")
    r.add_argument("--map-out", default="map.json")
    r.add_argument("--coverage-dir")
    h = sub.add_parser("has")
    h.add_argument("--selection", required=True)
    h.add_argument("--target", required=True)
    sub.add_parser("self-test")
    args = parser.parse_args()
    handlers = {"baseline": baseline, "select": select, "run": run, "has": has,
                "self-test": lambda _a: self_test()}
    return handlers[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
