#!/usr/bin/env python3
"""Finite-sample session-median candidate; standard library and exact integer coverage."""
import argparse, itertools, json, math
from fractions import Fraction

def median(x):
    y=sorted(x); n=len(y)
    if not n: raise ValueError('empty median')
    return y[n//2] if n%2 else (y[n//2-1]+y[n//2])/2

def choice(s,c,alpha=Fraction(1,20)):
    for k in range((s+1)//2,0,-1):
        tail=Fraction(2*sum(math.comb(s,j) for j in range(k)),2**s)
        if tail <= alpha/c:
            return k,tail
    return None,Fraction(0)

def interval(rows,declared_cases,alpha=Fraction(1,20)):
    ids=[r['session'] for r in rows]
    if len(set(ids))!=len(ids): raise ValueError('duplicate session')
    if not rows or not declared_cases: raise ValueError('empty design')
    if len(set(declared_cases))!=len(declared_cases): raise ValueError('duplicate case')
    for r in rows:
        if set(r['cells'])!=set(declared_cases): raise ValueError('missing/extra case')
        if not all(math.isfinite(v) for v in r['cells'].values()): raise ValueError('nonfinite cell')
        if r.get('failed',False): raise ValueError('failed session')
    k,tail=choice(len(rows),len(declared_cases),alpha)
    if k is None:return {'status':'UNBOUNDED','decision':'Inconclusive','lower':None,'upper':None,'k':None,'coverage_lower':'1'}
    lo=[];hi=[]
    for c in declared_cases:
        z=sorted(r['cells'][c] for r in rows);lo.append(z[k-1]);hi.append(z[-k])
    l,u=median(lo),median(hi);threshold=math.log(1.15)
    return {'status':'FINITE','lower':l,'upper':u,'k':k,'coverage_lower':str(1-len(declared_cases)*tail),'decision':'Promote' if l>threshold else 'Block' if u<threshold else 'Inconclusive'}

def main():
    p=argparse.ArgumentParser();p.add_argument('--output',required=True);args=p.parse_args()
    rows=[]
    for s in range(1,17):
        for c in [1,5]:
            k,t=choice(s,c)
            cov=Fraction(1)-c*t if k else Fraction(1)
            assert cov>=Fraction(19,20)
            rows.append({'sessions':s,'cases':c,'k':k,'per_case_failure':str(t),'simultaneous_coverage_lower':str(cov),'unbounded':k is None})
    enumerations=[]
    for s,c in itertools.product([6,8,12],[1,5]):
        k,t=choice(s,c)
        accepted=0
        for signs in itertools.product([0,1],repeat=s):
            n=sum(signs)
            accepted += k is None or k<=n<=s-k
        exact=Fraction(accepted,2**s)
        expected=1-t if k else Fraction(1)
        assert exact==expected
        enumerations.append({'sessions':s,'cases':c,'k':k,'accepted_sign_patterns':accepted,'total':2**s,'per_case_coverage':str(exact)})
    assert choice(6,1)[0]==1 and choice(6,5)[0] is None and choice(8,5)[0]==1
    controls=[]
    for s,c in [(6,1),(6,5),(8,5),(12,5)]:
        for ratio in [0.9,1.15,1.3]:
            cases=[str(j) for j in range(c)]
            sample=[{'session':i,'cells':{j:math.log(ratio) for j in cases}} for i in range(s)]
            result=interval(sample,cases)
            expected='Inconclusive' if ratio==1.15 or choice(s,c)[0] is None else 'Promote' if ratio>1.15 else 'Block'
            assert result['decision']==expected
            controls.append({'sessions':s,'cases':c,'ratio':ratio,**result})
    invalid=[]
    for mutation in ['duplicate_session','missing_case','nan','failed_session']:
        sample=[{'session':i,'cells':{'a':0.3}} for i in range(6)]
        if mutation=='duplicate_session':sample[1]['session']=0
        elif mutation=='missing_case':sample[1]['cells']={}
        elif mutation=='nan':sample[1]['cells']['a']=math.nan
        else:sample[1]['failed']=True
        try: interval(sample,['a'])
        except ValueError as e: invalid.append({'mutation':mutation,'rejected':str(e)})
        else: raise AssertionError(mutation)
    report={'schema':'vigilode-r4-exact-session-median-v1','status':'EXACT_CHECKS_PASS','source_commit':'1c54194123ee6abc6daa512e8574922f510b4e2c','alpha':'1/20','estimand':'median over fixed cases of population median of complete session-cell median log speedup','assumptions':['independent identically distributed session vectors','complete declared fixed case corpus','non-informative session inclusion','specified medians; ties conservative'],'not_claimed':['equivalence to pooled-pair population median outside symmetric model','general replacement validated in production','wall-time speedup','arbitrary missingness robustness'],'designs':rows,'enumerations':enumerations,'controls':controls,'invalid_inputs':invalid}
    with open(args.output,'w') as f:json.dump(report,f,indent=2,allow_nan=False);f.write('\n')
    print(json.dumps({'status':report['status'],'designs':len(rows),'enumerations':len(enumerations),'controls':len(controls),'invalid_inputs':len(invalid)}))
if __name__=='__main__':main()
