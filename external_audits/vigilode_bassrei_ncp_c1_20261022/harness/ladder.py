#!/usr/bin/env python3
"""Phases R + L: references, then work-precision ladders.

    python3 -s ladder.py CAMP [--cases a,b] [--rungs 1e-4,1e-8] [--plan-only]

Writes results/PLAN.json (every planned item, the a-priori NOT_APPLICABLE
arms included, and any dry-run override, which makes the run unofficial) and
appends one row per item to results/LADDER.jsonl (fsync per row; rows never
change once written).

Order: the references (VigilODE 1e-12 and Radau 1e-12; cap 1800 s) run first
on all workers, with nothing else running; then the ladder: VigilODE items
first, then the long-running comparator classes, then the rest. Workers are
threads, each owning one core of NCPC1_CORES (campaign: 8-59; cores 0-7 stay
free), and pin their children to it. Each Julia (case, solver) runs its rungs,
loosest first, in one pinned Julia server (solver.jl --server), at most
NCPC1_JULIA_SERVERS (campaign: 16, about 1.5 GB each) at a time, largest
cases first. Every run's timeout is clipped to the shared deadline
(runners.execute): items that cannot start are NOT_RUN, runs cut at a
deadline-capped timeout are DEADLINE.
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
from common import Deadline, append_jsonl, install_signal_cleanup, utc_now, write_json  # noqa: E402
import runners as RN  # noqa: E402

REF_CAP_S = 1800.0
OVERRIDES = ("NCPC1_CASES", "NCPC1_RUNGS", "NCPC1_LADDER_CORES", "NCPC1_VG_CORESET", "NCPC1_PARITY_CORESET",
             "NCPC1_WATCHDOG_S", "NCPC1_ALLOWED_CPUS", "NCPC1_JULIA_SERVERS_OVERRIDE", "NCPC1_CAP_MIN")


def parse_cores(spec):
    out = []
    for part in spec.split(","):
        if "-" in part:
            a, b = part.split("-")
            out += list(range(int(a), int(b) + 1))
        elif part:
            out.append(int(part))
    return out


def build_plan(cases, rungs=None):
    items = []
    for cid in cases:
        ref_arm = S.reference_arm(cid)
        items.append({"phase": "ref", "case": cid, "arm": ref_arm, "rtol": S.REF_RTOL, "priority": 0})
        items.append({"phase": "ref", "case": cid, "arm": "py_radau_ref", "rtol": S.REF_RTOL, "priority": 0})
        d = S.case_data(cid)
        for a in S.arms_for(cid):
            if a["not_applicable"] and not a["rungs"]:
                items.append({"phase": "ladder", "case": cid, "arm": a["arm"], "rtol": None,
                              "not_applicable": a["not_applicable"], "priority": 9})
                continue
            for r in a["rungs"]:
                if rungs is not None and r not in rungs and a["arm"] != "py_dop853_tight":
                    continue
                fam = S.FAMILY_OF_ARM[a["arm"]]
                if fam == "vigilode":
                    pr = 1
                elif fam in ("vode", "dop853") and (d["n"] > 1000 or d["stiff"]):
                    pr = 2
                else:
                    pr = 3
                items.append({"phase": "ladder", "case": cid, "arm": a["arm"], "rtol": r, "priority": pr})
    for k, it in enumerate(items):
        rt = "na" if it["rtol"] is None else f"{it['rtol']:.0e}"
        it["run_id"] = f"{it['phase']}-{it['case']}-{it['arm']}-{rt}"
        it["index"] = k
    return items


def expected_plan():
    """The preregistered plan: every case, every rung."""
    return build_plan([c[0] for c in S.CASES], None)


def cap_for(it):
    fam = S.FAMILY_OF_ARM.get(it["arm"], "vode")
    d = S.case_data(it["case"])
    if it["phase"] == "ref":
        return REF_CAP_S
    if fam == "dop853" and d["stiff"]:
        return 300.0
    if fam == "vigilode":
        return 600.0
    return 900.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("camp")
    ap.add_argument("--cases", default=None)
    ap.add_argument("--rungs", default=None)
    ap.add_argument("--plan-only", action="store_true")
    a = ap.parse_args()
    install_signal_cleanup()
    camp = RN.Camp(a.camp)
    os.environ["NCPC1_RESULTS"] = str(camp.results)
    cases = a.cases.split(",") if a.cases else [c[0] for c in S.CASES]
    rungs = [float(x) for x in a.rungs.split(",")] if a.rungs else None
    items = build_plan(cases, rungs)
    overrides = {k: os.environ[k] for k in OVERRIDES if os.environ.get(k)}
    if a.cases or a.rungs:
        overrides["argv"] = " ".join(sys.argv[1:])
    official = not overrides and items == expected_plan()
    write_json(camp.results / "PLAN.json", {"generated_utc": utc_now(), "cases": cases, "ladder": S.LADDER,
                                             "rung": S.RUNG, "items": items, "overrides": overrides,
                                             "official": official})
    if a.plan_only:
        return
    out = camp.results / "LADDER.jsonl"
    pin = os.environ.get("NCPC1_PIN", "1") == "1"
    cores = parse_cores(os.environ.get("NCPC1_CORES", "8-59"))
    max_julia = int(os.environ.get("NCPC1_JULIA_SERVERS", "16"))
    lock = threading.Lock()

    def write(it, row):
        row = dict(row)
        row.setdefault("phase", it["phase"])
        row["plan_index"] = it["index"]
        row.setdefault("timing_authority", "diagnostic_only")
        with lock:
            append_jsonl(out, row)

    def typed(it, status, why):
        write(it, {"run_id": it["run_id"], "case": it["case"], "arm": it["arm"], "rtol": it["rtol"],
                   "status": status, "reason": why, "family": S.FAMILY_OF_ARM[it["arm"]]})

    for it in items:
        if it.get("not_applicable"):
            typed(it, "NOT_APPLICABLE", it["not_applicable"])

    def drain(q, core):
        while True:
            try:
                it = q.get_nowait()
            except queue.Empty:
                return
            spec = dict(it, core=core if pin else None, mode="timing")
            try:
                row = RN.execute(camp, spec, cap=cap_for(it))
            except Exception as e:  # noqa: BLE001
                row = {"run_id": it["run_id"], "case": it["case"], "arm": it["arm"], "rtol": it["rtol"],
                       "status": "ENVIRONMENT_FAILURE", "reason": repr(e)[:300]}
            write(it, row)

    def run_threads(target, args_list):
        ts = [threading.Thread(target=target, args=args, daemon=True) for args in args_list]
        for t in ts:
            t.start()
        for t in ts:
            while t.is_alive():
                t.join(5)

    # --- references first, alone ----------------------------------------------------------------
    rq = queue.Queue()
    for it in sorted([it for it in items if it["phase"] == "ref"], key=lambda it: -S.case_data(it["case"])["n"]):
        rq.put(it)
    run_threads(drain, [(rq, c) for c in cores])
    print(f"references done {utc_now()}", flush=True)

    # --- ladder ----------------------------------------------------------------------------------
    work = [it for it in items if it["phase"] == "ladder" and not it.get("not_applicable")]
    julia_items, other = {}, []
    for it in work:
        if S.FAMILY_OF_ARM[it["arm"]] == "julia":
            julia_items.setdefault((it["case"], it["arm"]), []).append(it)
        else:
            other.append(it)
    other.sort(key=lambda it: (it["priority"], -S.case_data(it["case"])["n"]))
    q = queue.Queue()
    for it in other:
        q.put(it)
    jq = queue.Queue()
    for key in sorted(julia_items, key=lambda k: (-S.case_data(k[0])["n"], k[1])):
        jq.put(key)
    julia_ready = bool(camp.julia and camp.julia_depot)
    n_julia = min(max_julia, len(julia_items), max(len(cores) * 2 // 5, 1)) if julia_ready else 0
    julia_cores, other_cores = cores[:n_julia], (cores[n_julia:] or cores)

    def julia_worker(core):
        while True:
            try:
                key = jq.get_nowait()
            except queue.Empty:
                return
            cid, jarm = key
            its = sorted(julia_items[key], key=lambda it: -it["rtol"])
            server, ready = None, ""
            try:
                server = RN.JuliaServer(camp, core if pin else None, f"ladder_{cid}_{jarm}", "ladder")
                ready = server.ready_line if server.alive else \
                    f"no READY line ({server.ready_line!r}); stderr tail: {server.stderr_tail(400)}"
            except Exception as e:  # noqa: BLE001
                ready = f"start failed: {e!r}"
            for it in its:
                if server is None or not server.alive:
                    if Deadline().left() < RN.RESERVE_S + RN.MIN_RUN_S:
                        typed(it, "NOT_RUN", "campaign deadline")
                    else:
                        typed(it, "HOLD", f"Julia server unavailable: {ready}"[:700])
                    continue
                row = RN.execute(camp, dict(it, core=core, mode="timing"), julia_server=server, cap=cap_for(it))
                row["julia_ready_seconds"] = server.ready_seconds
                write(it, row)
                if row.get("status") in ("TIMEOUT", "DEADLINE") and not server.alive:
                    if Deadline().left() > 300:   # restart only when a restart can still be useful
                        try:
                            server = RN.JuliaServer(camp, core if pin else None, f"ladder_{cid}_{jarm}_r", "ladder")
                        except Exception:  # noqa: BLE001
                            server = None
                    else:
                        server = None
            if server is not None:
                server.close()

    if not julia_ready:
        for (cid, _), its in julia_items.items():
            for it in its:
                typed(it, "HOLD", "Julia not available (setup HOLD: no binary or no private depot)")
    threads = [threading.Thread(target=julia_worker, args=(c,), daemon=True) for c in julia_cores]
    threads += [threading.Thread(target=drain, args=(q, c), daemon=True) for c in other_cores]
    for t in threads:
        t.start()
    for t in threads:
        while t.is_alive():
            t.join(5)
    print(f"ladder done {utc_now()}", flush=True)


if __name__ == "__main__":
    main()
