//! Source-attested one-step reference contract, C9 hostile findings H1/H2.
//! Exact source identity is constructed inside the private callback bundle.
//! This is NOT a production derivative-free RODAS8 or full IVP integration.
use rodas5p_core::{N01MassActionOwner, WorkCounters, rodas5p_coefficients};
use rodas5p_integrators::n01_frozen_source_bundle::N01FrozenSourceProblem;

const Y0: [f64; 3] = [0.75, 0.125, 0.125];

fn make(rates: [f64; 3]) -> N01FrozenSourceProblem {
    N01FrozenSourceProblem::new(
        N01MassActionOwner::new(rates, Y0, [1.0; 3], 0.0, [1.0; 3], [1.0; 3], 0.0).unwrap(),
    )
    .unwrap()
}

#[test]
fn frozen_native_rhs_stage_probes_keep_model_generation_and_owner() {
    let bundle = make([0.5, 10.0, 200.0]);
    let step = bundle.begin_step(0.0, Y0, 0.0001).unwrap();
    let (rates, frozen_carrier) = step.represented_source().unwrap();
    assert_eq!(rates, [0.5, 10.0, 200.0]);
    assert_eq!(frozen_carrier, Y0);
    let mut work = WorkCounters::default();
    let mut at_start = [f64::NAN; 3];
    let mut at_half = [f64::NAN; 3];
    step.probe_rhs(0.0, Y0, &mut at_start, &mut work).unwrap();
    step.probe_rhs(0.00005, Y0, &mut at_half, &mut work)
        .unwrap();
    assert_eq!(at_start, [-7.0 / 32.0, -93.0 / 32.0, 25.0 / 8.0]);
    assert_eq!(at_start, at_half);
    assert_eq!(bundle.model_epoch(), 0);
    assert_eq!(work.rhs_evaluations, 2);
    assert_eq!(step.represented_source().unwrap(), (rates, Y0));

    // A different independently valid source gives a different result; no
    // API accepts another problem/owner in the TRUE step.
    let fake = make([0.0; 3]);
    let fake_step = fake.begin_step(0.0, Y0, 0.0001).unwrap();
    assert_eq!(fake_step.represented_source().unwrap().0, [0.0; 3]);
    let mut fake_rhs = [f64::NAN; 3];
    fake_step
        .probe_rhs(0.0, Y0, &mut fake_rhs, &mut WorkCounters::default())
        .unwrap();
    assert_eq!(fake_rhs, [0.0; 3]);
    assert_ne!(fake_rhs, at_start);
    println!(
        "C9_TRUSTED_FROZEN_PROBE rhs={at_start:?} fake={fake_rhs:?} epoch={} evaluations={}",
        bundle.model_epoch(),
        work.rhs_evaluations
    );
}

#[test]
fn actual_original_rodas8_reference_uses_privately_bound_callbacks() {
    let bundle = make([0.5, 10.0, 200.0]);
    let step = bundle.begin_step(0.0, Y0, 0.0001).unwrap();
    let mut work = WorkCounters::default();
    let result = step.original_rodas8_direct_reference(&mut work).unwrap();
    assert_eq!(result.stages.len(), 8);
    assert!(work.linear_solves >= 8);
    assert_eq!(work.jvp_calls, 0);
    assert_eq!(bundle.model_epoch(), 0);
    assert_eq!(step.represented_source().unwrap().0, [0.5, 10.0, 200.0]);

    let coeffs = rodas5p_coefficients().unwrap();
    assert_eq!(coeffs.stages(), 8);
    // At stage 0, native U_0 = gamma K_0; original target is
    // U_0 - h*gamma*J(y0) U_0 - h*gamma*f(y0) = 0.
    let tau = 0.0001 * coeffs.gamma;
    let u0: [f64; 3] = std::array::from_fn(|i| coeffs.gamma * result.stages[0][i]);
    let j0 = [
        [-0.5, 10.0 * Y0[2], 10.0 * Y0[1]],
        [0.5, -10.0 * Y0[2] - 400.0 * Y0[1], -10.0 * Y0[1]],
        [0.0, 400.0 * Y0[1], 0.0],
    ];
    let f0 = [-7.0 / 32.0, -93.0 / 32.0, 25.0 / 8.0];
    let residual = std::array::from_fn::<_, 3, _>(|i| {
        let ju: f64 = (0..3).map(|j| j0[i][j] * u0[j]).sum();
        u0[i] - tau * (ju + f0[i])
    });
    let max_r = residual.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    let mass_defect = (result.y_new.iter().sum::<f64>() - 1.0).abs();
    assert!(
        max_r < 1.0e-9,
        "original 8-stage first raw-U residual {max_r:?}"
    );
    assert!(mass_defect < 1.0e-9);
    assert!(result.y_new.iter().all(|x| x.is_finite() && *x >= 0.0));
    println!(
        "C9_TRUSTED_SOURCE_RODAS8 max_original_u0_residual={max_r:.17e} mass_defect={mass_defect:.17e} ref_linear_solves={} epoch={}",
        work.linear_solves,
        bundle.model_epoch()
    );
}

#[test]
fn malformed_steps_and_stage_probes_fail_closed_with_untouched_outputs() {
    let bundle = make([0.5, 10.0, 200.0]);
    assert!(bundle.begin_step(-0.0, Y0, 0.0001).is_err());
    assert!(bundle.begin_step(0.1, Y0, 0.0001).is_err());
    assert!(bundle.begin_step(0.0, Y0, 0.0).is_err());
    assert!(bundle.begin_step(0.0, Y0, f64::NAN).is_err());
    assert!(bundle.begin_step(0.0, Y0, f64::INFINITY).is_err());
    let mut moved = Y0;
    moved[1] += 0.000001;
    assert!(bundle.begin_step(0.0, moved, 0.0001).is_err());
    let step = bundle.begin_step(0.0, Y0, 0.0001).unwrap();
    let mut out = [77.0, 88.0, 99.0];
    let mut negative = Y0;
    negative[0] = -0.001;
    let mut work = WorkCounters::default();
    assert!(step.probe_rhs(0.0, negative, &mut out, &mut work).is_err());
    assert_eq!(out, [77.0, 88.0, 99.0]);
    let mut outside = Y0;
    outside[1] = 3.0;
    assert!(step.probe_rhs(0.0, outside, &mut out, &mut work).is_err());
    assert_eq!(out, [77.0, 88.0, 99.0]);
    assert!(step.probe_rhs(f64::NAN, Y0, &mut out, &mut work).is_err());
    assert_eq!(out, [77.0, 88.0, 99.0]);
    assert_eq!(bundle.model_epoch(), 0);
}
