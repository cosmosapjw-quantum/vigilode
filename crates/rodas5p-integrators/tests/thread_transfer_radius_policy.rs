//! Residual-seeded radius and stage/component radius box (research node
//! `research/thread_transfer_radius_proposal_20261002`, thread-transfer DAG
//! node P1-RADIUS-PROPOSAL). Proposals never decide: only the full closure
//! evaluation at the proposed box does.

#[path = "thread_transfer_common/mod.rs"]
mod common;

use common::{
    ATOL, INITIAL_RADIUS, R4_DIMENSIONS, RTOL, encloses_root, r4_fixture, root_distances,
    write_output,
};
use rodas5p_integrators::{
    AffineMajorant, DiagonalMajorant, ParallelExecution, PathEvaluation, RadiusBox,
    blocked_box_certificate_with_execution, blocked_doubling_certificate_with_execution,
    causal_radius_box, certify_stage_target, evaluate_radius_box, residual_seeded_common_radius,
};
use serde_json::json;

const MODES: [PathEvaluation; 2] = [PathEvaluation::MatrixOrder, PathEvaluation::ActionFirst];
const INFLATION: f64 = 1.0 / 1_048_576.0; // 2^-20

fn mode_name(mode: PathEvaluation) -> &'static str {
    match mode {
        PathEvaluation::MatrixOrder => "matrix",
        PathEvaluation::ActionFirst => "action",
    }
}

/// `k_0 = 1, k_i = 1 + k_(i-1)^2` at the zero candidate.
fn scalar_no_go() -> AffineMajorant {
    let sub = vec![
        vec![0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
    ];
    AffineMajorant::new(vec![vec![0.0; 3]; 3], sub.clone(), sub, vec![1.0; 3]).unwrap()
}

fn output_over_serial(
    certificate: &rodas5p_integrators::StageCertificate,
    serial: &rodas5p_integrators::StageCertificate,
) -> f64 {
    certificate
        .output_bound
        .iter()
        .zip(&serial.output_bound)
        .map(|(d, s)| d / s.max(f64::MIN_POSITIVE))
        .fold(0.0_f64, f64::max)
}

#[test]
fn radius_proposals_close_only_when_rechecked() {
    let execution = ParallelExecution::sequential();
    let mut seed_closes = true;
    let mut constant_box_identical = true;
    let mut causal_box_ok = true;
    let mut fixtures = Vec::new();
    for n in R4_DIMENSIONS {
        let f = r4_fixture(n).unwrap();
        let roots = root_distances(n);
        let entries =
            DiagonalMajorant::new(&f.target, &f.problem, &f.candidate, &f.witness).unwrap();
        let serial = certify_stage_target(
            &f.target,
            &f.problem,
            &f.candidate,
            &f.y_hat,
            &f.e_hat,
            &f.witness,
            ATOL,
            RTOL,
        )
        .unwrap();
        let schedule = blocked_doubling_certificate_with_execution(
            &f.target,
            &f.problem,
            &f.candidate,
            &f.y_hat,
            &f.e_hat,
            &f.witness,
            ATOL,
            RTOL,
            INITIAL_RADIUS,
            8,
            &execution,
        )
        .unwrap();
        let certify = |radii: &RadiusBox, mode| {
            blocked_box_certificate_with_execution(
                &f.target,
                &f.problem,
                &f.candidate,
                &f.y_hat,
                &f.e_hat,
                &f.witness,
                ATOL,
                RTOL,
                radii,
                mode,
                &execution,
            )
            .unwrap()
        };
        let mut arms = Vec::new();
        for mode in MODES {
            for factor in [2.0, 1.1] {
                let proposal =
                    residual_seeded_common_radius(&entries, factor, mode, &execution).unwrap();
                let radii = proposal.radii.clone().unwrap();
                let checked = certify(&radii, mode);
                let encloses = checked
                    .certificate
                    .as_ref()
                    .is_some_and(|c| encloses_root(c, &roots));
                let ok = proposal.closes
                    && proposal.evaluations.len() == 2
                    && checked.evaluation.closes
                    && encloses;
                if factor == 2.0 {
                    seed_closes &= ok;
                }
                let operations = proposal
                    .evaluations
                    .iter()
                    .map(|e| e.work.directed_operations)
                    .sum::<u64>();
                arms.push(json!({
                    "policy": proposal.policy, "mode": mode_name(mode), "closes": proposal.closes,
                    "evaluations": proposal.evaluations.len(), "radius": radii.max(),
                    "preflight_b": proposal.evaluations[0].max_state_radius,
                    "required_at_proposal": proposal.evaluations.last().unwrap().max_state_radius,
                    "directed_operations": operations, "certificate_encloses_exact_root": encloses,
                    "output_over_serial_max": checked.certificate.as_ref().map(|c| output_over_serial(c, &serial)),
                }));
            }
            // The causal box, verified by the full evaluation.
            let proposal = causal_radius_box(&entries, INFLATION, mode, &execution).unwrap();
            let radii = proposal.radii.clone().unwrap();
            let checked = certify(&radii, mode);
            let encloses = checked
                .certificate
                .as_ref()
                .is_some_and(|c| encloses_root(c, &roots));
            causal_box_ok &= proposal.closes && checked.evaluation.closes && encloses;
            arms.push(json!({
                "policy": proposal.policy, "mode": mode_name(mode), "closes": proposal.closes,
                "evaluations": proposal.evaluations.len(), "radius_max": radii.max(),
                "radius_stage_max": (0..f.target.stages()).map(|i| radii.radii.iter().map(|row| row[i]).fold(0.0_f64, f64::max)).collect::<Vec<_>>(),
                "directed_operations": proposal.evaluations[0].work.directed_operations,
                "certificate_encloses_exact_root": encloses,
                "output_over_serial_max": checked.certificate.as_ref().map(|c| output_over_serial(c, &serial)),
            }));
        }
        // A constant box in matrix order is the blocked certificate, bit for
        // bit, at a closing and at a failing radius.
        let closing =
            residual_seeded_common_radius(&entries, 2.0, PathEvaluation::MatrixOrder, &execution)
                .unwrap()
                .radii
                .unwrap()
                .max();
        let mut identical = true;
        for radius in [closing, INITIAL_RADIUS] {
            let blocked = blocked_doubling_certificate_with_execution(
                &f.target,
                &f.problem,
                &f.candidate,
                &f.y_hat,
                &f.e_hat,
                &f.witness,
                ATOL,
                RTOL,
                radius,
                1,
                &execution,
            )
            .unwrap();
            let boxed = certify(
                &RadiusBox::common(n, f.target.stages(), radius).unwrap(),
                PathEvaluation::MatrixOrder,
            );
            identical &= blocked.doubling.attempts[0].max_state_radius.to_bits()
                == boxed.evaluation.max_state_radius.to_bits()
                && blocked.doubling.attempts[0].closes == boxed.evaluation.closes
                && match (&blocked.doubling.certificate, &boxed.certificate) {
                    (Some(b), Some(x)) => {
                        b.stage_bound
                            .iter()
                            .flatten()
                            .zip(x.stage_bound.iter().flatten())
                            .all(|(p, q)| p.to_bits() == q.to_bits())
                            && b.directed_operations == x.directed_operations
                    }
                    (None, None) => true,
                    _ => false,
                };
        }
        constant_box_identical &= identical;
        println!("n={n}: arms {}", serde_json::to_string(&arms).unwrap());
        fixtures.push(json!({
            "dimension": n,
            "arms": arms,
            "constant_box_bit_identical_to_blocked": identical,
            "extended_schedule": {
                "first_closure": schedule.doubling.attempts.iter().position(|a| a.closes).map(|k| k + 1),
                "evaluations": schedule.doubling.attempts.len(),
                "directed_operations": schedule.work.directed_operations,
            },
            "serial_directed_operations": serial.directed_operations,
            "serial_encloses_exact_root": encloses_root(&serial, &roots),
        }));
    }

    // 3. The scalar no-go system against the box.
    let no_go = scalar_no_go();
    let mut no_go_rows = Vec::new();
    let mut common_fails = true;
    for d in [0.0, 1.0 / 1024.0, 1.0, 2.0, 1024.0, 2.0_f64.powi(40)] {
        let evaluation = evaluate_radius_box(
            &no_go,
            &RadiusBox::common(1, 3, d).unwrap(),
            PathEvaluation::MatrixOrder,
            &execution,
        )
        .unwrap();
        common_fails &= !evaluation.closes;
        no_go_rows.push(json!({"radius": d, "closes": evaluation.closes, "state_radius": evaluation.state_radius[0]}));
    }
    let mut seed_reports_failure = true;
    for factor in [2.0, 1.1] {
        for mode in MODES {
            let proposal = residual_seeded_common_radius(&no_go, factor, mode, &execution).unwrap();
            seed_reports_failure &= !proposal.closes && proposal.reason.is_some();
        }
    }
    let mut box_closes = true;
    for mode in MODES {
        box_closes &= causal_radius_box(&no_go, INFLATION, mode, &execution)
            .unwrap()
            .closes;
        box_closes &= evaluate_radius_box(
            &no_go,
            &RadiusBox {
                radii: vec![vec![0.0, 1.0, 2.0]],
            },
            mode,
            &execution,
        )
        .unwrap()
        .closes;
    }
    let causal =
        causal_radius_box(&no_go, INFLATION, PathEvaluation::MatrixOrder, &execution).unwrap();
    let scalar_no_go_ok = common_fails && seed_reports_failure && box_closes;

    // 5. Fail closed.
    let f = r4_fixture(2).unwrap();
    let entries = DiagonalMajorant::new(&f.target, &f.problem, &f.candidate, &f.witness).unwrap();
    let bad_boxes = [
        (
            "nan",
            RadiusBox {
                radii: vec![vec![f64::NAN; 8]; 2],
            },
        ),
        (
            "infinite",
            RadiusBox {
                radii: vec![vec![f64::INFINITY; 8]; 2],
            },
        ),
        (
            "negative",
            RadiusBox {
                radii: vec![vec![-1.0; 8]; 2],
            },
        ),
        (
            "wrong-components",
            RadiusBox {
                radii: vec![vec![1.0; 8]; 3],
            },
        ),
        (
            "wrong-stages",
            RadiusBox {
                radii: vec![vec![1.0; 7]; 2],
            },
        ),
    ];
    let mut fail_closed = Vec::new();
    let mut all_fail_closed = true;
    for (name, radii) in &bad_boxes {
        let rejected = blocked_box_certificate_with_execution(
            &f.target,
            &f.problem,
            &f.candidate,
            &f.y_hat,
            &f.e_hat,
            &f.witness,
            ATOL,
            RTOL,
            radii,
            PathEvaluation::MatrixOrder,
            &execution,
        )
        .is_err();
        all_fail_closed &= rejected;
        fail_closed.push(json!({"case": name, "rejected": rejected}));
    }
    for factor in [f64::NAN, f64::INFINITY, -2.0, 0.0] {
        let rejected = residual_seeded_common_radius(
            &entries,
            factor,
            PathEvaluation::MatrixOrder,
            &execution,
        )
        .is_err();
        all_fail_closed &= rejected;
        fail_closed.push(json!({"case": format!("factor {factor}"), "rejected": rejected}));
    }
    let overflow = AffineMajorant::new(
        vec![vec![0.0, 0.0], vec![0.0, 0.0]],
        vec![vec![0.0, 0.0], vec![0.0, 0.0]],
        vec![vec![0.0, 0.0], vec![1.0e300, 0.0]],
        vec![1.0e300, 0.0],
    )
    .unwrap();
    let rejected =
        residual_seeded_common_radius(&overflow, 2.0, PathEvaluation::MatrixOrder, &execution)
            .is_err();
    all_fail_closed &= rejected;
    fail_closed.push(json!({"case": "overflowing proposal", "rejected": rejected}));
    let zero = AffineMajorant::new(
        vec![vec![0.0, 0.0], vec![1.0, 0.0]],
        vec![vec![0.0, 0.0], vec![1.0, 0.0]],
        vec![vec![0.0, 0.0], vec![1.0, 0.0]],
        vec![0.0, 0.0],
    )
    .unwrap();
    let zero_proposal =
        residual_seeded_common_radius(&zero, 2.0, PathEvaluation::MatrixOrder, &execution).unwrap();
    let zero_ok = zero_proposal.closes
        && zero_proposal.evaluations.len() == 1
        && zero_proposal.radii.as_ref().unwrap().max() == 0.0;
    all_fail_closed &= zero_ok;
    fail_closed.push(json!({"case": "zero seed proposes D = 0 and closes", "holds": zero_ok}));

    let gate = json!({
        "residual_seed_factor_2_closes_r4": seed_closes,
        "constant_box_equals_blocked": constant_box_identical,
        "scalar_no_go_versus_box": scalar_no_go_ok,
        "causal_box_on_r4": causal_box_ok,
        "fail_closed": all_fail_closed,
    });
    let pass = gate
        .as_object()
        .unwrap()
        .values()
        .all(|v| v.as_bool() == Some(true));
    let result = json!({
        "schema": "vigilode-thread-transfer-radius-policy-v1",
        "gate": gate,
        "verdict": if pass { "PASS" } else { "FAIL" },
        "inflation": INFLATION,
        "r4_fixtures": fixtures,
        "scalar_no_go": {
            "common_radius": no_go_rows,
            "causal_box": causal.radii,
            "causal_box_state_radius": causal.evaluations[0].state_radius,
        },
        "fail_closed": fail_closed,
    });
    write_output("THREAD_TRANSFER_RADIUS_POLICY_OUTPUT", &result);
    println!(
        "{}",
        json!({"gate": result["gate"], "verdict": result["verdict"]})
    );
    assert!(constant_box_identical && all_fail_closed && scalar_no_go_ok);
}

#[test]
fn proposals_are_rebuilt_from_the_step_residual() {
    // The proposals see the residual only: the majorant is rebuilt from the
    // step's inputs and a changed candidate changes the seeds.
    let f = r4_fixture(4).unwrap();
    let mut other = f.candidate.clone();
    other[3][1] += 1.0e-3;
    let a = DiagonalMajorant::new(&f.target, &f.problem, &f.candidate, &f.witness).unwrap();
    let b = DiagonalMajorant::new(&f.target, &f.problem, &other, &f.witness).unwrap();
    let execution = ParallelExecution::sequential();
    let pa =
        residual_seeded_common_radius(&a, 2.0, PathEvaluation::MatrixOrder, &execution).unwrap();
    let pb =
        residual_seeded_common_radius(&b, 2.0, PathEvaluation::MatrixOrder, &execution).unwrap();
    // The preflight path sum moves with the residual of the changed stage.
    assert_ne!(pa.evaluations[0].bound[3][1], pb.evaluations[0].bound[3][1]);
    assert_eq!(pa.evaluations[0].bound[2], pb.evaluations[0].bound[2]);
}
