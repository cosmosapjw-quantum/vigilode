use rodas5p_core::{
    ClosureOperator, CoreError, DenseMatrix, DenseOperator, LinearOperator, ShiftedOperator,
    WorkCounters,
};
use rodas5p_integrators::{
    DirectionalQuadraticProposal, DirectionalQuadraticWorkspace, try_directional_quadratic,
};
use rodas5p_krylov::GmresConfig;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn config() -> GmresConfig {
    GmresConfig {
        rtol: 1e-12,
        atol: 1e-14,
        ..GmresConfig::default()
    }
}
fn rotation(sigma: f64) -> ShiftedOperator {
    let mut j = DenseMatrix::zeros(2, 2);
    j[(0, 1)] = -3.0;
    j[(1, 0)] = 3.0;
    ShiftedOperator::new_counted_jvp(None, Arc::new(DenseOperator::new(j).unwrap()), sigma, 1.0)
        .unwrap()
}
fn proposal() -> DirectionalQuadraticProposal {
    DirectionalQuadraticProposal::new(0.0, -9.0).unwrap()
}

#[test]
fn rotation_preserves_warm_start_and_original_residual_with_zero_arnoldi() {
    for x0 in [None, Some([0.7, -0.8])] {
        let op = rotation(0.25);
        let rhs = [2.0, -1.0];
        let mut output = [123.0; 2];
        let mut ws = DirectionalQuadraticWorkspace::default();
        let mut work = WorkCounters::default();
        let report = try_directional_quadratic(
            &op,
            0.25,
            proposal(),
            &rhs,
            x0.as_ref().map(|x| x.as_slice()),
            &config(),
            None,
            &mut output,
            &mut ws,
            &mut work,
        )
        .unwrap()
        .unwrap();
        let exact = [(2.0 + 0.75) / 1.5625, (-1.0 + 1.5) / 1.5625];
        for i in 0..2 {
            assert!((output[i] - exact[i]).abs() < 1e-14);
        }
        let mut ax = [0.0; 2];
        op.apply(&output, &mut ax).unwrap();
        assert!((rhs[0] - ax[0]).hypot(rhs[1] - ax[1]) <= 1e-12 * 5.0_f64.sqrt());
        assert_eq!(report.matvecs, if x0.is_some() { 3 } else { 2 });
        assert_eq!(work.jvp_calls, report.matvecs);
        assert_eq!(
            (
                report.iterations,
                report.cycles,
                report.least_squares_solves
            ),
            (0, 0, 0)
        );
        assert_eq!(work.linear_solves, 1);
        assert_eq!(ws.statistics().accepted, 1);
    }
}

#[test]
fn wrong_coefficients_refuse_without_writing_or_erasing_work() {
    let op = rotation(0.25);
    let mut output = [77.0; 2];
    let mut ws = DirectionalQuadraticWorkspace::default();
    let mut work = WorkCounters::default();
    let outcome = try_directional_quadratic(
        &op,
        0.25,
        DirectionalQuadraticProposal::new(0.0, 0.0).unwrap(),
        &[2.0, -1.0],
        None,
        &config(),
        None,
        &mut output,
        &mut ws,
        &mut work,
    )
    .unwrap();
    assert!(outcome.is_none());
    assert_eq!(output, [77.0; 2]);
    assert_eq!(work.linear_matvecs, 2);
    assert_eq!(work.linear_solves, 0);
    assert_eq!(ws.statistics().refused, 1);
}

#[test]
fn denominator_refusal_precedes_actions_and_zero_shift_is_identity() {
    let mut output = [77.0; 2];
    let mut ws = DirectionalQuadraticWorkspace::default();
    let mut work = WorkCounters::default();
    assert!(
        try_directional_quadratic(
            &rotation(1.0),
            1.0,
            DirectionalQuadraticProposal::new(1.0, 0.0).unwrap(),
            &[2.0, -1.0],
            Some(&[1.0, 1.0]),
            &config(),
            None,
            &mut output,
            &mut ws,
            &mut work
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(work.linear_matvecs, 0);
    assert_eq!(output, [77.0; 2]);
    let report = try_directional_quadratic(
        &rotation(0.0),
        0.0,
        proposal(),
        &[2.0, -1.0],
        None,
        &config(),
        None,
        &mut output,
        &mut ws,
        &mut work,
    )
    .unwrap()
    .unwrap();
    assert_eq!(output, [2.0, -1.0]);
    assert_eq!(report.matvecs, 1);
}

#[test]
fn initial_solution_shortcut_keeps_no_zero_action_contract() {
    let mut ws = DirectionalQuadraticWorkspace::default();
    let mut work = WorkCounters::default();
    let mut output = [77.0; 2];
    let report = try_directional_quadratic(
        &rotation(0.25),
        0.25,
        proposal(),
        &[0.0, 0.0],
        None,
        &config(),
        None,
        &mut output,
        &mut ws,
        &mut work,
    )
    .unwrap()
    .unwrap();
    assert_eq!(report.matvecs, 0);
    assert_eq!(output, [0.0; 2]);
    let report = try_directional_quadratic(
        &rotation(0.0),
        0.0,
        proposal(),
        &[2.0, -1.0],
        Some(&[2.0, -1.0]),
        &config(),
        None,
        &mut output,
        &mut ws,
        &mut work,
    )
    .unwrap()
    .unwrap();
    assert_eq!(report.matvecs, 1);
    assert_eq!(output, [2.0, -1.0]);
}

#[test]
fn callback_errors_and_partial_writes_are_fatal_and_paid() {
    for failing_action in [1, 2] {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let op = ClosureOperator::new(2, move |x: &[f64], y: &mut [f64]| {
            if c.fetch_add(1, Ordering::SeqCst) + 1 == failing_action {
                return Err(CoreError::InvalidInput(
                    "deliberate callback failure".into(),
                ));
            }
            y[0] = x[0] + 0.75 * x[1];
            y[1] = x[1] - 0.75 * x[0];
            Ok(())
        });
        let mut ws = DirectionalQuadraticWorkspace::default();
        let mut work = WorkCounters::default();
        let mut output = [77.0; 2];
        assert!(
            try_directional_quadratic(
                &op,
                0.25,
                proposal(),
                &[2.0, -1.0],
                None,
                &config(),
                None,
                &mut output,
                &mut ws,
                &mut work
            )
            .is_err()
        );
        assert_eq!(output, [77.0; 2]);
        assert_eq!(work.linear_matvecs, failing_action as u64);
        assert_eq!(ws.statistics().callback_failures, 1);
    }
    let op = ClosureOperator::new(2, |x: &[f64], y: &mut [f64]| {
        y[0] = x[0];
        Ok(())
    });
    assert!(
        try_directional_quadratic(
            &op,
            0.25,
            proposal(),
            &[2.0, -1.0],
            None,
            &config(),
            None,
            &mut [77.0; 2],
            &mut DirectionalQuadraticWorkspace::default(),
            &mut WorkCounters::default()
        )
        .is_err()
    );
}

#[test]
fn weighted_gate_and_live_operator_replacement_use_current_system() {
    let mut ws = DirectionalQuadraticWorkspace::default();
    let mut work = WorkCounters::default();
    let mut output = [77.0; 2];
    assert!(
        try_directional_quadratic(
            &rotation(0.25),
            0.25,
            proposal(),
            &[2.0, -1.0],
            None,
            &config(),
            Some(&[0.01, 2.0]),
            &mut output,
            &mut ws,
            &mut work
        )
        .unwrap()
        .is_some()
    );
    let before = output;
    assert!(
        try_directional_quadratic(
            &rotation(0.5),
            0.25,
            proposal(),
            &[2.0, -1.0],
            None,
            &config(),
            None,
            &mut output,
            &mut ws,
            &mut work
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(output, before);
    assert!(
        try_directional_quadratic(
            &rotation(0.25),
            0.25,
            proposal(),
            &[2.0, -1.0],
            None,
            &config(),
            Some(&[0.0, 2.0]),
            &mut output,
            &mut ws,
            &mut work
        )
        .is_err()
    );
    assert!(DirectionalQuadraticProposal::new(f64::NAN, 0.0).is_err());
}

#[test]
fn emit_actual_native_rotation_bits_for_independent_fraction_audit() {
    let mut records = Vec::new();
    for (sigma, rhs, x0) in [
        (0.0, vec![2.0, -1.0], None),
        (0.0625, vec![0.75, -2.5], None),
        (0.5, vec![2.0, -1.0], Some(vec![0.7, -0.8])),
        (2.0, vec![3.0, 4.0], None),
        (-0.25, vec![-0.5, 2.0], Some(vec![0.25, 0.5])),
        (
            0.125,
            vec![1.0, 2.0, -3.0, 0.5],
            Some(vec![0.1, -0.2, 0.3, 0.4]),
        ),
    ] {
        let n = rhs.len();
        let mut j = DenseMatrix::zeros(n, n);
        for i in (0..n).step_by(2) {
            j[(i, i + 1)] = -3.0;
            j[(i + 1, i)] = 3.0;
        }
        let op = ShiftedOperator::new_counted_jvp(
            None,
            Arc::new(DenseOperator::new(j).unwrap()),
            sigma,
            1.0,
        )
        .unwrap();
        let cfg = GmresConfig {
            atol: 1e-10,
            rtol: 1e-12,
            ..config()
        };
        let mut output = vec![77.0; n];
        let before = output.clone();
        let mut ws = DirectionalQuadraticWorkspace::default();
        let mut work = WorkCounters::default();
        let report = try_directional_quadratic(
            &op,
            sigma,
            proposal(),
            &rhs,
            x0.as_deref(),
            &cfg,
            None,
            &mut output,
            &mut ws,
            &mut work,
        )
        .unwrap()
        .unwrap();
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        records.push(serde_json::json!({
            "sigma_bits":sigma.to_bits(),"rhs_bits":bits(&rhs),
            "x0_bits":x0.as_deref().map(bits),"candidate_bits":bits(&output),
            "threshold_bits":cfg.atol.max(cfg.rtol*rodas5p_core::safe_l2(&rhs)).to_bits(),
            "a_bits":proposal().a.to_bits(),"b_bits":proposal().b.to_bits(),
            "status":"accepted","output_before_bits":bits(&before),"output_after_bits":bits(&output),
            "operator_actions":ws.statistics().operator_actions,"reported_residual_bits":report.residual_norm.to_bits()
        }));
    }
    for (sigma, coeffs) in [
        (0.25, DirectionalQuadraticProposal::new(0.0, 0.0).unwrap()),
        (1.0, DirectionalQuadraticProposal::new(1.0, 0.0).unwrap()),
    ] {
        let rhs = [2.0, -1.0];
        let mut output = [77.0; 2];
        let before = output;
        let mut ws = DirectionalQuadraticWorkspace::default();
        let mut work = WorkCounters::default();
        assert!(
            try_directional_quadratic(
                &rotation(sigma),
                sigma,
                coeffs,
                &rhs,
                None,
                &config(),
                None,
                &mut output,
                &mut ws,
                &mut work
            )
            .unwrap()
            .is_none()
        );
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        records.push(serde_json::json!({"sigma_bits":sigma.to_bits(),"rhs_bits":bits(&rhs),"x0_bits":null,"candidate_bits":null,"threshold_bits":config().atol.max(config().rtol*rodas5p_core::safe_l2(&rhs)).to_bits(),"a_bits":coeffs.a.to_bits(),"b_bits":coeffs.b.to_bits(),"status":"refused","output_before_bits":bits(&before),"output_after_bits":bits(&output),"operator_actions":ws.statistics().operator_actions}));
    }
    println!(
        "STRUCTURAL_NATIVE_BITS {}",
        serde_json::json!({"schema":"structural-native-bits-v1","source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").ok(),"records":records})
    );
}

#[test]
fn finite_input_range_refusals_do_not_invent_callback_failures() {
    let identity = ClosureOperator::new(2, |x: &[f64], y: &mut [f64]| {
        y.copy_from_slice(x);
        Ok(())
    });
    let cfg = GmresConfig {
        rtol: 0.0,
        atol: 1e-12,
        ..config()
    };
    let huge = [1.5e308, 1.5e308];
    let mut output = [77.0; 2];
    let mut ws = DirectionalQuadraticWorkspace::default();
    let mut work = WorkCounters::default();
    assert!(
        try_directional_quadratic(
            &identity,
            0.25,
            proposal(),
            &huge,
            Some(&huge),
            &cfg,
            None,
            &mut output,
            &mut ws,
            &mut work
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(work.linear_matvecs, 0);
    let rhs = [1e308, 0.0];
    let x0 = [-1e308, 0.0];
    assert!(
        try_directional_quadratic(
            &identity,
            0.25,
            proposal(),
            &rhs,
            Some(&x0),
            &cfg,
            None,
            &mut output,
            &mut ws,
            &mut work
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(work.linear_matvecs, 1);
    assert_eq!(ws.statistics().callback_failures, 0);
    assert_eq!(output, [77.0; 2]);
}
