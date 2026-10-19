#!/usr/bin/env python3
"""Independent references, accuracy-only selection, then same-host paired timing.

No old wall-clock sample is pooled with this campaign. Failures are retained.
"""
import argparse, csv, gzip, hashlib, itertools, json, math, os, platform, random, subprocess, sys, time
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.sparse import csc_matrix
from contract import assess
from models import model, exact_pivot

RTOLS=[10.0**(-k) for k in range(4,11)]
SCALES=[0.5,1.0,2.0]
GRIDS=['endpoint','uniform101']
TARGETS=[1e-4,1e-6,1e-8]
ARMS=['vigilode-fast','diffsol-bdf','netlib-dvode']

def _json_numpy_scalar(value):
    if isinstance(value,(np.integer,np.floating,np.bool_)):
        return value.item()
    raise TypeError(f"Unsupported JSON type: {type(value).__name__}")

def dump(path, value):
    Path(path).write_text(json.dumps(value,indent=2,sort_keys=True,allow_nan=False,default=_json_numpy_scalar)+'\n')
def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()
def grid(c,g):
    return [0.0,c['tf']] if g=='endpoint' else [c['tf']*i/100.0 for i in range(101)]
def call(binary,c,arm,tol,g,hs,mode='solve',warm=0,reps=1):
    cmd=[str(binary),mode,c['name'],arm,str(tol),g,str(hs),str(warm),str(reps)]
    start=time.monotonic()
    try:
        p=subprocess.run(cmd,text=True,capture_output=True,timeout=45)
        row=json.loads(p.stdout) if p.returncode==0 else {'status':'process_failure','error':p.stderr[-6000:],'states':[],'times':[]}
        row['exit_code']=p.returncode
        if p.returncode==0 and p.stderr: row['stderr']=p.stderr[-2000:]
    except subprocess.TimeoutExpired as e:
        row={'status':'timeout','error':'45 second process ceiling','states':[],'times':[],'exit_code':None}
    except (ValueError,OSError) as e:
        row={'status':'runtime_error','error':str(e),'states':[],'times':[],'exit_code':None}
    row.update(case=c['name'],arm=arm,rtol=tol,atol=tol*c['scale'],grid=g,h0_scale=hs,mode=mode,command=cmd,process_wall_seconds=time.monotonic()-start)
    return row

def references(cases,out):
    allrefs={}; parity=[]
    for c in cases:
        f,j=model(c['name'],c['dimension'])
        for k,s in enumerate(c['samples']):
            y=np.asarray(s['y']); a=np.asarray(s['jac']); b=j(0.,y)
            df=float(np.max(np.abs(f(0.,y)-np.asarray(s['f']))/(1+np.abs(s['f']))))
            dj=float(np.max(np.abs(a-b)/(1+np.abs(b))))
            v=np.sin(np.arange(len(y))+1.0)
            cs=np.imag(f(0.,y.astype(complex)+1e-24j*v))/1e-24
            jv=b@v
            dc=float(np.max(np.abs(cs-jv)/(1+np.abs(jv))))
            parity.append(dict(case=c['name'],sample=k,rhs=df,jac=dj,complex_step_jvp=dc))
            if max(df,dj,dc)>2e-11:
                dump(out/'PARITY.json',parity)
                raise RuntimeError('COMMON_MODEL_PARITY_FAILED:'+c['name'])
        ts=grid(c,'uniform101'); y0=np.asarray(c['y0']); logs=[]
        if c['name']=='pivot-linear':
            center=exact_pivot(ts); radius=np.full_like(center,8*np.finfo(float).eps)
            # At t=0 the exact specified input is the reference without radius.
            radius[0]=0.0
            logs=[{'method':'closed form','precision':'binary64 with conservative elementary rounding allowance; not a validated enclosure'}]
        else:
            jac=(lambda t,y:j(t,y)) if c['dimension']<100 else (lambda t,y:csc_matrix(j(t,y)))
            vals=[]
            for tol in [1e-10,1e-12]:
                a=solve_ivp(f,[0.,c['tf']],y0,method='Radau',jac=jac,t_eval=ts,rtol=tol,atol=tol*c['scale']*0.01)
                if not a.success or a.y.shape!=(c['dimension'],101):raise RuntimeError('REFERENCE_FAILURE:'+c['name'])
                vals.append(a.y.T)
                logs.append(dict(method='Radau',rtol=tol,atol=tol*c['scale']*.01,nfev=a.nfev,njev=a.njev,nlu=a.nlu))
            opts={} if c['dimension']<100 else {'lband':2,'uband':2}
            z=solve_ivp(f,[0.,c['tf']],y0,method='LSODA',t_eval=ts,rtol=2e-12,atol=2e-14*c['scale'],**opts)
            if not z.success or z.y.shape!=(c['dimension'],101):raise RuntimeError('REFERENCE_CROSSCHECK_FAILURE:'+c['name'])
            center=vals[-1]
            radius=2*np.maximum(np.abs(vals[-1]-vals[-2]),np.abs(center-z.y.T))
            radius=np.maximum(radius,8*np.finfo(float).eps*np.abs(center))
            radius[0]=0.0
            logs.append(dict(method='LSODA',rtol=2e-12,atol=2e-14*c['scale'],nfev=z.nfev,njev=z.njev))
        for g in GRIDS:
            ix=[0,100] if g=='endpoint' else list(range(101))
            allrefs[c['name']+'|'+g]={'times':grid(c,g),'states':center[ix].tolist(),'radius':radius[ix].tolist(),'methods':logs,'radius_kind':'empirical_not_certified'}
        print('reference:',c['name'],flush=True)
        dump(out/'references.json',allrefs)
    dump(out/'PARITY.json',parity)
    return allrefs

def metrics(row,c,refs):
    r=refs[c['name']+'|'+row['grid']]
    return assess(row,r['states'],r['radius'],r['times'],c['scale'])
def scalar_record(row,m):
    keys=['case','arm','rtol','atol','grid','h0_scale','status','error','exit_code','deterministic','source_commit']
    q={k:row.get(k) for k in keys};q.update(m)
    q['counters_json']=json.dumps(row.get('counters'),sort_keys=True)
    return q

def campaign(binary,out):
    start=time.monotonic()
    listing=json.loads(subprocess.check_output([str(binary),'models'],text=True))
    dump(out/'models.json',listing);cases=listing['cases']
    manifest={'schema':'strict-three-solver-v1','binary_sha256':sha(binary),'source_commit':listing['source_commit'],
        'python':sys.version,'numpy':np.__version__,'scipy':scipy.__version__,'platform':platform.platform(),
        'cpu_affinity':sorted(os.sched_getaffinity(0)),'rtols':RTOLS,'h0_scales':SCALES,'grids':GRIDS,'targets':TARGETS,
        'arms':ARMS,'timing_blocks':12,'seed':20261019,'clock_authority':'diagnostic_shared_runner',
        'counter_probe_timing_used_for_ranking':False,'old_host_samples_used':False}
    dump(out/'MANIFEST.json',manifest)
    refs=references(cases,out)
    rows=[];bycase={c['name']:c for c in cases};groups=defaultdict(list)
    with gzip.open(out/'raw_accuracy.jsonl.gz','wt') as stream:
        for c,g,arm,tol,hs in itertools.product(cases,GRIDS,ARMS,RTOLS,SCALES):
            if time.monotonic()-start>1000:
                row=dict(case=c['name'],arm=arm,rtol=tol,atol=tol*c['scale'],grid=g,h0_scale=hs,status='budget_not_run',states=[],times=[],error='campaign accuracy deadline')
            else: row=call(binary,c,arm,tol,g,hs)
            m=metrics(row,c,refs);q=scalar_record(row,m);rows.append(q);groups[(c['name'],g,arm,tol)].append(q)
            stream.write(json.dumps({'row':row,'metrics':m},allow_nan=False)+'\n');stream.flush()
            if len(rows)%63==0:print('accuracy rows',len(rows),flush=True)
    with (out/'accuracy.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    selections=[]
    for c,g,target in itertools.product(cases,GRIDS,TARGETS):
        selected={}
        for arm in ARMS:
            candidates=[]
            for tol in RTOLS:
                rs=groups[(c['name'],g,arm,tol)]
                if len(rs)==3 and all(x['admitted'] and x['E_up']<=target and x['E_reference']<=.1*target for x in rs):
                    candidates.append((tol,max(x['E_up'] for x in rs),max(x['E_reference'] for x in rs)))
            if candidates:
                tol,e,er=max(candidates)
                selected[arm]={'rtol':tol,'E_up_worst_h0':e,'E_reference':er}
        entry={'case':c['name'],'grid':g,'target':target,'arms':selected,'status':'READY' if len(selected)==len(ARMS) else 'HOLD_MISSING_ACCURACY_ARM'}
        if len(selected)==len(ARMS):
            ee=[x['E_up_worst_h0'] for x in selected.values()]
            entry['achieved_error_ratio']=max(ee)/max(min(ee),np.finfo(float).tiny)
            entry['comparison_scope']='matched_achieved_factor2' if entry['achieved_error_ratio']<=2 else 'same_target_only'
        selections.append(entry)
    dump(out/'FROZEN_SELECTION.json',selections)
    manifest['frozen_selection_sha256']=sha(out/'FROZEN_SELECTION.json')
    dump(out/'MANIFEST.json',manifest)
    timing=[];rng=random.Random(20261019)
    # One process per arm and block. Each process discards a warmup; all
    # repetitions are compared bitwise by the Rust harness, outside its timer.
    with gzip.open(out/'raw_timing.jsonl.gz','wt') as stream:
        for block in range(12):
            entries=[e for e in selections if e['status']=='READY'];rng.shuffle(entries)
            for entry in entries:
                c=bycase[entry['case']];order=list(ARMS)
                # Balanced six permutations, repeated twice across 12 blocks.
                order=list(list(itertools.permutations(ARMS))[block%6])
                for arm in order:
                    tol=entry['arms'][arm]['rtol']
                    if time.monotonic()-start>1350:
                        row=dict(case=c['name'],arm=arm,rtol=tol,atol=tol*c['scale'],grid=entry['grid'],h0_scale=1.,status='budget_not_run',states=[],times=[],error='campaign timing deadline')
                    else:row=call(binary,c,arm,tol,entry['grid'],1.0,'bench',1,3 if c['dimension']>=100 else 20)
                    m=metrics(row,c,refs)
                    valid=m['admitted'] and m['E_up']<=entry['target'] and row.get('deterministic') is True
                    times=row.get('elapsed_ns',[])
                    valid=valid and len(times)==row.get('repetitions') and all(isinstance(t,(int,float)) and t>0 for t in times)
                    t=dict(case=c['name'],grid=entry['grid'],target=entry['target'],arm=arm,rtol=tol,block=block,valid=bool(valid),status=row['status'],median_ns=float(np.median(times)) if valid else None)
                    timing.append(t);stream.write(json.dumps({'block':block,'target':entry['target'],'row':row,'metrics':m,'timing':t},allow_nan=False)+'\n');stream.flush()
            print('timing block',block+1,'/12',flush=True)
    dump(out/'timing_units.json',timing)
    paired=[];brng=np.random.default_rng(20261019)
    for entry in selections:
        if entry['status']!='READY':continue
        for baseline,candidate in [('netlib-dvode','vigilode-fast'),('diffsol-bdf','vigilode-fast'),('netlib-dvode','diffsol-bdf')]:
            group=[t for t in timing if t['case']==entry['case'] and t['grid']==entry['grid'] and t['target']==entry['target']]
            ratios=[];incomplete=False
            for block in range(12):
                pair={t['arm']:t for t in group if t['block']==block}
                if baseline not in pair or candidate not in pair or not pair[baseline]['valid'] or not pair[candidate]['valid']:
                    incomplete=True;continue
                ratios.append(pair[baseline]['median_ns']/pair[candidate]['median_ns'])
            result={k:entry[k] for k in ['case','grid','target','comparison_scope','achieved_error_ratio']}
            result.update(baseline=baseline,candidate=candidate,paired_blocks=len(ratios),ratios=ratios,status='HOLD_INCOMPLETE_BLOCKS' if incomplete else 'DIAGNOSTIC_COMPLETE')
            if not incomplete and len(ratios)==12:
                v=np.asarray(ratios);bs=np.median(v[brng.integers(0,12,size=(10000,12))],axis=1)
                result.update(ratio_median=float(np.median(v)),ci95_pointwise=np.quantile(bs,[.025,.975]).tolist(),multiplicity_adjusted=False)
            paired.append(result)
    dump(out/'paired_comparisons.json',paired)
    successful=[r for r in rows if r['admitted']]
    summary={'status':'COMPLETED_BOUNDED_CAMPAIGN','planned_accuracy_cells':len(cases)*2*3*7*3,'retained_accuracy_cells':len(rows),
        'status_counts':dict(Counter(r['status'] for r in rows)),'structurally_admitted':len(successful),
        'R_up_le_1':sum(r['R_up']<=1 for r in successful),'R_up_le_10':sum(r['R_up']<=10 for r in successful),
        'arm_accuracy':{a:{'cells':sum(r['arm']==a for r in rows),'admitted':sum(r['arm']==a for r in successful),
            'R_up_le_1':sum(r['arm']==a and r['R_up']<=1 for r in successful)} for a in ARMS},
        'selection_status':dict(Counter(e['status'] for e in selections)),'timing_processes':len(timing),
        'valid_timing_processes':sum(t['valid'] for t in timing),'complete_paired_comparisons':sum(p['status']=='DIAGNOSTIC_COMPLETE' for p in paired),
        'timing_authority':'diagnostic_shared_runner','global_error_guarantee':False,'production_promotion':False}
    dump(out/'SUMMARY.json',summary)
    text=['# 동일 호스트 VigilODE / Diffsol / Netlib DVODE 비교','',
        '이 보고서는 저장된 새 원자료에서 생성했다. 공유 GitHub runner의 시간은 diagnostic이며 생산 성능 승격 근거로 사용하지 않는다.','',
        '## 실행 및 정확도','', '```json',json.dumps(summary,ensure_ascii=False,indent=2),'```','',
        '## 정확도 목표별 paired 시간비','',
        '값은 baseline/candidate이며 1보다 크면 candidate의 측정 시간이 짧다. 실제 오차비가 2를 초과하는 경우 same-target 조건일 뿐 matched-achieved 비교가 아니다. 95% 구간은 pointwise이며 다중비교 보정되지 않았다.','',
        '|문제|격자|목표|Baseline|Candidate|시간비|95% 구간|정확도 정합성|',
        '|---|---|---:|---|---|---:|---|---|']
    for p in paired:
        if 'ratio_median' in p:text.append(f"|{p['case']}|{p['grid']}|{p['target']:.0e}|{p['baseline']}|{p['candidate']}|{p['ratio_median']:.3f}|{p['ci95_pointwise'][0]:.3f}, {p['ci95_pointwise'][1]:.3f}|{p['comparison_scope']}|")
    text += ['', '## 제한', '',
        '- Netlib 원본 Fortran DVODE와 Rust 두 solver는 동일 실행파일 및 공통 RHS/Jacobian 수식을 사용한다. Python wrapper callback 비용은 timing에 포함되지 않는다.',
        '- VigilODE fast는 요청 시각에 step을 맞추고 Diffsol/DVODE는 dense output을 사용할 수 있다. 출력 시각 계약은 같으나 내부 출력 알고리즘은 다르다.',
        '- E_up 및 R_up의 reference radius는 Radau ladder와 독립 LSODA의 경험적 차이이며 엄밀한 전방오차 인증이 아니다.',
        '- R_up>1은 관측된 격자/global tolerance miss이며 local tolerance만 제어하는 solver의 명세 위반으로 자동 분류하지 않는다.',
        '- 각 arm에서 초기 step 배율 세 가지를 모두 통과한 가장 큰 입력 tolerance를 accuracy만으로 선택했다. timing 최솟값을 골라 승자를 만들지 않았다.',
        '- Diffsol의 FaerLU dense matrix 경로와 DVODE의 Brusselator banded 경로를 명시적으로 구분한다. 이 결과는 특정 solver+backend 구성 비교이며 방법론 일반의 순위가 아니다.',
        '- 새 allocation/Callgrind 측정은 이 캠페인에 포함되지 않는다. 기존 자료의 수치를 새 호스트 결과와 섞지 않는다.']
    (out/'REPORT_KO.md').write_text('\n'.join(text)+'\n')
    print(json.dumps(summary,sort_keys=True),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--out',type=Path,required=True);args=p.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    try:campaign(args.binary.resolve(),args.out)
    except Exception as e:
        dump(args.out/'BLOCKER.json',{'status':'BLOCKED','error_type':type(e).__name__,'error':str(e)})
        raise
