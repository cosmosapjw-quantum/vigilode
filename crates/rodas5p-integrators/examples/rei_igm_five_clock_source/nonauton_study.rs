//! Opt-in, source-locked nonautonomous REI five-clock integrator comparison.
//! Original 15-state source, JVP and reference are reused from the parent
//! example. This does NOT implement original 26-event H/He/OTS thermochemistry.
use super::*;
use rodas5p_core::CoreError;
use rodas5p_integrators::{Dopri5Config, integrate_dopri5_dense_observed};
use serde_json::{Value, json};
use std::time::Instant;

const ORIGINAL_ERROR_BUDGET: f64 = 5.0e-10;
const INVARIANT_BUDGET: f64 = 2.0e-11;

/// Exact source-clock derivative for the original four-group, three-moment
/// rational photon allocator within the selected piecewise-linear segment.
/// u is dimensionless source time; the original RHS already multiplies
/// photon rates by h_seconds so no second factor of h is introduced.
fn partial_u_segment(
    s: &Source,
    segment: usize,
    u: f64,
    y: &[f64],
    out: &mut [f64],
) -> Result<f64, CoreError> {
    if segment >= 4 || out.len() != DIM || y.len() != DIM {
        return Err(CoreError::Dimension(
            "invalid original source derivative segment/state".into(),
        ));
    }
    let lo = s.times[segment];
    let hi = s.times[segment + 1];
    if !u.is_finite()
        || u < lo - 2e-14
        || u > hi + 2e-14
        || y[..3].iter().any(|x| !x.is_finite() || *x <= 0.)
    {
        return Err(CoreError::InvalidInput(
            "source-clock derivative outside physical branch".into(),
        ));
    }
    let (j, w) = s.forcing(u, false);
    let dt = hi - lo;
    if !(dt > 0. && dt.is_finite()) {
        return Err(CoreError::InvalidInput(
            "invalid source-clock spacing".into(),
        ));
    }
    let mut dj = [0.; 4];
    let mut dw = [[0.; 4]; 4];
    for g in 0..4 {
        dj[g] = (s.j[segment + 1][g] - s.j[segment][g]) / dt;
        for q in 0..4 {
            dw[g][q] = (s.w[segment + 1][g][q] - s.w[segment][g][q]) / dt;
        }
    }
    let mut dphoto = [0.; 3];
    let mut descape = [0.; 4];
    for g in 0..4 {
        let d = w[g][3] + (0..3).map(|i| w[g][i] * y[i]).sum::<f64>();
        let dd = dw[g][3] + (0..3).map(|i| dw[g][i] * y[i]).sum::<f64>();
        if !(d > 0. && d.is_finite()) {
            return Err(CoreError::NonFinite(
                "original photon denominator not positive".into(),
            ));
        }
        for i in 0..3 {
            dphoto[i] +=
                y[i] * ((dj[g] * w[g][i] + j[g] * dw[g][i]) / d - j[g] * w[g][i] * dd / (d * d));
        }
        descape[g] = (dj[g] * w[g][3] + j[g] * dw[g][3]) / d - j[g] * w[g][3] * dd / (d * d);
    }
    out.fill(0.);
    let dk: [f64; 3] = std::array::from_fn(|i| s.h * dphoto[i] / s.s0[i]);
    out[0] = -dk[0];
    out[1] = -dk[1];
    out[2] = (s.s0[1] / s.s0[2]) * dk[1] - dk[2];
    out[3..6].copy_from_slice(&dk);
    for g in 0..4 {
        out[6 + g] = s.h * descape[g] / s.s0[0];
        out[10 + g] = s.h * dj[g] / s.s0[0];
    }
    out[14] = s.h * (0..3).map(|i| s.energies[i] * dphoto[i]).sum::<f64>() / s.s0[0];
    if out.iter().any(|x| !x.is_finite()) {
        return Err(CoreError::NonFinite(
            "nonfinite original time derivative".into(),
        ));
    }
    let emitted = dj.iter().sum::<f64>();
    let assigned = dphoto.iter().sum::<f64>() + descape.iter().sum::<f64>();
    let maxscale = emitted
        .abs()
        .max(dphoto.iter().map(|z| z.abs()).sum::<f64>())
        .max(descape.iter().map(|z| z.abs()).sum::<f64>())
        .max(1e-300);
    let relative = (emitted - assigned).abs() / maxscale;
    if relative > 2.0e-10 {
        return Err(CoreError::InvalidInput(
            "source-clock photon derivative ledger failure".into(),
        ));
    }
    Ok(relative)
}

fn problem(s: Arc<Source>, forced: bool, k: usize) -> Result<OdeProblem, CoreError> {
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);
    OdeProblem::new(
        "original-REI-five-clock-native-corrected-nonautonomy",
        DIM,
        Arc::new(move |u, y, out| {
            s1.rhs(u, y, out, !forced);
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |u, y, v, out| {
            s2.jvp(u, y, v, out, !forced);
            Ok(())
        })),
        Some(Arc::new(move |u, y, out| {
            if forced {
                partial_u_segment(&s3, k, u, y, out)?;
            } else {
                out.fill(0.);
            }
            Ok(())
        })),
        !forced,
        None,
        None,
    )
}

// Post-hoc adversarial control. The actual old PR175 forced RHS used this
// incorrectly autonomous declaration. On the same knot-split output grid,
// it isolates clock partitioning from the nonautonomous f_t correction.
fn legacy_misdeclared_problem(s: Arc<Source>) -> Result<OdeProblem, CoreError> {
    let sr = Arc::clone(&s);
    let sj = Arc::clone(&s);
    OdeProblem::new(
        "original-REI-forced-declared-autonomous-control",
        DIM,
        Arc::new(move |u, y, out| {
            sr.rhs(u, y, out, false);
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |u, y, v, out| {
            sj.jvp(u, y, v, out, false);
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
}

fn run_trial(
    s: Arc<Source>,
    samples: &[f64],
    reference: &Value,
    forced: bool,
    method: &str,
    rtol: f64,
) -> Result<Value, String> {
    let atol = (rtol * 0.001).max(2e-14);
    let mut y = vec![0.; DIM];
    y[0..3].fill(1.);
    let mut collected: Vec<Option<Vec<f64>>> = vec![None; samples.len()];
    let mut segments = Vec::new();
    let segment_list: Vec<(usize, f64, f64)> = if forced {
        (0..4).map(|i| (i, s.times[i], s.times[i + 1])).collect()
    } else {
        vec![(0, 0., 1.)]
    };
    let mut max_inv: f64 = 0.;
    let mut max_physical_error: f64 = 0.;
    let mut observed_count = 0_usize;
    let mut total_wall = 0.;
    for (i, t0, t1) in segment_list {
        let mut times = vec![t0, t1];
        times.extend(samples.iter().copied().filter(|t| *t > t0 && *t < t1));
        times.sort_by(|a, b| a.total_cmp(b));
        times.dedup_by(|a, b| a.to_bits() == b.to_bits());
        let schedule = OutputSchedule::new(times.clone()).map_err(|e| e.to_string())?;
        let adaptive = AdaptiveStepConfig {
            rtol,
            atol,
            initial_step: 0.01_f64.min(t1 - t0),
            max_step: 0.1_f64.min(t1 - t0),
            max_attempts: 20_000,
            ..Default::default()
        };
        let before = Instant::now();
        let (observed, method_extra) = match method {
            "rodas5p-exact-ft" => {
                let p = problem(Arc::clone(&s), forced, i).map_err(|e| e.to_string())?;
                let linear = LinearSolverConfig {
                    method: LinearMethod::Gmres,
                    ..Default::default()
                };
                let r = integrate_sequential_matrix_free_adaptive_dense_observed(
                    &p,
                    (t0, t1),
                    &y,
                    &linear,
                    &adaptive,
                    &OutputSamplingPlan::dense(schedule),
                )
                .map_err(|e| e.to_string())?;
                (
                    r.observed,
                    json!({"kind":"8-stage-rodas5p","model_autonomous":!forced,
                                  "analytic_time_derivative":forced,
                                  "runtime_diagnostics":r.diagnostics}),
                )
            }
            "rodas5p-legacy-zero-ft" => {
                if !forced {
                    return Err(
                        "legacy negative control only valid on forced original source".into(),
                    );
                }
                let p = legacy_misdeclared_problem(Arc::clone(&s)).map_err(|e| e.to_string())?;
                let linear = LinearSolverConfig {
                    method: LinearMethod::Gmres,
                    ..Default::default()
                };
                let r = integrate_sequential_matrix_free_adaptive_dense_observed(
                    &p,
                    (t0, t1),
                    &y,
                    &linear,
                    &adaptive,
                    &OutputSamplingPlan::dense(schedule),
                )
                .map_err(|e| e.to_string())?;
                (
                    r.observed,
                    json!({"kind":"POSTHOC_NEGATIVE_CONTROL_OLD_AUTONOMY_DECLARATION",
                    "model_autonomous_declared":true,"actual_source_timevarying":true,
                    "time_derivative_deliberately_omitted":true,
                    "runtime_diagnostics":r.diagnostics}),
                )
            }
            "dopri5-rhs-only" => {
                let s_rhs = Arc::clone(&s);
                let rhs = move |u: f64, p: &[f64], out: &mut [f64]| {
                    s_rhs.rhs(u, p, out, !forced);
                    Ok(())
                };
                let cfg = Dopri5Config {
                    adaptive,
                    ..Default::default()
                };
                let r = integrate_dopri5_dense_observed(&rhs, (t0, t1), &y, &cfg, &schedule)
                    .map_err(|e| e.to_string())?;
                let extra = json!({"kind":"rhs-only-dopri5","attempts":r.attempts,
                                 "accepted_steps":r.accepted_steps,
                                 "rejected_steps":r.rejected_steps,
                                 "fsal_reuses":r.fsal_reuses,
                                 "stop_reason":format!("{:?}",r.stop_reason)});
                (r.observed, extra)
            }
            _ => return Err("unknown original VigilODE method".into()),
        };
        total_wall += before.elapsed().as_secs_f64();
        if !observed.success || observed.t.len() != times.len() || observed.y.len() != times.len() {
            return Err(format!(
                "incomplete original source segment {i}: success={} output={}/{}",
                observed.success,
                observed.y.len(),
                times.len()
            ));
        }
        for (j, &time) in observed.t.iter().enumerate() {
            if time.to_bits() != times[j].to_bits() {
                return Err(format!(
                    "original output-grid mismatch on segment {i}, output {j}"
                ));
            }
            let state = &observed.y[j];
            if state.len() != DIM
                || state.iter().any(|v| !v.is_finite())
                || state[..3].iter().any(|v| *v <= 0.)
            {
                return Err(format!(
                    "nonfinite or nonpositive original chemistry on segment {i}"
                ));
            }
            max_inv = max_inv.max(
                s.invariant(state)
                    .iter()
                    .map(|x| x.abs())
                    .fold(0., f64::max),
            );
            if let Some(k) = samples.iter().position(|x| x.to_bits() == time.to_bits()) {
                if let Some(old) = &collected[k] {
                    let diff = old
                        .iter()
                        .zip(state)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0., f64::max);
                    if diff > 1e-12 {
                        return Err("source knot continuation state disagreement".into());
                    }
                } else {
                    observed_count += 1;
                }
                collected[k] = Some(state.clone());
            }
        }
        y = observed
            .y
            .last()
            .cloned()
            .ok_or("no accepted source endpoint")?;
        segments.push(json!({
            "segment":i,"t0":t0,"t1":t1,
            "observed_times":observed.t,"work":observed.counters,
            "driver":method_extra
        }));
    }
    if observed_count != samples.len() || collected.iter().any(Option::is_none) {
        return Err(format!(
            "missing source reference output: {observed_count}/{}",
            samples.len()
        ));
    }
    let mut trajectory = Vec::new();
    for (i, part) in collected.into_iter().enumerate() {
        let part = part.ok_or("incomplete trajectory")?;
        for (j, value) in part.iter().enumerate() {
            let target = reference[i][j]
                .as_f64()
                .ok_or("malformed original independent reference")?;
            max_physical_error = max_physical_error.max((value - target).abs());
        }
        trajectory.push(part);
    }
    let accepted_original_budget =
        max_physical_error <= ORIGINAL_ERROR_BUDGET && max_inv <= INVARIANT_BUDGET;
    let last = trajectory.last().ok_or("no original endpoint")?;
    let sum_ionization = (0..3).map(|q| last[3 + q] * s.s0[q] / s.s0[0]).sum::<f64>();
    let sum_escaped = (6..10).map(|q| last[q]).sum::<f64>();
    let sum_emitted = (10..14).map(|q| last[q]).sum::<f64>();
    let final_electrons_per_initial_hi = sum_ionization;
    let report = json!({
        "method":method,"forced":forced,"rtol":rtol,"atol":atol,
        "stage_count":segments.len(),"complete_requested_observations":observed_count,
        "original_physical_source_sample_count":samples.len(),
        "max_native_minus_independent_Radau_abs":max_physical_error,
        "max_photon_nuclei_binding_invariant":max_inv,
        "source_original_scoped_budget":ORIGINAL_ERROR_BUDGET,
        "source_original_scoped_pass":accepted_original_budget,
        "scientific_declaration_valid":method!="rodas5p-legacy-zero-ft",
        "posthoc_adversarial_negative_control":method=="rodas5p-legacy-zero-ft",
        "final_ionized_electrons_per_original_HI":final_electrons_per_initial_hi,
        "final_escaped_photon_count_per_HI":sum_escaped,
        "final_emitted_photon_count_per_HI":sum_emitted,
        "final_ionization_binding_eV_per_HI":last[14],
        "segment_diagnostic_wall_seconds":total_wall,
        "wall_time_authority":"DIAGNOSTIC_ONLY",
        "samples_u":samples,"trajectory_15state":trajectory,
        "segments":segments
    });
    Ok(report)
}

fn study() -> Result<Value, Box<dyn std::error::Error>> {
    let s = Arc::new(Source::new());
    let refdata: Value = serde_json::from_str(REFERENCE)?;
    let samples: Vec<f64> = refdata["samples_u"]
        .as_array()
        .ok_or("reference missing sample times")?
        .iter()
        .map(|v| v.as_f64().ok_or("invalid source time"))
        .collect::<Result<_, _>>()?;
    if samples.len() != 11
        || samples[0] != s.times[0]
        || samples[10] != s.times[4]
        || !s
            .times
            .iter()
            .all(|k| samples.iter().any(|x| x.to_bits() == k.to_bits()))
    {
        return Err("original source/referenced time grid identity changed".into());
    }
    let mut runs = Vec::new();
    let mut good = 0_usize;
    for forced in [true, false] {
        let original = if forced {
            &refdata["forced_states"]
        } else {
            &refdata["frozen_states"]
        };
        for rtol in [2e-9_f64, 2e-11] {
            for method in ["rodas5p-exact-ft", "dopri5-rhs-only"] {
                match run_trial(Arc::clone(&s), &samples, original, forced, method, rtol) {
                    Ok(report) => {
                        if report["source_original_scoped_pass"].as_bool() == Some(true) {
                            good += 1;
                        }
                        runs.push(report);
                    }
                    Err(message) => {
                        runs.push(json!({"method":method,"forced":forced,"rtol":rtol,
                           "status":"METHOD_FAILED_OR_REJECTED",
                           "reason":message,"source_original_scoped_pass":false}));
                    }
                }
            }
        }
    }
    // Single post-hoc falsification: original misdeclared f_t=0, now split
    // at the exact same source knots. This diagnoses derivative versus
    // partition effects and is not among the eight preregistered candidates.
    match run_trial(
        Arc::clone(&s),
        &samples,
        &refdata["forced_states"],
        true,
        "rodas5p-legacy-zero-ft",
        2e-11,
    ) {
        Ok(x) => runs.push(x),
        Err(message) => runs.push(json!({"method":"rodas5p-legacy-zero-ft",
             "posthoc_adversarial_negative_control":true,"reason":message,
             "status":"CONTROL_FAILED_PRESERVED"})),
    }
    Ok(json!({
        "schema":"VIGILODE_ORIGINAL_REI_15STATE_NONAUTON_FT_METHODS_V1",
        "preregistered_candidate_count":8,
        "posthoc_adversarial_control_count":1,
        "science_scope":"ORIGINAL_46K_SOURCE_MOMENT_REDUCED_15STATE_ONLY_NOT_26EVENT",
        "source_reuse":"PR175_NATIVE_RHS_JVP_REF_BYTE_UNCHANGED",
        "source_knot_count":s.times.len(),
        "source_segment_count":4,
        "original_groups":4,"original_neutral_moments":3,
        "source_state_reference_times":11,"source_first_clock":s.times[0],
        "source_last_clock":s.times[4],
        "corrected_rodas_time_declaration":"NONAUTONOMOUS_ANALYTIC_FT_PER_SEGMENT",
        "old_PR175_timevarying_declaration":"AUTONOMOUS_TRUE_FT_ZERO_ERROR",
        "original_frozen_rodas":"AUTONOMOUS_CORRECT",
        "original_frozen_5e10_budget":ORIGINAL_ERROR_BUDGET,
        "source_scoped_successful_routes":good,
        "attempted_routes":runs.len(),"runs":runs,
        "original_26event": "NOT_RUN","original_tick161":"NOT_ACCEPTED",
        "vigilode_original_DAG":"7/23_REPORTED","vigilode_successor":"0/12",
        "rei_central":"28/88_REPORTED","rei_working":"33/88_REPORTED",
        "rei_fast":"4/7_REPORTED","canonical_delta":0,
        "production":"HOLD","no_auto_next_node":true
    }))
}
pub(super) fn cli() -> ExitCode {
    match study() {
        Ok(report) => {
            let mut out = std::io::stdout().lock();
            if serde_json::to_writer_pretty(&mut out, &report).is_err() {
                return ExitCode::FAILURE;
            }
            let _ = out.write_all(b"\n");
            let success = report["source_scoped_successful_routes"]
                .as_u64()
                .unwrap_or(0)
                >= 2;
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("REI original-source nonautonomous study failed: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_original_15state_analytic_time_derivative_matches_fixed_parent_fd() {
        let source = Source::new();
        let mut state = [0.; DIM];
        state[0..3].copy_from_slice(&[0.993, 0.982, 1.002]);
        let mut maximal_rel = 0.0_f64;
        for k in 0..4 {
            let lo = source.times[k];
            let hi = source.times[k + 1];
            let u = (lo + hi) * 0.5;
            // Source derivative is ~1e-8; too-small finite differences lose
            // significant digits in the ~1e-2 escape RHS (observed 2.8e-6
            // normalized cancellation at 1e-5). Use the source-smooth 1e-3
            // step, independently checked across all four original segments.
            let eps = (hi - lo) * 1e-3;
            let mut jet = [0.; DIM];
            let balance = partial_u_segment(&source, k, u, &state, &mut jet).unwrap();
            assert!(
                balance < 2e-10,
                "photon derivative ledger group segment={k}: {balance}"
            );
            let mut plus = [0.; DIM];
            let mut minus = [0.; DIM];
            source.rhs(u + eps, &state, &mut plus, false);
            source.rhs(u - eps, &state, &mut minus, false);
            let scale = jet.iter().map(|v| v.abs()).fold(0., f64::max).max(1e-16);
            for (j, analytic) in jet.iter().enumerate() {
                let empirical = (plus[j] - minus[j]) / (2. * eps);
                maximal_rel = maximal_rel.max((analytic - empirical).abs() / scale);
            }
        }
        assert!(
            maximal_rel < 2e-6,
            "analytic full clock derivative mismatch: {maximal_rel}"
        );
    }

    #[test]
    fn negative_time_derivative_branch_fails_closed_and_original_static_exact_zero() {
        let source = Arc::new(Source::new());
        let mut y = [0.; DIM];
        y[0..3].fill(1.);
        let mut ft = [0.; DIM];
        assert!(partial_u_segment(&source, 4, 0.5, &y, &mut ft).is_err());
        assert!(partial_u_segment(&source, 0, source.times[2], &y, &mut ft).is_err());
        let frozen = problem(Arc::clone(&source), false, 0).unwrap();
        let mut counters = rodas5p_core::WorkCounters::default();
        assert_eq!(
            frozen.eval_partial_t(0.5, &y, &mut counters).unwrap(),
            vec![0.; DIM]
        );
        let forced = problem(source, true, 2).unwrap();
        assert!(!forced.autonomous);
    }
}
