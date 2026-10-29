"""Pure plotting of already generated source-owned endpoint science outputs."""
from pathlib import Path
import json
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'results'
C=json.loads((OUT/'BASS25_SHEAR_SECANT_CURVATURE.json').read_text())
s=np.array([0,.003,.03,.15,.6])
fig,ax=plt.subplots(figsize=(7.2,4.4))
for c in C:
  tau=np.array(c['tau_at_shear'])
  ax.plot(s,1e4*(tau-tau[0]),marker='.',linewidth=.75,alpha=.55)
ax.set_xlabel(r'Initial shear $\Sigma_F$')
ax.set_ylabel(r'$10^4[\tau_T(\Sigma_F)-\tau_T(0)]$')
ax.set_title(r'BASS original 25 matched endpoint histories, forward stored $\tau_T$')
ax.grid(True,alpha=.2)
fig.tight_layout();fig.savefig(OUT/'original_25_shear_tau_curves.png',dpi=160);plt.close(fig)
fig,ax=plt.subplots(figsize=(7,4.5))
arr=np.array([c['divided_second_derivative'] for c in C])
for v in arr:
  ax.plot([.015,.0765,.315],v,marker='.',linewidth=.65,alpha=.55)
ax.set_xlabel('Representative initial shear (segment center)')
ax.set_ylabel('Twice-divided secant difference')
ax.set_title(r'Finite-grid concavity diagnostic, not certified continuum $\tau_T\prime\prime$')
ax.grid(True,alpha=.2)
fig.tight_layout();fig.savefig(OUT/'original_25_secant_curvature.png',dpi=160);plt.close(fig)
