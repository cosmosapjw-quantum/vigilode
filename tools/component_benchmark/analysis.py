"""Conservative numerical and paired-process analysis."""
from __future__ import annotations

import math
import statistics

TIMING_SCOPE = "warmed_solver_setup_integration_output_certificate"


def paired_log_cost(base, candidate):
    """Return a paired log ratio only for the frozen warmed timing boundary."""
    for result in (base, candidate):
        if not isinstance(result, dict) or result.get("measurement_mode") != "timing" or result.get("timing_scope") != TIMING_SCOPE:
            return None
        timing = result.get("timing")
        if not isinstance(timing, dict) or not finite_time(timing.get("timed_seconds")) or not finite_warmup(timing.get("warmup_seconds")):
            return None
    return math.log(base["timing"]["timed_seconds"] / candidate["timing"]["timed_seconds"])


def finite_time(value):
    return type(value) in (int,float) and math.isfinite(value) and value > 0


def finite_warmup(value):
    return type(value) in (int,float) and math.isfinite(value) and value >= 0


def reference_gate(case, candidate, reference, uncertainty):
    """Candidate-independent budget, with an empirical uncertainty ceiling."""
    if candidate is None or reference is None or uncertainty is None:
        return {"status": "HOLD", "reason": "MISSING_REFERENCE_OR_RESULT"}
    times = case["output_times"]
    if len(candidate) != len(times) or len(reference) != len(times) or len(uncertainty) != len(times):
        return {"status": "HOLD", "reason": "MESH_MISMATCH"}
    worst = 0.0
    lower = 0.0
    upper = 0.0
    worst_uncertainty = 0.0
    for row, ref, unc in zip(candidate, reference, uncertainty):
        if len(row) != len(case["atol"]) or len(ref) != len(row) or len(unc) != len(row):
            return {"status": "HOLD", "reason": "DIMENSION_MISMATCH"}
        for got, target, spread, atol in zip(row, ref, unc, case["atol"]):
            budget = atol + case["rtol"] * abs(target)
            if not all(math.isfinite(x) for x in (got, target, spread, budget)) or spread < 0 or budget <= 0:
                return {"status": "HOLD", "reason": "INVALID_NUMERICAL_INPUT"}
            worst = max(worst, abs(got-target)/budget)
            lower = max(lower, max(0.0, abs(got-target)-spread)/budget)
            upper = max(upper, (abs(got-target)+spread)/budget)
            worst_uncertainty = max(worst_uncertainty, spread/budget)
    if worst_uncertainty > .05:
        return {"status": "HOLD", "reason": "REFERENCE_UNCERTAINTY_GT_5_PERCENT", "max_uncertainty_budget_ratio": worst_uncertainty}
    return {"status": "PASS" if worst + worst_uncertainty <= 1 else "FAIL", "max_error_budget_ratio": worst,
            "max_uncertainty_budget_ratio": worst_uncertainty, "conservative_upper": worst + worst_uncertainty,
            "achieved_error_lower": lower, "achieved_error_upper": upper,
            "reference_authority": "NUMERICAL_CONVERGENCE_NOT_CERTIFICATE"}


def same_achieved_accuracy(base_gate, candidate_gate, limit=1.05):
    if base_gate.get("status") != "PASS" or candidate_gate.get("status") != "PASS":
        return {"status":"HOLD","reason":"ARM_ACCURACY_NOT_ADMITTED"}
    lb, ub = base_gate["achieved_error_lower"], base_gate["achieved_error_upper"]
    lc, uc = candidate_gate["achieved_error_lower"], candidate_gate["achieved_error_upper"]
    if lb == ub == lc == uc == 0:
        return {"status":"PASS","worst_ratio":1.0,"exact_zero_both":True}
    if lb <= 0 or lc <= 0:
        return {"status":"HOLD","reason":"ZERO_LOWER_ERROR_BOUND"}
    ratio = max(uc/lb,ub/lc)
    return {"status":"PASS" if ratio <= limit else "HOLD", "worst_ratio":ratio,
            "limit":limit,"exact_zero_both":False}


def interval_index(n=17, multiplicity=1, alpha=.05):
    if n != 17 or type(multiplicity) is not int or multiplicity < 1:
        raise ValueError("frozen design requires 17 rounds and positive multiplicity")
    return max((k for k in range(1, (n+1)//2+1) if 2*sum(math.comb(n,j) for j in range(k))/2**n <= alpha/multiplicity), default=None)


def paired_median_ci(log_ratios, multiplicity):
    if len(log_ratios) != 17 or any(not isinstance(x, (int,float)) or not math.isfinite(x) for x in log_ratios):
        return {"status": "HOLD", "reason": "REQUIRES_17_INDEPENDENT_FINITE_PAIRS"}
    k = interval_index(17, multiplicity)
    if k is None:
        return {"status": "HOLD", "reason": "MULTIPLICITY_PRECLUDES_95_PERCENT_INTERVAL"}
    ordered = sorted(log_ratios)
    return {"status": "INTERVAL", "n": 17, "multiplicity": multiplicity, "k": k,
            "median_log_ratio": statistics.median(ordered),
            "ci_log_ratio": [ordered[k-1], ordered[17-k]],
            "ci_speed_ratio": [math.exp(ordered[k-1]), math.exp(ordered[17-k])],
            "method": "exact_binomial_order_statistic_bonferroni"}


def recommendation(coverage, accuracy, ci, critical_regression=False, compensating_benefit=None):
    if not coverage or accuracy != "PASS" or ci.get("status") != "INTERVAL" or critical_regression:
        return {"action": "need to optimize", "evidence_status": "HOLD", "reason": "INCOMPLETE_OR_UNSAFE_EVIDENCE"}
    low, high = ci["ci_speed_ratio"]
    if low >= 1.10:
        return {"action": "promote", "evidence_status": "CONFIRMED_DIAGNOSTIC", "production_admission": "HOLD"}
    if high <= 1/1.20 and compensating_benefit is False:
        return {"action": "prune", "evidence_status": "CONFIRMED_DIAGNOSTIC", "source_deletion": False, "production_admission": "HOLD"}
    return {"action": "need to optimize", "evidence_status": "HOLD", "reason": "INCONCLUSIVE_CONTRAST", "production_admission": "HOLD"}
