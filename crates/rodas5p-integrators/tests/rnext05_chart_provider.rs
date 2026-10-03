//! Certified chart coordinate-error provider and controller (research node
//! `research/rnext05_chart_provider_20261003`, remaining-only DAG node
//! R-NEXT-05). Writes the runs to `RNEXT05_OUTPUT` for the 50-digit check
//! `tools/rnext05_chart_check.py`.
#![cfg(feature = "audit2-research")]

use rodas5p_integrators::chart_transport::{
    ChartControllerConfig, ChartIdentity, ChartPoint, ChartRun, ChartStepStart, chart_dense_point,
    chart_integrate,
};
use serde_json::{Value, json};

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn point_json(p: &ChartPoint) -> Value {
    json!({
        "x": hex(p.x), "w": hex(p.w), "y": hex(p.y),
        "x_box": [hex(p.x_box.lo), hex(p.x_box.hi)], "w_box": [hex(p.w_box.lo), hex(p.w_box.hi)],
        "b_phys": hex(p.physical_y_error_upper), "b_phys_f64": p.physical_y_error_upper,
        "b_loc": p.local_physical_bound,
        "proxy": p.embedded_proxy,
    })
}

const THETAS: [f64; 3] = [0.25, 0.5, 0.75];

/// Runs one case; returns its record and the native checks (controller,
/// tube regularity).
fn run_case(
    label: &str,
    identity: ChartIdentity,
    model_eps: f64,
    x0: f64,
    t_end: f64,
    tol: f64,
    d_min: f64,
) -> (Value, bool, bool, ChartRun) {
    let kappa = identity.kappa;
    let y0 = x0 * x0 * (0.1 + 1.0 / kappa);
    let config = ChartControllerConfig {
        atol: tol,
        rtol: tol,
        initial_step: 1.0e-6,
        min_step: 1.0e-12,
        max_step: 0.1,
        denominator_min: d_min,
    };
    let outputs = [0.25 / kappa, 1.0 / kappa, 4.0 / kappa];
    let run = chart_integrate(&identity, x0, y0, 0.0, t_end, &outputs, &config).unwrap();
    let mut controller_ok = true;
    let mut tube_ok = true;
    let mut steps = Vec::new();
    for (index, (start, end)) in run.steps.iter().enumerate() {
        controller_ok &= end.local_physical_bound <= tol + tol * end.y.abs();
        tube_ok &= !start.tube.contains_zero() && start.tube.mig() * start.tube.mig() >= d_min;
        let dense: Vec<Value> = THETAS
            .iter()
            .map(|&theta| {
                let p = chart_dense_point(&identity, &identity, start, theta, d_min).unwrap();
                let mut v = point_json(&p);
                v["theta"] = json!(theta);
                v["tau"] = json!(hex(theta * start.h));
                v
            })
            .collect();
        let mut v = point_json(end);
        v["tau"] = json!(hex(start.h));
        v["index"] = json!(index);
        v["dense"] = json!(dense);
        v["tube"] = json!([hex(start.tube.lo), hex(start.tube.hi)]);
        steps.push(v);
    }
    let output_steps: Vec<usize> = run
        .outputs
        .iter()
        .map(|o| {
            run.steps
                .iter()
                .position(|(_, e)| e.t.to_bits() == o.t.to_bits() && e.y.to_bits() == o.y.to_bits())
                .unwrap()
        })
        .collect();
    let record = json!({
        "label": label, "kappa": hex(kappa), "eps_stepper": hex(identity.eps), "eps_model": hex(model_eps),
        "branch": identity.branch, "x0": hex(x0), "y0": hex(y0), "t_end": t_end, "tol": tol, "d_min": d_min,
        "outputs_nominal": outputs, "output_steps": output_steps,
        "rejected_attempts": run.rejected_attempts, "refused": run.refused.as_ref().map(|(t, r)| json!({"t": t, "reason": r})),
        "steps": steps, "controller_ok": controller_ok, "tube_ok": tube_ok,
    });
    (record, controller_ok, tube_ok, run)
}

#[test]
fn chart_provider_runs() {
    let mut runs = Vec::new();
    let mut controller = true;
    let mut tube = true;
    let mut completed = true;
    for kappa in [40.0, 1000.0] {
        for (x0, t_end) in [(1.0, 0.5), (-1.0, 5.0)] {
            for eps in [0.0, 1.0e-3] {
                for tol in [1.0e-6, 1.0e-9] {
                    let identity = ChartIdentity {
                        kappa,
                        eps,
                        branch: if x0 > 0.0 { 1 } else { -1 },
                    };
                    let label = format!("kappa{kappa}-x0{x0}-eps{eps}-tol{tol}");
                    let (record, c, t, run) =
                        run_case(&label, identity, eps, x0, t_end, tol, 1.0e-6);
                    controller &= c;
                    tube &= t;
                    completed &= run.refused.is_none();
                    println!(
                        "{label}: steps {} rejected {} refused {:?}",
                        run.steps.len(),
                        run.rejected_attempts,
                        run.refused
                    );
                    runs.push(record);
                }
            }
        }
    }
    // Negative control: the model has eps = 1e-3, the stepper is told 0.
    let identity = ChartIdentity {
        kappa: 40.0,
        eps: 0.0,
        branch: 1,
    };
    let (negative, _, _, _) = run_case(
        "negative-control-forcing-omitted",
        identity,
        1.0e-3,
        1.0,
        0.5,
        1.0e-9,
        1.0e-6,
    );

    // Refusals.
    let identity = ChartIdentity {
        kappa: 40.0,
        eps: 0.0,
        branch: 1,
    };
    let (blowup, _, _, blowup_run) = run_case("blow-up", identity, 0.0, 1.0, 1.2, 1.0e-6, 1.0e-6);
    let blowup_refused = blowup_run.refused.is_some()
        // The flow formula needs x tau <= 1/2 on the step's start box.
        && blowup_run
            .steps
            .iter()
            .all(|(s, e)| s.x_box.hi * s.h <= 0.5 && e.t < 1.0);
    let identity = ChartIdentity {
        kappa: 40.0,
        eps: 0.0,
        branch: -1,
    };
    let (margin, _, _, margin_run) = run_case(
        "denominator-margin",
        identity,
        0.0,
        -1.0,
        50.0,
        1.0e-6,
        1.0e-3,
    );
    let margin_refused = margin_run.refused.is_some()
        && margin_run
            .steps
            .iter()
            .all(|(s, _)| s.tube.mig() * s.tube.mig() >= 1.0e-3);
    let (first_start, _): &(ChartStepStart, ChartPoint) = &margin_run.steps[0];
    let other = ChartIdentity {
        kappa: 41.0,
        eps: 0.0,
        branch: -1,
    };
    let identity_refused = chart_dense_point(&identity, &other, first_start, 0.5, 1.0e-3).is_err()
        && chart_dense_point(
            &identity,
            &ChartIdentity {
                eps: 1.0e-3,
                ..identity
            },
            first_start,
            0.5,
            1.0e-3,
        )
        .is_err()
        && chart_dense_point(&identity, &identity, first_start, 0.5, 1.0e-3).is_ok();

    let report = json!({
        "schema": "vigilode-rnext05-chart-runs-v1",
        "node": "research/rnext05_chart_provider_20261003",
        "runs": runs,
        "negative_control": negative,
        "refusals": {"blow_up": blowup, "denominator_margin": margin,
                     "blow_up_refused": blowup_refused, "margin_refused": margin_refused,
                     "identity_mismatch_refused": identity_refused},
        "native_gate": {
            "tube_regular": tube,
            "controller_uses_physical_bound": controller,
            "all_main_runs_completed": completed,
            "fail_closed": blowup_refused && margin_refused && identity_refused,
        },
    });
    if let Ok(path) = std::env::var("RNEXT05_OUTPUT") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(!path.exists(), "immutable output exists");
        std::fs::write(&path, serde_json::to_string(&report).unwrap() + "\n").unwrap();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report["native_gate"]).unwrap()
    );
    println!(
        "blow-up refused at {:?}; margin refused at {:?}",
        blowup_run.refused, margin_run.refused
    );
}
