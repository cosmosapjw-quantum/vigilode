#!/usr/bin/env python3
"""Aggregate conditioned-chart kernel JSONL; no timing rows are discarded."""
import argparse
import json
import statistics
from pathlib import Path


def main():
    p = argparse.ArgumentParser()
    p.add_argument('input', type=Path)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    rows = [json.loads(line) for line in args.input.read_text().splitlines() if line.strip()]
    groups = []
    for n in sorted({r['dimension'] for r in rows}):
        data = {r['method']: r for r in rows if r['dimension'] == n}
        if len(data) != 6:
            raise ValueError(f'Expected six distinct comparators at n={n}')
        accepted = all(r['accepted'] and r['max_original_residual'] <= r['residual_tolerance'] for r in data.values())
        medians = {k: statistics.median(v['seconds']) for k,v in data.items()}
        base = medians['conditioned_reused']
        groups.append({'dimension': n, 'all_accepted': accepted, 'median_seconds': medians,
            'relative_to_conditioned': {k: v/base if accepted else None for k,v in medians.items()},
            'max_reference_error': max(r['max_reference_error'] for r in data.values()),
            'max_original_residual': max(r['max_original_residual'] for r in data.values())})
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output/'CONDITIONED_CHART_BENCHMARK.json').write_text(json.dumps({'schema_version':1,
        'rows':len(rows),'groups':groups,'interpretation':'Single-process kernel sample; native low-rank action, all diagnostics timed. Exact direct control retained. No whole-ODE claim.'},indent=2)+'\n')
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig, ax = plt.subplots(figsize=(8,4.5))
    for name,label in [('dense_lu_reused','Reused dense LU'),('expanded_reused','Expanded polynomial'),('nilpotent_direct','Exact structured formula')]:
        ax.plot([g['dimension'] for g in groups], [g['relative_to_conditioned'][name] for g in groups],marker='o',label=label)
    ax.axhline(1,color='black',linewidth=0.8,linestyle='--')
    ax.set(xscale='log',yscale='log',xlabel='Dimension',ylabel='Comparator time / conditioned chart time',title='Same original residual and reference checks')
    ax.set_xticks([g['dimension'] for g in groups],labels=[str(g['dimension']) for g in groups])
    ax.grid(alpha=0.2);ax.legend();fig.tight_layout()
    for ext in ['png','svg']:
        fig.savefig(args.output/f'conditioned_chart_comparison.{ext}',dpi=180)
    plt.close(fig)
    print(json.dumps({'rows':len(rows),'all_accepted':all(g['all_accepted'] for g in groups)}))

if __name__ == '__main__':
    main()
