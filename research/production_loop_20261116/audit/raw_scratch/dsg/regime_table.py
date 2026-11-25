#!/usr/bin/env python3
"""Regime-coverage table: every row's n is read from a result/corpus/log file or decoded from the problem id
by a stated rule; the categorical columns (jacobian, backend, rhs_dominates, operator) are the auditor's reading
of the cited source and are labelled as classifications."""
import json, re, collections
W='/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/research/'; S='/home/cosmosapjw/vigilode/'
led={json.loads(l)['id']:json.loads(l) for l in open(W+'LEDGER.jsonl') if l.strip()}
corp={}
for f in ['bench/out/adversarial_rust_results.json','bench/out/adversarial_spatial_battery_v3_results.json']:
    for p in json.load(open(S+f))['corpus']['problems']: corp[p['id']]=p
def n_of(pid):
    if pid in corp: return corp[pid]['n']
    m=re.match(r'brusselator-1d-(\d+)$',pid)
    if m: return 2*int(m.group(1))
    m=re.match(r'brusselator-(\d+)-cells$',pid)
    if m: return 2*int(m.group(1))
    m=re.match(r'burgers-central-(\d+)$',pid)
    if m: return int(m.group(1))
    m=re.search(r'_(\d+)(_|$)',pid)
    if m: return int(m.group(1))
    return None
rows=[]
def add(src,ledger,kind,problem,n,jac,backend,operator,rhs_dom,stiff=True,note=''):
    rows.append({'source':src,'ledger':ledger,'evaluation':kind,'problem':problem,'n':n,'jacobian':jac,'linear_backend':backend,
                 'operator':operator,'rhs_cost_dominates':rhs_dom,'stiff':stiff,'note':note})
# A. L-0029 native work-precision (matched error, wall)
d=json.load(open(W+'stiff_native_benchmark_20261001/ANALYSIS.json'))
for p in d['matched_error']:
    add('research/stiff_native_benchmark_20261001/ANALYSIS.json','L-0029','matched-error wall vs CVODE/RADAU5/RODAS',p,n_of(p),'dense analytic','dense LU (faer)','0D ODE' if not p.startswith('bruss') else '1D MOL',False)
# B. switcher line cross-solver corpus (adversarial-cross-solver-v1; e1 corpus is a subset)
d=json.load(open(S+'bench/out/adversarial_rust_results.json'))
for p in d['corpus']['problems']:
    add('bench/out/adversarial_rust_results.json (switcher line 93392249)','L-0139/L-0142 lineage','matched-error wall vs DVODE/CVODE/Diffsol/Hairer (grid-clipped and dense bases)',p['id'],p['n'],'dense or banded analytic','dense LU / banded LU','1D MOL' if p['banded'] else '0D ODE',False)
# C. PDE battery v3 (switcher line)
d=json.load(open(S+'bench/out/adversarial_spatial_battery_v3_results.json'))
mc=collections.Counter((m['problem'],m['status']) for m in d['matched'])
for p in d['corpus']['problems']:
    op='3D MOL' if '3d' in p['id'] else '2D MOL'
    add('bench/out/adversarial_spatial_battery_v3_results.json (switcher line)','none (no ledger row cites this file)','matched-error wall, PDE battery v3',p['id'],p['n'],'dense or banded analytic','dense LU / banded LU',op,False,
        note=f"MATCHED levels {mc[(p['id'],'MATCHED')]}, DIAGNOSTIC_ONLY {mc[(p['id'],'DIAGNOSTIC_ONLY')]}")
# D. U11
src=open(W+'production_unification_20261115/production_holdout/runner/src/main.rs').read()
bn=int(re.search(r'const BN: usize = (\d+);',src).group(1))
u11={'hires':int(re.search(r'name: "hires",\s*n: (\d+)',src).group(1)),'oregonator':int(re.search(r'name: "oregonator",\s*n: (\d+)',src).group(1)),'brusselator_mol32':2*bn}
for p,n in u11.items():
    add('research/production_unification_20261115/production_holdout','none (U nodes have no ledger rows)','matched-error steps and wall, PPSW1 vs RODAS5P',p,n,'dense analytic','dense LU','1D MOL' if p.startswith('bruss') else '0D ODE',False)
# E. U04
raw=None
for line in open(W+'production_unification_20261115/matrix_free_w/logs/u04_crossover.log'):
    if 'QUAL {' in line:
        dd=json.loads(line[line.index('QUAL ')+5:])
        if dd['test']=='c5_crossover': raw=dd['data']
for n in sorted({r['n'] for r in raw}):
    add('research/production_unification_20261115/matrix_free_w','none','wall, PPSW1 matrix-free vs PPSW1 dense (same method)','fisher-1d',n,'JVP (MF) / dense analytic (comparator)','GMRES identity preconditioner / dense LU','1D MOL',False)
# F. INT-03 banded operation counts
d=json.load(open(W+'int03_banded_fast_20261003/RESULTS.json'))
for c in d['cases']:
    add('research/int03_banded_fast_20261003/RESULTS.json','L-0054','operation counts, banded vs dense RODAS5P (bitwise parity, no error axis, no timing)',c['case'],c['n'],'banded analytic','banded LU','1D MOL',False)
# G. SP03 routing / SPD-09 thresholds (instruction counts)
for f,lid,kind in (('sp03_declared_structure_routing_20261010/RESULTS.json','L-0101','instructions per attempt, routed banded vs dense'),('spd09_colext_threshold_20261007/RESULTS.json','L-0086','instructions per attempt, column-extent LU')):
    d=json.load(open(W+f)); dims=set()
    def walk(o):
        if isinstance(o,dict):
            for k,v in o.items():
                if k in('n','dimension') and isinstance(v,int): dims.add(v)
                walk(v)
        elif isinstance(o,list):
            for v in o: walk(v)
    walk(d)
    for n in sorted(dims):
        add('research/'+f,lid,kind,'brusselator-1d or small ODE (id by n)',n,'dense/banded analytic','in-place dense LU / banded LU','1D MOL' if n>=60 else '0D ODE',False)
# H. lagged LU kill gate
r=led['L-0141']; ns=re.findall(r'Brusselator n = (\d+)',r['claim'])
add('research/b1c_staleness_20261021/results.json','L-0141','factorizations per step at matched steps and error (lagged LU)','hires',corp['hires']['n'],'dense analytic','dense LU reused + GMRES','0D ODE',False)
add('research/b1c_staleness_20261021/results.json','L-0141','factorizations per step at matched steps and error (lagged LU)','brusselator-1d',int(ns[0]),'dense analytic','dense LU reused + GMRES','1D MOL',False)
# I. R7 / C5 native wall at matched error
for f,lid,kind,key in (('vigilode_r7_native_transactional_discriminant_20261101/RESULTS.json','L-0200','matched-error wall, transactional q1/q2 vs protected','all_source_and_method_records'),
                       ('vigilode_c5_native_graph_schur_20261102/RESULTS.json','L-0204','wall at same error, graph-Schur vs fast direct','original_native_source_and_full_output_rows')):
    d=json.load(open(W+f))
    for s in sorted({r['source'] for r in d[key]}):
        add('research/'+f,lid,kind,s,n_of(s),'JVP / source-owned structure','matrix-free or graph Schur vs dense LU','0D ODE',False,note='n decoded from the model id' if n_of(s) else 'n not encoded in id (small source model)')
# J. non-stiff external adapters
d=json.load(open(W+'nonstiff_external_adapters_20261023/RESULTS.json'))
for fam in d['families']:
    add('research/nonstiff_external_adapters_20261023/RESULTS.json','L-0165','fastest configuration under a common achieved-error cap (DOPRI5 vs RODAS/VODE/Diffsol/Julia)',fam['case'],fam['dimension'],'none (explicit) / dense for comparators','none / dense LU','0D ODE or vector field',False,stiff=False)
# K. derivative-free causal/structural lanes
d=json.load(open(W+'derivative_free_causal_homotopy_20261027/RESULTS.json'))
for s in d['structural']['summary']:
    add('research/derivative_free_causal_homotopy_20261027/RESULTS.json','none','paired wall ratio, structural quotient lane',f"structural-{s['candidate']}-threads{s['threads']}",s['n'],'none (jacobian_evaluations 0)','scalar/structural','separable source model',False)
# L. controller-copy microbenchmark
d=json.load(open(W+'dynamics_symbolic_validation_20261028/RESULTS.json'))
for c in d['controller_performance']:
    if 'dimension' in c: add('research/dynamics_symbolic_validation_20261028/RESULTS.json','L-0183','paired wall ratio, controller state copies',c['scenario'],c['dimension'],'n/a','n/a','controller microbenchmark',False,stiff=False)
tab=rows
def cnt(pred): return sum(1 for r in tab if pred(r))
summary={'evaluations':len(tab),
 'n_known':cnt(lambda r:r['n'] is not None),
 'n_gt_1000':cnt(lambda r:(r['n'] or 0)>1000),
 'n_gt_1000_by_kind':dict(collections.Counter(r['evaluation'] for r in tab if (r['n'] or 0)>1000)),
 'n_gt_1000_stiff_cross_method_matched_error':cnt(lambda r:(r['n'] or 0)>1000 and r['stiff'] and r['evaluation'].startswith('matched-error')),
 'n_le_64':cnt(lambda r:r['n'] is not None and r['n']<=64),'n_65_to_400':cnt(lambda r:r['n'] is not None and 64<r['n']<=400),'n_401_to_1000':cnt(lambda r:r['n'] is not None and 400<r['n']<=1000),
 'operators_2d_or_3d':cnt(lambda r:r['operator'] in('2D MOL','3D MOL')),
 'operators_2d_or_3d_max_n':max([r['n'] for r in tab if r['operator'] in('2D MOL','3D MOL')] or [0]),
 'operators_2d_or_3d_on_loop_base_074a5699':cnt(lambda r:r['operator'] in('2D MOL','3D MOL') and 'switcher' not in r['source']),
 'rhs_cost_dominates':cnt(lambda r:r['rhs_cost_dominates']),
 'matched_error_cross_method':cnt(lambda r:r['evaluation'].startswith('matched-error')),
 'matched_error_cross_method_max_n':max(r['n'] for r in tab if r['evaluation'].startswith('matched-error') and r['n']),
 'preconditioned_krylov_arms':0,
 'callback_share_evidence':led['L-0113']['claim'][:330]}
json.dump({'summary':summary,'rows':tab},open('/tmp/claude-1000/dsg-audit/regime_table.json','w'),indent=1,allow_nan=False)
print(json.dumps(summary,indent=1))
for r in tab: print(r['ledger'][:12].ljust(12), str(r['n']).rjust(6), r['operator'][:14].ljust(14), r['problem'][:36].ljust(36), r['evaluation'][:60])
