from pathlib import Path
import json
from source_time_ivp import SourceIVP
ROOT=Path(__file__).resolve().parent
forced=SourceIVP();f=forced.solve(rtol=1e-12,atol=1e-14)
frozen=SourceIVP(frozen=True);c=frozen.solve(rtol=1e-12,atol=1e-14)
assert list(f.t)==list(c.t)
ref={'schema':'VIGILODE_REI_IGM_FIVE_CLOCK_15STATE_TWO_ARM_REFERENCE_V1',
'origin':'Original REI PR332/PR342 verified frozen-state source, linear 5-site interpolation',
'samples_u':f.t.tolist(),'forced_states':f.y.T.tolist(),'frozen_states':c.y.T.tolist(),
'predeclared_native_state_budget':5e-10,'native_pass_does_not_validate_full_26_event':True}
out=ROOT/'reference_trajectory_15state.json';out.write_text(json.dumps(ref,indent=2,sort_keys=True)+'\n');print('states',len(f.t),'budget',ref['predeclared_native_state_budget'])
