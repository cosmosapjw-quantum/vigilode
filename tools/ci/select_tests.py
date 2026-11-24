#!/usr/bin/env python3
"""Change-based selection of the tests that the CI test jobs run (owner decision OD-L1-19).

A test job does not re-run the whole suite at every push. It runs the tests that the changes
since its baseline can affect, the tests that are new, and the tests that failed in the baseline
run. The whole suite runs when it is asked for (workflow_dispatch with full), on the main branch,
when no baseline is known, and when a file changed that every test depends on. The owner runs the
whole suite before a release.

Baseline: the head of the most recent earlier run of the same job, on the same branch, whose
outcome file says that its selection ran to the end ("complete"). A cancelled run or a run that
broke before its tests ran leaves no complete outcome, so its changes stay in the next difference.

Selection rules, in this order; none of them has a tunable number:
1. FULL when asked, when there is no baseline, when the baseline is not an ancestor of the head,
   or when a global file changed: the workspace manifest and lock file, the toolchain file,
   .cargo/, this tool, and the job's own workflow file.
2. A changed file inside a workspace package: src/**/*.rs, build.rs and Cargo.toml change the
   package; examples/ and benches/ change the package without its dependents; tests/**/*.rs
   change a test source of the package; any other file is a data file.
3. A data file (any changed file that is not Rust source of a package): a test can read a file at
   run time only through a path written in a string literal. Every path-like string literal of
   the Rust sources of the packages is compared with the changed file and its parent
   directories (equal, or a suffix after a '/'). A matching literal in src/ or build.rs changes
   its package; a matching literal in tests/ changes that test source. Without a match no test
   reads the file.
4. Every package that depends on a changed package (normal, dev or build dependency,
   transitively) is affected: all its tests run (unit, integration and doc tests).
5. A changed test source of a package that is not affected: the test targets whose dep-info
   (written by cargo when it builds the test binaries) lists the file run; if no target lists it,
   every test target of the package runs.
6. Targets that failed in the baseline run are added.
7. Only packages in the job's scope are selected.

Subcommands:
  baseline  --workflow <file> --artifact <name> --branch <b> --run-id <id> --out <json>
  select    --baseline <json> --head <sha> --scope <pkg ...> --workflow-file <path>
            [--full <true|false>] [--profile <p>] --out <json>
  run       --selection <json> --mode normal|ignored [--profile <p>] [--skip <test> ...]
            --outcome <json>
  has       --selection <json> --target <package>:<target>   (prints yes or no)
  self-test
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

SCHEMA_SELECTION = "vigilode.ci.test-selection.v1"
SCHEMA_OUTCOME = "vigilode.ci.test-outcome.v1"
GLOBAL_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rust-toolchain")
GLOBAL_PREFIXES = (".cargo/", "tools/ci/")
RERUN = re.compile(r"`-p (\S+) (--lib|--doc|--test (\S+)|--bin (\S+))`")
ORDER = ("rodas5p-core", "rodas5p-krylov", "rodas5p-integrators", "rodas5p-fair-ab", "rodas5p-cli",
         "vigilode", "vigilode-coupled")


def sh(args, cwd=None, check=True):
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if check and result.returncode != 0:
        raise SystemExit(f"command failed ({result.returncode}): {' '.join(args)}\n{result.stderr}")
    return result


def repo_root():
    return sh(["git", "rev-parse", "--show-toplevel"]).stdout.strip()


# ---------------------------------------------------------------------------- workspace


def workspace(root):
    """Workspace packages: name -> dir (relative), workspace dependencies, test targets."""
    meta = json.loads(sh(["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
                         cwd=root).stdout)
    names = {p["name"] for p in meta["packages"]}
    packages = {}
    for p in meta["packages"]:
        packages[p["name"]] = {
            "dir": os.path.relpath(os.path.dirname(p["manifest_path"]), root),
            "deps": sorted({d["name"] for d in p["dependencies"] if d["name"] in names}),
            "tests": sorted(t["name"] for t in p["targets"] if "test" in t["kind"]),
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


# ---------------------------------------------------------------------------- string literals


def rust_string_literals(text):
    """The contents of the string literals of a Rust source (comments and char literals skipped)."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j + 1
            continue
        if text.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if text.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif text.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
            continue
        word_start = i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")
        raw = re.match(r'(?:b|c)?r(#*)"', text[i:i + 300]) if c in "bcr" and word_start else None
        if raw:
            hashes = raw.group(1)
            start = i + raw.end()
            end = text.find('"' + hashes, start)
            end = n if end < 0 else end
            out.append(text[start:end])
            i = end + 1 + len(hashes)
            continue
        if c == '"' or (c in "bc" and word_start and text.startswith('"', i + 1)):
            i += 1 if c == '"' else 2
            buf = []
            while i < n and text[i] != '"':
                if text[i] == "\\" and i + 1 < n:
                    buf.append(text[i:i + 2])
                    i += 2
                else:
                    buf.append(text[i])
                    i += 1
            out.append("".join(buf))
            i += 1
            continue
        if c == "'":
            m = re.match(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'", text[i:i + 12])
            if m:
                i += m.end()
                continue
        i += 1
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


# ---------------------------------------------------------------------------- dep-info


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


def test_target_sources(root, package, profile):
    """Build the test binaries of a package and map each test target to its source files."""
    args = ["cargo", "test", "--locked", "--no-run", "--message-format=json", "-p", package]
    if profile and profile != "dev":
        args += ["--profile", profile]
    result = sh(args, cwd=root, check=False)
    mapping = {}
    for line in result.stdout.splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-artifact" or not msg.get("profile", {}).get("test"):
            continue
        if "test" not in msg["target"]["kind"]:
            continue
        exe = msg.get("executable")
        if not exe or not os.path.exists(exe + ".d"):
            continue
        with open(exe + ".d") as f:
            deps = parse_dep_info(f.read())
        mapping[msg["target"]["name"]] = sorted({os.path.relpath(d, root) for d in deps})
    return mapping, result.returncode


# ---------------------------------------------------------------------------- select


def changed_files(root, base, head):
    out = sh(["git", "diff", "--name-status", "-M", base, head], cwd=root).stdout
    files = set()
    for line in out.splitlines():
        files.update(line.split("\t")[1:])
    return sorted(files)


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
    selection = {"schema": SCHEMA_SELECTION, "head": head, "base": base, "scope": scope,
                 "baseline_run": baseline.get("run_id"), "mode": "full" if full else "selected",
                 "reasons": reasons, "changed_files": [], "packages": {}, "notes": [],
                 "baseline_failures_added": []}
    if not full:
        files = changed_files(root, base, head)
        selection["changed_files"] = files
        workflow = os.path.normpath(args.workflow_file)
        globals_changed = [p for p in files if classify(p, packages)[0] == "global" or p == workflow]
        if globals_changed:
            full = True
            selection["mode"] = "full"
            selection["reasons"] = [f"global file changed: {p}" for p in globals_changed]
    if full:
        for name in scope:
            selection["packages"][name] = {"all": True, "targets": [], "lib": True, "why": selection["reasons"]}
        return finish(selection, args.out)

    why_pkg = {}
    changed_pkgs, local_pkgs, test_sources, data = set(), set(), {}, []
    for path in selection["changed_files"]:
        kind, name = classify(path, packages)
        if kind == "package":
            changed_pkgs.add(name)
            why_pkg.setdefault(name, []).append(f"source changed: {path}")
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
                elif kind == "package":
                    changed_pkgs.add(name)
                    why_pkg.setdefault(name, []).append(f"{src} reads {path}")
                elif kind == "package_local":
                    local_pkgs.add(name)
                    why_pkg.setdefault(name, []).append(f"{src} reads {path}")
    affected = dependents_closure(changed_pkgs, packages) | local_pkgs
    for name in sorted(affected):
        if name in scope:
            reasons_here = why_pkg.get(name) or ["depends on changed package(s): " + ", ".join(
                sorted(changed_pkgs & dependencies_closure(name, packages)))]
            selection["packages"][name] = {"all": True, "targets": [], "lib": True, "why": reasons_here}
    for name, files in sorted(test_sources.items()):
        if name in affected or name not in scope:
            continue
        mapping, code = test_target_sources(root, name, args.profile)
        targets, unlisted = {}, []
        for path, how in sorted(files.items()):
            listing = [t for t, deps in mapping.items() if path in deps]
            for t in listing:
                targets.setdefault(t, []).append(f"{path}: {', '.join(how)}")
            if not listing:
                unlisted.append(path)
        if unlisted or code != 0:
            for t in packages[name]["tests"]:
                targets.setdefault(t, []).append(
                    "every target: " + "; ".join([f"listed by no target: {p}" for p in unlisted]
                                                 + ([f"cargo test --no-run exited {code}"] if code else [])))
        selection["packages"][name] = {"all": False, "targets": sorted(targets), "lib": False, "why": targets}
    for item in baseline.get("failed", []):
        name, target = item.get("package"), item.get("target")
        if name not in scope or name not in packages:
            continue
        entry = selection["packages"].setdefault(name, {"all": False, "targets": [], "lib": False, "why": {}})
        if entry["all"]:
            continue
        if target == "lib":
            entry["lib"] = True
        elif target == "doc":
            entry["all"] = True
        elif target not in entry["targets"]:
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
             f"{len(selection['changed_files'])} changed files"]
    lines += [f"- {r}" for r in selection["reasons"]]
    if not selection["packages"]:
        lines.append("- no test is affected by the changes")
    for name, entry in sorted(selection["packages"].items()):
        if entry["all"]:
            lines.append(f"- `{name}`: every test")
        else:
            parts = (["lib"] if entry["lib"] else []) + entry["targets"]
            lines.append(f"- `{name}`: {', '.join(parts)}")
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


def commands(selection, mode, profile, skips):
    cmds = []
    names = sorted(selection["packages"], key=lambda n: (ORDER.index(n) if n in ORDER else len(ORDER), n))
    for name in names:
        entry = selection["packages"][name]
        if not entry["all"] and not entry["lib"] and not entry["targets"]:
            continue
        cmd = ["cargo", "test", "--locked", "--no-fail-fast"]
        if profile and profile != "dev":
            cmd += ["--profile", profile]
        cmd += ["-p", name]
        if not entry["all"]:
            if entry["lib"]:
                cmd.append("--lib")
            for target in entry["targets"]:
                cmd += ["--bin", target[4:]] if target.startswith("bin:") else ["--test", target]
        if mode == "ignored":
            cmd += ["--", "--ignored"]
            for s in skips:
                cmd += ["--skip", s]
        cmds.append(cmd)
    return cmds


def run(args):
    root = repo_root()
    with open(args.selection) as f:
        selection = json.load(f)
    outcome = {"schema": SCHEMA_OUTCOME, "head": selection["head"], "mode": args.mode,
               "selection_mode": selection["mode"], "complete": True, "commands": [], "failed": []}
    status = 0
    for cmd in commands(selection, args.mode, args.profile, args.skip):
        print("+ " + " ".join(cmd), flush=True)
        proc = subprocess.Popen(cmd, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
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
    with open(args.outcome, "w") as f:
        json.dump(outcome, f, indent=2)
        f.write("\n")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as f:
            f.write(f"### Test outcome ({args.mode}): {len(outcome['commands'])} commands, "
                    f"{len(outcome['failed'])} failing targets, complete: {outcome['complete']}\n")
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
    out = {"base": None, "run_id": None, "failed": [], "note": ""}
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
    sel = {"packages": {"vigilode": {"all": True, "targets": [], "lib": True},
                        "rodas5p-core": {"all": False, "targets": ["a", "b"], "lib": False},
                        "rodas5p-krylov": {"all": False, "targets": [], "lib": False}}}
    cmds = commands(sel, "ignored", "measurement", ["kill"])
    assert len(cmds) == 2, cmds
    assert cmds[0][:8] == ["cargo", "test", "--locked", "--no-fail-fast", "--profile", "measurement", "-p",
                           "rodas5p-core"], cmds
    assert cmds[0][8:] == ["--test", "a", "--test", "b", "--", "--ignored", "--skip", "kill"], cmds
    assert cmds[1][-5:] == ["vigilode", "--", "--ignored", "--skip", "kill"], cmds
    cmds = commands(sel, "normal", "dev", [])
    assert cmds[1] == ["cargo", "test", "--locked", "--no-fail-fast", "-p", "vigilode"], cmds
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
