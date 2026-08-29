use serde::Deserialize;

use crate::{CoreError, CoreResult, DenseMatrix, inverse};

pub const RODAS5P_COEFFICIENT_SNAPSHOT_SCHEMA_VERSION: &str =
    "vigilode-rodas5p-coefficient-snapshot-v2";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCoefficients {
    schema_version: String,
    gamma: String,
    #[serde(rename = "A")]
    a: Vec<Vec<String>>,
    #[serde(rename = "C")]
    c_matrix: Vec<Vec<String>>,
    c: Vec<String>,
    b_code: Vec<String>,
    #[serde(rename = "H")]
    dense_h: Vec<Vec<String>>,
    provenance: Rodas5pCoefficientProvenance,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CoefficientPrecisionAvailability {
    NoPublicAuthoritativeValues,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rodas5pCoefficientProvenance {
    pub source_repository: String,
    pub author_commit: String,
    pub author_source_path: String,
    pub author_source_sha256: String,
    pub parity_commit: String,
    pub parity_source_path: String,
    pub parity_source_sha256: String,
    pub search_as_of: String,
    pub literal_semantics: String,
    pub higher_precision: CoefficientPrecisionAvailability,
}

#[derive(Clone, Debug)]
pub struct Rodas5pCoefficients {
    pub snapshot_schema_version: String,
    pub provenance: Rodas5pCoefficientProvenance,
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
    /// Official RODAS5P continuous-extension coefficients in raw-stage
    /// coordinates.
    pub dense_h: DenseMatrix,
    /// Continuous-extension coefficients transformed into this crate's K
    /// coordinates.  `StepResult::stages` are K, so dense evaluation uses
    /// this matrix directly, without another factor of h.
    pub dense_d: DenseMatrix,
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
    if raw.schema_version != RODAS5P_COEFFICIENT_SNAPSHOT_SCHEMA_VERSION {
        return Err(CoreError::Coefficients(format!(
            "unsupported RODAS5P coefficient snapshot schema: {}",
            raw.schema_version
        )));
    }
    let gamma = parse_number(&raw.gamma)?;
    let a = parse_matrix(raw.a)?;
    let c_matrix = parse_matrix(raw.c_matrix)?;
    let c: CoreResult<Vec<_>> = raw.c.into_iter().map(|s| parse_number(&s)).collect();
    let b_code: CoreResult<Vec<_>> = raw.b_code.into_iter().map(|s| parse_number(&s)).collect();
    let dense_h = parse_matrix(raw.dense_h)?;
    let c = c?;
    let b_code = b_code?;
    let n = c.len();
    if a.nrows() != n
        || a.ncols() != n
        || c_matrix.nrows() != n
        || c_matrix.ncols() != n
        || b_code.len() != n
        || dense_h.nrows() != 3
        || dense_h.ncols() != n
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
    let dense_d = dense_h.matmul(&gamma_matrix)?;
    Ok(Rodas5pCoefficients {
        snapshot_schema_version: raw.schema_version,
        provenance: raw.provenance,
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
        dense_h,
        dense_d,
    })
}
