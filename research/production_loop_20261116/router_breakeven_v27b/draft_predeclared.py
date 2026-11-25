#!/usr/bin/env python3
"""Write the DRAFT PREDECLARED.json of V30 (node V30_ROUTER_BREAKEVEN_V27B) from fresh_set.py, the
definition checks (logs/fresh_set_checks.json) and the E7 diagnosis (E7_DIAGNOSIS.json).

The draft is reviewed, committed and pushed by the orchestrator; it is frozen by that commit.
Usage: python3 -I draft_predeclared.py   (prints the sha256 of the written file)
"""
import hashlib
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fresh_set as fs  # noqa: E402

V27 = os.path.join(HERE, "..", "router_hyperparameter_free")
OUT = os.path.join(HERE, "PREDECLARED.json")
MAX_STEPS = 100000


def sha256(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def w_bound(rho, span):
    """The V03b work bound of unified_router_contracts.rs, in the same f64 arithmetic."""
    return math.floor(0.1 * 6.0 * rho * (span[1] - span[0]) / 3.3)


def main():
    checks = {r["id"]: r for r in json.load(open(os.path.join(HERE, "logs", "fresh_set_checks.json")))["problems"]}
    diag = json.load(open(os.path.join(HERE, "E7_DIAGNOSIS.json")))
    problems = fs.problems()

    def entry(p):
        e = {"id": p["id"], "n": p["n"], "kind": p["kind"],
             "autonomous": p["autonomous"], "jacobian": p["jacobian"],
             "definition": p["definition"], "y0": p["y0"], "span": p["span"],
             "tolerance_pairs": p["tolerances"], "reference": p["reference"], "reason": p["reason"]}
        return e

    groups = {}
    for p in problems:
        groups.setdefault(p["group"], []).append(entry(p))

    # the cases, in order
    cases = []
    for p in problems:
        for tol in p["tolerances"]:
            cases.append(f"{p['id']}|{tol}|default")
    variants = [
        "chem_d4|A|regime=stiff", "chem_d4|A|regime=nonstiff",
        "kepler_e05|A|regime=stiff", "kepler_e05|A|regime=nonstiff",
        "chem_d4|A|max_steps=500", "chem_d4|A|max_steps=50",
    ]
    cases += variants

    declared_cases = ["sr_chem_dense|A|default", "sr_pr_dense_ft|A|default", "sr_ac_jvp|A|default",
                      "sr_ac_band|A|default", "cb_blowup|A|default", "cb_mass|A|default",
                      "chem_d4|A|regime=stiff", "chem_d4|A|regime=nonstiff",
                      "kepler_e05|A|regime=stiff", "kepler_e05|A|regime=nonstiff"]
    undeclared = [c for c in cases if c not in declared_cases]

    # expectations without rule, per problem (no arm ran on any of these problems)
    def t_route(p):
        if p["group"] == "callback_budget_declared" and p["id"] not in ("cb_probe", "cb_column", "cb_time"):
            return "declared fact: RODAS5P from t0 (N equals T)"
        if p["n"] > 10:
            return "RODAS5P from t0 (dimension above the trial limit 10)"
        return "screen at t0, then RODAS5P or the DOPRI5 trial of min(1000, max_steps/2) attempts"

    expectations = []
    risk_high = {"kaps_1e6", "pre_equilibrium"}
    risk_moderate = {"pendulum_tracker"}
    for p in problems:
        c = checks.get(p["id"], {})
        st = c.get("stiffness", {})
        S = st.get("dopri5_stability_attempts_S")
        row = {"problem": p["id"], "b": fs.break_even_length(p["n"]),
               "dopri5_stability_attempts_S": S,
               "dopri5_expected": None if S is None else ("feasible" if S < 0.9 * MAX_STEPS else "infeasible: MaxStepsReached"),
               "T_route_expected": t_route(p)}
        if p["group"] == "non_stiff":
            row["handover_expected"] = "no (lost pilots only); N = Dopri5::solve plus pilots (E3 a)"
            row["E7_risk"] = "none while no handover (err(N) = err(D))"
        elif p["group"] == "stiff_or_late_stiff":
            row["handover_expected"] = ("uncertain (mild stiffness, grey zone B4)"
                                        if p["id"] in ("b1_linear4", "c1_nonlinear4", "burgers_80", "ac_100")
                                        else "yes")
            if p["id"] in risk_high:
                row["E7_risk"] = ("high: DOPRI5 infeasible, so the bar is 3 err(R) alone, and the end "
                                  "error of RODAS5P on a smooth slow manifold is the last step's "
                                  "truncation error (the P_scalar_fd mechanism of E7_DIAGNOSIS.json)")
            elif p["id"] in risk_moderate:
                row["E7_risk"] = ("moderate: DOPRI5 infeasible (bar 3 err(R)), but RODAS5P's steps stay "
                                  "accuracy-limited up to t1")
            else:
                row["E7_risk"] = ("low: DOPRI5 feasible, its stability-limited error enters the bar "
                                  "(0.1 to 0.7 tolerance units on the V27 stiff problems)")
        else:
            row["handover_expected"] = {
                "cb_probe": "possible; N succeeds (no off-trajectory call)",
                "cb_column": "no: D1 ends the solve at the first pilot",
                "cb_time": "no; the run stops on the fatal callback after t = 0.4",
            }.get(p["id"], "not applicable (declared fact)")
            row["E7_risk"] = {
                "cb_probe": "none while no handover (N succeeds; T is expected to fail at t0)",
                "cb_column": "not evaluated: N ends by D1 (E7 needs N to reach t1)",
                "cb_time": "not evaluated: no arm reaches t1 (no reference)",
                "cb_blowup": "not evaluated: no arm reaches t1 (no reference)",
            }.get(p["id"], "none: N is expected to equal T and R bit for bit (declared fact, the "
                           "RODAS5P leg is the direct call)")
        expectations.append(row)

    v27_pre = os.path.join(V27, "PREDECLARED.json")
    draft = {
        "schema": "vigilode.loop.predeclared.v1",
        "node": "V30_ROUTER_BREAKEVEN_V27B, evaluation (phase 2) of the forward break-even pilots "
                "(FBP) Auto route on a fresh problem set",
        "date_kst": "2026-11-25",
        "status": "DRAFT written in phase 1 by the builder; the orchestrator reviews it, commits and "
                  "pushes it; it is frozen by the commit that adds it on origin; no run of the "
                  "deciding set (any arm on any case below) before that commit is on origin",
        "owner_decision": "OD-L1-26 (2026-11-25 09:21 KST, verbatim '남은 노드들을 이어서 마저 진행해. "
                          "수정은 전부 승인할게.'): run V27b: fix the two conflicting sentences of the "
                          "V27 pre-registration (E1 over undeclared cases only, E3 without the runs "
                          "that owner decision D1 ends), diagnose E7 (P_scalar_fd) and correct the "
                          "design if needed, and evaluate on a fresh problem set under a new "
                          "pre-registration (node V30). Owner decisions D1 to D6 of OD-L1-22 stand.",
        "basis": {
            "v27_predeclared": {"file": "research/production_loop_20261116/router_hyperparameter_free/PREDECLARED.json",
                                "sha256": sha256(v27_pre)},
            "design": {"file": "research/production_loop_20261116/router_hyperparameter_free/DESIGN_RECOMMENDED.md",
                       "sha256": sha256(os.path.join(V27, "DESIGN_RECOMMENDED.md"))},
            "v27_results_and_review": ["research/production_loop_20261116/router_hyperparameter_free/RESULTS.json",
                                       "research/production_loop_20261116/router_hyperparameter_free/REVIEW.json"],
            "e7_diagnosis": {"file": "research/production_loop_20261116/router_breakeven_v27b/E7_DIAGNOSIS.json",
                             "sha256": sha256(os.path.join(HERE, "E7_DIAGNOSIS.json")),
                             "route_defect": diag["route_defect"], "cause": diag["cause"]},
            "definition_checks": {"file": "research/production_loop_20261116/router_breakeven_v27b/logs/fresh_set_checks.json",
                                  "sha256": sha256(os.path.join(HERE, "logs", "fresh_set_checks.json")),
                                  "note": "scipy and mpmath only (closed forms, well-posedness, a stiffness "
                                          "estimate, the stated derivative callbacks); no VigilODE driver ran"},
        },
        "algorithm": "unchanged from V27 (PREDECLARED algorithm section, DESIGN_RECOMMENDED sections 3 "
                     "and 4): the E7 diagnosis found no defect of the route, so no route code changes. "
                     "Code: branch research/production-loop-20261116-w4-v30 = the V27 implementation "
                     "(60d67729; crates/vigilode/src, the session pass-through and both contract test "
                     "files byte-identical to it) merged with the loop head d4e2772f; the diff of the "
                     "crates against d4e2772f is exactly the 13 files of the FBP change "
                     "(router_hyperparameter_free/fbp_implementation.patch)",
        "arms": {
            "T": "today's Auto route, built from a read-only checkout of the loop-branch commit that "
                 "N's branch merged last (now d4e2772f; its crates/vigilode/src equals that of V27's "
                 "arm T, 1ffbda20); if the loop head is merged again before phase 2, T moves with it",
            "N": "FBP, built from the head of research/production-loop-20261116-w4-v30 (commit recorded "
                 "in RESULTS.json); before the first deciding run, git diff of crates/ between the T "
                 "and N checkouts must list exactly the 13 files of the FBP change "
                 "(router_hyperparameter_free/fbp_implementation.patch), else phase 2 stops",
            "D": "Dopri5::solve with the same options",
            "R": "Rodas5p::solve with the same options",
            "sanity": "D and R built from both checkouts must agree bit for bit (final time and state "
                      "bits, status, counters, attempts, closure entries by kind); a disagreement is "
                      "reported and stops the evaluation",
            "exploratory_without_rule": {
                "R_H": "for every case whose run 1 of N hands over: Rodas5p::solve from t0 with "
                       "h0 = H of the accepted pilot and otherwise the same options; its error and "
                       "last-step length are reported next to E7 (the E7 diagnosis predicts that "
                       "err(R_H) / err(R) is of the size of err(N) / err(R)); no rule reads it",
            },
        },
        "problems": {
            "note": "fresh: disjoint from the 36 problem ids of V27 (and from their families) and from "
                    "every problem that the FBP route has run on (the fixtures of "
                    "tests/unified_breakeven_contracts.rs, tests/unified_router_contracts.rs, "
                    "tests/unified_production_solver_contracts.rs, examples/unified_production_solver.rs, "
                    "the V27 runner's toy problems and the V30 E7 diagnosis of P_scalar_fd); no problem "
                    "below has been run with the FBP route or with any arm before this file is on "
                    "origin. Finite-difference Jacobian unless stated; 'autonomous' means the problem "
                    "is declared autonomous (Problem::autonomous), otherwise the drivers take the "
                    "central-difference time derivative unless a time-derivative callback is stated. "
                    "Fatal errors are CallbackError::Fatal returned by the right-hand side.",
            "stiff_or_late_stiff": groups["stiff_or_late_stiff"],
            "non_stiff": groups["non_stiff"],
            "callback_budget_declared": groups["callback_budget_declared"],
            "declared_regime_variants": "chem_d4 and kepler_e05 at A, each with declared regime Stiff "
                                        "and with declared regime Nonstiff (VigilodeOptions::regime)",
            "budget_variants": "chem_d4 at A with max_steps 500 and with max_steps 50",
            "excluded_v27_ids": sorted(["F2", "N2", "P_blowup", "P_fatal", "P_heat_banded", "P_heat_dense",
                                        "P_heat_fd", "P_heat_jvp", "P_mass", "P_scalar_dense", "P_scalar_fd",
                                        "arenstorf", "brusselator_2", "damped_osc", "flame_1e-2", "flame_1e-3",
                                        "flame_1e-4", "heat_64_fd", "hires", "kt_vanishing", "lin_2000",
                                        "lin_3200", "lin_3400", "lorenz", "lorenz96_40", "lotka_volterra",
                                        "nonnormal", "orego", "osc_1", "osc_10", "osc_1000", "pleiades",
                                        "robertson_1e5", "robertson_40", "vdp_1", "vdp_1000"]),
        },
        "tolerances": {"A": [1e-6, 1e-9], "L": [1e-3, 1e-6], "S": [1e-8, 1e-11],
                       "assignment": "A for every problem; L and S also for every non_stiff problem and "
                                     "for pr_sin2t, a3_linear4, chem_d4, late_linear4, fhn_relax (five "
                                     "stiff or late-stiff problems, as V27 had five); the declared-regime "
                                     "and budget variants at A only"},
        "options": "Options::default() (h0 1e-3, max_steps 100000, no dense output), strategy Auto, "
                   "regime Unknown, unless a variant states otherwise (as V27)",
        "cases": {"count": len(cases), "ids": cases,
                  "declared": declared_cases, "undeclared_count": len(undeclared),
                  "case_id_format": "<problem>|<A|L|S>|<default | regime=stiff | regime=nonstiff | "
                                    "max_steps=500 | max_steps=50>"},
        "measures": [
            "charged callbacks by kind and callback entries counted inside the closures (stage, "
            "finite-difference column, time derivative), as V27",
            "callbacks unit cb = rhs_calls + jvp_calls + ft_calls + charged Jacobian callbacks "
            "(counters.jacobian_builds where the problem has a Jacobian callback), as V27; reported "
            "as well without ft_calls where the problem has no time-derivative callback (V27 review "
            "R2), no rule reads that variant",
            "operations by one formula for all arms: 52 n per DOPRI5 attempt, 128 n per RODAS5P "
            "attempt, 2 n^3/3 per factorization, 2 n^2 per solve pair and per finite-difference build",
            "attempts; pilots (number, verdicts, H, window cost, pilot cost); handover time and DOPRI5 "
            "attempts before it; status and typed stop",
            "error at t1 against the reference of the problem: closed form evaluated with mpmath at 50 "
            "digits and rounded to the nearest double, else Rodas5p::solve at rtol 1e-12, atol 1e-14; "
            "norm: weighted RMS with weights atol + rtol |y_ref_i| of the run's tolerances (V27)",
            "the last accepted step of the RODAS5P part (length and start) of N, R and R_H, read by "
            "prefix replays as in E7_DIAGNOSIS.json (no rule)",
            "final-state bits of three runs (two in one process, one in a fresh process) and the "
            "route digest",
        ],
        "discriminators": {
            "E1_economy": "over all cases (problem x tolerance x options) where T and N succeed (no case "
                          "is excluded): geometric mean of cost(N)/cost(T) < 1 in callbacks and in "
                          "operations; and, over the undeclared cases among them, max of "
                          "cost(N)/min(cost(D), cost(R)) < the same maximum for T, in each unit (min over "
                          "those of D and R that succeed; a case where neither succeeds enters the "
                          "geometric means but not the maxima). Undeclared case: neither the problem "
                          "nor the options declare a fact that decides the route (regime Unknown, "
                          "strategy Auto, no Jacobian, JVP, band or mass matrix supplied), i.e. every "
                          "case of cases.ids that is not in cases.declared; for N its analysis basis is "
                          "break-even (a mismatch is reported). CORRECTED from V27, where the maxima "
                          "also ran over declared cases on which N and T are the same run",
            "E2_no_new_failure": "no case where T succeeds and N does not, at the same max_steps "
                                 "(unchanged)",
            "E3_neutrality": "(a) every N run without a handover, except the runs that owner decision "
                             "D1 ends (see E3_excluded_runs), has the final-state bits and the DOPRI5 "
                             "counters of D plus exactly the pilots' counters, and cost <= 2 cost(D) in "
                             "each unit; (b) every run decided by a declared fact (cases.declared) "
                             "equals T bit for bit. CORRECTED from V27, where (a) also required D's "
                             "bits of the N2 run that D1 and E8 end at its first pilot",
            "E3_excluded_runs": "exactly the N runs whose last pilot record has verdict 'fatal' and "
                                "whose stop is CallbackFailed (owner decision D1). A priori members: "
                                "the three N runs of cb_column|A|default (its right-hand side fails "
                                "only at a finite-difference column, which every pilot builds); the "
                                "three N runs of cb_time|A|default only if one of its pilots reaches "
                                "t > 0.4 before DOPRI5 does. No other problem has a fatal callback. "
                                "The parser lists the members found; each is reported with its pilot "
                                "record",
            "E4_budget": "attempts over all legs and pilots <= max_steps; exhaustion reports "
                         "MaxStepsReached{max_steps} (unchanged)",
            "E5_callbacks": "no closure entry before DOPRI5's first stage; no entry with t = t0 except "
                            "DOPRI5's stages; every entry that is not a stage is a finite-difference "
                            "column or time-derivative call of a RODAS5P attempt that starts at an "
                            "accepted state; uncharged entries = 0 (unchanged; readings below)",
            "E6_determinism": "the three runs have equal digests, counters and bits (unchanged)",
            "E7_accuracy": "err(N) <= 3 max(err(D), err(R)) over those of D and R that succeed, on "
                           "every case with a reference where N succeeds (rule unchanged; V27 excluded "
                           "kt_vanishing, this set has no report-only problem)",
            "E8_callback_cases": "cb_probe succeeds under N; cb_time reports CallbackFailed{Rhs, "
                                 "'logger offline'}; cb_column ends with CallbackFailed at its first "
                                 "pilot (D1): one pilot record, verdict fatal. Rule of V27 on the fresh "
                                 "analogues of F2, P_fatal and N2",
            "E9_v03b_bounded_work": "pr_sin2t|A|default <= %d and a3_linear4|A|default <= %d charged "
                                    "right-hand-side calls, each with a recorded handover; the V03b "
                                    "bound W = floor(0.1 x 6 x rho x (t1 - t0) / 3.3) of "
                                    "unified_router_contracts.rs (rho = 1e4 for both) on the fresh "
                                    "analogues of P_scalar_fd and P_heat_fd (also reported at L and S "
                                    "without a rule)" % (w_bound(1e4, (0.0, 5.0)), w_bound(1e4, (0.0, 20.0))),
            "E10_pilot_accounting": "pilots <= 1 + floor(log2(A_D / b)) and pilot spend <= DOPRI5 "
                                    "spend in each unit, on every run (unchanged; reading R6 below)",
        },
        "readings_fixed_before_any_run": [
            "E3 (a) pilots' counters: the records' pilot_attempts, pilot_callbacks (charged rhs + ft) "
            "and pilot_operations; the DOPRI5 leg's counters field by field against D (V27 addendum 0b)",
            "E5 (i) per kind: uncharged = max(0, entries - charged counter); a counter above its "
            "entries (ft_calls of finite-difference time derivatives and of mass-matrix problems) is "
            "reported, not a failure (V27 erratum E5-i, review R3, now fixed before any run)",
            "E5 (ii)/(iii) apply to N runs that start with DOPRI5 (break-even route, declared "
            "Nonstiff) and are decided by the runner's replay split e5 (V27 addendum 0b); runs that "
            "start RODAS5P by a declared fact are covered by E3 (b)",
            "E10 is evaluated on every N run with at least one pilot; runs without a pilot are listed "
            "as 'no pilot', and runs with A_D = 0 as 'count bound not evaluable' next to the verdict "
            "(V27 R6 and review R4)",
            "E1 maxima only over cases where D or R succeeds; E7 only where N succeeds and has an "
            "error (a reference exists and N reached t1) (V27 R3, R4)",
            "the also-tolerances apply to the listed problems with the default options only (V27 R5)",
            "a run 'succeeds' iff run.success is true and the process exited 0 with its lines",
        ],
        "declared_expected_losses": "not failures: on problems stiff from t0 where T starts RODAS5P "
                                    "at t0 (today's screen fires: pr_sin2t, kaps_1e6, a3_linear4, "
                                    "chem_d4, decay_chain6, pre_equilibrium, pendulum_tracker; n > 10: "
                                    "lin20_dense), N pays a DOPRI5 prefix, the pilot and RODAS5P's ramp "
                                    "from h0 = H: up to about 2x T in callbacks (V27: P_scalar_fd 145 "
                                    "vs 74, P_heat_fd 945 vs 723, lin_3400 687 vs 436, heat_64_fd 3328 "
                                    "vs 2993); short non-stiff runs pay lost pilots (V27: +1.9 to +20 "
                                    "percent in callbacks, up to +43 percent in operations at L)",
        "frozen_decision": "the Auto route becomes N (FBP) iff E1 to E10 all pass on this set; "
                           "otherwise today's route stays and the FBP code stays on its builder branch "
                           "(not merged). E3, E4, E5, E6 or E10 failing refutes the unconditional "
                           "properties (design or implementation defect); E1 failing refutes the "
                           "economic claim; E2 or E7 failing blocks adoption. The default path "
                           "(Rodas5p::solve) and its golden files are not part of the decision and "
                           "must stay unchanged",
        "run_settings": "as V27: every cargo command under the host-wide build lock, taskset -c "
                        "0-5,12-17, cargo +1.94.1 --locked -j 4 --release, node-owned CARGO_TARGET_DIR "
                        "(w4-v30 for N, w4-v30base for T); the evaluation runner is a standalone cargo "
                        "package router_breakeven_v27b/runner: a copy of router_hyperparameter_free/"
                        "runner (src/main.rs, its lock with the root package renamed) with the problems "
                        "above added and an option --h0 (arm R only, for R_H); built once against each "
                        "checkout; per case and arm one process with --repeat 2 and a fresh process with "
                        "--repeat 1, entry log on, E5 replay split for N; D and R once more from the T "
                        "build (sanity); one heavy process at a time; counters and bits only, no wall "
                        "time; raw stdout, stderr and exit code of every process kept and indexed with "
                        "UTC start times",
        "order_of_work": [
            "0 (before any deciding run): PHASE2_NOTES.txt with the computation rules; the runner "
            "built and checked on the V27 toy problems (toy, toy_stiff, toy_osc, toy_ft, toy_fatal), "
            "which are outside this set",
            "1: references (closed forms by mpmath; the others by Rodas5p::solve at rtol 1e-12, atol "
            "1e-14 from the N build)",
            "2: every arm on every case (deciding runs), then R_H",
            "3: RESULTS.json (strict JSON) and REPORT_KO.md from a parser that reads only raw runner "
            "output and the references",
            "4: on the N head: unified_breakeven_contracts, unified_router_contracts, "
            "default_path_golden, the full facade suite with --no-fail-fast, clippy -D warnings, fmt; "
            "default-path golden files unchanged",
            "5: independent review",
        ],
        "expectations_without_rule": {
            "source": "no arm and no model ran on these problems; the values come from the definitions, "
                      "the scipy stiffness estimate S of logs/fresh_set_checks.json (attempts DOPRI5 "
                      "needs at its real stability limit; max_steps is 100000), today's screen rule "
                      "(L (t1 - t0) > 3.3 min(1000, max_steps/2) at t0, n <= 10) and the V27 results",
            "per_problem": expectations,
            "E7": "V27 (raw lines of RESULTS.json): of the 23 cases with a handover and an E7 row, "
                  "err(N)/err(R) exceeded 3 on 8; E7 failed only on 2 of the 7 of them where DOPRI5 "
                  "failed (P_scalar_fd at A and L), because elsewhere DOPRI5's error (0.095 to 0.745 "
                  "tolerance units on the 18 handover cases where it succeeded) set the bar. Here "
                  "DOPRI5 is expected to fail on kaps_1e6, pendulum_tracker, pre_equilibrium and the "
                  "two budget variants (A only); by the E7 diagnosis (h0 scan: 31 of 41 step "
                  "sequences of Rodas5p::solve alone exceed 3 times the default's error on "
                  "P_scalar_fd) an E7 failure on kaps_1e6 or pre_equilibrium is likely for a reason "
                  "that is not a defect of the route; R_H is reported to show it",
            "E1": "large gains expected where T runs RODAS5P on non-stiff or mildly stiff problems: "
                  "fput_30 and kuramoto_50 (n > 10), kepler_e09 (today's screen fires at the "
                  "pericentre start, L about 2000), b1_linear4 (a column norm 1e4 at t0 although the "
                  "spectral radius is 141), pr_ramp (k(0) = 0: today's screen is silent and the DOPRI5 "
                  "trial exhausts 1000 attempts); losses as declared above; the geometric means are "
                  "expected below 1 and N's worst ratio to the better reference near 2 (V27: 1.98)",
            "E2": "the budget case chem_d4|A|max_steps=50 is the one where N may fail while T "
                  "succeeds (DOPRI5 prefix, pilot and RODAS5P ramp from h0 = H within 50 attempts)",
            "E9": "pr_sin2t: a few hundred charged right-hand sides (V27 P_scalar_fd: 145 against "
                  "181818); a3_linear4: a handover during the initial transient",
        },
        "changes_from_v27": [
            "E1: maxima over undeclared cases only (owner decision OD-L1-26)",
            "E3: runs ended by owner decision D1 excluded from (a), with the exact rule and a priori "
            "members (OD-L1-26)",
            "E8 and E9: rules unchanged, instantiated on fresh analogues (the V27 problems they named "
            "are not in a disjoint set; the V27 instances stay covered by the contract tests of step 4)",
            "E5, E10: the computation readings that V27 fixed in addendum 0b, erratum E5-i and review "
            "R2 to R4 are fixed here before any run",
            "fresh problem set (37 problems, 79 cases) replaces the 36 problems of V27",
            "exploratory arm R_H and the last-step measures (no rule), from the E7 diagnosis",
            "no route code change: the E7 diagnosis found no defect of the route",
        ],
    }
    with open(OUT, "w") as f:
        json.dump(draft, f, indent=1, ensure_ascii=False, allow_nan=False)
        f.write("\n")
    print(sha256(OUT), len(cases), "cases", len(problems), "problems", len(undeclared), "undeclared")


if __name__ == "__main__":
    main()
