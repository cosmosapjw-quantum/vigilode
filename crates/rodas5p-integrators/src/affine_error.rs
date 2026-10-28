//! Error control in fixed physical variables for an affine reduced state.
//!
//! If `v = offset + lift * u`, the embedded difference is `lift * error`,
//! without the offset. This type evaluates a weighted RMS over the **physical**
//! components, using both lifted step endpoints to construct their scales.
//! It does not certify the reduction, the embedded estimator, or global error.

use rodas5p_core::{CoreError, CoreResult, scaled_error_component};

/// A fixed, client-supplied affine lift and physical error tolerances.
///
/// The constructor compiles a row-major lift into nonzero rows once. Successful
/// [`Self::evaluate`] calls allocate no memory and cost
/// `O(nnz(lift) + physical_dimension + reduced_dimension)`. The caller must
/// establish that the reduced RHS represents the intended physical dynamics.
/// A rank-deficient lift defines only a seminorm on the reduced variables.
///
/// Tolerances belong to the physical variables and are independent of any
/// reduced-state tolerances in the time-step controller configuration.
#[derive(Clone, Debug)]
pub struct AffineErrorNorm {
    reduced_dimension: usize,
    offset: Vec<f64>,
    row_offsets: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<f64>,
    atol: Vec<f64>,
    rtol: f64,
}

impl AffineErrorNorm {
    /// Construct the fixed lift `v = offset + row_major_lift * u`.
    ///
    /// `offset.len()` is the physical dimension. The lift contains exactly
    /// `offset.len() * reduced_dimension` entries, in row-major order. `atol`
    /// contains either one scalar or one value per physical component.
    /// Dimensions must be positive, and coefficients, offsets and tolerances
    /// must be finite. Tolerances must be nonnegative. Zero scales use the
    /// shared step-error convention: zero error contributes zero, nonzero
    /// error contributes infinity and rejects the attempt.
    pub fn new(
        reduced_dimension: usize,
        offset: Vec<f64>,
        row_major_lift: Vec<f64>,
        atol: Vec<f64>,
        rtol: f64,
    ) -> CoreResult<Self> {
        let physical_dimension = offset.len();
        if reduced_dimension == 0 || physical_dimension == 0 {
            return Err(CoreError::Dimension(
                "affine error norm requires positive dimensions".into(),
            ));
        }
        let expected = physical_dimension
            .checked_mul(reduced_dimension)
            .ok_or_else(|| CoreError::Dimension("affine lift size overflow".into()))?;
        if row_major_lift.len() != expected {
            return Err(CoreError::Dimension(
                "affine lift row-major shape mismatch".into(),
            ));
        }
        if atol.len() != 1 && atol.len() != physical_dimension {
            return Err(CoreError::Dimension(
                "affine atol must have length one or the physical dimension".into(),
            ));
        }
        if offset.iter().chain(&row_major_lift).any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite(
                "affine lift and offset must be finite".into(),
            ));
        }
        if !rtol.is_finite() || rtol < 0.0 || atol.iter().any(|x| !x.is_finite() || *x < 0.0) {
            return Err(CoreError::InvalidInput(
                "affine tolerances must be finite and nonnegative".into(),
            ));
        }
        let nonzeros = row_major_lift.iter().filter(|x| **x != 0.0).count();
        let mut row_offsets = Vec::with_capacity(physical_dimension + 1);
        let mut columns = Vec::with_capacity(nonzeros);
        let mut values = Vec::with_capacity(nonzeros);
        row_offsets.push(0);
        for row in row_major_lift.chunks_exact(reduced_dimension) {
            for (column, &value) in row.iter().enumerate() {
                if value != 0.0 {
                    columns.push(column);
                    values.push(value);
                }
            }
            row_offsets.push(values.len());
        }
        Ok(Self {
            reduced_dimension,
            offset,
            row_offsets,
            columns,
            values,
            atol,
            rtol,
        })
    }

    pub fn reduced_dimension(&self) -> usize {
        self.reduced_dimension
    }

    pub fn physical_dimension(&self) -> usize {
        self.offset.len()
    }

    pub fn nonzeros(&self) -> usize {
        self.values.len()
    }

    pub fn atol(&self) -> &[f64] {
        &self.atol
    }

    pub fn rtol(&self) -> f64 {
        self.rtol
    }

    /// Check the reduced state dimension before invoking any model callback.
    pub fn validate_dimension(&self, dimension: usize) -> CoreResult<()> {
        if dimension != self.reduced_dimension {
            Err(CoreError::Dimension(
                "affine error norm and reduced state dimensions differ".into(),
            ))
        } else {
            Ok(())
        }
    }

    /// Evaluate the physical weighted RMS of a reduced embedded difference.
    ///
    /// For physical component `j`, the numerator is `(lift * error)[j]` and
    /// the denominator is `atol[j] + rtol * max(abs(v_old[j]), abs(v_new[j]))`.
    /// The mean has the physical dimension as its divisor. Compensated row
    /// summation reduces cancellation; a scaled sum of squares avoids overflow
    /// in otherwise representable RMS values. Nonfinite inputs, lifted values
    /// or scales return a typed error. A quotient overflow returns infinity,
    /// which is a rejected local-error test, never an accepted zero error.
    ///
    /// Floating-point overflow during a row accumulation is refused even when
    /// an exact-arithmetic cancellation could have produced a finite result.
    pub fn evaluate(&self, old: &[f64], new: &[f64], error: &[f64]) -> CoreResult<f64> {
        self.validate_dimension(old.len())?;
        self.validate_dimension(new.len())?;
        self.validate_dimension(error.len())?;
        if old.iter().chain(new).chain(error).any(|x| !x.is_finite()) {
            return Err(CoreError::NonFinite(
                "affine error norm input contains NaN/Inf".into(),
            ));
        }
        let mut maximum = 0.0_f64;
        let mut sum = 0.0_f64;
        let mut infinite_ratio = false;
        for row in 0..self.physical_dimension() {
            let mut old_sum = Compensated::new(self.offset[row]);
            let mut new_sum = Compensated::new(self.offset[row]);
            let mut error_sum = Compensated::new(0.0);
            for entry in self.row_offsets[row]..self.row_offsets[row + 1] {
                let col = self.columns[entry];
                let value = self.values[entry];
                old_sum.add(value * old[col])?;
                new_sum.add(value * new[col])?;
                error_sum.add(value * error[col])?;
            }
            let old_value = old_sum.finish()?;
            let new_value = new_sum.finish()?;
            let lifted_error = error_sum.finish()?;
            let absolute = if self.atol.len() == 1 {
                self.atol[0]
            } else {
                self.atol[row]
            };
            let scale = absolute + self.rtol * old_value.abs().max(new_value.abs());
            if !scale.is_finite() {
                return Err(CoreError::InvalidInput(
                    "affine physical error scale overflow".into(),
                ));
            }
            let ratio = scaled_error_component(lifted_error, scale).abs();
            if !ratio.is_finite() {
                infinite_ratio = true;
                continue;
            }
            if ratio > maximum {
                sum = if maximum == 0.0 {
                    1.0
                } else {
                    1.0 + sum * (maximum / ratio).powi(2)
                };
                maximum = ratio;
            } else if maximum > 0.0 {
                sum += (ratio / maximum).powi(2);
            }
        }
        if infinite_ratio {
            Ok(f64::INFINITY)
        } else {
            Ok(maximum * (sum / self.physical_dimension() as f64).sqrt())
        }
    }
}

// Neumaier compensated summation; no scratch vectors or hot-path allocation.
struct Compensated {
    sum: f64,
    correction: f64,
}

impl Compensated {
    fn new(value: f64) -> Self {
        Self {
            sum: value,
            correction: 0.0,
        }
    }

    fn add(&mut self, value: f64) -> CoreResult<()> {
        let next = self.sum + value;
        if !value.is_finite() || !next.is_finite() {
            return Err(CoreError::NonFinite(
                "affine lift accumulation overflow".into(),
            ));
        }
        self.correction += if self.sum.abs() >= value.abs() {
            (self.sum - next) + value
        } else {
            (value - next) + self.sum
        };
        self.sum = next;
        if !self.correction.is_finite() {
            return Err(CoreError::NonFinite(
                "affine lift compensation overflow".into(),
            ));
        }
        Ok(())
    }

    fn finish(self) -> CoreResult<f64> {
        let result = self.sum + self.correction;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(CoreError::NonFinite(
                "affine lifted value is nonfinite".into(),
            ))
        }
    }
}
