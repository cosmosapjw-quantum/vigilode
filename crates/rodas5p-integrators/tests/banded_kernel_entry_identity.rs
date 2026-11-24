//! The banded entry points that take the kernel (loop L1 node V22, discriminator D1).
//!
//! Each twin is compared with the entry point it extends, on the Indexed and on the Slices
//! kernel: every output state bit for bit, the step counts, the work counters, the counted band
//! work, the partial record, and under dense output every stored segment and the interior work.
//! Three problems: a wide band, a two-species band from an analytic provider, and a chain
//! whose factorizations exchange rows; one run is stopped by its attempt budget.

use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedKernel, FastInteriorControl, OdeProblem,
    OutputSchedule, RODAS5P_FAST_BANDED_DENSE_DRIVER_ID,
    RODAS5P_FAST_BANDED_DENSE_FIXED2_DRIVER_ID, RODAS5P_FAST_BANDED_DENSE_SLICES_DRIVER_ID,
    RODAS5P_FAST_BANDED_DRIVER_ID, RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID,
    RODAS5P_FAST_BANDED_SLICES_DRIVER_ID, Rodas5pFastBandedResult, Rodas5pFastDenseResult,
    Rodas5pFastResult, integrate_rodas5p_fast_banded_dense_observed,
    integrate_rodas5p_fast_banded_dense_observed_with_atol,
    integrate_rodas5p_fast_banded_dense_observed_with_atol_and_kernel,
    integrate_rodas5p_fast_banded_dense_observed_with_kernel,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_banded_observed_with_atol_and_kernel,
    integrate_rodas5p_fast_banded_observed_with_kernel,
};

type Callback3 = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type Callback4 = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// The stage coefficient gamma of RODAS5P: the stage matrix is `I / (h gamma) - J`.
const GAMMA: f64 = 0.211_937_563_194_290_14;
const CELL: f64 = 1.0 / 64.0;
/// The kernels with an endpoint and a dense-output twin. `FixedWidth2` joined the dense twins
/// with SL-N12 (the facade's (2, 2) banded routes run it).
const KERNELS: [BandedKernel; 3] = [
    BandedKernel::Indexed,
    BandedKernel::Slices,
    BandedKernel::FixedWidth2,
];

struct Case {
    name: &'static str,
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
        name,
        problem: OdeProblem::new(name, n, rhs, None, None, Some(jvp), None, true, None, None)
            .unwrap(),
        band: BandedJacobian { lower, upper, fill },
        y0,
        t1,
    }
}

/// Allen-Cahn on a strip of `w` cells across and `rows` along (band lower = upper = w).
fn allen_cahn_strip(w: usize, rows: usize) -> Case {
    let s = 0.01 / (CELL * CELL);
    let rhs: Callback3 = Arc::new(move |_t, y, out| {
        for j in 0..rows {
            for i in 0..w {
                let c = j * w + i;
                let u = y[c];
                let mut sum = 0.0;
                if i > 0 {
                    sum += y[c - 1] - u;
                }
                if i + 1 < w {
                    sum += y[c + 1] - u;
                }
                if j > 0 {
                    sum += y[c - w] - u;
                }
                if j + 1 < rows {
                    sum += y[c + w] - u;
                }
                out[c] = s * sum + (u - u * u * u);
            }
        }
        Ok(())
    });
    let fill: Callback3 = Arc::new(move |_t, y, band| {
        let jw = 2 * w + 1;
        for j in 0..rows {
            for i in 0..w {
                let c = j * w + i;
                let diag = c * jw + w;
                let mut nb = 0.0;
                if i > 0 {
                    band[diag - 1] = s;
                    nb += 1.0;
                }
                if i + 1 < w {
                    band[diag + 1] = s;
                    nb += 1.0;
                }
                if j > 0 {
                    band[diag - w] = s;
                    nb += 1.0;
                }
                if j + 1 < rows {
                    band[diag + w] = s;
                    nb += 1.0;
                }
                let u = y[c];
                band[diag] = -nb * s + (1.0 - 3.0 * u * u);
            }
        }
        Ok(())
    });
    let y0 = (0..rows)
        .flat_map(|j| {
            (0..w).map(move |i| {
                let x = (i as f64 + 0.5) * CELL;
                let y = (j as f64 + 0.5) * CELL;
                0.5 * (2.0 * PI * x).cos() * (3.0 * PI * y).cos()
                    + 0.3 * (5.0 * PI * x).cos() * (PI * y).cos()
                    - 0.1
            })
        })
        .collect();
    case("allen-cahn-strip", w * rows, rhs, w, w, fill, y0, 0.5)
}

/// Two-species Brusselator on a strip of `m` cells across and `rows` along, species
/// interleaved per cell (band lower = upper = 2 m).
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

const EXCHANGE_K: f64 = 1.0e3;
const EXCHANGE_D: f64 = 0.5;

/// A linear chain of pairs `p' = -k q + d lap p`, `q' = k p - 2 k q + d lap q` (band 2). Both
/// eigenvalues of a pair are `-k`, and the first row of a pair has no reaction term on the
/// diagonal: once `1 / (h gamma) + 2 d < k`, the entry below the diagonal is the larger one and
/// the factorization exchanges the two rows.
fn exchange_chain(cells: usize) -> Case {
    let (k, d) = (EXCHANGE_K, EXCHANGE_D);
    let rhs: Callback3 = Arc::new(move |_t, y, out| {
        for i in 0..cells {
            let a = 2 * i;
            let (p, q) = (y[a], y[a + 1]);
            let (mut sp, mut sq) = (0.0, 0.0);
            if i > 0 {
                sp += y[a - 2] - p;
                sq += y[a - 1] - q;
            }
            if i + 1 < cells {
                sp += y[a + 2] - p;
                sq += y[a + 3] - q;
            }
            out[a] = -k * q + d * sp;
            out[a + 1] = k * p - 2.0 * k * q + d * sq;
        }
        Ok(())
    });
    let fill: Callback3 = Arc::new(move |_t, _y, band| {
        for i in 0..cells {
            let (da, db) = (2 * i * 5 + 2, (2 * i + 1) * 5 + 2);
            let mut nb = 0.0;
            if i > 0 {
                band[da - 2] = d;
                band[db - 2] = d;
                nb += 1.0;
            }
            if i + 1 < cells {
                band[da + 2] = d;
                band[db + 2] = d;
                nb += 1.0;
            }
            band[da] = -nb * d;
            band[da + 1] = -k;
            band[db - 1] = k;
            band[db] = -2.0 * k - nb * d;
        }
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 0.5) / cells as f64;
            [1.0 + (2.0 * PI * x).sin(), 0.5 + 0.25 * (PI * x).cos()]
        })
        .collect();
    case("exchange-chain", 2 * cells, rhs, 2, 2, fill, y0, 1.0)
}

fn cases() -> Vec<Case> {
    vec![
        allen_cahn_strip(16, 24),
        brusselator_strip(4, 32),
        exchange_chain(60),
    ]
}

fn adaptive(max_attempts: usize) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-4,
        min_step: 1.0e-14,
        max_step: 10.0,
        max_attempts,
        ..AdaptiveStepConfig::default()
    }
}

/// A non-uniform tolerance vector, so that the vector path differs from the scalar path.
fn atol_vector(n: usize) -> Vec<f64> {
    (0..n).map(|i| 1.0e-8 * (1.0 + (i % 3) as f64)).collect()
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

fn assert_same_fast(label: &str, twin: &Rodas5pFastResult, base: &Rodas5pFastResult) {
    assert_eq!(twin.attempts, base.attempts, "{label}: attempts");
    assert_eq!(
        twin.accepted_steps, base.accepted_steps,
        "{label}: accepted"
    );
    assert_eq!(
        twin.rejected_steps, base.rejected_steps,
        "{label}: rejected"
    );
    assert_eq!(
        twin.jacobian_reuses, base.jacobian_reuses,
        "{label}: reuses"
    );
    assert_eq!(
        format!("{:?}", twin.lu),
        format!("{:?}", base.lu),
        "{label}: LU record"
    );
    let (a, b) = (&twin.observed, &base.observed);
    assert_eq!(bits(&a.t), bits(&b.t), "{label}: output times");
    assert_eq!(a.y.len(), b.y.len(), "{label}: number of outputs");
    for (index, (ya, yb)) in a.y.iter().zip(&b.y).enumerate() {
        assert_eq!(bits(ya), bits(yb), "{label}: state at output {index}");
    }
    assert_eq!(a.success, b.success, "{label}: success");
    assert_eq!(a.message, b.message, "{label}: message");
    assert_eq!(a.counters, b.counters, "{label}: work counters");
    assert_eq!(
        a.internal_steps, b.internal_steps,
        "{label}: internal steps"
    );
    assert_eq!(
        a.output_clipped_steps, b.output_clipped_steps,
        "{label}: clipped steps"
    );
}

fn assert_same_banded(
    label: &str,
    twin: &Rodas5pFastBandedResult,
    base: &Rodas5pFastBandedResult,
    kernel: BandedKernel,
) {
    assert_same_fast(label, &twin.fast, &base.fast);
    assert_eq!(twin.work, base.work, "{label}: band work");
    assert_eq!(twin.partial, base.partial, "{label}: partial record");
    if let (Some(a), Some(b)) = (&twin.partial, &base.partial) {
        assert_eq!(bits(&a.y), bits(&b.y), "{label}: partial state");
        assert_eq!(a.t.to_bits(), b.t.to_bits(), "{label}: partial time");
    }
    assert_eq!(base.fast.driver, RODAS5P_FAST_BANDED_DRIVER_ID, "{label}");
    let expected = match kernel {
        BandedKernel::Slices => RODAS5P_FAST_BANDED_SLICES_DRIVER_ID,
        BandedKernel::FixedWidth2 => RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID,
        _ => RODAS5P_FAST_BANDED_DRIVER_ID,
    };
    assert_eq!(twin.fast.driver, expected, "{label}: driver identifier");
}

fn assert_same_dense(
    label: &str,
    twin: &Rodas5pFastDenseResult,
    base: &Rodas5pFastDenseResult,
    kernel: BandedKernel,
) {
    assert_same_fast(label, &twin.fast, &base.fast);
    assert_eq!(twin.banded, base.banded, "{label}: band work");
    assert_eq!(twin.interior, base.interior, "{label}: interior work");
    assert_eq!(twin.partial, base.partial, "{label}: partial record");
    let (a, b) = (&twin.segments, &base.segments);
    assert_eq!(a.len(), b.len(), "{label}: number of segments");
    assert_eq!(bits(a.times()), bits(b.times()), "{label}: segment times");
    for index in 0..a.len() {
        assert_eq!(
            a.t_old(index).to_bits(),
            b.t_old(index).to_bits(),
            "{label}: segment {index} start"
        );
        assert_eq!(
            a.t_new(index).to_bits(),
            b.t_new(index).to_bits(),
            "{label}: segment {index} end"
        );
        assert_eq!(
            bits(a.state(index)),
            bits(b.state(index)),
            "{label}: segment {index} state"
        );
        assert_eq!(
            bits(a.stage_rows(index)),
            bits(b.stage_rows(index)),
            "{label}: segment {index} stage rows"
        );
    }
    assert_eq!(
        base.fast.driver, RODAS5P_FAST_BANDED_DENSE_DRIVER_ID,
        "{label}"
    );
    let expected = match kernel {
        BandedKernel::Slices => RODAS5P_FAST_BANDED_DENSE_SLICES_DRIVER_ID,
        BandedKernel::FixedWidth2 => RODAS5P_FAST_BANDED_DENSE_FIXED2_DRIVER_ID,
        _ => RODAS5P_FAST_BANDED_DENSE_DRIVER_ID,
    };
    assert_eq!(twin.fast.driver, expected, "{label}: driver identifier");
}

#[test]
fn endpoint_twins_equal_the_existing_entry_points() {
    for case in cases() {
        let n = case.problem.dimension;
        let span = (0.0, case.t1);
        let output = OutputSchedule::new(vec![0.0, 0.5 * case.t1, case.t1]).unwrap();
        let config = adaptive(1_000_000);
        let atol = atol_vector(n);
        let scalar = integrate_rodas5p_fast_banded_observed(
            &case.problem,
            &case.band,
            span,
            &case.y0,
            &config,
            &output,
        )
        .unwrap();
        let vector = integrate_rodas5p_fast_banded_observed_with_atol(
            &case.problem,
            &case.band,
            span,
            &case.y0,
            &config,
            &output,
            &atol,
        )
        .unwrap();
        assert!(scalar.fast.observed.success && vector.fast.observed.success);
        assert!(scalar.partial.is_none() && vector.partial.is_none());
        // The two tolerance paths are different runs: the comparison below is not vacuous.
        assert_ne!(
            bits(scalar.fast.observed.y.last().unwrap()),
            bits(vector.fast.observed.y.last().unwrap()),
            "{}: scalar and vector tolerance give the same end state",
            case.name
        );
        for kernel in KERNELS {
            let twin = integrate_rodas5p_fast_banded_observed_with_kernel(
                &case.problem,
                &case.band,
                span,
                &case.y0,
                &config,
                &output,
                kernel,
            )
            .unwrap();
            assert_same_banded(
                &format!("{} scalar {kernel:?}", case.name),
                &twin,
                &scalar,
                kernel,
            );
            let twin = integrate_rodas5p_fast_banded_observed_with_atol_and_kernel(
                &case.problem,
                &case.band,
                span,
                &case.y0,
                &config,
                &output,
                &atol,
                kernel,
            )
            .unwrap();
            assert_same_banded(
                &format!("{} vector {kernel:?}", case.name),
                &twin,
                &vector,
                kernel,
            );
        }
        println!(
            "V22_D1_ENDPOINT case={} n={n} lower={} upper={} attempts_scalar={} attempts_vector={} rejected_scalar={} factor_operations={} solve_operations={}",
            case.name,
            case.band.lower,
            case.band.upper,
            scalar.fast.attempts,
            vector.fast.attempts,
            scalar.fast.rejected_steps,
            scalar.work.factor_operations,
            scalar.work.solve_operations,
        );
    }
}

#[test]
fn dense_output_twins_equal_the_existing_entry_points() {
    for case in cases() {
        let n = case.problem.dimension;
        let span = (0.0, case.t1);
        let config = adaptive(1_000_000);
        let atol = atol_vector(n);
        for interior in [
            FastInteriorControl::Off,
            FastInteriorControl::WFilteredEnforce,
        ] {
            let scalar = integrate_rodas5p_fast_banded_dense_observed(
                &case.problem,
                &case.band,
                span,
                &case.y0,
                &config,
                interior,
            )
            .unwrap();
            let vector = integrate_rodas5p_fast_banded_dense_observed_with_atol(
                &case.problem,
                &case.band,
                span,
                &case.y0,
                &config,
                interior,
                &atol,
            )
            .unwrap();
            assert!(scalar.fast.observed.success && vector.fast.observed.success);
            assert!(!scalar.segments.is_empty());
            for kernel in KERNELS {
                let twin = integrate_rodas5p_fast_banded_dense_observed_with_kernel(
                    &case.problem,
                    &case.band,
                    span,
                    &case.y0,
                    &config,
                    interior,
                    kernel,
                )
                .unwrap();
                assert_same_dense(
                    &format!("{} dense scalar {interior:?} {kernel:?}", case.name),
                    &twin,
                    &scalar,
                    kernel,
                );
                let twin = integrate_rodas5p_fast_banded_dense_observed_with_atol_and_kernel(
                    &case.problem,
                    &case.band,
                    span,
                    &case.y0,
                    &config,
                    interior,
                    &atol,
                    kernel,
                )
                .unwrap();
                assert_same_dense(
                    &format!("{} dense vector {interior:?} {kernel:?}", case.name),
                    &twin,
                    &vector,
                    kernel,
                );
            }
            println!(
                "V22_D1_DENSE case={} n={n} interior={interior:?} segments={} attempts={} rejected={} interior_samples={}",
                case.name,
                scalar.segments.len(),
                scalar.fast.attempts,
                scalar.fast.rejected_steps,
                scalar.interior.samples,
            );
        }
    }
}

/// The chain reaches steps at which the factorization must exchange rows: with the largest
/// accepted step `h`, the diagonal entry `1 / (h gamma) + 2 d` of the first row of a pair is
/// smaller than the entry `k` below it.
#[test]
fn the_exchange_chain_exchanges_rows() {
    let case = exchange_chain(60);
    let run = integrate_rodas5p_fast_banded_dense_observed(
        &case.problem,
        &case.band,
        (0.0, case.t1),
        &case.y0,
        &adaptive(1_000_000),
        FastInteriorControl::Off,
    )
    .unwrap();
    let largest = (0..run.segments.len())
        .map(|index| run.segments.t_new(index) - run.segments.t_old(index))
        .fold(0.0_f64, f64::max);
    let diagonal = 1.0 / (largest * GAMMA) + 2.0 * EXCHANGE_D;
    println!("V22_D1_EXCHANGE largest_step={largest:e} diagonal={diagonal:e} below={EXCHANGE_K:e}");
    assert!(
        diagonal < EXCHANGE_K,
        "no step is long enough to force a row exchange: diagonal {diagonal:e}"
    );
}

#[test]
fn a_run_stopped_by_its_budget_keeps_its_partial_record() {
    let case = allen_cahn_strip(16, 24);
    let n = case.problem.dimension;
    let span = (0.0, case.t1);
    let output = OutputSchedule::new(vec![0.0, case.t1]).unwrap();
    let config = adaptive(5);
    let atol = atol_vector(n);
    let base = integrate_rodas5p_fast_banded_observed_with_atol(
        &case.problem,
        &case.band,
        span,
        &case.y0,
        &config,
        &output,
        &atol,
    )
    .unwrap();
    assert!(
        base.partial.is_some(),
        "five attempts must not reach the end"
    );
    let dense = integrate_rodas5p_fast_banded_dense_observed(
        &case.problem,
        &case.band,
        span,
        &case.y0,
        &config,
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    assert!(dense.partial.is_some());
    for kernel in KERNELS {
        let twin = integrate_rodas5p_fast_banded_observed_with_atol_and_kernel(
            &case.problem,
            &case.band,
            span,
            &case.y0,
            &config,
            &output,
            &atol,
            kernel,
        )
        .unwrap();
        assert_same_banded(&format!("budget {kernel:?}"), &twin, &base, kernel);
        let twin = integrate_rodas5p_fast_banded_dense_observed_with_kernel(
            &case.problem,
            &case.band,
            span,
            &case.y0,
            &config,
            FastInteriorControl::WFilteredEnforce,
            kernel,
        )
        .unwrap();
        assert_same_dense(&format!("budget dense {kernel:?}"), &twin, &dense, kernel);
    }
}

#[test]
fn the_dense_output_twins_refuse_the_other_kernels() {
    let case = brusselator_strip(1, 16);
    let n = case.problem.dimension;
    let config = adaptive(1_000_000);
    for kernel in [BandedKernel::FixedWidth2ExactCache] {
        let scalar = integrate_rodas5p_fast_banded_dense_observed_with_kernel(
            &case.problem,
            &case.band,
            (0.0, case.t1),
            &case.y0,
            &config,
            FastInteriorControl::Off,
            kernel,
        );
        let vector = integrate_rodas5p_fast_banded_dense_observed_with_atol_and_kernel(
            &case.problem,
            &case.band,
            (0.0, case.t1),
            &case.y0,
            &config,
            FastInteriorControl::Off,
            &atol_vector(n),
            kernel,
        );
        for result in [scalar, vector] {
            match result {
                Err(CoreError::InvalidInput(message)) => assert_eq!(
                    message,
                    "the RODAS5P fast banded dense driver runs the Indexed, the Slices and the FixedWidth2 kernel only"
                ),
                other => panic!(
                    "{kernel:?} was not refused: {:?}",
                    other.map(|run| run.fast.driver)
                ),
            }
        }
    }
}

/// SL-N12 diagnostic: whole endpoint solves of the band (2, 2) on the Indexed kernel (the
/// facade's route before SL-N12) and on the FixedWidth2 kernel (after), alternating, with the
/// median wall time of each arm; every pair is first checked for identical bits. Host wall
/// time only; the owner-host instruction and cycle cells of SL-N12 stay preregistered.
#[test]
#[ignore = "timing diagnostic; run with --ignored --nocapture"]
fn measure_fixed_width_against_indexed_whole_solves() {
    use std::time::Instant;
    for rows in [5_000usize, 50_000] {
        let case = brusselator_strip(1, rows);
        let n = case.problem.dimension;
        let span = (0.0, case.t1);
        let output = OutputSchedule::new(vec![0.0, case.t1]).unwrap();
        let config = adaptive(10_000_000);
        let run = |kernel| {
            integrate_rodas5p_fast_banded_observed_with_kernel(
                &case.problem,
                &case.band,
                span,
                &case.y0,
                &config,
                &output,
                kernel,
            )
            .unwrap()
        };
        let base = run(BandedKernel::Indexed);
        let twin = run(BandedKernel::FixedWidth2);
        assert_same_banded("timing", &twin, &base, BandedKernel::FixedWidth2);
        let repeats = if n > 50_000 { 7 } else { 15 };
        let (mut indexed, mut fixed) = (Vec::new(), Vec::new());
        for _ in 0..repeats {
            for (kernel, times) in [
                (BandedKernel::Indexed, &mut indexed),
                (BandedKernel::FixedWidth2, &mut fixed),
            ] {
                let start = Instant::now();
                std::hint::black_box(run(kernel));
                times.push(start.elapsed().as_secs_f64());
            }
        }
        let median = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        };
        let (a, b) = (median(&mut indexed), median(&mut fixed));
        println!(
            "{{\"sl_n12\":\"whole_solve\",\"n\":{n},\"attempts\":{},\"repeats\":{repeats},\"indexed_s\":{a:.6},\"fixed2_s\":{b:.6},\"ratio\":{:.4}}}",
            base.fast.attempts,
            b / a
        );
    }
}
