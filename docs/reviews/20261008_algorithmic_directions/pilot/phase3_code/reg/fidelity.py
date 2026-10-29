"""Replica fidelity against recorded Rust counters (PROBE B2, EXPLORATORY).
 (1) base arm vs SPD07 BASE.json gmres_into_zero (Brusselator-50/160, rtol 1e-6/1e-8): attempts, JVP vectors, RHS,
     orthogonalisation inner products / vector updates.
 (2) direct arm (exact stage solves) vs the recorded Rust v2 campaign rows (n = 96, rtol 1e-4/1e-6/1e-8; Rust solves
     stages with GMRES(32)/previous start/WRMS heuristic) and vs probe A2's dense-LU replica (no last-reject cap):
     attempts, rejected, endpoint tight-WRMS error.
 (3) base arm vs the Rust campaign MF counters on the corpus (different inner configuration; informative only).
 (4) ROCK4: Bruss-50 957 f-evals @ 1.63e-6 and 2,037 @ 1.41e-8 (repro:BC numbers)."""
import json, math, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpus_v2 as cv

HERE = os.path.dirname(os.path.abspath(__file__))
rows = [json.loads(l) for l in open(os.path.join(HERE, 'res/runs.jsonl'))]
if os.path.exists(os.path.join(HERE, 'res/runs_b64.jsonl')):
    rows += [json.loads(l) for l in open(os.path.join(HERE, 'res/runs_b64.jsonl'))]
R = {(r['problem'], r['arm'], round(math.log10(r['rtol']) * 2) / 2): r for r in rows if r.get('ok')}
out = {}
base = json.load(open('/home/user/wt-speed/research/spd07_mf_step_warm_start_20261007/BASE.json'))
print('(1) base arm vs SPD07 BASE.json gmres_into_zero')
print(f'  {"case":24s} {"att R/py":>10s} {"JVP R/py":>14s} {"RHS R/py":>10s} {"dots R/py":>20s} {"vec upd R / py axpys":>22s}')
o1 = []
for b in base['rows']:
    if 'brusselator' not in b['case']:
        continue
    N = int(b['case'].split('-')[-1]); k = round(math.log10(b['rtol']) * 2) / 2
    r = R.get((f'bruss1d-{N}', 'base', k))
    c = b['gmres_into_zero']['counters']; att = b['gmres_into_zero']['attempts']
    if r is None:
        print(f'  {b["case"]} {b["rtol"]:g}: replica row missing'); continue
    print(f'  {b["case"] + " " + format(b["rtol"], "g"):24s} {att:4d}/{r["att"]:<5d} {c["jvp_vectors"]:6d}/{r["jvp"]:<7d} {c["rhs_evaluations"]:4d}/{r["rhs"]:<5d} '
          f'{c["orthogonalization_inner_products"]:9d}/{r["dots"]:<10d} {c["orthogonalization_vector_updates"]:9d}/{r["axpys"]:<10d}')
    o1.append(dict(case=b['case'], rtol=b['rtol'], rust=dict(att=att, jvp=c['jvp_vectors'], rhs=c['rhs_evaluations'],
                   dots=c['orthogonalization_inner_products'], upd=c['orthogonalization_vector_updates']),
                   replica=dict(att=r['att'], jvp=r['jvp'], rhs=r['rhs'], dots=r['dots'], axpys=r['axpys'])))
out['spd07'] = o1
print('\n(2) direct arm vs Rust v2 campaign rows (n=96) and probe A2 dense replica')
rr = cv.rust_recorded_rows()
a2 = json.load(open('/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/corpus/replica_dense_arm_n96.json'))
short = {v: k for k, v in __import__('pr').CORPUS_SHORT.items()}
o2 = []
print(f'  {"case":44s} {"att Rust/A2/mine":>18s} {"rej Rust/A2/mine":>18s} {"endpoint tight-WRMS Rust / mine":>34s} {"base-arm att/JVP vs Rust MF":>30s}')
for cid, row in rr.items():
    if row['n'] != 96:
        continue
    fam = row['family']; k = round(math.log10(row['rtol']) * 2) / 2
    pn = f'{short[fam]}-96'
    m = R.get((pn, 'direct', k)); bb = R.get((pn, 'base', k))
    d = row['dense']; a = a2.get(cid, {}).get('replica', {})
    if m is None:
        continue
    print(f'  {cid:44s} {d["attempts"]:5d}/{a.get("attempts", -1):4d}/{m["att"]:<5d} {d["rejected"]:5d}/{a.get("rejected", -1):4d}/{m["rej"]:<5d} '
          f'{d["endpoint_wrms"]:14.4g} / {m["err"]:<14.4g} '
          + (f'{bb["att"]:5d}/{bb["jvp"]:<7d} vs {d["attempts"]:5d}/{d["jvp_vectors"]:<7d}' if bb else ''))
    o2.append(dict(case=cid, rust=dict(att=d['attempts'], rej=d['rejected'], endpoint=d['endpoint_wrms'], jvp=d['jvp_vectors']),
                   a2=dict(att=a.get('attempts'), rej=a.get('rejected')), direct=dict(att=m['att'], rej=m['rej'], endpoint=m['err']),
                   base=(dict(att=bb['att'], jvp=bb['jvp'], err=bb['err']) if bb else None)))
out['corpus'] = o2
print('\n(4) ROCK4 Bruss-50 (repro:BC: 957 @ 1.63e-6, 2037 @ 1.41e-8)')
for k in (-6.0, -8.0):
    r = R.get(('bruss1d-50', 'rock4', k))
    if r:
        print(f'  rtol 1e{int(k)}: f-evals (steps + power) = {r["e_f"] + r["e_pow"]}  err = {r["err"]:.3e}')
        out[f'rock4_b50_{int(k)}'] = dict(nf=r['e_f'] + r['e_pow'], err=r['err'])
json.dump(out, open(os.path.join(HERE, 'res/fidelity.json'), 'w'), indent=1, default=float)
