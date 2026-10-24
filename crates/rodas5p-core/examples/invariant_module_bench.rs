//! Small certified linear-kernel discrimination, not an ODE campaign.
use rodas5p_core::invariant_module::*;
use rodas5p_core::shared_shift_jet::{CertificateStatus, certify_shift_candidate};
use rodas5p_core::{DenseMatrix, LuFactorization};
use std::hint::black_box;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for n in [16usize, 64, 128] {
        let mut b = DenseMatrix::zeros(n, n);
        let mut v = DenseMatrix::zeros(n, 2);
        let mut w = DenseMatrix::zeros(2, n);
        for i in (0..n).step_by(2) {
            b[(i, i)] = -2.;
            b[(i, i + 1)] = 1.;
            b[(i + 1, i + 1)] = -2.;
            v[(i, 0)] = 1.;
            v[(i + 1, 1)] = 1.;
        }
        w[(0, 0)] = 1.;
        w[(1, 1)] = 1.;
        let h = DenseMatrix::new(2, 2, vec![-2., 1., 0., -2.])?;
        let rhs = vec![
            (0..n)
                .map(|i| if i % 2 == 0 { 1. } else { 2. })
                .collect::<Vec<_>>(),
        ];
        let alpha = 0.25;
        let tol = 1e-10;
        let plan = InvariantModulePlan::new(
            1,
            b.clone(),
            v.clone(),
            w.clone(),
            h.clone(),
            InvariantModuleConfig::default(),
        )?;
        let mut a = DenseMatrix::identity(n);
        for i in 0..n {
            for j in 0..n {
                a[(i, j)] -= alpha * b[(i, j)];
            }
        }
        let lu = LuFactorization::new(&a)?;
        let modes = [
            "new_module",
            "reuse_module",
            "fresh_full_lu",
            "reuse_full_lu",
            "direct_jordan_blocks",
        ];
        for repetition in 0..10 {
            for offset in 0..modes.len() {
                let mode = modes[(offset + repetition) % modes.len()];
                let start = Instant::now();
                let (accepted, error, work) = if mode.contains("module") {
                    let fresh;
                    if mode == "new_module" {
                        fresh = Some(InvariantModulePlan::new(
                            1,
                            b.clone(),
                            v.clone(),
                            w.clone(),
                            h.clone(),
                            InvariantModuleConfig::default(),
                        )?);
                    } else {
                        fresh = None;
                    }
                    let p = fresh.as_ref().unwrap_or(&plan);
                    let mut work = InvariantModuleWork::default();
                    let r = p.evaluate(
                        1,
                        &[alpha],
                        &rhs,
                        tol,
                        InvariantModuleSolveBudget::default(),
                        &mut work,
                    )?;
                    (
                        r.candidates[0].accepted_values().is_some(),
                        r.candidates[0].error_l2_upper()[0],
                        Some(work),
                    )
                } else {
                    let values = if mode == "fresh_full_lu" {
                        LuFactorization::new(&a)?.solve_rows(&rhs)?
                    } else if mode == "reuse_full_lu" {
                        lu.solve_rows(&rhs)?
                    } else {
                        let mut x = vec![0.; n];
                        for i in (0..n).step_by(2) {
                            x[i + 1] = rhs[0][i + 1] / (1. + 2. * alpha);
                            x[i] = (rhs[0][i] + alpha * x[i + 1]) / (1. + 2. * alpha);
                        }
                        vec![x]
                    };
                    let certificate = if mode == "fresh_full_lu" {
                        certify_shift_candidate(&b, alpha, 1., &rhs, &values, tol)?
                    } else {
                        plan.certify_candidates(1, alpha, &rhs, &values, tol)?
                    };
                    (
                        certificate.status() == CertificateStatus::Certified,
                        certificate.rhs_error_upper()[0],
                        None,
                    )
                };
                let seconds = start.elapsed().as_secs_f64();
                black_box(error);
                println!(
                    "{}",
                    serde_json::json!({"kind":if repetition==0 {"warmup"}else{"measurement"},"dimension":n,"reduced_dimension":2,"mode":mode,"repetition":repetition,"seconds":seconds,"accepted":accepted,"error_l2_upper":error,"work":work,"timing_scope":"solve and full-space error certificate; new_module includes owned matrix clones and admission, fresh_full_lu includes factorization and dissipativity check; reuse and direct modes share cached dissipativity; preconstructed matrices and RHS excluded"})
                );
                if !accepted {
                    return Err("certification refused".into());
                }
            }
        }
    }
    Ok(())
}
