from pathlib import Path
import json
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
root=Path(__file__).resolve().parents[1]
rows=json.loads((root/'results'/'NATIVE_BASS21_ENDPOINT_PHYSICS.json').read_text())['rows']
labels=[r['case_id'].replace('_','\n') for r in rows]
x=np.arange(len(rows))
photo=np.array([r['photo_electron_source_m3_s'] for r in rows]); rr=-np.array([r['RR_electron_sink_m3_s'] for r in rows]);
net=np.array([r['chemical_dne_dt_fixed_nb_m3_s'] for r in rows]);tau=np.array([r['tau_T_stored'] for r in rows]);
fig,ax=plt.subplots(figsize=(8,4.5))
ax.plot(x,photo,'o-',label='Photoelectron production')
ax.plot(x,rr,'s-',label='Radiative recombination sink')
ax.plot(x,net,'^-',label='Net chemical electron source')
ax.set_yscale('log');ax.set_xticks(x,labels,rotation=0);ax.set_ylabel(r'Fixed-$n_b$ electron source [m$^{-3}$ s$^{-1}$]')
ax.set_title('Actual BASS21 saved $z=2$ endpoints: chemistry source balance')
ax.legend(loc='best',fontsize=8)
fig.tight_layout();fig.savefig(root/'results'/'electron_photo_rr_source.png',dpi=180);plt.close(fig)
fig,ax=plt.subplots(figsize=(7.5,4.5))
ax.plot(x,tau,'o-',label=r'Stored $\tau_T$')
ax.set_xticks(x,labels,rotation=0);ax.set_ylabel(r'Cumulative $\tau_T$ (stored BASS pilot)')
ax.set_title('Stored original BASS pilot optical depth: geometry-dependent')
fig.tight_layout();fig.savefig(root/'results'/'saved_tau_comparison.png',dpi=180);plt.close(fig)
