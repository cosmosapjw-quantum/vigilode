//! KS-N1 owner-host probe (research/kernel_spectral_foundation_20261124/PLAN.json).
//!
//! Whole endpoint solves of the band (2, 2) Brusselator strip of
//! `crates/rodas5p-integrators/tests/banded_kernel_entry_identity.rs`
//! (`brusselator_strip(1, rows)` with the tolerances of `adaptive(10_000_000)`, copied below)
//! on ONE kernel per process, so that `perf stat` counts one arm. The diagnostic test
//! `measure_fixed_width_against_indexed_whole_solves` alternates both arms in one process and
//! cannot give per-arm counts.
//!
//! Usage: `ks-n1-probe <indexed|fixed2> <rows> <solves>`. Prints one JSON line describing the
//! last solve; with `solves = 0` only the case is built.

use std::fmt::Write as _;
use std::sync::Arc;

use rodas5p_core::CoreResult;
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedKernel, OdeProblem, OutputSchedule,
    integrate_rodas5p_fast_banded_observed_with_kernel,
};

type Callback3 = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type Callback4 = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

const CELL: f64 = 1.0 / 64.0;

struct Case {
    problem: OdeProblem,
    band: BandedJacobian,
    y0: Vec<f64>,
    t1: f64,
}

#[allow(clippy::too_many_arguments)]
fn case(
    name: &'static str,
    n: usize,
    rhs: Callback3,
    lower: usize,
    upper: usize,
    fill: Callback3,
    y0: Vec<f64>,
    t1: f64,
) -> Case {
    // The banded driver never calls the product; the problem constructor asks for one.
    let jvp: Callback4 = Arc::new(|_t, _y, _w, out| {
        out.fill(0.0);
        Ok(())
    });
    Case {
        problem: OdeProblem::new(name, n, rhs, None, None, Some(jvp), None, true, None, None)
            .unwrap(),
        band: BandedJacobian { lower, upper, fill },
        y0,
        t1,
    }
}

/// Copy of `brusselator_strip` of the test file: two species interleaved per cell on a strip of
/// `m` cells across and `rows` along (band lower = upper = 2 m).
fn brusselator_strip(m: usize, rows: usize) -> Case {
    let (a_par, b_par) = (1.0, 3.4);
    let s = 0.02 / (CELL * CELL);
    let sy = 2 * m;
    let rhs: Callback3 = Arc::new(move |_t, y, out| {
        for j in 0..rows {
            for i in 0..m {
                let a = 2 * (i + m * j);
                let (u, v) = (y[a], y[a + 1]);
                let (mut su, mut sv) = (0.0, 0.0);
                if i > 0 {
                    su += y[a - 2] - u;
                    sv += y[a - 1] - v;
                }
                if i + 1 < m {
                    su += y[a + 2] - u;
                    sv += y[a + 3] - v;
                }
                if j > 0 {
                    su += y[a - sy] - u;
                    sv += y[a + 1 - sy] - v;
                }
                if j + 1 < rows {
                    su += y[a + sy] - u;
                    sv += y[a + 1 + sy] - v;
                }
                out[a] = a_par + u * u * v - (b_par + 1.0) * u + s * su;
                out[a + 1] = b_par * u - u * u * v + s * sv;
            }
        }
        Ok(())
    });
    let fill: Callback3 = Arc::new(move |_t, y, band| {
        let width = 2 * m;
        let jw = 2 * width + 1;
        for j in 0..rows {
            for i in 0..m {
                let a = 2 * (i + m * j);
                let (u, v) = (y[a], y[a + 1]);
                let (da, db) = (a * jw + width, (a + 1) * jw + width);
                let mut nb = 0.0;
                if i > 0 {
                    band[da - 2] = s;
                    band[db - 2] = s;
                    nb += 1.0;
                }
                if i + 1 < m {
                    band[da + 2] = s;
                    band[db + 2] = s;
                    nb += 1.0;
                }
                if j > 0 {
                    band[da - sy] = s;
                    band[db - sy] = s;
                    nb += 1.0;
                }
                if j + 1 < rows {
                    band[da + sy] = s;
                    band[db + sy] = s;
                    nb += 1.0;
                }
                band[da] = 2.0 * u * v - (b_par + 1.0) - nb * s;
                band[da + 1] = u * u;
                band[db - 1] = b_par - 2.0 * u * v;
                band[db] = -u * u - nb * s;
            }
        }
        Ok(())
    });
    let mut y0 = vec![0.0; 2 * m * rows];
    for j in 0..rows {
        for i in 0..m {
            let a = 2 * (i + m * j);
            y0[a] = 0.5 + (j as f64 + 0.5) / rows as f64;
            y0[a + 1] = 1.0 + 5.0 * (i as f64 + 0.5) / m as f64;
        }
    }
    case(
        "brusselator-strip",
        2 * m * rows,
        rhs,
        2 * m,
        2 * m,
        fill,
        y0,
        0.5,
    )
}

fn fnv1a(hash: &mut u64, word: u64) {
    for byte in word.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4, "usage: ks-n1-probe <indexed|fixed2> <rows> <solves>");
    let kernel = match args[1].as_str() {
        "indexed" => BandedKernel::Indexed,
        "fixed2" => BandedKernel::FixedWidth2,
        other => panic!("unknown arm {other}"),
    };
    let rows: usize = args[2].parse().expect("rows");
    let solves: usize = args[3].parse().expect("solves");
    let case = brusselator_strip(1, rows);
    let n = case.problem.dimension;
    // `adaptive(10_000_000)` of the test file.
    let config = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-4,
        min_step: 1.0e-14,
        max_step: 10.0,
        max_attempts: 10_000_000,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![0.0, case.t1]).unwrap();
    let mut last = None;
    for _ in 0..solves {
        let result = integrate_rodas5p_fast_banded_observed_with_kernel(
            &case.problem,
            &case.band,
            (0.0, case.t1),
            &case.y0,
            &config,
            &output,
            kernel,
        )
        .unwrap();
        last = Some(std::hint::black_box(result));
    }
    let mut line = format!(
        "{{\"probe\":\"ks_n1\",\"arm\":\"{}\",\"rows\":{rows},\"n\":{n},\"solves\":{solves}",
        args[1]
    );
    if let Some(result) = last {
        let observed = &result.fast.observed;
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for t in &observed.t {
            fnv1a(&mut hash, t.to_bits());
        }
        for state in &observed.y {
            for value in state {
                fnv1a(&mut hash, value.to_bits());
            }
        }
        write!(
            line,
            ",\"attempts\":{},\"accepted\":{},\"rejected\":{},\"jacobian_reuses\":{},\"driver\":\"{}\",\
             \"lu\":\"{}\",\"work\":\"{}\",\"partial\":{},\"success\":{},\"message\":\"{}\",\
             \"counters\":\"{}\",\"internal_steps\":{},\"output_clipped_steps\":{},\
             \"outputs\":{},\"state_fnv1a\":\"{hash:016x}\"",
            result.fast.attempts,
            result.fast.accepted_steps,
            result.fast.rejected_steps,
            result.fast.jacobian_reuses,
            result.fast.driver,
            escape(&format!("{:?}", result.fast.lu)),
            escape(&format!("{:?}", result.work)),
            result.partial.is_some(),
            observed.success,
            escape(&observed.message),
            escape(&format!("{:?}", observed.counters)),
            observed.internal_steps,
            observed.output_clipped_steps,
            observed.y.len(),
        )
        .unwrap();
    }
    line.push('}');
    println!("{line}");
}
