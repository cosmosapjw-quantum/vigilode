#!/usr/bin/env python3
"""Lightweight mathematical diagnostics; no interval or native-solver certificate.

All generated files stay outside the read-only repository. Python's libm/cmath
transcendentals are approximate. Exact Fraction checks concern only rational
combinatorial recurrences. Model equations are read from the pinned source.
"""
from __future__ import annotations

import cmath
from fractions import Fraction as Q
import hashlib
import json
import math
from pathlib import Path
import sys
import traceback

HERE = Path(__file__).resolve().parent


def norm(a, w):
    return math.sqrt(math.fsum(wj * abs(aj) ** 2 for wj, aj in zip(w, a)))


def distance(a, b, w):
    return norm([x - y for x, y in zip(a, b)], w)


def linear(a, h, kappa, omega):
    return [cmath.exp(complex(-kappa, oj) * h) * aj for aj, oj in zip(a, omega)]


def nonlinear(a, h, w, coupling, lam):
    s = math.fsum(w)
    mean = complex(math.fsum(wj * aj.real for wj, aj in zip(w, a)),
                   math.fsum(wj * aj.imag for wj, aj in zip(w, a)))
    theta = lam * coupling * s * abs(mean) ** 2 * h
    # Cancellation-resistant exact-form increment, still floating diagnostics.
    em1 = complex(-2 * math.sin(theta / 2) ** 2, math.sin(theta))
    shift = (mean / s) * em1
    return [aj + shift for aj in a]


def fine(a, h, steps, w, coupling, lam, kappa, omega):
    a = list(a)
    dt = h / steps
    for _ in range(steps):
        a = linear(a, dt / 2, kappa, omega)
        a = nonlinear(a, dt, w, coupling, lam)
        a = linear(a, dt / 2, kappa, omega)
    return a


def project(a, radius, w):
    n = norm(a, w)
    if n <= radius or n == 0:
        return list(a), False
    return [z * (radius / n) for z in a], True


def preconditioned_step(z, initial, h, m, w, chi, lam, kappa, omega, radius):
    if z[0] != initial:
        raise ValueError("initial boundary must be the prescribed initial state")
    defects = []
    for zj in z[:-1]:
        fj = fine(zj, h, m, w, chi, lam, kappa, omega)
        gj = linear(zj, h, kappa, omega)
        defects.append([f - g for f, g in zip(fj, gj)])
    # Full affine recurrence FIRST. Projection must not alter its prefixes.
    raw = [list(initial)]
    for dj in defects:
        previous = linear(raw[-1], h, kappa, omega)
        raw.append([g + d for g, d in zip(previous, dj)])
    projected = [list(initial)]
    activations = 0
    for j, rj in enumerate(raw[1:], 1):
        pj, active = project(rj, radius * math.exp(-kappa * j * h), w)
        projected.append(pj)
        activations += active
    return projected, activations


def parallel_case(name, w, kappa, chi, h, windows, substeps, lam=1.0):
    omega = [-0.31, 0.12, 0.43]
    initial = [0.55 + 0.1j, -0.2 + 0.4j, 0.25 - 0.3j]
    radius = norm(initial, w)
    s = math.fsum(w)
    g = math.exp(-kappa * h)
    beta = abs(lam * chi) * s * s * radius * radius
    dt = h / substeps
    tau = math.fsum(dt * math.exp(-kappa * (2 * l + 1) * dt)
                    for l in range(substeps))
    defect_lip = 3 * g * math.expm1(beta * tau)
    rho = defect_lip * math.fsum(g**j for j in range(windows))
    target = [list(initial)]
    for _ in range(windows):
        target.append(fine(target[-1], h, substeps, w, chi, lam, kappa, omega))
    # Deliberately poor feasible guess, rather than a nearly exact warm start.
    guess = [list(initial)]
    for j in range(1, windows + 1):
        candidate = [complex(math.cos(j + n), math.sin(1.7 * j - n))
                     for n in range(len(w))]
        factor = radius * math.exp(-kappa * j * h) / norm(candidate, w)
        guess.append([v * factor for v in candidate])
    initial_errors = [distance(x, y, w) for x, y in zip(guess, target)]
    e_initial = max(initial_errors)
    previous_majorant = initial_errors
    floor = 2e-12 * max(1.0, radius) * (windows + 1)
    rounds = []
    activations_total = 0
    max_bound_violation = 0.0
    for k in range(1, windows + 1):
        guess, active = preconditioned_step(guess, initial, h, substeps, w, chi,
                                            lam, kappa, omega, radius)
        activations_total += active
        actual = [distance(x, y, w) for x, y in zip(guess, target)]
        majorant = [0.0]
        for j in range(1, windows + 1):
            majorant.append(defect_lip * math.fsum(
                g ** (j - 1 - i) * previous_majorant[i] for i in range(j)))
        binomial = [e_initial * defect_lip**k * math.comb(j - 1, k)
                    if j > k else 0.0 for j in range(windows + 1)]
        factorial = [e_initial * (defect_lip * max(j - 1, 0))**k /
                     math.factorial(k) if j > k else 0.0
                     for j in range(windows + 1)]
        assert all(a <= b + floor for a, b in zip(actual, majorant))
        assert all(a <= b + floor for a, b in zip(actual, binomial))
        assert all(b <= f + floor for b, f in zip(binomial, factorial))
        assert max(actual[:k + 1]) <= floor
        max_bound_violation = max(max_bound_violation,
                                  max(a - b for a, b in zip(actual, majorant)))
        rounds.append({"k": k, "max_error_to_serial_fine": max(actual),
                       "finite_front_max_error": max(actual[:k + 1]),
                       "recurrence_majorant": max(majorant),
                       "binomial_majorant": max(binomial),
                       "factorial_majorant": max(factorial),
                       "projection_activations": active})
        previous_majorant = majorant
    energy_relative = max(abs(norm(yj, w) /
        (radius * math.exp(-kappa * j * h)) - 1)
        for j, yj in enumerate(target))
    assert energy_relative < 2e-12
    bad = [list(x) for x in guess]
    bad[0][0] += 1e-6
    try:
        preconditioned_step(bad, initial, h, substeps, w, chi, lam, kappa,
                            omega, radius)
    except ValueError:
        initial_guard = True
    else:
        raise AssertionError("initial boundary mismatch was not detected")
    # The unconstrained shooting equations have a different root if a0 changes.
    changed_target = fine(bad[0], h, substeps, w, chi, lam, kappa, omega)
    wrong_initial_root_distance = distance(changed_target, target[1], w)
    assert wrong_initial_root_distance > 1e-8
    return {"name": name, "status": "NUMERICALLY_CHECKED_FLOATING_DIAGNOSTIC",
            "weights": w, "actual_weight_sum_float": s,
            "constructor_weight_tolerance_admitted": abs(s - 1) <= 1e-12,
            "kappa": kappa, "chi": chi, "h": h, "windows": windows,
            "substeps": substeps, "lambda": lam, "radius": radius,
            "beta_nonlinear": beta, "d_derivative_defect": defect_lip,
            "rho_geometric_contraction_bound": rho,
            "roundoff_comparison_allowance": floor,
            "max_recurrence_violation_before_allowance": max_bound_violation,
            "energy_relative_deviation": energy_relative,
            "projection_activations": activations_total,
            "initial_guard_checked": initial_guard,
            "wrong_initial_root_distance": wrong_initial_root_distance,
            "rounds": rounds}


def fraction_combinatorics():
    cases = []
    checks = 0
    for g in [Q(0), Q(2, 3), Q(1)]:
        for d in [Q(1, 11), Q(3, 2)]:
            m = 16
            initial = [Q(0)] + [Q(1)] * m
            e = initial
            for k in range(1, m + 1):
                e = [Q(0)] + [d * sum((g**(j-1-i) * e[i]
                         for i in range(j)), Q(0)) for j in range(1, m + 1)]
                for j in range(m + 1):
                    exact_sum = (d**k * sum((Q(math.comb(l-1, k-1)) *
                        g**(l-k) for l in range(k, j)), Q(0))) if j > k else Q(0)
                    b = d**k * math.comb(j-1, k) if j > k else Q(0)
                    f = d**k * Q(max(j-1, 0)**k, math.factorial(k)) if j > k else Q(0)
                    assert e[j] == exact_sum
                    assert e[j] <= b <= f
                    if g == 1:
                        assert e[j] == b
                    if j <= k:
                        assert e[j] == 0
                    checks += 1
            cases.append({"g": str(g), "d": str(d), "windows": m,
                          "finite_front_final_exact_zero": all(v == 0 for v in e)})
    return {"status": "EXACT_RATIONAL_CHECK", "asserted_boundary_round_values": checks,
            "cases": cases}


def scalar_exact():
    cases = []
    for kappa in [0.0, 0.7]:
        for s in [1 + 5e-13, 1.2]:
            for lam in [0.0, 0.35, 1.0]:
                a0 = 0.6 + 0.2j
                omega, chi, h = 0.43, 1.7, 1.3
                r = math.sqrt(s) * abs(a0)
                c = abs(chi) * s*s * r*r
                damp_time = h if kappa == 0 else -math.expm1(-2*kappa*h)/(2*kappa)
                phase = chi * s**3 * abs(a0)**2 * damp_time
                exact = a0 * cmath.exp(complex(-kappa*h, omega*h + lam*phase))
                v_exact = r * math.exp(-kappa*h) * abs(phase)
                w_exact = r * math.exp(-kappa*h) * phase**2
                v_bound = r * math.exp(-kappa*h) * math.expm1(c*damp_time)
                w_bound = 6*r*math.exp(-kappa*h)*math.exp(c*damp_time)*(
                    math.expm1(c*damp_time)-c*damp_time)
                assert v_exact <= v_bound + 1e-14
                assert w_exact <= w_bound + 1e-14
                rows = []
                for m in [1, 2, 4, 8, 16, 32]:
                    dt = h/m
                    tau = math.fsum(dt*math.exp(-kappa*(2*l+1)*dt) for l in range(m))
                    # Independent N=1 closed form for the repeated split map.
                    split_closed = a0 * cmath.exp(complex(-kappa*h,
                        omega*h + lam*chi*s**3*abs(a0)**2*tau))
                    computed = fine([a0], h, m, [s], chi, lam, kappa, [omega])[0]
                    independent_agreement = math.sqrt(s)*abs(computed-split_closed)
                    actual_error = math.sqrt(s)*abs(computed-exact)
                    phase_error_bound = r*math.exp(-kappa*h)*abs(
                        lam*chi*s**3*abs(a0)**2*(damp_time-tau))
                    # Ω=0 after removing the common scalar frequency.
                    a = kappa
                    accumulated = 0.0
                    for l in range(m):
                        rl = r*math.exp(-kappa*l*dt)
                        beta = abs(lam*chi)*s*s*rl*rl
                        q = math.exp(-kappa*dt + math.asinh(beta*dt*math.exp(-kappa*dt)))
                        local_d = dt**3*math.exp(beta*dt)*rl*(2/3*a*a*beta+2*a*beta*beta)
                        accumulated = q*accumulated + local_d
                    assert independent_agreement < 3e-14
                    assert actual_error <= phase_error_bound + 3e-14
                    assert actual_error <= accumulated + 3e-14
                    rows.append({"m": m, "error_to_exact_ode": actual_error,
                        "agreement_with_independent_split_closed_form": independent_agreement,
                        "sharp_phase_bound": phase_error_bound,
                        "general_commutator_bound": accumulated})
                if kappa > 0 and lam > 0:
                    ratio = rows[-2]["error_to_exact_ode"]/rows[-1]["error_to_exact_ode"]
                    assert 3.9 < ratio < 4.1
                else:
                    ratio = None
                cases.append({"kappa": kappa, "S": s, "lambda": lam,
                    "V_exact": v_exact, "V_bound": v_bound,
                    "W_exact": w_exact, "W_bound": w_bound,
                    "fine_split_order_ratio_m16_m32": ratio, "rows": rows})
    return {"status": "NUMERICALLY_CHECKED_AGAINST_EXACT_SCALAR_ODE",
            "cases": cases}


def theoretical_cost_witness():
    # Costs below are explicit abstract assumptions, not measurements. Unit: one
    # full N-mode Strang substep, incl its reductions/transcendentals, at fixed N.
    h, substeps, kappa, chi, r, s = 1.0, 512, 1.0, 0.1, 1.0, 1.0
    g = math.exp(-kappa*h)
    tau = (h/substeps)*math.fsum(math.exp(-kappa*(2*l+1)*h/substeps)
                               for l in range(substeps))
    d = 3*g*math.expm1(chi*s*s*r*r*tau)
    epsilon, initial_error = 5e-7, 2.0
    witnesses = []
    for windows, workers in [(32,16),(64,32)]:
        rho = d*math.fsum(g**j for j in range(windows))
        k = math.ceil(math.log(epsilon/initial_error)/math.log(rho))
        batches = math.ceil(windows/workers)
        fine_cost = substeps
        vector_scan_span = 4*math.ceil(math.log2(windows)) # 4 units per tree level
        scalar_scan_span = math.ceil(math.log2(windows))
        projection_span = batches*2
        dispatch_sync_span = 4
        one_round = batches*fine_cost + vector_scan_span + projection_span + dispatch_sync_span
        setup = 2*windows + substeps # conservative serial phase/metadata setup
        final_fine = batches*fine_cost
        certification = scalar_scan_span + batches*2
        total = setup+k*one_round+final_fine+certification
        serial = windows*fine_cost
        assert rho < 1 and initial_error*rho**k <= epsilon
        assert total < serial
        witnesses.append({"windows":windows,"workers":workers,"fine_substeps":substeps,
          "rho":rho,"rounds":k,"shooting_error_majorant":initial_error*rho**k,
          "assumed_fine_map_cost":fine_cost,"setup_cost":setup,
          "vector_scan_span_per_round":vector_scan_span,
          "projection_span_per_round":projection_span,
          "dispatch_sync_span_per_round":dispatch_sync_span,
          "final_fine_pass_cost":final_fine,"scalar_scan_and_certification_cost":certification,
          "accepted_path_span":total,"serial_fine_span":serial,
          "conditional_span_ratio_serial_over_parallel":serial/total,
          "rejected_rounds":0,"rejected_round_extra_cost":one_round,
          "fallback_executed":False,"serial_fallback_extra_cost":serial,
          "span_if_one_rejected_round_then_accept":total+one_round,
          "span_if_full_attempt_then_serial_fallback":total+serial,
          "all_fine_map_evaluations":(k+1)*windows,
          "work_increased_over_serial":True})
    return {"status":"CONDITIONAL_COST_MODEL_WITNESS_NOT_MEASURED",
      "cost_unit":"one full N-mode Strang substep at fixed N; overhead costs assumed explicitly",
      "floating_cost_evaluation_only":True,
      "claim":"Nonempty sufficient span-reduction regime under listed costs; no hardware speed guarantee",
      "witnesses":witnesses}


def run():
    cases = [
        parallel_case("undamped_small_defect",[0.2,0.3,0.5000000000005],0.0,0.15,0.15,12,12),
        parallel_case("damped_small_defect",[0.2,0.3,0.5000000000005],0.8,0.4,0.3,12,12),
        parallel_case("undamped_noncontractive_defect",[0.2,0.3,0.5000000000005],0.0,8.0,0.5,10,16),
        parallel_case("generalized_S_1p2_stress",[0.2,0.3,0.7],0.3,5.0,0.4,10,16),
    ]
    assert any(c["rho_geometric_contraction_bound"]<1 for c in cases)
    assert any(c["rho_geometric_contraction_bound"]>1 for c in cases)
    assert any(c["beta_nonlinear"]<1 for c in cases)
    assert any(c["beta_nonlinear"]>1 for c in cases)
    assert sum(c["projection_activations"] for c in cases)>0
    return {"schema_version":1,"script_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
      "scope":"New mathematical discriminating checks only; no existing scientific suites rerun",
      "native_rust_execution":False,"repo_mutations":False,"push_performed":False,
      "transcendental_rounding_certified":False,
      "floating_claim_status":"NUMERICALLY_CHECKED; NOT IMPLEMENTATION_VERIFIED NATIVE CLIENT",
      "parallel_cases":cases,"fraction_combinatorics":fraction_combinatorics(),
      "scalar_exact":scalar_exact(),"cost_model":theoretical_cost_witness()}


if __name__ == "__main__":
    try:
        result = run()
        (HERE / "RESULTS.json").write_text(json.dumps(result,indent=2)+"\n")
        print(json.dumps({"status":"PASS_DIAGNOSTIC","parallel_cases":len(result["parallel_cases"]),
            "exact_rational_values":result["fraction_combinatorics"]["asserted_boundary_round_values"],
            "scalar_cases":len(result["scalar_exact"]["cases"]),
            "projection_activations":sum(c["projection_activations"] for c in result["parallel_cases"])}))
    except Exception:
        failure = traceback.format_exc()
        first = HERE / "FIRST_FAILURE.txt"
        if not first.exists():
            first.write_text(failure)
        print(failure,file=sys.stderr)
        raise
