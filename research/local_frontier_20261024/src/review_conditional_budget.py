#!/usr/bin/env python3
"""Independent deterministic exhaustive checks; no ODE or timing promotion."""
from pathlib import Path
from fractions import Fraction as F
import hashlib
import itertools
import json
import random
from conditional_budget import weights, allocate

PKG = Path(__file__).resolve().parents[1]
ROOT = PKG.parents[1]
rng = random.Random(20261024)
checked = []
for index in range(64):
    n = rng.randint(1, 4)
    w = [F(rng.randrange(5), rng.randint(1, 5)) for _ in range(n)]
    e = [F(rng.randrange(3), 10) for _ in range(n)]
    initial = [e[i] + F(rng.randrange(4), 3) for i in range(n)]
    q = [F(rng.randrange(4), 4) for _ in range(n)]
    costs = [rng.randint(1, 4) for _ in range(n)]
    caps = [rng.randrange(5) for _ in range(n)]
    target, max_work = F(rng.randint(1, 30), 20), rng.randint(1, 25)
    ans = allocate(w, initial, e, q, costs, caps, target, max_work=max_work)
    choices = []
    for ks in itertools.product(*(range(c + 1) for c in caps)):
        work = sum(c * k for c, k in zip(costs, ks))
        bound = sum(w[i] * (e[i] + (initial[i] - e[i]) * q[i]**ks[i]) for i in range(n))
        if work <= max_work and bound <= target:
            choices.append((work, bound, ks))
    expected = min(choices) if choices else None
    assert (ans is None) == (expected is None)
    if ans:
        assert ans["declared_work_units"] == expected[0]
        assert F(ans["residual_bound"]) <= target
        assert sum(c*k for c, k in zip(costs, ans["iterations"])) == ans["declared_work_units"]
    checked.append({"index": index, "n": n, "work": None if ans is None else ans["declared_work_units"],
                    "bruteforce_feasible_paths": len(choices), "pass": True})
edge = {
    "floor_equal_positive_q": allocate([1], [1], [F(1, 2)], [F(1, 2)], [1], [128], F(1, 2)) is None,
    "floor_equal_zero_q": allocate([1], [1], [F(1, 2)], [0], [1], [2], F(1, 2))["iterations"] == [1],
    "initial_already_at_floor": allocate([1], [F(1, 2)], [F(1, 2)], [F(1, 2)], [1], [0], F(1, 2))["declared_work_units"] == 0,
}
for index in range(24):
    n = rng.randint(1, 7)
    lower = [[F(rng.randrange(3), 5) if j < i else F(0) for j in range(n)] for i in range(n)]
    b = [F(rng.randrange(-4, 5), 7) for _ in range(n)]
    m = [F(rng.randint(1, 5), 3) for _ in range(n)]
    r = [F(rng.randrange(6), 11) for _ in range(n)]
    d = []
    for i in range(n):
        d.append(m[i] * r[i] + sum(lower[i][j] * d[j] for j in range(i)))
    assert sum(wi * ri for wi, ri in zip(weights(lower, b, m), r)) == sum(abs(bi) * di for bi, di in zip(b, d))
assert all(edge.values())
out = {
    "schema": "vigilode.conditional-budget-independent-checks.v1", "seed": 20261024,
    "exhaustive_allocation_cases": checked, "edge_cases": edge,
    "causal_sensitivity_exact_equality_cases_passed": 24,
    "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
                      for p in [PKG / "src/conditional_budget.py", PKG / "src/test_conditional_budget.py"]},
    "scope": "Fresh bounded adversarial checks; existing seven unit tests not rerun; no ODE speed claim",
}
(PKG / "analysis/CONDITIONAL_BUDGET_REVIEW.json").write_text(json.dumps(out, indent=2) + "\n")
print(json.dumps({"allocation_cases_passed": 64, "edge": edge, "causal_equalities": 24}))
