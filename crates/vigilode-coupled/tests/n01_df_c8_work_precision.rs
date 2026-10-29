//! C8 research-only native fixed-grid work-precision and original raw-U certificate.
//! NO solver default or production change; derivative-free RHS stage is primary candidate.
//! Direct n=3 Jacobian is an independent original-RODAS5P benchmark ONLY.
use rodas5p_core::directed::{Interval, add_up, div_up, mul_up, sub_down};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearSolverConfig, N01MassActionOwner, WorkCounters,
    rodas5p_coefficients,
};
use rodas5p_integrators::{OdeProblem, sequential_step};
use serde_json::json;
use std::sync::Arc;
use vigilode_coupled::n01_derivative_free::N01ValueStage;
use vigilode_coupled::rhs_homotopy::{RhsAcceleration, RhsHomotopyConfig};

const Y0: [f64; 3] = [0.75, 0.125, 0.125];
#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    k: [f64; 3],
    t_end: f64,
}
fn cases() -> [Case; 3] {
    [
        Case {
            id: "mild",
            k: [0.5, 1., 0.5],
            t_end: 0.1,
        },
        Case {
            id: "moderate",
            k: [0.5, 10., 200.],
            t_end: 0.02,
        },
        Case {
            id: "stiff_negative_control",
            k: [0.04, 10_000., 30_000_000.],
            t_end: 0.001,
        },
    ]
}
fn owner(case: Case, y: [f64; 3]) -> CoreResult<N01MassActionOwner> {
    N01MassActionOwner::new(case.k, y, [1.; 3], 0., [1.; 3], [1.; 3], 0.)
}
fn ref_problem(source: Arc<N01MassActionOwner>) -> OdeProblem {
    let rhs_owner = Arc::clone(&source);
    let rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            let z: [f64; 3] = y
                .try_into()
                .map_err(|_| CoreError::Dimension("c8 rhs state".into()))?;
            if out.len() != 3 {
                return Err(CoreError::Dimension("c8 rhs output".into()));
            }
            let mut val = [0.; 3];
            rhs_owner.rhs_at(&z, &mut val)?;
            out.copy_from_slice(&val);
            Ok(())
        },
    );
    let jac_owner = Arc::clone(&source);
    let jac = Arc::new(move |_t: f64, y: &[f64]| -> CoreResult<DenseMatrix> {
        let z: [f64; 3] = y
            .try_into()
            .map_err(|_| CoreError::Dimension("c8 jac state".into()))?;
        let j = jac_owner.jacobian_at(&z)?;
        DenseMatrix::from_rows(&[&j[0], &j[1], &j[2]])
    });
    OdeProblem::new(
        "c8-source-matched-RODAS5P-oracle",
        3,
        rhs,
        None,
        Some(jac),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
fn ip(x: f64) -> Result<Interval, String> {
    Interval::point(x).map_err(|e| e.to_string())
}
fn interval_rhs(k: [f64; 3], z: [Interval; 3]) -> Result<[Interval; 3], String> {
    let u = ip(k[0])?.mul(z[0]).map_err(|e| e.to_string())?;
    let v = ip(k[1])?
        .mul(z[1])
        .and_then(|q| q.mul(z[2]))
        .map_err(|e| e.to_string())?;
    let q = ip(k[2])?
        .mul(z[1])
        .and_then(|q| q.mul(z[1]))
        .map_err(|e| e.to_string())?;
    Ok([
        v.sub(u).map_err(|e| e.to_string())?,
        u.sub(v).and_then(|r| r.sub(q)).map_err(|e| e.to_string())?,
        q,
    ])
}
fn interval_jac(k: [f64; 3], y: [f64; 3]) -> Result<[[Interval; 3]; 3], String> {
    let c = ip(k[1])?.mul(ip(y[2])?).map_err(|e| e.to_string())?;
    let b = ip(k[1])?.mul(ip(y[1])?).map_err(|e| e.to_string())?;
    let bb = ip(2.)?
        .mul(ip(k[2])?)
        .and_then(|v| v.mul(ip(y[1]).unwrap()))
        .map_err(|e| e.to_string())?;
    let z = ip(0.)?;
    let ka = ip(k[0])?;
    Ok([
        [-ka, c, b],
        [ka, (-c).sub(bb).map_err(|e| e.to_string())?, -b],
        [z, bb, z],
    ])
}
// Only the source-specific actual original 8-stage raw-U equations, not projected GMRES diagnostics.
fn certified_raw_stage_bound(
    case: Case,
    y: [f64; 3],
    h: f64,
    stage_k: &[Vec<f64>],
    y_out: &[f64],
) -> Result<(f64, Option<f64>, String), String> {
    let coeffs = rodas5p_coefficients().map_err(|e| e.to_string())?;
    if stage_k.len() != 8 || stage_k.iter().any(|v| v.len() != 3) {
        return Err("stage dimensions".into());
    }
    let mut u = [[0.; 3]; 8];
    for (i, row) in u.iter_mut().enumerate() {
        for (j, k_stage) in stage_k.iter().enumerate().take(i + 1) {
            let gij = if i == j {
                coeffs.gamma
            } else {
                coeffs.gamma_matrix[(i, j)]
            };
            for (k, v) in row.iter_mut().enumerate() {
                *v += gij * k_stage[k];
            }
        }
    }
    let ji = interval_jac(case.k, y)?;
    let tau = ip(h)?.mul(ip(coeffs.gamma)?).map_err(|e| e.to_string())?;
    let mut rho = [0.; 8];
    let mut args = [[ip(0.)?; 3]; 8];
    for (i, stage) in u.iter().enumerate() {
        for k in 0..3 {
            let mut v = ip(y[k])?;
            for (j, prev) in u[..i].iter().enumerate() {
                v = v
                    .add(
                        ip(coeffs.a[(i, j)])?
                            .mul(ip(prev[k])?)
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
            }
            args[i][k] = v;
        }
        let fv = interval_rhs(case.k, args[i])?;
        let mut norm = 0.;
        for k in 0..3 {
            let mut ju = ip(0.)?;
            let mut mix = ip(0.)?;
            for (j, x) in stage.iter().enumerate() {
                ju = ju
                    .add(ji[k][j].mul(ip(*x)?).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            }
            for (j, prev) in u[..i].iter().enumerate() {
                mix = mix
                    .add(
                        ip(coeffs.c_matrix[(i, j)])?
                            .mul(ip(prev[k])?)
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
            }
            let r = ip(stage[k])?
                .sub(tau.mul(ju).map_err(|e| e.to_string())?)
                .and_then(|q| q.sub(ip(coeffs.gamma).unwrap().mul(mix).unwrap()))
                .and_then(|q| q.sub(tau.mul(fv[k]).unwrap()))
                .map_err(|e| e.to_string())?;
            norm = add_up(norm, r.mag()).map_err(|e| e.to_string())?;
        }
        rho[i] = norm;
    }
    let max_res = rho.iter().copied().fold(0., f64::max);
    // Conservative source-only J norm bound on an independently verified positivity tube.
    let mut m = 0.;
    for &x in &y {
        m = add_up(m, x).map_err(|e| e.to_string())?;
    }
    let m_tube = mul_up(1.1, m).map_err(|e| e.to_string())?;
    let k23 = add_up(case.k[1], mul_up(2., case.k[2]).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let lips = mul_up(
        2.,
        case.k[0].max(mul_up(k23, m_tube).map_err(|e| e.to_string())?),
    )
    .map_err(|e| e.to_string())?;
    let tau_f = mul_up(h, coeffs.gamma).map_err(|e| e.to_string())?;
    let margin =
        sub_down(1., mul_up(tau_f, lips).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if margin <= 0. || !margin.is_finite() {
        return Ok((max_res, None, "NEUMANN_MARGIN_NONPOSITIVE".into()));
    }
    let kappa = div_up(1., margin).map_err(|e| e.to_string())?;
    let mut err = [0.; 8];
    for i in 0..8 {
        let mut tube_r = 0.;
        for (j, &e) in err.iter().enumerate().take(i) {
            tube_r = add_up(
                tube_r,
                mul_up(coeffs.a[(i, j)].abs(), e).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        let mut hi_sum = 0.;
        let mut positive = true;
        for z in &args[i] {
            if sub_down(z.as_pair()[0], tube_r).map_err(|e| e.to_string())? < 0. {
                positive = false;
            }
            hi_sum = add_up(
                hi_sum,
                add_up(z.as_pair()[1], tube_r).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        if !positive || hi_sum > m_tube || !hi_sum.is_finite() {
            return Ok((max_res, None, "POSITIVE_TUBE_REFUSED".into()));
        }
        let mut coupling = 0.;
        for (j, &e) in err.iter().enumerate().take(i) {
            let c_term = mul_up(coeffs.gamma.abs(), coeffs.c_matrix[(i, j)].abs())
                .map_err(|e| e.to_string())?;
            let a_term = mul_up(
                mul_up(tau_f, lips).map_err(|e| e.to_string())?,
                coeffs.a[(i, j)].abs(),
            )
            .map_err(|e| e.to_string())?;
            let coeff = add_up(c_term, a_term).map_err(|e| e.to_string())?;
            coupling = add_up(coupling, mul_up(coeff, e).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
        err[i] = mul_up(kappa, add_up(rho[i], coupling).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if !err[i].is_finite() {
            return Ok((max_res, None, "NONFINITE_ERROR_BOUND".into()));
        }
    }
    let mut alg_err = 0.;
    for (i, &e) in err.iter().enumerate() {
        alg_err = add_up(
            alg_err,
            mul_up(coeffs.b_code[i].abs(), e).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    let mut out_round = 0.;
    for (k, actual) in y_out.iter().enumerate() {
        let mut exact = ip(y[k])?;
        for (i, stage) in u.iter().enumerate() {
            exact = exact
                .add(
                    ip(coeffs.b_code[i])?
                        .mul(ip(stage[k])?)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
        }
        out_round = add_up(
            out_round,
            exact.sub(ip(*actual)?).map_err(|e| e.to_string())?.mag(),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok((
        max_res,
        Some(add_up(alg_err, out_round).map_err(|e| e.to_string())?),
        "PROVED_CONDITIONAL_TUBE".into(),
    ))
}
fn run_rodas(case: Case, n: usize) {
    let mut y = Y0;
    let mut work = WorkCounters::default();
    let h = case.t_end / (n as f64);
    let source = match owner(case, Y0) {
        Ok(v) => Arc::new(v),
        Err(e) => {
            println!(
                "C8_FAIL {}",
                json!({"case":case.id,"method":"RODAS5P-direct-oracle","n":n,"error":e.to_string()})
            );
            return;
        }
    };
    let problem = ref_problem(source);
    let mut max_r: f64 = 0.;
    let mut max_alg: f64 = 0.;
    let mut certificate_fail = None;
    let mut passed = 0;
    for step in 0..n {
        match sequential_step(
            &problem,
            step as f64 * h,
            &y,
            h,
            &LinearSolverConfig::default(),
            None,
            1e-10,
            1e-7,
            true,
            &mut work,
        ) {
            Ok(res) => {
                match certified_raw_stage_bound(case, y, h, &res.stages, &res.y_new) {
                    Ok((r, b, status)) => {
                        max_r = max_r.max(r);
                        if let Some(x) = b {
                            max_alg = max_alg.max(x)
                        } else if certificate_fail.is_none() {
                            certificate_fail = Some(status);
                        }
                    }
                    Err(e) => {
                        if certificate_fail.is_none() {
                            certificate_fail = Some(e)
                        }
                    }
                }
                if !res.y_new.iter().all(|v| v.is_finite()) {
                    certificate_fail = Some("NONFINITE_OUTPUT".to_string());
                    break;
                }
                y.copy_from_slice(&res.y_new);
                passed += 1;
            }
            Err(e) => {
                println!(
                    "C8_FAIL {}",
                    json!({"case":case.id,"method":"RODAS5P-direct-oracle","n":n,"steps_completed":passed,"error":e.to_string(),"rhs":work.rhs_evaluations,"jac":work.jacobian_builds,"linear_solves":work.linear_solves})
                );
                return;
            }
        }
    }
    println!(
        "C8_ROW {}",
        json!({"case":case.id,"method":"RODAS5P-direct-oracle","n":n,"state":y,"mass_error":(y.iter().sum::<f64>()-1.).abs(),"min_state":y.iter().copied().fold(f64::INFINITY,f64::min),"forced_step_not_physical_admission":true,"rhs":work.rhs_evaluations,"jac":work.jacobian_builds,"linear_solves":work.linear_solves,"direct_factorizations":work.direct_factorizations,"JVP":work.jvp_calls,"original_interval_residual_l1_max":max_r,"algebraic_endpoint_bound_l1_max":if certificate_fail.is_none(){Some(max_alg)}else{None},"certificate_status":certificate_fail.unwrap_or("PROVED_CONDITIONAL_TUBE".to_string()),"steps_completed":passed})
    );
}
fn run_be(case: Case, n: usize, accel: RhsAcceleration) {
    let mut y = Y0;
    let h = case.t_end / n as f64;
    let mut attempted = 0u64;
    let mut completed = 0u64;
    let mut speculative = 0u64;
    let mut stages = 0;
    let mode = if accel == RhsAcceleration::Picard {
        "BE-Picard-value-only"
    } else {
        "BE-Homotopy-value-only"
    };
    let cfg = RhsHomotopyConfig {
        acceleration: accel,
        max_map_evaluations: if case.id == "stiff_negative_control" {
            128
        } else {
            16_384
        },
        ..RhsHomotopyConfig::default()
    };
    for _i in 0..n {
        let model = match owner(case, y) {
            Ok(o) => o,
            Err(e) => {
                println!(
                    "C8_FAIL {}",
                    json!({"case":case.id,"method":mode,"n":n,"steps_completed":stages,"error":e.to_string(),"map_attempted":attempted})
                );
                return;
            }
        };
        let trial = model.freeze();
        let stage = match N01ValueStage::new(&model, &trial, h) {
            Ok(s) => s,
            Err(e) => {
                println!(
                    "C8_FAIL {}",
                    json!({"case":case.id,"method":mode,"n":n,"steps_completed":stages,"error":e.to_string(),"map_attempted":attempted})
                );
                return;
            }
        };
        let mut output = [f64::NAN; 3];
        let a = stage.solve(&cfg, 1e-9, 1e-9, &mut output);
        attempted += a.homotopy.work.map_attempted;
        completed += a.homotopy.work.map_completed;
        speculative += a.homotopy.work.extrapolation_trials;
        if !a.admitted {
            let e = a
                .certificate_error
                .or_else(|| a.homotopy.failure.map(|f| f.message))
                .unwrap_or("ORIGINAL_OR_PHYSICAL_CERTIFICATE_REJECTED".into());
            println!(
                "C8_FAIL {}",
                json!({"case":case.id,"method":mode,"n":n,"steps_completed":stages,"error":e,"map_attempted":attempted,"map_completed":completed,"speculative_trials":speculative})
            );
            return;
        }
        y = output;
        stages += 1;
    }
    println!(
        "C8_ROW {}",
        json!({"case":case.id,"method":mode,"n":n,"state":y,"mass_error":(y.iter().sum::<f64>()-1.).abs(),"min_state":y.iter().copied().fold(f64::INFINITY,f64::min),"source_implicit_steps":stages,"map_attempted":attempted,"map_completed":completed,"speculative_trials":speculative,"jac":0,"JVP":0,"newton":0,"reference_not_RODAS":true})
    );
}
#[test]
fn c8_fixed_grid_work_precision_and_negative_controls() {
    // Entire benchmark is one bounded, source-matched, no-wall-clock numerical campaign.
    for (idx, case) in cases().into_iter().enumerate() {
        let grids: &[usize] = if idx == 2 {
            &[8, 16, 32, 64]
        } else {
            &[8, 16, 32, 64, 128]
        };
        for &n in grids {
            run_rodas(case, n);
        }
        let be_grids: &[usize] = if idx == 2 {
            &[8, 16, 32, 64]
        } else {
            &[8, 16, 32, 64, 128, 256, 512]
        };
        for &n in be_grids {
            run_be(case, n, RhsAcceleration::Picard);
            run_be(case, n, RhsAcceleration::ResidualDominance);
        }
    }
}
#[test]
fn mass_conservation_is_not_a_contractive_linearization_certificate() {
    let k = [0.5, 1., 0.5];
    let y = [0.5, 0.25, 0.25];
    let model = owner(
        Case {
            id: "counter",
            k,
            t_end: 2.,
        },
        y,
    )
    .unwrap();
    let j = model.jacobian_at(&y).unwrap();
    assert!(j[1][2] < 0., "Robertson J is not Metzler");
    let mu1 = (0..3)
        .map(|column| {
            j[column][column]
                + (0..3)
                    .filter(|&row| row != column)
                    .map(|row| j[row][column].abs())
                    .sum::<f64>()
        })
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((mu1 - 0.5).abs() < 1e-15);
    let m = DenseMatrix::from_rows(&[&[2., -0.5, -0.5], &[-1., 2., 0.5], &[0., -0.5, 1.]]).unwrap();
    let inverse = rodas5p_core::inverse(&m).unwrap();
    let norm1 = (0..3)
        .map(|j| (0..3).map(|i| inverse[(i, j)].abs()).sum::<f64>())
        .fold(0., f64::max);
    assert!((norm1 - 19. / 15.).abs() < 1e-14);
    println!(
        "C8_NEGATIVE {}",
        json!({"source":"Robertson","mu1":mu1,"resolvent_l1":norm1,"contraction_false":true})
    );
}
