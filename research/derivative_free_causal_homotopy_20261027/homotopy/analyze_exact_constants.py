#!/usr/bin/env python3
"""Check analytic constants and every physical observation in Decimal80."""
from decimal import Decimal, localcontext
from pathlib import Path
import hashlib,json

P=Path(__file__).resolve().parent
raw=(P/'EXACT_CONSTANTS_RESULTS.json').read_bytes()
data=json.loads(raw)
summary={'source_results_sha256':hashlib.sha256(raw).hexdigest(),'rows':[]}
with localcontext() as ctx:
    ctx.prec=80
    D=Decimal.from_float
    for row in data['rows']:
        assert row['completed'],row['trajectory']['failure']
        p=row['plan'];n=p['steps'];h=D(p['h']);T=D(p['horizon']);k=D(p['k']);g=D(p['g']);Y=D(p['y0']);eps=D(p['physical_tolerance'])
        assert n&(n-1)==0 and h*n==T
        phi=k*Y+g*Y**3;L=k+3*g*Y**2;C=L**2*phi/8
        q=h*(k+3*g*(Y+eps)**2)/2
        true_trunc=n*C*h**3/(1+h*k/2)
        assert D(p['truncation_constant'])>=C
        assert D(p['contraction_bound'])>=q and D(p['contraction_bound'])<1
        assert D(p['total_truncation_bound'])>=true_trunc
        assert D(p['total_truncation_bound'])<eps
        assert D(p['stage_residual_target'])<=(1+h*k/2)*D(p['algebraic_step_budget'])
        assert n*D(p['algebraic_step_budget'])<=eps-D(p['total_truncation_bound'])
        if n>1:
            hp=2*h;np=n//2
            assert np*C*hp**3/(1+hp*k/2)>=eps or hp*(k+3*g*(Y+eps)**2)/2>=1
        max_error=Decimal(0);max_ratio=Decimal(0)
        for point in row['trajectory']['points']:
            t=D(point['time']);exact=Y*(-k*t).exp()/(1+(g/k)*Y**2*(1-(-2*k*t).exp())).sqrt()
            error=abs(D(point['state'])-exact)
            bound=D(point['physical_error_bound'])
            assert error<=bound<=eps
            assert D(point['algebraic_step_error_bound'])<=D(p['algebraic_step_budget'])
            max_error=max(max_error,error)
            if bound: max_ratio=max(max_ratio,error/bound)
        traj=row['trajectory'];total=traj['map_evaluations']+traj['interval_polynomial_evaluations']
        assert total==row['total_polynomial_evaluations']<=p['max_polynomial_evaluations']
        summary['rows'].append({'physical_tolerance':row['physical_tolerance'],'mode':row['mode'],'warm':row['warm_previous_stage'],'steps':n,'stage_residual_target':p['stage_residual_target'],'map_evaluations':traj['map_evaluations'],'interval_evaluations':traj['interval_polynomial_evaluations'],'total_polynomial_evaluations':total,'polish_evaluations':traj['polish_evaluations'],'max_error_decimal80':str(max_error),'max_error_to_outward_bound_decimal80':str(max_ratio),'final_outward_physical_bound':traj['points'][-1]['physical_error_bound']})
summary.update(status='pass_scoped',successful_rows=len(summary['rows']),interpretation='Decimal80 independently checks formula/observation consistency; the native directed arithmetic produces the outward bound')
(P/'EXACT_CONSTANTS_SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({'status':summary['status'],'successful_rows':len(summary['rows']),'distinct_grids':sorted(set(x['steps'] for x in summary['rows']))}))
