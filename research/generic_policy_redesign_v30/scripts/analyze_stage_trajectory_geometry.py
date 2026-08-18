#!/usr/bin/env python3
import csv, json, math, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
NEW_DIRS = {
    96: ROOT / 'results' / 'discovery96_shards',
    256: ROOT / 'results' / 'discovery256_shards',
}
OLD_DIRS = {
    96: REPO / 'research' / 'generic_policy_redesign_v29' / 'results' / 'calibration96_shards',
    256: REPO / 'research' / 'generic_policy_redesign_v29' / 'results' / 'calibration256_shards',
}
OUT = ROOT / 'results' / 'discovery_analysis'
OUT.mkdir(parents=True, exist_ok=True)

NEW_FIELDS = {
    'rho2','rho3','rho4','stage_log_slope_s23','stage_log_slope_s34',
    'stage_log_curvature_kappa234'
}
WALL_FIELDS = {
    'rodas_wall_seconds','exponential_wall_seconds','exponential_prefix_wall_seconds',
    'prefix_wall_seconds','wall_seconds'
}
FEATURES = [
    'rho2','rho3','rho4','normalized_stage_growth_a34',
    'stage_log_slope_s23','stage_log_slope_s34','stage_log_curvature_kappa234'
]

def load(path):
    return json.loads(path.read_text())

def auc_binary(labels, values):
    pairs = sorted(zip(values, labels), key=lambda x: x[0])
    # average ranks for ties
    ranks = [0.0]*len(pairs)
    i=0
    while i<len(pairs):
        j=i+1
        while j<len(pairs) and pairs[j][0] == pairs[i][0]: j+=1
        avg=(i+1+j)/2.0
        for k in range(i,j): ranks[k]=avg
        i=j
    n1=sum(labels); n0=len(labels)-n1
    if n1==0 or n0==0: return None
    rank_sum=sum(r for r,(_,y) in zip(ranks,pairs) if y)
    return (rank_sum - n1*(n1+1)/2)/(n1*n0)

def strip_for_parity(obj):
    if isinstance(obj, dict):
        return {k: strip_for_parity(v) for k,v in obj.items()
                if k not in NEW_FIELDS and k not in WALL_FIELDS}
    if isinstance(obj, list): return [strip_for_parity(v) for v in obj]
    return obj

parity=[]
events=[]
for dim, ndir in NEW_DIRS.items():
    for np in sorted(ndir.glob('*.json')):
        op = OLD_DIRS[dim] / np.name
        new=load(np); old=load(op)
        parity.append({
            'dimension':dim,'family_file':np.stem,
            'numerical_nonwall_exact': strip_for_parity(new)==strip_for_parity(old),
            'new_rows':len(new['rows']),'old_rows':len(old['rows']),
            'new_budget_breaches':new['budget_breaches'],'old_budget_breaches':old['budget_breaches'],
            'new_runtime_full_e_continuations':new['runtime_full_e_continuations'],
            'old_runtime_full_e_continuations':old['runtime_full_e_continuations'],
        })
        for r in new['rows']:
            if not r.get('audit_full_e_completed'): continue
            row={k:r.get(k) for k in r.keys()}
            row['profile']=new['profile']
            row['unsafe']=not bool(r.get('audit_full_e_locally_admissible'))
            events.append(row)

# CSV writer helper
def write_csv(path, rows):
    rows=list(rows)
    if not rows:
        path.write_text('')
        return
    fields=[]
    seen=set()
    for r in rows:
        for k in r:
            if k not in seen:
                seen.add(k); fields.append(k)
    with path.open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=fields); w.writeheader(); w.writerows(rows)

write_csv(OUT/'PARITY_SUMMARY.csv', parity)
write_csv(OUT/'DISCOVERY_EVENT_ROWS.csv', events)

feature_summary=[]
for feat in FEATURES:
    valid=[r for r in events if isinstance(r.get(feat),(int,float)) and math.isfinite(r[feat])]
    labels=[1 if r['unsafe'] else 0 for r in valid]; vals=[float(r[feat]) for r in valid]
    auc=auc_binary(labels, vals)
    safe=[float(r[feat]) for r in valid if not r['unsafe']]
    unsafe=[float(r[feat]) for r in valid if r['unsafe']]
    feature_summary.append({
        'feature':feat,'n':len(valid),'unsafe_n':sum(labels),
        'signed_auc':auc,'orientation_free_auc':max(auc,1-auc) if auc is not None else None,
        'safe_min':min(safe) if safe else None,'safe_max':max(safe) if safe else None,
        'unsafe_min':min(unsafe) if unsafe else None,'unsafe_max':max(unsafe) if unsafe else None,
    })
write_csv(OUT/'FEATURE_SUMMARY.csv',feature_summary)

cross=[]
for feat in FEATURES:
    for dim in (96,256):
        valid=[r for r in events if r['dimension']==dim and isinstance(r.get(feat),(int,float)) and math.isfinite(r[feat])]
        labels=[1 if r['unsafe'] else 0 for r in valid]; vals=[float(r[feat]) for r in valid]
        auc=auc_binary(labels,vals) if 0<sum(labels)<len(labels) else None
        cross.append({'feature':feat,'dimension':dim,'n':len(valid),'unsafe_n':sum(labels),
                      'signed_auc':auc,'orientation_free_auc':max(auc,1-auc) if auc is not None else None})
write_csv(OUT/'CROSS_DIMENSION_FEATURE_SUMMARY.csv',cross)

families=sorted({r['family'] for r in events})
loo=[]
for feat in FEATURES:
    for held in families:
        valid=[r for r in events if r['family']!=held and isinstance(r.get(feat),(int,float)) and math.isfinite(r[feat])]
        labels=[1 if r['unsafe'] else 0 for r in valid]; vals=[float(r[feat]) for r in valid]
        auc=auc_binary(labels,vals) if 0<sum(labels)<len(labels) else None
        loo.append({'feature':feat,'held_out_family':held,'n':len(valid),'unsafe_n':sum(labels),
                    'signed_auc':auc,'orientation_free_auc':max(auc,1-auc) if auc is not None else None})
write_csv(OUT/'LEAVE_ONE_FAMILY_OUT_FEATURE_SUMMARY.csv',loo)

kappa=[r for r in feature_summary if r['feature']=='stage_log_curvature_kappa234'][0]
kcross=[r for r in cross if r['feature']=='stage_log_curvature_kappa234']
kloo=[r for r in loo if r['feature']=='stage_log_curvature_kappa234' and r['orientation_free_auc'] is not None]
summary={
    'schema':'gvjf-v30-stage-trajectory-discovery-analysis-v1',
    'events':len(events),'unsafe_events':sum(1 for r in events if r['unsafe']),
    'all_nonwall_numerical_parity_exact':all(r['numerical_nonwall_exact'] for r in parity),
    'all_runtime_full_e_continuations_zero':all(r['new_runtime_full_e_continuations']==0 for r in parity),
    'all_budget_breaches_zero':all(r['new_budget_breaches']==0 for r in parity),
    'kappa_pooled_orientation_free_auc':kappa['orientation_free_auc'],
    'kappa_cross_dimension':kcross,
    'kappa_leave_one_family_out_min_orientation_free_auc':min(r['orientation_free_auc'] for r in kloo),
    'kappa_leave_one_family_out':kloo,
    'discovery_threshold_selected':False,
    'N192_executed':False,'N384_executed':False,'N2048_executed':False,
    'active_switching':False,
}
(OUT/'DISCOVERY_RESULT_SUMMARY.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2))
