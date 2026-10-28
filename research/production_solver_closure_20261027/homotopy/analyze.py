#!/usr/bin/env python3
"""Independent Decimal80 ODE oracle and exact rational identity checks."""
from pathlib import Path
from decimal import Decimal, localcontext
from fractions import Fraction as F
import json, statistics, hashlib

ROOT = Path(__file__).resolve().parent

def decimal_float(x):
    return Decimal.from_float(float(x))

def audit(filename):
    source=ROOT/filename
    data=json.loads(source.read_text())
    rows=[]
    with localcontext() as ctx:
        ctx.prec=80
        for row in data['rows']:
            k,g,y=map(decimal_float,[row['k'],row['g'],row['y0']])
            maximum=Decimal(0); worst_ratio=Decimal(0)
            trajectory=row['trajectory']
            assert row['completed'] and trajectory['completed']
            for point in trajectory['points']:
                t=decimal_float(point['time'])
                exact=y*(-k*t).exp()/(1+(g/k)*y*y*(1-(-2*k*t).exp())).sqrt()
                error=abs(decimal_float(point['state'])-exact)
                bound=decimal_float(point['physical_error_bound'])
                assert error<=bound,(filename,row['case'],row['arm'],point['step'],str(error),str(bound))
                assert bound<=decimal_float(row['tolerance'])
                maximum=max(maximum,error)
                if bound:worst_ratio=max(worst_ratio,error/bound)
            rows.append({'case':row['case'],'arm':row['arm'],'steps':row['plan']['steps'],
                         'map_evaluations':trajectory['map_evaluations'],
                         'interval_evaluations':trajectory['interval_polynomial_evaluations'],
                         'algebraic_predictors':trajectory.get('algebraic_predictors',0),
                         'rational_predictors':trajectory.get('rational_predictors',0),
                         'predictor_fallbacks':trajectory.get('algebraic_predictor_fallbacks',0),
                         'maximum_error_decimal80':str(maximum),
                         'max_error_over_bound':str(worst_ratio),
                         'final_outward_bound':trajectory['points'][-1]['physical_error_bound'],
                         'observations':len(trajectory['points'])})
    return {'file':filename,'sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
            'all_completed':True,'all_grid_errors_below_outward_bound':True,'rows':rows}

def timings(filename):
    data=json.loads((ROOT/filename).read_text());groups={}
    for r in data['rows']:groups.setdefault((r['case'],r['a'],r['b']),[]).append(r['a_over_b'])
    return [{'case':k[0],'a':k[1],'b':k[2],'median_a_over_b':statistics.median(v),
             'minimum':min(v),'maximum':max(v),'paired_ratios':v,
             'interpretation':'descriptive paired-block range; not a confidence interval'}
            for k,v in groups.items()]

checks=0
for j in range(151):
    u=F(100+j,100)
    s=3/(u*u+1+1/(u*u))
    beta=(u**3-u**-3)**2/27
    assert s+beta*s**3==1
    checks+=1
rational_checks=0
for j in range(151):
    beta=F(j,100)
    r=(1+2*beta)/(1+3*beta)
    residual=beta**3*(3+8*beta)/(1+3*beta)**3
    assert r+beta*r**3-1==residual>=0
    rational_checks+=1

out={'decimal_precision':80,'algebraic_identity_exact_cells':checks,
     'rational_defect_exact_cells':rational_checks,
     'pass0':audit('AUDIT_RESULTS.json'),'pass0_timing':timings('TIMING_RESULTS.json'),
     'pass1':audit('PASS1_AUDIT_RESULTS.json'),'pass1_timing':timings('PASS1_TIMING_RESULTS.json')}
(ROOT/'ANALYSIS.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'algebraic_identity_exact_cells':checks,'rational_defect_exact_cells':rational_checks,
                  'pass0_arms':len(out['pass0']['rows']),'pass1_arms':len(out['pass1']['rows']),
                  'all_physical_checks':True,'timings':out['pass1_timing']},indent=2))
