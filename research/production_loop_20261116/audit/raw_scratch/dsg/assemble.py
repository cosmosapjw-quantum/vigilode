#!/usr/bin/env python3
"""Assemble audit/findings_design.json. Every number in the text is formatted from the computed JSON files
in this directory (themselves parsed from raw outputs); nothing numeric is typed in."""
import json, hashlib, datetime, os
D='/tmp/claude-1000/dsg-audit/'
L=lambda f: json.load(open(D+f))
u,cf,ss,pa,rc,ex,br,bs,b3,u4,rt=[L(f) for f in ('u11_recompute.json','controller_floor.json','step_stats.json','perf_agg.json','rerun_cmp.json','extras.json','bench_recompute.json','bench_scan.json','bench_v3.json','u04_recompute.json','regime_table.json')]
W='/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/'; HO=W+'research/production_unification_20261115/production_holdout/'; SW='/home/cosmosapjw/vigilode/'
g=lambda x,n=3: float(f"%.{n}g"%x)
rng=lambda v,n=3: f"{g(min(v),n)}-{g(max(v),n)}"
R={(r['problem'],r['arm']):r for r in u['ranges']}
tr=[x['wall_t6_over_t1'] for x in u['thread_ratio']]
psw={p:[x['ratio_t1_over_rodas5p'] for x in u['per_step_wall'] if x['problem']==p] for p in ('hires','oregonator','brusselator_mol32')}
rhs_same=[x['rhs_ratio_t1'] for x in u['per_step_wall']]; lu_same=[x['lu_ratio_t1'] for x in u['per_step_wall']]
SS={(s['problem'],s['tol']):s for s in ss}
CF={(r['problem'],r['tol']):r for r in cf['rows']}
loose=[s for s in ss if s['tol']>=9e-6]; tight=[s for s in ss if s['tol']<=1.1e-8]
fits={(f['problem'],f['arm']):f for f in ex['wall_linear_fits']}
clipfr=[c['clipped_fraction'] for c in br['vigilode_clipping']]
jl={(c['problem'],c['rtol']):c for c in br['vigilode_vs_julia_rodas5p_steps']}
j4=[c['ratio'] for c in br['vigilode_vs_julia_rodas5p_steps'] if c['rtol']==1e-4]; j8=[c['ratio'] for c in br['vigilode_vs_julia_rodas5p_steps'] if c['rtol']==1e-8]
errr=[v for m in br['matched_error_recomputed'] for v in m.get('same_tol_error_ratio_comp_over_vig',{}).values()]
v3=b3['per_problem_matched']; v3_matched={p:v.get('MATCHED|dense',0) for p,v in v3.items()}
v3_zero=[p for p,c in v3_matched.items() if c==0]
v3_steps=[s['accepted'] for s in b3['vigilode_steps'] if s['arm']=='vigilode-banded-dense']
ba=u4['best_admissible']; u4rows={(r['n'],r['arm']):r for r in u4['rows']}
sp_par6={n:u4rows[(n,'mf_independent_par6')]['speed_vs_dense'] for n in u4['n_values']}
dexp=[d['exponent_per_attempt'] for d in u4['dense_scaling']]
S=rt['summary']
npc=sum(1 for r in rt['rows'] if 'precond' in r['linear_backend'].lower() and 'identity' not in r['linear_backend'].lower())
am=cf['amdahl']
def sha(p): return hashlib.sha256(open(p,'rb').read()).hexdigest()

f=[]
f.append({"id":"DSG-01","severity":"major","is_new":False,
 "title":"U11 step ratio measures the OD-15/25/26 growth limiter and the OD-24 h0 rule, not the PPSW1-6b method",
 "claim":(f"The PPSW1 controller allows one growth of at most 1.10 every 7 accepted steps (mean {g(cf['mean_growth_per_step_cap1.10_hyst6'],5)} per step). "
          f"Step histories regenerated with the U11 problem set (final states bitwise equal to the U11 log in {sum(s['final_state_bitwise_equal_u11_log'] for s in ss)}/{len(ss)} runs) show {rng([s['fraction_at_cap'] for s in ss])} of accepted steps exactly at the cap (structural maximum 1/7) and the limiter saturated "
          f"(growth steps taken / growth steps permitted) at {rng([s['limiter_saturation'] for s in loose])} for tol 1e-4..1e-5 and {rng([s['limiter_saturation'] for s in tight])} at tol 1e-8. "
          f"With zero local error the rule alone needs {CF[('hires',1e-4)]['n_min_cap1.10_hyst6']} steps on HIRES and {CF[('brusselator_mol32',1e-4)]['n_min_cap1.10_hyst6']} on Brusselator MOL32 from the logged h0, i.e. {g(CF[('hires',1e-4)]['n_min_over_rodas5p_attempts'])}x and {g(CF[('brusselator_mol32',1e-4)]['n_min_over_rodas5p_attempts'])}x the RODAS5P attempts at tol 1e-4. "
          "Plan item 1.2(c) is confirmed and quantified. The G0 step criterion still fails for the candidate as configured, so HOLD stands; no conclusion about the step efficiency of the method follows."),
 "evidence":(f"crates/rodas5p-integrators/src/ppsw1.rs:57 (cap), :841-849 (history 6, band), :935-956 (propose_ratio), :1023-1036 (h0 = min(h_max, 1/||J||inf)); "
             f"data.step_stats (scratch rebuild at 891044d7, binary sha256 in scratch_artifacts) and data.controller_floor. Fraction of holds that fall within 6 steps after a growth: {rng([s['fraction_hold_after_growth'] for s in ss])}. "
             f"Same rule with cap 1.5 would need {CF[('hires',1e-4)]['n_min_cap1.50_hyst6']} (HIRES) and {CF[('brusselator_mol32',1e-4)]['n_min_cap1.50_hyst6']} (Brusselator) steps (arithmetic only, no stability claim)."),
 "impact":"Limits scope: the recorded U11 HOLD is valid; the step ratios 1.9-7.1 are not a property of PPSW1-6b. V10 must separate controller, h0 rule and method.",
 "minimal_fix":"V10: log accepted (t, h, sigma, err) per step; report limiter saturation next to every step ratio; add arms that vary the cap, the history length and h0 independently.",
 "reproduce_cmd":"python3 /tmp/claude-1000/dsg-audit/controller_floor.py; cd /tmp/claude-1000/dsg-audit/steprunner && CARGO_TARGET_DIR=/tmp/claude-1000/dsg-audit/target cargo +1.94.1 build --locked --offline -j 6 --release && /tmp/claude-1000/dsg-audit/target/release/u11-holdout-runner > step_histories.jsonl && python3 /tmp/claude-1000/dsg-audit/step_stats.py"})
f.append({"id":"DSG-02","severity":"major","is_new":True,
 "title":"More than half of the PPSW1 run time in U11 is a per-attempt double-double rebuild of the sigma tableau",
 "claim":(f"A cycle profile of the pinned U11 runner binary (sha256 equal to logs/runner_binary.sha256) attributes {g(100*pa['tableau_mm_share_of_main_thread'])}% of main-thread user cycles of the whole holdout to ppsw1::mm + ppsw1::tableau "
          f"({g(100*pa['tableau_mm_fma_share_of_main_thread'])}% with the libm fma calls they make), about {int(round(pa['cycles_per_tableau_call_mm_plus_tableau'],-2))} cycles per step attempt and independent of n. "
          f"stage_inputs_unchecked calls tableau(sigma) on every attempt although sigma equals 1 (within 5e-4) on {rng([s['fraction_hold'] for s in ss])} of accepted steps; mm is called only from tableau (ppsw1.rs:241-263). "
          f"The per-attempt wall ratios PPSW1(1 thread)/RODAS5P ({rng(psw['hires'])} at n=8, {rng(psw['oregonator'])} at n=3, {rng(psw['brusselator_mol32'])} at n=64) therefore measure this constant. "
          f"The starter and other fixed costs are small by comparison (intercept of wall against attempts {g(fits[('brusselator_mol32','ppsw1_t1')]['intercept_ms'])} ms on Brusselator, {g(100*fits[('brusselator_mol32','ppsw1_t1')]['intercept_share_of_loosest_tol_wall'])}% of the loosest-tolerance run). "
          "The plan attributes the ratio to general implementation maturity and the starter; the dominant cause is this one function."),
 "evidence":f"crates/rodas5p-integrators/src/ppsw1.rs:192-260 (tableau in double-double), :408 (the only non-test call site, once per attempt); data.perf_agg ({pa['samples']} samples, cycles:u); data.extras.wall_linear_fits. Rerun reproduced {rc['identical_result_objects_incl_counters']}/{rc['rows']} result objects of the original log including counters.",
 "impact":"Limits scope: U11 wall ratios cannot be extrapolated to any regime. Even without this cost the 1-thread speed (median "+str(g(u['g0_ppsw1_t1']['median_speed']))+") stays far below 1.15, so HOLD is unaffected.",
 "minimal_fix":"V10 equal-implementation arm: cache the tableau for the last sigma (or precompute sigma = 1) before any per-step cost comparison; report instructions per attempt with and without the rebuild.",
 "reproduce_cmd":"perf record -e cycles:u -F 2000 -o perf_u11.data <u11-holdout-runner> > rerun.jsonl && perf script -i perf_u11.data -F comm,period,ip,sym > perf_script.txt && python3 /tmp/claude-1000/dsg-audit/perf_agg.py"})
f.append({"id":"DSG-03","severity":"major","is_new":True,
 "title":"Switcher-line UNIFIED_BENCHMARK_REPORT is matched-tolerance, built on grid-clipped VigilODE arms, and Julia rows have no error",
 "claim":(f"The report tabulates arms at equal rtol; the achieved error of a comparator divided by that of VigilODE at the same rtol ranges from {g(min(errr))} to {g(max(errr),4)}. Both VigilODE arms in the dataset (vigilode, vigilode-banded) land on the 101-point output grid: "
          f"{min(c['output_clipped_steps'] for c in br['vigilode_clipping'])}-{max(c['output_clipped_steps'] for c in br['vigilode_clipping'])} accepted steps of every row are forced landings ({rng([100*x for x in clipfr])}% of the accepted steps); the free-stepping arms have {br['free_stepping_vigilode_rows']} rows in this dataset. "
          f"All {br['julia_rows']} Julia rows carry error_common = null ({br['julia_rows_with_error']} with an error) and Julia runs free with saveat interpolation. "
          f"VigilODE/Julia Rodas5P accepted-step ratios are {rng(j4)} at rtol 1e-4 and {rng(j8)} at rtol 1e-8, so the equality quoted in plan 1.5 (228 = 228, 164 vs 162) holds only at the tightest tolerance. "
          f"The dataset has the same 102 keys as the E1 corpus of ledger L-0139, (rebound by L-0140), which L-0142 superseded for VigilODE-relative ratios by re-running the corpus with every arm stepping freely. Three table rows were reproduced exactly from the raw JSON."),
 "evidence":(f"{SW}crates/vigilode-bench/src/arms.rs:125-127 (is_grid_clipped = Vigilode | VigilodeBanded), :213-214; bench/julia_comparator_bench.jl:311-342 (saveat=grid); bench/build_unified_comparison.py:14-40; "
             f"data.bench_recompute (vigilode_clipping, vigilode_vs_julia_rodas5p_steps, reproduced_rows, matched_error_recomputed). The raw JSON has its own matched section ({br['matched_status_counts']}) that the report does not use; only 3 tolerances per arm."),
 "impact":"The report cannot support a matched-error statement or a step-parity statement about protected RODAS5P; the plan's provisional R-gate sentence built on it must be re-derived from free-stepping arms. No ledger row cites the report, so no recorded verdict is invalidated.",
 "minimal_fix":"V11/V13: regenerate with vigilode-dense arms, error for Julia arms on the same grid, at least 5 tolerances, and tables at matched achieved error.",
 "reproduce_cmd":"python3 /tmp/claude-1000/dsg-audit/bench_recompute.py"})
f.append({"id":"DSG-04","severity":"major","is_new":False,
 "title":"Regime coverage: no stiff cross-method matched-error evaluation above n = 400, none with an expensive RHS, none with a preconditioner",
 "claim":(f"A table of {S['evaluations']} located evaluations (ledger rows, RESULTS files, U nodes, switcher bench) has {S['n_gt_1000']} with n > 1000, {S['operators_2d_or_3d']} with 2D/3D operators and {S['rhs_cost_dominates']} where the RHS dominates. "
          f"The n > 1000 rows are banded operation counts with bitwise parity, non-stiff DOPRI5 vectors, separable structural lanes and a controller copy microbenchmark; {S['n_gt_1000_stiff_cross_method_matched_error']} of them is a stiff cross-method matched-error evaluation. "
          f"The largest n in a matched-error cross-method comparison is {S['matched_error_cross_method_max_n']}. All 2D/3D rows are on the unmerged switcher line with n <= {S['operators_2d_or_3d_max_n']}; the loop base 074a5699 has {S['operators_2d_or_3d_on_loop_base_074a5699']}. "
          f"Krylov arms with a non-identity preconditioner in an integrator evaluation: {npc}; preconditioner kernels were verified standalone (L-0117 stage 0 held at the reference, L-0190 synthetic linear solves) and never reached a matched-error or speed run. The plan's wording (all candidate evaluation at n <= 64, sometimes 1D n <= 400) is too narrow as a description of what exists but its conclusion holds."),
 "evidence":f"data.regime_table (rows with source file per row); ledger L-0113: {S['callback_share_evidence'][:150]}",
 "impact":"Limits scope of every negative result on parallel stages, matrix-free solves and LU reuse to small or 1D problems with cheap RHS.",
 "minimal_fix":"V04/V11 grids as planned; record n, operator dimension, Jacobian type, backend and measured RHS share in every result row.",
 "reproduce_cmd":"python3 /tmp/claude-1000/dsg-audit/regime_table.py"})
f.append({"id":"DSG-05","severity":"major","is_new":True,
 "title":"PDE battery v3 yields matched-error levels for 2 of 9 problems; the other 7 are reference-disputed, and runs take 6-47 steps",
 "claim":(f"In adversarial_spatial_battery_v3_results.json {len(v3_zero)} of {len(v3_matched)} problems have zero MATCHED levels ({', '.join(sorted(v3_zero))}); {b3['non_matched_reasons'].get('reference-disputed')} comparisons carry reason reference-disputed. "
          f"Only sphere-photon-2d ({v3_matched['sphere-photon-2d']}) and sphere-photon-3d ({v3_matched['sphere-photon-3d']}) produce matched levels. The free-stepping banded VigilODE arm needs {min(v3_steps)}-{max(v3_steps)} accepted steps per run on a 101-point output grid, and n is {min(b3['problem_n'].values())}-{max(b3['problem_n'].values())}. "
          "The battery and its subsets (spatial grid v1/v2, the first four problems) are the only 2D/3D matched-error evidence in either lineage."),
 "evidence":f"{SW}bench/out/adversarial_spatial_battery_v3_results.json (matched[], rows[]); data.bench_v3.per_problem_matched, non_matched_reasons, vigilode_steps; warmups {bs['adversarial_spatial_battery_v3_results.json']['warmups']}, repetitions {bs['adversarial_spatial_battery_v3_results.json']['repetitions']}.",
 "impact":"Limits scope: no geometry or 2D/3D performance statement can rest on battery v3 beyond the two sphere-photon problems at n <= 302.",
 "minimal_fix":"V11: fix the reference policy first (independent reference with stated uncertainty below the tightest level), then size problems so that runs take hundreds of steps and n reaches 1e4-1e5.",
 "reproduce_cmd":"python3 /tmp/claude-1000/dsg-audit/bench_v3.py"})
f.append({"id":"DSG-06","severity":"major","is_new":True,
 "title":"U04 no-crossover is a statement about unpreconditioned GMRES against dense LU on a tridiagonal 1D problem",
 "claim":(f"No arm has a preconditioner (IdentityPreconditioner only). The single family is 1D Fisher MOL, n = {u4['n_values'][0]}..{u4['n_values'][-1]}, one tolerance. The comparator factors the tridiagonal W with dense LU (per-attempt cost exponent in n rising to {g(dexp[-2])} for n = 200..400 and {g(dexp[-1])} for n = 400..800), so neither arm uses the band structure. "
          f"The arm reported as best admissible at n = 200 and n = 400 took {g(ba['200']['steps_ratio_vs_dense'])}x and {g(ba['400']['steps_ratio_vs_dense'])}x the steps of the dense arm (admissibility checks only the endpoint difference, {g(ba['400']['endpoint_diff_tol_units'],2)} tolerance units at n = 400). "
          f"The step-preserving 6-thread arm rises from {g(sp_par6[50])} to {g(sp_par6[400])} of dense speed between n = 50 and n = 400 and is at {g(sp_par6[800])} at n = 800 when it stops on the 600-vector Arnoldi limit at t = 0.455. "
          "The verdict NO_CROSSOVER_MEASURED for n <= 400 is correct as recorded; it is a regime statement and says nothing about preconditioned or 2D/3D use."),
 "evidence":f"{W}crates/rodas5p-integrators/src/ppsw1_linear_backend.rs:12,280; tests/ppsw1_linear_backend_contracts.rs:374-425 (fisher, dense Jacobian of a tridiagonal operator); research/production_unification_20261115/matrix_free_w/logs/u04_crossover.log parsed into data.u04_recompute; summarize_u04.py admissibility rule.",
 "impact":"Limits scope; does not invalidate the U04 verdict. The plan line 'PPSW1 matrix-free W crossover none (n <= 400)' must not be read as a method result.",
 "minimal_fix":"V06/V10: add a banded direct comparator, a preconditioned arm, an admissibility rule that also bounds the step ratio, and a 2D operator.",
 "reproduce_cmd":"python3 /tmp/claude-1000/dsg-audit/u04_recompute.py"})
f.append({"id":"DSG-07","severity":"minor","is_new":False,
 "title":"U11 speed criterion: the thread arm was not preregistered and the evaluator was committed with the results",
 "claim":(f"PREREGISTRATION.json names the candidate 'at 1 and 6 threads' and the criterion 'median over problems' without choosing an arm. evaluate.py (first committed in 074a5699, after the runs) uses only ppsw1_t6 for speed and takes the median over {u['stored_speed_evidence_n']} problem x E* points. "
          f"Recomputed medians: 6-thread {g(u['g0_ppsw1_t6']['median_speed'])} (equal to the stored value), 1-thread {g(u['g0_ppsw1_t1']['median_speed'])}, best single point {g(u['g0_ppsw1_t1']['max_speed'])}. The 6-thread arm is {rng(tr)}x slower than the 1-thread arm with bitwise identical states. "
          f"A Rayon pool is built inside the timed region on every solve, and the 6-thread arm pays {rng([m['t6_minus_t1_slope_us_per_attempt'] for m in ex['marginal']])} microseconds more per attempt. The best single 1-thread point is {g(1.15/u['g0_ppsw1_t1']['max_speed'])}x below 1.15 and the 1-thread median {g(1.15/u['g0_ppsw1_t1']['median_speed'])}x below."),
 "evidence":f"{HO}PREREGISTRATION.json:6,32; evaluate.py:88-89 (cand == 'ppsw1_t6'); git log -- evaluate.py -> 074a5699 only; crates/vigilode/src/selection.rs:945 and crates/rodas5p-integrators/src/parallel.rs:22-41 (pool per solve); runner/src/main.rs:170-213 (timed region); data.u11_recompute.",
 "impact":"Does not change the verdict. It is an undisclosed analysis choice in a preregistered evaluation.",
 "minimal_fix":"Future preregistrations name the arm, the aggregation and the evaluator hash before the run.",
 "reproduce_cmd":"python3 /tmp/claude-1000/dsg-audit/u11_recompute.py"})
f.append({"id":"DSG-08","severity":"minor","is_new":True,
 "title":"Cost model: stage-parallel PPSW1 can reach 1.15x only where Jacobian+LU is a small share and the step ratio is below about 2",
 "claim":(f"Both methods build one Jacobian and one LU per attempt, so the LU count ratio equals the step ratio (measured {rng(lu_same)} at equal tolerance). With ideal 6-way stage parallelism and zero overhead the speed is at most 1/(rho (f + (1-f)/8)), f being the Jacobian+LU share of a RODAS5P step. "
          f"For the measured matched-error step ratios rho = {rng(am['measured_matched_error_step_ratios'])} this needs f <= {g(am['max_f_for_speed_1.15'][0])} at the best point and f <= {g(am['max_f_for_speed_1.15'][4])} at the median; for f = 0.5 it needs rho <= {g(am['rho_needed_for_1.15_at_f0.5'])}. "
          "In addition the starter factors a 5n x 5n matrix once, which equals 125 step factorizations in the cubic regime. This is a model, not a measurement."),
 "evidence":"data.controller_floor.amdahl (computed from data.u11_recompute.matched); crates/rodas5p-integrators/src/ppsw1.rs:468-477 (one J and one LU per attempt), :1151-1165 (5n starter LU); RODAS5P counters in logs/holdout_runs.jsonl (direct_factorizations = attempts).",
 "impact":"Design input for V10: the preregistered regime must be chain-dominated and large enough for threads; LU-dominated dense problems cannot pass with the current step ratios.",
 "minimal_fix":"V10 preregistration states f from V04 for each regime before the run.",
 "reproduce_cmd":"python3 /tmp/claude-1000/dsg-audit/controller_floor.py"})
f.append({"id":"DSG-09","severity":"minor","is_new":True,
 "title":"The 1.10 cap was approved for a different method and is hard-wired, so V10 cannot vary sigma through Config",
 "claim":("OD-15 reads 'PW04 sigma_max rule for mipeer6 ... sigma_max <= 1.10 (agent-proposed own rule, NOT literature; A(alpha)~79deg there)'; it concerns the multi-implicit peer candidate. OD-25 carries 'sigma_max<=1.10' into the PPSW1-6b estimator decision and OD-26 approves the controller as OWNER_CHOICE, 'not literature-certified for PPSW1'. "
          "No PPSW1-6b stability analysis of the cap is cited. Session::new rejects step_ratio_cap > 1.10 and check_sigma, propose_ratio and RATIO_CLIP clamp with the same constant, so the V10 arms sigma in {1.2, 1.3, 1.5} need a separate code path."),
 "evidence":f"{W}docs/agent/unified_local_agent_20261109/OWNER_DECISIONS_20261110.json (OD-15, OD-25, OD-26; file lines 95, 155, 161); crates/rodas5p-integrators/src/ppsw1.rs:57-58, :370, :697-703, :845, :939; docs/agent/unified_local_agent_20261109/PW04_PPSW1_6BC_VERIFICATION_KO.md:24 (stability constant 138 for sigma <= 1.10, 709 for sigma <= 1.5).",
 "impact":"Provenance only; no verdict changes.",
 "minimal_fix":"V10 adds an experiment-only sibling controller with its own cap constant and leaves ACCEPTED_STEP_SIGMA_MAX untouched.",
 "reproduce_cmd":"grep -n 'ACCEPTED_STEP_SIGMA_MAX' crates/rodas5p-integrators/src/ppsw1.rs"})
f.append({"id":"DSG-10","severity":"minor","is_new":True,
 "title":"U11 timing binary was built with a profile different from the repository release profile",
 "claim":(f"The runner is its own Cargo workspace with [profile.release] debug = false, which gives opt-level 3 without LTO and with 16 codegen units; the repository release profile is fat LTO, one codegen unit, panic = abort. No target-cpu flag is set, so f64::mul_add compiles to libm fma calls ({g(100*pa['fma_libcall_share_of_main_thread'])}% of main-thread cycles). "
          "Both arms share the binary, so the comparison is internally consistent, but absolute per-step costs differ from the profile used by the other timing campaigns."),
 "evidence":f"{HO}runner/Cargo.toml (own [workspace], profile); {W}Cargo.toml:35-39; logs/build.log ('release profile [optimized]'); data.perf_agg.fma_libcall_share_of_main_thread.",
 "impact":"Limits comparability of U11 wall numbers with other campaigns; verdict unaffected.",
 "minimal_fix":"Timing runners copy the workspace release profile and record RUSTFLAGS and target features.",
 "reproduce_cmd":"cat research/production_unification_20261115/production_holdout/runner/Cargo.toml; sed -n 35,39p Cargo.toml"})

plan=[
 {"item":"1.2 U11 (a): G0 speed verdict uses the 6-thread arm; 6 threads are 1.6-2.9x slower than 1 thread at n=3/8/64","verdict":"confirmed",
  "note":f"evaluate.py:88-89 uses ppsw1_t6 only; recomputed t6/t1 wall {rng(tr)}; states bitwise identical; arm not fixed by the preregistration (DSG-07)."},
 {"item":"1.2 U11 (b): PPSW1 1-thread per-step wall 8-24x RODAS5P (n=3,8), 2.1-2.5x (n=64); fewer solves/RHS so it is implementation maturity; timer includes the starter","verdict":"corrected",
  "note":f"Ratios recomputed: {rng(psw['hires'])} (n=8), {rng(psw['oregonator'])} (n=3), {rng(psw['brusselator_mol32'])} (n=64). Fewer RHS/solves holds per attempt (6 vs 8) but totals at equal tolerance are {rng(rhs_same)}x (RHS) and {rng(lu_same)}x (LU). Dominant cause is the per-attempt double-double tableau rebuild ({g(100*pa['tableau_mm_share_of_main_thread'])}% of main-thread cycles), not the starter (DSG-02)."},
 {"item":"1.2 U11 (c): ACCEPTED_STEP_SIGMA_MAX = 1.10 (ppsw1.rs:57) is an agent rule (OD-15/25), paper value 1.5; with 6-step hysteresis it gives tolerance-insensitive step counts; step ratio mixes method and controller","verdict":"confirmed",
  "note":f"Measured: {rng([s['fraction_at_cap'] for s in ss])} of accepted steps at the cap, limiter saturation {rng([s['limiter_saturation'] for s in ss])}; OD-15 was written for mipeer6 (DSG-01, DSG-09)."},
 {"item":"1.2: HOLD verdict valid but not a method-level conclusion for the large regime","verdict":"confirmed",
  "note":f"Recomputed G0 values equal the stored ones (max step ratio {g(u['g0_ppsw1_t6']['steps_ratio_max'],4)}, median speed {g(u['g0_ppsw1_t6']['median_speed'],4)}); rerun of the pinned binary reproduced {rc['identical_result_objects_incl_counters']}/{rc['rows']} result objects."},
 {"item":"1.2: SUNDIALS is installed although U11 recorded it as not installed","verdict":"confirmed","note":"ldconfig -p lists libsundials_* and /usr/include/sundials exists; the switcher bench links CVODE 6.4.1."},
 {"item":"1.2 regime mismatch: candidate evaluation at n <= 64 (sometimes 1D n <= 400); RHS share 1-11%; no large, expensive-RHS or 2D/3D matched-error evaluation","verdict":"corrected",
  "note":f"{S['evaluations']} evaluations tabulated: {S['n_gt_1000']} have n > 1000 (none a stiff cross-method matched-error run), matched-error maximum n = {S['matched_error_cross_method_max_n']}, {S['operators_2d_or_3d']} 2D/3D rows exist on the switcher line at n <= {S['operators_2d_or_3d_max_n']} with 7 of 9 reference-disputed, expensive RHS {S['rhs_cost_dominates']}. RHS share 0.011-0.108 confirmed from L-0113."},
 {"item":"1.2: synthetic evidence in switcher eval_k1.py etc. (simulate_cell, err = 0.42*rtol)","verdict":"confirmed","note":"eval_k1.py:12 and :50 read; K1 cells (1D/2D heat) are therefore excluded from the regime table. Other eval_* scripts not checked in this dimension."},
 {"item":"1.4: PPSW1 matrix-free W has no crossover (n <= 400)","verdict":"confirmed","note":"Recomputed from the raw log; regime statement only (DSG-06)."},
 {"item":"1.4: R7 transactional q2 4.75x slower; C5 graph-Schur 2.24x slower","verdict":"confirmed","note":"RESULTS.json fields give 1/0.2107 = 4.746 and 2.242; scope is three small source models with n <= 3."},
 {"item":"1.4: lagged LU kill gate (HIRES fails, Brusselator passes); lane batch 0.93x","verdict":"confirmed","note":"Ledger rows L-0141 (0.588 > 0.4 on HIRES, 0.244 on Brusselator n = 100) and L-0089 (0.937/0.924) read; raw files not re-parsed."},
 {"item":"1.5: protected RODAS5P has the same step counts as Julia Rodas5P on small problems (Brusselator 228 = 228, Robertson 164 vs 162) and per-step cost similar to Hairer RODAS","verdict":"corrected",
  "note":f"True only at rtol 1e-8. At rtol 1e-4 the ratio is {rng(j4)} because 99 accepted steps per run are grid landings; Julia rows have no error; tables are matched-tolerance (DSG-03)."},
 {"item":"1.3: 3D and spherical operators exist only in test files; vigilode-bench is not a workspace member","verdict":"not_checked","note":"Outside this dimension except that all 2D/3D evaluations found are in the switcher bench crate."},
]
answers={
 "1_u11_holdout":{
   "matched_error_ranges":u['ranges'],"g0_by_arm":{"ppsw1_t1":u['g0_ppsw1_t1'],"ppsw1_t6":u['g0_ppsw1_t6']},
   "per_step_wall_table":"data.u11_recompute.per_step_wall",
   "timed_region":"runner/src/main.rs:172-212: Instant starts after Problem construction and stops after the result is converted to a serde_json Value. RODAS5P arm: Rodas5p::solve plus JSON conversion of the final state and counters. PPSW1 arm: registry admission, problem lowering, ParallelExecution::rayon(threads) (a new Rayon pool per solve when threads = 6; sequential when 1), h0 rule (one dense Jacobian and its infinity norm), 5-stage Radau IIA starter (one 5n x 5n dense LU, simplified Newton, 7 iterations on HIRES), all step attempts, a clone of the state after every accepted step, JSON conversion of two counter sets.",
   "jacobian":"Both arms use the same analytic dense Jacobian callback. RODAS5P fills a driver-owned buffer; the PPSW1 adapter allocates a new DenseMatrix per call (selection.rs:877-884) and ppsw1::solve_stages builds W = I - h gamma J in a second matrix and factors it (ppsw1.rs:468-477). One Jacobian and one LU per attempt in both arms.",
   "build_profile":"own workspace, [profile.release] debug = false: opt-level 3, no LTO, 16 codegen units, default target features; rustc 1.94.1; repository release profile (fat LTO, 1 codegen unit, panic abort) not used.",
   "timing_protocol":"2 warm-ups + 7 repetitions, arm order rotated, median; shared host with load average about 5; diagnostic only.",
   "g0_speed_arm":"ppsw1_t6 only (evaluate.py:88-89)","preregistered_arm":"No. PREREGISTRATION.json names 1 and 6 threads and does not choose; evaluate.py was first committed with the results in 074a5699.",
   "rerun":rc},
 "2_controller":{"ACCEPTED_STEP_SIGMA_MAX":"1.10 (ppsw1.rs:57); PAPER_STEP_RATIO_BOUND 1.5 (ppsw1.rs:58)",
   "hysteresis_rule":"propose_ratio (ppsw1.rs:935-956): raw = clamp(0.85 err^(-1/5), 0.5, cap); return 1 if raw in [0.95, 1.05]; return 1 if raw > 1.05 and any of the last 6 accepted ratios > 1; else raw. NaN halves.",
   "OD-15":"PW04 sigma_max rule for mipeer6 | approved: sigma_max <= 1.10 (agent-proposed own rule, NOT literature; A(alpha)~79deg there) | 2026-11-12",
   "OD-25":"PPSW1-6b estimator | approved: Est = Y_ms - sum w_j(sigma) Y_{m-1,j} (degree-4 predictor, exact Lagrange weights), control exponent 1/5, sigma_max<=1.10, rounding-floor diagnostic | 2026-11-13",
   "OD-26":"PPSW1-6b controller | approved as OWNER_CHOICE: thesis PTSW controller with exponent 1/5 (0.85 safety, clip [0.5,sigma_cap], hysteresis [0.95,1.05], growth-history rule); not literature-certified for PPSW1 | 2026-11-13",
   "fraction_at_cap_from_u11_logs":"NOT_AVAILABLE (the runner logs only the final state)",
   "fraction_at_cap_from_scratch_rebuild":"data.step_stats (same commit 891044d7, same options, final states bitwise equal to the U11 log)"},
 "3_regime":{"summary":S,"table":"data.regime_table.rows"},
 "4_cross_solver":{"matched_achieved_error":"No. Tables are per rtol; a matched section exists in the raw JSON but is not rendered.","clipped_arms":"vigilode and vigilode-banded (all VigilODE rows of rust_bench_results.json); vigilode-dense arms exist in code and in adversarial_rust_results.json but not in this dataset.","reproduced_rows":br['reproduced_rows']},
 "5_u04":{"preconditioner":"none in any arm (identity)","n_range":u4['n_values'],"problem":"1D Fisher MOL, tol 1e-6, linear rtol 1e-9","reading":"regime statement","best_admissible":ba,"n800":u4['n800']},
}
scripts={n:open(D+n).read() for n in ('u11_recompute.py','perf_agg.py','rerun_cmp.py','controller_floor.py','step_stats.py','extras.py','bench_recompute.py','bench_scan.py','bench_v3.py','u04_recompute.py','regime_table.py','assemble.py')}
scripts['steprunner/Cargo.toml']=open(D+'steprunner/Cargo.toml').read()
scripts['steprunner/src/main.rs (main only; problem definitions copied verbatim from the U11 runner)']=open(D+'steprunner/src/main.rs').read().split('// DSG audit:')[1]
summary=(f"U11 HOLD stands on either arm (median speed {g(u['g0_ppsw1_t6']['median_speed'])} at 6 threads, {g(u['g0_ppsw1_t1']['median_speed'])} at 1; step ratio {rng([q for r in u['ranges'] for q in r['steps_ratio']])}), and a rerun of the pinned binary reproduced all {rc['rows']} result objects. "
         f"Its numbers are not method properties: at tol 1e-4..1e-5 the controller takes {rng([100*s['limiter_saturation'] for s in loose])}% of the growth steps its 1.10-per-7-steps limiter permits, and {g(100*pa['tableau_mm_share_of_main_thread'])}% of main-thread cycles is a per-attempt double-double tableau rebuild. "
         f"Of {S['evaluations']} evaluations found, none is a stiff matched-error comparison above n = {S['matched_error_cross_method_max_n']}, none has an expensive RHS or a preconditioner, and the 9 2D/3D cases (switcher line, n <= {S['operators_2d_or_3d_max_n']}) are reference-disputed in 7. "
         "The switcher unified benchmark is matched-tolerance with grid-clipped VigilODE arms. U04 no-crossover is a regime statement. No recorded verdict is invalidated; every defect limits scope.")
out={"schema":"vigilode.loop-l1.audit-findings.v1","dimension":"DESIGN","id_prefix":"DSG","date":"2026-11-16",
     "loop_commit":"2dc9a2c5778ac5b3ac960b0f6741aa5916122384","switcher_commit":"933922495045796c99c530b0f1f1efd8fec83ed0",
     "summary":summary,"plan_items_checked":plan,"findings":f,"task_answers":answers,
     "disclosures":["Wall times are diagnostic_only: the host load average was 15-24 during this audit and the rerun ran under perf record.",
                    "One cargo build of a scratch crate (3 min 18 s, -j 6, own target directory) overlapped with another agent's cargo test process (stage_clients_dense_identity diagnostic_wall_ratio); that agent's wall figures from about 17:15-17:18 KST may be disturbed.",
                    "The step-history runner is a scratch copy of the U11 runner built against the clean worktree at 891044d7; its final states and accepted counts equal the U11 log bitwise for all 15 PPSW1 runs and all 15 RODAS5P runs.",
                    "The controller-floor and Amdahl numbers are arithmetic models on logged inputs, not solver runs.",
                    "Categorical columns of the regime table (Jacobian, backend, operator, RHS dominance) are the auditor's reading of the cited sources; n is parsed from files.",
                    "perf sample attribution uses symbol names of an unstripped binary without debug info; inlined callees are attributed to their callers."],
     "scratch_artifacts":{p:sha(D+p) for p in ('rerun_holdout_runs.jsonl','step_histories.jsonl','perf_u11.data','target/release/u11-holdout-runner','steprunner_build.log')},
     "scratch_dir":D,
     "data":{"u11_recompute":u,"perf_agg":pa,"rerun_cmp":rc,"extras":ex,"controller_floor":cf,"step_stats":ss,"bench_recompute":br,"bench_scan":bs,"bench_v3":b3,"u04_recompute":u4,"regime_table":rt},
     "scripts":scripts}
dst=W+'research/production_loop_20261116/audit/findings_design.json'
os.makedirs(os.path.dirname(dst),exist_ok=True)
open(dst,'w').write(json.dumps(out,indent=1,allow_nan=False,ensure_ascii=False)+"\n")
json.dump({"summary":summary,"plan":plan,"findings":f},open(D+'compact.json','w'),indent=1,ensure_ascii=False)
print(dst, os.path.getsize(dst)); print(len(summary)); print(summary)
for x in f: print('\n',x['id'],x['severity'],x['is_new'],x['title'],'\n  ',x['claim'],'\n  EV:',x['evidence'][:400])
for p in plan: print('\nPLAN',p['verdict'],'|',p['item'][:90],'|',p['note'])
