#!/usr/bin/env python3
"""Accuracy-only refinement and structure-matched confirmation, new host/session.

The previous run supplies reference data and tolerance hints only. Every
calibration and timing result is newly executed and content-addressed.
"""
import argparse,csv,gzip,hashlib,itertools,json,math,os,random,sys,time
from collections import Counter,defaultdict
from pathlib import Path
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'scripts'))
from campaign import call,dump,sha
from contract import assess

SCALES=[.5,1.,2.]
CAP=1e-6
PLANNED_MULTIPLICITY=42  # 3 small cases*2 grids*3 pairs + 2 banded cases*2 grids*6 pairs

class Store:
    def __init__(self,out):
        self.out=out;self.pool=out/'states';self.pool.mkdir(parents=True,exist_ok=True)
        self.count=0
    def state(self,values):
        b=json.dumps(values,separators=(',',':'),allow_nan=False).encode();h=hashlib.sha256(b).hexdigest();p=self.pool/(h+'.json.gz')
        if not p.exists():
            with gzip.open(p,'wb',compresslevel=9) as f:f.write(b)
        return h
    def record(self,stream,row,m,**extra):
        r=dict(row);h=self.state(r.pop('states',[]));r['states_sha256']=h
        obj={'row':r,'metrics':m,**extra};stream.write(json.dumps(obj,allow_nan=False,separators=(',',':'))+'\n');stream.flush();self.count+=1
        return h

def main(binary,previous,out):
    started=time.monotonic();store=Store(out)
    listing=json.loads(__import__('subprocess').check_output([str(binary),'models'],text=True));dump(out/'models.json',listing)
    refs=json.loads((previous/'references.json').read_text());dump(out/'references.json',refs)
    cases=[c for c in listing['cases'] if c['name']!='pivot-linear']
    byname={c['name']:c for c in cases}
    old=list(csv.DictReader((previous/'accuracy.csv').open()))
    cache={};calibration=[]
    manifest={'source_commit':listing['source_commit'],'binary_sha256':sha(binary),'reference_sha256':sha(previous/'references.json'),
        'prior_run':previous.name,'prior_role':'references_and_tolerance_hints_only_not_timing','new_timing_session':True,
        'target_cap':CAP,'nominal_achieved_band':[.5,1.0],'all_h0_error_ceiling':1.0,'h0_scales':SCALES,
        'timing_blocks':12,'multiplicity_prespecified':PLANNED_MULTIPLICITY,'timing_authority':'diagnostic_shared_runner',
        'state_storage':'canonical_JSON_f64_semantics_SHA256_deduplicated_gzip','cpu_affinity':sorted(os.sched_getaffinity(0))}
    dump(out/'MANIFEST.json',manifest)
    def measure(c,g,arm,tol,stream):
        key=(c['name'],g,arm,tol)
        if key in cache:return cache[key]
        r=refs[c['name']+'|'+g];ms=[];hashes={};counters={}
        for hs in SCALES:
            if time.monotonic()-started>900:
                row=dict(case=c['name'],arm=arm,rtol=tol,atol=tol*c['scale'],grid=g,h0_scale=hs,status='budget_not_run',times=[],states=[])
            else:row=call(binary,c,arm,tol,g,hs,'solve',0,1)
            m=assess(row,r['states'],r['radius'],r['times'],c['scale']);h=store.record(stream,row,m,phase='calibration')
            hashes[str(hs)]=h;counters[str(hs)]=row.get('counters');ms.append(m)
        valid=all(m['admitted'] for m in ms)
        q={'rtol':tol,'valid':valid,'E_max':max(m['E_up'] for m in ms) if valid else None,'E_nominal':ms[1]['E_up'] if valid else None,
           'E_reference':max(m['E_reference'] for m in ms) if valid else None,'states_sha256':hashes,'counters':counters}
        calibration.append({'case':c['name'],'grid':g,'arm':arm,**q});cache[key]=q
        return q
    selections=[]
    with gzip.open(out/'calibration.jsonl.gz','wt') as stream:
        for c,g in itertools.product(cases,['endpoint','uniform101']):
            arms=['vigilode-fast','diffsol-bdf','netlib-dvode']
            if c['dimension']>=100:arms.append('vigilode-banded')
            coarse={}
            for arm in arms:
                hintarm='vigilode-fast' if arm=='vigilode-banded' else arm
                hints=defaultdict(list)
                for r in old:
                    if r['case']==c['name'] and r['grid']==g and r['arm']==hintarm:
                        hints[float(r['rtol'])].append(r)
                options=[t for t,rs in hints.items() if len(rs)==3 and all(x['status']=='ok' and float(x['E_up'])<=CAP and float(x['E_reference'])<=.1*CAP for x in rs)]
                tol=max(options) if options else 1e-8
                q=measure(c,g,arm,tol,stream)
                for _ in range(8):
                    if q['valid'] and q['E_max']<=CAP:break
                    tol/=10.;q=measure(c,g,arm,tol,stream)
                if q['valid'] and q['E_max']<=CAP:coarse[arm]=q
            if len(coarse)!=len(arms):
                selections.append({'case':c['name'],'grid':g,'status':'HOLD_COARSE_FAILURE','coarse':coarse,'matched':{}});continue
            anchor=min(q['E_max'] for q in coarse.values())
            if max(q['E_reference'] for q in coarse.values())>.1*anchor:
                selections.append({'case':c['name'],'grid':g,'status':'HOLD_REFERENCE_LIMITED','anchor':anchor,'coarse':coarse,'matched':{}});continue
            matched={};unmatched={}
            for arm in arms:
                q=coarse[arm];tol=q['rtol'];lo=None;hi=None;best=None
                for _ in range(8):
                    if q['valid'] and q['E_max']<=anchor:
                        lo=tol;best=q;break
                    hi=tol;tol/=10.;q=measure(c,g,arm,tol,stream)
                if lo is not None:
                    for _ in range(14):
                        if best['E_nominal']>=.5*anchor and best['E_max']<=anchor:break
                        if hi is None:
                            proposed=min(lo*10.,1e-2)
                            if proposed==lo:break
                        else:
                            proposed=math.sqrt(lo*hi)
                            if hi/lo<1.00001:break
                        q=measure(c,g,arm,proposed,stream)
                        if q['valid'] and q['E_max']<=anchor:
                            lo=proposed;best=q
                        else:hi=proposed
                    if best['E_nominal']>=.5*anchor and best['E_max']<=anchor and best['E_reference']<=.1*anchor:
                        matched[arm]=best
                    else:unmatched[arm]={'reason':'REFINEMENT_LIMIT_OR_H0_SPREAD','best':best}
                else:unmatched[arm]={'reason':'NO_LOWER_BRACKET'}
            entry={'case':c['name'],'grid':g,'target_cap':CAP,'anchor':anchor,'coarse':coarse,'matched':matched,'unmatched':unmatched,
                'status':'MATCHED_ALL_ARMS' if len(matched)==len(arms) else 'PARTIAL_MATCHED_ARMS' if len(matched)>=2 else 'HOLD_NO_MATCHED_PAIR'}
            selections.append(entry);dump(out/'CALIBRATION_PROGRESS.json',selections)
            print('refined',c['name'],g,entry['status'],len(matched),'/',len(arms),flush=True)
    dump(out/'calibration_groups.json',calibration);dump(out/'FROZEN_MATCHED_SELECTION.json',selections)
    manifest['selection_sha256']=sha(out/'FROZEN_MATCHED_SELECTION.json');dump(out/'MANIFEST.json',manifest)
    units=[];rng=random.Random(20261020)
    with gzip.open(out/'timing.jsonl.gz','wt') as stream:
        for block in range(12):
            entries=[x for x in selections if len(x['matched'])>=2];rng.shuffle(entries)
            for e in entries:
                c=byname[e['case']];arms=list(e['matched']);n=len(arms)
                if n==4:
                    order=[arms[(i+block)%4] for i in [0,1,3,2]]
                elif n==3:order=list(list(itertools.permutations(arms))[block%6])
                else:order=arms if block%2==0 else list(reversed(arms))
                for arm in order:
                    choice=e['matched'][arm];tol=choice['rtol'];r=refs[c['name']+'|'+e['grid']]
                    if time.monotonic()-started>1200:
                        row=dict(case=c['name'],arm=arm,rtol=tol,atol=tol*c['scale'],grid=e['grid'],h0_scale=1.,status='budget_not_run',times=[],states=[])
                    else:row=call(binary,c,arm,tol,e['grid'],1.,'bench',1,3 if c['dimension']>=100 else 20)
                    m=assess(row,r['states'],r['radius'],r['times'],c['scale']);h=store.record(stream,row,m,phase='timing',block=block,anchor=e['anchor'])
                    valid=m['admitted'] and row.get('deterministic') is True and h==choice['states_sha256']['1.0']
                    valid=valid and .5*e['anchor']<=m['E_up']<=e['anchor'] and m['E_reference']<=.1*e['anchor']
                    ns=row.get('elapsed_ns',[]);valid=valid and len(ns)==row.get('repetitions') and all(isinstance(t,int) and t>0 for t in ns)
                    units.append({'case':c['name'],'grid':e['grid'],'arm':arm,'rtol':tol,'block':block,'valid':bool(valid),
                        'median_ns':float(np.median(ns)) if valid else None,'E_up':m['E_up'],'E_reference':m['E_reference'],'anchor':e['anchor'],
                        'states_sha256':h,'status':row['status'],'repetitions':row.get('repetitions'),'counters':choice['counters']['1.0']})
            print('matched timing block',block+1,flush=True)
    dump(out/'timing_units.json',units)
    comparisons=[];brng=np.random.default_rng(20261020)
    for e in selections:
        for candidate,baseline in itertools.combinations(list(e['matched']),2):
            pairs=[]
            for b in range(12):
                q={u['arm']:u for u in units if u['case']==e['case'] and u['grid']==e['grid'] and u['block']==b}
                if all(a in q and q[a]['valid'] for a in [candidate,baseline]):pairs.append(q[baseline]['median_ns']/q[candidate]['median_ns'])
            p={'case':e['case'],'grid':e['grid'],'anchor':e['anchor'],'baseline':baseline,'candidate':candidate,'paired_blocks':len(pairs),'ratios':pairs,
               'comparison_scope':'matched_nominal_error_factor2_all_h0_below_anchor','status':'COMPLETE_DIAGNOSTIC' if len(pairs)==12 else 'HOLD_INCOMPLETE'}
            if len(pairs)==12:
                v=np.asarray(pairs);bs=np.median(v[brng.integers(0,12,size=(100000,12))],axis=1)
                alpha=.05/PLANNED_MULTIPLICITY
                nplus=sum(t>1 for t in pairs);nminus=sum(t<1 for t in pairs);nn=nplus+nminus
                pv=min(1.,2*sum(math.comb(nn,i) for i in range(min(nplus,nminus)+1))/2**nn) if nn else 1.
                p.update(ratio_median=float(np.median(v)),ci95_pointwise=np.quantile(bs,[.025,.975]).tolist(),
                    ci95_bonferroni_family=np.quantile(bs,[alpha/2,1-alpha/2]).tolist(),
                    sign_test_two_sided=pv,sign_test_bonferroni=min(1.,pv*PLANNED_MULTIPLICITY),multiplicity=PLANNED_MULTIPLICITY)
            comparisons.append(p)
    dump(out/'paired_comparisons.json',comparisons)
    summary={'source_commit':listing['source_commit'],'selection_groups':len(selections),'selection_status':dict(Counter(e['status'] for e in selections)),
        'calibration_groups':len(calibration),'calibration_solves':len(calibration)*3,'timing_processes':len(units),'valid_timing_processes':sum(u['valid'] for u in units),
        'timed_integrations':sum(u.get('repetitions') or 0 for u in units),'paired_comparisons':len(comparisons),
        'complete_pairs':sum(p['status']=='COMPLETE_DIAGNOSTIC' for p in comparisons),'reference_source_run':previous.name,
        'old_timing_pooled':False,'wall_clock_authority':'diagnostic_shared_runner','raw_state_objects':len(list(store.pool.glob('*.gz'))),
        'raw_state_compressed_bytes':sum(p.stat().st_size for p in store.pool.glob('*.gz')),'production_promotion':False}
    dump(out/'SUMMARY.json',summary)
    print(json.dumps(summary,sort_keys=True),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--previous',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
    try:main(a.binary.resolve(),a.previous.resolve(),a.out.resolve())
    except Exception as e:
        dump(a.out/'BLOCKER.json',{'status':'BLOCKED','error_type':type(e).__name__,'error':str(e)});raise
