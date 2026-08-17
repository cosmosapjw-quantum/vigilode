#!/usr/bin/env python3
import argparse, hashlib, json, math
from pathlib import Path

DELTA = 0.05
MAX_SIMULTANEOUS_UPPER = 0.15
MIN_RECOMMENDED = 12
SHRINK = 0.95


def load_rows(paths):
    rows=[]; hashes={}
    for path in map(Path, paths):
        data=path.read_bytes(); hashes[str(path)]=hashlib.sha256(data).hexdigest()
        doc=json.loads(data)
        if doc.get('schema') != 'g4-s5b0-stage-growth-safety-audit-v1':
            raise ValueError(f'unexpected schema in {path}')
        rows.extend(doc['rows'])
    return rows, hashes


def eligible_rows(rows):
    out=[]
    for row in rows:
        a=row.get('normalized_stage_growth_a34')
        if not (row.get('budget_admitted') and row.get('prefix_succeeded') and row.get('audit_full_e_completed')):
            continue
        if a is None or not math.isfinite(float(a)):
            continue
        r=dict(row)
        r['unsafe']=not bool(row.get('audit_full_e_locally_admissible'))
        out.append(r)
    return out


def bonferroni_zero_failure_upper(n, m, delta=DELTA):
    if n <= 0: return 1.0
    return 1.0 - (delta / max(1,m)) ** (1.0/n)


def evaluate(rows, tau, m):
    rec=[] if tau is None else [r for r in rows if float(r['normalized_stage_growth_a34']) <= tau]
    unsafe=sum(r['unsafe'] for r in rec)
    groups={}
    for r in rec:
        key=f"{r['family']}|N={r['dimension']}"
        g=groups.setdefault(key, {'recommended':0,'unsafe':0})
        g['recommended'] += 1; g['unsafe'] += int(r['unsafe'])
    upper=bonferroni_zero_failure_upper(len(rec), m)
    passes=(len(rec)>=MIN_RECOMMENDED and unsafe==0 and all(g['unsafe']==0 for g in groups.values()) and upper<=MAX_SIMULTANEOUS_UPPER)
    return {'tau':tau,'recommended':len(rec),'unsafe':unsafe,'groups':groups,'bonferroni_upper_95':upper,'passes':passes}


def select(rows):
    rows=eligible_rows(rows)
    values=sorted({float(r['normalized_stage_growth_a34']) for r in rows})
    m=len(values)+1
    scans=[evaluate(rows,None,m)] + [evaluate(rows,t,m) for t in values]
    survivors=[s for s in scans if s['passes']]
    if not survivors:
        return {'status':'all-abstain','tau_selected':None,'tau_final':None,'eligible_events':len(rows),'candidate_count':m,'scan':scans}
    best=sorted(survivors,key=lambda s:(-s['recommended'], s['tau']))[0]
    tau_final=SHRINK*best['tau']
    final=evaluate(rows,tau_final,m)
    if not final['passes']:
        return {'status':'all-abstain-after-shrink','tau_selected':best['tau'],'tau_final':None,'eligible_events':len(rows),'candidate_count':m,'selected':best,'final':final,'scan':scans}
    return {'status':'frozen-threshold','tau_selected':best['tau'],'tau_final':tau_final,'eligible_events':len(rows),'candidate_count':m,'selected':best,'final':final,'scan':scans}


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('inputs', nargs='+')
    ap.add_argument('--output', required=True)
    args=ap.parse_args()
    rows, hashes=load_rows(args.inputs)
    result=select(rows)
    result.update({'schema':'gvjf-v29-stage-growth-threshold-selection-v1','input_sha256':hashes,'delta':DELTA,'max_simultaneous_upper':MAX_SIMULTANEOUS_UPPER,'minimum_recommended':MIN_RECOMMENDED,'shrink':SHRINK})
    Path(args.output).write_text(json.dumps(result, indent=2, sort_keys=True)+"\n")

if __name__=='__main__': main()
