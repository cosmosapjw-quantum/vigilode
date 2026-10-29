from tools.component_benchmark.cases import build_cases
from tools.component_benchmark.references import analytic_reference,uncertainty_gate,build_reference


def test_analytic_reference_and_uncertainty():
    cases=build_cases()
    for name in ('ode-scalar-decay','ode-logistic','ode-pr101','ode-rotation-100','dae-index1'):
        c=next(c for c in cases if c['case_id']==name)
        ref=analytic_reference(c)
        assert ref['status']=='OK' and ref['reference_converged']
        assert not ref['verified_enclosure']
        assert uncertainty_gate(ref,c)
    c=cases[0]; r=analytic_reference(c)
    r['uncertainty_abs'][0][0]=.051*(c['atol'][0]+c['rtol']*abs(r['states'][0][0]))
    assert not uncertainty_gate(r,c)
    gap=next(c for c in cases if c['model']=='pendulum_index3')
    assert build_reference(gap)['status']=='UNSUPPORTED_CAPABILITY'


def test_periodic_reference_accounts_for_stored_ic_projection():
    c=next(c for c in build_cases() if c['case_id']=='ode-periodic-heat')
    r=analytic_reference(c)
    assert r['initial_projection_bound']>0
    assert r['states'][0]==c['initial_state']
    changed={**c,'initial_state':list(c['initial_state'])}
    changed['initial_state'][0]+=1e-6
    r2=analytic_reference(changed)
    assert r2['initial_projection_bound']>=1e-6-1e-14
    assert r2['states'][0][0]==changed['initial_state'][0]
    assert not uncertainty_gate(r2,changed)


def test_numerical_refinement_cannot_promote_noncontracting_levels(monkeypatch):
    import numpy as np
    from types import SimpleNamespace
    import scipy.integrate
    from tools.component_benchmark.references import numerical_reference
    c=build_cases()[0]
    scale=c['atol'][0]+c['rtol']
    offsets=[0.0,0.0,.002*scale,.002*scale]
    calls=iter(offsets)
    def fake_solve(*args,**kwargs):
        offset=next(calls)
        return SimpleNamespace(success=True,message='synthetic',nfev=1,njev=1,nlu=1,
                               t=np.asarray(c['output_times']),
                               y=np.asarray([[1.0+offset]*len(c['output_times'])]))
    monkeypatch.setattr(scipy.integrate,'solve_ivp',fake_solve)
    r=numerical_reference(c)
    assert r['status']=='OK' and len(r['levels'])==4
    assert not r['reference_converged']
    assert not r['refinement']['contraction_gate']
    assert r['refinement']['uncertainty_gate']


def test_nonfinite_numerical_level_is_json_safe_and_held(monkeypatch):
    import json
    import numpy as np
    from types import SimpleNamespace
    import scipy.integrate
    from tools.component_benchmark.references import numerical_reference
    c=build_cases()[0]
    def fake_solve(*args,**kwargs):
        data=np.ones((1,len(c['output_times'])));data[0,1]=float('nan')
        return SimpleNamespace(success=True,message='synthetic',nfev=1,njev=0,nlu=0,
                               t=np.asarray(c['output_times']),y=data)
    monkeypatch.setattr(scipy.integrate,'solve_ivp',fake_solve)
    r=numerical_reference(c)
    assert r['status']=='NUMERICAL_FAILURE' and not r['reference_converged']
    assert r['levels'][0]['states'][1][0]=='nan'
    json.dumps(r,allow_nan=False)
