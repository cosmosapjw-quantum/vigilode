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


# WP1 source-scoped analysis. No Rust or floating source formulas imported.
import csv
import re
from pathlib import Path

ALPHAS = (0.0625, 0.25, 1.0, 4.0, 16.0)
COHORTS = ('asymmetric_2', 'asymmetric_2_refined', 'coupled_3_cubic', 'linear_exchange_control')
TARGET_CAPS = ('0.00005', '0.0001', '0.0002', '0.0004', '0.0008')
BENCH_REGEX = re.compile(r'VWP_BENCH name=(\S+) alpha_id=(\d+) block=(\d+) fast=(true|false) ns=(\d+) reps=(\d+) map=(\d+) original=(\d+) inverse=(\d+) checksum=(\d+)')


def exact_interarm_delta(traces_a, traces_b):
    assert len(traces_a) == len(traces_b)
    out = F(0)
    for a,b in zip(traces_a, traces_b):
        xa=list(map(fr, a['state']));xb=list(map(fr,b['state']))
        assert len(xa) == len(xb)
        out=max(out,*(abs(x-y) for x,y in zip(xa,xb)))
    return out


def abs_to_decimal(v):
    return Decimal(v.numerator)/Decimal(v.denominator)


def achieved(z, ref):
    """Exactly compare represented binary64 output with represented Decimal70 reference.

    Decimal70 RK4 is a nonrigorous ODE approximation. Its finite decimal output
    nonetheless has an EXACT rational interpretation. Use Fraction throughout
    to avoid a false reverse-triangle FAIL from default Decimal precision=28.
    """
    assert len(z['points']) == len(ref)
    errors=[]
    for i,p in enumerate(z['points']):
        state=list(map(fr,p['state']))
        ref_rational=[F(str(t)) for t in ref[i]]
        err=max(abs(state[j]-ref_rational[j]) for j in range(len(state)))
        assert err <= fr(p['E']) + F(1,10**10), ('BE_PHYSICAL_ENVELOPE_REFUTED',z['name'],i)
        errors.append(err)
    return max(errors), errors[-1]


def relgap(a,b):
    assert a>0 and b>0
    return abs(a-b)/min(a,b)


def parse_inputs(logpath):
    groups={}; timing=defaultdict(list); rejected=[]
    for line in open(logpath,encoding='utf8'):
        if 'VWP_TRACE=' in line:
            z=json.loads(line.split('VWP_TRACE=',1)[1].strip())
            k=(z['name'],int(z['alpha_id']),bool(z['fast']))
            assert k not in groups, ('DUPLICATE_TRACE',k)
            groups[k]=z
        elif 'VWP_BENCH name=' in line:
            m=BENCH_REGEX.search(line)
            assert m, ('MALFORMED_BENCH_LINE',line[:120])
            name,ai,block,fast,ns,reps,mapc,origc,invc,checksum=m.groups()
            k=(name,int(ai),int(block),fast=='true')
            timing[k].append(dict(ns=int(ns),reps=int(reps),map=int(mapc),original=int(origc),inverse=int(invc),checksum=int(checksum)))
        elif 'VWP_REJECTED=' in line:
            rejected.append(json.loads(line.split('VWP_REJECTED=',1)[1].strip()))
    assert set(name for name,_,_ in groups).issubset(set(COHORTS))
    for k,rs in timing.items():
        assert len(rs)==2 and all(x['reps']==8 for x in rs),('ABBA_TWO_PER_BLOCK_REQUIRED',k)
    return groups,timing,rejected


def summarize_work(logpath,summary_path):
    traces,times,rejected=parse_inputs(logpath)
    assert all((case,2,fast) in traces for case in COHORTS for fast in (False,True)), 'PARENT_1X_CASE_NOT_REPLAYED'
    assert len(traces)+len(rejected)==len(COHORTS)*len(ALPHAS)*2, 'SINGLE_RECORD_PER_SOURCE_ALPHA_ARM'
    # Source and clock are per-cohort; independent numerical reference computed once.
    refs={}; ref_gaps={}; reference_sources={}
    for name in COHORTS:
        first=traces[(name,2,False)]
        _,_,_,_,_,_,_=unpack(first)
        reference_sources[name]=first
        data=exact_certificate(first)
        N=data['N']; source=data['source']
        fine32=ref_trajectory(source,N,32)
        fine64=ref_trajectory(source,N,64)
        diff=max(max(abs(fine32[i][j]-fine64[i][j]) for j in range(data['n'])) for i in range(N+1))
        assert diff < Decimal('1e-10'),('NONRIGOROUS_REFERENCE_NOT_CONVERGED',name,str(diff))
        refs[name]=fine64
        ref_gaps[name]=diff

    rows=[]; index={}; achieved_rational={}; all_good=0
    for key,z in sorted(traces.items(),key=lambda x:(COHORTS.index(x[0][0]),x[0][1],x[0][2])):
        name,alpha_id,fast=key
        assert alpha_id in range(5)
        source=reference_sources[name]
        assert z['source_bits']==source['source_bits'] and z['h_bits']==source['h_bits']
        assert z['n']==source['n'] and len(z['points'])==len(source['points'])
        assert fr(z['parent_eps_bits'])==fr(source['parent_eps_bits'])
        assert fr(z['alpha_bits'])==F.from_float(ALPHAS[alpha_id])
        if alpha_id==2:
            assert z['eps_bits']==z['parent_eps_bits'], 'EXACT_PARENT_EPS_MUST_BE_PRESERVED'
        cert=exact_certificate(z)
        all_good+=1
        peak_rational,term_rational=achieved(z,refs[name])
        achieved_rational[key]=(peak_rational,term_rational)
        with localcontext() as ctx:
            ctx.prec=90
            epeak=abs_to_decimal(peak_rational)
            eterm=abs_to_decimal(term_rational)
        values=defaultdict(list)
        for block in range(5):
            samples=times[(name,alpha_id,block,fast)]
            assert len(samples)==2,('MISSING_TIMING_BLOCK',name,alpha_id,fast,block)
            for sample in samples:
                assert sample['map']==sample['reps']*z['map_attempted']
                assert sample['original']==sample['reps']*z['original_components']
                assert sample['inverse']==sample['reps']*z['inverse_components']
                values[block].append(sample['ns']/sample['reps'])
        wall_blocks=[sum(values[i])/len(values[i]) for i in range(5)]
        wall_median=sorted(wall_blocks)[2]
        r=dict(case=name,alpha_id=alpha_id,alpha=ALPHAS[alpha_id],fast=fast,
               eps=ff(z['eps_bits']),steps=cert['N'],n=cert['n'],
               achieved_peak=float(epeak),achieved_final=float(eterm),
               achieved_peak_str=str(epeak),achieved_final_str=str(eterm),
               numerical_ref_gap_est=float(ref_gaps[name]),
               certified_upper=cert['final_E_upper'],source_q=cert['source_q'],
               original_max_rho=cert['max_rho_exact'],map_attempted=z['map_attempted'],
               inverse_components=z['inverse_components'],original_components=z['original_components'],
               wall_ns_median_per_ivp=wall_median,wall_blocks_ns_per_ivp=wall_blocks)
        rows.append(r);index[key]=r
    # Explicit negative tests: falsify original residual and E at an actual nonzero step.
    adversarial=traces[(COHORTS[0],2,True)]
    import copy
    tamper=copy.deepcopy(adversarial)
    tamper['points'][1]['rho']=0
    try:
        exact_certificate(tamper)
        raise AssertionError('FALSE_ZERO_RESIDUAL_WAS_ACCEPTED')
    except AssertionError as e:
        assert 'FALSE_RESIDUAL_UPPER' in repr(e),('NEGATIVE_TEST_MISCLASSIFIED',repr(e))
    tamper=copy.deepcopy(adversarial)
    tamper['points'][1]['E']=0
    try:
        exact_certificate(tamper)
        raise AssertionError('FALSE_ZERO_GLOBAL_UPPER_WAS_ACCEPTED')
    except AssertionError as e:
        assert 'FALSE_GLOBAL_UPPER' in repr(e),('NEGATIVE_TEST_MISCLASSIFIED',repr(e))

    cases={}; pair_count=0; parity_1pct_cases=0; checks=0
    with localcontext() as ctx:
        ctx.prec=70
        for name in COHORTS:
            pairs=[]
            matched_pair_same_alpha=[]
            for i in range(5):
                a=traces.get((name,i,False));b=traces.get((name,i,True))
                if a is None or b is None: continue
                delta=exact_interarm_delta(a['points'],b['points'])
                ae=Decimal(index[(name,i,False)]['achieved_peak_str']);be=Decimal(index[(name,i,True)]['achieved_peak_str'])
                # Strong exact-rational self-audit, no arbitrary tolerance fudge.
                # The (nonrigorous) Decimal70 reference values are treated here
                # as *exact represented decimal rationals* when applying the
                # mathematical reverse triangle inequality.
                gap_exact=abs(achieved_rational[(name,i,False)][0]-achieved_rational[(name,i,True)][0])
                assert gap_exact <= delta,('REVERSE_TRIANGLE_FAILED_EXACT_RATIONAL',name,i,gap_exact,delta)
                checks+=1
                gap=relgap(ae,be)
                cost_a=index[(name,i,False)]['wall_ns_median_per_ivp']
                cost_b=index[(name,i,True)]['wall_ns_median_per_ivp']
                by_blocks=[]
                for block in range(5):
                    bas=sum(v['ns'] for v in times[(name,i,block,False)])
                    fast=sum(v['ns'] for v in times[(name,i,block,True)])
                    by_blocks.append(bas/fast)
                pair=dict(alpha_id=i,delta_exact_rational=str(delta),relative_peak_gap=float(gap),
                          observed_speedup_median=sorted(by_blocks)[2],
                          observed_speedup_min=min(by_blocks),observed_speedup_max=max(by_blocks),
                          achieved_peak_parity_1pct=gap<=Decimal('0.01'),
                          achieved_final_gap=float(relgap(Decimal(index[(name,i,False)]['achieved_final_str']),Decimal(index[(name,i,True)]['achieved_final_str']))))
                matched_pair_same_alpha.append(pair)
            for ia in range(5):
                for ib in range(5):
                    aa=index.get((name,ia,False));bb=index.get((name,ib,True))
                    if not aa or not bb:continue
                    ea=Decimal(aa['achieved_peak_str']); eb=Decimal(bb['achieved_peak_str'])
                    gap=relgap(ea,eb)
                    pairs.append(dict(alpha_baseline=ia,alpha_fast=ib,
                                      achieved_peak_gap=float(gap),
                                      achieved_parity_1pct=gap<=Decimal('0.01'),
                                      map_baseline=aa['map_attempted'],map_fast=bb['map_attempted'],
                                      time_baseline_ns=aa['wall_ns_median_per_ivp'],
                                      time_fast_ns=bb['wall_ns_median_per_ivp']))
            pair_count+=len(pairs)
            # Deterministic preregistered ranking: smallest gap; tie total calls then indices.
            best=min(pairs,key=lambda a:(a['achieved_peak_gap'],a['map_baseline']+a['map_fast'],a['alpha_baseline'],a['alpha_fast']))
            if best['achieved_parity_1pct']:parity_1pct_cases+=1
            frontier={}
            for t in TARGET_CAPS:
                cap=Decimal(t)
                opts=[]
                for arm in (False,True):
                    v=[index[(name,i,arm)] for i in range(5) if (name,i,arm) in index
                       and Decimal(index[(name,i,arm)]['achieved_peak_str'])<=cap]
                    pick=min(v,key=lambda a:(a['wall_ns_median_per_ivp'],a['alpha_id'])) if v else None
                    opts.append(pick)
                f={label:None if item is None else dict(alpha_id=item['alpha_id'],
                      achieved_peak=item['achieved_peak'],calls=item['map_attempted'],
                      wall_ns_median=item['wall_ns_median_per_ivp'])
                    for label,item in zip(('baseline','fast'),opts)}
                f['wall_speedup']=(opts[0]['wall_ns_median_per_ivp']/opts[1]['wall_ns_median_per_ivp']) if all(opts) else None
                frontier[t]=f
            orig=next(x for x in matched_pair_same_alpha if x['alpha_id']==2)
            print('VWP_CASE={} fixed_N={} original_alpha1_peak_gap={:.9g} original_alpha1_terminal_gap={:.9g} original_wall_ratio={:.6f} closest_pair=({},{}) closest_gap={:.9g} paired_1pct={} reference_delta_est={:.4e}'.format(name,index[(name,2,False)]['steps'],orig['relative_peak_gap'],orig['achieved_final_gap'],orig['observed_speedup_median'],best['alpha_baseline'],best['alpha_fast'],best['achieved_peak_gap'],best['achieved_parity_1pct'],ref_gaps[name]))
            cases[name]=dict(ref_delta_est=str(ref_gaps[name]),alpha1_same_eps=orig,
                             same_alpha_pairs=matched_pair_same_alpha,
                             best_finite_sweep_pair=best,work_precision_frontier=frontier,
                             n=len(pairs))
    summary=dict(unit='VIGILODE_WP1_ACHIEVED_ERROR_WORK_PRECISION_20261031',
                 reference_type='NUMERICAL_DECIMAL70_RK4_32_64_NOT_CERTIFIED',
                 native_trace_count=len(traces),ref_source_fraction_checks=all_good,
                 exact_rational_reverse_triangle_pair_checks=checks,
                 negative_tamper_checks=2,
                 complete_ABBA_timing_rows=sum(len(x) for x in times.values()),
                 failed_solver_samples=rejected,
                 cases=cases,frontier_caps=list(TARGET_CAPS),
                 candidate_pair_count=pair_count,
                 cases_with_any_empirical_1pct_achieved_global_parity=parity_1pct_cases,
                 claims=dict(physical_error_certification='SOURCE_SCOPED_ORIGINAL_BE',
                             reference_uncertainty='UNRESOLVED_REFERENCE_REFINEMENT_ONLY',
                             achieved_parity='NUMERICALLY_CHECKED_NOT_RIGOROUS',
                             production='HOLD',official_vigilode_dag='7/23'))
    Path(summary_path).parent.mkdir(parents=True,exist_ok=True)
    with open(summary_path,'w',encoding='utf8') as f:json.dump(summary,f,indent=2,ensure_ascii=False,sort_keys=True)
    csv_path=str(Path(summary_path).with_suffix('.csv'))
    cols=['case','alpha_id','alpha','fast','eps','steps','n','achieved_peak','achieved_final','certified_upper','map_attempted','original_components','inverse_components','wall_ns_median_per_ivp','numerical_ref_gap_est']
    with open(csv_path,'w',encoding='utf8',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=cols);writer.writeheader();writer.writerows({k:row[k] for k in cols} for row in rows)
    print(f'VWP_FRACTION_ORIGINAL_SOURCE={all_good}/{len(traces)}_PASS')
    print(f'VWP_REFERENCE_FREE_DELTA={checks}/{checks}_PASS')
    print('VWP_ADVERSARIAL_FALSE_RESIDUAL_AND_E=2/2_REJECTED')
    print(f'VWP_BENCHMARK_SAMPLES={sum(len(x) for x in times.values())}_PASS')
    print(f'VWP_DECIMAL70_REFERENCE=4/4_REFINEMENT_ONLY_NONRIGOROUS')
    print(f'VWP_CANDIDATE_PAIRS={pair_count} parity_cases={parity_1pct_cases}/4')
    print(f'VWP_SUMMARY_PATH={summary_path}')
    return summary

if __name__ == '__main__':
    if len(sys.argv)!=3: raise SystemExit('usage: python independent_work_precision.py executed_native_BENCH.log WP_SUMMARY.json')
    summarize_work(sys.argv[1],sys.argv[2])
