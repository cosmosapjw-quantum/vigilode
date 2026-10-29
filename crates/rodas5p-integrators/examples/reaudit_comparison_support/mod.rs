//! One owned represented request, immutable budget, explicit certificate scope.
//! This benchmark support module is not exported as a public solver capability.
use rodas5p_core::directed::{Interval, add_up, div_down, exp_interval, mul_up, sqrt_up};
use rodas5p_core::polynomial_action::{JointPhiInput, SymmetricNonpositiveOperator};
use rodas5p_core::polynomial_router::{
    GuardedPolynomialConfig, GuardedPolynomialWorkspace, PolynomialBackend as B,
    PolynomialRoutingMode, guarded_polynomial_action,
};
use rodas5p_core::{DenseMatrix, DenseOperator, WorkCounters, sha256_hex};
use rodas5p_integrators::{
    ActualKHomotopyConfig, ActualKHomotopyWork, CollectiveModel, ExponentialKrylovConfig,
    NonlinearHomotopyConfig, NonlinearHomotopyWork, certify_collective_actual_k_homotopy,
    integrate_nonlinear_homotopy, krylov_phi_action,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

type Result<T> = std::result::Result<T, String>;
fn ns(start: Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub fn manifest() -> Value {
    json!({"schema":"vigilode.reaudit-comparison-manifest.v1", "version":1,
        "suites":[
            {"suite":"polynomial","cases":["p4","p16","p64"],
             "arms":["protected","krylov","leja","laguerre","chebyshev","router"],
             "original_budget":1e-7,"scope":"REQUESTED_ODE_ENDPOINTS_ONLY",
             "protected_baseline_kind":"analytic diagonal exponential; not RODAS5P",
             "polynomial_arms":"named backend priority with guarded fallback; router compares all with Leja priority"},
            {"suite":"homotopy","cases":["h3","h8","h16"],
             "arms":["direct","continuation"],"original_budget":2e-5,
             "scope":"REQUESTED_ODE_ENDPOINTS_ONLY"},
            {"suite":"actual-k","cases":["k2","k4"],
             "arms":["direct","continuation"],"original_budget":1e-9,
             "scope":"STAGE_FUNCTIONAL_ONLY"}],
        "full_curve_certified":false,"timing_gate":"HOLD_UNTIL_SOURCE_BOUND_PAIRED_NCP_RECEIPTS",
        "default_repetitions":21,"warmups":2})
}

pub fn parse_args(args: &[String]) -> Result<(String, String, String, usize)> {
    if args.len() != 8 {
        return Err("expected --suite SUITE --case CASE --arm ARM --threads N; or --list".into());
    }
    let mut values = std::collections::BTreeMap::new();
    for pair in args.chunks_exact(2) {
        if !["--suite", "--case", "--arm", "--threads"].contains(&pair[0].as_str())
            || values.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown or duplicate CLI key".into());
        }
    }
    let suite = values.get("--suite").ok_or("missing suite")?.to_string();
    let case = values.get("--case").ok_or("missing case")?.to_string();
    let arm = values.get("--arm").ok_or("missing arm")?.to_string();
    let threads = values
        .get("--threads")
        .ok_or("missing threads")?
        .parse::<usize>()
        .map_err(err)?;
    if threads == 0 || threads > 256 {
        return Err("threads must lie in1..=256".into());
    }
    validate(&suite, &case, &arm)?;
    Ok((suite, case, arm, threads))
}
fn validate(suite: &str, case: &str, arm: &str) -> Result<()> {
    let m = manifest();
    let s = m["suites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["suite"] == suite)
        .ok_or("unknown suite")?;
    if !s["cases"].as_array().unwrap().iter().any(|v| v == case)
        || !s["arms"].as_array().unwrap().iter().any(|v| v == arm)
    {
        return Err("unknown case/arm for suite".into());
    }
    Ok(())
}

/// The identity excludes arm/thread count but includes every represented input,
/// budget and resource cap common to both arms. Schedule differences are exported.
pub fn request(suite: &str, case: &str) -> Result<Value> {
    match (suite, case) {
        ("polynomial", "p4" | "p16" | "p64") => {
            let n = case[1..].parse::<usize>().map_err(err)?;
            let rates: Vec<_> = (0..n).map(|i| bits(-4.0 - (i % 4) as f64)).collect();
            let initial: Vec<_> = (0..n)
                .map(|i| bits(if i % 2 == 0 { 0.125 } else { -0.25 }))
                .collect();
            Ok(
                json!({"model":"owned_diagonal_linear_v1","dimension":n,"rates_bits":rates,
                "initial_bits":initial,"mesh_bits":[bits(0.0),bits(0.03125),bits(0.0625),bits(0.09375),bits(0.125)],
                "original_budget_bits":bits(1e-7),"polynomial_local_budget_fraction":0.125,
                "maximum_krylov_dimension":n.min(24),"maximum_steps":4,
                "candidate_policy":"four fixed steps; no budget relaxation or retries",
                "certificate":"independent outward exp from original IVP at all4 endpoints"}),
            )
        }
        ("homotopy", "h3" | "h8" | "h16") | ("actual-k", "k2" | "k4") => {
            let n = case[1..].parse::<usize>().map_err(err)?;
            let windows = if suite == "actual-k" {
                1
            } else if n == 3 {
                4
            } else {
                n
            };
            let model = collective(n, suite == "actual-k")?;
            Ok(
                json!({"model":"owned_collective_v1","dimension":2*n,"modes":n,
                "xi_bits":model.xi().iter().copied().map(bits).collect::<Vec<_>>(),
                "weight_bits":model.weights().iter().copied().map(bits).collect::<Vec<_>>(),
                "initial_bits":model.initial_amplitudes().iter().map(|z|[bits(z.0),bits(z.1)]).collect::<Vec<_>>(),
                "width_bits":bits(model.width()),"chi_bits":bits(model.chi()),
                "carrier_bits":bits(model.carrier()),"kappa_bits":bits(model.kappa()),
                "windows":windows,"horizon_bits":bits(if suite=="actual-k" {1.0/1024.0} else {0.25}),
                "original_budget_bits":bits(if suite=="actual-k" {1e-9} else {2e-5}),
                "correction_rounds":if suite=="actual-k" {10} else {2},
                "max_refinements":if suite=="actual-k" {0} else {2},
                "split_substeps":4,"packet_order":5,"packet_degree":10,"packet_sweeps":8,
                "damped_subdivision_levels":6,"common_tube_radius_bits":bits(1e-3),
                "schedule_choices":[[1.0],[0.0,0.5,1.0]]}),
            )
        }
        _ => Err("unknown represented request".into()),
    }
}
fn collective(n: usize, stage: bool) -> Result<CollectiveModel> {
    let xi = (0..n)
        .map(|j| -1.0 + 2.0 * j as f64 / (n - 1) as f64)
        .collect();
    let weights = vec![1.0 / n as f64; n];
    let initial = (0..n)
        .map(|j| {
            (
                0.25 + 0.015625 * (j % 4) as f64,
                -0.0625 + 0.015625 * (j % 3) as f64,
            )
        })
        .collect();
    CollectiveModel::new(
        xi,
        weights,
        initial,
        0.7,
        0.4,
        0.0,
        if stage { 0.1 } else { 0.0 },
    )
    .map_err(err)
}
fn schedule(arm: &str) -> Vec<f64> {
    if arm == "direct" {
        vec![1.0]
    } else {
        vec![0.0, 0.5, 1.0]
    }
}

pub fn run(suite: &str, case: &str, arm: &str, threads: usize) -> Value {
    let started = Instant::now();
    let mut row = json!({"schema":"vigilode.reaudit-comparison.v1","suite":suite,"case":case,"arm":arm,
        "threads":threads,"status":"REFUSED","full_curve_certified":false,
        "achieved_bound":null,"outputs":[],"work":{},"span":{"observed_wall_ns":null,
        "certified_scheduler_span":null,"note":"Wall time includes setup, all candidate/retry work, certificate, and output materialization; no invented FLOP/span scalar."},
        "scope":if suite=="actual-k" {"STAGE_FUNCTIONAL_ONLY"} else {"REQUESTED_ODE_ENDPOINTS_ONLY"},
        "original_budget":if suite=="polynomial" {1e-7} else if suite=="homotopy" {2e-5} else {1e-9},
        "solver_transaction_timed":true,"process_end_to_end_timed":false,"measured_allocated_bytes":null,"speed_promotion":"HOLD",
        "elapsed_excludes":["process startup","threadpool creation","JSON serialization after row construction"]});
    let result = (|| -> Result<()> {
        validate(suite, case, arm)?;
        let req = request(suite, case)?;
        row["input_sha256"] = json!(sha256_hex(&serde_json::to_vec(&req).map_err(err)?));
        row["request"] = req;
        match suite {
            "polynomial" => polynomial(case, arm, &mut row),
            "homotopy" => homotopy(case, arm, &mut row),
            "actual-k" => actual_k(case, arm, &mut row),
            _ => unreachable!(),
        }
    })();
    if let Err(e) = result {
        row["error"] = json!(e);
    }
    let elapsed = ns(started);
    row["elapsed_ns"] = json!(elapsed);
    row["span"]["observed_wall_ns"] = json!(elapsed);
    row
}

/// Closed fixture owner: no caller callback can replace the ODE or the oracle.
struct LinearOwner {
    rates: Vec<f64>,
    initial: Vec<f64>,
    matrix: DenseMatrix,
}
impl LinearOwner {
    fn new(n: usize) -> Self {
        let rates: Vec<_> = (0..n).map(|i| -4.0 - (i % 4) as f64).collect();
        let initial = (0..n)
            .map(|i| if i % 2 == 0 { 0.125 } else { -0.25 })
            .collect();
        let mut matrix = DenseMatrix::zeros(n, n);
        for i in 0..n {
            matrix[(i, i)] = rates[i];
        }
        Self {
            rates,
            initial,
            matrix,
        }
    }
    /// Each exact t*rate is dyadic in these frozen fixtures. Every operation
    /// after that product is outward, including norm summation and sqrt.
    fn endpoint_radius(&self, time: f64, center: &[f64]) -> Result<f64> {
        if center.len() != self.initial.len() {
            return Err("oracle dimension".into());
        }
        let mut square = 0.0;
        for ((&r, &x), &y) in self.rates.iter().zip(&self.initial).zip(center) {
            let z = Interval::point(r)
                .map_err(err)?
                .mul(Interval::point(time).map_err(err)?)
                .map_err(err)?;
            if z.lo != z.hi {
                return Err("fixture exponent is not exactly represented".into());
            }
            let exact = exp_interval(z.lo)
                .map_err(err)?
                .mul(Interval::point(x).map_err(err)?)
                .map_err(err)?;
            let radius = exact
                .sub(Interval::point(y).map_err(err)?)
                .map_err(err)?
                .mag();
            square = add_up(square, mul_up(radius, radius).map_err(err)?).map_err(err)?;
        }
        sqrt_up(square).map_err(err)
    }
}
fn polynomial(case: &str, arm: &str, row: &mut Value) -> Result<()> {
    let n = case[1..].parse::<usize>().map_err(err)?;
    let setup = Instant::now();
    let owner = LinearOwner::new(n);
    let op = SymmetricNonpositiveOperator::gershgorin(owner.matrix.clone()).map_err(err)?;
    let kop = Arc::new(DenseOperator::new(owner.matrix.clone()).map_err(err)?);
    let mut workspace = GuardedPolynomialWorkspace::default();
    let budget = 1e-7;
    let local_budget = div_down(budget, 8.0).map_err(err)?;
    let mut center = owner.initial.clone();
    let mut native = WorkCounters::default();
    let mut reports = Vec::new();
    let mut selected_backends = Vec::new();
    let mut fallback_used = false;
    let mut outputs = Vec::new();
    let mut maximum = 0.0_f64;
    let mut propagated = 0.0;
    let mut certificate_ns = 0u64;
    let mut candidate_ns = 0u64;
    let mut candidate_attempts = 0usize;
    let mut oracle_attempts = 0usize;
    row["setup_ns"] = json!(ns(setup));
    row["baseline_kind"] = json!(match arm {
        "protected" => "analytic diagonal exponential (not protected RODAS5P)",
        "krylov" => "Krylov phi0 candidate with independent endpoint certificate",
        _ => "guarded polynomial route with immutable action budget",
    });
    row["arm_semantics"] = json!(match arm {
        "protected" => "four scalar exponential propagation steps",
        "krylov" => "four Krylov phi0 propagation steps",
        "router" => "compare all families; deterministic Leja-first admitted selection",
        "leja" => "Leja-priority guarded routing with Chebyshev/Laguerre fallback",
        "laguerre" => "Laguerre-priority guarded routing with Leja/Chebyshev fallback",
        _ => "Chebyshev-priority guarded routing with Leja/Laguerre fallback",
    });
    // Record each completed/failed attempt before returning. Native work is not
    // a complete FLOP count; full wall timing captures setup and oracle work.
    let result = (|| -> Result<()> {
        for step in 1..=4 {
            candidate_attempts += 1;
            let start = Instant::now();
            let next = (|| -> Result<Vec<f64>> {
                if arm == "protected" {
                    Ok(center
                        .iter()
                        .zip(&owner.rates)
                        .map(|(&y, &r)| y * (r * 0.03125).exp())
                        .collect())
                } else if arm == "krylov" {
                    let cfg = ExponentialKrylovConfig {
                        maximum_dimension: n.min(24),
                        ..ExponentialKrylovConfig::default()
                    };
                    let p = krylov_phi_action(kop.clone(), 0.03125, 0, &center, cfg, &mut native)
                        .map_err(err)?;
                    reports.push(serde_json::to_value(&p).map_err(err)?);
                    // Converged is diagnostic only; the independent ODE oracle
                    // below is the sole authority for this candidate's radius.
                    Ok(p.value)
                } else {
                    let order = match arm {
                        "laguerre" => [B::Laguerre, B::EnclosedLeja, B::Chebyshev],
                        "chebyshev" => [B::Chebyshev, B::EnclosedLeja, B::Laguerre],
                        _ => [B::EnclosedLeja, B::Chebyshev, B::Laguerre],
                    };
                    let cfg = GuardedPolynomialConfig {
                        order,
                        mode: if arm == "router" {
                            PolynomialRoutingMode::CompareAll
                        } else {
                            PolynomialRoutingMode::FirstAdmitted
                        },
                        ..GuardedPolynomialConfig::default()
                    };
                    let p = guarded_polynomial_action(
                        &op,
                        0.03125,
                        JointPhiInput::SameVector {
                            vector: &center,
                            scales: [1.0, 0.0, 0.0, 0.0, 0.0],
                        },
                        local_budget,
                        cfg,
                        &mut workspace,
                        &mut native,
                    )
                    .map_err(err)?;
                    reports.push(serde_json::to_value(&p).map_err(err)?);
                    if let Some(selected) = p.selected() {
                        fallback_used |= selected.backend() != order[0];
                        selected_backends
                            .push(serde_json::to_value(selected.backend()).map_err(err)?);
                    }
                    let y = p
                        .fused()
                        .ok_or("all polynomial backends abstained")?
                        .to_vec();
                    propagated =
                        add_up(propagated, p.bound().ok_or("missing private action bound")?)
                            .map_err(err)?;
                    if propagated > budget {
                        return Err("original cumulative action budget exceeded".into());
                    }
                    Ok(y)
                }
            })();
            candidate_ns = candidate_ns.saturating_add(ns(start));
            center = next?;
            let cert = Instant::now();
            oracle_attempts += 1;
            let radius = owner.endpoint_radius(step as f64 * 0.03125, &center);
            certificate_ns = certificate_ns.saturating_add(ns(cert));
            let radius = radius?;
            maximum = maximum.max(radius);
            outputs.push(json!({"time":step as f64*0.03125,"center":center,"radius":radius}));
            if radius > budget {
                return Err("independent original-ODE endpoint budget exceeded".into());
            }
        }
        Ok(())
    })();
    row["work"] = json!({"native":native,"attempt_reports":reports,"operator_setup_dense_entries":n*n,
        "candidate_attempts":candidate_attempts,"endpoint_oracle_calls":oracle_attempts,"endpoint_oracle_components_attempted":oracle_attempts*n,"endpoint_oracle_components":outputs.len()*n,
        "oracle_component_operations":"interval exponent product, exp enclosure, initial multiply, center subtract, square, sum; final sqrt",
        "counter_scope":"native event counts plus oracle counts; no claim of complete FLOP instrumentation",
        "cumulative_action_radius":propagated,"local_action_budget":local_budget});
    row["selected_backend_per_step"] = json!(selected_backends);
    row["fallback_used"] = json!(fallback_used);
    row["candidate_ns"] = json!(candidate_ns);
    row["certificate_ns"] = json!(certificate_ns);
    row["outputs"] = json!(outputs);
    row["span"]["candidate_step_depth"] = json!(4);
    row["span"]["certificate_endpoint_depth"] = json!(4);
    result?;
    row["achieved_bound"] = json!(maximum);
    row["status"] = json!("PASS_CERTIFIED_ENDPOINTS");
    Ok(())
}

fn homotopy(case: &str, arm: &str, row: &mut Value) -> Result<()> {
    let n = case[1..].parse::<usize>().map_err(err)?;
    let setup = Instant::now();
    let model = collective(n, false)?;
    let windows = if n == 3 { 4 } else { n };
    let mesh: Vec<_> = (0..=windows)
        .map(|i| 0.25 * i as f64 / windows as f64)
        .collect();
    let cfg = NonlinearHomotopyConfig {
        split_substeps: 4,
        correction_rounds: 2,
        max_refinements: 2,
        lambdas: schedule(arm),
        weighted_tolerance: 2e-5,
        packet_order: 5,
        packet_degree: 10,
        packet_sweeps: 8,
        damped_subdivision_levels: 6,
    };
    row["schedule"] = json!(cfg.lambdas);
    row["setup_ns"] = json!(ns(setup));
    let mut work = NonlinearHomotopyWork::default();
    let solve = Instant::now();
    let result = integrate_nonlinear_homotopy(&model, &mesh, &cfg, &mut work);
    row["candidate_and_certificate_ns"] = json!(ns(solve));
    row["certificate_ns"] = Value::Null;
    row["work"] = json!({"native":work,"counter_scope":"all attempted native algorithm events; packet counters are selected operations, not complete FLOPs",
        "certificate_cost_included_in_full_time":true,"separate_certificate_timer_available":false});
    row["span"]["candidate_round_depth"] = json!(work.correction_rounds);
    row["span"]["fine_window_tasks"] = json!(work.fine_window_calls);
    row["span"]["certificate_window_tasks"] = json!(work.verification_window_calls);
    row["span"]["prefix_sweep_levels_per_round"] = json!(2 * windows.ilog2());
    let out = result.map_err(err)?;
    if out.source_model() != &model
        || out.mesh() != mesh
        || out.config().weighted_tolerance.to_bits() != cfg.weighted_tolerance.to_bits()
    {
        return Err("owned request identity changed".into());
    }
    let max = out
        .points()
        .iter()
        .map(|p| p.weighted_radius())
        .fold(0.0, f64::max);
    if !max.is_finite() || max > cfg.weighted_tolerance {
        return Err("original endpoint budget failed".into());
    }
    row["outputs"] = json!(
        out.points()
            .iter()
            .map(|p| json!({"time":p.time(),"center":p.modes(),
        "weighted_radius":p.weighted_radius(),"mode_radii":p.mode_radii()}))
            .collect::<Vec<_>>()
    );
    row["final_substeps"] = json!(out.final_substeps());
    row["final_rounds"] = json!(out.final_rounds());
    row["achieved_bound"] = json!(max);
    row["status"] = json!("PASS_CERTIFIED_ENDPOINTS");
    Ok(())
}
fn actual_k(case: &str, arm: &str, row: &mut Value) -> Result<()> {
    let n = case[1..].parse::<usize>().map_err(err)?;
    let setup = Instant::now();
    let model = collective(n, true)?;
    let cfg = ActualKHomotopyConfig {
        h: 1.0 / 1024.0,
        lambdas: schedule(arm),
        rounds_per_stage: 10,
        tube_radius: 1e-3,
        stage_output_tolerance: 1e-9,
    };
    row["schedule"] = json!(cfg.lambdas);
    row["setup_ns"] = json!(ns(setup));
    let mut work = ActualKHomotopyWork::default();
    let solve = Instant::now();
    let result = certify_collective_actual_k_homotopy(&model, &cfg, &mut work);
    row["candidate_and_certificate_ns"] = json!(ns(solve));
    row["certificate_ns"] = Value::Null;
    row["work"] = json!({"native":work,"certificate_cost_included_in_full_time":true,
        "separate_certificate_timer_available":false,"counter_scope":"native stage/certificate event counts, not complete FLOPs"});
    row["span"]["candidate_round_depth"] = json!(work.candidate_rounds);
    let out = result.map_err(err)?;
    let max = out.output_radii().iter().copied().fold(0.0, f64::max);
    if out.source_model() != &model
        || out.h().to_bits() != cfg.h.to_bits()
        || !max.is_finite()
        || max > cfg.stage_output_tolerance
    {
        return Err("original stage-functional request/budget failed".into());
    }
    row["outputs"] = json!({"center":out.output(),"radii":out.output_radii()});
    row["certificate"] = json!({"retained_alpha_beta_entries":work.retained_alpha_beta_entries,
        "inverse_residual":out.inverse_residual(),"leakage_q":out.leakage_q(),
        "noncausal_alpha_max":out.noncausal_alpha_max(),"noncausal_linear_max":out.noncausal_linear_max(),
        "ode_truncation_certified":false});
    row["achieved_bound"] = json!(max);
    row["status"] = json!("PASS_CERTIFIED_STAGE_FUNCTIONAL");
    Ok(())
}
