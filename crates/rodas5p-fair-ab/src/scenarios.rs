use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, StandardNormal};
use rand_pcg::Pcg64Mcg;
use rodas5p_core::{DenseMatrix, sha256_hex};
use serde::{Deserialize, Serialize};

use crate::{FairError, FairResult, LinearSystemCase, SequenceKind};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SequenceConfig {
    pub kind: SequenceKind,
    pub dimension: usize,
    pub steps: usize,
    pub stages: usize,
    pub seed: u64,
    pub stiffness: f64,
    pub nonnormality: f64,
}

impl SequenceConfig {
    pub fn validate(&self) -> FairResult<()> {
        if self.dimension < 2 || self.steps == 0 || self.stages == 0 {
            return Err(FairError::Invalid(
                "trace dimensions, steps, and stages must be positive".into(),
            ));
        }
        if !self.stiffness.is_finite() || self.stiffness <= 0.0 {
            return Err(FairError::Invalid(
                "stiffness must be finite and positive".into(),
            ));
        }
        if !self.nonnormality.is_finite() || self.nonnormality < 0.0 {
            return Err(FairError::Invalid(
                "nonnormality must be finite and nonnegative".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct LinearSystemTrace {
    pub trace_id: String,
    pub config: SequenceConfig,
    pub cases: Vec<LinearSystemCase>,
}

impl LinearSystemTrace {
    pub fn system_ids(&self) -> Vec<String> {
        self.cases
            .iter()
            .map(|case| case.system_id.clone())
            .collect()
    }
}

fn trace_digest(config: &SequenceConfig, cases: &[LinearSystemCase]) -> FairResult<String> {
    let mut bytes = serde_json::to_vec(config)?;
    for case in cases {
        bytes.extend_from_slice(case.system_id.as_bytes());
    }
    Ok(sha256_hex(&bytes))
}

fn base_matrix(config: &SequenceConfig) -> DenseMatrix {
    let n = config.dimension;
    let mut matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        let x = i as f64 / (n - 1) as f64;
        let diagonal = 1.0 + config.stiffness * (0.01 + x * x);
        matrix[(i, i)] = diagonal;
        if i + 1 < n {
            let next_x = (i + 1) as f64 / (n - 1) as f64;
            let next_diagonal = 1.0 + config.stiffness * (0.01 + next_x * next_x);
            matrix[(i, i + 1)] = config.nonnormality * (diagonal * next_diagonal).sqrt();
            matrix[(i + 1, i)] = 0.01 * (diagonal * next_diagonal).sqrt();
        }
    }
    matrix
}

fn givens_similarity(base: &DenseMatrix, angle: f64) -> FairResult<DenseMatrix> {
    let n = base.nrows();
    let mut q = DenseMatrix::identity(n);
    let c = angle.cos();
    let s = angle.sin();
    q[(0, 0)] = c;
    q[(0, n - 1)] = -s;
    q[(n - 1, 0)] = s;
    q[(n - 1, n - 1)] = c;
    q.transpose().matmul(base)?.matmul(&q).map_err(Into::into)
}

fn matrix_for_step(
    config: &SequenceConfig,
    base: &DenseMatrix,
    step: usize,
) -> FairResult<DenseMatrix> {
    match config.kind {
        SequenceKind::Fixed => Ok(base.clone()),
        SequenceKind::SlowDrift => {
            let factor = 1.0 + 0.01 * step as f64;
            let mut out = base.clone();
            for i in 0..out.nrows() {
                out[(i, i)] *= factor;
            }
            Ok(out)
        }
        SequenceKind::Abrupt => {
            if step < config.steps / 2 {
                Ok(base.clone())
            } else {
                let mut out = givens_similarity(base, 0.55)?;
                for i in 0..out.nrows() {
                    out[(i, i)] *= 1.35;
                }
                Ok(out)
            }
        }
        SequenceKind::Rotating => {
            let angle = 0.22 * step as f64;
            givens_similarity(base, angle)
        }
    }
}

fn oracle_vector(
    config: &SequenceConfig,
    step: usize,
    stage: usize,
    rng: &mut Pcg64Mcg,
) -> Vec<f64> {
    let phase = std::f64::consts::TAU * (stage as f64 + 0.17 * step as f64) / config.stages as f64;
    (0..config.dimension)
        .map(|i| {
            let x = (i + 1) as f64 / (config.dimension + 1) as f64;
            let noise: f64 = StandardNormal.sample(rng);
            (std::f64::consts::PI * x).sin()
                + 0.35 * phase.sin() * (2.0 * std::f64::consts::PI * x).cos()
                + 0.08 * step as f64 * (3.0 * std::f64::consts::PI * x).sin()
                + 1e-4 * noise
        })
        .collect()
}

pub fn generate_trace(config: &SequenceConfig) -> FairResult<LinearSystemTrace> {
    config.validate()?;
    let mut rng = Pcg64Mcg::seed_from_u64(config.seed);
    // Consume one value to make the RNG contract explicit and version-pinned.
    let _: u64 = rng.random();
    let base = base_matrix(config);
    let mut cases = Vec::with_capacity(config.steps * config.stages);
    for step in 0..config.steps {
        let matrix = matrix_for_step(config, &base, step)?;
        for stage in 0..config.stages {
            let oracle = oracle_vector(config, step, stage, &mut rng);
            let rhs = matrix.matvec(&oracle)?;
            let mut case = LinearSystemCase::from_matrix(matrix.clone(), rhs, step, stage)?;
            case.metadata
                .insert("sequence_kind".into(), format!("{:?}", config.kind));
            cases.push(case);
        }
    }
    let trace_id = trace_digest(config, &cases)?;
    Ok(LinearSystemTrace {
        trace_id,
        config: config.clone(),
        cases,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseDocument {
    pub dimension: usize,
    pub matrix: Vec<f64>,
    pub rhs: Vec<f64>,
    pub operator_id: String,
    pub system_id: String,
    pub step_index: usize,
    pub stage_index: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceDocument {
    pub schema: String,
    pub trace_id: String,
    pub config: SequenceConfig,
    pub cases: Vec<CaseDocument>,
}

impl TraceDocument {
    pub fn from_trace(trace: &LinearSystemTrace) -> Self {
        Self {
            schema: "rodas5p-rust-trace-v1".into(),
            trace_id: trace.trace_id.clone(),
            config: trace.config.clone(),
            cases: trace
                .cases
                .iter()
                .map(|case| CaseDocument {
                    dimension: case.dimension(),
                    matrix: case.matrix.as_slice().to_vec(),
                    rhs: case.rhs.clone(),
                    operator_id: case.operator_id.clone(),
                    system_id: case.system_id.clone(),
                    step_index: case.step_index,
                    stage_index: case.stage_index,
                })
                .collect(),
        }
    }

    pub fn into_trace(self) -> FairResult<LinearSystemTrace> {
        if self.schema != "rodas5p-rust-trace-v1" {
            return Err(FairError::Invalid("unsupported trace schema".into()));
        }
        let mut cases = Vec::with_capacity(self.cases.len());
        for record in self.cases {
            let matrix = DenseMatrix::new(record.dimension, record.dimension, record.matrix)?;
            let case = LinearSystemCase::from_matrix(
                matrix,
                record.rhs,
                record.step_index,
                record.stage_index,
            )?;
            if case.operator_id != record.operator_id || case.system_id != record.system_id {
                return Err(FairError::Invalid(format!(
                    "trace identity mismatch: operator {} != {}, system {} != {}",
                    case.operator_id, record.operator_id, case.system_id, record.system_id,
                )));
            }
            cases.push(case);
        }
        let trace_id = trace_digest(&self.config, &cases)?;
        if trace_id != self.trace_id {
            return Err(FairError::Invalid("trace digest mismatch".into()));
        }
        Ok(LinearSystemTrace {
            trace_id,
            config: self.config,
            cases,
        })
    }
}
