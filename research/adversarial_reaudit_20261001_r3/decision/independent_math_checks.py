#!/usr/bin/env python3
"""Reviewer-selected checks: no candidate implementation or production mutation."""
from fractions import Fraction as Q
from itertools import product
from collections import Counter
from math import comb, sqrt, log, exp
from pathlib import Path
import json

root=Path(__file__).resolve().parent
# The six-session statistic can be constructed from the middle two sorted
# represented ratios: exp((log r3+log r4)/2) = sqrt(r3*r4).
ratios=[Q(4,5),Q(9,10),Q(1),Q(11,10),Q(7,5),Q(3,2)]
products=Counter()
for draw in product(range(6), repeat=6):
    x=sorted(ratios[i] for i in draw)
    products[x[2]*x[3]]+=1
ordered=sorted(products)
# Reference linear-interpolated empirical quantiles: endpoints at both
# adjoining ranks belong to one exact-product atom in this fixture.
n=6**6
checks=[]
for prob in [Q(1,40), Q(39,40)]:
    r=prob*(n-1); lo=r.numerator//r.denominator; hi=(r.numerator+r.denominator-1)//r.denominator
    ranks={}; count=0
    for x in ordered:
        if count<=lo<count+products[x]: ranks['lo']=x
        if count<=hi<count+products[x]: ranks['hi']=x
        count+=products[x]
    assert ranks['lo']==ranks['hi']
    checks.append({'probability':str(prob),'rank_lo':lo,'rank_hi':hi,'square_of_endpoint':str(ranks['lo']),'endpoint':sqrt(float(ranks['lo']))})
# Direct finite-binomial coverage check for smaller B; exact probability
# sums use rational p, while epsilon is rounded float (well away from
# integer thresholds). This checks theorem direction and delta allocation.
delta=.01; B=64; epsilon=sqrt(log(2/delta)/(2*B)); probs=[]
for num in range(1,20):
    p=Q(num,20); failure=Q(0)
    for k in range(B+1):
        if abs(float(Q(k,B)-p))>epsilon:
            failure += comb(B,k)*p**k*(1-p)**(B-k)
    assert float(failure)<=delta
    probs.append({'p':str(p),'two_sided_failure':float(failure)})
result={'reviewer_independent_design':True,'scope':'Independent exact empirical endpoints and finite-binomial theorem-direction sanity check. Does not validate population-bootstrap coverage, PRNG randomness or production integration.','six_session_exact_endpoints':checks,'six_session_outcomes':n,'binomial_check':{'B':B,'delta':delta,'epsilon':epsilon,'p_grid_results':probs,'maximum_failure':max(x['two_sided_failure'] for x in probs)},'status':'PASS_WITHIN_STATED_SCOPE'}
(root/'independent_math_checks.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
