//! The kernel of the facade's banded route (loop L1 node V22, owner decision OD-L1-9).
//!
//! `golden/banded_kernel_route/wide_band.json` holds the record of facade runs on banded
//! problems with half band widths from 2 to 32: the route labels, the step counts, every work
//! counter and a hash of the bits of every returned state. The file was first written while
//! every banded facade run used the Indexed kernel of the protected banded driver. Since the
//! facade chooses the kernel by band width ([`BANDED_SLICES_MIN_HALF_WIDTH`]), the runs on the
//! Slices kernel differ from that first version in their two route labels and in nothing else.
//! `VIGILODE_BLESS_BANDED_KERNEL_ROUTE=1` writes the file instead of comparing.

use std::f64::consts::PI;
use std::path::PathBuf;

use serde_json::{Value, json};
use vigilode::{
    Atol, BANDED_SLICES_MIN_HALF_WIDTH, CallbackError, Jacobian, Options, Problem, Rodas5p,
    Solution,
};

const BLESS_ENV: &str = "VIGILODE_BLESS_BANDED_KERNEL_ROUTE";
/// Cell size of every problem here, in both directions.
const CELL: f64 = 1.0 / 64.0;

struct Case {
    name: String,
    lower: usize,
    upper: usize,
    problem: Problem,
    y0: Vec<f64>,
}

fn smooth(i: usize, j: usize) -> f64 {
    let x = (i as f64 + 0.5) * CELL;
    let y = (j as f64 + 0.5) * CELL;
    0.5 * (2.0 * PI * x).cos() * (3.0 * PI * y).cos() + 0.3 * (5.0 * PI * x).cos() * (PI * y).cos()
        - 0.1
}

/// Allen-Cahn `u_t = D lap u + u - u^3` on a strip of `w` cells across (fast index) and `rows`
/// along, zero-flux boundaries, with its band Jacobian. Band: lower = upper = w.
fn allen_cahn_strip(w: usize, rows: usize) -> Case {
    let s = 0.01 / (CELL * CELL);
    let rhs = move |_t: f64, y: &[f64], dy: &mut [f64]| {
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
                dy[c] = s * sum + (u - u * u * u);
            }
        }
        Ok::<(), CallbackError>(())
    };
    let band = move |_t: f64, y: &[f64], band: &mut [f64]| {
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
        Ok::<(), CallbackError>(())
    };
    Case {
        name: format!("allen-cahn-strip/{w}x{rows}"),
        lower: w,
        upper: w,
        problem: Problem::new(w * rows, rhs)
            .jacobian(Jacobian::banded(w, w, band))
            .autonomous(),
        y0: (0..rows)
            .flat_map(|j| (0..w).map(move |i| smooth(i, j)))
            .collect(),
    }
}

/// Two-species Brusselator on a strip of `m` cells across and `rows` along, zero-flux
/// boundaries, species interleaved per cell; the band comes from finite differences of the
/// declared band. Band: lower = upper = 2 m.
fn brusselator_strip(m: usize, rows: usize) -> Case {
    let (a_par, b_par) = (1.0, 3.4);
    let s = 0.02 / (CELL * CELL);
    let sy = 2 * m;
    let rhs = move |_t: f64, y: &[f64], dy: &mut [f64]| {
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
                dy[a] = a_par + u * u * v - (b_par + 1.0) * u + s * su;
                dy[a + 1] = b_par * u - u * u * v + s * sv;
            }
        }
        Ok::<(), CallbackError>(())
    };
    let mut y0 = vec![0.0; 2 * m * rows];
    for j in 0..rows {
        for i in 0..m {
            let a = 2 * (i + m * j);
            y0[a] = 0.5 + (j as f64 + 0.5) / rows as f64;
            y0[a + 1] = 1.0 + 5.0 * (i as f64 + 0.5) / m as f64;
        }
    }
    Case {
        name: format!("brusselator-strip-fd/{m}x{rows}"),
        lower: 2 * m,
        upper: 2 * m,
        problem: Problem::new(2 * m * rows, rhs)
            .jacobian(Jacobian::FiniteDifference)
            .banded(2 * m, 2 * m)
            .autonomous(),
        y0,
    }
}

/// Reaction and diffusion across the strip with first-order upwind transport along it:
/// `u_t = D u_xx - a u_y + u - u^3`. With `forward` the transport couples a cell to the row
/// before it (band lower = w, upper = 1), otherwise to the row after it (lower = 1, upper = w).
fn upwind_strip(w: usize, rows: usize, forward: bool) -> Case {
    let s = 0.01 / (CELL * CELL);
    let a = 0.5 / CELL;
    let rhs = move |_t: f64, y: &[f64], dy: &mut [f64]| {
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
                // The value outside the inflow boundary is zero.
                let upstream = if forward {
                    if j > 0 { y[c - w] } else { 0.0 }
                } else if j + 1 < rows {
                    y[c + w]
                } else {
                    0.0
                };
                dy[c] = s * sum + a * (upstream - u) + (u - u * u * u);
            }
        }
        Ok::<(), CallbackError>(())
    };
    let (lower, upper) = if forward { (w, 1) } else { (1, w) };
    let band = move |_t: f64, y: &[f64], band: &mut [f64]| {
        let jw = lower + upper + 1;
        for j in 0..rows {
            for i in 0..w {
                let c = j * w + i;
                let diag = c * jw + lower;
                let mut nb = 0.0;
                if i > 0 {
                    band[diag - 1] = s;
                    nb += 1.0;
                }
                if i + 1 < w {
                    band[diag + 1] = s;
                    nb += 1.0;
                }
                if forward {
                    if j > 0 {
                        band[diag - w] = a;
                    }
                } else if j + 1 < rows {
                    band[diag + w] = a;
                }
                let u = y[c];
                band[diag] = -nb * s - a + (1.0 - 3.0 * u * u);
            }
        }
        Ok::<(), CallbackError>(())
    };
    Case {
        name: format!(
            "upwind-strip-{}/{w}x{rows}",
            if forward { "forward" } else { "backward" }
        ),
        lower,
        upper,
        problem: Problem::new(w * rows, rhs)
            .jacobian(Jacobian::banded(lower, upper, band))
            .autonomous(),
        y0: (0..rows)
            .flat_map(|j| (0..w).map(move |i| smooth(i, j)))
            .collect(),
    }
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for w in [2, 3, 4, 6, 8, 12, 16, 32] {
        cases.push(allen_cahn_strip(w, (384 / w).max(6)));
    }
    for m in [1, 2, 4, 8] {
        cases.push(brusselator_strip(m, (128 / m).max(6)));
    }
    for w in [8, 16] {
        cases.push(upwind_strip(w, 384 / w, true));
        cases.push(upwind_strip(w, 384 / w, false));
    }
    cases
}

/// The four banded routes of the facade: endpoint and dense output, each with a scalar and a
/// vector absolute tolerance.
fn variants(n: usize) -> Vec<(&'static str, Options)> {
    let base = Options {
        rtol: 1.0e-6,
        atol: Atol::Scalar(1.0e-8),
        ..Options::default()
    };
    let vector = Atol::Vector((0..n).map(|i| 1.0e-8 * (1.0 + (i % 3) as f64)).collect());
    vec![
        ("endpoint", base.clone()),
        (
            "endpoint/vector-atol",
            Options {
                atol: vector.clone(),
                ..base.clone()
            },
        ),
        (
            "dense-output",
            Options {
                dense_output: true,
                ..base.clone()
            },
        ),
        (
            "dense-output/vector-atol",
            Options {
                dense_output: true,
                atol: vector,
                ..base
            },
        ),
    ]
}

/// FNV-1a over the bits of the values.
fn hash_bits(hash: &mut u64, values: &[f64]) {
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            *hash = (*hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

fn record(name: &str, lower: usize, upper: usize, solution: &Solution) -> Value {
    let stats = &solution.stats;
    let mut states = FNV_OFFSET;
    hash_bits(&mut states, &solution.t);
    for y in &solution.y {
        hash_bits(&mut states, y);
    }
    // Under dense output: the continuous extension at seven interior times.
    let (mut interior, mut samples) = (FNV_OFFSET, 0);
    if stats.options.dense_output {
        let (t0, t1) = (solution.t[0], *solution.t.last().unwrap());
        for k in 1..8 {
            if let Some(y) = solution.at(t0 + (t1 - t0) * f64::from(k) / 8.0) {
                hash_bits(&mut interior, &y);
                samples += 1;
            }
        }
    }
    json!({
        "case": name,
        "n": stats.dimension,
        "lower": lower,
        "upper": upper,
        "entry_point": stats.route.entry_point,
        "driver_id": stats.route.driver_id,
        "status": format!("{:?}", solution.status),
        "outputs": solution.t.len(),
        "attempts": stats.attempts,
        "accepted_steps": stats.accepted_steps,
        "rejected_steps": stats.rejected_steps,
        "counters": serde_json::to_value(stats.counters).unwrap(),
        "driver_counters": serde_json::to_value(stats.driver_counters).unwrap(),
        "driver_work": serde_json::to_value(&stats.driver_work).unwrap(),
        "finite_difference": serde_json::to_value(stats.finite_difference).unwrap(),
        "state_hash": format!("{states:016x}"),
        "interior_samples": samples,
        "interior_hash": format!("{interior:016x}"),
    })
}

fn run_all() -> Vec<Value> {
    let mut records = Vec::new();
    for case in cases() {
        for (variant, options) in variants(case.problem.dimension()) {
            let name = format!("{}/{variant}", case.name);
            let solution = Rodas5p::solve(&case.problem, (0.0, 0.5), &case.y0, &options)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(solution.is_success(), "{name}: {:?}", solution.status);
            records.push(record(&name, case.lower, case.upper, &solution));
        }
    }
    records
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/banded_kernel_route/wide_band.json")
}

#[test]
fn banded_facade_runs_keep_their_record() {
    let records = run_all();
    let text = serde_json::to_string_pretty(&Value::Array(records.clone())).unwrap() + "\n";
    let path = golden_path();
    if std::env::var(BLESS_ENV).as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "golden file {} cannot be read ({error}); write it once with {BLESS_ENV}=1",
            path.display()
        )
    });
    if golden == text {
        return;
    }
    let golden: Vec<Value> = serde_json::from_str(&golden).expect("golden file is JSON");
    assert_eq!(golden.len(), records.len(), "number of recorded runs");
    for (old, new) in golden.iter().zip(&records) {
        assert_eq!(
            old, new,
            "run {} differs from its golden record",
            new["case"]
        );
    }
    panic!("the golden file differs from the runs in its formatting only");
}

/// The route labels the rule gives a run: the entry point and the driver identifier.
fn expected_labels(variant: &str, lower: usize, upper: usize) -> (String, &'static str) {
    let slices = lower.min(upper) >= BANDED_SLICES_MIN_HALF_WIDTH;
    let dense = variant.starts_with("dense-output");
    let vector = variant.ends_with("vector-atol");
    let stem = if dense {
        "integrate_rodas5p_fast_banded_dense_observed"
    } else {
        "integrate_rodas5p_fast_banded_observed"
    };
    let suffix = match (vector, slices) {
        (false, false) => "",
        (true, false) => "_with_atol",
        (false, true) => "_with_kernel",
        (true, true) => "_with_atol_and_kernel",
    };
    let driver = match (dense, slices) {
        (false, false) => "rodas5p-fast-banded-v1",
        (false, true) => "rodas5p-fast-banded-v1-slices",
        (true, false) => "rodas5p-fast-banded-dense-v1",
        (true, true) => "rodas5p-fast-banded-dense-v1-slices",
    };
    (format!("{stem}{suffix}"), driver)
}

#[test]
fn the_route_labels_follow_the_width_rule() {
    let (mut on_slices, mut on_indexed) = (0, 0);
    for record in run_all() {
        let case = record["case"].as_str().unwrap();
        // "<problem>/<shape>/<variant>"
        let variant = case.splitn(3, '/').nth(2).unwrap();
        let (lower, upper) = (
            record["lower"].as_u64().unwrap() as usize,
            record["upper"].as_u64().unwrap() as usize,
        );
        let (entry_point, driver) = expected_labels(variant, lower, upper);
        assert_eq!(record["entry_point"], json!(entry_point), "{case}");
        assert_eq!(record["driver_id"], json!(driver), "{case}");
        if driver.ends_with("-slices") {
            on_slices += 1;
        } else {
            on_indexed += 1;
        }
    }
    // Both branches of the rule are exercised, and a band with one narrow side stays on Indexed.
    assert!(on_slices > 0 && on_indexed > 0);
    println!(
        "V22_D2_LABELS width={BANDED_SLICES_MIN_HALF_WIDTH} runs_on_slices={on_slices} runs_on_indexed={on_indexed}"
    );
}

#[test]
fn the_kernel_changes_at_the_declared_width() {
    const { assert!(BANDED_SLICES_MIN_HALF_WIDTH >= 2) };
    let options = Options {
        rtol: 1.0e-6,
        atol: Atol::Scalar(1.0e-8),
        ..Options::default()
    };
    for (width, driver) in [
        (BANDED_SLICES_MIN_HALF_WIDTH - 1, "rodas5p-fast-banded-v1"),
        (
            BANDED_SLICES_MIN_HALF_WIDTH,
            "rodas5p-fast-banded-v1-slices",
        ),
    ] {
        let case = allen_cahn_strip(width, 24);
        let solution = Rodas5p::solve(&case.problem, (0.0, 0.5), &case.y0, &options).unwrap();
        assert!(solution.is_success());
        assert_eq!(
            solution.stats.route.driver_id, driver,
            "half band width {width}"
        );
    }
}
