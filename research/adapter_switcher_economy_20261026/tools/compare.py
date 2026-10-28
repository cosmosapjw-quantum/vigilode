#!/usr/bin/env python3
"""Before/after comparison of two analyze.py outputs; gates on deterministic counters only.

usage: compare.py BEFORE_JSON AFTER_JSON OUT_MD OUT_GATES_JSON
"""
import json, sys

B = json.load(open(sys.argv[1])); A = json.load(open(sys.argv[2]))
SW = [k for k, v in A["runs"].items() if v["role"] == "switch" and k in B["runs"]]
DI = [k for k, v in A["runs"].items() if v["role"] == "direct" and k in B["runs"]]
gates = {}
L = ["# Before/after: adapter work (authority) and wall time (diagnostic only)\n",
     f"before: commit `{B['provenance']['source_commit']}` binaries `{B['provenance']['binaries']}`\n",
     f"after: commit `{A['provenance']['source_commit']}` binaries `{A['provenance']['binaries']}`\n",
     "## G1 — untouched direct paths are bit-identical (counters and final state)\n",
     "| direct cell | attempts | RHS | JVP | solves | final state bitwise | wall ms before→after (drift) |", "|---|---|---|---|---|---|---|"]
g1 = True
for k in DI:
    b, a = B["runs"][k], A["runs"][k]
    same = all(b["stats"][f] == a["stats"][f] for f in b["stats"]) and b["success"] == a["success"]
    state = (b["final_state"] is not None and a["final_state"] is not None and
             all(float(x).hex() == float(y).hex() for x, y in zip(b["final_state"], a["final_state"])) and len(b["final_state"]) == len(a["final_state"]))
    if b["final_state"] is None or a["final_state"] is None:
        state_txt = "n/a (not emitted)"
    else:
        state_txt = "yes" if state else "NO"; same &= state
    g1 &= same
    L.append(f"| {k} | {b['stats']['attempts']}/{a['stats']['attempts']} | {b['stats']['rhs_calls']}/{a['stats']['rhs_calls']} | {b['stats']['jvp_vectors']}/{a['stats']['jvp_vectors']} | {b['stats']['linear_solves']}/{a['stats']['linear_solves']} | {state_txt} | {b['timing']['median_seconds']*1e3:.3f}→{a['timing']['median_seconds']*1e3:.3f} ({a['timing']['median_seconds']/b['timing']['median_seconds']:.2f}x) |")
gates["G1_direct_bit_identical"] = g1
L += [f"\n**G1 = {g1}**\n", "## G2/G3 — switching cells: deterministic work never higher, accuracy caps kept\n",
      "| cell | attempts b→a | RHS b→a | pilots b→a (attempts) | probes b→a (attempts) | RODAS batches b→a (attempts) | error b→a (cap) | success/pass | wall ms b→a | best direct ms b→a | ratio b→a |",
      "|---|---|---|---|---|---|---|---|---|---|---|"]
g2 = True; g3 = True
for k in SW:
    b, a = B["runs"][k], A["runs"][k]
    tb, ta, bb, ba = b["totals"], a["totals"], b["buckets"], a["buckets"]
    cb, ca = B["comparison"][k], A["comparison"][k]
    if cb.get("best_direct") is None or ca.get("best_direct") is None:
        L.append(f"| {k} | no completed direct candidate | | | | | | | | | |"); continue
    ok2 = ta["attempts"] <= tb["attempts"] and ta["rhs_calls"] <= tb["rhs_calls"] and ba["implicit_pilot_discarded"]["attempts"] <= bb["implicit_pilot_discarded"]["attempts"] and ba["explicit_probe_discarded"]["attempts"] <= bb["explicit_probe_discarded"]["attempts"] and ba["implicit_committed"]["count"] <= bb["implicit_committed"]["count"]
    # The cap is gated only where it is attainable: a validated error and at least one direct
    # candidate of the same cell that passes the cap (the registered default-tolerance gyro cell has
    # none and is reported only).
    attainable = a["error"] is not None and any(dv["accuracy_pass"] for dk, dv in A["runs"].items() if dv["role"] == "direct" and dv["kind"] == a["kind"] and dv["case"] == a["case"])
    ok3 = a["success"] and (a["accuracy_pass"] if attainable else True) and a["reconciled"] and a["determinism"]["identical_across_runs"]
    gates.setdefault("accuracy_gated_cells" if attainable else "accuracy_report_only_cells", []).append(k)
    g2 &= ok2; g3 &= ok3
    gates[k] = {"work_not_higher": ok2, "success_accuracy_reconciled_deterministic": ok3}
    def bk(x, n): return f"{x[n]['count']}({x[n]['attempts']})"
    err = lambda v: "n/a" if v.get("error") is None else f"{v['error']:.2e}"
    L.append(f"| {k} | {tb['attempts']}→{ta['attempts']} | {tb['rhs_calls']}→{ta['rhs_calls']} | {bk(bb,'implicit_pilot_discarded')}→{bk(ba,'implicit_pilot_discarded')} | {bk(bb,'explicit_probe_discarded')}→{bk(ba,'explicit_probe_discarded')} | {bk(bb,'implicit_committed')}→{bk(ba,'implicit_committed')} | {err(b)}→{err(a)} ({a['cap']:.0e}) | {a['success']}/{a['accuracy_pass']} | {cb['switch_median_seconds']*1e3:.3f}→{ca['switch_median_seconds']*1e3:.3f} | {cb['best_direct_median_seconds']*1e3:.3f}→{ca['best_direct_median_seconds']*1e3:.3f} | {cb['time_ratio_median']:.2f}→{ca['time_ratio_median']:.2f} |")
gates["G2_work_not_higher_all"] = g2; gates["G3_success_accuracy_all"] = g3
L += [f"\n**G2 = {g2}, G3 = {g3}**\n", "## Decision sequences (after)\n"]
for k in SW:
    L.append(f"- **{k}** (switches={A['runs'][k]['totals']['switches']}): `{A['runs'][k]['decision_sequence']}`")
L += ["\n## Timed samples (ms, diagnostic only)\n"]
for k in SW + DI:
    L.append(f"- {k}: before {', '.join(f'{x*1e3:.3f}' for x in B['runs'][k]['timing']['timed_seconds'])}; after {', '.join(f'{x*1e3:.3f}' for x in A['runs'][k]['timing']['timed_seconds'])}")
open(sys.argv[3], "w").write("\n".join(L) + "\n")
json.dump(gates, open(sys.argv[4], "w"), indent=1)
print("\n".join(L[:40])); print("gates:", json.dumps({k: v for k, v in gates.items() if k.startswith("G")}))
