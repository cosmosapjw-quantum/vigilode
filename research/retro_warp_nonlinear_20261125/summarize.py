"""Summarize preserved Round06 receipts without re-running solver or table generation."""
import json,hashlib,re,statistics,math
from pathlib import Path
root=Path(__file__).resolve().parent
repo=root.parent.parent
def rows(file,marker):
    return [json.JSONDecoder().raw_decode(l.split(marker,1)[1])[0] for l in (root/file).read_text().splitlines() if marker+'{' in l]
wp=rows('timing.log','WP ')
native=rows('native.log','NATIVE ')
old=rows('timing_before_extension.log','WP ')
diag=rows('regression.log','DIAG ')
ivp=rows('regression.log','IVP ')
theta=rows('regression.log','THETA ')
bad=rows('regression.log','BAD ')
coupled=rows('regression.log','COUPLED ')
audit=json.loads((root/'AUDIT.json').read_text())
assert (len(wp),len(native),len(old),len(diag),len(ivp),len(theta),len(bad),len(coupled))==(15,27,12,3,9,27,1,4)
assert audit['formal_coefficient_count']==408 and audit['endpoint_nodes']==88 and audit['underbounds']==0
assert all(r['metrics']['bound']<=r['tol'] for r in wp)
assert all(r['observed_error']<=r['continuous_bound']+4e-15 for r in ivp+theta)
assert all(r['discrete_bound']<=1e-10 for r in ivp+theta)
for r in wp+native+old:
    assert len(r['samples_ms'])==5 and statistics.median(r['samples_ms'])==r['median_ms']
counts={};name=None
for l in (root/'final_tests.log').read_text().splitlines():
    m=re.search(r'Running tests/(\w+)\.rs',l)
    if m:name=m[1]
    m=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',l)
    if m:
        assert name
        counts[name]=dict(zip(('passed','failed','ignored'),map(int,m.groups())))
assert len(counts)==8 and sum(v['passed'] for v in counts.values())==58 and all(v['failed']==0 for v in counts.values()),counts
for f in ('timing.log','native.log'):assert 'test result: ok. 1 passed; 0 failed' in (root/f).read_text()
comp=[]
for n,steps in [(3,16),(8,64),(16,128)]:
    arms={r['arm']:r for r in wp if (r['n'],r['steps'])==(n,steps)}
    assert set(arms)=={'sequential','chord','collapsed','multi','degree2'}
    a=arms['sequential'];b=arms['collapsed'];c=arms['multi']
    comp.append(dict(n=n,steps=steps,seq_over_collapsed=a['median_ms']/b['median_ms'],
        chord_over_collapsed=arms['chord']['median_ms']/b['median_ms'],
        multi_over_collapsed=c['median_ms']/b['median_ms'],
        seq_over_degree2=a['median_ms']/arms['degree2']['median_ms']))
sources=['crates/rodas5p-integrators/src/lib.rs','crates/rodas5p-integrators/src/retro_nonlinear.rs','crates/rodas5p-integrators/src/retro_jet_table.txt',
         'crates/rodas5p-integrators/tests/retro_nonlinear_contracts.rs','crates/rodas5p-core/src/matrix.rs','crates/rodas5p-core/src/directed.rs','Cargo.lock']
out=dict(schema='vigilode.retro-warp.round06.v1',base=(root/'BASE.txt').read_text().splitlines(),authority='diagnostic_only',
         test_counts=counts,passed=sum(v['passed'] for v in counts.values()),timing_tests_run_separately=2,
         audit=audit,diagonal=diag,scalar_midpoint=ivp,theta=theta,bad_candidate=bad,coupled=coupled,
         timing=wp,timing_before_extension=old,native=native,comparisons=comp,
         source_sha256={p:hashlib.sha256((repo/p).read_bytes()).hexdigest() for p in sources},
         raw_sha256={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.glob('*.log'))})
(root/'RESULTS.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print(json.dumps({'passed':out['passed'],'comparisons':comp,'tests':counts},indent=2))
