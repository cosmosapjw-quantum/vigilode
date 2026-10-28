use rodas5p_core::{DenseMatrix, LuFactorization, rodas5p_coefficients};
use rodas5p_integrators::ParallelExecution;
use vigilode_coupled::structural_mean_field::MeanFieldAffine;

fn exact(y: &[f64], b: &[f64], d: f64, k: f64, t: f64) -> Vec<f64> {
    let my = y.iter().sum::<f64>() / y.len() as f64;
    let mb = b.iter().sum::<f64>() / b.len() as f64;
    let integral = |r: f64| if r == 0.0 { t } else { -(-r * t).exp_m1() / r };
    let collective = (-(d - k) * t).exp() * my + integral(d - k) * mb;
    y.iter()
        .zip(b)
        .map(|(v, bv)| collective + (-d * t).exp() * (v - my) + integral(d) * (bv - mb))
        .collect()
}

// Dense assembly exists only in this independent test oracle.
fn dense_oracle(y: &[f64], b: &[f64], d: f64, k: f64, h: f64) -> Vec<Vec<f64>> {
    let c = rodas5p_coefficients().unwrap();
    let n = y.len();
    let hg = h * c.gamma;
    let mut matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            matrix[(i, j)] = if i == j { 1.0 + hg * d } else { 0.0 } - hg * k / n as f64;
        }
    }
    let lu = LuFactorization::new(&matrix).unwrap();
    let mut u = Vec::<Vec<f64>>::new();
    for i in 0..8 {
        let mut state = y.to_vec();
        let mut mix = vec![0.0; n];
        for (j, previous_stage) in u.iter().take(i).enumerate() {
            for q in 0..n {
                state[q] += c.a[(i, j)] * previous_stage[q];
                mix[q] += c.c_matrix[(i, j)] * previous_stage[q];
            }
        }
        let mu = state.iter().sum::<f64>() / n as f64;
        let rhs: Vec<_> = (0..n)
            .map(|q| hg * (b[q] - d * state[q] + k * mu) + c.gamma * mix[q])
            .collect();
        u.push(lu.solve(&rhs).unwrap());
    }
    u
}

#[test]
fn exact_quotient_retains_arbitrary_transverse_state_and_forcing() {
    let execution = ParallelExecution::sequential();
    for n in [1, 3, 9] {
        let y: Vec<_> = (0..n).map(|i| 0.3 + (i as f64 + 0.5).sin()).collect();
        let b: Vec<_> = (0..n).map(|i| (2.0 * i as f64 + 0.1).cos()).collect();
        for (d, k, h) in [
            (0.0, 0.0, 0.1),
            (2.0, 2.0, 0.02),
            (2.0, 0.5, 0.02),
            (1000.0, 300.0, 0.001),
        ] {
            let model = MeanFieldAffine::new(d, k, b.clone()).unwrap();
            let output = model
                .plan(h)
                .unwrap()
                .step(&model, &y, 1e-9, &execution)
                .unwrap();
            let reference = dense_oracle(&y, &b, d, k, h);
            for (row, expected) in output.stages_u.iter().zip(reference) {
                for (v, r) in row.iter().zip(expected) {
                    assert!((v - r).abs() < 3e-12, "{n} {d} {k}: {v} != {r}");
                }
            }
            assert_eq!(output.work.rhs_evaluations, 9);
            assert_eq!(output.work.jvp_evaluations, 0);
            assert_eq!(output.work.matrix_factorizations, 0);
            assert_eq!(output.work.newton_updates, 0);
        }
    }
}

#[test]
fn stronger_structural_baseline_and_parallel_lift_keep_same_target() {
    let y: Vec<_> = (0..96).map(|i| 0.5 + 0.1 * (i as f64).sin()).collect();
    let b: Vec<_> = (0..96)
        .map(|i| 0.2 + 0.05 * (3.0 * i as f64).cos())
        .collect();
    let model = MeanFieldAffine::new(20.0, 19.0, b).unwrap();
    let plan = model.plan(1.0 / 64.0).unwrap();
    let sequential = ParallelExecution::sequential();
    let parallel = ParallelExecution::rayon(2).unwrap();
    let fast = plan.step(&model, &y, 1e-10, &sequential).unwrap();
    let threaded = plan.step(&model, &y, 1e-10, &parallel).unwrap();
    let direct = model
        .sequential_step(&y, plan.step_size(), 1e-10, &sequential)
        .unwrap();
    assert_eq!(fast.stages_u, threaded.stages_u);
    assert_eq!(fast.y_new, threaded.y_new);
    for (a, b) in fast.y_new.iter().zip(direct.y_new) {
        assert!((a - b).abs() < 2e-13);
    }
    assert!(fast.work.rhs_evaluations < direct.work.rhs_evaluations);
    assert!(
        fast.work.full_state_causal_accumulations < direct.work.full_state_causal_accumulations
    );
}

#[test]
fn fifth_order_affine_trajectory_against_closed_form() {
    let y0 = vec![0.3, 0.9, -0.2];
    let b = vec![0.7, -0.1, 0.4];
    let model = MeanFieldAffine::new(2.0, 0.5, b.clone()).unwrap();
    let reference = exact(&y0, &b, 2.0, 0.5, 1.0);
    let mut errors = Vec::new();
    for count in [4, 8, 16, 32] {
        let plan = model.plan(1.0 / count as f64).unwrap();
        let mut y = y0.clone();
        for _ in 0..count {
            y = plan
                .step(&model, &y, 1e-10, &ParallelExecution::sequential())
                .unwrap()
                .y_new;
        }
        errors.push(
            y.iter()
                .zip(&reference)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f64::max),
        );
    }
    println!("STRUCTURAL_ORDER_ERRORS={errors:?}");
    for pair in errors.windows(2) {
        assert!((pair[0] / pair[1]).log2() > 4.5, "{errors:?}");
    }
}

#[test]
fn invalid_inputs_and_wrong_rate_plan_are_refused() {
    assert!(MeanFieldAffine::new(1.0, 2.0, vec![0.0]).is_err());
    assert!(MeanFieldAffine::new(1.0, 0.0, vec![]).is_err());
    assert!(MeanFieldAffine::new(1.0, 0.0, vec![f64::NAN]).is_err());
    let a = MeanFieldAffine::new(1.0, 0.0, vec![0.0]).unwrap();
    let b = MeanFieldAffine::new(2.0, 0.0, vec![0.0]).unwrap();
    assert!(a.plan(0.0).is_err());
    assert!(a.plan(f64::INFINITY).is_err());
    let plan = a.plan(0.1).unwrap();
    assert!(
        plan.step(&b, &[1.0], 1e-10, &ParallelExecution::sequential())
            .is_err()
    );
    assert!(
        plan.step(&a, &[], 1e-10, &ParallelExecution::sequential())
            .is_err()
    );
    assert!(
        plan.step(&a, &[f64::NAN], 1e-10, &ParallelExecution::sequential())
            .is_err()
    );
    assert!(
        plan.step(&a, &[1.0], 0.0, &ParallelExecution::sequential())
            .is_err()
    );
}

#[test]
fn derived_residual_amplification_matches_independent_full_operator_and_is_sharp() {
    let c = rodas5p_coefficients().unwrap();
    for n in [1, 3, 7] {
        for (d, k, h) in [
            (0.0, 0.0, 0.125),
            (2.0, 2.0, 0.125),
            (20.0, 19.0, 1.0 / 64.0),
        ] {
            let m = MeanFieldAffine::new(d, k, vec![0.0; n]).unwrap();
            let plan = m.plan(h).unwrap();
            let budget = plan.algebraic_residual_budget(n, 64, 1e-8).unwrap();
            let hg = h * c.gamma;
            // Independent full original-stage operator, used only by this oracle.
            let mut matrix = DenseMatrix::zeros(8 * n, 8 * n);
            for i in 0..8 {
                for j in 0..=i {
                    for q in 0..n {
                        for p in 0..n {
                            let identity = if q == p { 1.0 } else { 0.0 };
                            let negative_j = d * identity - k / n as f64;
                            matrix[(i * n + q, j * n + p)] = if i == j {
                                identity + hg * negative_j
                            } else {
                                hg * c.a[(i, j)] * negative_j
                                    - c.gamma * c.c_matrix[(i, j)] * identity
                            };
                        }
                    }
                }
            }
            let lu = LuFactorization::new(&matrix).unwrap();
            let mut output_row = vec![0.0; 8 * n];
            for column in 0..8 * n {
                let mut r = vec![0.0; 8 * n];
                r[column] = 1.0;
                let stages = lu.solve(&r).unwrap();
                output_row[column] = (0..8).map(|i| c.b_code[i] * stages[i * n]).sum();
            }
            let independent_norm: f64 = output_row.iter().map(|x| x.abs()).sum();
            let scale = 1.0 + independent_norm;
            assert!((budget.output_amplification - independent_norm).abs() < 2e-12 * scale);
            // A sign-aligned residual realizes the row sum; this checks sharpness,
            // not merely whether a loose upper bound covers one benign sample.
            let extremizer: Vec<_> = output_row
                .iter()
                .map(|x| if *x < 0.0 { -1.0 } else { 1.0 })
                .collect();
            let stages = lu.solve(&extremizer).unwrap();
            let attained: f64 = (0..8).map(|i| c.b_code[i] * stages[i * n]).sum();
            assert!((attained - independent_norm).abs() < 2e-12 * scale);
            let transported = budget.global_residual_amplification * budget.residual_limit;
            assert!((transported - 1e-8).abs() <= 4.0 * f64::EPSILON * 1e-8);
            assert!(
                budget.global_residual_amplification
                    <= budget.output_amplification
                        * budget.propagation_sum
                        * (1.0 + 128.0 * f64::EPSILON)
            );
            if d == 0.0 {
                assert_eq!(budget.step_amplification, 1.0);
                assert_eq!(budget.propagation_sum, 64.0);
            }
        }
    }
    let p = MeanFieldAffine::new(1.0, 0.0, vec![0.0])
        .unwrap()
        .plan(0.1)
        .unwrap();
    assert!(p.algebraic_residual_budget(0, 64, 1e-8).is_err());
    assert!(p.algebraic_residual_budget(1, 0, 1e-8).is_err());
    assert!(p.algebraic_residual_budget(1, 64, 0.0).is_err());
}
