#!/usr/bin/env python3
"""Export actual original REI F05 nuclei, selected admissible point and PPoly forcing.
No Python IVP substitute: source identity and independent RHS oracle only.
"""
import argparse,hashlib,importlib.util,json,struct,subprocess,sys
from pathlib import Path
import numpy as np
N=46080
OWNER='f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa'
TAR='a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b'
KERNEL='579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644'
STATE='cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01'
def load(path,name):
 s=importlib.util.spec_from_file_location(name,str(path));m=importlib.util.module_from_spec(s);sys.modules[name]=m;s.loader.exec_module(m);return m
def sha(b):return hashlib.sha256(b).hexdigest()
def run(repo,out):
 if subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip()!=OWNER:raise ValueError('OWNER_HEAD_DRIFT')
 edge=load(repo/'research/rei_igm_original26_temperature_edges_20261101/edge_probe.py','peer_source_edge')
 stage=repo/edge.REL_STAGE;worker=load(stage/'analysis/attempt_worker.py','peer_source_worker');worker.jax_guard.install_if_missing()
 k=worker._kernel()
 if sha(worker.KERNEL_PATH.read_bytes())!=KERNEL:raise ValueError('SEALED_KERNEL_DRIFT')
 dec,identity=edge.source_state(stage,'LOCAL_NEUTRAL_HAZARD_PRIMARY',worker.state_io)
 if identity['archive_sha256']!=TAR or identity['accepted_array_sha256']!=STATE:raise ValueError('SOURCE_PARENT_DRIFT')
 base,initial,zinit,nh,nhe=worker._context(k,'LOCAL_NEUTRAL_HAZARD_PRIMARY')
 model=k.rim.ReducedIntervalModel.from_repo(repo)
 pop=edge.element_conserving_parent_candidate(dec.population_lower,dec.population_upper,nh,nhe)
 logt=0.5*(dec.log_temperature_lower+dec.log_temperature_upper)
 coords=np.vstack([pop[:,0]/nh,pop[:,2]/nhe,pop[:,4]/(pop[:,3]+pop[:,4]),logt])
 t0=float(base.forcing.duration_seconds(0))*160/131072;t1=float(base.forcing.duration_seconds(0))*161/131072;dt=t1-t0
 v=(np.asarray(k.policy.build_v_field_from_temperature('CELL_LOWER_STRICT',np.exp(logt)))+np.asarray(k.policy.build_v_field_from_temperature('CELL_UPPER_STRICT',np.exp(logt))))/2
 tablecells=np.floor((logt/np.log(10)-4)*4).astype(int);tablecells=np.where(logt<np.log(1e4),-1,tablecells)
 # Per-cell original input: authentic nuclei, comoving volume, 4 reduced coords, closure v.
 mat=np.column_stack([nh,nhe,model.inputs.comoving_volume_cm3,*coords,v]).astype('<f8')
 if mat.shape!=(N,8) or not np.isfinite(mat).all() or not(np.all(coords[:3]>0)&np.all(coords[:3]<1)):raise ValueError('SOURCE_POINT_CONE')
 binary=b'REIPW001'+struct.pack('<QQ',N,8)+mat.tobytes()
 out.mkdir(parents=True,exist_ok=True);(out/'SOURCE_POINT.raw').write_bytes(binary)
 def pp(p):return {'knots':np.asarray(p.x).tolist(),'coefficients':np.asarray(p.c).T.tolist()}
 fm=model.forcing._models[0]
 spec={'schema':'ORIGINAL_REI_F05_46K_FULL_POINT_PPOLY_SOURCE_V1','owner_head':OWNER,'source_tar_sha256':TAR,'kernel_sha256':KERNEL,'accepted_array_sha256':STATE,'cells':N,'state_sha256':sha(binary),'start_s':t0,'stop_s':t1,'duration_s':dt,'n_h_total':float(model.n_h_total),'n_he_total':float(model.n_he_total),'nh0_cm3':float(k.rim.NH0_CM3),'YHe':float(k.rim.YHE),'Mpc_cm':float(k.rim.MPC_CM),'H0_s':float(k.rim.H0),'omega_m':float(k.rim.OMEGA_M),'omega_l':float(k.rim.OMEGA_L),'eV_erg':float(k.rim.EV_ERG),'sigma':np.asarray(model.inputs.sigma_cm2).tolist(),'support':np.asarray(model.inputs.owner_support,dtype=int).tolist(),'excess_eV':np.asarray(model.excess_eV).tolist(),'current':[pp(x) for x in fm['current']],'external':[pp(x) for x in fm['external']],'z':pp(fm['z']),'f_closure':0.55,'v_closure':'fixed source-admissible original table-cell midpoint; fail on crossing','physical_scope':'complete coupled 26event point RHS selected closure; NOT interval IVP','official_tick161':'NOT_ACCEPTED','production':'HOLD'}
 (out/'SOURCE.json').write_text(json.dumps(spec,indent=2,sort_keys=True,allow_nan=False)+'\n')
 oracles=[]
 for tau in (0.0,0.5,1.0):
  t=t0+dt*tau
  f=model.floating_reference_rhs(coordinates=coords,time_s=t,v=v,f=.55)*dt
  oracles.append(np.ascontiguousarray(f.T,dtype='<f8'))
 oracle=b'REIPWOR1'+struct.pack('<QQ',N,3)+b''.join(a.tobytes() for a in oracles)
 (out/'OWNER_RHS_ORACLE.raw').write_bytes(oracle)
 manifest={'state_sha256':sha(binary),'forcing_sha256':sha((out/'SOURCE.json').read_bytes()),'oracle_sha256':sha(oracle),'source_shape':mat.shape,'dimension':N*4,'exact_original_nuclei':True,'python_runtime':{'numpy':np.__version__},'canonical_delta':0,'source_owner_unmodified':True}
 (out/'SOURCE_MANIFEST.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
 print(json.dumps(manifest,sort_keys=True))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--owner',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args();run(a.owner.resolve(),a.out.resolve())
