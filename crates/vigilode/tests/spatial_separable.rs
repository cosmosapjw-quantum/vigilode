//! The separable (Fourier) stage solver of `vigilode::spatial` (stage-lattice task SL-N5).
//!
//! Claims T1-T5 of `research/kernel_spectral_foundation_20261124/THEORY.md` §7: exact stage
//! solves on the sphere and the box, the stage mass identity, the inertia certificate, whole
//! IVPs against an independent dense matrix-exponential reference with conservation of the
//! area mass, `Verified` mode on a mismatched Jacobian, and refusals before any callback.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use vigilode::spatial::{
    DiffusionGrid, SEPARABLE_STAGE_CLIENT_ID, SeparableMode, SeparableReport, SeparableStages,
    SpatialRows,
};
use vigilode::{
    Atol, DriverWork, Jacobian, Options, Problem, Rodas5p, Solution, SolveError, Unsupported,
    VigilODE,
};

fn sphere(nt: usize, np: usize, kappa: f64) -> DiffusionGrid {
    DiffusionGrid::sphere(nt, np, 1.0, kappa).unwrap()
}

fn noise(n: usize, seed: u64) -> Vec<f64> {
    let mut z = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..n)
        .map(|_| {
            z ^= z << 13;
            z ^= z >> 7;
            z ^= z << 17;
            (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
        })
        .collect()
}

fn max_abs(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |m, x| m.max(x.abs()))
}

fn max_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).fold(0.0, |m, (x, y)| m.max((x - y).abs()))
}

fn mass(grid: &DiffusionGrid, u: &[f64]) -> f64 {
    u.iter()
        .enumerate()
        .map(|(i, v)| grid.cell_measure(i).unwrap() * v)
        .sum()
}

fn abs_mass(grid: &DiffusionGrid, u: &[f64]) -> f64 {
    u.iter()
        .enumerate()
        .map(|(i, v)| grid.cell_measure(i).unwrap() * v.abs())
        .sum()
}

/// `(z I - L - c I) x`.
fn apply_w(grid: &DiffusionGrid, z: f64, c: f64, x: &[f64]) -> Vec<f64> {
    let mut lx = vec![0.0; x.len()];
    grid.apply(x, &mut lx, SpatialRows::Serial).unwrap();
    x.iter()
        .zip(&lx)
        .map(|(xi, li)| z * xi - li - c * xi)
        .collect()
}

/// Dense `J = L + c I - diag(k)` by columns.
fn dense_jacobian(grid: &DiffusionGrid, c: f64, k: &[f64]) -> Vec<f64> {
    let n = grid.dimension();
    let mut jac = vec![0.0; n * n];
    let mut e = vec![0.0; n];
    let mut col = vec![0.0; n];
    for j in 0..n {
        e[j] = 1.0;
        grid.apply(&e, &mut col, SpatialRows::Serial).unwrap();
        for i in 0..n {
            jac[i * n + j] = col[i];
        }
        e[j] = 0.0;
        jac[j * n + j] += c - k[j];
    }
    jac
}

fn matmul(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0; n * n];
    for i in 0..n {
        for k in 0..n {
            let aik = a[i * n + k];
            if aik != 0.0 {
                for j in 0..n {
                    c[i * n + j] += aik * b[k * n + j];
                }
            }
        }
    }
    c
}

/// `exp(t J) y0` by scaling and squaring with a degree-24 Taylor polynomial (an independent
/// dense reference: no Fourier transform, no RODAS5P).
fn expm_apply(jac: &[f64], n: usize, t: f64, y0: &[f64]) -> Vec<f64> {
    let norm = (0..n)
        .map(|j| (0..n).map(|i| jac[i * n + j].abs()).sum::<f64>())
        .fold(0.0, f64::max)
        * t;
    let squarings = if norm > 0.25 {
        (norm / 0.25).log2().ceil() as u32
    } else {
        0
    };
    let scale = t / 2f64.powi(squarings as i32);
    let a: Vec<f64> = jac.iter().map(|v| v * scale).collect();
    let mut result = vec![0.0; n * n];
    let mut term = vec![0.0; n * n];
    for i in 0..n {
        result[i * n + i] = 1.0;
        term[i * n + i] = 1.0;
    }
    for degree in 1..=24 {
        term = matmul(&term, &a, n);
        for v in &mut term {
            *v /= degree as f64;
        }
        for (r, v) in result.iter_mut().zip(&term) {
            *r += v;
        }
    }
    for _ in 0..squarings {
        result = matmul(&result, &result, n);
    }
    (0..n)
        .map(|i| (0..n).map(|j| result[i * n + j] * y0[j]).sum())
        .collect()
}

/// Non-positive pivots of Gaussian elimination without pivoting on the dense `W`.
fn dense_nonpositive_pivots(mut w: Vec<f64>, n: usize) -> usize {
    let mut count = 0;
    for k in 0..n {
        let p = w[k * n + k];
        if p.is_nan() || p <= 0.0 {
            count += 1;
        }
        for i in k + 1..n {
            let m = w[i * n + k] / p;
            if m != 0.0 {
                for j in k..n {
                    w[i * n + j] -= m * w[k * n + j];
                }
            }
        }
    }
    count
}

/// `u' = L u + c u - k u` with an exact JVP.
fn problem(grid: &DiffusionGrid, c: f64, k: Vec<f64>, calls: Arc<AtomicUsize>) -> Problem {
    let (rhs_grid, jvp_grid) = (grid.clone(), grid.clone());
    let k = Arc::new(k);
    let rhs_k = Arc::clone(&k);
    Problem::new(grid.dimension(), move |_t, u, du| {
        calls.fetch_add(1, Ordering::Relaxed);
        rhs_grid.apply(u, du, SpatialRows::Serial)?;
        for ((d, &v), &ki) in du.iter_mut().zip(u).zip(rhs_k.iter()) {
            *d += (c - ki) * v;
        }
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |_t, _u, v, out| {
        jvp_grid.apply(v, out, SpatialRows::Serial)?;
        for ((o, &vi), &ki) in out.iter_mut().zip(v).zip(k.iter()) {
            *o += (c - ki) * vi;
        }
        Ok(())
    }))
    .autonomous()
}

fn smooth_field(grid: &DiffusionGrid, width: usize) -> Vec<f64> {
    (0..grid.dimension())
        .map(|i| {
            let (r, x) = ((i / width) as f64, (i % width) as f64);
            1.0 + 0.4 * (0.9 * x + r).sin() + 0.25 * (0.7 * r).cos() * (1.7 * x).cos()
        })
        .collect()
}

fn report(solution: &Solution) -> SeparableReport {
    match &solution.stats.driver_work {
        DriverWork::SeparableClient { client, report, .. } => {
            assert_eq!(*client, SEPARABLE_STAGE_CLIENT_ID);
            *report
        }
        other => panic!("not the separable client: {other:?}"),
    }
}

fn grids() -> Vec<(&'static str, DiffusionGrid)> {
    vec![
        ("sphere-7x12", sphere(7, 12, 0.7)),
        ("sphere-8x16", sphere(8, 16, 1.3)),
        ("sphere-9x15", sphere(9, 15, 0.2)),
        ("sphere-16x32", sphere(16, 32, 1.0)),
        (
            "box-8x6",
            DiffusionGrid::periodic_box([8, 6], [1.0, 0.7], [0.3, 1.1]).unwrap(),
        ),
        (
            "box-5x7",
            DiffusionGrid::periodic_box([5, 7], [2.0, 1.0], [1.0, 0.4]).unwrap(),
        ),
        (
            "box-4x6x5",
            DiffusionGrid::periodic_box([4, 6, 5], [1.0, 1.0, 2.0], [0.5, 0.2, 1.5]).unwrap(),
        ),
    ]
}

/// T1, T2: backward error and stage mass identity of single stage solves.
#[test]
fn separable_stage_solves_are_exact_and_mass_consistent() {
    let (mut worst_backward, mut worst_mass) = (0.0f64, 0.0f64);
    for (name, grid) in grids() {
        let total_measure: f64 = (0..grid.dimension())
            .map(|i| grid.cell_measure(i).unwrap())
            .sum();
        for (z, c) in [
            (0.5, 0.0),
            (10.0, -1.0),
            (3.0, 1.0),
            (1.0e4, 0.0),
            (1.0e-3, 0.0),
        ] {
            let stages = grid.separable_stages(c).unwrap();
            let bounds = stages.spectral_bounds();
            for seed in 0..3 {
                let b = noise(grid.dimension(), seed + 17);
                let mut x = b.clone();
                stages.solve_stage(z, &mut x).unwrap();
                let residual = max_diff(&b, &apply_w(&grid, z, c, &x));
                let scale = ((z - c).abs() + bounds.radius_bound) * max_abs(&x) + max_abs(&b);
                let backward = residual / scale;
                let defect =
                    ((z - c) * mass(&grid, &x) - mass(&grid, &b)).abs() / (total_measure * scale);
                assert!(
                    backward <= 1.0e-13,
                    "{name} z={z} c={c}: backward {backward:.2e}"
                );
                assert!(
                    defect <= 1.0e-13,
                    "{name} z={z} c={c}: mass defect {defect:.2e}"
                );
                worst_backward = worst_backward.max(backward);
                worst_mass = worst_mass.max(defect);
            }
        }
    }
    println!(
        "SL_N5_T1T2 {{\"max_backward_error\":{worst_backward:.3e},\"max_mass_defect\":{worst_mass:.3e}}}"
    );
}

/// T4 and THEORY.md §4.2: pivot signs of the mode systems count the eigenvalues of `J` at or
/// above `z`, against Gaussian elimination of the dense `W` in cell order.
#[test]
fn inertia_counts_the_modes_beyond_the_stage_pencil() {
    let mut crossings_seen = 0;
    for (name, grid) in [
        ("sphere-6x8", sphere(6, 8, 0.6)),
        ("sphere-5x9", sphere(5, 9, 1.4)),
        (
            "box-4x5",
            DiffusionGrid::periodic_box([4, 5], [1.0, 1.0], [0.6, 0.9]).unwrap(),
        ),
    ] {
        let n = grid.dimension();
        let zeros = vec![0.0; n];
        for (z, c) in [
            (5.0, 0.0),
            (5.0, 6.0),
            (2.0, 9.0),
            (0.7, 40.0),
            (11.0, 12.5),
        ] {
            let stages = grid.separable_stages(c).unwrap();
            let fourier = stages.inertia(z);
            let jac = dense_jacobian(&grid, c, &zeros);
            let w: Vec<f64> = (0..n * n)
                .map(|ij| {
                    let (i, j) = (ij / n, ij % n);
                    if i == j { z - jac[ij] } else { -jac[ij] }
                })
                .collect();
            let dense = dense_nonpositive_pivots(w, n);
            assert_eq!(fourier, dense, "{name} z={z} c={c}");
            crossings_seen += fourier;
            let mut b = noise(n, 3);
            let outcome = stages.solve_stage(z, &mut b);
            assert_eq!(outcome.is_err(), fourier > 0, "{name} z={z} c={c}");
        }
    }
    assert!(crossings_seen > 0);
}

fn options(rtol: f64, atol: f64) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(atol),
        ..Options::default()
    }
}

/// T3: whole IVPs against the dense matrix exponential; the area mass of a pure diffusion is
/// conserved to round-off on the separable route.
#[test]
fn separable_route_conserves_mass_and_matches_the_reference() {
    let cases = [
        ("sphere-8x16", sphere(8, 16, 1.0), 16, 0.0),
        ("sphere-7x10", sphere(7, 10, 0.4), 10, 0.0),
        (
            "box-12x10",
            DiffusionGrid::periodic_box([12, 10], [1.0, 1.0], [0.02, 0.05]).unwrap(),
            12,
            -0.5,
        ),
    ];
    for (name, grid, width, c) in cases {
        let n = grid.dimension();
        let y0 = smooth_field(&grid, width);
        let span = (0.0, 1.0);
        let zeros = vec![0.0; n];
        let reference = expm_apply(&dense_jacobian(&grid, c, &zeros), n, span.1, &y0);
        let scale = max_abs(&reference);
        let opts = options(1.0e-8, 1.0e-10);
        let p = problem(&grid, c, zeros.clone(), Arc::new(AtomicUsize::new(0)));
        for mode in [SeparableMode::Trusted, SeparableMode::default()] {
            let stages = grid.separable_stages(c).unwrap().with_mode(mode).unwrap();
            let solution = VigilODE::solve_separable(&p, span, &y0, &opts, &stages).unwrap();
            assert!(solution.is_success(), "{name} {mode:?}");
            let y = solution.final_state().unwrap();
            let error = max_diff(y, &reference);
            assert!(
                error <= 1.0e-6 * scale,
                "{name} {mode:?}: error {error:.2e}"
            );
            let r = report(&solution);
            assert_eq!(r.refused_preparations, 0, "{name}");
            assert!(r.max_mass_defect <= 1.0e-12, "{name}: {r:?}");
            if let SeparableMode::Verified { .. } = mode {
                assert_eq!(r.corrections, 0, "{name}: the declared J is exact");
                assert_eq!(r.residual_checks, r.solves, "{name}");
                assert!(r.max_backward_error <= 1.0e-12, "{name}: {r:?}");
            }
            let drift = (mass(&grid, y) - mass(&grid, &y0)).abs() / abs_mass(&grid, &y0);
            if c == 0.0 {
                assert!(drift <= 1.0e-13, "{name}: mass drift {drift:.2e}");
            }
            println!(
                "SL_N5_T3 {{\"case\":\"{name}\",\"mode\":\"{mode:?}\",\"error\":{error:.3e},\"attempts\":{},\"solves\":{},\"mass_drift\":{drift:.3e}}}",
                solution.stats.attempts, r.solves,
            );
        }
        let shipped = Rodas5p::solve(&p, span, &y0, &opts).unwrap();
        assert!(shipped.is_success(), "{name}: shipped");
        let ys = shipped.final_state().unwrap();
        let error = max_diff(ys, &reference);
        assert!(error <= 1.0e-6 * scale, "{name} shipped: error {error:.2e}");
        println!(
            "SL_N5_T3 {{\"case\":\"{name}\",\"mode\":\"shipped\",\"error\":{error:.3e},\"attempts\":{},\"mass_drift\":{:.3e}}}",
            shipped.stats.attempts,
            (mass(&grid, ys) - mass(&grid, &y0)).abs() / abs_mass(&grid, &y0)
        );
    }
}

/// T5: `Verified` mode on a ring-dependent decay `-k_r u` declared as one mean rate. It
/// corrects the stage solves and matches the reference; a far mismatch with one correction
/// is refused solve by solve, and the driver shrinks the step.
#[test]
fn verified_mode_corrects_a_mismatched_reaction_and_refuses_beyond_its_reach() {
    let (nt, np) = (8, 16);
    let grid = sphere(nt, np, 1.0);
    let n = grid.dimension();
    let k: Vec<f64> = (0..n)
        .map(|i| 1.0 + 0.2 * ((i / np) as f64).sin())
        .collect();
    let mean = k.iter().sum::<f64>() / n as f64;
    let y0 = smooth_field(&grid, np);
    let span = (0.0, 1.0);
    let reference = expm_apply(&dense_jacobian(&grid, 0.0, &k), n, span.1, &y0);
    let scale = max_abs(&reference);
    let p = problem(&grid, 0.0, k.clone(), Arc::new(AtomicUsize::new(0)));
    let opts = options(1.0e-8, 1.0e-10);
    let verified = grid.separable_stages(-mean).unwrap();
    let solution = VigilODE::solve_separable(&p, span, &y0, &opts, &verified).unwrap();
    assert!(solution.is_success());
    let r = report(&solution);
    assert!(r.corrections > 0, "{r:?}");
    assert!(r.max_backward_error <= 1.0e-10, "{r:?}");
    let error = max_diff(solution.final_state().unwrap(), &reference);
    assert!(error <= 1.0e-6 * scale, "verified error {error:.2e}");
    println!(
        "SL_N5_T5 {{\"mode\":\"verified\",\"error\":{error:.3e},\"corrections_per_solve\":{:.3},\"refused_solves\":{},\"attempts\":{}}}",
        r.corrections as f64 / r.solves as f64,
        r.refused_solves,
        solution.stats.attempts
    );
    // The same mismatch, trusted: the stage solves of a W-method with a wrong Jacobian;
    // reported, not asserted.
    let trusted = grid
        .separable_stages(-mean)
        .unwrap()
        .with_mode(SeparableMode::Trusted)
        .unwrap();
    if let Ok(s) = VigilODE::solve_separable(&p, span, &y0, &opts, &trusted)
        && let Some(y) = s.final_state()
    {
        println!(
            "SL_N5_T5 {{\"mode\":\"trusted-mismatched\",\"error\":{:.3e},\"attempts\":{}}}",
            max_diff(y, &reference),
            s.stats.attempts
        );
    }
    // A far mismatch (decay 40 on every other ring, declared 0) with one correction.
    let far: Vec<f64> = (0..n).map(|i| 40.0 * ((i / np) % 2) as f64).collect();
    let p_far = problem(&grid, 0.0, far, Arc::new(AtomicUsize::new(0)));
    let strict = grid
        .separable_stages(0.0)
        .unwrap()
        .with_mode(SeparableMode::Verified {
            max_corrections: 1,
            tolerance: 1.0e-12,
        })
        .unwrap();
    match VigilODE::solve_separable(&p_far, (0.0, 0.2), &y0, &opts, &strict) {
        Ok(s) => {
            let r = report(&s);
            assert!(r.refused_solves > 0, "{r:?}");
            println!(
                "SL_N5_T5 {{\"mode\":\"far-mismatch\",\"success\":{},\"refused_solves\":{},\"attempts\":{}}}",
                s.is_success(),
                r.refused_solves,
                s.stats.attempts
            );
        }
        Err(SolveError::Unsupported(u)) => panic!("refused before the run: {u}"),
        Err(other) => println!("SL_N5_T5 {{\"mode\":\"far-mismatch\",\"error\":\"{other}\"}}"),
    }
}

/// Refusals before any callback: dense output (by the facade's validation), a grid of
/// another size, `Verified` without a JVP. A finite-difference problem in `Trusted` mode runs (the declared J replaces it).
#[test]
fn separable_requests_outside_the_matrix_free_route_are_refused_before_any_callback() {
    let grid = sphere(6, 8, 0.5);
    let other = sphere(5, 8, 0.5);
    let n = grid.dimension();
    let y0 = smooth_field(&grid, 8);
    let calls = Arc::new(AtomicUsize::new(0));
    let p = problem(&grid, 0.0, vec![0.0; n], Arc::clone(&calls));
    let stages = grid.separable_stages(0.0).unwrap();
    let dense = Options {
        dense_output: true,
        ..options(1e-6, 1e-9)
    };
    let refused = |result: Result<Solution, SolveError>| {
        assert!(
            matches!(
                result,
                Err(SolveError::Unsupported(
                    Unsupported::SeparableStagesRequireMatrixFreeRoute
                ))
            ),
            "{result:?}"
        );
    };
    // Dense output on a JVP problem is refused by the facade's own validation first.
    assert!(matches!(
        VigilODE::solve_separable(&p, (0.0, 1.0), &y0, &dense, &stages),
        Err(SolveError::Unsupported(
            Unsupported::JvpWithMassOrDenseOutput
        ))
    ));
    refused(VigilODE::solve_separable(
        &p,
        (0.0, 1.0),
        &y0,
        &options(1e-6, 1e-9),
        &other.separable_stages(0.0).unwrap(),
    ));
    let fd_grid = grid.clone();
    let fd_calls = Arc::clone(&calls);
    let fd = Problem::new(n, move |_t, u, du| {
        fd_calls.fetch_add(1, Ordering::Relaxed);
        fd_grid.apply(u, du, SpatialRows::Serial)
    })
    .jacobian(Jacobian::FiniteDifference)
    .autonomous();
    refused(VigilODE::solve_separable(
        &fd,
        (0.0, 1.0),
        &y0,
        &options(1e-6, 1e-9),
        &stages,
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0, "a refusal ran a callback");
    let trusted: SeparableStages = stages.with_mode(SeparableMode::Trusted).unwrap();
    let s =
        VigilODE::solve_separable(&fd, (0.0, 1.0), &y0, &options(1e-6, 1e-9), &trusted).unwrap();
    assert!(s.is_success());
    assert!(matches!(
        grid.separable_stages(0.0)
            .unwrap()
            .with_mode(SeparableMode::Verified {
                max_corrections: 2,
                tolerance: 0.0
            }),
        Err(SolveError::InvalidInput(_))
    ));
    assert!(grid.separable_stages(f64::NAN).is_err());
}

/// Diagnostic (SL-N5 T6): whole solves on sphere grids, shipped matrix-free route against the
/// separable route, error against a tight separable self-reference, wall time and counters.
/// Host-only; the owner-host cells are preregistered in the run's PLAN.json.
#[test]
#[ignore = "timing diagnostic; run with --ignored --nocapture"]
fn measure_separable_against_the_shipped_matrix_free_route() {
    use std::time::Instant;
    for (nt, np) in [(32usize, 64usize), (64, 128), (128, 256)] {
        let grid = sphere(nt, np, 1.0e-2);
        let n = grid.dimension();
        let y0 = smooth_field(&grid, np);
        let span = (0.0, 1.0);
        let p = problem(&grid, 0.0, vec![0.0; n], Arc::new(AtomicUsize::new(0)));
        let trusted = grid
            .separable_stages(0.0)
            .unwrap()
            .with_mode(SeparableMode::Trusted)
            .unwrap();
        let verified = grid.separable_stages(0.0).unwrap();
        let reference = VigilODE::solve_separable(&p, span, &y0, &options(1e-11, 1e-13), &trusted)
            .unwrap()
            .final_state()
            .unwrap()
            .to_vec();
        let scale = max_abs(&reference);
        let bounds = trusted.spectral_bounds();
        let opts = options(1.0e-6, 1.0e-9);
        // The shipped arm runs last: at 128x256 it took more than 8 minutes per solve on the
        // 2-vCPU host, so the separable lines of every grid print first.
        let repeats = if n > 10_000 { 1 } else { 5 };
        for arm in ["separable-trusted", "separable-verified", "shipped"] {
            let run = || match arm {
                "shipped" => Rodas5p::solve(&p, span, &y0, &opts).unwrap(),
                "separable-trusted" => {
                    VigilODE::solve_separable(&p, span, &y0, &opts, &trusted).unwrap()
                }
                _ => VigilODE::solve_separable(&p, span, &y0, &opts, &verified).unwrap(),
            };
            let mut times = Vec::new();
            let mut last = None;
            for _ in 0..repeats {
                let start = Instant::now();
                let s = run();
                times.push(start.elapsed().as_secs_f64());
                last = Some(s);
            }
            times.sort_by(f64::total_cmp);
            let s = last.unwrap();
            let y = s.final_state().unwrap();
            let c = &s.stats.counters;
            println!(
                "{{\"sl_n5\":\"whole_solve\",\"grid\":\"{nt}x{np}\",\"n\":{n},\"arm\":\"{arm}\",\"median_s\":{:.5},\"relative_error\":{:.3e},\"attempts\":{},\"rhs_calls\":{},\"jvp_calls\":{},\"linear_iterations\":{},\"direct_solve_calls\":{},\"mass_drift\":{:.3e},\"rho_bar\":{:.4e},\"explicit_step_bound\":{:.4e}}}",
                times[times.len() / 2],
                max_diff(y, &reference) / scale,
                s.stats.attempts,
                c.rhs_calls,
                c.jvp_calls,
                c.linear_iterations,
                c.direct_solve_calls,
                (mass(&grid, y) - mass(&grid, &y0)).abs() / abs_mass(&grid, &y0),
                bounds.radius_bound,
                bounds.explicit_step_bound,
            );
        }
    }
}

/// Diagnostic (SL-N5 T6, stage level): one stage system `(I - sigma L) x = r` on sphere grids,
/// native GMRES with the identity and with the longitude line preconditioner
/// (`DiffusionGrid::longitude_preconditioner`, the P1 comparator of SL-N5) against the
/// separable solve of `(z I - L) x = z r`, `z = 1/sigma`. Median wall time and true residual.
#[test]
#[ignore = "timing diagnostic; run with --ignored --nocapture"]
fn measure_separable_against_preconditioned_gmres_stage_solves() {
    use rodas5p_core::{
        ClosureOperator, CoreError, IdentityPreconditioner, LinearOperator, Preconditioner,
        WorkCounters,
    };
    use rodas5p_krylov::{GmresConfig, solve_gmres};
    use std::time::Instant;
    let gamma = 0.211_937_563_194_290_14;
    for (nt, np) in [(32usize, 64usize), (64, 128), (128, 256)] {
        let grid = sphere(nt, np, 1.0e-2);
        let n = grid.dimension();
        let stages = grid
            .separable_stages(0.0)
            .unwrap()
            .with_mode(SeparableMode::Trusted)
            .unwrap();
        let rhs = noise(n, 5);
        for h in [1.0e-3, 1.0e-2, 1.0e-1] {
            let sigma = h * gamma;
            let op = ClosureOperator::new(n, |v: &[f64], out: &mut [f64]| {
                grid.apply(v, out, SpatialRows::Serial)
                    .map_err(|e| CoreError::Dimension(e.to_string()))?;
                for (a, b) in out.iter_mut().zip(v) {
                    *a = *b - sigma * (*a);
                }
                Ok(())
            });
            let relative_residual = |x: &[f64]| {
                let mut ax = vec![0.0; n];
                op.apply(x, &mut ax).unwrap();
                max_diff(&ax, &rhs) / max_abs(&rhs)
            };
            let config = GmresConfig {
                restart: 30,
                max_arnoldi: 3000,
                rtol: 1e-10,
                atol: 0.0,
            };
            let identity = IdentityPreconditioner::new(n);
            let line = grid
                .longitude_preconditioner(sigma, SpatialRows::Serial)
                .unwrap();
            for (name, pc) in [
                ("gmres-identity", &identity as &dyn Preconditioner),
                ("gmres-longitude", &line),
            ] {
                let mut times = Vec::new();
                let mut last = None;
                for _ in 0..3 {
                    let mut counters = WorkCounters::default();
                    let start = Instant::now();
                    let outcome = solve_gmres(&op, pc, &rhs, None, &config, &mut counters);
                    times.push(start.elapsed().as_secs_f64());
                    match outcome {
                        Ok(s) => last = Some(s),
                        Err(e) => {
                            println!(
                                "{{\"sl_n5\":\"stage\",\"grid\":\"{nt}x{np}\",\"h\":{h:e},\"arm\":\"{name}\",\"seconds\":{:.6},\"converged\":false,\"error\":\"{e}\"}}",
                                times[times.len() - 1]
                            );
                            break;
                        }
                    }
                }
                times.sort_by(f64::total_cmp);
                let Some(s) = last else { continue };
                println!(
                    "{{\"sl_n5\":\"stage\",\"grid\":\"{nt}x{np}\",\"h\":{h:e},\"arm\":\"{name}\",\"median_s\":{:.6},\"converged\":{},\"matvecs\":{},\"relative_residual\":{:.3e}}}",
                    times[1],
                    s.converged,
                    s.matvecs,
                    relative_residual(&s.x)
                );
            }
            let mut times = Vec::new();
            let mut x = Vec::new();
            for _ in 0..5 {
                x = rhs.iter().map(|v| v / sigma).collect::<Vec<f64>>();
                let start = Instant::now();
                stages.solve_stage(1.0 / sigma, &mut x).unwrap();
                times.push(start.elapsed().as_secs_f64());
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{{\"sl_n5\":\"stage\",\"grid\":\"{nt}x{np}\",\"h\":{h:e},\"arm\":\"separable\",\"median_s\":{:.6},\"converged\":true,\"matvecs\":0,\"relative_residual\":{:.3e},\"note\":\"includes plan and factor\"}}",
                times[2],
                relative_residual(&x)
            );
        }
    }
}

// KS-N7: the two-for-one real FFT of the periodic box (`BoxTransform::PairedReal`), against
// the full complex transform (`BoxTransform::Complex`, the default). `BoxTransform` is selected
// by name (`FromStr`), so these tests do not depend on its re-export path.
// Notes: research/kernel_spectral_foundation_20261124/ks_n7/NOTES.txt.

/// The `(z, c)` range of `separable_stage_solves_are_exact_and_mass_consistent`.
const KS_N7_SHIFTS: [(f64, f64); 5] = [
    (0.5, 0.0),
    (10.0, -1.0),
    (3.0, 1.0),
    (1.0e4, 0.0),
    (1.0e-3, 0.0),
];

fn ks_n7_box<const D: usize>(
    cells: [usize; D],
    lengths: [f64; D],
    kappa: [f64; D],
) -> DiffusionGrid {
    DiffusionGrid::periodic_box(cells, lengths, kappa).unwrap()
}

/// Every box grid of the tests above (T1/T2: 8x6, 5x7, 4x6x5; T4: 4x5; T3: 12x10), then even
/// and odd sizes: the minimum axis length 2, odd `nx`, odd row counts (`ny nz` odd: the last
/// row is packed alone), prime `nx` beyond rustfft's butterflies, 3-D boxes, and a larger
/// grid. The third entry of each tuple is `nx`.
fn ks_n7_box_grids() -> Vec<(&'static str, DiffusionGrid, usize)> {
    vec![
        ("box-8x6", ks_n7_box([8, 6], [1.0, 0.7], [0.3, 1.1]), 8),
        ("box-5x7", ks_n7_box([5, 7], [2.0, 1.0], [1.0, 0.4]), 5),
        (
            "box-4x6x5",
            ks_n7_box([4, 6, 5], [1.0, 1.0, 2.0], [0.5, 0.2, 1.5]),
            4,
        ),
        ("box-4x5", ks_n7_box([4, 5], [1.0, 1.0], [0.6, 0.9]), 4),
        (
            "box-12x10",
            ks_n7_box([12, 10], [1.0, 1.0], [0.02, 0.05]),
            12,
        ),
        ("box-2x2", ks_n7_box([2, 2], [1.0, 1.0], [0.8, 0.3]), 2),
        ("box-7x4", ks_n7_box([7, 4], [1.3, 0.9], [0.4, 0.7]), 7),
        ("box-6x9", ks_n7_box([6, 9], [1.0, 2.0], [1.2, 0.5]), 6),
        ("box-37x6", ks_n7_box([37, 6], [1.0, 0.5], [0.02, 0.3]), 37),
        (
            "box-3x3x3",
            ks_n7_box([3, 3, 3], [1.0, 1.0, 1.0], [0.3, 0.6, 0.9]),
            3,
        ),
        (
            "box-5x3x7",
            ks_n7_box([5, 3, 7], [1.0, 0.5, 1.5], [0.7, 0.2, 0.4]),
            5,
        ),
        (
            "box-2x5x3",
            ks_n7_box([2, 5, 3], [0.4, 1.0, 1.0], [0.1, 0.9, 0.5]),
            2,
        ),
        (
            "box-9x2x2",
            ks_n7_box([9, 2, 2], [1.0, 0.3, 0.3], [0.5, 0.05, 0.08]),
            9,
        ),
        (
            "box-16x12x4",
            ks_n7_box([16, 12, 4], [1.0, 1.0, 0.5], [0.3, 0.3, 0.1]),
            16,
        ),
        (
            "box-33x17",
            ks_n7_box([33, 17], [1.0, 1.0], [0.05, 0.05]),
            33,
        ),
        (
            "box-64x48",
            ks_n7_box([64, 48], [1.0, 0.75], [0.01, 0.02]),
            64,
        ),
        // Added at merge (review of KS-N7, finding F2): two packing batches with a lone odd
        // row in the later one, and more than 4096 cells along x (one pair per batch, then a
        // lone row).
        (
            "box-100x97",
            ks_n7_box([100, 97], [1.0, 1.0], [0.02, 0.03]),
            100,
        ),
        (
            "box-4097x3",
            ks_n7_box([4097, 3], [1.0, 1.0], [0.01, 0.5]),
            4097,
        ),
    ]
}

/// Right-hand sides: the three noise seeds of the T1/T2 test, and one whose rows alternate
/// in scale by 1e8 (the two rows of a pair share one complex FFT).
fn ks_n7_right_hand_sides(n: usize, nx: usize) -> Vec<(&'static str, Vec<f64>)> {
    let mut sides: Vec<(&'static str, Vec<f64>)> = ["seed17", "seed18", "seed19"]
        .into_iter()
        .zip(17u64..)
        .map(|(name, seed)| (name, noise(n, seed)))
        .collect();
    let mut scaled = noise(n, 23);
    for (row, chunk) in scaled.chunks_mut(nx).enumerate() {
        if row % 2 == 1 {
            chunk.iter_mut().for_each(|v| *v *= 1.0e-8);
        }
    }
    sides.push(("rows-1e8-apart", scaled));
    sides
}

/// The transform named `name`, parsed into the setter's parameter type.
fn ks_n7_parse(stages: &SeparableStages, name: &str) -> Result<SeparableStages, SolveError> {
    Ok(stages.clone().with_box_transform(name.parse()?))
}

fn ks_n7_with(stages: SeparableStages, transform: &str) -> SeparableStages {
    ks_n7_parse(&stages, transform).unwrap()
}

/// KS-N7 agreement: `PairedReal` returns the `Complex` solution to round-off on every box
/// grid, shift and right-hand side. Stated bound: on the box `W = (z - c) I - L` is an
/// M-matrix with `W 1 = (z - c) 1`, so `||W^{-1}||_inf = 1/(z - c)`; two solves whose
/// residuals meet the T1 bound `1e-13 S`, `S = ((z - c) + rho_bar) ||x||_inf + ||b||_inf`,
/// differ by at most `2e-13 S / (z - c)`. Asserted: `||x_p - x_c||_inf <= 1e-13 S / (z - c)`.
#[test]
fn paired_real_box_transform_agrees_with_the_complex_transform() {
    let (mut worst_relative, mut worst_normalized) = (0.0f64, 0.0f64);
    let (mut cases, mut differing) = (0usize, 0usize);
    let mut worst_case = String::new();
    for (name, grid, nx) in ks_n7_box_grids() {
        for (z, c) in KS_N7_SHIFTS {
            let complex = grid.separable_stages(c).unwrap();
            assert_eq!(complex.box_transform().to_string(), "complex", "{name}");
            let paired = ks_n7_with(complex.clone(), "paired-real");
            assert_eq!(paired.box_transform().to_string(), "paired-real", "{name}");
            let rho_bar = complex.spectral_bounds().radius_bound;
            for (side, b) in ks_n7_right_hand_sides(grid.dimension(), nx) {
                let (mut xc, mut xp) = (b.clone(), b.clone());
                complex.solve_stage(z, &mut xc).unwrap();
                paired.solve_stage(z, &mut xp).unwrap();
                let diff = max_diff(&xp, &xc);
                let scale = ((z - c).abs() + rho_bar) * max_abs(&xc) + max_abs(&b);
                let bound = 1.0e-13 * scale / (z - c);
                assert!(
                    diff <= bound,
                    "{name} z={z} c={c} {side}: |x_p - x_c| {diff:.2e} above {bound:.2e}"
                );
                let relative = diff / max_abs(&xc);
                if relative > worst_relative {
                    worst_relative = relative;
                    worst_case = format!("{name} z={z} c={c} {side}");
                }
                worst_normalized = worst_normalized.max(diff * (z - c) / scale);
                cases += 1;
                differing += usize::from(diff > 0.0);
            }
        }
    }
    // The paired arm is a different computation: some of its roundings must differ.
    assert!(
        differing > 0,
        "PairedReal returned the Complex bits on every case"
    );
    println!(
        "\nKS_N7_AGREEMENT {{\"cases\":{cases},\"differing_cases\":{differing},\"max_relative_difference\":{worst_relative:.3e},\"max_normalized_difference\":{worst_normalized:.3e},\"bound_normalized\":1e-13,\"worst_case\":\"{worst_case}\"}}"
    );
}

/// KS-N7: the T1 backward-error bound and the T2 stage mass identity hold for `PairedReal`
/// with the scales of `separable_stage_solves_are_exact_and_mass_consistent`, on every box
/// grid, shift and right-hand side; the `Complex` arm is measured on the same cases.
#[test]
fn paired_real_box_solves_meet_the_t1_t2_bounds() {
    for transform in ["paired-real", "complex"] {
        let (mut worst_backward, mut worst_mass) = (0.0f64, 0.0f64);
        let mut solves = 0usize;
        for (name, grid, nx) in ks_n7_box_grids() {
            let total_measure: f64 = (0..grid.dimension())
                .map(|i| grid.cell_measure(i).unwrap())
                .sum();
            for (z, c) in KS_N7_SHIFTS {
                let stages = ks_n7_with(grid.separable_stages(c).unwrap(), transform);
                let bounds = stages.spectral_bounds();
                for (side, b) in ks_n7_right_hand_sides(grid.dimension(), nx) {
                    let mut x = b.clone();
                    stages.solve_stage(z, &mut x).unwrap();
                    let residual = max_diff(&b, &apply_w(&grid, z, c, &x));
                    let scale = ((z - c).abs() + bounds.radius_bound) * max_abs(&x) + max_abs(&b);
                    let backward = residual / scale;
                    let defect = ((z - c) * mass(&grid, &x) - mass(&grid, &b)).abs()
                        / (total_measure * scale);
                    assert!(
                        backward <= 1.0e-13,
                        "{transform} {name} z={z} c={c} {side}: backward {backward:.2e}"
                    );
                    assert!(
                        defect <= 1.0e-13,
                        "{transform} {name} z={z} c={c} {side}: mass defect {defect:.2e}"
                    );
                    worst_backward = worst_backward.max(backward);
                    worst_mass = worst_mass.max(defect);
                    solves += 1;
                }
            }
        }
        println!(
            "\nKS_N7_T1T2 {{\"transform\":\"{transform}\",\"solves\":{solves},\"max_backward_error\":{worst_backward:.3e},\"max_mass_defect\":{worst_mass:.3e},\"bound\":1e-13}}"
        );
    }
}

/// KS-N7 client wiring: whole IVPs on the separable route with `PairedReal` (T3 cases of the
/// box: the 12x10 box of T3 and the 3-D box of T1/T2) match the dense matrix-exponential
/// reference within the T3 envelope, keep the stage mass identity, and agree with the
/// `Complex` route within the same envelope.
#[test]
fn paired_real_box_route_matches_the_reference_and_the_complex_route() {
    let cases = [
        (
            "box-12x10",
            ks_n7_box([12, 10], [1.0, 1.0], [0.02, 0.05]),
            12,
            -0.5,
        ),
        (
            "box-4x6x5",
            ks_n7_box([4, 6, 5], [1.0, 1.0, 2.0], [0.5, 0.2, 1.5]),
            4,
            -0.5,
        ),
    ];
    for (name, grid, width, c) in cases {
        let n = grid.dimension();
        let y0 = smooth_field(&grid, width);
        let span = (0.0, 1.0);
        let zeros = vec![0.0; n];
        let reference = expm_apply(&dense_jacobian(&grid, c, &zeros), n, span.1, &y0);
        let scale = max_abs(&reference);
        let opts = options(1.0e-8, 1.0e-10);
        let p = problem(&grid, c, zeros, Arc::new(AtomicUsize::new(0)));
        for mode in [SeparableMode::Trusted, SeparableMode::default()] {
            let mut finals = Vec::new();
            for transform in ["complex", "paired-real"] {
                let stages = ks_n7_with(
                    grid.separable_stages(c).unwrap().with_mode(mode).unwrap(),
                    transform,
                );
                let solution = VigilODE::solve_separable(&p, span, &y0, &opts, &stages).unwrap();
                assert!(solution.is_success(), "{name} {mode:?} {transform}");
                let y = solution.final_state().unwrap();
                let error = max_diff(y, &reference);
                assert!(
                    error <= 1.0e-6 * scale,
                    "{name} {mode:?} {transform}: error {error:.2e}"
                );
                let r = report(&solution);
                assert_eq!(r.refused_preparations, 0, "{name} {transform}");
                assert!(r.max_mass_defect <= 1.0e-12, "{name} {transform}: {r:?}");
                if let SeparableMode::Verified { .. } = mode {
                    assert_eq!(
                        r.corrections, 0,
                        "{name} {transform}: the declared J is exact"
                    );
                    assert!(r.max_backward_error <= 1.0e-12, "{name} {transform}: {r:?}");
                }
                println!(
                    "\nKS_N7_T3 {{\"case\":\"{name}\",\"mode\":\"{mode:?}\",\"transform\":\"{transform}\",\"error\":{error:.3e},\"attempts\":{},\"solves\":{},\"max_mass_defect\":{:.3e},\"max_backward_error\":{:.3e}}}",
                    solution.stats.attempts, r.solves, r.max_mass_defect, r.max_backward_error
                );
                finals.push(y.to_vec());
            }
            let between = max_diff(&finals[0], &finals[1]);
            assert!(
                between <= 1.0e-6 * scale,
                "{name} {mode:?}: routes differ by {between:.2e}"
            );
            println!(
                "\nKS_N7_T3_ROUTES {{\"case\":\"{name}\",\"mode\":\"{mode:?}\",\"max_difference\":{between:.3e},\"relative_difference\":{:.3e}}}",
                between / scale
            );
        }
    }
}

/// KS-N7 selection: `Complex` is the default; names parse and print; a sphere ignores the
/// setting (identical bits); `stage_solver` reuses its plans and buffers without carrying
/// state from one solve to the next (identical bits to fresh `solve_stage` calls).
#[test]
fn box_transform_selection_defaults_parses_and_reuses_prepared_solvers() {
    let box_grid = ks_n7_box([7, 5, 3], [1.0, 1.0, 1.0], [0.4, 0.5, 0.6]);
    let stages = box_grid.separable_stages(0.0).unwrap();
    assert_eq!(stages.box_transform().to_string(), "complex");
    for (name, canonical) in [
        ("complex", "complex"),
        ("paired-real", "paired-real"),
        ("paired", "paired-real"),
    ] {
        let s = ks_n7_with(stages.clone(), name);
        assert_eq!(s.box_transform().to_string(), canonical);
        assert_eq!(s.box_transform().name(), canonical);
    }
    assert!(matches!(
        ks_n7_parse(&stages, "real"),
        Err(SolveError::InvalidInput(_))
    ));
    // The sphere ignores the box transform.
    let ball = sphere(7, 12, 0.7);
    let b = noise(ball.dimension(), 3);
    let mut solutions = Vec::new();
    for transform in ["complex", "paired-real"] {
        let s = ks_n7_with(ball.separable_stages(-0.3).unwrap(), transform);
        let mut x = b.clone();
        s.solve_stage(2.5, &mut x).unwrap();
        solutions.push(x);
    }
    assert_eq!(
        solutions[0], solutions[1],
        "the sphere must ignore the box transform"
    );
    // A prepared solver gives the bits of fresh solves, solve after solve, on both arms and the
    // sphere; it refuses a wrong length and a shift beyond the stage pencil like solve_stage.
    for (name, grid) in [
        ("box-7x5x3", box_grid.clone()),
        ("sphere-7x12", ball.clone()),
    ] {
        for transform in ["complex", "paired-real"] {
            let s = ks_n7_with(grid.separable_stages(0.0).unwrap(), transform);
            let mut solve = s.stage_solver(4.0).unwrap();
            for seed in 0..3 {
                let rhs = noise(grid.dimension(), 40 + seed);
                let (mut reused, mut fresh) = (rhs.clone(), rhs.clone());
                solve(&mut reused).unwrap();
                s.solve_stage(4.0, &mut fresh).unwrap();
                assert_eq!(reused, fresh, "{name} {transform} seed {seed}");
            }
            let mut short = vec![0.0; grid.dimension() - 1];
            assert!(matches!(
                solve(&mut short),
                Err(SolveError::InvalidInput(_))
            ));
            assert!(s.stage_solver(0.0).is_err(), "{name} {transform}");
            assert!(
                grid.separable_stages(5.0)
                    .unwrap()
                    .with_box_transform(transform.parse().unwrap())
                    .stage_solver(4.0)
                    .is_err(),
                "{name} {transform}: c > z must be refused"
            );
        }
    }
}

/// KS-N7 measurement, ONE arm per process so that `perf stat` counts one arm:
/// `KS_N7_ARM` = `complex` | `paired` (or `paired-real`), `KS_N7_GRID` = `256x256` |
/// `128x128x64` (two or three axes), `KS_N7_SOLVES` (default 200), `KS_N7_SCOPE` =
/// `prepared` (default: `stage_solver`, plans and buffers built once and reused, as in the
/// driver) | `solve_stage` (kernel built per call). Prints one line
/// `KS_N7_MEASURE {json}`: median, minimum and maximum seconds per stage solve, the largest
/// backward error (first and last solve), the preparation seconds.
#[test]
#[ignore = "timing measurement; one arm per process, see KS_N7_ARM/KS_N7_GRID/KS_N7_SOLVES"]
fn measure_box_transform_one_arm() {
    use std::time::Instant;
    let env = |key: &str, default: &str| std::env::var(key).unwrap_or_else(|_| default.into());
    let arm = env("KS_N7_ARM", "complex");
    let grid_name = env("KS_N7_GRID", "256x256");
    let scope = env("KS_N7_SCOPE", "prepared");
    let solves: usize = env("KS_N7_SOLVES", "200")
        .parse()
        .expect("KS_N7_SOLVES must be a positive integer");
    assert!(solves > 0, "KS_N7_SOLVES must be positive");
    let cells: Vec<usize> = grid_name
        .split('x')
        .map(|s| s.parse().expect("KS_N7_GRID: axes like 256x256"))
        .collect();
    let grid = match cells[..] {
        [nx, ny] => ks_n7_box([nx, ny], [1.0, 1.0], [1.0e-2, 1.0e-2]),
        [nx, ny, nz] => ks_n7_box([nx, ny, nz], [1.0; 3], [1.0e-2; 3]),
        _ => panic!("KS_N7_GRID needs two or three axes, e.g. 256x256 or 128x128x64"),
    };
    let stages = grid
        .separable_stages(0.0)
        .unwrap()
        .with_mode(SeparableMode::Trusted)
        .unwrap()
        .with_box_transform(arm.parse().expect("KS_N7_ARM: complex or paired"));
    // A RODAS5P stage shift: h = 1e-2, gamma = 0.2119...
    let (z, c) = (1.0 / (1.0e-2 * 0.211_937_563_194_290_14), 0.0);
    let n = grid.dimension();
    let b = noise(n, 5);
    let rho_bar = stages.spectral_bounds().radius_bound;
    let backward = |x: &[f64]| {
        max_diff(&b, &apply_w(&grid, z, c, x))
            / (((z - c).abs() + rho_bar) * max_abs(x) + max_abs(&b))
    };
    let mut x = vec![0.0; n];
    let mut times = Vec::with_capacity(solves);
    let mut worst = 0.0f64;
    let mut prepare_s = 0.0;
    match scope.as_str() {
        "prepared" => {
            let start = Instant::now();
            let mut solve = stages.stage_solver(z).unwrap();
            prepare_s = start.elapsed().as_secs_f64();
            for i in 0..solves {
                x.copy_from_slice(&b);
                let start = Instant::now();
                solve(&mut x).unwrap();
                times.push(start.elapsed().as_secs_f64());
                if i == 0 || i + 1 == solves {
                    worst = worst.max(backward(&x));
                }
            }
        }
        "solve_stage" => {
            for i in 0..solves {
                x.copy_from_slice(&b);
                let start = Instant::now();
                stages.solve_stage(z, &mut x).unwrap();
                times.push(start.elapsed().as_secs_f64());
                if i == 0 || i + 1 == solves {
                    worst = worst.max(backward(&x));
                }
            }
        }
        other => panic!("KS_N7_SCOPE must be prepared or solve_stage, not {other}"),
    }
    times.sort_by(f64::total_cmp);
    let median = if solves % 2 == 1 {
        times[solves / 2]
    } else {
        0.5 * (times[solves / 2 - 1] + times[solves / 2])
    };
    println!(
        "\nKS_N7_MEASURE {{\"ks_n7\":\"box_stage_solve\",\"arm\":\"{arm}\",\"transform\":\"{}\",\"grid\":\"{grid_name}\",\"n\":{n},\"solves\":{solves},\"scope\":\"{scope}\",\"median_s\":{median:.9},\"min_s\":{:.9},\"max_s\":{:.9},\"prepare_s\":{prepare_s:.9},\"max_backward_error\":{worst:.3e},\"z\":{z:.6e}}}",
        stages.box_transform(),
        times[0],
        times[solves - 1],
    );
}

// KS-N7b: the run record names the box transform that ran (finding F5 of the KS-N7 review),
// then `PairedReal` becomes the box default.
// Notes: research/kernel_spectral_foundation_20261124/ks_n7b/NOTES.txt.

/// `stats.driver_work` of a separable run, serialized: the run record a Solution carries.
fn ks_n7b_record(solution: &Solution) -> serde_json::Value {
    let record = serde_json::to_value(&solution.stats.driver_work).unwrap();
    assert_eq!(record["driver"], "separable-client", "{record}");
    record
}

/// KS-N7b (review F5 of KS-N7): a Solution tells which box transform ran. Whole IVPs on the
/// 12x10 box of T3 with each transform selected by name, and on the 7x10 sphere of T3: the
/// serialized run record carries `report.box_transform`, `"complex"` or `"paired-real"` on the
/// box and `null` on the sphere (its kernel always pairs rings). The client id stays the same.
#[test]
fn the_run_record_names_the_box_transform_that_ran() {
    let opts = options(1.0e-8, 1.0e-10);
    let span = (0.0, 1.0);
    let grid = ks_n7_box([12, 10], [1.0, 1.0], [0.02, 0.05]);
    let c = -0.5;
    let y0 = smooth_field(&grid, 12);
    let zeros = vec![0.0; grid.dimension()];
    let p = problem(&grid, c, zeros, Arc::new(AtomicUsize::new(0)));
    let mut recorded = Vec::new();
    for transform in ["complex", "paired-real"] {
        let stages = ks_n7_with(grid.separable_stages(c).unwrap(), transform);
        let solution = VigilODE::solve_separable(&p, span, &y0, &opts, &stages).unwrap();
        assert!(solution.is_success(), "box-12x10 {transform}");
        let record = ks_n7b_record(&solution);
        assert_eq!(record["client"], SEPARABLE_STAGE_CLIENT_ID, "{record}");
        assert_eq!(
            record["report"]["box_transform"], transform,
            "box-12x10: the run record must name the transform that ran: {record}"
        );
        recorded.push(record["report"]["box_transform"].clone());
    }
    assert_ne!(
        recorded[0], recorded[1],
        "the two transforms must be told apart"
    );
    let ball = sphere(7, 10, 0.4);
    let y0 = smooth_field(&ball, 10);
    let zeros = vec![0.0; ball.dimension()];
    let p = problem(&ball, 0.0, zeros, Arc::new(AtomicUsize::new(0)));
    let stages = ball.separable_stages(0.0).unwrap();
    let solution = VigilODE::solve_separable(&p, span, &y0, &opts, &stages).unwrap();
    assert!(solution.is_success(), "sphere-7x10");
    let record = ks_n7b_record(&solution);
    assert_eq!(
        record["report"].get("box_transform"),
        Some(&serde_json::Value::Null),
        "sphere-7x10: the record carries the key, null: {record}"
    );
    println!(
        "\nKS_N7B_RECORD {{\"box_12x10\":{},\"sphere_7x10\":{}}}",
        serde_json::Value::Array(recorded),
        record["report"]["box_transform"]
    );
}
