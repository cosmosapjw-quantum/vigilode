//! Adapter equivalence: every arm integrates the shared linear problem with
//! a known exact solution and returns the full common grid through its own
//! output interface; the VigilODE arms reproduce the facade bit for bit.

use vigilode::{Atol, CallbackError, Jacobian, Options, Problem, Rodas5p, Status};
use vigilode_bench::arms::{self, Arm, RunStatus};
use vigilode_bench::metrics::max_wrms;
use vigilode_bench::model::{self, Kind, Model};

fn grid(t_end: f64, points: usize) -> Vec<f64> {
    let mut g: Vec<f64> = (0..points)
        .map(|k| t_end * k as f64 / (points - 1) as f64)
        .collect();
    *g.last_mut().unwrap() = t_end;
    g
}

#[test]
fn every_arm_matches_the_exact_solution_on_the_common_grid() {
    let m = Model::new(Kind::LinearExact, 2).unwrap();
    let g = grid(2.0, 101);
    let exact: Vec<Vec<f64>> = g.iter().map(|&t| m.exact(t).unwrap()).collect();
    let (rtol, atol) = (1e-8, 1e-10);
    for arm in Arm::ALL {
        model::reset_counts();
        let out = arms::run(arm, &m, &g, rtol, atol);
        let calls = model::counts();
        assert_eq!(out.status, RunStatus::Success, "{arm:?}: {:?}", out.status);
        assert_eq!(out.states.len(), g.len(), "{arm:?}");
        assert_eq!(out.states[0], m.y0(), "{arm:?} first grid point is y0");
        let err = max_wrms(&out.states, &exact, &[atol; 2], rtol).unwrap();
        let abs = out
            .states
            .iter()
            .zip(&exact)
            .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()))
            .fold(0.0f64, f64::max);
        assert!(
            abs < 1e-6 && err < 1e3,
            "{arm:?}: abs {abs:e}, wrms {err:e}"
        );
        assert!(calls.rhs > 0, "{arm:?} used the shared RHS");
        assert!(
            calls.jac + calls.jvp > 0,
            "{arm:?} used the shared Jacobian"
        );
        assert!(out.work.accepted_steps.unwrap_or(1) > 0, "{arm:?}");
    }
}

fn facade_problem(m: Model, banded: bool) -> Problem {
    let n = m.n;
    let p = Problem::new(n, move |t, y, dy| {
        m.rhs(t, y, dy);
        Ok::<(), CallbackError>(())
    });
    let p = if banded {
        let (l, u) = m.band();
        p.jacobian(Jacobian::banded(l, u, move |t, y, out| {
            m.jac_band_row_major(t, y, l, u, out);
            Ok(())
        }))
    } else {
        p.jacobian(Jacobian::dense(move |t, y, jac| {
            m.jac_dense_row_major(t, y, jac);
            Ok(())
        }))
    };
    p.autonomous()
}

#[test]
fn vigilode_arms_reproduce_the_facade_endpoint_bit_for_bit() {
    for (kind, n, t_end) in [
        (Kind::LinearExact, 2, 2.0),
        (Kind::Robertson, 3, 40.0),
        (Kind::Brusselator1d, 20, 10.0),
    ] {
        let m = Model::new(kind, n).unwrap();
        for (arm, banded) in [(Arm::Vigilode, false), (Arm::VigilodeBanded, true)] {
            if banded && kind == Kind::Robertson {
                continue;
            }
            let rtol = 1e-6;
            let atol = 1e-9;
            let opts = Options {
                rtol,
                atol: Atol::Scalar(atol),
                ..Default::default()
            };
            let sol = Rodas5p::solve(&facade_problem(m, banded), (0.0, t_end), &m.y0(), &opts)
                .expect("facade solves");
            assert_eq!(sol.status, Status::Success);
            let out = arms::run(arm, &m, &[0.0, t_end], rtol, atol);
            assert_eq!(out.status, RunStatus::Success);
            let a: Vec<u64> = out.states[1].iter().map(|x| x.to_bits()).collect();
            let b: Vec<u64> = sol
                .final_state()
                .unwrap()
                .iter()
                .map(|x| x.to_bits())
                .collect();
            assert_eq!(a, b, "{kind:?} {arm:?} endpoint differs from the facade");
            assert_eq!(
                out.work.accepted_steps,
                Some(sol.stats.accepted_steps as u64),
                "{kind:?} {arm:?}"
            );
            assert_eq!(
                out.work.rejected_steps,
                Some(sol.stats.rejected_steps as u64)
            );
        }
    }
}

#[test]
fn a_failed_arm_keeps_its_status_prefix_and_work() {
    // Van der Pol mu = 1000 over a long span with a tiny attempt budget.
    let m = Model::new(Kind::VanDerPolMu1000, 2).unwrap();
    let g = grid(2000.0, 101);
    let out = arms::run_with(Arm::Vigilode, &m, &g, 1e-6, 1e-8, Some(50));
    assert!(
        matches!(out.status, RunStatus::SolverFailure { .. }),
        "{:?}",
        out.status
    );
    assert!(!out.states.is_empty() && out.states.len() < g.len());
    assert_eq!(out.work.step_attempts, Some(50));
    let cv = arms::run_with(Arm::CvodeBdfDense, &m, &g, 1e-6, 1e-8, Some(50));
    assert!(matches!(cv.status, RunStatus::SolverFailure { .. }));
    assert_eq!(cv.work.accepted_steps, Some(50));
}
