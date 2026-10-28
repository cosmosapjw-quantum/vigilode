#!/usr/bin/env python3
"""Exact rational audit of sharp residual transport for represented coefficients."""
from fractions import Fraction as Q
from pathlib import Path
import hashlib, json, time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
TABLEAU = REPO / 'fixtures/rodas5p_coefficients_snapshot.json'

def number(s):
    if '/' in s:
        a, b = s.split('/')
        return Q(float(a)/float(b))
    return Q(float(s))

def main():
    started = time.perf_counter()
    data = json.loads(TABLEAU.read_text())
    gamma = number(data['gamma'])
    a = [[number(s) for s in row] for row in data['A']]
    c = [[number(s) for s in row] for row in data['C']]
    b = [number(s) for s in data['b_code']]
    cells = []
    for n in (1, 3, 7, 17):
        for decay, coupling, h in ((Q(0),Q(0),Q(1,8)), (Q(2),Q(2),Q(1,8)), (Q(20),Q(19),Q(1,64))):
            hg = h*gamma
            def solve(rate, residual):
                x = []
                for i in range(8):
                    rhs = residual[i] - sum(((hg*rate*a[i][j]-gamma*c[i][j])*x[j] for j in range(i)),Q(0))
                    x.append(rhs/(1+hg*rate))
                return x
            def adjoint(rate):
                w = [Q(0)]*8
                for j in reversed(range(8)):
                    rhs = b[j] - sum(((hg*rate*a[i][j]-gamma*c[i][j])*w[i] for i in range(j+1,8)),Q(0))
                    w[j] = rhs/(1+hg*rate)
                # Independent forward impulse response for every residual stage.
                for j in range(8):
                    impulse = [Q(i == j) for i in range(8)]
                    x = solve(rate,impulse)
                    assert sum((u*v for u,v in zip(b,x)),Q(0)) == w[j]
                return w
            wt,wc = adjoint(decay),adjoint(decay-coupling)
            def norm(t,c):
                return abs(t+(c-t)/n)+(n-1)*abs((c-t)/n)
            amp = sum((norm(t,c) for t,c in zip(wt,wc)),Q(0))
            # Construct sign-aligned original-coordinate residual and solve it
            # in physical channels. This realizes the claimed sharp row norm.
            residual=[]
            for t,cw in zip(wt,wc):
                row=[t+(cw-t)/n]+[(cw-t)/n]*(n-1)
                residual.append([Q(-1 if v<0 else 1) for v in row])
            collective = [sum(row,Q(0))/n for row in residual]
            transverse = [[row[k]-collective[i] for i,row in enumerate(residual)] for k in range(n)]
            uc=solve(decay-coupling,collective)
            ut=[solve(decay,r) for r in transverse]
            output=sum((b[i]*(uc[i]+ut[0][i]) for i in range(8)),Q(0))
            assert output == amp
            tt=solve(decay,[hg]*8)
            tc=solve(decay-coupling,[hg]*8)
            rt=1-decay*sum((u*v for u,v in zip(b,tt)),Q(0))
            rc=1-(decay-coupling)*sum((u*v for u,v in zip(b,tc)),Q(0))
            gain=norm(rt,rc)
            # Geometric recurrence avoids any divided difference near G=1.
            cumulative=Q(0)
            for _ in range(64): cumulative=1+gain*cumulative
            tight=Q(0)
            pt=pc=Q(1)
            for _ in range(64):
                tight += sum((norm(pt*t,pc*cw) for t,cw in zip(wt,wc)),Q(0))
                pt,pc=pt*rt,pc*rc
            assert tight <= amp*cumulative
            requested=Q(1,100_000_000)
            limit=requested/tight
            assert tight*limit == requested
            if decay == 0:
                assert gain == 1 and cumulative == 64
            cells.append({'n':n,'decay':float(decay),'coupling':float(coupling),'h':float(h),
                'sharp_amplification_float':float(amp),'step_norm_float':float(gain),
                'finite_sum_float':float(cumulative),'derived_limit_float':float(limit),
                'sharp_global_amplification_float':float(tight),
                'geometric_global_bound_float':float(amp*cumulative),
                'adjoint_equals_forward_impulse_exact':True,'sign_aligned_sharpness_exact':True,
                'global_budget_identity_exact':True})
    output={'status':'pass_scoped','coefficient_semantics':'exact rationals of parsed binary64 tableau literals',
        'tableau_sha256':hashlib.sha256(TABLEAU.read_bytes()).hexdigest(),
        'cells':cells,'elapsed_seconds':time.perf_counter()-started,
        'scope':'exact identities and sharp operator norms; not a floating trajectory certificate'}
    (HERE/'EXACT_STRUCTURAL_CONSTANTS.json').write_text(json.dumps(output,indent=2)+'\n')
    print(json.dumps({'status':output['status'],'cells':len(cells),'elapsed_seconds':output['elapsed_seconds'],
        'benchmark_constants':cells[-1]}))

if __name__=='__main__': main()
