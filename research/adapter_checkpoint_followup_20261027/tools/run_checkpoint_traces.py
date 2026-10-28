#!/usr/bin/env python3
"""Same-checkpoint traces of registered adapter decisions read from existing raw records.

Inputs are the published raw pair records (no new pair timing). Each decision
point is traced once for the chosen method and one alternative over the same
interval; the switching rerun must reproduce the raw record exactly.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

# (case, segment index of the probe/pilot, extra interval end or None)
POINTS = {
    "case5": [(0, None), (2, None), (6, None)],
    "stiff-forced": [(0, None), (4, None)],
    "relaxation": [(0, None), (4, None)],
    "transition-39": [(0, None), (51, None), (51, 0.5), (149, None), (153, None)],
}


def decision_point(segs, i, extend):
    probe = segs[i]
    start = probe["start_time"]
    peers = [s for s in segs if s["start_time"] == start and s is not probe]
    dopri = probe if probe["method"] == "dopri5" else next(s for s in peers if s["method"] == "dopri5")
    rodas = probe if probe["method"] == "rodas5p" else next(
        (s for s in peers if s["method"] == "rodas5p"), None)
    if probe["method"] == "rodas5p":
        target = probe["target_time"]          # entry pilot: the explicit prefix
    else:
        target = probe["target_time"]          # return recheck: the explicit window
    if extend is not None:
        target = extend
    return {"probe_index": i, "start": start, "target": target,
            "dopri_h0": dopri["initial_step"],
            "rodas_h0": rodas["initial_step"] if rodas else probe["initial_step"],
            "raw_probe": probe, "raw_peer_dopri": dopri, "raw_peer_rodas": rodas,
            "interval_kind": "extended-window counterfactual" if extend is not None
            else ("entry-pilot prefix" if probe["method"] == "rodas5p" else "return recheck window")}


def main():
    p = argparse.ArgumentParser()
    p.add_argument("binary", type=Path)
    p.add_argument("raw", type=Path)
    p.add_argument("output", type=Path)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    manifest = {"binary_sha256": hashlib.sha256(a.binary.read_bytes()).hexdigest(), "runs": []}
    for case, specs in POINTS.items():
        raw_file = a.raw / f"{case}__evidence__adaptive.stdout.json"
        raw = json.loads(raw_file.read_text())["initial_records"]["adaptive"]
        segs = raw["stats"]["segments"]
        points = [decision_point(segs, i, e) for i, e in specs]
        args = [str(a.binary), case, "adaptive"]
        for q in points:
            args += [repr(q["start"]), repr(q["target"]), repr(q["dopri_h0"]), repr(q["rodas_h0"])]
        r = subprocess.run(args, capture_output=True, text=True, timeout=3600,
                           env={"RAYON_NUM_THREADS": "1", "OMP_NUM_THREADS": "1", "PATH": "/usr/bin:/bin"})
        (a.output / f"{case}.stderr.log").write_text(r.stderr)
        out = json.loads(r.stdout)
        rerun = out["rerun"]
        identical = (rerun["rhs"] == raw["rhs"] and rerun["attempts"] == raw["attempts"]
                     and rerun["final_state"] == raw["state"]
                     and json.dumps(rerun["segments"], sort_keys=True) == json.dumps(segs, sort_keys=True))
        out["raw_input"] = {"file": str(raw_file), "sha256": hashlib.sha256(raw_file.read_bytes()).hexdigest(),
                            "rerun_bit_identical_to_raw_record": identical}
        for q, pt in zip(points, out["points"]):
            pt["selection"] = {k: q[k] for k in ("probe_index", "interval_kind", "dopri_h0", "rodas_h0")}
            pt["raw_probe"] = q["raw_probe"]
            pt["raw_peer_dopri"] = q["raw_peer_dopri"]
            pt["raw_peer_rodas"] = q["raw_peer_rodas"]
        (a.output / f"{case}.json").write_text(json.dumps(out, indent=1) + "\n")
        manifest["runs"].append({"case": case, "exit": r.returncode, "rerun_identical": identical})
        print(case, r.returncode, "identical" if identical else "DIFFERS", flush=True)
    (a.output / "RUN.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
