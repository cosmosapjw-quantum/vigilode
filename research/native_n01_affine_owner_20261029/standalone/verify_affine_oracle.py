#!/usr/bin/env python3
"""Independent exact rational reconstruction of the represented binary64 oracle.
No Rust certificate calculation is duplicated for expected results.
"""
import csv, json, struct, sys
from fractions import Fraction as F
from pathlib import Path

def num(bits):
    return F.from_float(struct.unpack('>d', int(bits).to_bytes(8,'big'))[0])

def norm(x, w):
    return max(abs(x[i])/w[i] for i in range(len(x)))

def mul(a,x):
    return [sum(a[i][j]*x[j] for j in range(len(x))) for i in range(len(x))]

def solve2(A,b):
    det=A[0][0]*A[1][1]-A[0][1]*A[1][0]
    assert det!=0
    return [(b[0]*A[1][1]-A[0][1]*b[1])/det,
            (A[0][0]*b[1]-b[0]*A[1][0])/det]

results=[]
with open(sys.argv[1] if len(sys.argv)>1 else 'oracle.csv',newline='') as stream:
    for ix,row in enumerate(csv.DictReader(stream)):
        x={k:num(v) for k,v in row.items() if k!='accepted'}
        ref,t,sigma=x['ref'],x['clock'],x['sigma']
        dt=t-ref
        J=[[x['j00'],x['j01']],[x['j10'],x['j11']]]
        K=[[x['k00'],x['k01']],[x['k10'],x['k11']]]
        b=[x['b0'],x['b1']]; q=[x['q0'],x['q1']]
        w=[x['w0'],x['w1']];p=[x['p0'],x['p1']]
        candidate=[x['x0'],x['x1']]; actual_y=[x['y0'],x['y1']]
        targetJ=[[J[i][j]+dt*K[i][j] for j in range(2)] for i in range(2)]
        targetb=[b[i]+dt*q[i] for i in range(2)]
        A0=[[int(i==j)-sigma*J[i][j] for j in range(2)] for i in range(2)]
        A=[[int(i==j)-sigma*targetJ[i][j] for j in range(2)] for i in range(2)]
        exact_y=mul(A0,candidate)
        eta=norm([actual_y[i]-exact_y[i] for i in range(2)],w)
        dj=max(sum(abs(dt*K[i][j])*w[j]/w[i] for j in range(2)) for i in range(2))
        db=norm([dt*q[i] for i in range(2)],w)
        mu=max(J[i][i]+sum(abs(J[i][j])*w[j]/w[i] for j in range(2) if j!=i) for i in range(2))
        rho0=norm([b[i]-actual_y[i] for i in range(2)],w)
        rho_exact=rho0+db+eta+sigma*dj*norm(candidate,w)
        actual_res=norm([targetb[i]-mul(A,candidate)[i] for i in range(2)],w)
        xstar=solve2(A,targetb)
        actual_error=norm([candidate[i]-xstar[i] for i in range(2)],p)
        margin=1-sigma*(x['mu']+x['delta_j'])
        cp=max(w[i]/p[i] for i in range(2))
        tests={
            'delta_j': dj<=x['delta_j'],
            'delta_b': db<=x['delta_b'],
            'eta': eta<=x['eta'],
            'mu': mu<=x['mu'],
            'margin': 0<x['margin']<=margin,
            'rho_formula': rho_exact<=x['rho'],
            'true_original': actual_res<=x['rho'],
            'physical_error': actual_error<=x['error'],
            'forward_enclosure': cp*x['rho']/x['margin']<=x['error'],
            'source_target_mu': max(targetJ[i][i]+sum(abs(targetJ[i][j])*w[j]/w[i] for j in range(2) if i!=j) for i in range(2)) <= x['mu']+x['delta_j'],
        }
        for k,ok in tests.items():
            if not ok: raise AssertionError(f'row={ix} failure {k} exact/upper: {actual_error} vs {x["error"]}')
        assert row['accepted']=='1'
        results.append({'row':ix, 'passes':len(tests),
            'rho_upper':float(x['rho']), 'rho_actual':float(actual_res),
            'stage_upper':float(x['error']), 'stage_actual':float(actual_error),
            'eta_exact':float(eta),'eta_upper':float(x['eta'])})
print(json.dumps({'records':len(results),'exact_fraction_checks':sum(t['passes'] for t in results),
'max_true_residual':max(t['rho_actual'] for t in results),
'max_true_stage_error':max(t['stage_actual'] for t in results),
'min_eta_upper':min(t['eta_upper'] for t in results),
'max_eta_upper':max(t['eta_upper'] for t in results),
'worst_rho_upper_over_actual':max(t['rho_upper']/t['rho_actual'] for t in results if t['rho_actual']>0),
'worst_stage_upper_over_actual':max(t['stage_upper']/t['stage_actual'] for t in results if t['stage_actual']>0),
'rows':results},indent=2))
