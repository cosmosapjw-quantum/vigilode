//! Identity contracts of the small-n fast driver (research node
//! `research/thread_transfer_smalln_cost_20261002`, thread-transfer DAG node
//! P1-SMALLN-COST): with the same operations as v2, it reproduces v2's
//! results (as values) on van der Pol and Robertson, including rejected
//! attempts and the reuse of `J`, `f` after them.

use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, SmallProblem, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_small_observed, robertson_problem, stiff_van_der_pol_problem,
};

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

fn config(rtol: f64, scale: f64, span: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * scale,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: span,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn the_small_driver_reproduces_v2() {
    let (vdp, vdp0) = stiff_van_der_pol_problem(1000.0).unwrap();
    let (rob, rob0) = robertson_problem().unwrap();
    for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
        let output = OutputSchedule::new(vec![0.0, 0.3, 2000.0]).unwrap();
        let adaptive = config(rtol, 1.0, 2000.0);
        let v2 = integrate_rodas5p_fast_observed(&vdp, (0.0, 2000.0), &vdp0, &adaptive, &output)
            .unwrap();
        let small = integrate_rodas5p_fast_small_observed(
            &VanDerPol(1000.0),
            (0.0, 2000.0),
            &[vdp0[0], vdp0[1]],
            &adaptive,
            &output,
        )
        .unwrap();
        assert!(v2.observed.success && small.observed.success);
        assert!(
            v2.rejected_steps > 0,
            "the comparison must include rejections"
        );
        assert_eq!(v2.observed.t, small.observed.t);
        assert_eq!(v2.observed.y, small.observed.y);
        assert_eq!(
            (v2.accepted_steps, v2.rejected_steps, v2.jacobian_reuses),
            (
                small.accepted_steps,
                small.rejected_steps,
                small.jacobian_reuses
            )
        );
        assert_eq!(v2.observed.counters, small.observed.counters);

        let output = OutputSchedule::new(vec![0.0, 40.0]).unwrap();
        let adaptive = config(rtol, 1.0e-4, 40.0);
        let v2 =
            integrate_rodas5p_fast_observed(&rob, (0.0, 40.0), &rob0, &adaptive, &output).unwrap();
        let small = integrate_rodas5p_fast_small_observed(
            &Robertson,
            (0.0, 40.0),
            &[rob0[0], rob0[1], rob0[2]],
            &adaptive,
            &output,
        )
        .unwrap();
        assert_eq!(v2.observed.y, small.observed.y);
        assert_eq!(v2.observed.counters, small.observed.counters);
    }
}

#[test]
fn invalid_input_is_refused() {
    let output = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let adaptive = config(1.0e-6, 1.0, 1.0);
    assert!(
        integrate_rodas5p_fast_small_observed(
            &VanDerPol(10.0),
            (1.0, 0.0),
            &[2.0, 0.0],
            &adaptive,
            &output
        )
        .is_err()
    );
    assert!(
        integrate_rodas5p_fast_small_observed(
            &VanDerPol(10.0),
            (0.0, 1.0),
            &[f64::NAN, 0.0],
            &adaptive,
            &output
        )
        .is_err()
    );
}
