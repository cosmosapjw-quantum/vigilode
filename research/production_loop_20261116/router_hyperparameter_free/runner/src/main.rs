//! V27 evaluation runner: one (arm, problem, tolerance, options) case per process and one
//! JSON line of raw counters on stdout (research loop L1, node V27_ROUTER_HYPERPARAMETER_FREE,
//! `PREDECLARED.json` in the parent directory).
//!
//! ```text
//! v27-runner --arm <N|T|D|R> --problem <id> [--rtol x --atol y] [--max-steps k]
//!            [--regime unknown|stiff|nonstiff] [--no-log] [--repeat k]
//! ```
//!
//! * `N` and `T` run `VigilODE::solve` (strategy Auto with the declared regime). The binary
//!   built with the feature `pilots` against this worktree is arm N; the binary built
//!   without it against the base checkout is arm T; each refuses the other's arm.
//! * `D` runs `Dopri5::solve`, `R` runs `Rodas5p::solve`, with the same options.
//! * `--repeat k` (phase 2) runs the case `k` times in this process, each time with a fresh
//!   problem and fresh entry counters, and prints one line per run (`repeat`, `pid`).
//!
//! The line holds: status, typed stop, success, final time and state (values and bits), the
//! charged counters, attempts and steps, callback entries counted inside the closures by
//! callback, the right-hand-side entries classified as stage, finite-difference column or
//! finite-difference time derivative, operations by the one formula of `PREDECLARED.json`,
//! the legs, route history and pilot records (arm N), and an FNV-1a 64 digest of the route.
//! Arm N with the entry log also carries `e5`, the split of its entries into those of
//! `Dopri5::solve`, of each replayed pilot and of the replayed RODAS5P run after a handover
//! ([`e5_split`]); the replays run on fresh copies of the problem and are not charged to the
//! case. No wall time is measured. Only names that exist in both APIs are used outside the
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
    repeat: usize,
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
        repeat: 1,
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
            "--repeat" => {
                args.repeat = value()?.parse().map_err(|e| format!("repeat: {e}"))?;
                if args.repeat == 0 {
                    return Err("--repeat must be at least 1".into());
                }
            }
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
        // plumbing checks of the phase-2 runner change (handover, lost pilots, time
        // derivative, fatal pilot), outside the deciding set
        "toy_stiff" => case(auto(1, plain(|y, dy| dy[0] = -5000.0 * y[0])), (0.0, 1.0), vec![1.0]),
        "toy_osc" => case(
            auto(
                2,
                plain(|y, dy| {
                    dy[0] = y[1];
                    dy[1] = -4.0 * y[0];
                }),
            ),
            (0.0, 30.0),
            vec![1.0, 0.0],
        ),
        "toy_ft" => case(
            counted(1, entries, timed(|t, y, dy| dy[0] = -5000.0 * (y[0] - t.sin()))),
            (0.0, 1.0),
            vec![0.0],
        ),
        "toy_fatal" => case(
            auto(
                2,
                guarded(
                    |y, dy| {
                        dy[0] = 0.0;
                        dy[1] = -2000.0 * y[1];
                    },
                    |_t, y| y[0] > 1.0,
                    "toy: y1 > 1",
                    fatal,
                ),
            ),
            (0.0, 1.0),
            vec![1.0, 1.0],
        ),
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

/// Whether [`build`] gives the problem a Jacobian callback (dense or banded): its entries are
/// `entries.jacobian`, charged as `counters.jacobian_builds`. The other problems build their
/// Jacobian by finite differences of the right-hand side (or take a JVP).
fn jacobian_callback(id: &str) -> bool {
    matches!(
        id,
        "P_blowup" | "P_mass" | "P_heat_dense" | "P_scalar_dense" | "P_heat_banded"
    )
}

// ----------------------------------------------------------------- classification

/// The facade's forward-difference shift of component `y_j` (`crates/vigilode/src/fd.rs`,
/// `increment`): `y_j + sqrt(eps) max(|y_j|, 1)`, or `y_j - sqrt(eps) max(|y_j|, 1)` where
/// the forward shift overflows.
fn fd_shift(yj: f64) -> f64 {
    let raw = f64::EPSILON.sqrt() * yj.abs().max(1.0);
    let forward = yj + raw;
    if forward.is_finite() { forward } else { yj - raw }
}

/// Classify the right-hand-side entries. A finite-difference column is an entry whose state
/// is, bit for bit, another entry's state at the same time with one component shifted by the
/// facade's increment ([`fd_shift`]). A finite-difference time derivative is an entry with the
/// state of an adjacent entry, bit for bit, at a time within twice `sqrt(eps) max(|t|, 1)` of
/// it (the driver calls `f(t + e, y)` and `f(t - e, y)` in a row). Every other entry is a
/// stage (the first-stage call `f(t, y)` of an attempt included). Deterministic: the kind of
/// an entry depends only on the entries at its own time and on its neighbours. Returns the
/// counts and the kind of every entry ([`STAGE`], [`COLUMN`], [`TIME_DERIVATIVE`]).
fn classify(n: usize, flat: &[f64], t0: f64, y0: &[f64]) -> (Value, Vec<u8>) {
    let width = n + 1;
    let count = flat.len() / width;
    let time = |i: usize| flat[i * width];
    let state = |i: usize| &flat[i * width + 1..(i + 1) * width];
    let same = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
    let mut by_time: HashMap<u64, Vec<usize>> = HashMap::new();
    for i in 0..count {
        by_time.entry(time(i).to_bits()).or_default().push(i);
    }
    let mut column = vec![false; count];
    for members in by_time.values() {
        for &e in members {
            let ye = state(e);
            'base: for &b in members {
                if b == e {
                    continue;
                }
                let yb = state(b);
                let mut differing = None;
                for j in 0..n {
                    if ye[j].to_bits() != yb[j].to_bits() {
                        if differing.is_some() {
                            continue 'base;
                        }
                        differing = Some(j);
                    }
                }
                if let Some(j) = differing
                    && ye[j].to_bits() == fd_shift(yb[j]).to_bits()
                {
                    column[e] = true;
                    break;
                }
            }
        }
    }
    let eps = f64::EPSILON.sqrt();
    let (mut stages, mut columns, mut time_derivatives) = (0u64, 0u64, 0u64);
    let mut kinds = vec![STAGE; count];
    for i in 0..count {
        if column[i] {
            columns += 1;
            kinds[i] = COLUMN;
            continue;
        }
        let (t, y) = (time(i), state(i));
        let derivative = [i.wrapping_sub(1), i + 1]
            .into_iter()
            .filter(|&k| k < count)
            .any(|k| {
                let tk = time(k);
                tk != t && same(state(k), y) && (tk - t).abs() <= 4.0 * eps * t.abs().max(1.0)
            });
        if derivative {
            time_derivatives += 1;
            kinds[i] = TIME_DERIVATIVE;
        } else {
            stages += 1;
        }
    }
    let at_t0 = (0..count).filter(|&i| time(i) == t0).count();
    let at_t0_other_state = (0..count)
        .filter(|&i| time(i) == t0 && !same(state(i), y0))
        .count();
    let first_is_t0_y0 = count > 0 && time(0) == t0 && same(state(0), y0);
    let counts = json!({
        "stage": stages,
        "fd_column": columns,
        "fd_time_derivative": time_derivatives,
        "at_t0": at_t0,
        "at_t0_not_y0": at_t0_other_state,
        "first_is_t0_y0": first_is_t0_y0,
    });
    (counts, kinds)
}

/// Kinds of a right-hand-side entry ([`classify`]).
const STAGE: u8 = 0;
/// See [`STAGE`].
const COLUMN: u8 = 1;
/// See [`STAGE`].
const TIME_DERIVATIVE: u8 = 2;

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
    for repeat in 1..=args.repeat {
        println!("{}", run_case(&args, repeat));
    }
}

fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

/// A fresh copy of the problem with its own entry counters (and log), for the replays of
/// [`e5_split`]; nothing of it is charged to the case's run.
#[cfg(feature = "pilots")]
fn fresh(problem: &str, log: bool) -> (Entries, Case) {
    let mut entries = Entries::default();
    if log {
        entries.log = Some(Arc::default());
    }
    let case = build(problem, &entries).expect("the problem was built for the case already");
    (entries, case)
}

#[cfg(feature = "pilots")]
fn taken(entries: &Entries) -> Vec<f64> {
    entries
        .log
        .as_ref()
        .map(|log| log.lock().unwrap().clone())
        .unwrap_or_default()
}

/// E5 of `PREDECLARED.json` by replay (arm N; the route must start with DOPRI5). The case's
/// right-hand-side entries are split, bit for bit and in order, into
///
/// * a prefix of the entries of `Dopri5::solve` with the same options (DOPRI5's stages);
/// * for each pilot, the entries of `Rodas5p::solve` with `h0 = H` and `max_steps = 1` from
///   the point `Dopri5::solve` has accepted after the pilot's DOPRI5 attempts (`max_steps` =
///   the record's `dopri5_attempts`: its final time and state are the last accepted point);
/// * after a handover, the entries of `Rodas5p::solve` with the attempts left, from the
///   replayed accepted pilot's end point with `h0 = H`, or, after a numerical DOPRI5 stop, from
///   the last accepted point of `Dopri5::solve` with the user's `h0`; nothing of DOPRI5 may
///   follow an accepted or fatal pilot.
///
/// The replays run on fresh copies of the problem. The result says whether the split exists,
/// where it fails, and the entries of each part by kind and at `t0`.
#[cfg(feature = "pilots")]
fn e5_split(args: &Args, options: &Options, n_log: &[f64], solution: &VigilodeSolution) -> Value {
    use vigilode::PilotVerdict;
    let starts_with_dopri5 = solution
        .legs
        .first()
        .is_some_and(|leg| leg.tier == SelectedTier::Tier1Explicit);
    if !starts_with_dopri5 {
        return json!({ "applicable": false });
    }
    let (d_entries, d_case) = fresh(&args.problem, true);
    let n = d_case.y0.len();
    let width = n + 1;
    let (t0, t1) = d_case.span;
    let dopri5 = Dopri5::solve(&d_case.problem, d_case.span, &d_case.y0, options);
    let d_log = taken(&d_entries);
    let d_final = match &dopri5 {
        Ok(s) => (s.final_time, s.final_state.clone()),
        Err(error) => {
            return json!({
                "applicable": true, "split": false,
                "mismatch": format!("Dopri5::solve refused: {error}"),
            });
        }
    };

    // the pilots, replayed
    let mut blocks: Vec<Vec<f64>> = Vec::new();
    let mut pilot_rows = Vec::new();
    let mut accepted_end: Option<(f64, Vec<f64>)> = None;
    for pilot in &solution.pilots {
        let (_, pre_case) = fresh(&args.problem, false);
        let mut pre_options = options.clone();
        pre_options.max_steps = pilot.dopri5_attempts;
        let pre = Dopri5::solve(&pre_case.problem, pre_case.span, &pre_case.y0, &pre_options);
        let Ok(pre) = pre else {
            pilot_rows.push(json!({ "index": pilot.index, "replay": "Dopri5::solve refused" }));
            blocks.push(Vec::new());
            continue;
        };
        let (t_p, y_p) = (pre.final_time, pre.final_state.clone());
        let (p_entries, p_case) = fresh(&args.problem, true);
        let mut p_options = options.clone();
        p_options.h0 = pilot.h;
        p_options.max_steps = 1;
        let replay = Rodas5p::solve(&p_case.problem, (t_p, t1), &y_p, &p_options);
        let block = taken(&p_entries);
        let end = replay
            .as_ref()
            .ok()
            .and_then(|s| Some((*s.t.last()?, s.final_state()?.to_vec())));
        if pilot.verdict == PilotVerdict::Accepted {
            accepted_end = end.clone();
        }
        pilot_rows.push(json!({
            "index": pilot.index,
            "verdict": pilot.verdict,
            "record_t_bits": bits(pilot.t),
            "dopri5_solve_t_bits": bits(t_p),
            "dopri5_solve_attempts": pre.stats.attempts,
            "start_equals_record": t_p.to_bits() == pilot.t.to_bits(),
            "start_after_t0": t_p > t0,
            "replay_status": replay
                .as_ref()
                .map(|s| json!(s.status))
                .unwrap_or_else(|e| json!(e.to_string())),
            "replay_end_t_bits": end.as_ref().map(|(t, _)| bits(*t)),
            "block_entries": block.len() / width,
        }));
        blocks.push(block);
    }

    // the RODAS5P run after a handover, replayed
    let last_verdict = solution.pilots.last().map(|p| p.verdict);
    let accepted_last = last_verdict == Some(PilotVerdict::Accepted);
    let fatal_last = last_verdict == Some(PilotVerdict::Fatal);
    let used = solution.legs[0].attempts
        + solution
            .pilots
            .iter()
            .map(|p| p.pilot_attempts)
            .sum::<usize>();
    let remaining = options.max_steps.saturating_sub(used);
    let mut tail_row = Value::Null;
    let mut tail: Option<Vec<f64>> = None;
    if solution.legs.len() >= 2 && remaining > 0 {
        let (kind, start, h0) = if accepted_last {
            ("accepted-pilot", accepted_end.clone(), solution.pilots.last().map(|p| p.h))
        } else {
            ("dopri5-stop", Some(d_final.clone()), None)
        };
        if let Some((ts, ys)) = start {
            let (tl_entries, tl_case) = fresh(&args.problem, true);
            let mut tl_options = options.clone();
            tl_options.max_steps = remaining;
            if let Some(h) = h0 {
                tl_options.h0 = h;
            }
            let replay = Rodas5p::solve(&tl_case.problem, (ts, t1), &ys, &tl_options);
            let entries = taken(&tl_entries);
            tail_row = json!({
                "kind": kind,
                "start_t_bits": bits(ts),
                "remaining": remaining,
                "h0_bits": bits(tl_options.h0),
                "replay_status": replay
                    .as_ref()
                    .map(|s| json!(s.status))
                    .unwrap_or_else(|e| json!(e.to_string())),
                "entries": entries.len() / width,
            });
            tail = Some(entries);
        }
    }

    // the split: 0 = DOPRI5, 1 = a pilot, 2 = the RODAS5P run after the handover
    let count = n_log.len() / width;
    let d_count = d_log.len() / width;
    let same = |a: &[f64], i: usize, b: &[f64], j: usize| {
        (0..width).all(|k| a[i * width + k].to_bits() == b[j * width + k].to_bits())
    };
    let block_at = |pos: usize, block: &[f64]| {
        let m = block.len() / width;
        pos + m <= count && (0..m).all(|k| same(n_log, pos + k, block, k))
    };
    let mut part = vec![0u8; count];
    let (mut pos, mut d) = (0usize, 0usize);
    let mut mismatch: Option<String> = None;
    'split: {
        for (q, block) in blocks.iter().enumerate() {
            if block.is_empty() {
                mismatch = Some(format!("pilot {q}: no replay"));
                break 'split;
            }
            loop {
                if block_at(pos, block) {
                    let m = block.len() / width;
                    part[pos..pos + m].fill(1);
                    pos += m;
                    break;
                }
                if pos < count && d < d_count && same(n_log, pos, &d_log, d) {
                    pos += 1;
                    d += 1;
                    continue;
                }
                mismatch = Some(format!(
                    "entry {pos}: neither entry {d} of Dopri5::solve nor the start of pilot {q}"
                ));
                break 'split;
            }
        }
        if let Some(tail) = &tail {
            loop {
                if block_at(pos, tail) {
                    let m = tail.len() / width;
                    part[pos..pos + m].fill(2);
                    pos += m;
                    break;
                }
                // after an accepted pilot nothing of DOPRI5 may come before the RODAS5P run
                if !accepted_last && pos < count && d < d_count && same(n_log, pos, &d_log, d) {
                    pos += 1;
                    d += 1;
                    continue;
                }
                mismatch = Some(format!("entry {pos}: not the start of the RODAS5P run"));
                break 'split;
            }
        } else if !accepted_last && !fatal_last {
            while pos < count && d < d_count && same(n_log, pos, &d_log, d) {
                pos += 1;
                d += 1;
            }
        }
        if pos != count {
            mismatch = Some(format!("entry {pos} of {count} is left after the split"));
        }
    }
    let (_, kinds) = classify(n, n_log, t0, &d_case.y0);
    let part_row = |which: u8| {
        let members: Vec<usize> = (0..count).filter(|&i| part[i] == which).collect();
        let kind = |k: u8| members.iter().filter(|&&i| kinds[i] == k).count();
        json!({
            "entries": members.len(),
            "stage": kind(STAGE),
            "fd_column": kind(COLUMN),
            "fd_time_derivative": kind(TIME_DERIVATIVE),
            "at_t0": members.iter().filter(|&&i| n_log[i * width] == t0).count(),
        })
    };
    json!({
        "applicable": true,
        "split": mismatch.is_none(),
        "mismatch": mismatch,
        "entries": count,
        "dopri5_solve_entries": d_count,
        "dopri5_entries": d,
        "dopri5_is_all_of_dopri5_solve": d == d_count,
        "first_entry_is_dopri5_solve_first": count > 0 && d_count > 0 && same(n_log, 0, &d_log, 0),
        "pilots": pilot_rows,
        "rodas5p_run": tail_row,
        "parts": { "dopri5": part_row(0), "pilots": part_row(1), "rodas5p_run": part_row(2) },
    })
}

/// One run of the case: its JSON line.
fn run_case(args: &Args, repeat: usize) -> Value {
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
    #[allow(unused_mut)]
    let mut e5 = Value::Null;
    let run: Value = match args.arm.as_str() {
        "N" | "T" => {
            let opts = VigilodeOptions::new(options.clone())
                .strategy(SolverStrategy::Auto)
                .regime(args.regime);
            match VigilODE::solve(problem, span, y0, opts) {
                Ok(solution) => {
                    #[cfg(feature = "pilots")]
                    {
                        if args.arm == "N"
                            && let Some(log) = &entries.log
                        {
                            let log = log.lock().unwrap().clone();
                            e5 = e5_split(args, &options, &log, &solution);
                        }
                    }
                    unified_fields(n, &solution)
                }
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
        .map(|log| classify(n, &log.lock().unwrap(), span.0, y0).0)
        .unwrap_or(Value::Null);
    json!({
        "schema": "vigilode.v27.runner.v1",
        "build": BUILD,
        "arm": args.arm,
        "problem": args.problem,
        "repeat": repeat,
        "pid": std::process::id(),
        "n": n,
        "span": [span.0, span.1],
        "span_bits": [bits(span.0), bits(span.1)],
        "y0_bits": state_bits(y0),
        "jacobian_callback": jacobian_callback(&args.problem),
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
        "e5": e5,
    })
}
