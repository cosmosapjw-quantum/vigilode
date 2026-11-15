#!/usr/bin/env python3
"""Independent complete original-owner point ODE reference, DOP853.
Consumes actual H/He source and original Python RHS, not the Rust adapter.
All 46,080 cells and original global radiation coupling are retained.
"""
import argparse,hashlib,importlib.util,json,struct,sys,time
from pathlib import Path
import numpy as np
from scipy.integrate import solve_ivp
N=46080
STATE_SHA='20b6066eade8a31ecfe46a47081a2e3a7fef7353dba4ac83204456c24ac5f628'
SPEC_SHA='0608145d1ab7c61f9b596ad1e407bb93b496df6f8fc2e79d5a33dba1df1fb581'
def module(path,name):
 s=importlib.util.spec_from_file_location(name,str(path));m=importlib.util.module_from_spec(s);sys.modules[name]=m;s.loader.exec_module(m);return m
def sha(b):return hashlib.sha256(b).hexdigest()
def run(owner,source,out):
 raw=(source/'SOURCE_POINT.raw').read_bytes();specraw=(source/'SOURCE.json').read_bytes()
 if sha(raw)!=STATE_SHA or sha(specraw)!=SPEC_SHA:raise ValueError('ORIGINAL_POINT_SOURCE_SHA')
 spec=json.loads(specraw);a=np.frombuffer(raw,dtype='<f8',offset=24).reshape(N,8)
 edge=module(owner/'research/rei_igm_original26_temperature_edges_20261101/edge_probe.py','independent_peer_edge')
 worker=module(owner/edge.REL_STAGE/'analysis/attempt_worker.py','independent_peer_worker');worker.jax_guard.install_if_missing();k=worker._kernel()
 if sha(worker.KERNEL_PATH.read_bytes())!=spec['kernel_sha256']:raise ValueError('ORIGINAL_KERNEL_DRIFT')
 model=k.rim.ReducedIntervalModel.from_repo(owner)
 if not(np.array_equal(model.n_h,a[:,0]) and np.array_equal(model.n_he,a[:,1])):raise ValueError('ORIGINAL_TRUE_NUCLEI_MISMATCH')
 initial=a[:,3:7].copy().ravel();v=a[:,7].copy();dur=spec['duration_s'];start=spec['start_s'];calls=0
 def rhs(t,y):
  nonlocal calls
  calls+=1
  z=np.ascontiguousarray(y.reshape(N,4).T)
  return np.ascontiguousarray(model.floating_reference_rhs(coordinates=z,time_s=start+dur*t,v=v,f=spec['f_closure']).T*dur).ravel()
 now=time.perf_counter();epochs=np.linspace(0,1,17)
 sol=solve_ivp(rhs,(0.,1.),initial,method='DOP853',t_eval=epochs,rtol=2e-11,atol=1e-13,max_step=1/8)
 elapsed=time.perf_counter()-now
 if not sol.success or sol.y.shape!=(4*N,17) or not np.isfinite(sol.y).all():raise ArithmeticError('ORIGINAL_OWNER_DOP853_FAILED '+sol.message)
 data=bytearray(b'REIPWOUT'+struct.pack('<QQ',N,17))
 for j,t in enumerate(sol.t):
  data.extend(struct.pack('<d',t));data.extend(np.asarray(sol.y[:,j],dtype='<f8').tobytes())
 out.mkdir(parents=True,exist_ok=True);(out/'OWNER_DOP853.raw').write_bytes(data)
 report={'schema':'REI_F05_FULL_POINT_OWNER_DOP853_REFERENCE_V1','source_sha256':STATE_SHA,'source_forcing_sha256':SPEC_SHA,'cells':N,'dimension':4*N,'epochs':17,'rhs_calls':calls,'elapsed_seconds':elapsed,'rtol':2e-11,'atol':1e-13,'max_step_fraction_of_original_tick':1/8,'reference_sha256':sha(data),'physical_source':'ORIGINAL_UNCHANGED_PYTHON_FLOATING_REFERENCE_RHS','closure':'f=.55; fixed original source-cell admissible v midpoint','continuum_global_error_rigorous':False,'official_tick161':'NOT_ACCEPTED','production':'HOLD'}
 (out/'OWNER_REFERENCE.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n');print(json.dumps(report,sort_keys=True))
if __name__=='__main__':
 p=argparse.ArgumentParser()
 for x in ('owner','source','out'):p.add_argument('--'+x,type=Path,required=True)
 a=p.parse_args();run(a.owner.resolve(),a.source.resolve(),a.out.resolve())
