#!/usr/bin/env python3
"""Independent frozen-row readback. Never reruns solvers or rewrites raw states."""
import csv,gzip,hashlib,json,math,statistics,sys
from collections import Counter
from pathlib import Path
out=Path(sys.argv[1]);refs=json.loads((out/'REFERENCES.json').read_text());manifest=json.loads((out/'MANIFEST.json').read_text())
cache={};checked=Counter();cal={};units=[]
def state(h):
    if h not in cache:
        with gzip.open(out/'states'/(h+'.json.gz'),'rb') as f:b=f.read()
        assert hashlib.sha256(b).hexdigest()==h
        cache[h]=json.loads(b)
    return cache[h]
def wrms(a,b,s):
    return math.sqrt(math.fsum(((x-y)/z)**2 for x,y,z in zip(a,b,s))/len(a))
def close(a,b):return math.isclose(a,b,rel_tol=3e-12,abs_tol=3e-25)
for filename,phase in [('accuracy.jsonl.gz','accuracy'),('timing.jsonl.gz','timing')]:
    with gzip.open(out/filename,'rt') as f:
        for line in f:
            rec=json.loads(line);r=rec['row'];m=rec['metrics'];obj=state(r['state_sha256']);ref=refs[r['case']+'|'+r['grid']];checked[phase]+=1
            assert r['binary_sha256']==manifest['binary_sha256'][r['arm']]
            assert r['source_commit']==manifest['parent_source' if r['arm']=='parent-slices' else 'candidate_source']
            if m['valid']:
                assert r['status']=='ok' and r['deterministic'] is True and obj['times']==ref['times']
                ys=obj['states'];vs=ref['y'];rs=ref['rho'];n=2*int(r['case'].rsplit('-',1)[1])
                assert len(ys)==len(vs)==len(rs)==len(obj['times']) and all(len(y)==n for y in ys)
                assert all(math.isfinite(x) for y in ys for x in y)
                er=0.;rr=0.
                for y,v,rho in zip(ys,vs,rs):
                    den=[1+abs(x) for x in v];er=max(er,wrms(y,v,den));rr=max(rr,wrms(rho,[0.]*n,den))
                assert close(er,m['E_raw']) and close(rr,m['E_reference']) and close(er+rr,m['E_up'])
                assert close((er+rr)/r['rtol'],m['R_up'])
                checked['recomputed_metric']+=1
            else:assert m['E_up'] is None and m['R_up'] is None
            if phase=='accuracy':cal[(r['case'],r['grid'],r['rtol'],r['h0_scale'],r['arm'])]=rec
            else:
                u=rec['unit'];units.append(u)
                if u['valid']:
                    assert len(r['elapsed_ns'])==r['repetitions'] and r['warmups']==1
                    assert u['median_ns']==statistics.median(r['elapsed_ns'])
                    assert r['state_sha256']==cal[(r['case'],r['grid'],1e-6,1.,r['arm'])]['row']['state_sha256']
                    checked['timing_raw_link']+=1
for p in json.loads((out/'PARITY.json').read_text()):
    assert p['equal']
    key=(p['case'],p['grid'],p['rtol'],p['h0_scale'])
    a=cal[key+('parent-slices',)]['row'];b=cal[key+(p['candidate'],)]['row']
    assert a['state_sha256']==b['state_sha256']
    assert a['attempts']==b['attempts'] and a['accepted']==b['accepted'] and a['rejected']==b['rejected']
    for k,v in a['counters'].items():
        if k!='direct_factorizations':assert v==b['counters'][k]
    checked['same_output_parity']+=1
for p in json.loads((out/'PAIRED.json').read_text()):
    rs=[]
    for block in range(12):
        q={u['arm']:u for u in units if u['case']==p['case'] and u['grid']==p['grid'] and u['block']==block}
        a=q['parent-slices'];b=q[p['candidate']]
        if a['valid'] and b['valid']:rs.append(a['median_ns']/b['median_ns'])
    assert rs==p['ratios'] and len(rs)==p['blocks']
    if p['status']=='COMPLETE_DIAGNOSTIC':
        assert len(rs)==12 and statistics.median(rs)==p['ratio_median'];checked['paired_recomputed']+=1
summary=json.loads((out/'SUMMARY.json').read_text())
assert checked['accuracy']==summary['accuracy_rows'] and checked['timing']==summary['timing_processes']
assert hashlib.sha256((out/'FROZEN_TIMING.json').read_bytes()).hexdigest()==manifest['timing_selection_sha256']
audit={'status':'PASS_SCOPED_READBACK','checked':dict(checked),'verified_state_objects':len(cache),
    'independence':'different metric implementation; same agent authorship, not independent personnel',
    'uncertified':'continuous time error, covariance of shared-host noise, general performance ranking'}
(out/'AUDIT.json').write_text(json.dumps(audit,indent=2)+'\n');print(json.dumps(audit))
pairs=json.loads((out/'PAIRED.json').read_text());cost=list(csv.DictReader((out/'COST.csv').open()))
lines=['# Banded exact-reuse improvement measurements','', 'This report is generated from retained new-run observations. No historical wall times are pooled.','',
    '## Scope and checks','', '```json',json.dumps(summary,indent=2),'```','', '```json',json.dumps(audit,indent=2),'```','',
    '## Integration cost and actual factor counts','', '|Case|Grid|Arm|ms|Attempts|LU|Exact reuse|Jacobian|E_empirical|',
    '|---|---|---|---:|---:|---:|---:|---:|---:|']
for r in cost:lines.append('|'+ '|'.join(r[k] for k in ['case','grid','arm','median_ms','attempts','factorizations','lu_reuses','jacobians','E_up'])+'|')
lines+=['','## Parent/candidate paired ratios','', 'Ratios >1 mean shorter candidate time. Conditional bootstrap intervals include a Bonferroni allocation for36registered comparisons, but shared-runner noise and12blocks limit coverage claims.','',
    '|Case|Grid|Candidate|Ratio|Family interval|','|---|---|---|---:|---|']
for p in pairs:
    if 'ratio_median' in p:lines.append(f"|{p['case']}|{p['grid']}|{p['candidate']}|{p['ratio_median']:.5f}|[{p['ci95_family'][0]:.5f}, {p['ci95_family'][1]:.5f}]|")
lines+=['','## Interpretation limits','',
    'Exact reuse does not reduce Jacobian evaluations. On changing nonlinear Jacobians it may add comparisons, copies and storage without any hit. No default kernel was changed. All old/new admitted observations are bit-identical, so this is implementation-cost comparison at identical observed accuracy, not evidence that the solver meets a continuous-time tolerance theorem.',
    'The declared bandwidth2 specialization falls back to the existing Slices arithmetic for other bandwidths. Exact cache matching includes the rounded shift and all stored Jacobian entry bits; signed-zero changes conservatively miss. A failed factorization clears the key. The four pre-existing nonfinite/size regression failures are tested separately from speed.',
    'No new Diffsol/DVODE binaries are timed in this node. The previously observed spatial performance gap is not claimed closed by ratios to a different host.',
    'Whole-workspace test status is in WORKSPACE_STATUS.json and must not be inferred from the scoped audit.']
(out/'REPORT.md').write_text('\n'.join(lines)+'\n')
