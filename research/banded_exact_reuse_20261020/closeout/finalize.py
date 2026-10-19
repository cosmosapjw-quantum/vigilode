#!/usr/bin/env python3
"""Verify frozen file identities, summarize executed tests, append one ledger row.
No solver execution and no changes to raw scientific observations.
"""
import csv,hashlib,json,re,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
NODE=ROOT/'research/banded_exact_reuse_20261020'
RUN=NODE/'runs'/sys.argv[1]
def read(name):return json.loads((RUN/name).read_text())
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def dump(name,x):(NODE/name).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
summary=read('SUMMARY.json');audit=read('AUDIT.json');manifest=read('MANIFEST.json')
assert summary['status']=='COMPLETED_BOUNDED' and audit['status']=='PASS_SCOPED_READBACK'
assert summary['accuracy_rows']==432 and summary['parity_pass']==summary['parity_pairs']==324
assert summary['timing_processes']==summary['valid_timing_processes']==576 and summary['complete_comparisons']==36
checked=[]
for line in (RUN/'SHA256SUMS').read_text().splitlines():
    want,path=line.split('  ',1);p=RUN/path;assert p.is_file() and sha(p)==want,path;checked.append(path)
assert read('RECONSTRUCTION.json')==[]
pat=re.compile(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
def tests(name):
    text=(RUN/name).read_text();rows=pat.findall(text)
    return {'completed_targets':len(rows),'passed_in_completed_targets':sum(int(r[1]) for r in rows),
            'failed_in_completed_targets':sum(int(r[2]) for r in rows),'ignored_in_completed_targets':sum(int(r[3]) for r in rows),
            'failure_lines':[s for s in text.splitlines() if s.startswith('error:') or s.strip().endswith(' ... FAILED')],
            'last_lines':text.splitlines()[-12:]}
validation={'regressions':tests('GREEN_REGRESSION.log'),'internal':tests('INTERNAL_TESTS.log'),'workspace':tests('WORKSPACE_TESTS.log'),
    'workspace_status':read('WORKSPACE_STATUS.json'),'workspace_reuse':read('WORKSPACE_REUSE.json'),
    'readback':audit,'verified_files':len(checked),'scope':'scoped validation; full workspace timeout remains NOT_PASS'}
assert validation['regressions']['passed_in_completed_targets']==9 and validation['regressions']['failed_in_completed_targets']==0
assert validation['internal']['passed_in_completed_targets']==17 and validation['internal']['failed_in_completed_targets']==0
source=manifest['candidate_source'];inputs={}
for path in ['crates/rodas5p-integrators/src/rodas5p_fast.rs','crates/rodas5p-integrators/src/exact_band_cache.rs','research/banded_exact_reuse_20261020/bench/src/main.rs']:
    data=subprocess.check_output(['git','show',f'{source}:{path}'],cwd=ROOT)
    assert data==(ROOT/path).read_bytes(),path
    inputs[path]=hashlib.sha256(data).hexdigest()
binding={'baseline_numerical_library_commit':manifest['parent_source'],'candidate_full_source_commit':source,
    'common_driver_origin_commit':source,'driver_sha256':inputs['research/banded_exact_reuse_20261020/bench/src/main.rs'],
    'baseline_context':'isolated historical library worktree with the new common benchmark directory copied into it; not a clean full historical-source build',
    'candidate_context':'same benchmark driver, improvement feature enabled, pinned current numerical implementation',
    'source_sha256':inputs,'binary_sha256':manifest['binary_sha256'],'shared_bench_lock_sha256':sha(RUN/'BENCH_Cargo.lock'),
    'patch_receipt_note':'Earlier SOURCE_PATCH.json recorded pre-rustfmt text; compiled source uses the hashes and commit in this binding.'}
dump('SOURCE_BINDING.json',binding);dump('VALIDATION.json',validation)
pairs=read('PAIRED.json');cost=list(csv.DictReader((RUN/'COST.csv').open()))
report=['# Banded improvement: executed results','',f"Run `{RUN.name}`; candidate `{source}`; parent numerical library `{manifest['parent_source']}`.",'',
    'The baseline uses the common new study driver copied into the historical numerical-library worktree. No older-host times are pooled. Default Indexed/Slices selection and Jacobian/RHS callback frequency are unchanged.','',
    '## Bounded results','', '```json',json.dumps(summary,indent=2),'```','',
    '## Tests and remaining whole-workspace limit','', '```json',json.dumps(validation,indent=2),'```','',
    'The workspace timeout receipt is from run39301232025 and reused only after proving byte identity of all crates and root Cargo files. It is NOT a full-suite PASS.','',
    '## Per-call time and actual factor counts','', '|Case|Grid|Arm|ms|Attempts|LU|Reuse|Jacobian|E_empirical|','|---|---|---|---:|---:|---:|---:|---:|---:|']
for r in cost:report.append('|'+ '|'.join(r[k] for k in ['case','grid','arm','median_ms','attempts','factorizations','lu_reuses','jacobians','E_up'])+'|')
report+=['','## Paired parent/candidate ratios','', 'A ratio above1 means shorter candidate time. Family intervals allocate Bonferroni error across36comparisons;12blocks and shared-host autocorrelation still limit inference.','',
         '|Case|Grid|Candidate|Ratio|Family interval|','|---|---|---|---:|---|']
for p in pairs:report.append(f"|{p['case']}|{p['grid']}|{p['candidate']}|{p['ratio_median']:.5f}|[{p['ci95_family'][0]:.5f}, {p['ci95_family'][1]:.5f}]|")
report+=['','## Preserved negative evidence','',
    'The initial retry-invalidity suspicion was refuted by baseline tests and the complete Err branch; no recovery-policy patch was made.',
    'Four standalone solve regressions were real: nonfinite upper matrix/RHS, unrepresentable solution overflow, and unchecked size arithmetic. These fail on the original and pass after the correction; a tiny but representable solve remains admitted.',
    'The first432benchmark attempts failed before integration because the new driver omitted the derivative capability required by OdeProblem::new. Those are adapter-construction failures, not numerical divergence; the repaired common JVP adapter and preflight smoke were then used for both arms.',
    'This exact cache does not freeze an approximate Jacobian. It requires bit-identical rounded shift and finite Jacobian entries, adds storage/copy checks, and may provide no benefit on changing nonlinear operators. Interpret actual cache hits and costs together.',
    'The empirical reference radius is not a continuous-time certificate. Identical outputs establish no extra observed numerical error relative to the old implementation, not universal accuracy.',
    'No new DVODE/Diffsol comparison, external-client compatibility proof, independent-personnel review or production-default promotion is claimed. BandedKernel gained enum variants, so external exhaustive matches may require updates.']
(NODE/'RESULTS.md').write_text('\n'.join(report)+'\n')
dump('CONCLUSION.json',{'status':'COMPLETE_SCOPED','run':int(RUN.name),'source':source,'baseline':manifest['parent_source'],
    'correctness_regressions':9,'internal_tests':17,'output_parity_pairs':324,'timing_processes':576,
    'default_changed':False,'approximate_jacobian_reuse':False,'whole_workspace_pass':False,
    'workspace_blocker':'timeout660seconds; see VALIDATION.json','promotion':'HOLD_GENERAL_PERFORMANCE','independent_personnel_review':False})
ledger=ROOT/'research/LEDGER.jsonl';raw=ledger.read_bytes();rows=[json.loads(s) for s in raw.splitlines()]
outputs={str((RUN/name).relative_to(ROOT)):sha(RUN/name) for name in ['SUMMARY.json','AUDIT.json','ACCURACY.csv','COST.csv']}
existing=next((r for r in rows if r.get('outputs_sha256')==outputs),None)
if existing is None:
    rid=f"L-{max([int(r['id'][2:]) for r in rows]+[0])+1:04d}"
    entry={'id':rid,'date':'2026-10-20','commit':source,'command':'python3 research/banded_exact_reuse_20261020/scripts/study.py (arguments in run manifest) and scripts/audit.py '+str(RUN.relative_to(ROOT)),
        'profile':'measurement opt3 LTOoff CGU1; shared-host diagnostic','inputs_sha256':inputs,'outputs_sha256':outputs,
        'claim':'Four original standalone band-solve regression failures are corrected and432accuracy rows produce324bit-identical parent/candidate pairs; fixed-width and exact-factor reuse remain opt-in and no global timing or full-workspace PASS is claimed.',
        'verdict':'PASS','supersedes':None}
    assert len(entry['claim'])<=600 and raw.endswith(b'\n')
    with ledger.open('ab') as f:f.write((json.dumps(entry,sort_keys=True)+'\n').encode())
else:entry=existing
assert ledger.read_bytes().startswith(raw)
dump('LEDGER_APPEND_RECEIPT.json',{'row':entry,'original_prefix_sha256':hashlib.sha256(raw).hexdigest(),'append_only':True})
print(json.dumps({'status':'CLOSED_SCOPED','run':RUN.name,'source':source,'ledger_id':entry['id'],'verified_files':len(checked)}))
