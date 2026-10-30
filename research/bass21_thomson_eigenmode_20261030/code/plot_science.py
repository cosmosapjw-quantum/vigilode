#!/usr/bin/env python3
"""Actual source-owned BASS21 eigenmode/shear-memory plots, no artificial data."""
from __future__ import annotations
import pathlib,sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
ROOT=pathlib.Path(__file__).resolve().parents[1]
F=ROOT/'figures';F.mkdir(exist_ok=True)
with np.load(ROOT/'results/EIGENMODE_50x193_SOURCE_PHYSICS.npz',allow_pickle=False) as z:
    x=z['x']
    arr={k:z[k] for k in ('U_fast','V_slow','H_U_shear','H_V_shear','U_dynamic_defect','V_dynamic_defect','P_U_free','P_V_free','deltaJ4','tau')}

def plot_lines(ysets,labels,stem,ylabel,title):
    fig,ax=plt.subplots(figsize=(8.5,5.2))
    for y,name in zip(ysets,labels):ax.plot(x,y,label=name,linewidth=1.8)
    ax.set_xlabel(r'$x=\ln(a/a_{\mathrm{ref}})$')
    ax.set_ylabel(ylabel)
    ax.set_title(title)
    ax.grid(alpha=.25)
    ax.legend(loc='best',fontsize=9)
    fig.tight_layout()
    fig.savefig(F/(stem+'.png'),dpi=180)
    fig.savefig(F/(stem+'.svg'))
    plt.close(fig)

plot_lines([arr['U_fast'][25:].mean(0),arr['H_U_shear'][25:].mean(0),arr['U_fast'][:25].mean(0)],
 ['s04 actual fast U','s04 shear-memory contribution H_U','s00 actual fast U'],
 'FAST_U_SHEAR_MEMORY','U / source brightness units','Thomson fast mode: source-owned shear-memory comparison')
plot_lines([arr['V_slow'][25:].mean(0),arr['H_V_shear'][25:].mean(0),arr['V_slow'][:25].mean(0)],
 ['s04 actual slow V','s04 shear-memory contribution H_V','s00 actual slow V'],
 'SLOW_V_SHEAR_MEMORY','V / source brightness units','Thomson slow mode = spatial coherency anisotropy')
plot_lines([np.max(np.abs(arr['U_dynamic_defect']),axis=0),np.max(np.abs(arr['V_dynamic_defect']),axis=0)],
 ['maximum abs(U − P_U − H_U)','maximum abs(V − P_V − H_V)'],
 'FROZEN_MODEL_DEFECT','absolute defect, source brightness units',
 'Source-frozen sampled-grid Duhamel mismatch (not BDF bound)')
plot_lines([np.exp(-arr['tau']).mean(0),np.exp(-.3*arr['tau']).mean(0)],
 ['fast collision survival exp(-tau)','slow collision survival exp(-0.3 tau)'],
 'COLLISION_ONLY_SURVIVAL','collision-only survival fraction','Original BASS21 optical-depth attenuation')
plot_lines([arr['deltaJ4'][25:].mean(0),arr['deltaJ4'][:25].mean(0)],
 ['s04 mean polarized feedback δJ4','s00 mean polarized feedback δJ4'],
 'NEW_J4_FEEDBACK','δJ4 / source brightness units','Reprojected source-owned J4 polarization correction')
print('FIGURES',sorted(p.name for p in F.iterdir()))
