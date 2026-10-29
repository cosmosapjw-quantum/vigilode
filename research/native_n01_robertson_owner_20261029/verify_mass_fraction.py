"""Independent exact-rational oracle for the actual represented binary64 Rust outputs.

Does not use Rust's interval formulas or write the source code under test.
"""
from __future__ import annotations
import csv, json, sys
from fractions import Fraction as Q
from pathlib import Path

FIELD_NAMES = ("k1,k2,k3,ra,rb,rc,la,lb,lc,sigma,w0,w1,w2,p0,p1,p2,x0,x1,x2,"
               "dj,db,eta,mu,margin,rho,forward,jvp_round,jvp_reference").split(",")

def exact(x): return Q.from_float(float(x))

def rhs_q(k,z):
    r1=k[0]*z[0]; r2=k[1]*z[1]*z[2]; r3=k[2]*z[1]*z[1]
    return [-r1+r2,r1-r2-r3,r3]

def jac_q(k,z):
    a,b,c=z; u=k[1]*c; v=k[1]*b; w=2*k[2]*b
    return [[-k[0],u,v],[k[0],-u-w,-v],[Q(0),w,Q(0)]]

def rhs_float(k,z):
    r1=k[0]*z[0];r2=(k[1]*z[1])*z[2];r3=(k[2]*z[1])*z[1]
    return [-r1+r2,r1-r2-r3,r3]

def jac_float(k,z):
    u=k[1]*z[2];v=k[1]*z[1];w=(2.0*k[2])*z[1]
    return [[-k[0],u,v],[k[0],-u-w,-v],[0.0,w,0.0]]

def solve3(a,b):
    x=[list(row)+[bb] for row,bb in zip(a,b)]
    for i in range(3):
        pivot=next((j for j in range(i,3) if x[j][i]),None)
        if pivot is None:raise ArithmeticError("singular exact matrix")
        x[i],x[pivot]=x[pivot],x[i]
        t=x[i][i]; x[i]=[v/t for v in x[i]]
        for j in range(3):
            if i==j:continue
            t=x[j][i]; x[j]=[v-t*u for v,u in zip(x[j],x[i])]
    return [row[3] for row in x]

def fmax(xs):return max(xs)

def check_row(raw):
    z={k:float(raw[k]) for k in FIELD_NAMES}
    k=[z[f'k{i}'] for i in (1,2,3)]
    ref=[z[s] for s in ('ra','rb','rc')]
    live=[z[s] for s in ('la','lb','lc')]
    weights=[z[f'w{i}'] for i in range(3)]
    phys=[z[f'p{i}'] for i in range(3)]
    xhat=[z[f'x{i}'] for i in range(3)]
    kq=list(map(exact,k)); rq=list(map(exact,ref)); lq=list(map(exact,live));
    wq=list(map(exact,weights)); pq=list(map(exact,phys)); xq=list(map(exact,xhat)); sq=exact(z['sigma'])
    fstar=rhs_q(kq,lq); jstar=jac_q(kq,lq)
    # Reference J0/b0 are represented binary64 outputs (not ideal exact f(anchor)).
    j0=[[exact(v) for v in row] for row in jac_float(k,ref)]
    b0=[exact(v) for v in rhs_float(k,ref)]
    # Reproduce only the represented nominal RHS evaluation to derive the *exact* gap.
    y=[]
    for i in range(3):
        s=0.0
        for j in range(3):s+=float(j0[i][j])*xhat[j]
        y.append(exact(xhat[i]-z['sigma']*s))
    
    def w_norm(qvec):return max(abs(v)/w for v,w in zip(qvec,wq))
    def p_norm(qvec):return max(abs(v)/p for v,p in zip(qvec,pq))
    dj=max(sum(abs(jstar[i][j]-j0[i][j])*wq[j]/wq[i] for j in range(3)) for i in range(3))
    db=w_norm([fstar[i]-b0[i] for i in range(3)])
    eta=w_norm([y[i]-(xq[i]-sq*sum(j0[i][j]*xq[j] for j in range(3))) for i in range(3)])
    mu=max(j0[i][i]+sum(abs(j0[i][j])*wq[j]/wq[i] for j in range(3) if i!=j) for i in range(3))
    margin=Q(1)-sq*(mu+dj)
    a=[[Q(1 if i==j else 0)-sq*jstar[i][j] for j in range(3)] for i in range(3)]
    rhs=[fstar[i]-sum(a[i][j]*xq[j] for j in range(3)) for i in range(3)]
    true_r=w_norm(rhs)
    sol=solve3(a,fstar)
    true_e=p_norm([sol[i]-xq[i] for i in range(3)])
    
    # JVP callback actual binary64 route J_float(live) v.
    jlive=jac_float(k,live); v=[0.125,-0.25,0.5]; vq=list(map(exact,v))
    jv_float=[]
    for i in range(3):
        row_sum=0.0
        for j in range(3):row_sum+=jlive[i][j]*v[j]
        jv_float.append(exact(row_sum))
    gap=w_norm([jv_float[i]-sum(jstar[i][j]*vq[j] for j in range(3)) for i in range(3)])
    oldgap=w_norm([jv_float[i]-sum(j0[i][j]*vq[j] for j in range(3)) for i in range(3)])
    bound_keys=['dj','db','eta','mu','margin','rho','forward','jvp_round','jvp_reference']
    vals={q:exact(z[q]) for q in bound_keys}
    criteria={
        'delta_j':dj<=vals['dj'],
        'delta_b':db<=vals['db'],
        'apply_eta':eta<=vals['eta'],
        'mu':mu<=vals['mu'],
        'margin':vals['margin']>0 and vals['margin']<=margin,
        'original_residual':true_r<=vals['rho'],
        'stage_error':true_e<=vals['forward'],
        'jvp_rounding':gap<=vals['jvp_round'],
        'jvp_reference':oldgap<=vals['jvp_reference'],
        'mass_conservation':sum(fstar)==0 and all(sum(jstar[i][j] for i in range(3))==0 for j in range(3)),
    }
    return criteria,{'case':int(raw['case']), 'actual_residual':float(true_r),
        'bound_residual':z['rho'],'actual_error':float(true_e),'bound_error':z['forward'],
        'true_delta_j':float(dj),'bound_delta_j':z['dj'], 'true_eta':float(eta),'bound_eta':z['eta']}

def main():
    basedir=Path(__file__).resolve().parent
    with open(basedir/'oracle.csv') as f:rows=list(csv.DictReader(f))
    failures=[]; checks={};scatter=[]
    for row in rows:
        if set(FIELD_NAMES+['case'])-set(row):raise ValueError('missing column')
        ok,s=check_row(row);scatter.append(s)
        for name,passed in ok.items():
            checks[name]=checks.get(name,0)+int(passed)
            if not passed:failures.append({'case':row['case'],'test':name})
    result={'rows':len(rows),'checks_per_row':len(checks),'checks_passed':sum(checks.values()),
            'checks_total':len(rows)*len(checks),'failures':failures,'by_check':checks,
            'max_actual_residual':max(s['actual_residual'] for s in scatter),
            'max_actual_stage_error':max(s['actual_error'] for s in scatter),
            'max_residual_bound_ratio':max(s['bound_residual']/s['actual_residual'] if s['actual_residual'] else 0 for s in scatter),
            'provenance':'fraction_from_binary64; independently assembled exact Robertson Jacobian,RHS and 3x3 solve; not nonlinear full-IVP'}
    (basedir/'fraction_result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    (basedir/'scatter.json').write_text(json.dumps(scatter,indent=2)+'\n')
    print(json.dumps(result,indent=2))
    return 1 if failures else 0
if __name__=='__main__':sys.exit(main())