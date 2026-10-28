#!/usr/bin/env python3
"""Independent finite bootstrap oracle and a simultaneous Monte Carlo gate.

This bounds only Monte Carlo error conditional on the empirical six-session
bootstrap distribution. It is not a coverage proof for population speedups.
No external dependencies. Outputs beside this script.
"""
from collections import Counter
from itertools import product
import json
import math
from pathlib import Path

MASK = (1 << 64) - 1

class SplitMix:
    def __init__(self, seed): self.state = seed
    def next(self):
        self.state = (self.state + 0x9e3779b97f4a7c15) & MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xbf58476d1ce4e5b9) & MASK
        z = ((z ^ (z >> 27)) * 0x94d049bb133111eb) & MASK
        return z ^ (z >> 31)
    def index(self, bound): return (self.next() * bound) >> 64

def median(a):
    s = sorted(a)
    return 0.5 * (s[len(s)//2-1] + s[len(s)//2])

def quantile(a, p):
    s = sorted(a); x = p*(len(s)-1); lo = math.floor(x); hi = math.ceil(x)
    return s[lo] * (1-(x-lo)) + s[hi] * (x-lo)

def mc_draws(logs, seed, b):
    rng = SplitMix(seed); result=[]
    for _ in range(b):
        draw = [logs[rng.index(6)] for _ in range(6)]
        rng.index(1)  # Native one-case bootstrap still draws the case index.
        result.append(median(draw))
    return result

def admission(draws, log_required, delta=0.01, tail=0.025):
    # Two one-sided Hoeffding bounds with delta/2 each. This is at a fixed
    # preregistered decision threshold, not a simultaneous-in-x CDF bound.
    b=len(draws); k=sum(x < log_required for x in draws)
    epsilon=math.sqrt(math.log(2/delta)/(2*b))
    lower=max(0.,k/b-epsilon); upper=min(1.,k/b+epsilon)
    decision="promote" if upper < tail else "block" if lower > 1-tail else "inconclusive"
    return dict(b=b,count_below=k,empirical_cdf=k/b,lower=lower,upper=upper,
                epsilon=epsilon,conditional_mc_confidence=1-delta,decision=decision)

def main():
    ratios=[.8,.9,1.,1.1,1.4,1.5]
    logs=list(map(math.log,ratios))
    exact=[median([logs[j] for j in ix]) for ix in product(range(6),repeat=6)]
    exact_endpoints=[math.exp(quantile(exact,p)) for p in [.025,.975]]
    draws=mc_draws(logs,1,10000)
    sampled_endpoints=[math.exp(quantile(draws,p)) for p in [.025,.975]]
    assert max(abs(a-b) for a,b in zip(exact_endpoints,sampled_endpoints)) < 1e-12
    # A fixed, exploratory boundary fixture is deliberately constructed
    # from this distribution and explicitly is not an independent holdout.
    unique=sorted(set(exact)); cumulative=Counter(exact)
    running=0; boundaries=[]
    for i,x in enumerate(unique[:-1]):
        running+=cumulative[x]
        prob=running/len(exact)
        if .015 < prob < .04:
            boundaries.append((abs(prob-.025),prob,.5*(x+unique[i+1])))
    _,prob,boundary=min(boundaries)
    scale=math.log(1.15)-boundary
    shifted=[x+scale for x in draws]
    naive_lower=math.exp(quantile(shifted,.025))
    boundary_result=admission(shifted,math.log(1.15))
    assert boundary_result['decision']=='inconclusive'
    good=admission([math.log(1.3)]*10000,math.log(1.15))
    bad=admission([math.log(.9)]*10000,math.log(1.15))
    assert good['decision']=='promote' and bad['decision']=='block'
    result=dict(status="NUMERICALLY_CHECKED",reference="all 6^6 draws, equal five pairs/session, one case",
        exact_outcomes=len(exact),exact_endpoints=exact_endpoints,sampled_endpoints=sampled_endpoints,
        original=admission(draws,math.log(1.15)),
        boundary=dict(scale=math.exp(scale),exact_cdf_below_required=prob,naive_lower=naive_lower,
                      naive_decision="promote" if naive_lower>=1.15 else "inconclusive",mc_gate=boundary_result,
                      holdout=False),good=good,bad=bad,
        limits=["Conditional empirical bootstrap Monte Carlo error only; no population confidence guarantee",
                "Independent identical bootstrap replicates and fixed B/threshold; optional stopping excluded",
                "SplitMix multiply-shift is only approximately uniform; see documented negligible PRNG bias",
                "No production edits, no observed performance, no independent reviewer decision here"])
    Path(__file__).with_suffix('.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))

if __name__=='__main__': main()
