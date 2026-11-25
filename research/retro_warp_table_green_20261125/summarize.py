"""Summarize actual executed evidence. No regenerated function tables."""
import json,hashlib,re,math,statistics
from pathlib import Path
root=Path(__file__).resolve().parent
repo=root.parent.parent
def records(name,marker):
    return [json.JSONDecoder().raw_decode(l.split(marker,1)[1])[0] for l in (root/name).read_text().splitlines() if marker in l]
before=records('timing_before_cache.log','WP ')
final=records('timing.log','WP ')
assert len(before)==len(final)==30
for r in before+final:
    assert len(r['samples_ms'])==5 and statistics.median(r['samples_ms'])==r['median_ms'] and r['grid']==97
    assert math.isfinite(r['result']['error'])
def row(rows,k,arm,tol):
    return next(r for r in rows if r['k']==k and r['arm']==arm and r['tol']==tol)
cache=[]
for k in [1.,12.,1000.]:
    a=row(before,k,'retro-green',1e-5);b=row(final,k,'retro-green',1e-5)
    assert a['result']['error']==b['result']['error']
    cache.append({'k':k,'before_ms':a['median_ms'],'after_ms':b['median_ms'],'speedup':a['median_ms']/b['median_ms'],'error':b['result']['error'],
                  'before_work':a['result']['metrics']['work'],'after_work':b['result']['metrics']['work']})
def interpolate(rs,target):
    points=sorted(set((r['result']['error'],r['median_ms']) for r in rs if r['result']['error']>0))
    for (a,ta),(b,tb) in zip(points,points[1:]):
        if a<=target<=b and b>a:
            q=math.log(target/a)/math.log(b/a)
            return math.exp((1-q)*math.log(ta)+q*math.log(tb))
    return None
matched=[]
for k in [1.,12.,1000.]:
    for target in [1e-10,1e-11,1e-12]:
        a=interpolate([r for r in final if r['k']==k and r['arm']=='retro-green'],target)
        b=interpolate([r for r in final if r['k']==k and r['arm']=='rodas5p-fast'],target)
        if a is not None and b is not None:
            matched.append({'k':k,'sampled_error':target,'retro_ms_interpolated':a,'rodas_ms_interpolated':b,'retro_over_rodas':a/b})
counts=[tuple(map(int,r)) for r in re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",(root/'final_tests.log').read_text())]
assert len(counts)==4 and sum(r[0] for r in counts)==14 and all(r[1]==0 for r in counts),counts
audit=json.loads((root/'PHI_AUDIT.json').read_text());assert audit['cases']==52 and audit['underbounds']==0
data={'schema':'vigilode.retro-warp.round03.v1','base':(root/'BASE.txt').read_text().splitlines(),
      'authority':'diagnostic_only','test_counts':{'passed':sum(r[0] for r in counts),'failed':0,'ignored':sum(r[2] for r in counts),'timing_run_separately':True},
      'phi_audit':audit,'scalar':records('regression.log','SCALAR '),'coupled':records('regression.log','COUPLED '),
      'bad_node':records('regression.log','BAD_NODE '),'forecast':records('regression.log','FORECAST '),
      'cache_comparison':cache,'matched_sampled_error_interpolation':matched,'timing':final,'timing_before_cache':before,
      'source_sha256':{p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in ['crates/rodas5p-integrators/src/lib.rs','crates/rodas5p-integrators/src/retro_green.rs','crates/rodas5p-integrators/src/retro_phi_cell.hex','crates/rodas5p-integrators/tests/retro_green_contracts.rs','Cargo.lock']},
      'raw_sha256':{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(root.glob('*.log'))}}
assert len(data['scalar'])==5 and len(data['coupled'])==len(data['bad_node'])==len(data['forecast'])==1
(root/'RESULTS.json').write_text(json.dumps(data,indent=2,sort_keys=True)+'\n')
print(json.dumps({k:data[k] for k in ('test_counts','cache_comparison','matched_sampled_error_interpolation')},indent=2))
