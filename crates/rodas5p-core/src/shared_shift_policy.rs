//! Opt-in research comparison of ways to solve a family of positive real
//! shifted systems `(I - gamma_i h J) x = b_j` for one current dissipative
//! real `J` (RVJ DAG nodes PP04 and PP03, research nodes
//! `research/pp04_shared_shift_policy_20261004` and
//! `research/pp03_rhs_compression_20261004`).
//!
//! Four methods produce candidates: one LU per target, one LU per distinct
//! shift, a Householder Hessenberg reduction reused for every shift, and the
//! shared shift jet of [`crate::shared_shift_jet`] on clusters of shifts.
//! Every candidate is accepted only by the directed current-target residual
//! of [`crate::shared_shift_jet::certify_shift_candidate`] (or the jet's
//! identical internal certificate). Work is counted in binary64 flops by
//! the executing code paths (an opaque LU by its standard count) and the
//! directed residual work separately; it is not time and no speed claim
//! follows from it. Nothing here selects a solver for the integrators.
// Dense index loops over rows and columns read more plainly here.
#![allow(clippy::needless_range_loop)]

use serde::Serialize;

use crate::shared_shift_jet::{
    CertificateStatus, SharedShiftJetConfig, certify_shift_candidate, shared_shift_jet,
};
use crate::{CoreError, CoreResult, DenseMatrix, LuFactorization};

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("shared shift policy: {message}"))
}

/// Largest normalized cluster radius the planner forms.
pub const MAX_CLUSTER_RADIUS: f64 = 0.5;
/// Largest jet degree the planner asks for (the jet's own cap).
pub const MAX_PLANNED_DEGREE: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShiftMethod {
    IndividualLu,
    CommonShiftLu,
    HessenbergReuse,
    SharedJet,
}

pub const SHIFT_METHODS: [ShiftMethod; 4] = [
    ShiftMethod::IndividualLu,
    ShiftMethod::CommonShiftLu,
    ShiftMethod::HessenbergReuse,
    ShiftMethod::SharedJet,
];

/// Why the planner does not consider the jet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JetAbstention {
    /// Every target has the same shift: one LU serves all.
    CommonShift,
    /// More clusters than half the distinct shifts.
    WideCluster,
}

/// Counted work. `flops` are binary64 operations; `directed` counts
/// directed matrix-entry products of residual certificates and witnesses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ShiftWork {
    pub flops: u64,
    pub directed: u64,
    pub factorizations: u64,
    pub hessenberg_reductions: u64,
    pub column_solves: u64,
    pub qr_factorizations: u64,
    pub jet_levels: u64,
    pub fallback_targets: u64,
}

impl ShiftWork {
    fn add(&mut self, other: ShiftWork) {
        self.flops += other.flops;
        self.directed += other.directed;
        self.factorizations += other.factorizations;
        self.hessenberg_reductions += other.hessenberg_reductions;
        self.column_solves += other.column_solves;
        self.qr_factorizations += other.qr_factorizations;
        self.jet_levels += other.jet_levels;
        self.fallback_targets += other.fallback_targets;
    }
}

fn lu_flops(n: u64) -> u64 {
    2 * n * n * n / 3
}

fn hessenberg_flops(n: u64) -> u64 {
    // Householder reduction 10n^3/3 plus accumulating Q 4n^3/3.
    14 * n * n * n / 3
}

/// One cluster of the plan: target indices, center and degree.
#[derive(Clone, Debug, Serialize)]
pub struct ShiftCluster {
    pub targets: Vec<usize>,
    pub gamma0: f64,
    pub rho: f64,
    pub degree: usize,
}

/// The prospective plan: predicted counted flops of each method (None when
/// the jet abstains), the choice, and the jet clusters.
#[derive(Clone, Debug, Serialize)]
pub struct ShiftPlan {
    pub predicted_flops: Vec<(ShiftMethod, Option<u64>)>,
    pub chosen: ShiftMethod,
    pub jet_abstention: Option<JetAbstention>,
    /// `r (d+1) >= cluster size` in every cluster: a coarse screen,
    /// reported, not used to decide.
    pub high_rank_screen: bool,
    pub clusters: Vec<ShiftCluster>,
}

fn distinct_shifts(gammas: &[f64]) -> Vec<f64> {
    let mut d: Vec<f64> = gammas.to_vec();
    d.sort_by(f64::total_cmp);
    d.dedup_by(|a, b| a.to_bits() == b.to_bits());
    d
}

fn planned_degree(rho: f64, max_column_norm: f64, tolerance: f64) -> Option<usize> {
    if rho == 0.0 {
        return Some(0);
    }
    // Least d with rho^(d+1) / (1 - rho) * ||b|| <= tol / 2.
    let mut tail = rho / (1.0 - rho) * max_column_norm;
    for d in 0..=MAX_PLANNED_DEGREE {
        if tail <= 0.5 * tolerance {
            return Some(d);
        }
        tail *= rho;
    }
    None
}

fn clusters_for(
    sorted_distinct: &[f64],
    max_column_norm: f64,
    tolerance: f64,
) -> Vec<(f64, f64, f64, usize)> {
    // Greedy: extend while hi <= 3 lo (radius <= 1/2 about the midpoint),
    // then halve any cluster whose degree would exceed the cap.
    let mut groups: Vec<Vec<f64>> = Vec::new();
    for &g in sorted_distinct {
        match groups.last_mut() {
            Some(group) if g <= 3.0 * group[0] => group.push(g),
            _ => groups.push(vec![g]),
        }
    }
    let mut out = Vec::new();
    let mut stack: Vec<Vec<f64>> = groups.into_iter().rev().collect();
    while let Some(group) = stack.pop() {
        let (lo, hi) = (group[0], group[group.len() - 1]);
        let gamma0 = 0.5 * lo + 0.5 * hi;
        let rho = (hi - lo) / (hi + lo);
        match planned_degree(rho, max_column_norm, tolerance) {
            Some(d) => out.push((lo, hi, gamma0, d)),
            None => {
                let mid = group.len() / 2;
                stack.push(group[mid..].to_vec());
                stack.push(group[..mid].to_vec());
            }
        }
    }
    out
}

/// Predicted flops of every method from sizes, shifts and column norms
/// only, and the cheapest method.
pub fn plan_shift_family(
    n: usize,
    r: usize,
    gammas: &[f64],
    max_column_norm: f64,
    tolerance: f64,
) -> CoreResult<ShiftPlan> {
    if n == 0 || r == 0 || gammas.is_empty() {
        return Err(invalid("sizes and targets must be nonempty"));
    }
    if !gammas.iter().all(|g| g.is_finite() && *g > 0.0) {
        return Err(invalid("shifts must be finite positive"));
    }
    if !(max_column_norm.is_finite() && tolerance.is_finite() && tolerance > 0.0) {
        return Err(invalid(
            "norm and tolerance must be finite, tolerance positive",
        ));
    }
    let (n64, r64, m64) = (n as u64, r as u64, gammas.len() as u64);
    let distinct = distinct_shifts(gammas);
    let k64 = distinct.len() as u64;
    let nn = n64 * n64;
    let build = 2 * nn;
    let solve = 2 * nn;
    let individual = m64 * (build + lu_flops(n64) + r64 * solve);
    let common = k64 * (build + lu_flops(n64) + r64 * solve);
    // Hessenberg: reduction once, Q^T b once per column, per distinct shift
    // the Hessenberg build (2n^2) and LU (n^2), per target and column the
    // Hessenberg solve (3n^2) and Q y (2n^2).
    let hessenberg =
        hessenberg_flops(n64) + r64 * 2 * nn + k64 * (2 * nn + nn) + m64 * r64 * (3 * nn + 2 * nn);
    let raw = clusters_for(&distinct, max_column_norm, tolerance);
    let mut clusters = Vec::with_capacity(raw.len());
    for (lo, hi, gamma0, degree) in &raw {
        let targets: Vec<usize> = gammas
            .iter()
            .enumerate()
            .filter(|(_, g)| **g >= *lo && **g <= *hi)
            .map(|(i, _)| i)
            .collect();
        clusters.push(ShiftCluster {
            targets,
            gamma0: *gamma0,
            rho: (hi - lo) / (hi + lo),
            degree: *degree,
        });
    }
    let jet_abstention = if distinct.len() == 1 {
        Some(JetAbstention::CommonShift)
    } else if 2 * clusters.len() > distinct.len() {
        Some(JetAbstention::WideCluster)
    } else {
        None
    };
    let high_rank_screen = clusters
        .iter()
        .all(|c| r * (c.degree + 1) >= c.targets.len());
    let jet = jet_abstention.is_none().then(|| {
        clusters
            .iter()
            .map(|c| {
                let levels = c.degree as u64 + 1;
                let s = c.targets.len() as u64;
                build
                    + lu_flops(n64)
                    + levels * r64 * solve
                    + c.degree as u64 * r64 * n64
                    + s * r64 * 2 * n64 * c.degree as u64
            })
            .sum::<u64>()
    });
    let predicted = vec![
        (ShiftMethod::IndividualLu, Some(individual)),
        (ShiftMethod::CommonShiftLu, Some(common)),
        (ShiftMethod::HessenbergReuse, Some(hessenberg)),
        (ShiftMethod::SharedJet, jet),
    ];
    let chosen = predicted
        .iter()
        .filter_map(|(m, c)| c.map(|c| (*m, c)))
        .min_by_key(|(_, c)| *c)
        .map(|(m, _)| m)
        .expect("three methods always have a prediction");
    Ok(ShiftPlan {
        predicted_flops: predicted,
        chosen,
        jet_abstention,
        high_rank_screen,
        clusters,
    })
}

/// The certified outcome of one target: its candidate columns and the
/// certificate's per-column error upper bounds.
#[derive(Clone, Debug, Serialize)]
pub struct TargetOutcome {
    pub gamma: f64,
    pub columns: Vec<Vec<f64>>,
    pub error_upper: Vec<f64>,
    pub certified: bool,
    /// The method that produced the accepted (or last) candidate.
    pub produced_by: ShiftMethod,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShiftFamilyResult {
    pub method: ShiftMethod,
    pub targets: Vec<TargetOutcome>,
    pub work: ShiftWork,
}

#[allow(clippy::too_many_arguments)]
fn certify(
    j: &DenseMatrix,
    h: f64,
    gamma: f64,
    rhs: &[Vec<f64>],
    columns: Vec<Vec<f64>>,
    tolerance: f64,
    produced_by: ShiftMethod,
    work: &mut ShiftWork,
) -> CoreResult<TargetOutcome> {
    let n = j.nrows() as u64;
    let certificate = certify_shift_candidate(j, h, gamma, rhs, &columns, tolerance)?;
    // Witness (n^2) and one residual (n^2) per column.
    work.directed += n * n + rhs.len() as u64 * n * n;
    Ok(TargetOutcome {
        gamma,
        error_upper: certificate.rhs_error_upper().to_vec(),
        certified: certificate.status() == CertificateStatus::Certified,
        columns,
        produced_by,
    })
}

fn shifted(j: &DenseMatrix, h: f64, gamma: f64, work: &mut ShiftWork) -> CoreResult<DenseMatrix> {
    let n = j.nrows();
    let scale = gamma * h;
    let mut a = DenseMatrix::identity(n);
    for i in 0..n {
        for k in 0..n {
            a[(i, k)] -= scale * j[(i, k)];
        }
    }
    work.flops += 2 * (n * n) as u64;
    if !a.as_slice().iter().all(|v| v.is_finite()) {
        return Err(invalid("shifted matrix overflow"));
    }
    Ok(a)
}

fn lu_solve_all(
    j: &DenseMatrix,
    h: f64,
    gamma: f64,
    rhs: &[Vec<f64>],
    work: &mut ShiftWork,
) -> CoreResult<Vec<Vec<f64>>> {
    let n = j.nrows() as u64;
    let a = shifted(j, h, gamma, work)?;
    let lu = LuFactorization::new(&a)?;
    work.factorizations += 1;
    work.flops += lu_flops(n);
    let x = lu.solve_rows(rhs)?;
    work.column_solves += rhs.len() as u64;
    work.flops += rhs.len() as u64 * 2 * n * n;
    Ok(x)
}

/// `J = Q H Q^T` by Householder reflections: `H` upper Hessenberg (entries
/// below the subdiagonal set to zero), `Q` orthogonal up to rounding.
/// Row-major `Vec<Vec<f64>>`.
pub fn hessenberg_reduce(j: &DenseMatrix) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let n = j.nrows();
    let mut a: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|k| j[(i, k)]).collect())
        .collect();
    let mut q: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|k| if i == k { 1.0 } else { 0.0 }).collect())
        .collect();
    for k in 0..n.saturating_sub(2) {
        let norm = (k + 1..n).map(|i| a[i][k] * a[i][k]).sum::<f64>().sqrt();
        if norm == 0.0 {
            continue;
        }
        let alpha = if a[k + 1][k] >= 0.0 { -norm } else { norm };
        let mut v = vec![0.0; n];
        v[k + 1] = a[k + 1][k] - alpha;
        for i in k + 2..n {
            v[i] = a[i][k];
        }
        let vnorm2 = (k + 1..n).map(|i| v[i] * v[i]).sum::<f64>();
        if vnorm2 == 0.0 {
            continue;
        }
        // A <- (I - 2 v v^T / v^T v) A (I - 2 v v^T / v^T v); Q <- Q P.
        for col in 0..n {
            let dot = (k + 1..n).map(|i| v[i] * a[i][col]).sum::<f64>() * 2.0 / vnorm2;
            for i in k + 1..n {
                a[i][col] -= dot * v[i];
            }
        }
        for row in a.iter_mut() {
            let dot = (k + 1..n).map(|i| row[i] * v[i]).sum::<f64>() * 2.0 / vnorm2;
            for i in k + 1..n {
                row[i] -= dot * v[i];
            }
        }
        for row in q.iter_mut() {
            let dot = (k + 1..n).map(|i| row[i] * v[i]).sum::<f64>() * 2.0 / vnorm2;
            for i in k + 1..n {
                row[i] -= dot * v[i];
            }
        }
        for row in a.iter_mut().skip(k + 2) {
            row[k] = 0.0;
        }
    }
    (q, a)
}

/// LU with partial pivoting of the upper Hessenberg `I - s H` and solves.
struct HessenbergLu {
    u: Vec<Vec<f64>>,
    multipliers: Vec<f64>,
    swapped: Vec<bool>,
}

impl HessenbergLu {
    fn new(hess: &[Vec<f64>], s: f64) -> CoreResult<Self> {
        let n = hess.len();
        let mut u: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|k| {
                        let identity = if i == k { 1.0 } else { 0.0 };
                        if k + 1 >= i {
                            identity - s * hess[i][k]
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect();
        let mut multipliers = vec![0.0; n.saturating_sub(1)];
        let mut swapped = vec![false; n.saturating_sub(1)];
        for k in 0..n.saturating_sub(1) {
            if u[k + 1][k].abs() > u[k][k].abs() {
                u.swap(k, k + 1);
                swapped[k] = true;
            }
            if u[k][k] == 0.0 {
                return Err(invalid("singular Hessenberg shifted matrix"));
            }
            let l = u[k + 1][k] / u[k][k];
            multipliers[k] = l;
            for col in k..n {
                let pivot_value = u[k][col];
                u[k + 1][col] -= l * pivot_value;
            }
        }
        if n > 0 && u[n - 1][n - 1] == 0.0 {
            return Err(invalid("singular Hessenberg shifted matrix"));
        }
        Ok(Self {
            u,
            multipliers,
            swapped,
        })
    }

    fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = b.len();
        let mut y = b.to_vec();
        for k in 0..n.saturating_sub(1) {
            if self.swapped[k] {
                y.swap(k, k + 1);
            }
            y[k + 1] -= self.multipliers[k] * y[k];
        }
        for i in (0..n).rev() {
            let mut s = y[i];
            for k in i + 1..n {
                s -= self.u[i][k] * y[k];
            }
            y[i] = s / self.u[i][i];
        }
        y
    }
}

fn mat_vec(m: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
    m.iter()
        .map(|row| row.iter().zip(x).map(|(a, b)| a * b).sum())
        .collect()
}

fn mat_t_vec(m: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
    let n = x.len();
    (0..n)
        .map(|k| (0..n).map(|i| m[i][k] * x[i]).sum())
        .collect()
}

struct HessenbergReuse {
    q: Vec<Vec<f64>>,
    hess: Vec<Vec<f64>>,
    qt_rhs: Vec<Vec<f64>>,
}

impl HessenbergReuse {
    fn new(j: &DenseMatrix, rhs: &[Vec<f64>], work: &mut ShiftWork) -> Self {
        let n = j.nrows() as u64;
        let (q, hess) = hessenberg_reduce(j);
        work.hessenberg_reductions += 1;
        work.flops += hessenberg_flops(n);
        let qt_rhs = rhs.iter().map(|b| mat_t_vec(&q, b)).collect();
        work.flops += rhs.len() as u64 * 2 * n * n;
        Self { q, hess, qt_rhs }
    }

    fn solve(&self, h: f64, gamma: f64, work: &mut ShiftWork) -> CoreResult<Vec<Vec<f64>>> {
        let n = self.q.len() as u64;
        let lu = HessenbergLu::new(&self.hess, gamma * h)?;
        work.factorizations += 1;
        work.flops += 3 * n * n;
        let mut out = Vec::with_capacity(self.qt_rhs.len());
        for c in &self.qt_rhs {
            let y = lu.solve(c);
            let x = mat_vec(&self.q, &y);
            if !x.iter().all(|v| v.is_finite()) {
                return Err(invalid("Hessenberg candidate overflow"));
            }
            out.push(x);
        }
        work.column_solves += self.qt_rhs.len() as u64;
        work.flops += self.qt_rhs.len() as u64 * 5 * n * n;
        Ok(out)
    }
}

/// Execute one method on the whole family and certify every target.
pub fn solve_shift_family(
    method: ShiftMethod,
    j: &DenseMatrix,
    h: f64,
    rhs: &[Vec<f64>],
    gammas: &[f64],
    tolerance: f64,
) -> CoreResult<ShiftFamilyResult> {
    let max_norm = rhs
        .iter()
        .map(|b| crate::safe_l2(b))
        .fold(0.0_f64, f64::max);
    let plan = plan_shift_family(j.nrows(), rhs.len(), gammas, max_norm, tolerance)?;
    let mut work = ShiftWork::default();
    let mut targets: Vec<Option<TargetOutcome>> = vec![None; gammas.len()];
    match method {
        ShiftMethod::IndividualLu => {
            for (i, &gamma) in gammas.iter().enumerate() {
                let x = lu_solve_all(j, h, gamma, rhs, &mut work)?;
                targets[i] = Some(certify(j, h, gamma, rhs, x, tolerance, method, &mut work)?);
            }
        }
        ShiftMethod::CommonShiftLu => {
            for gamma in distinct_shifts(gammas) {
                let x = lu_solve_all(j, h, gamma, rhs, &mut work)?;
                for (i, g) in gammas.iter().enumerate() {
                    if g.to_bits() == gamma.to_bits() {
                        targets[i] = Some(certify(
                            j,
                            h,
                            gamma,
                            rhs,
                            x.clone(),
                            tolerance,
                            method,
                            &mut work,
                        )?);
                    }
                }
            }
        }
        ShiftMethod::HessenbergReuse => {
            let reuse = HessenbergReuse::new(j, rhs, &mut work);
            for gamma in distinct_shifts(gammas) {
                let factor_flops = 3 * (j.nrows() * j.nrows()) as u64;
                let mut first = true;
                for (i, g) in gammas.iter().enumerate() {
                    if g.to_bits() == gamma.to_bits() {
                        let mut local = ShiftWork::default();
                        let x = reuse.solve(h, gamma, &mut local)?;
                        // One Hessenberg LU per distinct shift is charged;
                        // repeated targets reuse it.
                        if !first {
                            local.flops -= factor_flops;
                            local.factorizations -= 1;
                        }
                        first = false;
                        work.add(local);
                        targets[i] =
                            Some(certify(j, h, gamma, rhs, x, tolerance, method, &mut work)?);
                    }
                }
            }
        }
        ShiftMethod::SharedJet => {
            let n = j.nrows() as u64;
            let mut reuse: Option<HessenbergReuse> = None;
            for cluster in &plan.clusters {
                let cluster_gammas: Vec<f64> = cluster.targets.iter().map(|&i| gammas[i]).collect();
                let report = shared_shift_jet(
                    j,
                    h,
                    cluster.gamma0,
                    rhs,
                    &cluster_gammas,
                    SharedShiftJetConfig {
                        degree: cluster.degree,
                        absolute_tolerance: tolerance,
                        max_stored_scalars: usize::MAX,
                        max_work_units: usize::MAX,
                    },
                )?;
                let jw = report.work();
                let r64 = rhs.len() as u64;
                let s64 = cluster_gammas.len() as u64;
                work.factorizations += 1;
                work.jet_levels += jw.solve_batches as u64;
                work.column_solves += jw.rhs_solves as u64;
                work.flops += 2 * n * n
                    + lu_flops(n)
                    + jw.rhs_solves as u64 * 2 * n * n
                    + jw.recurrence_depth as u64 * r64 * n
                    + 2 * jw.evaluation_multiply_add_pairs as u64;
                work.directed += n * n + s64 * r64 * n * n;
                for (&i, candidate) in cluster.targets.iter().zip(report.candidates()) {
                    let certificate = candidate.certificate();
                    let certified = certificate.status() == CertificateStatus::Certified;
                    targets[i] = Some(if certified {
                        TargetOutcome {
                            gamma: gammas[i],
                            columns: candidate.rhs_columns().to_vec(),
                            error_upper: certificate.rhs_error_upper().to_vec(),
                            certified,
                            produced_by: ShiftMethod::SharedJet,
                        }
                    } else {
                        // Fallback to the Hessenberg reuse; both attempts charged.
                        work.fallback_targets += 1;
                        let reuse = match &mut reuse {
                            Some(r) => r,
                            slot @ None => slot.insert(HessenbergReuse::new(j, rhs, &mut work)),
                        };
                        let x = reuse.solve(h, gammas[i], &mut work)?;
                        certify(
                            j,
                            h,
                            gammas[i],
                            rhs,
                            x,
                            tolerance,
                            ShiftMethod::HessenbergReuse,
                            &mut work,
                        )?
                    });
                }
            }
        }
    }
    Ok(ShiftFamilyResult {
        method,
        targets: targets
            .into_iter()
            .map(|t| t.ok_or_else(|| invalid("a target was not produced")))
            .collect::<CoreResult<_>>()?,
        work,
    })
}

/// Column-pivoted Householder QR of the supplied columns (PP03): `Q` (k
/// orthonormal columns up to rounding), `C = Q^T B` (k x M, one column per
/// supplied RHS) and the numerical rank `k` (`|R_ii| > 1e-13 |R_11|`).
/// Flops are counted in the loops.
#[derive(Clone, Debug, Serialize)]
pub struct RhsCompression {
    pub q_columns: Vec<Vec<f64>>,
    pub coefficients: Vec<Vec<f64>>,
    pub rank: usize,
    pub flops: u64,
}

pub const COMPRESSION_RANK_TOLERANCE: f64 = 1.0e-13;

pub fn compress_rhs(rhs: &[Vec<f64>]) -> CoreResult<RhsCompression> {
    let m = rhs.len();
    if m == 0 {
        return Err(invalid("no RHS columns"));
    }
    let n = rhs[0].len();
    if n == 0
        || rhs
            .iter()
            .any(|b| b.len() != n || !b.iter().all(|v| v.is_finite()))
    {
        return Err(invalid(
            "RHS columns must be finite, nonempty and of one length",
        ));
    }
    let mut flops = 0_u64;
    let mut a: Vec<Vec<f64>> = rhs.to_vec(); // columns
    let mut perm: Vec<usize> = (0..m).collect();
    let mut reflectors: Vec<(usize, Vec<f64>, f64)> = Vec::new();
    let mut r11 = 0.0_f64;
    let mut rank = 0;
    for k in 0..m.min(n) {
        // Pivot: the remaining column with the largest trailing norm.
        let norms: Vec<f64> = (k..m)
            .map(|c| a[c][k..].iter().map(|v| v * v).sum::<f64>())
            .collect();
        flops += 2 * ((m - k) * (n - k)) as u64;
        let (offset, best) =
            norms.iter().enumerate().fold(
                (0, -1.0),
                |acc, (i, v)| if *v > acc.1 { (i, *v) } else { acc },
            );
        a.swap(k, k + offset);
        perm.swap(k, k + offset);
        let norm = best.sqrt();
        if k == 0 {
            r11 = norm;
        }
        if norm == 0.0 || norm <= COMPRESSION_RANK_TOLERANCE * r11 {
            break;
        }
        let alpha = if a[k][k] >= 0.0 { -norm } else { norm };
        let mut v = a[k][k..].to_vec();
        v[0] -= alpha;
        let vnorm2 = v.iter().map(|x| x * x).sum::<f64>();
        flops += 2 * (n - k) as u64;
        if vnorm2 > 0.0 {
            for col in a.iter_mut().skip(k) {
                let dot = col[k..].iter().zip(&v).map(|(x, y)| x * y).sum::<f64>() * 2.0 / vnorm2;
                for (x, y) in col[k..].iter_mut().zip(&v) {
                    *x -= dot * y;
                }
            }
            flops += 4 * ((m - k) * (n - k)) as u64;
        }
        reflectors.push((k, v, vnorm2));
        rank += 1;
    }
    // Q = P_1 ... P_k applied to the first k unit vectors.
    let mut q_columns = Vec::with_capacity(rank);
    for c in 0..rank {
        let mut e = vec![0.0; n];
        e[c] = 1.0;
        for (k, v, vnorm2) in reflectors.iter().rev() {
            if *vnorm2 == 0.0 {
                continue;
            }
            let dot = e[*k..].iter().zip(v).map(|(x, y)| x * y).sum::<f64>() * 2.0 / vnorm2;
            for (x, y) in e[*k..].iter_mut().zip(v) {
                *x -= dot * y;
            }
            flops += 4 * (n - k) as u64;
        }
        q_columns.push(e);
    }
    // C = Q^T B for the original column order.
    let coefficients: Vec<Vec<f64>> = rhs
        .iter()
        .map(|b| {
            q_columns
                .iter()
                .map(|q| q.iter().zip(b).map(|(x, y)| x * y).sum())
                .collect()
        })
        .collect();
    flops += 2 * (n * rank * m) as u64;
    let _ = perm;
    Ok(RhsCompression {
        q_columns,
        coefficients,
        rank,
        flops,
    })
}

/// The jet on compressed columns (PP03): one cluster around `gamma0`, the
/// jet built on the `k` columns of `Q`, each target's outputs
/// reconstructed as `U_Q c_j` and certified against the **original**
/// `b_j`. `None` (typed abstention `HighRank`) when `k` equals the number
/// of supplied columns.
pub fn compressed_shift_jet(
    j: &DenseMatrix,
    h: f64,
    gamma0: f64,
    rhs: &[Vec<f64>],
    gammas: &[f64],
    degree: usize,
    tolerance: f64,
) -> CoreResult<Option<(RhsCompression, ShiftFamilyResult)>> {
    let compression = compress_rhs(rhs)?;
    if compression.rank == rhs.len() || compression.rank == 0 {
        return Ok(None);
    }
    let n = j.nrows() as u64;
    let mut work = ShiftWork {
        qr_factorizations: 1,
        flops: compression.flops,
        ..ShiftWork::default()
    };
    let report = shared_shift_jet(
        j,
        h,
        gamma0,
        &compression.q_columns,
        gammas,
        SharedShiftJetConfig {
            degree,
            absolute_tolerance: tolerance,
            max_stored_scalars: usize::MAX,
            max_work_units: usize::MAX,
        },
    )?;
    let jw = report.work();
    let k64 = compression.rank as u64;
    let s64 = gammas.len() as u64;
    work.factorizations += 1;
    work.jet_levels += jw.solve_batches as u64;
    work.column_solves += jw.rhs_solves as u64;
    work.flops += 2 * n * n
        + lu_flops(n)
        + jw.rhs_solves as u64 * 2 * n * n
        + jw.recurrence_depth as u64 * k64 * n
        + 2 * jw.evaluation_multiply_add_pairs as u64;
    // The jet's internal certificates of the Q columns are charged too.
    work.directed += n * n + s64 * k64 * n * n;
    let mut targets = Vec::with_capacity(gammas.len());
    for (candidate, &gamma) in report.candidates().iter().zip(gammas) {
        let uq = candidate.rhs_columns();
        let columns: Vec<Vec<f64>> = compression
            .coefficients
            .iter()
            .map(|c| {
                let mut x = vec![0.0; j.nrows()];
                for (u, coefficient) in uq.iter().zip(c) {
                    for (xi, ui) in x.iter_mut().zip(u) {
                        *xi = coefficient.mul_add(*ui, *xi);
                    }
                }
                x
            })
            .collect();
        work.flops += 2 * k64 * n * rhs.len() as u64;
        targets.push(certify(
            j,
            h,
            gamma,
            rhs,
            columns,
            tolerance,
            ShiftMethod::SharedJet,
            &mut work,
        )?);
    }
    Ok(Some((
        compression,
        ShiftFamilyResult {
            method: ShiftMethod::SharedJet,
            targets,
            work,
        },
    )))
}
