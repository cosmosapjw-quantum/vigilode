"""E-01: zero-cost re-analysis of the committed scientific-validity-v2 54-case campaign.
Input: calibration_all_cases_compact.json (byte-identical to the tracked bundle file).
Outputs: E-01_rows.csv, E-01_summary.json. No solver is run."""
import json, csv, statistics as st, hashlib, sys
src = "calibration_all_cases_compact.json"
d = json.load(open(src))
sha = hashlib.sha256(open(src,'rb').read()).hexdigest()
rows = []
def g(x, *ks, default=None):
    for k in ks:
        if isinstance(x, dict) and k in x: x = x[k]
        else: return default
    return x
# discover metric key names from first record
a0 = d['records'][0]['artifact']
print("clipped keys:", list(a0['clipped'].keys()))
print("clipped.metrics keys:", list((a0['clipped'].get('metrics') or {}).keys()))
print("clipped.work keys:", list((a0['clipped'].get('work') or {}).keys())[:40])
print("reference keys:", list(a0['reference'].keys()) if isinstance(a0['reference'], dict) else type(a0['reference']))
print("config keys:", list(a0['config'].keys()) if isinstance(a0['config'], dict) else a0['config'])
for rec in d['records']:
    a = rec['artifact']; sp = a['spec']; c = a['clipped']; de = a['dense']
    cm = c.get('metrics') or {}; dm = de.get('metrics') or {}
    cw = c.get('counters') or {}; dw = de.get('counters') or {}; cd = c.get('diagnostics') or {}; dd = de.get('diagnostics') or {}
    gap = a['output_policy_discrepancy_wrms']
    r = dict(case_id=sp['id'], family=sp['family'], n=sp['dimension'], rtol=sp['rtol'], atol=sp['atol'],
             status=a['row']['status'] if isinstance(a.get('row'), dict) else rec['status'],
             ref_unc=a.get('reference_uncertainty_wrms'),
             gap=gap,
             c_end=cm.get('endpoint_wrms'), c_max=cm.get('max_grid_wrms'), c_rms=cm.get('rms_grid_wrms'),
             d_end=dm.get('endpoint_wrms'), d_max=dm.get('max_grid_wrms'), d_rms=dm.get('rms_grid_wrms'),
             c_acc=cw.get('accepted_steps') or cd.get('accepted_steps'), c_rej=cw.get('rejected_steps') or cd.get('rejected_steps'), c_clip=c.get('output_clipped_steps'), c_int=c.get('internal_steps'), d_int=de.get('internal_steps'),
             d_acc=dw.get('accepted_steps') or dd.get('accepted_steps'), d_rej=dw.get('rejected_steps') or dd.get('rejected_steps'),
             c_rhs=cw.get('rhs_calls'), d_rhs=dw.get('rhs_calls'), c_jvp=cw.get('jvp_calls'), d_jvp=dw.get('jvp_calls'),
             c_wall=c.get('wall_seconds'), d_wall=de.get('wall_seconds'))
    r['gap_over_dmax'] = gap/r['d_max'] if r['d_max'] else None
    r['cmax_over_dmax'] = r['c_max']/r['d_max'] if r['d_max'] and r['c_max'] is not None else None
    r['dmax_over_dend'] = r['d_max']/r['d_end'] if r['d_end'] else None
    r['cmax_over_cend'] = r['c_max']/r['c_end'] if r['c_end'] else None
    r['dmax_over_rtol'] = r['d_max']  # WRMS already tol-normalized: 1.0 == at tolerance
    rows.append(r)
rows.sort(key=lambda r:(r['family'], r['n'], r['rtol']))
with open("E-01_rows.csv","w",newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)
def q(xs, p):
    xs = sorted(x for x in xs if x is not None); 
    if not xs: return None
    k = (len(xs)-1)*p; i = int(k); return xs[i] if i==len(xs)-1 else xs[i]+(xs[i+1]-xs[i])*(k-i)
def summ(key):
    xs=[r[key] for r in rows if r.get(key) is not None]
    if not xs: return dict(n=0)
    return dict(n=len(xs), min=min(xs), q25=q(xs,.25), median=st.median(xs), q75=q(xs,.75), max=max(xs))
# replica check: same family+rtol across n -> compare gap_over_dmax & dmax
rep = {}
for r in rows: rep.setdefault((r['family'], r['rtol']), []).append((r['n'], r['gap_over_dmax'], r['d_max'], r['c_max']))
replica_spread = {f"{k[0]}@{k[1]:g}": dict(n_list=[v[0] for v in vs], gap_over_dmax=[round(v[1],4) for v in vs], d_max=[v[2] for v in vs], c_max=[v[3] for v in vs]) for k,vs in sorted(rep.items())}
summary = dict(
  experiment="E-01", source_file=src, source_sha256=sha, campaign_status=d['campaign']['status'],
  code_revision=d['campaign']['code_revision'], n_rows=len(rows),
  statuses={s: sum(1 for r in rows if r['status']==s) for s in sorted(set(r['status'] for r in rows))},
  gap_over_dense_max=summ('gap_over_dmax'),
  clipped_max_over_dense_max=summ('cmax_over_dmax'),
  frac_rows_clipped_lt_half_dense=sum(1 for r in rows if r['cmax_over_dmax'] is not None and r['cmax_over_dmax']<0.5)/len(rows),
  dense_max_over_dense_endpoint=summ('dmax_over_dend'),
  clipped_max_over_clipped_endpoint=summ('cmax_over_cend'),
  dense_max_wrms=summ('d_max'), clipped_max_wrms=summ('c_max'),
  rows_dense_max_wrms_gt_1=sum(1 for r in rows if r['d_max'] and r['d_max']>1), rows_clipped_max_wrms_gt_1=sum(1 for r in rows if r['c_max'] and r['c_max']>1),
  steps=dict(clipped_internal=summ('c_int'), clipped_accepted=summ('c_acc'), clipped_rejected=summ('c_rej'), clipped_output_clipped=summ('c_clip'), dense_internal=summ('d_int'), dense_accepted=summ('d_acc'), dense_rejected=summ('d_rej')),
  work=dict(clipped_rhs=summ('c_rhs'), dense_rhs=summ('d_rhs'), clipped_jvp=summ('c_jvp'), dense_jvp=summ('d_jvp')),
  config=a0['config'],
  reference_uncertainty=summ('ref_unc'),
  wall_total_s=sum((r['c_wall'] or 0)+(r['d_wall'] or 0) for r in rows),
  distinct_family_rtol_pairs=len(rep), replica_spread=replica_spread,
  criterion="Dominated iff gap > 0.1 * dense_max_grid_wrms (classify_output_policy_dominance)",
)
json.dump(summary, open("E-01_summary.json","w"), indent=1)
print(json.dumps({k:v for k,v in summary.items() if k!='replica_spread'}, indent=1))
print("--- replica spread (family@rtol: n_list, gap/dmax) ---")
for k,v in replica_spread.items(): print(k, v['n_list'], v['gap_over_dmax'], "dmax", [f"{x:.3g}" for x in v['d_max']], "cmax", [f"{x:.3g}" for x in v['c_max']])
