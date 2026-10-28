"""Exact n=6 resampling distribution; numerical sampling only for outer coverage.
All cases identical within session, balanced sessions, so case resampling changes nothing.
No actual solver performance measurements are used.
"""
import itertools, math, json, pathlib
import numpy as np
root=pathlib.Path(__file__).resolve().parent
n=6
weights={}
for indices in itertools.product(range(n),repeat=n):
    s=sorted(indices); pair=(s[2],s[3]);weights[pair]=weights.get(pair,0)+1
pairs=np.array(list(weights));w=np.array(list(weights.values()));total=n**n
assert int(w.sum())==46656
rng=np.random.default_rng(20260930)
trials=20000
out=[]
for family in ('uniform','gaussian'):
    values=rng.uniform(-0.3,0.3,(trials,n)) if family=='uniform' else rng.normal(0,0.15,(trials,n))
    values.sort(axis=1)
    vals=(values[:,pairs[:,0]]+values[:,pairs[:,1]])*.5
    order=np.argsort(vals,axis=1)
    v=np.take_along_axis(vals,order,axis=1); ws=np.take_along_axis(np.broadcast_to(w,vals.shape),order,axis=1)
    cum=np.cumsum(ws,axis=1)
    def quant(q):
        pos=q*(total-1); lo=int(math.floor(pos));hi=int(math.ceil(pos));f=pos-lo
        ilo=(cum<=lo).sum(axis=1);ihi=(cum<=hi).sum(axis=1);rows=np.arange(trials)
        return (1-f)*v[rows,ilo]+f*v[rows,ihi]
    low,high=quant(.025),quant(.975)
    covered=(low<=0)&(high>=0)
    rate=float(covered.mean()); se=math.sqrt(rate*(1-rate)/trials)
    out.append(dict(family=family,independent_sessions=n,trials=trials,exact_bootstrap_replicates=total,nominal_coverage=.95,coverage=rate,outer_binomial_standard_error=se,false_lower_excludes_truth=float((low>0).mean()),false_upper_excludes_truth=float((high<0).mean())))
# Independent oracle for the native one-bootstrap-resample fixture.
values=np.log([.8,.9,1.,1.1,1.4,1.5]); vals=(values[pairs[:,0]]+values[pairs[:,1]])*.5
order=np.argsort(vals); expanded=np.repeat(vals[order],w[order]);ci=np.quantile(expanded,[.025,.975])
result={"evidence":"numerically checked exact finite empirical bootstrap distribution; coverage Monte Carlo, not a theorem for all populations","seed":20260930,"coverage":out,"native_fixture_exact_bootstrap":{"point":float(np.exp(np.median(values))),"lower":float(np.exp(ci[0])),"upper":float(np.exp(ci[1]))},"method":"enumerate all 6^6 ordered session bootstrap samples, no pseudo-random inner bootstrap"}
(root/'exact_bootstrap_oracle.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
