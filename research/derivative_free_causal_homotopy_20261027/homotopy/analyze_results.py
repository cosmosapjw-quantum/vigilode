#!/usr/bin/env python3
"""Independent high-precision residual checks and paired work summaries."""
from decimal import Decimal, localcontext
import hashlib
import json
import math
from pathlib import Path

HERE=Path(__file__).resolve().parent
raw=(HERE/'RESULTS.json').read_bytes()
data=json.loads(raw)
summary={'results_sha256':hashlib.sha256(raw).hexdigest(),'stage_pairs':[],'failures':[], 'causal':[], 'trajectory_pairs':[], 'order':[]}
decimal_max=Decimal(0)
for cell in data['stage_cells']:
    assert cell['rhs_callbacks']==cell['attempt']['work']['map_attempted']
    assert all(cell[k]==0 for k in ['jacobian_callbacks','jvp_callbacks','ad_callbacks','matrix_factorizations','newton_updates'])
    if not cell['success']:
        summary['failures'].append({'n':cell['n'],'h':cell['h'],'mode':cell['mode'],'rhs':cell['rhs_callbacks'],'failure':cell['attempt']['failure']})
        continue
    assert cell['max_stage_error_vs_bisection']<=cell['stage_error_cap']
    assert cell['attempt']['points'][-1]['lambda']==1.0
    assert cell['attempt']['points'][-1]['residual_l2']<=1e-12
    with localcontext() as ctx:
        ctx.prec=80
        h,k,g=map(Decimal.from_float,[cell['h'],cell['decay'],cell['cubic']])
        for i,root in enumerate(cell['attempt']['root']):
            y=Decimal.from_float(1.0+0.01*(i%7)); root=Decimal.from_float(root)
            mid=y+root/2
            residual=abs(root+h*(k*mid+g*mid**3))
            decimal_max=max(decimal_max,residual)
            # Monotone F' >=1 means this real-arithmetic residual bounds root
            # error; Decimal80 is an independent high-precision diagnostic.
            assert residual<Decimal('1e-10')
keys=[]
for c in data['stage_cells']:
    key=(c['n'],c['decay'],c['cubic'],c['h'],tuple(c['lambdas']))
    if key not in keys: keys.append(key)
for key in keys:
    cells=[c for c in data['stage_cells'] if (c['n'],c['decay'],c['cubic'],c['h'],tuple(c['lambdas']))==key]
    summary['stage_pairs'].append({'n':key[0],'decay':key[1],'cubic':key[2],'h':key[3],'lambdas':list(key[4]),'arms':{c['mode']:{'success':c['success'],'rhs':c['rhs_callbacks'],'error':c['max_stage_error_vs_bisection']} for c in cells}})
for c in data['causal_cells']:
    assert c['success'] and c['max_stage_error']<1e-10
    assert c['rhs_callbacks']==4*c['attempt']['work']['map_attempted']
    summary['causal'].append({'mode':c['mode'],'rhs':c['rhs_callbacks'],'reference_rhs':c['sequential_reference_rhs'],'error':c['max_stage_error']})
for c in data['trajectories']:
    assert c['success'] and c['accepted_steps']==c['steps']
    assert c['max_stage_error_vs_bisection']<1e-10 and c['max_stage_residual']<=1e-12
    recomputed=[]
    for p in c['samples']:
        exact=math.exp(-p['t'])/math.sqrt(1+0.5*(1-math.exp(-2*p['t'])))
        recomputed.append(abs(p['y']-exact))
    assert abs(max(recomputed)-c['max_grid_error'])<2e-16
for steps in [8,16,32,64]:
    cells=[c for c in data['trajectories'] if c['steps']==steps]
    summary['trajectory_pairs'].append({'steps':steps,'arms':[{k:c[k] for k in ['mode','warm_previous_stage','rhs_callbacks','max_grid_error','endpoint_error']} for c in cells]})
for mode in ['Picard','Screened','SlowOnly']:
    for warm in [False,True]:
        rows=sorted((c for c in data['trajectories'] if c['mode']==mode and c['warm_previous_stage']==warm),key=lambda c:c['steps'])
        slopes=[math.log2(a['max_grid_error']/b['max_grid_error']) for a,b in zip(rows,rows[1:])]
        assert all(1.95<p<2.05 for p in slopes)
        summary['order'].append({'mode':mode,'warm':warm,'max_grid_error_slopes':slopes})
summary.update({'successful_stage_cells':sum(c['success'] for c in data['stage_cells']),'stage_cells':len(data['stage_cells']),'successful_causal_cells':3,'successful_trajectories':24,'decimal80_max_exact_polynomial_residual':str(decimal_max),'decimal80_interpretation':'independent high-precision numerical diagnostic, not outward interval certificate','status':'pass_scoped'})
(HERE/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ['successful_stage_cells','stage_cells','successful_causal_cells','successful_trajectories','decimal80_max_exact_polynomial_residual','status']}))
