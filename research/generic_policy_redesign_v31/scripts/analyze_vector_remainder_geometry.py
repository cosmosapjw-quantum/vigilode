#!/usr/bin/env python3
from pathlib import Path
import json, math
import pandas as pd
import numpy as np
from sklearn.metrics import roc_auc_score, average_precision_score

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'results' / 'discovery_analysis'
OUT.mkdir(parents=True, exist_ok=True)

profiles = [
    ('discovery96_shards', 96),
    ('discovery256_shards', 256),
]
rows=[]
parity=[]
new_fields={
    'remainder_chi23','remainder_chi34','remainder_chi24',
    'remainder_q34_perp','remainder_delta_chi'
}
wall_tokens=('wall','seconds')

for dirname, dim in profiles:
    newdir=ROOT/'results'/dirname
    olddir=ROOT.parent/'generic_policy_redesign_v30'/'results'/dirname
    for p in sorted(newdir.glob('*.json')):
        d=json.load(open(p))
        oldp=olddir/p.name
        old=json.load(open(oldp))
        assert d['status']=='complete'
        assert d['switching_active'] is False
        assert d['runtime_full_e_continuations']==0
        assert old['status']=='complete'
        # Strong parity: recursively compare after dropping only wall fields and new fields.
        def clean(obj):
            if isinstance(obj, dict):
                out={}
                for k,v in obj.items():
                    if k in new_fields or any(tok in k.lower() for tok in wall_tokens):
                        continue
                    out[k]=clean(v)
                return out
            if isinstance(obj, list): return [clean(x) for x in obj]
            return obj
        exact = clean(d)==clean(old)
        parity.append({'dimension':dim,'family_file':p.stem,'exact_nonwall_nonnew':exact})
        assert exact, (dim,p.name)
        for r in d['rows']:
            rec=dict(r)
            rec['unsafe']=not bool(r['audit_full_e_locally_admissible'])
            rows.append(rec)

frame=pd.DataFrame(rows)
par=pd.DataFrame(parity)
par.to_csv(OUT/'V30_V31_PARITY.csv',index=False)
frame.to_csv(OUT/'DISCOVERY_EVENT_ROWS.csv',index=False)

features=[
    'stage_log_curvature_kappa234',
    'remainder_chi23','remainder_chi34','remainder_chi24',
    'remainder_q34_perp','remainder_delta_chi',
]

def metrics(g, feature):
    d=g[[feature,'unsafe']].dropna()
    y=d['unsafe'].astype(int).to_numpy()
    x=d[feature].astype(float).to_numpy()
    if len(np.unique(y))<2 or len(x)==0:
        return None
    auc=roc_auc_score(y,x)
    auc_inv=roc_auc_score(y,-x)
    ap=average_precision_score(y,x)
    ap_inv=average_precision_score(y,-x)
    if auc_inv > auc:
        orient='lower-is-unsafe'
        best_auc=auc_inv; best_ap=ap_inv
    else:
        orient='higher-is-unsafe'
        best_auc=auc; best_ap=ap
    return {
        'rows':len(d),'unsafe':int(y.sum()),'orientation':orient,
        'orientation_free_auc':float(best_auc),
        'oriented_average_precision':float(best_ap),
        'raw_auc':float(auc),
    }

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
    for held in sorted(frame['family'].unique()):
        m=metrics(frame[frame['family']!=held],f)
        loo.append({'feature':f,'held_out_family':held,**(m or {})})
loo=pd.DataFrame(loo)
loo.to_csv(OUT/'LEAVE_ONE_FAMILY_OUT.csv',index=False)

# Worst-case and descriptive survivor comparison against v3.0 kappa baseline.
robust=[]
for f in features:
    pooled=summary[(summary.scope=='pooled') & (summary.feature==f)].iloc[0]
    dims=summary[(summary.scope=='dimension') & (summary.feature==f)]
    lf=loo[loo.feature==f]
    robust.append({
        'feature':f,
        'pooled_auc':pooled.get('orientation_free_auc',np.nan),
        'min_dimension_auc':dims['orientation_free_auc'].min(),
        'max_dimension_auc':dims['orientation_free_auc'].max(),
        'min_leave_one_family_out_auc':lf['orientation_free_auc'].min(),
        'median_leave_one_family_out_auc':lf['orientation_free_auc'].median(),
        'pooled_average_precision':pooled.get('oriented_average_precision',np.nan),
    })
robust=pd.DataFrame(robust).sort_values(['min_leave_one_family_out_auc','min_dimension_auc','pooled_auc'],ascending=False)
robust.to_csv(OUT/'ROBUSTNESS_RANKING.csv',index=False)

# Pairwise safe/unsafe ranges and sign patterns for interpretability.
ranges=[]
for f in features:
    for unsafe,g in frame.groupby('unsafe'):
        vals=g[f].dropna().astype(float)
        ranges.append({
            'feature':f,'class':'unsafe' if unsafe else 'safe',
            'count':len(vals),'min':vals.min() if len(vals) else np.nan,
            'median':vals.median() if len(vals) else np.nan,
            'max':vals.max() if len(vals) else np.nan,
        })
pd.DataFrame(ranges).to_csv(OUT/'FEATURE_RANGES.csv',index=False)

base=robust[robust.feature=='stage_log_curvature_kappa234'].iloc[0]
vector=robust[robust.feature!='stage_log_curvature_kappa234'].copy()
best=vector.iloc[0]
# Strict discovery-survival interpretation: improve pooled + min-dimension + LOFO-min over kappa.
survives = bool(
    best.pooled_auc > base.pooled_auc and
    best.min_dimension_auc > base.min_dimension_auc and
    best.min_leave_one_family_out_auc > base.min_leave_one_family_out_auc and
    best.min_leave_one_family_out_auc >= 0.60
)
result={
    'schema':'gvjf-v31-vector-remainder-geometry-discovery-analysis-v1',
    'events':len(frame),'unsafe_events':int(frame.unsafe.sum()),
    'all_shards_parity_exact':bool(par.exact_nonwall_nonnew.all()),
    'baseline_kappa':base.to_dict(),
    'best_vector_feature':best.to_dict(),
    'strict_survivor':survives,
    'threshold_selected':False,
    'N192_executed':False,'N384_executed':False,'N2048_executed':False,
    'active_switching':False,
}
json.dump(result,open(OUT/'RESULT_SUMMARY.json','w'),indent=2)
print(json.dumps(result,indent=2))
print('\nRobustness ranking:')
print(robust.to_string(index=False))
