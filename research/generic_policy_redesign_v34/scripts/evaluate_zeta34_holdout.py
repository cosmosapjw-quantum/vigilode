#!/usr/bin/env python3
import argparse,hashlib,json,math
from collections import Counter
from pathlib import Path

TAU=13.39706618860016
MIN_RECOMMENDED=6
MIN_DISTINCT_FAMILIES=3
MAX_SINGLE_FAMILY_FRACTION=0.50

def load(paths):
    rows=[]; hashes={}; shard_meta=[]
    for p in map(Path,paths):
        b=p.read_bytes(); hashes[str(p)]=hashlib.sha256(b).hexdigest(); d=json.loads(b)
        if d.get('schema')!='g4-s5b0-stage-growth-safety-audit-v1': raise ValueError(p)
        if d.get('profile')!='stage-growth-holdout-384': raise ValueError((p,d.get('profile')))
        shard_meta.append({'file':str(p),'status':d['status'],'budget_breaches':d['budget_breaches'],'runtime_full_e_continuations':d['runtime_full_e_continuations'],'switching_active':d['switching_active']})
        rows.extend(d['rows'])
    return rows,hashes,shard_meta

def evaluate(rows):
    eligible=[]
    for r in rows:
        z=r.get('quadratic_drift_zeta34')
        if not (r.get('budget_admitted') and r.get('prefix_succeeded') and r.get('audit_full_e_completed')): continue
        if z is None or not math.isfinite(float(z)): continue
        q=dict(r); q['zeta34']=float(z); q['unsafe']=not bool(r.get('audit_full_e_locally_admissible')); eligible.append(q)
    rec=[r for r in eligible if r['zeta34']<=TAU]
    unsafe=sum(int(r['unsafe']) for r in rec)
    c=Counter(r['family'] for r in rec); distinct=len(c); maxfrac=max(c.values())/len(rec) if rec else 1.0
    return eligible,rec,{
        'tau':TAU,'eligible':len(eligible),'recommended':len(rec),'unsafe_recommendations':unsafe,
        'distinct_recommended_families':distinct,'max_single_family_fraction':maxfrac,'family_counts':dict(sorted(c.items())),
        'passes_policy_gates':bool(unsafe==0 and len(rec)>=MIN_RECOMMENDED and distinct>=MIN_DISTINCT_FAMILIES and maxfrac<=MAX_SINGLE_FAMILY_FRACTION),
    }

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('inputs',nargs='+'); ap.add_argument('--output',required=True); ap.add_argument('--rows-output'); args=ap.parse_args()
    rows,hashes,meta=load(args.inputs); eligible,rec,result=evaluate(rows)
    shard_ok=all(x['status']=='complete' and x['budget_breaches']==0 and x['runtime_full_e_continuations']==0 and x['switching_active'] is False for x in meta)
    result.update({'schema':'gvjf-v34-zeta34-sealed-holdout-v1','input_sha256':hashes,'shards':meta,'all_shard_runtime_gates_pass':shard_ok,'passes_all_hard_gates':bool(shard_ok and result['passes_policy_gates']),'threshold_retuned':False,'active_switching':False,'N2048_executed':False})
    Path(args.output).write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    if args.rows_output:
        import csv
        fields=['trajectory_id','family','dimension','rtol','decision_accepted_step','target_attempt_index','zeta34','unsafe','audit_full_e_total_error']
        with Path(args.rows_output).open('w',newline='') as f:
            w=csv.DictWriter(f,fieldnames=fields); w.writeheader()
            for r in eligible:
                w.writerow({k:r.get(k) for k in fields})

if __name__=='__main__': main()
