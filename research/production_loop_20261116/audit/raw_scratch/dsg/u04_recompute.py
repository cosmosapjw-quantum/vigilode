import json, re
H='/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/research/production_unification_20261115/matrix_free_w/'
# parse raw QUAL line from the crossover log (not the folded RESULTS.json)
raw=None
for line in open(H+'logs/u04_crossover.log'):
    if 'QUAL {' in line:
        dd=json.loads(line[line.index('QUAL ')+5:])
        if dd['test']=='c5_crossover': raw=dd['data']
rows=[]
dense={r['n']:r for r in raw if r['arm']=='dense'}
for r in raw:
    d=dense[r['n']]; st=r['steps']
    acc=st['accepted']; att=acc+st['rejected']
    rows.append({'n':r['n'],'arm':r['arm'],'threads':r['threads'],'status':r['status'][:60],'median_s':r['median_seconds'],
                 'speed_vs_dense':(d['median_seconds']/r['median_seconds']),'accepted':acc,'rejected':st['rejected'],
                 'steps_ratio_vs_dense':att/(d['steps']['accepted']+d['steps']['rejected']),
                 'jvp_per_attempt':st['jvp_calls']/att if att else None,'lin_iters_per_stage_solve':st['linear_iterations']/(6*att) if att else None,
                 'endpoint_diff_tol_units':r['endpoint_wrms_diff_vs_dense_in_tol_units'],
                 'dense_us_per_attempt':1e6*d['median_seconds']/(d['steps']['accepted']+d['steps']['rejected']),
                 'keys':sorted(r.keys())})
out={'arms':sorted({r['arm'] for r in raw}),'n_values':sorted({r['n'] for r in raw}),'rows':rows}
# any preconditioner field?
txt=open(H+'logs/u04_crossover.log').read()
out['preconditioner_mentions_in_log']=len(re.findall('precond',txt,re.I))
src=open('/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/crates/rodas5p-integrators/src/ppsw1_linear_backend.rs').read()
out['backend_source_precond_lines']=[(i+1,l.strip()[:140]) for i,l in enumerate(src.splitlines()) if re.search('precond',l,re.I)][:12]
# dense arm scaling exponent
import math
ns=sorted(dense); 
out['dense_scaling']=[{'n0':a,'n1':b,'exponent_wall':math.log(dense[b]['median_seconds']/dense[a]['median_seconds'])/math.log(b/a),
  'exponent_per_attempt':math.log((dense[b]['median_seconds']/(dense[b]['steps']['accepted']+dense[b]['steps']['rejected']))/(dense[a]['median_seconds']/(dense[a]['steps']['accepted']+dense[a]['steps']['rejected'])))/math.log(b/a)} for a,b in zip(ns,ns[1:])]
out['dense_steps']={n:[dense[n]['steps']['accepted'],dense[n]['steps']['rejected']] for n in ns}
best={}
for r in rows:
    if r['arm']!='dense' and r['status']=='ok' and r['endpoint_diff_tol_units'] is not None and r['endpoint_diff_tol_units']<=0.1:
        if r['n'] not in best or r['speed_vs_dense']>best[r['n']]['speed_vs_dense']: best[r['n']]=r
out['best_admissible']={n:{k:best[n][k] for k in ('arm','threads','speed_vs_dense','steps_ratio_vs_dense','jvp_per_attempt','lin_iters_per_stage_solve','endpoint_diff_tol_units')} for n in sorted(best)}
out['n800']=[{k:r[k] for k in ('arm','status','median_s','accepted')} for r in rows if r['n']==800]
json.dump(out,open('/tmp/claude-1000/dsg-audit/u04_recompute.json','w'),indent=1,allow_nan=False)
print(out['arms'],out['n_values'],out['preconditioner_mentions_in_log']); print(out['backend_source_precond_lines'])
print(out['dense_scaling']); print(out['dense_steps'])
for n,b in out['best_admissible'].items(): print(n,b)
for r in rows: print({k:(round(v,4) if isinstance(v,float) else v) for k,v in r.items() if k!='keys'})
