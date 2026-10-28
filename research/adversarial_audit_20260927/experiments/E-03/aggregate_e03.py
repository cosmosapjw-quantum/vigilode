import json, os, glob, hashlib, statistics, subprocess, sys
RUN='/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z'; E='/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z/exp/E-03'
def sha(p): return hashlib.sha256(open(p,'rb').read()).hexdigest()
a=json.load(open(f'{E}/e03a_rows.json')); sci={}
for f in sorted(glob.glob(f'{E}/scipy/*.json')): d=json.load(open(f)); sci[d['family']]=d
complete=len(sci)==6 and all(len(d['rtol_rows'])==3 for d in sci.values()) and os.path.exists(f'{E}/scipy/run_stdout.txt') and open(f'{E}/scipy/run_stdout.txt').read().strip().endswith('DONE')
# --- E-03b scipy pairs
def tally(pairs):
    bc=[p for p in pairs if p['both_reference_admissible']]
    return {'pairs':len(pairs),'both_reference_admissible':len(bc),'violations_among_both_correct':sum(p['violates_0p1_rule'] for p in bc),
            'fraction_violating':(sum(p['violates_0p1_rule'] for p in bc)/len(bc) if bc else None),
            'median_ratio':statistics.median(p['ratio'] for p in pairs) if pairs else None,'min_ratio':min((p['ratio'] for p in pairs),default=None),'max_ratio':max((p['ratio'] for p in pairs),default=None),
            'bitwise_identical':sum(p['bitwise_identical_grid_states'] for p in pairs)}
sp={'A_vs_B_first_step':[], 'A_vs_C_teval':[], 'A_vs_D_maxstep_grid':[]}; sattr=[]; vattr=[]; tight=[]
for fam,d in sci.items():
    tight.append({'family':fam,**{k:v for k,v in d['tight']['L2'].items() if k in ('max_grid_wrms_vs_artifact','bitwise_equal_grid_points','steps','nfev')},'L1_vs_L2':d['tight']['L1']['max_grid_wrms_vs_L2'],'artifact_d1':d['tight']['L1']['artifact_d1']})
    for rr in d['rtol_rows']:
        for p in rr['pairs']:
            key={'B_h0x0.7_dense':'A_vs_B_first_step','C_h0_teval':'A_vs_C_teval','D_h0_maxstep_grid_dense':'A_vs_D_maxstep_grid'}[p['right']]
            sp[key].append({'case_id':rr['case_id'],**p})
        A=rr['arms'].get('A_h0_dense',{})
        if 'accepted_endpoints' in A: sattr.append({'case_id':rr['case_id'],**{k:v for k,v in A['accepted_endpoints'].items()},'endpoint_tf_wrms_tight':A['endpoint_wrms_tight'],'max_grid_wrms_tight':A['max_grid_wrms_tight']})
        v=rr['vigilode_attribution']
        if v: vattr.append({'case_id':rr['case_id'],**{k:v[k] for k in v if k not in ('accepted_wrms_tight','accepted_times')}})
def attr_summary(L):
    if not L: return None
    ratios=[x['ratio_grid_interior_max_over_accepted_max'] for x in L if x.get('ratio_grid_interior_max_over_accepted_max') is not None]
    return {'rows':len(L),'rows_ratio_gt5':sum(r>5 for r in ratios),'median_ratio_grid_interior_max_over_accepted_max':statistics.median(ratios) if ratios else None,'min_ratio':min(ratios) if ratios else None,'max_ratio':max(ratios) if ratios else None,
            'max_ref_interp_uncertainty':max(x['ref_interp_uncertainty_max'] for x in L)}
va=a['summary']
summary={'scipy_run_complete':complete,'families_with_scipy':sorted(sci),
 'E03a_vigilode_same_code_path':{k:va[k] for k in ('h0_pair_refadmissible','maxstep_pair_refadmissible','h0_pair_casetol_correct','maxstep_pair_casetol_correct','clipped_h0_pair','campaign_pair','arms_reference_admissible','arms_total')},
 'E03b_scipy_radau_control':{k:tally(v) for k,v in sp.items()},
 'E03b_tight_rerun_vs_artifact':tight,
 'E03c_vigilode_dense_base_attribution':attr_summary(vattr),
 'E03c_scipy_A_attribution':attr_summary(sattr),
 'E03c_endpoint_tf_vs_max_grid_dense_base':{'median_max_over_tf':va['dense_base_endpoint_vs_max_ratio_median'],'rows_gt5':va['dense_base_max_over_endpoint_gt5'],'rows':18}}
h1a_v=va['h0_pair_refadmissible']['fraction_violating']; h1a_m=va['maxstep_pair_refadmissible']['fraction_violating']
h1a='SUPPORTED' if (h1a_v>=0.8 and h1a_m>=0.8) else ('PARTIAL' if max(h1a_v,h1a_m)>=0.8 else 'REFUTED')
vs=summary['E03c_vigilode_dense_base_attribution']
h1b=('NOT_RUN' if not vs else ('SUPPORTED' if vs['rows_ratio_gt5']>vs['rows']/2 else 'REFUTED'))+('' if complete else ' (partial: %d/18 rows)'%(vs['rows'] if vs else 0))
inputs=[f'{RUN}/exp/E-02/out/e0203_rows.json',f'{RUN}/exp/E-02/refdir/reference_manifest.json']+sorted(glob.glob(f'{RUN}/exp/E-02/refdir/artifacts/*.json'))+[f'{E}/families.py',f'{RUN}/tree/tools/reference_v2/generate_references_v2.py',f'{RUN}/tree/tools/reference_v2/generate_references.py',f'{RUN}/harness/src/bin/e0203_v2rows.rs']
outputs=[f'{E}/e03a_rows.json']+sorted(glob.glob(f'{E}/scipy/*.json'))+[p for p in (f'{E}/scipy/run_stdout.txt',) if os.path.exists(p)]
rustc=subprocess.run(['rustc','--version'],capture_output=True,text=True).stdout.strip()
import scipy, numpy
res={'id':'E-03','purpose':'H1a: does the 0.1 output-policy rule fail for two admissible policies on the SAME dense code path (VigilODE) and for an established solver (scipy Radau)? H1b: is the dense-arm global error dominated by dense-output interpolation (grid points) rather than by the error at accepted step endpoints?',
 'commands':[f'RAYON_NUM_THREADS=1 {RUN}/cargo-target/measurement/e0203_v2rows --manifest {RUN}/exp/E-02/refdir/reference_manifest.json --out {RUN}/exp/E-02/out/e0203_rows.json  (policies clipped_base, dense_base, dense_h0x0.7, dense_maxstep_grid, clipped_h0x0.7, dense_base_union_accepted)',
   f'python3 {E}/analyze_e03a.py', f'OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 python3 {E}/e03bc_scipy.py <six families>', f'python3 {E}/aggregate_e03.py'],
 'inputs':[{'path':p,'sha256':sha(p)} for p in inputs],'outputs':[{'path':p,'sha256':sha(p)} for p in outputs],
 'env':{'RAYON_NUM_THREADS':'1','profile':'measurement','rustc':rustc,'python':sys.version.split()[0],'scipy':scipy.__version__,'numpy':numpy.__version__,'OMP_NUM_THREADS':'1'},
 'summary_metrics':summary,'e03b_pairs':sp,'e03c_vigilode_rows':vattr,'e03c_scipy_rows':sattr,
 'pass_fail':'EVIDENCE_ONLY' if complete else 'EVIDENCE_ONLY (scipy run incomplete)',
 'hypothesis_verdicts':{'H1a':h1a,'H1b':h1b},
 'notes':['Policies: initial_step h0=span/100 (campaign value; =1e-3 for robertson, 1e-2 otherwise) vs 0.7*h0; max_step=span (campaign) vs max_step=grid spacing. Pair gap = max-over-grid WRMS between the two arms in the tight reference basis (abs 1e-10, rel 1e-8, anchored on reference), identical to the campaign discrepancy_wrms (cross-checked against the Rust pair values to 1e-9 rel).',
  '"both-correct" = each arm reference-admissible under the campaign rule (reference_uncertainty_wrms <= 0.1*max_grid_wrms). A stricter reading (max grid error <= 1 in case-tolerance weights) is also tallied (h0_pair_casetol_correct).',
  'scipy arms: A first_step=h0 dense_output; B first_step=0.7h0 dense_output; C first_step=h0 t_eval=grid (scipy evaluates t_eval from the same dense interpolant, so C==A bitwise is expected); D first_step=h0 max_step=grid spacing dense_output. Radau with analytic sparse Jacobian, atol=0.01*rtol.',
  'E-03c reference at accepted-step times: scipy Radau rtol 1e-12/atol 1e-14 dense interpolant (L2, whose t_eval grid states are bitwise identical to the committed artifact in every family checked); interpolant uncertainty bounded by L1(1e-10/1e-12) vs L2 disagreement at the same times (ref_interp_uncertainty_max). VigilODE accepted-step states are exact: they come from a union-schedule dense run (grid + accepted times) that reproduced the identical accepted step sequence; the dense collector returns y_new at theta==1.',
  'Grid-interior = grid points not coinciding with an accepted step endpoint (these are the interpolated outputs); accepted set includes t_f.']}
json.dump(res,open(f'{E}/results.json','w'),indent=1)
print(json.dumps(summary,indent=1)); print('verdicts',res['hypothesis_verdicts'])
