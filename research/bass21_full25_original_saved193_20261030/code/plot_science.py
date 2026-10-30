"""Actual saved-BASS cosmology/chemistry figures; immutable source postprocessing only."""
import csv,pathlib
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
ROOT=pathlib.Path(__file__).resolve().parents[1]
res=ROOT/'results'
with (res/'MATCHED_25_PAIR_SUMMARY.csv').open() as f:p=list(csv.DictReader(f))
arr=np.zeros((5,5));ion=np.zeros((5,5));
for d in p:
 m,r=[int(x[1:]) for x in d['pair'].split('_')]
 arr[m,r]=100*float(d['geometry_fraction']);ion[m,r]=100*float(d['chemistry_fraction'])
fig,ax=plt.subplots(figsize=(6.8,5.2));im=ax.imshow(arr)
for i in range(5):
 for j in range(5):ax.text(j,i,f'{arr[i,j]:.3f}',ha='center',va='center',fontsize=8)
ax.set_xticks(range(5),[f'r{r:02d}' for r in range(5)]);ax.set_yticks(range(5),[f'm{m:02d}' for m in range(5)]);ax.set_xlabel('Radiation initial target index');ax.set_ylabel('Matter initial target index')
ax.set_title('BASS original 25 saved matched histories: geometry share (%)');fig.colorbar(im,ax=ax,label='Geometry fraction (%)')
fig.tight_layout();fig.savefig(res/'geometry_fraction_grid.png',dpi=165);plt.close(fig)
# Integrated contribution distribution by source x redshift bin
r=p[0]; labels=[f'{float(r[f"window{k}_z_start"]):.2f}→{float(r[f"window{k}_z_end"]):.2f}' for k in range(4)]
chem=np.array([np.mean([float(d[f'window{k}_chemistry']) for d in p]) for k in range(4)])
geo=np.array([np.mean([float(d[f'window{k}_geometry']) for d in p]) for k in range(4)])
fig,ax=plt.subplots(figsize=(7.4,4.9));xx=np.arange(4)
ax.bar(xx-.18,-geo,width=.36,label='Geometry |Δτ|');ax.bar(xx+.18,-chem,width=.36,label='Ionization |Δτ|')
ax.set_xticks(xx,labels);ax.set_xlabel('Original source redshift window z_start → z_end');ax.set_ylabel('Mean positive magnitude of integrated Δτ')
ax.set_title('BASS 25-pair saved-time opacity contributions');ax.legend();fig.tight_layout();fig.savefig(res/'opacity_redshift_windows.png',dpi=165);plt.close(fig)
# Electron fraction rate and split for representative source history
with (res/'MATCHED_25_X_193_OPTICAL_DEPTH.csv').open() as f:sel=[d for d in csv.DictReader(f) if d['pair']=='m02_r02']
z=np.array([float(d['z']) for d in sel]);chem=np.array([float(d['tau_simpson_chemistry']) for d in sel]);geo=np.array([float(d['tau_simpson_geometry']) for d in sel]);total=np.array([float(d['delta_tau_saved']) for d in sel])
fig,ax=plt.subplots(figsize=(7.3,4.9));ax.plot(z,geo,label='Integrated geometry');ax.plot(z,chem,label='Integrated ionization');ax.plot(z,total,label='Original saved total Δτ')
ax.set_xlim(20,2);ax.set_xlabel('Source redshift z (forward integration 20→2)');ax.set_ylabel('Cumulative contrast Δτ');ax.legend();ax.set_title('One physical matched pair: m02_r02 (saved BASS states)')
fig.tight_layout();fig.savefig(res/'representative_cumulative_contributions.png',dpi=165);plt.close(fig)
print('Plots 3/3 source-real')
