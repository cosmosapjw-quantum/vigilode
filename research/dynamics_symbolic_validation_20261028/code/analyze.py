#!/usr/bin/env python3
"""Exact saved-query audit and complete paired native summaries."""
import argparse
from collections import defaultdict
from fractions import Fraction as Q
import hashlib
import json
from pathlib import Path
import statistics as S
import struct

def bits(q):
    return Q.from_float(struct.unpack('>d',int(q).to_bytes(8,'big'))[0])

def mul(a,b):
    return [[sum((a[i][k]*b[k][j] for k in range(4)),Q()) for j in range(4)] for i in range(4)]

def original_trace(row):
    a,b=map(bits,row['state_bits'])
    matrix=[[Q(0),Q(0),Q(0),b-a*a],[Q(1),Q(0),Q(0),Q(-1)],[Q(0),Q(1),Q(0),2*a],[Q(0),Q(0),Q(1),Q(0)]]
    power=[[Q(int(i==j)) for j in range(4)] for i in range(4)]
    value=Q()
    for word in row['coefficient_bits'][:row['length']]:
        value+=bits(word)*sum((power[i][i] for i in range(4)),Q())
        power=mul(power,matrix)
    return value

def stats(ratios):
    return {'paired_ratios':ratios,'median_ratio':S.median(ratios),'minimum_ratio':min(ratios),'maximum_ratio':max(ratios),'new_faster_pairs':sum(x>1 for x in ratios),'pairs':len(ratios)}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=False)
    trace_path=args.run/'trace-support/stdout.log'
    switch_path=args.run/'switch-timing.json'
    trace=json.loads(trace_path.read_text());switch=json.loads(switch_path.read_text())
    audit={'queries':0,'accepted_intervals':0,'exact_singletons':0,'preserved_refusals':0}
    for row in trace['validation']+trace['large_finite_scope_cases']:
        exact=original_trace(row)
        assert exact==bits(row['exact_reference_bits'])
        audit['queries']+=1
        for arm in row['arms']:
            if arm['status']=='refused':
                audit['preserved_refusals']+=1;continue
            lo,hi=map(Q.from_float,arm['enclosure'])
            assert lo<=exact<=hi
            assert lo==hi==exact
            audit['accepted_intervals']+=1;audit['exact_singletons']+=1
    trace_rows=[]
    for family in trace['timing']:
        ratios=[]
        for repeat in family['repeats']:
            arms=defaultdict(float)
            for block in repeat['blocks']:
                assert block['calls']==block['work']['requests_attempted']==block['work']['requests_completed']
                assert block['failures']==block['work']['requests_rejected']==0
                arms[block['route']]+=block['seconds']
            ratios.append(arms['pr97_eager']/arms['requested_support'])
        first=family['repeats'][0]['blocks'];ops={}
        for block in first:
            ops[block['route']]={k:block['work'][k]/block['calls'] for k in ['interval_additions','interval_multiplications']}
        trace_rows.append({'family':family['family'],**stats(ratios),'operations_per_call':ops})
    groups=defaultdict(list)
    for row in switch['rows']:
        assert row['routing_and_snapshot_bit_parity']
        assert row['legacy']['copied_coordinates']-row['new']['copied_coordinates']==row['saved_coordinate_copies']
        groups[(row['dimension'],row['scenario'])].append(row)
    switch_rows=[]
    for (dimension,scenario),rows in sorted(groups.items()):
        switch_rows.append({'dimension':dimension,'scenario':scenario,**stats([r['legacy']['seconds']/r['new']['seconds'] for r in rows]),'copies_saved_per_pair':rows[0]['saved_coordinate_copies'],'legacy_copies':rows[0]['legacy']['copied_coordinates'],'new_copies':rows[0]['new']['copied_coordinates']})
    result={'schema':'vigilode.dynamics-analysis.v1','exact_companion_audit':audit,'trace':trace_rows,'switch':switch_rows,'ratio_definition':'legacy seconds / new seconds; trace sums both ABBA arm blocks within each repeat before pairing','scope':'native component full-call measurements; controller uses synthetic work receipts; not whole ODE trajectory speedup','inputs':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [trace_path,switch_path]},'analysis_source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (args.output/'ANALYSIS.json').write_text(json.dumps(result,indent=2)+'\n')
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig,axes=plt.subplots(1,2,figsize=(12,4.8),layout='constrained')
    for ax,rows,labels,title in [(axes[0],trace_rows,[r['family'] for r in trace_rows],'Quartic trace: exact singleton accuracy'),(axes[1],[r for r in switch_rows if r['scenario']=='guarded_protected'],[str(r['dimension']) for r in switch_rows if r['scenario']=='guarded_protected'],'Guarded reference updates: same routing words')]:
        for i,row in enumerate(rows):
            ax.scatter([i+(j-3)*.025 for j in range(7)],row['paired_ratios'],s=20,color='#6f8ca6',alpha=.8)
            ax.plot([i-.17,i+.17],[row['median_ratio']]*2,color='#173f5f',lw=3)
        ax.axhline(1,color='#ba3d45',lw=1.2,ls='--')
        ax.set_xticks(range(len(labels)),labels)
        ax.set_title(title,fontsize=11);ax.set_ylabel('Legacy / new elapsed time (7 pairs)')
        ax.grid(axis='y',alpha=.2)
    axes[1].set_xlabel('State dimension; one n-coordinate copy removed per update')
    fig.suptitle('Measured component costs; all pairs retained, ratio > 1 favors new',fontsize=12)
    fig.savefig(args.output/'paired_costs.png',dpi=180)
    fig.savefig(args.output/'paired_costs.pdf')
    print(json.dumps({'audit':audit,'trace':[(r['family'],r['median_ratio']) for r in trace_rows],'switch':[(r['dimension'],r['scenario'],r['median_ratio']) for r in switch_rows]}))

if __name__=='__main__': main()
