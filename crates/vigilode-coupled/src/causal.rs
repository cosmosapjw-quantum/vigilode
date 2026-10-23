//! Causal elimination and finite-propagation Picard for the projected raw-U root.
//! No current-stage derivative dispatch; frozen base actions remain in Basis.
use super::*;
use rodas5p_core::LuFactorization;

fn check_causality(c: &Rodas5pCoefficients) -> Result<()> {
    if c.stages() != S
        || c.a.nrows() != S
        || c.a.ncols() != S
        || c.c_matrix.nrows() != S
        || c.c_matrix.ncols() != S
    {
        return Err(Failure::new(
            FailureKind::Unsupported,
            "causal schedule requires eight raw-U stages",
        ));
    }
    for i in 0..S {
        for j in i..S {
            if c.a[(i, j)] != 0.0 || c.c_matrix[(i, j)] != 0.0 {
                return Err(Failure::new(
                    FailureKind::Unsupported,
                    "raw-U tableau contains noncausal entries",
                ));
            }
        }
    }
    Ok(())
}

fn factors(f: &Frozen<'_>, b: &Basis, w: &mut Work) -> Result<LuFactorization> {
    check_causality(f.c)?;
    let j = b.projected_jacobian();
    let m = b.len();
    let mut diagonal = DenseMatrix::zeros(m, m);
    for u in 0..m {
        for v in 0..m {
            diagonal[(u, v)] = if u == v { 1.0 } else { 0.0 } - f.hg * j[(u, v)];
        }
    }
    finite(diagonal.as_slice())?;
    w.small_factorizations += 1;
    LuFactorization::new(&diagonal)
        .map_err(|e| Failure::new(FailureKind::ProjectionSingular, e.to_string()))
}

fn block_solve(lu: &LuFactorization, rhs: &[f64], work: &mut Work) -> Result<Vec<f64>> {
    finite(rhs)?;
    work.small_block_solves += 1;
    let x = lu
        .solve(rhs)
        .map_err(|e| Failure::new(FailureKind::ProjectionSingular, e.to_string()))?;
    finite(&x)?;
    Ok(x)
}

/// For lambda=1, each stage's nonlinear argument contains only earlier stages.
/// Thus one shared W_m factorization and eight solves eliminate the entire root.
pub(super) fn forward(f: &Frozen<'_>, b: &Basis, out: &mut Attempt) -> Result<Vec<f64>> {
    let m = b.len();
    let lu = factors(f, b, &mut out.work)?;
    // Stage zero reuses f0. Reserve all seven new calls before entering the sweep.
    f.epoch()?;
    out.work.reserve(f.cfg, (S - 1) as u64, 0)?;
    out.work.batch(f.cfg, false)?;
    let mut z = vec![0.0; S * m];
    for i in 0..S {
        f.epoch()?;
        let projected = if i == 0 {
            b.project(&f.f0)
        } else {
            let mut xz = vec![0.0; m];
            for k in 0..i {
                for u in 0..m {
                    xz[u] += f.c.a[(i, k)] * z[k * m + u];
                }
            }
            let x = b.lift(&xz);
            let state: Vec<_> = f.y.iter().zip(&x).map(|(y, x)| y + x).collect();
            let mut local = Local::default();
            let result = callback_rhs(f, &state, &mut local);
            merge(&mut out.work, &local, false);
            let fy = result?;
            b.project(&fy)
        };
        let mut rhs: Vec<_> = projected.iter().map(|v| f.hg * v).collect();
        for k in 0..i {
            for u in 0..m {
                rhs[u] += f.c.gamma * f.c.c_matrix[(i, k)] * z[k * m + u];
            }
        }
        let zi = block_solve(&lu, &rhs, &mut out.work)?;
        z[i * m..(i + 1) * m].copy_from_slice(&zi);
        out.work.causal_forward_stages += 1;
        out.work.stage_tasks_completed += 1;
    }
    f.epoch()?;
    out.work.small_solves += 1;
    out.work.peak_parallel_tasks = out.work.peak_parallel_tasks.max(1);
    out.trace.push(TracePoint {
        basis_dimension: m,
        lambda: 1.0,
        iteration: 0,
        projected_relative_residual: None,
        full_residual_wrms: None,
        event: "causal-forward-common-factor".into(),
    });
    // The original-space residual checker is called by the parent unconditionally.
    Ok(z)
}

fn affine_solve(
    a: &DenseMatrix,
    rhs: &[f64],
    lu: &LuFactorization,
    m: usize,
    w: &mut Work,
) -> Result<Vec<f64>> {
    let mut x = vec![0.0; S * m];
    for i in 0..S {
        let offset = i * m;
        let block: Vec<_> = (0..m)
            .map(|u| rhs[offset + u] - dot(&a.row(offset + u)[..offset], &x[..offset]))
            .collect();
        let xi = block_solve(lu, &block, w)?;
        x[offset..offset + m].copy_from_slice(&xi);
    }
    w.small_solves += 1;
    Ok(x)
}

/// The affine inverse preserves strict causal dependence. After k simultaneous
/// updates, the first k+1 stages equal the unique projected target in exact
/// arithmetic. No contraction constant is required for this finite statement.
pub(super) fn picard(f: &Frozen<'_>, b: &Basis, out: &mut Attempt) -> Result<Vec<f64>> {
    let m = b.len();
    let lu = factors(f, b, &mut out.work)?;
    let (a, affine_rhs) = f.affine(b, 1.0);
    finite(a.as_slice())?;
    finite(&affine_rhs)?;
    let mut z = affine_solve(&a, &affine_rhs, &lu, m, &mut out.work)?;
    let rhs_norm = norm(&affine_rhs).max(f64::MIN_POSITIVE);
    let ids: Vec<_> = (0..S).collect();
    for iteration in 0..=S {
        f.epoch()?;
        out.work.reserve(f.cfg, (S - 1) as u64, 0)?;
        out.work.batch(f.cfg, false)?;
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let results = f
            .execution
            .map_ordered(&ids, |&i| {
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(count, Ordering::SeqCst);
                let _guard = Active(&active);
                let mut local = Local::default();
                let result = (|| -> Result<Vec<f64>> {
                    if i == 0 {
                        return Ok(vec![0.0; m]);
                    }
                    let mut xz = vec![0.0; m];
                    for k in 0..i {
                        for u in 0..m {
                            xz[u] += f.c.a[(i, k)] * z[k * m + u];
                        }
                    }
                    let x = b.lift(&xz);
                    let jx = b.image_lift(&xz);
                    let state: Vec<_> = f.y.iter().zip(&x).map(|(y, x)| y + x).collect();
                    let fy = callback_rhs(f, &state, &mut local)?;
                    let remainder: Vec<_> = fy
                        .iter()
                        .zip(&f.f0)
                        .zip(&jx)
                        .map(|((v, base), j)| v - base - j)
                        .collect();
                    finite(&remainder)?;
                    Ok(b.project(&remainder))
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
            merge(&mut out.work, local, false);
        }
        f.epoch()?;
        let mut rhs = affine_rhs.clone();
        for (i, (_, r)) in results.into_iter().enumerate() {
            for (u, v) in r?.into_iter().enumerate() {
                rhs[i * m + u] += f.hg * v;
            }
        }
        finite(&rhs)?;
        let residual: Vec<_> = (0..S * m).map(|i| dot(a.row(i), &z) - rhs[i]).collect();
        finite(&residual)?;
        let relative = norm(&residual) / rhs_norm;
        if !relative.is_finite() {
            return Err(Failure::new(
                FailureKind::NonFinite,
                "causal residual overflow",
            ));
        }
        out.trace.push(TracePoint {
            basis_dimension: m,
            lambda: 1.0,
            iteration,
            projected_relative_residual: Some(relative),
            full_residual_wrms: None,
            event: "causal-batched-picard".into(),
        });
        if relative <= f.cfg.projected_relative_tol {
            out.work.residual_only_stops += 1;
            return Ok(z);
        }
        if iteration == S {
            break;
        }
        z = affine_solve(&a, &rhs, &lu, m, &mut out.work)?;
        out.work.causal_picard_updates += 1;
    }
    Err(Failure::new(
        FailureKind::RootIteration,
        "causal Picard finite-sweep budget exhausted at represented precision",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn noncausal_tableau_entries_are_refused_exactly() {
        let c = rodas5p_coefficients().unwrap();
        check_causality(c).unwrap();
        for (i, j) in [(0, 0), (0, 7), (4, 6)] {
            let mut altered = c.clone();
            altered.a[(i, j)] = f64::MIN_POSITIVE;
            assert_eq!(
                check_causality(&altered).unwrap_err().kind,
                FailureKind::Unsupported
            );
            let mut altered = c.clone();
            altered.c_matrix[(i, j)] = f64::MIN_POSITIVE;
            assert_eq!(
                check_causality(&altered).unwrap_err().kind,
                FailureKind::Unsupported
            );
        }
    }
}
