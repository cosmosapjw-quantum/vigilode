"""Strict shared process and case protocol."""
from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def digest(value):
    return hashlib.sha256(canonical(value).encode()).hexdigest()


def file_digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


PHYSICAL_FIELDS = ("model", "equation_form", "parameters", "initial_state", "boundary_conditions", "mass_matrix", "initial_derivative", "t_span", "output_times")


def physical_input_digest(case):
    return digest({key: case.get(key) for key in PHYSICAL_FIELDS})


def strict_load(text):
    def reject(token):
        raise ValueError("nonfinite JSON constant: " + token)
    return json.loads(text, parse_constant=reject)


def finite_number(value, positive=False):
    return type(value) in (int, float) and math.isfinite(value) and (value > 0 if positive else value >= 0)


CASE_KEYS = ("case_id", "family", "model", "parameters", "initial_state", "t_span", "output_times", "rtol", "atol", "equation_form", "mass_matrix", "reference_policy")
STATUSES = {"OK", "UNSUPPORTED_CAPABILITY", "NUMERICAL_FAILURE", "ENVIRONMENT_FAILURE"}


def validate_case(case):
    errors = []
    if not isinstance(case, dict):
        return ["case_not_object"]
    errors.extend("missing_" + key for key in CASE_KEYS if key not in case)
    if errors:
        return errors
    if not isinstance(case["case_id"], str) or not case["case_id"]:
        errors.append("invalid_case_id")
    y = case["initial_state"]
    if not isinstance(y, list) or not y or any(type(v) not in (int, float) or not math.isfinite(v) for v in y):
        errors.append("invalid_initial_state")
    t = case["t_span"]
    if not isinstance(t, list) or len(t) != 2 or any(type(v) not in (int, float) or not math.isfinite(v) for v in t) or t[0] >= t[1]:
        errors.append("invalid_t_span")
    times = case["output_times"]
    if not isinstance(times, list) or len(times) < 2 or any(type(v) not in (int, float) or not math.isfinite(v) for v in times) or any(a >= b for a, b in zip(times, times[1:])):
        errors.append("invalid_output_times")
    elif isinstance(t, list) and len(t) == 2 and (times[0] != t[0] or times[-1] != t[1]):
        errors.append("output_endpoints_mismatch")
    if not finite_number(case["rtol"], positive=True):
        errors.append("invalid_rtol")
    if not isinstance(case["atol"], list) or not isinstance(y, list) or len(case["atol"]) != len(y) or any(not finite_number(v, positive=True) for v in case["atol"]):
        errors.append("invalid_atol")
    try:
        canonical(case)
    except (TypeError, ValueError):
        errors.append("noncanonical_case")
    return errors


def validate_result(result, case, expected_mode="timing"):
    errors = []
    if not isinstance(result, dict):
        return ["result_not_object"]
    if result.get("status") not in STATUSES:
        errors.append("invalid_status")
    for key in ("times", "states", "counters", "timing", "allocations"):
        if key not in result:
            errors.append("missing_" + key)
    if errors or result["status"] != "OK":
        return errors
    if result.get("measurement_mode") != expected_mode:
        errors.append("measurement_mode_mismatch")
    if result["times"] != case["output_times"]:
        errors.append("output_mesh_mismatch")
    states = result["states"]
    if not isinstance(states, list) or len(states) != len(case["output_times"]) or any(not isinstance(row, list) or len(row) != len(case["initial_state"]) or any(type(x) not in (int, float) or not math.isfinite(x) for x in row) for row in states):
        errors.append("invalid_states")
    if result["counters"] is not None and (not isinstance(result["counters"], dict) or any(v is not None and (type(v) is not int or v < 0) for v in result["counters"].values())):
        errors.append("invalid_counters")
    timing = result["timing"]
    if expected_mode == "timing":
        if result.get("timing_scope") != "warmed_solver_setup_integration_output_certificate":
            errors.append("incomparable_timing_scope")
        if not isinstance(timing, dict) or not finite_number(timing.get("timed_seconds"), positive=True) or not finite_number(timing.get("warmup_seconds")):
            errors.append("invalid_warmed_timing")
    elif expected_mode == "allocation":
        if timing is not None and (not isinstance(timing, dict) or finite_number(timing.get("timed_seconds"), positive=True)):
            errors.append("allocation_mode_timing_claim")
    elif expected_mode == "reference":
        if type(result.get("reference_converged")) is not bool:
            errors.append("invalid_reference_convergence_flag")
        uncertainty = result.get("uncertainty_abs")
        if not isinstance(uncertainty, list) or len(uncertainty) != len(case["output_times"]) or any(not isinstance(row, list) or len(row) != len(case["initial_state"]) or any(not finite_number(value) for value in row) for row in uncertainty):
            errors.append("invalid_reference_uncertainty")
    else:
        errors.append("invalid_expected_mode")
    alloc = result["allocations"]
    if alloc is not None and not isinstance(alloc, dict):
        errors.append("invalid_allocations")
    if expected_mode == "allocation" and not isinstance(alloc, dict):
        errors.append("missing_allocation_measurement")
    return errors
