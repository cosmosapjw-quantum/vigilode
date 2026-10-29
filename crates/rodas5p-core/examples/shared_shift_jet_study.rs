//! New native fixtures for the 2026-10-04 integration, not a historical replay.
//! Emits JSON only; no timings or speed claims.
use rodas5p_core::shared_shift_jet::{
    CertificateStatus, SharedShiftJetConfig, certify_shift_candidate, shared_shift_jet,
};
use rodas5p_core::{CoreResult, DenseMatrix};
use serde_json::{Value, json};

fn bits(values: &[f64]) -> Vec<String> {
    values
        .iter()
        .map(|v| format!("{:016x}", v.to_bits()))
        .collect()
}

fn identity_columns(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| (0..n).map(|k| if i == k { 1.0 } else { 0.0 }).collect())
        .collect()
}

fn shifts(center: f64, half_count: i32) -> Vec<f64> {
    (-half_count..=half_count)
        .map(|i| center * (1.0 + f64::from(i) / 256.0))
        .collect()
}

fn case(
    id: &str,
    j: &DenseMatrix,
    h: f64,
    gamma0: f64,
    rhs: &[Vec<f64>],
    gammas: &[f64],
    degree: usize,
) -> CoreResult<Value> {
    let config = SharedShiftJetConfig {
        degree,
        ..SharedShiftJetConfig::default()
    };
    let report = shared_shift_jet(j, h, gamma0, rhs, gammas, config)?;
    let candidate_bits: Vec<_> = report.candidates().iter().map(|candidate| {
        json!({
            "gamma": format!("{:016x}", candidate.gamma().to_bits()),
            "rhs_columns": candidate.rhs_columns().iter().map(|column| bits(column)).collect::<Vec<_>>(),
            "rhs_error_upper": bits(candidate.certificate().rhs_error_upper()),
        })
    }).collect();
    Ok(json!({
        "id": id,
        "j": {"nrows": j.nrows(), "ncols": j.ncols(), "row_major": j.as_slice()},
        "h": h,
        "gamma0": gamma0,
        "rhs_columns": rhs,
        "gammas": gammas,
        "config": config,
        "report": report,
        "exact_binary_hex": {
            "j_row_major": bits(j.as_slice()),
            "h": format!("{:016x}", h.to_bits()),
            "gamma0": format!("{:016x}", gamma0.to_bits()),
            "rhs_columns": rhs.iter().map(|column| bits(column)).collect::<Vec<_>>(),
            "gammas": bits(gammas),
            "candidates": candidate_bits,
        },
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    let scalar = DenseMatrix::from_rows(&[&[-8.0]])?;
    for degree in [0, 8, 17, 24] {
        cases.push(case(
            &format!("scalar_degree_{degree}"),
            &scalar,
            0.5,
            0.5,
            &[vec![1.0]],
            &shifts(0.5, 16),
            degree,
        )?);
    }
    let diagonal = DenseMatrix::from_rows(&[
        &[-0.5, 0.0, 0.0, 0.0],
        &[0.0, -2.0, 0.0, 0.0],
        &[0.0, 0.0, -8.0, 0.0],
        &[0.0, 0.0, 0.0, -32.0],
    ])?;
    for degree in [17, 24] {
        cases.push(case(
            &format!("diagonal4_rank4_degree_{degree}"),
            &diagonal,
            0.25,
            0.5,
            &identity_columns(4),
            &shifts(0.5, 32),
            degree,
        )?);
    }
    let skew2 = DenseMatrix::from_rows(&[&[-1.0, 16.0], &[-16.0, -1.0]])?;
    for degree in [8, 17, 24] {
        cases.push(case(
            &format!("skew_damping2_rank2_degree_{degree}"),
            &skew2,
            0.125,
            0.5,
            &identity_columns(2),
            &shifts(0.5, 16),
            degree,
        )?);
    }
    let skew4 = DenseMatrix::from_rows(&[
        &[-1.0, 7.0, 0.0, 0.0],
        &[-7.0, -2.0, 5.0, 0.0],
        &[0.0, -5.0, -3.0, 3.0],
        &[0.0, 0.0, -3.0, -4.0],
    ])?;
    cases.push(case(
        "skew_damping4_rank4",
        &skew4,
        0.25,
        0.5,
        &identity_columns(4),
        &shifts(0.5, 32),
        24,
    )?);
    cases.push(case(
        "zero_j",
        &DenseMatrix::zeros(2, 2),
        1.0,
        0.5,
        &identity_columns(2),
        &shifts(0.5, 16),
        0,
    )?);
    cases.push(case(
        "zero_h",
        &scalar,
        0.0,
        0.5,
        &[vec![3.0]],
        &shifts(0.5, 16),
        0,
    )?);
    for exponent in [-400, 400] {
        let gamma0 = 2_f64.powi(exponent);
        let j = DenseMatrix::new(1, 1, vec![-2_f64.powi(-exponent)])?;
        cases.push(case(
            &format!("extreme_center_2pow_{exponent}"),
            &j,
            1.0,
            gamma0,
            &[vec![1.0]],
            &shifts(gamma0, 16),
            24,
        )?);
    }
    let holdout =
        DenseMatrix::from_rows(&[&[-2.0, 7.0, 0.0], &[-7.0, -3.0, 5.0], &[0.0, -5.0, -4.0]])?;
    let holdout_shifts: Vec<f64> = (-16..=16)
        .map(|i| 0.75 * (1.0 + f64::from(i) / 128.0))
        .collect();
    cases.push(case(
        "fixed_holdout3_rank2",
        &holdout,
        3.0 / 16.0,
        0.75,
        &[vec![1.0, 0.0, 0.0], vec![1.0, -2.0, 3.0]],
        &holdout_shifts,
        24,
    )?);

    let config = SharedShiftJetConfig::default();
    let nondissipative = DenseMatrix::from_rows(&[&[-1.0, 100.0], &[0.0, -1.0]])?;
    let controls: Vec<(&str, CoreResult<_>)> = vec![
        (
            "stable_spectrum_nondissipative",
            shared_shift_jet(&nondissipative, 1.0, 0.5, &[vec![1.0, 0.0]], &[0.5], config),
        ),
        (
            "positive_j",
            shared_shift_jet(
                &DenseMatrix::from_rows(&[&[1.0]])?,
                1.0,
                0.5,
                &[vec![1.0]],
                &[0.5],
                config,
            ),
        ),
        (
            "wrong_rhs_dimension",
            shared_shift_jet(&scalar, 1.0, 0.5, &[vec![]], &[0.5], config),
        ),
        (
            "nonfinite_rhs",
            shared_shift_jet(&scalar, 1.0, 0.5, &[vec![f64::NAN]], &[0.5], config),
        ),
        (
            "empty_rhs",
            shared_shift_jet(&scalar, 1.0, 0.5, &[], &[0.5], config),
        ),
        (
            "empty_targets",
            shared_shift_jet(&scalar, 1.0, 0.5, &[vec![1.0]], &[], config),
        ),
        (
            "radius_one",
            shared_shift_jet(&scalar, 1.0, 0.5, &[vec![1.0]], &[1.0], config),
        ),
        (
            "degree_overflow",
            shared_shift_jet(
                &scalar,
                1.0,
                0.5,
                &[vec![1.0]],
                &[0.5],
                SharedShiftJetConfig {
                    degree: usize::MAX,
                    ..config
                },
            ),
        ),
        (
            "resource_cap",
            shared_shift_jet(
                &scalar,
                1.0,
                0.5,
                &[vec![1.0]],
                &[0.5],
                SharedShiftJetConfig {
                    max_stored_scalars: 0,
                    ..config
                },
            ),
        ),
    ];
    let mut negative_controls: Vec<Value> = controls
        .into_iter()
        .map(|(id, result)| match result {
            Ok(_) => json!({"id": id, "rejected": false}),
            Err(error) => json!({"id": id, "rejected": true, "error": error.to_string()}),
        })
        .collect();
    let wrong = certify_shift_candidate(&scalar, 0.5, 0.5, &[vec![1.0]], &[vec![1.0]], 1e-10)?;
    negative_controls.push(json!({"id": "wrong_candidate", "rejected": wrong.status() == CertificateStatus::Rejected, "certificate": wrong}));
    let tiny = shared_shift_jet(
        &scalar,
        0.5,
        0.5,
        &[vec![1.0]],
        &[0.5],
        SharedShiftJetConfig {
            absolute_tolerance: f64::MIN_POSITIVE,
            ..config
        },
    )?;
    negative_controls.push(json!({"id": "too_small_tolerance", "rejected": tiny.candidates()[0].certificate().status() == CertificateStatus::Rejected}));
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "claim_ceiling": "bounded real dissipative current-J resolvent action; no timing or production promotion",
            "cases": cases,
            "negative_controls": negative_controls,
        }))?
    );
    Ok(())
}
