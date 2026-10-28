#!/usr/bin/env python3
"""Preregistered import of a verified, copied-back campaign into the research node.

    python3 -s import_results.py LOCAL_NCP_DIR NODE_DIR

LOCAL_NCP_DIR is the verified copy written by fetch_results.sh
(~/vigilode-campaign/bassrei-ncp-c1-<UTC>/ncp). NODE_DIR is
research/bassrei_ncp_wp_c1_20261022 of the checkout. Copies:

- results.json: the campaign's VERDICT.json (verdict, gate table, gate values);
- campaign/: PLAN, PARITY, LADDER.jsonl, VG.jsonl, ABBA.jsonl, ABBA_PLAN,
  WORK_PRECISION, MATCHED, ABBA_SUMMARY, IDENTITY(_END), SETUP, DIFFSOL_SETUP,
  JULIA_SETUP, the diffsol lock, REPORT.md, OUTPUT_SHA256SUMS.txt (the raw
  manifest), IDENTITY_LOCAL.json, SOURCE_LS_TREE.txt;
- the tarball's sha256 file (the tarball itself, holding raw/ with every
  request and stdout, stays at its local path, recorded here; it is too big
  for the repository).

Writes IMPORT_MANIFEST.json with the sha256 of every imported file (the
ledger row's outputs_sha256 set). Refuses when the copy does not verify.
"""
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

FILES = ["PLAN.json", "PARITY.json", "LADDER.jsonl", "VG.jsonl", "ABBA.jsonl", "ABBA_PLAN.json", "WORK_PRECISION.json",
         "MATCHED.json", "ABBA_SUMMARY.json", "IDENTITY.json", "IDENTITY_END.json", "SETUP.json", "DIFFSOL_SETUP.json",
         "JULIA_SETUP.json", "DIFFSOL_Cargo.lock", "DIFFSOL_Cargo.lock.sha256"]
TOP = ["REPORT.md", "OUTPUT_SHA256SUMS.txt", "IDENTITY_LOCAL.json", "SOURCE_LS_TREE.txt"]


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    src, node = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
    tar = next((src / n for n in ("OUT.tar.gz", "OUT_PARTIAL.tar.gz") if (src / n).exists()), None)
    if tar is None:
        sys.exit("no OUT.tar.gz / OUT_PARTIAL.tar.gz in the copy")
    r = subprocess.run(["sha256sum", "-c", tar.name + ".sha256"], cwd=src, capture_output=True, text=True)
    r2 = subprocess.run(["sha256sum", "-c", "--quiet", "OUTPUT_SHA256SUMS.txt"], cwd=src, capture_output=True, text=True)
    if r.returncode or r2.returncode:
        sys.exit(f"copy does not verify: {r.stdout}{r.stderr}{r2.stdout}{r2.stderr}")
    (node / "campaign").mkdir(parents=True, exist_ok=True)
    manifest = {}
    shutil.copy2(src / "results" / "VERDICT.json", node / "results.json")
    manifest["results.json"] = sha(node / "results.json")
    for f in FILES:
        p = src / "results" / f
        if p.exists():
            shutil.copy2(p, node / "campaign" / f)
            manifest[f"campaign/{f}"] = sha(node / "campaign" / f)
    for f in TOP:
        p = src / f
        if p.exists():
            shutil.copy2(p, node / "campaign" / f)
            manifest[f"campaign/{f}"] = sha(node / "campaign" / f)
    shutil.copy2(Path(str(tar) + ".sha256"), node / "campaign" / (tar.name + ".sha256"))
    manifest[f"campaign/{tar.name}.sha256"] = sha(node / "campaign" / (tar.name + ".sha256"))
    doc = {"tarball_local_path": str(tar), "tarball_sha256": sha(tar),
           "verdict": json.loads((node / "results.json").read_text()).get("verdict"), "files_sha256": manifest}
    (node / "IMPORT_MANIFEST.json").write_text(json.dumps(doc, indent=1) + "\n")
    print(json.dumps({"imported": len(manifest), "verdict": doc["verdict"], "tarball_sha256": doc["tarball_sha256"]}))


if __name__ == "__main__":
    main()
