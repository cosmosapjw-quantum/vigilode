#!/usr/bin/env python3
"""Reaggregate every diagnostic pair and make compact, separately scoped figures."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import statistics


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--checks', type=Path, required=True)
    p.add_argument('--bench', type=Path, required=True)
    p.add_argument('--deployment', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    timings = json.loads((a.bench/'TIMINGS.json').read_text())
    pairs = json.loads((a.bench/'PAIRS.json').read_text())
    ownership = json.loads((a.bench/'OWNERSHIP_PAIRS.json').read_text())
    cases = ('dense-fd','banded-fd','time-jvp','autonomous-jvp')
    result = dict(schema='vigilode.synchronized-analysis.v1', production='HOLD',
                  timing_authority='diagnostic_only', cases={}, checks=[], phases=[],
                  ownership_pairs=len(ownership), ownership_all_parity=all(x['exact_parity'] for x in ownership))
    for label, directory in [('checks',a.checks),('bench',a.bench),('deployment',a.deployment)]:
        phase = json.loads((directory/'RUN.json').read_text())
        result['phases'].append(dict(label=label,source=phase['source'],wall_seconds=phase['wall_seconds'],execution=phase['execution']))
        for cmd in phase['commands']:
            for stream in ('stdout','stderr'):
                data = (directory/cmd['label']/(stream+'.txt')).read_bytes()
                if hashlib.sha256(data).hexdigest()!=cmd[stream+'_sha256']:
                    raise ValueError('raw receipt hash mismatch')
            if label=='checks':
                log=(directory/cmd['label']/'stdout.txt').read_text()+(directory/cmd['label']/'stderr.txt').read_text()
                counts=[tuple(map(int,m)) for m in re.findall(r'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored',log)]
                py=re.search(r'Ran (\d+) tests?',log)
                result['checks'].append(dict(label=cmd['label'],exit_code=cmd['exit_code'],rust_counts=counts,python_tests=int(py[1]) if py else None))
    result['aggregate_phase_wall_seconds']=sum(x['wall_seconds'] for x in result['phases'])
    for case in cases:
        rows=[x for x in timings if x['case']==case]
        if len(rows)!=7: raise ValueError('seven paired blocks required')
        ratios=[]
        for row in rows:
            samples={'baseline':[],'candidate':[]}
            for pos,arm in enumerate(row['order']):
                payload=json.loads((a.bench/f"{case}-b{row['block']}-{pos}-{arm}"/'stdout.txt').read_text())
                if len(payload['samples'])!=16 or not all(x['success'] for x in payload['samples']):
                    raise ValueError('incomplete/failed timed block')
                samples[arm].append(statistics.median(x['solve_plus_drop_ns'] for x in payload['samples']))
            ratio=sum(samples['baseline'])/sum(samples['candidate'])
            if ratio!=row['ratio']: raise ValueError('aggregation mismatch')
            ratios.append(ratio)
        allocation=next(x for x in pairs if x['case']==case and x['mode']=='alloc')
        if not all(x['exact_returned_parity'] for x in pairs if x['case']==case):
            raise ValueError('parity gate failed')
        result['cases'][case]=dict(ratios=ratios,median=statistics.median(ratios),minimum=min(ratios),maximum=max(ratios),
                                   baseline_allocations=allocation['baseline_allocations'],candidate_allocations=allocation['candidate_allocations'],
                                   endpoint_error=allocation['endpoint_error'])
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    plt.rcParams.update({'font.size':8,'axes.labelsize':8,'axes.titlesize':9,'legend.fontsize':7,'savefig.dpi':300})
    labels=['Dense FD','Banded FD','Time JVP','Autonomous JVP']
    fig,ax=plt.subplots(figsize=(3.54,2.6),layout='constrained')
    for i,case in enumerate(cases):
        row=result['cases'][case]
        ax.scatter(row['ratios'],[i+(j-3)*.055 for j in range(7)],s=13,color='#35639c',alpha=.8)
        ax.plot([row['minimum'],row['maximum']],[i,i],color='#17263c',linewidth=1)
        ax.plot(row['median'],i,'D',color='#b14b33',markersize=4)
    ax.axvline(1,color='gray',linestyle='--',linewidth=.8)
    ax.set(yticks=range(4),yticklabels=labels,xlabel='Baseline / candidate solve + drop time',title='Seven paired blocks; range is not a CI')
    fig.savefig(a.output/'paired_timing.png');fig.savefig(a.output/'paired_timing.pdf');plt.close(fig)
    fig,ax=plt.subplots(figsize=(3.54,2.6),layout='constrained')
    for i,case in enumerate(cases):
        row=result['cases'][case]
        for vals in (row['baseline_allocations'],row['candidate_allocations']):
            if len(set(vals))!=1: raise ValueError('allocation repetitions differ')
        ax.barh(i-.17,row['baseline_allocations'][0],height=.30,color='#899baa',label='Baseline' if i==0 else None)
        ax.barh(i+.17,row['candidate_allocations'][0],height=.30,color='#39776c',label='Candidate' if i==0 else None)
    ax.set(yticks=range(4),yticklabels=labels,xlabel='Allocations per complete solve',title='Three repeated allocation observations')
    ax.legend(loc='lower right');fig.savefig(a.output/'allocations.png');fig.savefig(a.output/'allocations.pdf');plt.close(fig)
    fig,ax=plt.subplots(figsize=(3.54,2.6),layout='constrained')
    errors=[result['cases'][x]['endpoint_error'] for x in cases]
    ax.semilogx(errors,range(4),'o',label='Both arms (exact parity)',markersize=4)
    ax.axvline(1e-5,color='#b14b33',linestyle='--',label='Diagnostic cap')
    ax.set(yticks=range(4),yticklabels=labels,xlabel='Endpoint max absolute error',title='Analytic binary64 reference; no enclosure')
    ax.legend(loc='lower right');fig.savefig(a.output/'endpoint_errors.png');fig.savefig(a.output/'endpoint_errors.pdf');plt.close(fig)
    (a.output/'ANALYSIS.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__': main()
