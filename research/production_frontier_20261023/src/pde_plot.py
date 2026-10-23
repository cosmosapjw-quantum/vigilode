#!/usr/bin/env python3
"""Render published raw PDE rows; no reconstructed or invented observations."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.ticker import NullFormatter, ScalarFormatter
base=Path(__file__).resolve().parents[1]
data=json.loads((base/'results/PDE_CAMPAIGN.json').read_text())
fig,ax=plt.subplots(2,3,figsize=(11,7.3))
fig.subplots_adjust(left=.085,right=.98,bottom=.09,top=.75,hspace=.48,wspace=.32)
colors={'euler':'#596579','rkl2':'#d55e00','uniformization':'#0072b2'}
names={'euler':'Euler','rkl2':'Legendre RKL2','uniformization':'Positive Poisson'}
for col,(prefix,title) in enumerate([('periodic2d','2D periodic'),('periodic3d','3D periodic'),('sphere','Sphere: latitude–longitude')]):
 for method in colors:
  rows=[x['selected'] for x in data['same_error_cap_selection'] if x['case'].startswith(prefix) and x['method']==method and x['error_cap']==1e-5 and x['selected'] is not None]
  rows.sort(key=lambda x:x['cells'])
  ax[0,col].plot([r['cells'] for r in rows],[r['operator_actions'] for r in rows],'-o',color=colors[method],label=names[method],lw=1.7,ms=4)
 spatial=[x for x in data['spatial_refinement'] if x['case'].startswith(prefix)]
 spatial.sort(key=lambda x:x['cells'])
 ax[1,col].plot([x['cells'] for x in spatial],[x['relative_weighted_l2_spatial_error'] for x in spatial],'-s',color='#009e73',lw=1.7,ms=4)
 ax[0,col].set_title(title+'\n'+('Temporal: axisymmetric μ mode' if prefix=='sphere' else 'Temporal: Fourier product'),fontsize=10)
 ax[1,col].set_title('Cell-averaged nonaxisymmetric x mode' if prefix=='sphere' else 'Center-sampled Fourier decay',fontsize=10)
 for row in range(2):
  ax[row,col].set_xscale('log');ax[row,col].set_yscale('log');ax[row,col].grid(True,which='both',alpha=.18)
  ax[row,col].set_xlabel('Cells N',fontsize=10)
  ax[row,col].set_xticks([x['cells'] for x in spatial])
  ax[row,col].xaxis.set_major_formatter(ScalarFormatter())
  ax[row,col].xaxis.set_minor_formatter(NullFormatter())
 ax[0,col].tick_params(labelsize=9);ax[1,col].tick_params(labelsize=9)
ax[0,0].set_ylabel('Graph actions\n(relative W-L² error ≤ 10⁻⁵)',fontsize=10)
ax[1,0].set_ylabel('Relative W-L²\ncontinuum discrepancy',fontsize=10)
handles,labels=ax[0,0].get_legend_handles_labels()
fig.legend(handles,labels,loc='upper center',bbox_to_anchor=(.52,.865),ncol=3,frameon=False)
fig.suptitle('Conservative diffusion: operation counts and independent spatial error',fontsize=13,y=.98)
fig.text(.52,.935,'T = 0.1. Controls chosen from measured relative weighted L² temporal error; counts are not wall-clock speed.',ha='center',fontsize=10)
fig.text(.52,.901,'Periodic spatial curves exclude cell-average projection error; sphere upper and lower panels use different modes.',ha='center',fontsize=10)
(base/'figures').mkdir(exist_ok=True)
fig.savefig(base/'figures/pde_scalability.png',dpi=180)
fig.savefig(base/'figures/pde_scalability.pdf')
