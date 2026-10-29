//! Positive, conservative overlap remapping of one-dimensional cell integrals.
//!
//! Edges are in a *measure coordinate*: interval lengths must equal the measure
//! used to define the cell integrals. For photon number this need not be raw
//! energy. A constructor allocates staging storage; successful and refused
//! `remap` calls allocate no heap storage. Caller output is unchanged on refusal.
//! This kernel does not resize an ODE driver or certify its temporal accuracy.

/// Refusals do not modify the caller's output slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemapError {
    ZeroCapacity,
    Allocation,
    EmptyGrid,
    CapacityExceeded,
    ShapeMismatch,
    NonFiniteEdge,
    NonPositiveOrNonFiniteWidth,
    DomainMismatch,
    InvalidCellIntegral,
    InvalidMassTolerance,
    ArithmeticOverflow,
    ContributionUnderflow,
    MassDefectExceeded,
}

/// Floating-point diagnostics, not outward-rounded error certificates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemapReport {
    pub source_cells: usize,
    pub destination_cells: usize,
    pub overlap_segments: usize,
    pub source_integral: f64,
    pub destination_integral: f64,
    pub relative_mass_defect: f64,
    /// L1 distance between the old and new piecewise-constant reconstructions
    /// on the common measure domain. This is a remap perturbation diagnostic,
    /// not an estimate of distance to the unknown physical solution.
    pub reconstruction_change_l1: f64,
}

/// Reusable staging and summation storage with fixed maximum cell count.
#[derive(Debug)]
pub struct ConservativeRemapWorkspace {
    staged: Vec<f64>,
    compensation: Vec<f64>,
}

impl ConservativeRemapWorkspace {
    pub fn new(max_cells: usize) -> Result<Self, RemapError> {
        if max_cells == 0 {
            return Err(RemapError::ZeroCapacity);
        }
        let mut staged = Vec::new();
        let mut compensation = Vec::new();
        staged
            .try_reserve_exact(max_cells)
            .map_err(|_| RemapError::Allocation)?;
        compensation
            .try_reserve_exact(max_cells)
            .map_err(|_| RemapError::Allocation)?;
        staged.resize(max_cells, 0.0);
        compensation.resize(max_cells, 0.0);
        Ok(Self {
            staged,
            compensation,
        })
    }

    pub fn max_cells(&self) -> usize {
        self.staged.len()
    }

    /// Map nonnegative cell integrals by piecewise-constant overlap fractions.
    ///
    /// Both grids must be strictly increasing, have finite positive widths,
    /// and exactly equal endpoints. Negative coordinates are allowed (for
    /// example mu=cos(theta)); negative integrals are refused. An application
    /// needing signed moments must select a separate representation.
    /// `max_relative_mass_defect` is a floating diagnostic admission threshold;
    /// the application must additionally budget discretization/remap errors.
    pub fn remap(
        &mut self,
        source_edges: &[f64],
        source_integrals: &[f64],
        destination_edges: &[f64],
        destination_integrals: &mut [f64],
        max_relative_mass_defect: f64,
    ) -> Result<RemapReport, RemapError> {
        let n = validate_grid(source_edges)?;
        let m = validate_grid(destination_edges)?;
        if n > self.max_cells() || m > self.max_cells() {
            return Err(RemapError::CapacityExceeded);
        }
        if source_integrals.len() != n || destination_integrals.len() != m {
            return Err(RemapError::ShapeMismatch);
        }
        if source_edges[0] != destination_edges[0] || source_edges[n] != destination_edges[m] {
            return Err(RemapError::DomainMismatch);
        }
        if !max_relative_mass_defect.is_finite() || max_relative_mass_defect < 0.0 {
            return Err(RemapError::InvalidMassTolerance);
        }
        let mut source_total = 0.0;
        let mut source_correction = 0.0;
        for &q in source_integrals {
            if !q.is_finite() || q < 0.0 {
                return Err(RemapError::InvalidCellIntegral);
            }
            compensated_add(&mut source_total, &mut source_correction, q)?;
        }
        self.staged[..m].fill(0.0);
        self.compensation[..m].fill(0.0);

        let mut segments = 0;
        visit_overlaps(source_edges, destination_edges, |i, j, overlap| {
            let term = contribution(
                source_integrals[i],
                overlap,
                source_edges[i + 1] - source_edges[i],
            )?;
            compensated_add(&mut self.staged[j], &mut self.compensation[j], term)?;
            segments += 1;
            Ok(())
        })?;

        let mut destination_total = 0.0;
        let mut destination_correction = 0.0;
        for &q in &self.staged[..m] {
            compensated_add(&mut destination_total, &mut destination_correction, q)?;
        }
        let relative_mass_defect = if source_total == 0.0 {
            0.0
        } else {
            (destination_total - source_total).abs() / source_total
        };
        if relative_mass_defect > max_relative_mass_defect {
            return Err(RemapError::MassDefectExceeded);
        }

        let mut change = 0.0;
        let mut change_correction = 0.0;
        visit_overlaps(source_edges, destination_edges, |i, j, overlap| {
            // Integrate the absolute difference without forming densities,
            // which could overflow on extremely narrow cells.
            let old = contribution(
                source_integrals[i],
                overlap,
                source_edges[i + 1] - source_edges[i],
            )?;
            let new = contribution(
                self.staged[j],
                overlap,
                destination_edges[j + 1] - destination_edges[j],
            )?;
            compensated_add(&mut change, &mut change_correction, (old - new).abs())
        })?;

        destination_integrals.copy_from_slice(&self.staged[..m]);
        Ok(RemapReport {
            source_cells: n,
            destination_cells: m,
            overlap_segments: segments,
            source_integral: source_total,
            destination_integral: destination_total,
            relative_mass_defect,
            reconstruction_change_l1: change,
        })
    }
}

fn validate_grid(edges: &[f64]) -> Result<usize, RemapError> {
    if edges.len() < 2 {
        return Err(RemapError::EmptyGrid);
    }
    if edges.iter().any(|x| !x.is_finite()) {
        return Err(RemapError::NonFiniteEdge);
    }
    for pair in edges.windows(2) {
        let width = pair[1] - pair[0];
        if !width.is_finite() || width <= 0.0 {
            return Err(RemapError::NonPositiveOrNonFiniteWidth);
        }
    }
    Ok(edges.len() - 1)
}

fn contribution(q: f64, overlap: f64, width: f64) -> Result<f64, RemapError> {
    if q == 0.0 {
        return Ok(0.0);
    }
    let fraction = overlap / width;
    let result = q * fraction;
    if result == 0.0 {
        return Err(RemapError::ContributionUnderflow);
    }
    if !result.is_finite() {
        return Err(RemapError::ArithmeticOverflow);
    }
    Ok(result)
}

fn compensated_add(sum: &mut f64, correction: &mut f64, value: f64) -> Result<(), RemapError> {
    let adjusted = value - *correction;
    let next = *sum + adjusted;
    if !next.is_finite() {
        return Err(RemapError::ArithmeticOverflow);
    }
    *correction = (next - *sum) - adjusted;
    *sum = next;
    Ok(())
}

fn visit_overlaps(
    a: &[f64],
    b: &[f64],
    mut visit: impl FnMut(usize, usize, f64) -> Result<(), RemapError>,
) -> Result<(), RemapError> {
    let (mut i, mut j) = (0, 0);
    while i + 1 < a.len() && j + 1 < b.len() {
        let right = a[i + 1].min(b[j + 1]);
        let left = a[i].max(b[j]);
        if right > left {
            visit(i, j, right - left)?;
        }
        let advance_a = a[i + 1] <= b[j + 1];
        let advance_b = b[j + 1] <= a[i + 1];
        if advance_a {
            i += 1;
        }
        if advance_b {
            j += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 32.0 * f64::EPSILON;

    #[test]
    fn nonuniform_refine_coarsen_preserves_integral_and_positivity() {
        let mut ws = ConservativeRemapWorkspace::new(8).unwrap();
        let coarse = [0.0, 1.0, 3.0];
        let fine = [0.0, 0.25, 1.0, 2.0, 3.0];
        let mut output = [0.0; 4];
        let report = ws
            .remap(&coarse, &[2.0, 6.0], &fine, &mut output, TOL)
            .unwrap();
        assert_eq!(output, [0.5, 1.5, 3.0, 3.0]);
        assert_eq!(report.source_integral, 8.0);
        assert_eq!(report.destination_integral, 8.0);
        assert_eq!(report.reconstruction_change_l1, 0.0);
        let mut back = [0.0; 2];
        ws.remap(&fine, &output, &coarse, &mut back, TOL).unwrap();
        assert_eq!(back, [2.0, 6.0]);
    }

    #[test]
    fn crossing_grid_has_hand_computed_projection_and_l1_change() {
        let mut ws = ConservativeRemapWorkspace::new(4).unwrap();
        let mut out = [0.0; 2];
        let report = ws
            .remap(
                &[0.0, 1.0, 3.0],
                &[2.0, 6.0],
                &[0.0, 2.0, 3.0],
                &mut out,
                TOL,
            )
            .unwrap();
        assert_eq!(out, [5.0, 3.0]);
        assert_eq!(report.reconstruction_change_l1, 1.0);
        assert_eq!(report.overlap_segments, 3);
    }

    #[test]
    fn weighted_l1_contracts_for_two_densities() {
        let mut ws = ConservativeRemapWorkspace::new(8).unwrap();
        let old = [-1.0, -0.5, 0.5, 1.0];
        let new = [-1.0, 0.0, 1.0];
        let q = [0.5, 2.0, 0.0];
        let r = [1.5, 0.0, 1.0];
        let mut q_new = [0.0; 2];
        let mut r_new = [0.0; 2];
        ws.remap(&old, &q, &new, &mut q_new, TOL).unwrap();
        ws.remap(&old, &r, &new, &mut r_new, TOL).unwrap();
        let before: f64 = q.iter().zip(r).map(|(a, b)| (a - b).abs()).sum();
        let after: f64 = q_new.iter().zip(r_new).map(|(a, b)| (a - b).abs()).sum();
        assert!(after < before);
        assert_eq!(after, 0.0);
    }

    #[test]
    fn invalid_inputs_and_capacity_leave_output_unchanged() {
        let mut ws = ConservativeRemapWorkspace::new(2).unwrap();
        let mut out = [17.0, 19.0];
        for (edges, mass, expected) in [
            (
                [0.0, 0.0, 1.0],
                [1.0, 1.0],
                RemapError::NonPositiveOrNonFiniteWidth,
            ),
            ([0.0, f64::NAN, 1.0], [1.0, 1.0], RemapError::NonFiniteEdge),
            (
                [0.0, 0.5, 1.0],
                [-1.0, 1.0],
                RemapError::InvalidCellIntegral,
            ),
            (
                [0.0, 0.5, 1.0],
                [f64::INFINITY, 1.0],
                RemapError::InvalidCellIntegral,
            ),
        ] {
            assert_eq!(
                ws.remap(&edges, &mass, &[0.0, 0.5, 1.0], &mut out, TOL),
                Err(expected)
            );
            assert_eq!(out, [17.0, 19.0]);
        }
        assert_eq!(
            ws.remap(
                &[0.0, 0.5, 1.0],
                &[1.0, 1.0],
                &[0.0, 0.5, 2.0],
                &mut out,
                TOL
            ),
            Err(RemapError::DomainMismatch)
        );
        assert_eq!(
            ws.remap(
                &[0.0, 0.25, 0.5, 1.0],
                &[1.0; 3],
                &[0.0, 0.5, 1.0],
                &mut out,
                TOL
            ),
            Err(RemapError::CapacityExceeded)
        );
        assert_eq!(
            ws.remap(&[0.0, 0.5, 1.0], &[1.0], &[0.0, 0.5, 1.0], &mut out, TOL),
            Err(RemapError::ShapeMismatch)
        );
        assert_eq!(out, [17.0, 19.0]);
    }

    #[test]
    fn arithmetic_refusals_are_atomic_and_workspace_remains_reusable() {
        let mut ws = ConservativeRemapWorkspace::new(4).unwrap();
        let mut out = [7.0, 11.0];
        assert_eq!(
            ws.remap(
                &[0.0, 0.5, 1.0],
                &[f64::MAX, f64::MAX],
                &[0.0, 0.5, 1.0],
                &mut out,
                TOL
            ),
            Err(RemapError::ArithmeticOverflow)
        );
        assert_eq!(
            ws.remap(
                &[0.0, 1.0],
                &[f64::from_bits(1)],
                &[0.0, 0.5, 1.0],
                &mut out,
                TOL
            ),
            Err(RemapError::ContributionUnderflow)
        );
        assert_eq!(out, [7.0, 11.0]);
        ws.remap(&[0.0, 1.0], &[1.0], &[0.0, 0.5, 1.0], &mut out, TOL)
            .unwrap();
        assert_eq!(out, [0.5, 0.5]);
    }

    #[test]
    fn repeated_dimension_changes_reuse_backing_storage() {
        let mut ws = ConservativeRemapWorkspace::new(8).unwrap();
        let staged_ptr = ws.staged.as_ptr();
        let correction_ptr = ws.compensation.as_ptr();
        let mut fine = [0.0; 4];
        let mut coarse = [1.0];
        for _ in 0..100 {
            ws.remap(
                &[0.0, 1.0],
                &coarse,
                &[0.0, 0.125, 0.25, 0.5, 1.0],
                &mut fine,
                TOL,
            )
            .unwrap();
            ws.remap(
                &[0.0, 0.125, 0.25, 0.5, 1.0],
                &fine,
                &[0.0, 1.0],
                &mut coarse,
                TOL,
            )
            .unwrap();
        }
        assert_eq!(coarse, [1.0]);
        assert_eq!(ws.staged.as_ptr(), staged_ptr);
        assert_eq!(ws.compensation.as_ptr(), correction_ptr);
        assert_eq!(ws.max_cells(), 8);
    }

    #[test]
    fn floating_mass_guard_refuses_before_commit() {
        let mut ws = ConservativeRemapWorkspace::new(8).unwrap();
        let mut out = [7.0; 4];
        let old = [0.0, 0.1, 0.2, 0.7, 1.0];
        let new = [0.0, 0.13, 0.41, 0.67, 1.0];
        let q = [1e-12, 0.026, 1.2, 1e-10];
        assert_eq!(
            ws.remap(&old, &q, &new, &mut out, 0.0),
            Err(RemapError::MassDefectExceeded)
        );
        assert_eq!(out, [7.0; 4]);
        let report = ws.remap(&old, &q, &new, &mut out, TOL).unwrap();
        assert!(report.relative_mass_defect > 0.0);
        assert!(report.relative_mass_defect < TOL);
        assert!(out.iter().all(|q| *q >= 0.0));
    }

    #[test]
    fn zero_mass_and_invalid_tolerance_are_explicit() {
        let mut ws = ConservativeRemapWorkspace::new(2).unwrap();
        let mut out = [3.0; 2];
        let report = ws
            .remap(&[-1.0, 1.0], &[0.0], &[-1.0, 0.0, 1.0], &mut out, 0.0)
            .unwrap();
        assert_eq!(out, [0.0, 0.0]);
        assert_eq!(report.relative_mass_defect, 0.0);
        assert_eq!(report.reconstruction_change_l1, 0.0);
        for tol in [f64::NAN, f64::INFINITY, -1.0] {
            assert_eq!(
                ws.remap(&[-1.0, 1.0], &[0.0], &[-1.0, 0.0, 1.0], &mut out, tol),
                Err(RemapError::InvalidMassTolerance)
            );
        }
        assert_eq!(out, [0.0, 0.0]);
    }
}
