//! Speed research node SPD06 (`research/spd06_small_static_stages_20261007`):
//! the fixed stage structure (`static_stages`) and the index-loop solves
//! (`static_solves`) of the small-n fast driver against the L-0041 driver,
//! through the library. Identity only; no timing.
//!
//! The problems and configurations are those of the CLI benchmark
//! (`rodas5p stiff-profile-run`): van der Pol (mu = 1000, N = 2), Robertson
//! (N = 3) and HIRES (N = 8), the output schedule `[t0, tf]`, initial step
//! 1e-6, minimum step 1e-14, maximum step the span, 10^6 attempts.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use rodas5p_core::rodas5p_coefficients;
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, RODAS5P_FAST_SMALL_DRIVER_ID, Rodas5pFastOptions,
    Rodas5pFastSmallOptions, Rodas5pFastSmallResult, SmallProblem,
    integrate_rodas5p_fast_small_observed,
    integrate_rodas5p_fast_small_observed_with_small_options, robertson_problem,
    rodas5p_fast_small_static_tables, stiff_van_der_pol_problem,
};
use serde_json::{Value, json};

/// Allocations counted per thread, as in `rodas5p_fast_allocations.rs`.
struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (value, ALLOCATIONS.with(Cell::get) - before)
}

const TOLERANCES: [f64; 7] = [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8, 1.0e-9];

/// van der Pol with the same operations as `stiff_van_der_pol_problem`.
struct VanDerPol(f64);

impl SmallProblem<2> for VanDerPol {
    fn rhs(&self, y: &[f64; 2], out: &mut [f64; 2]) {
        let mu = self.0;
        out[0] = y[1];
        out[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
    }
    fn jacobian(&self, y: &[f64; 2], out: &mut [[f64; 2]; 2]) {
        let mu = self.0;
        out[0][0] = 0.0;
        out[0][1] = 1.0;
        out[1][0] = -2.0 * mu * y[0] * y[1] - 1.0;
        out[1][1] = mu * (1.0 - y[0] * y[0]);
    }
}

/// Robertson with the same operations as `robertson_problem`.
struct Robertson;

impl SmallProblem<3> for Robertson {
    fn rhs(&self, y: &[f64; 3], out: &mut [f64; 3]) {
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
    }
    fn jacobian(&self, y: &[f64; 3], out: &mut [[f64; 3]; 3]) {
        out[0][0] = -0.04;
        out[0][1] = 1.0e4 * y[2];
        out[0][2] = 1.0e4 * y[1];
        out[1][0] = 0.04;
        out[1][1] = -1.0e4 * y[2] - 6.0e7 * y[1];
        out[1][2] = -1.0e4 * y[1];
        out[2][1] = 6.0e7 * y[1];
    }
}

/// HIRES with the same operations as the CLI's `SmallHires`.
struct Hires;

impl SmallProblem<8> for Hires {
    fn rhs(&self, y: &[f64; 8], out: &mut [f64; 8]) {
        out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        out[1] = 1.71 * y[0] - 8.75 * y[1];
        out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
    }
    fn jacobian(&self, y: &[f64; 8], j: &mut [[f64; 8]; 8]) {
        j[0][0] = -1.71;
        j[0][1] = 0.43;
        j[0][2] = 8.32;
        j[1][0] = 1.71;
        j[1][1] = -8.75;
        j[2][2] = -10.03;
        j[2][3] = 0.43;
        j[2][4] = 0.035;
        j[3][1] = 8.32;
        j[3][2] = 1.71;
        j[3][3] = -1.12;
        j[4][4] = -1.745;
        j[4][5] = 0.43;
        j[4][6] = 0.43;
        j[5][3] = 0.69;
        j[5][4] = 1.71;
        j[5][5] = -280.0 * y[7] - 0.43;
        j[5][6] = 0.69;
        j[5][7] = -280.0 * y[5];
        j[6][5] = 280.0 * y[7];
        j[6][6] = -1.81;
        j[6][7] = 280.0 * y[5];
        j[7][5] = -280.0 * y[7];
        j[7][6] = 1.81;
        j[7][7] = -280.0 * y[5];
    }
}

/// The library sub-arms compared with the L-0041 driver: the three CLI arms
/// (`rodas5p-fast-small-static`, `-static-stages`, `-static-ovh`) and the
/// solves-only sub-arm.
fn arms() -> [(&'static str, Rodas5pFastSmallOptions); 4] {
    let ovh = Rodas5pFastOptions {
        prevalidated_controller: true,
        fused_landing: true,
        ..Rodas5pFastOptions::default()
    };
    [
        (
            "rodas5p-fast-small-static",
            Rodas5pFastSmallOptions {
                static_stages: true,
                static_solves: true,
                ..Rodas5pFastSmallOptions::default()
            },
        ),
        (
            "rodas5p-fast-small-static-stages",
            Rodas5pFastSmallOptions {
                static_stages: true,
                ..Rodas5pFastSmallOptions::default()
            },
        ),
        (
            "rodas5p-fast-small-static-ovh",
            Rodas5pFastSmallOptions {
                fast: ovh,
                static_stages: true,
                static_solves: true,
            },
        ),
        (
            "static-solves-only",
            Rodas5pFastSmallOptions {
                static_solves: true,
                ..Rodas5pFastSmallOptions::default()
            },
        ),
    ]
}

const PROBLEMS: [&str; 3] = ["van-der-pol-mu1000", "robertson", "hires"];

/// `(t_span, atol / rtol)` of the CLI benchmark problem.
fn setup(problem: &str) -> ((f64, f64), f64) {
    match problem {
        "van-der-pol-mu1000" => ((0.0, 2000.0), 1.0),
        "robertson" => ((0.0, 40.0), 1.0e-4),
        "hires" => ((0.0, 321.8122), 1.0e-4),
        other => panic!("unknown problem {other}"),
    }
}

fn config(span: (f64, f64), scale: f64, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * scale,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: span.1 - span.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

/// One problem at one tolerance: `None` options is the L-0041 driver (its
/// own entry point), otherwise the SPD06 entry point.
fn run(
    problem: &str,
    rtol: f64,
    options: Option<Rodas5pFastSmallOptions>,
) -> Rodas5pFastSmallResult {
    let (span, scale) = setup(problem);
    let adaptive = config(span, scale, rtol);
    let output = OutputSchedule::new(vec![span.0, span.1]).unwrap();
    macro_rules! go {
        ($p:expr, $y0:expr) => {
            match options {
                None => integrate_rodas5p_fast_small_observed($p, span, $y0, &adaptive, &output),
                Some(options) => integrate_rodas5p_fast_small_observed_with_small_options(
                    $p, span, $y0, &adaptive, &output, options,
                ),
            }
        };
    }
    match problem {
        "van-der-pol-mu1000" => {
            let (_, y0) = stiff_van_der_pol_problem(1000.0).unwrap();
            go!(&VanDerPol(1000.0), &[y0[0], y0[1]])
        }
        "robertson" => {
            let (_, y0) = robertson_problem().unwrap();
            go!(&Robertson, &[y0[0], y0[1], y0[2]])
        }
        "hires" => go!(&Hires, &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057]),
        other => panic!("unknown problem {other}"),
    }
    .unwrap()
}

fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

/// Everything a run reports, with the floats as IEEE bits; the driver id
/// is kept apart (it names the arm).
fn row(r: &Rodas5pFastSmallResult) -> Value {
    let o = &r.observed;
    json!({
        "success": o.success, "message": o.message,
        "t": o.t.iter().map(|v| hx(*v)).collect::<Vec<_>>(),
        "y": o.y.iter().map(|s| s.iter().map(|v| hx(*v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "attempts": r.attempts, "accepted_steps": r.accepted_steps,
        "rejected_steps": r.rejected_steps, "jacobian_reuses": r.jacobian_reuses,
        "internal_steps": o.internal_steps, "output_clipped_steps": o.output_clipped_steps,
        "counters": serde_json::to_value(o.counters).unwrap(),
    })
}

/// The fixed tables against `rodas5p_coefficients()`, bit for bit, with the
/// structure the fixed loops rely on (full strictly lower `A`, `C`; full
/// `b_code`).
fn tables_report() -> Value {
    let coeffs = rodas5p_coefficients().unwrap();
    let (a, c, b) = rodas5p_fast_small_static_tables().unwrap();
    let mut mismatches = Vec::new();
    let mut zero_lower = 0;
    for i in 0..8 {
        for j in 0..8 {
            let (ea, ec) = if j < i {
                (coeffs.a[(i, j)], coeffs.c_matrix[(i, j)])
            } else {
                (0.0, 0.0)
            };
            if a[i][j].to_bits() != ea.to_bits() {
                mismatches.push(format!("a[{i}][{j}]"));
            }
            if c[i][j].to_bits() != ec.to_bits() {
                mismatches.push(format!("c[{i}][{j}]"));
            }
            if j < i && (ea == 0.0 || ec == 0.0) {
                zero_lower += 1;
            }
        }
        if b[i].to_bits() != coeffs.b_code[i].to_bits() {
            mismatches.push(format!("b[{i}]"));
        }
    }
    json!({
        "bitwise_equal": mismatches.is_empty(),
        "mismatches": mismatches,
        "zero_strictly_lower_entries": zero_lower,
        "zero_b_code_entries": coeffs.b_code.iter().filter(|v| **v == 0.0).count(),
        "stages": coeffs.stages(),
        "a": a.iter().map(|r| r.iter().map(|v| hx(*v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "c": c.iter().map(|r| r.iter().map(|v| hx(*v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "b": b.iter().map(|v| hx(*v)).collect::<Vec<_>>(),
    })
}

#[test]
fn the_fixed_tables_equal_the_coefficients_bitwise() {
    let report = tables_report();
    assert_eq!(report["bitwise_equal"], json!(true), "{report}");
    assert_eq!(report["zero_strictly_lower_entries"], json!(0));
    assert_eq!(report["zero_b_code_entries"], json!(0));
    assert_eq!(report["stages"], json!(8));
}

/// A few points of the identity export, for every sub-arm (the export runs
/// all 21).
#[test]
fn every_static_sub_arm_equals_the_l0041_driver_on_sample_points() {
    for problem in PROBLEMS {
        for rtol in [1.0e-3, 1.0e-6] {
            let legacy = run(problem, rtol, None);
            assert!(legacy.observed.success);
            assert_eq!(legacy.driver, RODAS5P_FAST_SMALL_DRIVER_ID);
            let reference = row(&legacy);
            for (name, options) in arms() {
                let r = run(problem, rtol, Some(options));
                assert_eq!(r.driver, options.driver_id());
                assert_eq!(row(&r), reference, "{name} {problem} {rtol:e}");
            }
        }
    }
    // van der Pol at 1e-6 rejects steps and reuses the Jacobian after them,
    // so both paths of the attempt are compared.
    let vdp = run("van-der-pol-mu1000", 1.0e-6, None);
    assert!(vdp.rejected_steps > 0 && vdp.jacobian_reuses > 0);
}

/// An integration that fails (finite-time blow-up, then the minimum step):
/// the failing attempts, the message and the partial output are the same.
#[test]
fn a_failing_integration_is_the_same_under_every_sub_arm() {
    struct Blowup;
    impl SmallProblem<2> for Blowup {
        fn rhs(&self, y: &[f64; 2], out: &mut [f64; 2]) {
            out[0] = y[0] * y[0];
            out[1] = -y[1];
        }
        fn jacobian(&self, y: &[f64; 2], out: &mut [[f64; 2]; 2]) {
            out[0][0] = 2.0 * y[0];
            out[0][1] = 0.0;
            out[1][0] = 0.0;
            out[1][1] = -1.0;
        }
    }
    let span = (0.0, 2.0);
    let adaptive = AdaptiveStepConfig {
        max_attempts: 5_000,
        ..config(span, 1.0, 1.0e-6)
    };
    let output = OutputSchedule::new(vec![0.0, 0.5, 2.0]).unwrap();
    let y0 = [1.0, 1.0];
    let legacy =
        integrate_rodas5p_fast_small_observed(&Blowup, span, &y0, &adaptive, &output).unwrap();
    assert!(!legacy.observed.success);
    for (name, options) in arms() {
        let r = integrate_rodas5p_fast_small_observed_with_small_options(
            &Blowup, span, &y0, &adaptive, &output, options,
        )
        .unwrap();
        assert_eq!(row(&r), row(&legacy), "{name}");
    }
}

/// The SPD06 entry point keeps the entry validation.
#[test]
fn the_static_sub_arms_refuse_invalid_input() {
    let span = (0.0, 40.0);
    let output = OutputSchedule::new(vec![span.0, span.1]).unwrap();
    let mut bad = config(span, 1.0e-4, 1.0e-6);
    bad.safety = f64::NAN;
    let y0 = [1.0, 0.0, 0.0];
    for (_, options) in arms() {
        assert!(
            integrate_rodas5p_fast_small_observed_with_small_options(
                &Robertson, span, &y0, &bad, &output, options
            )
            .is_err()
        );
        let good = config(span, 1.0e-4, 1.0e-6);
        assert!(
            integrate_rodas5p_fast_small_observed_with_small_options(
                &Robertson,
                span,
                &[f64::NAN, 0.0, 0.0],
                &good,
                &output,
                options
            )
            .is_err()
        );
    }
}

/// Allocation sanity (reported by the node, not gated): no sub-arm
/// allocates per step, and the fixed stage structure makes none of the
/// coefficient lists' allocations.
#[test]
fn the_static_sub_arms_allocate_nothing_per_step() {
    // Warm the coefficient cache first.
    run("robertson", 1.0e-6, None);
    let counts = |options: Option<Rodas5pFastSmallOptions>| {
        [1.0e-4, 1.0e-8].map(|rtol| {
            let (r, n) = allocations_during(|| run("robertson", rtol, options));
            (r.attempts, n)
        })
    };
    let legacy = counts(None);
    assert!(legacy[1].0 > 2 * legacy[0].0, "{legacy:?}");
    assert_eq!(legacy[0].1, legacy[1].1, "{legacy:?}");
    println!("L-0041: {legacy:?}");
    for (name, options) in arms() {
        let c = counts(Some(options));
        println!("{name}: {c:?}");
        assert_eq!(c[0].1, c[1].1, "{name}: {c:?}");
        if options.static_stages {
            assert!(c[0].1 < legacy[0].1, "{name}: {c:?} vs {legacy:?}");
        } else {
            assert_eq!(c[0].1, legacy[0].1, "{name}");
        }
    }
}

/// Identity export: the 21 points, every sub-arm against the L-0041 driver.
#[test]
#[ignore = "identity export of research/spd06_small_static_stages_20261007; release build; set SPD06_IDENTITY"]
fn spd06_identity_export() {
    let Ok(path) = std::env::var("SPD06_IDENTITY") else {
        println!("SPD06_IDENTITY not set: nothing written");
        return;
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    assert!(
        !path.exists(),
        "immutable output exists: {}",
        path.display()
    );
    let tables = tables_report();
    let mut rows = Vec::new();
    let mut all = serde_json::Map::new();
    for (name, _) in arms() {
        all.insert(name.into(), json!(true));
    }
    for problem in PROBLEMS {
        for rtol in TOLERANCES {
            let legacy = run(problem, rtol, None);
            let reference = row(&legacy);
            let mut entry = json!({
                "problem": problem, "rtol": rtol,
                "legacy": {"driver": legacy.driver, "row": reference.clone()},
                "arms": {},
            });
            let mut line = format!("{problem} {rtol:e}: attempts {}", legacy.attempts);
            for (name, options) in arms() {
                let r = run(problem, rtol, Some(options));
                let this = row(&r);
                let identical = this == reference;
                if !identical {
                    all[name] = json!(false);
                }
                // The full row is kept only when it differs.
                entry["arms"][name] = json!({
                    "driver": r.driver, "identical": identical,
                    "row": if identical { Value::Null } else { this },
                });
                line.push_str(&format!(", {name} {identical}"));
            }
            println!("{line}");
            rows.push(entry);
        }
    }
    let out = json!({
        "schema": "vigilode-spd06-identity-v1",
        "reference": "integrate_rodas5p_fast_small_observed (L-0041)",
        "tables": tables,
        "all_identical": all,
        "rows": rows,
    });
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
    println!("wrote {}", path.display());
}
