use rodas5p_core::{CoreError, CoreResult, LinearMethod};
use serde::Serialize;

use crate::{BdfOrder, RadauIiaStages};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateFamily {
    Sequential,
    Sabr,
    Homotopy,
    Bdf,
    RadauIrk,
    PeerW,
    ParallelSdc,
    RosenbrockKrylov,
    Borok,
    ExponentialLeja,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum CandidateStatus {
    Executable,
    Deferred { reason: &'static str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateRecycleLifetime {
    Off,
    Stage,
    Persistent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SabrBlockVariant {
    Forward,
    Explicit,
    Nilpotent,
    Gmres,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SabrPredictorVariant {
    Zero,
    ScaledLast,
    LinearHistory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HomotopyPredictorVariant {
    Euler,
    AdamsBashforth2,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "execution", rename_all = "kebab-case")]
pub enum CandidateExecution {
    Sequential {
        linear_method: LinearMethod,
        recycle_lifetime: CandidateRecycleLifetime,
    },
    Sabr {
        block_method: SabrBlockVariant,
        predictor: SabrPredictorVariant,
    },
    Homotopy {
        theta: f64,
        q: usize,
        path_rounds: usize,
        predictor: HomotopyPredictorVariant,
        corrections_per_point: usize,
    },
    Bdf {
        order: BdfOrder,
    },
    RadauIrk {
        stages: RadauIiaStages,
    },
    Deferred,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateSpec {
    id: String,
    family: CandidateFamily,
    status: CandidateStatus,
    execution: CandidateExecution,
}

impl CandidateSpec {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn family(&self) -> CandidateFamily {
        self.family
    }

    pub fn status(&self) -> CandidateStatus {
        self.status
    }

    pub fn execution(&self) -> &CandidateExecution {
        &self.execution
    }

    pub fn is_rodas_stage_candidate(&self) -> bool {
        matches!(
            self.execution,
            CandidateExecution::Sequential { .. }
                | CandidateExecution::Sabr { .. }
                | CandidateExecution::Homotopy { .. }
        )
    }

    pub fn is_native_complete_integrator(&self) -> bool {
        matches!(
            self.execution,
            CandidateExecution::Bdf { .. } | CandidateExecution::RadauIrk { .. }
        )
    }

    fn sequential(linear_method: LinearMethod, recycle_lifetime: CandidateRecycleLifetime) -> Self {
        let method = match linear_method {
            LinearMethod::Direct => "direct",
            LinearMethod::Gmres => "gmres",
            LinearMethod::Lgmres => "lgmres",
            LinearMethod::Gcrodr => "gcrodr",
        };
        let lifetime = match recycle_lifetime {
            CandidateRecycleLifetime::Off => "off",
            CandidateRecycleLifetime::Stage => "stage",
            CandidateRecycleLifetime::Persistent => "persistent",
        };
        Self {
            id: format!("sequential-{method}-{lifetime}"),
            family: CandidateFamily::Sequential,
            status: CandidateStatus::Executable,
            execution: CandidateExecution::Sequential {
                linear_method,
                recycle_lifetime,
            },
        }
    }

    fn sabr(block_method: SabrBlockVariant, predictor: SabrPredictorVariant) -> Self {
        let block = match block_method {
            SabrBlockVariant::Forward => "forward",
            SabrBlockVariant::Explicit => "explicit",
            SabrBlockVariant::Nilpotent => "nilpotent",
            SabrBlockVariant::Gmres => "gmres",
        };
        let predictor_id = match predictor {
            SabrPredictorVariant::Zero => "zero",
            SabrPredictorVariant::ScaledLast => "scaled-last",
            SabrPredictorVariant::LinearHistory => "linear-history",
        };
        Self {
            id: format!("sabr-{block}-{predictor_id}"),
            family: CandidateFamily::Sabr,
            status: CandidateStatus::Executable,
            execution: CandidateExecution::Sabr {
                block_method,
                predictor,
            },
        }
    }

    fn homotopy(
        theta: f64,
        q: usize,
        path_rounds: usize,
        predictor: HomotopyPredictorVariant,
        corrections_per_point: usize,
    ) -> CoreResult<Self> {
        if !theta.is_finite() || !(0.0..=1.0).contains(&theta) {
            return Err(if theta.is_finite() {
                CoreError::InvalidInput("candidate homotopy theta must lie in [0,1]".into())
            } else {
                CoreError::NonFinite("candidate homotopy theta contains NaN/Inf".into())
            });
        }
        if q >= 8 || path_rounds == 0 || corrections_per_point > 8 {
            return Err(CoreError::InvalidInput(
                "invalid registered homotopy configuration".into(),
            ));
        }
        let predictor_id = match predictor {
            HomotopyPredictorVariant::Euler => "euler",
            HomotopyPredictorVariant::AdamsBashforth2 => "ab2",
        };
        Ok(Self {
            id: format!(
                "homotopy-theta{theta:.3}-q{q}-r{path_rounds}-{predictor_id}-c{corrections_per_point}"
            ),
            family: CandidateFamily::Homotopy,
            status: CandidateStatus::Executable,
            execution: CandidateExecution::Homotopy {
                theta,
                q,
                path_rounds,
                predictor,
                corrections_per_point,
            },
        })
    }

    fn bdf(order: BdfOrder) -> Self {
        let id = match order {
            BdfOrder::One => "bdf1-fixed",
            BdfOrder::Two => "bdf2-fixed",
        };
        Self {
            id: id.to_string(),
            family: CandidateFamily::Bdf,
            status: CandidateStatus::Executable,
            execution: CandidateExecution::Bdf { order },
        }
    }

    fn radau(stages: RadauIiaStages) -> Self {
        let id = match stages {
            RadauIiaStages::One => "radau-iia1-fixed",
            RadauIiaStages::Three => "radau-iia3-fixed",
        };
        Self {
            id: id.to_string(),
            family: CandidateFamily::RadauIrk,
            status: CandidateStatus::Executable,
            execution: CandidateExecution::RadauIrk { stages },
        }
    }

    fn deferred(family: CandidateFamily, id: &str, reason: &'static str) -> Self {
        Self {
            id: id.to_string(),
            family,
            status: CandidateStatus::Deferred { reason },
            execution: CandidateExecution::Deferred,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateCatalog {
    entries: Vec<CandidateSpec>,
}

impl CandidateCatalog {
    pub fn research_default() -> CoreResult<Self> {
        let mut entries = Vec::new();
        entries.push(CandidateSpec::sequential(
            LinearMethod::Direct,
            CandidateRecycleLifetime::Off,
        ));
        entries.push(CandidateSpec::sequential(
            LinearMethod::Gmres,
            CandidateRecycleLifetime::Off,
        ));
        for lifetime in [
            CandidateRecycleLifetime::Off,
            CandidateRecycleLifetime::Stage,
            CandidateRecycleLifetime::Persistent,
        ] {
            entries.push(CandidateSpec::sequential(LinearMethod::Lgmres, lifetime));
        }
        for lifetime in [
            CandidateRecycleLifetime::Off,
            CandidateRecycleLifetime::Stage,
            CandidateRecycleLifetime::Persistent,
        ] {
            entries.push(CandidateSpec::sequential(LinearMethod::Gcrodr, lifetime));
        }

        for block_method in [
            SabrBlockVariant::Forward,
            SabrBlockVariant::Explicit,
            SabrBlockVariant::Nilpotent,
            SabrBlockVariant::Gmres,
        ] {
            entries.push(CandidateSpec::sabr(
                block_method,
                SabrPredictorVariant::LinearHistory,
            ));
        }

        for theta in [0.0, 0.5, 1.0] {
            for q in [0, 1, 2, 7] {
                for path_rounds in [2, 3, 4] {
                    for predictor in [
                        HomotopyPredictorVariant::Euler,
                        HomotopyPredictorVariant::AdamsBashforth2,
                    ] {
                        for corrections_per_point in [0, 1] {
                            entries.push(CandidateSpec::homotopy(
                                theta,
                                q,
                                path_rounds,
                                predictor,
                                corrections_per_point,
                            )?);
                        }
                    }
                }
            }
        }

        const DEFERRED_REASON: &str =
            "Rust implementation has not yet satisfied the unified step and work-ledger contract";
        entries.extend([
            CandidateSpec::bdf(BdfOrder::One),
            CandidateSpec::bdf(BdfOrder::Two),
            CandidateSpec::radau(RadauIiaStages::One),
            CandidateSpec::radau(RadauIiaStages::Three),
            CandidateSpec::deferred(
                CandidateFamily::Bdf,
                "bdf-variable-order",
                "variable-order BDF has not yet satisfied history, estimator, and controller gates",
            ),
            CandidateSpec::deferred(
                CandidateFamily::RadauIrk,
                "radau-adaptive",
                "adaptive Radau has not yet satisfied estimator and controller gates",
            ),
            CandidateSpec::deferred(CandidateFamily::PeerW, "peer-w", DEFERRED_REASON),
            CandidateSpec::deferred(
                CandidateFamily::ParallelSdc,
                "parallel-sdc",
                DEFERRED_REASON,
            ),
            CandidateSpec::deferred(
                CandidateFamily::RosenbrockKrylov,
                "rosenbrock-krylov",
                DEFERRED_REASON,
            ),
            CandidateSpec::deferred(CandidateFamily::Borok, "borok", DEFERRED_REASON),
            CandidateSpec::deferred(
                CandidateFamily::ExponentialLeja,
                "exponential-leja",
                DEFERRED_REASON,
            ),
        ]);

        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[CandidateSpec] {
        &self.entries
    }

    pub fn executable(&self) -> impl Iterator<Item = &CandidateSpec> {
        self.entries
            .iter()
            .filter(|entry| entry.status == CandidateStatus::Executable)
    }
}
