//! Complete dense RODAS trajectory with identical observation points and an
//! analytic solution. Source-compatible with the pre-optimization baseline.
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
};
use serde_json::json;
use std::{hint::black_box, sync::Arc, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: mode_dense_trajectory_20261027 N REPS".into());
    }
    let n: usize = args[0].parse()?;
    let reps: usize = args[1].parse()?;
    if ![65, 96, 192].contains(&n) || reps == 0 || reps > 10000 {
        return Err("outside bounded experiment".into());
    }
    faer::set_global_parallelism(faer::Par::Seq);
    let (lambda, kappa, tf) = (200.0, 50.0, 0.2);
    let mut mat = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            mat[(i, j)] = kappa / n as f64;
        }
        mat[(i, i)] -= lambda;
    }
    let mat = Arc::new(mat);
    let rm = mat.clone();
    let jm = mat.clone();
    let ji = mat.clone();
    let p = OdeProblem::new(
        format!("dense-rank1-{n}"),
        n,
        Arc::new(move |_t, y, f| rm.matvec_into(y, f)),
        None,
        Some(Arc::new(move |_t, _y| Ok((*jm).clone()))),
        None,
        None,
        true,
        None,
        None,
    )?
    .with_jacobian_into(Arc::new(move |_t, _y, out| {
        out.as_mut_slice().copy_from_slice(ji.as_slice());
        Ok(())
    }));
    let y0: Vec<_> = (0..n).map(|i| 1.0 + 0.25 * (i as f64).sin()).collect();
    let mean = y0.iter().sum::<f64>() / n as f64;
    let times: Vec<_> = (0..=32).map(|i| tf * i as f64 / 32.0).collect();
    let schedule = OutputSchedule::new(times)?;
    let cfg = AdaptiveStepConfig {
        atol: 1e-10,
        rtol: 1e-7,
        initial_step: 1e-5,
        min_step: 1e-14,
        max_step: tf,
        max_attempts: 10000,
        ..Default::default()
    };
    let run = || integrate_rodas5p_fast_observed(&p, (0.0, tf), &y0, &cfg, &schedule);
    let probe = run()?;
    if !probe.observed.success {
        return Err(probe.observed.message.into());
    }
    let mut max_error: f64 = 0.0;
    for (t, y) in probe.observed.t.iter().zip(&probe.observed.y) {
        for (yn, yinit) in y.iter().zip(&y0) {
            let exact = (-lambda * t).exp() * (yinit - mean) + ((kappa - lambda) * t).exp() * mean;
            max_error = max_error.max((yn - exact).abs());
        }
    }
    if max_error > 1e-7 {
        return Err("trajectory accuracy failure".into());
    }
    let start = Instant::now();
    let mut last = None;
    for _ in 0..reps {
        last = Some(black_box(run()?));
    }
    let seconds = start.elapsed().as_secs_f64();
    let last = last.unwrap();
    assert_eq!(last.observed.y, probe.observed.y);
    println!(
        "{}",
        json!({"n":n,"reps":reps,"seconds":seconds,
        "seconds_per_trajectory":seconds/reps as f64,"max_grid_error":max_error,
        "cap":1e-7,"times":probe.observed.t,"states":probe.observed.y,
        "counters":probe.observed.counters,"attempts":probe.attempts,
        "lu":format!("{:?}",probe.lu),"rtol":cfg.rtol,"atol":cfg.atol,
        "scope":"full setup, integration, observation, counters; 33 fixed times"})
    );
    Ok(())
}
