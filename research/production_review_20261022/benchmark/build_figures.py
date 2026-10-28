#!/usr/bin/env python3
"""Recompute descriptive aggregates from saved measurements; run no solver.

Usage: python3 research/production_review_20261022/benchmark/build_figures.py
Requires Python 3, numpy, matplotlib. Output: CSV/JSON and PNG/SVG figures.
The recorded observable errors are imported, not reconstructed from absent states.
"""
import csv
import hashlib
import json
import math
import statistics
import subprocess
from collections import Counter, defaultdict
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

HERE=Path(__file__).resolve().parent
REPO=HERE.parents[2]
FIG=HERE.parent/'figures'
DATA=REPO/'research/bassrei_ncp_wp_c1_20261022'
BASE='555e71e03fe31e6c9b21961c8e2d263b1794eb6f'
FIG.mkdir(exist_ok=True,parents=True)

def load(p): return json.loads(p.read_text())
def rows(p): return [json.loads(x) for x in p.read_text().splitlines() if x.strip()]
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,obj): (HERE/name).write_text(json.dumps(obj,indent=2,ensure_ascii=False,allow_nan=False)+'\n')
def csvwrite(name, records):
    with (HERE/name).open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(records[0]));w.writeheader();w.writerows(records)

audit={'schema_version':1,'review_base':BASE,'scope':'saved aggregate integrity, matched-rung and paired-time arithmetic; full-trajectory errors not independently reproducible from imported subset','attempts':{}}
for attempt in ('attempt2','attempt3'):
    p=DATA/attempt; manifest=load(p/'IMPORT_MANIFEST.json');checked=[]
    for rel,expected in manifest['files_sha256'].items():
        actual=sha(p/rel);checked.append({'path':str((p/rel).relative_to(REPO)),'expected':expected,'actual':actual,'ok':actual==expected})
    ladder=rows(p/'campaign/LADDER.jsonl');abba=rows(p/'campaign/ABBA.jsonl');wp=load(p/'campaign/WORK_PRECISION.json')['table'];matched=load(p/'campaign/MATCHED.json')['matched'];summary=load(p/'campaign/ABBA_SUMMARY.json')
    mismatches=[];nm=0
    for case,arms in wp.items():
        for arm,rungs in arms.items():
            passed=[v for v in rungs.values() if v.get('match')]
            if passed:
                best=max(passed,key=lambda x:x['rtol']);record=matched[case][arm];nm+=1
                if any(record[k]!=best[k] for k in ('rtol','error','solve_seconds','run_id')):mismatches.append([case,arm])
    pairs=[]
    for pair in sorted({x.get('pair') for x in abba}-{None}):
        block=defaultdict(dict)
        for x in abba:
            if x.get('pair')==pair and not x.get('warmup') and x.get('status')=='OK':block[x['block_index']][x['position_arm']]=x['solve_seconds']
        ratio=[x['B']/x['A'] for x in block.values() if set(x)=={'A','B'} and min(x.values())>0]
        if ratio:
            s=summary[pair];ok=statistics.median(ratio)==s['median_B_over_A'] and min(ratio)==s['min'] and max(ratio)==s['max'] and len(ratio)==s['confirmation_blocks_ok']
            pairs.append({'pair':pair,'n':len(ratio),'median':statistics.median(ratio),'minimum':min(ratio),'maximum':max(ratio),'ratios':ratio,'matches_published':ok})
    missing=sorted({x['stdout_rel'] for x in ladder+abba if x.get('stdout_rel') and not (p/x['stdout_rel']).exists()})
    res=load(p/'results.json');ref=res['gates']['G1_references']['values']
    audit['attempts'][attempt]={'manifest_checked':len(checked),'manifest_mismatches':[x for x in checked if not x['ok']], 'manifest_files':checked,'ladder_status':dict(Counter(x['status'] for x in ladder)), 'matched_selections_checked':nm,'matched_mismatches':mismatches,'paired_checks':pairs,'raw_stdout_paths_not_in_import':len(missing),'raw_examples':missing[:5], 'reference_max_pairwise_discrepancy':max(v['u_ref'] for v in ref.values()),'reference_errors':{k:v['u_ref'] for k,v in ref.items()}, 'source_commit':load(p/'campaign/IDENTITY.json')['expected_commit']}
    assert not audit['attempts'][attempt]['manifest_mismatches'] and not mismatches and all(x['matches_published'] for x in pairs)
write('AGGREGATE_AUDIT.json',audit)

# Keep the separate published heat follow-up identity and its summary. No old simulation is rerun.
oldpath='external_audits/vigilode_ncp_followup_fvm_5fd6e45_20261021/CAMPAIGN_REPORT_v2.json'
oldref='fbbc832fdf942396155c12b04c30e63dc67fefe9'
oldraw=(REPO/oldpath).read_bytes() if (REPO/oldpath).exists() else subprocess.check_output(['git','show',oldref+':'+oldpath],cwd=REPO)
old=json.loads(oldraw)
write('HEAT_FOLLOWUP_REUSED.json',{'source_results_commit':oldref,'source_path':oldpath,'source_blob_sha256':hashlib.sha256(oldraw).hexdigest(),'numerical_source_commit':old['identity']['expected_commit'],'summary':old['summary']['heat_paired'],'accuracy':old['accuracy_detail'],'work_counters':old['work_counters'],'timing_scopes':old['timing_scopes'],'reuse':'published result reused; numerical evidence not rerun'})

wp=load(DATA/'attempt3/campaign/WORK_PRECISION.json')['table'];mt=load(DATA/'attempt3/campaign/MATCHED.json')['matched'];res=load(DATA/'attempt3/results.json')
raw=[]
for case,arms in sorted(wp.items()):
    for arm,rungs in sorted(arms.items()):
        for rung,v in sorted(rungs.items()):
            raw.append({'case':case,'arm':arm,'rtol':v.get('rtol'),'status':v['status'],'recorded_error':v.get('error'),'application_pass':v.get('match',False),'solve_seconds':v.get('solve_seconds'),'timing_scope':v.get('timing_scope'),'raw_run_id':v.get('run_id')})
csvwrite('work_precision.csv',raw)
matchrows=[]
for case,arms in sorted(mt.items()):
    for arm,v in sorted(arms.items()):matchrows.append({'case':case,'arm':arm,'rtol':v['rtol'],'recorded_error':v['error'],'solve_seconds':v['solve_seconds'],'loose_edge_censored':v['censored_at_loose_edge']})
csvwrite('matched_costs.csv',matchrows)
csvwrite('paired_ratios.csv',[{k:v for k,v in x.items() if k!='ratios'} for x in audit['attempts']['attempt3']['paired_checks']])
accuracy=[{'case_arm':k,'recorded_error':v['error'],'budget':1e-6,'margin':1e-6/v['error']} for k,v in res['gates']['G3_vigilode_rung']['values'].items()]
csvwrite('accuracy_at_1e-8.csv',accuracy)

plt.rcParams.update({'font.family':'DejaVu Sans','font.size':9,'axes.titlesize':11,'axes.labelsize':9,'legend.fontsize':8,'figure.dpi':160,'savefig.dpi':180,'axes.spines.top':False,'axes.spines.right':False,'svg.fonttype':'none'})
colors=['#176C91','#D2691E','#3C8D55','#9C4B9E','#B39019','#53606B','#D44B52']
labels={'vg_client_exact':'VigilODE structured exact','vg_client_krylov':'VigilODE structured Krylov','vg_banded':'VigilODE banded','vg_dense':'VigilODE dense','vg_bordered_client':'VigilODE bordered','vg_bordered_dense':'VigilODE bordered dense','vg_chem_dense':'VigilODE chemistry dense','jl_rodas5p_sparse':'Julia Rodas5P sparse','jl_fbdf_sparse':'Julia FBDF sparse','jl_tsit5':'Julia Tsit5','jl_cvode_bdf_band':'Julia CVODE-BDF band','jl_cvode_bdf_klu':'Julia CVODE-BDF KLU','ds_bdf_dense':'diffsol BDF dense','ds_tsit45':'diffsol Tsit45','py_vode_adams_mf10':'VODE Adams MF10','py_vode_bdf_mf24':'VODE BDF MF24','py_vode_bdf_mf25':'VODE BDF MF25','py_vode_bdf_mf21':'VODE BDF MF21'}
def save(fig,name):
    for ext in ['png','svg']:fig.savefig(FIG/(name+'.'+ext),bbox_inches='tight',facecolor='white')
    plt.close(fig)

fig,axs=plt.subplots(2,2,figsize=(11.3,8.2));panels=[('S1-G2','Nonstiff transport: 546 variables',['vg_client_exact','vg_client_krylov','jl_tsit5','ds_tsit45','py_vode_adams_mf10']),('S2-G3','Stiff transport: 2114 variables',['vg_client_exact','vg_client_krylov','vg_banded','jl_rodas5p_sparse','jl_cvode_bdf_band','py_vode_bdf_mf24']),('S3-G2','Dynamic matter coupling: 549 variables',['vg_bordered_client','vg_bordered_dense','jl_fbdf_sparse','jl_rodas5p_sparse','ds_bdf_dense','py_vode_bdf_mf24']),('S4S','Stiff photon-gas chemistry: 446 variables',['vg_chem_dense','jl_rodas5p_sparse','jl_cvode_bdf_klu','ds_bdf_dense','py_vode_bdf_mf21'])]
for ax,(case,title,arms) in zip(axs.flat,panels):
    for i,arm in enumerate(arms):
        vals=sorted([v for v in wp[case][arm].values() if v.get('valid') and v.get('error',0)>0 and v.get('solve_seconds',0)>0],key=lambda v:v['rtol'],reverse=True)
        ax.loglog([v['error'] for v in vals],[v['solve_seconds'] for v in vals],'-o',ms=3,lw=1.2,color=colors[i],label=labels[arm])
    ax.axvline(1e-6,color='black',ls='--',lw=.8);ax.grid(alpha=.16,which='both');ax.set_title(title);ax.set_xlabel('Recorded floored observable error');ax.set_ylabel('Warmed solve time [s]');ax.legend(loc='best',framealpha=.85,fontsize=6.7)
fig.suptitle('Accuracy-cost curves on fixed grids',fontsize=14,y=1.01)
fig.text(.5,-.02,'One NCP host, saved 22 October campaign. Error is not a full-state norm; timings describe these runs only.',ha='center',fontsize=9)
fig.tight_layout();save(fig,'work_precision')

fig,axs=plt.subplots(1,2,figsize=(11.3,4.2));grids=[('S1',['S1-G1','S1-G2','S1-G3'],[146,546,2114]),('S2',['S2-G1','S2-G2','S2-G3','S2-G4'],[146,546,2114,8322])]
for ax,(prefix,cases,ns) in zip(axs,grids):
    arms=['vg_client_exact','vg_client_krylov','vg_banded','jl_cvode_bdf_band','jl_rodas5p_sparse']
    for i,arm in enumerate(arms):
        vals=[(n,mt[c][arm]['solve_seconds']) for c,n in zip(cases,ns) if arm in mt[c]]
        ax.loglog(*zip(*vals),'-o',ms=4,lw=1.4,color=colors[i],label=labels[arm])
    ax.set_title('Nonstiff transport' if prefix=='S1' else 'Stiff transport');ax.set_xlabel('State dimension');ax.set_ylabel('Matched-budget solve time [s]');ax.grid(alpha=.17,which='both');ax.set_xticks(ns,labels=[str(n) for n in ns]);ax.legend(fontsize=7)
fig.text(.5,-.01,'Each point uses its own loosest passing tolerance; 146-variable grids are auxiliary. These are not constant-tolerance scaling tests.',ha='center',fontsize=8)
fig.tight_layout();save(fig,'matched_grid_scaling')

pairs=audit['attempts']['attempt3']['paired_checks']
selected=[x for x in pairs if 'py_dop853' not in x['pair'] and ('S1-G2:' in x['pair'] or 'S2-G2:' in x['pair'] or 'S2-G4:' in x['pair'] or 'S3-G2:' in x['pair'] or 'S4' in x['pair'])]
fig,ax=plt.subplots(figsize=(10.4,6.4));y=np.arange(len(selected))
for j,x in enumerate(selected):
    med=x['median'];ax.errorbar(med,j,xerr=[[med-x['minimum']],[x['maximum']-med]],fmt='o',color='#176C91' if med>=1 else '#D2691E',capsize=3,ms=4)
ax.set_yticks(y,labels=[x['pair'].split(':')[0]+' / '+labels.get(x['pair'].split('~')[1].split('@')[0],x['pair'].split('~')[1]) for x in selected]);ax.invert_yaxis();ax.set_xscale('log');ax.axvline(1,color='black',lw=.8,ls='--');ax.set_xlabel('Comparator time / VigilODE time (median of seven AB/BA blocks)');ax.set_title('Paired cost at a common application error budget');ax.grid(axis='x',alpha=.2)
fig.text(.64,.006,'Whiskers: observed minimum-maximum, not confidence intervals.\n>1 favours VigilODE; <1 favours the comparator. One host/campaign.',ha='center',fontsize=9);fig.tight_layout(rect=(0,.085,1,1));save(fig,'paired_cost_ratios')

fig,ax=plt.subplots(figsize=(10.7,6.5));a=sorted(accuracy,key=lambda x:x['recorded_error']);ax.barh([x['case_arm'].replace('vg_','') for x in a],[x['recorded_error'] for x in a],color='#176C91');ax.set_xscale('log');ax.axvline(1e-6,color='#B43535',ls='--',label='Application budget');ax.set_xlim(5e-12,3e-6);ax.set_xlabel('Recorded floored observable error');ax.set_title('Native accuracy at internal relative tolerance 1e-8');ax.legend();ax.grid(axis='x',alpha=.2);fig.text(.5,-.015,'22 case/algorithm combinations; finite output times and selected observables only. Reference agreement is a numerical cross-check.',ha='center',fontsize=8.5);fig.tight_layout();save(fig,'native_accuracy')

# Deterministic work reduction in the earlier heat case; source identity differs.
heat=old['summary']['heat_paired']['A'];fig,axs=plt.subplots(1,2,figsize=(9.4,3.7));arms=['BASE','STEN'];axs[0].bar(['Basis-JVP\nassembly','Analytic stencil'],[heat['arms'][x]['rhs_total_including_derivative'] for x in arms],color=['#777777','#176C91']);axs[0].set_ylabel('Total RHS evaluations');axs[0].set_title('Deterministic derivative work');vals=[heat['arms'][x]['warmed_elapsed_ns']['median']/1e6 for x in arms];axs[1].bar(['Basis-JVP\nassembly','Analytic stencil'],vals,color=['#777777','#176C91']);axs[1].set_ylabel('Median warmed time [ms]');axs[1].set_title('Observed cost: 480 paired blocks');fig.text(.5,-.04,'Periodic heat, n=128, source 5fd6e459 (21 October); identical recorded trajectories. Timing is descriptive.',ha='center',fontsize=8);fig.tight_layout();save(fig,'stencil_work_reduction')
write('FIGURE_DATA.json',{'source_base':BASE,'attempt3_source':audit['attempts']['attempt3']['source_commit'],'figure_files':[str(p.relative_to(HERE.parent)) for p in sorted(FIG.iterdir())],'heat_block_selection':{'plot':'A: 16 cores times 30 confirmation blocks = 480','not_pooled':'A0: 9 additional single-core confirmation blocks','all_confirmation':489,'all_runs_including_warmup':1046},'full_state_recomputed':False,'timing_inference':'descriptive, no cross-host confidence or production speed guarantee','comparison_scope':'Plots use VODE/DVODE, diffsol and Julia implementations. CVODE is a Sundials method called through Julia and labelled explicitly. DOP853 is archived but excluded from comparative plots.'})
print(json.dumps({'status':'PASS','hash_files_checked':sum(a['manifest_checked'] for a in audit['attempts'].values()),'paired_groups_checked':sum(len(a['paired_checks']) for a in audit['attempts'].values()),'figures':len(list(FIG.glob('*.png')))}))
