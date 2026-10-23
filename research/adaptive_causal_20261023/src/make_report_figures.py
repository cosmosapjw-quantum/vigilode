#!/usr/bin/env python3
"""Render measured advantages and losses without mixing benchmark cohorts."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

root = Path(__file__).resolve().parents[1]
out = root / 'figures'
out.mkdir(exist_ok=True)
quotient = json.loads((root/'results/quotient_summary.json').read_text())
rows = [r for r in quotient['comparisons'] if r['sampled_error_cap'] == 1e-4]
rows.sort(key=lambda r: r['baseline_over_candidate'], reverse=True)
plt.rcParams.update({'font.size': 10, 'axes.spines.top': False, 'axes.spines.right': False})
fig, ax = plt.subplots(figsize=(10, 4.8), layout='constrained')
ratio = [r['baseline_over_candidate'] for r in rows]
y = np.arange(len(rows))
ax.barh(y, ratio, color=['#227e76' if v >= 1 else '#b85142' for v in ratio])
ax.set_yticks(y, [r['case'] for r in rows]); ax.invert_yaxis()
ax.set_xscale('log'); ax.axvline(1, color='black', lw=1)
ax.set_xlim(.04, 20000)
for i, v in enumerate(ratio): ax.text(v*1.12, i, f'{v:.3g}x', va='center')
ax.set_xlabel('Full-RHS DOPRI time / declared-flow Lawson time (log scale)')
ax.set_title('Known constant carrier: measured gains and counterexamples\nSame sampled absolute-error cap 1e-4; fastest eligible measured tolerance')
fig.savefig(out/'QUOTIENT_WORK_PRECISION.png', dpi=180)
fig.savefig(out/'QUOTIENT_WORK_PRECISION.pdf')
plt.close(fig)

causal = json.loads((root/'results/causal_benchmark_summary.json').read_text())['rows']
fig, ax = plt.subplots(figsize=(11, 5.8), layout='constrained')
y = np.arange(len(causal)); w=.36
ax.barh(y-w/2, [r['forward_speedup'] for r in causal], height=w, color='#227e76', label='Causal Forward')
ax.barh(y+w/2, [r['picard_speedup'] for r in causal], height=w, color='#b88837', label='Batched Picard (one thread)')
ax.axvline(1, color='black', lw=1)
ax.set_yticks(y, [f"{r['kind']}, n={r['n']}" for r in causal]); ax.invert_yaxis()
ax.set_xlabel('Residual-first time / candidate time')
ax.set_title('Identical projected raw-U stage equations, original-space validation\nStage-root timings; only 6/36 candidates pass the physical embedded test')
ax.legend(loc='lower right')
fig.savefig(out/'CAUSAL_STAGE_WORK.png', dpi=180)
fig.savefig(out/'CAUSAL_STAGE_WORK.pdf')
plt.close(fig)
print(json.dumps({'figures':4, 'quotient_cases':len(rows), 'causal_cases':len(causal)}))
