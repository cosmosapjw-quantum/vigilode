#!/usr/bin/env python3
"""Selections of the review cases with the input map of the local end-to-end run (task ci-v3,
verification 2). Not run by CI.

  python3 -I v3_selection_cases.py --repo <clone> --base <sha> --map <map.json> --out <json>

--repo is a clone of the repository (a sparse checkout of crates/, .github/ and tools/ suffices)
whose object store holds --base, the head of the measured run. For every case the script writes a
scratch commit on top of --base in that clone with plumbing on a temporary index (no branch, no
working-tree change) and evaluates it twice:
  restricted  the selector's own rules (select_tests.decide) on the real diff, the real change
              classification and the measured input map, with the targets of every package
              restricted to the measured ones. In CI every target has a map entry after the first
              full run; the local run measured only a few targets, and rule 5 would otherwise add
              every unmeasured target to every case.
  cli         the real `select` subcommand in the clone on the unrestricted workspace, where rule 5
              adds every unmeasured target; recorded as counts and as the presence of the expected
              targets.
"""

import argparse
import importlib.util
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
SELECTOR = os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "ci", "select_tests.py"))
SCOPE = ["rodas5p-core", "rodas5p-krylov", "rodas5p-integrators", "rodas5p-fair-ab", "rodas5p-cli",
         "vigilode", "vigilode-coupled"]
INT = "rodas5p-integrators"


def load_selector():
    spec = importlib.util.spec_from_file_location("select_tests", SELECTOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(repo, *args, env=None, stdin=None):
    r = subprocess.run(["git", "-C", repo, *args], capture_output=True, env=env, input=stdin,
                       encoding="utf-8", errors="surrogateescape")
    if r.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {r.stderr}")
    return r.stdout


def commit_with(repo, base, edits, message):
    """A commit on top of base whose tree applies edits {path: new text or None (delete)}."""
    with tempfile.TemporaryDirectory() as d:
        env = dict(os.environ, GIT_INDEX_FILE=os.path.join(d, "index"), GIT_AUTHOR_NAME="ci-v3",
                   GIT_AUTHOR_EMAIL="ci-v3@localhost", GIT_COMMITTER_NAME="ci-v3",
                   GIT_COMMITTER_EMAIL="ci-v3@localhost", GIT_AUTHOR_DATE="2026-11-24T00:00:00Z",
                   GIT_COMMITTER_DATE="2026-11-24T00:00:00Z")
        git(repo, "read-tree", base, env=env)
        for path, text in edits.items():
            if text is None:
                git(repo, "update-index", "--force-remove", "--", path, env=env)
            else:
                blob = git(repo, "hash-object", "-w", "--stdin", env=env, stdin=text).strip()
                git(repo, "update-index", "--add", "--cacheinfo", f"100644,{blob},{path}", env=env)
        tree = git(repo, "write-tree", env=env).strip()
        return git(repo, "commit-tree", tree, "-p", base, "-m", message, env=env).strip()


def show(repo, base, path):
    return git(repo, "show", f"{base}:{path}")


def insert_in_first_body(st, text, statement):
    """text with `statement` inserted as the first line of the first non-const function body that
    spans more than two lines."""
    lines = text.split("\n")
    for start, end, const in st.fn_body_spans(st.rust_tokens(text)):
        if not const and end - start > 2:
            indent = " " * (len(lines[start]) - len(lines[start].lstrip()) + 4)
            return "\n".join(lines[:start] + [indent + statement] + lines[start:])
    raise SystemExit("no function body found")


def cases(st, repo, base, cmap):
    gain = "research/reaudit_accuracy_speed_20261010/gain_witness.rs"
    cli_main = "crates/rodas5p-cli/src/main.rs"
    rnext = "crates/rodas5p-integrators/tests/rnext_common/mod.rs"
    unified = "crates/rodas5p-integrators/tests/unified_candidate_contracts.rs"
    a02 = "crates/rodas5p-integrators/tests/support/a02_packet_counter_contract.rs"
    frozen = "research/generic_policy_redesign_v33/results/calibration_analysis/FROZEN_ZETA34_POLICY.json"
    alg01 = "crates/rodas5p-integrators/tests/alg01_common/mod.rs"
    krylov_lib = "crates/rodas5p-krylov/src/lib.rs"
    vig_lib = "crates/vigilode/src/lib.rs"
    # N2: the integrators source executed by the fewest measured targets (at least one)
    executed = {}
    for key, item in cmap.items():
        for f in item.get("executed", []):
            if f.startswith("crates/rodas5p-integrators/src/") and f.endswith(".rs"):
                executed.setdefault(f, set()).add(key)
    body_file = sorted(executed, key=lambda f: (len(executed[f]), f))[0]
    out = [
        dict(id="F1", desc="numeric change in the identity fixture of gain_witness.rs (replays 95d589e86)",
             edits={gain: show(repo, base, gain).replace("let identity = [[1.0, 0.0], [0.0, 1.0]];",
                                                         "let identity = [[1.0, 1.0], [0.0, 1.0]];", 1)},
             expect=["rodas5p-core:residual_gain_contract"]),
        dict(id="F2", desc="comment-only change of the cli binary source that an integrators test include_str!s",
             edits={cli_main: show(repo, base, cli_main) + "// ci-v3 probe: a comment-only change\n"},
             expect=[f"{INT}:constants_provenance_contracts"]),
        dict(id="F3", desc="new helper in an integrators test module that a cli test #[path]-includes",
             edits={rnext: show(repo, base, rnext) + "\npub fn ci_v3_probe() -> u32 {\n    1\n}\n"},
             expect=["rodas5p-cli:alg02_predictive_controller"]),
        dict(id="F4", desc="new test in a test source whose sha256 two other targets pin at run time",
             edits={unified: show(repo, base, unified) + "\n#[test]\nfn ci_v3_probe() {}\n"},
             expect=[f"{INT}:ptsw2b_contracts", f"{INT}:ppsw1_6b_contracts"]),
        dict(id="F5", desc="change of a tests/support file that the lib unit tests include!",
             edits={a02: show(repo, base, a02) + "// ci-v3 probe\n"},
             expect=[f"{INT}:lib"]),
        dict(id="F6", desc="change of a research artifact named only in docs/CONSTANTS_PROVENANCE.toml",
             edits={frozen: show(repo, base, frozen).replace('"candidate_count": 24', '"candidate_count": 25', 1)},
             expect=[f"{INT}:constants_provenance_contracts"]),
        dict(id="F7", desc="new source file under crates/rodas5p-krylov/src",
             edits={"crates/rodas5p-krylov/src/ci_v3_probe.rs": "// ci-v3 probe\n"},
             expect=[f"{INT}:generic_scope_contracts"]),
        dict(id="F7-hybrid", desc="new source file in the non-member crate vigilode-hybrid (no package rule)",
             edits={"crates/vigilode-hybrid/src/ci_v3_probe.rs": "// ci-v3 probe\n"},
             expect=[f"{INT}:generic_scope_contracts"]),
        dict(id="F7-vigilode", desc="comment-only change of vigilode's lib.rs (vigilode is not an integrators dependency)",
             edits={vig_lib: show(repo, base, vig_lib) + "// ci-v3 probe\n"},
             expect=[f"{INT}:generic_scope_contracts"]),
        dict(id="F11", desc="new helper in tests/alg01_common/mod.rs, which the example sp01_dupfix_profile #[path]-includes",
             edits={alg01: show(repo, base, alg01) + "\npub fn ci_v3_probe() -> u32 {\n    1\n}\n"},
             expect=[f"{INT}:example:sp01_dupfix_profile"]),
        dict(id="N1", desc="comment-only change of crates/rodas5p-krylov/src/lib.rs",
             edits={krylov_lib: show(repo, base, krylov_lib) + "// ci-v3 probe: a comment-only change\n"},
             expect=[f"{INT}:generic_scope_contracts"],
             only=[f"{INT}:generic_scope_contracts", f"{INT}:doc"],
             note="every crates/*/src file is read as text by generic_scope_contracts, so it is the one "
                  "text consumer; no target that compiled or executed the file runs"),
        dict(id="N1-no-reader", desc="N1 evaluated without generic_scope_contracts, the only reader of the file",
             edits={krylov_lib: show(repo, base, krylov_lib) + "// ci-v3 probe: a comment-only change\n"},
             drop=[f"{INT}::generic_scope_contracts"], expect=[], only=[]),
        dict(id="N2", desc=f"body change (a new first statement) in {body_file}",
             edits={body_file: insert_in_first_body(st, show(repo, base, body_file), "let _ci_v3_probe = 0u8;")},
             expect=sorted(f"{k.split('::')[0]}:{k.split('::')[1]}" for k in executed[body_file] if "#" not in k),
             only="executed+readers+unit-tests+doc", body_file=body_file),
        dict(id="S1", desc="structural change: a new const in crates/rodas5p-core/src/lib.rs",
             edits={"crates/rodas5p-core/src/lib.rs": show(repo, base, "crates/rodas5p-core/src/lib.rs")
                    + "\npub const CI_V3_PROBE: u32 = 1;\n"},
             expect=["rodas5p-core:*", "rodas5p-krylov:*", f"{INT}:*", "rodas5p-cli:*"]),
    ]
    return out


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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True)
    ap.add_argument("--base", required=True)
    ap.add_argument("--map", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    st = load_selector()
    repo = os.path.abspath(args.repo)
    measured = st.load_json(args.map)
    cmap, pmap = measured["targets"], measured.get("packages", {})
    packages, _ = st.workspace(repo)
    mapped = {st.split_key(k)[:2] for k in cmap}

    def restricted(drop=()):
        out = {}
        for name, info in packages.items():
            keep = {t for p, t in mapped if p == name and f"{p}::{t}" not in drop}
            out[name] = dict(info, lib=info["lib"] and "lib" in keep,
                             bins=[b for b in info["bins"] if "bin:" + b in keep],
                             tests=[t for t in info["tests"] if t in keep],
                             examples=[e for e in info["examples"] if "example:" + e in keep])
        return out

    results, failures = [], 0
    for case in cases(st, repo, args.base, cmap):
        head = commit_with(repo, args.base, case["edits"], f"ci-v3 selection case {case['id']}")
        changes = st.changed_entries(repo, args.base, head)
        drop = set(case.get("drop", []))
        sel = st.decide(changes, restricted(drop), SCOPE, {k: v for k, v in cmap.items() if k not in drop}, pmap,
                        lambda path: st.classify_source_change(repo, args.base, head, path),
                        lambda side, dirs: st.existing_dirs(repo, args.base if side == "base" else head, dirs))
        got = chosen(sel)
        missing = [t for t in case["expect"] if t not in got]
        problems = [f"not selected: {t}" for t in missing]
        only = case.get("only")
        if only == "executed+readers+unit-tests+doc":
            f = case["body_file"]
            allowed = set()
            for key, item in cmap.items():
                pkg, target, control = st.split_key(key)
                if control:
                    continue
                if f in item.get("executed", []) or f in item.get("read", []) or (
                        target == "lib" and pkg == INT and f in item.get("compile", [])):
                    allowed.add(f"{pkg}:{target}")
            extra = sorted(t for t in got if not t.endswith(":doc") and t not in allowed)
            problems += [f"selected beyond executed, readers, unit tests and doc tests: {t}" for t in extra]
            only_text = "targets that executed the file, read it, or compile it as their own crate (lib unit tests), plus doc tests"
        elif only is not None:
            extra = sorted(got - set(only))
            problems += [f"selected unexpectedly: {t}" for t in extra]
            only_text = ", ".join(only) or "nothing"
        else:
            only_text = None
        why = {f"{p}:{t}": w for p, e in sel["packages"].items() for t, w in e["why"].items()}
        # the real subcommand on the unrestricted workspace
        with tempfile.TemporaryDirectory() as d:
            base_json = os.path.join(d, "baseline.json")
            with open(base_json, "w") as fh:
                json.dump({"base": args.base, "run_id": "local", "failed": [], "map_path": os.path.abspath(args.map),
                           "environment": {}, "note": "local e2e map"}, fh)
            out_json = os.path.join(d, "selection.json")
            env = {k: v for k, v in os.environ.items() if k not in ("ImageOS", "ImageVersion")}
            r = subprocess.run([sys.executable, "-I", SELECTOR, "select", "--baseline", base_json, "--head", head,
                                "--workflow-file", ".github/workflows/ignored-tests.yml", "--out", out_json,
                                "--scope", *SCOPE], cwd=repo, capture_output=True, text=True, env=env)
            cli = {"exit": r.returncode}
            if r.returncode == 0:
                cli_sel = st.load_json(out_json)
                cli_got = chosen(cli_sel)
                unmapped = sum(1 for e in cli_sel["packages"].values() for w in e["why"].values() if "no map entry" in w)
                cli.update(mode=cli_sel["mode"], units=len(cli_got), units_by_rule5=unmapped,
                           expected_selected=all(t in cli_got or t.split(":")[0] + ":*" in cli_got for t in case["expect"]),
                           classified=cli_sel["classified"], changed=cli_sel["changed"])
                if not cli["expected_selected"]:
                    problems.append("the select subcommand did not select every expected target")
            else:
                problems.append("the select subcommand failed: " + r.stderr[-300:])
        failures += bool(problems)
        results.append({"id": case["id"], "desc": case["desc"], "head": head,
                        "changes": [list(c) for c in changes], "classified": sel["classified"],
                        "expect": case["expect"], "only": only_text, "selected": sorted(got),
                        "why": {k: v for k, v in sorted(why.items())}, "notes": sel["notes"],
                        "note": case.get("note"), "cli": cli, "ok": not problems, "problems": problems})
        status = "OK  " if not problems else "FAIL"
        print(f"{status} {case['id']}: {case['desc']}")
        print(f"     changes {changes}; classified {sel['classified']}")
        print(f"     selected (restricted): {', '.join(sorted(got)) or 'nothing'}")
        for t in sorted(got):
            print(f"       {t}: {'; '.join(sorted(set(why.get(t, []))))[:300]}")
        if only_text:
            print(f"     must be only: {only_text}")
        print(f"     select subcommand (unrestricted): {cli}"[:400])
        for p in problems:
            print(f"     PROBLEM: {p}")
    st.write_json(args.out, {"schema": "vigilode.loop.ci-v3-selection-cases.v1", "base": args.base,
                             "map": os.path.abspath(args.map), "map_targets": sorted(cmap),
                             "cases": results, "failures": failures})
    print(f"{len(results)} cases, {failures} failing")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
