"""New exact rational diagnostics for conditional theory, not native benchmarks.

Frozen before first execution: polynomial branch identity in lambda; eight-stage
nilpotent majorant; a four-stage affine invariant model at three parameter points.
No existing VigilODE experiment is re-run and these checks do not prove the general
theorems (their proofs are in HOMOTOPY.md).
"""
from fractions import Fraction as Q
import json
from pathlib import Path


def padd(a, b):
    out = [Q(0)] * max(len(a), len(b))
    for p in (a, b):
        for i, x in enumerate(p):
            out[i] += x
    while len(out) > 1 and out[-1] == 0:
        out.pop()
    return out


def pscale(a, c):
    return [c * x for x in a]


def pmul(a, b):
    out = [Q(0)] * (len(a) + len(b) - 1)
    for i, x in enumerate(a):
        for j, y in enumerate(b):
            out[i + j] += x * y
    return out


def mv(a, x):
    return [sum((c * v for c, v in zip(row, x)), Q(0)) for row in a]


def mm(a, b):
    return [[sum((a[i][k] * b[k][j] for k in range(len(b))), Q(0))
             for j in range(len(b[0]))] for i in range(len(a))]


def add(a, b):
    return [x + y for x, y in zip(a, b)]


def scale(a, s):
    return [s * x for x in a]


def solve(a, b):
    m = [row[:] + [x] for row, x in zip(a, b)]
    n = len(b)
    for col in range(n):
        pivot = next(i for i in range(col, n) if m[i][col])
        m[col], m[pivot] = m[pivot], m[col]
        v = m[col][col]
        m[col] = [x / v for x in m[col]]
        for i in range(n):
            if i != col:
                v = m[i][col]
                m[i] = [x - v * y for x, y in zip(m[i], m[col])]
    return [row[-1] for row in m]


def require(condition, label):
    if not condition:
        raise AssertionError(label)


# H1: coefficient identity for a fixed rational instance, all lambda.
a, b, c, d, e = Q(2, 3), Q(-1, 5), Q(7, 4), Q(9, 8), Q(-3, 2)
lam = [Q(0), Q(1)]
k1 = [a]
k2 = [b, c * a * a]
k3 = [d, e * b * b, 2 * e * b * c * a * a, e * c * c * a**4]
r2 = padd(padd(k2, [-b]), pscale(pmul(lam, pmul(k1, k1)), -c))
r3 = padd(padd(k3, [-d]), pscale(pmul(lam, pmul(k2, k2)), -e))
require(all(x == 0 for x in r2 + r3), 'polynomial residual coefficients')

# H2: exact roundoff majorant and non-contraction nilpotence.
s = 8
B = [[Q(1, 4) if i == j + 1 else Q(0) for j in range(s)] for i in range(s)]
err = [Q(32)] * s
for _ in range(3):
    err = add(mv(B, err), [Q(3, 8)] * s)
require(max(err) <= 1, 'three-round epsilon guarantee')
B10 = [[Q(10) if i == j + 1 else Q(0) for j in range(s)] for i in range(s)]
power = [[Q(i == j) for j in range(s)] for i in range(s)]
for _ in range(s):
    power = mm(power, B10)
require(all(x == 0 for row in power for x in row), 'nilpotence without contraction')

# H4: genuine nonlinear g and stiff transverse dynamics, exact full stage root.
U = [Q(1), Q(3), Q(-2)]
z0, h, gamma = Q(1, 4), Q(1, 16), Q(1, 2)
y0 = scale(U, z0)
gp = -2 + 2 * z0
J = [[gp, Q(0), Q(0)], [3 * gp + 21, Q(-7), Q(0)],
     [-2 * gp - 22, Q(0), Q(-11)]]
require(mv(J, U) == scale(U, gp), 'exact current J invariance')
W = [[Q(i == j) - h * gamma * J[i][j] for j in range(3)] for i in range(3)]
wr = 1 - h * gamma * gp
alpha = [[Q(0)] * 4 for _ in range(4)]
alpha[1][0] = Q(1, 2)
alpha[2][1] = Q(1, 3)
alpha[3][0], alpha[3][2] = Q(1, 4), Q(1, 2)
L = [[Q(i + j + 1, 10) if j < i else Q(0) for j in range(4)] for i in range(4)]


def g(z):
    return -2 * z + z * z


def f(v):
    x, y, z = v
    return [g(x), 3 * g(x) - 7 * (y - 3 * x),
            -2 * g(x) - 11 * (z + 2 * x)]


f0 = f(y0)
points = []
for parameter in (Q(0), Q(1, 2), Q(1)):
    theta = Q(1, 3)
    eta = theta + parameter * (1 - theta)
    full, reduced = [], []
    for i in range(4):
        delta = [sum((alpha[i][j] * full[j][k] for j in range(i)), Q(0)) for k in range(3)]
        delta_r = sum((alpha[i][j] * reduced[j] for j in range(i)), Q(0))
        nval = add(add(f(add(y0, delta)), scale(f0, -1)), scale(mv(J, delta), -1))
        nval_r = g(z0 + delta_r) - g(z0) - gp * delta_r
        mix = [sum((L[i][j] * full[j][k] for j in range(i)), Q(0)) for k in range(3)]
        mix_r = sum((L[i][j] * reduced[j] for j in range(i)), Q(0))
        rhs = add(add(scale(f0, h), scale(mv(J, mix), eta * h)), scale(nval, parameter * h))
        rhs_r = h * g(z0) + eta * h * gp * mix_r + parameter * h * nval_r
        full.append(solve(W, rhs))
        reduced.append(rhs_r / wr)
        require(full[-1] == scale(U, reduced[-1]), 'full-reduced stage equality')
    points.append({'lambda': str(parameter), 'theta': str(theta), 'stages': 4,
                   'component_equalities': 12, 'status': 'PASS'})

report = {
    'status': 'PASS',
    'arithmetic': 'Python Fraction exact rational; independent Gaussian elimination for full 3x3 W',
    'scope': 'new small theory diagnostics, not native VigilODE fixtures, speed, or general theorem proof',
    'H1_polynomial_identity': {'status': 'PASS', 'lambda_degree': 3,
                                'all_residual_coefficients_zero': True},
    'H2_rounding_majorant': {'status': 'PASS', 's': 8, 'q': '1/4',
                            'initial_error': '32', 'round_error': '3/8',
                            'rounds': 3, 'maximum_final_error': str(max(err)), 'budget': '1'},
    'H2_noncontracting_nilpotence': {'status': 'PASS', 'infinity_norm': '10', 'power': 8},
    'H4_invariant_stage_equality': {'status': 'PASS', 'full_dimension': 3,
                                   'reduced_dimension': 1, 'points': points},
    'native_execution': False, 'published_experiment_reruns': 0,
}
destination = Path(__file__).with_name('HOMOTOPY_EXACT_CHECK.json')
destination.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
