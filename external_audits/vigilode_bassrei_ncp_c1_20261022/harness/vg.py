#!/usr/bin/env python3
"""Phase V: VigilODE zero-allocation and NCP cross-core bit-identity (VG-1/VG-3).

    python3 -s vg.py CAMP

Preregistered groups (analyze.py requires exactly these):

- ALLOCATION: one VigilODE arm on one case at rtol 1e-6 and 1e-9 in the
  runner's allocation mode (thread-local counting allocator, positive
  control): equal integrate counts, calibrated counter, attempts differing at
  least 2x.
- REJECTION: the same arm and rtol 1e-6, once with the default initial step
  and once with initial_step = T (forced step rejections): equal counts and
  more rejected steps in the second run (rejections allocate nothing).
- IDENTITY: the same rung-1e-8 request on three distinct cores of
  NCPC1_VG_CORES (campaign 60-63): validated trajectories and equal sha256 of
  times/states and of the counters.

Rows go to results/VG.jsonl; every timeout is clipped to the shared deadline.
"""
import hashlib
import json
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import scenarios as S  # noqa: E402
from common import append_jsonl, install_signal_cleanup, load_stdout_json  # noqa: E402
import runners as RN  # noqa: E402
from ladder import parse_cores  # noqa: E402

ALLOCATION = [("S1-G2", "vg_dense"), ("S1-G2", "vg_banded"), ("S1-G2", "vg_client_exact"),
              ("S1-G2", "vg_client_krylov"), ("S2-G3", "vg_banded"), ("S2-G3", "vg_client_exact"),
              ("S3-G2", "vg_bordered_dense"), ("S3-G2", "vg_bordered_client"), ("S4N1", "vg_chem_dense"),
              ("S4S", "vg_chem_dense")]
REJECTION = [("S1-G2", "vg_dense"), ("S1-G2", "vg_banded"), ("S1-G2", "vg_client_exact"),
             ("S1-G2", "vg_client_krylov"), ("S3-G2", "vg_bordered_dense"), ("S3-G2", "vg_bordered_client")]
# (chemistry: initial_step = T forces no rejection on S4N1 (pilot); its rejections are covered by the
# S4S allocation pair, whose two rungs differ in rejected steps)
IDENTITY = [("S2-G2", "vg_dense"), ("S2-G2", "vg_banded"), ("S2-G2", "vg_client_exact"),
            ("S2-G2", "vg_client_krylov"), ("S1-G3", "vg_banded"), ("S3-G2", "vg_bordered_dense"),
            ("S3-G2", "vg_bordered_client"), ("S4R", "vg_chem_dense")]


def digest(obj):
    return hashlib.sha256(json.dumps(obj, sort_keys=True, allow_nan=True).encode()).hexdigest()


def main():
    install_signal_cleanup()
    camp = RN.Camp(sys.argv[1])
    os.environ["NCPC1_RESULTS"] = str(camp.results)
    out = camp.results / "VG.jsonl"
    pin = os.environ.get("NCPC1_PIN", "1") == "1"
    cores = parse_cores(os.environ.get("NCPC1_VG_CORES", "60-63"))

    def run(spec, group, kind, cap=600.0):
        try:
            row = RN.execute(camp, spec, cap=cap)
        except Exception as e:  # noqa: BLE001
            row = dict(spec, status="ENVIRONMENT_FAILURE", reason=repr(e)[:300])
        row.update(group=group, kind=kind)
        return row

    k = 0
    for cid, arm in ALLOCATION:
        for rt in (1e-6, 1e-9):
            spec = {"run_id": f"vg-alloc-{cid}-{arm}-{rt:.0e}", "phase": "vg", "case": cid, "arm": arm, "rtol": rt,
                    "mode": "allocation", "core": cores[k % len(cores)] if pin else None}
            k += 1
            append_jsonl(out, run(spec, f"{cid}/{arm}", "allocation"))
    for cid, arm in REJECTION:
        T = S.case_data(cid)["T"]
        for j, extra in enumerate(({}, {"initial_step": float(T)})):
            spec = {"run_id": f"vg-reject-{cid}-{arm}-{j}", "phase": "vg", "case": cid, "arm": arm, "rtol": 1e-6,
                    "mode": "allocation", "core": cores[k % len(cores)] if pin else None, "extra": extra}
            k += 1
            row = run(spec, f"{cid}/{arm}", "rejection")
            row["forced_rejections"] = bool(extra)
            append_jsonl(out, row)
    distinct = sorted(set(cores))
    picks = [distinct[0], distinct[len(distinct) // 2], distinct[-1]] if len(distinct) >= 3 else None
    for cid, arm in IDENTITY:
        for j in range(3):
            core = picks[j] if picks else None
            spec = {"run_id": f"vg-ident-{cid}-{arm}-{j}", "phase": "vg", "case": cid, "arm": arm,
                    "rtol": S.RUNG, "mode": "timing", "core": core if pin else None}
            if picks is None:
                row = dict(spec, status="ENVIRONMENT_FAILURE", reason="fewer than 3 distinct VG cores")
            else:
                row = run(spec, f"{cid}/{arm}", "identity")
                doc, _ = load_stdout_json({"stdout_path": row.get("stdout_path", "/nonexistent")})
                if doc is not None:
                    d = S.case_data(cid)
                    row["trajectory_refusal"] = S.validate_trajectory(doc.get("times"), doc.get("states"),
                                                                      d["outputs"], d["n"])
                    row["states_sha256"] = digest({"times": doc.get("times"), "states": doc.get("states")})
                    row["counters_sha256"] = digest(doc.get("counters"))
            row.update(group=f"{cid}/{arm}", kind="identity", core=core)
            append_jsonl(out, row)
    print("vg done", flush=True)


if __name__ == "__main__":
    main()
