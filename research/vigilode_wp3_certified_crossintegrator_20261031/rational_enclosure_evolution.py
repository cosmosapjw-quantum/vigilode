#!/usr/bin/env python3
"""Exact rational Bernstein subdivision and higher-order rational integrating-factor bounds.
Source and binary64 endpoint states are immutable WP2. No trust in RK4 predictor.
"""
import hashlib, json, math, struct, sys, time
from fractions import Fraction as F
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'sources'))
import DEFECT_CERTIFIER as d


def de_casteljau_halves(b):
    assert len(b)==10
    ladder=[list(b)]
    while len(ladder[-1])>1:
        p=ladder[-1];ladder.append([(p[j]+p[j+1])/2 for j in range(len(p)-1)])
    left=[p[0] for p in ladder]
    right=[ladder[9-j][j] for j in range(10)]
    assert left[0]==b[0] and left[-1]==right[0] and right[-1]==b[-1]
    return left,right

def bounds_levels(coefs, depth=2):
    curr=[d.coeffs_bernstein9(x) for x in coefs]
    r=[max(abs(z) for b in curr for z in b)]
    for j in range(depth):
        halves=[]
        for b in curr:
            left,right=de_casteljau_halves(b)
            halves.extend([left,right])
        curr=halves
        r.append(max(abs(z) for b in curr for z in b))
    assert all(r[i+1]<=r[i] for i in range(len(r)-1))
    return r

def q_upper(kh, degree):
    assert degree>=1
    P=sum((kh**j/F(math.factorial(j)) for j in range(degree+1)),F(0))
    assert P>=1
    return d.upper_as_float(1/P)

def b_ratio_upper(x, degree):
    if degree==0:return F(1)
    assert degree in (3,5,7) and x>=0
    # Taylor odd L_degree <= e^{-x}, so (1-e^{-x})/x <= (1-L_degree)/x.
    ret=sum(((-x)**(j-1)*F(1,math.factorial(j)) for j in range(1,degree+1)),F(0))
    # Above expression equals 1-x/2+x^2/6-... for odd degree.
    assert ret>=0
    return min(F(1),ret)

def selftest():
    p=[F(i*i-3*i,7) for i in range(10)]
    l,r=de_casteljau_halves(p)
    for s in [F(0),F(1,11),F(1,2),F(3,4),F(1)]:
        assert d.bernstein9_eval(l,s)==d.bernstein9_eval(p,s/2)
        assert d.bernstein9_eval(r,s)==d.bernstein9_eval(p,(1+s)/2)
    for x in [F(0),F(1,100000),F(1,10),F(1),F(3),F(10)]:
        assert b_ratio_upper(x,3)>=0
        assert b_ratio_upper(x,5)<=b_ratio_upper(x,3)
        assert b_ratio_upper(x,7)<=b_ratio_upper(x,5)
        for k in (1,2,3,4):
            assert q_upper(x,k)>=0
    print('PROOF_SEARCH_UNIT=PASS exact subdivision transform; positive rational source envelopes; monotone Taylor hierarchy',flush=True)

def load(cohort='asymmetric_2',micro=32):
    traces,_=d.parse_log(ROOT/'sources/SWEEP.log')
    trial=traces[(cohort,2,False)]
    source=d.parse_source(trial); steps=len(trial['points'])-1
    name=f'{cohort}_micro{micro}_reference_state_bits.bin'
    bb=(ROOT/'sources'/name).read_bytes()
    certificate=json.loads((ROOT/'sources/CERTIFICATE.json').read_text())
    obj=next(v for v in certificate['micro_cohorts'][cohort] if v['reference_micro']==micro)
    assert hashlib.sha256(bb).hexdigest()==obj['reference_states_sha256']
    count=steps*micro; dim=source['n'];size=dim*8
    assert len(bb)==(count+1)*size
    ref=[tuple(struct.unpack('>'+('Q'*dim),bb[j*size:(j+1)*size])) for j in range(count+1)]
    assert tuple(map(d.rational,ref[0]))==tuple(source['y0'])
    return source,ref,steps,obj

def evaluate(cohort='asymmetric_2',micro=32,max_depth=2):
    selftest()
    src,ref,steps,parent=load(cohort,micro)
    x=min(src['k'])*src['h']/micro;h=src['h']/micro
    candidates=[{'name':f'D{depth}_Q{qdeg}_B{bdeg}','depth':depth,'q_degree':qdeg,'b_degree':bdeg} for depth in range(max_depth+1) for qdeg in (1,2,3,4) for bdeg in (0,3,5,7)]
    decays={n:q_upper(x,n) for n in [1,2,3,4]}
    source_weights={n:b_ratio_upper(x,n) for n in [0,3,5,7]}
    delta={c['name']:0.0 for c in candidates}; grid_max={c['name']:0.0 for c in candidates}
    sum_bound=[F(0)]*(max_depth+1);max_bounds=[F(0)]*(max_depth+1)
    t0=time.monotonic()
    for j in range(steps*micro):
        a=list(map(d.rational,ref[j]));b=list(map(d.rational,ref[j+1]))
        coeff,_=d.original_defect_coeffs(src,a,b,h)
        r=bounds_levels(coeff,max_depth)
        for i,v in enumerate(r):
            sum_bound[i]+=v;max_bounds[i]=max(max_bounds[i],v)
        for c in candidates:
            prev=F.from_float(delta[c['name']]);dec=F.from_float(decays[c['q_degree']]);bweight=source_weights[c['b_degree']]
            nxt=prev*dec+h*r[c['depth']]*bweight
            delta[c['name']]=d.upper_as_float(nxt)
            if (j+1)%micro==0:
                grid_max[c['name']]=max(grid_max[c['name']],delta[c['name']])
        if j and j%512==0:print('CERTIFIED_PANELS',j,'/',steps*micro,'elapsed_s',round(time.monotonic()-t0,1),flush=True)
    for c in candidates:c['max_grid_reference_delta']=grid_max[c['name']]
    c0=next(c for c in candidates if c['name']=='D0_Q1_B0')
    # Required proof that the reimplementation matches frozen original native-delta certificate.
    assert c0['max_grid_reference_delta']==parent['reference_delta_grid_up'],(c0['max_grid_reference_delta'],parent['reference_delta_grid_up'])
    best=min(candidates,key=lambda c:(c['max_grid_reference_delta'],c['depth'],c['q_degree'],c['b_degree']))
    res={'schema':'vigilode.wp3.rational_bernstein_enclosure.v1','cohort':cohort,'micro':micro,'panels':steps*micro,
       'parent_certified_delta':parent['reference_delta_grid_up'],'original_replay_delta':c0['max_grid_reference_delta'],
       'best_new_delta':best['max_grid_reference_delta'],'best':best,'candidates':candidates,
       'max_defect_by_depth':[float(z) for z in max_bounds],
       'mean_defect_by_depth':[float(z/(steps*micro)) for z in sum_bound],
       'source_time_x':float(x),'clock_bound_previously_certified':parent['clock_bound_float'],
       'input_source_sha256':hashlib.sha256((ROOT/'sources/SWEEP.log').read_bytes()).hexdigest(),
       'reference_sha256':parent['reference_states_sha256'],
       'proof_assumption':'exact represented binary64 source coefficients; de Casteljau hull; e^-x <= 1/Pq(x); exp alternating Taylor lower bound for B; upward dyadic roundoff',
       'elapsed_seconds':time.monotonic()-t0}
    path=ROOT/'results'/f'RATIONAL_ENCLOSURE_{cohort}_micro{micro}.json';path.write_text(json.dumps(res,indent=2)+'\n')
    print('REFINEMENT_RESULT',json.dumps({k:res[k] for k in ('panels','parent_certified_delta','best_new_delta','best','max_defect_by_depth','elapsed_seconds')}),flush=True)
    return res

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--cohort',default='asymmetric_2',choices=d.COHORTS);p.add_argument('--micro',type=int,choices=(32,64),default=32);p.add_argument('--depth',type=int,default=2); args=p.parse_args()
    evaluate(args.cohort,args.micro,args.depth)
