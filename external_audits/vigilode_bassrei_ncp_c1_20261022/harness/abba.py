#!/usr/bin/env python3
"""Phase A: accuracy-matched paired cost blocks (diagnostic_only).

    python3 -s abba.py CAMP [--warmup 2] [--confirm 7]

From the ladder (analyze.Evaluator): per main case, A = the VigilODE arm with
the lowest solve time at its matched rung (loosest rung meeting B_app, and
B_src for chemistry); for each comparator family (VODE, DOP853, Julia,
diffsol) B = its fastest matched arm. Each pair runs on its own pinned core
of NCPC1_ABBA_CORES (default 8-59): block k runs A then B if k is even, B
then A otherwise; blocks 0..warmup-1 are warm-up. A Julia B uses a Julia
server on the pair's core (its compilation is excluded by the server's
per-solver warm-up). The per-run in-process solve time is compared; rows go
to results/ABBA.jsonl. Pairs or blocks not started before the deadline get
NOT_RUN rows. No speed claim: STATISTICAL_AUTHORITY_HOLD.
"""
import argparse
import os
import queue
import sys
import threading
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import scenarios as S  # noqa: E402
from analyze import Evaluator, ladder_table, matched  # noqa: E402
from common import Deadline, append_jsonl, install_signal_cleanup, utc_now, write_json  # noqa: E402
import runners as RN  # noqa: E402
from ladder import parse_cores  # noqa: E402


def select_pairs(m):
    pairs = []
    for cid in [c for c in S.CASE if not S.CASE[c]["auxiliary"]]:
        arms = m.get(cid) or {}
        vg = [(a, v) for a, v in arms.items() if a in S.VIGILODE_ARMS and v.get("solve_seconds") is not None]
        if not vg:
            continue
        a_arm, a_v = min(vg, key=lambda kv: (kv[1]["solve_seconds"], kv[0]))
        for fam in ("vode", "dop853", "julia", "diffsol"):
            cand = [(a, v) for a, v in arms.items() if S.FAMILY_OF_ARM.get(a) == fam
                    and v.get("solve_seconds") is not None]
            if not cand:
                continue
            b_arm, b_v = min(cand, key=lambda kv: (kv[1]["solve_seconds"], kv[0]))
            # budget-matched (both meet B_app at their loosest passing rung), not error-matched: the two
            # errors and their ratio are recorded; a match at the loosest ladder rung is censored
            pairs.append({"pair": f"{cid}:{a_arm}@{a_v['rtol']:.0e}~{b_arm}@{b_v['rtol']:.0e}", "case": cid,
                          "family": fam,
                          "A": {"arm": a_arm, "rtol": a_v["rtol"], "error": a_v["error"],
                                "censored_at_loose_edge": a_v["rtol"] == max(S.LADDER)},
                          "B": {"arm": b_arm, "rtol": b_v["rtol"], "error": b_v["error"],
                                "censored_at_loose_edge": b_v["rtol"] == max(S.LADDER)},
                          "error_ratio_B_over_A": (b_v["error"] / a_v["error"]) if a_v["error"] else None})
    return pairs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("camp")
    ap.add_argument("--warmup", type=int, default=2)
    ap.add_argument("--confirm", type=int, default=7)
    ap.add_argument("--margin", type=float, default=30.0)
    ap.add_argument("--max-run-seconds", type=float, default=120.0,
                    help="pairs whose matched ladder run took longer are skipped (NOT_RUN): 9 blocks would not fit")
    a = ap.parse_args()
    install_signal_cleanup()
    camp = RN.Camp(a.camp)
    os.environ["NCPC1_RESULTS"] = str(camp.results)
    E = Evaluator(camp.root)
    m = matched(ladder_table(E))
    pairs, skipped = [], []
    for p in select_pairs(m):
        slow = max(m[p["case"]][p["A"]["arm"]]["solve_seconds"], m[p["case"]][p["B"]["arm"]]["solve_seconds"])
        (skipped if slow > a.max_run_seconds else pairs).append(dict(p, matched_run_seconds=slow))
    write_json(camp.results / "ABBA_PLAN.json", {"generated_utc": utc_now(), "pairs": pairs, "skipped": skipped,
                                                  "max_run_seconds": a.max_run_seconds,
                                                  "warmup": a.warmup, "confirm": a.confirm})
    out = camp.results / "ABBA.jsonl"
    for p in skipped:
        append_jsonl(out, {"run_id": f"abba-skip-{p['pair']}", "phase": "abba", "pair": p["pair"], "case": p["case"],
                           "arm": p["B"]["arm"], "status": "NOT_RUN",
                           "reason": f"matched run {p['matched_run_seconds']:.0f} s > {a.max_run_seconds:.0f} s"})
    deadline = Deadline()
    pin = os.environ.get("NCPC1_PIN", "1") == "1"
    cores = parse_cores(os.environ.get("NCPC1_ABBA_CORES", "8-59"))
    lock = threading.Lock()
    q = queue.Queue()
    for k, p in enumerate(pairs):
        q.put((k, p))

    def write(row):
        with lock:
            append_jsonl(out, row)

    def run_pair(core, k, p):
        server = None
        if S.FAMILY_OF_ARM[p["B"]["arm"]] == "julia" and camp.julia:
            try:
                server = RN.JuliaServer(camp, core if pin else None, f"abba_{k}", "abba")
            except Exception:  # noqa: BLE001
                server = None
        for b in range(a.warmup + a.confirm):
            order = ("A", "B") if b % 2 == 0 else ("B", "A")
            for pos, which in enumerate(order):
                arm, rt = p[which]["arm"], p[which]["rtol"]
                base = {"run_id": f"abba-{k:02d}-b{b:02d}-{pos}-{which}-{arm}", "phase": "abba", "case": p["case"],
                        "arm": arm, "rtol": rt, "pair": p["pair"], "block_index": b, "warmup": b < a.warmup,
                        "position": pos, "position_arm": which, "order": "".join(order), "core": core}
                if deadline.left() < a.margin:
                    write(dict(base, status="NOT_RUN", reason="campaign deadline"))
                    continue
                spec = dict(base, mode="timing", core=core if pin else None)
                row = RN.execute(camp, spec, julia_server=server, cap=600.0)
                row.update({k2: base[k2] for k2 in ("pair", "block_index", "warmup", "position", "position_arm",
                                                    "order", "core")})
                write(row)
        if server is not None:
            server.close()

    def worker(core):
        while True:
            try:
                k, p = q.get_nowait()
            except queue.Empty:
                return
            run_pair(core, k, p)

    threads = [threading.Thread(target=worker, args=(c,), daemon=True) for c in cores[:max(len(pairs), 1)]]
    for t in threads:
        t.start()
    for t in threads:
        while t.is_alive():
            t.join(5)
    print(f"abba done {len(pairs)} pairs {utc_now()}", flush=True)


if __name__ == "__main__":
    main()
