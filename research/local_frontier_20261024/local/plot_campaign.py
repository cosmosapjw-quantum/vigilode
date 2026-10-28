#!/usr/bin/env python3
"""Plot only completed measured groups; absent evidence stays absent."""
import argparse
import json
from pathlib import Path

def main():
    p=argparse.ArgumentParser();p.add_argument('--analysis',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    d=json.loads(a.analysis.read_text());a.output.mkdir(parents=True,exist_ok=False)
    good=[r for r in d['groups'] if r['complete_eligible_group']]
    if not good:
        (a.output/'NO_ELIGIBLE_TIMINGS.txt').write_text('No complete accuracy-eligible timing groups. No synthetic speed plot was produced.\n');return
    for case in sorted({r['case'] for r in good}):
        rows=[r for r in good if r['case']==case]
        layers=sorted({r['parallel_layer'] for r in rows})
        fig,axes=plt.subplots(1,len(layers),figsize=(5*len(layers),4),squeeze=False)
        for ax,layer in zip(axes[0],layers):
            for method in sorted({r['method'] for r in rows if r['parallel_layer']==layer}):
                group=sorted([r for r in rows if r['parallel_layer']==layer and r['method']==method],key=lambda r:r['threads'])
                ax.errorbar([r['threads'] for r in group],[r['seconds_median'] for r in group],
                  yerr=[[r['seconds_median']-r['seconds_min'] for r in group],[r['seconds_max']-r['seconds_median'] for r in group]],marker='o',capsize=3,label=method)
            ax.set(title=f'Case {case}: {layer}',xlabel='Physical CPU budget',ylabel='Whole-span solve seconds')
            ax.set_yscale('log');ax.legend(fontsize=8);ax.grid(alpha=.2)
        fig.suptitle('Median and observed range; separate process repetitions; endpoint accuracy filter')
        fig.tight_layout();fig.savefig(a.output/f'case_{case:02d}.pdf');fig.savefig(a.output/f'case_{case:02d}.png',dpi=160);plt.close(fig)
if __name__=='__main__':main()
