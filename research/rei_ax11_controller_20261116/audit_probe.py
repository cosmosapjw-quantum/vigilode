#!/usr/bin/env python3
"""Independently recover original AXBI error components from saved states.
No Rust diagnostic imports, synthetic state substitution or new IVP launch.
"""
import argparse,csv,hashlib,json,math,struct
from pathlib import Path
EV_ERG=1.602176634e-12
LABELS=['HII','HeII','HeIII','thermal_w','photon_number_L1','photon_energy_L1']

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def bits(x):return struct.pack('<d',x)
def require(ok,msg):
 if not ok:raise ValueError(msg)
def sequential_sum(values):
 out=0.0
 for x in values:out+=x
 return out

def state(path):
 lines=path.read_text().splitlines()
 require(len(lines)==6 and lines[0]=='AXBI_CHECKPOINT_V2','CHECKPOINT_SCHEMA')
 v=list(map(float,lines[1].split()));count=list(map(int,lines[2].split()))
 led=list(map(float,lines[3].split()));photons=list(map(float,lines[4].split()));initial=list(map(float,lines[5].split()))
 require(len(v)==11 and len(count)==6 and len(led)==24 and len(initial)==1,'CHECKPOINT_SHAPE')
 require(all(math.isfinite(x) for x in v+led+photons+initial),'NONFINITE_CHECKPOINT')
 require(all(x>=0 for x in photons),'NEGATIVE_PHOTONS')
 require(0<=v[3]<=1 and 0<=v[4] and 0<=v[5] and v[4]+v[5]<=1 and v[6]>0,'GAS_CONE')
 return {'v':v,'counts':count,'ledgers':led,'photons':photons,'initial':initial}

def components(full,half,E,reducer):
 F,H=full['v'],half['v'];P,Q=full['photons'],half['photons']
 require(len(P)==len(Q) and len(Q)%len(E)==0,'PHOTON_GRID_MISMATCH')
 w0,w1=F[6]/EV_ERG,H[6]/EV_ERG
 nums=[abs(F[i]-H[i]) for i in (3,4,5)]+[abs(w0-w1)]
 n=reducer(Q);u=reducer(x*E[i%len(E)] for i,x in enumerate(Q))
 dn=reducer(abs(p-q) for p,q in zip(P,Q));du=reducer(abs(p-q)*E[i%len(E)] for i,(p,q) in enumerate(zip(P,Q)))
 denom=[max(abs(H[i]),1e-5) for i in (3,4,5)]+[max(abs(w1),1e-6),max(n,1e-8),max(u,1e-8)]
 nums.extend([dn,du]);return [x/y for x,y in zip(nums,denom)],nums,denom,n,u

def physical_equal(a,b):
 xs=[a['v'][0],*a['v'][3:7],*a['ledgers'],*a['photons']]
 ys=[b['v'][0],*b['v'][3:7],*b['ledgers'],*b['photons']]
 return len(xs)==len(ys) and all(bits(x)==bits(y) for x,y in zip(xs,ys))

def audit(root,allocation_path,out):
 alloc=json.loads(allocation_path.read_text())
 require(alloc['central_review_pin']=='56dc609ea9b801c88d3364aed3eee140b3a20d46','CENTRAL_PIN_CHANGED')
 for case in ['ZERO','SHEAR_PLUS']:
  for name,key in [('config.cfg','config_sha256'),('checkpoint.txt','checkpoint_sha256')]:
   require(sha(root/'inputs'/case/name)==alloc[key][case],'SOURCE_INPUT_SHA_CHANGED')
 manifest=json.loads((root/'NATIVE_LAUNCH_MANIFEST.json').read_text())
 for name,expected in manifest['files'].items():
  p=Path(name) if Path(name).is_absolute() else root/name
  require(sha(p)==expected,'LAUNCH_SOURCE_OR_BINARY_CHANGED:'+name)
 receipt=json.loads((root/'receipts/native.json').read_text())
 require(receipt['exit_code']==0 and receipt['status']=='EXIT0','NATIVE_NOT_SUCCESSFUL')
 require(receipt['wall_s']<=30 and receipt['cpu_s']<=30,'NATIVE_ALLOCATION_EXCEEDED')
 with (root/'diagnostic/ERROR_COMPONENTS.csv').open() as file:
  rows=list(csv.DictReader(file))
 expected={f'{c}_{w}_{j}' for c in ['ZERO','SHEAR_PLUS'] for w in ['initial','plateau'] for j in range(3)}
 require(len(rows)==12 and {r['id'] for r in rows}==expected,'PROBE_CENSUS')
 results=[];total_rhs=0;metrics_exact=0;physical_exact=0;decisions_exact=0;max_fsum_rel=0.0
 for r in rows:
  directory=root/'diagnostic'/r['id'];parent=state(directory/'parent.txt');full=state(directory/'full.txt');half=state(directory/'twohalf.txt');actual=state(directory/'actual.txt')
  E=list(map(float,(root/'diagnostic'/(r['case']+'_energies.txt')).read_text().split()))
  require(E and all(math.isfinite(x) and x>0 for x in E),'INVALID_SOURCE_ENERGIES')
  h=float(r['dln_a']);target=float(r['local_target']);require(target==1e-5,'ORIGINAL_TOLERANCE_CHANGED')
  require(parent['counts'][0]==(0 if r['window']=='initial' else 199980),'ORIGINAL_PARENT_MISMATCH')
  if r['window']=='initial':require(all(x==0 for x in parent['photons']),'ORIGINAL_ZERO_PHOTON_IC_CHANGED')
  else:require((directory/'parent.txt').read_bytes()==(root/'inputs'/r['case']/'checkpoint.txt').read_bytes(),'ORIGINAL_PLATEAU_STATE_NOT_EXACT')
  expected_steps=alloc['specimens']['initial' if r['window']=='initial' else 'plateau']['dln_a']
  require(bits(h)==bits(expected_steps[int(r['id'][-1])]),'PROBE_STEP_CHANGED')
  es,nums,dens,n,u=components(full,half,E,sequential_sum)
  ef,_,_,_,_=components(full,half,E,math.fsum)
  for label,x,y in zip(LABELS,es,ef):
   observed=float(r[label]);require(math.isfinite(observed) and bits(observed)==bits(x),'COMPONENT_BITS_MISMATCH:'+label)
   max_fsum_rel=max(max_fsum_rel,abs(x-y)/max(abs(x),1e-300))
  metric=max(es);require(bits(metric)==bits(float(r['original_metric'])),'ORIGINAL_METRIC_BITS_MISMATCH');metrics_exact+=1
  winner=LABELS[es.index(metric)];require(winner==r['winner'],'WRONG_DOMINANT_COMPONENT')
  accepted=(metric<target and metric<2e-4);require(accepted==(r['accepted']=='true'),'ADAPTIVE_DECISION_MISMATCH')
  next_h=min(h*2,0.005) if accepted and metric<target*0.25 else h if accepted else h*0.5
  require(bits(next_h)==bits(actual['v'][7])==bits(float(r['native_next_dln_a'])),'NEXT_STEP_MISMATCH')
  require(bits(actual['v'][9])==bits(metric),'ACTUAL_RECORDED_METRIC_MISMATCH');decisions_exact+=1
  before,after=parent['counts'],actual['counts']
  require(after[0]-before[0]==int(accepted) and after[1]-before[1]==3 and after[2]-before[2]==int(not accepted) and after[5]-before[5]==1,'CONTROLLER_COUNTER_MISMATCH')
  tr=[int(r[k]) for k in ['full_rhs','half1_rhs','half2_rhs']]
  require(sum(tr)==int(r['adaptive_rhs'])==after[4]-before[4],'NATIVE_RHS_COUNTER_MISMATCH');total_rhs+=2*sum(tr)
  require(physical_equal(actual,half if accepted else parent),'ACTUAL_PHYSICAL_STATE_NOT_EXACT');physical_exact+=1
  require(bits(actual['v'][10])==bits(parent['v'][10]),'PRIOR_COST_WAS_RESET')
  results.append({'id':r['id'],'case':r['case'],'window':r['window'],'parent_accepted':before[0],'a':parent['v'][0],'dln_a':h,'components':dict(zip(LABELS,es)),'raw_defect_numerators':dict(zip(LABELS,nums)),'normalizations':dict(zip(LABELS,dens)),'twohalf_photon_inventory_per_H':n,'twohalf_photon_energy_eV_per_H':u,'photon_number_floor_active':n<1e-8,'photon_energy_floor_active':u<1e-8,'winner':winner,'original_metric':metric,'accepted':accepted,'next_dln_a':next_h,'same_parent_target_predicates_only':{str(t):metric<t and metric<2e-4 for t in [1e-5,2.5e-6,6.25e-7]},'rhs_per_trial':tr,'rhs_two_evaluations_total':2*sum(tr),'native_triplet_wall_ms':sum(int(r[x]) for x in ['full_ns','half1_ns','half2_ns'])/1e6,'original_adaptive_wall_ms':int(r['adaptive_ns'])/1e6,'diagnostic_snapshot_write_ms':int(r['record_write_ns'])/1e6})
 require(total_rhs<=12000,'RHS_CAP')
 scaling=[]
 for case in ['ZERO','SHEAR_PLUS']:
  for window in ['initial','plateau']:
   q=[x for x in results if x['case']==case and x['window']==window]
   scaling.append({'case':case,'window':window,'defect_ratios_at_halving':[q[i]['original_metric']/q[i+1]['original_metric'] for i in [0,1]],'local_fixed_parent_effective_exponents':[math.log2(q[i]['original_metric']/q[i+1]['original_metric']) for i in [0,1]],'not_full_history_convergence':True})
 answer={'schema':'vigilode.ax11.independent-original-controller-diagnostic.v1','central_pin':alloc['central_review_pin'],'original_source_sha256':alloc['source_identity_sha256'],'consumer':'REI PR377 / AX11 error-component and cost ledger','specimens':results,'all_12_component_maxima_bit_equal_original_metric':metrics_exact==12,'all_12_actual_controller_decisions_exact':decisions_exact==12,'all_12_physical_outputs_exact':physical_exact==12,'max_compensated_vs_original_component_relative_difference':max_fsum_rel,'native_work':{'processes':1,'comparisons':24,'tentative_trials':72,'rhs':total_rhs,'wall_s':receipt['wall_s'],'cpu_s':receipt['cpu_s'],'max_rss_kib':receipt['max_rss_kib']},'local_fixed_parent_scaling':scaling,'complete_history_component_dominance':'NOT_MEASURED','T1_T2_full_IC_refinements':'NOT_RUN','scientific_tolerance_changes':False,'new_controller':False,'original_goal':'10/17_UNCHANGED','expansion':'28/88_UNCHANGED','AX11_complete':False,'historical_cost':'UNKNOWN_PRESERVED','canonical_delta':0,'production':'HOLD'}
 out.write_text(json.dumps(answer,indent=2,sort_keys=True,allow_nan=False)+'\n')
 print(json.dumps({'metrics_exact':metrics_exact,'physical_exact':physical_exact,'decisions_exact':decisions_exact,'rhs':total_rhs,'max_compensated_relative_difference':max_fsum_rel,'scaling':scaling},sort_keys=True))
 return answer
if __name__=='__main__':
 p=argparse.ArgumentParser()
 for name in ['root','allocation','out']:p.add_argument('--'+name,type=Path,required=True)
 a=p.parse_args();audit(a.root,a.allocation,a.out)
