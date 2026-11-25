from pathlib import Path
from collections import Counter,defaultdict
import json, math, hashlib, statistics
root=Path(__file__).resolve().parent
run=root/'run'
base=Path('/home/cosmosapjw/vigilode-preserved/20261125-adversarial-rebaseline/run')
def jread(name):return json.loads((run/name).read_bytes())
def key(r):return (r['problem'],r['arm'],r['rtol'])
def ok(r):return r.get('ok',r.get('status',{}).get('kind')=='success')
def median(v):return statistics.median(v)
def finite(v):return isinstance(v,(int,float)) and not isinstance(v,bool) and math.isfinite(v)
def jr(f):
    if not f.exists():return []
    return [json.loads(l) for l in f.read_text().splitlines() if l.startswith('{')]
def brief(rows):
    return {'rows':len(rows),'success':sum(ok(r) for r in rows),'failure':sum(not ok(r) for r in rows),
            'methods':sorted({r['arm'] for r in rows}),
            'failure_reasons':dict(Counter(str(r.get('retcode',r.get('status',{}).get('message',r.get('reason',r.get('err_msg','unknown')))))[:240] for r in rows if not ok(r)))}
def unique(rows):
    assert len({key(r) for r in rows})==len(rows)
    return rows
suites={};rows_by_suite={};new_rust={}
for name,oldfile in [('canonical','cross_solver_v1_rust.json'),('spatial','spatial_battery_v3_rust.json')]:
    old=json.loads((base/oldfile).read_bytes());oldkeys={key(r) for r in old['rows']}
    expanded=jread(name+'_expanded.json')
    extra=[r for p in sorted(run.glob(name+'_tsit45_*.json')) for r in json.loads(p.read_bytes())['rows']]
    rows=unique(expanded['rows']+extra)
    new=[r for r in rows if key(r) not in oldkeys]
    repeated=[r for r in rows if key(r) in oldkeys]
    assert {key(r) for r in repeated}==oldkeys
    assert all(r['arm'].startswith('diffsol-') for r in new)
    refs={r['problem']:r['states'] for r in expanded['references']}
    oldrefs={r['problem']:r['states'] for r in old['references']}
    assert refs==oldrefs
    assert expanded['provenance']['source_identity']['status']=='VERIFIED_BUILD_SOURCE'
    assert expanded['protocol_violation_rows']==0
    for path in run.glob(name+'_tsit45_*.json'):
        d=json.loads(path.read_bytes());assert d['provenance']['source_identity']['status']=='VERIFIED_BUILD_SOURCE'
        assert d['protocol_violation_rows']==0
        for r in d['references']:assert r['states']==oldrefs[r['problem']]
    # Raw diagnostic matched rows from actual harness, not reconstructed.
    matched=[r for r in expanded['matched'] if r['arm'] in {r['arm'] for r in new}]
    suites[name+'_rust']={**brief(rows),'new':brief(new),'old_key_replay_count':len(repeated),
                        'matched_new_implicit':dict(Counter(r['status'] for r in matched)),
                        'matched_reason':dict(Counter(r['reason'] for r in matched)),
                        'disputed':{r['problem']:r['disputed'] for r in expanded['references']},
                        'reference_arrays_equal':len(refs)}
    rows_by_suite[name+'_rust']=rows;new_rust[name]=new
for name in ('canonical','full','spatial'):
    files=sorted(run.glob('julia_'+name+'*.jsonl'))
    rows=unique([r for p in files for r in jr(p)])
    for r in rows:
        if r['ok']:
            assert finite(r['wall_ms_median']) and r['wall_ms_median']>0
            samples=r.get('raw_timing_samples_ns',[])
            assert len(samples)==(3 if name=='spatial' else 5)
            assert abs(median(samples)/1e6-r['wall_ms_median'])<=1e-10
            if 'grid_points_completed' in r:assert r['grid_points_completed']==101 and finite(r['error_common'])
    suites['julia_'+name]=brief(rows);rows_by_suite['julia_'+name]=rows
df=run/'full_diffsol.json'
if df.exists():
    rows=unique(json.loads(df.read_bytes())['rows'])
    printed=jr(run/'full_diffsol.jsonl')
    assert rows==printed
else:
    rows=unique(jr(run/'full_diffsol.jsonl'))
suites['full_diffsol']=brief(rows);rows_by_suite['full_diffsol']=rows
# Success = actual completed output, not accuracy at nominal tolerance.
allrows=sum(len(r) for r in rows_by_suite.values())
okrows=sum(sum(ok(x) for x in r) for r in rows_by_suite.values())
summary={'schema':'vigilode.expanded-comparators.v1','timing_authority':'diagnostic_only','suites':suites,
         'total_fresh_rows':allrows,'fresh_success':okrows,'fresh_failure':allrows-okrows,
         'original_rows_preserved':1032,'original_key_replays':sum(suites[n+'_rust']['old_key_replay_count'] for n in ('canonical','spatial')),
         'rows_by_suite':rows_by_suite,'sha256':{str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(run.glob('*')) if p.is_file()}}
(root/'SUMMARY.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
for name,s in suites.items():print(name,s['rows'],s['success'],s['failure'], 'new',s.get('new',{}).get('rows'), 'methods',len(s['methods']))
print('total',allrows,okrows,allrows-okrows)
for name,rs in rows_by_suite.items():
 print('NOMINAL 1e-6',name)
 for problem in sorted({r['problem'] for r in rs}):
    candidates=[r for r in rs if r['problem']==problem and r['rtol']==1e-6 and ok(r) and finite(r.get('wall_ms_median'))]
    for r in sorted(candidates,key=lambda r:r['wall_ms_median'])[:3]:
        print(problem,r['arm'],r['wall_ms_median'],r.get('error_common'),r.get('constraint_residual',r.get('extra',{}).get('max_constraint_residual') if isinstance(r.get('extra'),dict) else None))
