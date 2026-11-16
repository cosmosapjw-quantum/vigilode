#!/usr/bin/env python3
"""Loop L1 audit synthesis: merge the three dimension findings files with the verifier verdicts.

Reads  : findings_evidence.json, findings_design.json, findings_code.json, synthesis_spot_checks.log
Writes : audit_findings.json, R_GATE_PROPOSAL.json   (same directory)
Run    : python3 -I build_audit_synthesis.py
Finding fields (claim, evidence, impact, minimal_fix, reproduce_cmd, severity, is_new, title) are
copied from the dimension files. Verifier verdicts are condensed from the orchestrator's task text
(the verifier agents wrote no file). Spot-check values are parsed from synthesis_spot_checks.log.
"""
import hashlib
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = "/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116"
AUD = "research/production_loop_20261116/audit"
UNI = "research/production_unification_20261115"
SW = "/home/cosmosapjw/vigilode"
DIMS = [("EVIDENCE", "findings_evidence.json"), ("DESIGN", "findings_design.json"), ("CODE", "findings_code.json")]


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def one(pattern, text, cast=str):
    m = re.search(pattern, text, re.S)
    if not m:
        raise SystemExit(f"spot-check log: pattern not found: {pattern}")
    return cast(m.group(1))


# --------------------------------------------------------------------------- spot-check log
log = (HERE / "synthesis_spot_checks.log").read_text()
sub = dict(re.findall(r"research/\w+/(eval_\w+)\.py subprocess=(\d+)", log))
ledger_rows = {
    m[0]: {"verdict": m[1], "supersedes": None if m[2] == "None" else m[2], "commit": m[3]}
    for m in re.findall(r"^(L-02\d\d) (\w+) supersedes= (\S+) commit= (\w+)", log, re.M)
}
spot = {
    "log": f"{AUD}/synthesis_spot_checks.log",
    "script": f"{AUD}/synthesis_spot_checks.sh",
    "log_sha256": sha(HERE / "synthesis_spot_checks.log"),
    "nature": "read-only commands run by the synthesis agent on 2026-11-16; a spot check corroborates a "
              "finding, it is not the independent verification that the preregistration asks for",
    "C01_subprocess_occurrences_per_eval_script": {k: int(v) for k, v in sub.items()},
    "C02_switcher_ledger_rows": ledger_rows,
    "C02_bench_example_paths_in_tree": one(r"grep -cE 'k1_shared_basis_bench\|b7_first_batch_bench'\n(\d+)", log, int),
    "C03_workflow_files_on_origin_main": len(re.findall(r"^\.github/workflows/\S+\.yml$", log, re.M)),
    "C03_production_yml_on_origin_main": bool(re.search(r"^\.github/workflows/production\.yml$", log, re.M)),
    "C03_workflow_files_at_loop_head": one(r"ls \.github/workflows \| wc -l\n(\d+)", log, int),
    "C04_loop_ledger_rows": one(r"wc -l < research/LEDGER\.jsonl;[^\n]*\n(\d+)\n", log, int),
    "C04_loop_ledger_rows_mentioning_production_unification": one(
        r"wc -l < research/LEDGER\.jsonl;[^\n]*\n\d+\n(\d+)\n", log, int),
    "C04_check_research_node_exit": one(r"check-research-node: FAIL[^\n]*\n\[exit (\d+)\]", log, int),
    "C07_spatial_grid_v2_matched_rows": one(r"\n(\d+) \{'DIAGNOSTIC_ONLY': \d+, 'MATCHED': \d+\}", log, int),
    "C07_spatial_grid_v2_diagnostic_only": one(r"\{'DIAGNOSTIC_ONLY': (\d+), 'MATCHED': \d+\}", log, int),
    "C07_spatial_grid_v2_matched": one(r"\{'DIAGNOSTIC_ONLY': \d+, 'MATCHED': (\d+)\}", log, int),
    "C08_commits_touching_agent_gate_after_4339facc": len(
        re.findall(r"^\w{8} 2026-11-0\d (?:feat|docs|chore)", one(r"## C08(.*?)## C09", log), re.M)),
    "C11_tracked_reject_files": len(re.findall(r"^\S+\.rej$", log, re.M)),
    "C11_pub_fn_solve_count": one(r"sed -n '3,4p' crates/vigilode/README\.md\n(\d+)\n", log, int),
    "C14_switcher_only_commits": one(r"switcher_only=(\d+)", log, int),
    "C14_loop_only_commits": one(r"loop_only=(\d+)", log, int),
    "C14_switcher_is_ancestor_of_loop": one(r"switcher_is_ancestor_of_loop_exit=(\d)", log, int) == 0,
    "C14_rei_only_commits": one(r"rei_only=(\d+)", log, int),
    "C14_rei_is_ancestor_of_loop": one(r"rei_is_ancestor_of_loop_exit=(\d)", log, int) == 0,
    "C14_followup_612063db_commits_not_on_switcher_head": one(r"followup_only_vs_switcher=(\d+)", log, int),
    "C14_protected_list_files_changed_on_switcher_since_merge_base": one(
        r"git diff --stat=200 569ce026\w+ 93392249\w+ --.*?\n (\d+) files? changed", log, int),
    "C14_protected_list_files_changed_on_loop_since_merge_base": one(
        r"git diff --stat=200 569ce026\w+ 2dc9a2c5\w+ --.*?\n (\d+) files? changed", log, int),
    "C14_lineage_tracks": one(r"tracks: (\d+) distinct", log, int),
    "C14_distinct_source_commits_cited_by_lineage_tracks": one(r"distinct source commits cited by lineage tracks: (\d+)", log, int),
    "C14_mentions_in_lineage_json": {
        k: int(v) for k, v in re.findall(r"^(93392249|612063db|6844c8fe|switcher|rei) lineage\.json=(\d+)", log, re.M)},
}
spot["C14_protected_list_files_changed_on_switcher"] = re.findall(
    r"^ (\S+)\s+\|", one(r"git diff --stat=200 569ce026\w+ 93392249\w+ --[^\n]*\n(.*?)\n \d+ files? changed", log), re.M)

# --------------------------------------------------------------------------- verifier verdicts
V = "independent verifier agent, loop L1 wave 1; text condensed from the orchestrator task; the verifier wrote no file"
VERIFICATION = {
    "EVD-02": {
        "verdict": "confirmed",
        "evidence": "eval_h1.py imports json, math, os, sys only; no subprocess or os.system; its single open() "
                    "(line 153) writes RESULTS.json. Work units are literals (lines 35-50), decision pairs are "
                    "literals (87-90), tv_ln_phi = 0.18 (100). Recomputed ln(185/192) - ln(98/340) = "
                    "1.2068385919901783 = RESULTS.json max_abs_log_ratio_bias. Ledger line 209 is L-0212 FAIL with "
                    "this script as its command and no measured data file in inputs_sha256; file hashes match "
                    "the row. PREREGISTRATION.md line 8 names WorkCounters that the script never collects. "
                    "PLAN_KO.md does not mention L-0212, eval_h1 or H1.",
        "correction": "Read 'synthetic' as 'typed-in without provenance': 185, 192, 193, 98, 144, 162 equal "
                      "work_units values in research/adapter_integration_20261027 artifacts, but the script "
                      "neither reads nor hashes them; 340 (the denominator behind |b| = 1.2069), 118, 182, 355, "
                      "47, the break-even switch counts and tv_ln_phi have no source found. Add L-0212 to the "
                      "V02 INVALID set. Defaults unaffected (entry_margin and return_margin stay None).",
    },
    "EVD-03": {
        "verdict": "confirmed",
        "evidence": "Hashes of eval_e1r.py and RESULTS.json match ledger row L-0211. Lines 160-161: the only "
                    "binary is the test target work_model_contracts; baseline is the Ir of test "
                    "rhs_cost_ir_scaling. Lines 163-304: nine cases, each a test_filter plus literal counters. "
                    "rotation_n2_dopri maps to polynomial_weight_exactness_and_powers (work_model_contracts.rs "
                    "87-97: four assert_eq!, no solver). Two cases share one filter (raw Ir 633716 against "
                    "633728). Stored result: 598825 - 598285 = 540 measured against 19250 predicted; "
                    "max_relative_error 182.97, Spearman 0.7667, FAIL. Row sets supersedes = L-0205. Nothing "
                    "was re-run under callgrind; numbers are read from the stored RESULTS.json.",
        "correction": "The third gate of L-0211 (transition_39 w_v2 = 264.2 WU, not below 200) does not depend "
                      "on callgrind (eval_e1r.py 354-356) but its counters are still typed literals. Only the "
                      "relative-error and Spearman metrics are meaningless as calibration. Which build of the "
                      "test binary was measured is not recorded.",
    },
    "EVD-04": {
        "verdict": "confirmed",
        "evidence": "git log --all --diff-filter=A: PREREGISTRATION.md, eval script and ledger row of each node "
                    "enter in one commit: K1 8800fb9a (02:53), B7 68692104 (02:57), E1R 157586cf (03:02), "
                    "H1 117e2870 (03:16), C1 16eb1b87 (04:08); D1 400cd0cc (03:37) with RESULTS.json only in "
                    "16eb1b87. DESIGN.md:1956 requires the preregistration to be committed and pushed before "
                    "the first run. Rows L-0209..L-0214 cite commit 4339facc (2026-11-03 05:30), 25 commits "
                    "before 8800fb9a; shared_basis.rs, coloring.rs, dopri5_stiffness.rs and margin.rs do not "
                    "exist there. The two example files appear in no ref's history.",
        "correction": "DESIGN.md lists the two example files under files_allowed (permitted paths), so "
                      "'prescribed' overstates it slightly. 'One commit per node' holds for five nodes; D1 is "
                      "split across 400cd0cc and 16eb1b87, so L-0213 was committed before its output file "
                      "existed in git.",
    },
    "DSG-02": {
        "verdict": "confirmed",
        "evidence": "ppsw1.rs: tableau(sigma) at :192 is double-double (one inverse, 12 products); the only call "
                    "site is :408, once per attempt, no cache. Runner sha256 0ef2e44a...b27d2 equals "
                    "logs/runner_binary.sha256; 45/45 result objects identical on rerun. Stored profile "
                    "re-aggregated (129305 samples): mm 28.58% + tableau 10.98% of all cycles = 54.02% of "
                    "main-thread cycles; 78.2k cycles per attempt. Fresh profile by the verifier on the same "
                    "binary: 51.4%, 88.9k cycles per attempt. Share of serial-arm main-thread cycles per "
                    "problem (stored / fresh): Oregonator n=3 71% / 68%, HIRES n=8 62% / 59%, Brusselator n=64 "
                    "16% / 16%. Wall ratios recomputed: 7.77-13.89, 22.2-24.4, 2.12-2.51.",
        "correction": "'The wall ratios mostly measure this constant' holds for n=3 and n=8 only; at n=64 the "
                      "rebuild is about 16% and the ratio without it would still be about 1.7-2.1 (estimates: "
                      "HIRES 3-5, Oregonator 5-7). 'Starter and fixed costs are small' is supported only by the "
                      "Brusselator fit (HIRES intercept is 19.8% of its loosest run; the Oregonator fit is not "
                      "informative). Quote the share as about 51-54% of main-thread user cycles over all three "
                      "arms, 78k-89k cycles per attempt, one loaded host.",
    },
    "DSG-03": {
        "verdict": "confirmed",
        "evidence": "Recomputed from bench/out/{UNIFIED_BENCHMARK_REPORT.md,rust_bench_results.json,"
                    "julia_bench_results.json}; report header hashes match the files. Tables are per rtol; raw "
                    "matched section: 80 MATCHED, 295 DIAGNOSTIC_ONLY. Only the grid-clipped arms vigilode "
                    "(12 rows) and vigilode-banded (3 rows) are present; output_clipped_steps is 99 in 13 rows "
                    "and 98 in 2, i.e. 0.103-0.943 of accepted steps. All 36 Julia rows have error_common null. "
                    "VigilODE / Julia Rodas5P accepted steps: 1.436-3.083 at rtol 1e-4, 0.943-1.012 at 1e-8. "
                    "Same-rtol error ratio over the tabulated arms: 0.158 to 1713.26. All 114 report rows "
                    "reproduce from the raw JSON. No ledger row cites the report.",
        "correction": "Including the untabulated internal-radau-iia3 arm the minimum error ratio is 0.033. The "
                      "on-disk bench/build_unified_comparison.py (1403e108, 2026-11-06) is later than the report "
                      "and lists oregonator, which the datasets lack. L-0142 supersedes L-0140 for "
                      "VigilODE-relative ratios only.",
    },
    "DSG-05": {
        "verdict": "confirmed",
        "evidence": "Own parse of bench/out/adversarial_spatial_battery_v3_results.json: matched[] has 2340 "
                    "rows = 88 MATCHED + 2252 DIAGNOSTIC_ONLY; MATCHED only on sphere-photon-2d (52) and "
                    "sphere-photon-3d (36); the other 7 problems have 260 reference-disputed rows each (1820). "
                    "Free-stepping dense arms take 6-47 accepted steps; n is 64-302; warmups 1, repetitions 3. "
                    "Search of both trees for JSON with MATCHED status: spatial grid v1 has 0, v2 has 88, "
                    "v3 has 88; all other files are 0D/1D.",
        "correction": "52 and 36 are matched comparison rows (baseline arm x comparator arm x level), not "
                      "levels: distinct matched error levels are 6 (2D) and 5 (3D). The 6-47 step range is for "
                      "the dense-output arms; the grid-clipped arms take 101-108 steps and yield no MATCHED "
                      "row. 2D/3D matched-error evidence is v2 and v3 only.",
    },
    "COD-01": {
        "verdict": "confirmed",
        "evidence": "ppsw1.rs:408 calls tableau(sigma) inside stage_inputs_unchecked on every attempt, no cache. "
                    "Verifier's own callgrind run (Oregonator n=3, tol 1e-6, 3341 attempts, 661860254 Ir): "
                    "tableau inclusive 79.87% = 158215 Ir per attempt; solve_stages 12.20%; alloc + dealloc "
                    "5.47%; starter + h0 + output 0.19%. perf stat: PPSW1 197907.4 and RODAS5P 13357.2 "
                    "instructions per attempt (14.82x); standalone tableau 158432 per call; without tableau "
                    "2.96x. memcheck: 61.02 against 0.06 allocations per attempt. The probe links 891044d7; "
                    "the audited sources are byte-identical to the loop head (cmp).",
        "correction": "The 80% share is specific to n=3: the cost is a fixed ~158k instructions per attempt, so "
                      "the share falls with n. 'sigma is held at 1.0 for most steps' was not measured here; a "
                      "cache hit rate must be measured before the ~3.0x residual is taken as reachable. Cycles "
                      "per call are load dependent (93068 on rerun against 76771). The allocation-site list and "
                      "the U11 impact text were not checked.",
    },
    "COD-02": {
        "verdict": "confirmed",
        "evidence": "ppsw1.rs:1125-1256: start() is one 5-stage Radau IIA step over 2*h0 accepted on Newton "
                    "convergence only, with no error estimate and no tolerance argument. selection.rs:959-979: "
                    "cap 0.5*(t_end-t0); default h0_rule_h_max is infinity, so a zero Jacobian gives h0 = cap. "
                    "selection.rs:1028 then reports success with zero Peer steps. Probe re-run by the verifier: "
                    "y' = 20 cos 20t gives success=true, error 8.3132 (9.0e6 tolerance units); 200 cos 200t "
                    "gives 50.245 (5.7e7); h0 = NaN gives success=true with h0 0.5. A numpy 5-point Radau "
                    "quadrature reproduces both end values to about 1e-13.",
        "correction": "The factor is 9.0e6 and 5.7e7, not 'about 1e7' for both. The NaN h0 case is accurate "
                      "(error 7.5e-10): it is an input-validation defect, not an accuracy failure. The impact "
                      "statements about U11, V10 and V03 were not verified.",
    },
    "COD-04": {
        "verdict": "confirmed",
        "evidence": "Source reading at loop head plus one float computation; the probe was not re-executed. "
                    "(a) admission() selection.rs:851-864 never reads problem.jacobian or problem.band; lower() "
                    ":876-898 wires a JVP provider regardless. (b) :1028 with span (0.3, 0.9): t0 + 2*h0 = "
                    "0.9000000000000001, so the loop is skipped and success=false with failure=None. "
                    "(c) callback_to_core :837-847 maps Recoverable to NonFinite and Fatal to InvalidInput; "
                    "try_step halves on NonFinite (ppsw1.rs:786-794) at the same Jacobian point. (d) :815 "
                    "failure is Option<String>. (e) lower() returns UnsupportedJacobian at :870-872 after "
                    "admission said Admitted.",
        "correction": "All five sub-claims hold as code facts. The probe counts 4 / 52 / 54 are consistent with "
                      "the code but not independently re-executed. Failure strings carry a 't=<t>: ' or "
                      "'starter: ' / 'h0 rule: ' prefix. Halving applies only inside the step loop. For (e) the "
                      "refusal is typed and precedes any callback; the defect is that admission() says Admitted. "
                      "(b) needs h0 equal to the endpoint cap and a product that rounds up.",
    },
}

# --------------------------------------------------------------------------- synthesis annotations
# plan_ref: the PLAN_KO.md section 1 statement a non-new finding re-derives.
# spot: section of synthesis_spot_checks.log and what it showed.
# effect: effect on recorded verdicts (invalidates_recorded_rows | limits_scope | none_recorded | plan_correction).
ANN = {
    "EVD-01": dict(plan_ref="1.2 synthetic evidence", related=["EVD-02", "EVD-03", "EVD-04"],
                   effect="invalidates_recorded_rows",
                   spot="C01: 0 occurrences of 'subprocess' in eval_k1/b7/c1/d1; C02: rows L-0209, L-0210, L-0213, "
                        "L-0214 have these scripts as their command. The EVD-04 verifier also found no cargo, "
                        "example or subprocess call in eval_k1.py and eval_b7.py; the DESIGN auditor read "
                        "eval_k1.py:12 and :50.",
                   note="Critical and not sent to a verifier. Rests on the plan stage, the EVIDENCE auditor and "
                        "the spot check; C1 and D1 were read by one auditor only."),
    "EVD-02": dict(related=["EVD-01"], effect="invalidates_recorded_rows",
                   spot="C01: eval_h1.py has 0 'subprocess' and one open(; C02: L-0212 FAIL with eval_h1.py as command."),
    "EVD-03": dict(related=["EVD-01"], effect="invalidates_recorded_rows",
                   spot="C02: L-0211 FAIL, supersedes = L-0205."),
    "EVD-04": dict(related=["EVD-01", "EVD-02", "EVD-03"], effect="invalidates_recorded_rows",
                   spot="C02: rows L-0209..L-0214 cite commit 4339facc; 0 bench example paths in the tree and in history."),
    "EVD-05": dict(effect="limits_scope",
                   spot="C08: six commits after 4339facc touch tools/agent_gate.py, among them X1, X2, X3 (K1), X4 and "
                        "X7 (C1). The content of the edits and PROGRESS.md were not re-read."),
    "EVD-06": dict(effect="plan_correction",
                   spot="C03: origin/main (8d0c7918, 2026-08-26, local ref) has 4 workflow files, none is "
                        "production.yml; the loop head has 31; production.yml push trigger is branches [main]. "
                        "The scan of all 31 'on:' blocks was not repeated."),
    "EVD-07": dict(plan_ref="1.1 ledger split; 1.5 release readiness", effect="none_recorded",
                   spot="C04: 202 ledger rows, 0 mention production_unification; check-research-node.py exits 1 "
                        "with the two FAIL lines quoted in the finding."),
    "EVD-08": dict(effect="limits_scope",
                   spot="C06: one static LIVE_BYTES, one #[global_allocator], three #[test], no Mutex or serial "
                        "guard. BASELINE_RESULTS.json flake_probe (orchestrator run): 3/3 pass single-threaded, "
                        "1 of 3 default runs fails."),
    "EVD-09": dict(related=["DSG-03", "DSG-05"], effect="limits_scope",
                   spot="C07: spatial grid v2 has 1040 matched rows = 952 DIAGNOSTIC_ONLY + 88 MATCHED; the v3 "
                        "report line 73 says 953 rows all DIAGNOSTIC_ONLY. Part (c) is covered by the DSG-03 "
                        "verification; the 88 MATCHED rows in v2 by the DSG-05 verification."),
    "EVD-10": dict(related=["DSG-07"], effect="none_recorded",
                   spot="C05: evaluate.py:113-115 sets depth and fallback criteria to pass True as literals; "
                        ":121 says SUNDIALS not installed; libsundials-dev 6.4.1 is installed."),
    "DSG-01": dict(plan_ref="1.2 U11 (c) growth cap", related=["DSG-09"], effect="limits_scope",
                   spot="C12: ACCEPTED_STEP_SIGMA_MAX = 1.10 at ppsw1.rs:57, PAPER_STEP_RATIO_BOUND = 1.5 at :58. "
                        "Limiter saturation numbers come from the auditor's scratch rebuild only."),
    "DSG-02": dict(related=["COD-01"], effect="limits_scope",
                   spot="C12: tableau(sigma) has a single call site, ppsw1.rs:408."),
    "DSG-03": dict(related=["EVD-09"], effect="limits_scope", spot=None),
    "DSG-04": dict(plan_ref="1.2 regime mismatch", related=["DSG-05", "DSG-06"], effect="limits_scope", spot=None,
                   note="The 87-row regime table is one auditor's tabulation; its categorical columns are the "
                        "auditor's reading. The evidence string in the input file is truncated after "
                        "'f in [0.011, 0.108] are'."),
    "DSG-05": dict(related=["DSG-04", "EVD-09"], effect="limits_scope", spot=None),
    "DSG-06": dict(related=["DSG-04"], effect="limits_scope", spot=None,
                   note="The EVIDENCE auditor regenerated U04 RESULTS.json byte for byte and matched 670 -> 9647 "
                        "steps with 257 rejections at n=400 and 569 -> 894 at n=200, which agree with the step "
                        "ratios quoted here; the cost exponents and the reading were not cross-checked."),
    "DSG-07": dict(plan_ref="1.2 U11 (a) 6-thread arm", related=["COD-08", "EVD-10"], effect="none_recorded",
                   spot="C13: evaluate.py:88 takes speed ratios only for cand == 'ppsw1_t6'; evaluate.py first "
                        "appears in 074a5699 (09:36:12), the preregistration in 745ffafb (09:28:25)."),
    "DSG-08": dict(effect="none_recorded", spot=None, note="A model, not a measurement (stated by the auditor)."),
    "DSG-09": dict(related=["DSG-01"], effect="none_recorded",
                   spot="C12: OD-15 exists at OWNER_DECISIONS_20261110.json:95; its text was not re-read."),
    "DSG-10": dict(effect="limits_scope",
                   spot="C12: the runner Cargo.toml has its own [workspace] and [profile.release] debug = false; the "
                        "root profile is opt-level 3, lto fat, codegen-units 1, panic abort."),
    "COD-01": dict(related=["DSG-02"], effect="limits_scope",
                   spot="C12: tableau(sigma) has a single call site, ppsw1.rs:408."),
    "COD-02": dict(related=["COD-04"], effect="limits_scope", spot=None),
    "COD-03": dict(plan_ref="1.3 MethodRegistry is not used for routing", effect="limits_scope", spot=None,
                   note="The nine disagreeing cases were executed by the auditor's probe (rows in "
                        "findings_code.json raw_evidence.registry_vs_select); no verifier re-ran them."),
    "COD-04": dict(related=["COD-02"], effect="limits_scope", spot=None),
    "COD-05": dict(plan_ref="1.3 switcher router tiers", related=["COD-06", "COD-07"], effect="limits_scope",
                   spot="C09: router.rs:63-66 sends Tier2, Tier4, Protected and Tier3 to dispatch_rodas5p_tier. "
                        "Switcher code was read, not built or run."),
    "COD-06": dict(related=["COD-05"], effect="limits_scope",
                   spot="C09: router.rs maps every unsuccessful SwitchingSolution to MaxStepsReached; "
                        "hybrid_automaton.rs accounting_complete() compares a sum with total_work(). The other "
                        "listed sites were not re-read. Not executed."),
    "COD-07": dict(related=["COD-05"], effect="limits_scope",
                   spot="C10: analyzer.rs:174-183 routes mu2 <= 0 with span <= 100 to Tier1Explicit and :191 sets "
                        "is_stiff = !is_contracting; workspace.rs:47 allocates vec![0.0; n * n]. Not executed: "
                        "the DOPRI5 step-count consequence is a hand estimate."),
    "COD-08": dict(plan_ref="1.2 U11 (a) 6-thread arm", related=["DSG-07"], effect="none_recorded", spot=None),
    "COD-09": dict(plan_ref="1.2 tracked .rej file; 1.3 crates outside the workspace", effect="none_recorded",
                   spot="C11: two tracked .rej files; root Cargo.toml lists 7 members without vigilode-bench and "
                        "vigilode-hybrid. Module categories are by name pattern (auditor's limitation)."),
    "COD-10": dict(effect="none_recorded",
                   spot="C11: 18 'pub fn solve*' in crates/vigilode/src; 0 README.md hits for PeerOde, "
                        "MethodRegistry, ProductionSolver, AutomaticOde; crates/vigilode/README.md:3 says one solver."),
}

STATUS_LEGEND = {
    "confirmed_by_verifier": "sent to an independent verifier agent and confirmed (corrections recorded)",
    "confirmed_plan_stage_and_auditor": "stated in PLAN_KO.md section 1 by the orchestrator's plan-stage audit "
                                        "and re-derived by a dimension auditor with command and path; not sent "
                                        "to a separate verifier",
    "lead": "found by one dimension auditor, not sent to a verifier; by the frozen V01 discriminator it stays a "
            "lead until reproduced (a synthesis spot check, where present, is corroboration only)",
    "refuted": "contradicted by a verifier (no finding has this status; one plan item does)",
}

findings = []
inputs = {}
plan_by_dim = {}
for dim, fn in DIMS:
    d = json.loads((HERE / fn).read_text())
    inputs[f"{AUD}/{fn}"] = sha(HERE / fn)
    plan_by_dim[dim] = d["plan_items_checked"]
    for f in d["findings"]:
        a = ANN[f["id"]]
        ver = VERIFICATION.get(f["id"])
        if ver:
            status = "confirmed_by_verifier"
            verification = dict(ver, source=V)
        else:
            status = "confirmed_plan_stage_and_auditor" if not f["is_new"] else "lead"
            verification = "not_sent"
        findings.append({
            "id": f["id"],
            "dimension": dim,
            "severity": f["severity"],
            "is_new": f["is_new"],
            "title": f["title"],
            "status": status,
            "verification": verification,
            "plan_item": not f["is_new"],
            "orchestrator_reproduced_at_plan_stage": (not f["is_new"]),
            "plan_ref": a.get("plan_ref"),
            "effect_on_recorded_verdicts": a["effect"],
            "related": a.get("related", []),
            "synthesis_spot_check": a.get("spot"),
            "synthesis_note": a.get("note"),
            "claim": f["claim"],
            "evidence": f["evidence"],
            "impact": f["impact"],
            "minimal_fix": f["minimal_fix"],
            "reproduce_cmd": f["reproduce_cmd"],
            "source_file": f"{AUD}/{fn}",
        })

assert len(findings) == 30 and len({f["id"] for f in findings}) == 30
assert set(VERIFICATION) <= {f["id"] for f in findings}

# --------------------------------------------------------------------------- merged plan items
# final: confirmed | corrected | refuted | not_checked. by: who derived it.
PLAN = [
    ("1.1", "Ledger split: unification line ends at L-0204 with 0 U rows; switcher holds L-0205..L-0214", "confirmed",
     "EVIDENCE; spot check C04", "The loop adds only L-0300; the program EVIDENCE_LEDGER.json has no U record."),
    ("1.1", "Only crates/rodas5p-integrators/src/lib.rs is modified on both sides; switcher toolchain is stable", "confirmed",
     "CODE; spot check C14", "Merge-base 569ce026; switcher 52 unique commits, loop 138."),
    ("1.1", "Switcher modified protected files sequential.rs, vigilode/src/lib.rs, switching.rs, Cargo.toml, "
            "rust-toolchain.toml, rodas5p_fast.rs, adaptive.rs", "corrected",
     "spot check C14 only (dimensions: not_checked)",
     "Eight files of the loop's protected list differ from the merge-base on the switcher line; the plan omits "
     "rodas5p_matrix_free_fast.rs (78 changed lines). Not independently verified."),
    ("1.1", "lineage.json, LINEAGE.md and HISTORY_RECOVERY.json do not mention the switcher line", "confirmed",
     "spot check C14 only", "0 hits in the three files; git_inventory.json does list both refs with "
                            "ancestor_of_current_research_base false."),
    ("1.1", "REI line (6844c8fe, 287 unique commits) is unmerged; N02, N03, 46,080-cell experiment", "confirmed",
     "spot check C14 only", "Commit count 287 and non-ancestry reproduced; the content claims (N02, N03, 46k run) "
                            "were not checked."),
    ("1.2", "eval_k1/eval_c1/eval_b7 are simulate_cell models; eval_d1 holds a literal 14/24", "corrected",
     "EVIDENCE; DESIGN (K1); spot check C01", "K1 and C1 use simulate_cell; B7 is a literal table; D1 is literal "
                                              "throughout. All four: 0 subprocess calls."),
    ("1.2", "Synthetic ledger rows are L-0209, L-0214, L-0210, L-0213", "corrected",
     "EVIDENCE; verifier (EVD-02, EVD-03)", "L-0212 (H1) is also typed-in; L-0211 (E1R) measures unrelated unit "
                                            "tests. Six rows L-0209..L-0214 lack executed support; L-0206 is "
                                            "already INVALID by L-0207."),
    ("1.2", "examples/k1_shared_basis_bench.rs does not exist", "confirmed", "EVIDENCE; verifier (EVD-04); spot check C02",
     "Also absent: examples/b7_first_batch_bench.rs and per-case E1R binaries. DESIGN.md lists them as allowed paths."),
    ("1.2", "Shared basis and colouring performance were never measured", "confirmed", "EVIDENCE",
     "Only contract tests exist."),
    ("1.2", "L-0211 E1R is the only row that ran callgrind", "corrected", "EVIDENCE; verifier (EVD-03)",
     "callgrind ran on work_model_contracts unit tests, not on the named solver cases."),
    ("1.2", "U11 (a): G0 speed uses the 6-thread arm, which is 1.6-2.9x slower than 1 thread", "confirmed",
     "EVIDENCE; DESIGN; CODE", "Recomputed 1.59-2.94 with bitwise identical states; the arm was not fixed by the "
                               "preregistration (DSG-07)."),
    ("1.2", "U11 (b): PPSW1 1-thread per-step wall 8-24x (n=3,8), 2.1-2.5x (n=64); implementation maturity; "
            "timer includes the starter", "corrected", "DESIGN; CODE; verifier (DSG-02, COD-01)",
     "Ratios reproduce (7.77-13.89, 22.2-24.4, 2.12-2.51). The cause is the per-attempt double-double tableau "
     "rebuild: 79.9% of instructions at n=3, about 16% of cycles at n=64. Starter + h0 rule + output are 0.19% "
     "of one n=3 solve."),
    ("1.2", "U11 (c): ACCEPTED_STEP_SIGMA_MAX = 1.10 (ppsw1.rs:57) is an agent rule, paper value 1.5; with the "
            "6-step hysteresis it gives tolerance-insensitive step counts", "confirmed", "EVIDENCE; DESIGN; CODE; spot check C12",
     "Limiter saturation 0.54-1.00 (auditor's scratch rebuild); OD-15 was written for mipeer6 (DSG-09, lead)."),
    ("1.2", "U11 matched-error step ratio 1.9-7.1x", "confirmed", "EVIDENCE; DESIGN; CODE",
     "1.911 to 7.108; 45/45 result objects reproduce on rerun of the recorded binary."),
    ("1.2", "U11 HOLD is valid but is not a method-level conclusion for the large regime", "confirmed", "DESIGN; CODE",
     "HOLD holds on either arm (median speed 0.00634 at 6 threads, 0.0166 at 1)."),
    ("1.2", "SUNDIALS is installed although U11 recorded it as not installed", "confirmed", "EVIDENCE; DESIGN; spot check C05",
     "libsundials-dev 6.4.1; literal string at evaluate.py:121."),
    ("1.2", "Regime mismatch: evaluation at n <= 64 (sometimes 1D n <= 400); RHS share 1-11%; no large, "
            "expensive-RHS or 2D/3D matched-error evaluation", "corrected", "DESIGN; verifier (DSG-05)",
     "87 evaluations tabulated: 16 have n > 1000 but none is a stiff cross-method matched-error run; matched-error "
     "maximum n = 400; nine 2D/3D cases exist on the switcher line (n <= 302), seven reference-disputed; 0 with "
     "an expensive RHS or a preconditioner. The conclusion holds."),
    ("1.2", "G0 (research gate) and release readiness were bundled in one HOLD", "not_checked", "-",
     "Addressed by owner decision OD-L1-4, not audited."),
    ("1.2", "Lineage document errors: L10 PR135-147 attribution, H14/H15 paths, DICTIONARY NOT_PORTED meaning", "not_checked", "-",
     "No dimension examined these."),
    ("1.2", "dense_output_v2.rs.rej is tracked", "confirmed", "CODE; spot check C11",
     "Plus research/production_reaudit_20261021/evidence/port/dense_patch_context.rej."),
    ("1.3", "Ten or more frontends coexist with different option and result types", "confirmed", "CODE; spot check C11",
     "10 entry structs, 6 result structs, 18 pub fn solve*."),
    ("1.3", "MethodRegistry describes 6 methods but is not used for routing; select hardcodes DOPRI5/RODAS5P", "confirmed", "CODE",
     "Extended by COD-03: admission and select disagree in 9 executed cases."),
    ("1.3", "BDF, Radau, PEXPRB, lagged, small/batch, SABR, homotopy, transactional, PTSW2B, PPSW1-MF, "
            "vigilode-coupled unreachable from the facade; Tier::Performance equals Protected", "confirmed", "CODE", ""),
    ("1.3", "Facade RODAS5P paths are sequential with no per-phase timers", "confirmed", "CODE", ""),
    ("1.3", "Switcher router: Tier2/3/4/Protected call the same Rodas5p::solve; switching failure always "
            "MaxStepsReached", "confirmed", "CODE; spot check C09", "Tier3Exponential is never produced. Read only, not built."),
    ("1.3", "3D and sphere operators exist only in test files; vigilode-bench and vigilode-hybrid are not "
            "workspace members", "confirmed", "EVIDENCE; CODE; spot check C11", "src/mol/mod.rs is a 3-line stub."),
    ("1.4", "PPSW1 matrix-free W has no crossover (n <= 400)", "confirmed", "DESIGN; EVIDENCE (byte-identical regeneration)",
     "Regime statement only: unpreconditioned GMRES against dense LU on a tridiagonal 1D problem (DSG-06, lead)."),
    ("1.4", "R7 transactional q2 4.75x slower; C5 graph-Schur 2.24x slower", "confirmed", "DESIGN",
     "RESULTS.json fields give 4.746 and 2.242; three source models with n <= 3."),
    ("1.4", "Lagged LU kill gate (HIRES fails, Brusselator passes); lane batch 0.93x", "confirmed", "DESIGN",
     "Ledger rows L-0141 and L-0089 read; raw files not re-parsed."),
    ("1.4", "R6 sweep, diagonal SDC, the five exact tableau facts, the cost-structure model and the 1D Schwarz toy", "not_checked", "-",
     "Outside the audit; tableau facts belong to V05, the toy is declared not evidence."),
    ("1.5", "No CI or independent reproduction of the current head", "confirmed", "EVIDENCE; spot check C03",
     "0 of 31 workflows trigger on a research-branch push. This audit regenerated U03/U04/U10/U11 results and "
     "re-ran three test targets."),
    ("1.5", "Protected RODAS5P has the same step counts as Julia Rodas5P (228 = 228, 164 vs 162)", "corrected",
     "EVIDENCE; DESIGN; verifier (DSG-03)",
     "True at rtol 1e-8 only. At 1e-4 the benchmark arm takes 1.44-3.08x the steps because 99 accepted steps per "
     "run are grid landings; Julia rows carry no error; dirty build."),
    ("1.5", "Per-step cost similar to Hairer RODAS", "not_checked", "-", "No dimension re-derived it."),
    ("3.1", "V02: supersede the four synthetic rows with INVALID rows", "corrected", "EVIDENCE; verifier (EVD-02, EVD-03)",
     "The set is L-0209..L-0214 (six rows)."),
    ("3.1", "V02: zero public name collisions; Jacobian::SparseJvp makes the match at selection.rs:306 non-exhaustive", "confirmed", "CODE",
     "PeerOde::lower has a wildcard arm and would refuse SparseJvp as UnsupportedJacobian."),
    ("3.1", "V13: independent CI through production.yml workflow_dispatch", "refuted", "EVIDENCE; spot check C03",
     "production.yml is absent on origin/main (local ref, no fetch); only a pull request would run it."),
    ("-", "U03, U04, U09, U10, U11 results are backed by executed runs", "confirmed", "EVIDENCE; DESIGN; CODE",
     "U03/U04/U10/U11 result files and reference.json regenerate byte-identically; 3060 U03 values and 5 U09 "
     "records reproduce at head; pass counts 18/6/5."),
    ("-", "U11 preregistration was pushed before any holdout run", "not_checked", "EVIDENCE; spot check C13",
     "Local commit order holds (745ffafb 09:28, runs 09:34, 074a5699 09:36); the push time is not verifiable locally."),
]
plan_items = [dict(plan_section=s, item=i, verdict=v, derived_by=b, note=n) for s, i, v, b, n in PLAN]

# --------------------------------------------------------------------------- recorded verdict effects
EFFECTS = {
    "stay_valid": [
        {"verdict": "U11 production holdout: HOLD (G0)", "basis": "Fails the step criterion on its own (1.911-7.108 against <= 1.2) and the speed criterion on either arm; 45/45 result objects reproduced by three agents.", "findings": ["DSG-01", "DSG-02", "COD-01", "DSG-07", "EVD-10"],
         "caveat": "The two G0 sub-criteria depth and fallback are literal passes in evaluate.py and must not be quoted as measured."},
        {"verdict": "U03 Peer qualification numbers", "basis": "RESULTS.json and NONLINEAR_RESULTS.json regenerate; 18 tests pass; 3060 values reproduce at head.", "findings": ["EVD-10"],
         "caveat": "RESULTS.json carries a stale ppsw1.rs input hash."},
        {"verdict": "U04 NO_CROSSOVER_MEASURED for n <= 400", "basis": "RESULTS.json regenerates byte-identically.", "findings": ["DSG-06"], "caveat": "Regime statement, see limited_in_scope."},
        {"verdict": "U09 geometry contracts and U10 switch-error transport", "basis": "5 tests pass and 5 QUAL records reproduce; transport_results.json regenerates byte-identically.", "findings": ["EVD-10"],
         "caveat": "U09 has no machine-readable results file; U10 is a pure Python model."},
        {"verdict": "L-0207 INVALID (supersedes L-0206)", "basis": "Already recorded on the switcher line.", "findings": ["EVD-01"], "caveat": ""},
        {"verdict": "L-0205 / L-0208 switcher admission", "basis": "Scripts execute the solver; AFTER.json and GATES.json hash-match the ledger.", "findings": [], "caveat": "The 216 raw files are untracked (disk only), so the row cannot be replayed from git."},
        {"verdict": "v3 qualification conclusion NOT_ESTABLISHED", "basis": "Conservative and unaffected by the receipt error.", "findings": ["EVD-09"], "caveat": "Receipts are typed text without logs."},
        {"verdict": "L-0300 loop baseline FAIL", "basis": "Valid as a record of the default-thread run.", "findings": ["EVD-08"], "caveat": "The two failures are a test-harness race (lead), not a solver defect."},
        {"verdict": "Defaults: WorkModel V1, entry_margin / return_margin None", "basis": "No invalidated row had changed a default.", "findings": ["EVD-02", "EVD-03"], "caveat": ""},
    ],
    "limited_in_scope": [
        {"verdict": "U11 step ratios 1.9-7.1", "limit": "Property of the candidate as configured (1.10 cap, 6-step history, h0 rule), not of PPSW1-6b.", "findings": ["DSG-01", "DSG-09"]},
        {"verdict": "U11 per-step wall ratios and speed_vs_production", "limit": "Dominated at n=3 and n=8 by the per-attempt tableau rebuild; 6-thread arm measures pool wake-up; non-repository build profile. Not method-level numbers and not extrapolable.", "findings": ["DSG-02", "COD-01", "COD-08", "DSG-07", "DSG-10"]},
        {"verdict": "U04 no crossover", "limit": "Unpreconditioned GMRES against dense LU on one 1D tridiagonal family at one tolerance.", "findings": ["DSG-06"]},
        {"verdict": "U06 unified capability registry", "limit": "A descriptive table; it does not reproduce GeneralOde::select (9 executed disagreements).", "findings": ["COD-03"]},
        {"verdict": "U07 PeerOde adapter claims (typed refusal before callbacks, capability constraint) and every matched-error PPSW1 number", "limit": "Admission ignores the provider form; failures are untyped strings; the starter interval is not error controlled.", "findings": ["COD-02", "COD-04"]},
        {"verdict": "Negative results on parallel stages, matrix-free solves and LU reuse", "limit": "Small or 1D problems with cheap RHS; nothing stiff and matched-error above n = 400.", "findings": ["DSG-04"]},
        {"verdict": "Switcher UNIFIED_BENCHMARK_REPORT and the plan's Julia step-parity sentence", "limit": "Matched tolerance, grid-clipped VigilODE arms, Julia rows without error; parity holds at rtol 1e-8 only.", "findings": ["DSG-03", "EVD-09"]},
        {"verdict": "Switcher PDE battery v3", "limit": "Matched rows for 2 of 9 problems (sphere-photon 2D/3D), n <= 302, 6-47 steps per run.", "findings": ["DSG-05"]},
        {"verdict": "Switcher 'unified production frontend' (93392249)", "limit": "Executed engines are DOPRI5, Rodas5p::solve and SwitchingSolver; tier names are labels. Read only, not built.", "findings": ["COD-05", "COD-06", "COD-07"]},
        {"verdict": "Switcher X1-X11 'gate passed' statements", "limit": "The gate tool was edited in gated commits (lead).", "findings": ["EVD-05"]},
        {"verdict": "L-0211 third gate (transition_39 w_v2 = 264.2 WU, not below 200)", "limit": "Independent of callgrind but computed from typed-in counters.", "findings": ["EVD-03"]},
    ],
    "invalid_as_evidence": [
        {"verdict": "L-0209 (K1 FAIL), L-0210 (B7 FAIL), L-0213 (D1 FAIL), L-0214 (C1 PASS)", "reason": "No process is started and no file is read; every number is a literal or arithmetic on literals. The FAIL rows are not negative results and the PASS has no support.", "findings": ["EVD-01", "EVD-04"], "confirmation": "plan stage + EVIDENCE auditor + spot check; not sent to a verifier"},
        {"verdict": "L-0212 (H1 FAIL)", "reason": "Typed-in work units without provenance.", "findings": ["EVD-02", "EVD-04"], "confirmation": "verifier"},
        {"verdict": "L-0211 (E1R FAIL) as a WorkModel V2 calibration", "reason": "callgrind measured unit tests unrelated to the named cases; WorkModel V2 is neither confirmed nor refuted.", "findings": ["EVD-03", "EVD-04"], "confirmation": "verifier"},
        {"verdict": "v3 receipt statement '953 matched rows, all DIAGNOSTIC_ONLY'", "reason": "The cited raw file has 1040 matched rows: 952 DIAGNOSTIC_ONLY and 88 MATCHED.", "findings": ["EVD-09"], "confirmation": "EVIDENCE auditor + spot check C07"},
        {"verdict": "U11 not_run reason 'SUNDIALS not installed'", "reason": "libsundials-dev 6.4.1 is installed; the CVODE comparator was omitted without a valid reason.", "findings": ["EVD-10"], "confirmation": "plan stage + two auditors + spot check C05"},
        {"verdict": "Plan V13 route 'production.yml workflow_dispatch'", "reason": "The workflow is absent on the default branch.", "findings": ["EVD-06"], "confirmation": "EVIDENCE auditor + spot check C03 (local refs)"},
    ],
}

NOT_EXECUTED = [
    "Switcher line (93392249): nothing was built or run; COD-05, COD-06, COD-07 are source readings. The V02 trial merge, the 1.94.1 build and the bitwise oracle are not part of this audit.",
    "Full test suites, clippy and fmt were not run by the audit; three test targets were re-run (18/6/5 passed). The 251-target integrators suite has not been run in loop L1.",
    "Clean-clone build and hosted CI: not run (no workflow triggers; dispatch unavailable).",
    "E1R: nothing was re-run under callgrind; EVD-03 rests on the stored RESULTS.json and the script source.",
    "COD-04 probe counts (4 JVP callbacks, 52 rejections, 54 Jacobian callbacks) were not re-executed by the verifier.",
    "Twelve findings were not sent to a verifier and stay leads: EVD-05, EVD-06, EVD-08, EVD-09, EVD-10, DSG-06, DSG-08, DSG-09, DSG-10, COD-06, COD-07, COD-10.",
    "EVD-01 (critical) was not sent to a verifier; C1 and D1 scripts were read by one auditor plus the spot check.",
    "Plan items not checked by any dimension: lineage document errors (L10, H14/H15, DICTIONARY), per-step cost against Hairer RODAS, the gate-bundling statement, the section 1.4 theory facts and toy, REI content claims, the push time of the U11 preregistration.",
    "Protected-file changes on the switcher line were checked by the synthesis spot check only (file list and line counts, not content).",
    "The 35 other test files that use a global allocation counter were not examined (EVD-08).",
    "The U11 CVODE and Julia comparators, isolated-process RSS and the independent reviewer rerun stay NOT_RUN as recorded by U11; this audit reran the recorded binary only.",
    "Wall-clock values in all three dimensions were taken at load average 15-34 and are diagnostic_only.",
]

RECONCILIATION = [
    {"topic": "Share of the tableau rebuild", "dimensions": ["DSG-02", "COD-01"],
     "resolution": "Not a contradiction: 54.0% is main-thread cycles over the whole holdout (three arms, three problems); 79.9% is PPSW1 instructions per attempt at n=3. The cost is fixed (~158k instructions, 78k-89k cycles per attempt), so the share falls with n: about 68-71% at n=3, 59-62% at n=8, 16% at n=64."},
    {"topic": "Number of ledger rows to supersede", "dimensions": ["EVD-01", "EVD-02", "EVD-03"],
     "resolution": "Five typed-in rows (L-0209, L-0210, L-0212, L-0213, L-0214) plus one mis-measured row (L-0211) = six rows L-0209..L-0214. EVD-02's 'five rows' counts the typed-in ones only."},
    {"topic": "Plan 1.2 (b) cause of the per-step wall ratio", "dimensions": ["DSG-02", "COD-01"],
     "resolution": "Both dimensions replace 'implementation maturity and starter in the timer' by the tableau rebuild; the starter share is 0.19% at n=3 (COD) but the HIRES linear-fit intercept is 19.8% of its loosest run (DSG-02 verifier), so 'negligible starter' is established for n=3 only."},
    {"topic": "2D/3D evidence", "dimensions": ["DSG-04", "DSG-05", "EVD-09"],
     "resolution": "Consistent: spatial grid v2 and battery v3 each have 88 MATCHED rows, all on the two sphere-photon problems; v1 has 0."},
]

sev_order = {"critical": 0, "major": 1, "minor": 2}
counts = {
    "total": len(findings),
    "by_severity": {s: sum(f["severity"] == s for f in findings) for s in sev_order},
    "by_dimension": {d: sum(f["dimension"] == d for f in findings) for d, _ in DIMS},
    "by_status": {s: sum(f["status"] == s for f in findings) for s in STATUS_LEGEND},
    "new": sum(f["is_new"] for f in findings),
    "sent_to_verifier": len(VERIFICATION),
    "plan_items": {v: sum(p["verdict"] == v for p in plan_items) for v in ("confirmed", "corrected", "refuted", "not_checked")},
}
assert counts["by_status"]["confirmed_by_verifier"] == 9

out = {
    "schema": "vigilode.loop-l1.audit-findings-merged.v1",
    "date": "2026-11-16",
    "node": "V01_AUDIT_RGATE",
    "refs": {
        "loop_worktree": f"{ROOT} @ 2dc9a2c5778ac5b3ac960b0f6741aa5916122384",
        "switcher_lineage": f"{SW} @ 933922495045796c99c530b0f1f1efd8fec83ed0",
        "rei_lineage": "6844c8fe18361f3c2243ee2db95bbf3d68a6ce7b",
        "merge_base_loop_switcher": "569ce02629498fec133db9ce368eb303ba943b69",
    },
    "generator": f"{AUD}/build_audit_synthesis.py",
    "inputs_sha256": inputs,
    "verifier_verdicts_source": V,
    "status_legend": STATUS_LEGEND,
    "counts": counts,
    "findings": sorted(findings, key=lambda f: (sev_order[f["severity"]], [d for d, _ in DIMS].index(f["dimension"]), f["id"])),
    "plan_items": plan_items,
    "plan_items_by_dimension": plan_by_dim,
    "cross_dimension_reconciliation": RECONCILIATION,
    "effects_on_recorded_verdicts": EFFECTS,
    "not_executed": NOT_EXECUTED,
    "synthesis_spot_checks": spot,
    "volatile_scratch": {
        "note": "Raw outputs of the dimension audits were written to scratch; small text files were copied to "
                f"{AUD}/raw_scratch with a sha256 manifest. Binaries, perf data and target directories were not copied.",
        "original_dirs": ["/tmp/claude-1000/evd-audit-l1", "/tmp/claude-1000/dsg-audit", "/tmp/claude-1000/cod-audit"],
    },
}
(HERE / "audit_findings.json").write_text(json.dumps(out, indent=1, ensure_ascii=False, allow_nan=False) + "\n")

# --------------------------------------------------------------------------- R-gate proposal
matrix_path = Path(ROOT) / "docs/agent/production_solver_program_20261112/RELEASE_GATE_MATRIX.json"
matrix = json.loads(matrix_path.read_text())
P = {
    "R1_CORRECTNESS": dict(status="NOT_MET", sub=[
        ("order and stiff convergence, Peer candidate", "EXECUTED_SUPPORT", "U03: 18 tests pass, 3060 logged values reproduce at head", ["EVD-10"]),
        ("agreement with references, every exposed method", "COUNTEREXAMPLE", "PeerOde (public, admitted without admit_unqualified) returns success=true with error 8.31 and 50.2 at rtol 1e-6 (9.0e6 and 5.7e7 tolerance units)", ["COD-02"]),
        ("stratified battery over every exposed method (RODAS5P routes, DOPRI5, Lawson, ensemble, switching)", "NOT_RUN", "this audit ran no correctness battery; baseline suites are a receipt, not a qualification", []),
    ], reasons=["One executed counterexample on an exposed method is enough to fail 'every exposed method'.",
                "Protected RODAS5P correctness was not re-evaluated here; it is neither supported nor questioned by this audit."],
        paths=[f"{AUD}/findings_code.json (COD-02, probe_sources)", f"{UNI}/peer_qualification/RESULTS.json", "research/production_loop_20261116/BASELINE_RESULTS.json"]),
    "R2_ROBUSTNESS": dict(status="NOT_MET", sub=[
        ("typed failures", "COUNTEREXAMPLE", "PeerOde failure is Option<String>; success=false with failure=None for span (0.3, 0.9); user Recoverable error ends as an unrelated InvalidInput after 52 halvings (count from the auditor's probe)", ["COD-04"]),
        ("capability refusal before callbacks", "COUNTEREXAMPLE", "a caller that does not allow provided_jvp still gets JVP callbacks; admission says Admitted for a banded problem that solve() refuses", ["COD-04"]),
        ("non-finite input", "COUNTEREXAMPLE", "options.h0 = NaN is replaced silently by the endpoint cap", ["COD-02"]),
        ("step-size underflow, session ownership, protected-route failure paths", "NOT_RUN", "not examined on the loop head", []),
        ("no silent fallback (switcher unified frontend, not on the judged head)", "LEAD", "failure reasons replaced, uncharged preflight, repeat instead of fallback; read only", ["COD-06", "COD-07"]),
    ], reasons=["Three confirmed defects of the public PeerOde adapter contradict the typed-failure and refusal sub-criteria."],
        paths=[f"{AUD}/findings_code.json (COD-04, COD-02, COD-06)", "crates/vigilode/src/selection.rs:815,837-864,959-979,1028", "crates/rodas5p-integrators/src/ppsw1.rs:786-794"]),
    "R3_API": dict(status="NOT_MET", sub=[
        ("one documented entry point", "ABSENT", "10 solver entry structs, 6 result types, 18 pub fn solve*", ["COD-10"]),
        ("capability and refusal semantics", "COUNTEREXAMPLE", "MethodRegistry admission and GeneralOde::select disagree in 9 executed cases; the registry is not used for routing", ["COD-03"]),
        ("documentation matches the surface", "LEAD", "README says one solver and never mentions PeerOde, MethodRegistry, AutomaticOde, ProductionSolver", ["COD-10"]),
        ("MSRV and feature policy", "NOT_RUN", "not examined", []),
    ], reasons=["No single entry point exists on the judged head; V03 is the node that would change this."],
        paths=[f"{AUD}/findings_code.json (COD-03 raw_evidence.registry_vs_select, COD-10)", "crates/vigilode/src/selection.rs:176-227,509-665", f"{AUD}/synthesis_spot_checks.log (C11)"]),
    "R4_REPRODUCIBILITY": dict(status="NOT_MET", sub=[
        ("build on the pinned toolchain with locked dependencies", "EXECUTED_SUPPORT", "baseline at 074a5699, toolchain 1.94.1, --locked (not a clean clone)", []),
        ("clean-checkout build", "NOT_RUN", "", []),
        ("deterministic tests", "COUNTEREXAMPLE", "baseline 789 passed / 2 failed with default threads (L-0300 FAIL); sp02_workspace_charge fails 1 of 3 default-thread runs, 0 of 3 single-threaded", ["EVD-08"]),
        ("CI evidence on the judged head", "ABSENT", "0 of 31 workflows trigger on a research-branch push; production.yml is not on the default branch, so dispatch is unavailable", ["EVD-06"]),
        ("independent rerun", "EXECUTED_SUPPORT", "U03, U04, U10, U11 result files regenerate byte-identically; targets re-run 18/6/5 passed; U11 recorded binary reproduces 45/45 result objects (coverage: U03, U04, U06, U09, U10, U11 only)", ["EVD-10", "DSG-02", "COD-01"]),
    ], reasons=["A flaky test target and the absence of any CI run on the head contradict two sub-criteria.",
                "The executed reruns support the recorded U numbers; their raw logs were in scratch (copied to audit/raw_scratch)."],
        paths=["research/production_loop_20261116/BASELINE_RESULTS.json", "research/production_loop_20261116/logs/baseline_facade_core_krylov.log:1007-1014", f"{AUD}/findings_evidence.json (part_b_unification_reproduction, part_c_governance)", ".github/workflows/production.yml:10-25", f"{AUD}/synthesis_spot_checks.log (C03)"]),
    "R5_PERFORMANCE": dict(status="PARTIAL", sub=[
        ("matched-achieved-error work-precision, small 0D/1D regime", "HISTORICAL_EXECUTED", "ledger L-0139/L-0142 (bench_wave2/3, n <= 400) and 26 matched-error cross-method evaluations in the auditor's table; not re-run on the judged head by this audit", ["DSG-04"]),
        ("external-solver comparison harness", "EXECUTED_OFF_HEAD", "crates/vigilode-bench executes RADAU5, RODAS, DVODE, CVODE, diffsol and Julia with raw per-repetition data, on the unmerged switcher line; its unified report is matched-tolerance with grid-clipped arms", ["DSG-03", "EVD-09"]),
        ("2D/3D regime", "EXECUTED_OFF_HEAD", "battery v3: matched rows for 2 of 9 problems, n <= 302, 6-47 steps per run; 7 problems reference-disputed", ["DSG-05"]),
        ("large n, expensive RHS, preconditioned Krylov", "NOT_RUN", "0 evaluations found among 87", ["DSG-04"]),
        ("U11 external comparators (CVODE, Julia)", "NOT_RUN", "recorded as not run; the stated reason for CVODE is wrong", ["EVD-10"]),
    ], reasons=["Executed matched-error data exists only for small 0D/1D problems and, off the judged head, for two sphere problems.",
                "The criterion asks for documentation per regime and does not require a win; three regimes have no run.",
                "All wall figures in the cited sources are diagnostic_only."],
        paths=[f"{AUD}/findings_design.json (DSG-03, DSG-04, DSG-05; data.regime_table, data.bench_v3)", f"{SW}/bench/out/adversarial_spatial_battery_v3_results.json", f"{SW}/bench/out/rust_bench_results.json", "research/LEDGER.jsonl (L-0139, L-0142)"]),
    "R6_PACKAGING": dict(status="NOT_MET", sub=[
        ("workspace membership", "COUNTEREXAMPLE", "crates/vigilode-bench and crates/vigilode-hybrid have their own [workspace] and lock file; root members list has 7 crates without them", ["COD-09"]),
        ("separation of research code from production crates", "COUNTEREXAMPLE", "rodas5p-integrators compiles 27 gate/campaign/candidate modules (21142 LOC, name-pattern count) unconditionally; two .rej files are tracked", ["COD-09"]),
        ("crate metadata", "LEAD", "crates/vigilode Cargo.toml description and README say one solver", ["COD-10"]),
        ("licences", "NOT_RUN", "not examined", []),
    ], reasons=["Research modules are not separated from the production crate and two crates are outside every workspace command."],
        paths=[f"{AUD}/findings_code.json (COD-09, raw_evidence.lib_rs_mod_categories)", "Cargo.toml:3-11", "crates/rodas5p-integrators/src/lib.rs", f"{AUD}/synthesis_spot_checks.log (C11)"]),
    "R7_EVIDENCE_GOVERNANCE": dict(status="NOT_MET", sub=[
        ("ledger continuity", "COUNTEREXAMPLE", "0 ledger rows for U01-U11; tools/check-research-node.py exits 1 for the unification node; rows L-0205..L-0214 live only on the unmerged switcher line", ["EVD-07"]),
        ("no evaluation without executed solver output", "COUNTEREXAMPLE", "six switcher rows L-0209..L-0214 rest on typed-in or mis-measured numbers; U11 evaluate.py has two literal G0 passes", ["EVD-01", "EVD-02", "EVD-03", "EVD-10"]),
        ("pre-registration before runs", "COUNTEREXAMPLE", "the six 2026-11-04 nodes commit preregistration, script and result together; the U11 speed arm and evaluator were fixed after the runs", ["EVD-04", "DSG-07"]),
        ("gate tooling independence (switcher line)", "LEAD", "agent_gate.py edited inside gated commits", ["EVD-05"]),
        ("loop L1 own process", "NOT_JUDGED", "PREREGISTRATION.md states it was pushed before any V run; not audited here", []),
    ], reasons=["All three named sub-criteria have confirmed counterexamples, two of them verifier-confirmed."],
        paths=[f"{AUD}/findings_evidence.json (EVD-01..EVD-07, part_a, part_c)", "research/LEDGER.jsonl", f"{SW}/research/LEDGER.jsonl (lines for L-0209..L-0214)", f"{UNI}/production_holdout/evaluate.py:88-89,113-121", f"{AUD}/synthesis_spot_checks.log (C01, C02, C04)"]),
    "R8_GEOMETRY_SCOPE": dict(status="PARTIAL", sub=[
        ("executed convergence contracts for 2D, 3D and sphere operators", "EXECUTED_SUPPORT", "U09 geometry_mol_contracts: 5 tests pass and 5 QUAL records reproduce at head (spatial order 1.98/1.99; sphere orders 2.00/2.00, 1.99/2.00, 1.87/1.89; pole-ring error 2.4e-2/1.2e-2/6.1e-3)", ["EVD-10"]),
        ("operators available outside tests, with stated limits", "ABSENT", "3D and sphere operators exist only under tests/; src/mol/mod.rs is a 3-line stub", []),
        ("conservation contracts", "NOT_RUN", "not examined by this audit", []),
        ("machine-readable receipt", "ABSENT", "U09 has no results file or summariser; numbers live in log lines", ["EVD-10"]),
        ("2D/3D matched-error battery", "EXECUTED_OFF_HEAD", "switcher battery v3: 2 of 9 problems matched, 7 reference-disputed", ["DSG-05"]),
    ], reasons=["Convergence contracts are executed and reproduce, but the operators are test-only and conservation was not examined."],
        paths=[f"{UNI}/geometry_mol/logs/u09_green.log", "crates/vigilode/tests/geometry_mol_contracts.rs", f"{AUD}/findings_evidence.json (part_b U09_geometry_mol, EVD-10)", f"{AUD}/findings_design.json (DSG-05)"]),
}
by_id = {f["id"]: f for f in findings}
axes = []
for ax in matrix["axes"]:
    p = P[ax["id"]]
    cited = sorted({x for _, _, _, fs in p["sub"] for x in fs})
    axes.append({
        "id": ax["id"],
        "criterion": ax["criterion"],
        "current_status_in_matrix": ax["status"],
        "proposed_status": p["status"],
        "reasons": p["reasons"],
        "sub_criteria": [{"item": i, "state": s, "basis": b, "findings": fs} for i, s, b, fs in p["sub"]],
        "findings": [{"id": x, "status": by_id[x]["status"], "severity": by_id[x]["severity"]} for x in cited],
        "evidence_paths": p["paths"],
    })
assert [a["id"] for a in axes] == [a["id"] for a in matrix["axes"]]
assert all(a["proposed_status"] in ("NOT_MET", "PARTIAL", "MET", "NOT_RUN") for a in axes)
rgate = {
    "schema": "vigilode.loop-l1.r-gate-proposal.v1",
    "date_kst": "2026-11-16",
    "nature": "Proposal from the V01 audit synthesis. It does not edit RELEASE_GATE_MATRIX.json and is not a verdict.",
    "matrix": "docs/agent/production_solver_program_20261112/RELEASE_GATE_MATRIX.json",
    "matrix_sha256": sha(matrix_path),
    "judged_head": "2dc9a2c5778ac5b3ac960b0f6741aa5916122384 (code equal to 074a5699 plus loop bootstrap)",
    "status_rule": {
        "MET": "every sub-criterion has an executed receipt on the judged head and an independent rerun",
        "NOT_MET": "at least one sub-criterion has an executed counterexample or is structurally absent",
        "PARTIAL": "some sub-criteria have executed support, none has a counterexample, others are not run or exist only off the judged head",
        "NOT_RUN": "nothing executed for the axis",
    },
    "sub_criterion_states": ["EXECUTED_SUPPORT", "HISTORICAL_EXECUTED", "EXECUTED_OFF_HEAD", "COUNTEREXAMPLE", "ABSENT", "LEAD", "NOT_RUN", "NOT_JUDGED"],
    "proposed_overall": "NOT_MET",
    "proposed_overall_basis": "Six of eight axes are proposed NOT_MET and two PARTIAL; no axis is MET. Unchanged from the matrix's plan-stage verdict.",
    "relation_to_g0": "G0 is not judged here; U11 HOLD stays as recorded.",
    "caveats": [
        "Sub-criteria in state LEAD rest on findings that no verifier reproduced; they do not drive any proposed status.",
        "No axis was evaluated by a dedicated battery; statuses follow from audit findings and reruns of existing evidence.",
        "Wall-clock figures in the cited evidence are diagnostic_only.",
    ],
    "axes": axes,
    "source_findings": f"{AUD}/audit_findings.json",
}
(HERE / "R_GATE_PROPOSAL.json").write_text(json.dumps(rgate, indent=1, ensure_ascii=False, allow_nan=False) + "\n")
print(json.dumps({"counts": counts, "r_gate": {a["id"]: a["proposed_status"] for a in axes}}, indent=1))
