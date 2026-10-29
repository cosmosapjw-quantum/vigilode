"""Presentation-only plots of computed matched BASS source-owned 193 epochs."""
from pathlib import Path
import csv,json
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
R=Path(__file__).resolve().parents[1];O=R/'results'
pairs=['m00_r00','m02_r02','m04_r04','m00_r04','m04_r00']
D=json.loads((O/'FIVE_PAIRS_SUMMARY.json').read_text())
fig,ax=plt.subplots(figsize=(8.5,4.5))
pct=[x['geometry_share']*100 for x in D['pairs']]
ax.bar(pairs,pct)
ax.set_ylim(89,95);ax.set_ylabel('Geometry share of integrated Δτ (%)')
ax.set_title('BASS21: matched initial-shear histories, original frozen source')
ax.grid(axis='y',alpha=.2)
fig.tight_layout();fig.savefig(O/'geometry_fraction_five_pairs.png',dpi=160);plt.close(fig)

fig,ax=plt.subplots(figsize=(8.5,5))
for pair in pairs:
    with (O/f'MATCHED_193_EPOCH_DECOMPOSITION_{pair}.csv').open(newline='') as f: rows=list(csv.DictReader(f))
    z=np.array([float(r['z']) for r in rows]);diff=np.array([float(r['tau_delta_saved']) for r in rows])
    ax.plot(z,diff,label=pair)
ax.invert_xaxis();ax.set_xlabel('Source mean-scale-factor redshift z');ax.set_ylabel('Stored Δτ_T (shear .6 minus 0)')
ax.set_title('Five original BASS saved matched histories');ax.legend();ax.grid(alpha=.2)
fig.tight_layout();fig.savefig(O/'five_paired_stored_delta_tau_vs_z.png',dpi=160);plt.close(fig)

fig,ax=plt.subplots(figsize=(8.5,5))
with (O/'MATCHED_193_EPOCH_DECOMPOSITION_m00_r00.csv').open(newline='') as f: rows=list(csv.DictReader(f))
z=np.array([float(r['z']) for r in rows]);columns=[('tau_simpson_chemistry','Ionization'),('tau_simpson_geometry','Geometry'),('tau_simpson_total','Total'),('tau_delta_saved','Original BASS saved Δτ')]
for col,label in columns:ax.plot(z,[float(r[col]) for r in rows],label=label)
ax.invert_xaxis();ax.set_xlabel('Mean-scale-factor z');ax.set_ylabel('Cumulative Δτ_T from initial z=20')
ax.set_title('Exact midpoint decomposition, reference matched pair');ax.legend();ax.grid(alpha=.2)
fig.tight_layout();fig.savefig(O/'reference_cumulative_decomposition.png',dpi=160);plt.close(fig)
print('PLOTS_CREATED',3)
