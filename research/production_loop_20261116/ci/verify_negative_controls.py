#!/usr/bin/env python3
"""Check research/qualification/negative_controls/negative_fixture_manifest.json against the tree.

usage: verify_negative_controls.py [--run-kill-gate <cargo target directory>]

1. Every entry of authoritative_fixtures: the file exists and has the recorded SHA-256 and size.
2. With --run-kill-gate: every test of historical_negative_controls is run
   (cargo +1.94.1 test --release --locked ... -- --ignored --exact <test>) and must fail with its
   recorded failure signature.
Prints one line per check and a JSON summary line; exit 0 only if every check holds.
"""
import hashlib
import json
import os
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
MANIFEST = "research/qualification/negative_controls/negative_fixture_manifest.json"


def main():
    args = sys.argv[1:]
    target = args[1] if args[:1] == ["--run-kill-gate"] else None
    with open(os.path.join(ROOT, MANIFEST), "rb") as f:
        raw = f.read()
    manifest = json.loads(raw)
    summary = {"manifest": MANIFEST, "manifest_sha256": hashlib.sha256(raw).hexdigest(),
               "fixtures": len(manifest["authoritative_fixtures"]), "fixtures_matching": 0,
               "negative_controls": len(manifest["historical_negative_controls"]),
               "negative_controls_run": 0, "negative_controls_failing_with_signature": 0}
    for fixture in manifest["authoritative_fixtures"]:
        path = os.path.join(ROOT, fixture["filename"])
        ok = False
        if os.path.isfile(path):
            with open(path, "rb") as f:
                data = f.read()
            ok = hashlib.sha256(data).hexdigest() == fixture["sha256"] and len(data) == fixture["bytes"]
        summary["fixtures_matching"] += ok
        print(f"FIXTURE {fixture['filename']} {'match' if ok else 'MISMATCH'}")
    if target:
        for control in manifest["historical_negative_controls"]:
            stem = os.path.splitext(os.path.basename(control["test_file"]))[0]
            package = control["test_file"].split("/")[1]
            cmd = ["cargo", "+1.94.1", "test", "--release", "--locked", "-j", "4", "-p", package,
                   "--test", stem, "--", "--ignored", "--exact", control["test_name"]]
            proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                                  env={**os.environ, "CARGO_TARGET_DIR": target})
            text = proc.stdout + proc.stderr
            failed = proc.returncode != 0 and "test result: FAILED" in text
            signed = control["failure_signature"] in text
            summary["negative_controls_run"] += 1
            summary["negative_controls_failing_with_signature"] += failed and signed
            print(f"NEGATIVE_CONTROL {control['test_name']} exit={proc.returncode} "
                  f"failed={failed} recorded_signature_present={signed}")
            print("  command: " + " ".join(cmd))
    ok = summary["fixtures_matching"] == summary["fixtures"] and (
        not target or summary["negative_controls_failing_with_signature"] == summary["negative_controls"])
    summary["all_checks_hold"] = bool(ok)
    print("SUMMARY " + json.dumps(summary, sort_keys=True))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
