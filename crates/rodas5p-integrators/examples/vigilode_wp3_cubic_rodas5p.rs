//! Research-only WP3: original eight-stage fifth-order RODAS5P on WP1 frozen cubic IVPs.
//! Uses the existing native library and analytic JVP. Does not change solver defaults.
//! All candidate and failure outputs remain raw JSON lines; performance is hardware-scoped.
use std::{hint::black_box, sync::Arc, time::Instant};
use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, OutputSamplingPlan,
    integrate_sequential_matrix_free_adaptive_dense_observed,
};
use serde_json::json;

#[derive(Clone)]
struct OriginalSource {
    name: &'static str,
    k: Vec<f64>,
    g: Vec<f64>,
    c: Vec<Vec<f64>>,
    y0: Vec<f64>,
    steps: usize,
}
fn cases() -> Vec<OriginalSource> {
    let a = OriginalSource {
        name: "asymmetric_2", k: vec![4., 7.], g: vec![20., 40.],
        c: vec![vec![0., 12.], vec![8., 0.]], y0: vec![0.2, 0.12], steps: 64,
    };
    let mut ar = a.clone();
    ar.name = "asymmetric_2_refined";
    ar.steps = 128;
    let b = OriginalSource {
        name: "coupled_3_cubic", k: vec![6., 8., 10.], g: vec![60., 100., 30.],
        c: vec![vec![0., 30., 4.], vec![15., 0., 10.], vec![10., 8., 0.]],
        y0: vec![0.04, 0.06, 0.02], steps: 128,
    };
    let l = OriginalSource {
        name: "linear_exchange_control", k: vec![4., 4.], g: vec![0., 0.],
        c: vec![vec![0., 6.], vec![6., 0.]], y0: vec![0.1, 0.05], steps: 128,
    };
    vec![a, ar, b, l]
}
fn rhs(s: &OriginalSource, y: &[f64], out: &mut [f64]) {
    for i in 0..y.len() {
        let mut x = -s.k[i] * y[i] - s.g[i] * y[i] * y[i] * y[i];
        for j in 0..y.len() {
            if i != j { x += s.c[i][j] * (y[j] - y[i]); }
        }
        out[i] = x;
    }
}
fn jvp(s: &OriginalSource, y: &[f64], v: &[f64], out: &mut [f64]) {
    for i in 0..y.len() {
        let mut x = -(s.k[i] + 3. * s.g[i] * y[i] * y[i]) * v[i];
        for j in 0..y.len() {
            if i != j { x += s.c[i][j] * (v[j] - v[i]); }
        }
        out[i] = x;
    }
}
fn source_bits(s: &OriginalSource) -> Vec<u64> {
    let mut b = Vec::new();
    b.extend(s.k.iter().map(|x| x.to_bits()));
    b.extend(s.g.iter().map(|x| x.to_bits()));
    b.extend(s.y0.iter().map(|x| x.to_bits()));
    for row in &s.c { b.extend(row.iter().map(|x| x.to_bits())); }
    b
}
fn problem(s: &OriginalSource) -> Result<OdeProblem, Box<dyn std::error::Error>> {
    let s = Arc::new(s.clone());
    let sr = Arc::clone(&s);
    let sj = Arc::clone(&s);
    Ok(OdeProblem::new(
        "wp3-original-cooperative-cubic",
        s.k.len(),
        Arc::new(move |_t, y, out| { rhs(&sr, y, out); Ok(()) }),
        None,
        None,
        Some(Arc::new(move |_t, y, v, out| { jvp(&sj, y, v, out); Ok(()) })),
        None,
        true,
        None,
        None,
    )?)
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let rtols = [1e-4_f64, 1e-6, 1e-8, 1e-10];
    for s in cases() {
        let h = 0.1_f64 / s.steps as f64;
        assert_eq!(h * s.steps as f64, 0.1_f64);
        let labels = (0..=s.steps).map(|i| h * i as f64).collect::<Vec<_>>();
        let schedule = OutputSchedule::new(labels.clone())?;
        let ode = problem(&s)?;
        let linear = LinearSolverConfig { method: LinearMethod::Gmres, ..Default::default() };
        println!("{}", json!({
            "kind": "WP3_CASE", "name": s.name, "source_bits": source_bits(&s),
            "initial_state_bits": s.y0.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
            "h_bits": h.to_bits(), "steps": s.steps, "horizon_bits": 0.1_f64.to_bits(),
            "method": "original-native-eight-stage-RODAS5P-fifth-order-JVP-only",
        }));
        for (index, rtol) in rtols.iter().enumerate() {
            let atol = rtol * 1e-2;
            let cfg = AdaptiveStepConfig {
                rtol: *rtol, atol, initial_step: h, max_step: 0.1,
                max_attempts: 100000, ..Default::default()
            };
            let mut timings = Vec::new();
            let mut first = None;
            for _ in 0..3 {
                let start = Instant::now();
                let result = integrate_sequential_matrix_free_adaptive_dense_observed(
                    &ode, (0., 0.1), &s.y0, &linear, &cfg,
                    &OutputSamplingPlan::dense(schedule.clone()),
                )?;
                timings.push(start.elapsed().as_nanos());
                black_box(&result);
                if first.is_none() { first = Some(result); }
            }
            let result = first.expect("three original native integrations");
            let complete = result.observed.success && result.observed.t == labels;
            let state_bits: Vec<Vec<u64>> = result.observed.y.iter()
                .map(|y| y.iter().map(|v| v.to_bits()).collect()).collect();
            let time_bits: Vec<u64> = result.observed.t.iter().map(|v| v.to_bits()).collect();
            println!("{}", json!({
                "kind": "WP3_NATIVE_TRACE", "name": s.name,
                "method_id": "RODAS5P_native_original", "rtol_index": index,
                "rtol_bits": rtol.to_bits(), "atol_bits": atol.to_bits(),
                "success": result.observed.success, "complete_output": complete,
                "message": result.observed.message, "time_bits": time_bits,
                "state_bits": state_bits, "wall_ns_raw": timings,
                "counters": result.observed.counters,
                "diagnostics": result.diagnostics,
                "method_quality": "ORIGINAL_NATIVE_API_NOT_EXTERNAL_SURROGATE",
            }));
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("WP3_NATIVE_ERROR: {e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_frozen_source_dimensions_and_bits() {
        let c = cases();
        assert_eq!(c.len(), 4);
        assert_eq!(c[0].steps, 64);
        assert_eq!(c[1].steps, 128);
        assert_eq!(c[2].k.len(), 3);
        for s in &c { assert_eq!(source_bits(s).len(), 3*s.k.len()+s.k.len()*s.k.len()); }
    }
    #[test]
    fn analytic_jvp_matches_centered_finite_difference() {
        for s in cases() {
            let n = s.k.len();
            let v = vec![0.11_f64; n];
            let mut j = vec![0.; n];
            jvp(&s, &s.y0, &v, &mut j);
            let eps = 1e-6;
            let yp = s.y0.iter().zip(&v).map(|(a,b)|a+eps*b).collect::<Vec<_>>();
            let ym = s.y0.iter().zip(&v).map(|(a,b)|a-eps*b).collect::<Vec<_>>();
            let mut fp = vec![0.; n];
            let mut fm = vec![0.; n];
            rhs(&s, &yp, &mut fp);
            rhs(&s, &ym, &mut fm);
            for i in 0..n { assert!((j[i]-(fp[i]-fm[i])/(2.*eps)).abs() < 1e-8); }
        }
    }
}
