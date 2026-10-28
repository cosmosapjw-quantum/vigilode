"""Source-derived inventory of active Rust numerical surfaces."""
from __future__ import annotations

import csv
import json
import re
from pathlib import Path

CRATES = ("rodas5p-core", "rodas5p-krylov", "rodas5p-integrators", "rodas5p-fair-ab", "rodas5p-cli")
DECL = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(?:async\s+)?(?:const\s+)?(?:fn|struct|enum|trait|type)\s+([A-Za-z_][A-Za-z_0-9]*)", re.M)
# Distinct selectable algorithms within one module require distinct decisions.
# The anchor must occur in the source; aliases are not merged by spelling.
VARIANTS = {
    "rodas5p-integrators:src::bdf": (("BDF1", "BDF1"), ("BDF2", "BDF2")),
    "rodas5p-integrators:src::radau": (("RadauIIA1", "radau-iia1"), ("RadauIIA3", "radau-iia3")),
    "rodas5p-integrators:src::exponential": (("EXPRB2", "exprb2_step"), ("EXPRB43", "exprb43_step"), ("PEXPRB54S4", "pexprb54s4")),
    "rodas5p-integrators:src::adaptive": (("Integral", "Integral,"), ("PI", "Pi,"), ("Predictive", "Predictive,"), ("PredictiveCapped", "PredictiveCapped,"), ("PredictiveCapped2", "PredictiveCapped2,")),
    "rodas5p-core:src::leja_action": (("LegacyEstimateOnly", "EstimateOnly"), ("CertifiedLeja", "leja_phi_action_enclosed_with_workspace")),
    "rodas5p-integrators:src::actual_k_homotopy": (("StageH5", "H5 witness"),),
    "rodas5p-integrators:src::packet_window": (("WindowHomotopy", "window"),),
    "rodas5p-krylov:src::gmres": (("GMRES", "gmres"),),
    "rodas5p-krylov:src::gmres_staged": (("StagedGMRES", "gmres"),),
    "rodas5p-krylov:src::block_gmres": (("BlockGMRES", "gmres"),),
    "rodas5p-krylov:src::lgmres": (("LGMRES", "lgmres"),),
    "rodas5p-krylov:src::gcrodr": (("GCRODR", "gcrodr"),),
}

# Distinct source-backed paths promoted from the bounded component coverage map.
ADDITIONAL_VARIANTS = {'rodas5p-core:src::complex_shift_jet': [('ComplexShiftJet', 'ComplexShiftJet')],
 'rodas5p-core:src::laguerre_adjoint': [('LaguerreAdjoint', 'laguerre_adjoint')],
 'rodas5p-core:src::leja_action': [('LejaCertifiedCapped', 'leja_phi_action_certified_capped'),
                                   ('LejaCachedWorkspace', 'leja_phi_action_certified_with_cache_and_workspace')],
 'rodas5p-core:src::matrix': [('DenseLU', 'LuFactorization'), ('FaerConversion', 'to_faer')],
 'rodas5p-core:src::polynomial_action': [('JointPhiChebyshev', 'Chebyshev'),
                                         ('JointPhiLaguerre', 'Laguerre'),
                                         ('JointPhiLaguerreScales', 'joint_phi_action_laguerre_scales'),
                                         ('JointPhiUnbounded', 'joint_phi_action_unbounded'),
                                         ('JointPhiDenseFallback', 'joint_phi_action_or_dense')],
 'rodas5p-core:src::polynomial_action::router': [('BasisRouteJoint', 'route_joint_phi'),
                                                 ('BasisRouteSingle', 'route_joint_phi_single')],
 'rodas5p-core:src::shared_shift_jet': [('SharedShiftJet', 'SharedShiftJet')],
 'rodas5p-core:src::sparse_direct': [('SparseDirect', 'SparseDirectSolver')],
 'rodas5p-core:src::taylor_phi_total': [('TaylorFusedTotal', 'certify_fused_phi_total'),
                                        ('TaylorNormalizedTotal', 'certify_fused_phi_total_normalized')],
 'rodas5p-integrators:src::actual_k_homotopy': [('ActualKHomotopyCertificate', 'certify_collective_actual_k_homotopy')],
 'rodas5p-integrators:src::adaptive': [('AdaptiveStepDecision', 'adaptive_next_step_after_attempt')],
 'rodas5p-integrators:src::collective_direct': [('RankTwoWoodbury', 'CollectiveDirectWork'),
                                                ('CollectiveStageFactors', 'CollectiveStageFactors')],
 'rodas5p-integrators:src::collective_rodas5p': [('CollectiveRodas5p', 'integrate_collective_rodas5p_observed')],
 'rodas5p-integrators:src::damped_packet': [('DampedPacketWindow', 'verify_damped_packet_window'),
                                            ('DampedPacketTrajectory', 'integrate_verified_damped_packet')],
 'rodas5p-integrators:src::dense_output_v2': [('RodasDenseOutput', 'rodas5p_dense_output'),
                                              ('RadauDenseOutput', 'radau_dense_output'),
                                              ('BdfDenseOutput', 'bdf_dense_output')],
 'rodas5p-integrators:src::fourier_path_candidate': [('FourierFFTRHS', 'fft_rhs_h')],
 'rodas5p-integrators:src::fourier_path_certificate': [('FourierCertificate', 'pub fn certificate('),
                                                       ('FourierTrialCommit', 'pub fn commit('),
                                                       ('FourierDriver', 'pub fn run_driver(')],
 'rodas5p-integrators:src::gray_scott_verified': [('GrayScottContinuousTube', 'gray_scott_2d_verified')],
 'rodas5p-integrators:src::homotopy': [('FixedHomotopyPath', 'run_fixed_homotopy_path'),
                                       ('ScheduledHomotopyPath', 'run_scheduled_homotopy_path'),
                                       ('HomotopyStep', 'homotopy_step')],
 'rodas5p-integrators:src::output': [('ProductionIndexedClock', 'PRODUCTION_CLOCK_POLICY'),
                                     ('ResearchReplayClock', 'RESEARCH_REPLAY_CLOCK_POLICY'),
                                     ('StrictRepresentedCap', 'StrictRepresentedCap'),
                                     ('ClockResolutionSlack', 'AllowClockResolutionSlack'),
                                     ('OutputSamplingPlan', 'OutputSamplingPlan')],
 'rodas5p-integrators:src::packet_window': [('PacketCompile', 'compile_packet'),
                                            ('PacketCandidate', 'generate_candidate'),
                                            ('PacketCertificate', 'compute_certificate'),
                                            ('PacketReconstruction', 'reconstruct_outward_at_times'),
                                            ('BernsteinSupBound', 'outward_polynomial_sup_bound')],
 'rodas5p-integrators:src::parallel': [('ParallelExecution', 'ParallelExecution')],
 'rodas5p-integrators:src::rodas5p_fast': [('DenseLegacyLU', 'Legacy'),
                                           ('DenseColumnExtentsLU', 'ColumnExtents'),
                                           ('DenseColumnExtentsAbove64', 'ColumnExtentsAbove64'),
                                           ('BandedIndexed', 'Indexed'),
                                           ('BandedSlices', 'Slices'),
                                           ('FastDenseDriver', 'integrate_rodas5p_fast_observed'),
                                           ('FastBandedDriver', 'integrate_rodas5p_fast_banded_observed_with_kernel')],
 'rodas5p-integrators:src::rodas5p_fast_small': [('SmallFixedArray', 'integrate_rodas5p_fast_small_observed')],
 'rodas5p-integrators:src::rodas5p_fast_small_batch': [('SmallBatch', 'integrate_rodas5p_fast_small_batch')],
 'rodas5p-integrators:src::rodas5p_matrix_free_fast': [('MatrixFreeDefault', 'integrate_rodas5p_mf_fast_observed'),
                                                       ('MatrixFreeGMRESInto',
                                                        'integrate_rodas5p_mf_fast_observed_gmres_into'),
                                                       ('MatrixFreeLSWorkspace',
                                                        'integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace'),
                                                       ('MatrixFreeGCRODR',
                                                        'integrate_rodas5p_mf_fast_observed_with_gcrodr_policy'),
                                                       ('MatrixFreeStageTarget',
                                                        'integrate_rodas5p_mf_fast_observed_with_stage_target'),
                                                       ('MatrixFreeResidualAccounting',
                                                        'integrate_rodas5p_mf_fast_observed_with_residual_accounting')],
 'rodas5p-integrators:src::routing': [('RoutedDenseOrMatrixFree', 'integrate_rodas5p_routed_observed'),
                                      ('DeclaredBandCheck', 'verify_declared_band')],
 'rodas5p-integrators:src::sabr': [('SabrStep', 'sabr_step')],
 'rodas5p-integrators:src::stage_batch': [('StageBatchFeasibility', 'run_stage_batch_feasibility')],
 'rodas5p-krylov:src::block_gmres': [('SeededGMRES', 'solve_seeded_gmres')],
 'rodas5p-krylov:src::gcrodr': [('GCRODRPolicy', 'solve_gcrodr_with_policy'),
                                ('GCRODRWorkspace', 'solve_gcrodr_with_workspace_and_options')],
 'rodas5p-krylov:src::gmres': [('GMRESWorkspace', 'solve_gmres_with_workspace'),
                               ('GMRESResidualScale', 'solve_gmres_with_workspace_and_residual_scale'),
                               ('GMRESIncremental', 'solve_gmres_incremental')],
 'rodas5p-krylov:src::gmres_staged': [('StagedGMRESObserved', 'solve_staged_gmres_observed')],
 'rodas5p-krylov:src::lgmres': [('LGMRESWorkspace', 'solve_lgmres_with_workspace_and_residual_scale')]}
for _module, _variants in ADDITIONAL_VARIANTS.items():
    VARIANTS[_module] = VARIANTS.get(_module, ()) + tuple(_variants)



def inventory(root):
    root = Path(root)
    rows = []
    for crate in CRATES:
        src = root / "crates" / crate / "src"
        if not src.is_dir():
            raise FileNotFoundError(src)
        for path in sorted(src.rglob("*.rs")):
            text = path.read_text()
            module = str(path.relative_to(root))
            names = sorted(set(DECL.findall(text.split("#[cfg(test)]",1)[0])))
            stem = crate + ":src::" + str(path.relative_to(src).with_suffix("")).replace("/", "::")
            rows.append({"component_id": stem,
                         "crate": crate, "source_path": module, "symbols": names,
                         "kind": "module_coverage",
                         "active": True, "recommended_action": "need to optimize",
                         "evidence_status": "HOLD", "reason": "UNMEASURED_OR_UNMAPPED"})
            for variant, anchor in VARIANTS.get(stem, ()):
                if anchor.lower() not in text.lower():
                    raise ValueError(f"variant anchor absent: {stem}#{variant}")
                rows.append({"component_id": stem + "#" + variant, "crate": crate,
                             "source_path": module, "symbols": [anchor], "kind": "algorithm_variant",
                             "active": True, "recommended_action": "need to optimize",
                             "evidence_status": "HOLD", "reason": "UNMEASURED_OR_UNMAPPED"})
    return rows


def write_reports(rows, output):
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    (output / "component_decisions.json").write_text(json.dumps(rows, indent=2, sort_keys=True) + "\n")
    with (output / "component_decisions.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=("component_id", "crate", "source_path", "symbols", "kind", "active", "recommended_action", "evidence_status", "reason", "solver_ids", "observed_jobs", "passing_gates", "failed_or_unsupported_jobs"))
        writer.writeheader()
        for row in rows:
            writer.writerow({**row, "symbols": ";".join(row["symbols"]), "solver_ids": ";".join(row.get("solver_ids", []))})
    lines = ["# Component decision report", "", "Every active source module is listed. Unmeasured actions are provisional and evidence status is HOLD.", "", "| Component | Symbols | Action | Evidence |", "|---|---:|---|---|"]
    lines += [f"| {r['component_id']} | {len(r['symbols'])} | {r['recommended_action']} | {r['evidence_status']} |" for r in rows]
    (output / "component_decisions.md").write_text("\n".join(lines) + "\n")
