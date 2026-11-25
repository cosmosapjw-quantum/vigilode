"""Audit actual selection receipts and summarize preserved timing, no rerun."""
from pathlib import Path
from collections import Counter
import json,statistics,re,hashlib,math
root=Path(__file__).resolve().parent
repo=root.parent.parent
def rows(file,marker):
    return [json.JSONDecoder().raw_decode(l.split(marker,1)[1])[0]
            for l in (root/file).read_text().splitlines() if marker+'{' in l]
receipts=rows('regression.log','RECEIPT ')
spatial=rows('spatial.log','SPATIAL ')+rows('spatial.log','SPATIAL_IVP ')+rows('spatial.log','SPATIAL_WRONG ')
assert len(receipts)>40 and len(spatial)==10
selected=Counter()
rejected=Counter()
errors=Counter()
for r in receipts+spatial:
    a=r['attempts'];assert 1<=len(a)<=24
    if 'total_ns' in r: assert r['total_ns']==sum(x['elapsed_ns'] for x in a)
    assert all(x['elapsed_ns']>=1 for x in a)
    if r['selected'] is None: assert all(x['status']!='Qualified' for x in a)
    else:
        assert a[-1]['route']==r['selected'] and a[-1]['status']=='Qualified'
        assert all(x['status']!='Qualified' for x in a[:-1])
        assert math.isfinite(a[-1]['bound']) and a[-1]['bound']>=0
        selected[r['selected']]+=1
    for x in a:
        if x['status']=='Rejected':rejected[x['route']]+=1
        if x['status']=='Error':errors[x['route']]+=1
expected={'CubicFolded','CubicBlock','FourierDirect','FourierLegacy','FftFull','FftCrt',
          'TemporalSequential','TemporalScan','TemporalPadded','TemporalCrt',
          'NonlinearSequential','NonlinearChord','Jet2','Jet4','Jet6','JetMulti',
          'AffineForward','AffineUniform','AffineRetro','Green','GreenHalf','GreenQuarter','NativeRodas',
          'SpatialDirect','SpatialPaired','SpatialComplex','SpatialNative','SpatialIvPPaired','SpatialIvPComplex'}
assert set(selected)==expected,(set(selected),expected-set(selected))
stiff=next(r for r in receipts if r['family']=='stiff-green-recovery')
assert stiff['selected']=='Green' and stiff['attempts'][0]['status']=='Rejected'
wrong=rows('spatial.log','SPATIAL_WRONG ')[0]
assert wrong['selected']=='SpatialDirect' and all(a['status']=='Rejected' for a in wrong['attempts'][:2])
policy=rows('policy.log','POLICY ')
assert len(policy)==30
summary=[]
for family in ('temporal','affine-dense'):
    for arm in ('cold-fixed','adaptive','round-robin'):
        rs=[r for r in policy if (r['family'],r['arm'])==(family,arm)]
        assert len(rs)==5 and {r['rep'] for r in rs}=={1,2,3,4,5}
        for r in rs:
            assert r['calls']==sum(r['selected'].values())==24 and r['bound']<=(1e-10 if family=='temporal' else 1e-7)
        summary.append(dict(family=family,arm=arm,median_total_ms=statistics.median(r['total_ms'] for r in rs),
            median_late_ms=statistics.median(r['late_ms'] for r in rs),max_bound=max(r['bound'] for r in rs),
            max_observed_difference=max(r['observed_error'] for r in rs),
            selected=[r['selected'] for r in rs],failed_attempts=sum(r['failed_attempts'] for r in rs)))
counts={}
for filename in ('final_integrators.log','final_core.log','final_spatial.log'):
    name=None
    for l in (root/filename).read_text().splitlines():
        m=re.search(r'Running tests/(\w+)\.rs',l)
        if m:name=m[1]
        m=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',l)
        if m:
            assert name
            counts[name]=dict(zip(('passed','failed','ignored'),map(int,m.groups())))
assert len(counts)==12 and all(r['failed']==0 for r in counts.values()),counts
assert counts['retro_portfolio_contracts']['passed']==7 and counts['spatial_portfolio']['passed']==3
assert 'test result: ok. 1 passed; 0 failed' in (root/'policy.log').read_text()
sources=['crates/rodas5p-integrators/src/lib.rs','crates/rodas5p-integrators/src/retro_portfolio.rs',
    'crates/rodas5p-integrators/src/retro_nonlinear.rs','crates/rodas5p-integrators/tests/retro_portfolio_contracts.rs',
    'crates/vigilode/src/spatial.rs','crates/vigilode/src/spatial/portfolio.rs','crates/vigilode/tests/spatial_portfolio.rs','Cargo.lock']
out=dict(schema='vigilode.retro-warp.round07.v1',base=(root/'BASE.txt').read_text().splitlines(),
    authority='diagnostic_only',passed=sum(r['passed'] for r in counts.values()),test_counts=counts,
    routes_qualified=len(selected),selected=selected,rejected=rejected,errors=errors,
    integrator_receipts=receipts,spatial_receipts=spatial,policy=policy,policy_summary=summary,
    source_sha256={p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in sources},
    raw_sha256={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.glob('*.log'))})
(root/'RESULTS.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print(json.dumps({k:out[k] for k in ('passed','test_counts','routes_qualified','selected','rejected','errors','policy_summary')},indent=2))
