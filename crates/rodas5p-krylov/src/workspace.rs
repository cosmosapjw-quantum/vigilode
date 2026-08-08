use rodas5p_core::{CoreResult, DenseMatrix};

pub(crate) fn ensure_len(buffer: &mut Vec<f64>, len: usize) {
    if buffer.len() != len {
        buffer.resize(len, 0.0);
    }
}

pub(crate) fn ensure_pool(pool: &mut Vec<Vec<f64>>, count: usize, dimension: usize) {
    while pool.len() < count {
        pool.push(vec![0.0; dimension]);
    }
    for vector in &mut pool[..count] {
        ensure_len(vector, dimension);
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CommonWorkspace {
    pub x: Vec<f64>,
    pub residual: Vec<f64>,
    pub operator_output: Vec<f64>,
    pub preconditioned: Vec<f64>,
    pub scratch_a: Vec<f64>,
    pub scratch_b: Vec<f64>,
    pub coefficients: Vec<f64>,
}

impl CommonWorkspace {
    pub fn prepare(&mut self, dimension: usize) {
        ensure_len(&mut self.x, dimension);
        self.x.fill(0.0);
        ensure_len(&mut self.residual, dimension);
        ensure_len(&mut self.operator_output, dimension);
        ensure_len(&mut self.preconditioned, dimension);
        ensure_len(&mut self.scratch_a, dimension);
        ensure_len(&mut self.scratch_b, dimension);
    }

    pub fn capacity_f64(&self) -> usize {
        self.x.capacity()
            + self.residual.capacity()
            + self.operator_output.capacity()
            + self.preconditioned.capacity()
            + self.scratch_a.capacity()
            + self.scratch_b.capacity()
            + self.coefficients.capacity()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ArnoldiWorkspace {
    pub basis: Vec<Vec<f64>>,
    pub directions: Vec<Vec<f64>>,
    pub hessenberg: DenseMatrix,
    pub hessenberg_prefix: DenseMatrix,
    pub h_column: Vec<f64>,
    pub rhs_small: Vec<f64>,
    pub correction: Vec<f64>,
    pub raw_operator_output: Vec<f64>,
}

impl ArnoldiWorkspace {
    pub fn prepare(&mut self, dimension: usize, total_columns: usize) -> CoreResult<()> {
        ensure_pool(&mut self.basis, total_columns + 1, dimension);
        ensure_pool(&mut self.directions, total_columns, dimension);
        self.hessenberg
            .resize_zeros(total_columns + 1, total_columns)?;
        self.hessenberg_prefix.resize_zeros(0, 0)?;
        ensure_len(&mut self.h_column, total_columns + 1);
        ensure_len(&mut self.rhs_small, total_columns + 1);
        ensure_len(&mut self.correction, dimension);
        ensure_len(&mut self.raw_operator_output, dimension);
        Ok(())
    }

    pub fn capacity_f64(&self) -> usize {
        self.basis.iter().map(Vec::capacity).sum::<usize>()
            + self.directions.iter().map(Vec::capacity).sum::<usize>()
            + self.hessenberg.capacity()
            + self.hessenberg_prefix.capacity()
            + self.h_column.capacity()
            + self.rhs_small.capacity()
            + self.correction.capacity()
            + self.raw_operator_output.capacity()
    }
}

#[derive(Clone, Debug, Default)]
pub struct GmresWorkspace {
    pub(crate) common: CommonWorkspace,
    pub(crate) arnoldi: ArnoldiWorkspace,
}

impl GmresWorkspace {
    pub fn capacity_f64(&self) -> usize {
        self.common.capacity_f64() + self.arnoldi.capacity_f64()
    }
}

#[derive(Clone, Debug, Default)]
pub struct LgmresWorkspace {
    pub(crate) common: CommonWorkspace,
    pub(crate) arnoldi: ArnoldiWorkspace,
}

impl LgmresWorkspace {
    pub fn capacity_f64(&self) -> usize {
        self.common.capacity_f64() + self.arnoldi.capacity_f64()
    }
}

#[derive(Clone, Debug, Default)]
pub struct GcrodrWorkspace {
    pub(crate) common: CommonWorkspace,
}

impl GcrodrWorkspace {
    pub fn capacity_f64(&self) -> usize {
        self.common.capacity_f64()
    }
}
