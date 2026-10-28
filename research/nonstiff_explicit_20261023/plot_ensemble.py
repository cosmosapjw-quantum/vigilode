#!/usr/bin/env python3
"""Plot already measured independent-trajectory throughput; no integration."""
import argparse,json,pathlib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

ap=argparse.ArgumentParser();ap.add_argument('campaign');args=ap.parse_args()
folder=pathlib.Path(args.campaign);summary=json.loads((folder/'SUMMARY.json').read_text())
fig,axes=plt.subplots(1,2,figsize=(9,4),sharey=True)
for ax,tol in zip(axes,(1e-6,1e-8)):
    rows=sorted((r for r in summary['rows'] if r['rtol']==tol and r['all_runs_successful']),key=lambda r:r['threads'])
    med=[r['median_seconds'] for r in rows]
    lower=[r['median_seconds']-min(r['seconds']) for r in rows]
    upper=[max(r['seconds'])-r['median_seconds'] for r in rows]
    ax.bar([r['threads'] for r in rows],med,width=.55,color=['#0072b2','#009e73'],yerr=[lower,upper],capsize=5)
    for r in rows:
        ax.scatter([r['threads']]*len(r['seconds']),r['seconds'],s=23,color='black',alpha=.7,zorder=3)
    ax.set_xticks([1,2],['1 worker','2 workers']);ax.set_title(f'rtol = {tol:.0e}')
    ax.set_xlabel('Local pool workers');ax.grid(axis='y',alpha=.2);ax.set_axisbelow(True)
axes[0].set_ylabel('Whole ensemble solve seconds')
fig.suptitle('16 independent trajectories, 4096 components each')
fig.text(.5,.015,'Bars: median; error bars: observed min–max; dots: three timed runs. Pool setup excluded.',ha='center',fontsize=9)
fig.tight_layout(rect=(0,.06,1,.94))
for ext in ('png','pdf','svg'):fig.savefig(folder/f'ENSEMBLE_TIME.{ext}',dpi=160)
