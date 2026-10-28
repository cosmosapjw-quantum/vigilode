//! Finite, exact matrix-identity key. This is not a stale-Jacobian policy.
//! A caller may record only factors successfully built for these entries.
pub(super) struct ExactBandCache {
    shift: Option<u64>,
    entries: Vec<u64>,
}
impl ExactBandCache {
    pub(super) fn new(len: usize) -> Self {
        Self {
            shift: None,
            entries: vec![0; len],
        }
    }
    pub(super) fn invalidate(&mut self) {
        self.shift = None;
    }
    pub(super) fn matches(&self, shift: f64, entries: &[f64]) -> bool {
        shift.is_finite()
            && self.shift == Some(shift.to_bits())
            && self.entries.len() == entries.len()
            && self
                .entries
                .iter()
                .zip(entries)
                .all(|(old, x)| *old == x.to_bits())
    }
    pub(super) fn record(&mut self, shift: f64, entries: &[f64]) -> bool {
        self.invalidate();
        if !shift.is_finite()
            || self.entries.len() != entries.len()
            || entries.iter().any(|x| !x.is_finite())
        {
            return false;
        }
        for (dst, x) in self.entries.iter_mut().zip(entries) {
            *dst = x.to_bits();
        }
        self.shift = Some(shift.to_bits());
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_matrix_hits() {
        let mut c = ExactBandCache::new(2);
        assert!(c.record(4., &[1., 2.]));
        assert!(c.matches(4., &[1., 2.]));
    }
    #[test]
    fn changed_shift_misses() {
        let mut c = ExactBandCache::new(2);
        c.record(4., &[1., 2.]);
        assert!(!c.matches(f64::from_bits(4f64.to_bits() + 1), &[1., 2.]));
    }
    #[test]
    fn changed_entry_misses() {
        let mut c = ExactBandCache::new(2);
        c.record(4., &[1., 2.]);
        assert!(!c.matches(4., &[1., f64::from_bits(2f64.to_bits() + 1)]));
    }
    #[test]
    fn failed_record_invalidates() {
        let mut c = ExactBandCache::new(2);
        c.record(4., &[1., 2.]);
        assert!(!c.record(4., &[f64::NAN, 2.]));
        assert!(!c.matches(4., &[1., 2.]));
    }
    #[test]
    fn shape_change_misses() {
        let mut c = ExactBandCache::new(2);
        c.record(4., &[1., 2.]);
        assert!(!c.matches(4., &[1.]));
        assert!(!c.record(4., &[1.]));
    }
    #[test]
    fn explicit_invalidation_misses() {
        let mut c = ExactBandCache::new(1);
        c.record(4., &[1.]);
        c.invalidate();
        assert!(!c.matches(4., &[1.]));
    }
    #[test]
    fn nonfinite_shift_misses() {
        let mut c = ExactBandCache::new(1);
        assert!(!c.record(f64::INFINITY, &[1.]));
        assert!(!c.matches(f64::INFINITY, &[1.]));
    }
    #[test]
    fn signed_zero_is_conservative() {
        let mut c = ExactBandCache::new(1);
        c.record(4., &[0.]);
        assert!(c.matches(4., &[0.]));
        assert!(!c.matches(4., &[-0.]));
    }
}
