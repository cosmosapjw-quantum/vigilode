#!/usr/bin/env python3
"""Plot only the saved D5 output/oracle; no callback, ODE, or old replay."""
import json,hashlib
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root=Path(__file__).resolve().parent
rows=[json.loads(x) for x in (root/'evidence/points.jsonl').read_text().splitlines()]
review=json.loads((root/'REVIEW.json').read_text())
data=json.loads((root/'figures/PLOT_DATA.json').read_text())
plt.rcParams.update({'font.size':9,'axes.titlesize':10,'axes.labelsize':9,'xtick.labelsize':8,'ytick.labelsize':8,'legend.fontsize':8,'pdf.fonttype':42,'savefig.dpi':220})

fig,axes=plt.subplots(2,1,figsize=(3.55,5.25),layout='constrained')
selected=[r for r in rows if r['kind']=='point' and r['i'][1:]==[2,6]]
axes[0].plot([r['t'] for r in selected],[r['jac'][0][1] for r in selected],color='#126A8A',marker='o',markersize=3,label='Saved J[0,1]')
axes[0].plot([r['t'] for r in selected],[r['jac'][1][0] for r in selected],color='#AD463E',linestyle='--',label='Transpose interpretation')
axes[0].set(title='A  Derivative distinction',xlabel='Nondimensional time t',ylabel='Jacobian entry',xticks=[0,.5,1],ylim=(2.9,3.9))
axes[0].legend(loc='upper left',frameon=False)
axes[0].text(.98,.05,'x=1/4, y=3/4',ha='right',va='bottom',transform=axes[0].transAxes,fontsize=8)
powers=sorted({r['power'] for r in data['fd']})
maximum=[max(r['max_absolute_error'] for r in data['fd'] if r['power']==p) for p in powers]
axes[1].plot(powers,maximum,color='#126A8A',marker='D',markersize=3,label='49 stencils: observed error')
bound=review['fd_min_gate']
axes[1].axhline(bound,color='#6A6260',linestyle='--',label='Frozen acceptance threshold')
axes[1].set(title='B  Time-difference error',xlabel=r'Step exponent p  ($\epsilon=2^{-p}$)',ylabel='Maximum absolute error',xticks=[3,5,7,9],ylim=(-.12*bound,1.3*bound))
axes[1].ticklabel_format(axis='y',style='sci',scilimits=(0,0))
axes[1].legend(loc='center',frameon=False)
for ax in axes:ax.spines[['top','right']].set_visible(False);ax.grid(alpha=.18)
paths=[]
for ext in ['png','pdf']:
 p=root/'figures'/('D5_DERIVATIVE_WITNESSES.'+ext);fig.savefig(p);paths.append(p)
plt.close(fig)

fig,axes=plt.subplots(2,1,figsize=(3.55,4.9),layout='constrained')
names=['Transpose','Off-diagonal sign','Finite J + I'];keys=['transpose','off_diagonal_sign','finite_J_plus_I']
vals=[review['mutations'][k]['detected_rows'] for k in keys]
axes[0].barh(names,vals,color=['#AD463E','#AD463E','#AD463E'],height=.55)
for i,v in enumerate(vals):axes[0].text(v-8,i,str(v)+'/729',va='center',ha='right',color='white',fontsize=8)
axes[0].invert_yaxis();axes[0].set(title='A  Wrong-J interpretations detected',xlabel='Frozen point rows',xlim=(0,750),xticks=[0,365,729])
labels=['Outside query','Outside stencil','Finite wrong J'];accepted=[0,0,1]
axes[1].barh(labels,accepted,color=['#126A8A','#126A8A','#D99329'],height=.55)
for i,(a,total) in enumerate(zip(accepted,[7,7,1])):axes[1].text(a+.04,i,f'{a}/{total}',va='center',ha='left',fontsize=8)
axes[1].invert_yaxis();axes[1].set(title='B  Interface admission boundary',xlabel='Requests admitted',xlim=(0,1.5),xticks=[0,1])
for ax in axes:ax.spines[['top','right']].set_visible(False);ax.grid(axis='x',alpha=.18);ax.set_axisbelow(True)
for ext in ['png','pdf']:
 p=root/'figures'/('D5_BOUNDARY_WITNESSES.'+ext);fig.savefig(p);paths.append(p)
plt.close(fig)
print(json.dumps({'saved_output_only':True,'scientific_replay':0,'intended_width_mm':90,'also_readable_at_width_mm':180,'files':[{'path':str(p.relative_to(root)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in paths]},indent=2))
