#!/usr/bin/env python3
"""New exact proof examples only; no repository science campaign reruns."""
from fractions import Fraction as F
import json
from pathlib import Path


def ceil_log2_fraction(x):
    k = 0
    p = F(1)
    while p < x:
        p *= 2
        k += 1
    return k


A = [[F(0), F(0), F(0)],
     [F(1, 2), F(0), F(0)],
     [F(1, 4), F(1, 3), F(0)]]
G = [F(2), F(3), F(4)]
b = [F(1), F(2), F(1)]
delta = [F(1, 100), F(1, 200), F(1, 500)]
q = []
for i in range(3):
    q.append(G[i] * delta[i] + sum(A[i][j] * q[j] for j in range(i)))
v = [F(0)] * 3
for i in reversed(range(3)):
    v[i] = b[i] + sum(A[j][i] * v[j] for j in range(i + 1, 3))
w = [G[i] * v[i] for i in range(3)]
forward = sum(b[i] * q[i] for i in range(3))
adjoint = sum(w[i] * delta[i] for i in range(3))
assert forward == adjoint
assert w == [F(29, 6), F(7), F(4)]

a = [F(4096), F(1), F(1), F(1)]
B = F(1, 16)
targets = [B / (len(a) * ai) for ai in a]
k = [ceil_log2_fraction(1 / x) for x in targets]
ku = ceil_log2_fraction(sum(a) / B)
assert k == [18, 6, 6, 6]
assert ku == 17
achieved = sum(ai * F(1, 2**ki) for ai, ki in zip(a, k))
assert achieved == B
assert sum(k) == 36 and len(a) * ku == 68

# Algebraic family identities at fixed disjoint exact parameters, not timing.
family = []
for r, b_exp, L in [(1, 1, 1), (2, 4, 12), (4, 3, 64), (6, 2, 256)]:
    m = 2**r
    budget = F(1, 2**b_exp)
    total_weight = 2**L + m - 1
    k_uniform = ceil_log2_fraction(F(total_weight) / budget)
    work_uniform = m * k_uniform
    work_allocated = L + m * (r + b_exp)
    first = L + r + b_exp
    rest = r + b_exp
    actual_budget = 2**L * F(1, 2**first) + (m - 1) * F(1, 2**rest)
    assert actual_budget == budget
    assert work_uniform == m * (L + b_exp + 1)
    assert work_uniform - work_allocated == (m - 1) * L + m * (1 - r)
    family.append(dict(m=m, L=L, b=b_exp, uniform=work_uniform,
                       allocated=work_allocated,
                       saved=work_uniform - work_allocated))

result = {
    "schema": "vigilode.global-theory-exact-examples.v1",
    "status": "PASS_EXACT_FRACTION_EXAMPLES",
    "claim_ceiling": "Arithmetic proof examples; no native solver or walltime claim",
    "adjoint_identity": {
        "q": list(map(str, q)), "w": list(map(str, w)),
        "forward_bound": str(forward), "adjoint_bound": str(adjoint)},
    "allocation_example": {
        "weights": list(map(str, a)), "budget": str(B),
        "targets": list(map(str, targets)), "iterations": k,
        "uniform_iterations_per_solve": ku,
        "weighted_bound": str(achieved), "uniform_work": 68,
        "allocated_work": 36, "saved_iteration_units": 32},
    "complexity_family_checks": family,
    "wolfram_exact_crosscheck": {
        "tool": "mcp__codex_apps__wolfram_wolframlanguageevaluator",
        "input": "a={4096,1,1,1}; b=1/16; r=ConstantArray[1,4]; rho=b/(Length[a] a); k=-Log[2,rho]; ku=Ceiling[Log[2,Total[a]/b]]; {rho,k,Total[a 2^(-k)],ku,Length[a] ku,Total[k],Length[a] ku-Total[k]}",
        "output": "{{1/262144,1/64,1/64,1/64},{18,6,6,6},1/16,17,68,36,32}"
    }
}
Path(__file__).with_name("GLOBAL_EXACT_CHECKS.json").write_text(
    json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({"status": result["status"], "allocated_work": 36,
                  "uniform_work": 68, "family_cases": len(family)}))
