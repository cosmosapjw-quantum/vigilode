#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
G4 = ROOT / "crates" / "rodas5p-integrators" / "src" / "g4_s5b0_regime_atlas.rs"
LIB = ROOT / "crates" / "rodas5p-integrators" / "src" / "lib.rs"
POLICY = ROOT / "crates" / "rodas5p-integrators" / "src" / "g4_s5b0_inner_tolerance.rs"

MODULE_SOURCE = '''use rodas5p_core::{
    CoreError, CoreResult, LinearMethod, LinearSolverConfig,
};

use crate::exponential::{FusedOrthogonalization, FusedPhiKrylovConfig};

const INNER_RELATIVE_FRACTION: f64 = 3.0e-2;
const INNER_ABSOLUTE_FRACTION: f64 = 3.0e-4;
const INNER_RELATIVE_FLOOR: f64 = 1.0e-12;
const INNER_ABSOLUTE_FLOOR: f64 = 1.0e-14;

/// One outer-error contract for both G4/S5B0 inner Krylov paths.
///
/// The protected RODAS5P/GMRES and exponential phi-Krylov paths consume the
/// same relative and absolute tolerances. Structural solver settings remain
/// lane-specific and are not changed by this policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct G4S5B0InnerTolerancePolicy {
    outer_rtol: f64,
    relative_tolerance: f64,
    absolute_tolerance: f64,
}

impl G4S5B0InnerTolerancePolicy {
    pub fn try_from_outer_rtol(outer_rtol: f64) -> CoreResult<Self> {
        if !outer_rtol.is_finite() || outer_rtol <= 0.0 {
            return Err(CoreError::InvalidInput(
                "G4/S5B0 outer rtol must be finite and positive".into(),
            ));
        }
        Ok(Self {
            outer_rtol,
            relative_tolerance: (INNER_RELATIVE_FRACTION * outer_rtol)
                .max(INNER_RELATIVE_FLOOR),
            absolute_tolerance: (INNER_ABSOLUTE_FRACTION * outer_rtol)
                .max(INNER_ABSOLUTE_FLOOR),
        })
    }

    pub fn outer_rtol(self) -> f64 {
        self.outer_rtol
    }

    pub fn relative_tolerance(self) -> f64 {
        self.relative_tolerance
    }

    pub fn absolute_tolerance(self) -> f64 {
        self.absolute_tolerance
    }

    pub fn linear_config(self) -> LinearSolverConfig {
        LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: self.relative_tolerance,
            atol: self.absolute_tolerance,
            restart: 32,
            maxiter: 256,
            ..LinearSolverConfig::default()
        }
    }

    pub fn phi_config(self, dimension: usize) -> FusedPhiKrylovConfig {
        FusedPhiKrylovConfig {
            minimum_dimension: 2,
            maximum_dimension: (dimension + 4).min(32),
            dimension_increment: 2,
            relative_tolerance: self.relative_tolerance,
            absolute_tolerance: self.absolute_tolerance,
            orthogonalization: FusedOrthogonalization::FullMgs,
            maximum_substeps: 16,
        }
    }
}
'''


def replace_exact(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


def patch_lib() -> None:
    text = LIB.read_text(encoding="utf-8")
    text = replace_exact(
        text,
        "mod g4_prefix_kernel_gate;\nmod g4_s5b0_regime_atlas;",
        "mod g4_prefix_kernel_gate;\nmod g4_s5b0_inner_tolerance;\nmod g4_s5b0_regime_atlas;",
        "module declaration",
    )
    text = replace_exact(
        text,
        "pub use g4_s5b0_regime_atlas::{",
        "pub use g4_s5b0_inner_tolerance::G4S5B0InnerTolerancePolicy;\npub use g4_s5b0_regime_atlas::{",
        "policy export",
    )
    LIB.write_text(text, encoding="utf-8")


def patch_g4() -> None:
    text = G4.read_text(encoding="utf-8")
    text = replace_exact(
        text,
        "    CoreError, CoreResult, LinearMethod, LinearSolverConfig, WorkCounters, error_scale, safe_l2,\n",
        "    CoreError, CoreResult, LinearSolverConfig, WorkCounters, error_scale, safe_l2, wrms,\n",
        "core import",
    )
    text = text.replace("    wrms,\n};", "};", 1)
    text = replace_exact(
        text,
        "    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, FusedOrthogonalization,\n",
        "    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, FusedPhiKrylovConfig,\n    FusedPhiPrefixSession, G4S5B0InnerTolerancePolicy, OdeProblem, ParallelExecution,\n    PersistenceLatch,\n",
        "crate import head",
    )
    text = replace_exact(
        text,
        "    FusedPhiKrylovConfig, FusedPhiPrefixSession, OdeProblem, ParallelExecution, PersistenceLatch,\n",
        "",
        "obsolete crate import continuation",
    )

    old_functions = '''fn phi_config(rtol: f64, dimension: usize) -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 2,
        maximum_dimension: (dimension + 4).min(32),
        dimension_increment: 2,
        relative_tolerance: (0.03 * rtol).max(1.0e-12),
        absolute_tolerance: (3.0e-4 * rtol).max(1.0e-14),
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 16,
    }
}

fn linear_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        ..LinearSolverConfig::default()
    }
}
'''
    new_functions = '''fn inner_tolerance_policy(rtol: f64) -> G4S5B0InnerTolerancePolicy {
    G4S5B0InnerTolerancePolicy::try_from_outer_rtol(rtol)
        .expect("G4/S5B0 profile rtol must be finite and positive")
}

fn phi_config(rtol: f64, dimension: usize) -> FusedPhiKrylovConfig {
    inner_tolerance_policy(rtol).phi_config(dimension)
}

fn linear_config(rtol: f64) -> LinearSolverConfig {
    inner_tolerance_policy(rtol).linear_config()
}
'''
    text = replace_exact(text, old_functions, new_functions, "inner configuration block")

    old_call = "let linear = linear_config();"
    call_count = text.count(old_call)
    if call_count != 6:
        raise SystemExit(f"linear call sites: expected 6, found {call_count}")
    text = text.replace(old_call, "let linear = linear_config(adaptive.rtol);")

    if "rtol: 1.0e-10" in text or "atol: 1.0e-12" in text:
        raise SystemExit("fixed A1 tolerance remains in G4/S5B0 source")
    if text.count("linear_config(adaptive.rtol)") != 6:
        raise SystemExit("not every G4/S5B0 linear path consumes outer rtol")
    G4.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    if POLICY.exists():
        raise SystemExit(f"policy module already exists: {POLICY}")
    POLICY.write_text(MODULE_SOURCE, encoding="utf-8")
    patch_lib()
    patch_g4()
    print("PASS: applied A1 shared inner-tolerance policy")
