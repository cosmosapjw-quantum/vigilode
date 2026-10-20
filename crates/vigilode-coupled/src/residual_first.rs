//! Residual-first correction of the unchanged projected raw-U stage target.
//! Stage states are retained from the residual phase, not recomputed for JVPs.
//! Full-space validation remains in the parent solver after convergence.
use super::*;

struct Projection {
    residual: Vec<f64>,
    relative: f64,
    affine: DenseMatrix,
    states: Vec<Vec<f64>>,
}

fn projected_residual(
    f: &Frozen<'_>,
    basis: &Basis,
    z: &[f64],
    lambda: f64,
    work: &mut Work,
) -> Result<Projection> {
    f.epoch()?;
    let m = basis.len();
    work.reserve(f.cfg, S as u64, 0)?;
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
            let outcome = (|| -> Result<(Vec<f64>, Vec<f64>)> {
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
                Ok((basis.project(&remainder), state))
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
    let mut states = Vec::with_capacity(S);
    for (_, result) in results {
        let (value, state) = result?;
        values.push(value);
        states.push(state);
    }
    let eta = f.cfg.theta + lambda * (1.0 - f.cfg.theta);
    let (matrix, rhs) = f.affine(basis, eta);
    let mut residual = vec![0.0; S * m];
    // Preserve eager evaluation's summation order and rounding.
    for i in 0..S * m {
        let mut row = vec![0.0; S * m];
        for j in 0..S * m {
            row[j] = matrix[(i, j)];
        }
        residual[i] = dot(&row, z) - rhs[i];
    }
    for i in 0..S {
        for u in 0..m {
            residual[i * m + u] -= lambda * f.hg * values[i][u];
        }
    }
    finite(&residual)?;
    finite(matrix.as_slice())?;
    let relative = norm(&residual) / norm(&rhs).max(f64::MIN_POSITIVE);
    if !relative.is_finite() {
        return Err(Failure::new(
            FailureKind::NonFinite,
            "projected residual norm overflow",
        ));
    }
    Ok(Projection {
        residual,
        relative,
        affine: matrix,
        states,
    })
}

fn derivative(
    f: &Frozen<'_>,
    basis: &Basis,
    lambda: f64,
    snapshot: &mut Projection,
    work: &mut Work,
) -> Result<()> {
    f.epoch()?;
    let m = basis.len();
    work.reserve(f.cfg, 0, (S * m) as u64)?;
    work.derivative_batch(f.cfg)?;
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
            let outcome = (|| -> Result<DenseMatrix> {
                // The saved state belongs to the SAME iterate and epoch as residual.
                let current =
                    f.p.linearize_matrix_free(f.t, &snapshot.states[i])
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
                Ok(derivative)
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
    for (_, result) in results {
        values.push(result?);
    }
    for i in 0..S {
        for u in 0..m {
            for j in 0..i {
                for v in 0..m {
                    snapshot.affine[(i * m + u, j * m + v)] -=
                        lambda * f.hg * f.c.a[(i, j)] * values[i][(u, v)];
                }
            }
        }
    }
    finite(snapshot.affine.as_slice())?;
    Ok(())
}

pub(super) fn correct(
    f: &Frozen<'_>,
    basis: &Basis,
    seed: &[f64],
    lambda: f64,
    out: &mut Attempt,
) -> Result<Vec<f64>> {
    let mut z = seed.to_vec();
    for iteration in 0..=f.cfg.max_newton {
        let mut snapshot = projected_residual(f, basis, &z, lambda, &mut out.work)?;
        out.trace.push(TracePoint {
            basis_dimension: basis.len(),
            lambda,
            iteration,
            projected_relative_residual: Some(snapshot.relative),
            full_residual_wrms: None,
            event: "projected-correction".into(),
        });
        // The eager counterfactual is (S - 1) m stage JVPs: stage 0's
        // derivative is never used, so eager does not evaluate it.
        if snapshot.relative <= f.cfg.projected_relative_tol {
            out.work.residual_only_stops += 1;
            out.work.unused_stage_jvp_avoided += ((S - 1) * basis.len()) as u64;
            return Ok(z);
        }
        if iteration == f.cfg.max_newton {
            out.work.residual_only_stops += 1;
            out.work.unused_stage_jvp_avoided += ((S - 1) * basis.len()) as u64;
            break;
        }
        derivative(f, basis, lambda, &mut snapshot, &mut out.work)?;
        let negative: Vec<_> = snapshot.residual.iter().map(|x| -x).collect();
        let delta = small_solve(&snapshot.affine, &negative, &mut out.work)?;
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
