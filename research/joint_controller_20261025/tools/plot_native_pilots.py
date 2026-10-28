#!/usr/bin/env python3
"""Plot the completed native pilot summary, keeping its three scopes separate."""
import argparse
import json
from pathlib import Path
import statistics

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--summary', type=Path, required=True)
    ap.add_argument('--outdir', type=Path, required=True)
    args = ap.parse_args()
    d = json.loads(args.summary.read_text())
    assert all(d[k]['status'] == 'MEASURED_SCOPED' for k in ['switching', 'execution', 'causal'])
    plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 9.5,
                         'axes.spines.top': False, 'axes.spines.right': False,
                         'svg.fonttype': 'none', 'pdf.fonttype': 42})
    fig, axes = plt.subplots(2, 2, figsize=(12.2, 9.0))
    cases = d['switching']['cases']
    lookup = {r['case']: r for r in cases}
    preferred = ['decay', 'rotation', 'stiff-forced', 'case5', 'relaxation']
    order = [n for n in preferred if n in lookup] + sorted(set(lookup) - set(preferred))
    cases = [lookup[n] for n in order]
    labels = [x['case'].replace('transition-', 'transition .') for x in cases]
    xx = np.arange(len(cases))
    for i, c in enumerate(cases):
        ratios = [r['adaptive_over_alternative_seconds'] for r in c['paired_comparisons']
                  if r['alternative'] == 'evidence' and r['status'] == 'COMPARABLE']
        if ratios:
            axes[0, 0].scatter([i + (j - (len(ratios)-1)/2)*.05 for j in range(len(ratios))],
                               ratios, color='#2477a8', alpha=.7, s=28)
            axes[0, 0].scatter(i, statistics.median(ratios), color='#102f43', marker='_', s=180, linewidths=2)
    axes[0, 0].axhline(1, color='#666', ls='--', lw=1)
    axes[0, 0].set_yscale('log')
    axes[0, 0].set_ylabel('New / previous inclusive solver time')
    axes[0, 0].set_title('A. Probe scheduling: paired full-trajectory costs', loc='left', fontweight='bold')
    for ax in axes[0]:
        ax.set_xticks(xx, labels, rotation=30, ha='right')
        ax.grid(axis='y', alpha=.18)
    old = [c['policies'].get('evidence', {}).get('median_discarded_probe_rhs', 0) for c in cases]
    new = [c['policies'].get('adaptive', {}).get('median_discarded_probe_rhs', 0) for c in cases]
    axes[0, 1].bar(xx-.18, old, width=.36, color='#b4b9bf', label='Previous schedule')
    axes[0, 1].bar(xx+.18, new, width=.36, color='#2477a8', label='Adaptive schedule')
    axes[0, 1].set_ylabel('Discarded probe RHS calls per trajectory')
    axes[0, 1].set_title('B. Charged exploration work', loc='left', fontweight='bold')
    axes[0, 1].legend(frameon=False, fontsize=8.5)

    phases = d['execution']['phases']
    xx2 = np.arange(len(phases))
    adaptive_dispatch = np.array([r['sum_dispatch_seconds']['adaptive'] * 1000 for r in phases])
    overhead = np.array([(r['calibration_seconds_including_warmup'] + r['adaptive_pool_startup_delta_seconds']) * 1000 for r in phases])
    best_direct = [min(r['sum_dispatch_seconds']['serial'],
                       r['direct_parallel_total_including_pool'] if r['direct_parallel_total_including_pool'] is not None
                       else r['sum_dispatch_seconds']['parallel']) * 1000 for r in phases]
    axes[1, 0].bar(xx2-.2, best_direct, width=.36, color='#b4b9bf', label='Best observed direct total')
    axes[1, 0].bar(xx2+.2, adaptive_dispatch, width=.36, color='#2477a8', label='Adaptive dispatch')
    axes[1, 0].bar(xx2+.2, overhead, bottom=adaptive_dispatch, width=.36, color='#d99a4a', label='Calibration + pool startup')
    axes[1, 0].set_xticks(xx2, [r['phase'].replace('-', '\n') for r in phases])
    axes[1, 0].set_ylabel('Five evaluated batches + setup (ms)')
    axes[1, 0].set_title('C. Execution selection: all exploration charged', loc='left', fontweight='bold')
    axes[1, 0].legend(frameon=False, fontsize=8.1)
    axes[1, 0].grid(axis='y', alpha=.18)

    causal = d['causal']
    cc = [causal['blanket'], causal['selected']]
    candidate = np.array([r['candidate_rhs'] for r in cc])
    verification = np.array([r['interval_verifier_rhs'] for r in cc])
    axes[1, 1].bar([0, 1], candidate, width=.55, color='#2477a8', label='Candidate RHS')
    axes[1, 1].bar([0, 1], verification, bottom=candidate, width=.55, color='#d99a4a', label='Interval residual checks')
    axes[1, 1].set_xticks([0, 1], ['Blanket residual 1e-14', 'Physical output budget'])
    axes[1, 1].set_ylabel('Evaluations on the same fixed mesh')
    axes[1, 1].set_title('D. Causal stage-root correction work', loc='left', fontweight='bold')
    axes[1, 1].legend(frameon=False, fontsize=8.5)
    axes[1, 1].grid(axis='y', alpha=.18)
    fig.suptitle('Joint adaptation and selection: measured scoped pilots', x=.03, ha='left', fontsize=14, fontweight='bold')
    fig.text(.03, .015,
             'A–B: same endpoint accuracy caps, different achieved errors; timing is diagnostic. '
             'C: direct pool startup is charged once; drift phase intentionally keeps a stale epoch.\n'
             'D: derivative-free original-target stage residuals; serial emulation of correction rounds, not measured parallel speedup. '
             'Scalar and interval evaluation counts have different unit costs.', fontsize=8.2)
    fig.tight_layout(rect=(0, .08, 1, .95), h_pad=2.0, w_pad=2.4)
    args.outdir.mkdir(parents=True, exist_ok=True)
    for ext in ['png', 'svg', 'pdf']:
        path = args.outdir / ('native_selection_pilots.' + ext)
        if path.exists():
            raise SystemExit(f'refusing to overwrite {path}')
        fig.savefig(path, dpi=180, bbox_inches='tight')


if __name__ == '__main__':
    main()
