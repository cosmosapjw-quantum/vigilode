#!/usr/bin/env python3
"""Run the changed-dependency test matrix and freeze its source identity.
Run from a configured Rust1.94.1 offline environment. No timing promotion.
"""
import argparse, hashlib, json, os, platform, re, subprocess, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def source_manifest():
    names = subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=ROOT).decode().split('\0')
    exact = {'Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo/config.toml','.github/workflows/production.yml',
             'research/production_reaudit_20261021/NCP_CONFIG.json'}
    paths = sorted({p for p in names if p and (p in exact or p.startswith(('crates/','fixtures/','tools/')) or (p.startswith('research/production_reaudit_20261021/') and p.endswith('.py'))) and (ROOT/p).is_file()})
    return {'schema':'vigilode.production_reaudit.source.v1','files':[{'path':p,'sha256':sha(ROOT/p)} for p in paths], 'scope':'tracked/unignored crates, fixtures, tools, pinned build files, audit programs; external runtime described separately'}
def tests(package, names, lib_filter=None):
    a=['cargo','test','-p',package,'--locked','--offline','-j','1']
    for name in names: a += ['--test',name]
    if lib_filter is not None: a += ['--lib'] + ([lib_filter] if lib_filter else [])
    return a+['--','--test-threads=1']
def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--output',required=True)
    ap.add_argument('--target-dir',required=True); ap.add_argument('--clippy-target-dir',required=True); args=ap.parse_args()
    native_target=Path(args.target_dir).resolve(); clippy_target=Path(args.clippy_target_dir).resolve()
    if native_target == clippy_target:
        ap.error('Clippy requires a distinct target directory')
    out=ROOT/args.output; out.mkdir(parents=True,exist_ok=False)
    before=source_manifest(); source_file=HERE/'SOURCE_MANIFEST.json'
    source_file.write_text(json.dumps(before,indent=2)+'\n'); source_sha=sha(source_file)
    (out/'SOURCE_BEFORE.json').write_text(source_file.read_text())
    groups=[
      ('core',tests('rodas5p-core',['leja_enclosed_contract','guarded_polynomial_router'])),
      ('krylov',tests('rodas5p-krylov',['reaudit_monitored_progress','rnext02_gmres_into_contracts','workspace_contracts'])),
      ('ported',tests('rodas5p-integrators',['all_dag_a01_packet','production_audit_packet_adaptive','production_audit_packet_verified','production_audit_damped_packet','all_dag_a05_verified','all_dag_a06_direct','collective_rodas5p_allocation_contract','nonlinear_homotopy_contracts','actual_k_homotopy_contracts','all_dag_a10_bernstein','all_dag_a11_verified','production_audit_packet_curve_output','production_audit_pr101_provider','independent_pr101_review','production_audit_pr101_cloud'])),
      ('compatibility',tests('rodas5p-integrators',['production_audit_packet_center_displacement','packet_trajectory_contracts'])),
      ('homotopy_unit',tests('rodas5p-integrators',[],'nonlinear_homotopy::tests')),
      ('dense_transaction',tests('rodas5p-integrators',['reaudit_admission_overflow','c1_transaction_contracts','sequential_estimate_failure_contracts','p0_partial_contracts','dense_wfiltered_contracts','a1_zero_tolerance_scale'])),
      ('lagged',tests('rodas5p-integrators',['reaudit_staleness_guards','reaudit_lagged_failure_work','b1_lagged_contracts','b1b_gmres_contracts','b1c_staleness_contracts','b1d_resolvent_contracts'])),
      ('global_budget',tests('rodas5p-integrators',['global_budget_contracts'])),
      ('n05_stage_target',tests('rodas5p-integrators',[],'stage_target_policy::n05_tests')),
      ('n05_diagonal',tests('rodas5p-integrators',['n05_diagonal_staleness_contracts'])),
      ('comparison_clients',tests('rodas5p-integrators',['reaudit_comparison_contract','reaudit_bound_clients_contract'])),
      ('facade',tests('vigilode',['events_contracts','p0_contracts','p0_api_contracts','facade_contracts','records_contracts','capabilities_contracts'],'')),
      ('core_leja_doc',['cargo','test','-p','rodas5p-core','--locked','--offline','-j','1','--doc','AdmittedLejaFused']),
      ('core_polynomial_doc',['cargo','test','-p','rodas5p-core','--locked','--offline','-j','1','--doc','GuardedPolynomialAction']),
      ('facade_doc',['cargo','test','-p','vigilode','--locked','--offline','-j','1','--doc']),
      ('examples',['cargo','check','-p','rodas5p-integrators','--locked','--offline','-j','1','--example','component_benchmark_native','--example','reaudit_comparison','--example','reaudit_bound_clients']),
      ('fmt',['cargo','fmt','--all','--check']),
      ('clippy',['cargo','clippy','-p','vigilode','-p','rodas5p-core','--all-targets','--locked','--offline','--no-deps','-j','1','--','-D','warnings']),
      ('python_contracts',[sys.executable,'-m','pytest','-q','tools/tests']),
    ]
    rows=[]
    for name,argv in groups:
        start=time.monotonic(); p=out/(name+'.log')
        env=dict(os.environ); env['CARGO_TARGET_DIR']=str(clippy_target if name=='clippy' else native_target)
        env['CARGO_BUILD_JOBS']='1'; env['RUST_TEST_THREADS']='1'; env['PYTHONDONTWRITEBYTECODE']='1'
        with p.open('w') as f: result=subprocess.run(argv,cwd=ROOT,env=env,stdout=f,stderr=subprocess.STDOUT)
        text=p.read_text(); counts=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
        row={'id':name,'kind':'python' if name=='python_contracts' else ('doctest' if 'doc' in name else 'native'),
             'argv':argv,'exit_code':result.returncode,'elapsed_seconds_diagnostic':time.monotonic()-start,
             'passed':sum(int(x[0]) for x in counts),'failed':sum(int(x[1]) for x in counts),'ignored':sum(int(x[2]) for x in counts),
             'log':str(p.relative_to(ROOT)),'log_sha256':sha(p),'target_dir':env['CARGO_TARGET_DIR']}
        if name=='python_contracts':
            summary=next((line for line in reversed(text.splitlines()) if re.search(r'\d+ (?:passed|failed|error|skipped)',line)), '')
            for field,word in [('passed','passed'),('failed','failed'),('skipped','skipped'),('errors','error')]:
                match=re.search(r'(\d+) '+word,summary); row[field]=int(match.group(1)) if match else 0
        rows.append(row); (out/'PROGRESS.json').write_text(json.dumps(rows,indent=2)+'\n')
        print(json.dumps({k:row[k] for k in ['id','exit_code','passed','failed','ignored']}),flush=True)
    after=source_manifest(); after_file=out/'SOURCE_AFTER.json'; after_file.write_text(json.dumps(after,indent=2)+'\n')
    same=before==after
    execution={'schema':'vigilode.production_reaudit.execution.v1','toolchain':subprocess.check_output(['rustc','--version'],text=True).strip(),'cargo':subprocess.check_output(['cargo','--version'],text=True).strip(),'platform':platform.platform(),'target_dir':str(native_target),'clippy_target_dir':str(clippy_target),'source_manifest_before_sha256':source_sha,'source_manifest_after_sha256':sha(after_file),'source_unchanged':same,'status':'PASS_TARGETED' if same and all(r['exit_code']==0 for r in rows) else 'FAIL','commands':rows,'native_tests_passed':sum(r['passed'] for r in rows if r['kind']=='native'),'doc_tests_passed':sum(r['passed'] for r in rows if r['kind']=='doctest'),'python_tests_passed':sum(r['passed'] for r in rows if r['kind']=='python'),'python_tests_skipped':sum(r.get('skipped',0) for r in rows if r['kind']=='python'),'ignored_tests':sum(r['ignored'] for r in rows if r['kind']!='python'),'not_run':['full workspace suite','full native benchmark crate: exact dependencies and native toolchain missing','controlled-host speed campaign'],'reused_execution_reports':['facade.json','integrator.json','dense_transaction.json','benchmark.json','evidence/port/PORT.json'],'scope':'Changed-dependency native/compatibility matrix plus Python contracts. Does not promote global accuracy or timing.'}
    (out/'EXECUTION.json').write_text(json.dumps(execution,indent=2)+'\n')
    (HERE/'EXECUTION.json').write_text(json.dumps(execution,indent=2)+'\n')
    return execution['status']!='PASS_TARGETED'
if __name__=='__main__': raise SystemExit(main())
