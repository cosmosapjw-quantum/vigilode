import math
from tools.component_benchmark.cases import build_cases
from tools.component_benchmark.scipy_adapter import solve
from tools.component_benchmark.protocol import validate_result


def test_scipy_raw_output_and_gaps():
    c=build_cases()[0]
    for sid in ('scipy-rk45','scipy-dop853','scipy-radau','scipy-bdf','scipy-lsoda','vode-adams','vode-bdf'):
        r=solve({'case':c,'solver_id':sid})
        assert r['status']=='OK', (sid,r.get('reason'))
        assert not validate_result(r,c), (sid,validate_result(r,c))
        assert abs(r['states'][-1][0]-math.exp(-10))<1e-7
    d=next(c for c in build_cases() if c['model']=='dae_index1')
    assert solve({'case':d,'solver_id':'scipy-bdf'})['status']=='UNSUPPORTED_CAPABILITY'


def test_missing_output_and_nonfinite_are_rejected():
    c=build_cases()[0]; r=solve({'case':c,'solver_id':'scipy-rk45'})
    r['states']=r['states'][:-1]
    assert 'invalid_states' in validate_result(r,c)
    r=solve({'case':c,'solver_id':'scipy-rk45'})
    r['states'][1][0]=float('nan')
    assert 'invalid_states' in validate_result(r,c)


def test_parity_probe_full_and_sampled_jacobian():
    from tools.component_benchmark.scipy_adapter import probe
    cases=build_cases()
    small=probe({'case':cases[0],'solver_id':'scipy-rk45'})
    assert small['status']=='OK' and len(small['probes'])==2
    assert small['probes'][0]['jacobian']==[[-1.0]]
    large=next(c for c in cases if c['case_id']=='stress-periodic_heat-4096')
    sampled=probe({'case':large,'solver_id':'scipy-rk45'})
    assert sampled['status']=='OK'
    assert sampled['probes'][0]['jacobian'] is None
    assert sampled['probes'][0]['jacobian_column_indices']==[0,2048,4095]
    assert len(sampled['probes'][0]['jacobian_columns'])==3
