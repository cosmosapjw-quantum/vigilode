//! KS-N3 owner-host probe (research/kernel_spectral_foundation_20261124/PLAN.json).
//!
//! The two ignored diagnostics of `crates/vigilode/tests/spatial_separable.rs`
//! (`measure_separable_against_the_shipped_matrix_free_route` and
//! `measure_separable_against_preconditioned_gmres_stage_solves`) run every arm inside one
//! process, so `perf stat` over them cannot count one arm. This probe runs ONE arm per process
//! with the helpers and settings of that file copied below (sphere, smooth_field, problem,
//! options, noise, max_abs, max_diff, mass, abs_mass; grids, tolerances, the reference, the
//! GMRES configuration and the stage systems).
//!
//! Usage:
//!   ks-n3-probe whole <shipped|separable-trusted|separable-verified> <nt> <np> <solves>
//!   ks-n3-probe stage <gmres-identity|gmres-longitude|separable> <nt> <np> <h> <solves>
//! Every solve is timed on its own; one JSON line is printed.

use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rodas5p_core::{
    ClosureOperator, CoreError, IdentityPreconditioner, LinearOperator, Preconditioner,
    WorkCounters,
};
use rodas5p_krylov::{GmresConfig, solve_gmres};
use vigilode::spatial::{DiffusionGrid, SeparableMode, SpatialRows};
use vigilode::{Atol, Jacobian, Options, Problem, Rodas5p, VigilODE};

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

fn options(rtol: f64, atol: f64) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(atol),
        ..Options::default()
    }
}

fn fnv1a(values: &[f64]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn times_json(times: &[f64]) -> String {
    let parts: Vec<String> = times.iter().map(|t| format!("{t:.9}")).collect();
    format!("[{}]", parts.join(","))
}

/// The whole-solve diagnostic for one arm (sphere grid, kappa 1e-2, span [0, 1], rtol 1e-6,
/// atol 1e-9; error against the separable Trusted self-reference at rtol 1e-11, atol 1e-13).
fn whole(arm: &str, nt: usize, np: usize, solves: usize) -> String {
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
    let opts = options(1.0e-6, 1.0e-9);
    let mut times = Vec::new();
    let mut last = None;
    for _ in 0..solves {
        let start = Instant::now();
        let s = match arm {
            "shipped" => Rodas5p::solve(&p, span, &y0, &opts).unwrap(),
            "separable-trusted" => {
                VigilODE::solve_separable(&p, span, &y0, &opts, &trusted).unwrap()
            }
            "separable-verified" => {
                VigilODE::solve_separable(&p, span, &y0, &opts, &verified).unwrap()
            }
            other => panic!("unknown whole arm {other}"),
        };
        times.push(start.elapsed().as_secs_f64());
        last = Some(std::hint::black_box(s));
    }
    let mut line = format!(
        "{{\"probe\":\"ks_n3\",\"mode\":\"whole\",\"arm\":\"{arm}\",\"grid\":\"{nt}x{np}\",\"n\":{n},\"solves\":{solves},\"times_s\":{}",
        times_json(&times)
    );
    if let Some(s) = last {
        let y = s.final_state().unwrap();
        let c = &s.stats.counters;
        write!(
            line,
            ",\"success\":{},\"relative_error\":{:e},\"attempts\":{},\"rhs_calls\":{},\"jvp_calls\":{},\"linear_iterations\":{},\"direct_solve_calls\":{},\"mass_drift\":{:e},\"state_fnv1a\":\"{:016x}\"",
            s.is_success(),
            max_diff(y, &reference) / scale,
            s.stats.attempts,
            c.rhs_calls,
            c.jvp_calls,
            c.linear_iterations,
            c.direct_solve_calls,
            (mass(&grid, y) - mass(&grid, &y0)).abs() / abs_mass(&grid, &y0),
            fnv1a(y),
        )
        .unwrap();
    }
    line.push('}');
    line
}

/// The stage diagnostic for one arm: `(I - sigma L) x = r`, `sigma = h gamma`, `r = noise(n, 5)`;
/// GMRES (restart 30, at most 3000 Arnoldi vectors, rtol 1e-10) with the identity or the
/// longitude line preconditioner, or the separable solve of `(z I - L) x = z r`, `z = 1/sigma`
/// (Trusted mode, every call plans and factors).
fn stage(arm: &str, nt: usize, np: usize, h: f64, solves: usize) -> String {
    let gamma = 0.211_937_563_194_290_14;
    let grid = sphere(nt, np, 1.0e-2);
    let n = grid.dimension();
    let stages = grid
        .separable_stages(0.0)
        .unwrap()
        .with_mode(SeparableMode::Trusted)
        .unwrap();
    let rhs = noise(n, 5);
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
    let mut times = Vec::new();
    let mut extra = String::new();
    match arm {
        "gmres-identity" | "gmres-longitude" => {
            let identity = IdentityPreconditioner::new(n);
            let line = grid
                .longitude_preconditioner(sigma, SpatialRows::Serial)
                .unwrap();
            let pc: &dyn Preconditioner = if arm == "gmres-identity" {
                &identity
            } else {
                &line
            };
            let mut last = None;
            let mut failure = None;
            for _ in 0..solves {
                let mut counters = WorkCounters::default();
                let start = Instant::now();
                let outcome = solve_gmres(&op, pc, &rhs, None, &config, &mut counters);
                times.push(start.elapsed().as_secs_f64());
                match outcome {
                    Ok(s) => last = Some(std::hint::black_box(s)),
                    Err(e) => {
                        failure = Some(e.to_string());
                        break;
                    }
                }
            }
            match (last, failure) {
                (_, Some(e)) => {
                    write!(extra, ",\"converged\":false,\"error\":\"{}\"", e.replace('"', "'"))
                        .unwrap()
                }
                (Some(s), None) => write!(
                    extra,
                    ",\"converged\":{},\"matvecs\":{},\"relative_residual\":{:e}",
                    s.converged,
                    s.matvecs,
                    relative_residual(&s.x)
                )
                .unwrap(),
                (None, None) => {}
            }
        }
        "separable" => {
            let mut x = Vec::new();
            for _ in 0..solves {
                x = rhs.iter().map(|v| v / sigma).collect::<Vec<f64>>();
                let start = Instant::now();
                stages.solve_stage(1.0 / sigma, &mut x).unwrap();
                times.push(start.elapsed().as_secs_f64());
            }
            if solves > 0 {
                write!(
                    extra,
                    ",\"converged\":true,\"matvecs\":0,\"relative_residual\":{:e}",
                    relative_residual(&x)
                )
                .unwrap();
            }
        }
        other => panic!("unknown stage arm {other}"),
    }
    format!(
        "{{\"probe\":\"ks_n3\",\"mode\":\"stage\",\"arm\":\"{arm}\",\"grid\":\"{nt}x{np}\",\"n\":{n},\"h\":{h:e},\"solves\":{solves},\"times_s\":{}{extra}}}",
        times_json(&times)
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let line = match args.get(1).map(String::as_str) {
        Some("whole") if args.len() == 6 => whole(
            &args[2],
            args[3].parse().expect("nt"),
            args[4].parse().expect("np"),
            args[5].parse().expect("solves"),
        ),
        Some("stage") if args.len() == 7 => stage(
            &args[2],
            args[3].parse().expect("nt"),
            args[4].parse().expect("np"),
            args[5].parse().expect("h"),
            args[6].parse().expect("solves"),
        ),
        _ => panic!(
            "usage: ks-n3-probe whole <arm> <nt> <np> <solves> | stage <arm> <nt> <np> <h> <solves>"
        ),
    };
    println!("{line}");
}
