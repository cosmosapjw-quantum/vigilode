#!/usr/bin/env python3
"""Independent original-coupled-source exact-Fraction verifier and Decimal reference.

Input: actual Rust VCB_TRACE JSON records. This does not import or call any Rust
source/map function. Each binary64 coefficient/state is interpreted as exact
rational. Decimal RK4 is a high-precision, *non-rigorous* accuracy cross-check;
exact Fraction inequalities supply the actual deterministic residual/majorant
verification. Requires only Python standard library.
"""
import json,sys,struct
from collections import defaultdict
from fractions import Fraction as F
from decimal import Decimal,localcontext

def ff(bits):return struct.unpack('>d',struct.pack('>Q',int(bits)))[0]
def fr(bits):return F.from_float(ff(bits))
def dec(bits):return Decimal.from_float(ff(bits))

def unpack(z):
    n=int(z['n']);s=list(map(fr,z['source_bits']));assert len(s)==3*n+n*n
    k=s[:n];g=s[n:2*n];y=s[2*n:3*n];c=[s[3*n+i*n:3*n+(i+1)*n] for i in range(n)]
    h=fr(z['h_bits']);eps=fr(z['eps_bits'])
    assert len(z['points'])>=2 and len(z['points'])<=4097
    assert all(k_i>0 for k_i in k) and all(g_i>=0 for g_i in g)
    assert all(c[i][i]==0 and all(v>=0 for v in c[i]) for i in range(n))
    return n,k,g,y,c,h,eps

def exact_certificate(z):
    n,k,g,y,c,h,eps=unpack(z)
    N=len(z['points'])-1
    assert N>0 and N&(N-1)==0
    M=max(y);kmin=min(k);A=1+h*kmin;assert A>1 and M>0
    Cs=[sum(row) for row in c]
    actual_q=max(h*Cs[i]/(1+h*(k[i]+Cs[i])) for i in range(n))
    P0=max(k[i]*M+g[i]*M**3+2*Cs[i]*M for i in range(n))
    major=M;E=F(0);prev=y;time0=F(0);max_actual_r=F(0)
    for idx,point in enumerate(z['points']):
        xs=list(map(fr,point['state']))
        assert len(xs)==n and all(0<=x<=M for x in xs)
        label=fr(point['time']); t_exact=h*idx
        q_represented=fr(point['q'])
        E_represented=fr(point['E'])
        rho_reported=fr(point['rho'])
        assert label>=time0
        time0=label
        if idx==0:
            assert xs==y and E_represented==0 and rho_reported==0
            continue
        assert actual_q<=q_represented<1
        L=max(k[i]+3*g[i]*major**2+2*Cs[i] for i in range(n))
        P=max(k[i]*major+g[i]*major**3+2*Cs[i]*major for i in range(n))
        tau=h*h*L*P/2
        original=[
            (1+h*(k[i]+Cs[i]))*xs[i] +h*g[i]*xs[i]**3-prev[i] -h*sum(c[i][j]*xs[j] for j in range(n))
            for i in range(n)
        ]
        absrho=max(map(abs,original))
        assert absrho<=rho_reported,(z['name'],idx,'FALSE_RESIDUAL_UPPER',float(absrho),float(rho_reported))
        max_actual_r=max(max_actual_r,absrho)
        E=(E+tau+rho_reported)/A
        clock=P0*abs(label-t_exact)
        assert E+clock<=E_represented,(z['name'],idx,'FALSE_GLOBAL_UPPER',float(E+clock),float(E_represented))
        assert E_represented<=eps,(z['name'],idx,'TARGET_OVER',float(E_represented),float(eps))
        major/=A;prev=xs
    assert z['map_attempted']==z['map_completed']
    assert z['original_components']==n*z['map_attempted']
    assert z['inverse_components']==n*z['map_attempted']
    return {'case':z['name'],'fast':z['fast'],'N':N,'n':n,'source_q':float(actual_q),
            'max_rho_exact':float(max_actual_r),'final_E_upper':ff(z['points'][-1]['E']),
            'source':(k,g,y,c,h),'points':z['points']}

def decimal_reference(x, microsteps_per_step):
    # Independent 70-digit RK4. This is NUMERICAL_ONLY, never a proof of accuracy.
    k,g,y,c,h=x
    n=len(y)
    with localcontext() as ctx:
        ctx.prec=70
        K=list(map(lambda t:Decimal(t.numerator)/Decimal(t.denominator),k))
        G=list(map(lambda t:Decimal(t.numerator)/Decimal(t.denominator),g))
        C=[[Decimal(v.numerator)/Decimal(v.denominator) for v in row] for row in c]
        Y=[Decimal(v.numerator)/Decimal(v.denominator) for v in y]
        dt=Decimal(h.numerator)/Decimal(h.denominator)/microsteps_per_step
        half=dt/2; sixth=dt/6
        def rhs(v):
            return [-K[i]*v[i]-G[i]*(v[i]**3)+sum((C[i][j]*(v[j]-v[i]) for j in range(n)),Decimal(0)) for i in range(n)]
        returner=[Y.copy()]
        u=Y.copy()
        # Call exactly N global steps, inferred from input below.
        return u,dt,rhs,half,sixth,returner

def ref_trajectory(x,N,micro):
    u,dt,rhs,half,sixth,points=decimal_reference(x,micro)
    for _ in range(N):
        for _ in range(micro):
            a=rhs(u)
            b=rhs([u[i]+half*a[i] for i in range(len(u))])
            c=rhs([u[i]+half*b[i] for i in range(len(u))])
            d=rhs([u[i]+dt*c[i] for i in range(len(u))])
            u=[u[i]+sixth*(a[i]+2*b[i]+2*c[i]+d[i]) for i in range(len(u))]
        points.append(u.copy())
    return points

def main(path):
    records=[]
    for line in open(path,encoding='utf8'):
        if 'VCB_TRACE=' in line:
            records.append(json.loads(line.split('VCB_TRACE=',1)[1].strip()))
    assert len(records)==8, f'exactly 8 native traces, got {len(records)}'
    groups=defaultdict(dict)
    for z in records:
        item=exact_certificate(z)
        name=item['case'];fast=bool(item['fast'])
        assert fast not in groups[name]
        groups[name][fast]=item
    assert set(groups)=={'asymmetric_2','asymmetric_2_refined','coupled_3_cubic','linear_exchange_control'}
    for name,arm in groups.items():
        assert set(arm)=={True,False}
        n=arm[True]['n']; N=arm[True]['N'];assert n==arm[False]['n'] and N==arm[False]['N']
        assert arm[True]['source']==arm[False]['source']
        ref1=ref_trajectory(arm[True]['source'],N,32)
        ref2=ref_trajectory(arm[True]['source'],N,64)
        max_ref_diff=max(max(abs(ref1[i][j]-ref2[i][j]) for j in range(n)) for i in range(N+1))
        assert max_ref_diff<Decimal('1e-10'),('INDEPENDENT_REFERENCE_UNSTABLE',name,str(max_ref_diff))
        achieved={};peak={}
        for fast,v in arm.items():
            errors=[]
            for i,p in enumerate(v['points']):
                ys=list(map(dec,p['state']))
                err=max(abs(ys[j]-ref2[i][j]) for j in range(n))
                errors.append(err)
                assert err<=dec(p['E'])+Decimal('1e-10'),('PHYSICAL_BOUND_REFUTED',name,fast,i,str(err),dec(p['E']))
            achieved[fast]=errors[-1];peak[fast]=max(errors)
        # A different *computed* accuracy is not silently called identical.
        lo=min(achieved.values());hi=max(achieved.values())
        rel_gap=(hi-lo)/max(hi,Decimal('1e-50'))
        print('VCB_INDEPENDENT case={} modes={} coarse={} ref_delta={:.4e} baseline_final_err={:.8e} fast_final_err={:.8e} achieved_rel_gap={:.8e} matched_achieved_1pct={}'.format(name,n,N,max_ref_diff,achieved[False],achieved[True],rel_gap,rel_gap<Decimal('0.01')))
    print('VCB_FRACTION_ORIGINAL_SOURCE=8/8_PASS')
    print('VCB_INDEPENDENT_DECIMAL_REFERENCE=4/4_PASS_NONRIGOROUS')
if __name__=='__main__':
    if len(sys.argv)!=2: raise SystemExit('usage: independent_fraction_oracle.py path/VCB_BENCH.log')
    main(sys.argv[1])
