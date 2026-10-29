"""Optional plots; same 193 saved epochs, no IVPs, no styling forced."""
from pathlib import Path
import csv
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
R=Path(__file__).resolve().parents[1]
with (R/'results'/'NATIVE_BASS_193_EPOCH_HISTORY.csv').open() as f:rows=list(csv.DictReader(f))
get=lambda k:np.array([float(r[k]) for r in rows])
z=get('z')
plt.figure(figsize=(8,4.6))
plt.plot(z,get('x_e'),label='BASS saved $x_e$')
plt.gca().invert_xaxis();plt.xlabel('Redshift $z$');plt.ylabel('Electron fraction $n_e/n_b$')
plt.grid(alpha=.3);plt.legend();plt.tight_layout();plt.savefig(R/'results'/'electron_fraction_history.png',dpi=165);plt.close()
plt.figure(figsize=(8,4.6))
plt.plot(z,get('tau_T_stored'),label='BDF-stored cumulative $\\tau_T$')
plt.plot(z,get('tau_T_simpson_193'),ls='--',label='193-point sampled Simpson')
plt.gca().invert_xaxis();plt.xlabel('Redshift $z$');plt.ylabel('Cumulative $\\tau_T$ from $z=20$')
plt.grid(alpha=.3);plt.legend();plt.tight_layout();plt.savefig(R/'results'/'tau_stored_vs_simpson.png',dpi=165);plt.close()
plt.figure(figsize=(8,4.6))
plt.plot(z,get('simpson_minus_saved_tau'),label='Simpson minus original BDF $\\tau_T$')
plt.gca().invert_xaxis();plt.xlabel('Redshift $z$');plt.ylabel('Saved quadrature residual')
plt.grid(alpha=.3);plt.legend();plt.tight_layout();plt.savefig(R/'results'/'tau_quadrature_residual.png',dpi=165);plt.close()
plt.figure(figsize=(8,4.6))
plt.plot(z,np.maximum(get('photo_e_m3_s'),1e-36),label='Photo $e^-$ production')
plt.plot(z,np.maximum(-get('RR_e_m3_s'),1e-36),label='RR $e^-$ removal')
plt.plot(z,np.maximum(np.abs(get('chemical_e_fixed_nb_m3_s')),1e-36),label='|Net chemical $e^-$ source|')
plt.yscale('log');plt.gca().invert_xaxis();plt.xlabel('Redshift $z$');plt.ylabel(r'Local source magnitude [m$^{-3}$ s$^{-1}$]')
plt.grid(alpha=.3);plt.legend();plt.tight_layout();plt.savefig(R/'results'/'electron_source_history.png',dpi=165);plt.close()
print('SAVED 4 REPLAY-ONLY PNGS')
