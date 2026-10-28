#!/usr/bin/env python3
"""Verify the unpacked source subset against the commit's git ls-tree and
record the host. Writes results/IDENTITY.json (start) or IDENTITY_END.json.

    python3 -s verify_source.py CAMP [--end]

Each file's git blob id (sha1 of "blob <size>\\0" + bytes) must equal the
ls-tree entry of the expected commit (SOURCE_LS_TREE.txt, made locally by
launch_from_local.sh with the commit and tree ids in IDENTITY_LOCAL.json).
"""
import hashlib
import json
import os
import platform
import subprocess
import sys
from pathlib import Path


def blob_id(p):
    b = Path(p).read_bytes()
    return hashlib.sha1(b"blob %d\0" % len(b) + b).hexdigest()


def sha256(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def sh(cmd):
    try:
        return subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=60).stdout.strip()
    except Exception as e:  # noqa: BLE001
        return f"unavailable: {e!r}"


def main():
    camp = Path(sys.argv[1]).resolve()
    end = "--end" in sys.argv
    src = camp / "src"
    (camp / "results").mkdir(parents=True, exist_ok=True)
    lines = (camp / "SOURCE_LS_TREE.txt").read_text().splitlines()
    bad, n = [], 0
    for line in lines:
        meta, path = line.split("\t", 1)
        mode, kind, oid = meta.split()
        if kind != "blob":
            continue
        n += 1
        p = src / path
        if not p.is_file() or blob_id(p) != oid:
            bad.append(path)
    local = json.loads((camp / "IDENTITY_LOCAL.json").read_text())
    bins = {p.name: sha256(p) for p in sorted((camp / "bin").glob("*")) if p.is_file()} if (camp / "bin").exists() else {}
    doc = {"expected_commit": local.get("commit"), "expected_tree": local.get("tree"),
           "source_paths": local.get("paths"), "files_checked": n, "mismatched_files": bad[:50],
           "tree_matches_expected": bool(n > 0 and not bad),
           "cargo_lock_sha256": sha256(src / "Cargo.lock") if (src / "Cargo.lock").exists() else None,
           "binaries": bins,
           "julia_project_sha256": {f.name: sha256(f) for f in sorted((camp / "julia-env").glob("*.toml"))}
           if (camp / "julia-env").exists() else None,
           "diffsol_lock_sha256": sha256(camp / "diffsol-adapter" / "Cargo.lock")
           if (camp / "diffsol-adapter" / "Cargo.lock").exists() else None,
           "host": {"hostname": platform.node(), "platform": platform.platform(), "nproc": os.cpu_count(),
                    "affinity": sorted(os.sched_getaffinity(0)), "uptime": sh("uptime"), "free_g": sh("free -g"),
                    "df_root": sh("df -h /root"), "lscpu": sh("lscpu | head -30"),
                    "other_heavy_processes": sh("ps -eo pid,pcpu,rss,etime,comm --sort=-pcpu | head -15"),
                    "rustc": sh("RUSTUP_TOOLCHAIN=1.94.1 rustc -V"), "cargo": sh("RUSTUP_TOOLCHAIN=1.94.1 cargo -V"),
                    "python": sh("python3 -s -c 'import sys,numpy,scipy;print(sys.version.split()[0],numpy.__version__,scipy.__version__)'")}}
    out = camp / "results" / ("IDENTITY_END.json" if end else "IDENTITY.json")
    out.write_text(json.dumps(doc, indent=1) + "\n")
    print(json.dumps({"tree_matches_expected": doc["tree_matches_expected"], "files": n, "bad": len(bad)}))
    sys.exit(0 if doc["tree_matches_expected"] else 3)


if __name__ == "__main__":
    main()
