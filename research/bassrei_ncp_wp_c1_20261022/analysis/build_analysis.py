#!/usr/bin/env python3
"""Build the external-review summary of the BASS/rei NCP C1 benchmark.

    python3 research/bassrei_ncp_wp_c1_20261022/analysis/build_analysis.py

Reads only the imported, ledger-bound results (attempt2/, attempt3/, julia_install/;
ledger rows L-0154 and L-0155) and writes, next to this script:

- benchmark_summary.json: verdicts, gates, coverage, matched rungs per case and
  arm, the AB/BA confirmation ratios, and an index of every result file with its
  sha256 and size;
- BENCHMARK_ANALYSIS.md: the analysis document; every number in its tables is
  generated from the same data.

It re-scores nothing and changes no imported file. Timing is diagnostic_only
(STATISTICAL_AUTHORITY_HOLD); no speed claim is made.
"""
from __future__ import annotations

import hashlib
import json
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
NODE = HERE.parent
REPO = NODE.parents[1]
ATTEMPTS = {"attempt2": "L-0154", "attempt3": "L-0155"}
CASE_INFO = {
    "S1-G1": ("P02 photon transport, Γ = 0.5 (auxiliary 16×8)", 146, "nonstiff", "H"),
    "S1-G2": ("P02 photon transport, Γ = 0.5, 32×16", 546, "nonstiff", "C"),
    "S1-G3": ("P02 photon transport, Γ = 0.5, 64×32", 2114, "nonstiff", "C"),
    "S2-G1": ("P02 photon transport, Γ = 1e4 (auxiliary 16×8)", 146, "stiff", "H"),
    "S2-G2": ("P02 photon transport, Γ = 1e4, 32×16", 546, "stiff", "C"),
    "S2-G3": ("P02 photon transport, Γ = 1e4, 64×32", 2114, "stiff", "C"),
    "S2-G4": ("P02 photon transport, Γ = 1e4, 128×64", 8322, "stiff", "C"),
    "S3-G1": ("P03 bordered model (auxiliary 16×8)", 149, "stiff", "H"),
    "S3-G2": ("P03 bordered model: dynamic shear, Riccati x_e, matter stress, 32×16", 549, "stiff", "C"),
    "S4N1": ("P04 photon–gas chemistry, scenario N1", 446, "nonstiff", "C"),
    "S4S": ("P04 photon–gas chemistry, scenario S", 446, "stiff", "C"),
    "S4R": ("P04 photon–gas chemistry, scenario R (recombination)", 446, "stiff", "C"),
}
FAMILY = {"vg": "VigilODE", "py": "scipy", "jl": "Julia", "ds": "diffsol"}
HEAVY = {
    "attempt2": ("~/vigilode-campaign/bassrei-ncp-c1-20261022T063627Z/ncp/OUT.tar.gz", 75490789),
    "attempt3": ("~/vigilode-campaign/bassrei-ncp-c1-20261022T070004Z/ncp/OUT.tar.gz", 120661665),
}


def sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def load(p: Path):
    return json.loads(p.read_text())


def g(x: float) -> str:
    return f"{x:.3g}"


def family(arm: str) -> str:
    return FAMILY.get(arm.split("_")[0], arm.split("_")[0])


def attempt_summary(name: str) -> dict:
    d = NODE / name
    res = load(d / "results.json")
    camp = d / "campaign"
    gates = res["gates"]
    status = Counter()
    by_family = Counter()
    for line in (camp / "LADDER.jsonl").read_text().splitlines():
        row = json.loads(line)
        status[row["status"]] += 1
        by_family[f"{row.get('family')}:{row['status']}"] += 1
    g3 = {k: {"error": v["error"], "margin": v["margin_B_app_over_error"]}
          for k, v in gates["G3_vigilode_rung"]["values"].items() if isinstance(v, dict) and "error" in v}
    parity = {k: v["parity"] for k, v in gates["G2_reference_parity"]["values"].items()
              if isinstance(v, dict) and "parity" in v}
    matched = load(camp / "MATCHED.json")["matched"]
    mt = {case: {arm: {"rtol": e["rtol"], "error": e["error"], "solve_seconds": e["solve_seconds"],
                       "censored_at_loose_edge": e.get("censored_at_loose_edge", False)}
                 for arm, e in arms.items()} for case, arms in matched.items()}
    abba = {}
    for key, e in load(camp / "ABBA_SUMMARY.json").items():
        abba[key] = {k: e.get(k) for k in ("median_B_over_A", "min", "max", "confirmation_blocks_ok",
                                           "confirmation_rows_by_status", "complete", "error_ratio_B_over_A",
                                           "skipped_reason")}
        abba[key]["A"], abba[key]["B"] = e.get("A"), e.get("B")
    manifest = load(d / "IMPORT_MANIFEST.json")
    return {
        "ledger": ATTEMPTS[name], "verdict": res["verdict"],
        "gate_table": res.get("gate_table") or {k: bool(v.get("pass")) for k, v in gates.items()},
        "infrastructure_issues": res.get("infrastructure_issues", []),
        "coverage": dict(status), "coverage_by_family": dict(sorted(by_family.items())),
        "G2_parity_max": max(parity.values()), "G2_parity": parity,
        "G3": g3, "G3_max_error": max(v["error"] for v in g3.values()),
        "G3_min_margin": min(v["margin"] for v in g3.values()),
        "G4_groups": len(gates["G4_cross_core_identity"]["values"]),
        "G5_groups": len(gates["G5_allocation"]["values"]),
        "matched": mt, "abba": abba,
        "raw_tarball_sha256": manifest["tarball_sha256"],
        "raw_tarball_not_uploaded": {"path": HEAVY[name][0], "bytes": HEAVY[name][1]},
    }


def file_index() -> list[dict]:
    rows = []
    for sub in ("attempt2", "attempt3", "julia_install", "attempts"):
        for p in sorted((NODE / sub).rglob("*")):
            if p.is_file():
                rows.append({"path": str(p.relative_to(REPO)), "bytes": p.stat().st_size, "sha256": sha(p)})
    for name in ("PREREGISTRATION.md", "DAG_STATUS.json"):
        p = NODE / name
        rows.append({"path": str(p.relative_to(REPO)), "bytes": p.stat().st_size, "sha256": sha(p)})
    return rows


def md(summary: dict) -> str:
    a2, a3 = summary["attempts"]["attempt2"], summary["attempts"]["attempt3"]
    L = []
    w = L.append
    w("# BASS/rei NCP C1: accuracy-matched work-precision benchmark — analysis for external review")
    w("")
    w("This document is generated by `analysis/build_analysis.py` from the imported, ledger-bound results of")
    w("the campaign (ledger rows **L-0154**, attempt 2, and **L-0155**, attempt 3). The machine-readable data")
    w("are in `analysis/benchmark_summary.json`. The preregistration, with its amendments and the Results")
    w("section, is `PREREGISTRATION.md` in this directory, and it is normative.")
    w("")
    w("## 1. What is and is not claimed")
    w("")
    w("**Claimed [C]:**")
    w("")
    w("- The VigilODE arms reach the application budget B_app = 1e-6 relative at the preregistered rung")
    w("  rtol 1e-8, on every preregistered case, with the source ledgers (B_src) held.")
    w("- Every comparator adapter evaluates the same right-hand side as the VigilODE kernels, to ≤ 3e-15.")
    w("- VigilODE results are bitwise identical across cores.")
    w("- VigilODE allocation does not depend on the step count, including forced rejections.")
    w("- Every preregistered item is accounted for, as OK or as a typed outcome.")
    w("")
    w("**Not claimed:**")
    w("")
    w("- Any speed claim. All timing is `diagnostic_only` under STATISTICAL_AUTHORITY_HOLD: one host, one")
    w("  campaign, no statistical authority. The matched-cost ratios below describe this run only.")
    w("- Production readiness, or a live BASS/rei integration. Both stay HOLD.")
    w("")
    w("## 2. Setup (summary; the preregistration is normative)")
    w("")
    w("**Host and execution.**")
    w("")
    w("- An NCP host with 64 cores. The campaign was pinned to cores 8–63, with cores 0–7 left free.")
    w("- BLAS ran with 1 thread, under a 120-minute hard cap.")
    w("- Rust 1.94.1, locked and offline.")
    w("- scipy 1.11.4, Julia 1.12.7 (pinned Manifest), diffsol 0.17.1.")
    w("")
    w("**Accuracy measure.**")
    w("")
    w("- B_app = 1e-6 relative on {n_γ, ρ_γ, D_γ/ρ_γ (depletion-weighted, case-scaled floor), D_γ, x_HII,")
    w("  u, s/H} at every output time.")
    w("- B_src ledgers: 5e-12 per H for number and 5e-11 eV per H for energy (chemistry cases).")
    w("- References: dense LU at rtol 1e-12, cross-checked with Radau.")
    w("")
    w("**Protocol.**")
    w("")
    w("- Work-precision ladder: rtol 1e-3 … 1e-10.")
    w("- The **matched rung** of an arm is the loosest rtol at which it meets B_app and B_src.")
    w("- Cost pairs: AB/BA blocks, 2 warm-up and 7 confirmation blocks. The ratio is the median of the")
    w("  comparator's solve time over VigilODE's.")
    w("")
    w("| case | model | n | regime | authority |")
    w("|---|---|---|---|---|")
    for c, (desc, n, reg, auth) in CASE_INFO.items():
        w(f"| {c} | {desc} | {n} | {reg} | {'[C]' if auth == 'C' else '[H] auxiliary'} |")
    w("")
    w("**Arms.** These are the preregistration's \"Arms\" table:")
    w("")
    w("- **VigilODE:** dense, banded, structured client exact/Krylov (P03), bordered (S3), chemistry dense (S4).")
    w("- **scipy:** VODE Adams MF10 and BDF MF21/22/24/25; DOP853 and a tight DOP853.")
    w("- **Julia:** Rodas5P and FBDF with an exact sparse Jacobian; CVODE_BDF Band and KLU; Tsit5 and Vern7.")
    w("- **diffsol:** BDF dense and Tsit45.")
    w("")
    w("## 3. Verdicts")
    w("")
    w("| attempt | ledger | verdict | G1 | G2 | G3 | G4 | G5 | G6 | G7 | OK | HOLD | TIMEOUT | n.a. |")
    w("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for name, a in (("attempt2", a2), ("attempt3", a3)):
        gt = a["gate_table"]
        cells = " | ".join("✓" if gt[k] else "✗" for k in sorted(gt))
        cov = a["coverage"]
        w(f"| {name} | {a['ledger']} | **{a['verdict']}** | {cells} | {cov.get('OK', 0)} | {cov.get('HOLD', 0)} | "
          f"{cov.get('TIMEOUT', 0)} | {cov.get('NOT_APPLICABLE', 0)} |")
    w("")
    w("**Attempt 1** ended INVALID in setup after 9 s: `cargo` was not on the detached shell's PATH. No")
    w("measurement ran, and there is no ledger row. Its evidence is in `attempts/`, and Amendment 1 records")
    w("it.")
    w("")
    w("**Attempt 2** held every Julia arm (424 HOLD rows), because the only Julia depot on the host lived on")
    w("tmpfs and a power cycle had cleared it.")
    w("")
    w("**Attempt 3** reran the full plan with all arms. The Julia environment was installed online at the")
    w("pinned Manifest, on the user's explicit instruction. This is a recorded deviation from the")
    w("offline-dependency rule (Amendment 2; evidence in `julia_install/`).")
    w("")
    w("Both attempts were scored by the same, unchanged `analyze.py`. Neither verdict was re-scored.")
    w("")
    w("## 4. Accuracy and correctness (attempt 3; attempt 2 is identical for the VigilODE arms)")
    w("")
    w(f"- **G2:** adapter right-hand-side parity against the VigilODE kernels: max {g(a3['G2_parity_max'])}.")
    w(f"- **G3:** {len(a3['G3'])} VigilODE rung items at rtol 1e-8.")
    w(f"  - Max error {g(a3['G3_max_error'])} against B_app 1e-6, so the minimum margin is "
      f"{a3['G3_min_margin']:.0f}×.")
    w("  - The source ledgers hold.")
    w(f"- **G4:** {a3['G4_groups']} groups are bitwise identical on 3 distinct cores.")
    w(f"- **G5:** {a3['G5_groups']} allocation groups, including 6 forced-rejection groups: equal allocations")
    w("  at two rtols.")
    w("")
    w("| VigilODE item (case/arm) | error at rtol 1e-8 | margin B_app / error |")
    w("|---|---|---|")
    for k, v in sorted(a3["G3"].items()):
        w(f"| {k} | {g(v['error'])} | {v['margin']:.3g} |")
    w("")
    w("## 5. Matched cost per case (attempt 3; `diagnostic_only`)")
    w("")
    w("Arms are sorted by solve time at their matched rung. `*` means the arm is censored at the loosest")
    w("rung, rtol 1e-3: it already meets the budget there, so its true matched cost may be lower.")
    w("")
    for case in CASE_INFO:
        cm = a3["matched"].get(case)
        if not cm:
            continue
        w(f"**{case}** ({CASE_INFO[case][0]}, n = {CASE_INFO[case][1]})")
        w("")
        w("| arm | family | matched rtol | error | solve s |")
        w("|---|---|---|---|---|")
        for arm, e in sorted(cm.items(), key=lambda kv: kv[1]["solve_seconds"]):
            star = "*" if e["censored_at_loose_edge"] else ""
            w(f"| {arm} | {family(arm)} | {g(e['rtol'])}{star} | {g(e['error'])} | {g(e['solve_seconds'])} |")
        w("")
    w("## 6. Paired cost confirmations (AB/BA, attempt 3; `diagnostic_only`)")
    w("")
    w("A is the VigilODE arm and B is the comparator, each at its matched rung. The ratio is the median, over")
    w("the 7 confirmation blocks, of B's time divided by A's: a value above 1 means VigilODE was faster in")
    w("this run.")
    w("")
    w("| case | A (VigilODE) | B (comparator) | median B/A | min–max | blocks OK | error ratio B/A | note |")
    w("|---|---|---|---|---|---|---|---|")
    for key, e in sorted(a3["abba"].items()):
        case, pair = key.split(":", 1)
        a_, b_ = pair.split("~")
        if e.get("skipped_reason"):
            w(f"| {case} | {a_} | {b_} | — | — | — | — | skipped: {e['skipped_reason']} |")
            continue
        note = []
        if (e.get("A") or {}).get("censored_at_loose_edge"):
            note.append("A censored")
        if (e.get("B") or {}).get("censored_at_loose_edge"):
            note.append("B censored")
        w(f"| {case} | {a_} | {b_} | {g(e['median_B_over_A'])} | {g(e['min'])}–{g(e['max'])} | "
          f"{e['confirmation_blocks_ok']} | {g(e['error_ratio_B_over_A'])} | {', '.join(note)} |")
    w("")
    w("## 7. Interpretation (provisional; for the reviewer to check)")
    w("")
    w("- **Stiff transport (S2):** VigilODE's structured client solver (P03) is the fastest arm at every grid.")
    w("  Its paired ratios against the strongest comparator, Julia CVODE_BDF Band, grow with the grid. The")
    w("  advantage comes from the exact energy-block solve, O(n), against a banded or sparse factorization.")
    w("- **Same method family:** VigilODE's banded RODAS5P against Julia's Rodas5P with an exact sparse")
    w("  Jacobian. Compare the matched-rung times in §5; they were not paired in AB/BA.")
    w("- **Bordered (S3):** the bordered structured client beats FBDF sparse, CVODE and VODE.")
    w("- **Stiff chemistry (S4S, S4R):** VigilODE dense and Julia CVODE_BDF KLU are within about 1.3× of each")
    w("  other, in both directions.")
    w("- **Non-stiff (S1, S4N1):** explicit Runge–Kutta methods (Julia Tsit5, diffsol Tsit45) are faster than")
    w("  every implicit method, VigilODE included, as expected.")
    w("- **The 2 TIMEOUTs** are DOP853 on the stiff chemistry case S4R. They are typed, with a 300 s cap.")
    w("")
    w("## 8. Threats to validity")
    w("")
    w("1. **Timing authority.** The results come from a single host and a single campaign, with no repeated")
    w("   campaigns or statistics (STATISTICAL_AUTHORITY_HOLD). The ratios can move with the host, the load")
    w("   and the build.")
    w("2. **Censoring.** Most VigilODE arms meet B_app already at the loosest rung (1e-3). The matched ratios")
    w("   are therefore bounded on VigilODE's side, not measured. It also means B_app does not discriminate")
    w("   among the VigilODE arms here; their rtol-1e-8 errors are 1e-11 to 1e-8.")
    w("3. **Jacobian asymmetry.**")
    w("   - VigilODE uses exact, structured Jacobians.")
    w("   - VODE MF22/MF25 and CVODE Band use difference-quotient Jacobians.")
    w("   - Julia's sparse arms use exact AD of a bitwise port.")
    w("   - diffsol uses the adapter's exact JVP or dense Jacobian.")
    w("4. **Problem size.** n ≤ 8322, and some arms are not applicable a priori (dense n ≤ 512 or 2048,")
    w("   diffsol without G4).")
    w("5. **Timing scope.** It differs by family; each row carries its `timing_scope`. Every runner")
    w("   integrates twice per call, a warm-up and then the measured call.")
    w("6. **The Julia environment was installed online** (Amendment 2). Its versions equal the pinned")
    w("   Manifest; the hashes are in `julia_install/ENV_SHA256SUMS.txt`.")
    w("7. **Non-blind design.** Pilots informed parts of the design; all of it is disclosed in the")
    w("   preregistration's design history and deviations.")
    w("")
    w("## 9. Data, provenance and re-scoring")
    w("")
    w("**Uploaded (this repository):**")
    w("")
    w("- `attempt2/` and `attempt3/`:")
    w("  - `results.json`: VERDICT, gates and values;")
    w("  - `campaign/`: PLAN, PARITY, LADDER.jsonl, VG.jsonl, ABBA.jsonl, ABBA_PLAN, WORK_PRECISION,")
    w("    MATCHED, ABBA_SUMMARY, IDENTITY at start and end, SETUP, the Julia and diffsol setup records and")
    w("    lock, REPORT.md, and the raw manifest `OUTPUT_SHA256SUMS.txt`;")
    w("  - `IMPORT_MANIFEST.json`.")
    w("- `julia_install/`: install evidence; the log is gzipped, with its raw sha256.")
    w("- `attempts/attempt1_…`: the INVALID attempt.")
    w("- `analysis/`: this document, `benchmark_summary.json` and the builder.")
    w("")
    w("Every file is listed with its sha256 under `file_index` in `benchmark_summary.json`.")
    w("")
    w("**Not uploaded (large).** These are the raw campaign tarballs, holding every request and stdout. Each")
    w("is bound by sha256 in the ledger and in `IMPORT_MANIFEST.json`. A reviewer who needs them can ask for")
    w("them by hash.")
    w("")
    for name in ("attempt2", "attempt3"):
        a = summary["attempts"][name]
        w(f"- {name}: `OUT.tar.gz`, {a['raw_tarball_not_uploaded']['bytes']} bytes, sha256 `{a['raw_tarball_sha256']}`.")
    w("")
    w("**Re-scoring.**")
    w("")
    w("- Extract a tarball and run `external_audits/vigilode_bassrei_ncp_c1_20261022/harness/analyze.py` on")
    w("  it. That reproduces `results.json`; the harness's own dry run checked this.")
    w("- Without the tarballs, the matched rungs, the paired ratios and the gates can be recomputed from")
    w("  `campaign/LADDER.jsonl`, `campaign/ABBA.jsonl` and `campaign/VG.jsonl`.")
    w("")
    w("**Rerunning the campaign.**")
    w("")
    w("- Command: `external_audits/vigilode_bassrei_ncp_c1_20261022/launch_from_local.sh <EXEC>` on a host")
    w("  that passes its preflight. EXEC is `e3341309` for attempt 2 and `2064366e` for attempt 3.")
    w("- See the harness `README.md`.")
    w("")
    w("## 10. The rest of the DAG (context)")
    w("")
    w("| node | ledger | verdict |")
    w("|---|---|---|")
    for node, lid, v in (("P02 fixed grid", "L-0148", "FAIL (superseded)"), ("P02b fixed grid", "L-0149", "PASS"),
                         ("P03 structured stage solvers", "L-0150", "PASS"),
                         ("P04 source-owned exchange, signed stress", "L-0151", "PASS"),
                         ("P05 accepted-step remap", "L-0152", "PASS"), ("P06 full sphere (scoped)", "L-0153", "PASS"),
                         ("C1 attempt 2", "L-0154", "PASS"), ("C1 attempt 3", "L-0155", "PASS")):
        w(f"| {node} | {lid} | {v} |")
    w("")
    w("Deviations awaiting the user's sign-off are listed in `handoff/HANDOFF.md` §5 and in each node's")
    w("preregistration.")
    return "\n".join(L) + "\n"


def main() -> None:
    summary = {
        "schema": "bassrei-c1-benchmark-summary-v1",
        "node": "research/bassrei_ncp_wp_c1_20261022",
        "timing_authority": "diagnostic_only",
        "statistical_authority": "STATISTICAL_AUTHORITY_HOLD",
        "B_app_relative": 1e-6, "B_src": {"number_per_H": 5e-12, "energy_eV_per_H": 5e-11},
        "preregistered_rung_rtol": 1e-8, "ladder_rtols": [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10],
        "cases": {c: {"model": d, "n": n, "regime": r, "authority": a} for c, (d, n, r, a) in CASE_INFO.items()},
        "attempts": {name: attempt_summary(name) for name in ATTEMPTS},
    }
    (HERE / "BENCHMARK_ANALYSIS.md").write_text(md(summary))
    summary["file_index"] = file_index()
    (HERE / "benchmark_summary.json").write_text(json.dumps(summary, indent=1, sort_keys=False) + "\n")


if __name__ == "__main__":
    main()
