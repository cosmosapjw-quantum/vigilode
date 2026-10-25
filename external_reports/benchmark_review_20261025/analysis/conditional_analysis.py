#!/usr/bin/env python3
"""Reaggregate immutable published conditional experiments; no solver execution.

Run from any directory. Figures show observations, not fitted guarantees.
No cross-host/cross-problem ranking or bootstrap over in-process repetitions.
"""
from pathlib import Path
from collections import Counter, defaultdict
from statistics import median
import hashlib,json,math
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.ticker import NullFormatter

ROOT=Path(__file__).resolve().parents[3]
BASE=ROOT/'research/local_frontier_20261024'
OUT=Path(__file__).resolve().parent
FIG=OUT.parent/'figures'
FIG.mkdir(exist_ok=True)
sources=[]
def rows(rel,kind=None):
    p=BASE/rel
    content=p.read_bytes()
    data=[json.loads(l) for l in content.decode().splitlines() if l.startswith('{')]
    selected=[r for r in data if kind is None or r.get('kind')==kind]
    sources.append(dict(path=str(p.relative_to(ROOT)),sha256=hashlib.sha256(content).hexdigest(),selector=f'kind == {kind}' if kind else 'all JSON object lines',input_rows=len(data),selected_rows=len(selected)))
    return selected

def stats(vals):
    return dict(count=len(vals),median=median(vals),minimum=min(vals),maximum=max(vals))

def by(rs,keys,value):
    d=defaultdict(list)
    for r in rs:d[tuple(r[k] for k in keys)].append(r[value])
    return {k:stats(v) for k,v in d.items()}

plt.rcParams.update({'font.family':'DejaVu Sans','font.size':10,'axes.titlesize':12,'axes.labelsize':10,'legend.fontsize':8.5,'axes.spines.top':False,'axes.spines.right':False,'savefig.dpi':200,'svg.fonttype':'none'})
C=['#176b9d','#d46a1e','#24865e','#9564ac','#69747b','#a63b57']
def save(fig,name):
    for ext in ['png','svg']:fig.savefig(FIG/f'{name}.{ext}',bbox_inches='tight',facecolor='white')
    plt.close(fig)
def errline(ax,x,st,label,c,marker='o',scale=1):
    y=np.array([s['median']*scale for s in st]); lo=np.array([s['minimum']*scale for s in st]); hi=np.array([s['maximum']*scale for s in st]);
    ax.errorbar(x,y,yerr=[y-lo,hi-y],label=label,color=c,marker=marker,linewidth=1.5,capsize=2.5,markersize=5)

# Exact polynomial identity and strongest direct comparator.
chart=rows('evidence/n1n2-chart-benchmark.stdout')
assert len(chart)==18 and all(r['accepted'] for r in chart)
chart_table=[]
for r in chart:
    chart_table.append(dict(dimension=r['dimension'],method=r['method'],seconds=stats(r['seconds']),max_reference_error=r['max_reference_error'],max_original_residual=r['max_original_residual']))
chart_idx={(x['dimension'],x['method']):x for x in chart_table}
chart_speeds=[]
for n in [32,128,512]:
    ch=chart_idx[n,'conditioned_reused']['seconds']['median']
    chart_speeds.append(dict(dimension=n,reused_lu_over_chart=chart_idx[n,'dense_lu_reused']['seconds']['median']/ch,chart_over_direct=ch/chart_idx[n,'nilpotent_direct']['seconds']['median']))

# All conditional-chart outcomes retained in summary; plot the approximate nonnormal case.
cc=rows('evidence/runs/conditional-chart-round3-sweep.stdout','result')
assert len(cc)==384
cc_grp=by(cc,['case','action','error_limit','method'],'elapsed_seconds')
cc_table=[]
for k,s in sorted(cc_grp.items()):
    rr=[r for r in cc if (r['case'],r['action'],r['error_limit'],r['method'])==k]
    direct=cc_grp[k[:3]+('direct',)]['median']
    cc_table.append(dict(case=k[0],action=k[1],tolerance=k[2],method=k[3],seconds=s,ratio_to_direct=s['median']/direct if all(r['final_accepted'] for r in rr) else None,all_final_accepted=all(r['final_accepted'] for r in rr),chart_accepts=sum(r['chart_accepted'] for r in rr),fallbacks=sum(r['fallback_solves'] for r in rr),max_final_error_upper=max(r['actual_error_upper'] for r in rr),last_chart_error_upper=max([r['last_chart_error_upper'] for r in rr if r['last_chart_error_upper'] is not None],default=None),reasons=dict(Counter(r['refusal_reason'] for r in rr))))
fig,(ax,bx)=plt.subplots(2,1,figsize=(7.1,8.9),layout='constrained')
for j,(method,label) in enumerate([('dense_lu_fresh','Dense LU, fresh'),('dense_lu_reused','Dense LU, reused'),('conditioned_fresh_setup','Polynomial, fresh'),('conditioned_reused','Polynomial, reused'),('nilpotent_direct','Structure-specific direct')]):
    ns=[32,128,512]
    errline(ax,ns,[chart_idx[n,method]['seconds'] for n in ns],label,C[j],scale=1e6)
ax.set(xscale='log',yscale='log',xlabel='State dimension n',ylabel='Time per linear solve (µs)',title='A  Exact low-degree structure')
ax.set_xticks([32,128,512],['32','128','512']);ax.legend(loc='upper left');ax.grid(alpha=.18,which='both')
# Refinement accuracy is the candidate before fallback for no_refinement; final route cost disclosed.
sel=[]
for action in ['structured','dense']:
    for tol in [1e-5,1e-9]:
        rr=[r for r in cc if r['case']=='nonnormal' and r['action']==action and r['error_limit']==tol and r['method']=='refinement']
        assert all(r['chart_accepted'] for r in rr)
        d=cc_grp['nonnormal',action,tol,'direct']['median']
        s=stats([r['elapsed_seconds']/d for r in rr]);error=max(r['actual_error_upper'] for r in rr)
        sel.append(dict(action=action,tolerance=tol,time_over_direct=s,attained_error_upper=error,corrections=max(r['corrections'] for r in rr)))
for action,c,m in [('structured',C[0],'o'),('dense',C[1],'s')]:
    ss=[r for r in sel if r['action']==action]
    bx.plot([r['attained_error_upper'] for r in ss],[r['time_over_direct']['median'] for r in ss],marker=m,color=c,label=f'{action.capitalize()} operator action')
    for r in ss:bx.annotate(f"τ={r['tolerance']:.0e}\n{r['corrections']} corrections",(r['attained_error_upper'],r['time_over_direct']['median']),xytext=(5,5),textcoords='offset points',fontsize=8)
bx.axhline(1,color='#555',lw=1,ls='--',label='Direct solve + same final check')
bx.set(xscale='log',xlabel='Attained absolute error upper bound',ylabel='Complete route time / direct route time',title='B  Refinement of a nonnormal candidate',ylim=(.8,4.05))
bx.grid(alpha=.18);bx.legend(loc='upper right')
save(fig,'conditional_polynomial')

# Continuation: published file stores medians (31 in-process calls), no fabricated spread.
con=rows('evidence/conditional-continuation/round3-sweep.jsonl','result');assert len(con)==16
ct=[]
for r in con:
    ct.append(dict(case=r['case_id'],n=r['config']['n'],p=r['config']['p'],tolerance=r['config']['tolerance'],seconds_median=r['seconds_median'],raw_status=r['raw']['status'],conditional_status=r['conditional']['status'],raw_root_bound=r['raw'].get('root_bound'),conditional_root_bound=r['conditional'].get('root_bound'),raw_f_calls=r['raw']['work']['residual_calls'],conditional_f_calls=r['conditional']['work']['residual_calls'],raw_error=r['raw'].get('error_to_reference'),conditional_error=r['conditional'].get('error_to_reference'),eligible=r['speed_comparison_eligible'],q_upper=r['planning']['q_upper']))
fig,(ax,bx)=plt.subplots(2,1,figsize=(7.1,8.9),layout='constrained')
ss=[r for r in con if r['case_id'].startswith('n') and r['speed_comparison_eligible']]
y=np.arange(len(ss));
for j,(m,label) in enumerate([('raw','Fixed-P'),('conditional','Fixed-P + checks'),('bisection_final_only','Bisection, final target only')]):
    ax.scatter([r['seconds_median'][m]*1e6 for r in ss],y+(j-1)*.18,color=C[j],label=label,s=30,marker=['o','s','^'][j])
ax.set_yticks(y,[f"n={r['config']['n']}, p={r['config']['p']:g}, τ={r['config']['tolerance']:.0e}" for r in ss]);ax.invert_yaxis()
ax.set(xscale='log',xlabel='Median root-client time (µs)',title='A  Contraction strength controls cost');ax.grid(axis='x',alpha=.18);ax.legend(loc='upper right')
r=next(r for r in con if r['case_id']=='root_budget_refinement')
for j,(arm,label) in enumerate([('raw','Residual criterion'),('conditional','Added root-error criterion')]):
    bx.scatter(r[arm]['root_bound'],r['seconds_median'][arm]*1e6,color=C[j],s=85,label=label)
    bx.annotate(f"{r[arm]['work']['residual_calls']} F calls",(r[arm]['root_bound'],r['seconds_median'][arm]*1e6),xytext=(8,4),textcoords='offset points')
bx.plot([r['raw']['root_bound'],r['conditional']['root_bound']],[r['seconds_median']['raw']*1e6,r['seconds_median']['conditional']*1e6],color='#aaa',lw=1,zorder=0)
bx.axvline(1e-10,color='#777',ls='--',lw=1,label='Requested root-error bound')
bx.set(xscale='log',xlabel='Certified root-error upper bound',ylabel='Median root-client time (µs)',title='B  Purchasing extra root accuracy',ylim=(5.8,14.0),xlim=(1e-12,2e-6));bx.legend(loc='lower left');bx.grid(alpha=.18)
save(fig,'conditional_continuation')

# Dense vs sparse certificates and identical mathematical original residual.
struc=rows('evidence/STRUCTURED_CERTIFICATION.jsonl','measurement');assert len(struc)==81 and all(r['certified'] for r in struc)
sd=[]
for k,vals in by(struc,['fixture','dimension','mode'],'certification_seconds').items():
    rr=[r for r in struc if (r['fixture'],r['dimension'],r['mode'])==k]
    sd.append(dict(fixture=k[0],dimension=k[1],mode=k[2],certificate=vals,solve=stats([r['solve_seconds'] for r in rr]),total=stats([r['total_seconds'] for r in rr]),max_error_upper=max(r['error_l2_upper'] for r in rr),operations=rr[0]['residual_operations']))
si={(x['fixture'],x['dimension'],x['mode']):x for x in sd}
sp=[]
for fixture in sorted({r['fixture'] for r in struc}):
 for n in [32,128,512]:
    sparse=si[fixture,n,'structured-directed'];dense=si[fixture,n,'dense-directed']
    sp.append(dict(fixture=fixture,n=n,certificate_speedup=dense['certificate']['median']/sparse['certificate']['median'],sparse_certificate_over_bare_solve=sparse['certificate']['median']/sparse['solve']['median'],dense_total_over_sparse_total=dense['total']['median']/sparse['total']['median'],dense_certificate_fraction=dense['certificate']['median']/dense['total']['median'],sparse_certificate_fraction=sparse['certificate']['median']/sparse['total']['median']))
module=rows('evidence/N6_microbenchmark.jsonl','measurement');assert len(module)==135 and all(r['accepted'] for r in module)
md=[]
for k,s in by(module,['dimension','mode'],'seconds').items():md.append(dict(n=k[0],mode=k[1],seconds=s))
mi={(x['n'],x['mode']):x['seconds'] for x in md}
fig,(ax,bx)=plt.subplots(2,1,figsize=(7.1,8.9),layout='constrained')
fixture='nonnormal-bidiagonal'
if fixture not in {r['fixture'] for r in struc}:fixture=sorted({r['fixture'] for r in struc})[1]
for j,(mode,key,label) in enumerate([('dense-directed','certificate','Dense residual check'),('structured-directed','certificate','Sparse residual check'),('structured-directed','solve','Underlying direct solve')]):
    errline(ax,[32,128,512],[si[fixture,n,mode][key] for n in [32,128,512]],label,C[j],scale=1e6)
ax.set(xscale='log',yscale='log',xlabel='State dimension n',ylabel='Component time (µs)',title='A  The cost of checking the original equation');ax.set_xticks([32,128,512],['32','128','512']);ax.legend(loc='upper left');ax.grid(alpha=.18,which='both')
ns=sorted({r['dimension'] for r in module})
for j,(mode,label) in enumerate([('new_module','Reduced solve, fresh'),('reuse_module','Reduced solve, reused'),('fresh_full_lu','Dense LU, fresh'),('reuse_full_lu','Dense LU, reused'),('direct_jordan_blocks','Structure-specific direct')]):
    errline(bx,ns,[mi[n,mode] for n in ns],label,C[j],scale=1e6)
bx.set(xscale='log',yscale='log',xlabel='State dimension n',ylabel='Solve + full residual check (µs)',title='B  Reduction with the same dense check');bx.set_xticks(ns,[str(n) for n in ns]);bx.xaxis.set_minor_formatter(NullFormatter());bx.legend(loc='upper left');bx.grid(alpha=.18,which='both')
save(fig,'conditional_certification')

# Stage parallelism and reference locality checking are kept at trajectory level.
n5=[]
for t in [1,2]:
    rr=rows(f'evidence/runs/n5-final-meanfield-{t}.stdout','sample')
    for r in rr:r['threads']=t
    n5+=rr
assert len(n5)==48 and all(r['result']['success'] for r in n5)
ng=by(n5,['mode','threads'],'elapsed_seconds')
n5summary=[]
for m in sorted({r['mode'] for r in n5}):
    s1=ng[m,1]['median'];s2=ng[m,2]['median']
    n5summary.append(dict(method=m,seconds_1=ng[m,1],seconds_2=ng[m,2],same_method_speedup=s1/s2,serial_dense_over_parallel=ng['protected-dense',1]['median']/s2,max_abs_error=max(r['result']['errors']['max_abs_error'] for r in n5 if r['mode']==m)))
n3=[]
for p in sorted((BASE/'evidence/runs').glob('n3-final-*.stdout')):
    if not any(t in p.name for t in ['tests','build']):
        rr=rows(str(p.relative_to(BASE)),'sample')
        if rr:
            gg=by(rr,['mode'],'elapsed_seconds')
            if ('guarded-switching-causal',) not in gg:continue
            guard=gg['guarded-switching-causal',]
            priced=gg['switching-causal',]
            n3.append(dict(case=p.name.removesuffix('.stdout').removeprefix('n3-final-'),samples=len(rr),guard=guard,priced=priced,guard_over_priced=guard['median']/priced['median'],max_abs_error=max(r['result']['errors']['max_abs_error'] for r in rr),all_success=all(r['result']['success'] for r in rr)))
fig,(ax,bx)=plt.subplots(2,1,figsize=(7.1,9.2),layout='constrained')
ms=['protected-dense','protected-raw','fixed-causal','fixed-picard','fixed-residual-first','switching-causal','switching-picard','switching-residual-first']
labels=['Dense linear solves','Standard matrix-free','Causal forward','Picard correction','Residual-first','Adaptive forward','Adaptive Picard','Adaptive residual-first']
y=np.arange(len(ms))
for j,t in enumerate([1,2]):
    st=[ng[m,t] for m in ms];med=np.array([s['median']*1e3 for s in st]);lo=np.array([s['minimum']*1e3 for s in st]);hi=np.array([s['maximum']*1e3 for s in st]);
    ax.errorbar(med,y+(j-.5)*.2,xerr=[med-lo,hi-med],fmt='o',capsize=2,color=C[j],label=f'{t} worker'+('s' if t==2 else ''))
ax.set_yticks(y,labels);ax.invert_yaxis();ax.set(xscale='log',xlabel='Complete trajectory time (ms)',title='A  Nonlinear all-pair phase model, n=65');ax.legend();ax.grid(axis='x',alpha=.18)
y=np.arange(len(n3));bx.scatter([r['guard_over_priced'] for r in n3],y,color=C[0],s=36)
for i,r in enumerate(n3):bx.annotate(f"{r['guard_over_priced']:.2f}×",(r['guard_over_priced'],i),xytext=(5,0),textcoords='offset points',va='center',fontsize=8)
bx.axvline(1,color='#555',ls='--',lw=1);bx.set_yticks(y,[r['case'].replace('nonlinear-','NL ').replace('-',' ') for r in n3]);bx.invert_yaxis();bx.set(xlabel='Checked / unchecked median trajectory time',title='B  Locality checks on cost references',xlim=(0,2.6));bx.grid(axis='x',alpha=.18)
save(fig,'conditional_adaptation')

# Descriptive setup-amortisation model: first-use minus reused median approximates setup.
# Both routes pay their own first-use cost. Micro-timing variation is not a bound.
amort=[]
for n in ns:
    snew=mi[n,'new_module']['median']-mi[n,'reuse_module']['median']
    sbase=mi[n,'fresh_full_lu']['median']-mi[n,'reuse_full_lu']['median']
    saving=mi[n,'reuse_full_lu']['median']-mi[n,'reuse_module']['median']
    delta=snew-sbase
    threshold=max(1,math.floor(delta/saving)+1) if saving>0 else None
    amort.append(dict(dimension=n,estimated_incremental_setup_difference=delta,per_solve_median_saving=saving,strict_break_even_solve_count=threshold,interpretation='descriptive additive-cost projection from medians; not measured repeated-plan crossover; small differences require process-level replication'))
summary=dict(schema_version=1,source_snapshot='2a4bf90d1445b67542bdac5030b4c5d10da20445',measurements_origin='Published 2026-10-24 experiment data; reanalysis only on 2026-10-25. No solver rerun.',sources=sources,statistical_contract=dict(intervals='Observed min/max, never confidence intervals',experimental_units='Each original experiment ran in one process; loops within process are descriptive repeats. Continuation stores 31-repeat medians only.',exclusions='Warmups excluded. All refusal and unresolved rows retained in tables; only accepted comparable cases contribute speed ratios.',pooling='No pooling across operators, root clients, ODE trajectories or hosts.'),polynomial_exact=dict(rows=chart_table,comparisons=chart_speeds,timing='11 cyclic >=2ms groups per method in one process; fresh/reused setup separated'),polynomial_conditional=dict(rows=cc_table,status_counts=dict(Counter(r['scientific_status'] for r in cc)),refusal_counts=dict(Counter(r['refusal_reason'] for r in cc)),selected_nonnormal=sel,timing='3 in-process calibrated timing groups; calibration and one-time setup excluded, final residual plus fallback included'),continuation=dict(rows=ct,root_accuracy_tradeoff=dict(root_bound_reduction=next(x for x in con if x['case_id']=='root_budget_refinement')['raw']['root_bound']/next(x for x in con if x['case_id']=='root_budget_refinement')['conditional']['root_bound'],time_ratio=next(x for x in con if x['case_id']=='root_budget_refinement')['conditional_over_raw_time']),comparator='Bisection solves final target only; fixed-P continues lambda=0 to1. Same residual request; attained errors and root-error demands differ where shown.'),certification=dict(rows=sd,comparisons=sp,plot_fixture=fixture,timing='9 in-process timed repeats per dimension/method/fixture. Preparation reported separately; direct solution shared mathematically.'),invariant_module=dict(rows=md,amortization=amort,timing='9 in-process repetitions; dense original-space residual check included.'),stage_parallelism=n5summary,cost_reference_checks=n3)
(OUT/'conditional_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2,allow_nan=False)+'\n')
print(json.dumps(dict(figures=4,inputs=len(sources),polynomial_comparisons=chart_speeds,certificate_speedup_range=[min(x['certificate_speedup'] for x in sp),max(x['certificate_speedup'] for x in sp)],sparse_certificate_over_bare_range=[min(x['sparse_certificate_over_bare_solve'] for x in sp),max(x['sparse_certificate_over_bare_solve'] for x in sp)],root_accuracy_tradeoff=summary['continuation']['root_accuracy_tradeoff'],guard_ratio_range=[min(x['guard_over_priced'] for x in n3),max(x['guard_over_priced'] for x in n3)]),indent=2))
