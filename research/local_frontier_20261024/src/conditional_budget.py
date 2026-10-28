#!/usr/bin/env python3
"""Exact-rational bounded offline planner for conditional stage residual work.

This research prototype does not alter solver tolerances or assert its caller's
sensitivity/contraction bounds. Costs are declared integer work units, not time.
"""
from fractions import Fraction as F


def weights(lower, output, inverse):
    """w=M(I-K)^(-T)|b| for strictly lower triangular nonnegative K.

    Inputs are interpreted exactly as Fractions; pass strings for decimal data.
    Stage perturbation contract: d_i <= M_i r_i + sum_j K_ij d_j.
    """
    n = len(output)
    if not 0 < n <= 16 or len(lower) != n or len(inverse) != n:
        raise ValueError('dimension outside bounded planner')
    k = [[F(x) for x in row] for row in lower]
    b, m = [abs(F(x)) for x in output], [F(x) for x in inverse]
    if any(len(row) != n for row in k) or any(x < 0 for row in k for x in row) or any(x <= 0 for x in m):
        raise ValueError('invalid sensitivity bounds')
    if any(k[i][j] != 0 for i in range(n) for j in range(i, n)):
        raise ValueError('K must be strictly lower triangular')
    z = b[:]
    for j in range(n - 1, -1, -1):
        z[j] += sum(k[i][j] * z[i] for i in range(j + 1, n))
    return [m[i] * z[i] for i in range(n)]


def allocate(weight, initial, floor, contraction, unit_cost, max_iterations, target, max_work=20000):
    """Minimum declared integer work under exact residual-envelope constraints.

    Each residual envelope is floor_i+(initial_i-floor_i)*q_i**k_i.
    Fixed setup cost does not affect the argmin; charge it separately.
    None means the given bounded choices cannot certify the target.
    """
    n = len(weight)
    if not 0 < n <= 16 or any(len(x) != n for x in [initial, floor, contraction, unit_cost, max_iterations]):
        raise ValueError('inconsistent dimension')
    w, r, e, q = [[F(v) for v in x] for x in [weight, initial, floor, contraction]]
    target = F(target)
    if target <= 0 or any(x < 0 for x in w) or any(e[i] < 0 or r[i] < e[i] or not 0 <= q[i] < 1 for i in range(n)):
        raise ValueError('invalid conditional residual envelope')
    if not isinstance(max_work, int) or not 0 < max_work <= 20000:
        raise ValueError('work cap outside planner limit')
    if any(type(c) is not int or c <= 0 for c in unit_cost) or any(type(k) is not int or not 0 <= k <= 128 for k in max_iterations):
        raise ValueError('invalid discrete costs/caps')
    if sum(w[i] * e[i] for i in range(n)) > target:
        return None
    states = {0: (F(0), ())}
    for i in range(n):
        choices = [(k * unit_cost[i], w[i] * (e[i] + (r[i] - e[i]) * q[i]**k), k)
                   for k in range(max_iterations[i] + 1)]
        next_states = {}
        for cost, (bound, path) in states.items():
            for extra, contribution, k in choices:
                total = cost + extra
                if total > max_work:
                    continue
                candidate = (bound + contribution, path + (k,))
                if total not in next_states or candidate[0] < next_states[total][0]:
                    next_states[total] = candidate
        # Higher cost with no smaller error can never improve a later stage.
        states, best = {}, None
        for cost in sorted(next_states):
            bound, path = next_states[cost]
            if best is None or bound < best:
                states[cost] = (bound, path)
                best = bound
    feasible = [(cost, bound, path) for cost, (bound, path) in states.items() if bound <= target]
    if not feasible:
        return None
    cost, bound, path = min(feasible)
    return {'declared_work_units': cost, 'residual_bound': str(bound), 'iterations': list(path),
            'target': str(target), 'scope': 'exact declared envelopes; no inferred callback bounds or wall-time guarantee'}
