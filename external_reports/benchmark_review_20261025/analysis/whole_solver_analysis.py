#!/usr/bin/env python3
"""Reanalyse published ODE measurements; never launch a solver or alter receipts."""
from __future__ import annotations
import collections
import hashlib
import json
from pathlib import Path
import statistics
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
from matplotlib.lines import Line2D

HERE = Path(__file__).resolve().parent
OUT = HERE.parent
REPO = OUT.parents[1]
FIG = OUT / 'figures'
PACKET = REPO / 'research/local_frontier_native_diffsol_20261024/audit_packet_20261025'
HIST = REPO / 'external_audits/vigilode_ncp_intensive_120m_v1_20261021'
SOURCES = []

def read(path):
    b = path.read_bytes()
    SOURCES.append({'path': str(path.relative_to(REPO)), 'sha256': hashlib.sha256(b).hexdigest()})
    return json.loads(b)

def jsonl(path):
    b = path.read_bytes()
    SOURCES.append({'path': str(path.relative_to(REPO)), 'sha256': hashlib.sha256(b).hexdigest()})
    return [json.loads(l) for l in b.splitlines() if l.strip()]

def save(fig, name):
    for ext in ('png', 'svg', 'pdf'):
        fig.savefig(FIG / f'{name}.{ext}', dpi=200, bbox_inches='tight')
    plt.close(fig)

plt.rcParams.update({'font.family':'DejaVu Sans', 'font.size':10, 'axes.titlesize':12,
    'axes.labelsize':10, 'axes.spines.top':False, 'axes.spines.right':False,
    'axes.grid':True, 'grid.alpha':.16, 'svg.fonttype':'none', 'pdf.fonttype':42})
FIG.mkdir(parents=True, exist_ok=True)

STATES = ['PASS_SCOPED', 'ACCURACY_FAILED', 'SOLVER_FAILED', 'NO_RESULT', 'UNVALIDATED_REFERENCE']
SLABEL = ['Accuracy criterion met', 'Accuracy criterion missed', 'Solver failure', 'Timeout / no result', 'Reference pending']
COLORS = ['#22846b', '#bb432d', '#7d477d', '#d49b38', '#b7c0c9']
METHODS = ['dopri5', 'rodas5p-mf', 'rodas5p-dense', 'switching']
MLABEL = ['DOPRI5', 'RODAS5P matrix-free', 'RODAS5P dense', 'Switching']
NAMES = ['Robertson','HIRES','Pleiades','Kuramoto','Boundary layer','Freeze-out transport',
    'Nonigniting scalar','Oregonator','FPUT','Gyromotion','Two-scale Lorenz-96',
    'Kuramoto–Sivashinsky','Low-Mach variant','Cahn–Hilliard','Exponential chirp',
    'Inspiral chirp','Sphere advection','Sphere diffusion','Spherical modal waves',
    'Multipole hierarchy','2D Brusselator','Anisotropic diffusion','3D Barkley','3D absorber transport']

index = read(PACKET / 'LEGACY_STANDARD_1539_STATUS_INDEX.json')
rows = index['rows']
assert len(rows) == 1539 and len({r['task_id'] for r in rows}) == 1539
for r in rows:
    matches = []
    for instrumented in (False, True):
        k = {'case':r['case'],'method':r['method'],'parallel_layer':r['layer'],
             'threads':r['threads'],'repeat':r['repeat'],'instrumented':instrumented}
        canonical = json.dumps(k,sort_keys=True,separators=(',',':'),allow_nan=False)
        if hashlib.sha256(canonical.encode()).hexdigest()[:16] == r['task_id']:
            matches.append(instrumented)
    assert len(matches)==1, r['task_id']
    r['instrumented'] = matches[0]
clean = [r for r in rows if not r['instrumented']]
profile = [r for r in rows if r['instrumented']]
assert len(clean)==1458 and len(profile)==81

def counts(rs): return dict(collections.Counter(r['science'] for r in rs))
matrix = []
for ci in range(1,25):
    for m in METHODS:
        rr = [r for r in clean if r['case']==ci and r['method']==m]
        matrix.append({'case':ci,'case_name':NAMES[ci-1],'method':m,'n':len(rr),'outcomes':counts(rr)})

fig, axes = plt.subplots(1,4,figsize=(14.1,9.6),sharey=True,gridspec_kw={'wspace':.24})
for j,(m,ax) in enumerate(zip(METHODS,axes)):
    for ci in range(1,25):
        item = next(x for x in matrix if x['case']==ci and x['method']==m)
        left=0
        if item['n']==0:
            ax.text(.5,ci,'not scheduled',ha='center',va='center',fontsize=8,color='#858585')
        else:
            for s,c in zip(STATES,COLORS):
                n=item['outcomes'].get(s,0)
                if n:
                    w=n/item['n']
                    ax.barh(ci,w,left=left,color=c,height=.76,edgecolor='white',linewidth=.4)
                    if w>.10: ax.text(left+w/2,ci,str(n),ha='center',va='center',fontsize=8,color='white' if s in STATES[:3] else '#202020')
                    left+=w
    ax.set(xlim=(0,1),xticks=[0,.5,1],xticklabels=['0%','50%','100%'],ylim=(24.7,.3))
    ax.set_title(MLABEL[j],pad=10)
    ax.grid(axis='y',visible=False)
    ax.set_xlabel('Fraction of 18 runs')
axes[0].set_yticks(range(1,25),[f'{i:02d}  {name}' for i,name in enumerate(NAMES,1)])
fig.suptitle('Published local benchmark: outcome by problem and method',y=.98,fontsize=15)
fig.text(.5,.935,'1,458 uninstrumented tasks · counts printed in each segment · 81 profiling tasks counted separately',ha='center',fontsize=10)
fig.legend(handles=[Patch(facecolor=c,label=l) for c,l in zip(COLORS,SLABEL)],loc='lower center',ncol=3,bbox_to_anchor=(.5,-.015),frameon=False)
fig.subplots_adjust(top=.90,bottom=.09,left=.23)
save(fig,'whole_local_outcomes')

fig,axs=plt.subplots(1,2,figsize=(11.8,4.1),gridspec_kw={'width_ratios':[1.5,1]})
total=counts(clean)
axs[0].barh(np.arange(5),[total.get(s,0) for s in STATES],color=COLORS)
for i,s in enumerate(STATES):
    n=total.get(s,0);axs[0].text(n+10,i,f'{n:,} ({n/len(clean):.1%})',va='center',fontsize=10)
axs[0].set_yticks(np.arange(5),SLABEL);axs[0].invert_yaxis();axs[0].set_xlim(0,1000)
axs[0].set_xlabel('Uninstrumented tasks');axs[0].set_title('(a) All 24 problems')
g=[counts(clean),counts(profile)]
for k,lab in enumerate(['Uninstrumented\n1,458 tasks','Profiling\n81 tasks']):
    left=0; n=sum(g[k].values())
    for s,c in zip(STATES,COLORS):
        v=g[k].get(s,0)/n
        axs[1].barh(k,v,left=left,color=c,height=.55)
        if v>.075:axs[1].text(left+v/2,k,str(g[k].get(s,0)),ha='center',va='center',fontsize=9,color='white' if s in STATES[:3] else '#202020')
        left+=v
axs[1].set_yticks([0,1],['Uninstrumented\n1,458 tasks','Profiling\n81 tasks']);axs[1].invert_yaxis()
axs[1].set_xlim(0,1);axs[1].set_xticks([0,.5,1],['0%','50%','100%']);axs[1].set_xlabel('Within-group proportion')
axs[1].set_title('(b) Execution classes kept separate')
fig.tight_layout(w_pad=3)
save(fig,'whole_local_coverage')

ROB_METHODS=['dopri5','rodas5p-mf','switching','native-dvode-adams','native-dvode-bdf','diffsol-bdf']
ROB_LABEL=['VigilODE DOPRI5','VigilODE RODAS5P MF','VigilODE switching','Native DVODE Adams','Native DVODE BDF','diffsol BDF']
RCOLOR=['#2070a5','#da6d24','#8a54a2','#759633','#252f55','#d34e77']
RMARK=['o','s','D','^','v','P']
rob = read(PACKET / 'evidence/references/case01/REASSESSMENT_V2.json')
robout=[]
fig,axes=plt.subplots(1,2,figsize=(11.7,5.2),sharey=True)
for ax,(cohort,c) in zip(axes,rob['cohorts'].items()):
    raw=[r for r in c['raw'] if not r['warmup']]
    assert len(raw)==30 and all(r['status']=='PASS_SCOPED' and r['execution_status']=='COMPLETED' for r in raw)
    for m,label,col,mark in zip(ROB_METHODS,ROB_LABEL,RCOLOR,RMARK):
        rr=[r for r in raw if r['method']==m];assert len(rr)==5
        t=np.array([r['solve_seconds'] for r in rr]); e=np.array([r['actual_endpoint_error_max_component'] for r in rr])
        med=float(np.median(t));err=float(np.max(e))
        a={'cohort':cohort,'t_end':c['t_end'],'method':m,'n':len(rr),'median_seconds':med,'min_seconds':float(min(t)),'max_seconds':float(max(t)),'max_error':err,'accuracy_cap':c['cap'],'all_meet_cap':bool(all(e<=c['cap']))}
        robout.append(a)
        ax.scatter(t*1e3,e,s=20,color=col,alpha=.45,marker=mark,zorder=2)
        ax.errorbar(med*1e3,err,xerr=[[1e3*(med-min(t))],[1e3*(max(t)-med)]],fmt=mark,color=col,markersize=8,capsize=3,zorder=3)
    ax.axhline(c['cap'],color='#8c4030',ls='--',lw=1)
    ax.axhspan(1e-14,rob['reference_uncertainty']['estimate'],color='#dbe2e8',alpha=.7,zorder=0)
    ax.set_xscale('log');ax.set_yscale('log');ax.set_ylim(8e-14,3e-4)
    ax.set_xlabel('Reported solve time (ms)')
    ax.set_title(f"Robertson · endpoint t = {c['t_end']:g}")
axes[0].set_ylabel('Maximum component endpoint error')
axes[0].text(.04,1.25e-12,'Numerical reference estimate: $10^{-12}$',fontsize=8,color='#546475')
axes[0].text(.04,1.35e-4,'Acceptance threshold: $10^{-4}$',fontsize=8,color='#8c4030')
fig.legend(handles=[Line2D([],[],marker=mark,color=col,ls='',label=lab) for mark,col,lab in zip(RMARK,RCOLOR,ROB_LABEL)],loc='lower center',ncol=3,bbox_to_anchor=(.5,-.055),frameon=False)
fig.suptitle('Local external comparison at fixed solver settings',fontsize=14,y=.98)
fig.subplots_adjust(bottom=.19,top=.87,wspace=.20)
save(fig,'whole_robertson_error_cost')

# Historical full-solver study is intentionally isolated by source and host.
historical=[]
hfiles=['TOLERANCE_RESULTS.jsonl','ROTATION_TOLERANCE_RESULTS_RECOVERED.jsonl']
HLABEL={'vigilode':'VigilODE RODAS5P (Oct 21)','scipy_vode_adams':'SciPy VODE Adams','scipy_vode_bdf':'SciPy VODE BDF','Julia OrdinaryDiffEq':'Julia Tsit5','diffsol':'diffsol Tsit45'}
HCOLOR=['#2070a5','#759633','#252f55','#ba5d28','#d34e77']
HMARK=['o','^','v','D','P']
HSOL=list(HLABEL)
fig,axes=plt.subplots(1,2,figsize=(11.7,5.1))
for ax,f,title in zip(axes,hfiles,['Diagonal decay, n = 128','Rotation, n = 2; frequency = 10']):
    rs=jsonl(HIST/f); measured=[r for r in rs if r.get('warmup') is False]
    assert len(measured)==140
    for s,col,mk in zip(HSOL,HCOLOR,HMARK):
        x=[];y=[]
        for tol in sorted({r['rtol'] for r in measured},reverse=True):
            rr=[r for r in measured if r['solver']==s and r['rtol']==tol];assert len(rr)==7
            tt=[float(r['timing']['timed_seconds'] if r.get('timing') else r.get('elapsed_seconds',r.get('process_wall_seconds'))) for r in rr]
            ee=[r['accuracy']['max_abs_error'] for r in rr]
            scaled=[r['accuracy']['max_scaled_error'] for r in rr]
            passed=sum(r['accuracy']['accuracy_pass_scaled_le_1'] for r in rr)
            med=float(np.median(tt)); q1,q3=np.percentile(tt,[25,75]); e=float(np.median(ee))
            historical.append({'file':f,'solver':s,'rtol':tol,'n':7,'median_seconds':med,'q1_seconds':float(q1),'q3_seconds':float(q3),'median_max_abs_error':e,'median_scaled_error':float(np.median(scaled)),'meets_requested_scaled_error':passed})
            ax.errorbar(med*1e3,e,xerr=[[1e3*(med-q1)],[1e3*(q3-med)]],fmt=mk,color=col,mfc=col if passed==7 else 'white',mec=col,ms=7,capsize=2,zorder=3)
            x.append(med*1e3);y.append(e)
        ax.plot(x,y,color=col,lw=.8,alpha=.65)
    ax.set_xscale('log');ax.set_yscale('log');ax.set_title(title);ax.set_xlabel('Reported solve / adapter time (ms)')
axes[0].set_ylabel('Maximum sampled component error')
fig.suptitle('Historical work–precision study · 21 October 2026',y=.99,fontsize=14)
fig.legend(handles=[Line2D([],[],marker=mk,color=co,ls='',label=HLABEL[s]) for s,co,mk in zip(HSOL,HCOLOR,HMARK)],loc='lower center',ncol=3,bbox_to_anchor=(.5,-.05),frameon=False)
fig.text(.5,.075,'Filled: original scaled-error criterion met. Hollow: criterion missed. Whiskers: interquartile time range.',ha='center',fontsize=9)
fig.subplots_adjust(bottom=.24,top=.86,wspace=.25)
save(fig,'whole_historical_work_precision')

summary={'schema_version':1,'analysis':'Published measurement reaggregation; no solver execution',
 'source_commit_read':'2a4bf90d1445b67542bdac5030b4c5d10da20445',
 'local_outcomes':{'source_commit':'aaf1f834d4c589e734ca7851a56c11bb44f7047a',
   'all_tasks':len(rows),'uninstrumented':len(clean),'instrumented':len(profile),
   'counts_all':counts(rows),'counts_uninstrumented':counts(clean),'counts_instrumented':counts(profile),
   'matrix_uninstrumented':matrix,
   'instrumentation_recovery':'Unique SHA256[:16] match against original canonical task dictionary, tested for instrumented=false and true.',
   'inference':'Published final classifications are reaggregated. Most original raw receipts are hash-indexed only; no new raw scientific validation. Timing scalars absent from this index.'},
 'robertson':{'measured_source':rob['cohorts']['standard']['measured_followup_source'],
   'reference':rob['reference_method'],'reference_uncertainty':rob['reference_uncertainty'],
   'summary':robout,'scope':'Five timed repetitions per method and endpoint after one warmup; fixed settings, not equal achieved error. Min-max whiskers are observed variation, not confidence intervals.'},
 'historical':{'source_commit':'de4c10a9f41321beef2313a3d7fb503bab35d714',
   'host':'NCP Xeon Gold 5220 KVM, one-thread workers pinned to logical CPU 8; existing 21 October archive only',
   'solver_versions':'SciPy 1.11.4 VODE, diffsol 0.17.1 Tsit45, Julia 1.12.7 / OrdinaryDiffEq 7.8.1 Tsit5',
    'rows':historical,
   'scope':'Seven measured repetitions after two warmups; rtol 1e-4,1e-6,1e-8,1e-10; arms not paired. Times are adapter scopes, not current-source speed ratios. Native reported time includes setup/integration/output certificate; VODE includes Python callback and output conversion; Julia excludes compilation; diffsol separate setup is excluded.'},
 'source_files':SOURCES,
 'checks':{'task_hashes_unique':1539,'status_counts_asserted':True,'robertson_timed_rows':60,'historical_timed_rows':280,
 'solver_runs_in_this_analysis':0}}
assert summary['local_outcomes']['counts_all']=={'PASS_SCOPED':323,'ACCURACY_FAILED':146,'SOLVER_FAILED':57,'NO_RESULT':208,'UNVALIDATED_REFERENCE':805}
(HERE/'whole_solver_summary.json').write_text(json.dumps(summary,indent=2,ensure_ascii=False,allow_nan=False)+'\n')
print(json.dumps({'figures':4,'local_all':counts(rows),'local_uninstrumented':counts(clean),'profile':counts(profile),'robertson_rows':60,'historical_rows':280,'solver_runs':0},indent=2))
