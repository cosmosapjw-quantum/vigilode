#!/usr/bin/env python3
"""Readback audit, separate vectorized metric path (not the campaign scorer)."""
import argparse,csv,gzip,hashlib,json,math,statistics
from collections import Counter,defaultdict
from pathlib import Path
import numpy as np

def load(p):return json.loads(Path(p).read_text())
def write(p,x):Path(p).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def csvout(p,rows):
    if not rows:return
    with Path(p).open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
def main(out,previous):
    manifest=load(out/'MANIFEST.json');refs=load(out/'references.json');summary=load(out/'SUMMARY.json');listing=load(out/'models.json')
    models={c['name']:c for c in listing['cases']};checked=Counter();issues=[];state_cache={};allrows=[]
    def states(h):
        if h not in state_cache:
            with gzip.open(out/'states'/(h+'.json.gz'),'rb') as f:b=f.read()
            if hashlib.sha256(b).hexdigest()!=h:raise AssertionError('STATE_HASH_MISMATCH:'+h)
            state_cache[h]=json.loads(b)
        return state_cache[h]
    for phase in ['calibration','timing']:
        with gzip.open(out/(phase+'.jsonl.gz'),'rt') as stream:
            for line in stream:
                rec=json.loads(line);r=rec['row'];m=rec['metrics'];c=models[r['case']];ref=refs[r['case']+'|'+r['grid']]
                y=np.asarray(states(r['states_sha256']),dtype=float);v=np.asarray(ref['states']);rho=np.asarray(ref['radius'])
                checked[phase]+=1
                if r['status']=='ok':
                    assert y.shape==v.shape==rho.shape and r['times']==ref['times'] and np.isfinite(y).all() and (rho>=0).all()
                    assert initial_consistency(y[0],np.asarray(c['y0']))
                    assert r['source_commit']==manifest['source_commit']
                    d=c['scale']+np.abs(v);w=r['atol']+r['rtol']*np.abs(v)
                    calc={'E_raw':float(np.max(np.linalg.norm((y-v)/d,axis=1)/np.sqrt(y.shape[1]))),
                          'E_reference':float(np.max(np.linalg.norm(rho/d,axis=1)/np.sqrt(y.shape[1]))),
                          'R_raw':float(np.max(np.linalg.norm((y-v)/w,axis=1)/np.sqrt(y.shape[1]))),
                          'R_reference':float(np.max(np.linalg.norm(rho/w,axis=1)/np.sqrt(y.shape[1])))}
                    calc['E_up']=calc['E_raw']+calc['E_reference'];calc['R_up']=calc['R_raw']+calc['R_reference']
                    assert m['admitted'] is True
                    for k,val in calc.items():assert math.isclose(val,m[k],rel_tol=2e-12,abs_tol=1e-25),(r['case'],r['arm'],k,val,m[k])
                    assert math.isclose(calc['R_up']*r['rtol'],calc['E_up'],rel_tol=2e-12,abs_tol=1e-25)
                    if phase=='timing':
                        assert r['mode']=='bench' and r['warmups']==1 and r['deterministic'] is True
                        assert len(r['elapsed_ns'])==r['repetitions'] and r['repetition_statuses']==['ok']*r['repetitions']
                    conservation=None
                    if r['case']=='robertson':conservation=float(np.max(np.abs(y.sum(axis=1)-1.)))
                    if r['case']=='hires':conservation=float(np.max(np.abs(y[:,6]+y[:,7]-.0057)))
                    allrows.append({'phase':phase,'case':r['case'],'grid':r['grid'],'arm':r['arm'],'rtol':r['rtol'],'h0_scale':r['h0_scale'],
                                    'E_up':calc['E_up'],'R_up':calc['R_up'],'conservation_residual':conservation,'minimum_state':float(y.min())})
                    checked['finite_complete_ok']+=1
                else:
                    assert not m['admitted'] and m['E_up'] is None and m['R_up'] is None
                    checked['non_success_preserved']+=1
    units=load(out/'timing_units.json');selection=load(out/'FROZEN_MATCHED_SELECTION.json');comparisons=load(out/'paired_comparisons.json')
    assert hashlib.sha256((out/'FROZEN_MATCHED_SELECTION.json').read_bytes()).hexdigest()==manifest['selection_sha256']
    assert hashlib.sha256((previous/'references.json').read_bytes()).hexdigest()==manifest['reference_sha256']
    compact=[];cost=[];work=[]
    for e in selection:
        if len(e['matched'])<2:continue
        selected=list(e['matched'])
        err=[e['matched'][a]['E_nominal'] for a in selected]
        assert max(err)/min(err)<=2.000000000001
        for arm in selected:
            q=[u for u in units if u['case']==e['case'] and u['grid']==e['grid'] and u['arm']==arm]
            good=[u for u in q if u['valid']]
            if len(good)!=12:continue
            med=statistics.median(u['median_ns'] for u in good)
            item={'case':e['case'],'grid':e['grid'],'arm':arm,'rtol':e['matched'][arm]['rtol'],'anchor':e['anchor'],
                  'E_nominal':e['matched'][arm]['E_nominal'],'E_max_h0':e['matched'][arm]['E_max'],
                  'median_ms':med/1e6,'processes':len(good),'IQR_over_median':float(np.subtract(*np.quantile([u['median_ns'] for u in good],[.75,.25]))/med)}
            cost.append(item)
            work.append({**item,'counters_json':json.dumps(e['matched'][arm]['counters']['1.0'],sort_keys=True)})
    for p in comparisons:
        ratios=[]
        for b in range(12):
            pair={u['arm']:u for u in units if u['case']==p['case'] and u['grid']==p['grid'] and u['block']==b and u['valid']}
            if p['baseline'] in pair and p['candidate'] in pair:ratios.append(pair[p['baseline']]['median_ns']/pair[p['candidate']]['median_ns'])
        assert ratios==p['ratios'] and len(ratios)==p['paired_blocks']
        if p['status']=='COMPLETE_DIAGNOSTIC':
            assert len(ratios)==12 and p['ratio_median']==statistics.median(ratios)
            assert p['ci95_bonferroni_family'][0]<=p['ci95_pointwise'][0]<=p['ci95_pointwise'][1]<=p['ci95_bonferroni_family'][1]
            compact.append({k:p[k] for k in ['case','grid','baseline','candidate','anchor','ratio_median','sign_test_bonferroni']})
            checked['paired_median_recomputed']+=1
    csvout(out/'COST_AT_MATCHED_ERROR.csv',cost);csvout(out/'WORK_AT_MATCHED_ERROR.csv',work);csvout(out/'RECOMPUTED_ACCURACY.csv',allrows)
    assert checked['calibration']==summary['calibration_solves'] and checked['timing']==summary['timing_processes']
    max_cons=max((r['conservation_residual'] or 0 for r in allrows),default=0.)
    audit={'status':'PASS_SCOPED_NUMERICAL_READBACK','checked':dict(checked),'unique_state_objects_verified':len(state_cache),
        'max_conservation_residual_robertson_or_hires':max_cons,'new_timing_points_only':True,
        'independence':'separate vectorized computation; same agent authorship, not independent personnel review',
        'unverified':'bootstrap coverage under correlated host noise; continuous-time certified enclosure; full workspace regression suite',
        'legacy_archival_gap':'Run 39257390707 omitted two >20MiB raw gzip files; its CSV and block medians remain. This run retains content-addressed full states and raw per-repeat times.'}
    write(out/'AUDIT.json',audit)
    text=['# 달성 오차 정합·banded 대조 확인 결과','',f"측정 source: `{manifest['source_commit']}`",'',
          '모든 시간은 이 새 runner에서 실행했다. 이전 호스트의 시간 표본을 합치지 않았다. 모든 h0 배율의 오차를 anchor 아래로 제한하고 nominal h0에서 실제 오차를 [anchor/2, anchor] 안에 맞춘 구성만 시간 비교했다.','',
          '## 실행 요약','', '```json',json.dumps(summary,ensure_ascii=False,indent=2),'```','',
          '## 공통 달성 오차에서의 시간','', '|문제|출력|Arm|입력 rtol|실제 E_up|중앙시간 ms|','|---|---|---|---:|---:|---:|']
    for r in cost:text.append(f"|{r['case']}|{r['grid']}|{r['arm']}|{r['rtol']:.7g}|{r['E_nominal']:.5g}|{r['median_ms']:.6f}|")
    text+=['','## Pairwise 비율','', '비율은 baseline/candidate. 1보다 크면 candidate가 짧다. 구간은 42개 사전 지정 비교를 고려한 Bonferroni bootstrap 구간이며 공유호스트·12블록이라는 제한은 여전히 남는다.','',
           '|문제|출력|Baseline|Candidate|비율|family 구간|','|---|---|---|---|---:|---|']
    for p in comparisons:
        if p['status']=='COMPLETE_DIAGNOSTIC':text.append(f"|{p['case']}|{p['grid']}|{p['baseline']}|{p['candidate']}|{p['ratio_median']:.4f}|{p['ci95_bonferroni_family'][0]:.4f}, {p['ci95_bonferroni_family'][1]:.4f}|")
    text+=['','## 읽기 검산','', '```json',json.dumps(audit,ensure_ascii=False,indent=2),'```','',
           'Netlib DVODE banded와 VigilODE banded의 비교가 구조 정합 비교다. Diffsol은 FaerLU dense 구성이므로 해당 차이를 방법 자체의 순위로 확대하지 않는다. local-error tolerance와 실제 전방/global error의 구분, 경험적 참조 폭, 실패·미완료의 독립 보존 원칙을 유지한다.']
    (out/'REPORT_KO.md').write_text('\n'.join(text)+'\n')
    print(json.dumps(audit,sort_keys=True))
    # Reconstructible chart only; all series use matplotlib's default color cycle.
    try:
        import matplotlib;matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        for grid in ['endpoint','uniform101']:
            cs=sorted({r['case'] for r in cost if r['grid']==grid});arms=['vigilode-fast','vigilode-banded','diffsol-bdf','netlib-dvode']
            fig,ax=plt.subplots(figsize=(10,5));x=np.arange(len(cs));width=.19
            for k,a in enumerate(arms):
                vals=[next((r['median_ms'] for r in cost if r['case']==c and r['grid']==grid and r['arm']==a),np.nan) for c in cs]
                ax.bar(x+(k-1.5)*width,vals,width,label=a)
            ax.set_yscale('log');ax.set_xticks(x,cs,rotation=15,ha='right');ax.set_ylabel('ms / integration (log scale)')
            ax.set_title('Matched achieved error, '+grid+'; shared-runner diagnostics');ax.legend();fig.tight_layout();fig.savefig(out/('MATCHED_'+grid+'.png'),dpi=170);plt.close(fig)
    except ImportError:
        (out/'PLOTS_NOT_RUN.txt').write_text('matplotlib unavailable; CSVs and analysis are complete\n')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--previous',type=Path,required=True);a=p.parse_args();main(a.out,a.previous)
