#!/usr/bin/env python3
"""Independent exact distribution check. No import from candidate implementation."""
import argparse,json
from fractions import Fraction
from pathlib import Path

def distribution(s):
    probabilities=[Fraction(1)]
    for _ in range(s):
        after=[Fraction(0)]*(len(probabilities)+1)
        for j,p in enumerate(probabilities):
            after[j]+=p/2
            after[j+1]+=p/2
        probabilities=after
    assert sum(probabilities)==1
    return probabilities

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--candidate',required=True);parser.add_argument('--output',required=True);a=parser.parse_args()
    candidate=json.loads(Path(a.candidate).read_text());rows=[]
    for row in candidate['designs']:
        s,c=row['sessions'],row['cases'];dist=distribution(s)
        eligible=[]
        for left in range(1,s//2+1):
            failure=sum(dist[:left])+sum(dist[s-left+1:])
            if c*failure<=Fraction(1,20):eligible.append((left,failure))
        expected=max(eligible,default=(None,Fraction(0)),key=lambda pair:pair[0] or 0)
        assert row['k']==expected[0],(row,expected)
        coverage=1-c*expected[1]
        assert Fraction(row['simultaneous_coverage_lower'])==coverage
        assert row['unbounded']==(expected[0] is None)
        rows.append({'sessions':s,'cases':c,'k':expected[0],'simultaneous_coverage_lower':str(coverage),'matched':True})
    out={'schema_version':'1.0','status':'PASS','method':'independent recursive exact fair-coin distribution, no candidate imports','design_count':len(rows),'checks':rows,'claim_ceiling':'Checks finite combinatorial coverage construction only, conditional on iid session vectors and fixed complete case corpus; does not verify population iid or observed speedup.'}
    Path(a.output).write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps({'status':'PASS','design_count':len(rows)}))
if __name__=='__main__':main()
