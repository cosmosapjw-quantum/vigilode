//! V30 phase 1 (node V30_ROUTER_BREAKEVEN_V27B, research loop L1): diagnosis of E7 of V27 on
//! P_scalar_fd, `y' = -1e6 (y - cos t)`, `y(0) = 1`, span `[0, 1]`, non-autonomous.
//!
//! ```text
//! v30-e7-diag --tol <A|L|S>
//! v30-e7-diag --tau
//! ```
//!
//! One JSON line per experiment on stdout; the values are raw (bits of times and states, counters,
//! the accepted points of every RODAS5P run). Errors against the exact solution are computed later
//! by `analyze_e7.py` with mpmath; nothing here decides anything. No wall time is measured.
//!
//! Experiments (same `Options` as V27 arm by arm: defaults, `rtol`/`atol` of the tolerance pair):
//!
//! * `D`: `Dopri5::solve`; `R`: `Rodas5p::solve`; `N`: `VigilODE::solve` (Auto, regime Unknown:
//!   the forward break-even pilots of this worktree).
//! * `NR`: the route of `N` replayed from its pilot record by the public drivers: `Dopri5::solve`
//!   with `max_steps` = the record's DOPRI5 attempts gives `(t_D, y_D)`; the pilot is
//!   `Rodas5p::solve` from `(t_D, y_D)` with `h0 = H`, `max_steps = 1`; the leg is `Rodas5p::solve`
//!   from the pilot's end with `h0 = H` and the attempts left. Its final bits must equal `N`'s.
//! * Isolating variants of `NR`, one change each: `C_cont` (one continuing RODAS5P run from
//!   `(t_D, y_D)` with `h0 = H`: no restart after the pilot, the controller's own next step);
//!   `C_exact_state` (`y_D` replaced by the exact solution at `t_D`, rounded to f64);
//!   `C_rest_h0_user` (the leg after the pilot starts with the user's `h0 = 1e-3`, not `H`);
//!   `R_h0_H` (no handover at all: `Rodas5p::solve` from `t0` with `h0 = H`); `R_from_tD`
//!   (`Rodas5p::solve` from `(t_D, y_D)` with the user's `h0`).
//! * Problem variants (the right-hand side is the same function in all of them): `fd` (P_scalar_fd
//!   of V27: finite-difference Jacobian and the drivers' central-difference time derivative);
//!   `ft` (the exact time derivative `-1e6 sin t`); `ft_fdclone` (the drivers' central difference,
//!   supplied as the time-derivative callback: a control that must give the bits of `fd`);
//!   `ft_pert+` / `ft_pert-` (the exact time derivative times `1 +- 1e-8`: the response of the end
//!   state to an error of `f_t`); `jac` (the exact Jacobian `-1e6`, finite-difference `f_t`);
//!   `jac_ft` (both exact).
//! * `scan`: `Rodas5p::solve` from `t0` with `h0 = 10^(-6 + j/8)`, `j = 0..=40`, for `fd`, `ft`,
//!   `jac_ft`: how much RODAS5P's own end-point error moves with the step sequence.
//!
//! The accepted points of a RODAS5P run are read by prefix replays: the same call with
//! `max_steps = 1, 2, ...` returns the last accepted point (deterministic drivers; checked: the
//! last replay equals the run). For each point the line also carries the drivers' central-difference
//! time derivative at that point (`ft_fd`), computed with the drivers' formula.

use serde_json::{Value, json};
use vigilode::{
    Atol, Dopri5, Jacobian, Options, PilotVerdict, Problem, ProblemRegime, Rodas5p,
    SolverStrategy, VigilODE, VigilodeOptions,
};

const K: f64 = 1.0e6;
const T0: f64 = 0.0;
const T1: f64 = 1.0;
const Y0: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Variant {
    Fd,
    Ft,
    FtFdClone,
    FtPert(f64),
    Jac,
    JacFt,
}

impl Variant {
    fn name(self) -> String {
        match self {
            Variant::Fd => "fd".into(),
            Variant::Ft => "ft".into(),
            Variant::FtFdClone => "ft_fdclone".into(),
            Variant::FtPert(eta) if eta > 0.0 => "ft_pert+".into(),
            Variant::FtPert(_) => "ft_pert-".into(),
            Variant::Jac => "jac".into(),
            Variant::JacFt => "jac_ft".into(),
        }
    }

    /// Whether `VigilODE::solve` takes the break-even route for it (no supplied Jacobian).
    fn break_even_route(self) -> bool {
        !matches!(self, Variant::Jac | Variant::JacFt)
    }
}

/// The right-hand side, with the arithmetic of the V27 runner's `scalar_fd`.
fn f(t: f64, y: f64) -> f64 {
    -1.0e6 * (y - t.cos())
}

/// The drivers' central difference in `t` (`OdeProblem::eval_partial_t_into_with_scratch`).
fn fd_time_derivative(t: f64, y: f64) -> f64 {
    let eps = f64::EPSILON.sqrt() * t.abs().max(1.0);
    let plus = f(t + eps, y);
    let minus = f(t - eps, y);
    (plus - minus) / (2.0 * eps)
}

/// The exact solution through `(0, 1)`, in f64 (only used as a start state).
fn exact(t: f64) -> f64 {
    let d = 1.0 + K * K;
    (K * K * t.cos() + K * t.sin()) / d + (-K * t).exp() / d
}

fn problem(variant: Variant) -> Problem {
    let base = Problem::new(1, |t: f64, y: &[f64], dy: &mut [f64]| {
        dy[0] = f(t, y[0]);
        Ok(())
    });
    let exact_jacobian = || {
        Jacobian::dense(|_t: f64, _y: &[f64], jac: &mut [f64]| {
            jac[0] = -1.0e6;
            Ok(())
        })
    };
    let exact_ft = |t: f64, _y: &[f64], out: &mut [f64]| {
        out[0] = -1.0e6 * t.sin();
        Ok(())
    };
    match variant {
        Variant::Fd => base,
        Variant::Ft => base.time_derivative(exact_ft),
        Variant::FtFdClone => base.time_derivative(|t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = fd_time_derivative(t, y[0]);
            Ok(())
        }),
        Variant::FtPert(eta) => {
            base.time_derivative(move |t: f64, _y: &[f64], out: &mut [f64]| {
                out[0] = -1.0e6 * t.sin() * (1.0 + eta);
                Ok(())
            })
        }
        Variant::Jac => base.jacobian(exact_jacobian()),
        Variant::JacFt => base.jacobian(exact_jacobian()).time_derivative(exact_ft),
    }
}

fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn options(rtol: f64, atol: f64) -> Options {
    let mut options = Options::default();
    options.rtol = rtol;
    options.atol = Atol::Scalar(atol);
    options
}

/// A RODAS5P run from `(t, y)` to `T1`.
struct RodasRun {
    value: Value,
    success: bool,
    final_t: f64,
    final_y: f64,
}

fn point(t: f64, y: f64) -> Value {
    json!({ "t": bits(t), "y": bits(y), "ft_fd": bits(fd_time_derivative(t, y)) })
}

fn rodas(variant: Variant, start: (f64, f64), h0: f64, max_steps: usize, base: &Options) -> RodasRun {
    let mut o = base.clone();
    o.h0 = h0;
    o.max_steps = max_steps;
    let run = Rodas5p::solve(&problem(variant), (start.0, T1), &[start.1], &o);
    let s = match run {
        Ok(s) => s,
        Err(error) => {
            return RodasRun {
                value: json!({ "error": error.to_string() }),
                success: false,
                final_t: f64::NAN,
                final_y: f64::NAN,
            };
        }
    };
    let final_t = s.t.last().copied().unwrap_or(start.0);
    let final_y = s.final_state().map_or(f64::NAN, |v| v[0]);
    // the accepted points, by prefix replays
    let mut points = vec![(start.0, start.1)];
    let mut replay_matches = true;
    for k in 1..=s.stats.attempts {
        let mut ok = o.clone();
        ok.max_steps = k;
        match Rodas5p::solve(&problem(variant), (start.0, T1), &[start.1], &ok) {
            Ok(p) => {
                let tk = p.t.last().copied().unwrap_or(start.0);
                let yk = p.final_state().map_or(f64::NAN, |v| v[0]);
                let last = points[points.len() - 1];
                if tk.to_bits() != last.0.to_bits() {
                    points.push((tk, yk));
                } else if yk.to_bits() != last.1.to_bits() {
                    replay_matches = false;
                }
                if k == s.stats.attempts
                    && (tk.to_bits() != final_t.to_bits() || yk.to_bits() != final_y.to_bits())
                {
                    replay_matches = false;
                }
            }
            Err(_) => replay_matches = false,
        }
    }
    let value = json!({
        "start_t": bits(start.0),
        "start_y": bits(start.1),
        "h0": bits(h0),
        "max_steps": max_steps,
        "success": s.is_success(),
        "status": s.status,
        "final_t": bits(final_t),
        "final_y": bits(final_y),
        "attempts": s.stats.attempts,
        "accepted": s.stats.accepted_steps,
        "rejected": s.stats.rejected_steps,
        "counters": s.stats.counters,
        "points": points.iter().map(|&(t, y)| point(t, y)).collect::<Vec<_>>(),
        "replay_matches": replay_matches,
    });
    RodasRun { value, success: s.is_success(), final_t, final_y }
}

/// The handover of the route, replayed: the pilot (one attempt, `h0 = h`) and the leg after it.
fn composition(
    variant: Variant,
    base: &Options,
    start: (f64, f64),
    h: f64,
    rest_h0: f64,
    rest_budget: usize,
) -> Value {
    let pilot = rodas(variant, start, h, 1, base);
    if !(pilot.final_t > start.0) {
        return json!({ "pilot": pilot.value, "pilot_accepted": false });
    }
    let rest = rodas(variant, (pilot.final_t, pilot.final_y), rest_h0, rest_budget, base);
    json!({
        "pilot": pilot.value,
        "pilot_accepted": true,
        "rest": rest.value,
        "success": rest.success,
        "final_t": bits(rest.final_t),
        "final_y": bits(rest.final_y),
    })
}

/// The pilot record of an accepted pilot: DOPRI5 attempts before it, `t_D`, `H`, lost pilots'
/// attempts.
struct Handover {
    dopri5_attempts: usize,
    t: f64,
    h: f64,
    spare_attempts: usize,
}

fn route(variant: Variant, base: &Options) -> (Value, Option<Handover>) {
    let opts = VigilodeOptions::new(base.clone())
        .strategy(SolverStrategy::Auto)
        .regime(ProblemRegime::Unknown);
    let solution = match VigilODE::solve(&problem(variant), (T0, T1), &[Y0], opts) {
        Ok(solution) => solution,
        Err(error) => return (json!({ "error": error.to_string() }), None),
    };
    let legs: Vec<Value> = solution
        .legs
        .iter()
        .map(|leg| {
            json!({
                "tier": leg.tier, "t_start": bits(leg.t_start), "t_end": bits(leg.t_end),
                "attempts": leg.attempts, "accepted": leg.accepted_steps,
                "rejected": leg.rejected_steps, "success": leg.success,
            })
        })
        .collect();
    let pilots: Vec<Value> = solution
        .pilots
        .iter()
        .map(|p| {
            json!({
                "index": p.index, "dopri5_attempts": p.dopri5_attempts, "t": bits(p.t),
                "h": bits(p.h), "window_attempts": p.window_attempts,
                "pilot_attempts": p.pilot_attempts, "verdict": p.verdict,
            })
        })
        .collect();
    let handover = solution.pilots.last().and_then(|last| {
        (last.verdict == PilotVerdict::Accepted).then(|| Handover {
            dopri5_attempts: last.dopri5_attempts,
            t: last.t,
            h: last.h,
            spare_attempts: solution
                .pilots
                .iter()
                .filter(|p| p.verdict != PilotVerdict::Accepted)
                .map(|p| p.pilot_attempts)
                .sum(),
        })
    });
    let value = json!({
        "success": solution.success(),
        "status": solution.status,
        "final_t": bits(solution.final_time),
        "final_y": bits(solution.final_state.first().copied().unwrap_or(f64::NAN)),
        "attempts": solution.stats.attempts,
        "accepted": solution.stats.accepted_steps,
        "rejected": solution.stats.rejected_steps,
        "counters": solution.stats.counters,
        "legs": legs,
        "pilots": pilots,
    });
    (value, handover)
}

/// `--tau`: one RODAS5P step of length `h` from the exact solution at `t` (the step's local
/// truncation error), and the same step from the exact solution plus `1e-8` (the step's
/// amplification of an initial deviation, `R(-1e6 h)`), on a grid of `(t, h)`; each at the
/// tolerance pair A (whether the step is accepted there) and at `rtol = atol = 0.5` (every step
/// accepted: the one-step map). `max_steps = 1`; the step's end time is read from the run.
fn tau() {
    let starts = [0.0, 0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    for variant in [Variant::Fd, Variant::Ft, Variant::JacFt] {
        for (tol, rtol, atol) in [("A", 1.0e-6, 1.0e-9), ("loose", 0.5, 0.5)] {
            let base = options(rtol, atol);
            for &t in &starts {
                for j in 0..=20i32 {
                    let h = 10f64.powf(-5.0 + f64::from(j) / 4.0);
                    if t + h >= T1 {
                        continue;
                    }
                    let y = exact(t);
                    let one = rodas(variant, (t, y), h, 1, &base);
                    let shifted = rodas(variant, (t, y + 1.0e-8), h, 1, &base);
                    println!(
                        "{}",
                        json!({
                            "schema": "vigilode.v30.e7diag.tau.v1",
                            "variant": variant.name(), "tol": tol, "rtol": rtol, "atol": atol,
                            "t": bits(t), "h": bits(h), "y": bits(y), "y_shift": bits(1.0e-8),
                            "step": one.value, "step_shifted": shifted.value,
                        })
                    );
                }
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (flag, tol) = (args.next(), args.next());
    if flag.as_deref() == Some("--tau") {
        tau();
        return;
    }
    let (name, rtol, atol) = match (flag.as_deref(), tol.as_deref()) {
        (Some("--tol"), Some("A")) => ("A", 1.0e-6, 1.0e-9),
        (Some("--tol"), Some("L")) => ("L", 1.0e-3, 1.0e-6),
        (Some("--tol"), Some("S")) => ("S", 1.0e-8, 1.0e-11),
        _ => {
            eprintln!("usage: v30-e7-diag --tol <A|L|S> | --tau");
            std::process::exit(2);
        }
    };
    let base = options(rtol, atol);
    let total = base.max_steps;
    let emit = |variant: &str, experiment: &str, value: Value| {
        println!(
            "{}",
            json!({
                "schema": "vigilode.v30.e7diag.v1",
                "tol": name, "rtol": rtol, "atol": atol,
                "variant": variant, "exp": experiment,
                "run": value,
            })
        );
    };

    // D: Dopri5::solve (the problem variants do not change what DOPRI5 computes)
    match Dopri5::solve(&problem(Variant::Fd), (T0, T1), &[Y0], &base) {
        Ok(s) => emit(
            "fd",
            "D",
            json!({
                "success": s.success, "final_t": bits(s.final_time),
                "final_y": bits(s.final_state.first().copied().unwrap_or(f64::NAN)),
                "attempts": s.stats.attempts, "counters": s.stats.counters,
                "stop": format!("{:?}", s.stop_reason),
            }),
        ),
        Err(error) => emit("fd", "D", json!({ "error": error.to_string() })),
    }

    // the route's handover point, from the fd run (DOPRI5 does not depend on the variant)
    let (n_fd, handover) = route(Variant::Fd, &base);
    emit("fd", "N", n_fd);
    let Some(handover) = handover else {
        emit("fd", "NO_HANDOVER", json!({}));
        return;
    };
    let mut pre = base.clone();
    pre.max_steps = handover.dopri5_attempts;
    let (t_d, y_d) = match Dopri5::solve(&problem(Variant::Fd), (T0, T1), &[Y0], &pre) {
        Ok(s) => (s.final_time, s.final_state.first().copied().unwrap_or(f64::NAN)),
        Err(error) => {
            emit("fd", "DOPRI5_PREFIX", json!({ "error": error.to_string() }));
            return;
        }
    };
    emit(
        "fd",
        "DOPRI5_PREFIX",
        json!({
            "max_steps": handover.dopri5_attempts, "t_d": bits(t_d), "y_d": bits(y_d),
            "record_t": bits(handover.t), "record_h": bits(handover.h),
            "t_equals_record": t_d.to_bits() == handover.t.to_bits(),
            "y_exact_t_d": bits(exact(t_d)),
        }),
    );
    let h = handover.h;
    let leg_budget = total - handover.dopri5_attempts - handover.spare_attempts;
    let rest_budget = leg_budget - 1;

    let variants = [
        Variant::Fd,
        Variant::Ft,
        Variant::FtFdClone,
        Variant::FtPert(1.0e-8),
        Variant::FtPert(-1.0e-8),
        Variant::Jac,
        Variant::JacFt,
    ];
    for variant in variants {
        let v = variant.name();
        emit(&v, "R", rodas(variant, (T0, Y0), base.h0, total, &base).value);
        if variant.break_even_route() && variant != Variant::Fd {
            emit(&v, "N", route(variant, &base).0);
        }
        emit(&v, "NR", composition(variant, &base, (t_d, y_d), h, h, rest_budget));
        emit(&v, "C_cont", rodas(variant, (t_d, y_d), h, leg_budget, &base).value);
        emit(
            &v,
            "C_exact_state",
            composition(variant, &base, (t_d, exact(t_d)), h, h, rest_budget),
        );
        emit(
            &v,
            "C_rest_h0_user",
            composition(variant, &base, (t_d, y_d), h, base.h0, rest_budget),
        );
        emit(&v, "R_h0_H", rodas(variant, (T0, Y0), h, total, &base).value);
        emit(&v, "R_from_tD", rodas(variant, (t_d, y_d), base.h0, leg_budget, &base).value);
    }

    for variant in [Variant::Fd, Variant::Ft, Variant::JacFt] {
        for j in 0..=40i32 {
            let h0 = 10f64.powf(-6.0 + f64::from(j) / 8.0);
            let mut value = rodas(variant, (T0, Y0), h0, total, &base).value;
            value["scan_j"] = json!(j);
            emit(&variant.name(), "scan", value);
        }
    }
}
