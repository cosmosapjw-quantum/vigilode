#!/usr/bin/env python3
"""Plot audited measured wrapper costs and work; no timing extrapolation."""
import argparse
import json
from pathlib import Path
import statistics

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--audit', type=Path, required=True)
    parser.add_argument('--outdir', type=Path, required=True)
    args = parser.parse_args()
    data = json.loads(args.audit.read_text())
    if data['checks']['result'] != 'PASS_SCOPED':
        raise SystemExit('audited data did not pass')
    order = ['chirp_grid', 'rotation_switch', 'gyro_endpoint', 'stiff_switch', 'case5_endpoint']
    labels = ['Declared chirp chart', 'Rotation adapter', 'Gyromotion adapter', 'Stiff-forced adapter', 'Case 5 adapter']
    groups = {x['cohort']: x for x in data['cohorts']}
    plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 10,
                         'axes.spines.top': False, 'axes.spines.right': False,
                         'svg.fonttype': 'none', 'pdf.fonttype': 42})
    fig, axes = plt.subplots(1, 2, figsize=(10.8, 4.7), sharey=True)
    for y, name in enumerate(order):
        block = [x for x in data['blocks'] if x['cohort'] == name]
        ratios = [x['selected_over_challenger'] for x in block]
        jitter = [(i - 2) * .055 for i in range(len(ratios))]
        axes[0].scatter(ratios, [y + j for j in jitter], s=30,
                        color='#2477a8', alpha=.7, zorder=2)
        axes[0].scatter(statistics.median(ratios), y, marker='|', s=230,
                        color='#102f43', linewidths=2, zorder=3)
        c = groups[name]
        rhs_ratio = c['selected_rhs_per_run'] / c['challenger_rhs_per_run']
        axes[1].scatter(rhs_ratio, y, s=65, marker='D', color='#b75e20', zorder=3)
        axes[1].annotate(f"{int(c['selected_rhs_per_run']):,} / {int(c['challenger_rhs_per_run']):,}",
                         (rhs_ratio, y), xytext=(8, 8), textcoords='offset points', fontsize=9)
    for ax in axes:
        ax.set_xscale('log')
        ax.axvline(1, color='#666666', ls='--', lw=1)
        ax.set_xlim(4e-4, 40)
        ax.set_yticks(range(len(order)), labels)
        ax.grid(axis='x', which='major', alpha=.18)
        ax.set_xlabel('Selected path / direct alternative')
    axes[0].invert_yaxis()
    axes[0].set_title('A. Inclusive solver time, 5 paired repeats', loc='left', fontweight='bold')
    axes[1].set_title('B. RHS work per completed trajectory', loc='left', fontweight='bold')
    fig.suptitle('Published local follow-up: structural gain and automatic-selection overhead',
                 x=.025, ha='left', fontweight='bold', fontsize=13)
    fig.text(.025, .02,
             'Source d914b4c8; published packet 4fe67602. Same physical error caps within each pair; achieved errors differ.\n'
             'Timing is diagnostic only. Dots are repeated runs of five problems, not independent problem draws. '
             'The chirp chart is explicitly declared.', fontsize=8.5)
    fig.tight_layout(rect=(0, .11, 1, .94))
    args.outdir.mkdir(parents=True, exist_ok=True)
    for ext in ('png', 'svg', 'pdf'):
        path = args.outdir / f'published_selection_costs.{ext}'
        if path.exists():
            raise SystemExit(f'refusing to overwrite {path}')
        fig.savefig(path, dpi=180, bbox_inches='tight')


if __name__ == '__main__':
    main()
