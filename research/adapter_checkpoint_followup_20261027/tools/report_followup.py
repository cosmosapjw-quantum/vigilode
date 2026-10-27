#!/usr/bin/env python3
"""Render RESULTS.json as a compact Markdown summary (no new numbers)."""
import json
import sys


def main():
    r = json.load(open(sys.argv[1]))
    out = [f"Verdict: **{r['verdict']}** (timing `{r['timing_authority']}`)", ""]
    g1 = r["G1_identity"]
    out.append(f"- G1 identity: {g1['records_compared']} records, differences {len(g1['differences'])} → {g1['pass']}")
    for row in r["G2_allocations"]["rows"]:
        out.append(f"- G2 {row['case']}: allocator events per attempt {row['base']['marginal_per_attempt']:.2f} → "
                   f"{row['candidate']['marginal_per_attempt']:.2f} ({row['candidate_over_base_marginal']:.3f}x)")
    g3 = r["G3_sentinel_stale_return"]
    out.append(f"- G3 stale-return work {g3['adaptive']['work_units']:.0f} → {g3['sentinel']['work_units']:.0f}, "
               f"RHS {g3['adaptive']['rhs']} → {g3['sentinel']['rhs']}, error {g3['adaptive']['error']:.2e} / "
               f"{g3['sentinel']['error']:.2e} → {g3['pass']}")
    for row in r["G4_sentinel_accuracy"]["rows"]:
        out.append(f"- G4 {row['case']}: sentinel/adaptive work {row['sentinel_over_adaptive_work']:.3f} "
                   f"(RHS {row['adaptive_rhs']} → {row['sentinel_rhs']}), error {row['sentinel_error']:.2e}, "
                   f"pass {row['sentinel_accuracy_pass']}")
    for k, v in r["G5_tests"].items():
        if k != "pass":
            out.append(f"- G5 {k}: {v['suites']} suites, {v['passed']} passed, {v['failed']} failed, {v['ignored']} ignored")
    out += ["", "| case | a | b | b/a time | range | class |", "|---|---|---|---:|---|---|"]
    for p in r["pairs"]:
        lo, hi = p["paired_ratio_range"]
        out.append(f"| {p['case']} | {p['a']} | {p['b']} | {p['b_over_a_seconds']:.3f} | {lo:.3f}–{hi:.3f} | {p['classification']} |")
    out += ["", "| cross-binary cell | candidate/base | identical records |", "|---|---:|---|"]
    for c in r["cross_binary"]["rows"]:
        out.append(f"| {c['case']} {c['arm']} | {c['candidate_over_base_median']:.3f} | "
                   f"{c['records_identical_after_krylov_id_normalisation']} |")
    print("\n".join(out))


if __name__ == "__main__":
    main()
