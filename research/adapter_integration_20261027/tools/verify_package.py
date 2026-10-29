#!/usr/bin/env python3
"""Verify the published packet and its final measured sources, without rerunning science."""
import hashlib
import json
from pathlib import Path


def main():
    root = Path(__file__).resolve().parents[3]
    packet = Path(__file__).resolve().parents[1]
    manifest = json.loads((packet / "ARTIFACT_MANIFEST.json").read_text())
    expected = set()
    for entry in manifest["files"]:
        path = root / entry["path"]
        data = path.read_bytes()
        assert len(data) == entry["bytes"], f"size mismatch: {path}"
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], f"hash mismatch: {path}"
        expected.add(path.resolve())
    actual = {p.resolve() for p in packet.rglob("*") if p.is_file()
              and p.name != "ARTIFACT_MANIFEST.json" and "__pycache__" not in p.parts}
    assert actual == expected, "packet coverage differs from manifest"
    for entry in manifest["source_files"]:
        data = (root / entry["path"]).read_bytes()
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["path"]
    for name in ["facade_r3", "measurement_build_r2", "pairs_r2"]:
        directory = packet / "validation" / name
        receipt = json.loads((directory / "RECEIPT.json").read_text())
        assert receipt["exit_code"] == 0 and receipt["source_unchanged_during_command"], name
        for rel, expected_hash in json.loads((directory / "SOURCE_INPUTS.json").read_text()).items():
            assert hashlib.sha256((root / rel).read_bytes()).hexdigest() == expected_hash, rel
        for rel in ["stdout.log", "stderr.log", "SOURCE_INPUTS.json"]:
            assert hashlib.sha256((directory / rel).read_bytes()).hexdigest() == receipt[rel + "_sha256"]
    analysis = json.loads((packet / "benchmark/analysis_r2/ANALYSIS.json").read_text())
    assert not analysis["failed_cells"] and len(analysis["rows"]) == 26
    assert any(not r["accuracy_pass"] and r["report_only"] for r in analysis["rows"])
    decision = json.loads((packet / "DECISIONS.json").read_text())
    assert decision["broad_production"]["status"] == "HOLD"
    for p in packet.rglob("*.json"):
        json.loads(p.read_text())
    print(json.dumps({"status": "PASS", "packet_files": len(expected),
                      "source_files": len(manifest["source_files"]),
                      "scope": "identity, coverage, receipt/source consistency; no new solver run"}))


if __name__ == "__main__":
    main()
