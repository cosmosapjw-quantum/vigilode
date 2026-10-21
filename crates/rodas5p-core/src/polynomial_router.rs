//! Guarded routing of one represented fused phi action across Leja, Laguerre,
//! and Chebyshev (all-DAG A08).
//!
//! All candidates receive the same immutable dense symmetric operator, `h`,
//! and five input vectors. The requested absolute 2-norm budget is unchanged.
//! Leja uses its private fused admission token; the other bases use their
//! existing complete-total admission. The result cannot be deserialized or
//! built with public fields. See `research/all_dag_completion_20261018/nodes/A08.md`.
//!
//! First-admitted routing avoids executing later candidates after success.
//! Compare-all collects real candidate costs on the identical request; its
//! priority remains deterministic and does not turn one noisy timing into a
//! speed claim. A Laguerre plan above the signed-adjoint degree limit abstains
//! before matrix products. Prior PY02 campaign ABSTAIN remains unchanged.

use std::time::Instant;

use serde::Serialize;

use crate::{
    CoreError, CoreResult, LejaSetupReceiptV1, LejaSetupWork, WorkCounters,
    directed::mul_down,
    laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT,
    leja_action::{
        CertifiedLejaCoeffCache, CertifiedLejaWorkspace, LEJA_DEGREE_CAP,
        leja_phi_action_enclosed_with_workspace,
    },
    polynomial_action::{
        EnclosureEvidence, JointPhiInput, PolynomialBasis, RouteCost, RouterCaches,
        SymmetricNonpositiveOperator, TotalErrorAdmission, execute_joint_phi_with_cost,
        plan_joint_phi, route_admission,
    },
    sha256_hex,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolynomialBackend {
    EnclosedLeja,
    Laguerre,
    Chebyshev,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolynomialRoutingMode {
    /// Stop immediately after the first admitted candidate in `order`.
    FirstAdmitted,
    /// Evaluate all candidates, retaining the same deterministic priority.
    CompareAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct GuardedPolynomialConfig {
    /// Must be a permutation of the three backends.
    pub order: [PolynomialBackend; 3],
    pub mode: PolynomialRoutingMode,
    pub leja_degree_cap: usize,
}

impl Default for GuardedPolynomialConfig {
    fn default() -> Self {
        Self {
            order: [
                PolynomialBackend::EnclosedLeja,
                PolynomialBackend::Chebyshev,
                PolynomialBackend::Laguerre,
            ],
            mode: PolynomialRoutingMode::FirstAdmitted,
            leja_degree_cap: LEJA_DEGREE_CAP,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolynomialRefusalStage {
    Domain,
    Planning,
    Preflight,
    Execution,
    Budget,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum PolynomialAttemptStatus {
    Admitted {
        bound: f64,
    },
    Abstained {
        stage: PolynomialRefusalStage,
        reason: String,
    },
}

/// Complete call boundaries are timed, including setup, failed work, admission
/// and copying retained outputs. Named counters remain their documented units;
/// they are not summed into a fictitious FLOP count. Allocation bytes are not
/// measured here and remain `None` until an external calibrated allocator is used.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PolynomialAttemptCost {
    pub elapsed_ns: u64,
    pub work: WorkCounters,
    pub polynomial: RouteCost,
    pub leja_setup: Option<LejaSetupReceiptV1>,
    pub leja_certification_operations: u64,
    pub copied_output_values: usize,
    pub measured_allocated_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GuardedPolynomialAttempt {
    backend: PolynomialBackend,
    status: PolynomialAttemptStatus,
    degree: Option<usize>,
    cost: PolynomialAttemptCost,
    #[serde(skip)]
    fused: Option<Vec<f64>>,
}

impl GuardedPolynomialAttempt {
    pub const fn backend(&self) -> PolynomialBackend {
        self.backend
    }
    pub fn status(&self) -> &PolynomialAttemptStatus {
        &self.status
    }
    pub const fn degree(&self) -> Option<usize> {
        self.degree
    }
    pub fn cost(&self) -> &PolynomialAttemptCost {
        &self.cost
    }
    pub const fn bound(&self) -> Option<f64> {
        match self.status {
            PolynomialAttemptStatus::Admitted { bound } => Some(bound),
            PolynomialAttemptStatus::Abstained { .. } => None,
        }
    }
    /// Only budget-admitted outputs are retained and exposed.
    pub fn fused(&self) -> Option<&[f64]> {
        self.fused.as_deref()
    }
}

/// Owned evidence for exactly one request. Mutating or reusing the workspace
/// does not change this result. Serialization is export only.
///
/// ```compile_fail
/// use rodas5p_core::polynomial_router::GuardedPolynomialAction;
/// let _: GuardedPolynomialAction = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GuardedPolynomialAction {
    schema: &'static str,
    request_sha256: String,
    operator_sha256: String,
    h: f64,
    total_budget: f64,
    config: GuardedPolynomialConfig,
    attempts: Vec<GuardedPolynomialAttempt>,
    selected: Option<usize>,
    work: WorkCounters,
    elapsed_ns: u64,
    measured_allocated_bytes: Option<u64>,
}

impl GuardedPolynomialAction {
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }
    pub const fn total_budget(&self) -> f64 {
        self.total_budget
    }
    pub fn attempts(&self) -> &[GuardedPolynomialAttempt] {
        &self.attempts
    }
    pub const fn is_abstained(&self) -> bool {
        self.selected.is_none()
    }
    pub fn selected(&self) -> Option<&GuardedPolynomialAttempt> {
        self.selected.map(|i| &self.attempts[i])
    }
    pub fn fused(&self) -> Option<&[f64]> {
        self.selected().and_then(|a| a.fused())
    }
    pub fn bound(&self) -> Option<f64> {
        self.selected().and_then(|a| a.bound())
    }
    pub const fn work(&self) -> WorkCounters {
        self.work
    }
    /// Entire valid router call, from validation/hash through result assembly.
    /// Operator construction and caller workspace construction are separate setup.
    pub const fn elapsed_ns(&self) -> u64 {
        self.elapsed_ns
    }
}

/// Reused caches have their native exact-bit identity guards. Default creates
/// empty dynamic buffers; their first allocation/resizing occurs inside timing.
#[derive(Clone, Debug, Default)]
pub struct GuardedPolynomialWorkspace {
    leja: CertifiedLejaWorkspace,
    leja_cache: CertifiedLejaCoeffCache,
    polynomial: RouterCaches,
}

fn elapsed(start: Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
fn invalid(why: &str) -> CoreError {
    CoreError::InvalidInput(format!("GUARDED_POLYNOMIAL_INPUT_UNSUPPORTED: {why}"))
}
fn refuse(stage: PolynomialRefusalStage, reason: impl Into<String>) -> PolynomialAttemptStatus {
    PolynomialAttemptStatus::Abstained {
        stage,
        reason: reason.into(),
    }
}

fn validate_and_identify(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    budget: f64,
    config: GuardedPolynomialConfig,
) -> CoreResult<String> {
    if !h.is_finite() || h < 0.0 || !budget.is_finite() || budget <= 0.0 {
        return Err(invalid(
            "h must be finite/nonnegative and absolute budget finite/positive",
        ));
    }
    if !(1..=LEJA_DEGREE_CAP).contains(&config.leja_degree_cap)
        || config.order[0] == config.order[1]
        || config.order[1] == config.order[2]
        || config.order[0] == config.order[2]
    {
        return Err(invalid(
            "backend order must be a permutation; invalid Leja degree cap",
        ));
    }
    if !op.matrix().as_slice().iter().all(|v| v.is_finite()) {
        return Err(invalid("the represented operator has a nonfinite entry"));
    }
    let n = op.dimension();
    let finite_vector = |v: &[f64]| v.len() == n && v.iter().all(|x| x.is_finite());
    let mut bytes = b"vigilode-guarded-phi-request-v1".to_vec();
    bytes.extend_from_slice(op.fingerprint().as_bytes());
    bytes.extend_from_slice(&h.to_bits().to_le_bytes());
    bytes.extend_from_slice(&budget.to_bits().to_le_bytes());
    match input {
        JointPhiInput::Distinct(v) => {
            if !v.iter().all(|w| finite_vector(w)) {
                return Err(invalid("five finite dimension-matched vectors required"));
            }
            bytes.push(0);
            for w in v {
                for x in w {
                    bytes.extend_from_slice(&x.to_bits().to_le_bytes());
                }
            }
        }
        JointPhiInput::SameVector { vector, scales } => {
            if !finite_vector(vector) || !scales.iter().all(|x| x.is_finite()) {
                return Err(invalid(
                    "finite dimension-matched vector and five finite scales required",
                ));
            }
            bytes.push(1);
            for x in vector.iter().chain(scales.iter()) {
                bytes.extend_from_slice(&x.to_bits().to_le_bytes());
            }
        }
    }
    Ok(sha256_hex(&bytes))
}

/// Select a budget-admitted representation of `sum_{k=0}^4 phi_k(h A) w_k`.
///
/// Malformed requests are errors. Valid but unsupported/over-budget candidates
/// are typed abstentions; all attempted work remains in the caller ledger and
/// the result. No external callback/dense oracle is silently used as a certified
/// fallback. A declared spectral enclosure abstains before backend execution.
#[allow(clippy::too_many_arguments)]
pub fn guarded_polynomial_action(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    config: GuardedPolynomialConfig,
    workspace: &mut GuardedPolynomialWorkspace,
    work: &mut WorkCounters,
) -> CoreResult<GuardedPolynomialAction> {
    let started = Instant::now();
    let request_sha256 = validate_and_identify(op, h, input, total_budget, config)?;
    let verified = matches!(op.enclosure().evidence, EnclosureEvidence::Gershgorin);
    let mut attempts = Vec::with_capacity(3);
    let mut selected = None;
    let mut total_work = WorkCounters::default();
    for backend in config.order {
        let begin = Instant::now();
        let mut counters = WorkCounters::default();
        let mut polynomial = RouteCost::default();
        let mut setup = LejaSetupWork::new();
        let mut leja_setup = None;
        let mut leja_certification_operations = 0;
        let mut degree = None;
        let mut fused = None;
        let status = if !verified {
            refuse(
                PolynomialRefusalStage::Domain,
                "a verified Gershgorin enclosure is required",
            )
        } else if backend == PolynomialBackend::EnclosedLeja {
            let result = leja_phi_action_enclosed_with_workspace(
                op,
                h,
                input,
                total_budget,
                config.leja_degree_cap,
                Some(&mut workspace.leja_cache),
                &mut counters,
                &mut workspace.leja,
                Some(&mut setup),
            );
            let status = match result {
                Ok(report) => {
                    degree = Some(report.degree());
                    match report.admit_requested_budget() {
                        Ok(admitted) => {
                            fused = Some(admitted.fused().to_vec());
                            PolynomialAttemptStatus::Admitted {
                                bound: admitted.bound(),
                            }
                        }
                        Err(why) => refuse(PolynomialRefusalStage::Budget, format!("{why:?}")),
                    }
                }
                Err(why) => refuse(PolynomialRefusalStage::Execution, why.to_string()),
            };
            leja_setup = Some(setup.snapshot());
            leja_certification_operations = workspace.leja.last_certification_ops();
            status
        } else {
            let basis = if backend == PolynomialBackend::Laguerre {
                PolynomialBasis::Laguerre
            } else {
                PolynomialBasis::Chebyshev
            };
            // Downward scaling can underflow to zero, which is a typed refusal,
            // never an automatic increase of the caller's budget.
            match mul_down(total_budget, 0.25).and_then(|truncation| {
                plan_joint_phi(
                    op,
                    h,
                    input,
                    basis,
                    truncation,
                    Some(&workspace.polynomial),
                    &mut polynomial,
                )
            }) {
                Err(why) => refuse(PolynomialRefusalStage::Planning, why.to_string()),
                Ok(plan) => {
                    degree = Some(plan.degree());
                    if backend == PolynomialBackend::Laguerre
                        && plan.degree() > LAGUERRE_ADJOINT_DEGREE_LIMIT
                    {
                        refuse(
                            PolynomialRefusalStage::Preflight,
                            format!(
                                "Laguerre degree {} exceeds signed-adjoint authority limit {}",
                                plan.degree(),
                                LAGUERRE_ADJOINT_DEGREE_LIMIT
                            ),
                        )
                    } else {
                        polynomial.executed_backends = 1;
                        match execute_joint_phi_with_cost(
                            &plan,
                            op,
                            h,
                            input,
                            Some(&mut workspace.polynomial),
                            None,
                            &mut counters,
                            &mut polynomial,
                        ) {
                            Err(why) => {
                                polynomial.failed_executions = 1;
                                refuse(PolynomialRefusalStage::Execution, why.to_string())
                            }
                            Ok(report) => match route_admission(&report, total_budget) {
                                TotalErrorAdmission::Admitted { bound, .. }
                                    if bound.is_finite()
                                        && bound >= 0.0
                                        && bound <= total_budget
                                        && report.fused.iter().all(|x| x.is_finite()) =>
                                {
                                    // Move the candidate's output, preserving the exact center.
                                    fused = Some(report.fused);
                                    PolynomialAttemptStatus::Admitted { bound }
                                }
                                admission => {
                                    polynomial.failed_executions = 1;
                                    refuse(PolynomialRefusalStage::Budget, format!("{admission:?}"))
                                }
                            },
                        }
                    }
                }
            }
        };
        polynomial.vector_products = counters.poly_vector_products;
        polynomial.block_products = counters.poly_block_products;
        let copied_output_values = if backend == PolynomialBackend::EnclosedLeja {
            fused.as_ref().map_or(0, Vec::len)
        } else {
            0
        };
        let cost = PolynomialAttemptCost {
            elapsed_ns: elapsed(begin),
            work: counters,
            polynomial,
            leja_setup,
            leja_certification_operations,
            copied_output_values,
            measured_allocated_bytes: None,
        };
        work.accumulate(counters);
        total_work.accumulate(counters);
        let admitted = matches!(status, PolynomialAttemptStatus::Admitted { .. });
        if admitted && selected.is_none() {
            selected = Some(attempts.len());
        }
        attempts.push(GuardedPolynomialAttempt {
            backend,
            status,
            degree,
            cost,
            fused,
        });
        if admitted && config.mode == PolynomialRoutingMode::FirstAdmitted {
            break;
        }
    }
    let mut out = GuardedPolynomialAction {
        schema: "vigilode-guarded-polynomial-action-v1",
        request_sha256,
        operator_sha256: op.fingerprint().to_owned(),
        h,
        total_budget,
        config,
        attempts,
        selected,
        work: total_work,
        elapsed_ns: 0,
        measured_allocated_bytes: None,
    };
    out.elapsed_ns = elapsed(started);
    Ok(out)
}
