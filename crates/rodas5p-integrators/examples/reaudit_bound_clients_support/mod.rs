//! Actual bounded clients for N03/N04/N05; no generic promotion authority.
use rodas5p_core::directed::{Interval, add_up, exp_interval, mul_up, sqrt_up};
use rodas5p_core::{DenseMatrix, sha256_hex};
use rodas5p_integrators::global_budget::{
    AllocationBlock, AllocationLimits, allocate_global_work, certify_global_run,
    validate_work_allocation,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, BoundLinearStageModel, OdeProblem, OutputSchedule, Pr101Model,
    Rodas5pLaggedOptions, StageSolver, StalenessRule, VerifiedContinuousProblem,
    integrate_rodas5p_lagged_observed, integrate_verified_residual,
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn ns(start: Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
pub fn manifest() -> Value {
    json!({"schema":"vigilode.reaudit-bound-clients-manifest.v1",
    "warmups":2,"paired_repetitions":21,"cases":[
    {"case":"diagonal","arms":["bounded","resolvent","diagonal"],"original_budget":1e-7,"scope":"REQUESTED_ODE_ENDPOINTS_ONLY","full_curve_certified":false},
    {"case":"global","arms":["provider","replay"],"original_budget":1e-4,"scope":"BOUND_MODEL_FULL_CURVE","full_curve_certified":true},
    {"case":"allocation","arms":["uniform","optimized"],"original_budget":0.5,"scope":"ALGEBRAIC_ALLOCATION_ONLY","full_curve_certified":false}]})
}
fn validate(case: &str, arm: &str) -> Result<()> {
    let m = manifest();
    let c = m["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["case"] == case)
        .ok_or("unknown bound case")?;
    if !c["arms"].as_array().unwrap().iter().any(|v| v == arm) {
        return Err("unknown bound arm".into());
    }
    Ok(())
}
pub fn parse(args: &[String]) -> Result<(String, String, usize)> {
    if args.len() != 6 {
        return Err("expected --case CASE --arm ARM --threads N".into());
    }
    let mut fields = std::collections::BTreeMap::new();
    for pair in args.chunks_exact(2) {
        if !["--case", "--arm", "--threads"].contains(&pair[0].as_str())
            || fields.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown/duplicate argument".into());
        }
    }
    let case = fields.get("--case").ok_or("missing case")?.to_string();
    let arm = fields.get("--arm").ok_or("missing arm")?.to_string();
    let threads = fields
        .get("--threads")
        .ok_or("missing threads")?
        .parse::<usize>()
        .map_err(err)?;
    if !(1..=256).contains(&threads) {
        return Err("invalid threads".into());
    }
    validate(&case, &arm)?;
    Ok((case, arm, threads))
}
fn request(case: &str) -> Value {
    match case {
        "diagonal" => json!({"model":"owned_diagonal_ivp32_v1","dimension":32,
        "rates_bits":(0..32).map(|j|bits(-((1usize<<(j%4)) as f64))).collect::<Vec<_>>(),
        "initial_bits":(0..32).map(|j|bits(if j%2==0 {0.5} else {-0.25})).collect::<Vec<_>>(),
        "mesh_bits":(0..=4).map(|j|bits(j as f64/256.0)).collect::<Vec<_>>(),
        "atol_bits":bits(1e-10),"rtol_bits":bits(1e-6),"initial_max_step_bits":bits(1.0/1024.0),
        "endpoint_l2_budget_bits":bits(1e-7),"max_attempts":1000,"stage_solver":"GMRES",
        "certificate":"outward original-IVP diagonal exp at requested endpoints"}),
        "global" => json!({"model":"sealed_PR101","lambda_bits":bits(-16.0),"eta_bits":bits(1.0),
        "initial_bits":[bits(0.0)],"initial_radius_bits":bits(0.0),
        "span_bits":[bits(0.0),bits(1.0/64.0)],"query_bits":[bits(0.0),bits(1.0/128.0),bits(1.0/64.0)],
        "atol_bits":bits(1e-4),"rtol_bits":bits(1e-6),"initial_max_step_bits":bits(1.0/128.0),"max_attempts":2000}),
        _ => {
            json!({"scope":"algebraic_only","A_bits":[bits(1.0),bits(9.0)],"c_bits":[bits(1.0),bits(1.0)],"order":1,"budget_bits":bits(0.5),"counts_choices":[[20,20],[8,24]],"limits":"AllocationLimits::default"})
        }
    }
}
pub fn run(case: &str, arm: &str, threads: usize) -> Value {
    let start = Instant::now();
    let budget = match case {
        "diagonal" => 1e-7,
        "global" => 1e-4,
        _ => 0.5,
    };
    let scope = match case {
        "diagonal" => "REQUESTED_ODE_ENDPOINTS_ONLY",
        "global" => "BOUND_MODEL_FULL_CURVE",
        _ => "ALGEBRAIC_ALLOCATION_ONLY",
    };
    let mut row = json!({"schema":"vigilode.reaudit-bound-clients.v1","case":case,"arm":arm,"threads":threads,
        "scope":scope,"status":"REFUSED","original_budget":budget,"achieved_bound":null,"full_curve_certified":false,
        "work":{},"outputs":[],"solver_transaction_timed":true,"process_end_to_end_timed":false,
        "speed_promotion":"HOLD","certified_scheduler_span":null,"measured_allocated_bytes":null});
    let result = (|| -> Result<()> {
        validate(case, arm)?;
        let req = request(case);
        row["input_sha256"] = json!(sha256_hex(&serde_json::to_vec(&req).map_err(err)?));
        row["request"] = req;
        match case {
            "diagonal" => diagonal(arm, &mut row),
            "global" => global(arm, &mut row),
            _ => allocation(arm, &mut row),
        }
    })();
    if let Err(e) = result {
        row["error"] = json!(e);
    }
    row["elapsed_ns"] = json!(ns(start));
    row["elapsed_excludes"] = json!(["process/threadpool startup", "final JSON serialization"]);
    row
}
fn diagonal(arm: &str, row: &mut Value) -> Result<()> {
    let setup = Instant::now();
    let n = 32;
    let rates: Vec<_> = (0..n).map(|j| -((1usize << (j % 4)) as f64)).collect();
    let initial: Vec<_> = (0..n)
        .map(|j| if j % 2 == 0 { 0.5 } else { -0.25 })
        .collect();
    let mut matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        matrix[(i, i)] = rates[i];
    }
    let rhs_rates = rates.clone();
    let jac = matrix.clone();
    let problem = OdeProblem::new(
        "reaudit-owned-diagonal32",
        n,
        Arc::new(move |_t, y, out| {
            for i in 0..n {
                out[i] = rhs_rates[i] * y[i];
            }
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t, _y| Ok(jac.clone()))),
        None,
        None,
        true,
        None,
        None,
    )
    .map_err(err)?;
    let cfg = AdaptiveStepConfig {
        atol: 1e-10,
        rtol: 1e-6,
        initial_step: 1.0 / 1024.0,
        max_step: 1.0 / 1024.0,
        max_attempts: 1000,
        ..Default::default()
    };
    // Exercise the sealed theorem planner under the same frozen matrix; these
    // targets remain a separately timed diagnostic, not injected into the
    // existing generic lagged driver's legacy stage policy.
    let planning = Instant::now();
    let owner = BoundLinearStageModel::new(matrix, vec![1.0; n]).map_err(err)?;
    let plan = owner
        .plan(&initial, cfg.initial_step, cfg.atol, cfg.rtol, 0.1)
        .map_err(err)?;
    row["stage_policy"] = json!({"calculus":format!("{:?}",plan.calculus()),"normalized_targets":plan.normalized_residual_targets(),
        "raw_euclidean_targets":plan.raw_euclidean_residual_targets().map_err(err)?,
        "used_by_driver":false,"setup_ns":ns(planning)});
    let mesh: Vec<_> = (0..=4).map(|j| j as f64 / 256.0).collect();
    let output = OutputSchedule::new(mesh.clone()).map_err(err)?;
    row["setup_ns"] = json!(ns(setup));
    let solve = Instant::now();
    let result = integrate_rodas5p_lagged_observed(
        &problem,
        (0.0, 1.0 / 64.0),
        &initial,
        &cfg,
        &output,
        Rodas5pLaggedOptions {
            stage_solver: StageSolver::Gmres,
            staleness: match arm {
                "bounded" => StalenessRule::Bounded,
                "resolvent" => StalenessRule::Resolvent,
                _ => StalenessRule::Diagonal,
            },
            ..Default::default()
        },
    )
    .map_err(err)?;
    row["candidate_ns"] = json!(ns(solve));
    let fallback = result.work.forced_refreshes
        + result.work.contraction_refreshes
        + result.work.budget_refreshes
        + result.staleness.staleness_refreshes;
    row["work"] = json!({"solver":result.fast.observed.counters,"lagged":result.work,"staleness":result.staleness,
        "attempts":result.fast.attempts,"fallback_count":fallback,"event_scope":"native events plus explicitly timed planner/oracle; no complete FLOP claim"});
    row["driver_policy_id"] = json!(result.fast.driver);
    if !result.fast.observed.success || result.fast.observed.t != mesh {
        return Err("driver did not complete unchanged output schedule".into());
    }
    let cert = Instant::now();
    let mut maximum = 0.0_f64;
    let mut outputs = Vec::new();
    for (&t, y) in result.fast.observed.t.iter().zip(&result.fast.observed.y) {
        let mut square = 0.0;
        for ((&rate, &x0), &center) in rates.iter().zip(&initial).zip(y) {
            let exponent = Interval::point(rate)
                .map_err(err)?
                .mul(Interval::point(t).map_err(err)?)
                .map_err(err)?;
            if exponent.lo != exponent.hi {
                return Err("inexact fixture exponent".into());
            }
            let exact = exp_interval(exponent.lo)
                .map_err(err)?
                .mul(Interval::point(x0).map_err(err)?)
                .map_err(err)?;
            let delta = exact
                .sub(Interval::point(center).map_err(err)?)
                .map_err(err)?
                .mag();
            square = add_up(square, mul_up(delta, delta).map_err(err)?).map_err(err)?;
        }
        let radius = sqrt_up(square).map_err(err)?;
        maximum = maximum.max(radius);
        outputs.push(json!({"time":t,"center":y,"radius":radius}));
    }
    row["certificate_ns"] = json!(ns(cert));
    row["outputs"] = json!(outputs);
    row["work"]["oracle_components"] = json!(n * mesh.len());
    if maximum > 1e-7 {
        return Err("original ODE endpoint budget exceeded".into());
    }
    if fallback != 0 {
        return Err("frozen zero-fallback comparison witness did not hold".into());
    }
    if arm == "diagonal"
        && (result.staleness.resolvent_solves != 0
            || result.staleness.diagonal_checks == 0
            || result.staleness.diagonal_refusals != 0)
    {
        return Err("diagonal no-extra-solve witness failed".into());
    }
    if arm == "resolvent" && result.staleness.resolvent_solves == 0 {
        return Err("resolvent baseline did not exercise inverse construction".into());
    }
    row["achieved_bound"] = json!(maximum);
    row["status"] = json!("PASS_CERTIFIED_ENDPOINTS");
    Ok(())
}
fn global(arm: &str, row: &mut Value) -> Result<()> {
    let setup = Instant::now();
    let owner = VerifiedContinuousProblem::from_pr101(Pr101Model::new(-16.0, 1.0).map_err(err)?)
        .map_err(err)?;
    let cfg = AdaptiveStepConfig {
        atol: 1e-4,
        rtol: 1e-6,
        initial_step: 1.0 / 128.0,
        max_step: 1.0 / 128.0,
        max_attempts: 2000,
        ..Default::default()
    };
    row["setup_ns"] = json!(ns(setup));
    let solve = Instant::now();
    let run = integrate_verified_residual(
        &owner,
        (0.0, 1.0 / 64.0),
        &[0.0],
        0.0,
        &[0.0, 1.0 / 128.0, 1.0 / 64.0],
        &cfg,
    )
    .map_err(err)?;
    row["candidate_and_provider_certificate_ns"] = json!(ns(solve));
    row["work"] = json!({"solver":run.solver_work,"provider":run.verification_work,"attempts":run.attempts,"rejected_attempts":run.rejected_attempts});
    if !run.success {
        return Err(run
            .failure
            .clone()
            .unwrap_or("sealed provider incomplete".into()));
    }
    let mut maximum = run
        .segments()
        .iter()
        .map(|s| s.diagnostics().uniform_radius)
        .fold(0.0_f64, f64::max);
    maximum = maximum.max(
        run.outputs()
            .iter()
            .map(|o| o.radius_l2())
            .fold(0.0_f64, f64::max),
    );
    if arm == "replay" {
        let verify = Instant::now();
        let replay = certify_global_run(&run, cfg.atol);
        row["global_replay_ns"] = json!(ns(verify));
        match replay {
            Ok(cert) => {
                if !cert.binds_run(&run) {
                    return Err("global certificate lost run identity".into());
                }
                let s = cert.snapshot();
                maximum = s.maximum_curve_radius.max(s.maximum_stored_query_radius);
                row["work"]["global_replay"] = json!(cert.work());
                row["global_snapshot"] = json!(s);
            }
            Err(failure) => {
                row["work"]["global_replay"] = json!(failure.work);
                return Err(failure.error.to_string());
            }
        }
    } else {
        row["global_replay_ns"] = json!(0);
    }
    row["outputs"] = json!(run.outputs());
    row["curve_segments"] = json!(
        run.segments()
            .iter()
            .map(|s| s.diagnostics())
            .collect::<Vec<_>>()
    );
    if maximum > cfg.atol {
        return Err("original full-curve/query budget exceeded".into());
    }
    row["achieved_bound"] = json!(maximum);
    row["full_curve_certified"] = json!(true);
    row["status"] = json!("PASS_CERTIFIED_FULL_CURVE");
    Ok(())
}
fn allocation(arm: &str, row: &mut Value) -> Result<()> {
    let blocks = [
        AllocationBlock {
            error_coefficient: 1.0,
            step_cost_upper: 1.0,
        },
        AllocationBlock {
            error_coefficient: 9.0,
            step_cost_upper: 1.0,
        },
    ];
    let counts = if arm == "uniform" { [20, 20] } else { [8, 24] };
    let begin = Instant::now();
    let validated = validate_work_allocation(&blocks, 1, 0.5, &counts, AllocationLimits::default())
        .map_err(err)?;
    row["certificate_ns"] = json!(ns(begin));
    row["work"] = json!({"validated_model_work_upper":validated.work_upper(),
        "algebraic_validation_blocks":2,"counts":validated.counts(),"actual_ode_steps_executed":0,
        "meaning":"cost ceiling for supplied algebraic coefficients; not measured solver work"});
    if arm == "optimized" {
        let search = Instant::now();
        let planned =
            allocate_global_work(&blocks, 1, 0.5, AllocationLimits::default()).map_err(err)?;
        row["planner_ns"] = json!(ns(search));
        row["heuristic_plan"] = json!(planned);
    }
    row["outputs"] = json!({"counts":validated.counts(),"error_upper":validated.error_upper(),"work_upper":validated.work_upper()});
    row["achieved_bound"] = json!(validated.error_upper());
    row["status"] = json!("PASS_VALIDATED_ALLOCATION");
    Ok(())
}
