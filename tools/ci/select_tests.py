#!/usr/bin/env python3
"""Change-based selection of the tests that the CI test jobs run, version 3 (owner decisions
OD-L1-19, OD-L1-20, OD-L1-21).

A test job does not re-run the whole suite at every push. It runs the test targets whose measured
inputs changed since its baseline, the targets that have no measurement yet, and the targets that
failed in the baseline run. The owner runs the whole suite before a release.

Baseline: the newest earlier run of the same job on the same branch whose outcome says that its
selection ran to the end ("complete") at a head that is an ancestor of this head. It hands over its
outcome (failed targets, runner image, pull-request base) and its input map.

Input map: every run measures, for every test target that it runs, the repository files the target
depends on and replaces the target's entry; entries of targets that did not run are carried over.
A target whose measurement fails loses its entry (rule 5 runs it next time).
  compile   files in the rustc dep-info of the target's own artifact (integration tests, lib and
            bin unit tests, examples): mod, #[path], include!, include_str!, include_bytes!
  executed  files with a covered region (-C instrument-coverage; the package's binaries included)
  read      paths the test process and its children opened, stat'ed, accessed, read as a link or
            executed successfully (strace attached through cargo's target runner)
  dirs      directories they opened as directories (their listing was read)
  absent    paths they probed that did not exist (ENOENT, ENOTDIR)
Per package the map also records the inputs of the library and binaries as other crates link them
(the dep-info of the non-test artifacts). Paths outside the repository, under the cargo target
directory, under .git or under the measurement directory are dropped. Doc tests are not measured.

Selection rules, in this order; none has a tunable number:
1. FULL when asked (workflow_dispatch with full, main), when there is no baseline or no input map,
   when the baseline head is not an ancestor of the head, when the runner image (ImageOS,
   ImageVersion) or the pull-request base sha differs from the baseline's, or when a global file
   changed: Cargo.toml, Cargo.lock, rust-toolchain*, .cargo/, tools/ci/ and the job's workflow.
2. The changed paths come from `git diff -z --name-status -M base head`; a rename is the deletion
   of its old path and the addition of its new path.
3. A changed path selects every mapped target whose compile, executed or read set contains it. An
   added or deleted path also selects every target that listed the directory whose listing changed
   (its nearest ancestor that exists on both sides), every target that probed or observed a
   directory that the change creates or removes, and (added) every target that probed the path.
4. Package sources of P: P/Cargo.toml, P/build.rs, P/src/**/*.rs and the recorded library and
   binary inputs of P. Cargo.toml, build.rs, an added, deleted or renamed .rs file and a non-Rust
   input (include_str!, include_bytes!) are structural. A modified .rs file is classified by a Rust
   tokenizer: noop (only whitespace and comments that are not doc comments changed), body (every
   changed line, old and new side, lies inside the body of a function that is not a const fn) or
   structural (anything else). Structural: every target of P and of every package that depends on
   P. Body: rule 3 and the doc tests of P and of its dependents. Noop: no package rule; rule 3
   counts the read sets and the compile sets of every target except the unit tests of P itself:
   text consumers (include_str!, include_bytes!, include!, #[path] from another crate or test
   crate) see the new bytes, the crate that compiles the file as a module sees the same tokens.
5. A target in scope without a map entry (new, or its measurement failed) runs whenever the diff is
   not empty; so does the target of a declared negative control that has no map entry. A package
   whose library and binary inputs are not recorded counts as structurally changed then.
6. Targets that failed in the baseline run (negative controls included) run again if they still
   exist and their required features are enabled; others are dropped with a note.
7. An example whose compile set contains a changed path is compiled (`cargo test --no-run
   --example`); a package that runs whole compiles all its examples.
8. Doc tests run whenever their package runs whole or has a selected target, and for body changes
   of the package or of a dependency (rule 4).
9. Only packages in the job's scope are selected; a target whose required features are not
   enabled by default is never named.

Subcommands:
  baseline  --workflow <file> --artifact <name> --branch <b> --run-id <id> [--head <sha>] --out <json>
  select    --baseline <json> --head <sha> [--pr-base <sha>] --scope <pkg ...> --workflow-file <path>
            [--full <true|false>] [--control <pkg>:<target>#<test> ...] --out <json>
  run       --selection <json> --mode normal|ignored [--profile <p>] [--skip <test> ...]
            --outcome <json> [--measure --baseline <json> --map-out <json> [--work-dir <dir>]
            [--control-maps <json> ...] [--failed-controls <file>] [--jobs <n>]]
  control   --selection <json> --target <pkg>:<target> --test <name> --signature <text>
            [--profile <p>] --map-out <json> --failed-file <file> [--work-dir <dir>]
  has       --selection <json> --target <package>:<target>   (prints yes or no)
  self-test
"""

import argparse
import concurrent.futures
import filecmp
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.parse

SCHEMA_SELECTION = "vigilode.ci.test-selection.v3"
SCHEMA_OUTCOME = "vigilode.ci.test-outcome.v2"
SCHEMA_MAP = "vigilode.ci.coverage-map.v2"
SCHEMA_CONTROL = "vigilode.ci.control-measurement.v1"
GLOBAL_FILES = ("Cargo.toml", "Cargo.lock")
GLOBAL_PREFIXES = (".cargo/", "tools/ci/", "rust-toolchain")
SETS = ("compile", "executed", "read", "dirs", "absent")
# '?' makes a name that this architecture or strace does not know non-fatal (the legacy calls do
# not exist on aarch64; openat2, faccessat2, execveat and statx are newer).
TRACED = ("openat,?open,?openat2,?creat,execve,?execveat,?stat,?lstat,newfstatat,?statx,?access,faccessat,"
          "?faccessat2,?readlink,readlinkat,chdir,fchdir")
MESSAGES = "--message-format=json-diagnostic-rendered-ansi"
RERUN = re.compile(r"`-p (\S+) (--lib|--doc|--test (\S+)|--bin (\S+))`")
ORDER = ("rodas5p-core", "rodas5p-krylov", "rodas5p-integrators", "rodas5p-fair-ab", "rodas5p-cli",
         "vigilode", "vigilode-coupled")
LIB_KINDS = ("lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro")
GIT = ["git", "-c", "core.quotePath=false", "-c", "color.ui=false"]


def sh(args, cwd=None, check=True, env=None, stdin=None):
    result = subprocess.run(args, cwd=cwd, capture_output=True, env=env, input=stdin,
                            encoding="utf-8", errors="surrogateescape")
    if check and result.returncode != 0:
        raise SystemExit(f"command failed ({result.returncode}): {' '.join(args)}\n{result.stderr}")
    return result


def git(root, *args, check=True, stdin=None):
    return sh(GIT + list(args), cwd=root, check=check, stdin=stdin)


def repo_root():
    return sh(GIT + ["rev-parse", "--show-toplevel"]).stdout.strip()


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def write_json(path, data):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=1, sort_keys=True)
        f.write("\n")


def is_ancestor(root, base, head):
    return git(root, "merge-base", "--is-ancestor", base, head, check=False).returncode == 0


# ---------------------------------------------------------------------------- workspace


def default_features(features):
    """The features that a build without --features enables (the closure of 'default')."""
    on, stack = set(), ["default"]
    while stack:
        name = stack.pop()
        if name in on or name not in features:
            continue
        on.add(name)
        stack += [x for x in features[name] if not x.startswith("dep:") and "/" not in x]
    return on


def packages_from_metadata(meta, root):
    """Workspace packages: dir, workspace dependencies, runnable targets, examples, doc tests and
    the targets left out because their required features are not enabled by default."""
    names = {p["name"] for p in meta["packages"]}
    packages = {}
    for p in meta["packages"]:
        enabled = default_features(p.get("features", {}))
        info = {"dir": os.path.relpath(os.path.dirname(p["manifest_path"]), root),
                "deps": sorted({d["name"] for d in p["dependencies"] if d["name"] in names}),
                "lib": False, "doc": False, "bins": [], "tests": [], "examples": [], "unmet": {}}
        for t in p["targets"]:
            kinds, name = t["kind"], t["name"]
            if "test" in kinds:
                key = name
            elif "example" in kinds:
                key = "example:" + name
            elif "bin" in kinds:
                key = "bin:" + name
            elif any(k in LIB_KINDS for k in kinds):
                key = "lib"
            else:
                continue  # build scripts and benches are not test targets
            missing = sorted(f for f in t.get("required-features", []) if f not in enabled)
            if missing:
                info["unmet"][key] = missing
            elif key == "lib":
                info["lib"] = bool(t.get("test", True))
                info["doc"] = bool(t.get("doctest", True))
            elif key.startswith("example:"):
                info["examples"].append(name)
            elif t.get("test", True):
                (info["bins"] if key.startswith("bin:") else info["tests"]).append(name)
        for k in ("bins", "tests", "examples"):
            info[k] = sorted(info[k])
        packages[p["name"]] = info
    return packages


def workspace(root):
    """(packages, cargo target directory)."""
    meta = json.loads(sh(["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
                         cwd=root).stdout)
    return packages_from_metadata(meta, root), meta["target_directory"]


def unit_keys(info):
    """The runnable test targets of a package: 'lib', 'bin:<name>' and integration tests."""
    return (["lib"] if info["lib"] else []) + ["bin:" + b for b in info["bins"]] + list(info["tests"])


def is_current(info, target):
    if target == "lib":
        return info["lib"]
    if target == "doc":
        return info["doc"]
    if target.startswith("bin:"):
        return target[4:] in info["bins"]
    if target.startswith("example:"):
        return target[8:] in info["examples"]
    return target in info["tests"]


def split_key(key):
    """(package, target, control test or None) of a map key '<package>::<target>[#<test>]'."""
    pkg, rest = key.split("::", 1)
    target, _, control = rest.partition("#")
    return pkg, target, control or None


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
        for dep in packages.get(stack.pop(), {}).get("deps", []):
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


def is_package_source(path, directory):
    inner = path[len(directory.rstrip("/")) + 1:]
    return inner in ("Cargo.toml", "build.rs") or (inner.startswith("src/") and inner.endswith(".rs"))


def is_global(path, workflow):
    return path in GLOBAL_FILES or path.startswith(GLOBAL_PREFIXES) or path == workflow


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
    old = git(root, "show", f"{base}:{path}", check=False)
    new = git(root, "show", f"{head}:{path}", check=False)
    if old.returncode != 0 or new.returncode != 0:
        return "structural"
    diff = git(root, "diff", "--no-ext-diff", "-U0", "--no-color", base, head, "--", path).stdout
    return classify_texts(old.stdout, new.stdout, diff)


# ---------------------------------------------------------------------------- git paths


def parse_name_status_z(data):
    """[(status letter, path)] from `git diff -z --name-status -M`. A rename gives the deletion of
    its old path (D) and the addition of its new path (A); a copy gives the addition."""
    parts = data.split("\0")
    out, i = [], 0
    while i < len(parts) and parts[i]:
        letter = parts[i][0]
        if letter in "RC":
            if letter == "R":
                out.append(("D", parts[i + 1]))
            out.append(("A", parts[i + 2]))
            i += 3
        else:
            out.append((letter, parts[i + 1]))
            i += 2
    return sorted(set(out), key=lambda e: (e[1], e[0]))


def changed_entries(root, base, head):
    return parse_name_status_z(git(root, "diff", "--no-ext-diff", "-z", "--name-status", "-M",
                                   base, head).stdout)


def ancestors(path):
    """The directories above a repository-relative path, nearest first, ending with '.'."""
    out, head = [], path
    while "/" in head:
        head = head.rsplit("/", 1)[0]
        out.append(head)
    return out + ["."]


def existing_dirs(root, commit, dirs):
    """The members of `dirs` ('.' is the root) that are directories at `commit`."""
    query = sorted(d for d in set(dirs) if "\n" not in d)
    if not query:
        return set()
    text = "".join(f"{commit}:{'' if d == '.' else d}\n" for d in query)
    out = git(root, "cat-file", "--batch-check", stdin=text, check=False).stdout.splitlines()
    return {d for d, line in zip(query, out) if line.split(" ")[1:2] == ["tree"]}


# ---------------------------------------------------------------------------- repository paths


class Repo:
    """Turns absolute or root-relative paths into repository-relative ones, dropping paths outside
    the repository, under .git, under the cargo target directory and under measurement dirs."""

    def __init__(self, root, target_dir=None, drop=()):
        self.root = os.path.abspath(root)
        self.real = os.path.realpath(root)
        self.drop = [os.path.realpath(p) for p in [target_dir, *drop] if p]
        self.drop.append(os.path.join(self.real, ".git"))

    def rel(self, path, base=None):
        p = os.path.normpath(os.path.join(base or self.root, path))
        for top in (self.root, self.real):
            if p == top:
                return "."
            if p.startswith(top + os.sep):
                inner = p[len(top) + 1:]
                full = os.path.join(self.real, inner)
                if any(full == d or full.startswith(d + os.sep) for d in self.drop):
                    return None
                return inner
        return None


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


HASHED = re.compile(r"-[0-9a-f]{16}$")


def dep_info_file(msg):
    """The rustc dep-info file of a cargo compiler-artifact message, or None."""
    for f in list(msg.get("filenames") or []) + [msg.get("executable")]:
        if not f:
            continue
        directory, base = os.path.split(f)
        stem = re.sub(r"\.(rlib|rmeta|so|dylib|a|dll|exe|wasm)$", "", base)
        if stem != base and stem.startswith("lib"):
            stem = stem[3:]
        if HASHED.search(stem) and os.path.exists(os.path.join(directory, stem + ".d")):
            return os.path.join(directory, stem + ".d")
    # A non-test binary or example is uplifted: a hard link (or copy) of the hashed artifact.
    exe = msg.get("executable")
    if not exe or not os.path.exists(exe):
        return None
    st = os.stat(exe)
    directory, base = os.path.split(exe)
    candidates = []
    for d in (directory, os.path.join(directory, "deps")):
        pattern = os.path.join(glob.escape(d), glob.escape(base.replace("-", "_")) + "-*")
        candidates += [g for g in glob.glob(pattern)
                       if HASHED.search(os.path.basename(g)) and os.path.exists(g + ".d")]
    for g in candidates:
        s = os.stat(g)
        if (s.st_ino, s.st_dev) == (st.st_ino, st.st_dev):
            return g + ".d"
    for g in candidates:
        if os.path.getsize(g) == st.st_size and filecmp.cmp(g, exe, shallow=False):
            return g + ".d"
    return None


def target_key(target):
    """'lib', 'bin:<name>', 'example:<name>' or the test name, from a cargo artifact target."""
    kinds = target["kind"]
    if "test" in kinds:
        return target["name"]
    if "example" in kinds:
        return "example:" + target["name"]
    if "bin" in kinds:
        return "bin:" + target["name"]
    return "lib"


class Artifacts:
    """Compile sets read from cargo's compiler-artifact messages."""

    def __init__(self, repo, packages):
        self.repo = repo
        self.by_dir = {os.path.normpath(info["dir"]): name for name, info in packages.items()}
        self.units = {}          # (package, key) -> {"exe": path, "compile": [...] or None}
        self.examples = {}       # (package, name) -> [...] or None
        self.inputs = {}         # package -> set of library and binary inputs
        self.inputs_failed = set()
        self.bins = {}           # package -> {non-test binary}

    def compile_set(self, msg):
        path = dep_info_file(msg)
        if not path:
            return None
        with open(path, encoding="utf-8", errors="surrogateescape") as f:
            deps = parse_dep_info(f.read())
        return sorted({r for r in (self.repo.rel(d) for d in deps) if r})

    def add(self, msg):
        if msg.get("reason") != "compiler-artifact" or not msg.get("manifest_path"):
            return
        name = self.by_dir.get(self.repo.rel(os.path.dirname(msg["manifest_path"])) or "")
        target = msg.get("target", {})
        if name is None or "custom-build" in target.get("kind", []):
            return
        key = target_key(target)
        deps = self.compile_set(msg)
        if key.startswith("example:"):
            self.examples[(name, key[8:])] = deps
        elif msg.get("profile", {}).get("test"):
            self.units[(name, key)] = {"exe": msg.get("executable"), "compile": deps}
        else:
            if deps is None:
                self.inputs_failed.add(name)
            else:
                self.inputs.setdefault(name, set()).update(deps)
            if key.startswith("bin:") and msg.get("executable"):
                self.bins.setdefault(name, set()).add(msg["executable"])


# ---------------------------------------------------------------------------- strace


TRACE_LINE = re.compile(r'^(?P<sc>[a-z0-9_]+)\((?P<args>.*)\)\s+=\s+(?P<ret>-?\d+|\?)'
                        r'(?:<(?P<rpath>(?:[^<>\\]|\\.)*)>)?(?:\s+(?P<err>E[A-Z0-9]+))?')
AT_ARGS = re.compile(r'^(?:AT_FDCWD|-?\d+)(?:<(?P<dir>(?:[^<>\\]|\\.)*)>)?,\s*"(?P<path>(?:[^"\\]|\\.)*)"')
PLAIN_ARGS = re.compile(r'^"(?P<path>(?:[^"\\]|\\.)*)"')
FD_ARG = re.compile(r'^-?\d+<(?P<dir>(?:[^<>\\]|\\.)*)>')
AT_SYSCALLS = {"openat", "openat2", "newfstatat", "statx", "faccessat", "faccessat2", "readlinkat", "execveat"}
OPEN_SYSCALLS = {"open", "openat", "openat2", "creat"}
ESCAPES = {"n": 10, "t": 9, "r": 13, "v": 11, "f": 12, "a": 7, "b": 8, "\\": 92, '"': 34, "'": 39,
           "<": 60, ">": 62}


def unescape(text):
    """A string as strace prints it (C escapes, octal and hex bytes) -> str (surrogateescape)."""
    out, i = bytearray(), 0
    while i < len(text):
        c = text[i]
        if c == "\\" and i + 1 < len(text):
            nxt = text[i + 1]
            hexed = re.match(r"x([0-9a-fA-F]{2})", text[i + 1:i + 4])
            octal = re.match(r"[0-7]{1,3}", text[i + 1:i + 4])
            if hexed:
                out.append(int(hexed.group(1), 16))
                i += 4
                continue
            if octal:
                out.append(int(octal.group(0), 8) & 0xFF)
                i += 1 + len(octal.group(0))
                continue
            if nxt in ESCAPES:
                out.append(ESCAPES[nxt])
                i += 2
                continue
        out += c.encode("utf-8", "surrogateescape")
        i += 1
    return out.decode("utf-8", "surrogateescape")


def trace_events(lines, initial_cwd):
    """(kind, absolute path) for the file-system observations in one `strace -ff -y` output file:
    'read' (opened, stat'ed, accessed, executed), 'dir' (opened as a directory), 'absent'."""
    cwd = initial_cwd
    for line in lines:
        m = TRACE_LINE.match(line)
        if not m or m.group("ret") == "?":
            continue
        sc, args, ok = m.group("sc"), m.group("args"), not m.group("ret").startswith("-")
        if sc in ("chdir", "fchdir"):
            a = (PLAIN_ARGS if sc == "chdir" else FD_ARG).match(args)
            if ok and a:
                given = unescape(a.group("path") if sc == "chdir" else a.group("dir"))
                cwd = os.path.normpath(os.path.join(cwd, given))
            continue
        if sc in AT_SYSCALLS:
            a = AT_ARGS.match(args)
            if not a:
                continue
            base = cwd if a.group("dir") is None else unescape(a.group("dir"))
            if args.startswith("AT_FDCWD") and a.group("dir") is not None:
                cwd = base
        else:
            a = PLAIN_ARGS.match(args)
            if not a:
                continue
            base = cwd
        given = unescape(a.group("path"))
        if not given:
            continue  # AT_EMPTY_PATH: the descriptor itself, already recorded when it was opened
        lexical = os.path.normpath(os.path.join(base, given))
        if not ok:
            yield ("absent" if m.group("err") in ("ENOENT", "ENOTDIR") else "read"), lexical
            continue
        if sc in OPEN_SYSCALLS:
            resolved = unescape(m.group("rpath")) if m.group("rpath") else lexical
            kind = "dir" if "O_DIRECTORY" in args or os.path.isdir(resolved) else "read"
            yield kind, lexical
            if resolved != lexical:
                yield kind, resolved
        else:
            yield "read", lexical


def trace_sets(trace_dir, repo, initial_cwd, stats=None):
    """{'read', 'dirs', 'absent'} repository paths from the strace files in trace_dir, or None.
    stats, when given, receives the number of trace files and traced system calls."""
    files = sorted(glob.glob(os.path.join(glob.escape(trace_dir), "t.*")))
    if not files:
        return None
    sets = {"read": set(), "dirs": set(), "absent": set()}
    name = {"read": "read", "dir": "dirs", "absent": "absent"}
    calls = 0
    for path in files:
        with open(path, encoding="utf-8", errors="surrogateescape") as f:
            lines = f.readlines()
        calls += sum(1 for line in lines if TRACE_LINE.match(line))
        for kind, full in trace_events(lines, initial_cwd):
            rel = repo.rel(full)
            if rel is not None:
                sets[name[kind]].add(rel)
    if stats is not None:
        stats.update(trace_files=len(files), traced_calls=calls)
    return {k: sorted(v) for k, v in sets.items()}


def strace_runner(strace, trace_dir):
    """The value of CARGO_TARGET_<host>_RUNNER that traces a test executable and its children."""
    if re.search(r"\s", strace + trace_dir):
        raise ValueError(f"cargo splits the runner at whitespace: {strace} {trace_dir}")
    return f"{strace} -ff --seccomp-bpf -qq -y -o {trace_dir}/t -e trace={TRACED}"


def strace_works(strace, work):
    """(path, version, None) when strace can trace a child here, else (None, None, reason)."""
    if not strace:
        return None, None, "strace is not installed"
    probe = os.path.join(work, "_strace_probe")
    os.makedirs(probe, exist_ok=True)
    r = sh(strace_runner(strace, probe).split() + ["true"], check=False)
    if r.returncode != 0 or not glob.glob(os.path.join(glob.escape(probe), "t.*")):
        return None, None, f"strace cannot trace here (exit {r.returncode}): {r.stderr.strip()[-300:]}"
    return strace, (sh([strace, "-V"], check=False).stdout.splitlines() or ["?"])[0], None


def host_triple():
    return re.search(r"^host: (\S+)", sh(["rustc", "-vV"]).stdout, re.M).group(1)


# ---------------------------------------------------------------------------- coverage


def llvm_bin():
    override = os.environ.get("VIGILODE_LLVM_BIN")
    if override:
        return override
    sysroot = sh(["rustc", "--print", "sysroot"]).stdout.strip()
    return os.path.join(sysroot, "lib", "rustlib", host_triple(), "bin")


def covered_files(repo, llvm, profraws, exe, objects, workdir):
    """(repository files with an executed region, None) or (None, reason)."""
    merged = os.path.join(workdir, "merged.profdata")
    r = sh([os.path.join(llvm, "llvm-profdata"), "merge", "-sparse", "-o", merged] + profraws, check=False)
    if r.returncode != 0:
        return None, "llvm-profdata failed: " + r.stderr.strip()[-300:]
    args = [os.path.join(llvm, "llvm-cov"), "export", "-summary-only", "-instr-profile", merged,
            "-ignore-filename-regex", r"(/\.cargo/registry/|/\.rustup/|^/rustc/)", exe]
    for o in objects:
        args += ["-object", o]
    r = sh(args, check=False)
    if r.returncode != 0:
        return None, "llvm-cov failed: " + r.stderr.strip()[-300:]
    files = set()
    for block in json.loads(r.stdout).get("data", []):
        for f in block.get("files", []):
            if f.get("summary", {}).get("regions", {}).get("covered", 0) > 0:
                rel = repo.rel(f["filename"])
                if rel is not None:
                    files.add(rel)
    return sorted(files), None


# ---------------------------------------------------------------------------- selection rules


def new_entry():
    return {"all": False, "lib": False, "doc": False, "targets": [], "examples": [], "why": {}}


def match_inputs(sets, status, path, cls, own_crate, created=(), listing=None):
    """Why a changed path selects a target with the given input sets ('' if it does not)."""
    why = []
    if path in sets["read"]:
        why.append(f"reads {path}")
    if path in sets["compile"] and not (cls == "noop" and own_crate):
        why.append(f"compiles {path}")
    if path in sets["executed"] and cls != "noop":
        why.append(f"executed {path}")
    if status == "A":
        probed = [d for d in [path, *created] if d in sets["absent"]]
        if probed:
            why.append(f"probed {probed[0]}, which the change adds")
    if status == "D":
        seen = [d for d in created if d in sets["read"] or d in sets["dirs"]]
        if seen:
            why.append(f"observed {seen[0]}, which the change removes")
    if status in ("A", "D") and listing is not None and listing in sets["dirs"]:
        why.append(f"lists {listing}")
    return "; ".join(why)


def decide(changes, packages, scope, cmap, pmap, classify_fn, exists_fn, failed=(), controls=()):
    """Rules 3-9 for the changes [(status, path)] since the baseline. classify_fn(path) classifies
    a modified .rs package source; exists_fn('base'|'head', dirs) returns the dirs that exist."""
    scope = [s for s in scope if s in packages]
    sel = {"packages": {}, "classified": {}, "owners": {}, "notes": [],
           "baseline_failures_added": [], "baseline_failures_dropped": []}

    def add(pkg, target, why):
        if pkg not in scope:
            return
        e = sel["packages"].setdefault(pkg, new_entry())
        if e["all"]:
            return
        if target == "lib":
            e["lib"] = True
        elif target == "doc":
            e["doc"] = True
        elif target.startswith("example:"):
            e["examples"].append(target[8:])
        else:
            e["targets"].append(target)
        e["why"].setdefault(target, []).append(why)

    inputs = {p: set(v.get("compile", [])) for p, v in pmap.items()}
    needed = set(scope).union(*[dependencies_closure(p, packages) for p in scope])
    structural, body, analysed = {}, {}, []
    for status, path in changes:
        owners = {p for p, files in inputs.items() if path in files and p in packages}
        name = package_of(path, packages)
        if name and is_package_source(path, packages[name]["dir"]):
            owners.add(name)
        cls = None
        if owners:
            manifests = {packages[p]["dir"].rstrip("/") + "/" + f for p in owners for f in ("Cargo.toml", "build.rs")}
            if path in manifests or not path.endswith(".rs") or status != "M":
                cls = "structural"
            else:
                cls = classify_fn(path)
            sel["classified"][path] = cls
            sel["owners"][path] = sorted(owners)
            for p in owners:
                if cls == "structural":
                    structural.setdefault(p, []).append(f"structural change: {path}")
                elif cls == "body":
                    body.setdefault(p, []).append(path)
        analysed.append((status, path, owners, cls))
    if changes:
        for p in sorted(needed):
            if p in packages and p not in inputs:
                structural.setdefault(p, []).append("the inputs of its library and binaries are not in the map")

    # Rule 4, structural: the package and every package that depends on it run whole.
    for p in sorted(dependents_closure(set(structural), packages)):
        if p in scope:
            reasons = structural.get(p) or ["depends on " + ", ".join(
                sorted(set(structural) & dependencies_closure(p, packages)))]
            sel["packages"][p] = dict(new_entry(), all=True, lib=packages[p]["lib"], doc=packages[p]["doc"],
                                      why={"*": reasons})

    # Rule 3 (and 7 for examples): measured inputs.
    adds = [p for s, p, _, _ in analysed if s == "A"]
    dels = [p for s, p, _, _ in analysed if s == "D"]
    at_base = exists_fn("base", {a for p in adds for a in ancestors(p)}) if adds else set()
    at_head = exists_fn("head", {a for p in dels for a in ancestors(p)}) if dels else set()
    mapped = []
    for key, item in sorted(cmap.items()):
        pkg, target, control = split_key(key)
        if pkg in scope and is_current(packages[pkg], target):
            mapped.append((pkg, target, control, {s: set(item.get(s, [])) for s in SETS}))
    for status, path, owners, cls in analysed:
        created, listing = [], None
        if status in ("A", "D"):
            exists = at_base if status == "A" else at_head
            for a in ancestors(path):
                if a == "." or a in exists:
                    listing = a
                    break
                created.append(a)
        hit = False
        for pkg, target, control, sets in mapped:
            own_crate = pkg in owners and (target == "lib" or target.startswith("bin:"))
            why = match_inputs(sets, status, path, cls, own_crate, created, listing)
            if why:
                hit = True
                add(pkg, target, f"negative control {control}: {why}" if control else why)
        if not hit and not owners:
            sel["notes"].append(f"no mapped target compiles, executes or reads {path}")

    # Rule 5: targets and negative controls without a map entry.
    if changes:
        for pkg in scope:
            info = packages[pkg]
            for t in unit_keys(info) + ["example:" + e for e in info["examples"]]:
                if f"{pkg}::{t}" not in cmap:
                    add(pkg, t, "no map entry")
        for c in controls:
            label, _, test = c.partition("#")
            pkg, _, target = label.partition(":")
            if pkg in scope and is_current(packages[pkg], target) and f"{pkg}::{target}#{test}" not in cmap:
                add(pkg, target, f"negative control {test} has no map entry")

    # Rule 4, body: doc tests of the package and of its dependents (doc tests are not measured).
    for p in sorted(dependents_closure(set(body), packages)):
        if p in scope and packages[p]["doc"]:
            files = sorted({f for q in body if q == p or q in dependencies_closure(p, packages) for f in body[q]})
            add(p, "doc", "body change: " + ", ".join(files))

    # Rule 6: baseline failures that still exist.
    for item in failed:
        pkg, target = item.get("package"), item.get("target")
        if pkg not in scope:
            continue
        label = f"{pkg}:{target}"
        info = packages.get(pkg)
        if info is None or not target or not is_current(info, target):
            unmet = info and info["unmet"].get(target)
            why = f"requires the features {', '.join(unmet)}" if unmet else "no longer a test target"
            sel["baseline_failures_dropped"].append(f"{label}: {why}")
            continue
        add(pkg, target, "failed in the baseline run" + (" (negative control)" if item.get("negative_control") else ""))
        sel["baseline_failures_added"].append(label)

    # Rule 8: doc tests run with every package that has a selected target.
    for pkg, e in sel["packages"].items():
        if not e["all"] and (e["lib"] or e["targets"]) and packages[pkg]["doc"] and not e["doc"]:
            e["doc"] = True
            e["why"].setdefault("doc", []).append("its package has a selected target (doc tests are not measured)")
    for e in sel["packages"].values():
        e["targets"] = sorted(set(e["targets"]))
        e["examples"] = sorted(set(e["examples"]))
    return sel


def full_reasons(asked, base, ancestor, map_ok, env_now, env_base):
    """Rule 1 without the global files: why the whole suite runs ([] when it does not)."""
    if asked:
        return ["the whole suite was asked for"]
    if not base:
        return ["no baseline run with a complete outcome"]
    if not ancestor:
        return [f"the baseline {base} is not an ancestor of the head"]
    if not map_ok:
        return ["the baseline run left no input map"]
    reasons = []
    for field, label in (("image_os", "runner image"), ("image_version", "runner image version"),
                         ("pr_base", "pull-request base")):
        if (env_now.get(field) or None) != (env_base.get(field) or None):
            reasons.append(f"{label} changed since the baseline: {env_base.get(field)} -> {env_now.get(field)}")
    return reasons


def load_map(path):
    if not path or not os.path.exists(path):
        return None
    data = load_json(path)
    return data if data.get("schema") == SCHEMA_MAP and isinstance(data.get("targets"), dict) else None


def select(args):
    root = repo_root()
    baseline = load_json(args.baseline)
    packages, _ = workspace(root)
    scope = list(dict.fromkeys(args.scope))
    unknown = [s for s in scope if s not in packages]
    if unknown:
        raise SystemExit("not workspace packages: " + ", ".join(unknown))
    head, base = args.head, baseline.get("base")
    env_now = {"image_os": os.environ.get("ImageOS") or None,
               "image_version": os.environ.get("ImageVersion") or None,
               "pr_base": args.pr_base or None}
    cmap = load_map(baseline.get("map_path")) if base else None
    reasons = full_reasons(str(args.full).lower() == "true", base,
                           bool(base) and is_ancestor(root, base, head), cmap is not None,
                           env_now, baseline.get("environment") or {})
    if reasons == ["no baseline run with a complete outcome"] and baseline.get("note"):
        reasons = [f"no baseline: {baseline['note']}"]
    selection = {"schema": SCHEMA_SELECTION, "head": head, "base": base, "scope": scope,
                 "baseline_run": baseline.get("run_id"), "mode": "selected", "reasons": reasons,
                 "environment": env_now, "changed": [], "packages": {}, "notes": [], "classified": {},
                 "owners": {}, "map_targets": None if cmap is None else len(cmap["targets"]),
                 "controls": list(args.control), "baseline_failures_added": [],
                 "baseline_failures_dropped": []}
    changes = []
    if not reasons:
        changes = changed_entries(root, base, head)
        workflow = os.path.normpath(args.workflow_file)
        reasons = [f"global file changed: {p}" for p in sorted({p for _, p in changes if is_global(p, workflow)})]
    selection["changed"] = [{"status": s, "path": p} for s, p in changes]
    if reasons:
        selection.update(mode="full", reasons=reasons)
        for name in scope:
            selection["packages"][name] = dict(new_entry(), all=True, lib=packages[name]["lib"],
                                               doc=packages[name]["doc"], why={"*": reasons})
        return finish(selection, args.out)
    selection.update(decide(
        changes, packages, scope, cmap["targets"], cmap.get("packages", {}),
        lambda path: classify_source_change(root, base, head, path),
        lambda side, dirs: existing_dirs(root, base if side == "base" else head, dirs),
        baseline.get("failed", []), args.control))
    return finish(selection, args.out)


def finish(selection, out):
    write_json(out, selection)
    lines = [f"### Test selection: {selection['mode']}",
             f"base `{selection['base']}`, head `{selection['head']}`, {len(selection['changed'])} changed "
             f"paths, input map: {selection.get('map_targets')} entries"]
    lines += [f"- {r}" for r in selection["reasons"]]
    if not selection["packages"]:
        lines.append("- no test is affected by the changes")
    for name, entry in sorted(selection["packages"].items()):
        if entry["all"]:
            lines.append(f"- `{name}`: every test")
            continue
        parts = (["lib"] if entry["lib"] else []) + entry["targets"] + (["doc"] if entry["doc"] else [])
        parts += [f"example {e} (compiled)" for e in entry["examples"]]
        lines.append(f"- `{name}`: {', '.join(parts) if parts else 'nothing'}")
        for target, why in sorted(entry["why"].items()):
            lines.append(f"  - {target}: {'; '.join(sorted(set(why)))[:400]}")
    lines += [f"- {path}: {cls}" for path, cls in sorted(selection.get("classified", {}).items())]
    lines += [f"- added from the baseline failures: {x}" for x in selection["baseline_failures_added"]]
    lines += [f"- dropped from the baseline failures: {x}" for x in selection["baseline_failures_dropped"]]
    lines += [f"- note: {n}" for n in selection["notes"]]
    text = "\n".join(lines) + "\n"
    print(text, flush=True)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as f:
            f.write(text)
    return 0


def selected(selection, pkg, target):
    entry = selection["packages"].get(pkg)
    if not entry:
        return False
    if entry["all"]:
        return True
    if target == "lib":
        return entry["lib"]
    if target == "doc":
        return entry["doc"]
    if target.startswith("example:"):
        return target[8:] in entry.get("examples", [])
    return target in entry["targets"]


# ---------------------------------------------------------------------------- run


def target_args(target):
    if target == "lib":
        return ["--lib"]
    if target == "doc":
        return ["--doc"]
    if target.startswith("bin:"):
        return ["--bin", target[4:]]
    if target.startswith("example:"):
        return ["--example", target[8:]]
    return ["--test", target]


def harness_args(mode, skips, exact=None):
    if mode != "ignored":
        return []
    out = ["--", "--ignored"]
    for s in skips:
        out += ["--skip", s]
    return out + (["--exact", exact] if exact else [])


def profile_args(profile):
    return ["--profile", profile] if profile and profile != "dev" else []


def ordered(names):
    return sorted(names, key=lambda n: (ORDER.index(n) if n in ORDER else len(ORDER), n))


def units(selection, packages):
    """(package, target) for every unit that runs; 'doc' for doc tests. Examples are only built."""
    out = []
    for name in ordered(selection["packages"]):
        entry, info = selection["packages"][name], packages.get(name)
        if info is None:
            continue
        if entry["all"]:
            out += [(name, t) for t in unit_keys(info)] + ([(name, "doc")] if info["doc"] else [])
            continue
        out += ([(name, "lib")] if entry["lib"] and info["lib"] else [])
        out += [(name, t) for t in entry["targets"] if is_current(info, t)]
        if entry["doc"] and info["doc"]:
            out.append((name, "doc"))
    return out


def build_filters(entry, info):
    """cargo test --no-run filters for one package ([] builds every target, None nothing)."""
    if entry["all"]:
        return []
    filters = (["--lib"] if entry["lib"] and info["lib"] else [])
    filters += [a for t in entry["targets"] if is_current(info, t) for a in target_args(t)]
    filters += [a for e in entry.get("examples", []) if e in info["examples"] for a in ("--example", e)]
    return filters or None


def commands(selection, packages, mode, profile, skips):
    """One cargo test per package (the unmeasured mode)."""
    cmds = []
    for name in ordered(selection["packages"]):
        entry, info = selection["packages"][name], packages.get(name)
        if info is None:
            continue
        base = ["cargo", "test", "--locked", "--no-fail-fast"] + profile_args(profile) + ["-p", name]
        if entry["all"]:
            cmds.append(base + harness_args(mode, skips))
            continue
        filters = build_filters(dict(entry, examples=[]), info)
        if filters:
            cmds.append(base + filters + harness_args(mode, skips))
        if entry.get("examples"):
            cmds.append(["cargo", "test", "--locked", "--no-run"] + profile_args(profile) + ["-p", name]
                        + [a for e in entry["examples"] if e in info["examples"] for a in ("--example", e)])
        if entry.get("doc") and info["doc"]:
            cmds.append(base + ["--doc"] + harness_args(mode, skips))
    return cmds


def stream(cmd, cwd, env, artifacts=None):
    """Run a command, echo its output line by line (cargo JSON messages are consumed: artifacts are
    recorded, diagnostics printed rendered) and return (exit code, text output)."""
    print("+ " + " ".join(cmd), flush=True)
    proc = subprocess.Popen(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=env,
                            encoding="utf-8", errors="surrogateescape")
    lines = []
    for line in proc.stdout:
        if line.startswith('{"reason":'):
            try:
                msg = json.loads(line)
            except ValueError:
                msg = None
            if isinstance(msg, dict):
                if msg.get("reason") == "compiler-artifact" and artifacts is not None:
                    artifacts.add(msg)
                elif msg.get("reason") == "compiler-message":
                    rendered = (msg.get("message") or {}).get("rendered") or ""
                    sys.stdout.write(rendered)
                    sys.stdout.flush()
                    lines.append(rendered)
                continue
        sys.stdout.write(line)
        sys.stdout.flush()
        lines.append(line)
    return proc.wait(), "".join(lines)


def failed_controls(path):
    """The failed negative controls that the control steps wrote ('<pkg>:<target>' per line) as
    outcome.failed entries; none when the file does not exist."""
    if not path or not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as f:
        lines = sorted({x.strip() for x in f if ":" in x})
    return [dict(zip(("package", "target"), line.split(":", 1)), negative_control=True) for line in lines]


def failed_targets(text):
    failed = set()
    for m in RERUN.finditer(text):
        target = "lib" if m.group(2) == "--lib" else "doc" if m.group(2) == "--doc" else (
            m.group(3) or "bin:" + m.group(4))
        failed.add((m.group(1), target))
    return failed


def measure_unit(repo, llvm, strace_ok, unit, bins, prof_dir, trace_dir, initial_cwd, workdir, stats=None):
    """(entry without head, None) or (None, reasons) for one test unit that ran; stats, when given,
    receives the trace statistics."""
    reasons = []
    if unit is None or unit.get("compile") is None:
        reasons.append("no dep-info of its artifact")
    profraws = sorted(glob.glob(os.path.join(glob.escape(prof_dir), "*.profraw")))
    executed = None
    if not profraws:
        reasons.append("no coverage profile")
    elif not unit or not unit.get("exe"):
        reasons.append("no test executable")
    else:
        executed, err = covered_files(repo, llvm, profraws, unit["exe"], sorted(bins), workdir)
        if executed is None:
            reasons.append(err)
    traced = trace_sets(trace_dir, repo, initial_cwd, stats) if strace_ok else None
    if traced is None:
        reasons.append("no strace output" if strace_ok else "strace unavailable")
    if reasons:
        return None, reasons
    return dict({"compile": unit["compile"], "executed": executed}, **traced), None


def run(args):
    root = repo_root()
    selection = load_json(args.selection)
    packages, target_dir = workspace(root)
    outcome = {"schema": SCHEMA_OUTCOME, "head": selection["head"], "mode": args.mode,
               "selection_mode": selection["mode"], "complete": True, "commands": [], "failed": [],
               "negative_controls_failed": [], "measurement": None,
               "environment": selection.get("environment", {})}
    status = 0
    work = os.path.abspath(args.work_dir or tempfile.mkdtemp(prefix="ci-measure-"))
    os.makedirs(work, exist_ok=True)
    repo = Repo(root, target_dir, drop=[work])
    arts = Artifacts(repo, packages)

    def execute(cmd, env=None, build=False):
        nonlocal status
        code, text = stream(cmd, root, env, arts)
        failed = failed_targets(text)
        broken = code != 0 and not failed
        outcome["commands"].append({"command": " ".join(cmd), "exit_code": code,
                                    "failed_targets": sorted(f"{p}:{t}" for p, t in failed),
                                    "did_not_run_its_tests": broken, "build": build})
        outcome["failed"] += [{"package": p, "target": t} for p, t in sorted(failed)]
        if broken:
            outcome["complete"] = False
        if code != 0:
            status = 1
        return code, not broken

    for item in failed_controls(args.failed_controls):
        outcome["failed"].append(item)
        outcome["negative_controls_failed"].append(f"{item['package']}:{item['target']}")

    if not args.measure:
        for cmd in commands(selection, packages, args.mode, args.profile, args.skip):
            execute(cmd)
    else:
        base_targets, base_packages = {}, {}
        if selection["mode"] != "full" and args.baseline:
            previous = load_map(load_json(args.baseline).get("map_path"))
            if previous:
                base_targets, base_packages = previous["targets"], previous.get("packages", {})
        llvm = llvm_bin()
        strace, strace_version, strace_note = strace_works(shutil.which(args.strace), work)
        strace_ok = strace is not None
        clean_env = {k: v for k, v in os.environ.items()
                     if not (k.startswith("CARGO_TARGET_") and k.endswith("_RUNNER"))}
        build_env = dict(clean_env, LLVM_PROFILE_FILE=os.path.join(work, "_unused", "%p-%m.profraw"))
        runner_var = "CARGO_TARGET_" + host_triple().upper().replace("-", "_") + "_RUNNER"
        prof = profile_args(args.profile)
        for name in ordered(selection["packages"]):
            if name not in packages:
                continue
            filters = build_filters(selection["packages"][name], packages[name])
            if filters is not None:
                execute(["cargo", "test", "--locked", "--no-run", MESSAGES] + prof + ["-p", name] + filters,
                        env=build_env, build=True)
        jobs = []
        for name, target in units(selection, packages):
            cmd = (["cargo", "test", "--locked", "--no-fail-fast", MESSAGES] + prof + ["-p", name]
                   + target_args(target) + harness_args(args.mode, args.skip))
            if target == "doc":
                execute(cmd, env=build_env)
                continue
            key = f"{name}::{target}"
            workdir = os.path.join(work, re.sub(r"[^A-Za-z0-9_.-]", "_", key))
            shutil.rmtree(workdir, ignore_errors=True)
            prof_dir, trace_dir = os.path.join(workdir, "profiles"), os.path.join(workdir, "trace")
            os.makedirs(prof_dir)
            os.makedirs(trace_dir)
            env = dict(clean_env, LLVM_PROFILE_FILE=os.path.join(prof_dir, "%p-%m.profraw"))
            if strace_ok:
                env[runner_var] = strace_runner(strace, trace_dir)
            _, ran = execute(cmd, env=env)
            jobs.append((key, name, target, workdir, prof_dir, trace_dir, ran))

        stats = {}

        def measure(job):
            key, name, target, workdir, prof_dir, trace_dir, ran = job
            stats[key] = {}
            try:
                if not ran:
                    return key, None, ["its command did not run its tests"]
                item, why = measure_unit(repo, llvm, strace_ok, arts.units.get((name, target)),
                                         arts.bins.get(name, set()), prof_dir, trace_dir,
                                         os.path.join(repo.real, packages[name]["dir"]), workdir, stats[key])
                return key, item, why
            except Exception as exc:  # a measurement error must not lose the outcome
                return key, None, [f"measurement error: {exc!r}"]
            finally:
                shutil.rmtree(workdir, ignore_errors=True)

        measured, lost = {}, []
        workers = args.jobs or len(os.sched_getaffinity(0))
        with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, workers)) as pool:
            for key, item, why in pool.map(measure, jobs):
                if item is None:
                    lost.append({"target": key, "why": why})
                else:
                    measured[key] = item
                    stats[key].update({s: len(item[s]) for s in SETS})
        for (name, example), deps in sorted(arts.examples.items()):
            key = f"{name}::example:{example}"
            if deps is None:
                lost.append({"target": key, "why": ["no dep-info of its artifact"]})
            else:
                measured[key] = {"compile": deps}
        for path in args.control_maps:
            if not os.path.exists(path):
                continue
            fragment = load_json(path)
            if fragment.get("schema") != SCHEMA_CONTROL:
                continue
            if fragment.get("entry") is None:
                lost.append({"target": fragment["key"], "why": fragment.get("why") or ["not measured"]})
            else:
                measured[fragment["key"]] = fragment["entry"]
        recorded = {n: sorted(files) for n, files in arts.inputs.items() if n not in arts.inputs_failed}
        cmap, pmap = update_map(base_targets, base_packages, measured, [x["target"] for x in lost],
                                recorded, sorted(arts.inputs_failed), packages, selection["scope"],
                                selection["head"])
        write_json(args.map_out, {"schema": SCHEMA_MAP, "head": selection["head"], "mode": args.mode,
                                  "targets": cmap, "packages": pmap})
        outcome["measurement"] = {
            "refreshed": len(measured), "lost": lost, "map_targets": len(cmap),
            "packages_recorded": sorted(recorded), "packages_lost": sorted(arts.inputs_failed),
            "llvm_bin": llvm, "strace": strace_version, "strace_note": strace_note, "work_dir": work,
            "stats": stats}
    write_json(args.outcome, outcome)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as f:
            f.write(f"### Test outcome ({args.mode}): {len(outcome['commands'])} commands, "
                    f"{len(outcome['failed'])} failing targets, complete: {outcome['complete']}\n")
            m = outcome["measurement"]
            if m:
                f.write(f"- input map: {m['refreshed']} entries measured, {len(m['lost'])} lost, "
                        f"{m['map_targets']} in all; strace: {m['strace'] or m['strace_note']}\n")
                f.writelines(f"- lost `{x['target']}`: {'; '.join(x['why'])}\n" for x in m["lost"])
            f.writelines(f"- failed `{x['package']}` `{x['target']}`\n" for x in outcome["failed"])
    return status


def update_map(base_targets, base_packages, measured, lost, recorded, packages_lost, packages, scope, head):
    """The next input map: measured entries replace, lost entries go, the rest carries over; entries
    of targets that are no longer current or not in scope are pruned."""
    targets = dict(base_targets)
    for key in lost:
        targets.pop(key, None)
    for key, item in measured.items():
        targets[key] = dict(item, head=head)
    keep = {}
    for key, item in targets.items():
        pkg, target, _ = split_key(key)
        if pkg in scope and pkg in packages and is_current(packages[pkg], target):
            keep[key] = item
    # Library inputs are united with the carried-over entry: a run may build a library only under
    # another package's feature unification, which must not drop an input; a full run starts empty.
    pkgs = {k: v for k, v in base_packages.items() if k in packages and k not in packages_lost}
    for name, files in recorded.items():
        pkgs[name] = {"compile": sorted(set(pkgs.get(name, {}).get("compile", [])) | set(files)), "head": head}
    return keep, pkgs


# ---------------------------------------------------------------------------- negative controls


def control(args):
    """Run one negative control (a test that fails by design) with measurement, when its target is
    selected. The step passes only while the test fails with its recorded signature."""
    root = repo_root()
    selection = load_json(args.selection)
    pkg, target = args.target.split(":", 1)
    key = f"{pkg}::{target}#{args.test}"
    if not selected(selection, pkg, target):
        print("not affected by the changes since the baseline: not run", flush=True)
        return 0
    packages, target_dir = workspace(root)
    work = os.path.abspath(args.work_dir or tempfile.mkdtemp(prefix="ci-control-"))
    os.makedirs(work, exist_ok=True)
    repo = Repo(root, target_dir, drop=[work])
    arts = Artifacts(repo, packages)
    prof = profile_args(args.profile)
    clean_env = {k: v for k, v in os.environ.items()
                 if not (k.startswith("CARGO_TARGET_") and k.endswith("_RUNNER"))}
    build_env = dict(clean_env, LLVM_PROFILE_FILE=os.path.join(work, "_unused", "%p-%m.profraw"))
    strace, strace_version, strace_note = strace_works(shutil.which(args.strace), work)
    print(f"strace: {strace_version or strace_note}", flush=True)
    prof_dir, trace_dir = os.path.join(work, "profiles"), os.path.join(work, "trace")
    os.makedirs(prof_dir, exist_ok=True)
    os.makedirs(trace_dir, exist_ok=True)
    env = dict(clean_env, LLVM_PROFILE_FILE=os.path.join(prof_dir, "%p-%m.profraw"))
    if strace:
        env["CARGO_TARGET_" + host_triple().upper().replace("-", "_") + "_RUNNER"] = strace_runner(strace, trace_dir)
    build_code, _ = stream(["cargo", "test", "--locked", "--no-run", MESSAGES] + prof + ["-p", pkg, "--test", target],
                           root, build_env, arts)
    code, text = stream(["cargo", "test", "--locked", MESSAGES] + prof + ["-p", pkg, "--test", target]
                        + harness_args("ignored", [], exact=args.test), root, env, arts)
    verdict = None
    if build_code != 0:
        verdict = "the target did not build"
    elif code == 0:
        verdict = "the test passed: the negative control no longer fails"
    elif args.signature not in text:
        verdict = "the test failed without its recorded signature"
    item, why = None, None
    if build_code == 0:
        try:
            item, why = measure_unit(repo, llvm_bin(), strace is not None, arts.units.get((pkg, target)),
                                     arts.bins.get(pkg, set()), prof_dir, trace_dir,
                                     os.path.join(repo.real, packages[pkg]["dir"]), work)
        except Exception as exc:
            why = [f"measurement error: {exc!r}"]
    else:
        why = ["the target did not build"]
    if why and strace is None:
        why.append(strace_note)
    write_json(args.map_out, {"schema": SCHEMA_CONTROL, "head": selection["head"], "key": key,
                              "entry": item, "why": why, "verdict": verdict or "fails with its recorded signature"})
    if verdict:
        with open(args.failed_file, "a", encoding="utf-8") as f:
            f.write(f"{pkg}:{target}\n")
        print(verdict, flush=True)
        return 1
    print(f"{args.test} fails with its recorded signature", flush=True)
    return 0


def has(args):
    selection = load_json(args.selection)
    package, target = args.target.split(":", 1)
    print("yes" if selected(selection, package, target) else "no")
    return 0


# ---------------------------------------------------------------------------- baseline


def baseline(args):
    out = {"base": None, "run_id": None, "failed": [], "map_path": None, "environment": {}, "note": "",
           "errors": []}
    repository = os.environ.get("GITHUB_REPOSITORY")
    if not repository or not shutil.which("gh"):
        out["note"] = "no GitHub repository or gh in this environment"
    else:
        root = repo_root()
        query = urllib.parse.urlencode({"branch": args.branch, "per_page": 100})
        listing = sh(["gh", "api", f"repos/{repository}/actions/workflows/{args.workflow}/runs?{query}"],
                     check=False)
        if listing.returncode != 0:
            print(listing.stderr, file=sys.stderr, flush=True)
            out["note"] = f"the run listing failed (exit {listing.returncode}): {listing.stderr.strip()[-400:]}"
        else:
            current = int(args.run_id) if str(args.run_id).isdigit() else None
            runs = [r for r in json.loads(listing.stdout).get("workflow_runs", [])
                    if str(r["id"]) != str(args.run_id) and (current is None or r["id"] < current)]
            runs.sort(key=lambda r: (r["created_at"], r["id"]), reverse=True)
            counts = {"without the artifact": 0, "not complete": 0, "not an ancestor": 0}
            for r in runs:
                tmp = tempfile.mkdtemp(prefix="ci-baseline-")
                got = sh(["gh", "run", "download", str(r["id"]), "-R", repository, "-n", args.artifact,
                          "-D", tmp], check=False)
                path = os.path.join(tmp, "outcome.json")
                if got.returncode != 0 or not os.path.exists(path):
                    if got.returncode == 0 or "no valid artifacts found" in got.stderr:
                        counts["without the artifact"] += 1
                    else:
                        out["errors"].append(f"download of run {r['id']} failed (exit {got.returncode}): "
                                             f"{got.stderr.strip()[-300:]}")
                    continue
                outcome = load_json(path)
                if outcome.get("schema") != SCHEMA_OUTCOME or not outcome.get("complete"):
                    counts["not complete"] += 1
                    continue
                if args.head and not is_ancestor(root, outcome["head"], args.head):
                    counts["not an ancestor"] += 1
                    continue
                map_path = os.path.join(tmp, "map.json")
                if os.path.exists(map_path):
                    keep = os.path.join(os.path.dirname(os.path.abspath(args.out)), "baseline_map.json")
                    shutil.copyfile(map_path, keep)
                    out["map_path"] = keep
                out.update(base=outcome["head"], run_id=r["id"], failed=outcome.get("failed", []),
                           environment=outcome.get("environment") or {},
                           note=f"run {r['id']} ran its selection to the end at {outcome['head'][:8]}")
                break
            else:
                out["note"] = "no earlier run of this job on this branch has a complete outcome"
            skipped = ", ".join(f"{v} {k}" for k, v in counts.items() if v)
            if skipped:
                out["note"] += f" (newer runs skipped: {skipped})" if out["base"] else f" ({skipped})"
            if out["errors"]:
                print("\n".join(out["errors"]), file=sys.stderr, flush=True)
                out["note"] += f"; {len(out['errors'])} artifact downloads failed"
    write_json(args.out, out)
    print(json.dumps(out, indent=2), flush=True)
    return 0


# ---------------------------------------------------------------------------- self-test


def check_workflows(root):
    """Text checks of the CI mechanics that the review asked for (F-01, F-03, F-05, F-06, F-07,
    F-11, F-12, F8/F-04, F14). Returns the list of problems."""
    problems = []
    wf = os.path.join(root, ".github", "workflows")

    def read(name):
        with open(os.path.join(wf, name), encoding="utf-8") as f:
            return f.read()

    def job_block(text, job):
        m = re.search(rf"^  {re.escape(job)}:\n(.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)", text, re.M | re.S)
        return m.group(1) if m else ""

    prod, ign, a1 = read("production.yml"), read("ignored-tests.yml"), read("a1-inner-tolerance-parity.yml")
    if re.search(r"^concurrency:", prod, re.M):
        problems.append("production.yml: a workflow-level concurrency group cancels the test job")
    for text, name, job in ((prod, "production.yml", "test"), (ign, "ignored-tests.yml", "ignored-tests")):
        block = job_block(text, job)
        if not re.search(r"concurrency:\s*\n\s+group: .*\n\s+cancel-in-progress: false", block):
            problems.append(f"{name}: job {job} does not queue (cancel-in-progress false)")
        for line in block.splitlines():
            code = line.split("#", 1)[0] if line.lstrip().startswith("#") else line
            if "tools/ci/select_tests.py" in code and "python3 -u tools/ci/select_tests.py" not in code:
                problems.append(f"{name}: selector step without python3 -u: {line.strip()}")
            if "${{ github.head_ref" in code and not re.match(r"\s+BRANCH: ", code):
                problems.append(f"{name}: github.head_ref expanded outside the env block: {line.strip()}")
        if "overwrite: true" not in block:
            problems.append(f"{name}: upload-artifact without overwrite")
        if "--pr-base" not in block or "pull_request.base.sha" not in block:
            problems.append(f"{name}: the pull-request base is not recorded")
        if "strace" not in block:
            problems.append(f"{name}: strace is not installed")
        if re.search(r"^\s+schedule:", text, re.M):
            problems.append(f"{name}: has a schedule (owner decision OD-L1-21: no periodic full run)")
    for job in ("fmt", "clippy", "docs", "semver", "bench"):
        if "cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}" not in job_block(prod, job):
            problems.append(f"production.yml: job {job} does not cancel on non-main refs")
    for text, name, job, minutes in ((prod, "production.yml", "test", 300), (ign, "ignored-tests.yml", "ignored-tests", 330)):
        if f"timeout-minutes: {minutes}\n" not in job_block(text, job):
            problems.append(f"{name}: job {job} has not the time limit of {minutes} minutes (F-10)")
    if "select_tests.py has" in ign and "|| exit 1" not in ign:
        problems.append("ignored-tests.yml: a failing has call does not fail its step")
    if ign.count("select_tests.py control") != 2 or "--failed-controls" not in ign:
        problems.append("ignored-tests.yml: negative controls are not measured or not merged into the outcome")
    if "Run the workspace test closure" in a1 or "cargo test --workspace --all-targets --locked\n" in a1:
        problems.append("a1-inner-tolerance-parity.yml: still runs the workspace test closure")
    return problems


def self_test():
    # Rust tokens and change classification (OD-L1-20)
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
            with open(pa, "w") as f:
                f.write(a)
            with open(pb, "w") as f:
                f.write(b)
            return sh(GIT + ["diff", "--no-index", "--no-ext-diff", "-U0", "--no-color", pa, pb], check=False).stdout
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

    # dep-info: first rule, escaped spaces, entries resolved against the root, not the cwd (F13)
    deps = parse_dep_info("/t/deps/a-1.d: /r/tests/a.rs crates/c/tests/../../../research/x/w.rs /r/tests/my\\ file.rs\n\n"
                          "/t/deps/a-1: /r/tests/a.rs\n")
    assert deps == ["/r/tests/a.rs", "crates/c/tests/../../../research/x/w.rs", "/r/tests/my file.rs"], deps
    with tempfile.TemporaryDirectory() as d:
        r = Repo(d, os.path.join(d, "target"), drop=[os.path.join(d, "measure")])
        here = os.getcwd()
        try:
            os.chdir(tempfile.gettempdir())
            assert r.rel(deps[1]) == "research/x/w.rs", r.rel(deps[1])
        finally:
            os.chdir(here)
        assert r.rel(os.path.join(d, "crates/c/src/lib.rs")) == "crates/c/src/lib.rs"
        assert r.rel(os.path.join(d, "target/debug/build/x/out/gen.rs")) is None
        assert r.rel(os.path.join(d, ".git/index")) is None
        assert r.rel(os.path.join(d, "measure/u/trace/t.1")) is None
        assert r.rel("/usr/lib/libc.so.6") is None
        assert r.rel(d) == "."

    # git diff -z: quotes, tabs and non-ASCII names come back verbatim; a rename gives both sides (F13)
    z = "M\0a \"q\".rs\0R087\0old\tname.rs\0new name.rs\0D\0dir/é.json\0A\0z.txt\0"
    assert parse_name_status_z(z) == [("M", 'a "q".rs'), ("D", "dir/é.json"), ("A", "new name.rs"),
                                      ("D", "old\tname.rs"), ("A", "z.txt")], parse_name_status_z(z)
    with tempfile.TemporaryDirectory() as d:
        ident = ["-c", "user.name=ci", "-c", "user.email=ci@localhost", "-c", "commit.gpgsign=false"]
        sh(GIT + ["init", "-q", d])
        names = ['quote"d.rs', "tab\there.json", "é/ü.txt", "keep.rs"]
        for n in names:
            os.makedirs(os.path.join(d, os.path.dirname(n)), exist_ok=True)
            with open(os.path.join(d, n), "w") as f:
                f.write(f"{n}\n" * 20)
        sh(GIT + ["-C", d, "add", "--"] + names)
        sh(GIT + ident + ["-C", d, "commit", "-q", "-m", "a"])
        base = sh(GIT + ["-C", d, "rev-parse", "HEAD"]).stdout.strip()
        with open(os.path.join(d, 'quote"d.rs'), "a") as f:
            f.write("more\n")
        sh(GIT + ["-C", d, "mv", "é/ü.txt", "é/ö.txt"])
        sh(GIT + ["-C", d, "rm", "-q", "tab\there.json"])
        sh(GIT + ident + ["-C", d, "commit", "-q", "-a", "-m", "b"])
        head = sh(GIT + ["-C", d, "rev-parse", "HEAD"]).stdout.strip()
        got = changed_entries(d, base, head)
        assert got == [("M", 'quote"d.rs'), ("D", "tab\there.json"), ("A", "é/ö.txt"), ("D", "é/ü.txt")], got
        assert existing_dirs(d, base, {".", "é", "nope"}) == {".", "é"}

    # strace -ff -y lines: resolved descriptors, relative probes, directories, chdir, escapes
    lines = [
        'openat(AT_FDCWD</r/crates/i>, "../../docs/T.toml", O_RDONLY|O_CLOEXEC) = 3</r/docs/T.toml>',
        'openat(AT_FDCWD</r/crates/i>, "../../research/gone.json", O_RDONLY|O_CLOEXEC) = -1 ENOENT (No such file or directory)',
        'openat(AT_FDCWD</r/crates/i>, "/r/crates", O_RDONLY|O_NONBLOCK|O_CLOEXEC|O_DIRECTORY) = 4</r/crates>',
        'statx(AT_FDCWD</r/crates/i>, "/r/crates/x/src", AT_STATX_SYNC_AS_STAT, STATX_ALL, {stx_mask=STATX_ALL}) = 0',
        'newfstatat(AT_FDCWD</r/crates/i>, "/r/crates/y/src", 0x7ffe, 0) = -1 ENOENT (No such file or directory)',
        'statx(3</r/docs/T.toml>, "", AT_STATX_SYNC_AS_STAT|AT_EMPTY_PATH, STATX_ALL, {stx_mask=STATX_ALL}) = 0',
        'chdir("/r/research")                    = 0',
        'access("q \\"x\\"/\\303\\251.py", R_OK) = 0',
        'execve("/usr/bin/python3", ["python3", "s.py"], 0x7ffd /* 9 vars */) = 0',
        'openat(5</r/research/d>, "f.json", O_RDONLY) = 6</r/research/d/f.json>',
        'readlink("/proc/self/exe", "/t/debug/deps/x-1", 4096) = 17',
        '+++ exited with 0 +++',
    ]
    events = list(trace_events(lines, "/r/crates/i"))
    assert ("read", "/r/docs/T.toml") in events, events
    assert ("absent", "/r/research/gone.json") in events, events
    assert ("dir", "/r/crates") in events, events
    assert ("read", "/r/crates/x/src") in events, events
    assert ("absent", "/r/crates/y/src") in events, events
    assert ("read", '/r/research/q "x"/é.py') in events, events
    assert ("read", "/r/research/d/f.json") in events, events
    assert not any(p == "/r/docs/T.toml" and k != "read" for k, p in events), events
    assert len(events) == 9, events
    assert unescape(r"a\x41\101\n\"") == 'aAA\n"', unescape(r"a\x41\101\n\"")
    with tempfile.TemporaryDirectory() as d:
        assert strace_works(None, d) == (None, None, "strace is not installed")
        found = shutil.which("strace")
        if found:
            path, version, note = strace_works(found, d)
            assert (path, note) == (found, None) and version.startswith("strace") or (path, version) == (None, None), (
                path, version, note)  # a sandbox may refuse ptrace: then no runner, and every entry is lost
    assert strace_runner("/usr/bin/strace", "/w/t").startswith("/usr/bin/strace -ff --seccomp-bpf -qq -y -o /w/t/t -e trace=")
    try:
        strace_runner("/usr/bin/strace", "/w w/t")
        raise AssertionError("whitespace in the runner was accepted")
    except ValueError:
        pass

    # required features never named; default-feature closure (F10, F-02)
    meta = {"packages": [
        {"name": "core", "manifest_path": "/r/crates/core/Cargo.toml", "dependencies": [], "features": {},
         "targets": [{"name": "core", "kind": ["lib"], "test": True, "doctest": True},
                     {"name": "c1", "kind": ["test"], "test": True}]},
        {"name": "int", "manifest_path": "/r/crates/int/Cargo.toml",
         "dependencies": [{"name": "core"}, {"name": "sha2"}],
         "features": {"default": ["base"], "base": [], "audit": ["dep:x"]},
         "targets": [{"name": "int", "kind": ["lib"], "test": True, "doctest": True},
                     {"name": "build-script-build", "kind": ["custom-build"], "test": False},
                     {"name": "t_ok", "kind": ["test"], "test": True},
                     {"name": "t_base", "kind": ["test"], "test": True, "required-features": ["base"]},
                     {"name": "audit2_x", "kind": ["test"], "test": True, "required-features": ["audit"]},
                     {"name": "ex", "kind": ["example"], "test": False},
                     {"name": "ex_audit", "kind": ["example"], "test": False, "required-features": ["audit"]}]},
        {"name": "cli", "manifest_path": "/r/crates/cli/Cargo.toml",
         "dependencies": [{"name": "int"}, {"name": "core"}], "features": {},
         "targets": [{"name": "cli", "kind": ["bin"], "test": True},
                     {"name": "alg02", "kind": ["test"], "test": True}]},
        {"name": "vig", "manifest_path": "/r/crates/vig/Cargo.toml", "dependencies": [{"name": "core"}],
         "features": {}, "targets": [{"name": "vig", "kind": ["lib"], "test": True, "doctest": True}]},
    ]}
    pk = packages_from_metadata(meta, "/r")
    assert pk["int"]["tests"] == ["t_base", "t_ok"] and pk["int"]["unmet"] == {
        "audit2_x": ["audit"], "example:ex_audit": ["audit"]}, pk["int"]
    assert pk["int"]["examples"] == ["ex"] and pk["int"]["deps"] == ["core"], pk["int"]
    assert pk["cli"]["bins"] == ["cli"] and not pk["cli"]["lib"] and not pk["cli"]["doc"], pk["cli"]
    assert unit_keys(pk["int"]) == ["lib", "t_base", "t_ok"]
    assert dependents_closure({"core"}, pk) == {"core", "int", "cli", "vig"}
    assert dependencies_closure("cli", pk) == {"int", "core"}
    sel_all = {"packages": {"int": dict(new_entry(), all=True), "cli": dict(new_entry(), all=True)}}
    plan = units(sel_all, pk)
    assert ("int", "audit2_x") not in plan and ("cli", "doc") not in plan and ("int", "doc") in plan, plan
    assert build_filters(dict(new_entry(), targets=["t_ok", "audit2_x"], examples=["ex", "ex_audit"]), pk["int"]) == [
        "--test", "t_ok", "--example", "ex"]
    assert build_filters(dict(new_entry(), doc=True), pk["int"]) is None
    cmds = commands({"packages": {"cli": dict(new_entry(), doc=True, targets=["alg02"]),
                                  "core": dict(new_entry(), targets=["c1"], doc=True)}},
                    pk, "ignored", "measurement", ["kill"])
    assert cmds == [["cargo", "test", "--locked", "--no-fail-fast", "--profile", "measurement", "-p", "cli",
                     "--test", "alg02", "--", "--ignored", "--skip", "kill"],
                    ["cargo", "test", "--locked", "--no-fail-fast", "--profile", "measurement", "-p", "core",
                     "--test", "c1", "--", "--ignored", "--skip", "kill"],
                    ["cargo", "test", "--locked", "--no-fail-fast", "--profile", "measurement", "-p", "core",
                     "--doc", "--", "--ignored", "--skip", "kill"]], cmds
    assert is_global("Cargo.lock", "w.yml") and is_global("rust-toolchain.toml", "w.yml")
    assert is_global(".cargo/config.toml", "w.yml") and is_global("tools/ci/select_tests.py", "w.yml")
    assert is_global(".github/workflows/p.yml", ".github/workflows/p.yml")
    assert not is_global("crates/int/Cargo.toml", "w.yml") and not is_global("tools/x.py", "w.yml")

    # selection rules 3-8 on a synthetic input map (findings F1-F7, F10, F11)
    pk["core"]["dir"], pk["int"]["dir"], pk["cli"]["dir"], pk["vig"]["dir"] = (
        "crates/core", "crates/int", "crates/cli", "crates/vig")

    def entry(**sets):
        return {s: sorted(sets.get(s, [])) for s in SETS}
    cmap = {
        "core::lib": entry(compile=["crates/core/src/lib.rs", "crates/core/src/a.rs"],
                           executed=["crates/core/src/lib.rs"]),
        "core::c1": entry(compile=["crates/core/tests/c1.rs", "research/r/gain_witness.rs"],
                          executed=["crates/core/src/a.rs", "research/r/gain_witness.rs"]),
        "int::lib": entry(compile=["crates/int/src/lib.rs", "crates/int/tests/support/a02.rs"],
                          executed=["crates/int/src/lib.rs"]),
        "int::t_ok": entry(compile=["crates/int/tests/t_ok.rs", "docs/TABLE.toml", "crates/cli/src/main.rs"],
                           executed=["crates/int/src/lib.rs", "crates/core/src/lib.rs"],
                           read=["research/z/FROZEN.json", "crates/int/tests/u.rs", "crates/vig/src/lib.rs"],
                           dirs=["crates", "crates/core/src", "crates/hyb/src"],
                           absent=["research/new/probe.json"]),
        "int::t_base": entry(compile=["crates/int/tests/t_base.rs"], executed=["crates/int/src/lib.rs"]),
        "int::example:ex": entry(compile=["crates/int/examples/ex.rs", "crates/int/tests/common/mod.rs"]),
        "int::t_ok#kill": entry(compile=["crates/int/tests/t_ok.rs"], executed=["crates/int/src/kill.rs"]),
        "cli::bin:cli": entry(compile=["crates/cli/src/main.rs"], executed=["crates/cli/src/main.rs"]),
        "cli::alg02": entry(compile=["crates/cli/tests/alg02.rs", "crates/int/tests/rnext/mod.rs"],
                            executed=["crates/cli/src/main.rs"]),
        "vig::lib": entry(compile=["crates/vig/src/lib.rs"], executed=["crates/vig/src/lib.rs"]),
    }
    pmap = {"core": {"compile": ["crates/core/src/lib.rs", "crates/core/src/a.rs", "fixtures/coef.json"]},
            "int": {"compile": ["crates/int/src/lib.rs", "crates/int/src/kill.rs"]},
            "cli": {"compile": ["crates/cli/src/main.rs"]}, "vig": {"compile": ["crates/vig/src/lib.rs"]}}
    scope = ["core", "int", "cli", "vig"]
    tree = {"base": {".", "crates", "crates/core", "crates/core/src", "research", "crates/hyb", "crates/hyb/src"},
            "head": {".", "crates", "crates/core", "research", "crates/hyb", "crates/hyb/src"}}

    def pick(changes, cls=None, failed=(), controls=(), the_map=None, the_pmap=None):
        return decide(changes, pk, scope, cmap if the_map is None else the_map, pmap if the_pmap is None else the_pmap,
                      lambda path: (cls or {}).get(path, "structural"),
                      lambda side, dirs: set(dirs) & tree[side], failed, controls)

    def chosen(sel):
        out = set()
        for name, e in sel["packages"].items():
            if e["all"]:
                out.add(f"{name}:*")
                continue
            out |= ({f"{name}:lib"} if e["lib"] else set()) | {f"{name}:{t}" for t in e["targets"]}
            out |= {f"{name}:doc"} if e["doc"] else set()
            out |= {f"{name}:example:{x}" for x in e["examples"]}
        return out
    # F1: a .rs file outside the members that a test compiles through #[path]
    assert chosen(pick([("M", "research/r/gain_witness.rs")])) == {"core:c1", "core:doc"}
    # F2: a body change of the cli binary selects the integrators test that include_str!s it
    s = pick([("M", "crates/cli/src/main.rs")], cls={"crates/cli/src/main.rs": "body"})
    assert chosen(s) == {"cli:bin:cli", "cli:alg02", "int:t_ok", "int:doc"}, chosen(s)
    # ... and a comment-only change still selects the text consumer, not the crate that compiles it
    s = pick([("M", "crates/cli/src/main.rs")], cls={"crates/cli/src/main.rs": "noop"})
    assert chosen(s) == {"int:t_ok", "int:doc"}, chosen(s)
    # F3: a test helper of one package compiled by a test of another package
    assert chosen(pick([("M", "crates/int/tests/rnext/mod.rs")])) == {"cli:alg02"}
    # F4: a test source read (hashed) at run time by another target
    assert chosen(pick([("M", "crates/int/tests/u.rs")])) == {"int:t_ok", "int:doc"}
    # F5: a tests/ file that the lib unit tests include!
    assert chosen(pick([("M", "crates/int/tests/support/a02.rs")])) == {"int:lib", "int:doc"}
    # F6: a file named only in a table that a test reads
    assert chosen(pick([("M", "research/z/FROZEN.json")])) == {"int:t_ok", "int:doc"}
    # F7: a new source file in a non-member crate that a test lists; a member's source it reads
    s = pick([("A", "crates/hyb/src/new.rs")])
    assert chosen(s) == {"int:t_ok", "int:doc"} and "lists crates/hyb/src" in s["packages"]["int"]["why"]["t_ok"][0], s
    assert chosen(pick([("M", "crates/vig/src/lib.rs")], cls={"crates/vig/src/lib.rs": "noop"})) == {"int:t_ok", "int:doc"}
    # an added path that a test probed; a deleted directory that a test listed
    assert chosen(pick([("A", "research/new/probe.json")])) == {"int:t_ok", "int:doc"}
    s = pick([("D", "crates/core/src/old.rs")], cls={})
    assert chosen(s) >= {"core:*", "int:*", "cli:*", "vig:*"}, chosen(s)
    s = pick([("D", "crates/core/src/x/old.txt")])
    assert chosen(s) == {"int:t_ok", "int:doc"} and "observed crates/core/src" in str(s["packages"]["int"]["why"]), s
    # body change: only the targets that executed the file, plus doc tests of the package and dependents
    s = pick([("M", "crates/core/src/a.rs")], cls={"crates/core/src/a.rs": "body"})
    assert chosen(s) == {"core:lib", "core:c1", "core:doc", "int:doc", "vig:doc"}, chosen(s)
    # structural change: the package and its dependents run whole
    s = pick([("M", "crates/int/src/lib.rs")], cls={"crates/int/src/lib.rs": "structural"})
    assert chosen(s) == {"int:*", "cli:*"}, chosen(s)
    # a non-Rust input of a library is structural for its package (include_str!)
    assert chosen(pick([("M", "fixtures/coef.json")])) == {"core:*", "int:*", "cli:*", "vig:*"}
    # noop change in a file no target reads or includes textually: nothing
    s = pick([("M", "crates/vig/src/lib.rs")], cls={"crates/vig/src/lib.rs": "noop"},
             the_map={k: v for k, v in cmap.items() if k != "int::t_ok"} | {"int::t_ok": entry()})
    assert chosen(s) == set(), chosen(s)
    # a negative control's own inputs select its target
    s = pick([("M", "crates/int/src/kill.rs")], cls={"crates/int/src/kill.rs": "body"})
    assert "int:t_ok" in chosen(s) and "negative control kill" in str(s["packages"]["int"]["why"]["t_ok"]), s
    # F11: an example whose compile set contains the changed path is compiled
    assert chosen(pick([("M", "crates/int/tests/common/mod.rs")])) == {"int:example:ex"}
    # rule 5: unmapped targets and controls run when the diff is not empty, not otherwise
    small = {k: v for k, v in cmap.items() if k not in ("vig::lib", "int::t_ok#kill")}
    s = pick([("M", "README.md")], the_map=small, controls=["int:t_ok#kill"])
    assert chosen(s) == {"vig:lib", "vig:doc", "int:t_ok", "int:doc"}, chosen(s)
    assert "no mapped target compiles, executes or reads README.md" in s["notes"], s["notes"]
    assert chosen(pick([], the_map=small)) == set()
    # rule 5: a package whose library inputs are not recorded counts as structurally changed
    assert chosen(pick([("M", "README.md")], the_pmap={k: v for k, v in pmap.items() if k != "vig"})) == {"vig:*"}
    # rule 6: baseline failures filtered to current targets (F10, F-02)
    s = pick([], failed=[{"package": "int", "target": "t_ok"}, {"package": "int", "target": "gone"},
                         {"package": "int", "target": "audit2_x"}, {"package": "vig", "target": "doc"},
                         {"package": "out", "target": "x"},
                         {"package": "int", "target": "t_base", "negative_control": True}])
    assert chosen(s) == {"int:t_ok", "int:t_base", "int:doc", "vig:doc"}, chosen(s)
    assert s["baseline_failures_dropped"] == ["int:gone: no longer a test target",
                                              "int:audit2_x: requires the features audit"], s
    assert all("audit2_x" not in str(e) for e in pick([("M", "x")], the_map={})["packages"].values())
    # rule 1: full-mode triggers (image and pull-request base: F8/F-04, F14)
    env = {"image_os": "ubuntu24", "image_version": "20261117.1", "pr_base": "aaa"}
    assert full_reasons(False, "b", True, True, env, dict(env)) == []
    assert full_reasons(True, "b", True, True, env, env) == ["the whole suite was asked for"]
    assert full_reasons(False, None, False, False, env, {}) == ["no baseline run with a complete outcome"]
    assert "not an ancestor" in full_reasons(False, "b", False, True, env, env)[0]
    assert full_reasons(False, "b", True, False, env, env) == ["the baseline run left no input map"]
    assert "runner image version" in full_reasons(False, "b", True, True, env, dict(env, image_version="x"))[0]
    assert "pull-request base" in full_reasons(False, "b", True, True, env, dict(env, pr_base="bbb"))[0]
    assert full_reasons(False, "b", True, True, {"pr_base": None}, {"pr_base": ""}) == []
    # the next map: measured entries replace, lost ones go, the rest carries over and is pruned
    targets, pkgs = update_map({"int::t_ok": {"compile": ["a"]}, "int::gone": {}, "int::t_base": {"compile": ["b"]},
                                "out::x": {}, "int::t_ok#kill": {}},
                               {"int": {"compile": ["x"]}, "core": {"compile": ["y"]}},
                               {"vig::lib": {"compile": ["c"]}}, ["int::t_base"], {"core": ["z"]}, ["int"],
                               pk, scope, "h")
    assert sorted(targets) == ["int::t_ok", "int::t_ok#kill", "vig::lib"] and targets["vig::lib"]["head"] == "h", targets
    assert pkgs == {"core": {"compile": ["y", "z"], "head": "h"}}, pkgs
    # failed negative controls go to outcome.failed, so rule 6 selects them again (F9, F-03)
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "failed-controls.txt")
        assert failed_controls(path) == []
        with open(path, "w") as f:
            f.write("rodas5p-integrators:dense_wfiltered_contracts\nrodas5p-integrators:dense_wfiltered_contracts\n")
        fc = failed_controls(path)
        assert fc == [{"package": "rodas5p-integrators", "target": "dense_wfiltered_contracts",
                       "negative_control": True}], fc
    # baseline lookup with a fake gh: newer, non-ancestor, incomplete and undownloadable runs are
    # skipped; a failing listing or download has its own note (F-09); the branch is data (F-12)
    with tempfile.TemporaryDirectory() as d:
        repo, fake = os.path.join(d, "repo"), os.path.join(d, "bin")
        ident = ["-c", "user.name=ci", "-c", "user.email=ci@localhost", "-c", "commit.gpgsign=false"]
        sh(GIT + ["init", "-q", repo])
        heads = []
        for i in range(3):
            sh(GIT + ident + ["-C", repo, "commit", "-q", "--allow-empty", "-m", f"c{i}"])
            heads.append(sh(GIT + ["-C", repo, "rev-parse", "HEAD"]).stdout.strip())
        other = sh(GIT + ident + ["-C", repo, "commit-tree", "-p", heads[0], "-m", "side",
                                  sh(GIT + ["-C", repo, "rev-parse", "HEAD^{tree}"]).stdout.strip()]).stdout.strip()
        os.makedirs(fake)
        runs = {"workflow_runs": [{"id": i, "created_at": f"2026-11-24T0{i - 100}:00:00Z"} for i in range(101, 107)]}
        with open(os.path.join(d, "listing.json"), "w") as f:
            json.dump(runs, f)
        outcomes = {104: {"head": other, "complete": True}, 103: {"head": heads[1], "complete": False},
                    101: {"head": heads[0], "complete": True, "failed": [{"package": "p", "target": "t"}],
                          "environment": {"pr_base": "b"}}}
        for run_id, o in outcomes.items():
            os.makedirs(os.path.join(d, "art", str(run_id)))
            with open(os.path.join(d, "art", str(run_id), "outcome.json"), "w") as f:
                json.dump(dict({"schema": SCHEMA_OUTCOME, "failed": []}, **o), f)
        with open(os.path.join(d, "art", "101", "map.json"), "w") as f:
            json.dump({"schema": SCHEMA_MAP, "targets": {}, "packages": {}}, f)
        with open(os.path.join(fake, "gh"), "w") as f:
            f.write('#!/bin/sh\nD="$(dirname "$0")/.."\n'
                    'if [ "$1" = api ]; then [ -f "$D/fail" ] && { echo "HTTP 403: rate limited" >&2; exit 1; }; '
                    'cat "$D/listing.json"; exit 0; fi\n'
                    'id="$3"; while [ $# -gt 0 ]; do [ "$1" = -D ] && dest="$2"; shift; done\n'
                    '[ "$id" = 102 ] && { echo "HTTP 502: Bad Gateway" >&2; exit 1; }\n'
                    '[ -f "$D/art/$id/outcome.json" ] && { cp "$D/art/$id/"* "$dest/"; exit 0; }\n'
                    'echo "no valid artifacts found to download" >&2; exit 1\n')
        os.chmod(os.path.join(fake, "gh"), 0o755)
        env = dict(os.environ, PATH=fake + os.pathsep + os.environ.get("PATH", ""), GITHUB_REPOSITORY="o/r")

        def lookup(name):
            out = os.path.join(d, name)
            r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "baseline", "--workflow", "w.yml",
                                "--artifact", "a", "--branch", "x;$(false)", "--run-id", "105", "--head", heads[2],
                                "--out", out], cwd=repo, env=env, capture_output=True, text=True)
            assert r.returncode == 0, r.stderr
            return load_json(out)
        got = lookup("b1.json")
        assert got["base"] == heads[0] and got["run_id"] == 101 and got["failed"] == [{"package": "p", "target": "t"}], got
        assert got["environment"] == {"pr_base": "b"} and got["map_path"], got
        assert got["errors"] == ["download of run 102 failed (exit 1): HTTP 502: Bad Gateway"], got
        assert "1 not complete, 1 not an ancestor" in got["note"], got
        open(os.path.join(d, "fail"), "w").close()
        got = lookup("b2.json")
        assert got["base"] is None and got["note"].startswith("the run listing failed (exit 1): HTTP 403"), got
    # has / selected
    sel = {"packages": {"int": dict(new_entry(), targets=["t_ok"], doc=True, examples=["ex"])}}
    assert selected(sel, "int", "t_ok") and selected(sel, "int", "doc") and selected(sel, "int", "example:ex")
    assert not selected(sel, "int", "lib") and not selected(sel, "core", "c1")
    m = RERUN.findall("error: test failed, to rerun pass `-p rodas5p-integrators --test stage_residual_telemetry`\n"
                      "    `-p vigilode --lib`\n    `-p vigilode --doc`")
    assert [x[0] for x in m] == ["rodas5p-integrators", "vigilode", "vigilode"], m
    # the workflows (F-01, F-03, F-05, F-06, F-07, F-11, F-12, F8/F-04, F14)
    root = repo_root()
    if os.path.isdir(os.path.join(root, ".github", "workflows")):
        problems = check_workflows(root)
        assert not problems, problems
    print("self-test passed", flush=True)
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("baseline")
    b.add_argument("--workflow", required=True)
    b.add_argument("--artifact", required=True)
    b.add_argument("--branch", required=True)
    b.add_argument("--run-id", required=True)
    b.add_argument("--head")
    b.add_argument("--out", required=True)
    s = sub.add_parser("select")
    s.add_argument("--baseline", required=True)
    s.add_argument("--head", required=True)
    s.add_argument("--pr-base", default="")
    s.add_argument("--scope", nargs="+", required=True)
    s.add_argument("--workflow-file", required=True)
    s.add_argument("--full", default="false")
    s.add_argument("--control", action="append", default=[])
    s.add_argument("--profile", default="dev")  # accepted for compatibility; selection needs no build
    s.add_argument("--out", required=True)
    r = sub.add_parser("run")
    r.add_argument("--selection", required=True)
    r.add_argument("--mode", choices=("normal", "ignored"), required=True)
    r.add_argument("--profile", default="dev")
    r.add_argument("--skip", action="append", default=[])
    r.add_argument("--outcome", required=True)
    r.add_argument("--measure", action="store_true")
    r.add_argument("--baseline")
    r.add_argument("--map-out", default="map.json")
    r.add_argument("--work-dir")
    r.add_argument("--control-maps", nargs="*", default=[])
    r.add_argument("--failed-controls")
    r.add_argument("--strace", default="strace")
    r.add_argument("--jobs", type=int, default=0)
    c = sub.add_parser("control")
    c.add_argument("--selection", required=True)
    c.add_argument("--target", required=True)
    c.add_argument("--test", required=True)
    c.add_argument("--signature", required=True)
    c.add_argument("--profile", default="dev")
    c.add_argument("--map-out", required=True)
    c.add_argument("--failed-file", required=True)
    c.add_argument("--work-dir")
    c.add_argument("--strace", default="strace")
    h = sub.add_parser("has")
    h.add_argument("--selection", required=True)
    h.add_argument("--target", required=True)
    sub.add_parser("self-test")
    args = parser.parse_args()
    handlers = {"baseline": baseline, "select": select, "run": run, "control": control, "has": has,
                "self-test": lambda _a: self_test()}
    return handlers[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
