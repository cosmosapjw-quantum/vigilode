"""Parse preserved temporal solve receipts, with no rerun or regenerated tables."""
import json,hashlib,re,statistics,math
from pathlib import Path
root=Path(__file__).resolve().parent
repo=root.parent.parent
def records(name,marker):
    return [json.JSONDecoder().raw_decode(l.split(marker,1)[1])[0] for l in (root/name).read_text().splitlines() if marker in l]
wp=records('timing.log','WP ')
old=records('timing_before_fast.log','WP ')
ordered=records('regression.log','ORDERED ')
cancel=records('regression.log','CANCEL ')
audit=json.loads((root/'AUDIT.json').read_text())
assert len(wp)==16 and len(old)==12 and len(ordered)==27 and len(cancel)==1
assert audit['nodes_checked']==512 and audit['scale_nodes_checked']==64 and audit['underbounds']==0
assert all(r['bound']<=1e-10 and r['error']<=r['bound'] for r in ordered)
for r in wp+old:
    assert r['metrics']['error_upper']<=r['tol']
    for k,v in [('raw','raw_ms'),('certificate','certificate_ms'),('fresh','fresh_ms')]:
        assert len(r[k+'_samples'])==5 and statistics.median(r[k+'_samples'])==r[v]
    assert r['prepared_total_ms']==statistics.median([a+b for a,b in zip(r['raw_samples'],r['certificate_samples'])])
counts={}
name=None
for l in (root/'final_tests.log').read_text().splitlines():
    m=re.search(r'Running tests/(\w+)\.rs',l)
    if m:name=m[1]
    m=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',l)
    if m:
        assert name
        counts[name]=dict(zip(('passed','failed','ignored'),map(int,m.groups())))
assert len(counts)==7 and all(r['failed']==0 for r in counts.values()),counts
assert counts['retro_temporal_contracts']['passed']==6
assert 'test result: ok. 1 passed; 0 failed' in (root/'timing.log').read_text()
comparisons=[]
for n,p,m in [(4,1,64),(16,2,256),(32,2,1024),(64,2,1024)]:
    arms={r['mode']:r for r in wp if (r['n'],r['p'],r['m'])==(n,p,m)}
    assert set(arms)=={'Sequential','Scan','Padded','Crt'}
    seq=arms['Sequential']
    for name in ('Scan','Padded','Crt'):
        a=arms[name]
        comparisons.append(dict(n=n,p=p,m=m,mode=name,
            raw_seq_over_mode=seq['raw_ms']/a['raw_ms'],
            prepared_seq_over_mode=seq['prepared_total_ms']/a['prepared_total_ms'],
            fresh_seq_over_mode=seq['fresh_ms']/a['fresh_ms']))
fast=[]
for r in old:
    a=next(x for x in wp if (x['n'],x['p'],x['m'],x['mode'])==(r['n'],r['p'],r['m'],r['mode']))
    fast.append(dict(n=r['n'],p=r['p'],m=r['m'],mode=r['mode'],
            before_certificate_ms=r['certificate_ms'],after_certificate_ms=a['certificate_ms'],
            ratio=r['certificate_ms']/a['certificate_ms'],
            before_bound=r['metrics']['error_upper'],after_bound=a['metrics']['error_upper']))
sources=['crates/rodas5p-integrators/src/lib.rs','crates/rodas5p-integrators/src/retro_temporal.rs',
         'crates/rodas5p-integrators/tests/retro_temporal_contracts.rs','crates/rodas5p-integrators/src/fourier_path_candidate/warp.rs',
         'crates/rodas5p-integrators/src/parallel.rs','crates/rodas5p-core/src/directed.rs','Cargo.lock']
out=dict(schema='vigilode.retro-warp.round05.v1',base=(root/'BASE.txt').read_text().splitlines(),
         authority='diagnostic_only',test_counts=counts,passed=sum(r['passed'] for r in counts.values()),timing_test_run_separately=True,
         audit=audit,ordered=ordered,cancellation_control=cancel,timing=wp,timing_before_fast=old,
         comparisons=comparisons,certificate_comparisons=fast,
         source_sha256={p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in sources},
         raw_sha256={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.glob('*.log'))})
(root/'RESULTS.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print(json.dumps({'passed':out['passed'],'test_counts':counts,'comparisons':comparisons,'certificate_comparisons':fast},indent=2))
