"""Decision reviewer checks. Does not alter lane evidence or production source."""
from fractions import Fraction as F
from pathlib import Path
import importlib.util, json, math
import numpy as np
from scipy.linalg import expm

HERE=Path(__file__).resolve().parent
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
checks={}
budget=F.from_float(1e-320)/F.from_float(1e-160)**2
checks['mixed_exact_budget_below_output_5']=budget<5
eps=F(1,2**104)
checks['m08_exact_epsilon']=1-(1+F(1,2**52))*(1-F(1,2**52))==eps

# Independent two-component rational stage fixture, unrelated to lane tableau.
def dot(a,b):return sum((x*y for x,y in zip(a,b)),F(0))
def mv(a,b):return [dot(row,b) for row in a]
def va(a,b):return [x+y for x,y in zip(a,b)]
def vs(a,b):return [x-y for x,y in zip(a,b)]
def sc(a,b):return [a*x for x in b]
W=[[F(2),F(-1)],[F(0),F(3)]]
Wi=[[F(1,2),F(1,6)],[F(0),F(1,3)]]
J=[[F(-1),F(4)],[F(0),F(-2)]]
s=8; h=F(3,2); q=F(1,4)
alpha=[[F(0)]*s for _ in range(s)]; L=[[F(0)]*s for _ in range(s)]
for i in range(1,s):
    alpha[i][i-1]=F(1,2);L[i][i-1]=F(1,5)
    if i>1: alpha[i][i-2]=F(-1,10);L[i][i-2]=F(-1,7)
def combine(row,k):return [sum((row[j]*k[j][c] for j in range(s)),F(0)) for c in range(2)]
rhs=[[F(i+1,10),F(-i-1,15)] for i in range(s)]
K=[[F(0)]*2 for _ in range(s)]
for i in range(s):
    d=combine(alpha[i],K)
    K[i]=mv(Wi,va(rhs[i],sc(h,va(mv(J,combine(L[i],K)),[q*x*x for x in d]))))
rows=[]
for kind in ['zero','perturbed']:
    Kh=[[F(0)]*2 for _ in range(s)] if kind=='zero' else [[K[i][c]+F((-1)**(i+c),100*(i+1)) for c in range(2)] for i in range(s)]
    E=[[F(0)]*2 for _ in range(s)]
    for i in range(s):
        dh=combine(alpha[i],Kh)
        R=vs(vs(mv(W,Kh[i]),rhs[i]),sc(h,va(mv(J,combine(L[i],Kh)),[q*x*x for x in dh])))
        rad=combine([abs(x) for x in alpha[i]],E)
        linear=mv([[abs(x) for x in row] for row in J],combine([abs(x) for x in L[i]],E))
        rem=[abs(q)*(2*abs(dh[c])*rad[c]+rad[c]**2) for c in range(2)]
        E[i]=mv(Wi,va([abs(x) for x in R],sc(h,va(linear,rem))))
    ok=all(abs(K[i][c]-Kh[i][c])<=E[i][c] for i in range(s) for c in range(2))
    rows.append({'candidate':kind,'exact_componentwise_stage_inequalities':ok})
checks['independent_fraction_componentwise_majorant']=all(x['exact_componentwise_stage_inequalities'] for x in rows)

# Re-execute bounded same-input Laguerre h*rho=100 fixture without overwriting outputs.
p=load('poly',HERE.parent/'research/polynomial_joint_probe.py')
A=-np.diag(np.geomspace(.1,100.,24));v=np.cos(np.arange(24)*.37)+.3;v/=np.linalg.norm(v)
ref=np.stack([p.scalar_phi(x) for x in np.diag(A)],axis=1)*v[None,:]
poly=[]
for cap in [64.,16.]:
    plan=p.plan(100.,1.,cap);got,terms,_=p.joint(A,v,1.,plan,256)
    errors=np.linalg.norm(got-ref,axis=1)
    poly.append({'cap':cap,'degree':plan[0],'errors':errors.tolist(),'pass_1e_10':bool(np.all(errors<=1e-10))})
checks['poly_uncapped_failure_and_capped_pass']=not poly[0]['pass_1e_10'] and poly[1]['pass_1e_10']

# Explicit similarity identity and physical endpoint error, independently formed.
M=np.array([[-1.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.],[0.,0.,0.,0.]])
qv=np.array([0.,0.,0.,1.]);P=np.array([1.,0.,0.,0.]);r=-M@M@qv
telemetry=[]
for a in [.25,1.,4.,16.,64.]:
    S=np.diag([1.,a*a,a,1.]);Si=np.linalg.inv(S);Mh=Si@M@S
    error=float(P@S@(expm(Mh)@(Si@qv)-Si@(qv+M@qv)))
    telemetry.append({'alpha':a,'physical_error':error,'residual_norm':float(np.linalg.norm(Si@r))})
checks['telemetry_physical_invariance']=all(abs(x['physical_error']-(.5-math.exp(-1)))<1e-13 for x in telemetry)
checks['telemetry_residual_scaling']=all(x['residual_norm']==1/x['alpha']**2 for x in telemetry)
result={'schema':'vigilode.independent_math_checks.v1','checks':checks,
        'exact_budget':str(budget),'exact_budget_float':float(budget),'m08_exact_epsilon':str(eps),
        'homotopy_independent_exact_fixture':rows,'polynomial_reexecution':poly,'telemetry':telemetry,
        'claim_ceiling':'bounded oracle checks; no production certificate, global convergence theorem, or measured speedup'}
(HERE/'INDEPENDENT_MATH_CHECKS.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
print(json.dumps(result,indent=2));assert all(checks.values())
