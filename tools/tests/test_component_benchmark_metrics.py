import csv
import math

from tools.component_benchmark.cases import build_cases
from tools.component_benchmark.metrics import diagnostics, receipt_rows, write_receipt_csv
from tools.component_benchmark.models import rhs

CASES={c['case_id']:c for c in build_cases()}


def pair(case, states=None):
    states=states or [list(case['initial_state']) for _ in case['output_times']]
    ref={'status':'OK','times':case['output_times'],'states':[list(s) for s in states],
         'reference_converged':True,'verified_enclosure':False}
    result={'status':'OK','times':case['output_times'],'states':[list(s) for s in states],
            'counters':None,'timing':{'timed_seconds':0.2},'allocations':None}
    return result,ref


def test_endpoint_and_grid_distinct_and_frozen_denominator():
    c=CASES['ode-scalar-decay'];r,ref=pair(c)
    r['states'][50][0]+=2e-8;r['states'][-1][0]+=1e-8
    d=diagnostics(c,r,ref)
    assert d['errors']['grid_max_normalized']>d['errors']['endpoint_max_normalized']
    assert math.isclose(d['errors']['endpoint_max_normalized'],abs(r['states'][-1][0]-ref['states'][-1][0])/(c['atol'][0]+c['rtol']))
    assert d['continuous_certificate_authority']=='NOT_CLAIMED'
    assert d['diagnostic_unscored'] is True


def test_missing_nonfinite_and_unconverged_reference_are_not_evaluable():
    c=CASES['ode-scalar-decay'];r,ref=pair(c)
    r['states']=r['states'][:-1]
    assert diagnostics(c,r,ref)['errors'] is None
    r,ref=pair(c);r['states'][1][0]=float('nan')
    assert diagnostics(c,r,ref)['reference_authority'].startswith('NOT_EVALUABLE')
    r,ref=pair(c);ref['reference_converged']=False
    assert diagnostics(c,r,ref)['errors'] is None


def test_rotation_phase_and_alias_guard():
    c=CASES['ode-rotation-1'];omega=c['parameters']['omega']
    states=[[math.cos(omega*t),math.sin(omega*t)] for t in c['output_times']]
    r,ref=pair(c,states)
    d=diagnostics(c,r,ref)
    assert d['phase']['alias_guard'] and d['phase']['valid_count']==len(states)
    assert abs(d['phase']['endpoint_phase_error_radians'])<1e-12
    r['states'][3]=[0.0,0.0]
    assert diagnostics(c,r,ref)['phase']['authority'].startswith('NOT_EVALUABLE')


def test_damping_overdamped_has_envelope_without_forced_phase():
    c=CASES['ode-damping-overdamped'];states=[[math.exp(-t),-math.exp(-t)] for t in c['output_times']]
    r,ref=pair(c,states)
    d=diagnostics(c,r,ref)['damping']
    assert 'phase' not in d
    assert d['amplitude_max_abs_error']==0
    assert abs(d['log_envelope_rate_error'])<1e-12


def test_kepler_robertson_spatial_gray_scott_invariants_are_descriptive():
    for cid, key in [('ode-kepler','energy'),('ode-robertson','mass'),
                    ('ode-periodic-heat','periodic_grid_mass_mean'),('ode-gray-scott-2d-4','minimum_state')]:
        c=CASES[cid];r,ref=pair(c)
        inv=diagnostics(c,r,ref)['invariants']
        assert key in inv and inv['authority']=='DIAGNOSTIC_UNSCORED'
    assert 'mass' not in diagnostics(CASES['ode-gray-scott-2d-4'],*pair(CASES['ode-gray-scott-2d-4']))['invariants']


def test_fourier_weighted_invariant_derivative_and_report():
    c=CASES['ode-fourier-path-client'];y=[0.25,0.1,0.5,-0.2]
    f=rhs(c,0.123,y)
    deriv=2*y[0]*f[0]+2*y[1]*f[1]+4*y[2]*f[2]+4*y[3]*f[3]
    assert abs(deriv)<1e-14
    r,ref=pair(c)
    assert 'two_b_squared_plus_a_squared' in diagnostics(c,r,ref)['invariants']


def test_dae_original_algebraic_rows_no_derivative_claim():
    c=CASES['dae-index1'];states=[[math.sin(t),math.sin(t)+math.cos(t)] for t in c['output_times']]
    r,ref=pair(c,states)
    d=diagnostics(c,r,ref)['dae_constraint']
    assert d['algebraic_rows']==[1]
    assert d['max_abs_residual']<1e-14
    assert d['rate_or_derivative_claim'] is None
    ci=CASES['dae-mass-invertible'];ri,refi=pair(ci)
    assert diagnostics(ci,ri,refi)['dae_constraint']['authority'].startswith('NOT_APPLICABLE')


def test_failed_receipt_retained_in_csv(tmp_path):
    c=CASES['ode-scalar-decay'];r,ref=pair(c)
    receipt={'job':{'job_id':'one','case_id':c['case_id'],'solver_id':'fake'},'result':r}
    failure={'job':{'job_id':'two','case_id':c['case_id'],'solver_id':'fake'},
             'result':{'status':'NUMERICAL_FAILURE','times':[],'states':[],'timing':None}}
    rows=receipt_rows([receipt,failure],CASES,{c['case_id']:ref})
    assert len(rows)==2 and rows[1]['status']=='NUMERICAL_FAILURE' and rows[1]['grid_max_normalized'] is None
    path=tmp_path/'report.csv'
    assert write_receipt_csv(path,[receipt,failure],CASES,{c['case_id']:ref})==2
    with path.open() as handle: data=list(csv.DictReader(handle))
    assert len(data)==2 and data[1]['status']=='NUMERICAL_FAILURE'


def test_candidate_invariants_survive_missing_reference():
    c=CASES['ode-robertson'];r,ref=pair(c)
    ref['reference_converged']=False
    d=diagnostics(c,r,ref)
    assert d['errors'] is None
    assert d['invariants']['mass']['max_abs_drift']==0

def test_work_and_receipt_resources_remain_separate():
    c=CASES['ode-scalar-decay'];r,ref=pair(c)
    r['derivative_work']={'extra_rhs_calls':3,'included_in_solver_counters':False}
    r['allocations']={'solve':0,'positive_control':2}
    r['counters']={'rhs_evaluations':8}
    d=diagnostics(c,r,ref)
    assert d['work']['counters']['rhs_evaluations']==8
    assert d['work']['extra_derivative_rhs_calls']==3
    receipt={'job':{'job_id':'a','case_id':c['case_id'],'solver_id':'fake'},
             'result':r,'attempts':[{'peak_rss_bytes':123456}]}
    row=receipt_rows([receipt],CASES,{c['case_id']:ref})[0]
    assert row['allocations']==0 and row['rss_bytes']==123456
