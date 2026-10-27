#!/usr/bin/env python3
"""Independent rational checks; no native solver or timing claims.

The tableau rationals are exact values of the represented binary64 literals.
Proofs remain in THEORY.md; these finite checks audit algebra and indexing.
"""
from fractions import Fraction as Q
from pathlib import Path
import hashlib
import json
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def matrix_product(a, b):
    return [[sum((x*y for x, y in zip(row, col)), Q(0))
             for col in zip(*b)] for row in a]


def matvec(a, x):
    return [sum((u*v for u, v in zip(row, x)), Q(0)) for row in a]


def identity(n):
    return [[Q(i == j) for j in range(n)] for i in range(n)]


def add(a, b):
    return [[x+y for x, y in zip(u, v)] for u, v in zip(a, b)]


def exact_number(value):
    if '/' in value:
        left, right = value.split('/')
        value = float(left)/float(right)
    else:
        value = float(value)
    return Q(value)


def causal_checks():
    # Norm(L) may be much greater than one; nilpotence still gives the inverse.
    ell = [[Q(0),Q(0),Q(0),Q(0)], [Q(4),Q(0),Q(0),Q(0)],
           [Q(2),Q(3),Q(0),Q(0)], [Q(1),Q(5),Q(7),Q(0)]]
    unit = identity(4)
    power = unit
    inverse = unit
    for _ in range(1, 4):
        power = matrix_product(power, ell)
        inverse = add(inverse, power)
    zero = [[Q(0)]*4 for _ in range(4)]
    assert matrix_product(power, ell) == zero
    lhs = [[unit[i][j]-ell[i][j] for j in range(4)] for i in range(4)]
    assert matrix_product(lhs, inverse) == unit
    residual = [Q(1,1000000), Q(0), Q(0), Q(0)]
    error = matvec(inverse, residual)
    assert error[-1] == Q(119,1000000)
    # Tight affine example: G(x)=a+Lx, candidate x*+e attains the majorant.
    return {"dimension":4, "infinity_norm_L":13,
            "finite_inverse_exact":True, "error_from_first_residual":[str(v) for v in error]}


def prefix_checks():
    def target(x):
        return [Q(1)] + [Q(1,3)+x[i-1]/2 for i in range(1,8)]
    def run(cache):
        x = [Q(1)]+[Q(0)]*7
        keys = [None]*8
        values = [Q(1)]+[Q(0)]*7
        calls = 0
        trace = []
        for iteration in range(9):
            g = [Q(1)]+[Q(0)]*7
            for i in range(1,8):
                key = tuple(x[:i])
                if cache and key == keys[i]:
                    g[i] = values[i]
                else:
                    calls += 1
                    g[i] = Q(1,3)+x[i-1]/2
                    keys[i], values[i] = key,g[i]
            trace.append([str(v) for v in x])
            if g == x:
                return calls, trace
            x = g
        raise AssertionError('finite propagation failed')
    old, old_trace = run(False)
    new, new_trace = run(True)
    assert old_trace == new_trace
    assert (old,new) == (56,28)
    return {"stages":8,"updates":7,"legacy_rhs_calls":old,
            "cached_rhs_calls":new,"all_iterates_exactly_equal":True}


def quotient_checks():
    path = ROOT/'fixtures/rodas5p_coefficients_snapshot.json'
    literal = json.loads(path.read_text())
    gamma = exact_number(literal['gamma'])
    aa = [[exact_number(v) for v in row] for row in literal['A']]
    cc = [[exact_number(v) for v in row] for row in literal['C']]
    def scalar(delta,h):
        tt=[]
        for i in range(8):
            value=h*gamma+sum(((gamma*cc[i][j]-h*gamma*delta*aa[i][j])*tt[j]
                              for j in range(i)),Q(0))
            tt.append(value/(1+h*gamma*delta))
        return tt
    checked=0
    component_equalities=0
    for n in (1,2,3,7):
        yy=[Q(i-2,7) for i in range(n)]
        bb=[Q((i*i+3)%11-4,9) for i in range(n)]
        for d,kappa in ((Q(0),Q(0)),(Q(2),Q(0)),(Q(2),Q(2)),(Q(10),Q(3))):
            for h in (Q(1,100),Q(1,2)):
                def mean(x): return sum(x,Q(0))/n
                def rhs(x):
                    mm=mean(x)
                    return [b-d*y+kappa*mm for y,b in zip(x,bb)]
                def shifted_inverse(r):
                    mm=mean(r)
                    return [mm/(1+h*gamma*(d-kappa))+(v-mm)/(1+h*gamma*d) for v in r]
                f0=rhs(yy)
                fm=mean(f0)
                tc,tt=scalar(d-kappa,h),scalar(d,h)
                lifted=[[tc[i]*fm+tt[i]*(v-fm) for v in f0] for i in range(8)]
                direct=[]
                for i in range(8):
                    state=[yy[u]+sum((aa[i][j]*direct[j][u] for j in range(i)),Q(0)) for u in range(n)]
                    fi=rhs(state)
                    rr=[h*gamma*fi[u]+gamma*sum((cc[i][j]*direct[j][u] for j in range(i)),Q(0)) for u in range(n)]
                    direct.append(shifted_inverse(rr))
                assert lifted == direct
                for i in range(8):
                    stage=lifted[i]
                    state=[yy[u]+sum((aa[i][j]*lifted[j][u] for j in range(i)),Q(0)) for u in range(n)]
                    fi=rhs(state)
                    residual=[stage[u]+h*gamma*(d*stage[u]-kappa*mean(stage))
                              -h*gamma*fi[u]-gamma*sum((cc[i][j]*lifted[j][u] for j in range(i)),Q(0))
                              for u in range(n)]
                    assert residual == [Q(0)]*n
                checked+=1
                component_equalities+=8*n
    return {"cells":checked,"stage_component_equalities":component_equalities,
            "all_original_target_residuals_exactly_zero":True,
            "tableau_sha256":hashlib.sha256(path.read_bytes()).hexdigest(),
            "coefficient_semantics":"exact rationals of binary64 parsed coefficients"}


def scalar_acceleration_checks():
    cells=0
    for q in (Q(-3,2),Q(-1,2),Q(0),Q(1,2),Q(9,10),Q(3,2)):
        for b in (Q(-2,7),Q(1,3)):
            x0,x1=Q(-2,3),Q(4,7)
            g0,g1=q*x0+b,q*x1+b
            r0,r1=g0-x0,g1-x1
            accel=g1-r1/(r1-r0)*(g1-g0)
            assert accel==b/(1-q)
            cells+=1
    # q>1 is a valid algebraic acceleration example, not a contraction theorem.
    return {"affine_cases":cells,"exact_scalar_root":True,
            "includes_noncontractive_controls":True}


def main():
    begin=time.perf_counter()
    results={"causal":causal_checks(),"prefix":prefix_checks(),
             "quotient":quotient_checks(),"acceleration":scalar_acceleration_checks()}
    results.update({"status":"pass_scoped","elapsed_seconds":time.perf_counter()-begin,
                    "scope":"finite exact-rational audits; general proofs are in THEORY.md"})
    out=HERE/'EXACT_RESULTS.json'
    out.write_text(json.dumps(results,indent=2)+'\n')
    print(json.dumps(results,indent=2))


if __name__=='__main__': main()
