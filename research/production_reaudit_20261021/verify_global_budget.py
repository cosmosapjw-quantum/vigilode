#!/usr/bin/env python3
"""Exact algebraic witnesses for G1/G2; not an ODE benchmark."""
from fractions import Fraction as Q
from pathlib import Path
import hashlib
import json

# Endpoint comparison with arbitrary rational gains. Future envelope uses
# max(1,gain), so it bounds every prefix, including contraction/expansion.
cases=[]
for gains in ([Q(1),Q(1),Q(1)],[Q(1,2),Q(3,2),Q(1,3)], [Q(2),Q(3),Q(1,4)]):
    local=[Q(1,100),Q(1,200),Q(1,400)]
    radius=Q(1,1000)
    envelope=[]
    for i in range(4):
        a=Q(1)
        for g in gains[i:]: a*=max(Q(1),g)
        envelope.append(a)
    bound=envelope[0]*radius+sum((envelope[i+1]*e for i,e in enumerate(local)),Q(0))
    actual=[radius]
    for g,e in zip(gains,local): actual.append(g*actual[-1]+e)
    assert max(actual)<=bound
    cases.append({'gains':list(map(str,gains)),'prefix_radii':list(map(str,actual)), 'uniform_bound':str(bound)})
# G2 exact optimum at p=1, square A values.
a=[Q(1),Q(81)]; c=[Q(1),Q(1)]; budget=Q(1,10)
optimal=[100,900]; uniform=[820,820]
def error(n):return sum((x/k for x,k in zip(a,n)),Q(0))
def work(n):return sum((x*k for x,k in zip(c,n)),Q(0))
assert error(optimal)==error(uniform)==budget
assert work(optimal)==1000 and work(uniform)==1640
# Integer perturbations around optimum cannot improve cost at equal budget.
feasible_neighbours=0
for n1 in range(90,111):
    for n2 in range(890,911):
        if error([n1,n2])<=budget:
            feasible_neighbours+=1
            assert work([n1,n2])>=work(optimal)
out={'schema':'vigilode.global-budget-proof.v1','scope':'Exact rational algebraic witnesses; conditional counted-work model, no native solver or wall-time claim','passed':True,'G1_cases':cases,'G2':{'p':1,'A':list(map(str,a)),'cost':list(map(str,c)),'budget':str(budget),'optimal_counts':optimal,'uniform_counts':uniform,'optimal_error':str(error(optimal)),'uniform_error':str(error(uniform)),'optimal_work':str(work(optimal)),'uniform_work':str(work(uniform)),'relative_work_reduction':str(1-work(optimal)/work(uniform)),'feasible_neighbours_checked':feasible_neighbours}}
out['script_sha256']=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
p=Path('research/production_reaudit_20261021/evidence/dense_transaction/GLOBAL_BUDGET_PROOF.json');p.write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'passed':True,'G1_cases':len(cases),'G2_work':[str(work(uniform)),str(work(optimal))],'neighbours':feasible_neighbours}))
