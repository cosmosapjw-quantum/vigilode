#!/usr/bin/env python3
"""Build COMPONENTS.json and CORPUS.json of the component triage node.

The component table below is the preregistered inventory (base commit da91553).
LOC is recomputed from the files at build time, so the manifest binds to the
tree it was generated from. Run from the repository root:

    python3 research/component_triage_20261019/build_manifests.py [TRIAGE_LIST.json]

With the `rodas5p triage-list` output given, the arms of every component are
taken from the binary and CORPUS.json is written as well.
"""
import json
import pathlib
import subprocess
import sys

NODE = pathlib.Path(__file__).resolve().parent
ROOT = NODE.parents[1]
I = "crates/rodas5p-integrators/src/"
C = "crates/rodas5p-core/src/"
K = "crates/rodas5p-krylov/src/"

# Prior evidence that the reuse rule (PREREGISTRATION.md section 4) admits.
# reuse=False means the verdict is recorded but must be re-measured.
def prior(ledger, verdict, reuse, why):
    return {"ledger": ledger, "verdict": verdict, "reuse": reuse, "reuse_reason": why}

ME01 = "ME01 rival rule compared same-rtol warm Ir without the matched-error band (PERFORMANCE_AUDIT_KO.md); re-measure"
SAT = "allocation figures came from allocation_audit warm saturating_sub; re-measure"

# (id, family, slot, files, reach, selector, capability, tiers, arms, priors,
#  blocked_by, static_class, unique_capability, notes)
T = []
def comp(*row):
    T.append(row)

# ---------------------------------------------------------------- F1 drivers
comp("drv.seq_direct", "F1", "driver", [I+"sequential.rs", I+"integrate.rs"], "D",
     "integrate_adaptive_observed_with_config(Sequential, LinearMethod::Direct)",
     "stiff-ivp", ["G", "P"], ["seq-direct"], [], [], None, False,
     "documented default B (LinearSolverConfig::default() is Direct)")
comp("drv.seq_krylov", "F1", "driver", [I+"sequential.rs"], "O",
     "LinearMethod::{Gmres,Lgmres,Gcrodr} with explicit Jacobian",
     "stiff-ivp", ["G", "P"], ["seq-gmres-pc-none", "seq-gmres-pc-jacobi", "seq-gmres-pc-direct", "seq-lgmres", "seq-gcrodr"],
     [], [], None, False, "")
comp("drv.seq_mf", "F1", "driver", [I+"sequential.rs", I+"dense_output_v2.rs"], "D",
     "integrate_sequential_matrix_free_adaptive_dense_observed",
     "stiff-ivp-jvp", ["G", "P"], ["seq-mf-gmres", "seq-mf-lgmres", "seq-mf-gcrodr"],
     [prior("L-0136", "SCOPED_USE_REMAINS_ALLOWED", True, "scope decision, not a performance claim")],
     [], None, False, "documented default A (README); refuses Direct")
comp("drv.fast", "F1", "driver", [I+"rodas5p_fast.rs"], "O", "integrate_rodas5p_fast_observed",
     "stiff-ivp", ["G", "P"], ["fast"],
     [prior("L-0032", "PASS", False, "wall-time diagnostic only"), prior("L-0041", "PASS", False, "wall-time diagnostic only")],
     [], None, False, "lean U-form dense driver; RODAS5P still 1-3.7x behind Hairer (L-0032)")
comp("drv.fast_banded", "F1", "driver", [I+"rodas5p_fast.rs"], "O", "integrate_rodas5p_fast_banded_observed",
     "stiff-ivp-banded", ["P"], ["fast-banded-indexed", "fast-banded-slices"],
     [prior("L-0083", "PASS", False, ME01)], [], None, True, "E4/SPD03; declared band only")
comp("drv.fast_small", "F1", "driver", [I+"rodas5p_fast_small.rs"], "O", "SmallProblem trait (compile-time N)",
     "stiff-ivp-small-n", ["G"], ["fast-small"], [], [], None, False, "")
comp("drv.fast_small_batch", "F1", "driver", [I+"rodas5p_fast_small_batch.rs"], "O", "BatchProblem lanes",
     "stiff-ensemble", ["G"], [],
     [prior("L-0089", "FAIL", True, "kill gate on identical outputs; methodology independent of matched error")],
     [], None, True, "SPD08; ensemble workload has no triage-run cell")
comp("drv.mf_fast", "F1", "driver", [I+"rodas5p_matrix_free_fast.rs"], "O", "integrate_rodas5p_mf_fast_observed_*",
     "stiff-ivp-jvp", ["G", "P"], ["mf-into", "mf-ls-ws", "mf-legacy-v2", "mf"], [], [], None, False, "U-form JVP-only driver")
comp("drv.routed", "F1", "driver", [I+"routing.rs"], "O", "integrate_rodas5p_routed_observed",
     "stiff-ivp", ["G", "P"], ["routed", "routed-undeclared"],
     [prior("L-0101", "FAIL", False, "Ir 1.074/1.035 at n=1000 from dense band verification; re-measure after harness fix")],
     [], None, False, "SP03; ProblemStructure::LinearPart never routed")
comp("drv.sabr", "F1", "driver", [I+"sabr.rs", I+"block.rs"], "O", "IntegrationMethod::Sabr + SabrConfig",
     "stiff-ivp", ["G", "P"], ["sabr-gmres", "sabr-forward", "sabr-explicit", "sabr-nilpotent"], [], [], None, False, "")
comp("drv.homotopy", "F1", "driver", [I+"homotopy.rs", I+"homotopy_policy.rs", I+"certification.rs"], "O",
     "integrate_homotopy_adaptive_observed", "stiff-ivp-client", ["G", "C"], ["homotopy"],
     [prior("L-0113", "PASS/ABSTAIN", True, "client contract only"), prior("L-0114", "INCONCLUSIVE/ABSTAIN_NOT_RUN", True, "not run")],
     ["A09"], None, False, "")
comp("drv.transactional_q1q2", "F1", "driver", [I+"transactional_q1_q2.rs"], "O", "integrate_transactional_q1_q2_*",
     "stiff-ivp", ["G", "P"], ["transactional-q1q2"], [], [], None, False, "")
comp("drv.bdf", "F1", "driver", [I+"bdf.rs", I+"nonlinear.rs"], "O", "BdfConfig {One, Two}",
     "stiff-ivp", ["G", "P"], ["bdf1", "bdf2", "bdf2-tier-a"],
     [prior("L-0028", "PASS", False, "wall-time diagnostic only")], [], None, False, "labelled reference-implementation-only")
comp("drv.radau", "F1", "driver", [I+"radau.rs", I+"nonlinear.rs"], "O", "RadauConfig {One, Three}",
     "stiff-ivp", ["G", "P"], ["radau1", "radau5", "radau5-tier-a"],
     [prior("L-0029", "PASS", False, "wall-time diagnostic only")], [], None, False, "labelled reference-implementation-only")
comp("drv.exp_pexprb54s4", "F1", "driver", [I+"exponential.rs", I+"adaptive_exponential.rs"], "O",
     "integrate_pexprb54s4_fused_adaptive_*", "stiff-ivp-semilinear", ["G", "P"], ["pexprb54s4-fused"],
     [prior("EX01", "HOLD(rival)", True, "no native ROCK4 rival; direct rival dense-fast-colext64")],
     ["A11"], None, False, "SA-06 partial open")

# ------------------------------------------------------------- F2 controllers
for kind, arm, pri in [
    ("integral", "fast", []),
    ("pi", "fast-ctl-pi", []),
    ("predictive", "fast-ctl-predictive", [prior("L-0092", "FAIL", True, "accuracy FAIL (ALG03/04 family)")]),
    ("predictive_capped", "fast-ctl-predictive-capped", []),
    ("predictive_capped2", "fast-ctl-predictive-capped2", [prior("L-0103", "PASS", False, ME01)]),
]:
    comp("ctl." + kind, "F2", "controller", [I+"adaptive.rs"], "D" if kind == "integral" else "O",
         "ControllerKind::" + kind, "step-control", ["G", "P"], [arm], pri, [], None, False, "")
comp("ctl.inner_tolerance", "F2", "inner_tolerance", [I+"g4_s5b0_inner_tolerance.rs"], "D",
     "rodas5p_inner_forcing_target (fraction 0.1, floor 64 eps, eta_max 0.5)", "inner-solve-forcing",
     ["G", "P"], ["seq-mf-gmres"], [], [], None, True, "runtime despite the gate name; rename on relocation")
comp("ctl.max_step_slack", "F2", "max_step_policy", [I+"output.rs"], "D", "MaxStepPolicy::AllowClockResolutionSlack",
     "step-control", ["G"], ["seq-direct"], [], [], None, False, "")
comp("ctl.max_step_strict", "F2", "max_step_policy", [I+"output.rs"], "O", "MaxStepPolicy::StrictRepresentedCap",
     "step-control", ["G"], ["seq-direct-maxstep-strict"], [], [], None, False, "")
comp("ctl.fast_prevalidated", "F2", "fast_controller_path", [I+"rodas5p_fast.rs", I+"rodas5p_fast_small.rs"], "O",
     "Rodas5pFastOptions.prevalidated_controller", "step-control", ["G", "P"], [],
     [prior("L-0081", "PASS", False, ME01)], [], None, False, "SPD01/E3; arms are the stiff_benchmark OPTION_ARMS")
comp("ctl.fast_fused_landing", "F2", "fast_controller_path", [I+"rodas5p_fast.rs"], "O",
     "Rodas5pFastOptions.fused_landing", "step-control", ["G", "P"], [],
     [prior("L-0081", "PASS", False, ME01)], [], None, False, "SPD01/E3")

# ------------------------------------------------------------------ F3 output
comp("out.collector", "F3", "output_collector", [I+"output.rs"], "D", "OutputCollector / OutputSchedule",
     "output", ["G", "P"], ["seq-direct"], [], [], None, True, "")
for mode, reach in [("off", "D"), ("report_defect", "O"), ("report", "O"), ("enforce", "O")]:
    comp("out.dense_" + mode, "F3", "dense_error_control", [I+"dense_output_v2.rs"], reach,
         "DenseErrorControl::" + mode, "dense-output", ["G", "V"], ["seq-direct-dense-" + mode.replace("_", "-")],
         [prior("DP-01", "OPEN", True, "PR probe: Off normalized 1801.6, Enforce 1.4986")] if mode in ("off", "enforce") else [],
         ["A05"] if mode == "enforce" else [], None, False, "")
comp("out.verified_pr101", "F3", "dense_error_control", [I+"bound_pr101.rs", I+"dense_output_v2.rs"], "R",
     "A05 verified continuous-residual route", "verified-dense-output", ["V"], [],
     [prior("L-0135", "PASS", True, "scoped helper; A05 HOLD")], ["A05"], None, True, "no triage arm yet")

# ------------------------------------------------------------ F4 direct LA
comp("la.faer_lu", "F4", "dense_lu", [C+"matrix.rs"], "D", "LuFactorization (faer partial_piv_lu)",
     "dense-linear-solve", ["G", "P"], ["seq-direct", "fast"], [], [], None, False, "")
comp("la.small_lu", "F4", "dense_lu", [I+"rodas5p_fast.rs"], "O", "RODAS5P_FAST_SMALL_LU_MAX = 64",
     "dense-linear-solve", ["G"], ["fast"], [], [], None, False, "in-place LU below n=64 inside the fast driver")
comp("la.lu_colext", "F4", "dense_lu", [I+"rodas5p_fast.rs"], "O", "FastLuPolicy::ColumnExtents",
     "dense-linear-solve", ["G", "P"], [], [prior("L-0082", "FAIL", True, "kill gate on identical outputs")],
     [], None, False, "SPD02; subsumed by E6")
comp("la.lu_colext64", "F4", "dense_lu", [I+"rodas5p_fast.rs"], "O", "FastLuPolicy::ColumnExtentsAbove64",
     "dense-linear-solve", ["P"], [], [prior("L-0086", "PASS", False, ME01)], [], None, False, "SPD09/E6")
comp("la.sparse_density_switch", "F4", "dense_lu", [I+"rodas5p_fast.rs"], "O", "RODAS5P_FAST_SPARSE_DENSITY_MAX = 0.1",
     "dense-linear-solve", ["P"], ["fast"], [], [], None, False, "")
comp("la.banded_indexed", "F4", "banded_kernel", [I+"rodas5p_fast.rs"], "O", "BandedKernel::Indexed",
     "banded-linear-solve", ["P"], ["fast-banded-indexed"], [prior("L-0083", "PASS", False, ME01)], [], None, False, "")
comp("la.banded_slices", "F4", "banded_kernel", [I+"rodas5p_fast.rs"], "O", "BandedKernel::Slices",
     "banded-linear-solve", ["P"], ["fast-banded-slices"], [], [], None, False, "")
comp("la.sparse_direct", "F4", "sparse_lu", [C+"sparse_direct.rs"], "R", "SparseDirectSolver (no integrator caller)",
     "sparse-linear-solve", ["P"], [], [prior("SA-03", "FIXED", True, "validation repaired")], [], None, True,
     "not wired; ProblemStructure::LinearPart unused by drivers")
comp("la.woodbury_collective", "F4", "structured_lu", [I+"collective_direct.rs"], "R", "O(N) rank-two Woodbury stage solve",
     "collective-linear-solve", ["C"], [], [prior("L-0133", "PASS", True, "math scope")], ["A06", "A07"], None, True,
     "mandatory strong baseline for packet comparisons")
comp("la.stage_batch", "F4", "stage_parallel", [I+"stage_batch.rs"], "R", "thread-pool stage feasibility probe",
     "parallel-stages", ["P"], [], [], [], None, False, "synthetic probe")

# -------------------------------------------------------------- F5 Krylov
KR = [
    ("kr.gmres", "krylov_method", [K+"gmres.rs"], "D", "LinearMethod::Gmres", ["seq-mf-gmres", "seq-gmres-pc-none"], [], ""),
    ("kr.gmres_into", "krylov_method", [K+"gmres_into.rs"], "O", "solve_gmres_into", ["mf-into"], [prior("L-0045", "PASS", False, ME01)], "E8"),
    ("kr.gmres_staged", "krylov_method", [K+"gmres_staged.rs"], "O", "StageTargetPolicy != Legacy/DupFix", [], [], ""),
    ("kr.gmres_givens", "krylov_method", [K+"gmres_givens.rs"], "R", "no caller", [], [], "PR #19 merged into main"),
    ("kr.lgmres", "krylov_method", [K+"lgmres.rs"], "O", "LinearMethod::Lgmres", ["seq-mf-lgmres", "seq-lgmres"], [], ""),
    ("kr.lgmres_into", "krylov_method", [K+"lgmres_into.rs"], "O", "KrylovWorkspaceOptions.lgmres_into",
     [], [prior("L-0088", "PASS", False, "no driver caller at the time")], "SPD05"),
    ("kr.gcrodr", "krylov_method", [K+"gcrodr.rs"], "O", "LinearMethod::Gcrodr", ["seq-mf-gcrodr", "seq-gcrodr"],
     [prior("L-0059", "PASS", False, "recycling saves nothing; re-measure on P tier")], ""),
    ("kr.block_gmres", "krylov_method", [K+"block_gmres.rs"], "R", "only common_w_gate", [], [], ""),
    ("kr.ls_workspace", "krylov_least_squares", [K+"workspace.rs", K+"small.rs"], "O", "KrylovWorkspaceOptions.ls_workspace",
     ["mf-ls-ws"], [prior("L-0087", "PASS", False, ME01)], "E7/SPD04"),
    ("kr.accounting_reuse_confirmed", "krylov_residual_accounting", [K+"accounting.rs"], "O", "ResidualAccounting::ReuseConfirmed",
     [], [prior("L-0100", "PASS", False, ME01)], "SP01/E1"),
    ("kr.accounting_recompute_final", "krylov_residual_accounting", [K+"accounting.rs"], "D", "ResidualAccounting::RecomputeFinal",
     ["seq-mf-gmres"], [], ""),
    ("kr.workspace_options", "krylov_workspace", [I+"rodas5p_matrix_free_fast.rs"], "O", "KrylovWorkspaceOptions",
     [], [prior("L-0104", "PASS", False, "opt-in; workspace savings measured per instruction only")], "SP02"),
]
for kid, slot, files, reach, sel, arms, pri, notes in KR:
    comp(kid, "F5", slot, files, reach, sel, "krylov-linear-solve", ["G", "P"], arms, pri, [], None, False, notes)
for x0 in ["zero", "previous", "prevstep", "prevstep_scaled"]:
    comp("kr.x0_" + x0, "F5", "krylov_initial_guess", [C+"solver_types.rs", I+"sequential.rs"],
         "D" if x0 == "previous" else "O", "InitialGuess::" + x0, "krylov-linear-solve", ["G", "P"],
         ["seq-mf-gmres-x0-" + x0.replace("_", "-")], [], [], None, False, "")
for pc in ["none", "jacobi", "direct"]:
    comp("kr.pc_" + pc, "F5", "preconditioner", [C+"solver_types.rs", C+"operator.rs"], "D" if pc == "none" else "O",
         "PreconditionerKind::" + pc, "krylov-linear-solve", ["G", "P"], ["seq-gmres-pc-" + pc], [], [], None, False, "")
comp("kr.pc_right", "F5", "preconditioner_side", [K+"gmres_into.rs", C+"sparse_direct.rs"], "R", "PC01 right preconditioner",
     "krylov-linear-solve", ["P"], [], [prior("PC01", "HOLD(reference)", False, "6/6 cells reference-limited")], [], None, False, "")
for st, pri in [("legacy", []), ("projl2", [prior("L-0090", "FAIL", True, "accuracy FAIL")]),
                ("l2coupled", [prior("L-0093", "FAIL", True, "accuracy FAIL")]),
                ("coupled", [prior("L-0094", "FAIL", True, "accuracy FAIL")]),
                ("coupled_guarded", [prior("L-0097", "PASS/HOLD", False, "favourable h0 draw; robust accuracy unsupported")]),
                ("dupfix", []), ("coupled_guarded2", [prior("L-0098", "FAIL", True, "accuracy FAIL")])]:
    comp("kr.stage_target_" + st, "F5", "stage_target_policy", [I+"rodas5p_matrix_free_fast.rs", I+"raw_stage_target.rs"],
         "O", "StageTargetPolicy::" + st, "krylov-linear-solve", ["G", "P"], [], pri, [], None, False,
         "arm ids assigned by triage_arms.rs")
for rp in ["legacy", "refresh_after_update", "cold"]:
    comp("kr.recycle_" + rp, "F5", "gcrodr_recycle_policy", [I+"rodas5p_matrix_free_fast.rs", K+"gcrodr.rs"], "O",
         "GcrodrRecyclePolicy::" + rp, "krylov-linear-solve", ["P"], [], [], [], None, False, "")
comp("kr.start_explicit_zero", "F5", "krylov_start_policy", [I+"rodas5p_matrix_free_fast.rs"], "O", "KrylovStartPolicy::ExplicitZero",
     "krylov-linear-solve", ["P"], [], [prior("L-0106", "FAIL", True, "items 4-6 failed on identical-output gates")], [], None, False, "SP04")
comp("kr.recycle_scope_currentw", "F5", "recycle_scope", [I+"rodas5p_matrix_free_fast.rs"], "O", "RecycleScope::CurrentW",
     "krylov-linear-solve", ["P"], [], [], [], None, False, "")
comp("kr.mf_warm_start", "F5", "krylov_start_policy", [I+"rodas5p_matrix_free_fast.rs"], "O", "SPD07 warm start",
     "krylov-linear-solve", ["P"], [], [prior("L-0084", "FAIL", True, "kill gate")], [], None, False, "SPD07")

# ------------------------------------------------------------- F2b Newton / small
comp("nl.fixed", "F2", "newton_tolerance", [I+"nonlinear.rs"], "O", "NewtonTolerancePolicy::FixedNewtonConfig",
     "newton", ["G", "P"], ["bdf2", "radau5"], [], [], None, False, "BDF/Radau only")
comp("nl.scaled", "F2", "newton_tolerance", [I+"nonlinear.rs"], "O", "NewtonTolerancePolicy::ScaledToOuterTolerance",
     "newton", ["G", "P"], ["bdf2-tier-a", "radau5-tier-a"], [], [], None, False, "")
comp("sm.static_stages", "F1", "small_static", [I+"rodas5p_fast_small.rs"], "O", "Rodas5pFastSmallOptions.static_stages",
     "stiff-ivp-small-n", ["G"], [], [prior("L-0085", "PASS", False, SAT)], [], None, False, "SPD06/E5")
comp("sm.static_solves", "F1", "small_static", [I+"rodas5p_fast_small.rs"], "O", "Rodas5pFastSmallOptions.static_solves",
     "stiff-ivp-small-n", ["G"], [], [], [], None, False, "")

# ---------------------------------------------------------- F6 phi / matrix fn
PHI = [
    ("phi.pade13", [C+"matrix_functions.rs"], "O", "dense Pade13 phi", ["pexprb54s4-fused"], [], [], ""),
    ("phi.binary_scaling", [C+"binary_scaling.rs"], "S", "power-of-two scaling", [], [], [], "support of exponential.rs"),
    ("phi.krylov", [I+"exponential.rs"], "O", "ExponentialKrylovConfig", [], [], [], ""),
    ("phi.fused", [I+"exponential.rs"], "O", "FusedPhiKrylovConfig", ["pexprb54s4-fused"], [], [], ""),
    ("phi.taylor", [C+"taylor_phi.rs"], "R", "tests only", [], [], [], ""),
    ("phi.taylor_total", [C+"taylor_phi_total.rs"], "R", "tests only", [], [prior("L-0111", "PASS", True, "fused normalization identity")], [], "PY04"),
    ("phi.leja", [C+"leja_action.rs", C+"leja_setup_work.rs"], "R", "tests/examples only", [],
     [prior("L-0112", "FAIL", True, "Gate 3 cost: Leja costlier than Chebyshev in h*rho>=40 niche")], ["A04"], "PY05; EstimateOnly"),
    ("phi.poly_router", [C+"polynomial_action.rs", C+"polynomial_action/router.rs"], "R", "RoutePolicy", [],
     [prior("L-0110", "FAIL", True, "niche empty")], ["A08"], "PY03"),
    ("phi.laguerre", [C+"polynomial_action.rs"], "R", "Laguerre component export", [], [prior("L-0108", "PASS", True, "opt-in export")], [], "PY01"),
    ("phi.laguerre_adjoint", [C+"laguerre_adjoint.rs"], "R", "Laguerre adjoint bound", [],
     [prior("PY02", "ABSTAIN(A3)", True, "0/32 cases fewer products than Chebyshev")], [], "PY02"),
    ("phi.shared_shift", [C+"shared_shift_jet.rs", C+"shared_shift_policy.rs"], "R", "tests/examples only", [], [], [], ""),
    ("phi.complex_shift_jet", [C+"complex_shift_jet.rs"], "R", "tests only", [], [], [], ""),
]
for pid, files, reach, sel, arms, pri, blk, notes in PHI:
    comp(pid, "F6", "phi_action", files, reach, sel, "matrix-function-action", ["P", "V"], arms, pri, blk, None,
         pid in ("phi.leja",), notes)

# --------------------------------------------------------------- F7 certificates
CERT = [
    ("cert.directed", [C+"directed.rs"], "S", "interval arithmetic support", [], []),
    ("cert.outward", [I+"outward_certificate.rs"], "O", "Q2 admission in transactional_q1_q2", [], []),
    ("cert.residual_gain", [C+"residual_gain.rs"], "R", "single linear-solve bound", [prior("L-0107", "PASS", True, "scoped")], []),
    ("cert.nonnormal", [C+"nonnormal_certificate.rs"], "R", "tests only", [], []),
    ("cert.transform_bound", [C+"transform_bound.rs"], "R", "tests only", [], []),
    ("cert.causal_majorant", [I+"causal_majorant.rs"], "R", "tests only", [], []),
    ("cert.certified_budget", [I+"certified_budget.rs"], "R", "tests only", [], []),
    ("cert.stage_target", [I+"stage_target.rs"], "R", "tests only", [], []),
    ("cert.raw_stage_target", [I+"raw_stage_target.rs"], "O", "residual budget of the MF fast driver", [], []),
    ("cert.fourier_path", [I+"fourier_path_certificate.rs", I+"fourier_path_candidate.rs"], "R", "tests only", [], []),
    ("cert.stage_chart", [I+"stage_chart_candidate.rs"], "R", "tests only", [], []),
    ("cert.bound_pr101", [I+"bound_pr101.rs"], "R", "A05 PR101 provider", [prior("L-0135", "PASS", True, "8/8 tests; pending review")], ["A05"]),
]
for cid, files, reach, sel, pri, blk in CERT:
    comp(cid, "F7", "certificate", files, reach, sel, "error-certificate", ["V"], [], pri, blk, None, False, "")

# ------------------------------------------------------------------ F8 packet
PK = [
    ("pk.collective_model", [I+"collective_model.rs"], ["A01"], [prior("L-0118", "FEASIBLE", True, "model contract")]),
    ("pk.window", [I+"packet_window.rs"], ["A01", "A10"], [prior("L-0119", "FEASIBLE", False, "PA-02 allocations; roundoff_enclosed=false")]),
    ("pk.trajectory", [I+"packet_trajectory.rs"], ["A02"], [prior("L-0121", "FEASIBLE", False, "AT01-AT12 NOT_RUN")]),
    ("pk.curve", [I+"packet_curve.rs", I+"packet_curve_output.rs"], ["A01"], []),
    ("pk.damped_moments", [C+"damped_moments.rs"], ["A03"], [prior("L-0120", "FEASIBLE", False, "kappa>0 refused; scalar rate not executed")]),
    ("pk.bound_collective", [I+"bound_collective.rs"], ["A01"], []),
    ("pk.work", [I+"packet_work.rs"], [], []),
]
for kid, files, blk, pri in PK:
    comp(kid, "F8", "packet", files, "R", "packet API", "collective-packet", ["C", "V"], [], pri, blk, None, True,
         "LP10 promotion superseded by PA DECISION; compare only against la.woodbury_collective and drv.fast")

# ---------------------------------------------------------------- F9 parallel
comp("par.pool", "F9", "parallel", [I+"parallel.rs"], "S", "ParallelExecution (local rayon pool)",
     "parallel", ["P", "C"], [], [], [], None, False, "Ir cannot measure span; wall authority required")
comp("par.homotopy_parallel", "F9", "parallel", [I+"homotopy.rs"], "R", "HT02 parallel pilot", "parallel", ["C"], [],
     [prior("L-0114", "INCONCLUSIVE", True, "callback share f<=0.108 < f_min 0.348")], ["A09"], None, False, "")

# ------------------------------------------------------- F10 scaffolding (static)
SCAF = ["g1_transactional_gate", "g2_exponential_gate", "g3_fused_adaptive_gate", "g4_prefix_kernel_gate",
        "g4_s5b0_regime_atlas", "g4_s5b0_trace_authority", "g4_s5b3_attempt_geometry", "unified_screen",
        "unified_gates", "candidates", "native_gates", "comparator_fidelity", "rhs_telemetry", "common_w_gate",
        "homotopy_experiments", "homotopy_order_policy", "path_controller", "policy_redesign_v25",
        "a1_two_arm_receipt", "v38d_performance_tournament", "scientific_validity_v2_gate",
        "audit2_research", "audit2_matrix_free_research", "audit2_reusable_transaction_research",
        "audit2_stage_certificate_research", "audit2_bateman_real_client_research", "chart_transport",
        "controller_telemetry"]
for m in SCAF:
    comp("scf." + m, "F10", "scaffolding", [I + m + ".rs"], "R", "research/gate/audit module", "none", [], [], [], [],
         "PRUNE_RELOCATE", False, "static reach must confirm no runtime dependents before relocation")

# ------------------------------------------------------------------ F11 corpus
for m in ["problems", "gray_scott", "scientific_corpus_v2"]:
    comp("corp." + m, "F11", "corpus", [I + m + ".rs"], "T", "test problems", "none", [], [], [], [],
         "PRUNE_RELOCATE", False, "move to a bench/test crate; keep as benchmark corpus")


# --------------------------------------------------------------------- corpus
# (id, tier, partition, reference kind). Dimension, structure, Jacobian mode,
# t_span and atol_scale come from `rodas5p triage-list` so the corpus cannot
# drift from the binary. Holdout problems are never used to fit a SPECIALIZE
# predicate (PREREGISTRATION.md rule 4).
CORPUS = [
    ("robertson", "G", "calibration", "two_method"),
    ("hires", "G", "calibration", "two_method"),
    ("van-der-pol-mu1000", "G", "calibration", "two_method"),
    ("prothero-robinson-nl", "G", "calibration", "exact"),
    ("scalar-linear", "G", "calibration", "exact"),
    ("complex-dahlquist-32", "G", "calibration", "exact"),
    ("manufactured-vector-32", "G", "calibration", "exact"),
    ("oscillatory-prothero-robinson", "G", "calibration", "exact"),
    ("mass-affine", "G", "calibration", "two_method"),
    ("mass-nonlinear", "G", "calibration", "exact"),
    ("semilinear-advdiff-32", "G", "calibration", "exact"),
    ("oregonator", "G", "holdout", "two_method"),
    ("pollution", "G", "holdout", "two_method"),
    ("brusselator-1d-50", "P", "calibration", "two_method"),
    ("brusselator-1d-250", "P", "calibration", "two_method"),
    ("brusselator-1d-1000", "P", "holdout", "two_method"),
    ("brusselator-1d-jvp-250", "P", "calibration", "two_method"),
    ("brusselator-1d-jvp-1000", "P", "holdout", "two_method"),
    ("brusselator-2d", "P", "holdout", "two_method"),
    ("medical-akzo", "P", "holdout", "two_method"),
    ("gray-scott-16", "P", "calibration", "two_method"),
    ("gray-scott-32", "P", "calibration", "two_method"),
    ("gray-scott-64", "P", "holdout", "two_method"),
    ("semilinear-advdiff-2d-96", "P", "calibration", "two_method"),
    ("semilinear-advdiff-2d-384", "P", "calibration", "two_method"),
    ("semilinear-advdiff-2d-1536", "P", "holdout", "two_method"),
]
SWEEP = {"rtols": [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10],
         "grids": ["endpoint", "uniform101"], "h0_scales": [0.5, 1.0, 2.0]}
PROFILE_SUBSET = {"rtols": [1e-4, 1e-6, 1e-8], "max_dimension": 2048, "grids": ["uniform101"], "h0_scales": [1.0]}


def build_corpus(listing):
    by_id = {p["id"]: p for p in listing["problems"]}
    missing = [c[0] for c in CORPUS if c[0] not in by_id]
    extra = sorted(set(by_id) - {c[0] for c in CORPUS})
    if missing:
        sys.exit(f"triage-list lacks corpus problems {missing}")
    problems = []
    for pid, tier, part, ref in CORPUS:
        p = by_id[pid]
        if p["tier"] != tier:
            sys.exit(f"{pid}: tier {p['tier']} in triage-list, {tier} in corpus")
        kind = "exact" if p["exact_solution"] else ref
        if ref == "exact" and not p["exact_solution"]:
            sys.exit(f"{pid}: corpus expects an exact solution, triage-list has none")
        problems.append({"id": pid, "tier": tier, "partition": part, "dimension": p["dimension"],
                         "structure": p["structure"], "jacobian": p["jacobian"], "mass": p["mass"],
                         "exact_solution": p["exact_solution"], "oscillatory": p["oscillatory"],
                         "t_span": p["t_span"], "atol_scale": p["atol_scale"],
                         "reference": {"kind": kind, "methods": ["exact"] if kind == "exact" else ["scipy-radau", "scipy-lsoda"]}})
    return {"schema": "vigilode-triage-corpus-v1", "problems": problems, "sweep": SWEEP,
            "profile_subset": PROFILE_SUBSET, "not_in_corpus": extra}


def loc(files):
    total = 0
    for f in files:
        p = ROOT / f
        if not p.exists():
            sys.exit(f"missing file {f}")
        total += sum(1 for _ in p.open(encoding="utf-8"))
    return total


def main():
    listing = None
    if len(sys.argv) == 2:
        listing = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()
    comps, seen = [], set()
    for (cid, fam, slot, files, reach, sel, cap, tiers, arms, pri, blk, static, uniq, notes) in T:
        if cid in seen:
            sys.exit(f"duplicate component {cid}")
        seen.add(cid)
        comps.append({"id": cid, "family": fam, "slot": slot, "files": files, "reach": reach, "selector": sel,
                      "capability": cap, "tiers": tiers, "arms": arms, "prior": pri, "blocked_by": blk,
                      "loc": loc(files), "static_class": static, "runtime_dependents": [],
                      "unique_capability": uniq, "notes": notes})
    if listing is not None:
        # The binary is the authority for which arm exercises which component.
        arms_of = {}
        for arm in listing["arms"]:
            for cid in arm["components"]:
                arms_of.setdefault(cid, []).append(arm["id"])
        unknown = sorted(set(arms_of) - seen)
        if unknown:
            sys.exit(f"triage-list names components missing from the manifest: {unknown}")
        for c in comps:
            c["arms"] = sorted(arms_of.get(c["id"], []))
        corpus = build_corpus(listing)
        (NODE / "CORPUS.json").write_text(json.dumps(corpus, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    out = {"schema": "vigilode-triage-components-v1", "base_commit": commit, "components": comps}
    (NODE / "COMPONENTS.json").write_text(json.dumps(out, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(f"{len(comps)} components")


if __name__ == "__main__":
    main()
