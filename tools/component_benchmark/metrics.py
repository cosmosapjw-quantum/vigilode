"""Descriptive component benchmark diagnostics. No admission thresholds live here."""
from __future__ import annotations

import csv
import math
from pathlib import Path

from .models import rhs


def _finite_number(x):
    return type(x) in (int, float) and math.isfinite(x)


def _series_error(case, result, reference):
    times=case['output_times']; actual=result.get('states'); expected=reference.get('states')
    if (result.get('status')!='OK' or reference.get('status')!='OK' or
        not reference.get('reference_converged', False) or
        result.get('times')!=times or reference.get('times')!=times or
        not isinstance(actual,list) or not isinstance(expected,list) or
        len(actual)!=len(times) or len(expected)!=len(times)):
        return None, 'NOT_EVALUABLE:missing_or_unconverged_reference_or_grid'
    n=len(case['initial_state']); atol=case['atol']; rtol=case['rtol']
    if any(not isinstance(a,list) or not isinstance(b,list) or len(a)!=n or len(b)!=n or
           any(not _finite_number(x) for x in a+b) for a,b in zip(actual,expected)):
        return None,'NOT_EVALUABLE:shape_or_nonfinite_state'
    weighted=[]; absolute=[]
    for a,b in zip(actual,expected):
        weighted.append([abs(x-y)/(atol[j]+rtol*abs(y)) for j,(x,y) in enumerate(zip(a,b))])
        absolute.append([abs(x-y) for x,y in zip(a,b)])
    return (weighted,absolute),'REFERENCE_CONVERGED_NOT_ENCLOSURE'


def _drift(values):
    first=values[0]
    return {'initial':first,'final':values[-1],'max_abs_drift':max(abs(x-first) for x in values)}


def _angle(x,y): return math.atan2(y,x)


def _unwrap(values):
    out=[values[0]]
    for x,prev in zip(values[1:],values[:-1]):
        d=(x-prev+math.pi)%(2*math.pi)-math.pi
        out.append(out[-1]+d)
    return out


def _phase_metric(case, actual, expected, velocity_scale=1.0):
    # The fixed output grid provides at least nine samples per period for the
    # frozen rotation cases. Both trajectories must still satisfy the guard.
    budgets=[math.hypot(case['atol'][0]+case['rtol']*abs(b[0]),
                        (case['atol'][1]+case['rtol']*abs(b[1]))*velocity_scale) for b in expected]
    amp=[math.hypot(a[0],a[1]*velocity_scale) for a in actual]
    refamp=[math.hypot(b[0],b[1]*velocity_scale) for b in expected]
    valid=[i for i,(a,b,f) in enumerate(zip(amp,refamp,budgets)) if min(a,b)>f]
    if len(valid)<2:
        return {'authority':'NOT_EVALUABLE:amplitude_below_original_error_budget','valid_count':len(valid),'alias_guard':False}
    cand_angles=[_angle(actual[i][0],actual[i][1]*velocity_scale) for i in valid]
    ref_angles=[_angle(expected[i][0],expected[i][1]*velocity_scale) for i in valid]
    cand_step=[abs((b-a+math.pi)%(2*math.pi)-math.pi) for a,b in zip(cand_angles,cand_angles[1:])]
    ref_step=[abs((b-a+math.pi)%(2*math.pi)-math.pi) for a,b in zip(ref_angles,ref_angles[1:])]
    contiguous=all(b==a+1 for a,b in zip(valid,valid[1:]))
    guard=contiguous and max(cand_step+ref_step)<math.pi/2
    if not guard:
        return {'authority':'NOT_EVALUABLE:phase_alias_guard','valid_count':len(valid),'alias_guard':False,
                'max_wrapped_step_radians':max(cand_step+ref_step)}
    c=_unwrap(cand_angles);r=_unwrap(ref_angles)
    diffs=[x-y for x,y in zip(c,r)]
    return {'authority':'DIAGNOSTIC_UNSCORED','valid_count':len(valid),'alias_guard':True,
            'endpoint_phase_error_radians':diffs[-1], 'max_abs_phase_error_radians':max(map(abs,diffs)),
            'max_wrapped_step_radians':max(cand_step+ref_step)}


def _damping(case, actual, expected, times):
    omega=case['parameters']['omega']
    a=[math.hypot(row[0],row[1]/omega) for row in actual]
    b=[math.hypot(row[0],row[1]/omega) for row in expected]
    floor=[math.hypot(case['atol'][0]+case['rtol']*abs(row[0]),
                      (case['atol'][1]+case['rtol']*abs(row[1]))/omega) for row in expected]
    valid=[i for i,(x,y,f) in enumerate(zip(a,b,floor)) if min(x,y)>f]
    out={'authority':'DIAGNOSTIC_UNSCORED','amplitude_max_abs_error':max(abs(x-y) for x,y in zip(a,b)),
         'amplitude_valid_count':len(valid)}
    if valid:
        out['amplitude_max_relative_error']=max(abs(a[i]-b[i])/b[i] for i in valid)
    else: out['amplitude_max_relative_error']=None
    if len(valid)>=2 and times[valid[-1]]>times[valid[0]]:
        i,j=valid[0],valid[-1]
        out['log_envelope_rate_candidate']=(math.log(a[j])-math.log(a[i]))/(times[j]-times[i])
        out['log_envelope_rate_reference']=(math.log(b[j])-math.log(b[i]))/(times[j]-times[i])
        out['log_envelope_rate_error']=out['log_envelope_rate_candidate']-out['log_envelope_rate_reference']
    else:
        out['log_envelope_rate_error']=None
        out['rate_authority']='NOT_EVALUABLE:envelope_below_original_error_budget'
    if case['parameters']['zeta']<1:
        out['phase']=_phase_metric(case,actual,expected,1/omega)
    return out


def _invariants(case,states,times):
    m=case['model']; p=case['parameters'];out={}
    if m=='kepler':
        mu=p['mu']; energy=[]; angular=[]
        for x,y,vx,vy in states:
            radius=math.hypot(x,y)
            if radius==0:
                return {'authority':'NOT_EVALUABLE:singular_kepler_state'}
            energy.append((vx*vx+vy*vy)/2-mu/radius)
            angular.append(x*vy-y*vx)
        out={'energy':_drift(energy),'angular_momentum':_drift(angular)}
    elif m in ('robertson','robertson_constraint'):
        out={'mass':_drift([sum(row) for row in states]),'minimum_state':min(min(row) for row in states)}
    elif m in ('periodic_heat','advection_diffusion'):
        out={'periodic_grid_mass_mean':_drift([sum(row)/len(row) for row in states])}
    elif m=='gray_scott_2d':
        out={'minimum_state':min(min(row) for row in states),'negative_state_count':sum(x<0 for row in states for x in row)}
    elif m=='fourier_path':
        # For da=i*q*c*conj(a)*b, db=i*q*c*a²/2 with real q,c,
        # d|a|²=-2*q*c*Im(conj(a)²*b), d|b|²=+q*c*Im(conj(a)²*b).
        out={'two_b_squared_plus_a_squared':_drift([r[0]**2+r[1]**2+2*(r[2]**2+r[3]**2) for r in states])}
    if out:out['authority']='DIAGNOSTIC_UNSCORED'
    return out


def _algebraic(case, states, times):
    matrix=case.get('mass_matrix')
    if matrix is None:return None
    rows=[i for i,row in enumerate(matrix) if all(x==0 for x in row)]
    if not rows:return {'authority':'NOT_APPLICABLE:invertible_mass','algebraic_rows':[]}
    residuals=[]
    try:
        for t,y in zip(times,states):
            f=rhs(case,t,y)
            values=[f[i] for i in rows]
            if any(not math.isfinite(v) for v in values):
                return {'authority':'NOT_EVALUABLE:nonfinite_algebraic_residual','algebraic_rows':rows}
            residuals.append(values)
    except (ArithmeticError, ValueError):
        return {'authority':'NOT_EVALUABLE:algebraic_residual_failure','algebraic_rows':rows}
    return {'authority':'DIAGNOSTIC_UNSCORED_ORIGINAL_ALGEBRAIC_ROWS',
            'algebraic_rows':rows,'max_abs_residual':max(abs(x) for row in residuals for x in row),
            'endpoint_residuals':residuals[-1], 'rate_or_derivative_claim':None}


def _valid_observed_grid(case, result):
    states=result.get('states')
    n=len(case['initial_state'])
    return (result.get('status')=='OK' and result.get('times')==case['output_times']
            and isinstance(states,list) and len(states)==len(case['output_times'])
            and all(isinstance(row,list) and len(row)==n
                    and all(_finite_number(x) for x in row) for row in states))


def diagnostics(case,result,reference):
    """Return deterministic observed metrics. Metrics do not alter gate decisions."""
    out={'case_id':case['case_id'],'result_status':result.get('status'),
         'reference_status':reference.get('status'),'diagnostic_unscored':True,
         'continuous_certificate_authority':'NOT_CLAIMED',
         'reference_authority':'NOT_EVALUABLE','errors':None,'phase':None,
         'damping':None,'invariants':None,'dae_constraint':None,
         'work':{'counters':result.get('counters'),
                 'extra_derivative_rhs_calls':(result.get('derivative_work') or {}).get('extra_rhs_calls'),
                 'unavailable_counters':None if isinstance(result.get('counters'),dict) else 'UNKNOWN'},
         'timing':result.get('timing'),'allocations':result.get('allocations'),
         'rss_bytes':result.get('peak_rss_bytes')}
    if _valid_observed_grid(case,result):
        out['invariants']=_invariants(case,result['states'],case['output_times']) or None
        out['dae_constraint']=_algebraic(case,result['states'],case['output_times'])
    err,authority=_series_error(case,result,reference)
    out['reference_authority']=authority
    if err is None:return out
    weighted,absolute=err;times=case['output_times'];states=result['states'];ref=reference['states']
    max_by_time=[max(row) for row in weighted]
    model=case['model']; fast=None
    if model in ('pr_nonlinear','pr_linear'):fast=1/abs(case['parameters']['lambda'])
    elif model=='scalar_decay':fast=1/abs(case['parameters']['rate'])
    elif model=='logistic':fast=1/abs(case['parameters']['rate'])
    elif model=='diagonal_stiff':fast=1/max(abs(x) for x in case['parameters']['rates'])
    elif model=='sabr':fast=1/abs(case['parameters']['lambda'])
    layer=[x for t,x in zip(times,max_by_time) if fast is not None and t<=10*fast]
    out['errors']={'authority':'REQUESTED_GRID_ONLY_REFERENCE_CONVERGED',
                   'normalized_denominator':'case.atol[j] + case.rtol * abs(reference_state[j])',
                   'endpoint_max_normalized':max_by_time[-1],
                   'grid_max_normalized':max(max_by_time),
                   'grid_max_abs':max(x for row in absolute for x in row),
                   'initial_layer_max_normalized':max(layer) if layer else None,
                   'initial_layer_points':len(layer), 'sample_count':len(times)}
    if model=='rotation':out['phase']=_phase_metric(case,states,ref)
    if model=='damping':out['damping']=_damping(case,states,ref,times)
    return out


def receipt_rows(receipts, cases, references):
    """Flat diagnostic rows; failed receipts remain present with null errors."""
    rows=[]
    for receipt in receipts:
        job=receipt.get('job') or {};result=receipt.get('result') or {}
        cid=job.get('case_id') or (job.get('request') or {}).get('case',{}).get('case_id')
        case=cases.get(cid);ref=references.get(cid)
        diag=diagnostics(case,result,ref) if case is not None and ref is not None else None
        errors=diag['errors'] if diag else None
        timing=result.get('timing') or {};alloc=result.get('allocations') or {}
        attempts=receipt.get('attempts') or []
        rss=attempts[-1].get('peak_rss_bytes') if attempts else None
        rows.append({'job_id':job.get('job_id'),'case_id':cid,'solver_id':job.get('solver_id'),
                     'status':result.get('status') or receipt.get('status'),
                     'endpoint_max_normalized':errors.get('endpoint_max_normalized') if errors else None,
                     'grid_max_normalized':errors.get('grid_max_normalized') if errors else None,
                     'timed_seconds':timing.get('timed_seconds'),
                     'allocations':alloc.get('solve') if isinstance(alloc,dict) else None,
                     'rss_bytes':rss,
                     'reference_authority':diag['reference_authority'] if diag else 'NOT_EVALUABLE:missing_case_or_reference'})
    return rows


def write_receipt_csv(path,receipts,cases,references):
    rows=receipt_rows(receipts,cases,references)
    fields=('job_id','case_id','solver_id','status','endpoint_max_normalized','grid_max_normalized',
            'timed_seconds','allocations','rss_bytes','reference_authority')
    with Path(path).open('w',newline='') as handle:
        writer=csv.DictWriter(handle,fieldnames=fields);writer.writeheader();writer.writerows(rows)
    return len(rows)
