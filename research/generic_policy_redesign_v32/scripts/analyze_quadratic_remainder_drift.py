#!/usr/bin/env python3
from pathlib import Path
import json, math
import pandas as pd
import numpy as np
from sklearn.metrics import roc_auc_score, average_precision_score

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'results' / 'discovery_analysis'
OUT.mkdir(parents=True, exist_ok=True)
V31 = ROOT.parent / 'generic_policy_redesign_v31'

profiles = [
    ('discovery96_prefix_shards', 'discovery96_shards', 96),
    ('discovery256_prefix_shards', 'discovery256_shards', 256),
]
wall_tokens = ('wall', 'seconds')
rows=[]
parity=[]
event_parity=[]

EVENT_KEYS = [
    'trajectory_id','family','dimension','rtol','decision_accepted_step',
    'target_attempt_index','target_accepted_steps_before','t_start','h'
]

def clean_nonwall(obj):
    if isinstance(obj, dict):
        return {
            k: clean_nonwall(v)
            for k,v in obj.items()
            if not any(tok in k.lower() for tok in wall_tokens)
        }
    if isinstance(obj, list):
        return [clean_nonwall(x) for x in obj]
    return obj

def fclose(a,b,rtol=0.0,atol=0.0):
    if a is None or b is None: return a is None and b is None
    if isinstance(a,(int,bool)) and isinstance(b,(int,bool)): return a==b
    try:
        return math.isclose(float(a),float(b),rel_tol=rtol,abs_tol=atol)
    except Exception:
        return a==b

def extract_prefix(r):
    pr=r['prefix_report']
    l1=pr['level1_report']
    e2=l1['early_flow_defect']
    e3=pr['stage3_flow_defect']
    e4=pr['stage4_flow_defect']
    vg=pr.get('remainder_vector_geometry') or {}
    qd=pr.get('quadratic_remainder_drift') or {}
    return {
        'rho2': e2.get('tolerance_scaled_defect_wrms'),
        'rho3': e3.get('tolerance_scaled_defect_wrms'),
        'rho4': e4.get('tolerance_scaled_defect_wrms'),
        'remainder_chi34': vg.get('chi34'),
        'zeta23': qd.get('zeta23'),
        'zeta34': qd.get('zeta34'),
        'relative_drift': qd.get('relative_drift'),
        'prefix_jvp_vectors': sum(
            int(fr.get('maximum_krylov_dimension',0))
            for fr in (l1.get('fused_phi_reports') or [])
        ) if False else None,
    }

for newdir_name, olddir_name, dim in profiles:
    newdir=ROOT/'results'/newdir_name
    olddir=V31/'results'/olddir_name
    nfiles=sorted(newdir.glob('*.json'))
    assert len(nfiles)==6,(dim,len(nfiles),[p.name for p in nfiles])
    for p in nfiles:
        d=json.load(open(p))
        old=json.load(open(olddir/p.name))
        assert d['status']=='complete'
        assert d['switching_active'] is False
        assert d['full_e_continuations']==0
        assert old['status']=='complete'
        assert old['switching_active'] is False
        assert old['runtime_full_e_continuations']==0

        # Exact R-JF numerical/work parity on arrays after dropping only timing fields.
        for section in ['attempt_rows','accepted_rows','trajectories']:
            exact = clean_nonwall(d[section]) == clean_nonwall(old[section])
            parity.append({
                'dimension':dim,'family_file':p.stem,'section':section,
                'exact_nonwall':exact,
                'new_rows':len(d[section]),'old_rows':len(old[section]),
            })
            assert exact,(dim,p.name,section)

        old_index={tuple(r[k] for k in EVENT_KEYS):r for r in old['rows']}
        new_keys=[]
        for r in d['prefix_rows']:
            key=tuple(r[k] for k in EVENT_KEYS)
            new_keys.append(key)
            assert key in old_index,(dim,p.name,key)
            label=old_index[key]
            ex=extract_prefix(r)
            assert r['prefix_succeeded'] is True
            assert r['full_e_continued'] is False
            # Stage diagnostics and vector geometry must match v3.1 authority rows exactly.
            checks={
                'rho2': fclose(ex['rho2'],label['rho2']),
                'rho3': fclose(ex['rho3'],label['rho3']),
                'rho4': fclose(ex['rho4'],label['rho4']),
                'remainder_chi34': fclose(ex['remainder_chi34'],label['remainder_chi34']),
                'target_r_error_norm': fclose(r['target_r_error_norm'],label['target_r_error_norm']),
                'feature_value': fclose(r['feature_value'],label['feature_value']),
            }
            assert all(checks.values()),(dim,p.name,key,checks)
            drift=prdrift=r['prefix_report'].get('quadratic_remainder_drift')
            assert drift is not None
            z23=drift.get('zeta23'); z34=drift.get('zeta34'); rel=drift.get('relative_drift')
            rec={k:r[k] for k in EVENT_KEYS}
            rec.update({
                'target_r_error_norm':r['target_r_error_norm'],
                'zeta23':z23,'zeta34':z34,'relative_drift':rel,
                'stage_log_curvature_kappa234':label['stage_log_curvature_kappa234'],
                'remainder_chi34':label['remainder_chi34'],
                'rho2':label['rho2'],'rho3':label['rho3'],'rho4':label['rho4'],
                'audit_full_e_total_error':label['audit_full_e_total_error'],
                'audit_full_e_locally_admissible':bool(label['audit_full_e_locally_admissible']),
                'unsafe':not bool(label['audit_full_e_locally_admissible']),
                'actual_prefix_jvp_vectors':label['actual_prefix_jvp_vectors'],
            })
            rows.append(rec)
            event_parity.append({
                'dimension':dim,'family_file':p.stem,'event_key_match':True,
                'stage_authority_match':True,
            })
        assert set(new_keys)==set(old_index.keys()),(
            dim,p.name,len(new_keys),len(old_index),set(old_index)-set(new_keys)
        )

frame=pd.DataFrame(rows).sort_values(EVENT_KEYS).reset_index(drop=True)
par=pd.DataFrame(parity)
evp=pd.DataFrame(event_parity)
par.to_csv(OUT/'V31_V32_RJF_PARITY.csv',index=False)
evp.to_csv(OUT/'V31_V32_EVENT_PARITY.csv',index=False)
frame.to_csv(OUT/'DISCOVERY_EVENT_ROWS.csv',index=False)

for f in ['zeta23','zeta34','relative_drift']:
    assert np.isfinite(pd.to_numeric(frame[f])).all(), f
assert len(frame)==48
assert int(frame.unsafe.sum())==5

features=['zeta34','relative_drift','zeta23','stage_log_curvature_kappa234','remainder_chi34']
authority=['zeta34','relative_drift']

def metrics(g, feature):
    d=g[[feature,'unsafe']].dropna()
    y=d['unsafe'].astype(int).to_numpy(); x=d[feature].astype(float).to_numpy()
    if len(x)==0 or len(np.unique(y))<2: return None
    auc=roc_auc_score(y,x); inv=roc_auc_score(y,-x)
    ap=average_precision_score(y,x); api=average_precision_score(y,-x)
    if inv>auc:
        return {'rows':len(d),'unsafe':int(y.sum()),'orientation':'lower-is-unsafe',
                'orientation_free_auc':float(inv),'oriented_average_precision':float(api),'raw_auc':float(auc)}
    return {'rows':len(d),'unsafe':int(y.sum()),'orientation':'higher-is-unsafe',
            'orientation_free_auc':float(auc),'oriented_average_precision':float(ap),'raw_auc':float(auc)}

summary=[]
for f in features:
    m=metrics(frame,f)
    summary.append({'scope':'pooled','dimension':None,'feature':f,**(m or {})})
    for dim,g in frame.groupby('dimension'):
        m=metrics(g,f)
        summary.append({'scope':'dimension','dimension':int(dim),'feature':f,**(m or {})})
summary=pd.DataFrame(summary)
summary.to_csv(OUT/'FEATURE_SUMMARY.csv',index=False)

loo=[]
for f in features:
    for held in sorted(frame.family.unique()):
        m=metrics(frame[frame.family!=held],f)
        loo.append({'feature':f,'held_out_family':held,**(m or {})})
loo=pd.DataFrame(loo)
loo.to_csv(OUT/'LEAVE_ONE_FAMILY_OUT.csv',index=False)

robust=[]
for f in features:
    pooled=summary[(summary.scope=='pooled')&(summary.feature==f)].iloc[0]
    dims=summary[(summary.scope=='dimension')&(summary.feature==f)]
    lf=loo[loo.feature==f]
    orientations=dims['orientation'].dropna().tolist()
    orientation_agrees=(len(set(orientations))==1 and len(orientations)==2)
    rec={
        'feature':f,
        'pooled_auc':float(pooled.orientation_free_auc),
        'min_dimension_auc':float(dims.orientation_free_auc.min()),
        'max_dimension_auc':float(dims.orientation_free_auc.max()),
        'dimension96_orientation':dims[dims.dimension==96].iloc[0].orientation,
        'dimension256_orientation':dims[dims.dimension==256].iloc[0].orientation,
        'dimension_orientation_agrees':bool(orientation_agrees),
        'min_leave_one_family_out_auc':float(lf.orientation_free_auc.min()),
        'median_leave_one_family_out_auc':float(lf.orientation_free_auc.median()),
        'pooled_average_precision':float(pooled.oriented_average_precision),
    }
    rec['passes_predeclared_gate']=bool(
        f in authority and
        rec['pooled_auc']>=0.70 and
        rec['min_dimension_auc']>=0.70 and
        rec['min_leave_one_family_out_auc']>=0.60 and
        rec['dimension_orientation_agrees']
    )
    robust.append(rec)
robust=pd.DataFrame(robust).sort_values(
    ['passes_predeclared_gate','min_leave_one_family_out_auc','min_dimension_auc','pooled_auc'],
    ascending=False
)
robust.to_csv(OUT/'ROBUSTNESS_RANKING.csv',index=False)

ranges=[]
for f in features:
    for unsafe,g in frame.groupby('unsafe'):
        v=g[f].dropna().astype(float)
        ranges.append({'feature':f,'class':'unsafe' if unsafe else 'safe','count':len(v),
                       'min':v.min(),'median':v.median(),'max':v.max()})
pd.DataFrame(ranges).to_csv(OUT/'FEATURE_RANGES.csv',index=False)

surv=robust[(robust.feature.isin(authority)) & robust.passes_predeclared_gate]
selected=None
if len(surv):
    selected=surv.iloc[0].feature
result={
    'schema':'gvjf-v32-quadratic-remainder-drift-discovery-analysis-v1',
    'events':int(len(frame)),'unsafe_events':int(frame.unsafe.sum()),
    'all_rjf_parity_exact':bool(par.exact_nonwall.all()),
    'all_event_keys_and_stage_authority_match':bool(evp.event_key_match.all() and evp.stage_authority_match.all()),
    'all_drift_features_finite':True,
    'authority_witnesses':authority,
    'selected_survivor':selected,
    'survivor_count':int(len(surv)),
    'threshold_selected':False,
    'N192_executed':False,'N384_executed':False,'N2048_executed':False,
    'active_switching':False,
    'runtime_full_e_activation':False,
    'robustness':robust.to_dict('records'),
}
json.dump(result,open(OUT/'RESULT_SUMMARY.json','w'),indent=2)
print(json.dumps(result,indent=2))
print('\nROBUSTNESS')
print(robust.to_string(index=False))
