#!/usr/bin/env python3
"""Independent rational oracle for the emitted dyadic linear-curve fixture.

No third-party runtime. The ODE exact solution sin(t) is used only here, outside
production code. Alternating rational series give strict enclosing endpoints.
"""
import json
import sys
from fractions import Fraction as F
from math import factorial
from pathlib import Path


def sin_box(x):
    s = sum(((-1)**k*x**(2*k+1)/factorial(2*k+1) for k in range(24)), F())
    return s, s+x**49/factorial(49)


def cos_box(x):
    c = sum(((-1)**k*x**(2*k)/factorial(2*k) for k in range(25)), F())
    return c-x**50/factorial(50), c


log = Path(sys.argv[1]).read_text()
line = next(s.split('PR101_ORACLE=', 1)[1] for s in log.splitlines() if 'PR101_ORACLE=' in s)
data = json.loads(line)
h, radius, residual = map(lambda k: F.from_float(data[k]), ('h', 'radius', 'residual'))
lam, eta = F(data['lambda']), F(data['eta'])
checks = 0
for k in range(65):
    t = h*k/64
    slo, shi = sin_box(t)
    clo, chi = cos_box(t)
    zlo, zhi = t-shi, t-slo
    # z>=0 follows from sin(t)<=t; intersect with that exact inequality.
    zlo=max(zlo,F())
    rlo = 1-chi-lam*zlo-eta*zhi*zhi
    rhi = 1-clo-lam*zhi-eta*zlo*zlo
    assert max(abs(rlo), abs(rhi)) <= residual
    assert zhi <= radius
    checks += 2
# q-sin increases on [0,h] since derivative is 1-cos>=0. Therefore this
# endpoint rational enclosure also validates the entire exact output curve.
endpoint_error = h-sin_box(h)[0]
assert endpoint_error <= radius
print(json.dumps({
    'schema': 'vigilode-pr101-fraction-oracle-v1',
    'status': 'pass_scoped',
    'rational_checks': checks+1,
    'fixture': 'q(t)=t, h=2^-10, y(0)=0, lambda=-10^6, eta=10',
    'whole_interval_error_proof': 'q-sin monotone because 1-cos>=0; rational endpoint enclosure',
    'sampled_residual_checks': 65,
    'endpoint_error_upper_float': float(endpoint_error),
    'verified_radius': float(radius),
    'verified_residual': float(residual),
}, indent=2))
