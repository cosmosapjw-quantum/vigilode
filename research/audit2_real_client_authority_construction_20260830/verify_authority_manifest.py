#!/usr/bin/env python3
"""Candidate-free exact verifier for the frozen Bateman authority manifest.

Only Python's standard library is used.  The reference proof compares exact
`Fraction.from_float` values against rational Taylor--Lagrange endpoint bounds;
no solver-under-test output is loaded or generated.
"""

from __future__ import annotations

import argparse
from fractions import Fraction
import hashlib
import json
import math
import pathlib
import struct
from typing import Any

DOMAIN = b"VIGILODE\0AUDIT2\0FROZEN_W\0"
MODEL_ID = "bateman_two_pair_v1"
SCALAR_FORMAT = "IEEE754_BINARY64"
MATRIX_LAYOUT = "ROW_MAJOR"
W_CONVENTION = "W=I-(h*gamma)*J; h_gamma rounded binary64 before row construction"
EXPECTED_CASES = ("nominal-h1e-3", "changed-w-h5e-4")
EXPECTED_SCENARIOS = (
    "same-live-context-reuse",
    "changed-w-invalidation",
    "nominal-independent-budget",
    "over-strict-budget-fallback",
    "late-preconditioner-failure",
    "terminal-rejection",
)


def float_from_bits(bits: int) -> float:
    return struct.unpack(">d", struct.pack(">Q", bits))[0]


def float_bits(value: float) -> int:
    return struct.unpack(">Q", struct.pack(">d", value))[0]


def taylor_exp_neg(x: Fraction, order: int) -> Fraction:
    if x < 0:
        raise ValueError("Taylor-Lagrange input must be nonnegative")
    term = Fraction(1)
    total = Fraction(1)
    for k in range(1, order + 1):
        term *= x / k
        total += -term if k % 2 else term
    return total


def exp_neg_bounds(x: Fraction) -> tuple[Fraction, Fraction]:
    # Taylor's theorem gives the sign of the Lagrange remainder for exp(-x):
    # S_41 <= exp(-x) <= S_40 for every x >= 0.  No monotone-term premise is
    # used; the nominal fast exponent is slightly greater than one in binary64.
    lower = taylor_exp_neg(x, 41)
    upper = taylor_exp_neg(x, 40)
    if lower > upper:
        raise ValueError("invalid Taylor-Lagrange bracket")
    return lower, upper


def stable_daughter_bounds(
    parent0: Fraction, daughter0: Fraction, rate: Fraction, h: Fraction
) -> tuple[tuple[Fraction, Fraction], tuple[Fraction, Fraction]]:
    exp_lower, exp_upper = exp_neg_bounds(rate * h)
    conserved = parent0 + daughter0
    return (
        (parent0 * exp_lower, parent0 * exp_upper),
        (conserved - parent0 * exp_upper, conserved - parent0 * exp_lower),
    )


def append_u32(payload: bytearray, value: int) -> None:
    payload.extend(struct.pack(">I", value))


def append_frame(payload: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    append_u32(payload, len(encoded))
    payload.extend(encoded)


def append_f64(payload: bytearray, value: float) -> None:
    payload.extend(struct.pack(">d", value))


def shifted_w_entries(h_gamma: float, rates: tuple[float, float]) -> tuple[float, ...]:
    fast = h_gamma * rates[0]
    slow = h_gamma * rates[1]
    return (
        1.0 + fast,
        0.0,
        0.0,
        0.0,
        -fast,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 + slow,
        0.0,
        0.0,
        0.0,
        -slow,
        1.0,
    )


def frozen_w_digest(
    case_id: str,
    t: float,
    h: float,
    gamma: float,
    rates: tuple[float, float],
    initial_state: tuple[float, ...],
) -> tuple[str, tuple[float, ...]]:
    h_gamma = h * gamma
    entries = shifted_w_entries(h_gamma, rates)
    payload = bytearray(DOMAIN)
    append_u32(payload, 1)
    for field in (MODEL_ID, SCALAR_FORMAT, MATRIX_LAYOUT, W_CONVENTION, case_id):
        append_frame(payload, field)
    append_u32(payload, 4)
    for value in (t, h, gamma):
        append_f64(payload, value)
    append_u32(payload, len(rates))
    for value in rates:
        append_f64(payload, value)
    append_u32(payload, len(initial_state))
    for value in initial_state:
        append_f64(payload, value)
    append_u32(payload, len(entries))
    for value in entries:
        append_f64(payload, value)
    return hashlib.sha256(payload).hexdigest(), entries


def require_finite_nonnegative(value: Any, field: str) -> float:
    if not isinstance(value, (int, float)):
        raise ValueError(f"{field} must be numeric")
    converted = float(value)
    if not math.isfinite(converted) or converted < 0.0:
        raise ValueError(f"{field} must be finite and nonnegative")
    return converted


def verify_manifest(path: pathlib.Path) -> dict[str, Any]:
    manifest = json.loads(path.read_text())
    if manifest.get("schema") != "vigilode-audit2-bateman-real-client-authority/v1":
        raise ValueError("authority schema mismatch")
    if manifest.get("client_id") != "bateman-two-timescale-parent-stable-daughter-v1":
        raise ValueError("client identity mismatch")
    if manifest.get("candidate_execution_during_construction") != "FORBIDDEN":
        raise ValueError("candidate execution was not forbidden during construction")
    if manifest.get("holdout_access") != "NOT_OPENED_OR_EXECUTED":
        raise ValueError("holdout boundary mismatch")
    if tuple(manifest.get("execution_scenarios", ())) != EXPECTED_SCENARIOS:
        raise ValueError("six-case execution plan mismatch")

    rates = tuple(float_from_bits(bits) for bits in manifest["rate_bits"])
    initial_state = tuple(float_from_bits(bits) for bits in manifest["initial_state_bits"])
    gamma = float_from_bits(manifest["coefficient_gamma_bits"])
    if rates != (1000.0, 1.0) or initial_state != (0.5, 0.0, 0.5, 0.0):
        raise ValueError("Bateman rate or initial-state bits mismatch")
    if float_bits(gamma) != 0x3FCB20C5235B5100:
        raise ValueError("coefficient gamma bits mismatch")

    cases = manifest.get("operator_cases", [])
    if tuple(case.get("case_id") for case in cases) != EXPECTED_CASES:
        raise ValueError("operator case order or identity mismatch")

    max_reference_l2_bound = 0.0
    fast_exponent_exceeds_one = False
    for case in cases:
        case_id = case["case_id"]
        t = float(case["t"])
        h = float(case["h"])
        if not math.isfinite(t) or not math.isfinite(h) or h <= 0.0:
            raise ValueError(f"invalid time interval for {case_id}")

        reference = tuple(float(value) for value in case["reference"]["state"])
        if len(reference) != 4 or not all(math.isfinite(value) for value in reference):
            raise ValueError(f"invalid reference state for {case_id}")
        uncertainty = require_finite_nonnegative(
            case["reference"]["uncertainty_l2"], "reference uncertainty"
        )
        if reference_document := case.get("reference"):
            uncertainty_treatment = reference_document.get("uncertainty_treatment")
        else:
            uncertainty_treatment = None
        if uncertainty_treatment != "DECLARED_UPPER_BOUND":
            raise ValueError("reference uncertainty is not an asserted upper bound")

        exact_h = Fraction.from_float(h)
        exact_rates = tuple(Fraction.from_float(rate) for rate in rates)
        exact_initial = tuple(Fraction.from_float(value) for value in initial_state)
        bounds = (
            *stable_daughter_bounds(
                exact_initial[0], exact_initial[1], exact_rates[0], exact_h
            ),
            *stable_daughter_bounds(
                exact_initial[2], exact_initial[3], exact_rates[1], exact_h
            ),
        )
        component_error_bounds = tuple(
            max(abs(Fraction.from_float(value) - lower), abs(Fraction.from_float(value) - upper))
            for value, (lower, upper) in zip(reference, bounds, strict=True)
        )
        l2_squared = sum(error * error for error in component_error_bounds)
        exact_uncertainty = Fraction.from_float(uncertainty)
        if l2_squared > exact_uncertainty * exact_uncertainty:
            raise ValueError(f"reference uncertainty is understated for {case_id}")
        max_reference_l2_bound = max(max_reference_l2_bound, math.sqrt(float(l2_squared)))
        if case_id == "nominal-h1e-3":
            fast_exponent_exceeds_one = exact_rates[0] * exact_h > 1

        expected_digest, entries = frozen_w_digest(
            case_id, t, h, gamma, rates, initial_state
        )
        if case["frozen_w_semantic"].get("schema") != "vigilode-audit2-bateman-frozen-w/v1":
            raise ValueError(f"frozen-W schema mismatch for {case_id}")
        if case["frozen_w_semantic"].get("sha256") != expected_digest:
            raise ValueError(f"frozen-W digest mismatch for {case_id}")

        expected_configuration = [
            float_bits(h),
            float_bits(gamma),
            float_bits(rates[0]),
            float_bits(rates[1]),
            float_bits(entries[0]),
            float_bits(entries[10]),
        ]
        identity = case["preconditioner_identity"]
        if identity.get("provider") != "analytic-bateman-jacobi-inverse-multiply":
            raise ValueError(f"preconditioner provider mismatch for {case_id}")
        if identity.get("revision") != 1:
            raise ValueError(f"preconditioner revision mismatch for {case_id}")
        if identity.get("configuration_bits") != expected_configuration:
            raise ValueError(f"preconditioner configuration mismatch for {case_id}")
        inverse = (1.0 / entries[0], 1.0, 1.0 / entries[10], 1.0)
        expected_inverse_bits = [float_bits(value) for value in inverse]
        if identity.get("expected_inverse_diagonal_bits") != expected_inverse_bits:
            raise ValueError(f"preconditioner inverse diagonal mismatch for {case_id}")
        if all(bits == float_bits(1.0) for bits in expected_inverse_bits):
            raise ValueError(f"preconditioner is the identity for {case_id}")

        budget = case["budget"]
        budget_values = [
            require_finite_nonnegative(budget[field], field)
            for field in (
                "output_atol_l2",
                "output_rtol",
                "max_embedded_l2",
                "max_original_target_residual_l2",
                "max_original_target_contraction",
            )
        ]
        if budget_values[0] <= uncertainty:
            raise ValueError(f"output budget does not dominate reference authority for {case_id}")
        if budget_values[-1] >= 1.0:
            raise ValueError(f"contraction budget is not contractive for {case_id}")

    return {
        "status": "AUTHORITY_CONSTRUCTION_VERIFIED",
        "candidate_executions": 0,
        "verified_operator_cases": len(cases),
        "execution_scenarios": len(EXPECTED_SCENARIOS),
        "max_reference_l2_bound": max_reference_l2_bound,
        "declared_reference_l2_uncertainty": 1.0e-15,
        "fast_exponent_exceeds_one": fast_exponent_exceeds_one,
        "holdout_access": manifest["holdout_access"],
    }


def sha256_file(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_receipt(
    manifest_path: pathlib.Path, receipt_path: pathlib.Path
) -> dict[str, Any]:
    """Bind the candidate-free exact proof summary to exact checked-in bytes."""
    summary = verify_manifest(manifest_path)
    receipt = json.loads(receipt_path.read_text())
    if receipt.get("schema") != "vigilode-audit2-bateman-authority-verification-receipt/v1":
        raise ValueError("authority verification receipt schema mismatch")
    if receipt.get("authority_manifest_sha256") != sha256_file(manifest_path):
        raise ValueError("authority verification receipt manifest hash mismatch")
    if receipt.get("exact_verifier_sha256") != sha256_file(pathlib.Path(__file__)):
        raise ValueError("authority verification receipt verifier hash mismatch")
    for field in (
        "status",
        "verified_operator_cases",
        "execution_scenarios",
        "candidate_executions",
        "declared_reference_l2_uncertainty",
        "max_reference_l2_bound",
        "fast_exponent_exceeds_one",
        "holdout_access",
    ):
        if receipt.get(field) != summary[field]:
            raise ValueError(f"authority verification receipt {field} mismatch")
    if receipt.get("uncertainty_treatment") != "DECLARED_UPPER_BOUND":
        raise ValueError("authority verification receipt uncertainty treatment mismatch")
    if receipt.get("output_admission_rule") != (
        "E_ref + u <= B_abs + B_rel * norm2(reference)"
    ):
        raise ValueError("authority verification receipt output admission rule mismatch")
    if receipt.get("local_six_case_status") != "NOT_RUN_DURING_AUTHORITY_CONSTRUCTION":
        raise ValueError("authority verification receipt candidate boundary mismatch")
    return {
        **summary,
        "local_six_case_status": receipt["local_six_case_status"],
        "receipt_sha256": sha256_file(receipt_path),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "manifest",
        nargs="?",
        type=pathlib.Path,
        default=pathlib.Path(__file__).with_name("authority_manifest.json"),
    )
    parser.add_argument(
        "--receipt",
        type=pathlib.Path,
        default=pathlib.Path(__file__).with_name("evidence")
        / "AUTHORITY_VERIFICATION_RECEIPT.json",
    )
    args = parser.parse_args()
    print(json.dumps(verify_receipt(args.manifest, args.receipt), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
