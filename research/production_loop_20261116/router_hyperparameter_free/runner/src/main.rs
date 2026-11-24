//! V27 evaluation runner: one (arm, problem, tolerance, options) case per process and one
//! JSON line of raw counters on stdout (research loop L1, node V27_ROUTER_HYPERPARAMETER_FREE,
//! `PREDECLARED.json` in the parent directory).
//!
//! ```text
//! v27-runner --arm <N|T|D|R> --problem <id> [--rtol x --atol y] [--max-steps k]
//!            [--regime unknown|stiff|nonstiff] [--no-log]
//! ```
//!
//! * `N` and `T` run `VigilODE::solve` (strategy Auto with the declared regime). The binary
//!   built with the feature `pilots` against this worktree is arm N; the binary built
//!   without it against the base checkout is arm T; each refuses the other's arm.
//! * `D` runs `Dopri5::solve`, `R` runs `Rodas5p::solve`, with the same options.
//!
//! The line holds: status, typed stop, success, final time and state (values and bits), the
//! charged counters, attempts and steps, callback entries counted inside the closures by
//! callback, the right-hand-side entries classified as stage, finite-difference column or
//! finite-difference time derivative, operations by the one formula of `PREDECLARED.json`,
//! the legs, route history and pilot records (arm N), and an FNV-1a 64 digest of the route.
//! No wall time is measured. Only names that exist in both APIs are used outside the
//! `pilots` feature.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use vigilode::{
    Atol, CallbackError, Dopri5, Jacobian, Options, Problem, ProblemRegime, Rodas5p,
    SelectedTier, SolverStrategy, VigilODE, VigilodeOptions, VigilodeSolution,
};

/// The build this binary is: arm N (with the pilot records) or arm T.
const BUILD: &str = if cfg!(feature = "pilots") { "N" } else { "T" };

// ------------------------------------------------------------------------- arguments

struct Args {
    arm: String,
    problem: String,
    rtol: Option<f64>,
    atol: Option<f64>,
    max_steps: Option<usize>,
    regime: ProblemRegime,
    log: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        arm: String::new(),
        problem: String::new(),
        rtol: None,
        atol: None,
        max_steps: None,
        regime: ProblemRegime::Unknown,
        log: true,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--arm" => args.arm = value()?,
            "--problem" => args.problem = value()?,
            "--rtol" => args.rtol = Some(value()?.parse().map_err(|e| format!("rtol: {e}"))?),
            "--atol" => args.atol = Some(value()?.parse().map_err(|e| format!("atol: {e}"))?),
            "--max-steps" => {
                args.max_steps = Some(value()?.parse().map_err(|e| format!("max-steps: {e}"))?)
            }
            "--regime" => {
                args.regime = match value()?.as_str() {
                    "unknown" => ProblemRegime::Unknown,
                    "stiff" => ProblemRegime::Stiff,
                    "nonstiff" => ProblemRegime::Nonstiff,
                    other => return Err(format!("unknown regime {other}")),
                }
            }
            "--no-log" => args.log = false,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !["N", "T", "D", "R"].contains(&args.arm.as_str()) {
        return Err(format!("--arm must be N, T, D or R, got {:?}", args.arm));
    }
    if (args.arm == "N" || args.arm == "T") && args.arm != BUILD {
        return Err(format!("this binary is the build of arm {BUILD}, not {}", args.arm));
    }
    Ok(args)
}

// ------------------------------------------------------------------- callback entries

/// Callback entries counted inside the closures, and the right-hand-side entries in order.
#[derive(Clone, Default)]
struct Entries {
    rhs: Arc<AtomicU64>,
    jacobian: Arc<AtomicU64>,
    jvp: Arc<AtomicU64>,
    time_derivative: Arc<AtomicU64>,
    fatal: Arc<AtomicU64>,
    /// `(t, y)` of every right-hand-side entry, flat: `t, y_0, ..., y_{n-1}` per entry.
    log: Option<Arc<Mutex<Vec<f64>>>>,
}

impl Entries {
    fn count(counter: &AtomicU64) -> u64 {
        counter.load(Ordering::Relaxed)
    }
}

type Rhs = Box<dyn Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;

/// A problem whose right-hand side counts and logs its entries before it calls `f`.
fn counted(n: usize, entries: &Entries, f: Rhs) -> Problem {
    let (rhs, log) = (entries.rhs.clone(), entries.log.clone());
    Problem::new(n, move |t: f64, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        if let Some(log) = &log {
            let mut log = log.lock().unwrap();
            log.push(t);
            log.extend_from_slice(y);
        }
        f(t, y, dy)
    })
}

fn plain(f: impl Fn(&[f64], &mut [f64]) + Send + Sync + 'static) -> Rhs {
    Box::new(move |_t, y, dy| {
        f(y, dy);
        Ok(())
    })
}

fn timed(f: impl Fn(f64, &[f64], &mut [f64]) + Send + Sync + 'static) -> Rhs {
    Box::new(move |t, y, dy| {
        f(t, y, dy);
        Ok(())
    })
}

// --------------------------------------------------------------------------- problems

/// One problem of the evaluation: the problem, its span and initial state.
struct Case {
    problem: Problem,
    span: (f64, f64),
    y0: Vec<f64>,
}

fn heat_rhs(n: usize, y: &[f64], dy: &mut [f64]) {
    let inv_dx2 = ((n + 1) as f64).powi(2);
    for i in 0..n {
        let left = if i == 0 { 0.0 } else { y[i - 1] };
        let right = if i + 1 == n { 0.0 } else { y[i + 1] };
        dy[i] = (left - 2.0 * y[i] + right) * inv_dx2;
    }
}

fn heat_sine(n: usize) -> Vec<f64> {
    (1..=n)
        .map(|i| (std::f64::consts::PI * i as f64 / (n + 1) as f64).sin())
        .collect()
}

fn robertson(y: &[f64], dy: &mut [f64]) {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
}

/// The heat equation with zero Dirichlet ends, `dx = 1 / (n + 1)`, finite differences.
fn heat_fd(n: usize, entries: &Entries) -> Problem {
    counted(n, entries, plain(move |y, dy| heat_rhs(n, y, dy))).autonomous()
}

fn heat_dense(n: usize, entries: &Entries) -> Problem {
    let jacobian = entries.jacobian.clone();
    heat_fd(n, entries).jacobian(Jacobian::dense(
        move |_t, _y, jac: &mut [f64]| {
            jacobian.fetch_add(1, Ordering::Relaxed);
            let inv_dx2 = ((n + 1) as f64).powi(2);
            for i in 0..n {
                jac[i * n + i] = -2.0 * inv_dx2;
                if i > 0 {
                    jac[i * n + i - 1] = inv_dx2;
                }
                if i + 1 < n {
                    jac[i * n + i + 1] = inv_dx2;
                }
            }
            Ok(())
        },
    ))
}

fn heat_band_callback(n: usize, entries: &Entries) -> Problem {
    let jacobian = entries.jacobian.clone();
    heat_fd(n, entries).jacobian(Jacobian::banded(
        1,
        1,
        move |_t, _y, band: &mut [f64]| {
            jacobian.fetch_add(1, Ordering::Relaxed);
            let inv_dx2 = ((n + 1) as f64).powi(2);
            for i in 0..n {
                band[i * 3] = inv_dx2;
                band[i * 3 + 1] = -2.0 * inv_dx2;
                band[i * 3 + 2] = inv_dx2;
            }
            Ok(())
        },
    ))
}

fn heat_jvp(n: usize, entries: &Entries) -> Problem {
    let jvp = entries.jvp.clone();
    heat_fd(n, entries).jvp(move |_t, _y, v: &[f64], out: &mut [f64]| {
        jvp.fetch_add(1, Ordering::Relaxed);
        heat_rhs(n, v, out);
        Ok(())
    })
}

/// `y' = -1e6 (y - cos t)` (P_scalar_fd), non-autonomous.
fn scalar_fd(entries: &Entries) -> Problem {
    counted(
        1,
        entries,
        timed(|t, y, dy| dy[0] = -1.0e6 * (y[0] - t.cos())),
    )
}

/// A right-hand side that returns a fatal error when `fails(t, y)`.
fn guarded(
    f: impl Fn(&[f64], &mut [f64]) + Send + Sync + 'static,
    fails: impl Fn(f64, &[f64]) -> bool + Send + Sync + 'static,
    message: &'static str,
    fatal: Arc<AtomicU64>,
) -> Rhs {
    Box::new(move |t, y, dy| {
        if fails(t, y) {
            fatal.fetch_add(1, Ordering::Relaxed);
            return Err(CallbackError::Fatal(message.into()));
        }
        f(y, dy);
        Ok(())
    })
}

fn pleiades(y: &[f64], dy: &mut [f64]) {
    // state: x_1..x_7, y_1..y_7, x'_1..x'_7, y'_1..y'_7; masses m_j = j
    for i in 0..7 {
        dy[i] = y[14 + i];
        dy[7 + i] = y[21 + i];
        let (mut ax, mut ay) = (0.0, 0.0);
        for j in 0..7 {
            if j == i {
                continue;
            }
            let (dx, dyy) = (y[j] - y[i], y[7 + j] - y[7 + i]);
            let r2 = dx * dx + dyy * dyy;
            let r3 = r2 * r2.sqrt();
            let m = (j + 1) as f64;
            ax += m * dx / r3;
            ay += m * dyy / r3;
        }
        dy[14 + i] = ax;
        dy[21 + i] = ay;
    }
}

fn build(id: &str, entries: &Entries) -> Option<Case> {
    let case = |problem: Problem, span: (f64, f64), y0: Vec<f64>| Some(Case { problem, span, y0 });
    let auto = |n: usize, f: Rhs| counted(n, entries, f).autonomous();
    let fatal = entries.fatal.clone();
    match id {
        // plumbing check of phase 1, outside the deciding set
        "toy" => case(auto(1, plain(|y, dy| dy[0] = -y[0])), (0.0, 1.0), vec![1.0]),
        // stiff or late stiff
        "P_scalar_fd" => case(scalar_fd(entries), (0.0, 1.0), vec![1.0]),
        "lin_2000" | "lin_3200" | "lin_3400" => {
            let lambda: f64 = id[4..].parse().ok()?;
            case(auto(1, plain(move |y, dy| dy[0] = -lambda * y[0])), (0.0, 1.0), vec![1.0])
        }
        "P_heat_fd" => case(heat_fd(8, entries), (0.0, 100.0), heat_sine(8)),
        "robertson_40" => case(auto(3, plain(robertson)), (0.0, 40.0), vec![1.0, 0.0, 0.0]),
        "robertson_1e5" => case(auto(3, plain(robertson)), (0.0, 1.0e5), vec![1.0, 0.0, 0.0]),
        "hires" => case(
            auto(
                8,
                plain(|y, dy| {
                    let f7 = 280.0 * y[5] * y[7] - 1.81 * y[6];
                    dy[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
                    dy[1] = 1.71 * y[0] - 8.75 * y[1];
                    dy[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
                    dy[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
                    dy[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
                    dy[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5]
                        + 0.69 * y[6];
                    dy[6] = f7;
                    dy[7] = -f7;
                }),
            ),
            (0.0, 321.8122),
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
        ),
        "orego" => case(
            auto(
                3,
                plain(|y, dy| {
                    dy[0] = 77.27 * (y[1] + y[0] * (1.0 - 8.375e-6 * y[0] - y[1]));
                    dy[1] = (y[2] - (1.0 + y[0]) * y[1]) / 77.27;
                    dy[2] = 0.161 * (y[0] - y[2]);
                }),
            ),
            (0.0, 360.0),
            vec![1.0, 2.0, 3.0],
        ),
        "vdp_1000" | "vdp_1" => {
            let (mu, t1) = if id == "vdp_1000" { (1000.0, 1.0) } else { (1.0, 20.0) };
            case(
                auto(
                    2,
                    plain(move |y, dy| {
                        dy[0] = y[1];
                        dy[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
                    }),
                ),
                (0.0, t1),
                vec![2.0, 0.0],
            )
        }
        "flame_1e-2" | "flame_1e-3" | "flame_1e-4" => {
            let delta: f64 = id[6..].parse().ok()?;
            case(
                auto(1, plain(|y, dy| dy[0] = y[0] * y[0] - y[0] * y[0] * y[0])),
                (0.0, 2.0 / delta),
                vec![delta],
            )
        }
        "heat_64_fd" => case(heat_fd(64, entries), (0.0, 10.0), heat_sine(64)),
        "kt_vanishing" => case(
            counted(
                1,
                entries,
                timed(|t, y, dy| {
                    let k = 1.0e4 * (-20.0 * t).exp() + 1.0;
                    dy[0] = -k * (y[0] - t.sin());
                }),
            ),
            (0.0, 10.0),
            vec![0.0],
        ),
        // non-stiff
        "osc_1" | "osc_10" | "osc_1000" => {
            let t1: f64 = id[4..].parse().ok()?;
            case(
                auto(
                    2,
                    plain(|y, dy| {
                        dy[0] = y[1];
                        dy[1] = -y[0];
                    }),
                ),
                (0.0, t1),
                vec![1.0, 0.0],
            )
        }
        "lotka_volterra" => case(
            auto(
                2,
                plain(|y, dy| {
                    dy[0] = 1.5 * y[0] - y[0] * y[1];
                    dy[1] = -3.0 * y[1] + y[0] * y[1];
                }),
            ),
            (0.0, 100.0),
            vec![1.0, 1.0],
        ),
        "lorenz" => case(
            auto(
                3,
                plain(|y, dy| {
                    dy[0] = 10.0 * (y[1] - y[0]);
                    dy[1] = y[0] * (28.0 - y[2]) - y[1];
                    dy[2] = y[0] * y[1] - 8.0 / 3.0 * y[2];
                }),
            ),
            (0.0, 10.0),
            vec![1.0, 1.0, 1.0],
        ),
        "arenstorf" => case(
            auto(
                4,
                plain(|y, dy| {
                    let mu = 0.012277471;
                    let mup = 1.0 - mu;
                    let d1 = ((y[0] + mu).powi(2) + y[1] * y[1]).powf(1.5);
                    let d2 = ((y[0] - mup).powi(2) + y[1] * y[1]).powf(1.5);
                    dy[0] = y[2];
                    dy[1] = y[3];
                    dy[2] = y[0] + 2.0 * y[3] - mup * (y[0] + mu) / d1 - mu * (y[0] - mup) / d2;
                    dy[3] = y[1] - 2.0 * y[2] - mup * y[1] / d1 - mu * y[1] / d2;
                }),
            ),
            (0.0, 17.065_216_560_157_964),
            vec![0.994, 0.0, 0.0, -2.001_585_106_379_082_4],
        ),
        "brusselator_2" => case(
            auto(
                2,
                plain(|y, dy| {
                    dy[0] = 1.0 + y[0] * y[0] * y[1] - 4.0 * y[0];
                    dy[1] = 3.0 * y[0] - y[0] * y[0] * y[1];
                }),
            ),
            (0.0, 20.0),
            vec![1.5, 3.0],
        ),
        "damped_osc" => case(
            auto(
                2,
                plain(|y, dy| {
                    dy[0] = y[1];
                    dy[1] = -1.0e4 * y[0] - 0.2 * y[1];
                }),
            ),
            (0.0, 10.0),
            vec![1.0, 0.0],
        ),
        "nonnormal" => case(
            auto(
                2,
                plain(|y, dy| {
                    dy[0] = -y[0] + 1.0e3 * y[1];
                    dy[1] = -2.0 * y[1];
                }),
            ),
            (0.0, 10.0),
            vec![1.0, 1.0],
        ),
        "lorenz96_40" => {
            let mut y0 = vec![8.0; 40];
            y0[19] += 0.01;
            case(
                auto(
                    40,
                    plain(|y, dy| {
                        let n = y.len();
                        for i in 0..n {
                            dy[i] = (y[(i + 1) % n] - y[(i + n - 2) % n]) * y[(i + n - 1) % n]
                                - y[i]
                                + 8.0;
                        }
                    }),
                ),
                (0.0, 10.0),
                y0,
            )
        }
        "pleiades" => {
            let mut y0 = vec![0.0; 28];
            y0[..7].copy_from_slice(&[3.0, 3.0, -1.0, -3.0, 2.0, -2.0, 2.0]);
            y0[7..14].copy_from_slice(&[3.0, -3.0, 2.0, 0.0, 0.0, -4.0, 4.0]);
            y0[14..21].copy_from_slice(&[0.0, 0.0, 0.0, 0.0, 0.0, 1.75, -1.5]);
            y0[21..28].copy_from_slice(&[0.0, 0.0, 0.0, -1.25, 1.0, 0.0, 0.0]);
            case(auto(28, plain(pleiades)), (0.0, 3.0), y0)
        }
        // callbacks, budget, declared facts
        "F2" => case(
            auto(1, guarded(|y, dy| dy[0] = -y[0], |_t, y| y[0] > 1.0, "F2: y > 1", fatal)),
            (0.0, 1.0),
            vec![1.0],
        ),
        "N2" => case(
            auto(
                2,
                guarded(
                    |y, dy| {
                        dy[0] = 0.0;
                        dy[1] = -1000.0 * y[1];
                    },
                    |_t, y| y[0] > 1.0,
                    "N2: y1 > 1",
                    fatal,
                ),
            ),
            (0.0, 1.0),
            vec![1.0, 1.0],
        ),
        "P_fatal" => case(
            auto(1, guarded(|y, dy| dy[0] = -y[0], |t, _y| t > 0.3, "sensor offline", fatal)),
            (0.0, 1.0),
            vec![1.0],
        ),
        "P_blowup" => {
            let jacobian = entries.jacobian.clone();
            case(
                auto(1, plain(|y, dy| dy[0] = y[0] * y[0])).jacobian(Jacobian::dense(
                    move |_t, y: &[f64], jac: &mut [f64]| {
                        jacobian.fetch_add(1, Ordering::Relaxed);
                        jac[0] = 2.0 * y[0];
                        Ok(())
                    },
                )),
                (0.0, 2.0),
                vec![1.0],
            )
        }
        "P_mass" => {
            let jacobian = entries.jacobian.clone();
            case(
                auto(
                    3,
                    plain(|y, dy| {
                        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
                        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
                        dy[2] = y[0] + y[1] + y[2] - 1.0;
                    }),
                )
                .jacobian(Jacobian::dense(move |_t, y: &[f64], jac: &mut [f64]| {
                    jacobian.fetch_add(1, Ordering::Relaxed);
                    jac.copy_from_slice(&[
                        -0.04,
                        1.0e4 * y[2],
                        1.0e4 * y[1],
                        0.04,
                        -1.0e4 * y[2] - 6.0e7 * y[1],
                        -1.0e4 * y[1],
                        1.0,
                        1.0,
                        1.0,
                    ]);
                    Ok(())
                }))
                .mass(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
                (0.0, 1.0),
                vec![1.0, 0.0, 0.0],
            )
        }
        // the start rule (owner decision D2), the spans of the V03b fixtures
        "P_heat_dense" => case(heat_dense(8, entries), (0.0, 10.0), heat_sine(8)),
        "P_scalar_dense" => {
            let jacobian = entries.jacobian.clone();
            let ft = entries.time_derivative.clone();
            case(
                scalar_fd(entries)
                    .jacobian(Jacobian::dense(move |_t, _y, jac: &mut [f64]| {
                        jacobian.fetch_add(1, Ordering::Relaxed);
                        jac[0] = -1.0e6;
                        Ok(())
                    }))
                    .time_derivative(move |t: f64, _y: &[f64], out: &mut [f64]| {
                        ft.fetch_add(1, Ordering::Relaxed);
                        out[0] = -1.0e6 * t.sin();
                        Ok(())
                    }),
                (0.0, 1.0),
                vec![1.0],
            )
        }
        "P_heat_jvp" => case(heat_jvp(8, entries), (0.0, 0.01), vec![1.0; 8]),
        "P_heat_banded" => case(heat_band_callback(8, entries), (0.0, 0.01), vec![1.0; 8]),
        _ => None,
    }
}

// ----------------------------------------------------------------- classification

/// Classify the right-hand-side entries: a finite-difference column differs from an entry
/// at the same time in exactly one component, by at most twice the facade's increment
/// `sqrt(eps) max(|y_j|, 1)`; a finite-difference time derivative has the state of an
/// adjacent entry and a time within twice `sqrt(eps) max(|t|, 1)` of it; every other entry
/// is a stage (the first stage `f(t, y)` of an attempt included).
fn classify(n: usize, flat: &[f64], t0: f64, y0: &[f64]) -> Value {
    let width = n + 1;
    let count = flat.len() / width;
    let entry = |i: usize| (flat[i * width], &flat[i * width + 1..(i + 1) * width]);
    let bits = |y: &[f64]| y.iter().map(|v| v.to_bits()).collect::<Vec<u64>>();
    let eps = f64::EPSILON.sqrt();
    // entries by time, and the most frequent state at each time (the base of a column)
    let mut by_time: HashMap<u64, Vec<usize>> = HashMap::new();
    for i in 0..count {
        by_time.entry(entry(i).0.to_bits()).or_default().push(i);
    }
    let mut base: HashMap<u64, Vec<u64>> = HashMap::new();
    for (t, members) in &by_time {
        // states in order of first occurrence with their frequency; the base is the most
        // frequent one, the earliest on a tie (deterministic: no hash order is read)
        let mut states: Vec<(Vec<u64>, usize)> = Vec::new();
        for &i in members {
            let state = bits(entry(i).1);
            match states.iter_mut().find(|(s, _)| *s == state) {
                Some((_, frequency)) => *frequency += 1,
                None => states.push((state, 1)),
            }
        }
        let best = states
            .iter()
            .enumerate()
            .max_by(|(ia, a), (ib, b)| a.1.cmp(&b.1).then(ib.cmp(ia)))
            .map(|(_, (state, _))| state.clone())
            .unwrap_or_default();
        base.insert(*t, best);
    }
    let (mut stages, mut columns, mut time_derivatives) = (0u64, 0u64, 0u64);
    for i in 0..count {
        let (t, y) = entry(i);
        let b = &base[&t.to_bits()];
        let differing: Vec<usize> = (0..n).filter(|&j| y[j].to_bits() != b[j]).collect();
        if differing.len() == 1 {
            let j = differing[0];
            let reference = f64::from_bits(b[j]);
            if (y[j] - reference).abs() <= 2.0 * eps * reference.abs().max(1.0) {
                columns += 1;
                continue;
            }
        }
        let adjacent = [i.wrapping_sub(1), i + 1].into_iter().filter(|&k| k < count);
        let derivative = adjacent.into_iter().any(|k| {
            let (tk, yk) = entry(k);
            tk != t && bits(yk) == bits(y) && (tk - t).abs() <= 4.0 * eps * t.abs().max(1.0)
        });
        if derivative {
            time_derivatives += 1;
        } else {
            stages += 1;
        }
    }
    let at_t0 = (0..count).filter(|&i| entry(i).0 == t0).count();
    let at_t0_other_state = (0..count)
        .filter(|&i| entry(i).0 == t0 && bits(entry(i).1) != bits(y0))
        .count();
    let first_is_t0_y0 = count > 0 && entry(0).0 == t0 && bits(entry(0).1) == bits(y0);
    json!({
        "stage": stages,
        "fd_column": columns,
        "fd_time_derivative": time_derivatives,
        "at_t0": at_t0,
        "at_t0_not_y0": at_t0_other_state,
        "first_is_t0_y0": first_is_t0_y0,
    })
}

// ------------------------------------------------------------------------- reporting

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn state_bits(y: &[f64]) -> Vec<String> {
    y.iter().map(|v| format!("{:016x}", v.to_bits())).collect()
}

/// Three times the operations of `PREDECLARED.json`: 52 n per DOPRI5 attempt, 128 n per
/// RODAS5P attempt, 2 n^3 / 3 per factorization, 2 n^2 per solve pair and per
/// finite-difference build.
fn ops3(n: u64, dopri5: u64, rodas5p: u64, factorizations: u64, solves: u64, builds: u64) -> u128 {
    let n = u128::from(n);
    156 * n * u128::from(dopri5)
        + 384 * n * u128::from(rodas5p)
        + 2 * n.pow(3) * u128::from(factorizations)
        + 6 * n * n * (u128::from(solves) + u128::from(builds))
}

fn unified_fields(n: usize, solution: &VigilodeSolution) -> Value {
    let dopri5_attempts: u64 = solution
        .legs
        .iter()
        .filter(|leg| leg.tier == SelectedTier::Tier1Explicit)
        .map(|leg| leg.attempts as u64)
        .sum();
    let rodas5p_attempts = (solution.stats.attempts as u64).saturating_sub(dopri5_attempts);
    let c = &solution.stats.counters;
    let operations3 = ops3(
        n as u64,
        dopri5_attempts,
        rodas5p_attempts,
        c.direct_factorizations,
        c.linear_solves,
        c.jacobian_builds,
    );
    let legs: Vec<Value> = solution
        .legs
        .iter()
        .map(|leg| {
            json!({
                "tier": leg.tier, "driver_id": leg.driver_id, "route_id": leg.route_id,
                "t_start": leg.t_start, "t_end": leg.t_end,
                "t_start_bits": format!("{:016x}", leg.t_start.to_bits()),
                "t_end_bits": format!("{:016x}", leg.t_end.to_bits()),
                "success": leg.success, "attempt_budget": leg.attempt_budget,
                "attempts": leg.attempts, "accepted_steps": leg.accepted_steps,
                "rejected_steps": leg.rejected_steps, "counters": leg.counters, "stop": leg.stop,
            })
        })
        .collect();
    let history = serde_json::to_value(&solution.route_history).unwrap_or(Value::Null);
    #[cfg(feature = "pilots")]
    let pilots = serde_json::to_value(&solution.pilots).unwrap_or(Value::Null);
    #[cfg(not(feature = "pilots"))]
    let pilots = Value::Null;
    let digest = fnv1a64(json!([legs, history, pilots]).to_string().as_bytes());
    json!({
        "success": solution.success(),
        "status": solution.status,
        "stop": solution.stop,
        "final_time": solution.final_time,
        "final_time_bits": format!("{:016x}", solution.final_time.to_bits()),
        "final_state": solution.final_state,
        "final_state_bits": state_bits(&solution.final_state),
        "counters": solution.stats.counters,
        "attempts": solution.stats.attempts,
        "accepted_steps": solution.stats.accepted_steps,
        "rejected_steps": solution.stats.rejected_steps,
        "dopri5_attempts": dopri5_attempts,
        "rodas5p_attempts": rodas5p_attempts,
        "operations3": operations3.to_string(),
        "operations": operations3 as f64 / 3.0,
        "analysis_basis": solution.analysis.basis,
        "legs": legs,
        "route_history": history,
        "pilots": pilots,
        "route_digest": format!("{digest:016x}"),
    })
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("v27-runner: {message}");
            std::process::exit(2);
        }
    };
    let mut entries = Entries::default();
    if args.log {
        entries.log = Some(Arc::default());
    }
    let Some(case) = build(&args.problem, &entries) else {
        eprintln!("v27-runner: unknown problem {}", args.problem);
        std::process::exit(2);
    };
    let n = case.y0.len();
    let mut options = Options::default();
    if let Some(rtol) = args.rtol {
        options.rtol = rtol;
    }
    if let Some(atol) = args.atol {
        options.atol = Atol::Scalar(atol);
    }
    if let Some(max_steps) = args.max_steps {
        options.max_steps = max_steps;
    }
    let (problem, span, y0) = (&case.problem, case.span, &case.y0);
    let run: Value = match args.arm.as_str() {
        "N" | "T" => {
            let opts = VigilodeOptions::new(options.clone())
                .strategy(SolverStrategy::Auto)
                .regime(args.regime);
            match VigilODE::solve(problem, span, y0, opts) {
                Ok(solution) => unified_fields(n, &solution),
                Err(error) => json!({ "error": error.to_string() }),
            }
        }
        "D" => match Dopri5::solve(problem, span, y0, &options) {
            Ok(s) => {
                let attempts = s.stats.attempts as u64;
                let operations3 = ops3(n as u64, attempts, 0, 0, 0, 0);
                json!({
                    "success": s.success, "status": s.message,
                    "stop": { "reason": format!("{:?}", s.stop_reason),
                              "callback": s.callback_failure.as_ref().map(|c| json!({
                                  "t": c.t, "message": c.message, "malformed": c.malformed_output })) },
                    "final_time": s.final_time,
                    "final_time_bits": format!("{:016x}", s.final_time.to_bits()),
                    "final_state": s.final_state, "final_state_bits": state_bits(&s.final_state),
                    "counters": s.stats.counters, "attempts": s.stats.attempts,
                    "accepted_steps": s.stats.accepted_steps, "rejected_steps": s.stats.rejected_steps,
                    "dopri5_attempts": attempts, "rodas5p_attempts": 0,
                    "operations3": operations3.to_string(), "operations": operations3 as f64 / 3.0,
                })
            }
            Err(error) => json!({ "error": error.to_string() }),
        },
        _ => match Rodas5p::solve(problem, span, y0, &options) {
            Ok(s) => {
                let c = &s.stats.counters;
                let attempts = s.stats.attempts as u64;
                let operations3 = ops3(
                    n as u64,
                    0,
                    attempts,
                    c.direct_factorizations,
                    c.linear_solves,
                    c.jacobian_builds,
                );
                let final_state = s.final_state().map(<[f64]>::to_vec).unwrap_or_default();
                let final_time = s.t.last().copied().unwrap_or(span.0);
                json!({
                    "success": s.is_success(), "status": s.status, "stop": s.stats.stop,
                    "driver_id": s.stats.route.driver_id,
                    "final_time": final_time,
                    "final_time_bits": format!("{:016x}", final_time.to_bits()),
                    "final_state_bits": state_bits(&final_state), "final_state": final_state,
                    "counters": s.stats.counters, "attempts": s.stats.attempts,
                    "accepted_steps": s.stats.accepted_steps, "rejected_steps": s.stats.rejected_steps,
                    "dopri5_attempts": 0, "rodas5p_attempts": attempts,
                    "operations3": operations3.to_string(), "operations": operations3 as f64 / 3.0,
                })
            }
            Err(error) => json!({ "error": error.to_string() }),
        },
    };
    let classified = entries
        .log
        .as_ref()
        .map(|log| classify(n, &log.lock().unwrap(), span.0, y0))
        .unwrap_or(Value::Null);
    let line = json!({
        "schema": "vigilode.v27.runner.v1",
        "build": BUILD,
        "arm": args.arm,
        "problem": args.problem,
        "n": n,
        "span": [span.0, span.1],
        "options": options,
        "regime": format!("{:?}", args.regime),
        "entries": {
            "rhs": Entries::count(&entries.rhs),
            "jacobian": Entries::count(&entries.jacobian),
            "jvp": Entries::count(&entries.jvp),
            "time_derivative": Entries::count(&entries.time_derivative),
            "fatal": Entries::count(&entries.fatal),
            "rhs_by_kind": classified,
        },
        "run": run,
    });
    println!("{line}");
}
