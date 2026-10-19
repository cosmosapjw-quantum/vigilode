#!/usr/bin/env python3
"""Compare opt-in kernels against the same current-parent solver binary.

Only finite complete bit-identical state/time outputs are admitted to time ratios.
Independent references assess accuracy but do not choose the fastest tolerance.
"""
import argparse,csv,gzip,hashlib,itertools,json,math,os,platform,random,statistics,subprocess,sys,time
from collections import Counter,defaultdict
from pathlib import Path
import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.sparse import csc_matrix

ARMS=['parent-slices','patched-slices','fixed2','exact-cache']
RTOLS=[1e-4,1e-6,1e-8]
H0=[.5,1.,2.]
CASES=[f'{kind}-{n}' for kind in ['brusselator','linear-diffusion'] for n in [50,127,200]]
def dump(p,x):
    Path(p).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def grids(case):return ['endpoint','dyadic129'] if case.startswith('linear') else ['endpoint','uniform101']
def times(case,grid):
    tf=1. if case.startswith('linear') else 10.
    m={'endpoint':1,'uniform101':100,'dyadic129':128}[grid]
    return [tf*i/m for i in range(m+1)]
def state_object(out,row):
    obj={k:row.pop(k,[]) for k in ['times','states']}
    b=json.dumps(obj,separators=(',',':'),allow_nan=False).encode();h=hashlib.sha256(b).hexdigest()
    p=out/'states'/(h+'.json.gz');p.parent.mkdir(exist_ok=True)
    if not p.exists():
        with gzip.open(p,'wb',compresslevel=9) as f:f.write(b)
    row['state_sha256']=h
    return obj

def numerical_model(cells):
    n=2*cells;d=(cells+1)**2/50.
    x=np.arange(1,cells+1)/(cells+1);y0=np.empty(n);y0[::2]=1+np.sin(2*np.pi*x);y0[1::2]=3.
    def rhs(t,y):
        u=y[::2];v=y[1::2];up=np.r_[1.,u,1.];vp=np.r_[3.,v,3.]
        f=np.empty(n);f[::2]=1+u*u*v-4*u+d*(up[:-2]-2*u+up[2:]);f[1::2]=3*u-u*u*v+d*(vp[:-2]-2*v+vp[2:]);return f
    def jac(t,y):
        a=np.zeros((n,n))
        for k in range(cells):
            i=2*k;u,v=y[i:i+2];a[i,i]=2*u*v-4-2*d;a[i,i+1]=u*u;a[i+1,i]=3-2*u*v;a[i+1,i+1]=-u*u-2*d
            if k:a[i,i-2]=a[i+1,i-1]=d
            if k+1<cells:a[i,i+2]=a[i+1,i+3]=d
        return csc_matrix(a)
    return rhs,jac,y0

def references(out):
    ref={}
    for case in CASES:
        n=int(case.rsplit('-',1)[1]);linear=case.startswith('linear')
        ts=times(case,grids(case)[1]);tt=np.asarray(ts)
        if linear:
            d=(n+1)**2/50.;x=np.arange(1,n+1)/(n+1);base=np.empty(2*n);base[::2]=np.sin(np.pi*x);base[1::2]=.5*np.sin(2*np.pi*x)
            lam=np.empty(2*n);lam[::2]=-4*d*np.sin(np.pi/(2*(n+1)))**2;lam[1::2]=-4*d*np.sin(np.pi/(n+1))**2
            y=base[None,:]*np.exp(tt[:,None]*lam);rho=np.full_like(y,2e-13)
            method='discrete sine eigenmodes with conservative binary64 allowance; not interval-certified'
        else:
            f,j,y0=numerical_model(n);sols=[]
            for rt in [1e-10,1e-12]:
                r=solve_ivp(f,[0,10],y0,method='Radau',jac=j,t_eval=ts,rtol=rt,atol=rt*.01)
                if not r.success or r.y.shape!=(2*n,len(ts)):raise RuntimeError('reference failure '+case)
                sols.append(r.y.T)
            y=sols[-1];rho=2*np.abs(sols[-1]-sols[0])+2e-13
            method='independent Python equations, two tight sparse Radau solves; empirical radius, not certified'
        for g in grids(case):
            ix=[0,len(ts)-1] if g=='endpoint' else list(range(len(ts)))
            ref[case+'|'+g]={'times':times(case,g),'y':y[ix].tolist(),'rho':rho[ix].tolist(),'method':method}
        dump(out/'REFERENCES.json',ref);print('reference',case,flush=True)
    return ref

def score(row,obj,reference):
    bad={'valid':False,'E_up':None,'R_up':None}
    y=np.asarray(obj.get('states',[]));v=np.asarray(reference['y']);rho=np.asarray(reference['rho'])
    if row.get('status')!='ok' or row.get('deterministic') is not True:return bad
    if obj.get('times')!=reference['times'] or y.shape!=v.shape or y.shape!=rho.shape:return bad
    if not np.isfinite(y).all() or not np.isfinite(rho).all() or (rho<0).any():return bad
    err=float(np.max(np.sqrt(np.mean(((y-v)/(1+np.abs(v)))**2,axis=1))))
    radius=float(np.max(np.sqrt(np.mean((rho/(1+np.abs(v)))**2,axis=1))))
    return {'valid':True,'E_raw':err,'E_reference':radius,'E_up':err+radius,'R_up':(err+radius)/row['rtol']}

def main(a):
    out=a.out;out.mkdir(parents=True,exist_ok=True)
    parent=a.parent.resolve();candidate=a.candidate.resolve()
    binary={'parent-slices':parent,**{x:candidate for x in ARMS[1:]}}
    kernel={'parent-slices':'slices','patched-slices':'slices','fixed2':'fixed2','exact-cache':'exact-cache'}
    hashes={x:sha(p) for x,p in binary.items()}
    sources={'parent-slices':a.parent_source,**{x:a.candidate_source for x in ARMS[1:]}}
    manifest={'schema':'banded-improvement-study-v1','parent_source':a.parent_source,'candidate_source':a.candidate_source,'binary_sha256':hashes,
        'python':sys.version,'numpy':np.__version__,'scipy':scipy.__version__,'host':platform.platform(),'affinity':sorted(os.sched_getaffinity(0)),
        'rtols':RTOLS,'h0_scales':H0,'cases':CASES,'timing_rtol':1e-6,'timing_h0':1.,'blocks':12,'selection':'all registered case/grids with bit-identical outputs; no time-selected tolerance',
        'timing_authority':'diagnostic_shared_runner','multiplicity':36,'reference':'empirical, not certified','historical_times_used':False}
    dump(out/'MANIFEST.json',manifest)
    refs=references(out);allrows=[];cal={}
    def call(case,arm,rt,g,hs,reps=1,warm=0):
        cmd=[str(binary[arm]),case,kernel[arm],str(rt),g,str(hs),str(reps),str(warm)]
        try:
            p=subprocess.run(cmd,capture_output=True,text=True,timeout=30)
            r=json.loads(p.stdout) if p.returncode==0 else {'status':'process_failure','error':p.stderr[-4000:],'states':[],'times':[]}
            r['exit_code']=p.returncode
        except subprocess.TimeoutExpired:r={'status':'timeout','error':'30 second per-process ceiling','states':[],'times':[]}
        except (OSError,ValueError) as e:r={'status':'runtime_error','error':str(e),'states':[],'times':[]}
        r.update(case=case,arm=arm,rtol=rt,grid=g,h0_scale=hs,binary_sha256=hashes[arm],source_commit=sources[arm],command=cmd)
        obj=state_object(out,r);m=score(r,obj,refs[case+'|'+g]);return r,m
    with gzip.open(out/'accuracy.jsonl.gz','wt') as f:
        for case in CASES:
            for g,rt,hs in itertools.product(grids(case),RTOLS,H0):
                for arm in ARMS:
                    r,m=call(case,arm,rt,g,hs);key=(case,g,rt,hs,arm);cal[key]=(r,m)
                    f.write(json.dumps({'row':r,'metrics':m},allow_nan=False,separators=(',',':'))+'\n');f.flush()
                    allrows.append({'case':case,'grid':g,'rtol':rt,'h0_scale':hs,'arm':arm,'status':r['status'],'state_sha256':r['state_sha256'],**m,
                        'attempts':r.get('attempts'),'accepted':r.get('accepted'),'rejected':r.get('rejected'),
                        'factorizations':r.get('counters',{}).get('direct_factorizations'),'jacobians':r.get('counters',{}).get('jacobian_builds'),
                        'lu_reuses':r.get('exact_lu_reuses_in_successful_run'),'stored_slots':r.get('banded_work',{}).get('stored_slots')})
            print('accuracy',case,flush=True)
    parity=[]
    for case in CASES:
        for g,rt,hs in itertools.product(grids(case),RTOLS,H0):
            old,om=cal[(case,g,rt,hs,'parent-slices')]
            for arm in ARMS[1:]:
                new,nm=cal[(case,g,rt,hs,arm)]
                equal=om['valid'] and nm['valid'] and old['state_sha256']==new['state_sha256']
                equal=equal and all(old.get(k)==new.get(k) for k in ['attempts','accepted','rejected','jacobian_reuses'])
                if equal:
                    oc=old['counters'];nc=new['counters']
                    equal=all(v==nc.get(k) for k,v in oc.items() if k!='direct_factorizations')
                    if arm!='exact-cache':equal=equal and oc['direct_factorizations']==nc['direct_factorizations']
                    else:equal=equal and nc['direct_factorizations']<=oc['direct_factorizations']
                parity.append({'case':case,'grid':g,'rtol':rt,'h0_scale':hs,'candidate':arm,'equal':bool(equal)})
    dump(out/'PARITY.json',parity)
    fields=[]
    for row in allrows:
        for k in row:
            if k not in fields:fields.append(k)
    with (out/'ACCURACY.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(allrows)
    if not all(p['equal'] for p in parity):
        dump(out/'SUMMARY.json',{'status':'BLOCKED_NUMERICAL_PARITY','accuracy_rows':len(allrows),'failed_parity':sum(not p['equal'] for p in parity)})
        raise RuntimeError('numerical parity failed; no time ranking')
    ready=[]
    for case in CASES:
        for g in grids(case):ready.append({'case':case,'grid':g,'rtol':1e-6,'h0_scale':1.,'states':{arm:cal[(case,g,1e-6,1.,arm)][0]['state_sha256'] for arm in ARMS}})
    dump(out/'FROZEN_TIMING.json',ready);manifest['timing_selection_sha256']=sha(out/'FROZEN_TIMING.json');dump(out/'MANIFEST.json',manifest)
    units=[];rng=random.Random(20261020)
    with gzip.open(out/'timing.jsonl.gz','wt') as f:
        for block in range(12):
            ordercases=ready.copy();rng.shuffle(ordercases)
            order=[ARMS[(i+block)%4] for i in [0,1,3,2]]
            for cell in ordercases:
                case=cell['case'];g=cell['grid']
                for arm in order:
                    r,m=call(case,arm,1e-6,g,1.,3 if case.startswith('brusselator') else 10,1)
                    ns=r.get('elapsed_ns',[])
                    valid=m['valid'] and r['state_sha256']==cell['states'][arm] and len(ns)==r.get('repetitions') and all(isinstance(x,int) and x>0 for x in ns)
                    u={'case':case,'grid':g,'arm':arm,'block':block,'valid':bool(valid),'median_ns':statistics.median(ns) if valid else None,
                        'state_sha256':r['state_sha256'],'repetitions':r.get('repetitions'),'E_up':m['E_up'],'rtol':1e-6}
                    units.append(u);f.write(json.dumps({'block':block,'row':r,'metrics':m,'unit':u},allow_nan=False,separators=(',',':'))+'\n');f.flush()
            print('timing block',block+1,flush=True)
    dump(out/'TIMING_UNITS.json',units)
    pairs=[];brng=np.random.default_rng(20261020)
    for cell in ready:
        for arm in ARMS[1:]:
            ratios=[]
            for block in range(12):
                q={u['arm']:u for u in units if u['case']==cell['case'] and u['grid']==cell['grid'] and u['block']==block}
                if q['parent-slices']['valid'] and q[arm]['valid']:ratios.append(q['parent-slices']['median_ns']/q[arm]['median_ns'])
            p={'case':cell['case'],'grid':cell['grid'],'candidate':arm,'baseline':'parent-slices','blocks':len(ratios),'ratios':ratios,'status':'COMPLETE_DIAGNOSTIC' if len(ratios)==12 else 'HOLD_INCOMPLETE'}
            if len(ratios)==12:
                v=np.asarray(ratios);bs=np.median(v[brng.integers(0,12,(100000,12))],axis=1);alpha=.05/36
                p.update(ratio_median=float(np.median(v)),ci95_pointwise=np.quantile(bs,[.025,.975]).tolist(),ci95_family=np.quantile(bs,[alpha/2,1-alpha/2]).tolist())
            pairs.append(p)
    dump(out/'PAIRED.json',pairs)
    cost=[]
    for cell in ready:
        for arm in ARMS:
            q=[u['median_ns'] for u in units if u['case']==cell['case'] and u['grid']==cell['grid'] and u['arm']==arm and u['valid']]
            r,m=cal[(cell['case'],cell['grid'],1e-6,1.,arm)]
            cost.append({'case':cell['case'],'grid':cell['grid'],'arm':arm,'median_ms':statistics.median(q)/1e6 if len(q)==12 else None,
                'factorizations':r['counters']['direct_factorizations'],'jacobians':r['counters']['jacobian_builds'],
                'attempts':r['attempts'],'lu_reuses':r['exact_lu_reuses_in_successful_run'],'E_up':m['E_up']})
    with (out/'COST.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(cost[0]));w.writeheader();w.writerows(cost)
    summary={'status':'COMPLETED_BOUNDED','accuracy_rows':len(allrows),'parity_pairs':len(parity),'parity_pass':sum(p['equal'] for p in parity),
        'timing_processes':len(units),'valid_timing_processes':sum(u['valid'] for u in units),'timed_integrations':sum(u['repetitions'] or 0 for u in units),
        'comparisons':len(pairs),'complete_comparisons':sum(p['status']=='COMPLETE_DIAGNOSTIC' for p in pairs),
        'default_kernel_changed':False,'jacobian_calls_skipped':False,'reference_certified':False,'timing_authority':'diagnostic_shared_runner',
        'state_objects':len(list((out/'states').glob('*.gz'))),'state_compressed_bytes':sum(p.stat().st_size for p in (out/'states').glob('*.gz'))}
    dump(out/'SUMMARY.json',summary);print(json.dumps(summary),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--parent',type=Path,required=True);p.add_argument('--candidate',type=Path,required=True);p.add_argument('--parent-source',required=True);p.add_argument('--candidate-source',required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    try:main(a)
    except Exception as e:
        a.out.mkdir(exist_ok=True,parents=True);dump(a.out/'BLOCKER.json',{'type':type(e).__name__,'message':str(e)});raise
