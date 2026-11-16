#!/usr/bin/env python3
"""Independent saved-state audit: original source, full Peer trajectories, Python DOP853.
Does not import Rust adapter, Peer coefficients or its runtime checker.
"""
import argparse,hashlib,json,math,struct
from pathlib import Path
import numpy as np
N=46080;D=4*N;KB=1.380649e-16
STATE_SHA='20b6066eade8a31ecfe46a47081a2e3a7fef7353dba4ac83204456c24ac5f628'
def sha(b):return hashlib.sha256(b).hexdigest()
def trajectory(p,expected=None):
 b=p.read_bytes()
 if expected is not None and sha(b)!=expected:raise ValueError('OUTPUT_SHA_MISMATCH:'+p.name)
 if len(b)<24 or b[:8]!=b'REIPWOUT':raise ValueError('OUTPUT_MAGIC')
 n,k=struct.unpack_from('<QQ',b,8)
 if n!=N or not 2<=k<=1000 or len(b)!=24+k*(D+1)*8:raise ValueError('OUTPUT_DIMENSION')
 a=np.frombuffer(b,dtype='<f8',offset=24).reshape(k,D+1)
 t=a[:,0].copy();y=a[:,1:].copy().reshape(k,N,4)
 if not(np.isfinite(a).all() and abs(t[0])<1e-13 and abs(t[-1]-1)<1e-13 and np.all(np.diff(t)>0)):raise ValueError('OUTPUT_TIME_FINITE')
 return t,y,sha(b)
def fractions(y):
 out=y.copy();out[...,0]=1-y[...,0];out[...,1]=(1-y[...,1])*(1-y[...,2]);out[...,2]=(1-y[...,1])*y[...,2];return out
def cmp(t,y,tr,yr):
 errs=np.zeros(4);epoch=[]
 for j,z in enumerate(t):
  i=int(np.argmin(abs(tr-z)))
  if abs(tr[i]-z)>1e-12:raise ValueError('UNMATCHED_PHYSICAL_EPOCH')
  e=np.max(abs(fractions(y[j])-fractions(yr[i])),axis=0)
  errs=np.maximum(errs,e);epoch.append(e.tolist())
 return {'max_xHII_xHeII_xHeIII_lnT':errs.tolist(),'endpoint_errors':epoch[-1],'matched_epochs':len(t)}
def physical(y,h,he,source):
 q=y[...,:3]
 if not(np.all(q>0)&np.all(q<1)&np.all(y[...,3]<=np.log(1e5))):raise ValueError('SOURCE_PHYSICAL_CONE_VIOLATED')
 tab=np.floor((y[...,3]/np.log(10)-4)*4);tab=np.where(y[...,3]<np.log(1e4),-1,tab)
 initial_tab=np.floor((source[:,6]/np.log(10)-4)*4);initial_tab=np.where(source[:,6]<np.log(1e4),-1,initial_tab)
 if not np.all(tab==initial_tab[None,:]):raise ValueError('SOURCE_CLOSURE_TABLE_EVENT_UNHANDLED')
 x=y[-1];ne=h*(1-x[:,0])+he*(1-x[:,1])*(1+x[:,2]);np_=h+he+ne;U=1.5*KB*np_*np.exp(x[:,3])
 ini=source[:,3:7];ne0=h*(1-ini[:,0])+he*(1-ini[:,1])*(1+ini[:,2]);U0=1.5*KB*(h+he+ne0)*np.exp(ini[:,3])
 return {'positive_cone_all_stored_states':True,'no_v_table_cell_crossings_at_stored_epochs':True,'global_electrons_final':math.fsum(ne),'global_gas_U_final_erg':math.fsum(U),'delta_Ne_electrons':math.fsum(ne-ne0),'delta_U_erg':math.fsum(U-U0),'minimum_fraction_cone_margin':float(np.min(np.minimum(q,1-q))),'nuclear_conservation':'exact_by_reduced_coordinate_parametrization_not_independent_14_ledgers'}
def validate_records(records):
 # Validate the actual preregistered four-arm census, including preserved
 # failed attempts; a missing case is not a completed negative experiment.
 expected={(4,1),(8,1),(16,1),(8,6)}
 seen=[]
 for rec in records['runs']:
  if rec.get('status')=='FAILED_NO_PROMOTION':
   import re
   match=re.fullmatch(r'peer_n(\d+)_p(\d+)',rec.get('method',''))
   if match is None:raise ValueError('CASE_CENSUS_MISMATCH')
   seen.append(tuple(map(int,match.groups())))
   continue
  if rec.get('status')!='SCOPED_POINT_IVP_EXECUTED':raise ValueError('CASE_STATUS_INVALID')
  n,p=rec['steps'],rec['threads'];seen.append((n,p))
  residual=rec['max_original_W_relative_residual']
  if not isinstance(residual,(float,int)) or not math.isfinite(residual) or not 0<=residual<=1e-10:
   raise ValueError('W_RESIDUAL_FAILED')
  if rec['W_factorizations']!=n-2 or rec['W_solves']!=6*(n-2):
   raise ValueError('EXECUTED_WORK_COUNT_MISMATCH')
  elapsed,starter=rec['elapsed_including_start_seconds'],rec['starter_seconds']
  if not(math.isfinite(elapsed) and math.isfinite(starter) and 0<=starter<=elapsed):
   raise ValueError('ELAPSED_OR_STARTER_COST_INVALID')
 if len(seen)!=4 or set(seen)!=expected:raise ValueError('CASE_CENSUS_MISMATCH')

def audit(source,runs,independent,out):
 raw=(source/'SOURCE_POINT.raw').read_bytes()
 if sha(raw)!=STATE_SHA:raise ValueError('SOURCE_POINT_SHA')
 s=np.frombuffer(raw,dtype='<f8',offset=24).reshape(N,8);h=s[:,0];he=s[:,1]
 refrec=json.loads((independent/'OWNER_REFERENCE.json').read_text())
 tr,yr,_=trajectory(independent/'OWNER_DOP853.raw',refrec['reference_sha256'])
 records=json.loads((runs/'RESULT.json').read_text())
 if records['official_tick161']!='NOT_ACCEPTED' or records['canonical_delta']!=0:raise ValueError('FORGED_OFFICIAL_ACCEPTANCE')
 validate_records(records)
 a,b,_=trajectory(runs/'RK4_128.raw',records['reference128_sha256']);ref128cmp=cmp(a,b,tr,yr)
 c,d,_=trajectory(runs/'RK4_64.raw',records['reference64_sha256']);ref64cmp=cmp(c,d,tr,yr)
 successful=[];failures=[];sha8={}
 for rec in records['runs']:
  if rec['status']!='SCOPED_POINT_IVP_EXECUTED':failures.append(rec);continue
  n=rec['steps'];p=rec['threads'];name=f'peer_n{n}_p{p}'
  t,y,digest=trajectory(runs/(name+'.raw'),rec['output_sha256'])
  compare=cmp(t,y,tr,yr);ph=physical(y,h,he,s)
  if n==8:sha8[p]=digest
  if rec['max_original_W_relative_residual']>1e-10:raise ValueError('W_RESIDUAL_FAILED')
  successful.append({'case':name,'steps':n,'threads':p,'raw_sha256':digest,'vs_independent_original_owner_DOP853':compare,'physical':ph,'elapsed_including_starter_seconds':rec['elapsed_including_start_seconds'],'starter_seconds':rec['starter_seconds'],'rhs_calls':rec['rhs_calls'],'W_factorizations':rec['W_factorizations'],'W_solves':rec['W_solves'],'max_W_residual':rec['max_original_W_relative_residual']})
 equal=bool(1 in sha8 and 6 in sha8 and sha8[1]==sha8[6])
 if 1 in sha8 and 6 in sha8 and not equal:raise ValueError('PARALLEL_CHANGED_STATE_BYTES')
 result={'schema':'REI_F05_CROSS_BRANCH_PEER_FULL_POINT_INDEPENDENT_AUDIT_V1','cells':N,'dimension':D,'owner_reference':refrec,'native_RK4_128_vs_owner_DOP853':ref128cmp,'native_RK4_64_vs_owner_DOP853':ref64cmp,'successful_cases':successful,'failed_cases':failures,'n8_serial_six_stage_parallel_bit_identical':equal,'native_source_interval_14_final_ledgers':'NOT_RUN','formal_continuum_global_error_bound':False,'physical_closure':'fixed source-admissible v midpoint and f=.55','official_tick161':'NOT_ACCEPTED','canonical_delta':0,'production':'HOLD'}
 out.write_text(json.dumps(result,indent=2,sort_keys=True,allow_nan=False)+'\n')
 print(json.dumps({'rk4_128_vs_owner':ref128cmp,'successful_cases':[{'case':x['case'],'errors':x['vs_independent_original_owner_DOP853'],'seconds':x['elapsed_including_starter_seconds']} for x in successful],'failures':failures,'parallel_bits_identical':equal},sort_keys=True))
 return result
if __name__=='__main__':
 p=argparse.ArgumentParser()
 for x in ('source','runs','independent','out'):p.add_argument('--'+x,type=Path,required=True)
 a=p.parse_args();audit(a.source,a.runs,a.independent,a.out)
