//! Conditional research prototype: immutable CSR original-residual certificates.
//! No public solver dispatch or invariant-module kernel is changed.
use rodas5p_core::directed::{Interval, add_up};
use rodas5p_core::{CoreError, CoreResult};
use std::hint::black_box;
use std::time::Instant;

#[derive(Clone)]
struct OwnedSparse {
    n: usize,
    offsets: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<f64>,
    epoch: u64,
}
fn invalid(s: &str) -> CoreError {
    CoreError::InvalidInput(s.into())
}
impl OwnedSparse {
    fn new(
        n: usize,
        offsets: Vec<usize>,
        columns: Vec<usize>,
        values: Vec<f64>,
        epoch: u64,
    ) -> CoreResult<Self> {
        if n == 0
            || offsets.len() != n + 1
            || offsets[0] != 0
            || offsets[n] != values.len()
            || columns.len() != values.len()
            || offsets.windows(2).any(|w| w[0] > w[1])
            || values.iter().any(|x| !x.is_finite())
        {
            return Err(invalid("invalid CSR ownership"));
        }
        let mut diagonal = vec![0.; n];
        let mut radius = vec![0.; n];
        for i in 0..n {
            let mut previous = None;
            for k in offsets[i]..offsets[i + 1] {
                let j = columns[k];
                if j >= n || previous.is_some_and(|p| p >= j) {
                    return Err(invalid("noncanonical CSR columns"));
                }
                previous = Some(j);
                if i == j {
                    diagonal[i] = values[k];
                } else {
                    // Triangle inequality permits both directed entries; no dense symmetric matrix is needed.
                    let half = Interval::point(values[k].abs())?.scale(0.5)?.mag();
                    radius[i] = add_up(radius[i], half)?;
                    radius[j] = add_up(radius[j], half)?;
                }
            }
        }
        if diagonal
            .iter()
            .zip(radius)
            .any(|(&d, r)| add_up(d, r).map_or(true, |u| u > 0.))
        {
            return Err(invalid("dissipativity not proved"));
        }
        Ok(Self {
            n,
            offsets,
            columns,
            values,
            epoch,
        })
    }
    fn check(&self, epoch: u64, alpha: f64, b: &[f64], x: &[f64]) -> CoreResult<()> {
        if epoch != self.epoch
            || !alpha.is_finite()
            || alpha < 0.
            || b.len() != self.n
            || x.len() != self.n
            || b.iter().chain(x).any(|a| !a.is_finite())
        {
            return Err(invalid("invalid certificate target"));
        }
        Ok(())
    }
    fn residual(
        &self,
        epoch: u64,
        alpha: f64,
        b: &[f64],
        x: &[f64],
    ) -> CoreResult<(Vec<Interval>, f64)> {
        self.check(epoch, alpha, b, x)?;
        let mut out = Vec::with_capacity(self.n);
        let mut bound = 0.;
        for i in 0..self.n {
            let mut action = Interval::point(0.)?;
            for k in self.offsets[i]..self.offsets[i + 1] {
                action = action.add(
                    Interval::point(self.values[k])?.mul(Interval::point(x[self.columns[k]])?)?,
                )?;
            }
            let r = Interval::point(b[i])?
                .sub(Interval::point(x[i])?)?
                .add(action.scale(alpha)?)?;
            bound = add_up(bound, r.mag())?;
            out.push(r);
        }
        Ok((out, bound))
    }
    fn dense(&self) -> Vec<f64> {
        let mut b = vec![0.; self.n * self.n];
        for i in 0..self.n {
            for k in self.offsets[i]..self.offsets[i + 1] {
                b[i * self.n + self.columns[k]] = self.values[k];
            }
        }
        b
    }
}
fn dense_residual(
    n: usize,
    bmat: &[f64],
    alpha: f64,
    b: &[f64],
    x: &[f64],
) -> CoreResult<(Vec<Interval>, f64)> {
    let mut out = Vec::with_capacity(n);
    let mut bound = 0.;
    for i in 0..n {
        let mut r = Interval::point(b[i])?;
        for j in 0..n {
            let a = Interval::point(if i == j { 1. } else { 0. })?
                .sub(Interval::point(bmat[i * n + j])?.scale(alpha)?)?;
            r = r.sub(a.mul(Interval::point(x[j])?)?)?;
        }
        bound = add_up(bound, r.mag())?;
        out.push(r);
    }
    Ok((out, bound))
}
fn fixture(n: usize, beta: f64) -> OwnedSparse {
    let mut offsets = vec![0];
    let mut columns = Vec::new();
    let mut values = Vec::new();
    for i in 0..n {
        columns.push(i);
        values.push(-2.);
        if beta != 0. && i + 1 < n {
            columns.push(i + 1);
            values.push(beta);
        }
        offsets.push(columns.len());
    }
    OwnedSparse::new(n, offsets, columns, values, 11).unwrap()
}
fn direct(b: &[f64], alpha: f64, beta: f64) -> Vec<f64> {
    let mut x = vec![0.; b.len()];
    for i in (0..b.len()).rev() {
        x[i] = (b[i]
            + if i + 1 < b.len() {
                alpha * beta * x[i + 1]
            } else {
                0.
            })
            / (1. + 2. * alpha);
    }
    x
}
// Exact integer oracle for these dyadic fixtures. All input multiples of 2^-20
// are exactly represented; magnitudes remain far below i128 limits.
fn exact_residual(b: &[f64], x: &[f64], beta: f64) -> Vec<f64> {
    const Q: f64 = 1048576.;
    let mut out = Vec::new();
    for i in 0..b.len() {
        let bi = (b[i] * Q) as i128;
        let xi = (x[i] * Q) as i128;
        assert_eq!(bi as f64, b[i] * Q);
        assert_eq!(xi as f64, x[i] * Q);
        let next = if beta != 0. && i + 1 < x.len() {
            (x[i + 1] * Q) as i128
        } else {
            0
        };
        let numerator = 8 * bi - 16 * xi + next;
        // The actual cancellation leaves <=2^53 in all manufactured cases.
        assert!(numerator.unsigned_abs() <= (1u128 << 53));
        out.push(numerator as f64 / (8. * Q));
    }
    out
}
fn verify_intervals(actual: &[Interval], exact: &[f64]) {
    assert!(actual.iter().zip(exact).all(|(r, &e)| r.contains(e)));
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut negative_checks = 0;
    assert!(OwnedSparse::new(1, vec![0, 2], vec![0, 0], vec![-1., -1.], 1).is_err());
    negative_checks += 1;
    assert!(OwnedSparse::new(1, vec![0, 1], vec![0], vec![f64::NAN], 1).is_err());
    negative_checks += 1;
    assert!(OwnedSparse::new(1, vec![0, 1], vec![0], vec![1.], 1).is_err());
    negative_checks += 1;
    for n in [32usize, 128, 512] {
        for (name, beta, cancel) in [
            ("normal-diagonal", 0., false),
            ("nonnormal-upper-bidiagonal", 0.25, false),
            ("nonnormal-cancellation", 0.25, true),
        ] {
            let setup_start = Instant::now();
            let plan = fixture(n, beta);
            let setup = setup_start.elapsed().as_secs_f64();
            let dense = plan.dense();
            let alpha = 0.5;
            let tolerance = 1e-10;
            let exact: Vec<f64> = (0..n)
                .map(|i| {
                    if cancel {
                        if i % 2 == 0 {
                            2f64.powi(40)
                        } else {
                            -2f64.powi(40)
                        }
                    } else {
                        if i % 2 == 0 { 1. } else { -2. }
                    }
                })
                .collect();
            let b: Vec<f64> = (0..n)
                .map(|i| {
                    2. * exact[i]
                        - if i + 1 < n {
                            alpha * beta * exact[i + 1]
                        } else {
                            0.
                        }
                })
                .collect();
            assert!(plan.residual(12, alpha, &b, &exact).is_err());
            negative_checks += 1;
            assert!(plan.residual(11, alpha, &b[..n - 1], &exact).is_err());
            negative_checks += 1;
            let mut bad = exact.clone();
            bad[n / 2] += 2f64.powi(-10);
            let truth = exact_residual(&b, &bad, beta);
            let (sp, sb) = plan.residual(11, alpha, &b, &bad)?;
            let (dp, db) = dense_residual(n, &dense, alpha, &b, &bad)?;
            verify_intervals(&sp, &truth);
            verify_intervals(&dp, &truth);
            assert!(sb > tolerance && db > tolerance);
            negative_checks += 1;
            // Rejected speculative candidate triggers one current full-equation direct fallback.
            let fallback_start = Instant::now();
            let corrected = direct(&b, alpha, beta);
            let (_, fallback_bound) = plan.residual(11, alpha, &b, &corrected)?;
            let fallback_seconds = fallback_start.elapsed().as_secs_f64();
            assert!(fallback_bound <= tolerance);
            println!(
                "{}",
                serde_json::json!({"kind":"negative_control","dimension":n,"fixture":name,"bad_candidate_refused":true,"structured_error_upper":sb,"dense_error_upper":db,"fallback_count":1,"fallback_seconds":fallback_seconds,"fallback_certified":true})
            );
            for repetition in 0..4 {
                for offset in 0..3 {
                    let mode = ["structured-directed", "dense-directed", "trusted-exact"]
                        [(offset + repetition) % 3];
                    let all_start = Instant::now();
                    let solve_start = Instant::now();
                    let x = direct(&b, alpha, beta);
                    let solve = solve_start.elapsed().as_secs_f64();
                    let start = Instant::now();
                    let (intervals, bound) = match mode {
                        "structured-directed" => plan.residual(11, alpha, &b, &x)?,
                        "dense-directed" => dense_residual(n, &dense, alpha, &b, &x)?,
                        _ => {
                            let r = exact_residual(&b, &x, beta);
                            let bound = r.iter().map(|v| v.abs()).sum::<f64>();
                            (
                                r.into_iter().map(|v| Interval::point(v).unwrap()).collect(),
                                bound,
                            )
                        }
                    };
                    let cert = start.elapsed().as_secs_f64();
                    let total = all_start.elapsed().as_secs_f64();
                    black_box(bound);
                    let truth = exact_residual(&b, &x, beta);
                    verify_intervals(&intervals, &truth);
                    let true_error = x
                        .iter()
                        .zip(&exact)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    assert!(true_error <= bound && bound <= tolerance);
                    println!(
                        "{}",
                        serde_json::json!({"kind":if repetition==0 {"warmup"}else{"measurement"},"dimension":n,"fixture":name,"mode":mode,"repetition":repetition,"setup_seconds":setup,"solve_seconds":solve,"certification_seconds":cert,"total_seconds":total,"certified":true,"error_l2_upper":bound,"true_error_l2":true_error,"residual_operations":if mode=="dense-directed" {n*n}else{plan.values.len()},"fallback_seconds":0.,"fallback_count":0,"structure_reused":true,"run_scope":"conditional_structured_residual_only","cost_scope":"same direct solve plus certificate; one-time sparse setup reported separately; dense materialization/oracle checks excluded"})
                    );
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"kind":"checks","negative_controls_passed":negative_checks,"status":"PASS_SCOPED","production_integration":false})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structure_rejects_duplicates_out_of_range_and_rank_independent_instability() {
        assert!(OwnedSparse::new(2, vec![0, 1, 2], vec![2, 1], vec![-2., -2.], 1).is_err());
        assert!(OwnedSparse::new(2, vec![0, 2, 2], vec![0, 1], vec![-1., 4.], 1).is_err());
    }
    #[test]
    fn sparse_and_dense_enclose_exact_cancellation_residual() {
        let p = fixture(4, 0.25);
        let x = vec![2f64.powi(40), -2f64.powi(40), 1., -2.];
        let b = (0..4)
            .map(|i| 2. * x[i] - if i + 1 < 4 { 0.125 * x[i + 1] } else { 0. })
            .collect::<Vec<_>>();
        let truth = exact_residual(&b, &x, 0.25);
        verify_intervals(&p.residual(11, 0.5, &b, &x).unwrap().0, &truth);
        verify_intervals(
            &dense_residual(4, &p.dense(), 0.5, &b, &x).unwrap().0,
            &truth,
        );
    }
}
