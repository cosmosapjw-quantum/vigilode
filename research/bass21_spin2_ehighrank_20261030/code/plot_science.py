#!/usr/bin/env python3
"""Scientific plots from measured immutable frozen-source high-rank E+ NPZ."""
import pathlib,math
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
ROOT=pathlib.Path(__file__).resolve().parents[1]
FIG=ROOT/'figures';FIG.mkdir(exist_ok=True)
with np.load(ROOT/'results/HIGH_E_HIERARCHY_50x193.npz') as z:
    d={k:z[k] for k in z.files}
case=d['cases'].tolist(); x=d['x']; ell=d['l'];E=d['E_l0_plus'];F=d['boundary_flux_16_18_20_22'];

with open(ROOT/'results/SCIENCE_SUMMARY.json') as q:
 import json;s=json.load(q)

f,ax=plt.subplots(figsize=(7.5,4.7))
ax.semilogy(ell,s['max_abs_by_shear_family']['s00'],marker='o',label='s00 (weak shear)')
ax.semilogy(ell,s['max_abs_by_shear_family']['s04'],marker='s',label='s04 (strong shear)')
ax.semilogy(ell,s['max_gap_by_mode'],linestyle='--',marker='.',label='GL64/96 max discrepancy')
ax.set(xlabel='Even spin-2 multipole l',ylabel='max |E_l0^(+)| [source brightness units]',title='Actual BASS21 high-l polarization vs quadrature discrepancy')
ax.grid(True,which='both',alpha=.25);ax.legend(fontsize=8)
f.tight_layout();f.savefig(FIG/'E_HIGH_L_RESOLUTION.png',dpi=165);f.savefig(FIG/'E_HIGH_L_RESOLUTION.svg');plt.close(f)

f,ax=plt.subplots(figsize=(7.5,4.7))
for i,L in enumerate((16,18,20,22)):
    maxflux=np.max(np.abs(F[25:,:,i]),axis=0)
    ax.semilogy(x,np.maximum(maxflux,1e-30),label=f'L={L} <- L+2')
ax.set(xlabel='x=ln(a/a_ref)',ylabel='max |shear boundary flux| [per x]',title='Measured omitted-neighbour flux in strong-shear BASS21 histories')
ax.grid(True,which='both',alpha=.25);ax.legend()
f.tight_layout();f.savefig(FIG/'HIGH_L_BOUNDARY_FLUX.png',dpi=165);f.savefig(FIG/'HIGH_L_BOUNDARY_FLUX.svg');plt.close(f)

f,ax=plt.subplots(figsize=(7.5,4.7))
idx={k:i for i,k in enumerate(case)}
pairflux=[]
for m in range(5):
 for k in range(5):
  a=idx[f's00_m{m:02d}_r{k:02d}'];b=idx[f's04_m{m:02d}_r{k:02d}'];pairflux.append(F[b,:,0]-F[a,:,0])
pairflux=np.asarray(pairflux)
ax.plot(x,np.mean(pairflux,axis=0),label='mean matched (s04-s00), L=16')
ax.fill_between(x,np.min(pairflux,axis=0),np.max(pairflux,axis=0),alpha=.18,label='pairwise min/max')
ax.set(xlabel='x=ln(a/a_ref)',ylabel='matched flux difference [per x]',title='25 matched physical upper-boundary flux differences')
ax.grid(True,alpha=.25);ax.legend(fontsize=8)
f.tight_layout();f.savefig(FIG/'MATCHED_25_BOUNDARY_FLUX.png',dpi=165);f.savefig(FIG/'MATCHED_25_BOUNDARY_FLUX.svg');plt.close(f)

f,ax=plt.subplots(figsize=(7.5,4.7))
for idx,L in enumerate((2,16,20)):
 j=np.where(ell[:-1]==L)[0][0]
 for name,k in (('shear','shear_E'),('Thomson','collision_E')):
  arr=d[k][25:,:,j]
  rms=np.sqrt(np.mean(arr**2,axis=0))
  ax.semilogy(x,np.maximum(rms,1e-30),label=f'l={L}: {name}')
ax.set(xlabel='x=ln(a/a_ref)',ylabel='s04 ensemble RMS local |dE/dx|',title='Shear and collision at original source epochs')
ax.grid(True,which='both',alpha=.25);ax.legend(fontsize=8,ncol=2)
f.tight_layout();f.savefig(FIG/'SHEAR_VS_THOMSON_RMS.png',dpi=165);f.savefig(FIG/'SHEAR_VS_THOMSON_RMS.svg');plt.close(f)
print('PLOTS_DONE', sorted(p.name for p in FIG.iterdir()))
