#!/usr/bin/env python3
"""Derive summaries/figures from preserved results, without executing solvers."""
import json,pathlib,statistics,hashlib
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
B=pathlib.Path(__file__).resolve().parents[1]
def read(p):return json.loads((B/p).read_text())
def write(p,d):(B/p).write_text(json.dumps(d,indent=2,ensure_ascii=False)+'\n')
sw=read('results/switching_campaign/SUMMARY.json');ca=read('results/causal_campaign/SUMMARY.json');af=read('results/AFFINE_SUMMARY.json')
pde=[json.loads(x) for x in (B/'results/PDE_NATIVE_BENCHMARK_FINAL.jsonl').read_text().splitlines()]
assert len(pde)==135 and all(x['relative_weighted_l2_error']<=1e-5 and x['hot_allocations']==0 for x in pde)
pde_groups=[]
for case in sorted(set(x['case'] for x in pde), key=lambda c: (c.split('_')[0], min(x['cells'] for x in pde if x['case']==c))):
 methods={}
 for m in ['euler','rkl2','uniformization']:
  rr=[x for x in pde if x['case']==case and x['method']==m]
  assert len(rr)==5
  methods[m]={'seconds_median':statistics.median(x['duration_ns'] for x in rr)/1e9,'seconds_min':min(x['duration_ns'] for x in rr)/1e9,'seconds_max':max(x['duration_ns'] for x in rr)/1e9,'worst_relative_error':max(x['relative_weighted_l2_error'] for x in rr),'actions':rr[0]['operator_actions'],'hot_allocations':max(x['hot_allocations'] for x in rr),'cells':rr[0]['cells']}
 for m in methods:methods[m]['euler_over_method']=methods['euler']['seconds_median']/methods[m]['seconds_median']
 pde_groups.append({'case':case,'methods':methods})
write('results/PDE_NATIVE_SUMMARY.json',{'schema':'vigilode.frontier-pde-native-summary.v1','source_receipt':'evidence/pde_campaign_final_receipt.json','rows':135,'accuracy_passes':135,'hot_allocation_max':0,'error_metric':'relative weighted L2 against exact discrete mode at T=.1','scope':'calibrated fixed linear diffusion; setup excluded; prototype not whole solver','groups':pde_groups})
summary={'schema':'vigilode.frontier-summary.v1','broad_production':'HOLD','native_targeted_tests':59,'native_pde_tests':9,'python_pde_tests':10,'python_pole_split_tests':3,'switching':{'timed_rows':120,'completed':114,'failed':6,'same_error_cap_passes':105,'same_error_cap_failures_among_completed':9,'not_eligible_for_same_error_cap_comparison':15,'raw_processes_including_warmups':160,'same_error_cap':1e-5},'causal':{'timed_rows_threads1':126,'timed_rows_threads2':84,'completed':210,'same_error_cap_passes':195,'same_error_cap_failures':15,'bitwise_thread_pairs':84},'affine':af['counts'],'pde':{'final_native_rows':135,'final_native_passes':135,'previous_native_rows_preserved':135,'python_accuracy_rows':166,'spatial_rows':9,'pole_split_rows':42},'external_comparators':'NOT_RUN','ncp':False,'timing':'Separate cohorts; no timing pooled across builds, threads, sources or historical reports.'}
write('SUMMARY.json',summary)
plt.rcParams.update({'font.size':10,'axes.spines.top':False,'axes.spines.right':False})
fig,axs=plt.subplots(2,2,figsize=(15,10),layout='constrained')
colors=['#2563eb','#c56612']
def panel(ax,labels,a,b,names,title):
 x=np.arange(len(labels));w=.35
 ax.bar(x-w/2,a,w,label=names[0],color=colors[0]);ax.bar(x+w/2,b,w,label=names[1],color=colors[1])
 ax.axhline(1,color='#333333',ls='--',lw=1);ax.set_yscale('log');ax.set_xticks(x,labels,rotation=25,ha='right');ax.set_ylabel('Median speed ratio (>1 = faster)');ax.set_title(title,loc='left',fontweight='bold');ax.grid(axis='y',alpha=.2);ax.legend(loc='best',fontsize=8)
s=[]
for x in sw['comparisons']:
 e=x['eligible_best'];s.append((e['legacy-fd']['median_seconds']/e['screen-fd']['median_seconds'],e['legacy-jvp']['median_seconds']/e['screen-jvp']['median_seconds']))
panel(axs[0,0],[x['case'] for x in sw['comparisons']],[x[0] for x in s],[x[1] for x in s],['Legacy FD / screened FD','Legacy JVP / screened JVP'],'A  Bounded switching: absolute error cap 1e-5')
panel(axs[0,1],[{'few-mode':'few-mode96','few-mode-large':'few-mode384'}.get(x['case'],x['case']) for x in ca['comparisons']],[x['eligible_best']['causal-forward']['speed_ratio_baseline_over_method'] for x in ca['comparisons']],[x['eligible_best']['causal-picard']['speed_ratio_baseline_over_method'] for x in ca['comparisons']],['Residual-first / forward','Residual-first / Picard'],'B  Complete causal trajectories: cap 1e-5')
labels=['zero','slow','stiff','rapid','shared128','time forcing']
panel(axs[1,0],labels,[x['selected']['full-rhs-dopri5']['seconds_median']/x['selected']['affine-lawson']['seconds_median'] for x in af['cases']],[x['selected']['full-rhs-dopri5']['seconds_median']/x['selected']['exact-declared-forcing']['seconds_median'] for x in af['cases']],['Full RHS / affine Lawson','Full RHS / exact declared action'],'C  Declared forcing: absolute error cap 1e-5')
labels=[x['case'].replace('periodic','').replace('sphere_','S').replace('_n','-') for x in pde_groups]
panel(axs[1,1],labels,[x['methods']['rkl2']['euler_over_method'] for x in pde_groups],[x['methods']['uniformization']['euler_over_method'] for x in pde_groups],['Euler / RKL2','Euler / positive Poisson'],'D  Native diffusion (sphere mu): relative W-L2 cap 1e-5')
fig.suptitle('Scoped improvements and retained losses — independent exploratory cohorts',fontsize=16,fontweight='bold')
fig.supxlabel('Same error cap does not mean identical achieved error. Medians only; ranges in JSON. No external-solver ranking.',fontsize=10)
fig.savefig(B/'figures/frontier_performance.png',dpi=170);fig.savefig(B/'figures/frontier_performance.pdf');plt.close(fig)
print(json.dumps(summary,indent=2))
