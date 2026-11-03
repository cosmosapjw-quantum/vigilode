//! Margin-shifted strict comparisons (hysteresis band) and their
//! decision-preserving callback caps. A ratio is `num / den` with `0 < num <= den`.

/// Hysteresis ratio `num / den` in (0, 1].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarginRatio {
    /// Numerator, `0 < num <= den`.
    pub num: u64,
    /// Denominator.
    pub den: u64,
}

impl MarginRatio {
    /// `Some` when `0 < num <= den`.
    pub fn new(num: u64, den: u64) -> Option<Self> {
        (num > 0 && num <= den).then_some(Self { num, den })
    }
}

/// True when the band is non-degenerate: `rho_e * rho_r < 1`.
pub fn band_is_open(entry: MarginRatio, ret: MarginRatio) -> bool {
    (entry.num as u128) * (ret.num as u128) < (entry.den as u128) * (ret.den as u128)
}

/// Enter iff `candidate < rho_e * incumbent`, evaluated as
/// `candidate * den < incumbent * num` in u128. `None` on overflow.
pub fn margin_entry_wins(candidate: u128, incumbent: u128, rho: MarginRatio) -> Option<bool> {
    let lhs = candidate.checked_mul(rho.den as u128)?;
    let rhs = incumbent.checked_mul(rho.num as u128)?;
    Some(lhs < rhs)
}

/// Largest callback prefix L that can still win entry when the candidate cost
/// is at least L: floor((incumbent * num - 1) / den), or 0 when incumbent * num == 0.
pub fn margin_entry_cap(incumbent: u128, rho: MarginRatio) -> Option<u128> {
    let scaled = incumbent.checked_mul(rho.num as u128)?;
    Some(scaled.saturating_sub(1) / rho.den as u128)
}

/// Return to explicit iff `explicit < rho_r * implicit`, i.e.
/// `explicit * den < implicit * num`. `None` on overflow.
pub fn margin_return_wins(explicit: u128, implicit: u128, rho: MarginRatio) -> Option<bool> {
    let lhs = explicit.checked_mul(rho.den as u128)?;
    let rhs = implicit.checked_mul(rho.num as u128)?;
    Some(lhs < rhs)
}

/// An implicit comparator whose callback prefix L exceeds floor(explicit * den / num)
/// already proves the return: cap = floor(explicit * den / num).
pub fn margin_return_cap(explicit: u128, rho: MarginRatio) -> Option<u128> {
    Some(explicit.checked_mul(rho.den as u128)? / rho.num as u128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_validation() {
        assert_eq!(MarginRatio::new(0, 5), None);
        assert_eq!(MarginRatio::new(6, 5), None);
        assert_eq!(MarginRatio::new(4, 5), Some(MarginRatio { num: 4, den: 5 }));
        let r = MarginRatio::new(4, 5).expect("valid");
        let one = MarginRatio::new(1, 1).expect("valid");
        assert!(band_is_open(r, r));
        assert!(!band_is_open(one, one));
        let two = MarginRatio::new(2, 2).expect("valid");
        assert!(!band_is_open(two, two));
        assert!(!band_is_open(two, one));
        assert!(!band_is_open(one, two));
        assert!(band_is_open(r, one));
    }

    #[test]
    fn boundary_table() {
        let r = MarginRatio::new(4, 5).expect("valid");
        assert_eq!(margin_entry_wins(79, 100, r), Some(true));
        assert_eq!(margin_entry_wins(80, 100, r), Some(false));
        assert_eq!(margin_entry_cap(100, r), Some(79));
        assert_eq!(margin_entry_cap(0, r), Some(0));
        assert_eq!(margin_return_wins(79, 100, r), Some(true));
        assert_eq!(margin_return_wins(80, 100, r), Some(false));
        assert_eq!(margin_return_cap(80, r), Some(100));
        assert_eq!(margin_entry_wins(u128::MAX, 1, r), None);
        assert_eq!(margin_return_cap(u128::MAX, r), None);
    }

    #[test]
    fn caps_are_decision_preserving() {
        for (num, den) in [(1u64, 1u64), (4, 5), (1, 3), (7, 10)] {
            let r = MarginRatio::new(num, den).expect("valid");
            for inc in 0u128..200 {
                let cap = margin_entry_cap(inc, r).expect("small");
                for cost in 0u128..300 {
                    let wins = margin_entry_wins(cost, inc, r).expect("small");
                    if wins {
                        assert!(
                            cost <= cap,
                            "winner cost {cost} above cap {cap} (inc {inc}, {num}/{den})"
                        );
                    }
                }
                let rcap = margin_return_cap(inc, r).expect("small");
                for imp in 0u128..400 {
                    if imp > rcap {
                        assert_eq!(
                            margin_return_wins(inc, imp, r),
                            Some(true),
                            "imp {imp} cap {rcap}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn unit_ratio_reduces_to_existing_strict_rules() {
        let one = MarginRatio::new(1, 1).expect("valid");
        for e in 0u128..100 {
            assert_eq!(margin_entry_cap(e, one), Some(e.saturating_sub(1)));
            assert_eq!(margin_return_cap(e, one), Some(e));
        }
    }
}
