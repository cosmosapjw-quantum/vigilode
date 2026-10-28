"""Measured stage RHS batching on a memory-bound 1D reaction-diffusion kernel.
Kernel only: no solver accuracy or end-to-end scaling inference is admitted.
Includes allocation, scheduling, result collection; pool setup is recorded apart.
"""
import os,time,json,random,statistics
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import numpy as np
from probe import OUT,save

def rhs(y):
    # periodic discrete reaction-diffusion, dimensionless test problem
    return 100.*(np.roll(y,-1)-2*y+np.roll(y,1))+y-y*y*y

def main():
    rng=np.random.default_rng(50);order_rng=random.Random(20261020)
    pools={};setup={}
    for p in [2,4]:
        t=time.perf_counter_ns();pools[p]=ThreadPoolExecutor(max_workers=p)
        list(pools[p].map(lambda x:x,range(8)));setup[str(p)]=time.perf_counter_ns()-t
    raw=[];checks=[]
    for n in [256,16384,131072]:
        Y=rng.normal(size=(8,n))*.1
        def batch(p):
            if p==1:return np.stack([rhs(y) for y in Y])
            return np.stack(list(pools[p].map(rhs,[y for y in Y])))
        ref=batch(1)
        for p in [1,2,4]:
            got=batch(p);checks.append({'n':n,'workers':p,'bit_equal':bool(np.array_equal(got.view(np.uint64),ref.view(np.uint64)))})
            for _ in range(2):batch(p)
        for block in range(12):
            order=[1,2,4];order_rng.shuffle(order)
            for p in order:
                ns=[]
                for _ in range(5):
                    t=time.perf_counter_ns();batch(p);ns.append(time.perf_counter_ns()-t)
                raw.append({'n':n,'workers':p,'block':block,'samples_ns':ns,'median_ns':statistics.median(ns)})
    for p in pools.values():p.shutdown()
    summary=[]
    for n in [256,16384,131072]:
        for p in [1,2,4]:
            sel=[r for r in raw if r['n']==n and r['workers']==p]
            ratios=[]
            for r in sel:
                b=next(x for x in raw if x['n']==n and x['workers']==1 and x['block']==r['block'])
                ratios.append(b['median_ns']/r['median_ns'])
            summary.append({'n':n,'workers':p,'median_ms':statistics.median(r['median_ns'] for r in sel)/1e6,
                            'median_paired_speed_ratio':statistics.median(ratios),
                            'paired_ratio_min':min(ratios),'paired_ratio_max':max(ratios)})
    assert all(c['bit_equal'] for c in checks)
    save('stage_kernel_timing.json',{'raw':raw,'summary':summary,'checks':checks,'pool_setup_ns':setup,
         'cpu_affinity':sorted(os.sched_getaffinity(0)),'blocks':12,'repeats_per_block':5,
         'scope':'single-process warmed thread-pool kernel experiment, not native solver speed or independent-host confidence interval',
         'assembly':False,'output_allocation_and_collection_included':True})
    print(json.dumps(summary,indent=2))
if __name__=='__main__':main()
