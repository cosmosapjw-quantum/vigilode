//! Compile through run_bench.py so the reference uses the same snapshot.
#[path = "../../crates/vigilode/src/modal_rodas5p.rs"]
mod modal_rodas5p;
use modal_rodas5p::ModalRodas5pStep;
use std::{hint::black_box, time::Instant};
include!(env!("VIGILODE_MODAL_TABLE"));

fn raw(z: f64) -> (f64, f64) {
    let mut u = [0.0; 8];
    let w = G / (1.0 - G * z);
    for i in 0..8 {
        let mut state = 1.0;
        let mut mix = 0.0;
        for j in 0..i {
            state += A[i][j] * u[j];
            mix += C[i][j] * u[j];
        }
        u[i] = w * (z * state + mix);
    }
    let value = B.iter().zip(u).fold(1.0, |sum, (b, v)| sum + b * v);
    (value, u[7])
}

fn products(factors: &[(f64, f64)], state: &[f64], step: &mut [f64], err: &mut [f64]) {
    for (((&(r, e), &y), out), err) in factors.iter().zip(state).zip(step).zip(err) {
        *out = r * y;
        *err = e * y;
    }
}

fn median_us(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn main() {
    let n = 16384;
    let h = 0.25;
    let modes: Vec<f64> = (0..n)
        .map(|i| -4000.0 * ((i as f64) * 0.00113).sin().powi(2))
        .collect();
    let state: Vec<f64> = (0..n).map(|i| ((i as f64) * 0.00451).cos()).collect();
    let mut high = vec![0.; n];
    let mut err = vec![0.; n];
    let plan = ModalRodas5pStep::new(h, &modes).unwrap();
    let raw_plan: Vec<_> = modes.iter().map(|r| raw(r * h)).collect();
    let exp_plan: Vec<_> = modes.iter().map(|r| (r * h).exp()).collect();
    plan.apply_into(&state, &mut high, &mut err).unwrap();
    let mut raw_out = vec![0.; n];
    let mut raw_err = vec![0.; n];
    products(&raw_plan, &state, &mut raw_out, &mut raw_err);
    let mut endpoint_delta = 0.0_f64;
    let mut embedded_delta = 0.0_f64;
    for i in 0..n {
        endpoint_delta = endpoint_delta.max((raw_out[i] - high[i]).abs());
        embedded_delta = embedded_delta.max((raw_err[i] - err[i]).abs());
    }
    assert!(endpoint_delta < 1e-12 && embedded_delta < 1e-12);
    println!(
        "modes {n} h {h} native_raw_endpoint_delta {endpoint_delta:e} native_raw_embedded_delta {embedded_delta:e}"
    );
    let methods = [
        "modal_cold",
        "raw_cold",
        "exp_cold",
        "modal_cached",
        "raw_cached",
        "exp_cached",
    ];
    let mut timings: [Vec<f64>; 6] = Default::default();
    let reps = 16;
    for block in 0..12 {
        for offset in 0..methods.len() {
            let m = (block + offset) % methods.len();
            let start = Instant::now();
            for _ in 0..reps {
                let amp = black_box(&state[..]);
                let step = black_box(h);
                if m != 0 && m != 3 {
                    assert!(amp.iter().fold(true, |valid, y| valid & y.is_finite()));
                }
                match m {
                    0 => ModalRodas5pStep::new(step, black_box(&modes))
                        .unwrap()
                        .apply_into(amp, &mut high, &mut err)
                        .unwrap(),
                    1 => {
                        let p: Vec<_> = black_box(&modes).iter().map(|&r| raw(r * step)).collect();
                        products(&p, amp, &mut high, &mut err);
                    }
                    2 => {
                        let p: Vec<_> = black_box(&modes)
                            .iter()
                            .map(|&r| (r * step).exp())
                            .collect();
                        for ((&f, &y), out) in p.iter().zip(amp).zip(&mut high) {
                            *out = f * y;
                        }
                    }
                    3 => plan.apply_into(amp, &mut high, &mut err).unwrap(),
                    4 => products(&raw_plan, amp, &mut high, &mut err),
                    _ => {
                        for ((&f, &y), out) in exp_plan.iter().zip(amp).zip(&mut high) {
                            *out = f * y;
                        }
                    }
                }
                black_box(&high);
                black_box(&err);
            }
            if block > 0 {
                timings[m].push(start.elapsed().as_secs_f64() * 1e6 / (reps as f64));
            }
        }
    }
    for (method, samples) in methods.into_iter().zip(timings) {
        println!(
            "{method} median_us {:.4} samples_us {:?}",
            median_us(samples.clone()),
            samples
        );
    }
}
