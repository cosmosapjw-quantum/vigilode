import json, hashlib, sys, os, subprocess, statistics
RUN='/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z'
E02=f'{RUN}/exp/E-02'
def sha(p): return hashlib.sha256(open(p,'rb').read()).hexdigest()
new=json.load(open(f'{E02}/out/e0203_rows.json'))
old=json.load(open(f'{RUN}/exp/E-01/calibration_all_cases_compact.json'))
oldmap={r['artifact']['spec']['id']:r['artifact'] for r in old['records']}
def rel(a,b):
    if a==b: return 0.0
    return abs(a-b)/max(abs(a),abs(b),1e-300)
rows=[]; bit_cs=0; bit_gap=0; n=0
for r in new['rows']:
    a=oldmap[r['case_id']]
    arms={x['policy']['name']:x for x in r['arms']}
    row={'case_id':r['case_id'],'family':r['family'],'rtol':r['rtol'],'t_span':r['t_span'],
         'committed_status':a['row']['status'],'rerun_status':r['e02_status'],
         'committed_config':{k:a['config'][k] for k in ('initial_step','max_step','min_step')},
         'rerun_config':{'initial_step':r['base_initial_step'],'max_step':r['base_max_step'],'min_step':1e-12},
         'reference_checksum_match': a['reference']['reference_checksum_sha256']==r['reference_checksum_sha256'],
         'gap':{'committed':a['output_policy_discrepancy_wrms'],'rerun':r['e02_output_policy_discrepancy_wrms']}}
    row['gap']['rel_diff']=rel(row['gap']['committed'],row['gap']['rerun'])
    row['gap']['bitwise']=row['gap']['committed']==row['gap']['rerun']
    bit_gap+=row['gap']['bitwise']
    for mode,arm in (('clipped','clipped_base'),('dense','dense_base')):
        o=a[mode]; nn=arms[arm]; m=nn['metrics']; om=o['metrics']
        d={'checksum_committed':o['output_checksum_sha256'],'checksum_rerun':nn['grid_output_checksum_sha256'],
           'checksum_bitwise':o['output_checksum_sha256']==nn['grid_output_checksum_sha256'],
           'max_grid_wrms':{'committed':om['max_grid_wrms'],'rerun':m['max_grid_wrms'],'rel_diff':rel(om['max_grid_wrms'],m['max_grid_wrms'])},
           'endpoint_wrms':{'committed':om['endpoint_wrms'],'rerun':m['endpoint_wrms'],'rel_diff':rel(om['endpoint_wrms'],m['endpoint_wrms'])},
           'rms_grid_wrms':{'committed':om['rms_grid_wrms'],'rerun':m['rms_grid_wrms'],'rel_diff':rel(om['rms_grid_wrms'],m['rms_grid_wrms'])},
           'steps':{'committed':o['internal_steps'],'rerun':nn['internal_steps']},
           'output_clipped_steps':{'committed':o['output_clipped_steps'],'rerun':nn['output_clipped_steps']},
           'rhs_calls':{'committed':o['counters']['rhs_calls'],'rerun':nn['counters']['rhs_calls']},
           'jvp_calls':{'committed':o['counters']['jvp_calls'],'rerun':nn['counters']['jvp_calls']},
           'accepted_steps':{'committed':o['counters']['accepted_steps'],'rerun':nn['counters']['accepted_steps']},
           'rejected_steps':{'committed':o['counters']['rejected_steps'],'rerun':nn['counters']['rejected_steps']},
           'solver_max_accepted_error_norm':nn['max_accepted_error_norm'],
           'solver_mean_accepted_error_norm':nn['mean_accepted_error_norm'],
           'measured_max_grid_wrms':m['max_grid_wrms'],
           'measured_endpoint_wrms':m['endpoint_wrms'],
           'ratio_measured_max_grid_over_solver_max_norm': m['max_grid_wrms']/nn['max_accepted_error_norm'] if nn['max_accepted_error_norm'] else None,
           'reference_uncertainty_wrms': r['reference_uncertainty_wrms']}
        bit_cs+=d['checksum_bitwise']; n+=1
        row[mode]=d
    rows.append(row)
summary={'rows':len(rows),'arms':n,'arm_checksum_bitwise_equal':bit_cs,'gap_bitwise_equal':bit_gap,
 'status_match':sum(r['committed_status']==r['rerun_status'] for r in rows),
 'config_match':sum(r['committed_config']==r['rerun_config'] for r in rows),
 'reference_checksum_match':sum(r['reference_checksum_match'] for r in rows),
 'max_rel_diff_max_grid_wrms':max(max(r['clipped']['max_grid_wrms']['rel_diff'],r['dense']['max_grid_wrms']['rel_diff']) for r in rows),
 'max_rel_diff_gap':max(r['gap']['rel_diff'] for r in rows),
 'steps_match':sum(r['clipped']['steps']['committed']==r['clipped']['steps']['rerun'] and r['dense']['steps']['committed']==r['dense']['steps']['rerun'] for r in rows),
 'rhs_jvp_match':sum(all(r[m][k]['committed']==r[m][k]['rerun'] for m in ('clipped','dense') for k in ('rhs_calls','jvp_calls')) for r in rows),
 'dense_max_grid_wrms_gt_1':sum(r['dense']['max_grid_wrms']['rerun']>1 for r in rows),
 'clipped_max_grid_wrms_gt_1':sum(r['clipped']['max_grid_wrms']['rerun']>1 for r in rows),
 'dense_solver_max_accepted_norm_le_1':sum(r['dense']['solver_max_accepted_error_norm']<=1.0 for r in rows),
 'clipped_solver_max_accepted_norm_le_1':sum(r['clipped']['solver_max_accepted_error_norm']<=1.0 for r in rows),
 'median_dense_measured_max_grid_over_solver_max_norm':statistics.median(r['dense']['ratio_measured_max_grid_over_solver_max_norm'] for r in rows),
 'median_clipped_measured_max_grid_over_solver_max_norm':statistics.median(r['clipped']['ratio_measured_max_grid_over_solver_max_norm'] for r in rows),
 'median_gap_over_dense_max':statistics.median(r['gap']['rerun']/r['dense']['max_grid_wrms']['rerun'] for r in rows),
 'compiled_revision':new['compiled_revision'],'committed_code_revision':old['records'][0]['artifact']['code_revision'],
 'source_dirty_at_build':new['source_dirty_at_build']}
rustc=subprocess.run(['rustc','--version'],capture_output=True,text=True).stdout.strip()
inputs=[f'{E02}/refdir/reference_manifest.json']+[f'{E02}/refdir/artifacts/{f}' for f in sorted(os.listdir(f'{E02}/refdir/artifacts'))]+[f'{RUN}/exp/E-01/calibration_all_cases_compact.json',f'{RUN}/harness/src/bin/e0203_v2rows.rs']
outputs=[f'{E02}/out/e0203_rows.json',f'{E02}/out/run_stderr.txt']
pf='PASS' if (bit_cs==n and bit_gap==len(rows)) else ('EVIDENCE_ONLY' if summary['max_rel_diff_gap']<1e-6 else 'FAIL')
res={'id':'E-02','purpose':'Re-run the 18 n=96 scientific-validity-v2 calibration rows on the target tree (b3e8165) through the campaign-identical arm code path (execute_arm replicated verbatim; run_scientific_validity_v2_case refuses references whose implementation_revision (ab8fbcd) differs from the compiled revision) and compare per-row metrics, counters and campaign output checksums with the committed campaign records.',
 'commands':[f'{RUN}/bin/cargo-wrapped.sh build --profile measurement --bin e0203_v2rows',
  f'RAYON_NUM_THREADS=1 {RUN}/cargo-target/measurement/e0203_v2rows --manifest {E02}/refdir/reference_manifest.json --out {E02}/out/e0203_rows.json',
  f'python3 {E02}/analyze_e02.py'],
 'inputs':[{'path':p,'sha256':sha(p)} for p in inputs],
 'outputs':[{'path':p,'sha256':sha(p)} for p in outputs],
 'env':{'RAYON_NUM_THREADS':'1','profile':'measurement','rustc':rustc},
 'summary_metrics':summary,'rows':rows,
 'pass_fail':pf,
 'hypothesis_verdicts':{'H1':'EVIDENCE: see summary_metrics (reproduction status) and E-03 for H1a/H1b'},
 'notes':['Code path: execute_arm replicated verbatim (same public entry points integrate_sequential_matrix_free_adaptive_observed / _dense_observed, same LinearSolverConfig, AdaptiveStepConfig, OutputSchedule, wrms_basis.metrics/discrepancy_wrms, classify_* order). run_scientific_validity_v2_case was NOT callable because it refuses references with implementation_revision != compiled revision.',
  'Checksum comparison uses the campaign formula vigilode-scientific-v2-mode-output-v1 over (mode tag, status tag, 101 times, states).',
  'solver_max_accepted_error_norm is the maximum embedded error norm among accepted steps (the controller quantity; <= 1.0 means within tolerance in the local sense). measured_max_grid_wrms is the global WRMS error vs the exact Radau reference on the same basis.']}
json.dump(res,open(f'{E02}/results.json','w'),indent=1)
print(json.dumps(summary,indent=1))
for r in rows:
    print(f"{r['case_id']:70s} st_c={r['committed_status'][:6]}/{r['rerun_status'][:6]} cs_clip={r['clipped']['checksum_bitwise']} cs_dense={r['dense']['checksum_bitwise']} gapbit={r['gap']['bitwise']} gaprel={r['gap']['rel_diff']:.1e} dmax_c={r['dense']['max_grid_wrms']['committed']:.4g} dmax_r={r['dense']['max_grid_wrms']['rerun']:.4g} cmax_r={r['clipped']['max_grid_wrms']['rerun']:.4g} steps={r['clipped']['steps']['rerun']}/{r['dense']['steps']['rerun']} solvmax_d={r['dense']['solver_max_accepted_error_norm']:.3f} solvmax_c={r['clipped']['solver_max_accepted_error_norm']:.3f}")
