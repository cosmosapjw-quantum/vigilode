#!/usr/bin/env python3
"""Plot the measured quotient-refinement kernel domain without pooling protocols."""
import json
import pathlib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

base = pathlib.Path(__file__).resolve().parents[1]
data = json.loads((base / 'analysis/ALGEBRAIC_BENCHMARK.json').read_text())
rows = data['kernel_summary']
x = np.arange(len(rows))
fig, axes = plt.subplots(1, 2, figsize=(11.5, 4.4), constrained_layout=True)
ax = axes[0]
a = [r['cold_lu_over_cold_refined'] for r in rows]
b = [r['best_baseline_over_reused_refined'] for r in rows]
ax.bar(x-.18,a,.35,label='Fresh LU / fresh quotient',color='#2166ac')
ax.bar(x+.18,b,.35,label='Best reusable baseline / reused quotient',color='#d6604d')
ax.axhline(1,color='#222222',linewidth=1,linestyle='--')
for xi,yi in zip(x-.18,a):ax.text(xi,yi+.12,f'{yi:.2f}',ha='center',fontsize=9)
for xi,yi in zip(x+.18,b):ax.text(xi,yi/2,f'{yi:.2f}',ha='center',va='center',fontsize=9,color='white',fontweight='bold')
ax.set_xticks(x,[str(r['dimension']) for r in rows]);ax.set_ylim(0,6)
ax.set_xlabel('State dimension');ax.set_ylabel('Median baseline time / quotient time')
ax.set_title('Setup amortization changes the winner')
ax.legend(fontsize=8,loc='upper left');ax.grid(axis='y',alpha=.18);ax.set_axisbelow(True)
ax=axes[1]
ax.semilogy(x,[r['legacy_refused_residual'] for r in rows],'o-',color='#d6604d',label='One polynomial: refused')
ax.semilogy(x,[r['refined_result']['residual'] for r in rows],'s-',color='#2166ac',label='Bounded correction: accepted')
ax.axhline(1e-10,color='#222222',linewidth=1,linestyle='--',label='Original residual target')
ax.set_xticks(x,[str(r['dimension']) for r in rows]);ax.set_ylim(1e-14,1)
ax.set_xlabel('State dimension');ax.set_ylabel('Original-space infinity residual')
ax.set_title('Approximate relations become usable')
ax.legend(fontsize=8,loc='center left');ax.grid(axis='y',alpha=.18)
fig.suptitle('Declared near-scalar operator: kernel results only',fontsize=13)
out=base/'figures';out.mkdir(exist_ok=True)
for extension in ['png','svg','pdf']:fig.savefig(out/f'algebraic_refinement.{extension}',dpi=220)
plt.close(fig)
print('figures/algebraic_refinement.{png,svg,pdf}')
