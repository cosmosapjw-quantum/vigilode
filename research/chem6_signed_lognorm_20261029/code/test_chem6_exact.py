from fractions import Fraction as F
import random
import sympy as sp
import pytest
from chem6_exact import (
    N, NR, TH, vertices, jac_basis, full_jac, mu_weighted,
    sharp_mu, entrywise_upper, full_corner_mu, rational_error_radius,
    default_fixture,
)


def test_vertex_count_and_physical_domain():
    vs = list(vertices())
    assert len(vs) == 24 and len(set(vs)) == 24
    for h, u, v, l1, l2, l3 in vs:
        assert 0 <= h <= TH and 0 <= u and 0 <= v and u + v <= 1
        assert 0 <= l1 and 0 <= l2 and 0 <= l3 and l1 + l2 + l3 <= 1


def test_symbolic_source_flux_jacobian_independent():
    # Independently differentiate the six original transition fluxes.
    z = sp.symbols('h u v l1 l2 l3')
    wh, whe, wli = sp.symbols('wh whe wli')
    E = wh*z[0] + whe*(z[1] + 2*z[2]) + wli*(z[3] + 2*z[4] + 3*z[5])
    p = sp.symbols('ph phe0 phe1 pl0 pl1 pl2')
    r = sp.symbols('rh rhe1 rhe2 rl1 rl2 rl3')
    flux = sp.Matrix([
        p[0]*(sp.Rational(2**53-1, 2**53)-z[0])-E*r[0]*z[0],
        p[1]*(1-z[1]-z[2])-p[2]*z[1]+E*(-r[1]*z[1]+r[2]*z[2]),
        p[2]*z[1]-E*r[2]*z[2],
        p[3]*(1-z[3]-z[4]-z[5])-p[4]*z[3]+E*(-r[3]*z[3]+r[4]*z[4]),
        p[4]*z[3]-p[5]*z[4]+E*(-r[4]*z[4]+r[5]*z[5]),
        p[5]*z[4]-E*r[5]*z[5],
    ])
    jac = flux.jacobian(z)
    substitutions = {z[i]: sp.Rational(1, i+17) for i in range(6)}
    substitutions.update({wh: sp.Rational(1,8), whe: sp.Rational(1,8), wli: sp.Rational(1,8)})
    e = (F(1,8),F(1,8),F(1,4),F(1,8),F(1,4),F(3,8))
    zv = tuple(F(1,i+17) for i in range(6))
    for a, rate in enumerate((*p, *r)):
        direct = jac.diff(rate).subs(substitutions)
        from_basis = jac_basis(zv, e, a)
        for i in range(6):
            for j in range(6):
                assert F(direct[i,j]) == from_basis[i][j]


def test_exact_sharp_witness_and_correlation_gain():
    e, w, bounds = default_fixture()
    best = sharp_mu(e, w, bounds)
    assert mu_weighted(full_jac(best.state, e, best.rate_endpoints), w) == best.value
    naive = entrywise_upper(e, w, bounds)
    assert best.value < naive  # this declared fixture has strict correlation gain
    assert best.value == F(378302368699121663,2305843009213693952)
    assert naive == F(49,256)


def test_full_corner_independent_for_four_variable_rates():
    e, w, bounds = default_fixture()
    rates = [(lo,lo) for lo,hi in bounds]
    for a in (0,3,7,11):
        rates[a] = bounds[a]
    assert sharp_mu(e,w,rates).value == full_corner_mu(e,w,rates)


def test_interior_random_samples_never_exceed_sharp_bound():
    rng = random.Random(20261029)
    e,w,bounds=default_fixture()
    best=sharp_mu(e,w,bounds).value
    for _ in range(300):
        # Convex combination of two physical product vertices remains in S.
        va,vb=rng.sample(list(vertices()),2)
        t=F(rng.randrange(257),256)
        z=tuple(t*a+(1-t)*b for a,b in zip(va,vb))
        rr=tuple(lo+F(rng.randrange(257),256)*(hi-lo) for lo,hi in bounds)
        assert mu_weighted(full_jac(z,e,rr),w) <= best


def test_feedback_omission_is_detected():
    e,w,bounds=default_fixture()
    z=tuple(F(1,20) for _ in range(6))
    r=tuple(lo for lo,hi in bounds)
    j=full_jac(z,e,r)
    # With a nonzero recombination rate, J_{0,1}=(Qz)_0 e_1 is nonzero.
    assert j[0][1] == -r[6]*z[0]*e[1]
    assert j[0][1] != 0


def test_dini_rational_gate_and_event_dual_norm():
    e,w,bounds=default_fixture()
    mu=sharp_mu(e,w,bounds).value
    R=rational_error_radius(mu,F(1,32),F(1,4096),F(1,65536))
    assert R is not None and R>0
    assert R >= F(1,65536)+F(1,32)*F(1,4096)
    assert rational_error_radius(F(2),F(1,2),F(1,100),F(0)) is None
    # |e.z| <= sum |e_i| w_i ||z||_w
    assert sum(ei*wi for ei,wi in zip(e,w)) == F(5,4)


def test_invalid_boxes_and_weights_refuse():
    e,w,bounds=default_fixture()
    with pytest.raises(AssertionError):
        sharp_mu(e,(F(0),*w[1:]),bounds)
    with pytest.raises(AssertionError):
        sharp_mu(e,w,((F(-1),F(1)),*bounds[1:]))
    with pytest.raises(AssertionError):
        sharp_mu(e,w,((F(2),F(1)),*bounds[1:]))