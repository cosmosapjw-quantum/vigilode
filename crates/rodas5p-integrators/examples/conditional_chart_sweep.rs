//! Bounded structural accuracy/cost study, not a whole-ODE solver benchmark.
//! All operator entries are dyadic and all final residuals are enclosed outward.
#![recursion_limit = "256"]
use rodas5p_core::directed::{Interval, add_up, div_down, div_up, mul_down, mul_up, sub_down};
use rodas5p_integrators::conditioned_polynomial::*;
use rodas5p_integrators::polynomial_resolvent::{
    PolynomialPhysicalNorm as Norm, PolynomialRefinementPolicy as Policy,
    PolynomialRefinementStop as Stop, PolynomialResolventError as Error,
};
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    hint::black_box,
    time::{Duration, Instant},
};

fn point(x: f64) -> Interval {
    Interval::point(x).unwrap()
}
fn inf(x: &[f64]) -> f64 {
    x.iter().fold(0.0_f64, |a, b| a.max(b.abs()))
}

struct Provider {
    declaration: ChartDeclaration,
    delta: f64,
    rho: f64,
    beta: f64,
    kappa: f64,
    dense: bool,
    c: Vec<f64>,
    b: Vec<f64>,
    state_cap: f64,
    capture: Cell<bool>,
    last_candidate: RefCell<Vec<f64>>,
}
impl Provider {
    fn new(case: &str, n: usize, dense: bool) -> Self {
        let (center, scale, delta, rho, q) = match case {
            "nilpotent" => (-1., 1., 0., 1., vec![0., 0., 1.]),
            "leakage" => (-1., 1., 0.125, 1., vec![0., 0., 1.]),
            "nonnormal" => (-1., 1., 1.0 / 64., 16., vec![0., 0., 1.]),
            // B=0 exactly while alpha=(1-center)/scale rounds to the C=1 root.
            "degenerate" => (-2.0_f64.powi(60), 2.0_f64.powi(60), 1., 0., vec![-1., 1.]),
            _ => panic!("unknown case"),
        };
        let relation = if case == "degenerate" {
            0.
        } else {
            add_up(
                mul_up(delta, delta).unwrap(),
                mul_up(2. * delta, rho).unwrap(),
            )
            .unwrap()
        };
        let beta = center + scale * delta;
        let kappa = scale * rho;
        let mut c = vec![0.; n * n];
        let mut b = vec![0.; n * n];
        for i in 0..n {
            c[i * n + i] = delta;
            b[i * n + i] = beta;
            if i % 2 == 0 {
                c[i * n + i + 1] = rho;
                b[i * n + i + 1] = kappa;
            }
        }
        Self {
            declaration: ChartDeclaration::new(
                ChartOperatorIdentity {
                    operator_key: 104,
                    dimension: n,
                    model_epoch: 1,
                    provider_epoch: 1,
                    operator_scale_bits: 1.0_f64.to_bits(),
                    physical_norm: Norm::Infinity,
                },
                center,
                scale,
                &q,
                relation,
                delta + rho,
                104,
            )
            .unwrap(),
            delta,
            rho,
            beta,
            kappa,
            dense,
            c,
            b,
            state_cap: 32.,
            capture: Cell::new(false),
            last_candidate: RefCell::new(vec![f64::NAN; n]),
        }
    }
    fn action(&self, x: &[f64], out: &mut [f64], chart: bool) {
        if self.dense {
            let matrix = if chart { &self.c } else { &self.b };
            for (i, o) in out.iter_mut().enumerate() {
                let mut s = 0.;
                for j in 0..x.len() {
                    s += matrix[i * x.len() + j] * x[j];
                }
                *o = s;
            }
        } else {
            let (d, r) = if chart {
                (self.delta, self.rho)
            } else {
                (self.beta, self.kappa)
            };
            for i in (0..x.len()).step_by(2) {
                out[i] = d * x[i] + r * x[i + 1];
                out[i + 1] = d * x[i + 1];
            }
        }
    }
    fn direct(&self, rhs: &[f64], out: &mut [f64]) {
        let a = 1. - self.beta;
        for i in (0..rhs.len()).step_by(2) {
            out[i + 1] = rhs[i + 1] / a;
            out[i] = (rhs[i] + self.kappa * out[i + 1]) / a;
        }
    }
    fn inverse_upper(&self) -> f64 {
        let a = sub_down(1., self.beta).unwrap();
        add_up(
            div_up(1., a).unwrap(),
            div_up(self.kappa.abs(), mul_down(a, a).unwrap()).unwrap(),
        )
        .unwrap()
    }
    fn exact_solution(&self, rhs: &[f64]) -> Vec<Interval> {
        let a = point(1.).sub(point(self.beta)).unwrap();
        let mut x = vec![point(0.); rhs.len()];
        for i in (0..rhs.len()).step_by(2) {
            x[i + 1] = point(rhs[i + 1]).div(a).unwrap();
            x[i] = point(rhs[i])
                .add(point(self.kappa).mul(x[i + 1]).unwrap())
                .unwrap()
                .div(a)
                .unwrap();
        }
        x
    }
    fn residual_upper(&self, rhs: &[f64], x: &[f64]) -> f64 {
        let a = point(1.).sub(point(self.beta)).unwrap();
        let mut norm = 0.0_f64;
        for i in (0..rhs.len()).step_by(2) {
            let even = point(rhs[i])
                .sub(a.mul(point(x[i])).unwrap())
                .unwrap()
                .add(point(self.kappa).mul(point(x[i + 1])).unwrap())
                .unwrap();
            let odd = point(rhs[i + 1])
                .sub(a.mul(point(x[i + 1])).unwrap())
                .unwrap();
            norm = norm.max(even.mag()).max(odd.mag());
        }
        norm
    }
    fn residual_allowance(&self, rhs: &[f64]) -> f64 {
        // Standard gamma_m bound, plus a conservative gradual-underflow allowance.
        // The original-action callback refuses x outside state_cap before output.
        let m = if self.dense { 2 * rhs.len() + 6 } else { 10 };
        let mu = mul_up(m as f64, f64::EPSILON / 2.).unwrap();
        let gamma = div_up(mu, sub_down(1., mu).unwrap()).unwrap();
        let magnitude = add_up(
            inf(rhs),
            mul_up(
                add_up(1., self.beta.abs() + self.kappa.abs()).unwrap(),
                self.state_cap,
            )
            .unwrap(),
        )
        .unwrap();
        add_up(
            mul_up(gamma, magnitude).unwrap(),
            (m + 16) as f64 * f64::MIN_POSITIVE,
        )
        .unwrap()
    }
}
impl DeclaredChartOperator for Provider {
    fn declaration(&self) -> &ChartDeclaration {
        &self.declaration
    }
    fn apply_chart(&self, x: &[f64], out: &mut [f64]) -> Result<(), Error> {
        self.action(x, out, true);
        Ok(())
    }
    fn apply_original(&self, x: &[f64], out: &mut [f64]) -> Result<(), Error> {
        if inf(x) > self.state_cap {
            return Err(Error::OperatorFailure);
        }
        if self.capture.get() {
            self.last_candidate.borrow_mut().copy_from_slice(x);
        }
        self.action(x, out, false);
        Ok(())
    }
}
fn error_upper(x: &[f64], exact: &[Interval]) -> f64 {
    x.iter().zip(exact).fold(0.0_f64, |e, (x, exact)| {
        e.max(point(*x).sub(*exact).unwrap().mag())
    })
}

fn error_lower(x: &[f64], exact: &[Interval]) -> f64 {
    x.iter().zip(exact).fold(0.0_f64, |e, (x, exact)| {
        e.max(point(*x).sub(*exact).unwrap().mig())
    })
}

#[derive(Clone, Copy)]
enum Mode {
    Direct,
    NoRefinement,
    Refinement,
    Capped,
}
impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::NoRefinement => "no_refinement",
            Self::Refinement => "refinement",
            Self::Capped => "capped_refinement",
        }
    }
}
fn stop_name(stop: Stop) -> &'static str {
    match stop {
        Stop::Accepted => "accepted",
        Stop::RelationTooWeak => "relation_too_weak",
        Stop::ActionBudget => "action_budget",
        Stop::CorrectionBudget => "correction_budget",
        Stop::Stagnation => "stagnation",
        Stop::FallbackAccepted => "fallback_accepted",
        Stop::FallbackResidualRejected => "fallback_residual_rejected",
    }
}
struct Outcome {
    work: ChartWork,
    chart_accepted: bool,
    accepted: bool,
    reason: &'static str,
    original_residual: f64,
    certified_error: f64,
    last_chart_reported_residual: Option<f64>,
    last_chart_reported_error: Option<f64>,
    corrections: usize,
    fallback: bool,
    fallback_seconds: f64,
    certificate_seconds: f64,
}
struct Context<'a> {
    provider: &'a Provider,
    plan: Option<&'a ConditionedPolynomialPlan>,
    rhs: &'a [f64],
    tolerance: f64,
    residual_tolerance: Option<f64>,
    allowance: f64,
    correction_limit: usize,
    capped_passes: usize,
}
fn execute(ctx: &Context<'_>, mode: Mode, ws: &mut ChartWorkspace, out: &mut [f64]) -> Outcome {
    let mut r = Outcome {
        work: ChartWork::default(),
        chart_accepted: false,
        accepted: false,
        reason: "not_attempted",
        original_residual: 0.,
        certified_error: 0.,
        last_chart_reported_residual: None,
        last_chart_reported_error: None,
        corrections: 0,
        fallback: false,
        fallback_seconds: 0.,
        certificate_seconds: 0.,
    };
    out.fill(17.);
    if !matches!(mode, Mode::Direct) {
        if let Some(plan) = ctx.plan {
            let threshold = ctx.residual_tolerance.unwrap();
            if threshold <= ctx.allowance {
                r.reason = "residual_uncertainty_floor";
            } else if matches!(mode, Mode::Capped) && ctx.capped_passes == 0 {
                r.reason = "estimated_cost_cap";
            } else {
                let max_corrections = match mode {
                    Mode::NoRefinement => 0,
                    Mode::Capped => ctx.correction_limit.min(ctx.capped_passes - 1),
                    _ => ctx.correction_limit,
                };
                match plan.apply(
                    ctx.provider,
                    ctx.rhs,
                    out,
                    ws,
                    Policy {
                        max_corrections,
                        max_operator_actions: plan.degree() * (max_corrections + 1),
                        maximum_relation_ratio: 0.9,
                        stagnation_ratio: 0.99,
                    },
                    threshold,
                    ctx.allowance,
                    &mut r.work,
                ) {
                    Ok(report) => {
                        r.chart_accepted = report.accepted;
                        r.reason = stop_name(report.stop);
                        r.last_chart_reported_residual = report.original_residual_norm;
                        r.last_chart_reported_error = report.conditional_error_upper;
                        r.corrections = report.corrections;
                    }
                    Err(_) => r.reason = "chart_callback_or_arithmetic_error",
                }
                if !r.chart_accepted {
                    assert!(out.iter().all(|x| *x == 17.));
                }
            }
        } else {
            r.reason = "chart_setup_refused";
        }
    }
    if !r.chart_accepted {
        let then = Instant::now();
        ctx.provider.direct(ctx.rhs, out);
        r.fallback_seconds = then.elapsed().as_secs_f64();
        r.fallback = !matches!(mode, Mode::Direct);
        if matches!(mode, Mode::Direct) {
            r.reason = "direct";
        }
    }
    let then = Instant::now();
    r.original_residual = ctx.provider.residual_upper(ctx.rhs, out);
    r.certified_error = mul_up(ctx.provider.inverse_upper(), r.original_residual).unwrap();
    r.accepted = r.certified_error <= ctx.tolerance;
    r.certificate_seconds = then.elapsed().as_secs_f64();
    r
}
fn mean_seconds(mut f: impl FnMut(), minimum: Duration) -> f64 {
    let then = Instant::now();
    let mut count = 0;
    while count < 8 || then.elapsed() < minimum {
        f();
        count += 1;
    }
    then.elapsed().as_secs_f64() / count as f64
}

struct Options {
    n: usize,
    repeats: usize,
    cases: Vec<String>,
    actions: Vec<String>,
    tolerances: Vec<f64>,
    cap: f64,
    corrections: usize,
    min_ms: u64,
}
fn options() -> Options {
    let mut o = Options {
        n: 32,
        repeats: 3,
        cases: ["nilpotent", "leakage", "nonnormal", "degenerate"]
            .map(str::to_string)
            .to_vec(),
        actions: ["structured", "dense"].map(str::to_string).to_vec(),
        tolerances: vec![1e-5, 1e-9, 1e-13, 1e-17],
        cap: 1.5,
        corrections: 8,
        min_ms: 2,
    };
    let mut args = std::env::args().skip(1);
    while let Some(key) = args.next() {
        if key == "--help" {
            println!(
                "conditional_chart_sweep --dimension EVEN<=1024 --repeats 1..20 --case all|nilpotent|leakage|nonnormal|degenerate --action all|structured|dense --tolerances 1e-5,1e-9,1e-13,1e-17 --cost-ratio-cap 1.5 --max-corrections 8 --min-ms 2"
            );
            std::process::exit(0);
        }
        let value = args.next().expect("option requires value");
        match key.as_str() {
            "--dimension" => o.n = value.parse().unwrap(),
            "--repeats" => o.repeats = value.parse().unwrap(),
            "--case" if value != "all" => o.cases = vec![value],
            "--action" if value != "all" => o.actions = vec![value],
            "--case" | "--action" => {}
            "--tolerances" => o.tolerances = value.split(',').map(|v| v.parse().unwrap()).collect(),
            "--cost-ratio-cap" => o.cap = value.parse().unwrap(),
            "--max-corrections" => o.corrections = value.parse().unwrap(),
            "--min-ms" => o.min_ms = value.parse().unwrap(),
            _ => panic!("unknown option {key}"),
        }
    }
    assert!(o.n >= 2 && o.n <= 1024 && o.n % 2 == 0);
    assert!((1..=20).contains(&o.repeats) && (1..=50).contains(&o.min_ms));
    assert!(o.corrections <= 32 && o.cap.is_finite() && (1.0..=10.0).contains(&o.cap));
    assert!(
        !o.tolerances.is_empty()
            && o.tolerances.len() <= 12
            && o.tolerances.iter().all(|t| t.is_finite() && *t > 0.)
    );
    assert!(
        o.cases
            .iter()
            .all(|x| ["nilpotent", "leakage", "nonnormal", "degenerate"].contains(&x.as_str()))
    );
    assert!(
        o.actions
            .iter()
            .all(|x| ["structured", "dense"].contains(&x.as_str()))
    );
    o
}

fn main() {
    let options = options();
    let minimum = Duration::from_millis(options.min_ms);
    println!(
        "{}",
        json!({"schema_version":1,"kind":"header","study":"conditional_chart_accuracy_cost",
        "scope":"fixed linear-system structural kernels, no whole ODE promotion",
        "dimension":options.n,"repeats":options.repeats,"cost_ratio_cap":options.cap,
        "instrumentation":"work counts and per-route certificate/fallback Instant timers included; diagnostic candidate copy and exact-reference comparison excluded from timing",
        "uncertainty":"timing means are empirical, cost cap is a routing estimate not a verified wall-time upper bound",
        "reference":"directed binary64 interval triangular solve of exact binary B and RHS; no manufactured rounded RHS",
        "action_variants":"same mathematical block-Jordan operator; structured O(n) and materialized dense O(n^2), no synthetic burn loop"})
    );
    for case in &options.cases {
        for action in &options.actions {
            let setup_start = Instant::now();
            let provider = Provider::new(case, options.n, action == "dense");
            let mut setup_work = ChartWork::default();
            let plan_result =
                ConditionedPolynomialPlan::prepare(&provider.declaration, 1e-12, &mut setup_work);
            let setup_refusal = plan_result.as_ref().err().map(|e| format!("{e:?}"));
            let plan = plan_result.ok();
            let setup_seconds = setup_start.elapsed().as_secs_f64();
            let rhs: Vec<f64> = (0..options.n)
                .map(|i| ((i % 7) as f64 - 3.) / 8. + 0.0625)
                .collect();
            let exact = provider.exact_solution(&rhs);
            let allowance = provider.residual_allowance(&rhs);
            let mut ws = ChartWorkspace::new(options.n);
            let mut out = vec![0.; options.n];
            let mut tmp = vec![0.; options.n];
            let calibration_start = Instant::now();
            let chart_action_seconds = mean_seconds(
                || {
                    provider.apply_chart(&rhs, &mut tmp).unwrap();
                    black_box(&tmp);
                },
                minimum,
            );
            let original_action_seconds = mean_seconds(
                || {
                    provider.apply_original(&rhs, &mut tmp).unwrap();
                    black_box(&tmp);
                },
                minimum,
            );
            let base = Context {
                provider: &provider,
                plan: plan.as_ref(),
                rhs: &rhs,
                tolerance: 1e10,
                residual_tolerance: Some(1e10),
                allowance,
                correction_limit: 0,
                capped_passes: 1,
            };
            let direct_seconds = mean_seconds(
                || {
                    black_box(execute(&base, Mode::Direct, &mut ws, &mut out));
                },
                minimum,
            );
            let first_pass_seconds = if let Some(prepared) = plan.as_ref() {
                mean_seconds(
                    || {
                        let mut work = ChartWork::default();
                        black_box(
                            prepared
                                .apply(
                                    &provider,
                                    &rhs,
                                    &mut out,
                                    &mut ws,
                                    Policy {
                                        max_corrections: 0,
                                        ..Default::default()
                                    },
                                    1e10,
                                    allowance,
                                    &mut work,
                                )
                                .unwrap(),
                        );
                    },
                    minimum,
                )
            } else {
                0.
            };
            let calibration_seconds = calibration_start.elapsed().as_secs_f64();
            // Per-pass proxy excludes the common final certificate, charged once through F.
            // The factor two allows vector-update overhead empirically, not by proof.
            let pass_proxy = 2. * first_pass_seconds;
            let capped_passes = if pass_proxy > 0. {
                (((options.cap - 1.) * direct_seconds / pass_proxy)
                    .floor()
                    .max(0.) as usize)
                    .min(options.corrections + 1)
            } else {
                0
            };
            for &tolerance in &options.tolerances {
                let residual_tolerance = plan
                    .as_ref()
                    .map(|p| div_down(tolerance, p.inverse_norm_upper()).unwrap());
                let ctx = Context {
                    provider: &provider,
                    plan: plan.as_ref(),
                    rhs: &rhs,
                    tolerance,
                    residual_tolerance,
                    allowance,
                    correction_limit: options.corrections,
                    capped_passes,
                };
                let modes = [
                    Mode::Direct,
                    Mode::NoRefinement,
                    Mode::Refinement,
                    Mode::Capped,
                ];
                for repeat in 0..options.repeats {
                    for offset in 0..modes.len() {
                        let mode = modes[(repeat + offset) % modes.len()];
                        provider.capture.set(false);
                        let start = Instant::now();
                        let mut iterations = 0usize;
                        let mut all_accepted = true;
                        let mut fallback_seconds = 0.;
                        let mut certificate_seconds = 0.;
                        while iterations == 0 || start.elapsed() < minimum {
                            let r = execute(&ctx, mode, &mut ws, &mut out);
                            all_accepted &= r.accepted;
                            fallback_seconds += r.fallback_seconds;
                            certificate_seconds += r.certificate_seconds;
                            black_box(&out);
                            iterations += 1;
                        }
                        let elapsed_seconds = start.elapsed().as_secs_f64() / iterations as f64;
                        // One separate diagnostic pass reveals a refused transactional candidate.
                        provider.capture.set(true);
                        provider.last_candidate.borrow_mut().fill(f64::NAN);
                        let result = execute(&ctx, mode, &mut ws, &mut out);
                        provider.capture.set(false);
                        let candidate = provider.last_candidate.borrow();
                        let last_candidate_error = candidate
                            .iter()
                            .all(|v| v.is_finite())
                            .then(|| error_upper(&candidate, &exact));
                        let last_candidate_residual = candidate
                            .iter()
                            .all(|v| v.is_finite())
                            .then(|| provider.residual_upper(&rhs, &candidate));
                        let actual_error = error_upper(&out, &exact);
                        let actual_error_lower = error_lower(&out, &exact);
                        let scientific_status =
                            if result.accepted && all_accepted && actual_error <= tolerance {
                                "PASS_SCOPED"
                            } else if actual_error_lower > tolerance {
                                "HOLD_PROVEN_ACCURACY"
                            } else {
                                "HOLD_UNRESOLVED_ERROR_BOUND"
                            };
                        println!(
                            "{}",
                            json!({"schema_version":1,"kind":"result","case":case,"action":action,
                    "method":mode.name(),"repeat":repeat,"dimension":options.n,"error_limit":tolerance,
                    "actual_error_upper":actual_error,"actual_error_lower":actual_error_lower,"actual_error":actual_error,"original_residual_upper":result.original_residual,
                    "certified_error_upper":result.certified_error,"scientific_status":scientific_status,
                    "chart_accepted":result.chart_accepted,"final_accepted":result.accepted,
                    "refusal_reason":result.reason,"last_chart_error_upper":last_candidate_error,
                    "last_chart_residual_upper":last_candidate_residual,
                    "last_chart_reported_residual":result.last_chart_reported_residual,
                    "last_chart_conditional_error_upper":result.last_chart_reported_error,
                    "residual_tolerance":residual_tolerance,"residual_uncertainty_allowance":allowance,
                    "physical_uncertainty_floor":plan.as_ref().map(|p| mul_up(p.inverse_norm_upper(),allowance).unwrap()),
                    "state_cap":provider.state_cap,"corrections":result.corrections,
                    "max_corrections":options.corrections,"estimated_capped_passes":capped_passes,
                    "chart_work":result.work,"fallback_solves":usize::from(result.fallback),
                    "final_original_interval_checks":1,"final_residual_scalar_products":3*options.n/2,
                    "elapsed_seconds":elapsed_seconds,"timing_iterations":iterations,
                    "fallback_seconds":if result.fallback {fallback_seconds/iterations as f64} else {0.},
                    "direct_or_fallback_solve_seconds":fallback_seconds/iterations as f64,
                    "certificate_seconds":certificate_seconds/iterations as f64,
                    "setup_seconds":setup_seconds,"setup_work":setup_work,"setup_refusal":setup_refusal,
                    "calibration_seconds":calibration_seconds,"calibration_chart_action_seconds":chart_action_seconds,
                    "calibration_original_action_seconds":original_action_seconds,"calibration_direct_seconds":direct_seconds,
                    "first_pass_seconds":first_pass_seconds,"empirical_pass_cost_proxy":pass_proxy,
                    "cost_ratio_cap":options.cap,"estimated_worst_route_seconds":capped_passes as f64*pass_proxy+direct_seconds,"observed_ratio_to_calibration_direct":elapsed_seconds/direct_seconds,
                    "prepared_defect_upper":plan.as_ref().map(|p| p.total_defect_upper()),
                    "prepared_inverse_norm_upper":plan.as_ref().map(|p| p.inverse_norm_upper()),
                    "theoretical_operator_action_cap":plan.as_ref().map(|p| p.degree()*(options.corrections+1)),
                    "timing_scope":"reused plan/workspace; complete candidate and any fallback plus outward original residual certificate; setup and cost calibration are separate one-time costs, diagnostic copy/reference comparison excluded"})
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interval_reference_covers_direct_and_original_residual() {
        for case in ["nilpotent", "leakage", "nonnormal", "degenerate"] {
            let p = Provider::new(case, 4, false);
            let b = [0.25, -0.125, 0.375, 0.0625];
            let mut x = [0.; 4];
            p.direct(&b, &mut x);
            assert!(error_upper(&x, &p.exact_solution(&b)) < 1e-14);
            assert!(p.residual_upper(&b, &x) < 1e-14);
        }
    }
    #[test]
    fn refinement_improves_leakage_and_floor_refuses_before_action() {
        let p = Provider::new("leakage", 4, false);
        let plan =
            ConditionedPolynomialPlan::prepare(&p.declaration, 1e-12, &mut ChartWork::default())
                .unwrap();
        let b = [0.25, -0.125, 0.375, 0.0625];
        let mut ws = ChartWorkspace::new(4);
        let mut x = [0.; 4];
        let mut ctx = Context {
            provider: &p,
            plan: Some(&plan),
            rhs: &b,
            tolerance: 1e-9,
            residual_tolerance: Some(div_down(1e-9, plan.inverse_norm_upper()).unwrap()),
            allowance: p.residual_allowance(&b),
            correction_limit: 8,
            capped_passes: 0,
        };
        let no = execute(&ctx, Mode::NoRefinement, &mut ws, &mut x);
        assert!(!no.chart_accepted && no.fallback && no.accepted);
        let yes = execute(&ctx, Mode::Refinement, &mut ws, &mut x);
        assert!(yes.chart_accepted && yes.corrections > 0 && yes.accepted);
        ctx.residual_tolerance = Some(ctx.allowance / 2.);
        let floor = execute(&ctx, Mode::Refinement, &mut ws, &mut x);
        assert_eq!(floor.reason, "residual_uncertainty_floor");
        assert_eq!(floor.work.original_actions_attempted, 0);
        let cap = execute(
            &Context {
                residual_tolerance: Some(1e-9),
                ..ctx
            },
            Mode::Capped,
            &mut ws,
            &mut x,
        );
        assert_eq!(cap.reason, "estimated_cost_cap");
        assert_eq!(cap.work.applications_attempted, 0);
    }
    #[test]
    fn chart_degeneracy_does_not_mean_original_singularity() {
        let p = Provider::new("degenerate", 4, false);
        assert_eq!(
            ConditionedPolynomialPlan::prepare(&p.declaration, 1e-12, &mut ChartWork::default())
                .unwrap_err(),
            Error::DenominatorMargin
        );
        let b = [1., 2., 3., 4.];
        let mut x = [0.; 4];
        p.direct(&b, &mut x);
        assert_eq!(x, b);
        assert_eq!(p.residual_upper(&b, &x), 0.);
    }
}
