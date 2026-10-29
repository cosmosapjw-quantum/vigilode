#!/usr/bin/env python3
"""Prospective exact-rational checks; execute only after source registration.

These checks independently solve small full-space stage targets. They do not
exercise a Rust implementation, floating-point enclosure or runtime performance.
"""

from fractions import Fraction as F
import json


def trim(p):
    p = list(p)
    while len(p) > 1 and p[-1] == 0:
        p.pop()
    return p or [F(0)]


def padd(p, q):
    r = [F(0)] * max(len(p), len(q))
    for i, value in enumerate(p):
        r[i] += value
    for i, value in enumerate(q):
        r[i] += value
    return trim(r)


def pscale(p, factor):
    return trim([factor * value for value in p])


def psub(p, q):
    return padd(p, pscale(q, -1))


def pmul(p, q):
    r = [F(0)] * (len(p) + len(q) - 1)
    for i, pi in enumerate(p):
        for j, qj in enumerate(q):
            r[i + j] += pi * qj
    return trim(r)


def pdivmod(p, q):
    p, q = trim(p), trim(q)
    assert q != [0]
    if len(p) < len(q):
        return [F(0)], p
    out = [F(0)] * (len(p) - len(q) + 1)
    while p != [0] and len(p) >= len(q):
        shift = len(p) - len(q)
        factor = p[-1] / q[-1]
        out[shift] += factor
        p = psub(p, [F(0)] * shift + pscale(q, factor))
    return trim(out), trim(p)


def peval(p, x):
    out = F(0)
    for value in reversed(p):
        out = out * x + value
    return out


def ident(n):
    return [[F(i == j) for j in range(n)] for i in range(n)]


def madd(a, b):
    return [[x + y for x, y in zip(ar, br)] for ar, br in zip(a, b)]


def mscale(a, k):
    return [[k * x for x in row] for row in a]


def mmul(a, b):
    return [
        [sum((a[i][k] * b[k][j] for k in range(len(b))), F(0))
         for j in range(len(b[0]))]
        for i in range(len(a))
    ]


def mv(a, v):
    return [sum((x * y for x, y in zip(row, v)), F(0)) for row in a]


def va(a, b):
    return [x + y for x, y in zip(a, b)]


def vs(v, k):
    return [k * x for x in v]


def vn(v):
    return max(abs(x) for x in v)


def mn(a):
    return max(sum(abs(x) for x in row) for row in a)


def solve(a, rhs):
    n = len(a)
    rows = [list(row) + [rhs[i]] for i, row in enumerate(a)]
    for j in range(n):
        pivot = next(i for i in range(j, n) if rows[i][j] != 0)
        rows[j], rows[pivot] = rows[pivot], rows[j]
        denominator = rows[j][j]
        rows[j] = [x / denominator for x in rows[j]]
        for i in range(n):
            if i == j:
                continue
            factor = rows[i][j]
            rows[i] = [x - factor * y for x, y in zip(rows[i], rows[j])]
    return [row[-1] for row in rows]


def inverse(a):
    n = len(a)
    columns = [solve(a, [F(i == j) for i in range(n)]) for j in range(n)]
    return [[columns[j][i] for j in range(n)] for i in range(n)]


def paction(p, c, f):
    result = [F(0)] * len(f)
    for coefficient in reversed(p):
        result = va(mv(c, result), vs(f, coefficient))
    return result


def pmatrix(p, c):
    n = len(c)
    columns = [paction(p, c, [F(i == j) for i in range(n)]) for j in range(n)]
    return [[columns[j][i] for j in range(n)] for i in range(n)]


def coeff_bound(p, radius):
    return sum((abs(value) * radius**i for i, value in enumerate(p)), F(0))


def tableau(stages):
    a = [[F(0)] * stages for _ in range(stages)]
    c = [[F(0)] * stages for _ in range(stages)]
    for i in range(stages):
        for j in range(i):
            a[i][j] = F((-1)**(i + j) * (i + j + 1), 20 * (i + 1) * (j + 1))
            c[i][j] = F((-1)**(i - j) * (j + 1), 15 * (i + 1))
    beta = [F(i + 1, stages * (stages + 1) // 2) for i in range(stages)]
    return a, c, beta


def compile_stages(pi, center, scale, gamma, a, c):
    zeta = (1 - center) / scale
    denominator = peval(pi, zeta)
    if denominator == 0:
        raise ValueError("zero quotient denominator")
    numerator = psub([denominator], pi)
    v, remainder = pdivmod(numerator, [zeta, F(-1)])
    assert remainder == [0]
    v = pscale(v, 1 / (scale * denominator))
    z = [center, scale]
    w = [1 - center, -scale]
    assert psub(pmul(w, v), psub([F(1)], pscale(pi, 1 / denominator))) == [0]
    r = []
    for i in range(len(a)):
        rhs = [F(1)]
        for j in range(i):
            bij = padd(pscale(z, a[i][j]), [gamma * c[i][j]])
            rhs = padd(rhs, pmul(bij, r[j]))
        _, ri = pdivmod(pmul(v, rhs), pi)
        r.append(ri)
    return r


def weighted(stages, weights):
    out = [F(0)] * len(stages[0])
    for stage, weight in zip(stages, weights):
        out = va(out, vs(stage, weight))
    return out


def original_stages(w, j0, f0, tau, gamma, a, c, nonlinear):
    stages, arguments = [], []
    for i in range(len(a)):
        arg = [F(0)] * len(f0)
        cmix = [F(0)] * len(f0)
        for j in range(i):
            arg = va(arg, vs(stages[j], a[i][j]))
            cmix = va(cmix, vs(stages[j], c[i][j]))
        fy = va(va(f0, mv(j0, arg)), [nonlinear * x * x for x in arg])
        stages.append(solve(w, va(vs(fy, tau), vs(cmix, gamma))))
        arguments.append(arg)
    return stages, arguments


def compiler_case(name, cmat, pi, stages, nonlinear=F(0), coefficient_error=False,
                  input_error=False, require_nonzero=False):
    n = len(cmat)
    center, scale, gamma, tau = F(-1, 4), F(1, 2), F(1, 4), F(1, 20)
    a, c, beta = tableau(stages)
    zmat = madd(mscale(ident(n), center), mscale(cmat, scale))
    w = madd(ident(n), mscale(zmat, -1))
    j0 = mscale(zmat, 1 / tau)
    r = compile_stages(pi, center, scale, gamma, a, c)
    if coefficient_error:
        r[min(1, stages - 1)] = padd(r[min(1, stages - 1)], [F(1, 10000)])
    f0 = [F((-1)**i * (i + 1), 1) for i in range(n)]
    supplied = list(f0)
    if input_error:
        supplied[0] += F(1, 1000)
    eps0 = vn(va(supplied, vs(f0, -1)))
    virtual = [vs(paction(ri, cmat, supplied), tau) for ri in r]
    target, target_arguments = original_stages(w, j0, f0, tau, gamma, a, c, nonlinear)
    pi_c = pmatrix(pi, cmat)
    rho, radius = mn(pi_c), mn(cmat)
    pi_f = mv(pi_c, supplied)
    omega = mn(inverse(w))
    local_bounds, actual_residuals, virtual_arguments, remainder_bounds = [], [], [], []
    alpha_values = []
    for i in range(stages):
        si = psub(pmul([1 - center, -scale], r[i]), [F(1)])
        arg = [F(0)] * n
        cmix = [F(0)] * n
        for j in range(i):
            bij = padd(pscale([center, scale], a[i][j]), [gamma * c[i][j]])
            si = psub(si, pmul(bij, r[j]))
            arg = va(arg, vs(virtual[j], a[i][j]))
            cmix = va(cmix, vs(virtual[j], c[i][j]))
        quotient, remainder = pdivmod(si, pi)
        assert len(quotient) == 1
        alpha = quotient[0]
        alpha_values.append(alpha)
        leading_ri = r[i][len(pi) - 2] if len(r[i]) >= len(pi) - 1 else F(0)
        leading_sum = leading_ri
        for j in range(i):
            leading_rj = r[j][len(pi) - 2] if len(r[j]) >= len(pi) - 1 else F(0)
            leading_sum += a[i][j] * leading_rj
        assert alpha == -scale * leading_sum / pi[-1]
        if not coefficient_error:
            assert remainder == [0]
        nonlinear_defect = [nonlinear * x * x for x in arg]
        fy = va(va(f0, mv(j0, arg)), nonlinear_defect)
        residual = va(mv(w, virtual[i]), vs(va(vs(fy, tau), vs(cmix, gamma)), -1))
        rhs_identity = va(vs(pi_f, alpha), paction(remainder, cmat, supplied))
        rhs_identity = va(rhs_identity, va(supplied, vs(f0, -1)))
        rhs_identity = va(rhs_identity, vs(nonlinear_defect, -1))
        rhs_identity = vs(rhs_identity, tau)
        assert residual == rhs_identity
        mu = nonlinear * vn(arg)**2
        local_bound = abs(tau) * ((abs(alpha) * rho + coeff_bound(remainder, radius))
                                 * vn(supplied) + eps0 + mu)
        assert vn(residual) <= local_bound
        radius_bound = abs(tau) * sum(
            (abs(a[i][j]) * coeff_bound(r[j], radius) * vn(supplied)
             for j in range(i)), F(0))
        assert vn(arg) <= radius_bound
        local_bounds.append(local_bound)
        actual_residuals.append(residual)
        virtual_arguments.append(arg)
        remainder_bounds.append(coeff_bound(remainder, radius))
    domain_radius = F(2)
    assert all(vn(arg) <= domain_radius for arg in virtual_arguments + target_arguments)
    lipschitz = mn(j0) + 2 * nonlinear * domain_radius
    k_matrix = [[F(0)] * stages for _ in range(stages)]
    for i in range(stages):
        for j in range(i):
            k_matrix[i][j] = omega * (abs(tau) * lipschitz * abs(a[i][j])
                                    + abs(gamma * c[i][j]))
    transfer, power = ident(stages), ident(stages)
    for _ in range(1, stages):
        power = mmul(power, k_matrix)
        transfer = madd(transfer, power)
    assert mmul(power, k_matrix) == [[F(0)] * stages for _ in range(stages)]
    error_bounds = vs(mv(transfer, local_bounds), omega)
    for vi, ui, bound in zip(virtual, target, error_bounds):
        assert vn(va(vi, vs(ui, -1))) <= bound
    p_beta = [F(0)]
    for weight, ri in zip(beta, r):
        p_beta = padd(p_beta, pscale(ri, weight))
    compiled = vs(paction(p_beta, cmat, supplied), tau)
    assert compiled == weighted(virtual, beta)
    endpoint_error = vn(va(compiled, vs(weighted(target, beta), -1)))
    endpoint_bound = sum((abs(weight) * error for weight, error in zip(beta, error_bounds)), F(0))
    assert endpoint_error <= endpoint_bound
    correlated_checked = False
    if not nonlinear and not coefficient_error and not input_error:
        # B14 on the exactly diagonal/self-adjoint fixtures, independently
        # solving the scalar stage transfer at each actual eigenvalue.
        if all(cmat[i][j] == 0 for i in range(n) for j in range(n) if i != j):
            signed_predictions, signed_factors = [], []
            for component in range(n):
                eigenvalue = cmat[component][component]
                z_value = center + scale * eigenvalue
                scalar_l = [[F(0)] * stages for _ in range(stages)]
                for i in range(stages):
                    scalar_l[i][i] = 1 - z_value
                    for j in range(i):
                        scalar_l[i][j] = -(a[i][j] * z_value + gamma * c[i][j])
                transfer_alpha = solve(scalar_l, alpha_values)
                signed_factor = sum((weight * value for weight, value in zip(beta, transfer_alpha)), F(0))
                signed_predictions.append(tau * signed_factor * peval(pi, eigenvalue) * f0[component])
                signed_factors.append(signed_factor)
            endpoint_difference = va(compiled, vs(weighted(target, beta), -1))
            assert endpoint_difference == signed_predictions
            squared_bound = tau**2 * rho**2 * max(abs(x) for x in signed_factors)**2 * sum(x*x for x in f0)
            assert sum(x*x for x in endpoint_difference) <= squared_bound
            correlated_checked = True
    if rho == 0 and not nonlinear and not coefficient_error and not input_error:
        assert virtual == target
        assert endpoint_error == endpoint_bound == 0
    if require_nonzero:
        assert endpoint_error > 0
        assert any(vn(residual) > 0 for residual in actual_residuals)
    return {"name": name, "status": "PASS", "dimension": n, "stages": stages,
            "annihilator_degree": len(pi) - 1, "relation_defect": str(rho),
            "endpoint_error": str(endpoint_error), "endpoint_bound": str(endpoint_bound),
            "nonlinear_remainder_charged": bool(nonlinear),
            "coefficient_defect_charged": bool(coefficient_error),
            "input_defect_charged": bool(input_error),
            "signed_correlated_defect_checked": correlated_checked}


def refinement_checks():
    theta, eta, zeta, upsilon = F(1, 4), F(1, 10000), F(1, 20000), F(1, 20000)
    kappa = (1 + theta) * eta + zeta + upsilon
    floor = kappa / (1 - theta)
    assert kappa == F(9, 40000) and floor == F(3, 10000)
    epsilon, r0 = F(1, 1000), F(1)
    bounds = [theta**m * r0 + floor * (1 - theta**m) for m in range(21)]
    minimum = next(m for m, bound in enumerate(bounds) if bound <= epsilon)
    assert minimum == 6
    assert theta**minimum <= (epsilon - floor) / (r0 - floor)
    assert theta**(minimum - 1) > (epsilon - floor) / (r0 - floor)
    assert all(bound > floor for bound in bounds)
    # Boundary: theta=0 reaches the uniform floor in one correction.
    eta_zero, zeta_zero, update_zero = F(1, 2000), F(1, 4000), F(1, 4000)
    floor_zero = eta_zero + zeta_zero + update_zero
    initial_zero = F(-1)
    represented_residual_zero = -initial_zero + eta_zero
    corrected_zero = initial_zero + represented_residual_zero + zeta_zero + update_zero
    assert abs(corrected_zero) == floor_zero == F(1, 1000)
    assert abs(initial_zero) > floor_zero
    # A=1, P=1+theta makes D=-theta. Signed defects attain C2 exactly.
    f, x, p = F(0), F(-1), 1 + theta
    for k in range(1, 13):
        residual = f - x
        sign = F(1 if residual > 0 else -1)
        supplied = residual + sign * eta
        correction = p * supplied + sign * zeta
        x = x + correction + sign * upsilon
        assert abs(f - x) == bounds[k]
    # Noncommuting A and P; only the right defect I-AP is needed.
    a = [[F(2), F(1)], [F(0), F(1)]]
    p = [[F(1, 2), F(-1, 2)], [F(1, 20), F(1)]]
    ap = mmul(a, p)
    assert ap != mmul(p, a)
    defect = madd(ident(2), mscale(ap, -1))
    theta_matrix = mn(defect)
    assert theta_matrix == F(1, 20)
    f, x = [F(1), F(-2)], [F(1, 3), F(-1, 4)]
    residual = va(f, vs(mv(a, x), -1))
    e = [F(1, 100), F(-1, 200)]
    z = [F(1, 300), F(-1, 400)]
    u = [F(-1, 500), F(1, 600)]
    updated = va(va(x, va(mv(p, va(residual, e)), z)), u)
    next_residual = va(f, vs(mv(a, updated), -1))
    identity = va(mv(defect, residual), vs(va(mv(ap, e), mv(a, va(z, u))), -1))
    assert identity == next_residual
    next_bound = theta_matrix * vn(residual) + (1 + theta_matrix) * vn(e) + mn(a) * (vn(z) + vn(u))
    assert vn(next_residual) <= next_bound
    # Varying, independently certified standalone correction plans: C5.
    thetas = [F(1, 2), F(1, 3), F(1, 4)]
    eta_v, zeta_v, upsilon_v = F(1, 100), F(1, 500), F(1, 1000)
    kappas = [(1 + t) * eta_v + zeta_v + upsilon_v for t in thetas]
    f, x = F(0), F(-2)
    recursive = F(2)
    for t, kappa_v in zip(thetas, kappas):
        residual = f - x
        sign = F(1 if residual > 0 else -1)
        x += (1 + t) * (residual + sign * eta_v) + sign * (zeta_v + upsilon_v)
        recursive = t * recursive + kappa_v
        assert abs(f - x) == recursive
    product = F(1)
    for t in thetas:
        product *= t
    unrolled = product * 2
    for k, kappa_v in enumerate(kappas):
        future = F(1)
        for t in thetas[k + 1:]:
            future *= t
        unrolled += future * kappa_v
    assert unrolled == recursive
    return {"status": "PASS", "uniform_floor": str(floor),
            "minimum_corrections": minimum, "attaining_steps": 12,
            "noncommuting_right_defect": str(theta_matrix),
            "variable_plan_identity": True,
            "zero_contraction_floor_equality": str(floor_zero)}


def main():
    zero = F(0)
    nilpotent = [[zero, F(1)], [zero, zero]]
    pi_nilpotent = [zero, zero, F(1)]
    delta = F(1, 10)
    near = [[delta, F(1)], [zero, -delta]]
    diagonal = [[F(-1, 2), zero, zero], [zero, zero, zero], [zero, zero, F(3, 4)]]
    pi_diagonal = pmul(pmul([F(1, 2), F(1)], [zero, F(1)]), [F(-3, 4), F(1)])
    cases = [
        compiler_case("scalar_exact", [[F(-2, 3)]], [F(2, 3), F(1)], 3),
        compiler_case("nonnormal_nilpotent_exact_eight_stages", nilpotent, pi_nilpotent, 8),
        compiler_case("three_channel_exact", diagonal, pi_diagonal, 3),
        compiler_case("near_annihilator", near, pi_nilpotent, 3, require_nonzero=True),
        compiler_case("self_adjoint_signed_near_defect", [[delta, zero], [zero, -delta]],
                      pi_nilpotent, 3, require_nonzero=True),
        compiler_case("zero_structure_budget_counterexample", near, pi_nilpotent, 1, require_nonzero=True),
        compiler_case("stored_coefficient_defect", nilpotent, pi_nilpotent, 3, coefficient_error=True,
                      require_nonzero=True),
        compiler_case("near_affine_original_target", nilpotent, pi_nilpotent, 3, nonlinear=F(1, 25),
                      require_nonzero=True),
        compiler_case("base_rhs_representation_defect", nilpotent, pi_nilpotent, 3, input_error=True,
                      require_nonzero=True),
    ]
    a, c, _ = tableau(3)
    try:
        compile_stages([F(-5, 2), F(1)], F(-1, 4), F(1, 2), F(1, 4), a, c)
    except ValueError as error:
        assert str(error) == "zero quotient denominator"
    else:
        raise AssertionError("zero denominator accepted")
    # The same rejected polynomial plan says nothing about an unrelated W:
    # C=0 gives W=5/4, whose ordinary scalar system remains solvable.
    assert solve([[F(5, 4)]], [F(1)]) == [F(4, 5)]
    result = {"status": "PASS", "scope": "exact finite algebra identities and bounds",
              "compiler_cases": cases, "compiler_case_count": len(cases),
              "denominator_refusal": "PASS", "refinement": refinement_checks(),
              "native_solver_tests": "NOT_RUN", "native_performance": "NOT_RUN"}
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
