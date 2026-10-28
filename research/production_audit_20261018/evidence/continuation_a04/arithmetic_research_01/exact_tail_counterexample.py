#!/usr/bin/env python3
"""Exact historical-tail falsifier; no Rust code, tests or fixtures are run."""
from fractions import Fraction as Q
from pathlib import Path
import hashlib
import json
import platform
import subprocess


ROOT = Path(__file__).resolve().parents[5]
SOURCE_SHA256 = {
    "crates/rodas5p-core/src/leja_action.rs": "25904301d40f11d6a5fee1d27aac7454ca02874a9e995634e7909af2e4ae97e8",
    "crates/rodas5p-core/src/directed.rs": "2adadb6a5d77073e0a7d3d60aedfbf37e124d15a586a880d7d5a8c8d5307a814",
    "crates/rodas5p-core/src/polynomial_action.rs": "656d494fd74783090676e1d3a18aaaa5dbd5df99c0541745979aeb6e26394a48",
    "docs/reviews/20261011_py05/LEJA_TOTAL_BOUND.md": "a6bfee9ed0283d5a0fc24629ab6e0495b38ba7236670d5f343a3a1884c438613",
    "docs/reviews/20261018_production_audit/SOLVER_AUDIT_KO.md": "f9b4d1cab5730da6e632268f50f9a2a6a2938a7b05bdd23d0b7ebb4a9d759fc7",
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def matvec(matrix, vector):
    return [sum((a * b for a, b in zip(row, vector)), Q(0)) for row in matrix]


def encode(value):
    if isinstance(value, Q):
        return str(value)
    if isinstance(value, (list, tuple)):
        return [encode(x) for x in value]
    if isinstance(value, dict):
        return {k: encode(v) for k, v in value.items()}
    return value


def main():
    actual = {path: digest(ROOT / path) for path in SOURCE_SHA256}
    assert actual == SOURCE_SHA256, "immutable proof/code source binding changed"
    matrix = [[Q(int(i == j + 1)) for j in range(4)] for i in range(4)]
    terms = [[Q(1), Q(0), Q(0), Q(0)]]
    for p in range(1, 5):
        terms.append([x / p for x in matvec(matrix, terms[-1])])
    assert terms[4] == [Q(0)] * 4
    n = 2
    tail = [sum((term[i] for term in terms[n + 1:]), Q(0)) for i in range(4)]
    norm_one = max(sum((abs(matrix[i][j]) for i in range(4)), Q(0)) for j in range(4))
    norm_inf = max(sum((abs(x) for x in row), Q(0)) for row in matrix)
    q_one = norm_one / (n + 1)
    q_inf = norm_inf / (n + 1)
    assert 0 <= q_one < 1 and 0 <= q_inf < 1
    factor_one = q_one / (1 - q_one)
    factor_inf = q_inf / (1 - q_inf)
    historical_vector_bound = [x * factor_one for x in terms[n]]
    uniform_bound = max(abs(x) for x in terms[n]) * factor_inf
    failed_components = [i for i, x in enumerate(tail) if x > historical_vector_bound[i]]
    checks = {
        "nilpotent_series_terminated_exactly": terms[4] == [Q(0)] * 4,
        "historical_norm_inequality_holds": sum(map(abs, tail), Q(0)) <= sum(map(abs, terms[n]), Q(0)) * factor_one,
        "historical_componentwise_claim_refuted": failed_components == [3],
        "correct_uniform_bound_encloses_all_components": all(0 <= x <= uniform_bound for x in tail),
        "exact_witness_tail": tail[3] == Q(1, 6),
        "false_historical_component_bound": historical_vector_bound[3] == 0,
        "correct_uniform_component_bound": uniform_bound == Q(1, 4),
    }
    assert all(checks.values()), checks
    here = Path(__file__).resolve().parent
    result = {
        "schema": "vigilode-a04-t2-exact-tail-research-v1",
        "decision": "HISTORICAL_COMPONENTWISE_T2_SUPERSEDED",
        "total_leja_authority": "EstimateOnly",
        "a04_total_status": "HOLD",
        "source_head_observed_at_execution": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "source_sha256": actual,
        "script_sha256": digest(Path(__file__)),
        "preregistration_sha256": digest(here / "PREREGISTRATION.md"),
        "python": platform.python_version(),
        "witness": {"matrix": matrix, "N": n, "terms_0_through_4": terms, "tail": tail, "norm_one": norm_one, "norm_inf": norm_inf, "q": q_inf, "historical_component_bounds": historical_vector_bound, "correct_uniform_component_bound": uniform_bound, "failed_historical_components": failed_components},
        "checks": checks,
        "scope": "Exact lemma falsification and corrected norm implication only; no native stopping-rule or total-action admission claim.",
    }
    print(json.dumps(encode(result), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
