#!/usr/bin/env python3
"""Independent high-precision all-entry stage oracle and exact closure check.
The Decimal roots are numerical checks, not a replacement for the H5 proof.
The d+A*r<=r and r<=R checks use exact fractions of binary64 inputs.
"""
import json, sys
from decimal import Decimal as D, localcontext
from fractions import Fraction as F

def dec(x):
    return D.from_float(float(x))
def mv(a,x):
    return [sum((v*w for v,w in zip(row,x)),D(0)) for row in a]
def inverse(a):
    n=len(a); t=[list(r)+[D(int(i==j)) for j in range(n)] for i,r in enumerate(a)]
    for c in range(n):
        pivot=max(range(c,n),key=lambda r:abs(t[r][c]));t[c],t[pivot]=t[pivot],t[c]
        v=t[c][c]; assert v!=0
        t[c]=[x/v for x in t[c]]
        for r in range(n):
            if r!=c:
                v=t[r][c];t[r]=[x-v*y for x,y in zip(t[r],t[c])]
    return [r[n:] for r in t]
def run(data):
    source=data['source']; weights=list(map(dec,source['weights']));xi=list(map(dec,source['xi']))
    y=[dec(v) for z in source['initial'] for v in z];n=len(y);s=len(data['alpha'])
    width,chi,kappa=map(dec,(source['width'],source['chi'],source['kappa']))
    alpha=[list(map(dec,r)) for r in data['alpha']];beta=[list(map(dec,r)) for r in data['beta']]
    jac=[list(map(dec,r)) for r in data['frozen_jacobian']];h=dec(data['h']);gamma=dec(data['gamma'])
    b=list(map(dec,data['b']));candidate=[list(map(dec,r)) for r in data['candidate']]
    radii=list(map(dec,data['stage_radii']))
    W=[[D(int(i==j))-h*gamma*jac[i][j] for j in range(n)] for i in range(n)];V=inverse(W)
    def rhs(x):
        u=sum((w*x[2*j] for j,w in enumerate(weights)),D(0));v=sum((w*x[2*j+1] for j,w in enumerate(weights)),D(0))
        r=u*u+v*v;out=[]
        for j,omega in enumerate(xi):
            out.extend([-kappa*x[2*j]-width*omega*x[2*j+1]-chi*r*v,
                        -kappa*x[2*j+1]+width*omega*x[2*j]+chi*r*u])
        return out
    f0=rhs(y);rows=[];checks=0
    for lam in map(D,['0','.25','.5','.75','1']):
        state=[[D(0)]*n for _ in range(s)]
        for iteration in range(200):
            new=[]
            for i in range(s):
                delta=[sum((alpha[i][j]*state[j][a] for j in range(s)),D(0)) for a in range(n)]
                coupling=[sum(((beta[i][j]-(gamma if i==j else 0))*state[j][a] for j in range(s)),D(0)) for a in range(n)]
                f=rhs([a+d for a,d in zip(y,delta)]);jd=mv(jac,delta);jc=mv(jac,coupling)
                new.append(mv(V,[h*(f0[a]+jc[a]+lam*(f[a]-f0[a]-jd[a])) for a in range(n)]))
            change=max(abs(a-b) for r,t in zip(new,state) for a,b in zip(r,t));state=new
            if change<D('1e-70'):break
        else:raise AssertionError('reference failed to converge')
        max_error=D(0)
        for i in range(s):
            for a in range(n):
                error=abs(candidate[i][a]-state[i][a]);max_error=max(max_error,error)
                assert error<=radii[i],(lam,i,a,error,radii[i]);checks+=1
        rows.append({'lambda':str(lam),'iterations':iteration+1,'last_change':str(change),'maximum_stage_distance':str(max_error)})
        if lam==1:
            output=[y[a]+sum((b[i]*state[i][a] for i in range(s)),D(0)) for a in range(n)]
            for got,ref,radius in zip(data['output'],output,data['output_radii']):
                assert abs(dec(got)-ref)<=dec(radius);checks+=1
    A=data['majorant'];d=data['common_defect'];r=data['stage_radii'];R=F(data['common_tube_radius'])
    for i in range(s):
        assert F(d[i])+sum(F(a)*F(v) for a,v in zip(A[i],r))<=F(r[i])<=R
    return {'status':'PASS','evidence_kind':'numerically_checked_90_decimal_digits_plus_exact_fraction_closure',
        'stage_and_output_containments':checks,'exact_fraction_radius_closures':s,'rows':rows,
        'scope':'actual represented all-entry K target; not an ODE flow oracle'}
if __name__=='__main__':
    with localcontext() as ctx:
        ctx.prec=90
        print(json.dumps(run(json.load(open(sys.argv[1]))),indent=2))
