from pathlib import Path
import json, numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
root=Path(__file__).resolve().parents[1]
data=json.loads((root/'source'/'BASS_127_SHEAR_GROUP_READBACK.json').read_text())
x=np.array(data['initial_Sigma_F']); y=np.array(data['tau_mean']); lo=y-np.array(data['tau_min']);hi=np.array(data['tau_max'])-y
fig,ax=plt.subplots(figsize=(7.5,4.2))
ax.errorbar(x,y,yerr=[lo,hi],fmt='o-',capsize=3)
ax.set_xlabel(r'Original BASS initial $\Sigma_F$ parameter (dimensionless)')
ax.set_ylabel(r'Stored $\tau_T$ at $z=2$')
ax.set_title('Actual 125 BASS pilot histories: 25-case grouped optical depth')
ax.ticklabel_format(useOffset=False,axis='y')
fig.tight_layout();fig.savefig(root/'results'/'tau_vs_initial_shear.png',dpi=200);plt.close(fig)
