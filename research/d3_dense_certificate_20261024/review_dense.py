#!/usr/bin/env python3
"""Independent saved-output audit for the scoped VigilODE D3 Riccati model.

This program reads stored curves. It never advances an ODE solver or evaluates
the Rust certificate code. Exact Fraction polynomial arithmetic is independent
of the implementation's Bernstein product/elevation route. Decimal(80) analytic
reference comparisons are numerical audit evidence, not interval proofs.

Run only under the root task's measured subprocess budget.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
from decimal import Decimal, localcontext
from fractions import Fraction
from functools import lru_cache
import hashlib
import json
from math import comb, isfinite
from pathlib import Path
import struct
import sys
import time

DIM = 30
FULL = 33
RETAINED = tuple(list(range(23)) + [24, 26, 27, 29, 30, 31, 32])
GROUPS = ((23, 23, 1, 1.0), (25, 24, 2, 0.5), (28, 26, 3, 0.2))
DEFAULT_LAMBDAS = tuple((1.0, 20.0, 300.0)[j % 3] for j in range(DIM))


class AuditFailure(Exception):
    pass


def require(condition, message):
    if not condition:
        raise AuditFailure(message)


def number(value):
    """Read roundtrip JSON binary64 values (or an explicit hex float/bit token)."""
    if isinstance(value, dict):
        if "bits" in value:
            bits = value["bits"]
            bits = int(bits, 16) if isinstance(bits, str) else int(bits)
            return struct.unpack(">d", struct.pack(">Q", bits))[0]
        if "hex" in value:
            return float.fromhex(value["hex"])
        raise AuditFailure("Unknown numeric object")
    if isinstance(value, str):
        return float.fromhex(value) if value.lstrip("+-").startswith("0x") else float(value)
    return float(value)


def bitkey(value):
    return struct.pack(">d", number(value))


def exact(value):
    value = number(value)
    require(isfinite(value), "non-finite number in saved evidence")
    return Fraction.from_float(value)


def vector(value, length, label):
    require(isinstance(value, list) and len(value) == length, f"{label}: dimension")
    out = tuple(number(x) for x in value)
    require(all(isfinite(x) for x in out), f"{label}: non-finite")
    return out


@lru_cache(maxsize=4096)
def power_coefficients(controls):
    """B_i^n to power coefficients using forward differences at zero."""
    n = len(controls) - 1
    diffs = [Fraction.from_float(x) for x in controls]
    out = []
    for k in range(n + 1):
        out.append(comb(n, k) * diffs[0])
        diffs = [diffs[i + 1] - diffs[i] for i in range(len(diffs) - 1)]
    return tuple(out)


def power_evaluate(coefficients, theta):
    value = Fraction(0)
    for a in reversed(coefficients):
        value = value * theta + a
    return value


@lru_cache(maxsize=4096)
def defect_coefficients(controls, delta, lam, alpha):
    """Independent exact power derivative/product followed by power->Bernstein."""
    a = power_coefficients(controls)
    n = len(a) - 1
    m = 2 * n
    d = [Fraction(0) for _ in range(m + 1)]
    for k in range(n):
        d[k] += (k + 1) * a[k + 1] / delta
    for k in range(n + 1):
        d[k] += lam * a[k]
    for i in range(n + 1):
        for j in range(n + 1):
            d[i + j] += alpha * a[i] * a[j]
    return tuple(
        sum((d[j] * Fraction(comb(k, j), comb(m, j)) for j in range(k + 1)), Fraction(0))
        for k in range(m + 1)
    )


def full_lift(values):
    """Exact affine lift of Fraction or Decimal components; exact binary64 totals."""
    out = [values[0] * 0 for _ in range(FULL)]
    for j, dst in enumerate(RETAINED):
        out[dst] = values[j]
    if isinstance(values[0], Decimal):
        total = lambda x: Decimal.from_float(x)
    else:
        total = Fraction.from_float
    for row, start, count, limit in GROUPS:
        out[row] = total(limit) - sum(values[start:start + count])
    return tuple(out)


def full_radius(values):
    out = [values[0] * 0 for _ in range(FULL)]
    for j, dst in enumerate(RETAINED):
        out[dst] = values[j]
    for row, start, count, _ in GROUPS:
        out[row] = sum(values[start:start + count])
    return tuple(out)


def to_decimal(value):
    if isinstance(value, Fraction):
        return Decimal(value.numerator) / Decimal(value.denominator)
    return Decimal.from_float(number(value))


def json_digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_rows(path):
    if path.suffix.lower() == ".jsonl":
        rows = []
        with path.open() as stream:
            for lineno, line in enumerate(stream, 1):
                if line.strip():
                    row = json.loads(line)
                    require(isinstance(row, dict), f"JSONL row {lineno}: object required")
                    if row.get("type", "segment") == "segment":
                        rows.append(row)
        return rows
    raw = json.loads(path.read_text())
    return raw if isinstance(raw, list) else raw["segments"]


def run_key(row):
    return str(row.get("run", row.get("run_id")))


def get_summary(path):
    raw = json.loads(path.read_text())
    runs = raw["runs"] if isinstance(raw, dict) and "runs" in raw else raw
    if isinstance(runs, dict):
        rows = []
        for key, value in runs.items():
            require(isinstance(value, dict), "summary run must be an object")
            row = dict(value)
            row.setdefault("run", key)
            rows.append(row)
        runs = rows
    require(isinstance(runs, list), "summary requires runs")
    mapping = {}
    for row in runs:
        key = run_key(row)
        require(key not in mapping, f"duplicate summary run {key}")
        mapping[key] = row
    return raw, mapping


def field(row, *names, default=None, required=True):
    for name in names:
        if name in row:
            return row[name]
    if required:
        raise AuditFailure("Missing field: " + "/".join(names))
    return default


def matrix_components(raw, width, label):
    """The canonical layout is [30][width]; controls use [degree+1][30]."""
    require(isinstance(raw, list) and len(raw) == DIM, f"{label}: component-major dimension")
    return tuple(vector(row, width, label) for row in raw)


def choose_coefficient_rows(rows, mode):
    if mode == "all":
        return set(range(len(rows)))
    by_run = defaultdict(list)
    for i, row in enumerate(rows):
        by_run[run_key(row)].append(i)
    chosen = set()
    for indices in by_run.values():
        chosen.update((indices[0], indices[-1]))
        chosen.add(max(indices, key=lambda i: max(number(x) for x in rows[i]["rho"])))
    return chosen


def audit(raw_path, summary_path, coefficient_mode, precision):
    started = time.monotonic()
    rows = load_rows(raw_path)
    require(rows, "No stored segments")
    raw_summary, summaries = get_summary(summary_path)
    require(len(summaries) == 8, "Frozen D3 campaign requires exactly eight summary runs")
    selected = choose_coefficient_rows(rows, coefficient_mode)
    by_run = defaultdict(list)
    for i, row in enumerate(rows):
        by_run[run_key(row)].append((i, row))
    require(set(by_run) == set(summaries), "run identity mismatch between summary and segments")
    counts = Counter()
    kinds = Counter()
    details = []
    query_audit_rows = []
    max_query_error = Decimal(0)
    max_query_ratio = Decimal(0)
    max_endpoint_ratio = Decimal(0)
    maximum_rounding_ratio = Fraction(0)
    minimum_domain_control = None
    source_inputs = {
        str(raw_path): json_digest(raw_path),
        str(summary_path): json_digest(summary_path),
        str(Path(__file__).resolve()): json_digest(Path(__file__).resolve()),
    }

    with localcontext() as context:
        context.prec = precision
        # This threshold only describes Decimal audit resolution; it is never
        # substituted into the Rust certificate or the user curve budget.
        audit_slack = Decimal(10) ** (-(precision - 12))
        for key, indexed_rows in by_run.items():
            summary = summaries[key]
            initial = vector(field(summary, "initial_state", "initial", "z0"), DIM, "initial")
            init_time = exact(field(summary, "initial_time", "t_initial", default=0.0, required=False))
            tf = exact(field(summary, "tf"))
            budget = exact(field(summary, "curve_budget", "budget"))
            alpha = exact(field(summary, "alpha"))
            degree = int(field(summary, "degree"))
            lambdas = vector(field(summary, "lambda", "lambdas", default=list(DEFAULT_LAMBDAS), required=False),
                             DIM, "lambda")
            require(alpha >= 0 and all(l > 0 for l in lambdas), f"{key}: contraction assumptions")
            require(lambdas == DEFAULT_LAMBDAS, f"{key}: lambda pattern differs from frozen model")
            require(degree in (3, 5), f"{key}: unsupported degree")
            require(budget > 0, f"{key}: positive curve budget")
            require(initial and all(x >= 0 for x in initial), f"{key}: exact solution positivity")
            require(tf > init_time, f"{key}: final time")
            previous = None
            previous_index = None
            per_run_counts = Counter()
            per_run_kinds = Counter()
            run_max_query_error = Decimal(0)
            run_max_query_ratio = Decimal(0)
            run_max_endpoint_ratio = Decimal(0)
            run_max_returned_radius = Decimal(0)
            requested_time_bits = []
            exponent_cache = {}

            def reference(time_value):
                elapsed = exact(time_value) - init_time
                require(elapsed >= 0, f"{key}: reference time before initial state")
                z = []
                for j, lam_float in enumerate(lambdas):
                    lam_exact = Fraction.from_float(lam_float)
                    exp_key = (elapsed, lam_exact)
                    if exp_key not in exponent_cache:
                        exponent_cache[exp_key] = (-to_decimal(lam_exact * elapsed)).exp()
                    e = exponent_cache[exp_key]
                    z0 = Decimal.from_float(initial[j])
                    aa = to_decimal(alpha)
                    ll = Decimal.from_float(lam_float)
                    z.append(z0 * e / (Decimal(1) + aa * z0 * (Decimal(1) - e) / ll))
                return tuple(z)

            for global_index, row in indexed_rows:
                idx = int(field(row, "idx", "segment"))
                if previous_index is not None:
                    require(idx == previous_index + 1, f"{key}/{idx}: nonconsecutive segment index")
                previous_index = idx
                require(int(row["degree"]) == degree and exact(row["alpha"]) == alpha,
                        f"{key}/{idx}: model/degree binding mismatch")
                t0, t1 = exact(row["t0"]), exact(row["t1"])
                dt = t1 - t0
                require(dt > 0 and t1 <= tf, f"{key}/{idx}: nonincreasing/outside time")
                controls_raw = row["controls"]
                require(isinstance(controls_raw, list) and len(controls_raw) == degree + 1,
                        f"{key}/{idx}: controls degree")
                controls = tuple(vector(v, DIM, "controls") for v in controls_raw)
                incoming = vector(row["incoming"], DIM, "incoming")
                endpoint = vector(row["endpoint"], DIM, "endpoint radius")
                uniform = vector(row["uniform"], DIM, "uniform radius")
                rho = vector(row["rho"], DIM, "rho")
                require(all(x >= 0 for a in (incoming, endpoint, uniform, rho) for x in a),
                        f"{key}/{idx}: negative radius/residual bound")
                lifted_uniform = full_radius(tuple(Fraction.from_float(x) for x in uniform))
                require(all(x <= budget for x in lifted_uniform), f"{key}/{idx}: uniform full33 curve budget exceeded")
                counts["full33_uniform_budget_checks"] += FULL
                delta = vector(row["delta"], 2, "delta")
                require(0 < exact(delta[0]) <= dt <= exact(delta[1]), f"{key}/{idx}: delta enclosure")
                if previous is None:
                    require(t0 == init_time, f"{key}: missing initial interval")
                    require(all(bitkey(a) == bitkey(b) for a, b in zip(controls[0], initial)),
                            f"{key}: initial state binding")
                    require(all(x == 0 for x in incoming), f"{key}: initial radius must be zero")
                else:
                    require(bitkey(row["t0"]) == bitkey(previous["t1"]), f"{key}/{idx}: clock gap")
                    require(all(bitkey(a) == bitkey(b) for a, b in zip(controls[0], previous["controls"][-1])),
                            f"{key}/{idx}: curve endpoint continuity")
                    require(all(bitkey(a) == bitkey(b) for a, b in zip(incoming, previous["endpoint"])),
                            f"{key}/{idx}: incoming radius was not carried exactly")
                    counts["inherited_component_bit_checks"] += DIM

                for control in controls:
                    exact_full = full_lift(tuple(Fraction.from_float(x) for x in control))
                    require(all(x >= 0 for x in exact_full), f"{key}/{idx}: Bernstein domain violation")
                    minimum = min(exact_full)
                    minimum_domain_control = minimum if minimum_domain_control is None else min(minimum_domain_control, minimum)
                    counts["domain_control_components"] += FULL

                lo = matrix_components(row["defect_lo"], 2 * degree + 1, "defect_lo")
                hi = matrix_components(row["defect_hi"], 2 * degree + 1, "defect_hi")
                for j in range(DIM):
                    require(all(a <= b for a, b in zip(lo[j], hi[j])), f"{key}/{idx}/{j}: reversed defect interval")
                    r = Fraction.from_float(rho[j])
                    require(all(abs(Fraction.from_float(x)) <= r for x in (*lo[j], *hi[j])),
                            f"{key}/{idx}/{j}: rho below stored Bernstein interval extremum")
                    counts["stored_defect_interval_checks"] += 2 * degree + 1
                    if global_index in selected:
                        component_controls = tuple(control[j] for control in controls)
                        coefficients = defect_coefficients(component_controls, dt, Fraction.from_float(lambdas[j]), alpha)
                        for k, value in enumerate(coefficients):
                            require(Fraction.from_float(lo[j][k]) <= value <= Fraction.from_float(hi[j][k]),
                                    f"{key}/{idx}/{j}/{k}: exact residual coefficient outside stored interval")
                            counts["exact_defect_coefficient_checks"] += 1
                    e0 = Decimal.from_float(incoming[j])
                    rr = Decimal.from_float(rho[j])
                    lam = Decimal.from_float(lambdas[j])
                    exp_key = (dt, Fraction.from_float(lambdas[j]))
                    if exp_key not in exponent_cache:
                        exponent_cache[exp_key] = (-lam * to_decimal(dt)).exp()
                    decay = exponent_cache[exp_key]
                    endpoint_reference = decay * e0 + (Decimal(1) - decay) * rr / lam
                    stored_endpoint = Decimal.from_float(endpoint[j])
                    require(stored_endpoint + audit_slack * max(Decimal(1), abs(endpoint_reference)) >= endpoint_reference,
                            f"{key}/{idx}/{j}: endpoint contraction underestimate")
                    require(exact(uniform[j]) >= max(exact(incoming[j]), exact(endpoint[j])),
                            f"{key}/{idx}/{j}: uniform does not contain incoming/endpoint bounds")
                    counts["contraction_component_checks"] += 1

                exact_at_endpoint = reference(row["t1"])
                for j in range(DIM):
                    error = abs(Decimal.from_float(controls[-1][j]) - exact_at_endpoint[j])
                    radius = Decimal.from_float(endpoint[j])
                    require(error <= radius + audit_slack, f"{key}/{idx}/{j}: analytic endpoint outside radius")
                    ratio = error / radius if radius else Decimal(0)
                    max_endpoint_ratio = max(max_endpoint_ratio, ratio)
                    run_max_endpoint_ratio = max(run_max_endpoint_ratio, ratio)
                    counts["analytic_endpoint_components"] += 1

                queries = row["queries"]
                require(isinstance(queries, list), f"{key}/{idx}: queries must be a list")
                segment_interior_queries = 0
                for q_index, query in enumerate(queries):
                    tq = exact(query["t"])
                    require(t0 <= tq <= t1, f"{key}/{idx}/{q_index}: query outside segment")
                    kind = str(query.get("kind", "unknown"))
                    require(kind in ("requested", "interior"), f"{key}/{idx}/{q_index}: unknown query kind")
                    if kind == "interior":
                        require(t0 < tq < t1, f"{key}/{idx}/{q_index}: interior query is not strictly interior")
                        segment_interior_queries += 1
                    else:
                        requested_time_bits.append(bitkey(query["t"]))
                    theta = (tq - t0) / dt
                    center = vector(query["center"], FULL, "query center")
                    radius = vector(query["radius"], FULL, "query radius")
                    rounding = vector(query["rounding"], FULL, "query rounding")
                    require(all(x >= 0 for a in (center, radius, rounding) for x in a),
                            f"{key}/{idx}/{q_index}: query nonnegativity")
                    require(all(exact(x) <= budget for x in radius),
                            f"{key}/{idx}/{q_index}: full33 query budget exceeded")
                    p_exact = tuple(power_evaluate(power_coefficients(tuple(c[j] for c in controls)), theta)
                                    for j in range(DIM))
                    full_exact = full_lift(p_exact)
                    base_radius = full_radius(tuple(Fraction.from_float(x) for x in uniform))
                    exact_reference = full_lift(reference(query["t"]))
                    query_max_error = Decimal(0)
                    query_max_ratio = Decimal(0)
                    query_max_radius = max(Decimal.from_float(x) for x in radius)
                    for j in range(FULL):
                        round_error = abs(Fraction.from_float(center[j]) - full_exact[j])
                        round_bound = Fraction.from_float(rounding[j])
                        require(round_error <= round_bound, f"{key}/{idx}/{q_index}/{j}: query rounding underestimate")
                        if round_bound:
                            maximum_rounding_ratio = max(maximum_rounding_ratio, round_error / round_bound)
                        require(Fraction.from_float(radius[j]) >= base_radius[j] + round_bound,
                                f"{key}/{idx}/{q_index}/{j}: full33 radius sum underestimate")
                        error = abs(Decimal.from_float(center[j]) - exact_reference[j])
                        bound = Decimal.from_float(radius[j])
                        require(error <= bound + audit_slack,
                                f"{key}/{idx}/{q_index}/{j}: analytic returned query outside radius")
                        max_query_error = max(max_query_error, error)
                        query_max_error = max(query_max_error, error)
                        if bound:
                            max_query_ratio = max(max_query_ratio, error / bound)
                            query_max_ratio = max(query_max_ratio, error / bound)
                    run_max_query_error = max(run_max_query_error, query_max_error)
                    run_max_query_ratio = max(run_max_query_ratio, query_max_ratio)
                    run_max_returned_radius = max(run_max_returned_radius, query_max_radius)
                    query_audit_rows.append({
                        "run": key, "idx": idx, "kind": kind, "t": number(query["t"]),
                        "maxabs_error": str(query_max_error), "maxradius": str(query_max_radius),
                        "max_error_bound_ratio": str(query_max_ratio)
                    })
                    kinds[kind] += 1
                    per_run_kinds[kind] += 1
                    counts["query_components"] += FULL
                    counts["queries"] += 1
                    per_run_counts["queries"] += 1
                require(segment_interior_queries == 1,
                        f"{key}/{idx}: exactly one strictly interior query required in this segment")
                previous = row
                counts["segments"] += 1
                per_run_counts["segments"] += 1
                if global_index in selected:
                    counts["exact_coefficient_segments"] += 1
                    per_run_counts["exact_coefficient_segments"] += 1

            require(exact(previous["t1"]) == tf, f"{key}: incomplete final interval")
            require(per_run_kinds["requested"] == 37, f"{key}: frozen 37 requested outputs not present")
            # Reconstruct the frozen query schedule independently of both saved
            # queries and summary metadata, preserving binary64 operation order.
            tf_float = float(tf)
            expected_times = [tf_float * float(k) / 37.0 for k in range(1, 38)]
            expected_times[-1] = tf_float
            expected_time_bits = [bitkey(t) for t in expected_times]
            require(len(set(expected_time_bits)) == 37, f"{key}: frozen query schedule is not unique")
            require(len(set(requested_time_bits)) == 37, f"{key}: duplicate requested query times")
            require(Counter(requested_time_bits) == Counter(expected_time_bits),
                    f"{key}: requested query times differ from frozen binary64 tf*k/37 schedule")
            if "requested_times" in summary:
                summary_requested = vector(summary["requested_times"], 37, "summary requested_times")
                require([bitkey(t) for t in summary_requested] == expected_time_bits,
                        f"{key}: summary requested_times differs from independently reconstructed schedule")
            require(per_run_kinds["interior"] == per_run_counts["segments"],
                    f"{key}: exactly one interior query per accepted segment required")
            accepted_summary = field(summary, "accepted", "segments", required=False)
            if accepted_summary is not None:
                require(int(accepted_summary) == per_run_counts["segments"], f"{key}: summary accepted count mismatch")
            for kind, expected in summary.get("query_counts", {}).items():
                require(per_run_kinds[kind] == int(expected), f"{key}: query count mismatch for {kind}")
            expected_queries = summary.get("queries")
            if isinstance(expected_queries, int):
                require(expected_queries == per_run_counts["queries"], f"{key}: total query count mismatch")
            details.append({"run": key, "alpha": float(alpha), "degree": degree,
                            "curve_budget": float(budget), **per_run_counts, "query_kinds": dict(per_run_kinds),
                            "max_absolute_query_error": str(run_max_query_error),
                            "max_query_error_over_bound": str(run_max_query_ratio),
                            "max_endpoint_error_over_bound": str(run_max_endpoint_ratio),
                            "max_returned_radius": str(run_max_returned_radius)})

        require({(r["alpha"], r["degree"], r["curve_budget"]) for r in details} ==
                {(a, d, b) for a in (0.0, 4.0) for d in (3, 5) for b in (2e-6, 2e-8)},
                "Frozen campaign parameter grid mismatch")

        return {
            "schema": "vigilode.d3.saved-independent-oracle.v1",
            "status": ("PASS_ALL_SAVED_DEFECTS_BOUNDS_AND_QUERIES" if coefficient_mode == "all"
                       else "PASS_ALL_SAVED_QUERIES_AND_SELECTED_EXACT_COEFFICIENTS"),
            "scope": "new D3 stored Riccati curves only; no solver execution; no D2 replay",
            "methods": {
                "defect": "Fraction Bernstein-to-power, derivative / exact(t1-t0), polynomial convolution, power-to-Bernstein",
                "domain": "Exact Fraction affine lift of every Bernstein control",
                "query_rounding": "Exact Fraction Horner evaluation at exact query time and affine lift",
                "analytic_reference": f"Python Decimal with {precision} significant digits, exp and Riccati formula",
                "contraction": f"Decimal{precision} numerical audit of exp(-lambda*delta) E0 + (1-exp(-lambda*delta)) rho/lambda",
                "decimal_audit_slack": str(audit_slack),
                "decimal_is_formal_interval_proof": False,
                "inheritance": "Bitwise incoming E0 == preceding stored endpoint E1, all segments",
                "full33_budget": "All query radii include exact component sums and separate recorded rounding bound"
            },
            "coefficient_scope": coefficient_mode,
            "counts": dict(counts),
            "query_kinds": dict(kinds),
            "runs": details,
            "query_audit_rows": query_audit_rows,
            "max_absolute_query_error": str(max_query_error),
            "max_query_error_over_bound": str(max_query_ratio),
            "max_endpoint_error_over_bound": str(max_endpoint_ratio),
            "max_query_rounding_error_over_bound": str(to_decimal(maximum_rounding_ratio)),
            "minimum_exact_domain_control": str(minimum_domain_control),
            "input_sha256": source_inputs,
            "new_solver_runs": 0,
            "old_campaign_replays": 0,
            "elapsed_seconds": time.monotonic() - started,
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", type=Path)
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--coefficients", choices=("all", "representative"), default="all")
    parser.add_argument("--precision", type=int, default=80)
    args = parser.parse_args()
    require(args.precision >= 80, "At least 80 Decimal digits required")
    try:
        result = audit(args.raw, args.summary, args.coefficients, args.precision)
    except (AuditFailure, KeyError, ValueError, TypeError, ArithmeticError) as error:
        result = {"schema": "vigilode.d3.saved-independent-oracle.v1", "status": "FAIL",
                  "reason": f"{type(error).__name__}: {error}", "coefficient_scope": args.coefficients,
                  "new_solver_runs": 0, "old_campaign_replays": 0}
        args.output.write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
        return 1
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    # Plot rows remain in the saved report; avoid dumping all of them into logs.
    console_result = {k: v for k, v in result.items() if k != "query_audit_rows"}
    console_result["query_audit_rows_saved"] = len(result["query_audit_rows"])
    print(json.dumps(console_result, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

