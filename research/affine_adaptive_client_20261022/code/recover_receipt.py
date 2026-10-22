#!/usr/bin/env python3
"""Recover saved numerical evidence without any cosmological integration.
Replays only the authorized manufactured invalid-metric transaction check.
"""
from pathlib import Path
import sys, json, ast, hashlib, time
import numpy as np
from lifted_bdf import CNorm, LiftedBDF, NormFailure
from affine_system import LIFT, KEEP
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'vendor/bass_compact/code'))
from compact_system import CompactSystem
from compact_run import native_scalar_metadata
from inherited_gates import evaluate
start=time.perf_counter();cpu=time.process_time()
r=ROOT/'evidence/pair_002';a=np.load(r/'trajectory.npz')
CONFIG=dict(Sigma_F=.6,matter_A=1.5561956189864556,radiation_A=-.4275708327913344,backend='prepared',opacity=True)
RTOL=2e-8;ATOL=np.r_[np.full(23,2e-11),np.full(9,2e-10),2e-11]
# Load only the named fixture function's AST, never run driver module-level code.
tree=ast.parse((ROOT/'code/run_pair.py').read_text());fn=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='injected_transaction')
exec(compile(ast.Module(body=[fn],type_ignores=[]),'<isolated injected_transaction>','exec'),globals())
norm=CNorm(ROOT/'code/liblifted_error_norm.so');test=injected_transaction(norm)
(ROOT/'evidence/TRANSACTION_TEST.json').write_text(json.dumps(native_scalar_metadata(test),indent=2)+'\n')
assert test['status']=='PASS'
metadata=dict(true=np.bool_(True),false=np.bool_(False),count=np.int64(2))
roundtrip=json.loads(json.dumps(native_scalar_metadata(metadata),allow_nan=False))
assert roundtrip==dict(true=True,false=False,count=2)
(ROOT/'evidence/SERIALIZER_TEST.json').write_text(json.dumps(dict(status='PASS',actual_numpy_boolean_roundtrip=roundtrip),indent=2)+'\n')
full=CompactSystem(CONFIG);db=evaluate(full,a['x'],a['full33'].T,CONFIG);ds=evaluate(full,a['x'],a['affine_lifted33'].T,CONFIG)
err=a['affine_lifted33']-a['full33'];scale=ATOL[None,:]+RTOL*abs(a['full33']);wrms=np.sqrt(np.sum((err/scale)**2,axis=1)/33)
assert np.array_equal(wrms,a['full33_wrms_difference'])
missing=dict(status='UNKNOWN',reason='unflushed in-memory counters lost when final NumPy scalar JSON serialization failed; no repeated integration authorized')
passed=max(wrms)<=1 and all(db['gates'].values()) and all(ds['gates'].values())
receipt=dict(status='PASS_SCOPED' if passed else 'FAIL',evidence_recovery='postprocess original raw trajectory only; no repeated cosmological IVP',config=CONFIG,rtol=RTOL,atol33=ATOL.tolist(),max_step=.025,x_end=.01,samples=9,max_sampled_wrms=float(max(wrms)),sampled_wrms=wrms.tolist(),max_full33_abs_difference=float(np.max(abs(err))),full33_diagnostics=db,affine30_diagnostics=ds,full33_counts=missing,affine30_counts=missing,common_first_step=missing,original_pair_process_wall_seconds=5.4222388839989435,wall_scope='whole original process includes manufactured check, pair, diagnostics and failed serialization; not isolated trajectory timing',completed_cosmological_ivps=2,completed_matched_pairs=1,full_interval_or_cohort_integrations=0,transactional_invalid_norm=test,serializer_test='PASS',max_species_scaled_difference=float(np.max(abs(a['affine_lifted62'][:,40:61]-a['full62'][:,40:61]))),final_signed_difference=(np.array(ds['final_signed_observables'])-db['final_signed_observables']).tolist(),final_tau_difference=ds['final_tau_T']-db['final_tau_T'],trajectory_sha256=hashlib.sha256((r/'trajectory.npz').read_bytes()).hexdigest(),postprocess_wall_seconds=time.perf_counter()-start,postprocess_cpu_seconds=time.process_time()-cpu,physical_scope='Physical HOLD; conditional prescribed T/UV, test-fluid baryons, single-photoelectron yield; no speed claim')
(r/'RESULT.json').write_text(json.dumps(native_scalar_metadata(receipt),indent=2,allow_nan=False)+'\n')
print(json.dumps(native_scalar_metadata(dict(status=receipt['status'],max_sampled_wrms=receipt['max_sampled_wrms'],full33_gates=db['gates'],affine30_gates=ds['gates'],transaction=test)),indent=2))
