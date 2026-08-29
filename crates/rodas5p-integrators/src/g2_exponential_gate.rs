use std::sync::Arc;

use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, WorkCounters, dense_phi_action, safe_l2,
};
use serde::{Deserialize, Serialize};

use crate::{
    ExponentialKrylovConfig, OdeProblem, ParallelExecution, exprb2_step, exprb43_step,
    krylov_phi_action, pexprb54s4_step, pexprb54s4_tableau,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum G2ExponentialGateProfile {
    Smoke,
    Canonical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhiCoefficientTerm {
    pub phi_index: usize,
    pub coefficient: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4WeightSet {
    pub b2: Vec<PhiCoefficientTerm>,
    pub b3: Vec<PhiCoefficientTerm>,
    pub b4: Vec<PhiCoefficientTerm>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4CoefficientRecord {
    pub c2: f64,
    pub c3: f64,
    pub c4: f64,
    pub a32_phi3_at_c3_z: f64,
    pub a42_phi3_at_c4_z: f64,
    pub a43: f64,
    pub main: Pexprb54s4WeightSet,
    pub embedded: Pexprb54s4WeightSet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialCoefficientAuthority {
    pub method: String,
    pub publication: String,
    pub doi: String,
    pub order: usize,
    pub embedded_order: usize,
    pub stages: usize,
    pub logical_critical_depth: usize,
    pub coefficients: Pexprb54s4CoefficientRecord,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialOrderConditionRow {
    pub z: f64,
    pub main_condition_1: f64,
    pub main_condition_2: f64,
    pub weakened_main_condition_3_at_zero: f64,
    pub psi3_stage_3: f64,
    pub psi3_stage_4: f64,
    pub embedded_condition_1: f64,
    pub embedded_condition_2: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhiOracleRow {
    pub case_id: String,
    pub phi_index: usize,
    pub scale: f64,
    pub relative_error: f64,
    pub dense_value: Vec<f64>,
    pub krylov_value: Vec<f64>,
    pub krylov_dimension: usize,
    pub converged: bool,
    pub jvp_vectors: u64,
    pub projected_exponentials: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialOrderRow {
    pub method: String,
    pub h: f64,
    pub endpoint_error: f64,
    pub observed_order_to_next: Option<f64>,
    pub rhs_evaluations: u64,
    pub jvp_vectors: u64,
    pub phi_actions: u64,
    pub explicit_jacobian_builds: u64,
    pub direct_factorizations: u64,
    pub nonlinear_iterations: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StiffLinearExponentialRow {
    pub method: String,
    pub z: f64,
    pub numerical: f64,
    pub exact: f64,
    pub absolute_error: f64,
    pub explicit_jacobian_builds: u64,
    pub nonlinear_iterations: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OscillatoryExponentialRow {
    pub method: String,
    pub h: f64,
    pub endpoint_error: f64,
    pub amplitude_error: f64,
    pub phase_error_radians: f64,
    pub accepted_steps: usize,
    pub rhs_evaluations: u64,
    pub jvp_vectors: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G2ExponentialGateSummary {
    pub coefficient_rows: usize,
    pub phi_oracle_rows: usize,
    pub order_rows: usize,
    pub oscillatory_rows: usize,
    pub stiff_linear_rows: usize,
    pub maximum_stiff_linear_error: f64,
    pub maximum_order_condition_residual: f64,
    pub maximum_phi_relative_error: f64,
    pub observed_exprb2_order: f64,
    pub observed_exprb43_order: f64,
    pub observed_pexprb54s4_order: f64,
    pub explicit_jacobian_builds: u64,
    pub direct_factorizations: u64,
    pub nonlinear_iterations: u64,
    pub false_convergence: usize,
    pub coefficient_gate_pass: bool,
    pub phi_oracle_gate_pass: bool,
    pub order_gate_pass: bool,
    pub structural_jf_newton_free_gate_pass: bool,
    pub g2_foundation_pass: bool,
    pub performance_promotion_authorized: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G2ExponentialGateReport {
    pub schema: String,
    pub profile: String,
    pub scope: String,
    pub coefficient_authority: ExponentialCoefficientAuthority,
    pub order_conditions: Vec<ExponentialOrderConditionRow>,
    pub phi_oracle: Vec<PhiOracleRow>,
    pub order_screen: Vec<ExponentialOrderRow>,
    pub stiff_linear_screen: Vec<StiffLinearExponentialRow>,
    pub oscillatory_screen: Vec<OscillatoryExponentialRow>,
    pub summary: G2ExponentialGateSummary,
    pub limitations: Vec<String>,
}

fn scalar_phi(z: f64, index: usize) -> CoreResult<f64> {
    let matrix = DenseMatrix::new(1, 1, vec![z])?;
    Ok(dense_phi_action(&matrix, 1.0, index, &[1.0])?[0])
}

fn coefficient_authority() -> ExponentialCoefficientAuthority {
    let t = pexprb54s4_tableau();
    ExponentialCoefficientAuthority {
        method: "pexprb54s4".into(),
        publication: "V. T. Luan and A. Ostermann, Parallel exponential Rosenbrock methods, Computers & Mathematics with Applications 71 (2016) 1137-1150".into(),
        doi: "10.1016/j.camwa.2016.01.020".into(),
        order: 5,
        embedded_order: 4,
        stages: 4,
        logical_critical_depth: 3,
        coefficients: Pexprb54s4CoefficientRecord {
            c2: t.c2,
            c3: t.c3,
            c4: t.c4,
            a32_phi3_at_c3_z: t.a32_phi3,
            a42_phi3_at_c4_z: t.a42_phi3,
            a43: t.a43,
            main: Pexprb54s4WeightSet {
                b2: Vec::new(),
                b3: vec![
                    PhiCoefficientTerm { phi_index: 3, coefficient: t.b3_phi3 },
                    PhiCoefficientTerm { phi_index: 4, coefficient: t.b3_phi4 },
                ],
                b4: vec![
                    PhiCoefficientTerm { phi_index: 3, coefficient: t.b4_phi3 },
                    PhiCoefficientTerm { phi_index: 4, coefficient: t.b4_phi4 },
                ],
            },
            embedded: Pexprb54s4WeightSet {
                b2: vec![
                    PhiCoefficientTerm { phi_index: 3, coefficient: t.embedded_b2_phi3 },
                    PhiCoefficientTerm { phi_index: 4, coefficient: t.embedded_b2_phi4 },
                ],
                b3: vec![
                    PhiCoefficientTerm { phi_index: 3, coefficient: t.embedded_b3_phi3 },
                    PhiCoefficientTerm { phi_index: 4, coefficient: t.embedded_b3_phi4 },
                ],
                b4: vec![PhiCoefficientTerm {
                    phi_index: 4,
                    coefficient: t.embedded_b4_phi4,
                }],
            },
        },
    }
}

fn order_condition_rows(
    profile: G2ExponentialGateProfile,
) -> CoreResult<Vec<ExponentialOrderConditionRow>> {
    let nodes: &[f64] = match profile {
        G2ExponentialGateProfile::Smoke => &[-1.0, 0.0, 0.5],
        G2ExponentialGateProfile::Canonical => &[-5.0, -1.0, -0.1, 0.0, 0.5, 2.0],
    };
    let t = pexprb54s4_tableau();
    let phi3_zero = scalar_phi(0.0, 3)?;
    let phi4_zero = scalar_phi(0.0, 4)?;
    let b3_zero = t.b3_phi3 * phi3_zero + t.b3_phi4 * phi4_zero;
    let b4_zero = t.b4_phi3 * phi3_zero + t.b4_phi4 * phi4_zero;
    let weak3 = b3_zero * t.c3.powi(4) + b4_zero * t.c4.powi(4) - 0.2;
    let mut rows = Vec::new();
    for &z in nodes {
        let phi3 = scalar_phi(z, 3)?;
        let phi4 = scalar_phi(z, 4)?;
        let b3 = t.b3_phi3 * phi3 + t.b3_phi4 * phi4;
        let b4 = t.b4_phi3 * phi3 + t.b4_phi4 * phi4;
        let main1 = b3 * t.c3.powi(2) + b4 * t.c4.powi(2) - 2.0 * phi3;
        let main2 = b3 * t.c3.powi(3) + b4 * t.c4.powi(3) - 6.0 * phi4;

        let phi3_c3 = scalar_phi(t.c3 * z, 3)?;
        let phi3_c4 = scalar_phi(t.c4 * z, 3)?;
        let psi3_stage_3 = t.a32_phi3 * phi3_c3 * t.c2.powi(2) / 2.0 - t.c3.powi(3) * phi3_c3;
        let psi3_stage_4 = t.a42_phi3 * phi3_c4 * t.c2.powi(2) / 2.0 + t.a43 * t.c3.powi(2) / 2.0
            - t.c4.powi(3) * phi3_c4;

        let embedded_b2 = t.embedded_b2_phi3 * phi3 + t.embedded_b2_phi4 * phi4;
        let embedded_b3 = t.embedded_b3_phi3 * phi3 + t.embedded_b3_phi4 * phi4;
        let embedded_b4 = t.embedded_b4_phi3 * phi3 + t.embedded_b4_phi4 * phi4;
        let embedded1 =
            embedded_b2 * t.c2.powi(2) + embedded_b3 * t.c3.powi(2) + embedded_b4 * t.c4.powi(2)
                - 2.0 * phi3;
        let embedded2 =
            embedded_b2 * t.c2.powi(3) + embedded_b3 * t.c3.powi(3) + embedded_b4 * t.c4.powi(3)
                - 6.0 * phi4;
        rows.push(ExponentialOrderConditionRow {
            z,
            main_condition_1: main1,
            main_condition_2: main2,
            weakened_main_condition_3_at_zero: weak3,
            psi3_stage_3,
            psi3_stage_4,
            embedded_condition_1: embedded1,
            embedded_condition_2: embedded2,
        });
    }
    Ok(rows)
}

fn phi_oracle_rows(profile: G2ExponentialGateProfile) -> CoreResult<Vec<PhiOracleRow>> {
    let cases = vec![
        (
            "nonnormal-upper-chain",
            DenseMatrix::from_vec_rows(vec![
                vec![-5.0, 8.0, 0.0, 0.0],
                vec![0.0, -6.0, 7.0, 0.0],
                vec![0.0, 0.0, -7.0, 6.0],
                vec![0.0, 0.0, 0.0, -8.0],
            ])?,
            vec![1.0, -0.25, 0.5, -0.75],
        ),
        (
            "oscillatory-damped-block",
            DenseMatrix::from_vec_rows(vec![vec![-3.0, -12.0], vec![12.0, -3.0]])?,
            vec![1.0, 0.25],
        ),
    ];
    let scales: &[f64] = match profile {
        G2ExponentialGateProfile::Smoke => &[0.1, 0.7],
        G2ExponentialGateProfile::Canonical => &[0.05, 0.2, 0.7, 1.5],
    };
    let mut rows = Vec::new();
    for (case_id, matrix, vector) in cases {
        let operator = Arc::new(DenseOperator::new(matrix.clone())?);
        for &scale in scales {
            for phi_index in 1..=5 {
                let dense = dense_phi_action(&matrix, scale, phi_index, &vector)?;
                let mut counters = WorkCounters::default();
                let report = krylov_phi_action(
                    operator.clone(),
                    scale,
                    phi_index,
                    &vector,
                    ExponentialKrylovConfig {
                        minimum_dimension: 1,
                        maximum_dimension: matrix.nrows(),
                        dimension_increment: 1,
                        relative_tolerance: 1.0e-13,
                        absolute_tolerance: 1.0e-15,
                        reorthogonalize: true,
                    },
                    &mut counters,
                )?;
                let defect: Vec<f64> = report
                    .value
                    .iter()
                    .zip(&dense)
                    .map(|(a, b)| a - b)
                    .collect();
                rows.push(PhiOracleRow {
                    case_id: case_id.into(),
                    phi_index,
                    scale,
                    relative_error: safe_l2(&defect) / safe_l2(&dense).max(1.0e-300),
                    dense_value: dense,
                    krylov_value: report.value.clone(),
                    krylov_dimension: report.krylov_dimension,
                    converged: report.converged,
                    jvp_vectors: counters.jvp_vectors,
                    projected_exponentials: counters.phi_projected_exponentials,
                });
            }
        }
    }
    Ok(rows)
}

fn square_problem() -> CoreResult<OdeProblem> {
    OdeProblem::new(
        "g2-scalar-square",
        1,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = 2.0 * y[0] * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        Some(Arc::new(|t| vec![1.0 / (1.0 - t)])),
    )
}

fn krylov_config(n: usize) -> ExponentialKrylovConfig {
    ExponentialKrylovConfig {
        minimum_dimension: 1,
        maximum_dimension: n.clamp(1, 24),
        dimension_increment: 1,
        relative_tolerance: 1.0e-13,
        absolute_tolerance: 1.0e-15,
        reorthogonalize: true,
    }
}

fn integrate_square(method: &str, h: f64) -> CoreResult<(f64, WorkCounters)> {
    let problem = square_problem()?;
    let final_time = 0.25;
    let steps = (final_time / h).round() as usize;
    let execution = ParallelExecution::sequential();
    let mut y = vec![1.0];
    let mut t = 0.0;
    let mut work = WorkCounters::default();
    for _ in 0..steps {
        let report = match method {
            "exprb2" => exprb2_step(&problem, t, &y, h, krylov_config(1))?,
            "exprb43" => exprb43_step(&problem, t, &y, h, krylov_config(1))?,
            "pexprb54s4" => pexprb54s4_step(&problem, t, &y, h, krylov_config(1), &execution)?,
            _ => unreachable!(),
        };
        y = report.y_new;
        work.accumulate(report.work);
        t += h;
    }
    Ok(((y[0] - 1.0 / (1.0 - final_time)).abs(), work))
}

fn order_screen(profile: G2ExponentialGateProfile) -> CoreResult<Vec<ExponentialOrderRow>> {
    let h_values: &[f64] = match profile {
        G2ExponentialGateProfile::Smoke => &[0.025, 0.0125],
        G2ExponentialGateProfile::Canonical => &[0.05, 0.025, 0.0125, 0.00625],
    };
    let mut rows = Vec::new();
    for method in ["exprb2", "exprb43", "pexprb54s4"] {
        let mut method_rows = Vec::new();
        for &h in h_values {
            let (error, work) = integrate_square(method, h)?;
            method_rows.push((h, error, work));
        }
        for index in 0..method_rows.len() {
            let (h, error, work) = method_rows[index];
            let observed_order_to_next = (index + 1 < method_rows.len()).then(|| {
                let fine_error = method_rows[index + 1].1;
                (error / fine_error).log2()
            });
            rows.push(ExponentialOrderRow {
                method: method.into(),
                h,
                endpoint_error: error,
                observed_order_to_next,
                rhs_evaluations: work.rhs_evaluations,
                jvp_vectors: work.jvp_vectors,
                phi_actions: work.phi_actions,
                explicit_jacobian_builds: work.jacobian_builds,
                direct_factorizations: work.direct_factorizations,
                nonlinear_iterations: work.nonlinear_iterations,
            });
        }
    }
    Ok(rows)
}

fn scalar_linear_problem(rate: f64) -> CoreResult<OdeProblem> {
    OdeProblem::new(
        format!("g2-scalar-linear-{rate}"),
        1,
        Arc::new(move |_, y: &[f64], out: &mut [f64]| {
            out[0] = rate * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            move |_, _y: &[f64], v: &[f64], out: &mut [f64]| {
                out[0] = rate * v[0];
                Ok(())
            },
        )),
        None,
        true,
        None,
        Some(Arc::new(move |t| vec![(rate * t).exp()])),
    )
}

fn stiff_linear_screen(
    profile: G2ExponentialGateProfile,
) -> CoreResult<Vec<StiffLinearExponentialRow>> {
    let z_values: &[f64] = match profile {
        G2ExponentialGateProfile::Smoke => &[-1.0, -20.0],
        G2ExponentialGateProfile::Canonical => &[-1.0, -20.0, -100.0, -500.0],
    };
    let execution = ParallelExecution::sequential();
    let mut rows = Vec::new();
    for method in ["exprb2", "exprb43", "pexprb54s4"] {
        for &z in z_values {
            let problem = scalar_linear_problem(z)?;
            let report = match method {
                "exprb2" => exprb2_step(&problem, 0.0, &[1.0], 1.0, krylov_config(1))?,
                "exprb43" => exprb43_step(&problem, 0.0, &[1.0], 1.0, krylov_config(1))?,
                "pexprb54s4" => {
                    pexprb54s4_step(&problem, 0.0, &[1.0], 1.0, krylov_config(1), &execution)?
                }
                _ => unreachable!(),
            };
            let exact = z.exp();
            rows.push(StiffLinearExponentialRow {
                method: method.into(),
                z,
                numerical: report.y_new[0],
                exact,
                absolute_error: (report.y_new[0] - exact).abs(),
                explicit_jacobian_builds: report.work.jacobian_builds,
                nonlinear_iterations: report.work.nonlinear_iterations,
            });
        }
    }
    Ok(rows)
}

fn oscillatory_problem() -> CoreResult<OdeProblem> {
    let sigma: f64 = 3.0;
    let omega: f64 = 12.0;
    OdeProblem::new(
        "g2-complex-dahlquist",
        2,
        Arc::new(move |_, y: &[f64], out: &mut [f64]| {
            out[0] = -sigma * y[0] - omega * y[1];
            out[1] = omega * y[0] - sigma * y[1];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            move |_, _y: &[f64], v: &[f64], out: &mut [f64]| {
                out[0] = -sigma * v[0] - omega * v[1];
                out[1] = omega * v[0] - sigma * v[1];
                Ok(())
            },
        )),
        None,
        true,
        None,
        Some(Arc::new(move |t| {
            let amplitude = (-sigma * t).exp();
            vec![amplitude * (omega * t).cos(), amplitude * (omega * t).sin()]
        })),
    )
}

fn oscillatory_screen(
    profile: G2ExponentialGateProfile,
) -> CoreResult<Vec<OscillatoryExponentialRow>> {
    let h_values: &[f64] = match profile {
        G2ExponentialGateProfile::Smoke => &[0.05],
        G2ExponentialGateProfile::Canonical => &[0.1, 0.05, 0.025],
    };
    let final_time = 0.3;
    let sigma: f64 = 3.0;
    let omega: f64 = 12.0;
    let exact_amplitude = (-sigma * final_time).exp();
    let exact_phase = omega * final_time;
    let problem = oscillatory_problem()?;
    let execution = ParallelExecution::rayon(2)?;
    let mut rows = Vec::new();
    for method in ["exprb2", "exprb43", "pexprb54s4"] {
        for &h in h_values {
            let steps = (final_time / h).round() as usize;
            let actual_h = final_time / steps as f64;
            let mut y = vec![1.0, 0.0];
            let mut t = 0.0;
            let mut work = WorkCounters::default();
            for _ in 0..steps {
                let report = match method {
                    "exprb2" => exprb2_step(&problem, t, &y, actual_h, krylov_config(2))?,
                    "exprb43" => exprb43_step(&problem, t, &y, actual_h, krylov_config(2))?,
                    "pexprb54s4" => {
                        pexprb54s4_step(&problem, t, &y, actual_h, krylov_config(2), &execution)?
                    }
                    _ => unreachable!(),
                };
                y = report.y_new;
                work.accumulate(report.work);
                t += actual_h;
            }
            let exact = vec![
                exact_amplitude * exact_phase.cos(),
                exact_amplitude * exact_phase.sin(),
            ];
            let defect: Vec<f64> = y.iter().zip(&exact).map(|(a, b)| a - b).collect();
            let amplitude = safe_l2(&y);
            let phase = y[1].atan2(y[0]);
            let phase_error = (phase - exact_phase)
                .sin()
                .atan2((phase - exact_phase).cos())
                .abs();
            rows.push(OscillatoryExponentialRow {
                method: method.into(),
                h: actual_h,
                endpoint_error: safe_l2(&defect),
                amplitude_error: (amplitude - exact_amplitude).abs(),
                phase_error_radians: phase_error,
                accepted_steps: steps,
                rhs_evaluations: work.rhs_evaluations,
                jvp_vectors: work.jvp_vectors,
            });
        }
    }
    Ok(rows)
}

fn minimum_observed_order(rows: &[ExponentialOrderRow], method: &str) -> f64 {
    rows.iter()
        .filter(|row| row.method == method)
        .filter_map(|row| row.observed_order_to_next)
        .fold(f64::INFINITY, f64::min)
}

pub fn run_g2_exponential_gate(
    profile: G2ExponentialGateProfile,
) -> CoreResult<G2ExponentialGateReport> {
    let order_conditions = order_condition_rows(profile)?;
    let phi_oracle = phi_oracle_rows(profile)?;
    let order_screen = order_screen(profile)?;
    let stiff_linear_screen = stiff_linear_screen(profile)?;
    let oscillatory_screen = oscillatory_screen(profile)?;
    let maximum_order_condition_residual = order_conditions
        .iter()
        .flat_map(|row| {
            [
                row.main_condition_1,
                row.main_condition_2,
                row.weakened_main_condition_3_at_zero,
                row.psi3_stage_3,
                row.psi3_stage_4,
                row.embedded_condition_1,
                row.embedded_condition_2,
            ]
        })
        .map(f64::abs)
        .fold(0.0, f64::max);
    let maximum_stiff_linear_error = stiff_linear_screen
        .iter()
        .map(|row| row.absolute_error)
        .fold(0.0, f64::max);
    let maximum_phi_relative_error = phi_oracle
        .iter()
        .map(|row| row.relative_error)
        .fold(0.0, f64::max);
    let observed_exprb2_order = minimum_observed_order(&order_screen, "exprb2");
    let observed_exprb43_order = minimum_observed_order(&order_screen, "exprb43");
    let observed_pexprb54s4_order = minimum_observed_order(&order_screen, "pexprb54s4");
    let explicit_jacobian_builds = order_screen
        .iter()
        .map(|row| row.explicit_jacobian_builds)
        .sum();
    let direct_factorizations = order_screen
        .iter()
        .map(|row| row.direct_factorizations)
        .sum();
    let nonlinear_iterations = order_screen
        .iter()
        .map(|row| row.nonlinear_iterations)
        .sum();
    let false_convergence = phi_oracle.iter().filter(|row| !row.converged).count();
    let coefficient_gate_pass = maximum_order_condition_residual <= 2.0e-12;
    let phi_oracle_gate_pass = maximum_phi_relative_error <= 2.0e-11 && false_convergence == 0;
    let order_gate_pass = observed_exprb2_order >= 1.8
        && observed_exprb43_order >= 3.7
        && observed_pexprb54s4_order >= 4.7;
    let structural_gate =
        explicit_jacobian_builds == 0 && direct_factorizations == 0 && nonlinear_iterations == 0;
    Ok(G2ExponentialGateReport {
        schema: "generic-g2-exponential-foundation-v1".into(),
        profile: match profile {
            G2ExponentialGateProfile::Smoke => "smoke",
            G2ExponentialGateProfile::Canonical => "canonical",
        }
        .into(),
        scope: "autonomous regular ODEs with identity mass; counted full-Arnoldi matrix-free phi-action reference".into(),
        coefficient_authority: coefficient_authority(),
        summary: G2ExponentialGateSummary {
            coefficient_rows: order_conditions.len(),
            phi_oracle_rows: phi_oracle.len(),
            order_rows: order_screen.len(),
            oscillatory_rows: oscillatory_screen.len(),
            stiff_linear_rows: stiff_linear_screen.len(),
            maximum_stiff_linear_error,
            maximum_order_condition_residual,
            maximum_phi_relative_error,
            observed_exprb2_order,
            observed_exprb43_order,
            observed_pexprb54s4_order,
            explicit_jacobian_builds,
            direct_factorizations,
            nonlinear_iterations,
            false_convergence,
            coefficient_gate_pass,
            phi_oracle_gate_pass,
            order_gate_pass,
            structural_jf_newton_free_gate_pass: structural_gate,
            g2_foundation_pass: coefficient_gate_pass
                && phi_oracle_gate_pass
                && order_gate_pass
                && maximum_stiff_linear_error <= 5.0e-13
                && structural_gate,
            performance_promotion_authorized: false,
        },
        order_conditions,
        phi_oracle,
        order_screen,
        stiff_linear_screen,
        oscillatory_screen,
        limitations: vec![
            "The matrix-free phi engine is a full-Arnoldi correctness reference, not KIOPS: incomplete orthogonalization, adaptive substepping, restart and fused phi combinations are not implemented.".into(),
            "Nonautonomous augmentation and nonsingular mass-matrix exponential treatment are not implemented in this bounded node.".into(),
            "The declared pexprb54s4 critical depth of three is a dependency-graph property; separate unfused phi actions currently dominate work, so no wall-time promotion is authorized.".into(),
            "BDF2 remains a legacy context comparator; Radau IIA3 uses the authorized Cell-G frozen-Jacobian embedded-estimator baseline, and physical clients are excluded from this gate.".into(),
        ],
    })
}
