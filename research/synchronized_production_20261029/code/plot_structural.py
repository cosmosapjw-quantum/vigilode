#!/usr/bin/env python3
"""Plot retained raw paired blocks; does not execute a solver."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root = Path('/workspace/scratch/f9615df1ce94')
raw = root / 'lane-structural02'
out = root / 'lane-structural-figures01'
out.mkdir(exist_ok=True)
data = json.loads((raw / 'CASE_SUMMARY.json').read_text())
cases = ['rotation', 'rotation-large', 'damped-rotation', 'time-jvp', 'badcoeff']
labels = ['Rotation, n=16', 'Rotation, n=4096', 'Damped rotation, n=16', '16 distinct rates', 'Wrong coefficients']
fig, axes = plt.subplots(1, 2, figsize=(11.5, 4.6), gridspec_kw={'width_ratios': [1.25, 1]})
for i, case in enumerate(cases):
    row = data[case]
    color = '#2563a6' if row['median'] > 1 else '#b94b32'
    axes[0].scatter(row['ratios'], [i + (k-3)*.025 for k in range(7)], s=22, color=color, alpha=.65)
    axes[0].plot(row['median'], i, marker='D', color='black', markersize=5)
    arms = [json.loads((raw / (case+'-previous-probe-'+arm) / 'stdout.txt').read_text())['samples'][0] for arm in ['gmres', 'proposal']]
    jvp = arms[1]['callback_counts']['jvp'] / arms[0]['callback_counts']['jvp']
    mgs = arms[1]['work']['orthogonalization_inner_products'] / arms[0]['work']['orthogonalization_inner_products']
    axes[1].barh(i-.13, jvp, height=.23, color='#2563a6', label='JVP calls' if i == 0 else None)
    axes[1].barh(i+.13, mgs, height=.23, color='#d88c35', label='MGS inner products' if i == 0 else None)
    axes[1].text(jvp+.035, i-.13, f'{jvp:.2f}', va='center', fontsize=8)
    axes[1].text(max(mgs,.01)+.035, i+.13, f'{mgs:.2f}', va='center', fontsize=8)
for axis in axes:
    axis.set_yticks(range(5), labels if axis is axes[0] else ['']*5)
    axis.invert_yaxis()
    axis.axvline(1, color='#555555', ls='--', lw=1)
    axis.grid(axis='x', alpha=.18)
    axis.spines[['top', 'right']].set_visible(False)
axes[0].set_xlim(.5, 4.2)
axes[0].set_xlabel('GMRES / proposal time (larger is faster)')
axes[0].set_title('Full-IVP paired timings: all 7 blocks')
axes[1].set_xlim(0, 1.8)
axes[1].set_xlabel('Proposal / GMRES count (smaller is less work)')
axes[1].set_title('Actual per-solve work, previous-start mode')
axes[1].legend(loc='upper right', bbox_to_anchor=(1,1.18), ncol=2, frameon=False, fontsize=8)
fig.suptitle('Residual-validated quadratic stage proposals', fontsize=14)
fig.text(.02, .012, 'Same binary; 17 returned grid points; common error cap 1e-5. Dots are blocks, diamonds medians, not confidence intervals.\nTiming includes solve + owned-result/workspace drop. Work counts include refused proposals and fallback; these counts are not total FLOPs.', fontsize=8)
fig.tight_layout(rect=(0,.09,1,.94))
for ext in ['png', 'pdf']:
    fig.savefig(out / ('structural_performance.'+ext), dpi=180, bbox_inches='tight')
plt.close(fig)
print(out)
