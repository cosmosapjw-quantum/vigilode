"""Adversarial exact-scope readback for actual VigilODE 15-state RODAS5P.

It compares to independent Python SciPy Radau/DOP853 on the same declared
five-knot interpolated REI owner-source reduced physics, not the full owner IVP.
"""
from __future__ import annotations
import json,math,sys
from pathlib import Path
import numpy as np
from source_time_ivp import SourceIVP
ROOT=Path(__file__).resolve().parent

def verify(path):
    p=Path(path);a=json.loads(p.read_text())
    ref=json.loads((ROOT/'reference_trajectory_15state.json').read_text())
    assert a['source_pr332_artifact_sha256']=='83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9'
    assert a['source_pr342_artifact_sha256']=='88f7e5bb1c6c44c0ca0c7f2ef2c81ac70098cc7ed773096868b1e7b4db25c035'
    assert a['source_scoped_pass'] and a['production']=='HOLD' and a['canonical_REI_delta']==0
    assert len(a['cases'])==2
    reported={}
    for r in a['cases']:
        frozen=r['case']=='frozen'
        assert r['case'] in ('frozen','timevarying')
        model=SourceIVP(frozen=frozen)
        native=np.asarray(r['y'],float)
        reference=np.asarray(ref['frozen_states' if frozen else 'forced_states'],float)
        times=np.asarray(r['t'],float)
        assert native.shape==reference.shape==(len(times),15)
        np.testing.assert_allclose(times,ref['samples_u'],atol=0,rtol=0)
        err=float(np.max(abs(native-reference)))
        conservation=float(np.max(abs(model.invariants(native))))
        assert err<=ref['predeclared_native_state_budget'],err
        assert conservation<1e-9,conservation
        assert np.all(native[:,:3]>0) and np.all(native[:,3:14]>-1e-11)
        reported[r['case']]={'max_ref_state_error':err,'max_nuclei_photon_binding_residual':conservation,
                            'final_photoelectrons_particles':float(np.dot(native[-1,3:6],model.S))}
    f=a['cases'][0] if a['cases'][0]['case']=='timevarying' else a['cases'][1]
    x=a['cases'][0] if a['cases'][0]['case']=='frozen' else a['cases'][1]
    native_delta=float(f['ionization_binding_eV_per_initial_HI'])-float(x['ionization_binding_eV_per_initial_HI'])
    ref_delta=np.asarray(ref['forced_states'])[-1,14]-np.asarray(ref['frozen_states'])[-1,14]
    assert abs(native_delta-ref_delta)<1e-9
    assert 4e-8<native_delta<5e-8
    report={'schema':'VIGILODE_REI_FIVE_CLOCK_ACTUAL_NATIVE_15STATE_CROSSCHECK_V1',
       'native_Rodas5p':'PASS_SCOPED','source_only_no_26_event_owner':True,
       'cases':reported,'binding_energy_response_eV_per_initial_HI':native_delta,
       'reference_response':float(ref_delta),'REI_central_DAG':'28/88',
       'VIGILODE_original_DAG':'7/23','canonical_delta':0,'production':'HOLD'}
    (p.parent/'NATIVE_CROSSCHECK.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
    print(json.dumps(report,indent=2))
    return report
if __name__=='__main__':verify(sys.argv[1])
