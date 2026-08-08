use serde::Deserialize;

use crate::{CoreError, CoreResult, DenseMatrix, inverse};

#[derive(Debug, Deserialize)]
struct RawCoefficients {
    gamma: String,
    #[serde(rename = "A")]
    a: Vec<Vec<String>>,
    #[serde(rename = "C")]
    c_matrix: Vec<Vec<String>>,
    c: Vec<String>,
    b_code: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Rodas5pCoefficients {
    pub gamma: f64,
    pub a: DenseMatrix,
    pub c_matrix: DenseMatrix,
    pub c: Vec<f64>,
    pub b_code: Vec<f64>,
    pub gamma_matrix: DenseMatrix,
    pub alpha: DenseMatrix,
    pub beta: DenseMatrix,
    pub l: DenseMatrix,
    pub b: Vec<f64>,
    pub btilde: Vec<f64>,
    pub gamma_rows: Vec<f64>,
}

impl Rodas5pCoefficients {
    pub fn stages(&self) -> usize {
        self.c.len()
    }
}

fn parse_number(s: &str) -> CoreResult<f64> {
    if let Some((n, d)) = s.split_once('/') {
        let n: f64 = n
            .parse()
            .map_err(|_| CoreError::Coefficients(format!("bad numerator {n}")))?;
        let d: f64 = d
            .parse()
            .map_err(|_| CoreError::Coefficients(format!("bad denominator {d}")))?;
        if d == 0.0 {
            return Err(CoreError::Coefficients("zero denominator".into()));
        }
        Ok(n / d)
    } else {
        s.parse()
            .map_err(|_| CoreError::Coefficients(format!("bad number {s}")))
    }
}

fn parse_matrix(rows: Vec<Vec<String>>) -> CoreResult<DenseMatrix> {
    let parsed: CoreResult<Vec<Vec<f64>>> = rows
        .into_iter()
        .map(|r| r.into_iter().map(|s| parse_number(&s)).collect())
        .collect();
    DenseMatrix::from_vec_rows(parsed?)
}

pub fn load_rodas5p_coefficients() -> CoreResult<Rodas5pCoefficients> {
    let raw: RawCoefficients = serde_json::from_str(include_str!(
        "../../../fixtures/rodas5p_coefficients_snapshot.json"
    ))
    .map_err(|e| CoreError::Coefficients(e.to_string()))?;
    let gamma = parse_number(&raw.gamma)?;
    let a = parse_matrix(raw.a)?;
    let c_matrix = parse_matrix(raw.c_matrix)?;
    let c: CoreResult<Vec<_>> = raw.c.into_iter().map(|s| parse_number(&s)).collect();
    let b_code: CoreResult<Vec<_>> = raw.b_code.into_iter().map(|s| parse_number(&s)).collect();
    let c = c?;
    let b_code = b_code?;
    let n = c.len();
    if a.nrows() != n
        || a.ncols() != n
        || c_matrix.nrows() != n
        || c_matrix.ncols() != n
        || b_code.len() != n
    {
        return Err(CoreError::Coefficients(
            "snapshot dimensions are inconsistent".into(),
        ));
    }
    let gamma_inv = DenseMatrix::identity(n).scale(1.0 / gamma).sub(&c_matrix)?;
    let gamma_matrix = inverse(&gamma_inv)?;
    let alpha = a.matmul(&gamma_matrix)?;
    let beta = alpha.add(&gamma_matrix)?;
    let l = beta.sub(&DenseMatrix::identity(n).scale(gamma))?;
    let b = gamma_matrix.transpose().matvec(&b_code)?;
    let btilde = gamma_matrix.row(n - 1).to_vec();
    let gamma_rows = (0..n).map(|i| gamma_matrix.row(i).iter().sum()).collect();
    Ok(Rodas5pCoefficients {
        gamma,
        a,
        c_matrix,
        c,
        b_code,
        gamma_matrix,
        alpha,
        beta,
        l,
        b,
        btilde,
        gamma_rows,
    })
}
