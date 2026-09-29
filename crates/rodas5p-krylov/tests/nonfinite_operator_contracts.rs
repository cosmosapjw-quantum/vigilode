//! A custom operator that writes NaN must not be certified as a zero residual
//! (external audit VIG-A04, 2026-09-29).

use rodas5p_core::{ClosureOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{GmresConfig, solve_gmres};

fn nan_operator(
    pattern: fn(usize) -> f64,
) -> ClosureOperator<impl Fn(&[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync> {
    ClosureOperator::new(2, move |_: &[f64], out: &mut [f64]| {
        for (i, value) in out.iter_mut().enumerate() {
            *value = pattern(i);
        }
        Ok(())
    })
}

#[test]
fn a_nan_operator_output_is_not_a_zero_true_residual() {
    let patterns: [fn(usize) -> f64; 3] = [
        |_| f64::NAN,
        |i| if i == 0 { 0.0 } else { f64::NAN },
        |i| if i == 0 { 1.0 } else { f64::NAN },
    ];
    for (index, pattern) in patterns.into_iter().enumerate() {
        for x0 in [Some([1.0, 1.0]), None] {
            let result = solve_gmres(
                &nan_operator(pattern),
                &IdentityPreconditioner::new(2),
                &[1.0, 1.0],
                x0.as_ref().map(|x| x.as_slice()),
                &GmresConfig::default(),
                &mut WorkCounters::default(),
            );
            assert!(
                result.is_err(),
                "pattern {index}, x0 {x0:?}: NaN output certified: {result:?}"
            );
        }
    }
}
