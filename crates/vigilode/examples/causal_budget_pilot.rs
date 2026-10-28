//! Bounded derivative-free causal-stage study on y'=-y^3, y(0)=1.
//! Actual exported DOPRI5 coefficients; native sequential workspace is an
//! independently executed stage oracle, and the ODE endpoint is analytic.
//! Candidate stage rounds are ordered serial emulation of parallel Jacobi.
//! Reported parallel depth is theoretical; no parallel speedup is measured.
use rodas5p_core::{
    CoreResult, WorkCounters,
    directed::{Interval, add_up, mul_up},
};
use rodas5p_integrators::{DOPRI5_A, DOPRI5_B, DOPRI5_B_EMBEDDED, Dopri5Workspace};
use serde_json::json;
use vigilode::{CausalBudgetInput, CausalTargetKey, SessionEpochs, assess_causal_budget};

const S: usize = 7;
const STEPS: usize = 256;
const H: f64 = 1.0 / STEPS as f64;
const INNER: f64 = 1e-9;
const OUTER: f64 = 1e-8;

fn rhs(_: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
    out[0] = -y[0] * y[0] * y[0];
    Ok(())
}
fn point(x: f64) -> Interval {
    Interval::point(x).unwrap()
}
fn exact_rhs(x: Interval) -> Interval {
    -x.mul(x).unwrap().mul(x).unwrap()
}
fn weighted(y: f64, u: &[f64; S], w: &[f64; S]) -> (f64, f64) {
    let mut ordinary = y;
    let mut exact = point(y);
    for i in 0..S {
        ordinary += w[i] * u[i];
        exact = exact.add(point(w[i]).mul(point(u[i])).unwrap()).unwrap();
    }
    (ordinary, point(ordinary).sub(exact).unwrap().mag())
}
fn run(blanket: bool) -> serde_json::Value {
    let mut propagation = [0.; S * S];
    let mut weights = [0.; S];
    let mut ew = [0.; S];
    for i in 0..S {
        ew[i] = DOPRI5_B[i] - DOPRI5_B_EMBEDDED[i];
        let exact_error_weight = point(DOPRI5_B[i]).sub(point(DOPRI5_B_EMBEDDED[i])).unwrap();
        // Reserve both principal-output perturbation and correction of the
        // embedded estimate; max would only bound each channel separately.
        weights[i] = add_up(DOPRI5_B[i].abs(), exact_error_weight.mag()).unwrap();
        for j in 0..i {
            propagation[i * S + j] = mul_up(mul_up(H, 12.).unwrap(), DOPRI5_A[i][j].abs()).unwrap();
        }
    }
    // On |x|<=2, |f|<=8 and Lip(f)<=12. Every exact causal stage stays
    // inside that domain if |y| +8h*max_i sum_j |a_ij|<=2, by induction.
    let rowsum = DOPRI5_A
        .iter()
        .map(|r| r.iter().fold(0., |sum, a| add_up(sum, a.abs()).unwrap()))
        .fold(0., f64::max);
    let mut y: f64 = 1.;
    let mut oracle = Dopri5Workspace::new(1).unwrap();
    let mut oracle_work = WorkCounters::default();
    let mut total_rhs = 0usize;
    let mut verifier_rhs = 0usize;
    let mut rounds = 0usize;
    let mut maximum_rounds = 0usize;
    let mut maximum_inner_bound: f64 = 0.;
    let mut maximum_stage_output_error: f64 = 0.;
    let mut maximum_embedded_plus_bound: f64 = 0.;
    let mut fallback_steps = 0;
    let mut per_step = vec![];
    for step in 0..STEPS {
        assert!(add_up(y.abs(), mul_up(mul_up(8., H).unwrap(), rowsum).unwrap()).unwrap() < 2.);
        let t = step as f64 * H;
        let mut reference = [0.];
        let mut reference_error = [0.];
        oracle
            .step_into(
                &rhs,
                t,
                &[y],
                H,
                &mut reference,
                &mut reference_error,
                &mut oracle_work,
            )
            .unwrap();
        // lambda=0 uncoupled predictor; only lambda=1 residuals can admit.
        let first = -H * y * y * y;
        let mut u = [first; S];
        total_rhs += 1;
        let key = CausalTargetKey::new(SessionEpochs::default(), 0, 0, step as u64, t, H).unwrap();
        let mut accepted = None;
        let mut local_rhs = 1;
        for iteration in 0..=S {
            let mut target = [0.; S];
            let mut residual = [0.; S];
            for i in 0..S {
                let mut x = y;
                let mut exact = point(y);
                for j in 0..i {
                    x += DOPRI5_A[i][j] * u[j];
                    exact = exact
                        .add(point(DOPRI5_A[i][j]).mul(point(u[j])).unwrap())
                        .unwrap();
                }
                assert!(exact.lo >= -2. && exact.hi <= 2.);
                if i == 0 {
                    target[i] = first;
                } else {
                    target[i] = -H * x * x * x;
                    total_rhs += 1;
                    local_rhs += 1;
                }
                // Independent outward original-target residual including all
                // stage reconstruction and cube roundoff. This work is charged.
                let true_target = exact_rhs(exact).scale(H).unwrap();
                residual[i] = point(u[i]).sub(true_target).unwrap().mag();
                verifier_rhs += 1;
            }
            let (endpoint, reconstruction) = weighted(y, &u, &DOPRI5_B);
            let (embedded, _) = weighted(0., &u, &ew);
            // Enclose the exact difference of principal and embedded tableau
            // coefficients as well as the floating weighted summation.
            let embedded_interval = (0..S).fold(point(0.), |acc, i| {
                acc.add(
                    point(DOPRI5_B[i])
                        .sub(point(DOPRI5_B_EMBEDDED[i]))
                        .unwrap()
                        .mul(point(u[i]))
                        .unwrap(),
                )
                .unwrap()
            });
            let embedded_roundoff = point(embedded).sub(embedded_interval).unwrap().mag();
            let i = CausalBudgetInput {
                expected: key,
                observed: key,
                lambda: 1.,
                propagation: &propagation,
                residual_seeds: &residual,
                output_weights: &weights,
                reconstruction_error: add_up(reconstruction, embedded_roundoff).unwrap(),
                output_budget: INNER,
            };
            let receipt = assess_causal_budget(&i, &mut [0.; S]).unwrap();
            let error_measure = add_up(embedded.abs(), receipt.output_error_upper).unwrap();
            let stop = if blanket {
                residual.iter().copied().fold(0., f64::max) <= 1e-14
            } else {
                receipt.admitted
            };
            if stop && receipt.admitted && error_measure <= OUTER {
                let actual = (endpoint - reference[0]).abs();
                // Native oracle itself rounds. A small explicit allowance is
                // only used in this diagnostic comparison, never in admission.
                assert!(actual <= receipt.output_error_upper + 32. * f64::EPSILON);
                maximum_stage_output_error = maximum_stage_output_error.max(actual);
                maximum_inner_bound = maximum_inner_bound.max(receipt.output_error_upper);
                maximum_embedded_plus_bound = maximum_embedded_plus_bound.max(error_measure);
                rounds += iteration;
                maximum_rounds = maximum_rounds.max(iteration);
                accepted = Some(endpoint);
                per_step.push(json!({"step":step,"correction_rounds":iteration,"candidate_rhs":local_rhs,
                    "output_bound":receipt.output_error_upper,"actual_vs_native_stage_output":actual,
                    "embedded_plus_bound":error_measure}));
                break;
            }
            u = target;
        }
        y = match accepted {
            Some(v) => v,
            None => {
                fallback_steps += 1;
                reference[0]
            }
        };
    }
    let exact = 1. / 3f64.sqrt();
    json!({"mode":if blanket {"blanket-original-residual-1e-14"} else {"physical-budget-original-residual"},
        "equation":"y'=-y^3; y(0)=1; t in [0,1]", "steps":STEPS,"h":H,
        "inner_output_budget":INNER,"embedded_plus_inner_limit":OUTER,
        "candidate_rhs":total_rhs,"interval_verifier_rhs":verifier_rhs,
        "combined_rhs_and_verifier_evaluations":total_rhs+verifier_rhs,
        "correction_rounds":rounds,"maximum_correction_rounds":maximum_rounds,
        "maximum_output_error_upper":maximum_inner_bound,"maximum_actual_vs_native_step":maximum_stage_output_error,
        "maximum_embedded_plus_bound":maximum_embedded_plus_bound,
        "endpoint":y,"analytic_endpoint":exact,"endpoint_absolute_error":(y-exact).abs(),
        "fallback_steps":fallback_steps,"sequential_native_oracle_rhs":oracle_work.rhs_calls,
        "parallel_execution":"serial emulation; rounds describe candidate critical depth only",
        "comparison_scope":"stage root work at same fixed mesh; not wall-time or global-error certificate",
        "steps_detail":per_step})
}

fn main() {
    let selected = run(false);
    let blanket = run(true);
    assert!(selected["fallback_steps"].as_u64().unwrap() == 0);
    assert!(blanket["fallback_steps"].as_u64().unwrap() == 0);
    assert!(selected["endpoint_absolute_error"].as_f64().unwrap() < 1e-6);
    assert!(
        selected["combined_rhs_and_verifier_evaluations"].as_u64()
            < blanket["combined_rhs_and_verifier_evaluations"].as_u64()
    );
    println!(
        "{}",
        json!({"schema":"vigilode.causal-budget-pilot.v1","selected":selected,"blanket":blanket,
        "notes":["The simple scalar workload should select sequential RK: bounded parallel correction saves work relative to blanket root polishing, not relative to seven sequential stages.",
            "The output admission actively stops correction; no Jacobian/JVP/AD/Newton callback is used.",
            "The same nilpotent majorant extends to Rosenbrock only with valid inverse-diagonal and original-target Lipschitz bounds."]})
    );
}
