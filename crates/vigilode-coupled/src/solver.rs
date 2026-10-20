use crate::algebra::{Basis, dot, finite, norm, small_solve};
use crate::{
    Attempt, Candidate, Config, Failure, FailureKind, Result, TARGET_ID, TracePoint, Work,
};
use rodas5p_core::{
    CoreError, DenseMatrix, LinearOperator, Rodas5pCoefficients, WorkCounters, rodas5p_coefficients,
};
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
const S: usize = 8;

struct Frozen<'a> {
    p: &'a OdeProblem,
    t: f64,
    y: &'a [f64],
    hg: f64,
    c: &'static Rodas5pCoefficients,
    cfg: &'a Config,
    execution: &'a ParallelExecution,
    base: Arc<dyn LinearOperator>,
    f0: Vec<f64>,
    epoch: Option<u64>,
}
impl Frozen<'_> {
    fn epoch(&self) -> Result<()> {
        if self.p.model_epoch() != self.epoch {
            Err(Failure::new(
                FailureKind::ModelChanged,
                "client model epoch changed during attempt",
            ))
        } else {
            Ok(())
        }
    }
    fn affine(&self, b: &Basis, eta: f64) -> (DenseMatrix, Vec<f64>) {
        let m = b.len();
        let j = b.projected_jacobian();
        let mut a = DenseMatrix::zeros(S * m, S * m);
        let mut rhs = vec![0.0; S * m];
        let f = b.project(&self.f0);
        for i in 0..S {
            for u in 0..m {
                rhs[i * m + u] = self.hg * f[u];
                for v in 0..m {
                    a[(i * m + u, i * m + v)] =
                        if u == v { 1.0 } else { 0.0 } - self.hg * j[(u, v)];
                }
                for k in 0..i {
                    for v in 0..m {
                        a[(i * m + u, k * m + v)] = -eta
                            * (self.c.gamma
                                * self.c.c_matrix[(i, k)]
                                * if u == v { 1.0 } else { 0.0 }
                                + self.hg * self.c.a[(i, k)] * j[(u, v)]);
                    }
                }
            }
        }
        (a, rhs)
    }
}
#[derive(Default)]
struct Local {
    rhs: u64,
    rhs_ok: u64,
    jvp: u64,
    jvp_ok: u64,
}
struct Active<'a>(&'a AtomicUsize);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
fn merge(work: &mut Work, local: &Local, validation: bool) {
    work.rhs_attempted += local.rhs;
    work.rhs_completed += local.rhs_ok;
    work.jvp_attempted += local.jvp;
    work.jvp_completed += local.jvp_ok;
    if validation {
        work.validation_jvp += local.jvp;
    } else {
        work.stage_jvp += local.jvp;
    }
}
fn callback_rhs(f: &Frozen<'_>, state: &[f64], local: &mut Local) -> Result<Vec<f64>> {
    finite(state)?;
    let mut y = vec![f64::NAN; state.len()];
    let mut ignored = WorkCounters::default();
    local.rhs += 1;
    f.p.eval_rhs_into(f.t, state, &mut y, &mut ignored)
        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
    finite(&y)?;
    local.rhs_ok += 1;
    Ok(y)
}
fn callback_jvp(op: &dyn LinearOperator, v: &[f64], local: &mut Local) -> Result<Vec<f64>> {
    let mut y = vec![f64::NAN; v.len()];
    local.jvp += 1;
    op.apply(v, &mut y)
        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
    finite(&y)?;
    local.jvp_ok += 1;
    Ok(y)
}
struct Evaluation {
    residual: Vec<f64>,
    jacobian: DenseMatrix,
    relative: f64,
}

/// Every task sees the same z. No stage consumes another task's updated stage.
fn evaluate(
    f: &Frozen<'_>,
    basis: &Basis,
    z: &[f64],
    lambda: f64,
    work: &mut Work,
) -> Result<Evaluation> {
    f.epoch()?;
    let m = basis.len();
    work.reserve(f.cfg, S as u64, (S * m) as u64)?;
    work.batch(f.cfg, false)?;
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let ids: Vec<_> = (0..S).collect();
    let results = f
        .execution
        .map_ordered(&ids, |&i| {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(count, Ordering::SeqCst);
            let _active = Active(&active);
            let mut local = Local::default();
            let outcome = (|| -> Result<(Vec<f64>, DenseMatrix)> {
                let mut xz = vec![0.0; m];
                for k in 0..i {
                    for u in 0..m {
                        xz[u] += f.c.a[(i, k)] * z[k * m + u];
                    }
                }
                let x = basis.lift(&xz);
                let jx = basis.image_lift(&xz);
                let state: Vec<_> = f.y.iter().zip(&x).map(|(y, x)| y + x).collect();
                let fy = callback_rhs(f, &state, &mut local)?;
                let remainder: Vec<_> = fy
                    .iter()
                    .zip(&f.f0)
                    .zip(jx)
                    .map(|((y, base), j)| y - base - j)
                    .collect();
                finite(&remainder)?;
                let projected = basis.project(&remainder);
                let current =
                    f.p.linearize_matrix_free(f.t, &state)
                        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
                let mut derivative = DenseMatrix::zeros(m, m);
                for v in 0..m {
                    let mut d = callback_jvp(current.as_ref(), &basis.q[v], &mut local)?;
                    for (x, j) in d.iter_mut().zip(&basis.jq[v]) {
                        *x -= j;
                    }
                    for u in 0..m {
                        derivative[(u, v)] = dot(&basis.q[u], &d);
                    }
                }
                finite(derivative.as_slice())?;
                Ok((projected, derivative))
            })();
            Ok::<_, CoreError>((local, outcome))
        })
        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
    work.peak_parallel_tasks = work.peak_parallel_tasks.max(peak.load(Ordering::SeqCst));
    work.stage_tasks_completed += results.len() as u64;
    for (local, _) in &results {
        merge(work, local, false);
    }
    f.epoch()?;
    let mut values = Vec::with_capacity(S);
    for (_, r) in results {
        values.push(r?);
    }
    let eta = f.cfg.theta + lambda * (1.0 - f.cfg.theta);
    let (mut k, rhs) = f.affine(basis, eta);
    let mut residual = vec![0.0; S * m];
    for i in 0..S * m {
        let mut row = vec![0.0; S * m];
        for j in 0..S * m {
            row[j] = k[(i, j)];
        }
        residual[i] = dot(&row, z) - rhs[i];
    }
    for i in 0..S {
        for u in 0..m {
            residual[i * m + u] -= lambda * f.hg * values[i].0[u];
            for j in 0..i {
                for v in 0..m {
                    k[(i * m + u, j * m + v)] -=
                        lambda * f.hg * f.c.a[(i, j)] * values[i].1[(u, v)];
                }
            }
        }
    }
    finite(&residual)?;
    finite(k.as_slice())?;
    let relative = norm(&residual) / norm(&rhs).max(f64::MIN_POSITIVE);
    if !relative.is_finite() {
        return Err(Failure::new(
            FailureKind::NonFinite,
            "projected residual norm overflow",
        ));
    }
    Ok(Evaluation {
        residual,
        jacobian: k,
        relative,
    })
}
fn correct(
    f: &Frozen<'_>,
    basis: &Basis,
    seed: &[f64],
    lambda: f64,
    out: &mut Attempt,
) -> Result<Vec<f64>> {
    let mut z = seed.to_vec();
    for iteration in 0..=f.cfg.max_newton {
        let e = evaluate(f, basis, &z, lambda, &mut out.work)?;
        out.trace.push(TracePoint {
            basis_dimension: basis.len(),
            lambda,
            iteration,
            projected_relative_residual: Some(e.relative),
            full_residual_wrms: None,
            event: "projected-correction".into(),
        });
        if e.relative <= f.cfg.projected_relative_tol {
            return Ok(z);
        }
        if iteration == f.cfg.max_newton {
            break;
        }
        let negative: Vec<_> = e.residual.iter().map(|x| -x).collect();
        let delta = small_solve(&e.jacobian, &negative, &mut out.work)?;
        for (x, d) in z.iter_mut().zip(delta) {
            *x += d;
        }
        finite(&z)?;
        out.work.newton_updates += 1;
    }
    Err(Failure::new(
        FailureKind::RootIteration,
        "projected corrector exhausted its iteration budget",
    ))
}
fn homotopy(f: &Frozen<'_>, basis: &Basis, out: &mut Attempt) -> Result<Vec<f64>> {
    let (a, b) = f.affine(basis, f.cfg.theta);
    let mut accepted = small_solve(&a, &b, &mut out.work)?;
    let (mut lambda, mut target, mut splits) = (0.0, 1.0, 0usize);
    loop {
        match correct(f, basis, &accepted, target, out) {
            Ok(z) => {
                accepted = z;
                lambda = target;
                if lambda == 1.0 {
                    return Ok(accepted);
                }
                target = 1.0;
            }
            Err(error)
                if error.kind == FailureKind::RootIteration && splits < f.cfg.max_subdivisions =>
            {
                let next = (lambda + target) * 0.5;
                if next <= lambda || next >= target {
                    return Err(error);
                }
                out.work.lambda_rejections += 1;
                splits += 1;
                target = next;
                out.trace.push(TracePoint {
                    basis_dimension: basis.len(),
                    lambda: target,
                    iteration: 0,
                    projected_relative_residual: None,
                    full_residual_wrms: None,
                    event: "lambda-subdivision-after-declined-corrector".into(),
                });
            }
            Err(error) => return Err(error),
        }
    }
}
fn full_check(
    f: &Frozen<'_>,
    basis: &Basis,
    z: &[f64],
    out: &mut Attempt,
) -> Result<(Candidate, Vec<Vec<f64>>)> {
    f.epoch()?;
    let m = basis.len();
    let n = f.y.len();
    let u: Vec<_> = (0..S).map(|i| basis.lift(&z[i * m..(i + 1) * m])).collect();
    let mut ynew = f.y.to_vec();
    for i in 0..S {
        for k in 0..n {
            ynew[k] += f.c.b_code[i] * u[i][k];
        }
    }
    finite(&ynew)?;
    let error = u[S - 1].clone();
    let scale: Vec<_> =
        f.y.iter()
            .zip(&ynew)
            .map(|(a, b)| f.cfg.atol + f.cfg.rtol * a.abs().max(b.abs()))
            .collect();
    finite(&scale)?;
    if scale.iter().any(|x| *x <= 0.0) {
        return Err(Failure::new(FailureKind::NonFinite, "invalid error scale"));
    }
    out.work.reserve(f.cfg, S as u64, S as u64)?;
    out.work.batch(f.cfg, true)?;
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let ids: Vec<_> = (0..S).collect();
    let results = f
        .execution
        .map_ordered(&ids, |&i| {
            let a = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(a, Ordering::SeqCst);
            let _active = Active(&active);
            let mut local = Local::default();
            let result = (|| -> Result<Vec<f64>> {
                let mut state = f.y.to_vec();
                let mut mix = vec![0.0; n];
                for j in 0..i {
                    for k in 0..n {
                        state[k] += f.c.a[(i, j)] * u[j][k];
                        mix[k] += f.c.c_matrix[(i, j)] * u[j][k];
                    }
                }
                let fy = callback_rhs(f, &state, &mut local)?;
                let ju = callback_jvp(f.base.as_ref(), &u[i], &mut local)?;
                let r = (0..n)
                    .map(|k| u[i][k] - f.hg * ju[k] - f.c.gamma * mix[k] - f.hg * fy[k])
                    .collect::<Vec<_>>();
                finite(&r)?;
                Ok(r)
            })();
            Ok::<_, CoreError>((local, result))
        })
        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
    out.work.peak_parallel_tasks = out
        .work
        .peak_parallel_tasks
        .max(peak.load(Ordering::SeqCst));
    out.work.stage_tasks_completed += results.len() as u64;
    for (local, _) in &results {
        merge(&mut out.work, local, true);
    }
    f.epoch()?;
    let mut residuals = Vec::with_capacity(S);
    for (_, r) in results {
        residuals.push(r?);
    }
    let mut wrms = 0.0_f64;
    let mut l2 = 0.0_f64;
    for r in &residuals {
        let scaled: Vec<_> = r.iter().zip(&scale).map(|(x, s)| x / s).collect();
        wrms = wrms.max(norm(&scaled) / (n as f64).sqrt());
        l2 = l2.max(norm(r));
    }
    let scaled_error: Vec<_> = error.iter().zip(&scale).map(|(x, s)| x / s).collect();
    let embedded = norm(&scaled_error) / (n as f64).sqrt();
    let relative = l2 / (f.hg.abs() * norm(&f.f0)).max(f64::MIN_POSITIVE);
    if !wrms.is_finite() || !embedded.is_finite() || !relative.is_finite() {
        return Err(Failure::new(
            FailureKind::NonFinite,
            "original-space residual/error overflow",
        ));
    }
    out.trace.push(TracePoint {
        basis_dimension: m,
        lambda: 1.0,
        iteration: 0,
        projected_relative_residual: None,
        full_residual_wrms: Some(wrms),
        event: "original-space-target-check".into(),
    });
    Ok((
        Candidate {
            target_id: TARGET_ID,
            y_new: ynew,
            stages_u: u,
            error_vector: error,
            embedded_wrms: embedded,
            full_residual_wrms: wrms,
            full_relative_residual: relative,
            basis_dimension: m,
            embedded_test_passes: embedded <= 1.0,
            accuracy_authority: "operational-residual-only-not-certified",
        },
        residuals,
    ))
}
fn validate(p: &OdeProblem, t: f64, y: &[f64], h: f64, c: &Config, work: &mut Work) -> Result<()> {
    let input = |s| Failure::new(FailureKind::Input, s);
    if y.len() != p.dimension || y.is_empty() {
        return Err(input("state dimension mismatch"));
    }
    if !t.is_finite() || !h.is_finite() || h <= 0.0 || !(t + h).is_finite() || t + h <= t {
        return Err(input("invalid or unrepresentable positive step"));
    }
    finite(y)?;
    if !p.autonomous || p.mass_matrix.is_some() || !p.supports_matrix_free_jvp() {
        return Err(Failure::new(
            FailureKind::Unsupported,
            "requires autonomous identity-mass problem with supplied JVP",
        ));
    }
    if c.initial_basis == 0
        || c.initial_basis > c.max_basis
        || c.max_basis > 64
        || c.max_expansions > 64
        || c.max_newton > 64
        || c.max_subdivisions > 16
        || c.max_batches > 10000
    {
        return Err(input("invalid bounded solver limits"));
    }
    for v in [
        c.atol,
        c.rtol,
        c.projected_relative_tol,
        c.full_residual_wrms_limit,
        c.rank_relative_tol,
    ] {
        if !v.is_finite() || v <= 0.0 {
            return Err(input("tolerances must be finite and positive"));
        }
    }
    if !c.theta.is_finite() || !(0.0..=1.0).contains(&c.theta) || c.rank_relative_tol >= 1.0 {
        return Err(input("invalid theta or rank tolerance"));
    }
    let n = y.len();
    let m = c.max_basis.min(n);
    let sm = m.checked_mul(S).ok_or_else(|| input("size overflow"))?;
    let bytes = n
        .checked_mul(m + S * 8)
        .and_then(|v| v.checked_mul(64))
        .and_then(|v| {
            sm.checked_mul(sm)
                .and_then(|s| s.checked_mul(64))
                .and_then(|s| v.checked_add(s))
        })
        .ok_or_else(|| input("storage-size overflow"))?;
    work.planned_storage_bytes = bytes;
    if bytes > c.max_storage_bytes {
        return Err(Failure::new(
            FailureKind::Budget,
            "principal-array storage plan exceeds budget",
        ));
    }
    Ok(())
}
pub(crate) fn run(
    p: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    cfg: &Config,
    execution: &ParallelExecution,
    out: &mut Attempt,
) -> Result<Candidate> {
    validate(p, t, y, h, cfg, &mut out.work)?;
    let epoch = p.model_epoch();
    let c = rodas5p_coefficients().map_err(|e| Failure::new(FailureKind::Input, e.to_string()))?;
    let hg = h * c.gamma;
    if !hg.is_finite() || hg == 0.0 {
        return Err(Failure::new(FailureKind::Input, "unrepresentable h*gamma"));
    }
    let base = p
        .linearize_matrix_free(t, y)
        .map_err(|e| Failure::new(FailureKind::Unsupported, e.to_string()))?;
    out.work.reserve(cfg, 1, 0)?;
    out.work.rhs_attempted += 1;
    let mut counters = WorkCounters::default();
    let mut f0 = vec![f64::NAN; y.len()];
    p.eval_rhs_into(t, y, &mut f0, &mut counters)
        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
    finite(&f0)?;
    out.work.rhs_completed += 1;
    let f = Frozen {
        p,
        t,
        y,
        hg,
        c,
        cfg,
        execution,
        base,
        f0,
        epoch,
    };
    f.epoch()?;
    if norm(&f.f0) == 0.0 {
        return Ok(Candidate {
            target_id: TARGET_ID,
            y_new: y.to_vec(),
            stages_u: vec![vec![0.0; y.len()]; S],
            error_vector: vec![0.0; y.len()],
            embedded_wrms: 0.0,
            full_residual_wrms: 0.0,
            full_relative_residual: 0.0,
            basis_dimension: 0,
            embedded_test_passes: true,
            accuracy_authority: "operational-stationary-autonomous-root",
        });
    }
    let mut basis = Basis::new(y.len());
    basis.add(&f.f0, f.base.as_ref(), cfg, &mut out.work)?;
    basis.grow(cfg.initial_basis, f.base.as_ref(), cfg, &mut out.work)?;
    for expansion in 0..=cfg.max_expansions {
        f.epoch()?;
        out.work.maximum_reduced_dimension =
            out.work.maximum_reduced_dimension.max(S * basis.len());
        let z = homotopy(&f, &basis, out)?;
        let (candidate, residuals) = full_check(&f, &basis, &z, out)?;
        if candidate.full_residual_wrms <= cfg.full_residual_wrms_limit {
            return Ok(candidate);
        }
        out.partial_stages = Some(candidate.stages_u);
        if expansion == cfg.max_expansions
            || !basis.expand(&residuals, f.base.as_ref(), cfg, &mut out.work)?
        {
            return Err(Failure::new(
                FailureKind::ProjectionLeakage,
                "projected root failed original-space residual gate; expansion budget exhausted",
            ));
        }
    }
    Err(Failure::new(
        FailureKind::ProjectionLeakage,
        "no admissible candidate",
    ))
}
