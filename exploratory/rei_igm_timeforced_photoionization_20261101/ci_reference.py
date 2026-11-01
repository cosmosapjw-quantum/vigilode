"""Hosted-replay checks of pinned physical time-velocity source from offline original files.

The three original external source ZIP SHA checks are NOT performed in hosted CI:
only the derived fixture, mathematical model and SciPy observables are replayed.
"""
import json,hashlib
from pathlib import Path
import numpy as np
from source_time_ivp import SourceIVP
ROOT=Path(__file__).resolve().parent
f=ROOT/'five_clock_pr342_original_source.json'
r=ROOT/'reference_trajectory_15state.json'
s=ROOT/'results/SCIENCE_RESULT.json'
fixture=json.loads(f.read_text());ref=json.loads(r.read_text());science=json.loads(s.read_text())
if not (fixture['source_pr332_artifact_sha256'] == '83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9'): raise ValueError('ci_reference.py original source numerical contract line 15')
if not (fixture['source_pr342_artifact_sha256'] == '88f7e5bb1c6c44c0ca0c7f2ef2c81ac70098cc7ed773096868b1e7b4db25c035'): raise ValueError('ci_reference.py original source numerical contract line 16')
if not (fixture['N'] == science['source_cell_count'] == 46080): raise ValueError('ci_reference.py original source numerical contract line 17')
if not (science['canonical_delta'] == 0 and science['PRODUCTION'] == 'HOLD'): raise ValueError('ci_reference.py original source numerical contract line 18')
if not (ref['predeclared_native_state_budget'] == 5e-10): raise ValueError('ci_reference.py original source numerical contract line 19')
result={}
for frozen in [False,True]:
  model=SourceIVP(frozen=frozen);a=model.solve(rtol=1e-12,atol=1e-14);b=model.solve(method='DOP853',rtol=1e-12,atol=1e-14)
  stored=np.asarray(ref['frozen_states'] if frozen else ref['forced_states'],float)
  err=float(np.max(np.abs(a.y.T-stored)));alternate=float(np.max(np.abs(a.y-b.y)))
  if not (err < 3e-12 and alternate < 3e-12): raise ValueError('ci_reference.py original source numerical contract line 25')
  if not (np.max(abs(model.invariants(a.y.T))) < 8e-13): raise ValueError('ci_reference.py original source numerical contract line 26')
  if not (all(a.y[:3, -1] > 0)): raise ValueError('ci_reference.py original source numerical contract line 27')
  result['frozen' if frozen else 'timeforced']={'max_ref_replay':err,'Radau_vs_DOP853':alternate,'final_binding_eV_per_initial_HI':float(a.y[14,-1])}
print(json.dumps({'schema':'VIGILODE_REI_FIVE_CLOCK_HOSTED_REPLAY_V1','identity':'PINNED_ORIGINAL_SOURCE_FIXTURE_CONSUMER_ONLY','result':result,'canonical_delta':0,'status':'PASS'},sort_keys=True))
