from pathlib import Path
import json
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
R=Path(__file__).resolve().parents[1]
fig,ax=plt.subplots(2,2,figsize=(12,8),layout='constrained')
colors={'physical-dopri5':'#d55e00','declared-chirp-chart-dopri5':'#0072b2'}
o=json.loads((R/'oscillation/PILOT_RESULTS.json').read_text())['rows']
for method in sorted({r['method'] for r in o}):
 rows=[r for r in o if r['method']==method and r['case']=='catalog-15-exponential-chirp']
 ax[0,0].loglog([r['rhs_calls'] for r in rows],[r['actual_endpoint_linf_error'] for r in rows],'-o',label=method)
ax[0,0].axhline(1e-4,color='0.4',ls='--',label='endpoint criterion')
ax[0,0].set(xlabel='RHS evaluations',ylabel='Physical endpoint max error',title='A. Actual exponential chirp (T=8)'); ax[0,0].legend(fontsize=8)
a=json.loads((R/'adaptation/PILOT_SUMMARY.json').read_text())
xx=np.arange(2)
for off,method,label,color in [(-.18,'literal','Original coordinates','#d55e00'),(.18,'cofactor','Cofactor coordinates','#0072b2')]:
 rows=[next(r for r in a if r['epsilon']==1e-6 and r['end']==t and r['method']==method) for t in [.01,2.]]
 bars=ax[0,1].bar(xx+off,[r['attempts'] for r in rows],width=.36,label=label,color=color)
 for b,r in zip(bars,rows):
  if not r['success']:b.set_hatch('///');ax[0,1].text(b.get_x()+b.get_width()/2,r['attempts']*1.1,'incomplete',ha='center',fontsize=8)
ax[0,1].set(yscale='log',xticks=xx,xticklabels=['T=0.01','T=2'],ylabel='Attempted steps',title='B. Boundary-layer case (epsilon=1e-6)'); ax[0,1].legend(fontsize=8)
s=json.loads((R/'switching/results_r2/SUMMARY.json').read_text())['groups']
case=['decay','rotation','stiff-forced']; modes=['dopri','rodas','pressure-only','evidence'];width=.18
for j,m in enumerate(modes):
 v=[next(r for r in s if r['case']==c and r['h0']==1e-4 and r['mode']==m)['median_seconds']*1e3 for c in case]
 ax[1,0].bar(np.arange(3)+(j-1.5)*width,v,width,label=m)
ax[1,0].set(yscale='log',xticks=np.arange(3),xticklabels=case,ylabel='Whole solve median (ms)',title='C. Method selection: common endpoint criterion');ax[1,0].legend(fontsize=7,ncol=2)
for off,name,label,color in [(-.18,'PILOT_SUMMARY.json','Initial selection','#d55e00'),(.18,'PILOT_SUMMARY_REPAIRED.json','Repaired selection','#0072b2')]:
 p=json.loads((R/'parallel'/name).read_text())['results'];ax[1,1].bar(np.arange(3)+off,[r['adaptive_time_over_best_median'] for r in p],.36,label=label,color=color)
ax[1,1].axhline(1,color='0.4',ls='--');ax[1,1].set(xticks=np.arange(3),xticklabels=['1','64','256'],xlabel='Independent ODE batch size',ylabel='Adaptive / best direct median',title='D. Execution choice (two-worker candidate)');ax[1,1].legend(fontsize=8)
for a1 in ax.flat:a1.grid(axis='y',alpha=.2);a1.spines[['top','right']].set_visible(False)
fig.suptitle('VigilODE adapter pilots: accuracy, work and selection cost',fontsize=16)
fig.supxlabel('Short same-host studies; achieved errors differ. D excludes calibration in medians; total costs are reported separately.',fontsize=9)
for ext in ['png','svg','pdf']:fig.savefig(R/('pilot_overview.'+ext),dpi=160)
