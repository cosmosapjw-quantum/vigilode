"""Scientific comparison of *actually executed* VigilODE RODAS5P with source-frozen Radau."""
from __future__ import annotations
from pathlib import Path
import json,sys,math
import numpy as np
from scipy.integrate import solve_ivp
from rei_moments import MomentSource

def run(native:Path):
    j=json.loads(native.read_text())
    assert j['success'] and j['complete'] and j['source_scoped_comparison_pass']
    assert j['production']=='HOLD' and j['canonical_rei_delta']==0
    assert 'matrix-free' in j['method'] and 'RODAS5P' in j['method']
    model=MomentSource()
    t=np.linspace(0.,1.,9)
    a=solve_ivp(model.rhs_aug,(0,1),np.r_[np.ones(3),np.zeros(3)],method='Radau',jac=model.jac_aug,rtol=3e-12,atol=2e-14,t_eval=t)
    b=solve_ivp(model.rhs_aug,(0,1),np.r_[np.ones(3),np.zeros(3)],method='DOP853',rtol=3e-12,atol=2e-14,t_eval=t)
    assert a.success and b.success
    y=np.asarray(j['y'],float)
    assert y.shape==(9,6),(y.shape)
    diff=float(np.max(np.abs(y-a.y.T)))
    diff_dop=float(np.max(np.abs(y-b.y.T)))
    assert diff<=2e-8 and diff_dop<=2e-8,(diff,diff_dop)
    end=y[-1];identities=[end[0]+end[3]-1,end[1]+end[4]-1,end[2]-1-model.S0[1]/model.S0[2]*end[4]+end[5]]
    assert np.max(np.abs(identities))<=2e-8,identities
    assert np.all(y[:,:3]>0) and np.all(y[:,3:]>-1e-13)
    state={'schema':'VIGILODE_ORIGINAL_REI_MOMENT_RODAS_NATIVE_CROSSCHECK_V1',
      'native_rust_Rodas5p_comparison':'PASS',
      'max_native_vs_Radau':diff,'max_native_vs_DOP853':diff_dop,
      'max_conservation_residual':float(np.max(np.abs(identities))),
      'original_cell_count':46080,'original_source_artifact_sha256':model.f['source_artifact_sha256'],
      'canonical_rei_delta':0,'vigilode_dag':'7/23','rei_central':'28/88',
      'native_genuine_code_executed':True,'not_full_owner_ivp':True}
    native.with_name('NATIVE_CROSSCHECK.json').write_text(json.dumps(state,indent=2,sort_keys=True)+'\n')
    print(json.dumps(state,indent=2))
if __name__=='__main__':run(Path(sys.argv[1]))
