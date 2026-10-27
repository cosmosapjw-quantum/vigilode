use rodas5p_integrators::fixed_point_continuation::*;
use std::cell::Cell;

fn id() -> ContinuationIdentity {
    ContinuationIdentity {
        target_id: 91,
        model_epoch: 3,
        branch_id: 7,
    }
}

struct Quadratic {
    epoch: Cell<u64>,
    f_calls: Cell<usize>,
    final_calls: Cell<usize>,
    change_epoch: bool,
    fail_f: bool,
    partial: bool,
    wrong_inverse: bool,
    large_error: bool,
    negative_second: bool,
}
impl Default for Quadratic {
    fn default() -> Self {
        Self {
            epoch: Cell::new(3),
            f_calls: Cell::new(0),
            final_calls: Cell::new(0),
            change_epoch: false,
            fail_f: false,
            partial: false,
            wrong_inverse: false,
            large_error: false,
            negative_second: false,
        }
    }
}
impl FixedPointHomotopyProvider for Quadratic {
    fn identity(&self) -> ContinuationIdentity {
        ContinuationIdentity {
            model_epoch: self.epoch.get(),
            ..id()
        }
    }
    fn dimension(&self) -> usize {
        1
    }
    fn inverse_identity(&self, patch: u64) -> u64 {
        patch + if self.wrong_inverse { 11 } else { 10 }
    }
    fn residual(&self, x: &[f64], lambda: f64, out: &mut [f64]) -> Result<f64, String> {
        self.f_calls.set(self.f_calls.get() + 1);
        if lambda == 1.0 {
            self.final_calls.set(self.final_calls.get() + 1);
        }
        if self.change_epoch {
            self.epoch.set(4);
        }
        if self.fail_f {
            return Err("deliberate F failure".into());
        }
        if self.partial {
            return Ok(0.0);
        }
        out[0] = x[0] * x[0] - (1.0 + lambda);
        Ok(if self.large_error {
            1.0
        } else {
            8.0 * f64::EPSILON * (x[0] * x[0] + 1.0 + lambda)
        })
    }
    fn apply_inverse(&self, patch: u64, x: &[f64], out: &mut [f64]) -> Result<f64, String> {
        let p = if patch == 2 && self.negative_second {
            -0.4
        } else if patch == 1 {
            0.45
        } else {
            0.4
        };
        out[0] = p * x[0];
        Ok(4.0 * f64::EPSILON * out[0].abs())
    }
}

fn patch(
    number: u64,
    left: f64,
    right: f64,
    center: f64,
    radius: f64,
    q: f64,
    beta: f64,
    p: f64,
) -> DeclaredFixedPointPatch {
    DeclaredFixedPointPatch::new(
        id(),
        number,
        number + 10,
        left,
        right,
        vec![center],
        radius,
        FixedPointBounds {
            contraction_upper: q,
            center_correction_upper: beta,
            inverse_action_norm_upper: p,
            inverse_action_inverse_norm_upper: 3.0,
        },
    )
    .unwrap()
}
fn single() -> DeclaredFixedPointAtlas {
    DeclaredFixedPointAtlas::new(id(), vec![patch(0, 0.0, 1.0, 1.25, 0.5, 0.5, 0.23, 0.4)]).unwrap()
}
fn two(second_center: f64) -> DeclaredFixedPointAtlas {
    DeclaredFixedPointAtlas::new(
        id(),
        vec![
            patch(1, 0.0, 0.5, 1.1, 0.3, 0.3, 0.14, 0.45),
            patch(2, 0.5, 1.0, second_center, 0.3, 0.3, 0.13, 0.4),
        ],
    )
    .unwrap()
}

#[test]
fn nonlinear_root_is_followed_without_derivative_callbacks() {
    let p = Quadratic::default();
    let mut work = FixedPointContinuationWork::default();
    let mut out = [-99.0];
    let result = continue_fixed_point(
        &p,
        &single(),
        &Default::default(),
        &[1.0],
        &mut out,
        &mut work,
    )
    .unwrap();
    assert!((out[0] - 2.0_f64.sqrt()).abs() <= result.final_conditional_root_error_upper + 2e-16);
    assert!(result.final_residual_upper <= 1e-10);
    assert!(p.final_calls.get() > 0);
    assert_eq!(result.stations[0].lambda, 0.0);
    assert_eq!(result.stations.last().unwrap().lambda, 1.0);
    assert_eq!(work.residual_calls, work.inverse_calls);
    assert_eq!(work.residual_calls, p.f_calls.get());
    assert_eq!(work.completed_stations, 2);
}

#[test]
fn overlapping_balls_transport_the_same_branch() {
    let p = Quadratic::default();
    let mut work = FixedPointContinuationWork::default();
    let mut out = [0.0];
    let result = continue_fixed_point(
        &p,
        &two(1.3),
        &Default::default(),
        &[1.0],
        &mut out,
        &mut work,
    )
    .unwrap();
    assert_eq!(work.branch_overlap_checks, 1);
    assert_eq!(result.stations.len(), 3);
    assert!((out[0] - 2.0_f64.sqrt()).abs() < 1e-10);
}

#[test]
fn same_identity_cannot_jump_to_disjoint_negative_branch() {
    let p = Quadratic {
        negative_second: true,
        ..Default::default()
    };
    let mut work = FixedPointContinuationWork::default();
    let mut out = [99.0];
    let error = continue_fixed_point(
        &p,
        &two(-1.3),
        &Default::default(),
        &[1.0],
        &mut out,
        &mut work,
    )
    .unwrap_err();
    assert_eq!(
        error,
        FixedPointContinuationError::BranchOverlap { patch_id: 2 }
    );
    assert_eq!(out, [99.0]);
    assert_eq!(work.completed_stations, 2);
    assert_eq!(p.final_calls.get(), 0);
}

#[test]
fn stale_epoch_and_inverse_are_refused_before_work() {
    for p in [
        Quadratic {
            epoch: Cell::new(4),
            ..Default::default()
        },
        Quadratic {
            wrong_inverse: true,
            ..Default::default()
        },
    ] {
        let mut work = FixedPointContinuationWork::default();
        let mut out = [99.0];
        assert_eq!(
            continue_fixed_point(
                &p,
                &single(),
                &Default::default(),
                &[1.0],
                &mut out,
                &mut work
            )
            .unwrap_err(),
            FixedPointContinuationError::IdentityMismatch
        );
        assert_eq!(p.f_calls.get(), 0);
        assert_eq!(work.residual_calls, 0);
        assert_eq!(out, [99.0]);
    }
}

#[test]
fn mutation_during_callback_is_terminal_and_charged() {
    let p = Quadratic {
        change_epoch: true,
        ..Default::default()
    };
    let mut work = FixedPointContinuationWork::default();
    let mut out = [99.0];
    assert_eq!(
        continue_fixed_point(
            &p,
            &single(),
            &Default::default(),
            &[1.0],
            &mut out,
            &mut work
        )
        .unwrap_err(),
        FixedPointContinuationError::IdentityMismatch
    );
    assert_eq!(work.residual_calls, 1);
    assert_eq!(work.inverse_calls, 0);
    assert_eq!(out, [99.0]);
}

#[test]
fn callback_failure_and_partial_writes_are_not_numerical_successes() {
    for (p, partial) in [
        (
            Quadratic {
                fail_f: true,
                ..Default::default()
            },
            false,
        ),
        (
            Quadratic {
                partial: true,
                ..Default::default()
            },
            true,
        ),
    ] {
        let mut work = FixedPointContinuationWork::default();
        let mut out = [99.0];
        let error = continue_fixed_point(
            &p,
            &single(),
            &Default::default(),
            &[1.0],
            &mut out,
            &mut work,
        )
        .unwrap_err();
        if partial {
            assert_eq!(error, FixedPointContinuationError::InvalidCallbackOutput);
        } else {
            assert!(matches!(
                error,
                FixedPointContinuationError::Callback { kind: "F", .. }
            ));
        }
        assert_eq!(work.residual_calls, 1);
        assert_eq!(work.inverse_calls, 0);
        assert_eq!(out, [99.0]);
    }
}

#[test]
fn cumulative_callback_budget_and_iteration_limit_preserve_output() {
    let p = Quadratic::default();
    let mut work = FixedPointContinuationWork::default();
    let mut out = [99.0];
    let config = FixedPointContinuationConfig {
        max_residual_calls: 2,
        ..Default::default()
    };
    assert_eq!(
        continue_fixed_point(&p, &single(), &config, &[1.0], &mut out, &mut work).unwrap_err(),
        FixedPointContinuationError::Budget("F calls")
    );
    assert_eq!(work.residual_calls, 2);
    assert_eq!(out, [99.0]);
    let calls = p.f_calls.get();
    assert_eq!(
        continue_fixed_point(&p, &single(), &config, &[1.0], &mut out, &mut work).unwrap_err(),
        FixedPointContinuationError::Budget("F calls")
    );
    assert_eq!(p.f_calls.get(), calls);
    let config = FixedPointContinuationConfig {
        max_iterations_per_station: 1,
        ..Default::default()
    };
    assert!(matches!(
        continue_fixed_point(
            &p,
            &single(),
            &config,
            &[1.0],
            &mut out,
            &mut Default::default()
        ),
        Err(FixedPointContinuationError::IterationLimit { lambda: 1.0 })
    ));
    assert_eq!(out, [99.0]);
}

#[test]
fn callback_error_allowance_is_part_of_original_residual_acceptance() {
    let p = Quadratic {
        large_error: true,
        ..Default::default()
    };
    let mut out = [99.0];
    let config = FixedPointContinuationConfig {
        max_iterations_per_station: 2,
        ..Default::default()
    };
    assert_eq!(
        continue_fixed_point(
            &p,
            &single(),
            &config,
            &[1.0],
            &mut out,
            &mut Default::default()
        )
        .unwrap_err(),
        FixedPointContinuationError::IterationLimit { lambda: 0.0 }
    );
    assert_eq!(out, [99.0]);
}

#[test]
fn every_requested_parameter_station_and_final_target_is_evaluated() {
    let p = Quadratic::default();
    let mut work = FixedPointContinuationWork::default();
    let mut out = [0.0];
    let config = FixedPointContinuationConfig {
        substeps_per_patch: 4,
        ..Default::default()
    };
    let result = continue_fixed_point(&p, &two(1.3), &config, &[1.0], &mut out, &mut work).unwrap();
    assert_eq!(result.stations.len(), 9);
    for (i, s) in result.stations.iter().enumerate() {
        assert_eq!(s.lambda, i as f64 / 8.0);
    }
    assert_eq!(
        result.stations.last().unwrap().residual_upper,
        result.final_residual_upper
    );
}

#[test]
fn invalid_bounds_and_gapped_or_relabelled_atlases_fail_admission() {
    for (q, beta) in [(1.0, 0.0), (-0.1, 0.0), (0.5, 1.0), (0.5, f64::NAN)] {
        assert!(
            DeclaredFixedPointPatch::new(
                id(),
                0,
                10,
                0.0,
                1.0,
                vec![1.25],
                0.5,
                FixedPointBounds {
                    contraction_upper: q,
                    center_correction_upper: beta,
                    inverse_action_norm_upper: 0.4,
                    inverse_action_inverse_norm_upper: 2.5
                }
            )
            .is_err()
        );
    }
    assert!(
        DeclaredFixedPointAtlas::new(
            id(),
            vec![
                patch(1, 0.0, 0.4, 1.1, 0.3, 0.3, 0.14, 0.45),
                patch(2, 0.5, 1.0, 1.3, 0.3, 0.3, 0.13, 0.4)
            ]
        )
        .is_err()
    );
    let other = ContinuationIdentity {
        branch_id: 8,
        ..id()
    };
    assert!(DeclaredFixedPointAtlas::new(other, single().patches().to_vec()).is_err());
}

#[test]
fn outside_initial_ball_and_invalid_budget_never_call_f() {
    let p = Quadratic::default();
    let mut out = [99.0];
    assert_eq!(
        continue_fixed_point(
            &p,
            &single(),
            &Default::default(),
            &[-1.0],
            &mut out,
            &mut Default::default()
        )
        .unwrap_err(),
        FixedPointContinuationError::OutsidePatch { patch_id: 0 }
    );
    let config = FixedPointContinuationConfig {
        max_stations: 1,
        ..Default::default()
    };
    assert_eq!(
        continue_fixed_point(
            &p,
            &single(),
            &config,
            &[1.0],
            &mut out,
            &mut Default::default()
        )
        .unwrap_err(),
        FixedPointContinuationError::Budget("stations")
    );
    assert_eq!(p.f_calls.get(), 0);
    assert_eq!(out, [99.0]);
}

#[test]
fn no_singular_fold_patch_is_admitted_by_a_label() {
    // F=x²-lambda at lambda=0 has F_x=0. Every finite P gives T_x=1,
    // so an honest patch containing the fold necessarily has q>=1.
    let fold = DeclaredFixedPointPatch::new(
        id(),
        0,
        10,
        0.0,
        1.0,
        vec![0.0],
        1.0,
        FixedPointBounds {
            contraction_upper: 1.0,
            center_correction_upper: 1.0,
            inverse_action_norm_upper: 1.0,
            inverse_action_inverse_norm_upper: 1.0,
        },
    );
    assert!(fold.is_err());
}

#[test]
fn rounded_ball_boundary_uses_outward_comparison() {
    let p = Quadratic::default();
    let mut out = [99.0];
    for (center, initial) in [(-2.0_f64.powi(-53), 1.0), (2.0_f64.powi(-53), -1.0)] {
        // Nearest subtraction rounds the distance to exactly one, although
        // the exact represented-input distance is strictly greater than one.
        assert_eq!((initial - center).abs(), 1.0);
        let ball = DeclaredFixedPointAtlas::new(
            id(),
            vec![patch(0, 0.0, 1.0, center, 1.0, 0.0, 0.0, 0.4)],
        )
        .unwrap();
        assert_eq!(
            continue_fixed_point(
                &p,
                &ball,
                &Default::default(),
                &[initial],
                &mut out,
                &mut Default::default()
            )
            .unwrap_err(),
            FixedPointContinuationError::OutsidePatch { patch_id: 0 }
        );
    }
    assert_eq!(p.f_calls.get(), 0);
    assert_eq!(out, [99.0]);
}

#[test]
fn inconsistent_inverse_norms_cannot_form_a_declared_patch() {
    let result = DeclaredFixedPointPatch::new(
        id(),
        0,
        10,
        0.0,
        1.0,
        vec![0.0],
        1.0,
        FixedPointBounds {
            contraction_upper: 0.5,
            center_correction_upper: 0.0,
            inverse_action_norm_upper: 0.4,
            inverse_action_inverse_norm_upper: 1.0,
        },
    );
    assert_eq!(
        result.unwrap_err(),
        FixedPointContinuationError::Input("inverse norm bounds are inconsistent")
    );
}

#[test]
fn failed_partial_or_epoch_mutating_inverse_preserves_output() {
    struct Broken {
        base: Quadratic,
        mode: usize,
    }
    impl FixedPointHomotopyProvider for Broken {
        fn identity(&self) -> ContinuationIdentity {
            self.base.identity()
        }
        fn dimension(&self) -> usize {
            1
        }
        fn inverse_identity(&self, p: u64) -> u64 {
            self.base.inverse_identity(p)
        }
        fn residual(&self, x: &[f64], l: f64, o: &mut [f64]) -> Result<f64, String> {
            self.base.residual(x, l, o)
        }
        fn apply_inverse(&self, _: u64, _: &[f64], o: &mut [f64]) -> Result<f64, String> {
            match self.mode {
                0 => Err("deliberate P failure".into()),
                1 => Ok(0.0),
                _ => {
                    o[0] = 0.0;
                    self.base.epoch.set(4);
                    Ok(0.0)
                }
            }
        }
    }
    for mode in 0..3 {
        let p = Broken {
            base: Default::default(),
            mode,
        };
        let mut out = [99.0];
        let mut work = FixedPointContinuationWork::default();
        let error = continue_fixed_point(
            &p,
            &single(),
            &Default::default(),
            &[1.0],
            &mut out,
            &mut work,
        )
        .unwrap_err();
        match mode {
            0 => assert!(matches!(
                error,
                FixedPointContinuationError::Callback { kind: "P", .. }
            )),
            1 => assert_eq!(error, FixedPointContinuationError::InvalidCallbackOutput),
            _ => assert_eq!(error, FixedPointContinuationError::IdentityMismatch),
        }
        assert_eq!(out, [99.0]);
        assert_eq!(work.residual_calls, 1);
        assert_eq!(work.inverse_calls, 1);
    }
}
