#!/usr/bin/env python3
"""Exact finite checks for the integrated physical-error contract.

This program deliberately has no native-solver dependency. It is not to be
executed before the parent records its frozen source for the supporting run.
"""
from fractions import Fraction as Q
from itertools import product
import json


def encoded(value):
    if isinstance(value, Q):
        return str(value.numerator) + "/" + str(value.denominator)
    if isinstance(value, dict):
        return {key: encoded(item) for key, item in value.items()}
    if isinstance(value, (tuple, list)):
        return [encoded(item) for item in value]
    return value


def switching_check():
    """Actual-state selector may be discontinuous; per-step induction holds."""
    gains = [Q(1, 2), Q(3, 4), Q(1, 4), Q(2, 3)] * 2
    offsets = [Q(1, 7), Q(-1, 5), Q(1, 3), Q(-1, 9)] * 2
    exact = Q(1, 4)
    actual = Q(1, 3)
    bound = abs(actual - exact)
    rows = []
    for i, (gain, offset) in enumerate(zip(gains, offsets)):
        # A discontinuous choice depending on the actual represented state.
        selected = "positive" if actual >= 0 else "negative"
        defect = Q(1, 100) if selected == "positive" else Q(-1, 40)
        exact = gain * exact + offset
        actual = gain * actual + offset + defect
        bound = gain * bound + abs(defect)
        error = abs(actual - exact)
        assert error <= bound
        rows.append({"step": i + 1, "backend": selected,
                     "actual_error": error, "certified_bound": bound})
    assert {row["backend"] for row in rows} == {"positive", "negative"}
    # Saturating family: every defect and initial error has the same sign.
    initial = Q(1, 12)
    error = initial
    bound = initial
    for gain in gains:
        error = gain * error + Q(1, 100)
        bound = gain * bound + Q(1, 100)
        assert error == bound
    return {"status": "PASS", "rows": rows,
            "positive_defect_family_saturates": error}


def anchors_check():
    exact_input = Q(0)
    actual_input = Q(1, 5)
    flow = lambda x: x / 2
    discrete = lambda x: x
    error = abs(discrete(actual_input) - flow(exact_input))
    exact_truncation = abs(discrete(exact_input) - flow(exact_input))
    numerical_truncation = abs(discrete(actual_input) - flow(actual_input))
    invalid = Q(1, 2) * abs(actual_input - exact_input) + exact_truncation
    valid_flow = Q(1, 2) * abs(actual_input - exact_input) + numerical_truncation
    valid_discrete = abs(actual_input - exact_input) + exact_truncation
    assert error > invalid
    assert error == valid_flow == valid_discrete
    return {"status": "PASS", "error": error, "invalid_mixed_bound": invalid,
            "valid_flow_bound": valid_flow, "valid_discrete_bound": valid_discrete}


def representation_clock_check():
    weights = [Q(1), Q(1000)]
    vector = [Q(0), Q(1000)]
    coordinate_norm = max(abs(x) / w for x, w in zip(vector, weights))
    physical_norm = max(abs(x) for x in vector)
    assert coordinate_norm == 1
    assert physical_norm == max(weights) * coordinate_norm == 1000
    # Internal exact trajectory y(tau)=tau, with separately represented labels.
    clock_rows = []
    cumulative_label_terms = Q(0)
    for i in range(1, 6):
        tau = Q(i, 7)
        displacement = Q((-1) ** i, 100)
        label = tau + displacement
        numerical_state = tau
        exact_at_label = label
        observed = abs(numerical_state - exact_at_label)
        bound = abs(label - tau)  # derivative bound M=1, internal error=0
        cumulative_label_terms += abs(displacement)
        assert observed == bound == Q(1, 100)
        clock_rows.append({"step": i, "observation_error": observed,
                           "unnecessarily_repropagated_sum": cumulative_label_terms})
    assert cumulative_label_terms == Q(1, 20)
    # Exact edge ownership: one scalar strongly monotone root and an update.
    root = Q(1, 2)
    approximate_root = Q(51, 100)
    residual = 2 * approximate_root - 1
    update_defect = Q(1, 1000)
    actual_output = approximate_root + update_defect
    bound = abs(residual) / 2 + abs(update_defect)
    assert abs(actual_output - root) == bound
    return {"status": "PASS", "coordinate_norm": coordinate_norm,
            "physical_norm": physical_norm, "clock_rows": clock_rows,
            "root_plus_update_bound": bound}


def dominates(left, right):
    """No larger error and no larger cost, with at least one strict."""
    le = left["error"] <= right["error"] and left["cost"] <= right["cost"]
    strict = left["error"] < right["error"] or left["cost"] < right["cost"]
    return le and strict


def menu_check():
    weights = [Q(2), Q(1)]
    menus = [[(Q(1, 10), 4), (Q(1, 20), 7)],
             [(Q(1, 5), 1), (Q(1, 10), 3), (Q(1, 20), 6)]]
    cap = Q(3, 10)
    frontier = [{"error": Q(0), "cost": 0, "choices": ()}]
    for weight, menu in zip(weights, menus):
        extended = []
        for label in frontier:
            for j, (error, cost) in enumerate(menu):
                extended.append({"error": label["error"] + weight * error,
                                 "cost": label["cost"] + cost,
                                 "choices": label["choices"] + (j,)})
        frontier = [label for label in extended
                    if not any(dominates(other, label) for other in extended)]
    feasible_frontier = [label for label in frontier if label["error"] <= cap]
    chosen = min(feasible_frontier, key=lambda row: row["cost"])
    exhaustive = []
    for indices in product(*(range(len(menu)) for menu in menus)):
        error = sum((weight * menu[index][0]
                     for weight, menu, index in zip(weights, menus, indices)), Q(0))
        cost = sum(menu[index][1] for menu, index in zip(menus, indices))
        if error <= cap:
            exhaustive.append({"error": error, "cost": cost, "choices": indices})
    assert chosen == min(exhaustive, key=lambda row: row["cost"])
    assert chosen == {"error": Q(3, 10), "cost": 7, "choices": (0, 1)}
    equal_half_choices = []
    for weight, menu in zip(weights, menus):
        permitted = [(cost, j) for j, (error, cost) in enumerate(menu)
                     if weight * error <= cap / 2]
        equal_half_choices.append(min(permitted))
    equal_half_cost = sum(row[0] for row in equal_half_choices)
    assert equal_half_cost == 10
    # These partial states have different future cost contracts.
    cold_total = 0 + 100
    warm_total = 1 + 0
    assert 0 < 1 and warm_total < cold_total
    return {"status": "PASS", "optimum": chosen, "equal_half_cost": equal_half_cost,
            "frontier": frontier,
            "state_sensitive_counterexample": {"cold_total": cold_total,
                                                  "warm_total": warm_total}}


def continuous_allocation_check():
    a = [Q(1), Q(3)]
    w = [Q(2), Q(1)]
    budget = Q(1, 2)
    eta = [budget * ai / (wi * sum(a)) for ai, wi in zip(a, w)]
    assert eta == [Q(1, 16), Q(3, 8)]
    assert sum(wi * ei for wi, ei in zip(w, eta)) == budget
    multipliers = [ai / (wi * ei) for ai, wi, ei in zip(a, w, eta)]
    assert multipliers == [Q(8), Q(8)]
    optimal_product = eta[0] * eta[1] ** 3
    equal_eta = [budget / (2 * wi) for wi in w]
    equal_product = equal_eta[0] * equal_eta[1] ** 3
    assert optimal_product == Q(27, 8192) > equal_product == Q(1, 512)
    return {"status": "PASS", "allocation": eta, "multipliers": multipliers,
            "optimal_product": optimal_product, "equal_product": equal_product}


def precision_check():
    target = Q(1, 3)
    spacing = Q(1, 8)
    lower_index = target // spacing
    candidates = [spacing * lower_index, spacing * (lower_index + 1)]
    nearest = min(candidates, key=lambda x: abs(x - target))
    lower_distance = abs(nearest - target)
    assert nearest == Q(3, 8)
    assert lower_distance == Q(1, 24) > Q(1, 100)
    representable_target = Q(1, 2)
    loose_upper = Q(1)
    requested = Q(1, 10)
    actual_error = abs(representable_target - representable_target)
    assert loose_upper > requested > actual_error
    return {"status": "PASS", "nearest_lattice_output": nearest,
            "proved_distance": lower_distance,
            "loose_upper_bound_does_not_prove_impossibility": True}


def main():
    results = {
        "status": "PASS",
        "scope": "exact finite supporting identities; no native solver validation",
        "switching": switching_check(),
        "anchors": anchors_check(),
        "representation_clock": representation_clock_check(),
        "finite_menu": menu_check(),
        "continuous_allocation": continuous_allocation_check(),
        "precision": precision_check(),
    }
    print(json.dumps(encoded(results), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
