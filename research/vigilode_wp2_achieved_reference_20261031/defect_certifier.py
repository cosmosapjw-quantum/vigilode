#!/usr/bin/env python3
"""VigilODE WP2: exact binary-rational continuous-time defect certificate.

Reuses the immutable GitHub WP1 executed SWEEP.log. A reference trajectory is
*generated* using Python float RK4 but its own accuracy is NEVER assumed.
Every cubic-Hermite reference piece is reinterpreted as an EXACT rational cubic
on an exact dyadic clock, and its original source defect is a degree <=9
polynomial. Its degree-9 Bernstein coefficients give a provable full-panel
sup-norm defect upper bound. A dissipative contractive inequality propagates
this bound to the exact IVP solution independently of the reference integrator.

The 1% achieved-error parity conclusion is admitted only if whole-IVP error
intervals have a strictly positive lower endpoint and the worst-case interval
ratio is <= 1.01; a numerical reference discrepancy is not an admission gate.

No external Python dependencies; default reference subdiv=64; all original
source/candidate f64 values are interpreted as exactly represented dyadic reals.
"""
import argparse
import hashlib
import json
import math
from fractions import Fraction as F
from math import comb
from pathlib import Path
import struct
import time

COHORTS=("asymmetric_2","asymmetric_2_refined","coupled_3_cubic","linear_exchange_control")
MICRO_CHOICES=(32,64,128)
EXPECTED_SWEEP_SHA256='9dae90079893170edddcf5cf8af193e53b43ab5bcf0c436cb81ce7aa0f8790b0'
EXPECTED_WP1_SUMMARY_SHA256='56746487ac5c5c8702b53ef0ffd47694c040ab4e0adcfea1ec25aa3b8e9ba909'
ALPHAS=(0.0625,0.25,1.0,4.0,16.0)
M=9
# Convert power basis sum a_j*s^j to degree-9 Bernstein via exact binomial identity.
BERN_WEIGHTS=tuple(tuple(F(comb(k,j),comb(M,j)) if j<=k else F(0) for j in range(M+1)) for k in range(M+1))


def bits_to_f(bits):
    return struct.unpack('>d',struct.pack('>Q',int(bits)))[0]

def f_to_bits(x):
    assert math.isfinite(x)
    return struct.unpack('>Q',struct.pack('>d',x))[0]

def rational(bits):
    return F.from_float(bits_to_f(bits))

def upper_as_float(value):
    """Return the least finite binary64 >= a nonnegative exact rational."""
    assert isinstance(value,F) and value>=0
    guess=float(value)
    if not math.isfinite(guess):
        raise ValueError('no finite binary64 outward upper endpoint')
    if F.from_float(guess)<value:
        guess=math.nextafter(guess,math.inf)
    assert math.isfinite(guess) and F.from_float(guess)>=value
    return guess

def mul(a,b):
    out=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):
            out[i+j]+=x*y
    return out

def polynomial_eval(coeff,s):
    acc=F(0)
    for a in reversed(coeff):
        acc=acc*s+a
    return acc

def coeffs_bernstein9(a):
    aa=list(a)+[F(0)]*(M+1-len(a))
    assert len(aa)==M+1
    return [sum((aa[j]*BERN_WEIGHTS[k][j] for j in range(k+1)),F(0)) for k in range(M+1)]

def bernstein9_eval(coeff,s):
    return sum((coeff[k]*F(comb(9,k))*s**k*(1-s)**(9-k) for k in range(10)),F(0))


def parse_source(z):
    n=int(z['n']); raw=list(map(rational,z['source_bits']))
    assert len(raw)==3*n+n*n
    k=raw[:n];g=raw[n:2*n];y=raw[2*n:3*n]
    c=[raw[3*n+i*n:3*n+(i+1)*n] for i in range(n)]
    assert all(x>0 for x in k) and all(x>=0 for x in g)
    assert all(c[i][i]==0 and all(v>=0 for v in c[i]) for i in range(n))
    h=rational(z['h_bits'])
    assert h>0
    return {'n':n,'k':k,'g':g,'y0':y,'c':c,'h':h}


def rhs_rational(y,source):
    n=source['n'];K=source['k'];G=source['g'];C=source['c']
    return [-(K[i]*y[i]+G[i]*y[i]**3)+sum((C[i][j]*(y[j]-y[i]) for j in range(n)),F(0)) for i in range(n)]

def rhs_float(y,k,g,c):
    n=len(k)
    return [-(k[i]*y[i]+g[i]*y[i]*y[i]*y[i])+sum((c[i][j]*(y[j]-y[i]) for j in range(n)),0.0) for i in range(n)]

def generate_reference(source,steps,micro):
    """Untrusted numerical predictor: f64 RK4 at exact binary64 h/micro.

    All states are returned as raw 64-bit vectors. The subsequent certificate
    trusts ONLY their represented values and the exact source, NOT the RK4 order.
    """
    assert micro>0 and micro&(micro-1)==0
    n=source['n'];h=float(source['h']);small=h/micro
    assert math.isfinite(small) and small>0 and F.from_float(small)*micro==source['h']
    k=list(map(float,source['k']));g=list(map(float,source['g']));c=[[float(x) for x in row] for row in source['c']]
    u=[float(v) for v in source['y0']]
    assert all(F.from_float(u[i])==source['y0'][i] for i in range(n))
    ref=[tuple(f_to_bits(v) for v in u)]
    for idx in range(steps*micro):
        a=rhs_float(u,k,g,c)
        b=rhs_float([u[i]+0.5*small*a[i] for i in range(n)],k,g,c)
        d=rhs_float([u[i]+0.5*small*b[i] for i in range(n)],k,g,c)
        e=rhs_float([u[i]+small*d[i] for i in range(n)],k,g,c)
        u=[u[i]+(small/6)*(a[i]+2*b[i]+2*d[i]+e[i]) for i in range(n)]
        assert all(math.isfinite(v) for v in u),(idx,u)
        ref.append(tuple(f_to_bits(v) for v in u))
    return ref,F.from_float(small)


def hermite_coeff(y0,y1,dy0,dy1,h):
    return [y0, h*dy0, -3*y0-2*h*dy0+3*y1-h*dy1,
            2*y0+h*dy0-2*y1+h*dy1]


def original_defect_coeffs(source,y0,y1,h):
    """Fully exact rational coefficients of X'(t)-f(X(t)), in s=(t-t0)/h."""
    n=source['n'];k=source['k'];g=source['g'];c=source['c']
    dy0=rhs_rational(y0,source);dy1=rhs_rational(y1,source)
    p=[hermite_coeff(y0[i],y1[i],dy0[i],dy1[i],h) for i in range(n)]
    cubic=[mul(mul(q,q),q) for q in p]
    co=[]
    for i in range(n):
        ai=k[i]+sum(c[i])
        b=[F(0)]*10
        for j in range(3):b[j]+=(j+1)*p[i][j+1]/h
        for j in range(4):
            b[j]+=ai*p[i][j]
            for l in range(n): b[j]-=c[i][l]*p[l][j]
        for j in range(10):b[j]+=g[i]*cubic[i][j]
        co.append(b)
    return co,p


def defect_panel(source,y0,y1,h,audit=False):
    coefficients,poly=original_defect_coeffs(source,y0,y1,h)
    residual_bern=[coeffs_bernstein9(c) for c in coefficients]
    bound=max(abs(v) for part in residual_bern for v in part)
    if audit:
        # Independent exact identity checks at rational points including ends.
        for s in (F(0),F(1,7),F(1,2),F(5,7),F(1)):
            X=[polynomial_eval(p,s) for p in poly]
            dX=[polynomial_eval([j*p[j]/h for j in range(1,4)],s) for p in poly]
            original=[dx-f for dx,f in zip(dX,rhs_rational(X,source))]
            assert original==[polynomial_eval(c,s) for c in coefficients],('ORIGINAL_DEFECT_MISMATCH',s)
            assert original==[bernstein9_eval(b,s) for b in residual_bern],('BERNSTEIN_IDENTITY_MISMATCH',s)
            assert all(abs(z)<=bound for z in original),('FALSE_BERNSTEIN_BOUND',s)
    return bound


def certify_reference(source,coarse_steps,micro,reference=None,do_audit=True):
    if reference is None:
        reference,h=generate_reference(source,coarse_steps,micro)
    else:
        assert len(reference)==coarse_steps*micro+1
        h=source['h']/micro
    assert len(reference)==coarse_steps*micro+1
    hF=source['h']/micro
    assert h==hF
    assert list(map(rational,reference[0]))==source['y0']
    q_upper=upper_as_float(1/(1+min(source['k'])*h))
    d=0.0;grid=[F(0)];max_all=F(0); max_panel=F(0)
    error_bounds=[F(0)];panel_reports=[]
    audit_count=0
    for j in range(coarse_steps*micro):
        left=list(map(rational,reference[j]));right=list(map(rational,reference[j+1]))
        R=defect_panel(source,left,right,h,audit=(do_audit and j in (0,1,17,coarse_steps*micro-1)))
        if do_audit and j in (0,1,17,coarse_steps*micro-1):audit_count+=1
        max_panel=max(max_panel,R)
        # exact real expression, rounded upward once to binary64; current
        # d and q_upper are already valid binary64 upper bounds.
        max_all=max(max_all,F.from_float(d)+h*R)
        next_d=F.from_float(d)*F.from_float(q_upper)+h*R
        d=upper_as_float(next_d)
        if (j+1)%micro==0:
            grid.append(F.from_float(d))
            error_bounds.append(F.from_float(d))
            panel_reports.append({"step":(j+1)//micro,"delta_upper":d})
    assert len(grid)==coarse_steps+1
    return {'micro':micro,'h_ref':h,'states':reference,'certified_delta_grid_exact':max(grid),
            'certified_delta_grid':float(max(grid)), 'certified_delta_overall':float(max(max_all,max(grid))),
            'max_defect_panel':float(max_panel),'reference_interval_checks':coarse_steps*micro,
            'audited_intervals':audit_count,'delta_by_coarse_step':panel_reports,
            'q_decay_upper':q_upper}


def parse_log(path):
    from collections import defaultdict
    traces={};bench=defaultdict(list);parent=[]
    with open(path,encoding='utf-8') as f:
        for line in f:
            if 'VWP_TRACE=' in line:
                z=json.loads(line.split('VWP_TRACE=',1)[1]);key=(z['name'],int(z['alpha_id']),bool(z['fast']))
                assert key not in traces;traces[key]=z
            elif 'VWP_BENCH name=' in line:
                parent.append(line)
    assert len(traces)==40 and all((c,a,fast) in traces for c in COHORTS for a in range(5) for fast in (False,True))
    return traces,parent


def achieved_intervals(traces,ref,delta,micro):
    from collections import defaultdict
    q=defaultdict(dict)
    for (name,aid,fast),z in traces.items():
        if name!=ref['name']:continue
        assert len(z['points'])==ref['steps']+1
        achieved=F(0)
        for idx,point in enumerate(z['points']):
            state=list(map(rational,point['state']))
            r=list(map(rational,ref['states'][idx*micro]))
            assert len(state)==len(r)
            achieved=max(achieved,*(abs(x-y) for x,y in zip(state,r)))
        lower=max(F(0),achieved-delta)
        upper=achieved+delta
        assert lower<=upper
        q[aid][fast]={'observed_error':achieved,'lower':lower,'upper':upper,'source_calls':z['map_attempted']}
    return q


def pair_certificate(a,b,eta=F(1,100)):
    # Exact rational relative-parity sufficient condition for the *true*
    # achieved peak-global error, even if reference generator was garbage.
    if min(a['lower'],b['lower'])<=0:return False,None
    ratio=max(a['upper']/b['lower'],b['upper']/a['lower'])
    return ratio<=1+eta,ratio


def exact_crossarm_delta(z1,z2):
    out=F(0)
    for p1,p2 in zip(z1['points'],z2['points']):
        a=list(map(rational,p1['state']));b=list(map(rational,p2['state']))
        out=max(out,*(abs(x-y) for x,y in zip(a,b)))
    return out


def fraction_to_text(x):
    return format(float(x),'.16e')


def read_states_from_bytes(path,dim,panel_count):
    b=Path(path).read_bytes()
    assert len(b)==(panel_count+1)*dim*8,('REFERENCE_BITS_LENGTH_MISMATCH',path,len(b))
    return [tuple(struct.unpack('>'+('Q'*dim),b[i*dim*8:(i+1)*dim*8])) for i in range(panel_count+1)]


def run_analysis(logpath,out_dir,wp1_summary_path,micros=(64,),cases=COHORTS,run_selftests=True,readback_dir=None):
    assert micros and all(x in MICRO_CHOICES for x in micros)
    outdir=Path(out_dir);outdir.mkdir(parents=True,exist_ok=True)
    logpath=Path(logpath)
    digest=hashlib.sha256(logpath.read_bytes()).hexdigest()
    assert digest==EXPECTED_SWEEP_SHA256, ('WP1_IMMUTABLE_SOURCE_LOG_CHANGED',digest)
    wp1_file=Path(wp1_summary_path)
    summary_digest=hashlib.sha256(wp1_file.read_bytes()).hexdigest()
    assert summary_digest==EXPECTED_WP1_SUMMARY_SHA256, ('WP1_ORIGINAL_SELECTION_CHANGED',summary_digest)
    parent_summary=json.loads(wp1_file.read_text())
    assert parent_summary['native_trace_count']==40 and parent_summary['candidate_pair_count']==100
    traces,_=parse_log(logpath)
    if run_selftests:selftest()
    summary={'schema':'WP2_reference_defect_1','source_run_id':41181475675,
         'source_head':'72bd52d6936b923bd110222f36df79172b9ced49',
         'source_log_sha256':digest, 'prior_selected_pair_file_sha256':summary_digest,
         'reference_mechanism':'float RK4 predictions; exact Fraction Hermite/Bernstein post-validation',
         'proof_scope':'dyadic original source, exact real ODE, coarse grid t_n=n*h, true error interval rigorous, no RODAS8',
         'micro_cohorts':{},'candidate_counts':{},'matched_achieved_parity_1pct':{},
         'unresolved':[]}
    for name in cases:
        assert name in COHORTS
        refsource=traces[(name,2,False)]
        source=parse_source(refsource)
        steps=len(refsource['points'])-1
        M0=max(source['y0']);max_rhs=max(source['k'][i]*M0+source['g'][i]*M0**3+2*sum(source['c'][i])*M0 for i in range(source['n']))
        # Verify the source's real mathematical clock t_n=n*h against the
        # recorded f64 clock label. An exact physical time label may differ
        # by a small dyadic displacement, which is bounded using |f|<=max_rhs
        # on the exact invariant positive box [0,M0]^d.
        clock_bound=F(0)
        for ai in range(5):
            for arm in (False,True):
                tr=traces[(name,ai,arm)]
                assert tr['source_bits']==refsource['source_bits'] and tr['h_bits']==refsource['h_bits']
                for i,point in enumerate(tr['points']):
                    clock_bound=max(clock_bound,max_rhs*abs(rational(point['time'])-i*source['h']))
        group=[]
        for micro in micros:
            start=time.monotonic()
            bin_name=f'{name}_micro{micro}_reference_state_bits.bin'
            if readback_dir is None:
                reference,h_ref=generate_reference(source,steps,micro)
                raw=b''.join(struct.pack('>Q',bit) for states in reference for bit in states)
                (outdir/bin_name).write_bytes(raw)
            else:
                reference=read_states_from_bytes(Path(readback_dir)/bin_name,source['n'],steps*micro)
            cert=certify_reference(source,steps,micro,reference=reference)
            dt=time.monotonic()-start
            delta=cert['certified_delta_grid_exact']+clock_bound
            ref={'name':name,'steps':steps,'states':cert['states']}
            err=achieved_intervals(traces,ref,delta,micro)
            score=[]; pairs=[]
            for aid in range(5):
                a,b=err[aid][False],err[aid][True]
                ismatch,ratio=pair_certificate(a,b)
                observed_relative=abs(a['observed_error']-b['observed_error'])/min(a['observed_error'],b['observed_error'])
                pair_info={'alpha_id':aid,'alpha':[0.0625,0.25,1,4,16][aid],
                    'same_alpha':True,'guaranteed_1pct':ismatch,
                    'observed_gap':float(observed_relative),
                    'worst_ratio':float(ratio) if ratio is not None else None,
                    'baseline_error_interval':[float(a['lower']),float(a['upper'])],
                    'fast_error_interval':[float(b['lower']),float(b['upper'])],
                    'baseline_calls':a['source_calls'],'fast_calls':b['source_calls']}
                pairs.append(pair_info)
            for ia in range(5):
                for ib in range(5):
                    a,b=err[ia][False],err[ib][True]
                    good,ratio=pair_certificate(a,b)
                    obs_gap=abs(a['observed_error']-b['observed_error'])/min(a['observed_error'],b['observed_error'])
                    score.append((obs_gap,ia,ib,good,ratio,a,b))
            winner=min(score,key=lambda x:(x[0],x[5]['source_calls']+x[6]['source_calls'],x[1],x[2]))
            wg,ia,ib,can_certify,ratio,aa,bb=winner
            selected_previous=parent_summary['cases'][name]['best_finite_sweep_pair']
            previous_baseline=int(selected_previous['alpha_baseline'])
            previous_fast=int(selected_previous['alpha_fast'])
            assert 0<=previous_baseline<5 and 0<=previous_fast<5
            ca,cb=err[previous_baseline][False],err[previous_fast][True]
            prev_certified,prev_ratio=pair_certificate(ca,cb)
            certificate={'reference_micro':micro,'reference_delta_grid_up':cert['certified_delta_grid'],
                 'clock_bound_exact':str(clock_bound), 'clock_bound_float':float(clock_bound),
                 'total_reference_plus_clock_delta_up':upper_as_float(delta),
                 'PREREGISTERED_WP1_SELECTED_PAIR':{
                    'alpha_baseline':selected_previous['alpha_baseline'],
                    'alpha_fast':selected_previous['alpha_fast'],
                    'certified_1pct':prev_certified,
                    'worst_possible_true_error_ratio':float(prev_ratio) if prev_ratio is not None else None,
                    'baseline_error_interval':[float(ca['lower']),float(ca['upper'])],
                    'fast_error_interval':[float(cb['lower']),float(cb['upper'])],
                    'source_calls':[ca['source_calls'],cb['source_calls']]},
                 'reference_delta_full_up':cert['certified_delta_overall'],
                 'max_defect_panel':cert['max_defect_panel'],
                 'proof_audited_intervals':cert['audited_intervals'],
                 'reference_panels':cert['reference_interval_checks'],
                 'same_alpha':pairs,
                 'best_empirical_pair':{'alpha_baseline':[.0625,.25,1,4,16][ia],
                        'alpha_fast':[.0625,.25,1,4,16][ib],
                        'observed_gap':float(wg),
                        'certified_1pct':can_certify,
                        'worst_error_ratio':float(ratio) if ratio is not None else None,
                        'baseline_interval':[float(aa['lower']),float(aa['upper'])],
                        'fast_interval':[float(bb['lower']),float(bb['upper'])],
                        'baseline_calls':aa['source_calls'],'fast_calls':bb['source_calls']},
                 'number_certified_same_alpha':sum(p['guaranteed_1pct'] for p in pairs),
                 'number_certified_all25':sum(bool(x[3]) for x in score)}
            statehash=hashlib.sha256()
            for line in cert['states']:
                for x in line:statehash.update(struct.pack('>Q',x))
            certificate['reference_states_sha256']=statehash.hexdigest()
            certificate['reference_states_name']=bin_name
            expected_state_bytes=(steps*micro+1)*source['n']*8
            assert len((outdir/bin_name).read_bytes() if readback_dir is None
                       else (Path(readback_dir)/bin_name).read_bytes())==expected_state_bytes
            if readback_dir is not None:
                # Reproduce a saved binary64 reference exactly; the theorem
                # relies only on these bits and original source, not the predictor.
                assert hashlib.sha256((Path(readback_dir)/bin_name).read_bytes()).hexdigest()==statehash.hexdigest()
            group.append(certificate)
            print(f"WP2_CERT name={name} micro={micro} panels={cert['reference_interval_checks']} delta={cert['certified_delta_grid']:.4e} clock={float(clock_bound):.4e} maxR={cert['max_defect_panel']:.4e} same_alpha={certificate['number_certified_same_alpha']}/5 WP1_FROZEN_PAIR={previous_baseline},{previous_fast} WP1_CERTIFIED={prev_certified} max_ratio={certificate['PREREGISTERED_WP1_SELECTED_PAIR']['worst_possible_true_error_ratio']} elapsed={dt:.2f}s",flush=True)
        summary['micro_cohorts'][name]=group
    summary['total_panels']=sum(r['reference_panels'] for vv in summary['micro_cohorts'].values() for r in vv)
    summary['frozen_prior_pairs_rigorously_certified']=sum(int(r['PREREGISTERED_WP1_SELECTED_PAIR']['certified_1pct']) for vv in summary['micro_cohorts'].values() for r in vv)
    output=outdir/'CERTIFICATE.json'
    output.write_text(json.dumps(summary,indent=2,sort_keys=True)+'\n')
    print(f'WP2_CERT_RESULT={output} SHA256={hashlib.sha256(output.read_bytes()).hexdigest()}',flush=True)
    return summary


def selftest():
    # Known coefficient shape: x^3 as power vs degree9 Bernstein polynomial.
    for s in [F(0),F(1,7),F(1,2),F(2,3),F(1)]:
        coeff=[F(3),F(-2),F(5),F(4),F(0),F(0),F(2),F(0),F(0),F(-1)]
        bern=coeffs_bernstein9(coeff)
        assert polynomial_eval(coeff,s)==bernstein9_eval(bern,s)
    # Strong adversarial negative: a false proposed envelope zero must be rejected.
    src={'n':2,'k':[F(4),F(7)],'g':[F(20),F(40)],'c':[[F(0),F(12)],[F(8),F(0)]],
         'y0':[F(1,5),F(3,25)],'h':F(1,1024)}
    a=src['y0'];b=[F(19,100),F(11,100)]
    R=defect_panel(src,a,b,src['h'],audit=True)
    assert R>0
    cs,_=original_defect_coeffs(src,a,b,src['h'])
    # Hermite interpolant enforces X'=f(X) at BOTH endpoints; the
    # adversarial false-zero bound must be tested at an interior time.
    assert any(polynomial_eval(c,F(1,2))!=0 for c in cs)
    assert all(abs(polynomial_eval(c,F(1,2)))<=R for c in cs)
    assert any(abs(polynomial_eval(c,F(1,2)))>0 for c in cs), 'FALSE_ZERO_BOUND_NOT_REFUTED'
    assert upper_as_float(F(1,10))>=.1
    assert F.from_float(upper_as_float(F(1,10)))>=F(1,10)
    assert not pair_certificate({'lower':F(0),'upper':F(1)}, {'lower':F(0),'upper':F(1)})[0]
    print('WP2_SELFTEST=PASS Bernstein identity, original cubic defect, false zero envelope rejected, outward conversion, parity lower-zero',flush=True)

if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--log',required=True)
    parser.add_argument('--out',required=True)
    parser.add_argument('--wp1-summary',required=True)
    parser.add_argument('--micro',type=int,nargs='+',default=[64],choices=MICRO_CHOICES)
    parser.add_argument('--cases',nargs='+',choices=COHORTS,default=list(COHORTS))
    parser.add_argument('--readback-dir',default=None,help='Use frozen binary64 reference bits instead of re-generating RK4 predictor')
    a=parser.parse_args()
    run_analysis(a.log,a.out,a.wp1_summary,tuple(a.micro),tuple(a.cases),readback_dir=a.readback_dir)
