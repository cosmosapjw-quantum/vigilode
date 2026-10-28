#!/usr/bin/env python3
"""Execute only the new sharp-bound contract and non-timed accuracy consumer."""
import datetime, hashlib, json, os, subprocess, time
from pathlib import Path
HERE=Path(__file__).resolve().parent
REPO=HERE.parents[2]
SCRATCH=REPO.parent
env=os.environ.copy()
env.update(PATH=str(SCRATCH/'tmp/rust1941/bin')+os.pathsep+env['PATH'],
    CARGO_HOME=str(SCRATCH/'tmp/cargo_home'),CARGO_TARGET_DIR=str(SCRATCH/'tmp/vigil_target'),
    CARGO_BUILD_JOBS='2',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',RAYON_NUM_THREADS='1')
binary=SCRATCH/'tmp/vigil_target/measurement/examples/structural_bound_bench_20261027'
receipt={'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'commands':[],
    'source_sha256':{p:hashlib.sha256((REPO/p).read_bytes()).hexdigest() for p in [
        'crates/vigilode-coupled/src/structural_mean_field.rs',
        'crates/vigilode-coupled/tests/structural_mean_field_contracts.rs',
        'crates/vigilode-coupled/examples/structural_bound_bench_20261027.rs']}}
commands=[
    ('exact_structural_contract',['cargo','test','--profile','measurement','--locked','--offline','-p','vigilode-coupled','--test','structural_mean_field_contracts','derived_residual_amplification','--','--test-threads=1','--nocapture']),
    ('exact_structural_build',['cargo','build','--profile','measurement','--locked','--offline','-p','vigilode-coupled','--example','structural_bound_bench_20261027']),
    ('exact_structural_diagnostic',[str(binary)])]
for name,cmd in commands:
    start=time.monotonic()
    result=subprocess.run(cmd,cwd=REPO,env=env,capture_output=True,text=True,timeout=180)
    (HERE/(name+'.stdout.log')).write_text(result.stdout)
    (HERE/(name+'.stderr.log')).write_text(result.stderr)
    entry={'name':name,'command':cmd,'exit_code':result.returncode,'elapsed_seconds':time.monotonic()-start}
    receipt['commands'].append(entry)
    (HERE/'EXACT_STRUCTURAL_EXECUTION.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(entry),flush=True)
    if result.returncode:
        print(result.stdout[-3000:]+result.stderr[-3000:],flush=True)
        raise SystemExit(result.returncode)
    if name.endswith('diagnostic'):
        data=json.loads(result.stdout)
        (HERE/'EXACT_STRUCTURAL_DIAGNOSTIC.json').write_text(json.dumps(data,indent=2)+'\n')
receipt['binary_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
receipt['finished_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
(HERE/'EXACT_STRUCTURAL_EXECUTION.json').write_text(json.dumps(receipt,indent=2)+'\n')
