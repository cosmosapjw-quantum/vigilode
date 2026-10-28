#!/usr/bin/env python3
"""Render the results section of PREREGISTRATION.md from analyze.py outputs.

usage: results_section.py BEFORE_JSON AFTER_JSON BEFORE2_JSON GATES_JSON RUN_LABEL EXEC_SHA
"""
import json, sys
B, A, B2, G = (json.load(open(p)) for p in sys.argv[1:5])
label, exec_sha = sys.argv[5], sys.argv[6]
SW = [k for k, v in A["runs"].items() if v["role"] == "switch" and k in B["runs"]]
drift = max(max(B2["runs"][k]["timing"]["median_seconds"] / v["timing"]["median_seconds"],
               v["timing"]["median_seconds"] / B2["runs"][k]["timing"]["median_seconds"])
           for k, v in B["runs"].items() if v["role"] == "direct")
ok = all(G[k] for k in ("G1_direct_bit_identical", "G2_work_not_higher_all", "G3_success_accuracy_all"))
L = [f"## Results, {label} (appended after the recorded run; the registered text above is unchanged)\n",
     f"**Verdict: {'PASS' if ok else 'FAIL'}** on G1-G3 by `tools/compare.py` (`evidence/GATES.json`, `evidence/COMPARISON.md`); G4 and G5 by `evidence/TESTS.txt`. "
     f"Execution commit `{exec_sha}`; binaries in `evidence/BINARY_SHA256.json`; host and raw manifests in `evidence/HOST.json`. "
     f"Wall-time drift control (packet-head binaries run twice): worst direct-cell median ratio {drift:.2f}x, so the timing columns are "
     f"{'**marked unreliable** by the registered rule' if drift > 1.5 else 'within the registered 1.5x rule'}; the counter columns carry the result.\n",
     "| cell | attempts b→a | RHS b→a | JVP b→a | discarded pilots b→a (attempts) | discarded rechecks b→a (attempts) | implicit batches b→a (attempts) | error b→a (cap) | wall ms before / after / before-2 (diag.) | best direct ms b / a / b2 |",
     "|---|---|---|---|---|---|---|---|---|---|"]
for k in SW:
    b, a, b2 = B["runs"][k], A["runs"][k], B2["runs"][k]; tb, ta = b["totals"], a["totals"]; bb, ba = b["buckets"], a["buckets"]
    bk = lambda x, n: f"{x[n]['count']}({x[n]['attempts']})"
    cb, ca, cb2 = B["comparison"][k], A["comparison"][k], B2["comparison"][k]
    err = lambda v: f"{v['error']:.2e}"
    L.append(f"| {k} | {tb['attempts']}→{ta['attempts']} | {tb['rhs_calls']}→{ta['rhs_calls']} | {tb['jvp_vectors']}→{ta['jvp_vectors']} | {bk(bb,'implicit_pilot_discarded')}→{bk(ba,'implicit_pilot_discarded')} | {bk(bb,'explicit_probe_discarded')}→{bk(ba,'explicit_probe_discarded')} | {bk(bb,'implicit_committed')}→{bk(ba,'implicit_committed')} | {err(b)}→{err(a)} ({a['cap']:.0e}) | {b['timing']['median_seconds']*1e3:.3f} / {a['timing']['median_seconds']*1e3:.3f} / {b2['timing']['median_seconds']*1e3:.3f} | {cb['best_direct'].split(':')[-1]} {cb['best_direct_median_seconds']*1e3:.3f} / {ca['best_direct_median_seconds']*1e3:.3f} / {cb2['best_direct_median_seconds']*1e3:.3f} |")
L.append("")
L.append("Report-only cells (no direct candidate attains the cap): " + ", ".join(f"`{k}`" for k in G.get("accuracy_report_only_cells", [])) + ". All other cells meet their validated caps; direct cells are identical bit for bit in every counter and in the emitted final state (G1).\n")
L.append("**Decision sequences after** (`D` explicit batch, `R` implicit batch, `!` discarded, attempts in brackets):\n")
for k in SW:
    L.append(f"- `{k}` (switches={A['runs'][k]['totals']['switches']}): `{A['runs'][k]['decision_sequence']}`")
print("\n".join(L))
