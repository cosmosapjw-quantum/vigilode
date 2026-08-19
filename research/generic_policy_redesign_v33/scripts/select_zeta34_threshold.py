#!/usr/bin/env python3
import argparse, hashlib, json, math
from collections import Counter
from pathlib import Path

MIN_RECOMMENDED = 6
MIN_DISTINCT_FAMILIES = 3
MAX_SINGLE_FAMILY_FRACTION = 0.50


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
        z=row.get('quadratic_drift_zeta34')
        if not (
            row.get('budget_admitted')
            and row.get('prefix_succeeded')
            and row.get('audit_full_e_completed')
        ):
            continue
        if z is None or not math.isfinite(float(z)):
            continue
        r=dict(row)
        r['zeta34']=float(z)
        r['unsafe']=not bool(row.get('audit_full_e_locally_admissible'))
        out.append(r)
    return out


def evaluate(rows, tau):
    rec=[] if tau is None else [r for r in rows if r['zeta34'] <= tau]
    unsafe=sum(int(r['unsafe']) for r in rec)
    counts=Counter(r['family'] for r in rec)
    distinct=len(counts)
    max_fraction=(max(counts.values())/len(rec)) if rec else 1.0
    passes=(
        len(rec) >= MIN_RECOMMENDED
        and unsafe == 0
        and distinct >= MIN_DISTINCT_FAMILIES
        and max_fraction <= MAX_SINGLE_FAMILY_FRACTION
    )
    return {
        'tau':tau,
        'recommended':len(rec),
        'unsafe':unsafe,
        'distinct_families':distinct,
        'max_single_family_fraction':max_fraction,
        'family_counts':dict(sorted(counts.items())),
        'passes':passes,
    }


def select(rows):
    rows=eligible_rows(rows)
    values=sorted({r['zeta34'] for r in rows})
    scans=[evaluate(rows,None)] + [evaluate(rows,t) for t in values]
    if not any(r['unsafe'] for r in rows):
        return {
            'status':'all-abstain-no-unsafe-boundary',
            'tau_selected':None,'tau_final':None,
            'eligible_events':len(rows),'candidate_count':len(scans),'scan':scans,
        }
    survivors=[s for s in scans if s['passes']]
    if not survivors:
        return {
            'status':'all-abstain','tau_selected':None,'tau_final':None,
            'eligible_events':len(rows),'candidate_count':len(scans),'scan':scans,
        }
    best=sorted(survivors,key=lambda s:(-s['recommended'],s['tau']))[0]
    idx=values.index(best['tau'])
    if idx == 0:
        return {
            'status':'all-abstain-no-margin-predecessor',
            'tau_selected':best['tau'],'tau_final':None,
            'eligible_events':len(rows),'candidate_count':len(scans),
            'selected':best,'scan':scans,
        }
    tau_final=values[idx-1]
    final=evaluate(rows,tau_final)
    if not final['passes']:
        return {
            'status':'all-abstain-after-rank-margin',
            'tau_selected':best['tau'],'tau_final':None,
            'eligible_events':len(rows),'candidate_count':len(scans),
            'selected':best,'final':final,'scan':scans,
        }
    return {
        'status':'frozen-threshold',
        'tau_selected':best['tau'],'tau_final':tau_final,
        'eligible_events':len(rows),'candidate_count':len(scans),
        'selected':best,'final':final,'scan':scans,
    }


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('inputs',nargs='+')
    ap.add_argument('--output',required=True)
    args=ap.parse_args()
    rows,hashes=load_rows(args.inputs)
    result=select(rows)
    result.update({
        'schema':'gvjf-v33-zeta34-independent-calibration-v1',
        'input_sha256':hashes,
        'minimum_recommended':MIN_RECOMMENDED,
        'minimum_distinct_families':MIN_DISTINCT_FAMILIES,
        'maximum_single_family_fraction':MAX_SINGLE_FAMILY_FRACTION,
        'selection_direction':'recommend-if-zeta34-lte-tau',
        'safety_margin':'one-preceding-distinct-zeta34-rank',
        'statistical_coverage_claim':False,
    })
    Path(args.output).write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')

if __name__=='__main__': main()
