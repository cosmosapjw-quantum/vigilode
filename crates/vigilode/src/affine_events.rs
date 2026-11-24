//! Post-solve event enclosures for the scalar model `x' = -lambda*x + b`,
//! with constant parameter intervals, positive lambda and event `x=threshold`.
pub use rodas5p_core::directed::Interval;
use rodas5p_core::{CoreError, CoreResult};
pub use rodas5p_integrators::affine_event_certificate::{
    AFFINE_EVENT_CLOCK_ID, AffineEventBinding, AffineEventSign, AffineEventSpec, AffineEventStatus,
    AffineScalarModel,
};
use rodas5p_integrators::{FastDenseSegments, affine_event_certificate::AffineEventCertificate};

use crate::{Solution, solution::Extension};

/// One accepted segment which may contain a crossing. NoCrossing segments
/// are omitted. Adjacent unresolved segments can describe the same event.
#[derive(Debug, Clone)]
pub struct AffineEventWindow {
    /// Zero-based index in the native accepted-segment storage.
    pub segment_index: usize,
    /// The whole-segment existence and uniqueness verdict.
    pub status: AffineEventStatus,
    /// Certified enclosing times for a unique crossing, or the whole
    /// segment for an unresolved or existence-only verdict.
    pub times: [f64; 2],
}

fn bracket_unique(
    segments: &FastDenseSegments,
    cert: &AffineEventCertificate,
    spec: &AffineEventSpec,
) -> CoreResult<[f64; 2]> {
    let endpoints = cert.times();
    let signs = cert.endpoint_symbols();
    let mut bracket = endpoints;
    // Each end stays on its original certified side. An unresolved query can
    // limit that side's search, but never becomes an endpoint of the result.
    for side in 0..2 {
        let mut inside = endpoints[side];
        let mut outside = endpoints[1 - side];
        for _ in 0..64 {
            let mid = inside.midpoint(outside);
            if mid == inside || mid == outside {
                break;
            }
            if segments
                .query_affine_scalar_certificate(cert, spec, mid)?
                .symbol
                == signs[side]
            {
                inside = mid;
            } else {
                outside = mid;
            }
        }
        bracket[side] = inside;
    }
    Ok(bracket)
}

impl Solution {
    /// Enclose crossings of a declared scalar affine event on all accepted
    /// native fast-dense RODAS5P segments. The caller owns the model and
    /// parameter bounds, source identity and initial absolute error bound.
    /// The dense polynomial defect is checked against the declared equation.
    ///
    /// Each unique window encloses the exact crossing for every admissible
    /// trajectory. Existence-only and unresolved slabs are retained in full.
    /// A completed fast-dense run with intact native endpoints is required.
    /// This postprocessing does not change the trajectory or stop a solve.
    pub fn certified_affine_event_windows(
        &self,
        spec: &AffineEventSpec,
        initial_error: f64,
    ) -> CoreResult<Vec<AffineEventWindow>> {
        let Some(Extension::Fast(segments)) = &self.extension else {
            return Err(CoreError::InvalidInput(
                "affine event windows require native fast dense output".into(),
            ));
        };
        if !self.is_success()
            || segments.dimension() != 1
            || segments.is_empty()
            || segments.times().first() != self.t.first()
            || segments.times().last() != self.t.last()
            || !initial_error.is_finite()
            || initial_error < 0.0
        {
            return Err(CoreError::InvalidInput(
                "affine event windows require intact scalar segments and a finite nonnegative incoming error".into(),
            ));
        }
        let mut windows = Vec::new();
        let mut carry = None;
        for k in 0..segments.len() {
            let cert = match &carry {
                Some(previous) => {
                    segments.certify_affine_scalar_segment_with_carry(k, spec, previous)?
                }
                None => segments.certify_affine_scalar_segment(k, spec, initial_error)?,
            };
            let status = cert.event_status();
            if status != AffineEventStatus::NoCrossing {
                let times = if status == AffineEventStatus::UniqueCrossing {
                    bracket_unique(segments, &cert, spec)?
                } else {
                    cert.times()
                };
                windows.push(AffineEventWindow {
                    segment_index: k,
                    status,
                    times,
                });
            }
            carry = Some(cert.carry()?);
        }
        Ok(windows)
    }
}
