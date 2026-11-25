#!/usr/bin/env python3
"""Source and arrival status of the port contracts (loop L1 node V12).

For every item of port_contracts.json this script checks, with git only:
  - that each declared source revision exists and holds each declared source path;
  - whether the revision is an ancestor of the checked-out head;
  - whether each path that the item needs is present at the head;
  - which unmerged commits of a declared follow-up revision touch the item's paths.
It writes SOURCE_STATUS.json next to itself. Nothing is typed in: every field comes from git.

usage: python3 -I verify_sources.py        (run from anywhere inside the repository)
"""
import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = subprocess.run(["git", "-C", HERE, "rev-parse", "--show-toplevel"], capture_output=True,
                      text=True, check=True).stdout.strip()


def git(*args, check=False):
    # Pathspecs are relative to the directory git runs in: always run at the repository root.
    proc = subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True)
    if check and proc.returncode != 0:
        raise RuntimeError(proc.stderr)
    return proc.returncode, proc.stdout.strip()


def has_commit(rev):
    return git("cat-file", "-e", rev + "^{commit}")[0] == 0


def has_path(rev, path):
    return git("cat-file", "-e", f"{rev}:{path}")[0] == 0


def is_ancestor(rev):
    return git("merge-base", "--is-ancestor", rev, "HEAD")[0] == 0


def main():
    root = ROOT
    head = git("rev-parse", "HEAD", check=True)[1]
    with open(os.path.join(HERE, "port_contracts.json"), encoding="utf-8") as f:
        contracts = json.load(f)
    items = []
    for item in contracts["items"]:
        refs = []
        for ref in item.get("source_refs", []):
            rev = ref["rev"]
            present = has_commit(rev)
            entry = {
                "rev": rev,
                "commit_present": present,
                "ancestor_of_head": present and is_ancestor(rev),
                "paths": {p: (present and has_path(rev, p)) for p in ref.get("paths", [])},
            }
            if present and not entry["ancestor_of_head"] and ref.get("paths"):
                base = git("merge-base", rev, "HEAD")[1]
                code, out = git("log", "--format=%h %s", f"{base}..{rev}", "--", *ref["paths"])
                entry["unmerged_commits_touching_paths"] = out.splitlines() if out else []
                code, out = git("diff", "--stat", "HEAD", rev, "--", *ref["paths"])
                entry["differs_from_head"] = bool(out)
            refs.append(entry)
        head_paths = {p: os.path.exists(os.path.join(root, p)) for p in item.get("head_paths", [])}
        tests = {p: os.path.exists(os.path.join(root, p)) for p in item.get("head_tests", [])}
        if not item.get("head_paths"):
            arrival = "no_code_path_declared"
        elif all(head_paths.values()):
            behind = any(r.get("differs_from_head") and r.get("unmerged_commits_touching_paths")
                         for r in refs)
            arrival = "at_head_but_follow_up_commits_unmerged" if behind else "at_head"
        elif any(head_paths.values()):
            arrival = "partly_at_head"
        else:
            arrival = "not_at_head"
        items.append({"id": item["id"], "name": item["name"], "arrival": arrival,
                      "source_refs": refs, "head_paths": head_paths, "head_tests": tests})
    out = {"schema": "vigilode.port-source-status.v1", "head": head,
           "generated_by": "verify_sources.py (git only)", "items": items}
    with open(os.path.join(HERE, "SOURCE_STATUS.json"), "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2, ensure_ascii=False)
        f.write("\n")
    for it in items:
        missing = [p for r in it["source_refs"] for p, ok in r["paths"].items() if not ok]
        print(f"{it['id']} {it['arrival']:40s} missing_source_paths={len(missing)}")


if __name__ == "__main__":
    main()
