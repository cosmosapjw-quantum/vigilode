import json
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from pathlib import Path
root=Path(__file__).parent
j=json.loads((root/'bench_summary.json').read_text())['results']
fig,ax=plt.subplots(figsize=(8,4.8))
for n in (16,64,128):
 b=j[str(n)]['blocks'];x=[r['block']+1 for r in b]
 ax.plot(x,[r['ratio_id_left'] for r in b],marker='o',label=f'Left diagonal, n={n}')
 ax.plot(x,[r['ratio_id_right'] for r in b],marker='x',linestyle='--',label=f'Right diagonal, n={n}')
ax.axhline(1,linestyle=':',linewidth=1)
ax.set(xlabel='Paired ABBA/BAAB block',ylabel='Identity / preconditioned wall-time ratio',title='PREFIX02: synthetic point-matrix diagnostic (not full IVP)')
ax.legend(fontsize=7,ncol=2)
fig.tight_layout();fig.savefig(root/'prefix02_ratios.png',dpi=160)
