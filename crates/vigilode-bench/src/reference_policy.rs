//! Input checks for an independent empirical reference comparison.

/// Validate the numerical reference policy before any solver is run.
/// Different implementations are necessary for independence, not sufficient
/// for a rigorous reference enclosure or a particular agreement threshold.
pub fn validate_reference_policy(
    rtol: f64,
    crosscheck_rtols: &[f64],
    max_steps: usize,
    dispute_threshold: f64,
    reference_family: &str,
    crosscheck_family: &str,
) -> Result<(), String> {
    let positive = |x: f64| x.is_finite() && x > 0.0;
    if !positive(rtol)
        || crosscheck_rtols.is_empty()
        || crosscheck_rtols.iter().any(|&x| !positive(x))
        || max_steps == 0
        || !dispute_threshold.is_finite()
        || dispute_threshold < 0.0
    {
        return Err("reference tolerances and attempt budget must be positive; dispute threshold must be finite and nonnegative".into());
    }
    if crosscheck_rtols.windows(2).any(|w| w[0] >= w[1]) {
        return Err("cross-check tolerances must be strictly increasing (tightest first)".into());
    }
    if reference_family == crosscheck_family {
        return Err("decisive cross-check must use a different solver implementation".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_reference_policy as validate;

    #[test]
    fn positive_independent_policy_and_exact_agreement_are_valid() {
        assert!(validate(1e-12, &[1e-12, 1e-11], 100, 1e-8, "rodas5p", "diffsol").is_ok());
        assert!(validate(1e-12, &[1e-12], 100, 0.0, "radau5", "cvode").is_ok());
    }

    #[test]
    fn nan_infinite_or_negative_threshold_cannot_disable_disputes() {
        for threshold in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(validate(1e-12, &[1e-12], 100, threshold, "a", "b").is_err());
        }
    }

    #[test]
    fn invalid_tolerances_budgets_and_order_are_refused() {
        for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(validate(tolerance, &[1e-12], 100, 1e-8, "a", "b").is_err());
            assert!(validate(1e-12, &[tolerance], 100, 1e-8, "a", "b").is_err());
        }
        assert!(validate(1e-12, &[], 100, 1e-8, "a", "b").is_err());
        assert!(validate(1e-12, &[1e-12], 0, 1e-8, "a", "b").is_err());
        assert!(validate(1e-12, &[1e-10, 1e-12], 100, 1e-8, "a", "b").is_err());
        assert!(validate(1e-12, &[1e-12, 1e-12], 100, 1e-8, "a", "b").is_err());
    }

    #[test]
    fn same_code_with_different_arm_names_is_not_independent() {
        assert!(validate(1e-12, &[1e-12], 100, 1e-8, "rodas5p", "rodas5p").is_err());
    }
}

/// New, opt-in empirical reference tier. It is not an interval enclosure.
pub const INDEPENDENT_TIER: &str = "radau5_cvode_refinement_v1";

/// Validate a preregistered refinement ladder; every level must run, with no
/// first-success fallback or relaxed post-hoc thresholds.
pub fn validate_independent_policy(
    tier: &str,
    rtols: &[f64],
    atol_factors: &[f64],
    agreement_threshold: f64,
    refinement_threshold: f64,
) -> Result<(), String> {
    let positive = |x: f64| x.is_finite() && x > 0.0;
    if tier != INDEPENDENT_TIER
        || rtols.len() < 2
        || rtols.len() != atol_factors.len()
        || rtols.iter().any(|&x| !positive(x))
        || atol_factors.iter().any(|&x| !positive(x))
        || rtols.first().is_none_or(|&x| x > 1e-10)
        || atol_factors.first().is_none_or(|&x| x > 1.0)
        || rtols.windows(2).any(|w| w[0] <= w[1])
        || atol_factors.windows(2).any(|w| w[0] <= w[1])
        || !positive(agreement_threshold)
        || agreement_threshold > 1e-10
        || !positive(refinement_threshold)
        || refinement_threshold > 1e-10
    {
        return Err("independent tier requires at least two strictly tighter rtol/atol levels, initial rtol <= 1e-10, initial atol factor <= 1, and positive agreement/refinement thresholds <= 1e-10".into());
    }
    Ok(())
}

/// Fail closed on incomplete, failed or nonfinite comparisons at any level.
/// Adjacent refinement checks are required except at the first level.
pub fn independent_level_pass(
    complete_success: bool,
    cross_family: Option<f64>,
    previous_levels: Option<(Option<f64>, Option<f64>)>,
    agreement_threshold: f64,
    refinement_threshold: f64,
) -> bool {
    let within =
        |d: Option<f64>, limit: f64| d.is_some_and(|x| x.is_finite() && x >= 0.0 && x <= limit);
    complete_success
        && within(cross_family, agreement_threshold)
        && previous_levels
            .is_none_or(|(a, b)| within(a, refinement_threshold) && within(b, refinement_threshold))
}

#[cfg(test)]
mod independent_tests {
    use super::*;
    #[test]
    fn refinement_policy_is_predeclared_and_strict() {
        let valid = |rt: &[f64], at: &[f64], t| {
            validate_independent_policy(INDEPENDENT_TIER, rt, at, t, 1e-10)
        };
        assert!(valid(&[1e-11, 1e-12], &[1.0, 0.1], 1e-10).is_ok());
        for (rt, at) in [
            (vec![1e-12], vec![1.0]),
            (vec![1e-12, 1e-11], vec![1.0, 0.1]),
            (vec![1e-11, 1e-12], vec![1.0, 1.0]),
            (vec![1e-11, 1e-12], vec![1.0]),
        ] {
            assert!(valid(&rt, &at, 1e-10).is_err());
        }
        for threshold in [0.0, 1e-8, f64::NAN, f64::INFINITY] {
            assert!(valid(&[1e-11, 1e-12], &[1.0, 0.1], threshold).is_err());
        }
    }
    #[test]
    fn all_levels_both_families_and_finite_metrics_are_required() {
        assert!(independent_level_pass(
            true,
            Some(1e-11),
            None,
            1e-10,
            1e-10
        ));
        assert!(independent_level_pass(
            true,
            Some(1e-11),
            Some((Some(1e-11), Some(0.0))),
            1e-10,
            1e-10
        ));
        for d in [
            None,
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(1e-8),
            Some(-1.0),
        ] {
            assert!(!independent_level_pass(true, d, None, 1e-10, 1e-10));
            assert!(!independent_level_pass(
                true,
                Some(0.0),
                Some((d, Some(0.0))),
                1e-10,
                1e-10
            ));
        }
        assert!(!independent_level_pass(
            false,
            Some(0.0),
            None,
            1e-10,
            1e-10
        ));
    }
}
