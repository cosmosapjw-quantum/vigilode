//! Frozen shifted-periodic-diffusion stage experiment. The operator counts
//! every product, including each trigger's independent original-residual check.
use rodas5p_core::{
    CoreResult, LinearOperator, Preconditioner, PreconditionerSide, WorkCounters, safe_l2,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresMonitorDecision, GmresWorkspace, solve_gmres_into,
    solve_gmres_into_right_flexible_triggered_monitored,
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ShiftedDiffusion {
    n: usize,
    theta: f64,
    products: AtomicUsize,
}
impl LinearOperator for ShiftedDiffusion {
    fn dimension(&self) -> usize {
        self.n
    }
    fn token(&self) -> u64 {
        self.theta.to_bits()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.products.fetch_add(1, Ordering::Relaxed);
        for i in 0..self.n {
            let left = (i + self.n - 1) % self.n;
            let right = (i + 1) % self.n;
            y[i] = x[i] + self.theta * (2.0 * x[i] - x[left] - x[right]);
        }
        Ok(())
    }
}
struct RightIdentity(usize);
impl Preconditioner for RightIdentity {
    fn dimension(&self) -> usize {
        self.0
    }
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y.copy_from_slice(x);
        Ok(())
    }
}

#[test]
fn compare_shifted_diffusion_at_common_residual_tolerance() -> CoreResult<()> {
    let n = 64;
    let x0 = vec![0.0; n];
    let exact: Vec<f64> = (0..n)
        .map(|i| (i as f64 * 0.37).cos() + if i < n / 3 { 0.75 } else { 0.0 })
        .collect();
    let config = GmresConfig {
        restart: 20,
        max_arnoldi: 220,
        rtol: 1e-10,
        atol: 0.0,
    };
    for theta in [0.05, 1.0, 5.0, 20.0] {
        let op = ShiftedDiffusion {
            n,
            theta,
            products: AtomicUsize::new(0),
        };
        let pc = RightIdentity(n);
        let mut rhs = vec![0.0; n];
        op.apply(&exact, &mut rhs)?;
        let limit = config.rtol * safe_l2(&rhs);
        for triggered in [false, true] {
            let mut x = vec![0.0; n];
            let mut residual = vec![0.0; n];
            let mut workspace = GmresWorkspace::default();
            let mut counters = WorkCounters::default();
            op.products.store(0, Ordering::Relaxed);
            let iterations = if triggered {
                let report = solve_gmres_into_right_flexible_triggered_monitored(
                    &op,
                    &pc,
                    &rhs,
                    &x0,
                    &config,
                    limit,
                    GmresCapacity::unbounded(),
                    &mut x,
                    &mut workspace,
                    &mut counters,
                    &mut |_, candidate, _| {
                        op.apply(candidate, &mut residual)?;
                        for (r, b) in residual.iter_mut().zip(&rhs) {
                            *r -= b;
                        }
                        Ok(if safe_l2(&residual) <= limit {
                            GmresMonitorDecision::Accept
                        } else {
                            GmresMonitorDecision::Continue
                        })
                    },
                )?;
                assert!(report.accepted);
                report.iterations
            } else {
                solve_gmres_into(
                    &op,
                    &pc,
                    &rhs,
                    Some(&x0),
                    &config,
                    None,
                    &mut x,
                    &mut workspace,
                    GmresCapacity::unbounded(),
                    &mut counters,
                )?
                .iterations
            };
            let products = op.products.load(Ordering::Relaxed);
            op.apply(&x, &mut residual)?;
            for (r, b) in residual.iter_mut().zip(&rhs) {
                *r -= b;
            }
            let residual_norm = safe_l2(&residual);
            let solution_error =
                safe_l2(&x.iter().zip(&exact).map(|(a, b)| a - b).collect::<Vec<_>>());
            // Symmetric W has smallest eigenvalue 1.
            assert!(residual_norm <= limit);
            assert!(solution_error <= limit + 1e-13);
            println!(
                "N02_STAGE theta={theta} triggered={triggered} iterations={iterations} products={products} relative_residual={} error_l2={solution_error}",
                residual_norm / safe_l2(&rhs)
            );
        }
    }
    Ok(())
}
