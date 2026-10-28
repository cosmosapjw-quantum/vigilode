#!/usr/bin/env python3
"""Independent exact original-equation checks for the bounded RVJ native runs.

Only the standard library is used. This program is executed after the parent
publication/GO gate. It does not import a native implementation, its generated
coefficients, or either attachment's certificate implementation.

Usage:
  python3 verify_native.py --closure closure.json --trace trace.json \
      --output exact_oracle_receipt.json

The output is create-only. Failed physical-cap rows are retained as outcomes,
not confused with a failed verification of an honest enclosure. A coefficient,
semantic, domain, or falsely claimed acceptance failure exits nonzero.
"""

from __future__ import annotations

import argparse
import copy
from fractions import Fraction as Q
import hashlib
import json
from math import comb, isfinite
from pathlib import Path
import struct
import sys
import time


ZERO = Q(0)
ONE = Q(1)
CAP = Q(1, 100_000_000)
CLOSURE_DIMS = (4, 8, 16)
CLOSURE_STEPS = (16, 32, 64, 128)
TRACE_STEPS = (16, 32, 64, 128, 256)
CLOSURE_ARMS = (
    "historical_power_legacy",
    "direct_bernstein_legacy",
    "direct_bernstein_tight",
)
TRACE_ROUTES = ("direct_moments", "shared_polynomial")


class VerificationError(Exception):
    """An original-equation or protocol check failed."""


class Checks:
    def __init__(self):
        self.count = 0

    def require(self, condition, message):
        self.count += 1
        if not condition:
            raise VerificationError(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def finite_number(value, name):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise VerificationError(f"{name}: expected finite JSON number")
    value = float(value)
    if not isfinite(value):
        raise VerificationError(f"{name}: nonfinite value")
    return value


def fraction_number(value, name):
    return Q.from_float(finite_number(value, name))


def bits_fraction(word, name):
    if isinstance(word, bool) or not isinstance(word, (str, int)):
        raise VerificationError(f"{name}: expected u64 integer or decimal string")
    if isinstance(word, str) and (not word or not word.isdecimal()):
        raise VerificationError(f"{name}: invalid decimal bit word")
    integer = int(word)
    if not 0 <= integer < 1 << 64:
        raise VerificationError(f"{name}: u64 outside range")
    value = struct.unpack(">d", integer.to_bytes(8, "big"))[0]
    if not isfinite(value):
        raise VerificationError(f"{name}: nonfinite binary64 bit word")
    return Q.from_float(value)


def bound_value(row, key, checks, context):
    value = fraction_number(row[key], f"{context}.{key}")
    if key + "_bits" in row:
        exact = bits_fraction(row[key + "_bits"], f"{context}.{key}_bits")
        checks.require(value == exact, f"{context}.{key}: JSON/bits disagree")
        value = exact
    checks.require(value >= 0, f"{context}.{key}: negative bound")
    return value


def interval(value, context):
    if isinstance(value, dict) and "lo" in value and "hi" in value:
        lo, hi = value["lo"], value["hi"]
    elif isinstance(value, list) and len(value) == 2:
        lo, hi = value
    else:
        raise VerificationError(f"{context}: expected [lo,hi] interval")
    lo, hi = fraction_number(lo, context + ".lo"), fraction_number(hi, context + ".hi")
    if lo > hi:
        raise VerificationError(f"{context}: reversed interval")
    return lo, hi


def enclosed(exact, box, checks, context):
    lo, hi = interval(box, context)
    checks.require(lo <= exact <= hi, f"{context}: exact original coefficient not enclosed")


def pad(poly, length):
    if len(poly) > length and any(poly[length:]):
        raise VerificationError("unexpected original polynomial degree")
    return list(poly[:length]) + [ZERO] * max(0, length - len(poly))


def add(*polynomials):
    result = [ZERO] * max((len(p) for p in polynomials), default=1)
    for poly in polynomials:
        for k, value in enumerate(poly):
            result[k] += value
    return result


def scale(poly, scalar):
    return [scalar * coefficient for coefficient in poly]


def multiply(left, right):
    result = [ZERO] * (len(left) + len(right) - 1)
    for i, a in enumerate(left):
        for j, b in enumerate(right):
            result[i + j] += a * b
    return result


def power(poly, exponent):
    result = [ONE]
    for _ in range(exponent):
        result = multiply(result, poly)
    return result


def derivative(poly):
    return [k * poly[k] for k in range(1, len(poly))] or [ZERO]


def bernstein(poly, degree=None):
    """Exact power-to-Bernstein conversion, independent of native direct formulas."""
    degree = len(poly) - 1 if degree is None else degree
    coefficients = pad(poly, degree + 1)
    return [
        sum((Q(comb(i, k), comb(degree, k)) * coefficients[k]
             for k in range(i + 1)), ZERO)
        for i in range(degree + 1)
    ]


def magnitude(values):
    return max((abs(value) for value in values), default=ZERO)


def hermite(a, b, fa, fb, h):
    return [a, h * fa, 3 * (b - a) - h * (2 * fa + fb),
            2 * (a - b) + h * (fa + fb)]


def cyclic_original_slope(y):
    """Independent n-term cyclic inverse formula, not native backward recurrence."""
    n = len(y) // 2
    x, q = y[:n], y[n:]
    f = [-x[i] + (q[i] + q[(i + 1) % n]) / 16
         + Q(1, 32) + Q(i % 3, 256) for i in range(n)]
    b = [q[(i + 1) % n] / 2 for i in range(n)]
    product = ONE
    for coefficient in b:
        product *= -coefficient
    denominator = ONE - product
    if denominator == 0:
        raise VerificationError("original cyclic tangent denominator is zero")
    tangent = []
    for i in range(n):
        numerator, coefficient = ZERO, ONE
        for k in range(n):
            numerator += coefficient * f[(i + k) % n]
            coefficient *= -b[(i + k) % n]
        tangent.append(numerator / denominator)
    # This identity uses the original differentiated equations directly.
    if any(tangent[i] + b[i] * tangent[(i + 1) % n] != f[i] for i in range(n)):
        raise VerificationError("independent cyclic inverse does not solve original tangent equation")
    return f + tangent


def cyclic_exact_slab(y0, y1, h, f0, f1, checks, context):
    n = len(y0) // 2
    paths = [hermite(a, b, fa, fb, h) for a, b, fa, fb in zip(y0, y1, f0, f1)]
    path_coefficients = [bernstein(poly, 3) for poly in paths]
    for i, coefficients in enumerate(path_coefficients):
        limit = Q(1, 8) if i < n else Q(1, 4)
        checks.require(magnitude(coefficients) <= limit, f"{context}: exact dense path outside X/Q")
    slow, algebraic = [], []
    for i in range(n):
        j = (i + 1) % n
        # Original G_i and slow F_i, in the POWER basis throughout.
        original_g = add(paths[n + i], scale(multiply(paths[n + j], paths[n + j]), Q(1, 4)),
                         scale(paths[i], -ONE))
        original_f = add(scale(paths[i], -ONE),
                         scale(add(paths[n + i], paths[n + j]), Q(1, 16)),
                         [Q(1, 32) + Q(i % 3, 256)])
        original_r = add(scale(derivative(paths[i]), ONE / h), scale(original_f, -ONE))
        slow.append(bernstein(original_r, 3))
        algebraic.append(bernstein(original_g, 6))
        checks.require(slow[-1][0] == 0 == slow[-1][3], f"{context}: exact-field endpoint defect identity failed")
        checks.require(algebraic[-1][0] == algebraic[-1][1], f"{context}: left G derivative identity failed")
        checks.require(algebraic[-1][5] == algebraic[-1][6], f"{context}: right G derivative identity failed")
    legacy_df = max(map(magnitude, slow))
    legacy_dg = max(map(magnitude, algebraic))
    direct_df = Q(3, 4) * max(max(abs(c[1]), abs(c[2])) for c in slow)
    direct_dg = ZERO
    for coefficients in algebraic:
        g = max(abs(coefficients[0]), abs(coefficients[6]))
        middle = max(map(abs, coefficients[2:5]))
        direct_dg = max(direct_dg, g, (7 * g + 25 * middle) / 32)
    return {
        "path_bernstein": path_coefficients,
        "slow_bernstein": slow,
        "algebraic_bernstein": algebraic,
        "legacy_df": legacy_df, "legacy_dg": legacy_dg,
        "direct_df": direct_df, "direct_dg": direct_dg,
    }


def compare_coefficient_arrays(record, exact, checks, context):
    for key in ("path_bernstein", "slow_bernstein", "algebraic_bernstein"):
        observed = record[key]
        expected = exact[key]
        checks.require(len(observed) == len(expected), f"{context}.{key}: component count")
        for i, (row, reference) in enumerate(zip(observed, expected)):
            checks.require(len(row) == len(reference), f"{context}.{key}[{i}]: degree")
            for k, (box, coefficient) in enumerate(zip(row, reference)):
                enclosed(coefficient, box, checks, f"{context}.{key}[{i}][{k}]")


def cyclic_negative_control(record, exact):
    """Mutate an actually exported coefficient; demonstrate oracle rejection."""
    corrupt = copy.deepcopy(record)
    original = exact["slow_bernstein"][0][0]
    if original != 0:
        raise VerificationError("negative-control setup expected exact endpoint zero")
    corrupt["slow_bernstein"][0][0] = [1.0, 1.0]
    try:
        compare_coefficient_arrays(corrupt, exact, Checks(), "corrupted-export")
    except VerificationError:
        return {"kind": "changed_native_coefficient_interval", "rejected": True,
                "change": "first original slow endpoint-zero interval replaced by [1,1]"}
    raise VerificationError("corrupted coefficient export falsely accepted")


def scalar_radius(record, arm_name, h, previous, checks, context):
    e0 = bits_fraction(record["e0_bits"], context + ".e0_bits")
    if "e0" in record:
        checks.require(fraction_number(record["e0"], context + ".e0") == e0,
                       f"{context}: incoming radius JSON/bits disagree")
    checks.require(e0 == previous, f"{context}: incoming radius not previous returned endpoint radius")
    checks.require(all(key + "_bits" in record for key in ("df", "dg", "ew", "ee", "qw")),
                   f"{context}: required scalar bound bit words absent")
    values = {key: bound_value(record, key, checks, context) for key in ("df", "dg", "ew", "ee", "qw")}
    delta = values["df"] + values["dg"] / 7
    z = Q(6, 7) * h
    if arm_name == "direct_bernstein_tight":
        u1 = (e0 + h * delta) / (1 + z)
        u2 = ((1 - z / 2) * e0 + h * delta) / (1 + z / 2)
        endpoint = max(u1, u2)
        whole = max(e0, endpoint)
    else:
        endpoint = e0 / (1 + z) + h * delta
        whole = e0 + h * delta
    checks.require(values["ee"] >= endpoint, f"{context}: endpoint recurrence underestimates exact rational formula")
    checks.require(values["ew"] >= whole, f"{context}: whole-slab recurrence underestimates exact rational formula")
    # Use the actually returned whole bound: this also checks the composed implementation.
    checks.require(values["qw"] >= Q(8, 7) * (values["ew"] + values["dg"]),
                   f"{context}: root transfer underestimates exact rational formula")
    return values


def timing_rows(rows, checks, context):
    checks.require(len(rows) == 7, f"{context}: expected preregistered seven raw repeats")
    for index, row in enumerate(rows):
        for key in ("candidate_seconds", "preparation_seconds", "certificate_seconds", "total_seconds"):
            value = finite_number(row[key], f"{context}[{index}].{key}")
            checks.require(value >= 0, f"{context}: negative timer")
        checks.require(row["total_seconds"] >= row["certificate_seconds"],
                       f"{context}: total excludes certificate timer")


def closure_work(counts, n, steps, name, checks, context):
    legacy = name == "historical_power_legacy"
    expected = {
        "candidate_rhs_evaluations": 4 * steps,
        "candidate_cyclic_solves": 4 * steps,
        "interval_slope_evaluations": 2 * steps if legacy else steps + 1,
        "interval_cyclic_solves": 2 * steps if legacy else steps + 1,
        "endpoint_original_residual_evaluations": 0 if legacy else steps + 1,
        "endpoint_reuses": 0 if legacy else steps - 1,
        "slab_attempts": steps,
        "slab_certificates": steps,
        "power_to_bernstein_transforms": 4 * n * steps if legacy else 0,
        "polynomial_convolution_products": 16 * n * steps if legacy else 0,
        "direct_slow_components": 0 if legacy else n * steps,
        "direct_algebraic_components": 0 if legacy else n * steps,
        "jacobian_callbacks": 0, "jvp_callbacks": 0, "ad_callbacks": 0,
        "newton_updates": 0, "state_matrix_factorizations": 0,
    }
    for key, value in expected.items():
        checks.require(isinstance(counts[key], int) and not isinstance(counts[key], bool)
                       and counts[key] == value, f"{context}.counts.{key}: work differs from declared finite algorithm")


def verify_closure(data, checks):
    checks.require(data.get("schema") == "vigilode.rvj_closure_campaign.v1", "closure schema mismatch")
    checks.require(data.get("model_id") == "rvj-cyclic-quarter-square-v1"
                   and data.get("branch_id") == "cyclic-small-real-zero", "closure model/branch identity mismatch")
    checks.require(data.get("path_definition") == "exact-original-slope-hermite-v1", "closure path definition mismatch")
    checks.require(data.get("physical_norm") == "all-state-infinity"
                   and data.get("physical_time_interval") == [0, 1], "closure physical norm/horizon mismatch")
    checks.require(fraction_number(data["cap"], "closure.cap") == Q.from_float(1e-8), "closure cap mismatch")
    checks.require(data.get("cap_exact") == str(CAP), "closure exact rational cap metadata mismatch")
    checks.require(data.get("warmups") == 1 and data.get("repeats") == 7, "closure preregistered timing repetition metadata mismatch")
    configurations = data["configurations"]
    expected_configurations = {(n, steps) for n in CLOSURE_DIMS for steps in CLOSURE_STEPS}
    observed_configurations = [(row["n"], row["steps"]) for row in configurations]
    checks.require(len(observed_configurations) == len(expected_configurations)
                   and set(observed_configurations) == expected_configurations, "closure configuration coverage or duplicates")
    report = {"schema": "exact-cyclic-original-equation-check-v1", "configurations": [],
              "distinct_paths_checked": 0, "native_slab_records_checked": 0,
              "coefficient_intervals_checked": 0, "negative_controls": [],
              "physical_cap_exact": str(CAP), "new_native_runs_executed_by_oracle": 0}
    for config in configurations:
        n, steps = config["n"], config["steps"]
        context = f"cyclic[n={n},steps={steps}]"
        print(json.dumps({"oracle_progress": context}), flush=True)
        endpoint_rows = config["endpoint_bits"]
        checks.require(len(endpoint_rows) == steps + 1, f"{context}: endpoint count")
        endpoints = []
        for k, row in enumerate(endpoint_rows):
            checks.require(len(row) == 2 * n, f"{context}: endpoint dimension")
            endpoints.append([bits_fraction(value, f"{context}.endpoint[{k}]") for value in row])
        checks.require(all(value == 0 for value in endpoints[0]), f"{context}: original exact initial state changed")
        h = Q(1, steps)
        slopes = [cyclic_original_slope(row) for row in endpoints]
        exact_slabs = [cyclic_exact_slab(endpoints[k], endpoints[k + 1], h, slopes[k], slopes[k + 1], checks,
                                         f"{context}.slab[{k}]") for k in range(steps)]
        report["distinct_paths_checked"] += steps
        arms = config["arms"]
        names = [arm["name"] for arm in arms]
        checks.require(len(names) == 3 and set(names) == set(CLOSURE_ARMS), f"{context}: arm coverage or duplicates")
        config_result = {"n": n, "steps": steps, "arms": []}
        for arm in arms:
            name = arm["name"]
            slabs = arm["slabs"]
            checks.require(len(slabs) == steps, f"{context}.{name}: slab count")
            timing_rows(arm["raw_timings"], checks, f"{context}.{name}.raw_timings")
            closure_work(arm["counts"], n, steps, name, checks, f"{context}.{name}")
            previous = ZERO
            maximum = {"ew": ZERO, "qw": ZERO, "df": ZERO, "dg": ZERO}
            for k, (record, exact) in enumerate(zip(slabs, exact_slabs)):
                slab_context = f"{context}.{name}.slab[{k}]"
                checks.require(record["index"] == k, f"{slab_context}: slab index")
                checks.require(bits_fraction(record["h_bits"], slab_context + ".h_bits") == h,
                               f"{slab_context}: represented clock mismatch")
                for key, expected_time in (("t0", k * h), ("t1", (k + 1) * h)):
                    checks.require(bits_fraction(record[key + "_bits"], slab_context + "." + key + "_bits") == expected_time,
                                   f"{slab_context}: {key} bit label differs from original clock")
                    checks.require(fraction_number(record[key], slab_context + "." + key) == expected_time,
                                   f"{slab_context}: {key} JSON label differs from original clock")
                if "h" in record:
                    checks.require(fraction_number(record["h"], slab_context + ".h") == h,
                                   f"{slab_context}: clock JSON/bits disagree")
                compare_coefficient_arrays(record, exact, checks, slab_context)
                if not report["negative_controls"]:
                    report["negative_controls"].append(cyclic_negative_control(record, exact))
                values = scalar_radius(record, name, h, previous, checks, slab_context)
                prefix = "legacy" if name == "historical_power_legacy" else "direct"
                checks.require(values["df"] >= exact[prefix + "_df"], f"{slab_context}: original slow residual bound too small")
                checks.require(values["dg"] >= exact[prefix + "_dg"], f"{slab_context}: original algebraic residual bound too small")
                if "physical_budget_met" in record:
                    checks.require(isinstance(record["physical_budget_met"], bool)
                                   and record["physical_budget_met"] == (max(values["ew"], values["qw"]) <= CAP),
                                   f"{slab_context}: slab physical-cap claim differs from exact bound")
                # Validate the declared native whole-path domain, not only exact coefficient membership.
                for component, row in enumerate(record["path_bernstein"]):
                    limit = Q(1, 8) if component < n else Q(1, 4)
                    for box in row:
                        lo, hi = interval(box, slab_context + ".path_domain")
                        checks.require(-limit <= lo <= hi <= limit, f"{slab_context}: native path domain enclosure leaves X/Q")
                previous = values["ee"]
                for key in maximum:
                    maximum[key] = max(maximum[key], values[key])
                report["native_slab_records_checked"] += 1
                report["coefficient_intervals_checked"] += 19 * n
            for key, value in maximum.items():
                checks.require(fraction_number(arm["max_" + key], f"{context}.{name}.max_{key}") == value,
                               f"{context}.{name}: summary maximum {key} differs from actual slabs")
            passes = max(maximum["ew"], maximum["qw"]) <= CAP
            checks.require(isinstance(arm["physical_budget_met"], bool), f"{context}.{name}: acceptance is not boolean")
            checks.require(arm["physical_budget_met"] == passes, f"{context}.{name}: physical-cap claim differs from exact represented bounds")
            config_result["arms"].append({
                "name": name, "status": "PASS_SCOPED_ENCLOSURE_CHECKS",
                "physical_budget_met": passes, "failed_cap_is_retained_negative_evidence": not passes,
                "maxima_exact_represented_native_bounds": {key: str(value) for key, value in maximum.items()},
                "endpoint_error_exact_represented_native_bound": str(previous),
                "slabs_checked": steps,
            })
        report["configurations"].append(config_result)
    checks.require(report["distinct_paths_checked"] == 720, "closure all-slab coverage should be720")
    checks.require(report["native_slab_records_checked"] == 2160, "closure three-arm all-slab coverage should be2160")
    checks.require(report["coefficient_intervals_checked"] == 383040, "closure coefficient interval coverage should be383040")
    report["claim_ceiling"] = "original bounded cyclic model, exact-field Hermite path and exported native interval bounds only"
    return report


def matrix_multiply(left, right):
    n = len(left)
    return [[sum((left[i][k] * right[k][j] for k in range(n)), ZERO)
             for j in range(n)] for i in range(n)]


def companion_moments(a, b, maximum=6):
    companion = [[ZERO, ZERO, ZERO, b - a * a], [ONE, ZERO, ZERO, -ONE],
                 [ZERO, ONE, ZERO, 2 * a], [ZERO, ZERO, ONE, ZERO]]
    current = [[Q(i == j) for j in range(4)] for i in range(4)]
    out = []
    for _ in range(maximum + 1):
        out.append(sum((current[i][i] for i in range(4)), ZERO))
        current = matrix_multiply(current, companion)
    return out


def original_trace_field(a, b):
    # Original F6 combination, intentionally not the shared T12 N circuit.
    s2 = 4 * a
    s4 = 4 * a * a + 4 * b
    s6 = 4 * a * a * a + 12 * a * b + 3
    return (-a + s2 / 32 + s4 / 128 + s6 / 512,
            -b + s2 / 64 + s4 / 64 + s6 / 256)


def exact_reference(t):
    return Q(3, 4) + t / 8 + t ** 5 / 64


def original_trace_rhs(t, state):
    p = exact_reference(t)
    dp = Q(1, 8) + 5 * t ** 4 / 64
    value = original_trace_field(*state)
    control = original_trace_field(p, p)
    return [dp + value[i] - control[i] for i in range(2)]


def original_trace_field_polynomials(a, b):
    # Power-basis substitution in the original moment combination.
    s2 = scale(a, 4)
    s4 = add(scale(multiply(a, a), 4), scale(b, 4))
    s6 = add(scale(multiply(multiply(a, a), a), 4), scale(multiply(a, b), 12), [Q(3)])
    return [add(scale(a, -ONE), scale(s2, Q(1, 32)), scale(s4, Q(1, 128)), scale(s6, Q(1, 512))),
            add(scale(b, -ONE), scale(s2, Q(1, 64)), scale(s4, Q(1, 64)), scale(s6, Q(1, 256)))]


def ceil_dyadic(value, bits=112):
    """An exact upward quantization, never a fitted safety multiplier."""
    scaled = value * (1 << bits)
    numerator = -((-scaled.numerator) // scaled.denominator)
    return Q(numerator, 1 << bits)


def trace_path_certificate(states, checks, context):
    lam = Q(39, 64)
    previous = max(abs(value - exact_reference(states[0][0])) for value in states[0][1])
    maximum_whole, maximum_direct, maximum_defect = previous, ZERO, ZERO
    minimum_margin = ONE
    records = []
    slopes = [original_trace_rhs(t, y) for t, y in states]
    for index, ((t0, y0), (t1, y1)) in enumerate(zip(states, states[1:])):
        h = t1 - t0
        checks.require(h > 0, f"{context}: noncausal trace slab")
        paths = [hermite(y0[i], y1[i], slopes[index][i], slopes[index + 1][i], h) for i in range(2)]
        time_poly = [t0, h]
        reference = add([Q(3, 4)], scale(time_poly, Q(1, 8)), scale(power(time_poly, 5), Q(1, 64)))
        reference_derivative = add([Q(1, 8)], scale(power(time_poly, 4), Q(5, 64)))
        field = original_trace_field_polynomials(*paths)
        control = original_trace_field_polynomials(reference, reference)
        rhs = [add(reference_derivative, field[i], scale(control[i], -ONE)) for i in range(2)]
        residuals = [add(scale(derivative(paths[i]), ONE / h), scale(rhs[i], -ONE)) for i in range(2)]
        delta = max(magnitude(bernstein(poly)) for poly in residuals)
        whole = previous + h * delta
        endpoint = ceil_dyadic(previous / (1 + lam * h) + h * delta)
        path_boxes = [(min(c), max(c)) for c in (bernstein(poly, 3) for poly in paths)]
        margin = min(min(lo - Q(1, 2), ONE - hi) for lo, hi in path_boxes)
        checks.require(margin > whole, f"{context}.slab[{index}]: complete error tube leaves contraction domain")
        direct = max(magnitude(bernstein(add(poly, scale(reference, -ONE)))) for poly in paths)
        maximum_whole = max(maximum_whole, whole)
        maximum_direct = max(maximum_direct, direct)
        maximum_defect = max(maximum_defect, delta)
        minimum_margin = min(minimum_margin, margin)
        records.append({"index": index, "t0": str(t0), "t1": str(t1),
                        "residual_upper": str(delta), "incoming_error_upper": str(previous),
                        "whole_error_upper": str(whole), "endpoint_error_upper": str(endpoint),
                        "direct_quintic_path_error_upper": str(direct), "domain_margin": str(margin)})
        previous = endpoint
    return {"arithmetic": "external exact Fraction original-field power/Bernstein certificate",
            "native_interval_whole_path_certificate": False,
            "lambda_exact": str(lam), "propagation": "E/(1+lambda*h)+h*delta; whole E+h*delta",
            "endpoint_upward_quantization_bits": 112, "physical_cap_exact": str(CAP),
            "physical_budget_met": maximum_whole <= CAP,
            "whole_path_error_upper": str(maximum_whole), "endpoint_error_upper": str(previous),
            "direct_quintic_path_error_upper": str(maximum_direct), "max_residual_upper": str(maximum_defect),
            "min_domain_margin": str(minimum_margin), "slabs_checked": len(records), "slab_records": records}


def trace_no_root_work(work, checks, context):
    for key in ("root_enumerations", "jacobian_evaluations", "jvp_evaluations", "ad_evaluations", "newton_updates"):
        checks.require(work[key] == 0, f"{context}.{key}: derivative/root work outside admitted trace route")


def verify_trace(data, checks):
    checks.require(data.get("schema_version") == 1, "trace schema mismatch")
    checks.require(data.get("task_id") == "RVJ-TRACE-20261028", "trace task mismatch")
    checks.require(data.get("target_id") == "rvj-quartic-all-complex-multiplicity-v1", "trace target identity mismatch")
    checks.require(data.get("semantics") == "all_complex_roots_algebraic_multiplicity", "trace observable semantics changed")
    checks.require(data.get("all_requested_trajectories_completed") is True, "trace requested native trajectory/point-field run rejected")
    cases = data["interval_cases"]
    coordinates = []
    point_results = []
    for row in cases:
        context = "trace.point[" + row["name"] + "]"
        a, b = bits_fraction(row["a_bits"], context + ".a_bits"), bits_fraction(row["b_bits"], context + ".b_bits")
        coordinates.append((a, b))
        sums = companion_moments(a, b)
        checks.require(len(row["moments"]) == 7, f"{context}: moment coverage")
        for k, value in enumerate(sums):
            enclosed(value, row["moments"][k], checks, f"{context}.moment[{k}]")
        original = (-a + sums[2] / 32 + sums[4] / 128 + sums[6] / 512,
                    -b + sums[2] / 64 + sums[4] / 64 + sums[6] / 256)
        checks.require(original == original_trace_field(a, b), f"{context}: independent companion/moment field mismatch")
        for key in ("field", "field_direct"):
            checks.require(len(row[key]) == 2, f"{context}.{key}: field dimension")
            for i in range(2):
                enclosed(original[i], row[key][i], checks, f"{context}.{key}[{i}]")
        # q2=a-q1^2 in the original quotient; use companion sums, not native invariant shortcuts.
        invariant_one = sums[2] + a * sums[0] - sums[2]
        invariant_two = a * sums[1] - sums[3]
        enclosed(invariant_one, row["invariant_q1_squared_plus_q2"], checks, context + ".invariant1")
        enclosed(invariant_two, row["invariant_q1_q2"], checks, context + ".invariant2")
        trace_no_root_work(row["work"], checks, context + ".work")
        point_results.append({"name": row["name"], "a": str(a), "b": str(b),
                              "exact_companion_moments": list(map(str, sums)), "status": "PASS_SCOPED"})
    checks.require((Q(3, 4), Q(3, 4)) in coordinates, "trace triple-root control absent")
    checks.require((Q(-1, 4), Q(-1, 4)) in coordinates, "trace complex-pair control absent")
    checks.require((Q(1, 2), Q(3, 4)) in coordinates, "trace regular point control absent")
    semantics = data["semantics_checks"]
    rejected_queries = set()
    for row in semantics:
        checks.require(row["expected"] in ("accepted", "rejected"), "trace semantics expectation invalid")
        checks.require(row["actual"] == row["expected"], "trace native semantic refusal differs from expected")
        work = row["work"]
        accepted = row["actual"] == "accepted"
        checks.require(work["requests_attempted"] == 1
                       and work["requests_completed"] == int(accepted)
                       and work["requests_rejected"] == int(not accepted)
                       and work["semantics_rejections"] == int(not accepted), "trace semantic refusal work ledger mismatch")
        trace_no_root_work(work, checks, "trace.semantics.work")
        if row["actual"] == "rejected":
            rejected_queries.add(row["query"])
    checks.require({"selected_branch", "real_only"} <= rejected_queries, "trace selected-branch/real-only refusal controls absent")
    negative_checks = Checks()
    try:
        enclosed(Q(4), [2.0, 2.0], negative_checks, "wrong-distinct-root-multiplicity")
    except VerificationError:
        multiplicity_rejected = True
    else:
        raise VerificationError("distinct-root count substituted for algebraic multiplicity falsely accepted")
    runs = data["runs"]
    expected = {(route, steps) for route in TRACE_ROUTES for steps in TRACE_STEPS}
    observed = [(row["route"], row["slabs"]) for row in runs]
    checks.require(len(observed) == len(expected) and set(observed) == expected, "trace trajectory coverage or duplicates")
    results = []
    for run in runs:
        route, steps = run["route"], run["slabs"]
        context = f"trace[{route},steps={steps}]"
        print(json.dumps({"oracle_progress": context}), flush=True)
        checks.require(run.get("status") == "completed", f"{context}: native trajectory failed")
        checks.require(run.get("whole_path_certified_natively") is False,
                       f"{context}: candidate is not a native interval whole-path certificate")
        checks.require(run["rhs_calls"] == 4 * steps, f"{context}: RK4 work ledger mismatch")
        work = run["work"]
        expected_work = {"requests_attempted": 8 * steps, "requests_completed": 8 * steps,
                         "requests_rejected": 0, "semantics_rejections": 0,
                         "moment_batches": (8 if route == "direct_moments" else 4) * steps,
                         "shared_field_batches": 0 if route == "direct_moments" else 4 * steps,
                         "scalar_multiplications": (104 if route == "direct_moments" else 84) * steps,
                         "scalar_additions": (72 if route == "direct_moments" else 64) * steps}
        for key, value in expected_work.items():
            checks.require(work[key] == value, f"{context}.work.{key}: stated finite field/candidate algorithm differs")
        trace_no_root_work(work, checks, context + ".work")
        node_fields = run["node_field_enclosure"]
        checks.require(node_fields["status"] == "completed", f"{context}: native point-field interval phase rejected")
        trace_no_root_work(node_fields["work"], checks, context + ".node_field.work")
        nodes = run["states"]
        checks.require(len(nodes) == steps + 1, f"{context}: endpoint count")
        states = []
        for index, row in enumerate(nodes):
            t = bits_fraction(row["t_bits"], context + ".t_bits")
            checks.require(t == -ONE + Q(2 * index, steps), f"{context}: original observation clock mismatch")
            checks.require(len(row["state_bits"]) == 2, f"{context}: state dimension")
            values = [bits_fraction(word, context + ".state_bits") for word in row["state_bits"]]
            checks.require(fraction_number(row["t"], context + ".t") == t, f"{context}: time JSON/bits disagree")
            checks.require(len(row["state"]) == 2, f"{context}: JSON state dimension")
            for i in range(2):
                checks.require(fraction_number(row["state"][i], context + ".state") == values[i],
                               f"{context}: state JSON/bits disagree")
            states.append((t, values))
        checks.require(states[0][1] == [Q(39, 64), Q(39, 64)], f"{context}: prescribed exact initial condition changed")
        node_error = max(abs(value - exact_reference(t)) for t, y in states for value in y)
        started = time.perf_counter()
        certificate = trace_path_certificate(states, checks, context)
        certificate["external_oracle_seconds"] = time.perf_counter() - started
        results.append({"route": route, "steps": steps, "status": "PASS_SCOPED_EXACT_PATH_CHECKS",
                        "exact_node_error_inf": str(node_error), "certificate": certificate})
    timing = data["timing"]
    checks.require(timing.get("excludes_exact_oracle") is True, "trace timing must disclose omitted external exact certificate")
    checks.require(timing.get("repeat_count") == 7 and len(timing["samples"]) == 56,
                   "trace registered phase/route raw timing coverage mismatch")
    for kind in ("warmups", "samples"):
        for row in timing[kind]:
            checks.require(row["status"] == "completed", f"trace.timing.{kind}: failed phase cannot support completed comparison")
            checks.require(finite_number(row["seconds"], "trace.timing.seconds") >= 0, "trace negative timing")
            trace_no_root_work(row["work"], checks, "trace.timing.work")
    return {"schema": "exact-trace-original-equation-check-v1", "point_cases": point_results,
            "semantics_checks": semantics, "negative_controls": [{"kind": "distinct_root_count_at_triple_root", "rejected": multiplicity_rejected}],
            "runs": results, "slabs_checked": sum(run["certificate"]["slabs_checked"] for run in results),
            "claim_ceiling": "all-complex-root multiplicity observable; external exact whole-path certificate, not native interval path implementation",
            "timing_limit": "native integration-only timings exclude this exact oracle; any combined total must add actual measured oracle cost"}


def load_json(path):
    def invalid_constant(value):
        raise VerificationError(f"nonfinite JSON constant {value}")
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise VerificationError(f"duplicate JSON key {key}")
            result[key] = value
        return result
    result = json.loads(path.read_text(), parse_constant=invalid_constant, object_pairs_hook=unique_object)
    if not isinstance(result, dict):
        raise VerificationError("top-level native export must be an object")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--closure", type=Path)
    parser.add_argument("--trace", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not args.closure and not args.trace:
        parser.error("supply --closure, --trace, or both")
    if args.output.exists():
        parser.error("output already exists; receipts are create-only")
    started = time.perf_counter()
    checks = Checks()
    receipt = {"schema": "vigilode-rvj-independent-native-oracle-v1", "status": "RUNNING",
               "oracle_sha256": digest(__file__), "inputs": {},
               "acceptance_arithmetic": "exact Fraction of represented binary64 values; exact rational polynomial algebra",
               "decimal_or_float_displays_used_for_acceptance": False,
               "scope": "bounded RVJ original equations only; not cumulative production qualification"}
    try:
        for name, path, verifier in (("closure", args.closure, verify_closure), ("trace", args.trace, verify_trace)):
            if path is None:
                continue
            receipt["inputs"][name] = {"path": str(path.resolve()), "sha256": digest(path), "bytes": path.stat().st_size}
            phase_started = time.perf_counter()
            receipt[name] = verifier(load_json(path), checks)
            receipt[name]["external_oracle_seconds_total"] = time.perf_counter() - phase_started
        receipt["status"] = "PASS_SCOPED_ALL_EXPORTED_SLABS_CHECKED"
    except (VerificationError, KeyError, IndexError, AttributeError, TypeError, ValueError,
            OSError, OverflowError, ZeroDivisionError) as exc:
        receipt["status"] = "FAIL"
        receipt["failure"] = {"exception": type(exc).__name__, "message": str(exc)}
    receipt["checks_completed_including_first_failure"] = checks.count
    receipt["external_oracle_seconds_total"] = time.perf_counter() - started
    with args.output.open("x") as stream:
        json.dump(receipt, stream, indent=2, allow_nan=False)
        stream.write("\n")
    print(json.dumps({"oracle_receipt": str(args.output), "status": receipt["status"],
                      "seconds": receipt["external_oracle_seconds_total"], "checks": checks.count}), flush=True)
    return 0 if receipt["status"] != "FAIL" else 1


if __name__ == "__main__":
    sys.exit(main())
