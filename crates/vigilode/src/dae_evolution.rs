//! Opt-in DAE evolution: actual native mass or Schur reduced integration,
//! branch-verified point projection and a posteriori physical tube proof.
//! The underlying numerical trajectory is not mutated by projection.
#![allow(missing_docs)]
use crate::{
    Atol, CallbackError, Jacobian, Options, Problem, Rodas5p, Solution,
    polynomial_events::initialize_mass_problem, solution::Extension,
};
use rodas5p_core::{
    CoreError, CoreResult,
    directed::{Interval, add_up, mul_up},
};
use rodas5p_integrators::{
    retro_dae_evolution::{self as de, DenseBasis, SlabCertificate, WarpSeed},
    retro_dae_init::{
        AttemptCount, DaeSource, InitPolicy, InitReport, InitRoute, InitWork,
        ProjectionAttemptWork, ScalarReuseProjection,
    },
    retro_foundation::QuotientMode,
    retro_portfolio::{Portfolio, Route, Selection},
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
type Iv = Interval;
fn bad(s: impl ToString) -> CoreError {
    CoreError::InvalidInput(format!("DAE client: {}", s.to_string()))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum EvolutionRoute {
    Mass,
    Reduced3,
    Reduced5,
    Prepared5,
    WarpDirect,
    WarpFolded,
    Scalar5,
    /// Explicit exact-affine initialization and existing five-node reconstruction.
    Affine5,
    /// Explicit bounded prepared scalar projection candidate.
    BoundedPrepared5,
    /// General (non-conserved) original-source hybrid spectral slab residual.
    Reduced5Legendre,
    BoundedPreparedLegendre5,
    BoundedAffine5,
    /// Source-bound endpoint variants; original full slab proof retained.
    Reduced5OrderedPicard,
    Reduced5PolynomialPicard,
    BoundedAffinePicard5,
    BoundedVector5,
    BoundedVectorRefined5,
    BoundedVectorPivot5,
    Reduced5NormalLs,
    Reduced5BoundedLs,
    BoundedSturm5,
    BoundedSturmNewton5,
    Reduced5ComplexDictionary,
    Reduced5ComplexScaled,
}
impl EvolutionRoute {
    fn init(self) -> InitRoute {
        match self {
            Self::Prepared5 => InitRoute::PreparedBatch,
            Self::BoundedVector5 | Self::BoundedVectorPivot5 => InitRoute::BoundedVector,
            Self::BoundedVectorRefined5 => InitRoute::BoundedVectorRefined,
            Self::BoundedPrepared5 | Self::BoundedPreparedLegendre5 => {
                InitRoute::BoundedPreparedScalar
            }
            Self::Scalar5 => InitRoute::ScalarRoot,
            Self::BoundedSturm5 => InitRoute::BoundedSturmScalar,
            Self::BoundedSturmNewton5 => InitRoute::BoundedSturmNewtonScalar,
            Self::Affine5 => InitRoute::AffineExact,
            Self::BoundedAffine5 | Self::BoundedAffinePicard5 => InitRoute::BoundedAffineScalar,
            _ => InitRoute::Reuse,
        }
    }
    fn spectral(self) -> bool {
        matches!(
            self,
            Self::Reduced5Legendre | Self::BoundedPreparedLegendre5
        )
    }
    fn picard(self) -> Option<de::PicardEndpointRoute> {
        match self {
            Self::Reduced5OrderedPicard => Some(de::PicardEndpointRoute::OrderedWords),
            Self::Reduced5PolynomialPicard | Self::BoundedAffinePicard5 => {
                Some(de::PicardEndpointRoute::Adaptive7)
            }
            _ => None,
        }
    }
    fn linear_ls(self) -> Option<de::LinearLsEndpointRoute> {
        match self {
            Self::Reduced5NormalLs => Some(de::LinearLsEndpointRoute::NormalEquations),
            Self::Reduced5BoundedLs => Some(de::LinearLsEndpointRoute::BoundedNormal),
            _ => None,
        }
    }
    fn complex(self) -> Option<de::ComplexEndpointRoute> {
        match self {
            Self::Reduced5ComplexDictionary => Some(de::ComplexEndpointRoute::DictionaryDirected),
            Self::Reduced5ComplexScaled => Some(de::ComplexEndpointRoute::ScaledRecurrence),
            _ => None,
        }
    }
    fn nodes(self) -> usize {
        if self == Self::Reduced3 { 3 } else { 5 }
    }
}
/// Absolute physical clock and certified differential-state radius.
/// The caller owns the incoming certificate. None means unresolved, not zero.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct EvolutionEpoch {
    pub start: f64,
    pub end: f64,
    pub incoming_x_error: Option<f64>,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct EvolutionPolicy {
    pub init: InitPolicy,
    pub tube_radius: f64,
    pub global_tol: f64,
    pub green: bool,
}
impl Default for EvolutionPolicy {
    fn default() -> Self {
        Self {
            init: InitPolicy::default(),
            tube_radius: 1e-3,
            global_tol: 1e-3,
            green: true,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ProjectionWork {
    pub calls: usize,
    pub refused: usize,
    pub rhs_count: usize,
    pub linear_batches: usize,
    pub prepared_owners: usize,
    pub prepared_support_batches: usize,
    pub root_calls: usize,
    pub dictionary_uses: usize,
    pub horner_uses: usize,
    pub warp_predictions: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub root_map_rechecks: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub root_map_regrouped: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub root_map_expanded: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub bounded_prepared_batches: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub bounded_residual_terms: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub affine_dyadic_success: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub bounded_sturm_chains: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub bounded_sturm_iterations: usize,
    #[serde(skip_serializing_if = "projection_count_zero")]
    pub bounded_sturm_newton: usize,
}
fn projection_count_zero(n: &usize) -> bool {
    *n == 0
}
impl ProjectionWork {
    fn add(&mut self, w: &InitWork, warp: bool) {
        self.calls += 1;
        self.rhs_count += w.rhs_count;
        self.linear_batches += w.linear_batches;
        self.prepared_owners += w.prepared_owners;
        self.prepared_support_batches += w.prepared_support_batches;
        self.root_calls += w.root_calls;
        self.dictionary_uses += w.dictionary_uses;
        self.horner_uses += w.conditioned_horner_uses;
        self.warp_predictions += usize::from(warp);
        self.root_map_rechecks += w.root_map_rechecks;
        self.root_map_regrouped += w.root_map_regrouped;
        self.root_map_expanded += w.root_map_expanded;
        self.bounded_prepared_batches += w.bounded_prepared_batches;
        self.bounded_residual_terms += w.bounded_residual_terms;
        self.affine_dyadic_success += w.affine_dyadic_success;
        self.bounded_sturm_chains += w.bounded_sturm_chains;
        self.bounded_sturm_iterations += w.bounded_sturm_iterations;
        self.bounded_sturm_newton += w.bounded_sturm_newton;
    }
}
/// Diagnostic boundary receipt, never an event or trajectory authority.
/// No returned Stats after driver entry means unknown native work, not zero.
#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeResourceAudit {
    pub initialization: ProjectionAttemptWork,
    pub point_projection: ProjectionAttemptWork,
    pub scan_nodes: ProjectionAttemptWork,
    pub replay_nodes: ProjectionAttemptWork,
    pub scan_reconstruction: AttemptCount,
    pub replay_reconstruction: AttemptCount,
    pub scan_certificate: AttemptCount,
    pub replay_certificate: AttemptCount,
    pub scan_global_certificates: usize,
    pub replay_global_certificates: usize,
    pub guard_build: AttemptCount,
    pub guard_kernel: de::GuardKernelAudit,
    pub difference_kernel: de::ResetKernelAudit,
    pub difference_comparisons: AttemptCount,
    pub positive_differences: usize,
    pub warp_setup: AttemptCount,
    pub driver_invoked: bool,
    pub stats: Option<crate::Stats>,
}
impl NativeResourceAudit {
    /// A missing returned driver report after entry cannot be inferred.
    pub fn measured_attempts(&self) -> Option<usize> {
        if !self.driver_invoked {
            Some(0)
        } else {
            self.stats.as_ref().map(|v| v.attempts)
        }
    }
}
type AuditHandle = Arc<Mutex<NativeResourceAudit>>;
fn update_audit(a: &Option<AuditHandle>, f: impl FnOnce(&mut NativeResourceAudit)) {
    if let Some(a) = a {
        f(&mut a.lock().expect("native audit lock"));
    }
}
fn audited_init(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    policy: InitPolicy,
    route: InitRoute,
    audit: &Option<AuditHandle>,
) -> CoreResult<InitReport> {
    if audit.is_some() {
        let (r, w) = src.initialize_audited(t, x, z, branch, policy, route);
        update_audit(audit, |a| a.initialization.record(&w, r.is_ok()));
        r
    } else {
        src.initialize(t, x, z, branch, policy, route)
    }
}
#[derive(Clone)]
struct Engine {
    src: DaeSource,
    branch: Vec<[f64; 2]>,
    seed: Vec<f64>,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    warp: Option<WarpSeed>,
    work: Arc<Mutex<ProjectionWork>>,
    conservation: Option<(de::RobertsonModel, de::ConservationRoute)>,
    audit: Option<AuditHandle>,
}
impl Engine {
    fn certify(
        &self,
        times: [f64; 2],
        power: &[Vec<Iv>],
        incoming: Option<f64>,
        scan: bool,
    ) -> CoreResult<SlabCertificate> {
        let r = if let Some((model, route)) = &self.conservation {
            model.certify(
                times,
                power,
                &self.branch,
                self.policy.tube_radius,
                incoming,
                self.policy.init.condition_cap,
                *route,
            )
        } else if let Some(route) = self.route.complex() {
            de::certify_slab_with_complex(
                &self.src,
                times,
                power,
                &self.branch,
                self.policy.tube_radius,
                incoming,
                self.policy.green,
                self.policy.init.condition_cap,
                route,
            )
        } else if let Some(route) = self.route.linear_ls() {
            de::certify_slab_with_linear_ls(
                &self.src,
                times,
                power,
                &self.branch,
                self.policy.tube_radius,
                incoming,
                self.policy.green,
                self.policy.init.condition_cap,
                6,
                route,
            )
        } else if let Some(route) = self.route.picard() {
            de::certify_slab_with_picard(
                &self.src,
                times,
                power,
                &self.branch,
                self.policy.tube_radius,
                incoming,
                self.policy.green,
                self.policy.init.condition_cap,
                route,
            )
        } else if self.route.spectral() {
            de::certify_slab_with_residual(
                &self.src,
                times,
                power,
                &self.branch,
                self.policy.tube_radius,
                incoming,
                self.policy.green,
                self.policy.init.condition_cap,
                de::SlabResidualRoute::HybridCentered,
            )
        } else {
            de::certify_slab(
                &self.src,
                times,
                power,
                &self.branch,
                self.policy.tube_radius,
                incoming,
                self.policy.green,
                self.policy.init.condition_cap,
            )
        };
        update_audit(&self.audit, |a| {
            let (c, q) = if scan {
                (&mut a.scan_certificate, &mut a.scan_global_certificates)
            } else {
                (&mut a.replay_certificate, &mut a.replay_global_certificates)
            };
            c.record(r.is_ok());
            *q += usize::from(r.as_ref().is_ok_and(|v| v.global_qualified));
        });
        r
    }
    fn reconstruct(
        &self,
        times: [f64; 2],
        power: &[Vec<Iv>],
        scan: bool,
    ) -> CoreResult<(Vec<Vec<Iv>>, Vec<InitWork>)> {
        if self.audit.is_some() {
            let (r, w) = de::reconstruct_audited(
                &self.src,
                times,
                power,
                &self.seed,
                &self.branch,
                self.policy.init,
                self.route.init(),
                self.warp.as_ref(),
                self.route.nodes(),
            );
            update_audit(&self.audit, |a| {
                let (c, n) = if scan {
                    (&mut a.scan_reconstruction, &mut a.scan_nodes)
                } else {
                    (&mut a.replay_reconstruction, &mut a.replay_nodes)
                };
                c.record(r.is_ok());
                n.merge(&w);
            });
            r
        } else {
            de::reconstruct(
                &self.src,
                times,
                power,
                &self.seed,
                &self.branch,
                self.policy.init,
                self.route.init(),
                self.warp.as_ref(),
                self.route.nodes(),
            )
        }
    }
    fn project(&self, t: f64, x: &[f64], seed: &[f64]) -> CoreResult<InitReport> {
        let r = if self.audit.is_some() {
            let (r, w) = de::project_audited(
                &self.src,
                t,
                x,
                seed,
                &self.branch,
                self.policy.init,
                self.route.init(),
                self.warp.as_ref(),
            );
            update_audit(&self.audit, |a| a.point_projection.record(&w, r.is_ok()));
            r
        } else {
            de::project(
                &self.src,
                t,
                x,
                seed,
                &self.branch,
                self.policy.init,
                self.route.init(),
                self.warp.as_ref(),
            )
        };
        let mut w = self.work.lock().map_err(|_| bad("work lock"))?;
        if let Ok(v) = &r {
            w.add(&v.work, self.warp.is_some());
        } else {
            w.calls += 1;
            w.refused += 1;
        }
        r
    }
    fn scalar_callback(&self) -> bool {
        matches!(
            self.route,
            EvolutionRoute::Reduced3
                | EvolutionRoute::Reduced5
                | EvolutionRoute::BoundedPrepared5
                | EvolutionRoute::Reduced5Legendre
                | EvolutionRoute::BoundedPreparedLegendre5
                | EvolutionRoute::BoundedAffine5
                | EvolutionRoute::Reduced5OrderedPicard
                | EvolutionRoute::Reduced5PolynomialPicard
                | EvolutionRoute::BoundedAffinePicard5
                | EvolutionRoute::Reduced5NormalLs
                | EvolutionRoute::Reduced5BoundedLs
                | EvolutionRoute::BoundedSturm5
                | EvolutionRoute::BoundedSturmNewton5
                | EvolutionRoute::Reduced5ComplexDictionary
                | EvolutionRoute::Reduced5ComplexScaled
        ) && self.src.nz() == 1
            && self.warp.is_none()
    }
    fn fixed_scalar(&self, t: f64, x: &[f64]) -> Result<ScalarReuseProjection, CallbackError> {
        // Same clamp and ProjectAlgebraic policy as de::project. The
        // immutable initialized Engine owns a valid one-component branch.
        let mut policy = self.policy.init;
        policy.mode = rodas5p_integrators::retro_dae_init::InitMode::ProjectAlgebraic;
        let seed = [self.seed[0].clamp(self.branch[0][0], self.branch[0][1])];
        let (r, receipt) = if matches!(
            self.route,
            EvolutionRoute::BoundedSturm5 | EvolutionRoute::BoundedSturmNewton5
        ) {
            self.src.initialize_scalar_bounded_sturm_audited(
                t,
                x,
                &seed,
                &self.branch,
                policy,
                self.route == EvolutionRoute::BoundedSturmNewton5,
            )
        } else if matches!(
            self.route,
            EvolutionRoute::BoundedAffine5 | EvolutionRoute::BoundedAffinePicard5
        ) {
            self.src
                .initialize_scalar_bounded_affine_audited(t, x, &seed, &self.branch, policy)
        } else if matches!(
            self.route,
            EvolutionRoute::BoundedPrepared5 | EvolutionRoute::BoundedPreparedLegendre5
        ) {
            self.src
                .initialize_scalar_bounded_prepared_audited(t, x, &seed, &self.branch, policy)
        } else {
            self.src
                .initialize_scalar_reuse_audited(t, x, &seed, &self.branch, policy)
        };
        update_audit(&self.audit, |a| {
            a.point_projection.record(&receipt, r.is_ok())
        });
        let mut w = self
            .work
            .lock()
            .map_err(|_| CallbackError::Fatal("work lock".into()))?;
        if let Ok(v) = &r {
            w.add(v.work(), false);
        } else {
            w.calls += 1;
            w.refused += 1;
        }
        r.map_err(|e| CallbackError::Fatal(e.to_string()))
    }
    fn vector_callback(&self) -> bool {
        matches!(
            self.route,
            EvolutionRoute::BoundedVector5 | EvolutionRoute::BoundedVectorRefined5
        )
    }
    fn fixed_vector(
        &self,
        t: f64,
        x: &[f64],
    ) -> Result<rodas5p_integrators::retro_dae_init::BoundedVectorProjection, CallbackError> {
        let mut seed = [0.; 32];
        for (i, &v) in self.seed.iter().enumerate() {
            seed[i] = v.clamp(self.branch[i][0], self.branch[i][1]);
        }
        let mut policy = self.policy.init;
        policy.mode = rodas5p_integrators::retro_dae_init::InitMode::ProjectAlgebraic;
        let (r, receipt) = self.src.initialize_bounded_vector_audited(
            t,
            x,
            &seed[..self.seed.len()],
            &self.branch,
            policy,
            self.route == EvolutionRoute::BoundedVectorRefined5,
            &mut rodas5p_integrators::retro_dae_init::VectorProjectionWorkspace::new(),
        );
        update_audit(&self.audit, |a| {
            a.point_projection.record(&receipt, r.is_ok())
        });
        let mut w = self
            .work
            .lock()
            .map_err(|_| CallbackError::Fatal("work lock".into()))?;
        if let Ok(v) = &r {
            w.add(v.work(), false);
        } else {
            w.calls += 1;
            w.refused += 1;
        }
        r.map_err(|e| CallbackError::Fatal(e.to_string()))
    }
    fn fixed(&self, t: f64, x: &[f64]) -> Result<InitReport, CallbackError> {
        self.project(t, x, &self.seed)
            .map_err(|e| CallbackError::Fatal(e.to_string()))
    }
}
fn reduced_problem(e: &Engine) -> Problem {
    let nx = e.src.nx();
    let a = e.clone();
    let b = e.clone();
    let c = e.clone();
    Problem::new(nx, move |t, x, out| {
        if a.vector_callback() {
            out.copy_from_slice(a.fixed_vector(t, x)?.xdot());
        } else if a.scalar_callback() {
            out.copy_from_slice(a.fixed_scalar(t, x)?.xdot());
        } else {
            out.copy_from_slice(&a.fixed(t, x)?.xdot);
        }
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, x, out| {
        if b.vector_callback() {
            out.copy_from_slice(b.fixed_vector(t, x)?.reduced_jacobian());
        } else if b.scalar_callback() {
            out.copy_from_slice(b.fixed_scalar(t, x)?.reduced_jacobian());
        } else {
            let r = b.fixed(t, x)?;
            for (i, row) in r.reduced_jacobian.iter().enumerate() {
                out[i * nx..(i + 1) * nx].copy_from_slice(row);
            }
        }
        Ok(())
    }))
    .time_derivative(move |t, x, out| {
        if c.vector_callback() {
            out.copy_from_slice(c.fixed_vector(t, x)?.reduced_time());
            return Ok(());
        }
        if c.scalar_callback() {
            let r = c.fixed_scalar(t, x)?;
            let n = nx + 1;
            let mut y = [0.; 32];
            y[..nx].copy_from_slice(x);
            y[nx] = r.z();
            let mut j = [0.; 32 * 32];
            let mut ft = [0.; 32];
            c.src
                .jacobian_into(t, &y[..n], &mut j[..n * n])
                .map_err(|e| CallbackError::Fatal(e.to_string()))?;
            c.src
                .time_into(t, &y[..n], &mut ft[..n])
                .map_err(|e| CallbackError::Fatal(e.to_string()))?;
            for i in 0..nx {
                out[i] = j[i * n + nx].mul_add(r.zt(), ft[i]);
            }
            return Ok(());
        }
        let r = c.fixed(t, x)?;
        let mut y = x.to_vec();
        y.extend_from_slice(&r.z);
        let j = c
            .src
            .jacobian_values(t, &y)
            .map_err(|e| CallbackError::Fatal(e.to_string()))?;
        let ft = c
            .src
            .time_values(t, &y)
            .map_err(|e| CallbackError::Fatal(e.to_string()))?;
        let n = y.len();
        for i in 0..nx {
            out[i] =
                r.zt.iter()
                    .enumerate()
                    .fold(ft[i], |s, (k, &v)| j[i * n + nx + k].mul_add(v, s));
        }
        Ok(())
    })
}
/// Prepare the same reduced callbacks used by evolve_epoch, from an
/// original-source certified initialization. WARP setup needs an epoch
/// window and is not supported by this direct client constructor.
#[allow(clippy::too_many_arguments)]
pub fn initialize_reduced_problem(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    policy: EvolutionPolicy,
    route: EvolutionRoute,
) -> CoreResult<crate::polynomial_events::InitializedDae> {
    if matches!(
        route,
        EvolutionRoute::Mass | EvolutionRoute::WarpDirect | EvolutionRoute::WarpFolded
    ) {
        return Err(bad("direct reduced client route"));
    }
    let report = src.initialize(t, x, z, branch, policy.init, route.init())?;
    let e = Engine {
        src: src.clone(),
        branch: branch.to_vec(),
        seed: report.z.clone(),
        policy,
        route,
        warp: None,
        work: Arc::new(Mutex::new(ProjectionWork::default())),
        conservation: None,
        audit: None,
    };
    Ok(crate::polynomial_events::InitializedDae {
        problem: reduced_problem(&e),
        initial_state: x.to_vec(),
        report,
    })
}
#[derive(Clone, Debug, Serialize)]
pub struct Query {
    pub t: f64,
    pub raw: Vec<f64>,
    pub state: Vec<f64>,
    pub raw_constraint_upper: Option<f64>,
    pub projected_constraint_upper: f64,
    pub projection: InitReport,
    pub physical_error_upper: Option<f64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct NativePacket {
    pub basis: DenseBasis,
    pub coefficients: Vec<f64>,
    pub y0: Vec<f64>,
    pub y1: Vec<f64>,
    pub stages: Vec<Vec<f64>>,
}
impl NativePacket {
    fn power(&self) -> CoreResult<Vec<Vec<Iv>>> {
        de::dense_power(&self.y0, &self.y1, &self.stages, self.basis)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct SlabRecord {
    pub times: [f64; 2],
    pub native: NativePacket,
    pub power: Vec<Vec<[f64; 2]>>,
    pub certificate: Option<SlabCertificate>,
    pub reason: Option<String>,
}
#[derive(Serialize)]
pub struct EvolutionRun {
    pub route: EvolutionRoute,
    pub initial: InitReport,
    /// Nonlegacy epoch authority, retained in the receipt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<EvolutionEpoch>,
    driver: Solution,
    pub accepted: Vec<Query>,
    pub queries: Vec<Query>,
    slabs: Vec<SlabRecord>,
    pub work: ProjectionWork,
    pub warp: Option<WarpSeed>,
    pub pointwise_qualified: bool,
    pub branch_qualified: bool,
    pub global_qualified: bool,
    pub max_physical_error: Option<f64>,
    pub reason: Option<String>,
    #[serde(skip)]
    engine: Engine,
}
fn projected(
    e: &Engine,
    s: &Solution,
    t: f64,
    raw: Vec<f64>,
    slabs: &[SlabRecord],
) -> CoreResult<Query> {
    let nx = e.src.nx();
    let mass = e.route == EvolutionRoute::Mass;
    if raw.len() != if mass { nx + e.src.nz() } else { nx } {
        return Err(bad("raw shape"));
    }
    let x = &raw[..nx];
    let seed = if mass { &raw[nx..] } else { &e.seed };
    let r = e.project(t, x, seed)?;
    let mut state = x.to_vec();
    state.extend_from_slice(&r.z);
    let raw_constraint_upper = if mass {
        Some(e.src.constraint_upper(t, &raw)?)
    } else {
        None
    };
    let mut physical_error_upper = None;
    if let Some(a) = slabs.get(slabs.partition_point(|a| a.times[1] < t))
        && let Some(c) = a.certificate.as_ref().filter(|c| c.global_qualified)
        && t >= a.times[0]
        && t <= a.times[1]
        && x.iter()
            .zip(&c.x_range)
            .all(|(&x, b)| b[0] <= x && x <= b[1])
    {
        let p = a.power[..nx]
            .iter()
            .map(|r| {
                r.iter()
                    .map(|b| Iv::new(b[0], b[1]))
                    .collect::<CoreResult<Vec<_>>>()
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let ideal = c.whole_x_error.expect("qualified");
        if c.conservation.is_some() {
            physical_error_upper =
                Some(de::conservation_query_full(&p, a.times, t, &state, ideal)?);
        } else {
            let ex = de::query_x_error(&p, a.times, t, x, ideal)?;
            physical_error_upper =
                Some(ex.max(add_up(r.state_error_upper, mul_up(c.zx_norm_upper, ex)?)?));
        }
    }
    // Do not permit extrapolation even on a direct accepted-state query.
    if t < s.t[0] || t > *s.t.last().expect("initial") {
        return Err(bad("query outside driver domain"));
    }
    Ok(Query {
        t,
        raw,
        state,
        raw_constraint_upper,
        projected_constraint_upper: r.residual_upper,
        projection: r,
        physical_error_upper,
    })
}
impl EvolutionRun {
    pub fn driver(&self) -> &Solution {
        &self.driver
    }
    pub fn slabs(&self) -> &[SlabRecord] {
        &self.slabs
    }

    /// Fresh original-source projection, retaining no cached algebraic answer.
    /// Physical bound is available only when the containing slab is proved.
    pub fn query(&self, t: f64) -> CoreResult<Query> {
        let y = self
            .driver
            .at(t)
            .ok_or_else(|| bad("query outside driver"))?;
        projected(&self.engine, &self.driver, t, y, &self.slabs)
    }
}
fn packet(s: &Solution, k: usize) -> CoreResult<NativePacket> {
    match s
        .extension
        .as_ref()
        .ok_or_else(|| bad("native dense store missing"))?
    {
        Extension::Sequential(v) => {
            let a = v.get(k).ok_or_else(|| bad("sequential packet index"))?;
            if [a.t_old, a.t_new] != [s.t[k], s.t[k + 1]]
                || a.y_old != s.y[k]
                || a.y_new != s.y[k + 1]
            {
                return Err(bad("sequential packet identity"));
            }
            Ok(NativePacket {
                basis: DenseBasis::SequentialD,
                coefficients: DenseBasis::SequentialD.coefficients()?.as_slice().to_vec(),
                y0: a.y_old.clone(),
                y1: a.y_new.clone(),
                stages: a.stages.clone(),
            })
        }
        Extension::Fast(v) => {
            if [v.t_old(k), v.t_new(k)] != [s.t[k], s.t[k + 1]]
                || v.state(k) != s.y[k]
                || v.state(k + 1) != s.y[k + 1]
            {
                return Err(bad("fast packet identity"));
            }
            let stage = v
                .stage_rows(k)
                .chunks(v.dimension())
                .map(<[f64]>::to_vec)
                .collect::<Vec<_>>();
            Ok(NativePacket {
                basis: DenseBasis::FastH,
                coefficients: DenseBasis::FastH.coefficients()?.as_slice().to_vec(),
                y0: v.state(k).to_vec(),
                y1: v.state(k + 1).to_vec(),
                stages: stage,
            })
        }
    }
}
/// Execute one full numerical candidate plus original source verification.
/// A returned report can be unqualified; no fallback silently changes route.
#[allow(clippy::too_many_arguments)]
pub fn evolve(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    end: f64,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
) -> CoreResult<EvolutionRun> {
    evolve_epoch(
        src,
        x,
        z,
        branch,
        EvolutionEpoch {
            start: 0.,
            end,
            incoming_x_error: Some(0.),
        },
        opts,
        policy,
        route,
        grid,
    )
}
/// Integrate at an absolute physical clock with caller-certified incoming
/// differential error. No event is detected or committed by this API.
/// Unknown incoming radius allows numerical evolution but no global proof.
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
) -> CoreResult<EvolutionRun> {
    evolve_epoch_impl(src, x, z, branch, epoch, opts, policy, route, grid, None)
}
/// Opt-in diagnostic snapshot of the executed epoch, even on Err.
/// Future queries are not included in this returned snapshot.
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch_audited(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
) -> (CoreResult<EvolutionRun>, NativeResourceAudit) {
    let a = Arc::new(Mutex::new(NativeResourceAudit::default()));
    let r = evolve_epoch_authority(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        grid,
        None,
        None,
        Some(a.clone()),
    );
    let receipt = a.lock().expect("native audit lock").clone();
    (r, receipt)
}
#[allow(clippy::too_many_arguments)]
fn evolve_epoch_impl(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
    scan: Option<&mut dyn EpochObserver>,
) -> CoreResult<EvolutionRun> {
    evolve_epoch_authority(
        src, x, z, branch, epoch, opts, policy, route, grid, scan, None, None,
    )
}
#[allow(clippy::too_many_arguments)]
fn evolve_epoch_authority(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
    mut scan: Option<&mut dyn EpochObserver>,
    conservation: Option<(de::RobertsonModel, de::ConservationRoute)>,
    audit: Option<AuditHandle>,
) -> CoreResult<EvolutionRun> {
    let EvolutionEpoch {
        start,
        end,
        incoming_x_error,
    } = epoch;
    if !start.is_finite()
        || !end.is_finite()
        || end <= start
        || !(end - start).is_finite()
        || incoming_x_error.is_some_and(|v| !v.is_finite() || v < 0.)
        || !opts.dense_output
        || !matches!(opts.atol, Atol::Scalar(_))
        || !policy.tube_radius.is_finite()
        || policy.tube_radius <= 0.
        || !policy.global_tol.is_finite()
        || policy.global_tol <= 0.
        || grid.len() > 4096
        || grid
            .iter()
            .any(|t| !t.is_finite() || *t < start || *t > end)
        || grid.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(bad("input/clock/dense scalar tolerance contract"));
    }
    if conservation.is_some()
        && (route.spectral()
            || route.picard().is_some()
            || route.linear_ls().is_some()
            || route.complex().is_some())
    {
        return Err(bad("spectral route requires general slab authority"));
    }
    let initial = audited_init(src, start, x, z, branch, policy.init, route.init(), &audit)?;
    let warp = match route {
        EvolutionRoute::WarpDirect | EvolutionRoute::WarpFolded => {
            let mode = if route == EvolutionRoute::WarpDirect {
                QuotientMode::DirectBlock
            } else {
                QuotientMode::Folded
            };
            let r = WarpSeed::new_window(src, start, end, mode);
            update_audit(&audit, |a| a.warp_setup.record(r.is_ok()));
            Some(r?)
        }
        _ => None,
    };
    let e = Engine {
        src: src.clone(),
        branch: branch.to_vec(),
        seed: initial.z.clone(),
        policy,
        route,
        warp: warp.clone(),
        work: Arc::new(Mutex::new(ProjectionWork::default())),
        conservation,
        audit: audit.clone(),
    };
    let (p, y) = if route == EvolutionRoute::Mass {
        let a = if audit.is_some() {
            let report = audited_init(src, start, x, z, branch, policy.init, route.init(), &audit)
                .map_err(|e| bad(crate::SolveError::InvalidInput(e.to_string())))?;
            crate::polynomial_events::mass_problem_from_report(src, report)
        } else {
            initialize_mass_problem(src, start, x, z, branch, policy.init, route.init())
                .map_err(bad)?
        };
        (a.problem, a.initial_state)
    } else {
        let p = reduced_problem(&e);
        (p, x.to_vec())
    };
    update_audit(&audit, |a| a.driver_invoked = true);
    let driver = if let Some(scan) = scan.as_deref_mut() {
        let mut inspect = |proposal: crate::solve::DenseProposal<'_>| scan.inspect(&e, proposal);
        Rodas5p::solve_dense_controlled(&p, (start, end), &y, opts, &mut inspect).map_err(bad)?
    } else {
        Rodas5p::solve(&p, (start, end), &y, opts).map_err(bad)?
    };
    update_audit(&audit, |a| a.stats = Some(driver.stats.clone()));
    let expected_stop = scan
        .as_ref()
        .is_some_and(|v| v.stopped_at().is_some_and(|t| driver.t.last() == Some(&t)));
    let actual_end = if expected_stop {
        *driver.t.last().expect("accepted prefix")
    } else {
        end
    };
    let mut out = EvolutionRun {
        route,
        initial,
        epoch: (start != 0. || incoming_x_error != Some(0.)).then_some(epoch),
        driver,
        accepted: vec![],
        queries: vec![],
        slabs: vec![],
        work: ProjectionWork::default(),
        warp,
        pointwise_qualified: false,
        branch_qualified: false,
        global_qualified: false,
        max_physical_error: None,
        reason: None,
        engine: e.clone(),
    };
    if (!out.driver.is_success() && !expected_stop)
        || out.driver.t.last() != Some(&actual_end)
        || !out.driver.splices.is_empty()
    {
        out.reason = Some(format!(
            "driver incomplete or unsupported splice: {:?}",
            out.driver.status
        ));
        out.work = e.work.lock().map_err(|_| bad("work lock"))?.clone();
        return Ok(out);
    }
    let mut incoming = incoming_x_error;
    for k in 0..out.driver.t.len() - 1 {
        let times = [out.driver.t[k], out.driver.t[k + 1]];
        let native = packet(&out.driver, k)?;
        let mut p = native.power()?;
        if route != EvolutionRoute::Mass {
            let (a, w) = match e.reconstruct(times, &p, false) {
                Ok(v) => v,
                Err(err) => {
                    out.slabs.push(SlabRecord {
                        times,
                        native,
                        power: vec![],
                        certificate: None,
                        reason: Some(err.to_string()),
                    });
                    incoming = None;
                    continue;
                }
            };
            for w in &w {
                e.work
                    .lock()
                    .map_err(|_| bad("work lock"))?
                    .add(w, e.warp.is_some());
            }
            p.extend(a);
        }
        let power = p
            .iter()
            .map(|a| a.iter().map(|v| v.as_pair()).collect())
            .collect();
        let r = e.certify(times, &p, incoming, false);
        let (certificate, reason) = match r {
            Ok(c) => {
                incoming = if c.global_qualified { c.outgoing } else { None };
                (Some(c), None)
            }
            Err(err) => {
                incoming = None;
                (None, Some(err.to_string()))
            }
        };
        out.slabs.push(SlabRecord {
            times,
            native,
            power,
            certificate,
            reason,
        });
    }
    out.branch_qualified = out.slabs.iter().all(|s| s.certificate.is_some());
    for (&t, y) in out.driver.t.iter().zip(&out.driver.y) {
        match projected(&e, &out.driver, t, y.clone(), &out.slabs) {
            Ok(v) => out.accepted.push(v),
            Err(err) => {
                out.reason = Some(err.to_string());
                break;
            }
        }
    }
    for &t in grid {
        match out.query(t) {
            Ok(v) => out.queries.push(v),
            Err(err) => {
                out.reason = Some(err.to_string());
                break;
            }
        }
    }
    out.pointwise_qualified =
        out.accepted.len() == out.driver.t.len() && out.queries.len() == grid.len();
    if out.branch_qualified
        && out.pointwise_qualified
        && incoming.is_some()
        && out
            .slabs
            .iter()
            .all(|s| s.certificate.as_ref().is_some_and(|c| c.global_qualified))
    {
        let mut mx = 0.0_f64;
        for s in &out.slabs {
            let c = s.certificate.as_ref().unwrap();
            mx = mx
                .max(c.whole_x_error.unwrap())
                .max(c.whole_z_error.unwrap());
        }
        let qs = out
            .accepted
            .iter()
            .chain(&out.queries)
            .map(|q| q.physical_error_upper)
            .collect::<Option<Vec<_>>>();
        if let Some(q) = qs {
            mx = q.into_iter().fold(mx, f64::max);
            out.max_physical_error = Some(mx);
            out.global_qualified = mx <= policy.global_tol;
        }
    }
    out.work = e.work.lock().map_err(|_| bad("work lock"))?.clone();
    Ok(out)
}
/// RETRO measured route choice over the same continuous branch/pointwise
/// contract. Global certificate status is retained separately, not implied.
#[allow(clippy::too_many_arguments)]
pub fn adaptive(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    end: f64,
    opts: &Options,
    policy: EvolutionPolicy,
    grid: &[f64],
    portfolio: &mut Portfolio,
) -> CoreResult<Selection<EvolutionRun>> {
    let mut routes = vec![
        Route::EvolutionMass,
        Route::EvolutionReduced3,
        Route::EvolutionReduced5,
        Route::EvolutionPrepared5,
        Route::EvolutionWarpDirect,
        Route::EvolutionWarpFolded,
    ];
    if src.nz() == 1 {
        routes.push(Route::EvolutionScalar5);
    }
    portfolio.choose(
        "dae-evolution",
        &[
            src.nx() as u64,
            src.nz() as u64,
            opts.rtol.to_bits(),
            policy.init.state_tol.to_bits(),
            policy.tube_radius.to_bits(),
            grid.len() as u64,
        ],
        &routes,
        policy.init.residual_tol,
        |r| {
            let route = match r {
                Route::EvolutionMass => EvolutionRoute::Mass,
                Route::EvolutionReduced3 => EvolutionRoute::Reduced3,
                Route::EvolutionReduced5 => EvolutionRoute::Reduced5,
                Route::EvolutionPrepared5 => EvolutionRoute::Prepared5,
                Route::EvolutionWarpDirect => EvolutionRoute::WarpDirect,
                Route::EvolutionWarpFolded => EvolutionRoute::WarpFolded,
                Route::EvolutionScalar5 => EvolutionRoute::Scalar5,
                _ => return Err(bad("route")),
            };
            let v = evolve(src, x, z, branch, end, opts, policy, route, grid)?;
            let b = if v.branch_qualified && v.pointwise_qualified {
                v.accepted
                    .iter()
                    .chain(&v.queries)
                    .map(|q| q.projected_constraint_upper)
                    .fold(0., f64::max)
            } else {
                f64::INFINITY
            };
            Ok((v, b))
        },
    )
}

/// Root-search routes. All require the same source-owned physical tube;
/// a represented-polynomial hint has no physical authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ThresholdRoute {
    Bisection,
    IntervalNewton,
    DictionaryHint,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ThresholdWork {
    pub inspected: usize,
    pub guard_evals: usize,
    pub hint_calls: usize,
    pub hint_dictionary_uses: usize,
    pub projection: ProjectionWork,
    pub certified_slabs: usize,
}
pub use de::{GuardDerivativeRoute, GuardSlabEvidence, GuardValueRoute};
struct DeclaredGuard {
    source: rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
}
/// Bounded opt-in handling of an ambiguous accepted-step endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PendingMethod {
    Hull,
    Piecewise,
    PiecewiseProbe,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PendingPolicy {
    pub method: PendingMethod,
    pub max_slabs: usize,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PendingSlab {
    pub times: [f64; 2],
    pub left: [f64; 2],
    pub right: [f64; 2],
    pub velocity: [f64; 2],
    pub probe: Option<(f64, [f64; 2])>,
    pub incoming_radius: f64,
    pub outgoing_radius: f64,
    pub safe_before: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct PendingReport {
    pub policy: PendingPolicy,
    pub records: Vec<PendingSlab>,
    pub candidate: Option<[f64; 2]>,
    pub episodes: usize,
    pub refinements: usize,
    pub resolution: Option<&'static str>,
    pub blocked_initial: bool,
}
struct PendingState {
    report: PendingReport,
    active_start: Option<usize>,
}
impl PendingState {
    fn new(policy: PendingPolicy) -> CoreResult<Self> {
        if !(1..=64).contains(&policy.max_slabs) {
            return Err(bad("pending slab cap"));
        }
        Ok(Self {
            report: PendingReport {
                policy,
                records: Vec::with_capacity(policy.max_slabs),
                candidate: None,
                episodes: 0,
                refinements: 0,
                resolution: None,
                blocked_initial: true,
            },
            active_start: None,
        })
    }
    fn active(&self) -> bool {
        self.active_start.is_some()
    }
    fn room(&self) -> bool {
        self.report.records.len() < self.report.policy.max_slabs
    }
    fn add(&mut self, r: PendingSlab) -> CoreResult<()> {
        if !self.room() {
            self.report.resolution = Some("slab-cap");
            return Err(bad("pending slab cap exhausted"));
        }
        if let Some(i) = self.active_start {
            let prev = self.report.records.last().expect("active pending record");
            if i >= self.report.records.len()
                || prev.times[1] != r.times[0]
                || prev.right[0] > r.left[1]
                || r.left[0] > prev.right[1]
                || prev.outgoing_radius != r.incoming_radius
            {
                self.report.resolution = Some("inconsistent-boundary");
                return Err(bad("inconsistent pending boundary or physical radius"));
            }
        } else {
            if r.left[1] >= 0. {
                return Err(bad("pending requires negative anchor"));
            }
            self.active_start = Some(self.report.records.len());
            self.report.episodes += 1;
            self.report.candidate = None;
        }
        self.report.records.push(r);
        Ok(())
    }
    fn enclose(&mut self) -> CoreResult<[f64; 2]> {
        let rs =
            &self.report.records[self.active_start.ok_or_else(|| bad("pending not active"))?..];
        let first = rs.first().ok_or_else(|| bad("empty pending"))?;
        let last = rs.last().expect("pending records");
        if first.left[1] >= 0. || last.right[0] <= 0. {
            return Err(bad("missing strict physical signs"));
        }
        fn newton(t: f64, g: [f64; 2], v: Iv) -> CoreResult<Iv> {
            Iv::point(t)?.sub(Iv::new(g[0], g[1])?.div(v)?)
        }
        fn meet(a: Iv, b: Iv) -> Option<Iv> {
            let lo = a.lo.max(b.lo);
            let hi = a.hi.min(b.hi);
            (lo <= hi).then_some(Iv { lo, hi })
        }
        let mut n = 0;
        let answer = match self.report.policy.method {
            PendingMethod::Hull => {
                let d = Iv::new(
                    rs.iter()
                        .map(|r| r.velocity[0])
                        .fold(f64::INFINITY, f64::min),
                    rs.iter()
                        .map(|r| r.velocity[1])
                        .fold(f64::NEG_INFINITY, f64::max),
                )?;
                if d.lo <= 0. {
                    return Err(bad("pending nontransverse"));
                }
                let mut i = Iv::new(first.times[0], last.times[1])?;
                for r in rs {
                    for (t, g) in [(r.times[0], r.left), (r.times[1], r.right)] {
                        n += 1;
                        i = meet(i, newton(t, g, d)?)
                            .ok_or_else(|| bad("inconsistent pending hull"))?;
                    }
                }
                i
            }
            PendingMethod::Piecewise | PendingMethod::PiecewiseProbe => {
                let mut hull: Option<Iv> = None;
                for r in rs {
                    let d = Iv::new(r.velocity[0], r.velocity[1])?;
                    if d.lo <= 0. {
                        return Err(bad("pending nontransverse"));
                    }
                    let i = Iv::new(r.times[0], r.times[1])?;
                    n += 2;
                    let a = newton(r.times[0], r.left, d)?;
                    let b = newton(r.times[1], r.right, d)?;
                    let mut local = meet(i, a).and_then(|i| meet(i, b));
                    if let Some((t, g)) = r.probe {
                        n += 1;
                        let w = newton(t, g, d)?;
                        local = local.and_then(|i| meet(i, w));
                    }
                    if let Some(i) = local {
                        hull = Some(match hull {
                            None => i,
                            Some(j) => Iv {
                                lo: i.lo.min(j.lo),
                                hi: i.hi.max(j.hi),
                            },
                        });
                    }
                }
                hull.ok_or_else(|| bad("inconsistent pending piecewise enclosure"))?
            }
        };
        self.report.refinements += n;
        self.report.candidate = Some(answer.as_pair());
        Ok(answer.as_pair())
    }
}
struct ThresholdScan {
    pending: Option<PendingState>,
    threshold: f64,
    guard: Option<DeclaredGuard>,
    guard_slabs: Vec<GuardSlabEvidence>,
    tolerance: f64,
    route: ThresholdRoute,
    incoming: Option<f64>,
    safe: f64,
    numerical_end: f64,
    root: Option<[f64; 2]>,
    stopped: bool,
    reason: Option<String>,
    work: ThresholdWork,
}
fn proposal_packet(
    proposal: crate::solve::DenseProposal<'_>,
) -> CoreResult<([f64; 2], NativePacket)> {
    use crate::solve::DenseProposal;
    let (times, basis, y0, y1, stages) = match proposal {
        DenseProposal::Sequential(s) => (
            [s.t_old, s.t_new],
            DenseBasis::SequentialD,
            s.y_old.clone(),
            s.y_new.clone(),
            s.stages.clone(),
        ),
        DenseProposal::Fast(s) => {
            let k = s.len() - 1;
            (
                [s.t_old(k), s.t_new(k)],
                DenseBasis::FastH,
                s.state(k).to_vec(),
                s.state(k + 1).to_vec(),
                s.stage_rows(k)
                    .chunks(s.dimension())
                    .map(<[f64]>::to_vec)
                    .collect(),
            )
        }
    };
    Ok((
        times,
        NativePacket {
            basis,
            coefficients: basis.coefficients()?.as_slice().to_vec(),
            y0,
            y1,
            stages,
        },
    ))
}
fn guard_interval(p: &[Iv], times: [f64; 2], t: f64, error: f64, threshold: f64) -> CoreResult<Iv> {
    let u = Iv::point(t)?
        .sub(Iv::point(times[0])?)?
        .div(Iv::point(times[1])?.sub(Iv::point(times[0])?)?)?;
    let u = Iv::new(u.lo.max(0.), u.hi.min(1.))?;
    let x = p
        .iter()
        .rev()
        .try_fold(Iv::point(0.)?, |a, &b| a.mul(u)?.add(b))?;
    x.sub(Iv::point(threshold)?)?.add(Iv::new(-error, error)?)
}
fn diameter(a: f64, b: f64) -> CoreResult<f64> {
    rodas5p_core::directed::sub_up(b, a)
}
trait EpochObserver {
    fn inspect(
        &mut self,
        e: &Engine,
        proposal: crate::solve::DenseProposal<'_>,
    ) -> rodas5p_integrators::AcceptControl;
    fn stopped_at(&self) -> Option<f64>;
}
impl EpochObserver for ThresholdScan {
    fn stopped_at(&self) -> Option<f64> {
        self.stopped.then_some(self.numerical_end)
    }
    fn inspect(
        &mut self,
        e: &Engine,
        proposal: crate::solve::DenseProposal<'_>,
    ) -> rodas5p_integrators::AcceptControl {
        use rodas5p_integrators::AcceptControl;
        self.work.inspected += 1;
        let answer = proposal_packet(proposal).and_then(|(times, native)| {
            self.numerical_end = times[1];
            self.scan(e, times, &native)
        });
        match answer {
            Ok(false) => AcceptControl::Continue,
            Ok(true) => {
                self.stopped = true;
                AcceptControl::Stop
            }
            Err(err) => {
                self.reason = Some(err.to_string());
                self.stopped = true;
                AcceptControl::Stop
            }
        }
    }
}
fn certify_native_packet(
    e: &Engine,
    times: [f64; 2],
    native: &NativePacket,
    incoming: Option<f64>,
    work: &mut ProjectionWork,
) -> CoreResult<(Vec<Vec<Iv>>, SlabCertificate)> {
    let mut p = native.power()?;
    if e.route != EvolutionRoute::Mass {
        let (z, w) = e.reconstruct(times, &p, true)?;
        for q in &w {
            work.add(q, e.warp.is_some());
        }
        p.extend(z);
    }
    let c = e.certify(times, &p, incoming, true)?;
    if !c.global_qualified
        || c.whole_z_error.is_none_or(|v| v > e.policy.global_tol)
        || c.whole_x_error.is_none_or(|v| v > e.policy.global_tol)
    {
        return Err(bad("unqualified physical slab"));
    }
    Ok((p, c))
}
impl ThresholdScan {
    fn scan(&mut self, e: &Engine, times: [f64; 2], native: &NativePacket) -> CoreResult<bool> {
        let (p, c) =
            certify_native_packet(e, times, native, self.incoming, &mut self.work.projection)?;
        self.scan_certified(e, times, &p, &c)
    }
    fn scan_certified(
        &mut self,
        e: &Engine,
        times: [f64; 2],
        p: &[Vec<Iv>],
        c: &SlabCertificate,
    ) -> CoreResult<bool> {
        self.work.certified_slabs += 1;
        let proof = self
            .guard
            .as_ref()
            .map(|g| {
                let mut work = de::GuardKernelAudit::default();
                let r = de::GuardProofCore::new_recorded(
                    &e.src,
                    &g.source,
                    p,
                    c,
                    g.derivative,
                    g.value,
                    &mut work,
                );
                update_audit(&e.audit, |a| {
                    a.guard_build.record(r.is_ok());
                    a.guard_kernel.merge(&work);
                });
                r
            })
            .transpose()?;
        let velocity = if let Some(proof) = &proof {
            let v = proof.velocity();
            self.guard_slabs.push(proof.owned_evidence());
            Iv::new(v[0], v[1])?
        } else {
            de::threshold_velocity(&e.src, c)?
        };
        if velocity.lo <= 0. {
            return Err(bad("strict transverse increasing guard not proved"));
        }
        let incoming = self
            .incoming
            .ok_or_else(|| bad("unknown incoming radius"))?;
        let outgoing = c.outgoing.ok_or_else(|| bad("unknown outgoing radius"))?;
        let whole = c.whole_x_error.ok_or_else(|| bad("unknown whole radius"))?;
        let a = times[0];
        let b = times[1];
        let mut g = |t: f64, error: f64| {
            self.work.guard_evals += 1;
            if let Some(proof) = &proof {
                let v = proof.value_at(t, error)?;
                self.guard_slabs
                    .last_mut()
                    .expect("current guard evidence")
                    .evaluations
                    .push(de::GuardPointEvidence {
                        time: t,
                        differential_radius: error,
                        value: v.as_pair(),
                    });
                Ok(v)
            } else {
                guard_interval(&p[0], times, t, error, self.threshold)
            }
        };
        let ga = g(a, incoming)?;
        let gb = g(b, outgoing)?;
        if let Some(pending) = self.pending.as_mut() {
            if self.work.inspected == 1 && ga.hi < 0. {
                pending.report.blocked_initial = false;
            }
            if !pending.active() && ga.hi >= 0. {
                if self.work.inspected == 1 {
                    pending.report.blocked_initial = true;
                }
                pending.report.resolution = Some("uncertain-left");
                return Err(bad("uncertain left guard"));
            }
            if pending.active() || (gb.lo <= 0. && gb.hi >= 0.) {
                if !pending.room() {
                    pending.report.resolution = Some("slab-cap");
                    return Err(bad("pending slab cap exhausted"));
                }
                let probe = if pending.report.policy.method == PendingMethod::PiecewiseProbe {
                    let mut t = a * 0.5 + b * 0.5;
                    if self.route == ThresholdRoute::DictionaryHint {
                        use rodas5p_integrators::retro_event_poly::{
                            RepresentedPolynomial, RootRoute,
                        };
                        self.work.hint_calls += 1;
                        let cp = proof
                            .as_ref()
                            .expect("declared pending guard")
                            .hint_coefficients();
                        if let Ok(r) = RepresentedPolynomial::from_power(&cp)
                            .and_then(|r| r.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-35)))
                        {
                            self.work.hint_dictionary_uses += r.work.dictionary_uses;
                            if r.roots.len() == 1 {
                                let u = r.roots[0].theta;
                                let tt = a + (b - a) * (u[0] * 0.5 + u[1] * 0.5);
                                if a < tt && tt < b {
                                    t = tt;
                                }
                            }
                        }
                    }
                    if a < t && t < b {
                        Some((t, g(t, whole)?.as_pair()))
                    } else {
                        None
                    }
                } else {
                    None
                };
                pending.add(PendingSlab {
                    times,
                    left: ga.as_pair(),
                    right: gb.as_pair(),
                    velocity: velocity.as_pair(),
                    probe,
                    incoming_radius: incoming,
                    outgoing_radius: outgoing,
                    safe_before: self.safe,
                })?;
                self.incoming = Some(outgoing);
                if gb.hi < 0. {
                    // Monotonicity through the pending episode proves the
                    // entire numerical continuation was still pre-event.
                    self.safe = b;
                    pending.active_start = None;
                    pending.report.resolution = Some("negative-recovered");
                    return Ok(false);
                }
                if gb.lo > 0. {
                    let i = pending.enclose()?;
                    if diameter(i[0], i[1])? > self.tolerance {
                        pending.report.resolution = Some("width-unresolved");
                        return Err(bad("pending physical root width exceeds tolerance"));
                    }
                    self.root = Some(i);
                    self.safe = i[0];
                    pending.active_start = None;
                    pending.report.resolution = Some("certified");
                    return Ok(true);
                }
                pending.report.resolution = Some("awaiting-positive");
                return Ok(false);
            }
        }
        if ga.hi >= 0. {
            return Err(bad("uncertain left guard"));
        }
        if gb.hi < 0. {
            self.safe = b;
            self.incoming = Some(outgoing);
            return Ok(false);
        }
        if gb.lo <= 0. {
            return Err(bad("uncertain right guard; refine before committing"));
        }
        // Unique physical root by strict signs and uniform positive derivative.
        let mut lo = a;
        let mut hi = b;
        let mut hint = None;
        if self.route == ThresholdRoute::DictionaryHint {
            use rodas5p_integrators::retro_event_poly::{RepresentedPolynomial, RootRoute};
            self.work.hint_calls += 1;
            let cp = if let Some(proof) = &proof {
                proof.hint_coefficients()
            } else {
                let mut cp = p[0]
                    .iter()
                    .map(|v| v.lo * 0.5 + v.hi * 0.5)
                    .collect::<Vec<_>>();
                cp[0] -= self.threshold;
                cp
            };
            if let Ok(r) = RepresentedPolynomial::from_power(&cp)
                .and_then(|r| r.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-35)))
            {
                self.work.hint_dictionary_uses += r.work.dictionary_uses;
                if r.roots.len() == 1 {
                    let u = r.roots[0].theta;
                    let t = a + (b - a) * (u[0] * 0.5 + u[1] * 0.5);
                    if t > a && t < b {
                        hint = Some(t);
                    }
                }
            }
        }
        for _ in 0..64 {
            if diameter(lo, hi)? <= self.tolerance {
                self.root = Some([lo, hi]);
                self.safe = lo;
                self.incoming = Some(outgoing);
                return Ok(true);
            }
            let mid = hint.take().unwrap_or(lo * 0.5 + hi * 0.5);
            if !(lo < mid && mid < hi) {
                break;
            }
            let v = g(mid, whole)?;
            let old = (lo, hi);
            if self.route != ThresholdRoute::Bisection {
                let n = Iv::point(mid)?.sub(v.div(velocity)?)?;
                lo = lo.max(n.lo);
                hi = hi.min(n.hi);
                if lo > hi {
                    return Err(bad("inconsistent physical root enclosure"));
                }
            } else if v.hi < 0. {
                lo = mid;
            } else if v.lo > 0. {
                hi = mid;
            } else {
                let left = lo * 0.5 + mid * 0.5;
                let right = mid * 0.5 + hi * 0.5;
                if g(left, whole)?.hi < 0. {
                    lo = left;
                }
                if g(right, whole)?.lo > 0. {
                    hi = right;
                }
            }
            if old == (lo, hi) {
                break;
            }
        }
        if diameter(lo, hi)? <= self.tolerance {
            self.root = Some([lo, hi]);
            self.safe = lo;
            return Ok(true);
        }
        Err(bad("physical root width exceeds tolerance"))
    }
}
/// The numerical continuation is private: only the certified old-mode
/// prefix can be queried. All trial solver work stays in stats.
pub struct ThresholdRun {
    trial: EvolutionRun,
    root: Option<[f64; 2]>,
    committed_until: f64,
    numerical_end: f64,
    reason: Option<String>,
    pub work: ThresholdWork,
    guard_slabs: Vec<GuardSlabEvidence>,
    pending_report: Option<PendingReport>,
}
impl ThresholdRun {
    pub fn pending_report(&self) -> Option<&PendingReport> {
        self.pending_report.as_ref()
    }
    pub fn guard_slabs(&self) -> &[GuardSlabEvidence] {
        &self.guard_slabs
    }
    pub fn root(&self) -> Option<[f64; 2]> {
        self.root
    }
    pub fn committed_until(&self) -> f64 {
        self.committed_until
    }
    pub fn numerical_end(&self) -> f64 {
        self.numerical_end
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn stats(&self) -> &crate::Stats {
        &self.trial.driver.stats
    }
    /// Recorded projection work of the numerical driver and certificate replay.
    /// Guard-scanning projection work is recorded separately in work.
    pub fn projection_work(&self) -> &ProjectionWork {
        &self.trial.work
    }
    pub fn pre_query(&self, t: f64) -> CoreResult<Query> {
        if self
            .pending_report
            .as_ref()
            .is_some_and(|r| r.blocked_initial)
            || !t.is_finite()
            || t > self.committed_until
        {
            return Err(bad("outside certified pre-event prefix"));
        }
        let q = self.trial.query(t)?;
        if q.physical_error_upper
            .is_none_or(|b| b > self.trial.engine.policy.global_tol)
        {
            return Err(bad("unqualified physical query"));
        }
        Ok(q)
    }
}
/// Single strictly increasing scalar differential threshold. No claim for
/// grazing, opaque or competing guards. The caller supplies initial authority.
#[allow(clippy::too_many_arguments)]
pub fn evolve_to_threshold(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    threshold: f64,
    tolerance: f64,
    root_route: ThresholdRoute,
) -> CoreResult<ThresholdRun> {
    if src.nx() != 1
        || x.len() != 1
        || !threshold.is_finite()
        || !tolerance.is_finite()
        || tolerance <= 0.
    {
        return Err(bad("scalar threshold contract"));
    }
    let rad = epoch
        .incoming_x_error
        .ok_or_else(|| bad("physical incoming radius required"))?;
    if !rad.is_finite() || rad < 0. || Iv::point(x[0])?.add(Iv::new(-rad, rad)?)?.hi >= threshold {
        return Err(bad("initial guard not strictly below threshold"));
    }
    let mut scan = ThresholdScan {
        pending: None,
        threshold,
        guard: None,
        guard_slabs: vec![],
        tolerance,
        route: root_route,
        incoming: Some(rad),
        safe: epoch.start,
        numerical_end: epoch.start,
        root: None,
        stopped: false,
        reason: None,
        work: ThresholdWork::default(),
    };
    let trial = evolve_epoch_impl(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        &[],
        Some(&mut scan),
    )?;
    if !scan.stopped && (!trial.driver.is_success() || trial.driver.t.last() != Some(&epoch.end)) {
        scan.reason = Some("native driver ended before certified event or horizon".into());
    }
    if scan.root.is_some() && !trial.branch_qualified {
        scan.root = None;
        scan.safe = epoch.start;
        scan.reason = Some("replayed physical branch proof failed".into());
    }
    Ok(ThresholdRun {
        trial,
        root: scan.root,
        committed_until: scan.safe,
        numerical_end: scan.numerical_end,
        reason: scan.reason,
        work: scan.work,
        guard_slabs: scan.guard_slabs,
        pending_report: scan.pending.map(|p| p.report),
    })
}

/// Source-owned single polynomial guard H(t,x,z)=0 on the certified branch.
/// Initial physical sign must be negative and all inspected slabs strictly
/// increasing in H. No root can be authorized by a polynomial hint alone.
#[allow(clippy::too_many_arguments)]
pub fn evolve_to_guard(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    guard: &rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
    tolerance: f64,
    root_route: ThresholdRoute,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
) -> CoreResult<ThresholdRun> {
    evolve_to_guard_impl(
        src, x, z, branch, epoch, opts, policy, route, guard, tolerance, root_route, derivative,
        value, None,
    )
}
/// Bounded speculative continuation in the ORIGINAL mode. A source
/// extension across the event is required; only the safe prefix is exposed.
#[allow(clippy::too_many_arguments)]
pub fn evolve_to_guard_pending(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    guard: &rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
    tolerance: f64,
    root_route: ThresholdRoute,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
    pending: PendingPolicy,
) -> CoreResult<ThresholdRun> {
    evolve_to_guard_impl(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        guard,
        tolerance,
        root_route,
        derivative,
        value,
        Some(PendingState::new(pending)?),
    )
}
#[allow(clippy::too_many_arguments)]
fn evolve_to_guard_impl(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    guard: &rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
    tolerance: f64,
    root_route: ThresholdRoute,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
    pending: Option<PendingState>,
) -> CoreResult<ThresholdRun> {
    guard.validate(src.nx() + src.nz())?;
    if x.len() != src.nx() || !tolerance.is_finite() || tolerance <= 0. {
        return Err(bad("declared guard contract"));
    }
    let rad = epoch
        .incoming_x_error
        .ok_or_else(|| bad("physical incoming radius required"))?;
    if !rad.is_finite() || rad < 0. {
        return Err(bad("invalid incoming radius"));
    }
    let mut scan = ThresholdScan {
        pending,
        threshold: 0.,
        guard: Some(DeclaredGuard {
            source: guard.clone(),
            derivative,
            value,
        }),
        guard_slabs: vec![],
        tolerance,
        route: root_route,
        incoming: Some(rad),
        safe: epoch.start,
        numerical_end: epoch.start,
        root: None,
        stopped: false,
        reason: None,
        work: ThresholdWork::default(),
    };
    let trial = evolve_epoch_impl(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        &[],
        Some(&mut scan),
    )?;
    if !scan.stopped && (!trial.driver.is_success() || trial.driver.t.last() != Some(&epoch.end)) {
        scan.reason = Some("native driver ended before certified event or horizon".into());
    }
    if scan.root.is_some() && !trial.branch_qualified {
        scan.root = None;
        scan.safe = epoch.start;
        scan.reason = Some("replayed physical branch proof failed".into());
    }
    if let Some(p) = scan.pending.as_mut() {
        if p.active() && scan.reason.is_none() && scan.root.is_none() {
            p.report.resolution = Some("horizon-unresolved");
            scan.reason = Some("pending guard unresolved at horizon".into());
        }
        if scan.root.is_some() && (!trial.global_qualified || !trial.pointwise_qualified) {
            scan.root = None;
            scan.safe = epoch.start;
            scan.reason = Some("pending replay physical proof failed".into());
            p.report.resolution = Some("replay-unqualified");
        }
        if scan.reason.is_some() && p.report.resolution == Some("awaiting-positive") {
            p.report.resolution = Some("continuation-refused");
        }
    }
    Ok(ThresholdRun {
        trial,
        root: scan.root,
        committed_until: scan.safe,
        numerical_end: scan.numerical_end,
        reason: scan.reason,
        work: scan.work,
        guard_slabs: scan.guard_slabs,
        pending_report: scan.pending.map(|p| p.report),
    })
}

/// The differential reset restricted to the single threshold is
/// x^+ = offset + time_scale * tau. The new differential source is
/// constructed here, not accepted as an unrelated user callback.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct AffineDaeReset {
    pub rate: f64,
    pub equilibrium: f64,
    pub offset: f64,
    pub time_scale: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AffineSyncRoute {
    Rectangle,
    GreenMoment,
    CriticalPoint,
}
#[derive(Serialize)]
pub struct AffineRestartRun {
    pub sync_route: AffineSyncRoute,
    pub event_time: [f64; 2],
    pub synchronized_x: [f64; 2],
    pub incoming_radius: f64,
    pub post: EvolutionRun,
}
impl AffineDaeReset {
    fn sync(self, times: [f64; 2], route: AffineSyncRoute) -> CoreResult<Iv> {
        use rodas5p_core::retro_dictionary::RealMode;
        use rodas5p_integrators::retro_green::RetroTable;
        let table = RetroTable::with_dictionary(RealMode::Hybrid)?;
        let s = Iv::point(times[1])?;
        let t = Iv::new(times[0], times[1])?;
        let a = Iv::point(self.rate)?;
        let e = Iv::point(self.equilibrium)?;
        let aa = Iv::point(self.offset)?;
        let b = Iv::point(self.time_scale)?;
        let at = |t: Iv| -> CoreResult<Iv> {
            let dt = s.sub(t)?.mul(a)?;
            let gain = table.phi(Iv::new(dt.lo.max(0.), dt.hi)?, 0)?;
            e.add(gain.mul(aa.add(b.mul(t)?)?.sub(e)?)?)
        };
        match route {
            AffineSyncRoute::Rectangle => at(t),
            AffineSyncRoute::GreenMoment => {
                let dt = s.sub(t)?;
                let u = Iv::new(0., dt.hi)?;
                let au = a.mul(u)?;
                let au = Iv::new(au.lo.max(0.), au.hi)?;
                let ph1 = table.phi(au, 1)?;
                let ph2 = table.phi(au, 2)?;
                let d = ph1.sub(ph2)?;
                let d = Iv::new(d.lo.max(0.), d.hi)?;
                let rs = aa.add(b.mul(s)?)?;
                rs.sub(b.add(a.mul(rs.sub(e)?)?)?.mul(u)?.mul(ph1)?)?
                    .add(a.mul(b)?.mul(u)?.mul(u)?.mul(d)?)
            }
            AffineSyncRoute::CriticalPoint => {
                let l = at(Iv::point(times[0])?)?;
                let r = at(s)?;
                let mut out = Iv::new(l.lo.min(r.lo), l.hi.max(r.hi))?;
                if self.time_scale != 0. {
                    let turn = -(b.add(a.mul(aa.sub(e)?)?)?.div(a.mul(b)?)?);
                    let lo = turn.lo.max(t.lo);
                    let hi = turn.hi.min(t.hi);
                    if lo <= hi {
                        let v = at(Iv::new(lo, hi)?)?;
                        out = Iv::new(out.lo.min(v.lo), out.hi.max(v.hi))?;
                    }
                }
                Ok(out)
            }
        }
    }
}
impl ThresholdRun {
    /// A single mode transition. No second guard is declared in this API.
    /// Do not restart without a physically certified root. Algebraic
    /// projection authority is explicit; no reset of error radius to zero.
    #[allow(clippy::too_many_arguments)]
    pub fn restart_affine(
        &self,
        reset: AffineDaeReset,
        constraints: Vec<rodas5p_integrators::retro_event_poly::DeclaredPolynomial>,
        z_seed: &[f64],
        branch: &[[f64; 2]],
        end: f64,
        opts: &Options,
        policy: EvolutionPolicy,
        route: EvolutionRoute,
        sync_route: AffineSyncRoute,
        grid: &[f64],
    ) -> CoreResult<AffineRestartRun> {
        use rodas5p_integrators::retro_dae_init::InitMode;
        use rodas5p_integrators::retro_event_poly::{AffineFactor, DeclaredPolynomial, EventTerm};
        let root = self
            .root
            .filter(|_| self.reason.is_none())
            .ok_or_else(|| bad("no physical event authority"))?;
        if policy.init.mode != InitMode::ProjectAlgebraic
            || !reset.rate.is_finite()
            || reset.rate <= 0.
            || ![reset.equilibrium, reset.offset, reset.time_scale, end]
                .iter()
                .all(|v| v.is_finite())
            || end <= root[1]
        {
            return Err(bad(
                "affine reset requires explicit algebraic projection and positive rate",
            ));
        }
        let forcing = Iv::point(reset.rate)?.mul(Iv::point(reset.equilibrium)?)?;
        if forcing.lo != forcing.hi {
            return Err(bad("affine source coefficient not exactly represented"));
        }
        let src = DaeSource::new(
            vec![DeclaredPolynomial::new(vec![
                EventTerm {
                    coefficient: -reset.rate,
                    factors: vec![AffineFactor::component(0, 0.)],
                },
                EventTerm {
                    coefficient: forcing.lo,
                    factors: vec![],
                },
            ])?],
            constraints,
        )?;
        let sync = reset.sync(root, sync_route)?;
        let center = sync.lo * 0.5 + sync.hi * 0.5;
        let radius = diameter(sync.lo, center)?.max(diameter(center, sync.hi)?);
        let post = evolve_epoch(
            &src,
            &[center],
            z_seed,
            branch,
            EvolutionEpoch {
                start: root[1],
                end,
                incoming_x_error: Some(radius),
            },
            opts,
            policy,
            route,
            grid,
        )?;
        Ok(AffineRestartRun {
            sync_route,
            event_time: root,
            synchronized_x: sync.as_pair(),
            incoming_radius: radius,
            post,
        })
    }
}

/// Receipt for one source-bound checkpoint. Radius encloses the actual
/// differential state at the represented checkpoint center.
#[derive(Clone, Debug, Serialize)]
pub struct GuardCheckpoint {
    pub time: f64,
    pub differential: Vec<f64>,
    pub algebraic: Vec<f64>,
    pub radius: f64,
    pub projection: InitWork,
    pub retry_h0: f64,
    pub remaining_attempts: usize,
}
/// Private ownership of both numerical attempts and the original safe prefix.
/// A retry failure cannot erase the first run or authorize a reset.
pub struct GuardCheckpointRun {
    first: ThresholdRun,
    retry: Option<ThresholdRun>,
    checkpoint: Option<GuardCheckpoint>,
    retry_error: Option<String>,
}
impl GuardCheckpointRun {
    pub fn first(&self) -> &ThresholdRun {
        &self.first
    }
    pub fn retry(&self) -> Option<&ThresholdRun> {
        self.retry.as_ref()
    }
    pub fn checkpoint(&self) -> Option<&GuardCheckpoint> {
        self.checkpoint.as_ref()
    }
    pub fn retry_error(&self) -> Option<&str> {
        self.retry_error.as_deref()
    }
    pub fn attempts_used(&self) -> usize {
        self.first.stats().attempts + self.retry.as_ref().map_or(0, |r| r.stats().attempts)
    }
    pub fn root(&self) -> Option<[f64; 2]> {
        self.first
            .root()
            .or_else(|| self.retry.as_ref().and_then(|r| r.root()))
    }
    pub fn committed_until(&self) -> f64 {
        self.retry
            .as_ref()
            .map_or(self.first.committed_until(), |r| {
                self.first.committed_until().max(r.committed_until())
            })
    }
    pub fn pre_query(&self, t: f64) -> CoreResult<Query> {
        if t <= self.first.committed_until() {
            self.first.pre_query(t)
        } else {
            self.retry
                .as_ref()
                .ok_or_else(|| bad("no certified retry prefix"))?
                .pre_query(t)
        }
    }
    /// The selected certified event authority may be passed to the existing
    /// affine restart transaction. Never exposes an uncertified candidate.
    pub fn event_authority(&self) -> Option<&ThresholdRun> {
        if self.first.root().is_some() {
            Some(&self.first)
        } else {
            self.retry.as_ref().filter(|r| r.root().is_some())
        }
    }
}
/// One bounded re-integration from the last certified negative checkpoint.
/// Same immutable source, branch, guard, tolerance, pending cap, and shared
/// native attempt budget. This is not an in-place driver rollback.
#[allow(clippy::too_many_arguments)]
pub fn evolve_to_guard_checkpoint(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    guard: &rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
    tolerance: f64,
    root_route: ThresholdRoute,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
    pending: PendingPolicy,
) -> CoreResult<GuardCheckpointRun> {
    let first = evolve_to_guard_pending(
        src, x, z, branch, epoch, opts, policy, route, guard, tolerance, root_route, derivative,
        value, pending,
    )?;
    let mut out = GuardCheckpointRun {
        first,
        retry: None,
        checkpoint: None,
        retry_error: None,
    };
    let r = &out.first;
    if r.root().is_some()
        || !r.pending_report().is_some_and(|p| {
            matches!(p.resolution, Some("slab-cap" | "width-unresolved")) && !p.blocked_initial
        })
    {
        return Ok(out);
    }
    let make_checkpoint = || -> CoreResult<GuardCheckpoint> {
        let time = r.committed_until();
        let remaining = opts
            .max_steps
            .checked_sub(r.stats().attempts)
            .filter(|v| *v > 0)
            .ok_or_else(|| bad("shared attempt budget exhausted"))?;
        let q = r.pre_query(time)?;
        let slab = r
            .trial
            .slabs
            .get(r.trial.slabs.partition_point(|s| s.times[1] < time))
            .ok_or_else(|| bad("checkpoint slab missing"))?;
        let cert = slab
            .certificate
            .as_ref()
            .filter(|c| c.global_qualified)
            .ok_or_else(|| bad("checkpoint physical certificate missing"))?;
        let ein = if time == slab.times[0] {
            cert.incoming
        } else if time == slab.times[1] {
            cert.outgoing
                .ok_or_else(|| bad("checkpoint outgoing radius"))?
        } else {
            return Err(bad("checkpoint must be a native endpoint"));
        };
        let nx = src.nx();
        let p = slab.power[..nx]
            .iter()
            .map(|row| {
                row.iter()
                    .map(|v| Iv::new(v[0], v[1]))
                    .collect::<CoreResult<Vec<_>>>()
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let radius = de::query_x_error(&p, slab.times, time, &q.state[..nx], ein)?;
        let h0 = opts.h0 * 0.5;
        if !h0.is_finite() || h0 <= 0. || time + h0 <= time {
            return Err(bad("unrepresentable checkpoint refinement"));
        }
        Ok(GuardCheckpoint {
            time,
            differential: q.state[..nx].to_vec(),
            algebraic: q.state[nx..].to_vec(),
            radius,
            projection: q.projection.work,
            retry_h0: h0,
            remaining_attempts: remaining,
        })
    };
    let cp = match make_checkpoint() {
        Ok(cp) => cp,
        Err(e) => {
            out.retry_error = Some(e.to_string());
            return Ok(out);
        }
    };
    let mut retry_opts = opts.clone();
    retry_opts.h0 = cp.retry_h0;
    retry_opts.max_steps = cp.remaining_attempts;
    let retry = evolve_to_guard_pending(
        src,
        &cp.differential,
        &cp.algebraic,
        branch,
        EvolutionEpoch {
            start: cp.time,
            end: epoch.end,
            incoming_x_error: Some(cp.radius),
        },
        &retry_opts,
        policy,
        route,
        guard,
        tolerance,
        root_route,
        derivative,
        value,
        pending,
    );
    out.checkpoint = Some(cp);
    match retry {
        Ok(v) => out.retry = Some(v),
        Err(e) => out.retry_error = Some(e.to_string()),
    }
    Ok(out)
}

pub use de::{ResetEnvelopeRoute, ResetSlabEvidence, TransitionBranchProof};
/// Source-owned post differential flow, deliberately restricted to scalar
/// autonomous damping. The post algebraic source is declared separately.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DampedResetFlow {
    pub rate: f64,
    pub equilibrium: f64,
}
/// A rejected reset retains its evidence but does not expose a committed
/// post-event trajectory. Old event authority is immutable.
#[derive(Serialize)]
pub struct PolynomialRestartRun {
    pub event_time: [f64; 2],
    pub envelope_route: ResetEnvelopeRoute,
    pub slab_evidence: Vec<ResetSlabEvidence>,
    pub reset_x: [f64; 2],
    pub synchronized_x: [f64; 2],
    pub transition_x: [f64; 2],
    pub incoming_radius: f64,
    pub branch_proof: Option<TransitionBranchProof>,
    pub reason: Option<String>,
    post_trial: Option<EvolutionRun>,
}
impl PolynomialRestartRun {
    pub fn post(&self) -> Option<&EvolutionRun> {
        if self.reason.is_none() {
            self.post_trial
                .as_ref()
                .filter(|r| r.global_qualified && r.pointwise_qualified && r.branch_qualified)
        } else {
            None
        }
    }
    pub fn post_diagnostic(&self) -> Option<&EvolutionRun> {
        self.post_trial.as_ref()
    }
}
impl ThresholdRun {
    /// Polynomial state-dependent reset with source-owned root authority.
    /// The pre-event continuation is used privately only within the proven
    /// root interval. No external event time or arbitrary postflow callback.
    #[allow(clippy::too_many_arguments)]
    pub fn restart_polynomial(
        &self,
        reset: &rodas5p_integrators::retro_event_poly::DeclaredPolynomial,
        flow: DampedResetFlow,
        constraints: Vec<rodas5p_integrators::retro_event_poly::DeclaredPolynomial>,
        z_seed: &[f64],
        branch: &[[f64; 2]],
        end: f64,
        opts: &Options,
        policy: EvolutionPolicy,
        route: EvolutionRoute,
        envelope: ResetEnvelopeRoute,
        derivative: GuardDerivativeRoute,
        branch_route: InitRoute,
        grid: &[f64],
    ) -> CoreResult<PolynomialRestartRun> {
        use rodas5p_core::retro_dictionary::RealMode;
        use rodas5p_integrators::retro_dae_init::InitMode;
        use rodas5p_integrators::retro_event_poly::{AffineFactor, DeclaredPolynomial, EventTerm};
        use rodas5p_integrators::retro_green::RetroTable;
        let root = self
            .root
            .filter(|_| self.reason.is_none())
            .ok_or_else(|| bad("no physical event authority"))?;
        if !self.trial.global_qualified
            || !self.trial.pointwise_qualified
            || !self.trial.branch_qualified
            || policy.init.mode != InitMode::ProjectAlgebraic
            || !flow.rate.is_finite()
            || flow.rate <= 0.
            || !flow.equilibrium.is_finite()
            || !end.is_finite()
            || end <= root[1]
        {
            return Err(bad("polynomial reset source/flow authority"));
        }
        reset.validate(self.trial.engine.src.nx() + self.trial.engine.src.nz())?;
        let forcing = Iv::point(flow.rate)?.mul(Iv::point(flow.equilibrium)?)?;
        if forcing.lo != forcing.hi {
            return Err(bad("affine source coefficient not exactly represented"));
        }
        let post_src = DaeSource::new(
            vec![DeclaredPolynomial::new(vec![
                EventTerm {
                    coefficient: -flow.rate,
                    factors: vec![AffineFactor::component(0, 0.)],
                },
                EventTerm {
                    coefficient: forcing.lo,
                    factors: vec![],
                },
            ])?],
            constraints,
        )?;
        let table = RetroTable::with_dictionary(RealMode::Hybrid)?;
        let mut evidence = Vec::new();
        let mut covered = root[0];
        let mut reset_box: Option<Iv> = None;
        let mut sync_box: Option<Iv> = None;
        fn hull(old: Option<Iv>, pair: [f64; 2]) -> CoreResult<Iv> {
            let v = Iv::new(pair[0], pair[1])?;
            Ok(old.map_or(v, |u| Iv {
                lo: u.lo.min(v.lo),
                hi: u.hi.max(v.hi),
            }))
        }
        for s in &self.trial.slabs {
            let lo = root[0].max(s.times[0]);
            let hi = root[1].min(s.times[1]);
            if lo > hi {
                continue;
            }
            if lo > covered {
                return Err(bad("uncovered event clock"));
            }
            let c = s
                .certificate
                .as_ref()
                .filter(|c| c.global_qualified)
                .ok_or_else(|| bad("unqualified event slab"))?;
            let power = s
                .power
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|v| Iv::new(v[0], v[1]))
                        .collect::<CoreResult<Vec<_>>>()
                })
                .collect::<CoreResult<Vec<_>>>()?;
            let e = de::reset_slab(
                &self.trial.engine.src,
                reset,
                &power,
                c,
                [lo, hi],
                root[1],
                flow.rate,
                flow.equilibrium,
                envelope,
                derivative,
                &table,
            )?;
            reset_box = Some(hull(reset_box, e.reset_x)?);
            sync_box = Some(hull(sync_box, e.synchronized_x)?);
            covered = covered.max(hi);
            evidence.push(e);
        }
        if covered != root[1] || evidence.is_empty() {
            return Err(bad("event slab coverage incomplete"));
        }
        let reset_box = reset_box.expect("nonempty reset");
        let sync = sync_box.expect("nonempty sync");
        let center = (sync.lo * 0.5 + sync.hi * 0.5).clamp(sync.lo, sync.hi);
        let radius = diameter(sync.lo, center)?.max(diameter(center, sync.hi)?);
        let ball = Iv::point(center)?.add(Iv::new(-radius, radius)?)?;
        // Each affine damped trajectory is monotone between its own reset
        // and synchronization value. Include the full rounded restart ball.
        let transition = Iv::new(reset_box.lo.min(ball.lo), reset_box.hi.max(ball.hi))?;
        let mut out = PolynomialRestartRun {
            event_time: root,
            envelope_route: envelope,
            slab_evidence: evidence,
            reset_x: reset_box.as_pair(),
            synchronized_x: sync.as_pair(),
            transition_x: transition.as_pair(),
            incoming_radius: radius,
            branch_proof: None,
            reason: None,
            post_trial: None,
        };
        let proof = match de::certify_transition_branch(
            &post_src,
            root,
            transition.as_pair(),
            center,
            z_seed,
            branch,
            policy.init,
            branch_route,
        ) {
            Ok(v) => v,
            Err(e) => {
                out.reason = Some(e.to_string());
                return Ok(out);
            }
        };
        let z = proof.initial.z.clone();
        out.branch_proof = Some(proof);
        match evolve_epoch(
            &post_src,
            &[center],
            &z,
            branch,
            EvolutionEpoch {
                start: root[1],
                end,
                incoming_x_error: Some(radius),
            },
            opts,
            policy,
            route,
            grid,
        ) {
            Ok(v) => {
                if !v.global_qualified || !v.pointwise_qualified || !v.branch_qualified {
                    out.reason = Some("post physical trajectory not qualified".into());
                }
                out.post_trial = Some(v);
            }
            Err(e) => out.reason = Some(e.to_string()),
        }
        Ok(out)
    }
}

pub use de::{
    VectorBranchProof, VectorResetComponent, VectorSyncEvidence, VectorSyncRoute, VectorTubeProof,
};
#[derive(Clone, Copy, Debug, Serialize)]
pub struct VectorResetPolicy {
    pub sync: VectorSyncRoute,
    pub derivative: GuardDerivativeRoute,
    pub branch_route: InitRoute,
    pub subdivisions: usize,
}
/// Transaction-local diagnostics, not reset authority. A returned
/// preparation can carry a refusal reason and must not imply commit.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ResetTransactionAudit {
    pub preparation: AttemptCount,
    pub table_setup: AttemptCount,
    pub cells: AttemptCount,
    pub restart_branch: AttemptCount,
    pub kernel: de::ResetKernelAudit,
    pub transport: AttemptCount,
    pub transport_cells: AttemptCount,
    pub transport_qualified: usize,
    pub gap_checks: AttemptCount,
    pub negative_gaps: usize,
    pub committed: bool,
}
#[derive(Serialize)]
pub struct VectorResetCell {
    pub times: [f64; 2],
    pub reset: Vec<VectorResetComponent>,
    pub tube: Option<VectorTubeProof>,
    pub sync: Option<VectorSyncEvidence>,
    pub reason: Option<String>,
}
#[derive(Serialize)]
pub struct VectorRestartRun {
    pub event_time: [f64; 2],
    pub policy: VectorResetPolicy,
    pub cells: Vec<VectorResetCell>,
    pub synchronized_x: Option<Vec<[f64; 2]>>,
    pub incoming_radius: Option<f64>,
    pub restart_branch: Option<VectorBranchProof>,
    pub reason: Option<String>,
    post_trial: Option<EvolutionRun>,
}
impl VectorRestartRun {
    pub fn post(&self) -> Option<&EvolutionRun> {
        if self.reason.is_none() {
            self.post_trial
                .as_ref()
                .filter(|r| r.global_qualified && r.branch_qualified && r.pointwise_qualified)
        } else {
            None
        }
    }
    pub fn post_diagnostic(&self) -> Option<&EvolutionRun> {
        self.post_trial.as_ref()
    }
}
impl ThresholdRun {
    /// Source-bound vector polynomial reset and short uncertain-clock
    /// flowpipe for a genuinely coupled original index-one post source.
    #[allow(clippy::too_many_arguments)]
    pub fn restart_vector(
        &self,
        reset: &[rodas5p_integrators::retro_event_poly::DeclaredPolynomial],
        post_src: &DaeSource,
        z_seed: &[f64],
        branch: &[[f64; 2]],
        end: f64,
        opts: &Options,
        policy: EvolutionPolicy,
        route: EvolutionRoute,
        reset_policy: VectorResetPolicy,
        grid: &[f64],
    ) -> CoreResult<VectorRestartRun> {
        let mut out = self.prepare_vector_restart(
            reset,
            post_src,
            z_seed,
            branch,
            end,
            policy,
            reset_policy,
            64,
        )?;
        if out.reason.is_some() {
            return Ok(out);
        }
        let proof = out.restart_branch.as_ref().expect("prepared branch");
        let center = proof.initial.x.clone();
        let z = proof.initial.z.clone();
        let root = out.event_time;
        let radius = out.incoming_radius.expect("prepared radius");
        match evolve_epoch(
            post_src,
            &center,
            &z,
            branch,
            EvolutionEpoch {
                start: root[1],
                end,
                incoming_x_error: Some(radius),
            },
            opts,
            policy,
            route,
            grid,
        ) {
            Ok(v) => {
                if !v.global_qualified || !v.branch_qualified || !v.pointwise_qualified {
                    out.reason = Some("vector post physical trajectory not qualified".into());
                }
                out.post_trial = Some(v);
            }
            Err(e) => out.reason = Some(e.to_string()),
        }
        Ok(out)
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_vector_restart(
        &self,
        reset: &[DeclaredPolynomial],
        post_src: &DaeSource,
        z_seed: &[f64],
        branch: &[[f64; 2]],
        end: f64,
        policy: EvolutionPolicy,
        reset_policy: VectorResetPolicy,
        max_cells: usize,
    ) -> CoreResult<VectorRestartRun> {
        self.prepare_vector_restart_recorded(
            reset,
            post_src,
            z_seed,
            branch,
            end,
            policy,
            reset_policy,
            max_cells,
            &mut ResetTransactionAudit::default(),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_vector_restart_recorded(
        &self,
        reset: &[DeclaredPolynomial],
        post_src: &DaeSource,
        z_seed: &[f64],
        branch: &[[f64; 2]],
        end: f64,
        policy: EvolutionPolicy,
        reset_policy: VectorResetPolicy,
        max_cells: usize,
        audit: &mut ResetTransactionAudit,
    ) -> CoreResult<VectorRestartRun> {
        let r = self.prepare_vector_restart_inner(
            reset,
            post_src,
            z_seed,
            branch,
            end,
            policy,
            reset_policy,
            max_cells,
            audit,
        );
        audit.preparation.record(r.is_ok());
        r
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_vector_restart_inner(
        &self,
        reset: &[DeclaredPolynomial],
        post_src: &DaeSource,
        z_seed: &[f64],
        branch: &[[f64; 2]],
        end: f64,
        policy: EvolutionPolicy,
        reset_policy: VectorResetPolicy,
        max_cells: usize,
        audit: &mut ResetTransactionAudit,
    ) -> CoreResult<VectorRestartRun> {
        use rodas5p_core::retro_dictionary::RealMode;
        use rodas5p_integrators::{retro_dae_init::InitMode, retro_green::RetroTable};
        let root = self
            .root
            .filter(|_| self.reason.is_none())
            .ok_or_else(|| bad("no physical event authority"))?;
        if !self.trial.global_qualified
            || !self.trial.pointwise_qualified
            || !self.trial.branch_qualified
            || policy.init.mode != InitMode::ProjectAlgebraic
            || !end.is_finite()
            || end <= root[1]
            || reset.len() != post_src.nx()
            || !matches!(reset_policy.subdivisions, 1 | 4)
        {
            return Err(bad("vector reset authority/domain"));
        }
        for r in reset {
            r.validate(self.trial.engine.src.nx() + self.trial.engine.src.nz())?;
        }
        let table = RetroTable::with_dictionary(RealMode::Hybrid);
        audit.table_setup.record(table.is_ok());
        let table = table?;
        let mut out = VectorRestartRun {
            event_time: root,
            policy: reset_policy,
            cells: vec![],
            synchronized_x: None,
            incoming_radius: None,
            restart_branch: None,
            reason: None,
            post_trial: None,
        };
        let mut covered = root[0];
        let mut total: Option<Vec<[f64; 2]>> = None;
        for slab in &self.trial.slabs {
            let lo = root[0].max(slab.times[0]);
            let hi = root[1].min(slab.times[1]);
            if lo > hi {
                continue;
            }
            if lo > covered {
                return Err(bad("vector reset coverage gap"));
            }
            let c = slab
                .certificate
                .as_ref()
                .filter(|v| v.global_qualified)
                .ok_or_else(|| bad("uncertified vector reset slab"))?;
            let power = slab
                .power
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|v| Iv::new(v[0], v[1]))
                        .collect::<CoreResult<Vec<_>>>()
                })
                .collect::<CoreResult<Vec<_>>>()?;
            let n = if lo == hi {
                1
            } else {
                reset_policy.subdivisions
            };
            let mut a = lo;
            for j in 1..=n {
                let b = if j == n {
                    hi
                } else {
                    (lo + (hi - lo) * (j as f64 / n as f64)).clamp(a, hi)
                };
                if out.cells.len() >= max_cells {
                    out.reason = Some("vector reset cell cap exhausted".into());
                    return Ok(out);
                }
                let mut cell = VectorResetCell {
                    times: [a, b],
                    reset: vec![],
                    tube: None,
                    sync: None,
                    reason: None,
                };
                let result = (|| -> CoreResult<()> {
                    for r in reset {
                        cell.reset.push(de::vector_reset_component_recorded(
                            &self.trial.engine.src,
                            r,
                            &power,
                            c,
                            [a, b],
                            reset_policy.sync,
                            reset_policy.derivative,
                            &mut audit.kernel,
                        )?);
                    }
                    let r = cell.reset.iter().map(|v| v.enclosure).collect::<Vec<_>>();
                    let tube = de::certify_vector_tube_recorded(
                        post_src,
                        [a, root[1]],
                        &r,
                        z_seed,
                        branch,
                        policy.init,
                        reset_policy.branch_route,
                        &mut audit.kernel,
                    )?;
                    let sync = de::synchronize_vector_recorded(
                        &cell.reset,
                        [a, b],
                        root[1],
                        &tube,
                        reset_policy.sync,
                        &table,
                        &mut audit.kernel,
                    );
                    cell.tube = Some(tube);
                    let sync = sync?;
                    if let Some(h) = total.as_mut() {
                        for (v, &w) in h.iter_mut().zip(&sync.synchronized) {
                            v[0] = v[0].min(w[0]);
                            v[1] = v[1].max(w[1]);
                        }
                    } else {
                        total = Some(sync.synchronized.clone());
                    }
                    cell.sync = Some(sync);
                    Ok(())
                })();
                audit.cells.record(result.is_ok());
                if let Err(e) = result {
                    let e = e.to_string();
                    cell.reason = Some(e.clone());
                    out.cells.push(cell);
                    out.reason = Some(e);
                    return Ok(out);
                }
                out.cells.push(cell);
                a = b;
            }
            covered = covered.max(hi);
        }
        if covered != root[1] || total.is_none() {
            return Err(bad("vector reset coverage incomplete"));
        }
        let sync = total.expect("nonempty synchronized box");
        let center = sync
            .iter()
            .map(|v| (v[0] * 0.5 + v[1] * 0.5).clamp(v[0], v[1]))
            .collect::<Vec<_>>();
        let mut radius = 0_f64;
        for (b, &c) in sync.iter().zip(&center) {
            radius = radius.max(diameter(b[0], c)?.max(diameter(c, b[1])?));
        }
        let ball = center
            .iter()
            .map(|&v| {
                Iv::point(v)?
                    .add(Iv::new(-radius, radius)?)
                    .map(|x| x.as_pair())
            })
            .collect::<CoreResult<Vec<_>>>()?;
        out.synchronized_x = Some(sync);
        out.incoming_radius = Some(radius);
        let result = de::certify_vector_branch_recorded(
            post_src,
            [root[1], root[1]],
            &ball,
            &center,
            z_seed,
            branch,
            policy.init,
            reset_policy.branch_route,
            &mut audit.kernel,
        );
        audit.restart_branch.record(result.is_ok());
        let proof = match result {
            Ok(v) => v,
            Err(e) => {
                out.reason = Some(e.to_string());
                return Ok(out);
            }
        };
        out.restart_branch = Some(proof);
        Ok(out)
    }
}

use rodas5p_integrators::retro_event_poly::DeclaredPolynomial;

/// One physical initial family, 2..8 competing increasing guards. Independent
/// repeats native integration; Shared and Correlated use one accepted-step run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum MultiGuardRoute {
    Independent,
    Shared,
    Correlated,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct MultiGuardPolicy {
    pub method: MultiGuardRoute,
    pub tolerance: f64,
    pub root_route: ThresholdRoute,
    pub derivative: GuardDerivativeRoute,
    pub value: GuardValueRoute,
    pub pending: PendingPolicy,
    pub max_slabs: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct GuardOrderEvidence {
    pub index: usize,
    pub root: Option<[f64; 2]>,
    pub safe: f64,
    pub initial_negative: bool,
    pub reason: Option<String>,
    pub work: ThresholdWork,
    pub slabs: Vec<GuardSlabEvidence>,
    pub pending: Option<PendingReport>,
}
#[derive(Clone, Debug, Serialize)]
pub struct GuardDifferenceCell {
    pub times: [f64; 2],
    pub enclosure: [f64; 2],
    pub prepared_work: InitWork,
}
#[derive(Clone, Debug, Serialize)]
pub struct GuardOrderComparison {
    pub earlier: usize,
    pub later: usize,
    pub compared_root: [f64; 2],
    pub cells: Vec<GuardDifferenceCell>,
    pub positive: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct MultiGuardWork {
    pub native_runs: usize,
    pub attempts: usize,
    pub driver_rhs: u64,
    pub physical_certificates: usize,
    pub shared_projection: ProjectionWork,
    pub replay_projection_rhs: usize,
    pub independent_scan_projection_rhs: usize,
    pub guard_evals: usize,
    pub hint_calls: usize,
    pub dictionary_uses: usize,
    pub guard_prepared_rhs: usize,
    pub comparison_attempts: usize,
    pub difference_cells: usize,
    pub difference_prepared_rhs: usize,
}
impl MultiGuardWork {
    fn driver(&mut self, r: &EvolutionRun) {
        self.native_runs += 1;
        self.attempts += r.driver.stats.attempts;
        self.driver_rhs += r.driver.stats.counters.rhs_calls;
        self.replay_projection_rhs += r.work.rhs_count;
    }
    fn guard(&mut self, r: &GuardOrderEvidence, independent: bool) {
        self.guard_evals += r.work.guard_evals;
        self.hint_calls += r.work.hint_calls;
        self.dictionary_uses += r.work.hint_dictionary_uses;
        self.guard_prepared_rhs += r
            .slabs
            .iter()
            .map(|s| s.prepared_work.rhs_count)
            .sum::<usize>();
        if independent {
            self.physical_certificates += r.work.certified_slabs;
            self.independent_scan_projection_rhs += r.work.projection.rhs_count;
        }
    }
}
/// Only this facade may authorize one of the private single-guard runs.
pub struct MultiGuardRun {
    method: MultiGuardRoute,
    guards: Vec<GuardOrderEvidence>,
    comparisons: Vec<GuardOrderComparison>,
    winner: Option<usize>,
    safe: f64,
    initial_negative: bool,
    reason: Option<String>,
    runs: Vec<ThresholdRun>,
    pub work: MultiGuardWork,
}
impl MultiGuardRun {
    pub fn winner(&self) -> Option<usize> {
        self.winner
    }
    pub fn guards(&self) -> &[GuardOrderEvidence] {
        &self.guards
    }
    pub fn comparisons(&self) -> &[GuardOrderComparison] {
        &self.comparisons
    }
    pub fn committed_until(&self) -> f64 {
        self.safe
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    /// Diagnostic native Stats for every actual returned run, even
    /// without a winning guard. This confers no event authority.
    pub fn native_stats(&self) -> impl Iterator<Item = &crate::Stats> {
        self.runs.iter().map(ThresholdRun::stats)
    }
    pub fn event_authority(&self) -> Option<&ThresholdRun> {
        let j = self.winner?;
        self.runs
            .get(if self.method == MultiGuardRoute::Independent {
                j
            } else {
                0
            })
    }
    pub fn pre_query(&self, t: f64) -> CoreResult<Query> {
        if !self.initial_negative || !t.is_finite() || t > self.safe {
            return Err(bad("outside ordered physical prefix"));
        }
        self.event_authority()
            .or_else(|| self.runs.first())
            .ok_or_else(|| bad("no physical ordered trajectory"))?
            .pre_query(t)
    }
}
struct SharedGuardSlab {
    power: Vec<Vec<Iv>>,
    cert: SlabCertificate,
}
struct MultiScan {
    scans: Vec<ThresholdScan>,
    incoming: Option<f64>,
    numerical_end: f64,
    stopped: bool,
    selected: Option<usize>,
    reason: Option<String>,
    policy: MultiGuardPolicy,
    history: Vec<SharedGuardSlab>,
    comparisons: Vec<GuardOrderComparison>,
    work: MultiGuardWork,
}
fn guard_scan(
    guard: &DeclaredPolynomial,
    epoch: EvolutionEpoch,
    policy: MultiGuardPolicy,
) -> CoreResult<ThresholdScan> {
    Ok(ThresholdScan {
        pending: Some(PendingState::new(policy.pending)?),
        threshold: 0.,
        guard: Some(DeclaredGuard {
            source: guard.clone(),
            derivative: policy.derivative,
            value: policy.value,
        }),
        guard_slabs: vec![],
        tolerance: policy.tolerance,
        route: policy.root_route,
        incoming: epoch.incoming_x_error,
        safe: epoch.start,
        numerical_end: epoch.start,
        root: None,
        stopped: false,
        reason: None,
        work: ThresholdWork::default(),
    })
}
fn initial_proved(s: &ThresholdScan) -> bool {
    s.pending
        .as_ref()
        .is_some_and(|p| !p.report.blocked_initial)
}
fn root_valid(s: &ThresholdScan) -> Option<[f64; 2]> {
    s.root.filter(|_| s.reason.is_none())
}
fn scan_receipt(s: &ThresholdScan, index: usize) -> GuardOrderEvidence {
    GuardOrderEvidence {
        index,
        root: root_valid(s),
        safe: s.safe,
        initial_negative: initial_proved(s),
        reason: s.reason.clone(),
        work: s.work.clone(),
        slabs: s.guard_slabs.clone(),
        pending: s.pending.as_ref().map(|p| p.report.clone()),
    }
}
fn run_receipt(r: &ThresholdRun, index: usize, start: f64) -> GuardOrderEvidence {
    let qualified =
        r.trial.global_qualified && r.trial.pointwise_qualified && r.trial.branch_qualified;
    GuardOrderEvidence {
        index,
        root: r.root.filter(|_| qualified && r.reason.is_none()),
        safe: if qualified { r.committed_until } else { start },
        initial_negative: qualified
            && r.pending_report
                .as_ref()
                .is_some_and(|p| !p.blocked_initial),
        reason: r
            .reason
            .clone()
            .or_else(|| (!qualified).then_some("unqualified physical replay".into())),
        work: r.work.clone(),
        slabs: r.guard_slabs.clone(),
        pending: r.pending_report.clone(),
    }
}
fn disjoint_or_absent(candidate: [f64; 2], other: &GuardOrderEvidence) -> bool {
    other.initial_negative
        && match other.root {
            Some(r) => candidate[1] < r[0],
            None => candidate[1] <= other.safe,
        }
}
fn independent_winner(gs: &[GuardOrderEvidence]) -> Option<usize> {
    if !gs.iter().all(|g| g.initial_negative) {
        return None;
    }
    let mut found = None;
    for g in gs {
        if let Some(root) = g.root {
            if gs
                .iter()
                .all(|h| g.index == h.index || disjoint_or_absent(root, h))
            {
                if found.is_some() {
                    return None;
                }
                found = Some(g.index);
            }
        }
    }
    found
}
/// Exact opposite coefficient cancellation for syntactically identical
/// affine-factor lists. No rounded coefficient addition or factor expansion.
fn guard_difference(
    a: &DeclaredPolynomial,
    b: &DeclaredPolynomial,
) -> CoreResult<DeclaredPolynomial> {
    use rodas5p_integrators::retro_event_poly::EventTerm;
    fn same(a: &EventTerm, b: &EventTerm) -> bool {
        a.factors.len() == b.factors.len()
            && a.factors.iter().zip(&b.factors).all(|(x, y)| {
                x.constant.to_bits() == y.constant.to_bits()
                    && x.time.to_bits() == y.time.to_bits()
                    && x.state.len() == y.state.len()
                    && x.state
                        .iter()
                        .zip(&y.state)
                        .all(|(&(i, a), &(j, b))| i == j && a.to_bits() == b.to_bits())
            })
    }
    let mut terms = a.terms().to_vec();
    for t in b.terms() {
        if let Some(i) = terms
            .iter()
            .position(|x| x.coefficient.to_bits() == t.coefficient.to_bits() && same(x, t))
        {
            terms.remove(i);
        } else {
            let mut q = t.clone();
            q.coefficient = -q.coefficient;
            terms.push(q);
        }
    }
    if terms.is_empty() {
        terms.push(EventTerm {
            coefficient: 0.,
            factors: vec![],
        });
    }
    DeclaredPolynomial::new(terms)
}
impl MultiScan {
    fn compare(&mut self, e: &Engine, j: usize, k: usize, root: [f64; 2]) -> bool {
        if let Some(v) = self
            .comparisons
            .iter()
            .find(|v| v.earlier == j && v.later == k)
        {
            return v.positive;
        }
        self.work.comparison_attempts += 1;
        let mut out = GuardOrderComparison {
            earlier: j,
            later: k,
            compared_root: root,
            cells: vec![],
            positive: false,
            reason: None,
        };
        let mut kernel = de::ResetKernelAudit::default();
        let result = (|| -> CoreResult<bool> {
            let a = &self.scans[j].guard.as_ref().expect("declared guard").source;
            let b = &self.scans[k].guard.as_ref().expect("declared guard").source;
            let d = guard_difference(a, b)?;
            let mut covered = root[0];
            for slab in &self.history {
                let lo = slab.cert.times[0].max(root[0]);
                let hi = slab.cert.times[1].min(root[1]);
                if lo > hi {
                    continue;
                }
                if lo > covered {
                    return Err(bad("guard difference coverage gap"));
                }
                let v = de::vector_reset_component_recorded(
                    &e.src,
                    &d,
                    &slab.power,
                    &slab.cert,
                    [lo, hi],
                    de::VectorSyncRoute::SchurPicard,
                    self.policy.derivative,
                    &mut kernel,
                )?;
                self.work.difference_cells += 1;
                self.work.difference_prepared_rhs += v.prepared_work.rhs_count;
                out.cells.push(GuardDifferenceCell {
                    times: [lo, hi],
                    enclosure: v.enclosure,
                    prepared_work: v.prepared_work,
                });
                if v.enclosure[0] <= 0. {
                    return Ok(false);
                }
                covered = covered.max(hi);
            }
            Ok(!out.cells.is_empty() && covered == root[1])
        })();
        update_audit(&e.audit, |a| {
            a.difference_kernel.components.merge(kernel.components);
            a.difference_kernel
                .component_guards
                .merge(&kernel.component_guards);
            a.difference_comparisons.record(result.is_ok());
            a.positive_differences += usize::from(result.as_ref().is_ok_and(|&v| v));
        });
        match result {
            Ok(v) => out.positive = v,
            Err(e) => out.reason = Some(e.to_string()),
        }
        let yes = out.positive;
        self.comparisons.push(out);
        yes
    }
    fn choose(&mut self, e: &Engine) -> Option<usize> {
        // A failure to certify one initial guard blocks the transaction.
        if !self.scans.iter().all(initial_proved) {
            return None;
        }
        let mut found = None;
        for j in 0..self.scans.len() {
            let Some(root) = root_valid(&self.scans[j]) else {
                continue;
            };
            let mut ok = true;
            for k in 0..self.scans.len() {
                if k == j {
                    continue;
                }
                let other = root_valid(&self.scans[k]);
                let strict = match other {
                    Some(r) => root[1] < r[0],
                    None => root[1] <= self.scans[k].safe,
                };
                if strict {
                    continue;
                }
                // At competitor's root, H_k=0. If H_j>0 there,
                // assuming tau_k<=tau_j contradicts H_j<0 before
                // its unique first root (or H_j=0 at equality).
                if self.policy.method == MultiGuardRoute::Correlated
                    && other.is_some_and(|b| b[1] >= root[0] && self.compare(e, j, k, b))
                {
                    continue;
                }
                ok = false;
                break;
            }
            if ok {
                if found.is_some() {
                    return None;
                }
                found = Some(j);
            }
        }
        found
    }
}
impl EpochObserver for MultiScan {
    fn stopped_at(&self) -> Option<f64> {
        self.stopped.then_some(self.numerical_end)
    }
    fn inspect(
        &mut self,
        e: &Engine,
        proposal: crate::solve::DenseProposal<'_>,
    ) -> rodas5p_integrators::AcceptControl {
        use rodas5p_integrators::AcceptControl;
        let result = (|| -> CoreResult<bool> {
            let (times, native) = proposal_packet(proposal)?;
            self.numerical_end = times[1];
            if self.work.physical_certificates >= self.policy.max_slabs {
                return Err(bad("multi-guard slab budget exhausted"));
            }
            let (p, c) = certify_native_packet(
                e,
                times,
                &native,
                self.incoming,
                &mut self.work.shared_projection,
            )?;
            self.work.physical_certificates += 1;
            for scan in &mut self.scans {
                if scan.stopped {
                    continue;
                }
                scan.work.inspected += 1;
                scan.numerical_end = times[1];
                if scan.incoming != self.incoming {
                    return Err(bad("guard shared radius mismatch"));
                }
                match scan.scan_certified(e, times, &p, &c) {
                    Ok(stop) => scan.stopped = stop,
                    Err(e) => {
                        scan.reason = Some(e.to_string());
                        scan.stopped = true;
                    }
                }
            }
            self.incoming = c.outgoing;
            if self.policy.method == MultiGuardRoute::Correlated {
                self.history.push(SharedGuardSlab { power: p, cert: c });
            }
            self.selected = self.choose(e);
            if self.selected.is_some() {
                return Ok(true);
            }
            if self.scans.iter().all(|s| s.stopped) {
                return Err(bad("first-event order unresolved"));
            }
            // No need for an additional accepted proposal after exhausting
            // the physical certificate budget.
            if self.work.physical_certificates >= self.policy.max_slabs {
                return Err(bad("multi-guard slab budget exhausted"));
            }
            Ok(false)
        })();
        match result {
            Ok(false) => AcceptControl::Continue,
            Ok(true) => {
                self.stopped = true;
                AcceptControl::Stop
            }
            Err(e) => {
                self.reason = Some(e.to_string());
                self.stopped = true;
                AcceptControl::Stop
            }
        }
    }
}
/// Competing guards all belong to the same original physical source,
/// initial family and branch; no index-order tie-breaking.
#[allow(clippy::too_many_arguments)]
pub fn evolve_to_first_guard(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    guards: &[DeclaredPolynomial],
    mp: MultiGuardPolicy,
) -> CoreResult<MultiGuardRun> {
    evolve_to_first_guard_authority(
        src, x, z, branch, epoch, opts, policy, route, guards, mp, None, None,
    )
}
#[allow(clippy::too_many_arguments)]
fn evolve_to_first_guard_authority(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    guards: &[DeclaredPolynomial],
    mp: MultiGuardPolicy,
    conservation: Option<(de::RobertsonModel, de::ConservationRoute)>,
    audit: Option<AuditHandle>,
) -> CoreResult<MultiGuardRun> {
    if conservation.is_some() && mp.method == MultiGuardRoute::Independent {
        return Err(bad("conservation shared observer required"));
    }
    if !(2..=8).contains(&guards.len())
        || !(1..=128).contains(&mp.max_slabs)
        || !(1..=64).contains(&mp.pending.max_slabs)
        || !mp.tolerance.is_finite()
        || mp.tolerance <= 0.
        || epoch
            .incoming_x_error
            .is_none_or(|v| !v.is_finite() || v < 0.)
    {
        return Err(bad("multi-guard policy/initial authority"));
    }
    for g in guards {
        g.validate(src.nx() + src.nz())?;
    }
    if mp.method == MultiGuardRoute::Independent {
        let mut out = MultiGuardRun {
            method: mp.method,
            guards: vec![],
            comparisons: vec![],
            winner: None,
            safe: epoch.start,
            initial_negative: false,
            reason: None,
            runs: vec![],
            work: MultiGuardWork::default(),
        };
        for (i, g) in guards.iter().enumerate() {
            let remaining = opts.max_steps.saturating_sub(out.work.attempts);
            if remaining == 0 {
                out.reason = Some("shared native attempt budget exhausted".into());
                break;
            }
            let mut o = opts.clone();
            o.max_steps = remaining.min(mp.max_slabs);
            match evolve_to_guard_pending(
                src,
                x,
                z,
                branch,
                epoch,
                &o,
                policy,
                route,
                g,
                mp.tolerance,
                mp.root_route,
                mp.derivative,
                mp.value,
                mp.pending,
            ) {
                Ok(r) => {
                    let receipt = run_receipt(&r, i, epoch.start);
                    out.work.driver(&r.trial);
                    out.work.guard(&receipt, true);
                    out.guards.push(receipt);
                    out.runs.push(r);
                }
                Err(e) => {
                    out.reason = Some(e.to_string());
                    break;
                }
            }
        }
        if out.guards.len() == guards.len() {
            out.initial_negative = out.guards.iter().all(|r| r.initial_negative);
            if out.initial_negative {
                out.winner = independent_winner(&out.guards);
                out.safe = if let Some(i) = out.winner {
                    out.guards[i].root.expect("winner")[0]
                } else {
                    out.guards.iter().map(|g| g.safe).fold(epoch.end, f64::min)
                };
            }
            if out.winner.is_none() && (!out.initial_negative || out.safe < epoch.end) {
                out.reason = Some("first-event order unresolved".into());
            }
        } else if out.reason.is_none() {
            out.reason = Some("incomplete guard receipts".into());
        }
        return Ok(out);
    }
    let scans = guards
        .iter()
        .map(|g| guard_scan(g, epoch, mp))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut multi = MultiScan {
        scans,
        incoming: epoch.incoming_x_error,
        numerical_end: epoch.start,
        stopped: false,
        selected: None,
        reason: None,
        policy: mp,
        history: Vec::with_capacity(if mp.method == MultiGuardRoute::Correlated {
            mp.max_slabs
        } else {
            0
        }),
        comparisons: vec![],
        work: MultiGuardWork::default(),
    };
    let trial = evolve_epoch_authority(
        src,
        x,
        z,
        branch,
        epoch,
        opts,
        policy,
        route,
        &[],
        Some(&mut multi),
        conservation,
        audit,
    )?;
    let qualified = trial.global_qualified && trial.pointwise_qualified && trial.branch_qualified;
    let initial = qualified && multi.scans.iter().all(initial_proved);
    if !qualified {
        multi.selected = None;
        multi.reason = Some("shared replay physical proof failed".into());
    }
    if !initial {
        multi.selected = None;
    }
    let receipts = multi
        .scans
        .iter()
        .enumerate()
        .map(|(i, s)| scan_receipt(s, i))
        .collect::<Vec<_>>();
    let safe = if !initial {
        epoch.start
    } else if let Some(j) = multi.selected {
        receipts[j].root.expect("winner")[0]
    } else {
        receipts.iter().map(|r| r.safe).fold(epoch.end, f64::min)
    };
    if multi.selected.is_none() && (!initial || safe < epoch.end) && multi.reason.is_none() {
        multi.reason = Some("first-event order unresolved at horizon".into());
    }
    multi.work.driver(&trial);
    for g in &receipts {
        multi.work.guard(g, false);
    }
    let root = multi.selected.and_then(|j| receipts[j].root);
    let chosen = multi.selected.map(|i| &receipts[i]);
    let r = ThresholdRun {
        trial,
        root,
        committed_until: safe,
        numerical_end: multi.numerical_end,
        reason: multi.reason.clone(),
        work: chosen.map(|g| g.work.clone()).unwrap_or_default(),
        guard_slabs: chosen.map(|g| g.slabs.clone()).unwrap_or_default(),
        pending_report: chosen.and_then(|g| g.pending.clone()),
    };
    Ok(MultiGuardRun {
        method: mp.method,
        guards: receipts,
        comparisons: multi.comparisons,
        winner: multi.selected,
        safe,
        initial_negative: initial,
        reason: multi.reason,
        runs: vec![r],
        work: multi.work,
    })
}
/// One declared transition per input guard, in the original input order.
/// Only the uniquely certified earliest guard's transition is evaluated.
pub struct OrderedVectorTransition<'a> {
    pub reset: &'a [DeclaredPolynomial],
    pub source: &'a DaeSource,
    pub z_seed: &'a [f64],
    pub branch: &'a [[f64; 2]],
    pub route: EvolutionRoute,
    pub policy: VectorResetPolicy,
}
pub struct OrderedVectorRestart {
    pub event_index: usize,
    pub run: VectorRestartRun,
}
impl MultiGuardRun {
    pub fn restart_ordered_vector(
        &self,
        transitions: &[OrderedVectorTransition<'_>],
        end: f64,
        opts: &Options,
        policy: EvolutionPolicy,
        grid: &[f64],
    ) -> CoreResult<OrderedVectorRestart> {
        if transitions.len() != self.guards.len() {
            return Err(bad("ordered transition shape"));
        }
        let j = self
            .winner
            .ok_or_else(|| bad("no unique first-event authority"))?;
        let a = self
            .event_authority()
            .ok_or_else(|| bad("missing first-event run"))?;
        let tr = &transitions[j];
        let run = a.restart_vector(
            tr.reset, tr.source, tr.z_seed, tr.branch, end, opts, policy, tr.route, tr.policy, grid,
        )?;
        Ok(OrderedVectorRestart {
            event_index: j,
            run,
        })
    }
}

pub use de::{GapGuardEvidence, GapGuardRoute};
/// Immutable declaration during the chain. Each edge matches the guard at
/// the same index. Terminal modes have no outgoing guards or transitions.
pub struct ChainMode {
    pub source: DaeSource,
    pub z_seed: Vec<f64>,
    pub branch: Vec<[f64; 2]>,
    pub route: EvolutionRoute,
    pub guards: Vec<DeclaredPolynomial>,
    pub transitions: Vec<ChainTransition>,
}
pub struct ChainTransition {
    pub target: usize,
    pub reset: Vec<DeclaredPolynomial>,
    pub policy: VectorResetPolicy,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ChainBudget {
    pub native_attempts: usize,
    pub scan_slabs: usize,
    pub transition_attempts: usize,
    pub reset_cells: usize,
    pub gap_checks: usize,
    pub min_dwell: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ChainStop {
    Horizon,
    OrderUnresolved,
    NativeBudget,
    SlabBudget,
    TransitionBudget,
    ResetBudget,
    GapBudget,
    DwellUnproved,
    ResetRefused,
    GapUnproved,
    NativeRefused,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ChainWork {
    pub native_attempts: usize,
    pub charged_native_attempts: usize,
    pub scan_slabs: usize,
    pub charged_scan_slabs: usize,
    pub native_runs: usize,
    pub driver_rhs: u64,
    pub projection_rhs: usize,
    pub guard_evals: usize,
    pub dictionary_uses: usize,
    pub comparison_attempts: usize,
    pub transitions_attempted: usize,
    pub transitions_committed: usize,
    pub reset_cells: usize,
    pub charged_reset_cells: usize,
    pub gap_checks: usize,
    pub gap_rhs: usize,
    pub picard_proofs: usize,
    pub reset_rhs: usize,
    pub green_calls: usize,
    pub warp_predictions: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct ChainGapCheck {
    pub guard: usize,
    pub cell: Option<usize>,
    pub times: [f64; 2],
    pub enclosure: [f64; 2],
    pub evidence: GapGuardEvidence,
}
#[derive(Serialize)]
pub struct ChainEvent {
    pub from: usize,
    pub target: usize,
    pub guard: usize,
    pub event_time: [f64; 2],
    pub dwell_lower: Option<f64>,
    pub committed: bool,
    pub reset: Option<VectorRestartRun>,
    pub gap: Vec<ChainGapCheck>,
    pub reason: Option<String>,
}
pub struct ChainEpoch {
    pub mode: usize,
    pub start: f64,
    pub safe: f64,
    pub incoming_radius: f64,
    ordered: Option<MultiGuardRun>,
    terminal: Option<EvolutionRun>,
}
impl ChainEpoch {
    pub fn ordered(&self) -> Option<&MultiGuardRun> {
        self.ordered.as_ref()
    }
    pub fn terminal(&self) -> Option<&EvolutionRun> {
        self.terminal.as_ref().filter(|r| {
            r.global_qualified
                && r.branch_qualified
                && r.pointwise_qualified
                && r.driver.t.last() == Some(&self.safe)
        })
    }
    pub fn terminal_diagnostic(&self) -> Option<&EvolutionRun> {
        self.terminal.as_ref()
    }
}
pub struct HybridChainRun {
    epochs: Vec<ChainEpoch>,
    transitions: Vec<ChainEvent>,
    final_mode: usize,
    stop: ChainStop,
    reason: Option<String>,
    work: ChainWork,
}
impl HybridChainRun {
    pub fn epochs(&self) -> &[ChainEpoch] {
        &self.epochs
    }
    pub fn transitions(&self) -> &[ChainEvent] {
        &self.transitions
    }
    pub fn final_mode(&self) -> usize {
        self.final_mode
    }
    pub fn stop(&self) -> ChainStop {
        self.stop
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn work(&self) -> &ChainWork {
        &self.work
    }
    /// Epoch-indexed one-sided query. No authority in asynchronous gaps.
    pub fn query(&self, epoch: usize, t: f64) -> CoreResult<Query> {
        let e = self
            .epochs
            .get(epoch)
            .ok_or_else(|| bad("unknown chain epoch"))?;
        if !t.is_finite() || t < e.start || t > e.safe {
            return Err(bad("outside committed chain epoch"));
        }
        if let Some(r) = &e.ordered {
            return r.pre_query(t);
        }
        let r = e
            .terminal
            .as_ref()
            .ok_or_else(|| bad("no committed terminal trajectory"))?;
        if !r.global_qualified || !r.branch_qualified || !r.pointwise_qualified {
            return Err(bad("unqualified terminal epoch"));
        }
        let q = r.query(t)?;
        if q.physical_error_upper
            .is_none_or(|x| x > r.engine.policy.global_tol)
        {
            return Err(bad("unqualified chain query"));
        }
        Ok(q)
    }
}
fn add_reset_work(w: &mut ChainWork, r: &VectorRestartRun) {
    w.reset_cells += r.cells.len();
    w.charged_reset_cells += r.cells.len();
    for c in &r.cells {
        w.reset_rhs += c
            .reset
            .iter()
            .map(|r| r.prepared_work.rhs_count)
            .sum::<usize>();
        if let Some(t) = &c.tube {
            w.picard_proofs += t.attempts.len();
            w.reset_rhs += t
                .attempts
                .iter()
                .map(|p| p.work.rhs_count + p.initial.work.rhs_count)
                .sum::<usize>();
        }
        if let Some(s) = &c.sync {
            w.green_calls += s.green_calls;
        }
    }
    if let Some(p) = &r.restart_branch {
        w.reset_rhs += p.work.rhs_count + p.initial.work.rhs_count;
    }
}
/// One bounded chain of certified single transitions. Shared/correlated
/// scan only; every mode transition is checked on the complete new-mode
/// synchronization gap before it can be committed.
#[allow(clippy::too_many_arguments)]
pub fn evolve_hybrid_chain(
    modes: &[ChainMode],
    initial_mode: usize,
    x: &[f64],
    z: &[f64],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: ChainBudget,
    gap_route: GapGuardRoute,
) -> CoreResult<HybridChainRun> {
    if modes.is_empty()
        || modes.len() > 64
        || initial_mode >= modes.len()
        || mp.method == MultiGuardRoute::Independent
        || budget.native_attempts == 0
        || budget.scan_slabs == 0
        || !budget.min_dwell.is_finite()
        || budget.min_dwell <= 0.
        || budget.transition_attempts > 64
        || !epoch.start.is_finite()
        || !epoch.end.is_finite()
        || epoch.end <= epoch.start
        || epoch
            .incoming_x_error
            .is_none_or(|x| !x.is_finite() || x < 0.)
    {
        return Err(bad("chain policy/input contract"));
    }
    for m in modes {
        if m.guards.len() != m.transitions.len()
            || (!m.guards.is_empty() && !(2..=8).contains(&m.guards.len()))
        {
            return Err(bad("chain mode guard/edge shape"));
        }
        for (g, tr) in m.guards.iter().zip(&m.transitions) {
            g.validate(m.source.nx() + m.source.nz())?;
            if tr.target >= modes.len() || tr.reset.len() != modes[tr.target].source.nx() {
                return Err(bad("chain edge target/reset shape"));
            }
            for r in &tr.reset {
                r.validate(m.source.nx() + m.source.nz())?;
            }
        }
    }
    let mut out = HybridChainRun {
        epochs: vec![],
        transitions: vec![],
        final_mode: initial_mode,
        stop: ChainStop::Horizon,
        reason: None,
        work: ChainWork::default(),
    };
    let mut mode = initial_mode;
    let mut clock = epoch.start;
    let mut xx = x.to_vec();
    let mut zz = z.to_vec();
    let mut radius = epoch.incoming_x_error.expect("validated");
    let mut prior: Option<[f64; 2]> = None;
    let total_native = budget.native_attempts.min(opts.max_steps);
    macro_rules! stop {
        ($kind:expr,$msg:expr) => {{
            out.stop = $kind;
            out.reason = Some($msg.to_string());
            return Ok(out);
        }};
    }
    loop {
        let remaining = total_native.saturating_sub(out.work.charged_native_attempts);
        if remaining == 0 {
            stop!(ChainStop::NativeBudget, "chain native attempt budget");
        }
        let slabs = budget
            .scan_slabs
            .saturating_sub(out.work.charged_scan_slabs);
        if slabs == 0 {
            stop!(ChainStop::SlabBudget, "chain slab budget");
        }
        let m = &modes[mode];
        let mut o = opts.clone();
        o.max_steps = remaining;
        let ep = EvolutionEpoch {
            start: clock,
            end: epoch.end,
            incoming_x_error: Some(radius),
        };
        if m.guards.is_empty() {
            o.max_steps = o.max_steps.min(slabs);
            let r = match evolve_epoch(&m.source, &xx, &zz, &m.branch, ep, &o, policy, m.route, &[])
            {
                Ok(v) => v,
                Err(e) => {
                    out.work.charged_native_attempts += o.max_steps;
                    out.work.charged_scan_slabs += o.max_steps;
                    stop!(ChainStop::NativeRefused, e);
                }
            };
            let n = r.driver.stats.attempts;
            let c = r.slabs.len();
            out.work.native_runs += 1;
            out.work.native_attempts += n;
            out.work.charged_native_attempts += n;
            out.work.scan_slabs += c;
            out.work.charged_scan_slabs += r.driver.stats.accepted_steps;
            out.work.driver_rhs += r.driver.stats.counters.rhs_calls;
            out.work.projection_rhs += r.work.rhs_count;
            out.work.warp_predictions += r.work.warp_predictions;
            let good = r.global_qualified
                && r.branch_qualified
                && r.pointwise_qualified
                && r.driver.t.last() == Some(&epoch.end);
            out.epochs.push(ChainEpoch {
                mode,
                start: clock,
                safe: if good { epoch.end } else { clock },
                incoming_radius: radius,
                ordered: None,
                terminal: Some(r),
            });
            if !good {
                let k = if out.work.charged_native_attempts >= total_native {
                    ChainStop::NativeBudget
                } else if out.work.charged_scan_slabs >= budget.scan_slabs || n >= slabs {
                    ChainStop::SlabBudget
                } else {
                    ChainStop::NativeRefused
                };
                stop!(k, "terminal physical evolution unqualified");
            }
            return Ok(out);
        }
        let mut q = mp;
        q.max_slabs = q.max_slabs.min(slabs);
        let r = match evolve_to_first_guard(
            &m.source, &xx, &zz, &m.branch, ep, &o, policy, m.route, &m.guards, q,
        ) {
            Ok(v) => v,
            Err(e) => {
                out.work.charged_native_attempts += remaining;
                out.work.charged_scan_slabs += q.max_slabs;
                stop!(ChainStop::NativeRefused, e);
            }
        };
        let w = &r.work;
        out.work.native_runs += w.native_runs;
        out.work.native_attempts += w.attempts;
        out.work.charged_native_attempts += w.attempts;
        out.work.scan_slabs += w.physical_certificates;
        out.work.charged_scan_slabs += r
            .runs
            .first()
            .map_or(w.physical_certificates, |v| v.stats().accepted_steps);
        out.work.driver_rhs += w.driver_rhs;
        out.work.projection_rhs += w.shared_projection.rhs_count
            + w.replay_projection_rhs
            + w.guard_prepared_rhs
            + w.difference_prepared_rhs;
        out.work.guard_evals += w.guard_evals;
        out.work.dictionary_uses += w.dictionary_uses;
        out.work.comparison_attempts += w.comparison_attempts;
        out.work.warp_predictions += w.shared_projection.warp_predictions
            + r.runs.first().map_or(0, |v| v.trial.work.warp_predictions);
        let winner = r.winner();
        let safe = r.committed_until();
        let reason = r.reason().map(str::to_owned);
        out.epochs.push(ChainEpoch {
            mode,
            start: clock,
            safe,
            incoming_radius: radius,
            ordered: Some(r),
            terminal: None,
        });
        let Some(j) = winner else {
            if reason.is_none() && safe == epoch.end {
                return Ok(out);
            }
            let kind = if out.work.charged_native_attempts >= total_native {
                ChainStop::NativeBudget
            } else if out.work.charged_scan_slabs >= budget.scan_slabs {
                ChainStop::SlabBudget
            } else {
                ChainStop::OrderUnresolved
            };
            stop!(
                kind,
                reason.unwrap_or_else(|| "chain order unresolved".into())
            );
        };
        let authority = out
            .epochs
            .last()
            .unwrap()
            .ordered
            .as_ref()
            .unwrap()
            .event_authority()
            .expect("ordered winner");
        let root = authority.root().expect("ordered root");
        let edge = &m.transitions[j];
        let target = &modes[edge.target];
        let dwell = prior
            .map(|p| rodas5p_core::directed::sub_down(root[0], p[1]))
            .transpose()?;
        let mut ev = ChainEvent {
            from: mode,
            target: edge.target,
            guard: j,
            event_time: root,
            dwell_lower: dwell,
            committed: false,
            reset: None,
            gap: vec![],
            reason: None,
        };
        let kind = if dwell.is_some_and(|v| v < budget.min_dwell) {
            Some(ChainStop::DwellUnproved)
        } else if out.work.transitions_attempted >= budget.transition_attempts {
            Some(ChainStop::TransitionBudget)
        } else if out.work.charged_reset_cells >= budget.reset_cells {
            Some(ChainStop::ResetBudget)
        } else if root[1] >= epoch.end {
            Some(ChainStop::ResetRefused)
        } else {
            None
        };
        if let Some(k) = kind {
            ev.reason = Some(format!("{k:?}"));
            out.transitions.push(ev);
            stop!(k, format!("{k:?}"));
        }
        out.work.transitions_attempted += 1;
        let cap = (budget.reset_cells - out.work.charged_reset_cells).min(64);
        let prep = match authority.prepare_vector_restart(
            &edge.reset,
            &target.source,
            &target.z_seed,
            &target.branch,
            epoch.end,
            policy,
            edge.policy,
            cap,
        ) {
            Ok(v) => v,
            Err(e) => {
                out.work.charged_reset_cells += cap;
                ev.reason = Some(e.to_string());
                out.transitions.push(ev);
                stop!(ChainStop::ResetRefused, e);
            }
        };
        add_reset_work(&mut out.work, &prep);
        if let Some(e) = prep.reason.clone() {
            let k = if e.contains("cell cap") {
                ChainStop::ResetBudget
            } else {
                ChainStop::ResetRefused
            };
            ev.reason = Some(e.clone());
            ev.reset = Some(prep);
            out.transitions.push(ev);
            stop!(k, e);
        }
        let check = (|| -> Result<(), (ChainStop, String)> {
            for (ci, bp) in prep
                .cells
                .iter()
                .map(|c| {
                    c.tube
                        .as_ref()
                        .expect("prepared tube")
                        .attempts
                        .last()
                        .expect("tube proof")
                })
                .enumerate()
                .map(|(i, b)| (Some(i), b))
                .chain(std::iter::once((
                    None,
                    prep.restart_branch.as_ref().expect("prepared restart"),
                )))
            {
                for (k, g) in target.guards.iter().enumerate() {
                    if out.work.gap_checks >= budget.gap_checks {
                        return Err((ChainStop::GapBudget, "chain gap check budget".into()));
                    }
                    out.work.gap_checks += 1;
                    let e = de::gap_guard_enclosure(
                        &target.source,
                        g,
                        bp,
                        gap_route,
                        edge.policy.branch_route,
                    )
                    .map_err(|e| (ChainStop::GapUnproved, e.to_string()))?;
                    out.work.gap_rhs += e.work.rhs_count;
                    let negative = e.enclosure[1] < 0.;
                    ev.gap.push(ChainGapCheck {
                        guard: k,
                        cell: ci,
                        times: bp.times,
                        enclosure: e.enclosure,
                        evidence: e,
                    });
                    if !negative {
                        return Err((
                            ChainStop::GapUnproved,
                            "new-mode guard not strictly negative over synchronization tube".into(),
                        ));
                    }
                }
            }
            Ok(())
        })();
        if let Err((k, e)) = check {
            ev.reason = Some(e.clone());
            ev.reset = Some(prep);
            out.transitions.push(ev);
            stop!(k, e);
        }
        let bp = prep.restart_branch.as_ref().expect("prepared restart");
        xx = bp.initial.x.clone();
        zz = bp.initial.z.clone();
        radius = prep.incoming_radius.expect("prepared radius");
        clock = root[1];
        prior = Some(root);
        mode = edge.target;
        ev.committed = true;
        ev.reset = Some(prep);
        out.transitions.push(ev);
        out.work.transitions_committed += 1;
        out.final_mode = mode;
    }
}

pub use de::{ConservationReceipt, ConservationRoute, RobertsonModel};
/// Initial physical family is the intersection of the exact conserved
/// simplex and the explicitly L1-bounded differential ball. The public
/// constructor checks a physical witness (its center). No generic epoch
/// infinity radius is accepted as an invariant-set assertion.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct RobertsonInitial {
    center: [f64; 2],
    l1_radius: f64,
}
impl RobertsonInitial {
    pub fn exact(center: [f64; 2]) -> CoreResult<Self> {
        Self::simplex_ball(center, 0.)
    }
    pub fn simplex_ball(center: [f64; 2], l1_radius: f64) -> CoreResult<Self> {
        if !l1_radius.is_finite()
            || l1_radius < 0.
            || center.iter().any(|x| !x.is_finite() || *x < 0.)
            || add_up(center[0], center[1])? > 1.
        {
            return Err(bad("Robertson initial simplex authority"));
        }
        Ok(Self { center, l1_radius })
    }
    pub fn center(&self) -> [f64; 2] {
        self.center
    }
    pub fn l1_radius(&self) -> f64 {
        self.l1_radius
    }
}
const ROBERTSON_BRANCH: [[f64; 2]; 1] = [[-0.25, 1.25]];
fn robertson_policy(policy: EvolutionPolicy) -> CoreResult<()> {
    if policy.init.mode != rodas5p_integrators::retro_dae_init::InitMode::ProjectAlgebraic {
        return Err(bad("Robertson requires original algebraic projection"));
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn robertson_epoch(
    model: &RobertsonModel,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    grid: &[f64],
) -> CoreResult<EvolutionRun> {
    robertson_epoch_recorded(
        model,
        initial,
        times,
        opts,
        policy,
        route,
        certificate,
        grid,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn robertson_epoch_recorded(
    model: &RobertsonModel,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    grid: &[f64],
    audit: Option<AuditHandle>,
) -> CoreResult<EvolutionRun> {
    robertson_policy(policy)?;
    let epoch = EvolutionEpoch {
        start: times[0],
        end: times[1],
        incoming_x_error: Some(initial.l1_radius),
    };
    let r = evolve_epoch_authority(
        model.source(),
        &initial.center,
        &[0.125],
        &ROBERTSON_BRANCH,
        epoch,
        opts,
        policy,
        route,
        grid,
        None,
        Some((model.clone(), certificate)),
        audit,
    )?;
    if r.initial.x != initial.center || r.driver.y.first().is_none_or(|v| v[..2] != initial.center)
    {
        return Err(bad("Robertson differential center changed"));
    }
    Ok(r)
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson(
    rate: f64,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    grid: &[f64],
) -> CoreResult<EvolutionRun> {
    robertson_epoch(
        &RobertsonModel::new(rate)?,
        initial,
        times,
        opts,
        policy,
        route,
        certificate,
        grid,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson_to_first_guard(
    rate: f64,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    guards: &[DeclaredPolynomial],
    mp: MultiGuardPolicy,
) -> CoreResult<MultiGuardRun> {
    robertson_first_guard_recorded(
        rate,
        initial,
        times,
        opts,
        policy,
        route,
        certificate,
        guards,
        mp,
        None,
    )
}
/// Native shared/correlated guard call and diagnostic resource snapshot.
/// No event authority is granted by the resource receipt.
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson_to_first_guard_audited(
    rate: f64,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    guards: &[DeclaredPolynomial],
    mp: MultiGuardPolicy,
) -> (CoreResult<MultiGuardRun>, NativeResourceAudit) {
    let a = Arc::new(Mutex::new(NativeResourceAudit::default()));
    let r = robertson_first_guard_recorded(
        rate,
        initial,
        times,
        opts,
        policy,
        route,
        certificate,
        guards,
        mp,
        Some(a.clone()),
    );
    let receipt = a.lock().expect("native audit lock").clone();
    (r, receipt)
}
#[allow(clippy::too_many_arguments)]
fn robertson_first_guard_recorded(
    rate: f64,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    guards: &[DeclaredPolynomial],
    mp: MultiGuardPolicy,
    audit: Option<AuditHandle>,
) -> CoreResult<MultiGuardRun> {
    robertson_policy(policy)?;
    let model = RobertsonModel::new(rate)?;
    evolve_to_first_guard_authority(
        model.source(),
        &initial.center,
        &[0.125],
        &ROBERTSON_BRANCH,
        EvolutionEpoch {
            start: times[0],
            end: times[1],
            incoming_x_error: Some(initial.l1_radius),
        },
        opts,
        policy,
        route,
        guards,
        mp,
        Some((model.clone(), certificate)),
        audit,
    )
}
/// Exact clock declarations and canonical positive mass-preserving reset.
/// General asynchronous state-dependent reset is deliberately not admitted.
pub struct RobertsonClockRun {
    pre: MultiGuardRun,
    pub event_time: Option<f64>,
    pub incoming_l1: Option<f64>,
    pub reset_center: Option<[f64; 2]>,
    pub reason: Option<String>,
    pub native_attempts: usize,
    post: Option<EvolutionRun>,
}
impl RobertsonClockRun {
    pub fn pre(&self) -> &MultiGuardRun {
        &self.pre
    }
    pub fn post(&self) -> Option<&EvolutionRun> {
        if self.reason.is_none() {
            self.post
                .as_ref()
                .filter(|r| r.global_qualified && r.branch_qualified && r.pointwise_qualified)
        } else {
            None
        }
    }
    pub fn post_diagnostic(&self) -> Option<&EvolutionRun> {
        self.post.as_ref()
    }
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson_clock_reset(
    rates: [f64; 2],
    initial: RobertsonInitial,
    times: [f64; 2],
    clocks: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    mp: MultiGuardPolicy,
    grid: &[f64],
) -> CoreResult<RobertsonClockRun> {
    use rodas5p_integrators::retro_event_poly::{AffineFactor, EventTerm};
    let target = RobertsonModel::new(rates[1])?;
    if clocks
        .iter()
        .any(|t| !t.is_finite() || *t <= times[0] || *t >= times[1])
    {
        return Err(bad("Robertson declared clock domain"));
    }
    let guards = clocks
        .iter()
        .map(|&t| {
            DeclaredPolynomial::new(vec![EventTerm {
                coefficient: 1.,
                factors: vec![AffineFactor::clock(t)],
            }])
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let pre = evolve_robertson_to_first_guard(
        rates[0],
        initial,
        times,
        opts,
        policy,
        route,
        certificate,
        &guards,
        mp,
    )?;
    let mut out = RobertsonClockRun {
        native_attempts: pre.work.attempts,
        pre,
        event_time: None,
        incoming_l1: None,
        reset_center: None,
        reason: None,
        post: None,
    };
    let Some(j) = out.pre.winner() else {
        out.reason = Some("Robertson first clock unresolved".into());
        return Ok(out);
    };
    let a = out.pre.event_authority().expect("winner authority");
    let tau = clocks[j];
    let root = a.root().expect("winner root");
    if !(root[0] <= tau && tau <= root[1] && tau < clocks[1 - j]) {
        return Err(bad("Robertson clock identity/order"));
    }
    // The declared guard is exactly t-tau, not an approximate root hint.
    // Thus evaluation at tau uses private original-mode extension, not
    // an extrapolation or uncertain-time synchronization.
    let q = a.trial.query(tau)?;
    let slab = a
        .trial
        .slabs
        .get(a.trial.slabs.partition_point(|s| s.times[1] < tau))
        .ok_or_else(|| bad("Robertson clock slab missing"))?;
    let c = slab
        .certificate
        .as_ref()
        .filter(|c| c.global_qualified && c.conservation.is_some())
        .ok_or_else(|| bad("Robertson clock L1 authority missing"))?;
    let p = slab.power[..2]
        .iter()
        .map(|v| {
            v.iter()
                .map(|x| Iv::new(x[0], x[1]))
                .collect::<CoreResult<Vec<_>>>()
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let old = de::conservation_query_l1(
        &p,
        slab.times,
        tau,
        &q.state[..2],
        c.whole_x_error.expect("qualified"),
    )?;
    let center = [q.state[0], q.state[1] * 0.5];
    let correction = Iv::point(q.state[1])?
        .scale(0.5)?
        .sub(Iv::point(center[1])?)?
        .mag();
    let radius = add_up(old, correction)?;
    // R(u,v,z)=(u,v/2,z+v/2) maps the invariant simplex into itself
    // and has reduced induced L1 norm 1. Only this private constructor
    // can carry that authority even if the numerical center is nonphysical.
    let start = RobertsonInitial {
        center,
        l1_radius: radius,
    };
    out.event_time = Some(tau);
    out.incoming_l1 = Some(radius);
    out.reset_center = Some(center);
    let mut o = opts.clone();
    o.max_steps = o.max_steps.saturating_sub(out.native_attempts);
    if o.max_steps == 0 {
        out.reason = Some("Robertson native attempt budget exhausted".into());
        return Ok(out);
    }
    match robertson_epoch(
        &target,
        start,
        [tau, times[1]],
        &o,
        policy,
        route,
        certificate,
        grid,
    ) {
        Ok(r) => {
            out.native_attempts += r.driver.stats.attempts;
            if !r.global_qualified || !r.branch_qualified || !r.pointwise_qualified {
                out.reason = Some("Robertson post physical proof unresolved".into());
            }
            out.post = Some(r);
        }
        Err(e) => {
            out.reason = Some(e.to_string());
        }
    }
    Ok(out)
}

pub use de::{SimplexReset, SimplexSyncRoute, SimplexTransportCell};
#[derive(Clone, Debug, Serialize)]
pub struct SimplexTransportReceipt {
    pub route: SimplexSyncRoute,
    pub operator_l1: f64,
    pub center: [f64; 2],
    pub box_l1: f64,
    pub component_l1: f64,
    pub joint_l1: Option<f64>,
    pub selected_l1: f64,
    pub cells: Vec<SimplexTransportCell>,
}
/// One genuinely asynchronous source-owned Robertson reset into a terminal
/// conserved mode. Never interprets a generic infinity ball as L1.
pub struct RobertsonAsyncRun {
    pre: MultiGuardRun,
    pub event_time: Option<[f64; 2]>,
    pub preparation: Option<VectorRestartRun>,
    pub transport: Option<SimplexTransportReceipt>,
    pub incoming_l1: Option<f64>,
    pub reason: Option<String>,
    pub native_attempts: usize,
    pub charged_native_attempts: usize,
    post: Option<EvolutionRun>,
}
impl RobertsonAsyncRun {
    pub fn pre(&self) -> &MultiGuardRun {
        &self.pre
    }
    /// Original-mode numerical extension receipts, for audit only.
    /// These slabs are not a physical trajectory after the first event.
    pub fn original_slab_diagnostics(&self) -> &[SlabRecord] {
        self.pre.runs.first().map_or(&[], |r| r.trial.slabs())
    }
    pub fn post(&self) -> Option<&EvolutionRun> {
        if self.reason.is_none() {
            self.post
                .as_ref()
                .filter(|r| r.global_qualified && r.branch_qualified && r.pointwise_qualified)
        } else {
            None
        }
    }
    pub fn post_diagnostic(&self) -> Option<&EvolutionRun> {
        self.post.as_ref()
    }
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson_async_reset(
    rates: [f64; 2],
    initial: RobertsonInitial,
    times: [f64; 2],
    guards: &[DeclaredPolynomial],
    reset: &SimplexReset,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    certificate: ConservationRoute,
    mp: MultiGuardPolicy,
    rp: VectorResetPolicy,
    sync: SimplexSyncRoute,
    grid: &[f64],
) -> CoreResult<RobertsonAsyncRun> {
    let target = RobertsonModel::new(rates[1])?;
    let pre = evolve_robertson_to_first_guard(
        rates[0],
        initial,
        times,
        opts,
        policy,
        route,
        certificate,
        guards,
        mp,
    )?;
    let mut out = RobertsonAsyncRun {
        native_attempts: pre.work.attempts,
        charged_native_attempts: pre.work.attempts,
        pre,
        event_time: None,
        preparation: None,
        transport: None,
        incoming_l1: None,
        reason: None,
        post: None,
    };
    let Some(a) = out.pre.event_authority() else {
        out.reason = Some("conserved asynchronous first event unresolved".into());
        return Ok(out);
    };
    let root = a.root().expect("ordered root");
    out.event_time = Some(root);
    let reset_polys = reset.polynomials()?;
    let prep = match a.prepare_vector_restart(
        &reset_polys,
        target.source(),
        &[0.125],
        &ROBERTSON_BRANCH,
        times[1],
        policy,
        rp,
        64,
    ) {
        Ok(v) => v,
        Err(e) => {
            out.reason = Some(e.to_string());
            return Ok(out);
        }
    };
    if let Some(reason) = prep.reason.clone() {
        out.reason = Some(reason);
        out.preparation = Some(prep);
        return Ok(out);
    }
    let (evidence, refusal) = simplex_transport_receipt(a, reset, &prep, root, sync)?;
    if let Some(e) = refusal {
        out.reason = Some(e);
        out.preparation = Some(prep);
        out.transport = Some(evidence);
        return Ok(out);
    }
    let center = evidence.center;
    let radius = evidence.selected_l1;
    out.incoming_l1 = Some(radius);
    out.transport = Some(evidence);
    out.preparation = Some(prep);
    // Every physical reset state lies in the exact simplex. The target
    // original source preserves it during synchronization and postflow.
    // The rounded center need not itself be physical; this constructor
    // is private and requires the preceding source-bound proof.
    let start = RobertsonInitial {
        center,
        l1_radius: radius,
    };
    let mut o = opts.clone();
    o.max_steps = o.max_steps.saturating_sub(out.charged_native_attempts);
    if o.max_steps == 0 {
        out.reason = Some("conserved asynchronous native attempt budget".into());
        return Ok(out);
    }
    match robertson_epoch(
        &target,
        start,
        [root[1], times[1]],
        &o,
        policy,
        route,
        certificate,
        grid,
    ) {
        Ok(r) => {
            out.native_attempts += r.driver.stats.attempts;
            out.charged_native_attempts += r.driver.stats.attempts;
            if !r.global_qualified || !r.branch_qualified || !r.pointwise_qualified {
                out.reason = Some("conserved asynchronous post physical proof unresolved".into());
            }
            out.post = Some(r);
        }
        Err(e) => {
            out.charged_native_attempts += o.max_steps;
            out.reason = Some(e.to_string());
        }
    }
    Ok(out)
}

fn simplex_transport_receipt(
    a: &ThresholdRun,
    reset: &SimplexReset,
    prep: &VectorRestartRun,
    root: [f64; 2],
    sync: SimplexSyncRoute,
) -> CoreResult<(SimplexTransportReceipt, Option<String>)> {
    simplex_transport_recorded(a, reset, prep, root, sync, &mut AttemptCount::default())
}
fn simplex_transport_recorded(
    a: &ThresholdRun,
    reset: &SimplexReset,
    prep: &VectorRestartRun,
    root: [f64; 2],
    sync: SimplexSyncRoute,
    audit: &mut AttemptCount,
) -> CoreResult<(SimplexTransportReceipt, Option<String>)> {
    let bp = prep.restart_branch.as_ref().expect("prepared branch");
    let center = [bp.initial.x[0], bp.initial.x[1]];
    let radius = prep.incoming_radius.expect("prepared radius");
    let box_l1 = mul_up(2., radius)?;
    let mut component_l1 = 0.;
    for (b, &c) in prep
        .synchronized_x
        .as_ref()
        .expect("prepared synchronization")
        .iter()
        .zip(&center)
    {
        component_l1 = add_up(component_l1, diameter(b[0], c)?.max(diameter(c, b[1])?))?;
    }
    let mut evidence = SimplexTransportReceipt {
        route: sync,
        operator_l1: reset.operator_l1(),
        center,
        box_l1,
        component_l1,
        joint_l1: None,
        selected_l1: box_l1,
        cells: vec![],
    };
    if sync == SimplexSyncRoute::JointTransport {
        let result = (|| -> CoreResult<f64> {
            let mut joint = 0_f64;
            for cell in &prep.cells {
                let slab = a
                    .trial
                    .slabs
                    .iter()
                    .find(|s| s.times[0] <= cell.times[0] && cell.times[1] <= s.times[1])
                    .ok_or_else(|| bad("simplex transport native coverage"))?;
                let c = slab
                    .certificate
                    .as_ref()
                    .filter(|c| c.global_qualified && c.conservation.is_some())
                    .ok_or_else(|| bad("simplex pre L1 authority"))?;
                let y = slab
                    .power
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|v| Iv::new(v[0], v[1]))
                            .collect::<CoreResult<Vec<_>>>()
                    })
                    .collect::<CoreResult<Vec<_>>>()?;
                let r = de::simplex_transport_cell(
                    reset,
                    &y,
                    c,
                    cell.times,
                    root[1],
                    cell.tube.as_ref().expect("prepared tube"),
                    center,
                );
                audit.record(r.is_ok());
                let v = r?;
                joint = joint.max(v.joint_l1);
                evidence.cells.push(v);
            }
            Ok(joint)
        })();
        match result {
            Ok(v) => {
                evidence.joint_l1 = Some(v);
                evidence.selected_l1 = box_l1.min(component_l1).min(v);
            }
            Err(e) => return Ok((evidence, Some(e.to_string()))),
        }
    }
    Ok((evidence, None))
}

pub use de::{SimplexGapEvidence, SimplexGapRoute};
pub struct ConservedMode {
    pub rate: f64,
    pub route: EvolutionRoute,
    pub certificate: ConservationRoute,
    pub guards: Vec<DeclaredPolynomial>,
    pub transitions: Vec<ConservedTransition>,
}
pub struct ConservedTransition {
    pub target: usize,
    pub reset: SimplexReset,
    pub policy: VectorResetPolicy,
    pub sync: SimplexSyncRoute,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ConservedChainBudget {
    pub limits: ChainBudget,
    /// Conservative quota: up to eight Picard attempts per reset cell
    /// and one uniform restart branch proof. Failed hidden work is charged.
    pub picard_attempts: usize,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ConservedChainWork {
    pub common: ChainWork,
    pub charged_picard_attempts: usize,
    pub joint_cells: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConservedGapCheck {
    pub guard: usize,
    pub cell: Option<usize>,
    pub enclosure: [f64; 2],
    pub evidence: SimplexGapEvidence,
}
#[derive(Serialize)]
pub struct ConservedEvent {
    pub from: usize,
    pub target: usize,
    pub guard: usize,
    pub event_time: [f64; 2],
    pub dwell_lower: Option<f64>,
    pub committed: bool,
    pub reset: Option<VectorRestartRun>,
    pub transport: Option<SimplexTransportReceipt>,
    pub gap: Vec<ConservedGapCheck>,
    pub reason: Option<String>,
}
pub struct ConservedEpoch {
    inner: ChainEpoch,
    initial: RobertsonInitial,
}
impl ConservedEpoch {
    pub fn mode(&self) -> usize {
        self.inner.mode
    }
    pub fn start(&self) -> f64 {
        self.inner.start
    }
    pub fn safe(&self) -> f64 {
        self.inner.safe
    }
    pub fn incoming_l1(&self) -> f64 {
        self.initial.l1_radius
    }
    pub fn center(&self) -> [f64; 2] {
        self.initial.center
    }
    pub fn ordered(&self) -> Option<&MultiGuardRun> {
        self.inner.ordered()
    }
    pub fn terminal(&self) -> Option<&EvolutionRun> {
        self.inner.terminal()
    }
    pub fn original_slab_diagnostics(&self) -> &[SlabRecord] {
        if let Some(r) = &self.inner.ordered {
            r.runs.first().map_or(&[], |r| r.trial.slabs())
        } else {
            self.inner.terminal.as_ref().map_or(&[], |r| r.slabs())
        }
    }
}
pub struct ConservedChainRun {
    epochs: Vec<ConservedEpoch>,
    transitions: Vec<ConservedEvent>,
    final_mode: usize,
    stop: ChainStop,
    reason: Option<String>,
    work: ConservedChainWork,
}
impl ConservedChainRun {
    pub fn epochs(&self) -> &[ConservedEpoch] {
        &self.epochs
    }
    pub fn transitions(&self) -> &[ConservedEvent] {
        &self.transitions
    }
    pub fn final_mode(&self) -> usize {
        self.final_mode
    }
    pub fn stop(&self) -> ChainStop {
        self.stop
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn work(&self) -> &ConservedChainWork {
        &self.work
    }
    pub fn query(&self, epoch: usize, t: f64) -> CoreResult<Query> {
        let e = self
            .epochs
            .get(epoch)
            .ok_or_else(|| bad("unknown conserved epoch"))?;
        if !t.is_finite() || t < e.start() || t > e.safe() {
            return Err(bad("outside conserved physical prefix"));
        }
        if let Some(r) = e.ordered() {
            return r.pre_query(t);
        }
        let r = e
            .terminal()
            .ok_or_else(|| bad("unqualified conserved terminal epoch"))?;
        let q = r.query(t)?;
        if q.physical_error_upper
            .is_none_or(|v| v > r.engine.policy.global_tol)
        {
            return Err(bad("unqualified conserved query"));
        }
        Ok(q)
    }
}
/// Actual native source-owned Robertson mode chain. The only way to
/// manufacture the private post initial set is a certified stochastic
/// reset followed by source-owned flow and whole-gap guard exclusion.
#[allow(clippy::too_many_arguments)]
pub fn evolve_conserved_chain(
    modes: &[ConservedMode],
    initial_mode: usize,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: ConservedChainBudget,
    gap: SimplexGapRoute,
) -> CoreResult<ConservedChainRun> {
    conserved_chain_recorded(
        modes,
        initial_mode,
        initial,
        times,
        opts,
        policy,
        mp,
        budget,
        gap,
        None,
    )
}
#[derive(Clone, Debug, Serialize)]
pub struct ChainTransitionAudit {
    pub from: usize,
    pub target: usize,
    pub guard: usize,
    pub root: [f64; 2],
    pub cell_quota: usize,
    pub charged_cells: usize,
    pub charged_picard_attempts: usize,
    pub work: ResetTransactionAudit,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ConservedChainResourceAudit {
    pub admitted: bool,
    pub native: Vec<AtlasNativeAudit>,
    pub transitions: Vec<ChainTransitionAudit>,
}
/// Diagnostic execution snapshot including rejected reset work.
#[allow(clippy::too_many_arguments)]
pub fn evolve_conserved_chain_audited(
    modes: &[ConservedMode],
    initial_mode: usize,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: ConservedChainBudget,
    gap: SimplexGapRoute,
) -> (CoreResult<ConservedChainRun>, ConservedChainResourceAudit) {
    let mut a = ConservedChainResourceAudit::default();
    let r = conserved_chain_recorded(
        modes,
        initial_mode,
        initial,
        times,
        opts,
        policy,
        mp,
        budget,
        gap,
        Some(&mut a),
    );
    (r, a)
}
#[allow(clippy::too_many_arguments)]
fn conserved_chain_recorded(
    modes: &[ConservedMode],
    initial_mode: usize,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: ConservedChainBudget,
    gap: SimplexGapRoute,
    mut audit: Option<&mut ConservedChainResourceAudit>,
) -> CoreResult<ConservedChainRun> {
    let b = budget.limits;
    robertson_policy(policy)?;
    if modes.is_empty()
        || modes.len() > 64
        || initial_mode >= modes.len()
        || mp.method == MultiGuardRoute::Independent
        || b.native_attempts == 0
        || b.scan_slabs == 0
        || b.transition_attempts > 64
        || !b.min_dwell.is_finite()
        || b.min_dwell <= 0.
        || !times[0].is_finite()
        || !times[1].is_finite()
        || times[1] <= times[0]
    {
        return Err(bad("conserved chain policy/input contract"));
    }
    let models = modes
        .iter()
        .map(|m| RobertsonModel::new(m.rate))
        .collect::<CoreResult<Vec<_>>>()?;
    for m in modes {
        if m.guards.len() != m.transitions.len()
            || (!m.guards.is_empty() && !(2..=8).contains(&m.guards.len()))
        {
            return Err(bad("conserved chain guard/edge shape"));
        }
        for (g, e) in m.guards.iter().zip(&m.transitions) {
            g.validate(3)?;
            if e.target >= modes.len() || !matches!(e.policy.subdivisions, 1 | 4) {
                return Err(bad("conserved chain edge contract"));
            }
        }
    }
    if let Some(a) = audit.as_deref_mut() {
        a.admitted = true;
    }
    let mut out = ConservedChainRun {
        epochs: vec![],
        transitions: vec![],
        final_mode: initial_mode,
        stop: ChainStop::Horizon,
        reason: None,
        work: ConservedChainWork::default(),
    };
    let mut mode = initial_mode;
    let mut clock = times[0];
    let mut start = initial;
    let mut prior: Option<[f64; 2]> = None;
    let total_native = b.native_attempts.min(opts.max_steps);
    macro_rules! stop {
        ($k:expr,$e:expr) => {{
            out.stop = $k;
            out.reason = Some($e.to_string());
            return Ok(out);
        }};
    }
    loop {
        let w = &mut out.work.common;
        let remaining = total_native.saturating_sub(w.charged_native_attempts);
        if remaining == 0 {
            stop!(ChainStop::NativeBudget, "conserved chain native budget");
        }
        let slabs = b.scan_slabs.saturating_sub(w.charged_scan_slabs);
        if slabs == 0 {
            stop!(ChainStop::SlabBudget, "conserved chain slab budget");
        }
        let m = &modes[mode];
        let model = &models[mode];
        let mut o = opts.clone();
        o.max_steps = remaining;
        if m.guards.is_empty() {
            o.max_steps = o.max_steps.min(slabs);
            let h = audit
                .as_ref()
                .map(|_| Arc::new(Mutex::new(NativeResourceAudit::default())));
            let result = robertson_epoch_recorded(
                model,
                start,
                [clock, times[1]],
                &o,
                policy,
                m.route,
                m.certificate,
                &[],
                h.clone(),
            );
            if let (Some(a), Some(h)) = (audit.as_deref_mut(), h) {
                let native = h.lock().expect("native audit lock").clone();
                let (charged, slabs) = result.as_ref().map_or((o.max_steps, o.max_steps), |r| {
                    (r.driver.stats.attempts, r.driver.stats.accepted_steps)
                });
                a.native.push(AtlasNativeAudit {
                    mode,
                    clock: [clock, clock],
                    center: start.center,
                    route: m.route,
                    native_quota: o.max_steps,
                    slab_quota: o.max_steps,
                    returned: result.is_ok(),
                    charged_native_attempts: charged,
                    charged_scan_slabs: slabs,
                    measured_attempts: native.measured_attempts(),
                    native,
                });
            }
            let r = match result {
                Ok(v) => v,
                Err(e) => {
                    w.charged_native_attempts += o.max_steps;
                    w.charged_scan_slabs += o.max_steps;
                    stop!(ChainStop::NativeRefused, e);
                }
            };
            let n = r.driver.stats.attempts;
            w.native_runs += 1;
            w.native_attempts += n;
            w.charged_native_attempts += n;
            w.scan_slabs += r.slabs.len();
            w.charged_scan_slabs += r.driver.stats.accepted_steps;
            w.driver_rhs += r.driver.stats.counters.rhs_calls;
            w.projection_rhs += r.work.rhs_count;
            w.warp_predictions += r.work.warp_predictions;
            let good = r.global_qualified
                && r.branch_qualified
                && r.pointwise_qualified
                && r.driver.t.last() == Some(&times[1]);
            out.epochs.push(ConservedEpoch {
                initial: start,
                inner: ChainEpoch {
                    mode,
                    start: clock,
                    safe: if good { times[1] } else { clock },
                    incoming_radius: start.l1_radius,
                    ordered: None,
                    terminal: Some(r),
                },
            });
            if !good {
                let k = if w.charged_native_attempts >= total_native {
                    ChainStop::NativeBudget
                } else if w.charged_scan_slabs >= b.scan_slabs || n >= slabs {
                    ChainStop::SlabBudget
                } else {
                    ChainStop::NativeRefused
                };
                stop!(k, "conserved terminal physical proof unresolved");
            }
            return Ok(out);
        }
        let mut q = mp;
        q.max_slabs = q.max_slabs.min(slabs);
        let h = audit
            .as_ref()
            .map(|_| Arc::new(Mutex::new(NativeResourceAudit::default())));
        let result = robertson_first_guard_recorded(
            m.rate,
            start,
            [clock, times[1]],
            &o,
            policy,
            m.route,
            m.certificate,
            &m.guards,
            q,
            h.clone(),
        );
        if let (Some(a), Some(h)) = (audit.as_deref_mut(), h) {
            let native = h.lock().expect("native audit lock").clone();
            let (charged, slabs) = result.as_ref().map_or((remaining, q.max_slabs), |r| {
                (
                    r.work.attempts,
                    r.runs
                        .first()
                        .map_or(r.work.physical_certificates, |v| v.stats().accepted_steps),
                )
            });
            a.native.push(AtlasNativeAudit {
                mode,
                clock: [clock, clock],
                center: start.center,
                route: m.route,
                native_quota: remaining,
                slab_quota: q.max_slabs,
                returned: result.is_ok(),
                charged_native_attempts: charged,
                charged_scan_slabs: slabs,
                measured_attempts: native.measured_attempts(),
                native,
            });
        }
        let r = match result {
            Ok(v) => v,
            Err(e) => {
                w.charged_native_attempts += remaining;
                w.charged_scan_slabs += q.max_slabs;
                stop!(ChainStop::NativeRefused, e);
            }
        };
        let a = &r.work;
        w.native_runs += a.native_runs;
        w.native_attempts += a.attempts;
        w.charged_native_attempts += a.attempts;
        w.scan_slabs += a.physical_certificates;
        w.charged_scan_slabs += r
            .runs
            .first()
            .map_or(a.physical_certificates, |v| v.stats().accepted_steps);
        w.driver_rhs += a.driver_rhs;
        w.projection_rhs += a.shared_projection.rhs_count
            + a.replay_projection_rhs
            + a.guard_prepared_rhs
            + a.difference_prepared_rhs;
        w.guard_evals += a.guard_evals;
        w.dictionary_uses += a.dictionary_uses;
        w.comparison_attempts += a.comparison_attempts;
        w.warp_predictions += a.shared_projection.warp_predictions
            + r.runs.first().map_or(0, |v| v.trial.work.warp_predictions);
        if r.runs.first().is_none_or(|v| {
            v.trial.initial.x != start.center
                || v.trial
                    .driver
                    .y
                    .first()
                    .is_none_or(|x| x[..2] != start.center)
        }) {
            stop!(
                ChainStop::NativeRefused,
                "conserved differential center changed"
            );
        }
        let winner = r.winner();
        let safe = r.committed_until();
        let reason = r.reason().map(str::to_owned);
        out.epochs.push(ConservedEpoch {
            initial: start,
            inner: ChainEpoch {
                mode,
                start: clock,
                safe,
                incoming_radius: start.l1_radius,
                ordered: Some(r),
                terminal: None,
            },
        });
        let Some(j) = winner else {
            if reason.is_none() && safe == times[1] {
                return Ok(out);
            }
            let k = if w.charged_native_attempts >= total_native {
                ChainStop::NativeBudget
            } else if w.charged_scan_slabs >= b.scan_slabs {
                ChainStop::SlabBudget
            } else {
                ChainStop::OrderUnresolved
            };
            stop!(
                k,
                reason.unwrap_or_else(|| "conserved order unresolved".into())
            );
        };
        let authority = out
            .epochs
            .last()
            .unwrap()
            .ordered()
            .unwrap()
            .event_authority()
            .expect("ordered winner");
        let root = authority.root().expect("physical root");
        let edge = &m.transitions[j];
        let target = &modes[edge.target];
        let target_model = &models[edge.target];
        let dwell = prior
            .map(|p| rodas5p_core::directed::sub_down(root[0], p[1]))
            .transpose()?;
        let mut ev = ConservedEvent {
            from: mode,
            target: edge.target,
            guard: j,
            event_time: root,
            dwell_lower: dwell,
            committed: false,
            reset: None,
            transport: None,
            gap: vec![],
            reason: None,
        };
        let cap = (b.reset_cells.saturating_sub(w.charged_reset_cells))
            .min(64)
            .min(
                budget
                    .picard_attempts
                    .saturating_sub(out.work.charged_picard_attempts)
                    .saturating_sub(1)
                    / 8,
            );
        let refuse = if dwell.is_some_and(|v| v < b.min_dwell) {
            Some(ChainStop::DwellUnproved)
        } else if w.transitions_attempted >= b.transition_attempts {
            Some(ChainStop::TransitionBudget)
        } else if cap == 0 {
            Some(ChainStop::ResetBudget)
        } else if root[1] >= times[1] {
            Some(ChainStop::ResetRefused)
        } else {
            None
        };
        if let Some(k) = refuse {
            ev.reason = Some(format!("{k:?}"));
            out.transitions.push(ev);
            stop!(k, format!("{k:?}"));
        }
        w.transitions_attempted += 1;
        let mut discarded = ChainTransitionAudit {
            from: mode,
            target: edge.target,
            guard: j,
            root,
            cell_quota: cap,
            charged_cells: 0,
            charged_picard_attempts: 0,
            work: ResetTransactionAudit::default(),
        };
        let a = if let Some(a) = audit.as_deref_mut() {
            a.transitions.push(discarded.clone());
            a.transitions.last_mut().unwrap()
        } else {
            &mut discarded
        };
        let polynomials = edge.reset.polynomials()?;
        let prep = match authority.prepare_vector_restart_recorded(
            &polynomials,
            target_model.source(),
            &[0.125],
            &ROBERTSON_BRANCH,
            times[1],
            policy,
            edge.policy,
            cap,
            &mut a.work,
        ) {
            Ok(v) => v,
            Err(e) => {
                w.charged_reset_cells += cap;
                out.work.charged_picard_attempts += 8 * cap + 1;
                a.charged_cells = cap;
                a.charged_picard_attempts = 8 * cap + 1;
                ev.reason = Some(e.to_string());
                out.transitions.push(ev);
                stop!(ChainStop::ResetRefused, e);
            }
        };
        add_reset_work(w, &prep);
        out.work.charged_picard_attempts += 8 * prep.cells.len() + 1;
        a.charged_cells = prep.cells.len();
        a.charged_picard_attempts = 8 * prep.cells.len() + 1;
        if let Some(e) = prep.reason.clone() {
            let k = if e.contains("cell cap") {
                ChainStop::ResetBudget
            } else {
                ChainStop::ResetRefused
            };
            ev.reason = Some(e.clone());
            ev.reset = Some(prep);
            out.transitions.push(ev);
            stop!(k, e);
        }
        let result = simplex_transport_recorded(
            authority,
            &edge.reset,
            &prep,
            root,
            edge.sync,
            &mut a.work.transport_cells,
        );
        a.work.transport.record(result.is_ok());
        a.work.transport_qualified += usize::from(result.as_ref().is_ok_and(|(_, r)| r.is_none()));
        let (transport, refusal) = match result {
            Ok(v) => v,
            Err(e) => {
                ev.reason = Some(e.to_string());
                ev.reset = Some(prep);
                out.transitions.push(ev);
                stop!(ChainStop::ResetRefused, e);
            }
        };
        out.work.joint_cells += transport.cells.len();
        if let Some(e) = refusal {
            ev.reason = Some(e.clone());
            ev.reset = Some(prep);
            ev.transport = Some(transport);
            out.transitions.push(ev);
            stop!(ChainStop::ResetRefused, e);
        }
        let next = RobertsonInitial {
            center: transport.center,
            l1_radius: transport.selected_l1,
        };
        let checked = (|| -> Result<(), (ChainStop, String)> {
            // Actual branch tube at each possible transition clock.
            for (ci, cell) in prep.cells.iter().enumerate() {
                let tube = cell.tube.as_ref().expect("prepared tube");
                let bp = tube.attempts.last().expect("tube branch");
                let xb = [bp.differential_box[0], bp.differential_box[1]];
                let center = [bp.initial.x[0], bp.initial.x[1]];
                for (k, g) in target.guards.iter().enumerate() {
                    if w.gap_checks >= b.gap_checks {
                        return Err((ChainStop::GapBudget, "conserved gap budget".into()));
                    }
                    w.gap_checks += 1;
                    let r = target_model.simplex_gap_enclosure(g, bp.times, xb, center, None, gap);
                    a.work.gap_checks.record(r.is_ok());
                    let e = r.map_err(|e| (ChainStop::GapUnproved, e.to_string()))?;
                    let negative = e.enclosure[1] < 0.;
                    a.work.negative_gaps += usize::from(negative);
                    ev.gap.push(ConservedGapCheck {
                        guard: k,
                        cell: Some(ci),
                        enclosure: e.enclosure,
                        evidence: e,
                    });
                    if !negative {
                        return Err((
                            ChainStop::GapUnproved,
                            "outgoing guard not negative over conserved transition tube".into(),
                        ));
                    }
                }
            }
            // Check the actual transported L1 set, not the smaller
            // original infinity box or just its rounded center.
            for (k, g) in target.guards.iter().enumerate() {
                if w.gap_checks >= b.gap_checks {
                    return Err((ChainStop::GapBudget, "conserved gap budget".into()));
                }
                w.gap_checks += 1;
                let r = target_model.simplex_gap_enclosure(
                    g,
                    [root[1], root[1]],
                    [[0.; 2]; 2],
                    next.center,
                    Some(next.l1_radius),
                    gap,
                );
                a.work.gap_checks.record(r.is_ok());
                let e = r.map_err(|e| (ChainStop::GapUnproved, e.to_string()))?;
                let negative = e.enclosure[1] < 0.;
                a.work.negative_gaps += usize::from(negative);
                ev.gap.push(ConservedGapCheck {
                    guard: k,
                    cell: None,
                    enclosure: e.enclosure,
                    evidence: e,
                });
                if !negative {
                    return Err((
                        ChainStop::GapUnproved,
                        "outgoing guard not negative on transported L1 restart set".into(),
                    ));
                }
            }
            Ok(())
        })();
        ev.reset = Some(prep);
        ev.transport = Some(transport);
        if let Err((k, e)) = checked {
            ev.reason = Some(e.clone());
            out.transitions.push(ev);
            stop!(k, e);
        }
        start = next;
        clock = root[1];
        prior = Some(root);
        mode = edge.target;
        ev.committed = true;
        a.work.committed = true;
        out.transitions.push(ev);
        w.transitions_committed += 1;
        out.final_mode = mode;
    }
}

pub use de::{PhaseCellEvidence, PhaseQueryRoute};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PhaseReuseRoute {
    Fresh,
    Reuse,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PhaseCycleBudget {
    pub native_attempts: usize,
    pub scan_slabs: usize,
    pub transitions: usize,
    pub min_dwell: f64,
    pub absolute_width: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PhaseCycleStop {
    Horizon,
    OrderUnresolved,
    NativeBudget,
    SlabBudget,
    TransitionBudget,
    DwellUnproved,
    AbsoluteWidth,
    HorizonAmbiguous,
    NativeRefused,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct PhaseCycleWork {
    pub native_runs: usize,
    pub native_attempts: usize,
    pub charged_native_attempts: usize,
    pub scan_slabs: usize,
    pub charged_scan_slabs: usize,
    pub driver_rhs: u64,
    pub projection_rhs: usize,
    pub guard_evals: usize,
    pub dictionary_uses: usize,
    pub comparison_attempts: usize,
    pub cache_hits: usize,
    pub reset_checks: usize,
    pub committed: usize,
}
pub struct RelativePhase {
    pub initial: RobertsonInitial,
    run: MultiGuardRun,
}
impl RelativePhase {
    pub fn ordered(&self) -> &MultiGuardRun {
        &self.run
    }
    pub fn original_slab_diagnostics(&self) -> &[SlabRecord] {
        self.run.runs.first().map_or(&[], |r| r.trial.slabs())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct PhaseEpoch {
    pub phase_index: usize,
    pub clock: [f64; 2],
    pub relative_safe: f64,
    pub absolute_safe: f64,
    pub cached: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct PhaseEvent {
    pub epoch: usize,
    pub phase_index: usize,
    pub guard: usize,
    pub relative: [f64; 2],
    pub absolute: [f64; 2],
    pub width_upper: f64,
    pub dwell_lower: Option<f64>,
    pub committed: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PhaseQuery {
    pub t: f64,
    pub epoch: usize,
    pub phase: [f64; 2],
    pub route: PhaseQueryRoute,
    pub enclosure: [[f64; 2]; 3],
    pub cells: Vec<PhaseCellEvidence>,
}
pub struct PhaseCycleRun {
    phases: Vec<RelativePhase>,
    epochs: Vec<PhaseEpoch>,
    events: Vec<PhaseEvent>,
    reset_center: [f64; 2],
    reset_guards: Vec<SimplexGapEvidence>,
    stop: PhaseCycleStop,
    reason: Option<String>,
    work: PhaseCycleWork,
}
impl PhaseCycleRun {
    pub fn phases(&self) -> &[RelativePhase] {
        &self.phases
    }
    pub fn epochs(&self) -> &[PhaseEpoch] {
        &self.epochs
    }
    pub fn events(&self) -> &[PhaseEvent] {
        &self.events
    }
    pub fn reset_center(&self) -> [f64; 2] {
        self.reset_center
    }
    pub fn reset_guards(&self) -> &[SimplexGapEvidence] {
        &self.reset_guards
    }
    pub fn stop(&self) -> PhaseCycleStop {
        self.stop
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn work(&self) -> &PhaseCycleWork {
        &self.work
    }
    /// One-sided epoch query, only when every admissible clock is in
    /// this epoch. The result encloses true state over the entire
    /// relative phase interval; no nominal-clock substitution.
    pub fn query(&self, epoch: usize, t: f64, route: PhaseQueryRoute) -> CoreResult<PhaseQuery> {
        let e = self
            .epochs
            .get(epoch)
            .ok_or_else(|| bad("unknown relative phase epoch"))?;
        phase_query_run(&self.phases[e.phase_index].run, e, epoch, t, route)
    }
}
/// Autonomous source, state-only guards, identical constant stochastic
/// self-loop reset. The immutable reset-phase receipt can be reused
/// because the full initial state and model are identical on every
/// post-reset phase. Absolute uncertainty is never erased.
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson_phase_cycle(
    mode: &ConservedMode,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: PhaseCycleBudget,
    reuse: PhaseReuseRoute,
) -> CoreResult<PhaseCycleRun> {
    robertson_policy(policy)?;
    if mode.guards.len() != mode.transitions.len()
        || !(2..=8).contains(&mode.guards.len())
        || mp.method == MultiGuardRoute::Independent
        || budget.native_attempts == 0
        || budget.scan_slabs == 0
        || budget.transitions > 64
        || !budget.min_dwell.is_finite()
        || budget.min_dwell <= 0.
        || !budget.absolute_width.is_finite()
        || budget.absolute_width <= 0.
        || !times[0].is_finite()
        || !times[1].is_finite()
        || times[1] <= times[0]
    {
        return Err(bad("phase cycle input/budget"));
    }
    let model = RobertsonModel::new(mode.rate)?;
    let matrix = *mode.transitions[0].reset.matrix();
    if matrix.iter().any(|r| r[0] != r[1] || r[0] != r[2])
        || mode
            .transitions
            .iter()
            .any(|e| e.target != 0 || e.reset.matrix() != &matrix || e.reset.operator_l1() != 0.)
    {
        return Err(bad(
            "phase cycle requires identical rank-zero stochastic self-loop",
        ));
    }
    let reset = RobertsonInitial::exact([matrix[0][0], matrix[1][0]])?;
    let mut reset_guards = Vec::with_capacity(mode.guards.len());
    for g in &mode.guards {
        g.validate(3)?;
        if g.terms()
            .iter()
            .flat_map(|t| &t.factors)
            .any(|f| f.time != 0.)
        {
            return Err(bad("phase cycle requires autonomous state-only guards"));
        }
        let v = model.simplex_gap_enclosure(
            g,
            [0., 0.],
            [[0.; 2]; 2],
            reset.center,
            Some(0.),
            SimplexGapRoute::AffineDual,
        )?;
        if v.enclosure[1] >= 0. {
            return Err(bad(
                "phase reset does not prove strict negative target guard",
            ));
        }
        reset_guards.push(v);
    }
    let span = Iv::point(times[1])?.sub(Iv::point(times[0])?)?.hi;
    let mut out = PhaseCycleRun {
        phases: Vec::with_capacity(budget.transitions + 1),
        epochs: Vec::with_capacity(budget.transitions + 1),
        events: Vec::with_capacity(budget.transitions + 1),
        reset_center: reset.center,
        reset_guards,
        stop: PhaseCycleStop::Horizon,
        reason: None,
        work: PhaseCycleWork {
            reset_checks: mode.guards.len(),
            ..Default::default()
        },
    };
    let total = budget.native_attempts.min(opts.max_steps);
    let mut clock = Iv::point(times[0])?;
    let mut template = None;
    let mut family = initial;
    macro_rules! stop {
        ($k:expr,$e:expr) => {{
            out.stop = $k;
            out.reason = Some($e.to_string());
            return Ok(out);
        }};
    }
    loop {
        let cached = reuse == PhaseReuseRoute::Reuse && template.is_some();
        let idx = if cached {
            out.work.cache_hits += 1;
            template.expect("cached phase")
        } else {
            let remaining = total.saturating_sub(out.work.charged_native_attempts);
            if remaining == 0 {
                stop!(PhaseCycleStop::NativeBudget, "relative phase native budget");
            }
            let slabs = budget
                .scan_slabs
                .saturating_sub(out.work.charged_scan_slabs);
            if slabs == 0 {
                stop!(PhaseCycleStop::SlabBudget, "relative phase slab budget");
            }
            let mut o = opts.clone();
            o.max_steps = remaining;
            let mut q = mp;
            q.max_slabs = q.max_slabs.min(slabs);
            let r = match evolve_robertson_to_first_guard(
                mode.rate,
                family,
                [0., span],
                &o,
                policy,
                mode.route,
                mode.certificate,
                &mode.guards,
                q,
            ) {
                Ok(v) => v,
                Err(e) => {
                    out.work.charged_native_attempts += remaining;
                    out.work.charged_scan_slabs += q.max_slabs;
                    stop!(PhaseCycleStop::NativeRefused, e);
                }
            };
            let w = &r.work;
            out.work.native_runs += w.native_runs;
            out.work.native_attempts += w.attempts;
            out.work.charged_native_attempts += w.attempts;
            out.work.scan_slabs += w.physical_certificates;
            out.work.charged_scan_slabs += r
                .runs
                .first()
                .map_or(w.physical_certificates, |v| v.stats().accepted_steps);
            out.work.driver_rhs += w.driver_rhs;
            out.work.projection_rhs += w.shared_projection.rhs_count
                + w.replay_projection_rhs
                + w.guard_prepared_rhs
                + w.difference_prepared_rhs;
            out.work.guard_evals += w.guard_evals;
            out.work.dictionary_uses += w.dictionary_uses;
            out.work.comparison_attempts += w.comparison_attempts;
            if r.runs.first().is_none_or(|v| {
                v.trial.initial.x != family.center
                    || v.trial
                        .driver
                        .y
                        .first()
                        .is_none_or(|x| x[..2] != family.center)
            }) {
                stop!(
                    PhaseCycleStop::NativeRefused,
                    "relative differential center changed"
                );
            }
            let i = out.phases.len();
            if reuse == PhaseReuseRoute::Reuse && out.work.committed > 0 {
                template = Some(i);
            }
            out.phases.push(RelativePhase {
                initial: family,
                run: r,
            });
            i
        };
        let r = &out.phases[idx].run;
        let safe = r.committed_until();
        let absolute_safe = Iv::point(clock.lo)?.add(Iv::point(safe)?)?.lo.min(times[1]);
        out.epochs.push(PhaseEpoch {
            phase_index: idx,
            clock: clock.as_pair(),
            relative_safe: safe,
            absolute_safe,
            cached,
        });
        let needed = Iv::point(times[1])?.sub(clock)?.hi;
        let Some(authority) = r.event_authority() else {
            if r.initial_negative && (safe > needed || (safe == needed && r.reason().is_none())) {
                return Ok(out);
            }
            let k = if out.work.charged_native_attempts >= total {
                PhaseCycleStop::NativeBudget
            } else if out.work.charged_scan_slabs >= budget.scan_slabs {
                PhaseCycleStop::SlabBudget
            } else {
                PhaseCycleStop::OrderUnresolved
            };
            stop!(k, r.reason().unwrap_or("relative first event unresolved"));
        };
        let relative = authority.root().expect("ordered relative root");
        let absolute = clock.add(Iv::new(relative[0], relative[1])?)?;
        if absolute.lo > times[1] && safe >= needed {
            return Ok(out);
        }
        let width = Iv::point(absolute.hi)?.sub(Iv::point(absolute.lo)?)?.hi;
        let dwell = (out.work.committed > 0).then_some(relative[0]);
        let mut ev = PhaseEvent {
            epoch: out.epochs.len() - 1,
            phase_index: idx,
            guard: r.winner().expect("winner"),
            relative,
            absolute: absolute.as_pair(),
            width_upper: width,
            dwell_lower: dwell,
            committed: false,
            reason: None,
        };
        let refusal = if absolute.lo > times[1] {
            Some(PhaseCycleStop::OrderUnresolved)
        } else if absolute.hi >= times[1] {
            Some(PhaseCycleStop::HorizonAmbiguous)
        } else if width > budget.absolute_width {
            Some(PhaseCycleStop::AbsoluteWidth)
        } else if dwell.is_some_and(|v| v < budget.min_dwell) {
            Some(PhaseCycleStop::DwellUnproved)
        } else if out.work.committed >= budget.transitions {
            Some(PhaseCycleStop::TransitionBudget)
        } else {
            None
        };
        if let Some(k) = refusal {
            ev.reason = Some(format!("{k:?}"));
            out.events.push(ev);
            stop!(k, format!("{k:?}"));
        }
        ev.committed = true;
        out.events.push(ev);
        out.work.committed += 1;
        clock = absolute;
        family = reset;
    }
}

fn phase_query_run(
    a: &MultiGuardRun,
    e: &PhaseEpoch,
    epoch: usize,
    t: f64,
    route: PhaseQueryRoute,
) -> CoreResult<PhaseQuery> {
    if !t.is_finite() || t < e.clock[1] || t > e.absolute_safe {
        return Err(bad("outside common absolute phase prefix"));
    }
    if !a.initial_negative {
        return Err(bad("unqualified initial phase"));
    }
    let span = Iv::point(t)?.sub(Iv::new(e.clock[0], e.clock[1])?)?;
    if span.lo < 0. || span.hi > e.relative_safe {
        return Err(bad("relative phase outside certified prefix"));
    }
    let mut covered = span.lo;
    let mut cells = Vec::new();
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for s in a.runs.first().map_or(&[][..], |r| r.trial.slabs()) {
        let lo = span.lo.max(s.times[0]);
        let hi = span.hi.min(s.times[1]);
        if lo > hi {
            continue;
        }
        if lo > covered {
            return Err(bad("relative phase coverage gap"));
        }
        let c = s
            .certificate
            .as_ref()
            .filter(|c| c.global_qualified && c.conservation.is_some())
            .ok_or_else(|| bad("uncertified phase slab"))?;
        let y = s
            .power
            .iter()
            .map(|row| {
                row.iter()
                    .map(|v| Iv::new(v[0], v[1]))
                    .collect::<CoreResult<Vec<_>>>()
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let v = de::phase_slab_enclosure(&y, c, [lo, hi], route)?;
        for (h, x) in hull.iter_mut().zip(v.enclosure) {
            h[0] = h[0].min(x[0]);
            h[1] = h[1].max(x[1]);
        }
        cells.push(v);
        covered = covered.max(hi);
    }
    if cells.is_empty() || covered != span.hi {
        return Err(bad("incomplete relative phase coverage"));
    }
    Ok(PhaseQuery {
        t,
        epoch,
        phase: span.as_pair(),
        route,
        enclosure: hull,
        cells,
    })
}

/// Per-call phase atlas. Source/guard/policy identity is bound by the
/// immutable borrowed mode table and invocation, never by a global cache.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct PhaseAtlasBudget {
    pub limits: PhaseCycleBudget,
    pub reset_checks: usize,
    pub cache_entries: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PhaseAtlasStop {
    Horizon,
    OrderUnresolved,
    NativeBudget,
    SlabBudget,
    TransitionBudget,
    DwellUnproved,
    AbsoluteWidth,
    HorizonAmbiguous,
    NativeRefused,
    ResetBudget,
    ResetRefused,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct PhaseAtlasWork {
    pub phase: PhaseCycleWork,
    pub cache_lookups: usize,
    pub cache_inserts: usize,
    pub cache_full_misses: usize,
}
pub struct AtlasRelativePhase {
    pub mode: usize,
    pub initial: RobertsonInitial,
    run: MultiGuardRun,
}
impl AtlasRelativePhase {
    pub fn ordered(&self) -> &MultiGuardRun {
        &self.run
    }
    pub fn original_slab_diagnostics(&self) -> &[SlabRecord] {
        self.run.runs.first().map_or(&[], |r| r.trial.slabs())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct PhaseAtlasEpoch {
    pub mode: usize,
    pub phase: PhaseEpoch,
}
#[derive(Clone, Debug, Serialize)]
pub struct PhaseAtlasEvent {
    pub from: usize,
    pub target: usize,
    pub reset_center: [f64; 2],
    pub event: PhaseEvent,
    pub reset_guards: Vec<SimplexGapEvidence>,
}
pub struct PhaseAtlasRun {
    phases: Vec<AtlasRelativePhase>,
    epochs: Vec<PhaseAtlasEpoch>,
    events: Vec<PhaseAtlasEvent>,
    final_mode: usize,
    stop: PhaseAtlasStop,
    reason: Option<String>,
    work: PhaseAtlasWork,
}
impl PhaseAtlasRun {
    pub fn phases(&self) -> &[AtlasRelativePhase] {
        &self.phases
    }
    pub fn epochs(&self) -> &[PhaseAtlasEpoch] {
        &self.epochs
    }
    pub fn events(&self) -> &[PhaseAtlasEvent] {
        &self.events
    }
    pub fn final_mode(&self) -> usize {
        self.final_mode
    }
    pub fn stop(&self) -> PhaseAtlasStop {
        self.stop
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn work(&self) -> &PhaseAtlasWork {
        &self.work
    }
    pub fn query(&self, epoch: usize, t: f64, route: PhaseQueryRoute) -> CoreResult<PhaseQuery> {
        let e = self
            .epochs
            .get(epoch)
            .ok_or_else(|| bad("unknown atlas epoch"))?;
        let p = &self.phases[e.phase.phase_index];
        if p.mode != e.mode {
            return Err(bad("atlas phase identity mismatch"));
        }
        phase_query_run(&p.run, &e.phase, epoch, t, route)
    }
}
/// Finite autonomous Robertson graph, every edge a constant exact
/// stochastic reset. Unsupported graphs are explicitly rejected;
/// callers may separately select the existing conserved chain.
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson_phase_atlas(
    modes: &[ConservedMode],
    initial_mode: usize,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: PhaseAtlasBudget,
    reuse: PhaseReuseRoute,
) -> CoreResult<PhaseAtlasRun> {
    atlas_recorded(
        modes,
        initial_mode,
        initial,
        times,
        opts,
        policy,
        mp,
        budget,
        reuse,
        None,
    )
}
#[derive(Clone, Debug, Serialize)]
pub struct AtlasNativeAudit {
    pub mode: usize,
    pub clock: [f64; 2],
    pub center: [f64; 2],
    pub route: EvolutionRoute,
    pub native_quota: usize,
    pub slab_quota: usize,
    pub returned: bool,
    pub charged_native_attempts: usize,
    pub charged_scan_slabs: usize,
    pub measured_attempts: Option<usize>,
    pub native: NativeResourceAudit,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct AtlasResourceAudit {
    pub admitted: bool,
    pub native: Vec<AtlasNativeAudit>,
    pub cache_hits: usize,
    pub reset_guards: AttemptCount,
    pub negative_reset_guards: usize,
    pub committed: usize,
}
/// Audited finite atlas. Diagnostic receipts are not used for decisions,
/// do not change quota debits, and survive even a refused native run.
#[allow(clippy::too_many_arguments)]
pub fn evolve_robertson_phase_atlas_audited(
    modes: &[ConservedMode],
    initial_mode: usize,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: PhaseAtlasBudget,
    reuse: PhaseReuseRoute,
) -> (CoreResult<PhaseAtlasRun>, AtlasResourceAudit) {
    let mut a = AtlasResourceAudit::default();
    let r = atlas_recorded(
        modes,
        initial_mode,
        initial,
        times,
        opts,
        policy,
        mp,
        budget,
        reuse,
        Some(&mut a),
    );
    (r, a)
}
#[allow(clippy::too_many_arguments)]
fn atlas_recorded(
    modes: &[ConservedMode],
    initial_mode: usize,
    initial: RobertsonInitial,
    times: [f64; 2],
    opts: &Options,
    policy: EvolutionPolicy,
    mp: MultiGuardPolicy,
    budget: PhaseAtlasBudget,
    reuse: PhaseReuseRoute,
    mut audit: Option<&mut AtlasResourceAudit>,
) -> CoreResult<PhaseAtlasRun> {
    robertson_policy(policy)?;
    let b = budget.limits;
    if modes.is_empty()
        || modes.len() > 64
        || initial_mode >= modes.len()
        || mp.method == MultiGuardRoute::Independent
        || b.native_attempts == 0
        || b.scan_slabs == 0
        || b.transitions > 64
        || budget.cache_entries > 65
        || !b.min_dwell.is_finite()
        || b.min_dwell <= 0.
        || !b.absolute_width.is_finite()
        || b.absolute_width <= 0.
        || !times[0].is_finite()
        || !times[1].is_finite()
        || times[1] <= times[0]
    {
        return Err(bad("phase atlas input/budget"));
    }
    let models = modes
        .iter()
        .map(|m| RobertsonModel::new(m.rate))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut centers = Vec::with_capacity(modes.len());
    for m in modes {
        if m.guards.len() != m.transitions.len() || !(2..=8).contains(&m.guards.len()) {
            return Err(bad("phase atlas guard/edge shape"));
        }
        for g in &m.guards {
            g.validate(3)?;
            if g.terms()
                .iter()
                .flat_map(|t| &t.factors)
                .any(|f| f.time != 0.)
            {
                return Err(bad("phase atlas requires state-only guards"));
            }
        }
        let mut c = Vec::with_capacity(m.transitions.len());
        for edge in &m.transitions {
            let matrix = edge.reset.matrix();
            if edge.target >= modes.len()
                || edge.reset.operator_l1() != 0.
                || matrix.iter().any(|r| r[0] != r[1] || r[0] != r[2])
            {
                return Err(bad("phase atlas requires rank-zero stochastic edges"));
            }
            c.push(RobertsonInitial::exact([matrix[0][0], matrix[1][0]])?);
        }
        centers.push(c);
    }
    let span = Iv::point(times[1])?.sub(Iv::point(times[0])?)?.hi;
    if let Some(a) = audit.as_deref_mut() {
        a.admitted = true;
    }
    let total = b.native_attempts.min(opts.max_steps);
    let mut out = PhaseAtlasRun {
        phases: Vec::with_capacity(b.transitions + 1),
        epochs: Vec::with_capacity(b.transitions + 1),
        events: Vec::with_capacity(b.transitions + 1),
        final_mode: initial_mode,
        stop: PhaseAtlasStop::Horizon,
        reason: None,
        work: PhaseAtlasWork::default(),
    };
    // No cache escapes this invocation. Index identifies the complete
    // immutable mode, bitwise center identifies its exact reset state.
    // Only successful exact-relative first-event receipts are inserted.
    let mut cache: Vec<(usize, [u64; 2], usize)> = Vec::with_capacity(budget.cache_entries);
    let mut mode = initial_mode;
    let mut family = initial;
    let mut clock = Iv::point(times[0])?;
    macro_rules! stop {
        ($k:expr,$why:expr) => {{
            out.stop = $k;
            out.reason = Some($why.to_string());
            return Ok(out);
        }};
    }
    loop {
        let m = &modes[mode];
        let key = family.center.map(f64::to_bits);
        let hit = if reuse == PhaseReuseRoute::Reuse && family.l1_radius == 0. {
            out.work.cache_lookups += 1;
            cache
                .iter()
                .find(|v| v.0 == mode && v.1 == key)
                .map(|v| v.2)
        } else {
            None
        };
        let idx = if let Some(i) = hit {
            out.work.phase.cache_hits += 1;
            if let Some(a) = audit.as_deref_mut() {
                a.cache_hits += 1;
            }
            i
        } else {
            let remaining = total.saturating_sub(out.work.phase.charged_native_attempts);
            if remaining == 0 {
                stop!(PhaseAtlasStop::NativeBudget, "phase atlas native budget");
            }
            let slabs = b
                .scan_slabs
                .saturating_sub(out.work.phase.charged_scan_slabs);
            if slabs == 0 {
                stop!(PhaseAtlasStop::SlabBudget, "phase atlas slab budget");
            }
            let mut o = opts.clone();
            o.max_steps = remaining;
            let mut q = mp;
            q.max_slabs = q.max_slabs.min(slabs);
            let h = audit
                .as_ref()
                .map(|_| Arc::new(Mutex::new(NativeResourceAudit::default())));
            let result = robertson_first_guard_recorded(
                m.rate,
                family,
                [0., span],
                &o,
                policy,
                m.route,
                m.certificate,
                &m.guards,
                q,
                h.clone(),
            );
            if let (Some(a), Some(h)) = (audit.as_deref_mut(), h) {
                let native = h.lock().expect("native audit lock").clone();
                let (charged, slabs) = match &result {
                    Ok(v) => (
                        v.work.attempts,
                        v.runs
                            .first()
                            .map_or(v.work.physical_certificates, |r| r.stats().accepted_steps),
                    ),
                    Err(_) => (remaining, q.max_slabs),
                };
                a.native.push(AtlasNativeAudit {
                    mode,
                    clock: clock.as_pair(),
                    center: family.center,
                    route: m.route,
                    native_quota: remaining,
                    slab_quota: q.max_slabs,
                    returned: result.is_ok(),
                    charged_native_attempts: charged,
                    charged_scan_slabs: slabs,
                    measured_attempts: native.measured_attempts(),
                    native,
                });
            }
            let r = match result {
                Ok(v) => v,
                Err(e) => {
                    out.work.phase.charged_native_attempts += remaining;
                    out.work.phase.charged_scan_slabs += q.max_slabs;
                    stop!(PhaseAtlasStop::NativeRefused, e);
                }
            };
            let w = &r.work;
            let a = &mut out.work.phase;
            a.native_runs += w.native_runs;
            a.native_attempts += w.attempts;
            a.charged_native_attempts += w.attempts;
            a.scan_slabs += w.physical_certificates;
            a.charged_scan_slabs += r
                .runs
                .first()
                .map_or(w.physical_certificates, |v| v.stats().accepted_steps);
            a.driver_rhs += w.driver_rhs;
            a.projection_rhs += w.shared_projection.rhs_count
                + w.replay_projection_rhs
                + w.guard_prepared_rhs
                + w.difference_prepared_rhs;
            a.guard_evals += w.guard_evals;
            a.dictionary_uses += w.dictionary_uses;
            a.comparison_attempts += w.comparison_attempts;
            if r.runs.first().is_none_or(|v| {
                v.trial.initial.x != family.center
                    || v.trial
                        .driver
                        .y
                        .first()
                        .is_none_or(|x| x[..2] != family.center)
            }) {
                stop!(
                    PhaseAtlasStop::NativeRefused,
                    "atlas differential center changed"
                );
            }
            let i = out.phases.len();
            if reuse == PhaseReuseRoute::Reuse
                && family.l1_radius == 0.
                && r.event_authority().is_some()
            {
                if cache.len() < budget.cache_entries {
                    cache.push((mode, key, i));
                    out.work.cache_inserts += 1;
                } else {
                    out.work.cache_full_misses += 1;
                }
            }
            out.phases.push(AtlasRelativePhase {
                mode,
                initial: family,
                run: r,
            });
            i
        };
        let r = &out.phases[idx].run;
        let safe = r.committed_until();
        let abs_safe = Iv::point(clock.lo)?.add(Iv::point(safe)?)?.lo.min(times[1]);
        out.epochs.push(PhaseAtlasEpoch {
            mode,
            phase: PhaseEpoch {
                phase_index: idx,
                clock: clock.as_pair(),
                relative_safe: safe,
                absolute_safe: abs_safe,
                cached: hit.is_some(),
            },
        });
        let needed = Iv::point(times[1])?.sub(clock)?.hi;
        let Some(authority) = r.event_authority() else {
            if r.initial_negative && (safe > needed || (safe == needed && r.reason().is_none())) {
                return Ok(out);
            }
            let k = if out.work.phase.charged_native_attempts >= total {
                PhaseAtlasStop::NativeBudget
            } else if out.work.phase.charged_scan_slabs >= b.scan_slabs {
                PhaseAtlasStop::SlabBudget
            } else {
                PhaseAtlasStop::OrderUnresolved
            };
            stop!(k, r.reason().unwrap_or("atlas relative order unresolved"));
        };
        let j = r.winner().expect("winner");
        let target = m.transitions[j].target;
        let next = centers[mode][j];
        let relative = authority.root().expect("relative root");
        let absolute = clock.add(Iv::new(relative[0], relative[1])?)?;
        if absolute.lo > times[1] && safe >= needed {
            return Ok(out);
        }
        let width = Iv::point(absolute.hi)?.sub(Iv::point(absolute.lo)?)?.hi;
        let dwell = (out.work.phase.committed > 0).then_some(relative[0]);
        let mut ev = PhaseAtlasEvent {
            from: mode,
            target,
            reset_center: next.center,
            event: PhaseEvent {
                epoch: out.epochs.len() - 1,
                phase_index: idx,
                guard: j,
                relative,
                absolute: absolute.as_pair(),
                width_upper: width,
                dwell_lower: dwell,
                committed: false,
                reason: None,
            },
            reset_guards: Vec::with_capacity(modes[target].guards.len()),
        };
        let refusal = if absolute.lo > times[1] {
            Some(PhaseAtlasStop::OrderUnresolved)
        } else if absolute.hi >= times[1] {
            Some(PhaseAtlasStop::HorizonAmbiguous)
        } else if width > b.absolute_width {
            Some(PhaseAtlasStop::AbsoluteWidth)
        } else if dwell.is_some_and(|v| v < b.min_dwell) {
            Some(PhaseAtlasStop::DwellUnproved)
        } else if out.work.phase.committed >= b.transitions {
            Some(PhaseAtlasStop::TransitionBudget)
        } else {
            None
        };
        if let Some(k) = refusal {
            ev.event.reason = Some(format!("{k:?}"));
            out.events.push(ev);
            stop!(k, format!("{k:?}"));
        }
        // Even for a cache hit, the actual selected edge is checked
        // against the target mode's guards at the exact reset point.
        let admission = (|| -> Result<(), (PhaseAtlasStop, String)> {
            for g in &modes[target].guards {
                if out.work.phase.reset_checks >= budget.reset_checks {
                    return Err((
                        PhaseAtlasStop::ResetBudget,
                        "atlas reset guard budget".into(),
                    ));
                }
                out.work.phase.reset_checks += 1;
                let result = models[target].simplex_gap_enclosure(
                    g,
                    [0., 0.],
                    [[0.; 2]; 2],
                    next.center,
                    Some(0.),
                    SimplexGapRoute::AffineDual,
                );
                if let Some(a) = audit.as_deref_mut() {
                    a.reset_guards.record(result.is_ok());
                    a.negative_reset_guards +=
                        usize::from(result.as_ref().is_ok_and(|e| e.enclosure[1] < 0.));
                }
                let e = result.map_err(|e| (PhaseAtlasStop::ResetRefused, e.to_string()))?;
                let good = e.enclosure[1] < 0.;
                ev.reset_guards.push(e);
                if !good {
                    return Err((
                        PhaseAtlasStop::ResetRefused,
                        "target guard not strictly negative at exact reset".into(),
                    ));
                }
            }
            Ok(())
        })();
        if let Err((k, e)) = admission {
            ev.event.reason = Some(e.clone());
            out.events.push(ev);
            stop!(k, e);
        }
        ev.event.committed = true;
        out.events.push(ev);
        out.work.phase.committed += 1;
        if let Some(a) = audit.as_deref_mut() {
            a.committed += 1;
        }
        clock = absolute;
        family = next;
        mode = target;
        out.final_mode = mode;
    }
}
