#!/usr/bin/env python3
"""Exact arithmetic checks, reading the implementation coefficients. No third-party deps.
The RODAS check verifies scalar series only; it is not a nonlinear order proof.
"""
import ast, hashlib, json, math, re
from fractions import Fraction as Q
from functools import lru_cache
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
SRC = ROOT / 'crates/rodas5p-integrators/src/dopri5.rs'
SNAP = ROOT / 'fixtures/rodas5p_coefficients_snapshot.json'
text = SRC.read_text()
def eval_exact(n):
    if isinstance(n, ast.List): return [eval_exact(e) for e in n.elts]
    if isinstance(n, ast.Constant): return Q(str(n.value))
    if isinstance(n, ast.UnaryOp) and isinstance(n.op, ast.USub): return -eval_exact(n.operand)
    if isinstance(n, ast.BinOp):
        a,b=eval_exact(n.left),eval_exact(n.right)
        if isinstance(n.op,ast.Div): return a/b
        if isinstance(n.op,ast.Add): return a+b
        if isinstance(n.op,ast.Sub): return a-b
        if isinstance(n.op,ast.Mult): return a*b
    raise ValueError(ast.dump(n))
def array(name):
    # Every literal in these constants is an integer written with .0; no f64
    # arithmetic is used when recovering its intended exact rational value.
    s=re.search(r'const '+name+r':[^=]+=(.*?);\n',text,re.S).group(1).strip()
    s=s.replace('[0.0; 7]', '['+','.join(['0.0']*7)+']')
    return eval_exact(ast.parse(s,mode='eval').body)
a=array('DOPRI5_A'); b=a[-1]; bh=array('DOPRI5_B_EMBEDDED'); ew=array('ERROR_WEIGHTS')
assert len(a)==7 and all(len(r)==7 for r in a)
assert all(a[i][j]==0 for i in range(7) for j in range(i,7))
assert ew==[x-y for x,y in zip(b,bh)]
assert array('DOPRI5_C')==[sum(r) for r in a]
@lru_cache(None)
def order(t): return 1+sum(order(s) for s in t)
@lru_cache(None)
def gamma(t): return order(t)*math.prod(gamma(s) for s in t)
trees={1:[()]}
for n in range(2,7):
    pool=sorted([t for k in range(1,n) for t in trees[k]],key=lambda t:(order(t),repr(t)))
    def multi(left,start,prefix):
        if left==0:
            yield tuple(prefix); return
        for i in range(start,len(pool)):
            t=pool[i]; k=order(t)
            if k>left: break
            yield from multi(left-k,i,prefix+[t])
    trees[n]=list(multi(n-1,0,[]))
@lru_cache(None)
def phi(t):
    if not t: return (Q(1),)*7
    return tuple(math.prod(sum(a[i][j]*phi(s)[j] for j in range(7)) for s in t) for i in range(7))
checks=[]
for n,ts in trees.items():
    for t in ts:
        exact=Q(1,gamma(t)); p=phi(t)
        checks.append({'order':n,'tree':repr(t),'exact':str(exact),
                       'endpoint':str(sum(x*y for x,y in zip(b,p))),
                       'embedded':str(sum(x*y for x,y in zip(bh,p)))})
assert sum(len(trees[n]) for n in range(1,6))==17
assert all(c['endpoint']==c['exact'] for c in checks if c['order']<=5)
assert all(c['embedded']==c['exact'] for c in checks if c['order']<=4)
def stability(weights):
    v=[Q(1)]*7; r=[Q(1)]
    for k in range(1,8):
        r.append(sum(x*y for x,y in zip(weights,v)))
        v=[sum(aij*vj for aij,vj in zip(row,v)) for row in a]
    return r
r=stability(b); rh=stability(bh)
assert r[:6]==[Q(1,math.factorial(j)) for j in range(6)]
assert r[6]==Q(1,600) and r[7]==0
# A constructive contractive interval: drop the negative theta^4 term.
q09=-Q(1,1800)+Q(9,10)**2/Q(1600)+Q(9,10)**6/Q(360000)
assert q09<0
snap=json.loads(SNAP.read_text()); s=len(snap['c']); assert s==8
A=[[Q(x) for x in row] for row in snap['A']]; C=[[Q(x) for x in row] for row in snap['C']]
B=[Q(x) for x in snap['b_code']]; g=Q(snap['gamma'])
# Formal power series of the transformed scalar recurrence, y'=lambda*y.
# u_i = gamma/(1-gamma*z) [z(1+sum A_ij u_j)+sum C_ij u_j].
M=7; u=[]
for i in range(s):
    numerator=[Q(0)]*M; numerator[1]=1
    for j in range(i):
        for k in range(M):
            numerator[k]+=C[i][j]*u[j][k]
            if k: numerator[k]+=A[i][j]*u[j][k-1]
    u.append([sum(g**(k-l+1)*numerator[l] for l in range(k+1)) for k in range(M)])
rr=[Q(k==0)+sum(B[j]*u[j][k] for j in range(s)) for k in range(M)]
ui=[]
for i in range(s): ui.append(-1-sum(A[i][j]*ui[j] for j in range(i)))
limit=1+sum(B[j]*ui[j] for j in range(s))
result={
 'schema':'vigilode.exact-numerics-check.v1',
 'source':{'path':str(SRC.relative_to(ROOT)),'sha256':hashlib.sha256(SRC.read_bytes()).hexdigest(),
           'coefficient_snapshot_sha256':hashlib.sha256(SNAP.read_bytes()).hexdigest()},
 'dopri5':{'stages':7,'tree_counts':{str(n):len(ts) for n,ts in trees.items()},
  'endpoint_order5_conditions_pass':17,'embedded_order4_conditions_pass':8,
  'order6_endpoint_conditions_failed':sum(c['endpoint']!=c['exact'] for c in checks if c['order']==6),
  'error_weights_exact_difference':True,'fsal_last_row_equals_endpoint':True,
  'stability_coefficients':[str(x) for x in r],'embedded_stability_coefficients':[str(x) for x in rh],
  'oscillator_abs_theta_le_09_contractivity_upper_bracket':str(q09),'all_tree_conditions':checks},
 'rodas5p_decimal_snapshot':{'stages':s,'interpretation':'exact rationals of published decimal literals; scalar check only, not full nonlinear order certification',
  'scalar_series_defects_orders_0_to_5':[float(rr[k]-Q(1,math.factorial(k))) for k in range(6)],
  'scalar_stiff_limit_decimal_residual':float(limit)},
 'scope':'exact algebra on parsed source coefficients; not a floating-point execution benchmark, nonlinear stiff order proof, or general global-error certificate'
}
(HERE/'NUMERICS_CHECK.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['source','dopri5']},indent=2))
print('DOPRI5: 17/17 order-five and 8/8 embedded order-four conditions passed.')
