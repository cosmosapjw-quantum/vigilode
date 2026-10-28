#!/usr/bin/env python3
"""Integer exponent overapproximation of fixed F6 slab arithmetic; see F6_FAILURE_RANGE.md."""
mx = 0

def track(k):
    global mx
    mx = max(mx, k)
    return k

def add(a, b): return track(max(a, b) + 2)
def mul(a, b): return track(a + b + 1)
def scale(a, c): return mul(a, c)
def div(a, lower_exp): return track(a - lower_exp + 1)
def ref(t):
    t2 = mul(t, t); t4 = mul(t2, t2); t5 = mul(t4, t)
    return (add(add(0, scale(t, 0)), scale(t5, 0)), add(0, scale(t4, 0)))
def fld(a, b):
    a2 = mul(a, a); s2 = scale(a, 2); s4 = scale(add(a2, b), 2)
    a3 = mul(a2, a); ab = mul(a, b)
    s6 = add(add(scale(a3, 2), scale(ab, 4)), 2)
    return (add(add(add(a, scale(s2, 0)), scale(s4, 0)), scale(s6, 0)),
            add(add(add(b, scale(s2, 0)), scale(s4, 0)), scale(s6, 0)))
p, dp = ref(0); g = fld(0, 0); gp = fld(p, p)
sl = [add(add(g[i], dp), gp[i]) for i in range(2)]
print('reference_bound_exponents', p, dp, 'slopes', sl, 'max_so_far', mx)
def pc(k): return [k]
def pa(a, b):
    return [add(a[i] if i < len(a) else 0, b[i] if i < len(b) else 0) for i in range(max(len(a), len(b)))]
def ps(a, c): return [scale(v, c) for v in a]
def pm(a, b):
    out = [0] * (len(a) + len(b) - 1)
    for i, x in enumerate(a):
        for j, y in enumerate(b): out[i+j] = add(out[i+j], mul(x, y))
    return out
def pf(a, b):
    a2 = pm(a, a); a3 = pm(a2, a); ab = pm(a, b)
    s2 = ps(a, 2); s4 = ps(pa(a2, b), 2)
    s6 = pa(pa(ps(a3, 2), ps(ab, 4)), pc(2))
    return (pa(pa(pa(a, ps(s2, 0)), ps(s4, 0)), ps(s6, 0)),
            pa(pa(pa(b, ps(s2, 0)), ps(s4, 0)), ps(s6, 0)))
s = max(sl)
path = [0, scale(s, 0), add(scale(add(0, 0), 2), scale(add(scale(s, 1), s), 0)), add(scale(add(0, 0), 1), scale(add(s, s), 0))]
t = [0, 0]; t2 = pm(t, t); t4 = pm(t2, t2); t5 = pm(t4, t)
p = pa(pa(pc(0), ps(t, 0)), ps(t5, 0)); dp = pa(pc(0), ps(t4, 0))
g = pf(path, path); gp = pf(p, p)
der = [div(scale(path[k], 2), -7) for k in range(1, 4)]
r = [pa(der, pa(pa(g[i], dp), gp[i])) for i in range(2)]
ratio = div(13, 0)
def bern(poly, degree):
    out = []
    for i in range(degree + 1):
        v = 0
        for k in range(min(i, len(poly)-1) + 1): v = add(v, mul(poly[k], ratio))
        out.append(v)
    return out
rb = [bern(v, 15) for v in r]; pb = bern(path, 3)
print('path_coefficient_max_exponent', max(path), 'residual_power_max_exponent', max(map(max, r)), 'residual_bernstein_max_exponent', max(map(max, rb)), 'path_bernstein_max_exponent', max(pb), 'all_arithmetic_max_exponent', mx)
delta = max(map(max, rb)); d = div(delta, -1); ans = add(mul(0, 0), mul(0, d))
print('propagation_D_exponent', d, 'endpoint_upper_exponent', ans, 'complete_scalar_max_exponent', mx)
assert (p is not None and max(path) == 42 and max(map(max, r)) == 154 and delta == 189 and max(pb) == 61 and d == 191 and ans == 194 and mx == 194)
