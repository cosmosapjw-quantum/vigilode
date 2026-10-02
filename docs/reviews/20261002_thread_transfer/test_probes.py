from fractions import Fraction as F
import pytest
from probes import path_sum_action, feasible_radius

def test_all_paths_act_on_rhs():
    h = [[F(0),F(0),F(0)], [F(2),F(0),F(0)], [F(3),F(5),F(0)]]
    assert path_sum_action(h, [F(1),F(7),F(11)]) == [F(1),F(9),F(59)]

def test_radius_grid_miss_is_repaired_by_feasible_interval():
    p = [F(29,100), F(0), F(4,5)]
    result = feasible_radius([p])
    assert result["status"] == "feasible"
    d = result["radius"]
    assert F(0) < d and p[0] + p[2]*d*d <= d

def test_radius_infeasibility_has_algebraic_certificate():
    result = feasible_radius([[F(1),F(0),F(1)]])
    assert result["status"] == "infeasible"
    assert result["certificate"]

def test_non_nilpotent_input_is_rejected():
    with pytest.raises(ValueError, match="strict-lower"):
        path_sum_action([[F(1)]], [F(1)])

import json
import math
import random
from pathlib import Path
import sympy as sp
from probes import (path_sum_matrix, peval, quadratic_component, matrix_at,
                    decode_bits, chart_output_bound)

@pytest.mark.parametrize('n',range(1,10))
def test_action_matches_independent_triangular_recurrence(n):
    rng=random.Random(800+n)
    h=[[F(rng.randrange(0,7),8) if j<i else F(0) for j in range(n)] for i in range(n)]
    a=[F(rng.randrange(1,9),16) for _ in range(n)]
    expected=[]
    for i in range(n): expected.append(a[i]+sum(h[i][j]*expected[j] for j in range(i)))
    assert path_sum_action(h,a)==path_sum_matrix(h,a)==expected

@pytest.mark.parametrize('ps',[
    [[F(-1)]], [], [[]], [[F(1), F(-1)]],
])
def test_radius_rejects_unsupported_majorants(ps):
    with pytest.raises(ValueError): feasible_radius(ps)

@pytest.mark.parametrize('ps,expected',[
    ([[F(0)]],F(0)),
    ([[F(1),F(0)]],None),
    ([[F(1,4),F(0),F(1)]],F(1,2)),
    ([[F(0),F(2)],[F(1)]],None),
])
def test_radius_edge_cases(ps,expected):
    r=feasible_radius(ps)
    if r['status']=='feasible':
        d=r['radius']; assert all(peval(p,d)<=d for p in ps)
        if expected is not None: assert d==expected
    else:
        assert r['status']=='infeasible'
        assert r['certificate']

@pytest.mark.parametrize('component',range(16))
def test_source_bound_covers_independent_midpoint_target(component):
    t=json.loads((Path(__file__).parent/'inputs/target_bits.json').read_text())
    c=quadratic_component(t,component)
    d=2*max(p[0] for p in c['constraints'])
    assert all(peval(p,d)<=d for p in c['constraints'])
    h=matrix_at(c,d)
    full=path_sum_matrix(h,c['a']); action=path_sum_action(h,c['a'])
    assert full==action==[peval(p,d) for p in c['E_polynomials']]
    assert all(abs(r-k)<=e for r,k,e in zip(c['root'],c['candidate'],action))
    assert all(abs(r-k)<=e for r,k,e in zip(c['root'],c['candidate'],c['serial']))

@pytest.mark.parametrize('n', [1,2,4,8,16])
def test_preregistered_r4_radius_schedule_and_twoB_seed(n):
    t=json.loads((Path(__file__).parent/'inputs/target_bits.json').read_text())
    cases=[quadratic_component(t,u) for u in range(n)]
    ps=[p for c in cases for p in c['constraints']]
    succeeds=[all(peval(p,F(.001)*4**k)<=F(.001)*4**k for p in ps) for k in range(6)]
    assert any(succeeds)==(n in [1,2,4])
    answer=feasible_radius(ps); assert answer['status']=='feasible'
    for endpoint in answer['safe_interval']:
        if endpoint is not None:
            radius=F(endpoint)
            assert all(peval(p,radius)<=radius for p in ps)
    d=2*max(p[0] for p in ps)
    assert all(peval(p,d)<=d for p in ps)

@pytest.mark.parametrize('which', ['numerator','denominator','coordinate'])
def test_darboux_identities(which):
    x,y,k=sp.symbols('x y k', nonzero=True)
    f=sp.Matrix([x*x,(-k+2*x)*y+x*x])
    def lie(p): return (sp.Matrix([p]).jacobian([x,y])*f)[0]
    c=k*y-x*x; d=k*x*x; w=y/x**2-1/k
    expressions={'numerator':lie(c)-(-k+2*x)*c,'denominator':lie(d)-2*x*d,'coordinate':lie(w)+k*w}
    assert sp.cancel(expressions[which])==0

@pytest.mark.parametrize('k',[F(1),F(64),F(10**6)])
def test_physical_chart_error_transport(k):
    for x in [F(1),F(3,2),F(2)]:
        for w in [F(-1,7),F(0),F(2,7)]:
            ex,ew=F(1,100),F(1,1000)
            b=chart_output_bound(x,w,k,ex,ew)
            for sx in [-1,0,1]:
                for sw in [-1,0,1]:
                    exact=(x+sx*ex)**2*(w+sw*ew+1/k)
                    predicted=x*x*(w+1/k)
                    assert abs(exact-predicted)<=b


def test_embedded_stiff_blindness_is_not_a_rodas_claim():
    z=sp.symbols('z')
    Q=1-3*z/5+3*z*z/20-z**3/60
    p5=(z*z-5*z+12)/(1440*Q)
    effect=sp.cancel(120*p5/(1-120*p5))
    expected=-5*(z*z-5*z+12)/(z*(z*z-4*z+11))
    assert sp.cancel(effect-expected)==0
    assert sp.limit(effect,z,-sp.oo)==0
    assert abs(float(effect.subs(z,-10**6)))<1/100000


def test_strict_lower_target_jacobian_has_no_fold():
    a,b,c,lam,w=sp.symbols('a b c lam w')
    residual=sp.Matrix([w*a-1,w*b-(1+lam*a*a),w*c-(1+lam*a*b+b**3)])
    assert sp.factor(residual.jacobian([a,b,c]).det())==w**3


def test_small_fixed_W_error_does_not_preserve_RVJ_stiff_stability():
    z,alpha=sp.symbols('z alpha',nonzero=True)
    r=lambda t:(1+sp.Rational(2,5)*t+t*t/20)/(1-sp.Rational(3,5)*t+3*t*t/20-t**3/60)
    t4=lambda t:sum(t**j/sp.factorial(j) for j in range(5))
    amplification=t4(z)+(r(alpha*z)-t4(alpha*z))/alpha**5
    assert sp.simplify(sp.limit(amplification/z**4,z,sp.oo)-(alpha-1)/(24*alpha))==0


def test_clock_cleanup_instruction_ceiling_is_not_native_wall_speedup():
    known_clock_fraction=F('0.05527225')+F('0.03512647')
    assert 1/(1-known_clock_fraction)<F(11,10)


def test_batched_homotopy_cost_requires_positive_overhead_margin():
    # P>=8, equal per-vector W cost, certificate/RHS overhead not free.
    p1,pf=F(7,10),F(1,5)
    cost=7-p1+8*pf
    assert 8-cost==F(1,10)
    assert 1+F(0)-8*F(1,8)==0


def test_static_stage_coordinate_change_preserves_target_and_regular_jacobian():
    a,b,c,w,lam=sp.symbols('a b c w lam')
    z1,z2,z3=sp.symbols('z1 z2 z3')
    psi=sp.Matrix([z1,z2+z1**2,z3+z1*z2])
    inverse=sp.Matrix([a,b-a*a,c-a*(b-a*a)])
    composed=inverse.subs(dict(zip([a,b,c],psi)),simultaneous=True)
    assert all(sp.expand(v)==0 for v in composed-sp.Matrix([z1,z2,z3]))
    residual=sp.Matrix([w*a-1,w*b-(1+lam*a*a),w*c-(1+lam*a*b+b**3)])
    changed=residual.subs(dict(zip([a,b,c],psi)),simultaneous=True)
    assert sp.expand(changed.jacobian([z1,z2,z3]).det()-w**3)==0
