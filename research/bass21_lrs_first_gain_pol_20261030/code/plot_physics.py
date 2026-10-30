from __future__ import annotations
import csv,pathlib,sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
p=pathlib.Path(__file__).resolve().parents[1]/'results';d=np.load(p/'LRS_50x193x17_POLARIZATION.npz');mu=d['mu'];cs=d['cases'].tolist()
fig,ax=plt.subplots(figsize=(8.2,4.5))
for name in ['s00_m02_r02','s04_m02_r02','s00_m04_r04','s04_m04_r04']:
    ax.plot(mu,d['Q'][cs.index(name),-1,:],label=name)
ax.set(xlabel='Propagation-axis cosine |μ| at z = 2',ylabel='Normalized first-gain Stokes Q',title='Same-source BASS21 LRS polarization: endpoint angular structure')
ax.legend(fontsize=8);ax.grid(alpha=.25);fig.tight_layout();fig.savefig(p/'Q_FINAL_ANGULAR.png',dpi=180);plt.close(fig)
fig,ax=plt.subplots(figsize=(8.2,4.5));z=21*np.exp(-d['x'])-1
for name in ['s00_m02_r02','s04_m02_r02','s00_m04_r04','s04_m04_r04']:
    ax.plot(z,d['Q'][cs.index(name),:,0],label=name)
ax.set(xlabel='Source redshift z (20→2)',ylabel='Normalized first-gain Q, μ=0',title='Source-owned integrated quadrupole scattering history');ax.invert_xaxis();ax.legend(fontsize=8);ax.grid(alpha=.25);fig.tight_layout();fig.savefig(p/'Q_EQUATOR_HISTORY.png',dpi=180);plt.close(fig)
# Three-block exact attribution is algebraic, not causal.
rows=list(csv.DictReader((p/'MATCHED_25_SHAPLEY_3BLOCK.csv').open()));eq=[r for r in rows if r['mu_observer']=='0.0'];keys=['geometry_b','photon_J2','opacity_g_tau'];vals=[np.mean([float(r[k]) for r in eq]) for k in keys]
fig,ax=plt.subplots(figsize=(8.2,4.5));ax.bar(keys,vals);ax.set(ylabel='Mean contribution to ΔQ at μ=0',title='Shapley exact 3-block attribution (noncausal)');ax.axhline(0,lw=1);ax.grid(axis='y',alpha=.25);fig.tight_layout();fig.savefig(p/'SHAPLEY_MEAN_EQUATOR.png',dpi=180);plt.close(fig)
print('Generated 3 physical-source plots')
