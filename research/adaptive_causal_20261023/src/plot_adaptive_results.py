#!/usr/bin/env python3
"""Render existing summary; does not repeat any solver or reference calculation."""
import argparse,json,pathlib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
p=argparse.ArgumentParser(description=__doc__);p.add_argument('folder');a=p.parse_args();folder=pathlib.Path(a.folder);summary=json.loads((folder/'SUMMARY.json').read_text());cfg=summary['configurations']
colors={'dopri5':'#1f77b4','rodas-fd':'#d62728','rodas-jvp':'#9467bd','switch-fd':'#ff7f0e','switch-jvp':'#2ca02c','dopri-whole':'#17becf','dopri-whole-dense':'#444444','rodas-whole-fd':'#8c564b','switch-rhs':'#e377c2'}
names=['decay','logistic','prothero','robertson','rapid','damped','nonnormal','transition','vector'];fig,axes=plt.subplots(3,3,figsize=(13,10))
for ax,case in zip(axes.flat,names):
 any_valid=False
 for mode,color in colors.items():
  rows=sorted([r for r in cfg if r['case']==case and r['mode']==mode and r['eligible']],key=lambda r:r['max_error'])
  if not rows:continue
  any_valid=True;med=[r['median_seconds']for r in rows];lo=[r['median_seconds']-r['min_seconds']for r in rows];hi=[r['max_seconds']-r['median_seconds']for r in rows]
  ax.errorbar([max(r['max_error'],1e-17)for r in rows],med,yerr=[lo,hi],fmt='-o',label=mode,color=color,markersize=4,capsize=2,lw=1.3)
 if any_valid:ax.set_xscale('log');ax.set_yscale('log')
 else:ax.text(.5,.5,'No completed configuration\nAttempt budget exhausted',ha='center',va='center',transform=ax.transAxes,color='#a22',fontsize=11)
 ax.set_title(case);ax.grid(True,which='both',alpha=.25);ax.set_xlabel('max sampled normalized error');ax.set_ylabel('seconds, median and min-max')
h,l=axes.flat[0].get_legend_handles_labels();fig.legend(h,l,loc='lower center',ncol=min(4,len(l)));fig.suptitle('Single-thread persistent RHS follow-up: three timed repeats, all setup included');fig.tight_layout(rect=[0,.04,1,.96]);fig.savefig(folder/'WORK_PRECISION.png',dpi=160);fig.savefig(folder/'WORK_PRECISION.pdf');plt.close(fig)
