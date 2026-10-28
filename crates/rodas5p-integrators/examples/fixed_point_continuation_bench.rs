//! Small mathematical callback comparison; not an ODE campaign.
use rodas5p_integrators::fixed_point_continuation::*;
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;

struct Cubic {
    n: usize,
    coefficient: f64,
}
impl Cubic {
    fn target(&self, i: usize, lambda: f64) -> f64 {
        0.25 + 0.5 * lambda + 0.0078125 * (i as f64 / self.n as f64)
    }
    fn scalar(&self, x: f64, i: usize, lambda: f64) -> f64 {
        x + self.coefficient * x * x * x - self.target(i, lambda)
    }
    fn atlas(&self) -> DeclaredFixedPointAtlas {
        let patch = DeclaredFixedPointPatch::new(
            self.identity(),
            1,
            21,
            0.0,
            1.0,
            vec![0.5; self.n],
            0.5,
            FixedPointBounds {
                contraction_upper: 3.0 * self.coefficient,
                center_correction_upper: 0.27,
                inverse_action_norm_upper: 1.0,
                inverse_action_inverse_norm_upper: 1.0,
            },
        )
        .unwrap();
        DeclaredFixedPointAtlas::new(self.identity(), vec![patch]).unwrap()
    }
    fn bisection(&self, tolerance: f64) -> (Vec<f64>, usize) {
        let mut lo = vec![0.0; self.n];
        let mut hi = vec![1.0; self.n];
        let mut out = vec![0.0; self.n];
        let mut calls = 0;
        for _ in 0..64 {
            let mut largest = 0.0_f64;
            calls += 1;
            for i in 0..self.n {
                out[i] = 0.5 * (lo[i] + hi[i]);
                let f = self.scalar(out[i], i, 1.0);
                largest = largest.max(f.abs());
                if f < 0.0 {
                    lo[i] = out[i];
                } else {
                    hi[i] = out[i];
                }
            }
            if largest <= tolerance {
                return (out, calls);
            }
        }
        panic!("reference bisection did not converge");
    }
}
impl FixedPointHomotopyProvider for Cubic {
    fn identity(&self) -> ContinuationIdentity {
        ContinuationIdentity {
            target_id: self.coefficient.to_bits(),
            model_epoch: self.n as u64,
            branch_id: 1,
        }
    }
    fn dimension(&self) -> usize {
        self.n
    }
    fn inverse_identity(&self, _: u64) -> u64 {
        21
    }
    fn residual(&self, x: &[f64], lambda: f64, out: &mut [f64]) -> Result<f64, String> {
        let mut allowance = 0.0_f64;
        for i in 0..self.n {
            out[i] = self.scalar(x[i], i, lambda);
            allowance = allowance.max(
                16.0 * f64::EPSILON
                    * (x[i].abs()
                        + self.coefficient * x[i].abs().powi(3)
                        + self.target(i, lambda).abs()),
            );
        }
        Ok(allowance)
    }
    fn apply_inverse(&self, _: u64, input: &[f64], output: &mut [f64]) -> Result<f64, String> {
        output.copy_from_slice(input);
        Ok(0.0)
    }
}
fn median(a: &mut [f64]) -> f64 {
    a.sort_by(|a, b| a.total_cmp(b));
    a[a.len() / 2]
}
fn main() {
    let repeats = 101;
    for n in [1, 128] {
        for coefficient in [0.015625, 0.0625, 0.125] {
            let model = Cubic { n, coefficient };
            let atlas = model.atlas();
            let initial = vec![0.25; n];
            let (reference, _) = model.bisection(4e-15);
            let mut fixed_times = Vec::new();
            let mut bisect_times = Vec::new();
            let mut fixed_work = FixedPointContinuationWork::default();
            let mut bisect_calls = 0;
            let mut final_error = 0.0_f64;
            let mut final_residual = 0.0_f64;
            for repeat in 0..repeats {
                for slot in 0..2 {
                    if (slot + repeat) % 2 == 0 {
                        let then = Instant::now();
                        let mut out = vec![0.0; n];
                        let mut work = FixedPointContinuationWork::default();
                        let report = continue_fixed_point(
                            black_box(&model),
                            &atlas,
                            &Default::default(),
                            &initial,
                            &mut out,
                            &mut work,
                        )
                        .unwrap();
                        fixed_times.push(then.elapsed().as_secs_f64());
                        final_residual = final_residual.max(report.final_residual_upper);
                        final_error = final_error.max(
                            out.iter()
                                .zip(&reference)
                                .map(|(x, y)| (x - y).abs())
                                .fold(0.0, f64::max),
                        );
                        fixed_work = work;
                    } else {
                        let then = Instant::now();
                        let (out, calls) = black_box(&model).bisection(1e-10);
                        bisect_times.push(then.elapsed().as_secs_f64());
                        black_box(out);
                        bisect_calls = calls;
                    }
                }
            }
            let fixed = median(&mut fixed_times);
            let bisect = median(&mut bisect_times);
            println!(
                "{}",
                json!({"schema_version":1,"problem":"independent_monotone_cubic_roots","dimension":n,
                "coefficient":coefficient,"repeats":repeats,"tolerance":1e-10,"fixed_p_seconds_median":fixed,
                "bisection_seconds_median":bisect,"bisection_over_fixed_p":bisect/fixed,
                "fixed_p_work":fixed_work,"bisection_final_only_vector_f_calls":bisect_calls,
                "max_final_original_residual_upper":final_residual,"max_error_to_tight_bisection":final_error,
                "pass":final_residual<=1e-10&&final_error<=1e-10,
                "scope":"fixed-P follows lambda=0 and lambda=1; bisection solves final root only; both include solve allocations, immutable model/atlas setup excluded; separable structure allows stronger component-specific solvers"})
            );
        }
    }
}
