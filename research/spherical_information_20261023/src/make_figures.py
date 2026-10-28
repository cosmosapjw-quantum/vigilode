from pathlib import Path
import json
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'figures'; OUT.mkdir(exist_ok=True)
plt.rcParams.update({'font.size':10,'axes.spines.top':False,'axes.spines.right':False,
                     'savefig.dpi':160,'figure.facecolor':'white'})
def save(fig,name):
    fig.savefig(OUT/(name+'.png'),bbox_inches='tight')
    fig.savefig(OUT/(name+'.pdf'),bbox_inches='tight')
    plt.close(fig)

m=json.loads((ROOT/'results/mesh_results.json').read_text())
profiles=list(dict.fromkeys(r['profile'] for r in m['records']))
fig,axs=plt.subplots(2,3,figsize=(13,7.4),layout='constrained')
for ax,p in zip(axs.flat,profiles):
    rows=sorted([r for r in m['records'] if r['profile']==p],key=lambda r:r['budget_fraction'])
    for key,label,col,mark in [('uniform','Nested uniform','#b86222','s'),('adaptive','Entropy adaptive','#146d8c','o')]:
        ax.loglog([r['budget_fraction'] for r in rows],[r[key]['cells'] for r in rows],marker=mark,label=label,color=col)
    ax.set_title(p.replace('_',' '));ax.set_xlabel('Allowed loss / root loss');ax.set_ylabel('Active cells')
    ax.grid(alpha=.2,which='both');ax.invert_xaxis()
axs.flat[0].legend(fontsize=9)
ax=axs.flat[-1]
rows=[r for r in m['records'] if r['budget_fraction']==.01]
ax.bar(np.arange(len(rows)),[100*r['adaptive']['number_relative_drift'] for r in rows],color='#b86222')
ax.set_xticks(np.arange(len(rows)),[r['profile'].replace('_','\n') for r in rows],fontsize=8)
ax.set_ylabel('Particle-number change (%)');ax.set_title('Energy-only remap, 1% entropy budget')
ax.grid(alpha=.2,axis='y')
fig.suptitle('Finite-grid compression: same allowed entropy budget, not equal realized error\n8192-cell reference; cell-count reduction is not a measured PDE speedup',fontsize=13)
save(fig,'mesh_budget')

f=json.loads((ROOT/'results/fisher_results.json').read_text())
rows=f['cases']; xx=np.arange(len(rows))
labels=[f"{r['statistics']} z={r['z']:g}\nL={r['L']}"+(' narrow' if r['narrow'] else (' iso' if r['anisotropy']==0 else '')) for r in rows]
fig,axs=plt.subplots(3,1,figsize=(13,10),layout='constrained',sharex=True)
axs[0].semilogy(xx,[r['raw_condition'] for r in rows],'o-',label='Original Hessian',color='#b86222')
axs[0].semilogy(xx,[r['whitened_condition'] for r in rows],'s-',label='Fisher whitened',color='#146d8c')
axs[0].semilogy(xx,[r['condition_upper'] for r in rows],'k:',label='Derived ratio upper bound')
axs[0].set_ylabel('2-norm condition number');axs[0].legend(ncol=3,fontsize=9)
for key,label,off,color in [('plain_cg','CG',-.18,'#b86222'),('fisher_pcg','Fisher PCG',.18,'#146d8c')]:
    axs[1].bar(xx+off,[r['runs'][key]['iterations'] for r in rows],width=.36,label=label,color=color)
axs[1].set_ylabel('Iterations');axs[1].legend(ncol=2)
ratios=[r['median_solve_speed_ratio'] for r in rows]
axs[2].bar(xx,ratios,color=['#146d8c' if r>=1 else '#b86222' for r in ratios])
axs[2].axhline(1,color='black',lw=1);axs[2].set_ylabel('CG time / PCG time');axs[2].set_ylim(0,max(ratios)*1.2)
axs[2].set_xticks(xx,labels,rotation=35,ha='right',fontsize=8)
for ax in axs:ax.grid(alpha=.2,axis='y')
fig.suptitle('Entropy moment inversion: same natural-coordinate residual <= 1e-9\nPCG timing includes block setup; dense Hessian assembly is shared and excluded',fontsize=13)
save(fig,'fisher_conditioning')

t=json.loads((ROOT/'results/thomson_results.json').read_text())
r=json.loads((ROOT/'results/remap_budget_results.json').read_text())
fig,axs=plt.subplots(1,2,figsize=(12,4.4),layout='constrained')
for key,label,marker in [('scalar','Scalar prototype','o'),('polarized','Polarized coherency','s')]:
    c=[x for x in t[key]['cases'] if x['tau']>0]
    axs[0].loglog([x['tau'] for x in c],[x['max_abs_dense_difference'] for x in c],marker+'-',label=label)
axs[0].set_xlabel('Optical depth per collision step');axs[0].set_ylabel('Max absolute difference from dense expm')
axs[0].set_title('Exact fixed-shell collision update');axs[0].legend();axs[0].grid(alpha=.2,which='both')
case=next(c for c in r['cases'] if c['block']==4 and c['optical_depth_per_step']==0 and c['remap_every']==1)
h=case['history']
axs[1].plot([v['step'] for v in h],[v['hellinger'] for v in h],label='Actual Hellinger error')
axs[1].plot([v['step'] for v in h],[v['accumulated_sqrt_kl_bound'] for v in h],label='Sum of sqrt(local KL) bound')
axs[1].axhline(np.sqrt(2),ls=':',color='grey',label='Trivial unit-mass maximum')
axs[1].set_xlabel('Repeated remap step');axs[1].set_ylabel('Unnormalized Hellinger distance')
axs[1].set_title('Discrete rotation + remap budget');axs[1].legend(fontsize=8);axs[1].grid(alpha=.2)
fig.suptitle('Small verification problems: collision exactness and conservative accumulated error bound',fontsize=12)
save(fig,'collision_and_remap')
print('Wrote 3 PNG/PDF figure pairs')
