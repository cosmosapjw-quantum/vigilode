//! Opt-in C5 source-owned graph-Schur / affine path forest W stage solver.
//! Original RODAS5P stage, never a backward-Euler nonlinear root.
//! Unscaled `W = inv_h_gamma I - J`.  Source owner is immutable, with an epoch.
//! Unsupported graph is refused at construction; numerical candidate failure
//! delegates to the existing BandedStageLu (full-band, original W) and charges work.
use crate::{BandedJacobian, BandedStageLu, OdeProblem, StageLinearSystem};
use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct CooperativeCubicOwner {
    pub name: String,
    pub epoch: u64,
    pub k: Vec<f64>,
    pub g: Vec<f64>,
    pub c: Vec<Vec<f64>>,
    pub feedback: Vec<usize>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct GraphSchurTelemetry {
    pub source_rhs_probes: u64,
    pub linearizations: u64,
    pub factorization_attempts: u64,
    pub certified_stage_solves: u64,
    pub fallback_stage_solves: u64,
    pub fallback_factorizations: u64,
    pub original_residual_checks: u64,
    pub affine_pair_compositions: u64,
    pub graph_schur_small_core_solves: u64,
    pub candidate_failure_count: u64,
    pub max_stage_residual_bound: f64,
    pub original_stage_identity: &'static str,
    pub fallback_identity: &'static str,
}

#[derive(Clone)]
struct GraphChains {
    chains: Vec<Vec<usize>>,
    a: Vec<usize>,
    b: Vec<usize>,
    n: usize,
}
impl GraphChains {
    fn validate(owner: &CooperativeCubicOwner) -> CoreResult<Self> {
        let n = owner.k.len();
        if n == 0 || owner.g.len() != n || owner.c.len() != n || owner.feedback.len() > 3 {
            return Err(CoreError::InvalidInput(
                "C5 graph source dimension/core unsupported".into(),
            ));
        }
        if owner.k.iter().any(|v| !v.is_finite() || *v <= 0.)
            || owner.g.iter().any(|v| !v.is_finite() || *v < 0.)
            || owner.c.iter().enumerate().any(|(i, row)| {
                row.len() != n || row[i] != 0. || row.iter().any(|v| !v.is_finite() || *v < 0.)
            })
        {
            return Err(CoreError::InvalidInput(
                "C5 requires immutable cooperative dissipative cubic source".into(),
            ));
        }
        let mut seen = vec![false; n];
        for &i in &owner.feedback {
            if i >= n || seen[i] {
                return Err(CoreError::InvalidInput(
                    "C5 duplicate/invalid feedback vertex".into(),
                ));
            }
            seen[i] = true;
        }
        let a = (0..n).filter(|&i| !seen[i]).collect::<Vec<_>>();
        let mut parent = vec![None; n];
        let mut child = vec![None; n];
        for &i in &a {
            for &j in &a {
                if owner.c[i][j] != 0. {
                    if parent[i].replace(j).is_some() {
                        return Err(CoreError::InvalidInput(
                            "C5 NOT_PATH_FOREST_MULTI_PARENT".into(),
                        ));
                    }
                    if child[j].replace(i).is_some() {
                        return Err(CoreError::InvalidInput("C5 NOT_PATH_FOREST_BRANCH".into()));
                    }
                }
            }
        }
        let mut covered = vec![false; n];
        let mut chains = Vec::new();
        for &root in &a {
            if parent[root].is_none() {
                let mut chain = Vec::new();
                let mut cur = Some(root);
                while let Some(i) = cur {
                    if covered[i] {
                        return Err(CoreError::InvalidInput("C5 graph cycle not broken".into()));
                    }
                    covered[i] = true;
                    chain.push(i);
                    cur = child[i];
                }
                chains.push(chain);
            }
        }
        if a.iter().any(|&i| !covered[i]) {
            return Err(CoreError::InvalidInput("C5 graph cycle not broken".into()));
        }
        Ok(Self {
            chains,
            a,
            b: owner.feedback.clone(),
            n,
        })
    }
}

/// The explicit client must be created with immutable source coefficients and
/// only paired with its matching OdeProblem.  Callbacks for the physical RHS/JVP
/// are independent and their binding is checked at every new linearization.
pub struct GraphSchurStageClient {
    owner: Arc<CooperativeCubicOwner>,
    graph: GraphChains,
    telemetry: Arc<Mutex<GraphSchurTelemetry>>,
    protected: BandedStageLu,
    saved_problem: Option<OdeProblem>,
    t: f64,
    y: Vec<f64>,
    inv_h_gamma: f64,
    diag: Vec<f64>,
    t_matrix: Vec<f64>,
    columns: Vec<Vec<f64>>,
    small_inverse: Vec<f64>,
    ready: bool,
    fallback_ready: bool,
    last_stage_bound: f64,
}

impl GraphSchurStageClient {
    pub fn new(
        owner: CooperativeCubicOwner,
        telemetry: Arc<Mutex<GraphSchurTelemetry>>,
    ) -> CoreResult<Self> {
        let graph = GraphChains::validate(&owner)?;
        let n = graph.n;
        let frozen = Arc::new(owner);
        let frozen_band = Arc::clone(&frozen);
        let width = 2 * n - 1;
        let band = BandedJacobian {
            lower: n - 1,
            upper: n - 1,
            fill: Arc::new(move |_t, y, buf| {
                if y.len() != n || buf.len() != n * width {
                    return Err(CoreError::Dimension("C5 fallback band shape".into()));
                }
                buf.fill(0.);
                for i in 0..n {
                    let row_sum = frozen_band.c[i].iter().copied().sum::<f64>();
                    for j in 0..n {
                        let value = if i == j {
                            -(frozen_band.k[i] + row_sum + 3. * frozen_band.g[i] * y[i] * y[i])
                        } else {
                            frozen_band.c[i][j]
                        };
                        buf[i * width + j + n - 1 - i] = value;
                    }
                }
                Ok(())
            }),
        };
        let protected = BandedStageLu::new(n, &band)?;
        let mut guard = telemetry
            .lock()
            .map_err(|_| CoreError::InvalidInput("C5 telemetry lock poisoned".into()))?;
        *guard = GraphSchurTelemetry {
            original_stage_identity: "original-eight-stage-W",
            fallback_identity: "protected-original-W-full-band-LU",
            ..GraphSchurTelemetry::default()
        };
        drop(guard);
        Ok(Self {
            owner: frozen,
            graph,
            telemetry,
            protected,
            saved_problem: None,
            t: 0.,
            y: Vec::new(),
            inv_h_gamma: 0.,
            diag: vec![0.; n],
            t_matrix: vec![0.; n * n],
            columns: Vec::new(),
            small_inverse: Vec::new(),
            ready: false,
            fallback_ready: false,
            last_stage_bound: 0.,
        })
    }
    fn bump(&self, f: impl FnOnce(&mut GraphSchurTelemetry)) {
        if let Ok(mut t) = self.telemetry.lock() {
            f(&mut t);
        }
    }
    pub fn telemetry(&self) -> GraphSchurTelemetry {
        self.telemetry.lock().map(|x| x.clone()).unwrap_or_default()
    }
    fn analytic_rhs(&self, y: &[f64], out: &mut [f64]) {
        let n = self.graph.n;
        for i in 0..n {
            let mut v = -self.owner.k[i] * y[i] - self.owner.g[i] * y[i] * y[i] * y[i];
            for j in 0..n {
                if i != j {
                    v += self.owner.c[i][j] * (y[j] - y[i]);
                }
            }
            out[i] = v;
        }
    }
    fn scan_into(&self, z: &[f64], out: &mut [f64]) -> u64 {
        out.fill(0.);
        let mut total = 0;
        for chain in &self.graph.chains {
            if chain.is_empty() {
                continue;
            }
            let mut pair = Vec::with_capacity(chain.len());
            for (i, &vertex) in chain.iter().enumerate() {
                let w = if i == 0 {
                    0.
                } else {
                    self.t_matrix[vertex * self.graph.n + chain[i - 1]]
                };
                pair.push((w, z[vertex]));
            }
            let mut stride = 1;
            while stride < chain.len() {
                let prev = pair.clone();
                for i in stride..chain.len() {
                    let (a, b) = prev[i];
                    let (c, d) = prev[i - stride];
                    pair[i] = (a * c, b + a * d);
                    total += 1;
                }
                stride *= 2;
            }
            for (i, &vertex) in chain.iter().enumerate() {
                out[vertex] = pair[i].1;
            }
        }
        total
    }
    fn invert_core(s: &mut [f64], n: usize) -> Option<Vec<f64>> {
        if n == 0 {
            return Some(Vec::new());
        }
        let mut inv = vec![0.; n * n];
        for i in 0..n {
            inv[i * n + i] = 1.;
        }
        for k in 0..n {
            let mut p = k;
            for i in k + 1..n {
                if s[i * n + k].abs() > s[p * n + k].abs() {
                    p = i;
                }
            }
            let pivot = s[p * n + k];
            if !pivot.is_finite() || pivot.abs() < 32. * f64::EPSILON {
                return None;
            }
            if p != k {
                for j in 0..n {
                    s.swap(p * n + j, k * n + j);
                    inv.swap(p * n + j, k * n + j);
                }
            }
            let diag = s[k * n + k];
            for j in 0..n {
                s[k * n + j] /= diag;
                inv[k * n + j] /= diag;
            }
            for i in 0..n {
                if i == k {
                    continue;
                }
                let value = s[i * n + k];
                for j in 0..n {
                    s[i * n + j] -= value * s[k * n + j];
                    inv[i * n + j] -= value * inv[k * n + j];
                }
            }
        }
        if inv.iter().any(|x| !x.is_finite() || *x < -1e-12) {
            return None;
        }
        Some(inv)
    }
    fn compute_factor(&mut self, inv: f64) -> bool {
        let n = self.graph.n;
        self.inv_h_gamma = inv;
        if !(inv.is_finite() && inv > 0.) {
            return false;
        }
        let mut max_q = 0.;
        for i in 0..n {
            let sum = self.owner.c[i].iter().copied().sum::<f64>();
            let d = inv + self.owner.k[i] + sum + 3. * self.owner.g[i] * self.y[i] * self.y[i];
            if !(d.is_finite() && d > 0.) {
                return false;
            }
            self.diag[i] = d;
            for j in 0..n {
                self.t_matrix[i * n + j] = if i == j { 0. } else { self.owner.c[i][j] / d };
            }
            max_q += 0.;
            let q = (sum / d).abs();
            if q > max_q {
                max_q = q;
            }
        }
        if !(max_q < 1.) {
            return false;
        }
        // R_A columns solving T_AB for each retained core vertex.
        self.columns.clear();
        let mut products = 0;
        for &vertex in &self.graph.b {
            let mut z = vec![0.; n];
            for i in 0..n {
                z[i] = self.t_matrix[i * n + vertex];
            }
            let mut col = vec![0.; n];
            products += self.scan_into(&z, &mut col);
            self.columns.push(col);
        }
        let k = self.graph.b.len();
        let mut s = vec![0.; k * k];
        for i in 0..k {
            let v = self.graph.b[i];
            let mut row_z = 0.;
            for j in 0..k {
                let u = self.graph.b[j];
                let mut z = self.t_matrix[v * n + u];
                for &a in &self.graph.a {
                    z += self.t_matrix[v * n + a] * self.columns[j][a];
                }
                if !(z.is_finite() && z >= -1e-14) {
                    return false;
                }
                row_z += z;
                s[i * k + j] = (if i == j { 1. } else { 0. }) - z;
            }
            if !(row_z < 1.) {
                return false;
            }
        }
        let inverse = match Self::invert_core(&mut s, k) {
            Some(v) => v,
            None => return false,
        };
        self.small_inverse = inverse;
        self.bump(|t| {
            t.affine_pair_compositions += products;
        });
        true
    }
    fn candidate(&self, rhs: &[f64]) -> Option<(Vec<f64>, u64, f64)> {
        let n = self.graph.n;
        let k = self.graph.b.len();
        let mut z = vec![0.; n];
        for i in 0..n {
            z[i] = rhs[i] / self.diag[i];
        }
        let mut a = vec![0.; n];
        let mut pairs = self.scan_into(&z, &mut a);
        let mut rb = vec![0.; k];
        for i in 0..k {
            let v = self.graph.b[i];
            rb[i] = z[v];
            for &w in &self.graph.a {
                rb[i] += self.t_matrix[v * n + w] * a[w];
            }
        }
        let mut ub = vec![0.; k];
        for i in 0..k {
            for j in 0..k {
                ub[i] += self.small_inverse[i * k + j] * rb[j];
            }
        }
        let mut out = a;
        for i in 0..k {
            out[self.graph.b[i]] = ub[i];
        }
        for &w in &self.graph.a {
            for j in 0..k {
                out[w] += self.columns[j][w] * ub[j];
            }
        }
        if out.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let mut rho: f64 = 0.;
        let mut norm_wx: f64 = 0.;
        let mut norm_b: f64 = 0.;
        let mut min_margin = f64::INFINITY;
        for i in 0..n {
            let mut wx = self.diag[i] * out[i];
            let sum = self.owner.c[i].iter().copied().sum::<f64>();
            min_margin = min_margin.min(self.diag[i] - sum);
            let mut row_abs = self.diag[i] * out[i].abs();
            for j in 0..n {
                if i != j {
                    wx -= self.owner.c[i][j] * out[j];
                    row_abs += self.owner.c[i][j] * out[j].abs();
                }
            }
            rho = rho.max((wx - rhs[i]).abs());
            norm_wx = norm_wx.max(row_abs);
            norm_b = norm_b.max(rhs[i].abs());
        }
        if !(min_margin.is_finite() && min_margin > 0.) {
            return None;
        }
        let floor = 128. * f64::EPSILON * norm_wx.max(norm_b).max(f64::MIN_POSITIVE);
        if !rho.is_finite() || rho > floor {
            return None;
        }
        let error_bound = rho / min_margin;
        if !error_bound.is_finite() {
            return None;
        }
        pairs += 0;
        Some((out, pairs, error_bound))
    }
    fn prepare_fallback(&mut self, counters: &mut WorkCounters) -> CoreResult<()> {
        if self.fallback_ready {
            return Ok(());
        }
        let p = self.saved_problem.as_ref().ok_or_else(|| {
            CoreError::InvalidInput("C5 missing source linearization owner".into())
        })?;
        self.protected.linearize(p, self.t, &self.y, counters)?;
        self.protected.factor(self.inv_h_gamma, counters)?;
        self.fallback_ready = true;
        self.bump(|t| t.fallback_factorizations += 1);
        Ok(())
    }
}
impl StageLinearSystem for GraphSchurStageClient {
    fn dimension(&self) -> usize {
        self.graph.n
    }
    fn linearize(
        &mut self,
        p: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if p.name != self.owner.name
            || p.dimension != self.graph.n
            || !p.autonomous
            || p.mass_matrix.is_some()
            || p.model_epoch() != Some(self.owner.epoch)
            || y.len() != self.graph.n
            || !t.is_finite()
            || y.iter().any(|x| !x.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "C5 source owner, epoch, clock, mass or shape mismatch".into(),
            ));
        }
        let mut expected = vec![0.; y.len()];
        self.analytic_rhs(y, &mut expected);
        let observed = p.eval_rhs(t, y, counters)?; // charged true callback witness
        for (a, b) in observed.iter().zip(expected.iter()) {
            let threshold = 16. * f64::EPSILON * (a.abs() + b.abs()).max(f64::MIN_POSITIVE);
            if (a - b).abs() > threshold {
                return Err(CoreError::InvalidInput(
                    "C5 source RHS callback identity mismatch".into(),
                ));
            }
        }
        self.t = t;
        self.y = y.to_vec();
        self.saved_problem = Some(p.clone());
        self.ready = false;
        self.fallback_ready = false;
        self.bump(|tel| {
            tel.linearizations += 1;
            tel.source_rhs_probes += 1;
        });
        Ok(())
    }
    fn factor(&mut self, inv: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        if self.saved_problem.is_none() {
            return Err(CoreError::InvalidInput(
                "C5 factor before owner linearize".into(),
            ));
        }
        self.bump(|t| t.factorization_attempts += 1);
        self.ready = self.compute_factor(inv);
        if self.ready {
            counters.direct_factorizations += 1;
            return Ok(());
        }
        self.inv_h_gamma = inv;
        self.prepare_fallback(counters)
    }
    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if b.len() != self.graph.n {
            return Err(CoreError::Dimension("C5 original W stage RHS shape".into()));
        }
        if self.ready {
            counters.diagnostic_matvecs += 1;
            counters.linear_matvec_vectors += 1;
            self.bump(|t| t.original_residual_checks += 1);
            if let Some((x, pairs, bound)) = self.candidate(b) {
                b.copy_from_slice(&x);
                counters.direct_solve_calls += 1;
                self.last_stage_bound = self.last_stage_bound.max(bound);
                self.bump(|t| {
                    t.certified_stage_solves += 1;
                    t.affine_pair_compositions += pairs;
                    t.graph_schur_small_core_solves += 1;
                    t.max_stage_residual_bound = t.max_stage_residual_bound.max(bound);
                });
                return Ok(());
            }
            self.bump(|t| t.candidate_failure_count += 1);
            self.ready = false;
        }
        self.prepare_fallback(counters)?;
        self.bump(|t| t.fallback_stage_solves += 1);
        self.protected.solve_in_place(b, counters)
    }
}

#[cfg(test)]
mod c5_source_scoped_tests {
    use super::*;

    fn owner() -> CooperativeCubicOwner {
        CooperativeCubicOwner {
            name: "C5-original-cooperative-frozen".into(),
            epoch: 29,
            k: vec![4., 7.],
            g: vec![20., 40.],
            c: vec![vec![0., 12.], vec![8., 0.]],
            feedback: vec![0],
        }
    }

    fn native_problem(owner: &CooperativeCubicOwner, corrupt: bool, epoch: u64) -> OdeProblem {
        let source = owner.clone();
        OdeProblem::new(
            owner.name.clone(),
            source.k.len(),
            Arc::new(move |_t, y, out| {
                for (i, target) in out.iter_mut().enumerate() {
                    let mut v = -source.k[i] * y[i] - source.g[i] * y[i] * y[i] * y[i];
                    for (j, value) in y.iter().enumerate() {
                        if j != i {
                            v += source.c[i][j] * (*value - y[i]);
                        }
                    }
                    *target = if corrupt { v + 1. } else { v };
                }
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(move |_t, _y, v, out| {
                out.copy_from_slice(v);
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap()
        .with_model_epoch(Arc::new(move || epoch))
    }

    fn owned() -> GraphSchurStageClient {
        GraphSchurStageClient::new(
            owner(),
            Arc::new(Mutex::new(GraphSchurTelemetry::default())),
        )
        .unwrap()
    }

    #[test]
    fn exact_original_unscaled_w_candidate_and_work_count() {
        let o = owner();
        let mut c = owned();
        let p = native_problem(&o, false, 29);
        let mut work = WorkCounters::default();
        c.linearize(&p, 0., &[0.2, 0.12], &mut work).unwrap();
        c.factor(300., &mut work).unwrap();
        let mut v = vec![1.0, -2.0];
        c.solve_in_place(&mut v, &mut work).unwrap();
        let w00 = 300. + 4. + 12. + 3. * 20. * 0.2 * 0.2;
        let w11 = 300. + 7. + 8. + 3. * 40. * 0.12 * 0.12;
        assert!((w00 * v[0] - 12. * v[1] - 1.).abs() < 1e-12);
        assert!((-8. * v[0] + w11 * v[1] + 2.).abs() < 1e-12);
        assert_eq!(c.telemetry().certified_stage_solves, 1);
        assert_eq!(c.telemetry().fallback_stage_solves, 0);
        assert_eq!(c.telemetry().original_residual_checks, 1);
        assert_eq!(work.jvp_calls, 0);
    }

    #[test]
    fn actually_falls_back_to_original_full_band_w_on_residual_failure() {
        let o = owner();
        let p = native_problem(&o, false, 29);
        let mut c = owned();
        let mut work = WorkCounters::default();
        c.linearize(&p, 0., &[0.2, 0.12], &mut work).unwrap();
        c.factor(300., &mut work).unwrap();
        // Deliberate same-author adversarial corruption of opt-in candidate
        // after the actual source factorization, not a physics-source change.
        c.small_inverse[0] = 0.;
        let mut rhs = vec![1., -2.];
        c.solve_in_place(&mut rhs, &mut work).unwrap();
        let w00 = 300. + 4. + 12. + 3. * 20. * 0.2 * 0.2;
        let w11 = 300. + 7. + 8. + 3. * 40. * 0.12 * 0.12;
        assert!((w00 * rhs[0] - 12. * rhs[1] - 1.).abs() < 1e-12);
        assert!((-8. * rhs[0] + w11 * rhs[1] + 2.).abs() < 1e-12);
        assert_eq!(c.telemetry().candidate_failure_count, 1);
        assert_eq!(c.telemetry().fallback_stage_solves, 1);
        assert_eq!(c.telemetry().fallback_factorizations, 1);
        assert!(work.direct_factorizations >= 2);
    }

    #[test]
    fn fails_closed_for_changed_source_epoch() {
        let mut c = owned();
        let p = native_problem(&owner(), false, 30);
        let error = c
            .linearize(&p, 0., &[0.2, 0.12], &mut WorkCounters::default())
            .unwrap_err();
        assert!(matches!(error, CoreError::InvalidInput(_)));
    }

    #[test]
    fn refuses_rhs_callback_divergence_even_with_same_owner_name() {
        let mut c = owned();
        let p = native_problem(&owner(), true, 29);
        let error = c
            .linearize(&p, 0., &[0.2, 0.12], &mut WorkCounters::default())
            .unwrap_err();
        assert!(matches!(error, CoreError::InvalidInput(_)));
    }

    #[test]
    fn refuses_non_cooperative_or_missing_feedback_fvs() {
        let mut bad = owner();
        bad.c[0][1] = -2.;
        assert!(
            GraphSchurStageClient::new(bad, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
                .is_err()
        );
        let mut dense = owner();
        dense.k = vec![1.; 6];
        dense.g = vec![1.; 6];
        dense.c = (0..6)
            .map(|i| (0..6).map(|j| if i == j { 0. } else { 1. }).collect())
            .collect();
        dense.feedback = vec![0, 1];
        assert!(
            GraphSchurStageClient::new(dense, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
                .is_err()
        );
    }

    #[test]
    fn source_dimension_and_full_integer_feedback_guard() {
        let mut bad = owner();
        bad.feedback = vec![0, 0];
        assert!(
            GraphSchurStageClient::new(bad, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
                .is_err()
        );
        let mut bad = owner();
        bad.feedback = vec![0, 1, 2, 3];
        assert!(
            GraphSchurStageClient::new(bad, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
                .is_err()
        );
    }
}

/// Bind the immutable cooperative source, native RHS/JVP and original eight-stage
/// linear client in a single operation. The callbacks capture the SAME
/// immutable Arc, not a caller-supplied unrelated OdeProblem. Opt-in research.
pub fn source_owned_cooperative_cubic_problem(
    owner: CooperativeCubicOwner,
    telemetry: Arc<Mutex<GraphSchurTelemetry>>,
) -> CoreResult<(OdeProblem, GraphSchurStageClient)> {
    let client = GraphSchurStageClient::new(owner.clone(), telemetry)?;
    let source = Arc::new(owner);
    let rhs_source = Arc::clone(&source);
    let jvp_source = Arc::clone(&source);
    let epoch_source = Arc::clone(&source);
    let n = source.k.len();
    let problem = OdeProblem::new(
        source.name.clone(),
        n,
        Arc::new(move |_t, y, out| {
            if y.len() != n || out.len() != n {
                return Err(CoreError::Dimension("C5 owned RHS shape mismatch".into()));
            }
            for i in 0..n {
                let mut v = -rhs_source.k[i] * y[i] - rhs_source.g[i] * y[i] * y[i] * y[i];
                for j in 0..n {
                    if i != j {
                        v += rhs_source.c[i][j] * (y[j] - y[i]);
                    }
                }
                out[i] = v;
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_t, y, direction, out| {
            if y.len() != n || direction.len() != n || out.len() != n {
                return Err(CoreError::Dimension("C5 owned JVP shape mismatch".into()));
            }
            for i in 0..n {
                let mut v = -(jvp_source.k[i] + 3. * jvp_source.g[i] * y[i] * y[i]) * direction[i];
                for j in 0..n {
                    if i != j {
                        v += jvp_source.c[i][j] * (direction[j] - direction[i]);
                    }
                }
                out[i] = v;
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )?
    .with_model_epoch(Arc::new(move || epoch_source.epoch));
    Ok((problem, client))
}
