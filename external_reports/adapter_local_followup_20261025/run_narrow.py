#!/usr/bin/env python3
"""Sequential, source-locked narrow adapter comparison; all attempts retained."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path('/tmp/vigilode-adapter-local-followup.1PRDjIEs')
BIN = Path('/tmp/vigilode-adapter-local-target-20261025/measurement/examples')
REFERENCE = Path('/tmp/vigilode-adapter-review.rjwwJ4Cn/checkout/research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json')
SOURCE = json.loads((ROOT / 'IDENTITY.json').read_text())
RUNS = ROOT / 'narrow_measurement'
RUNS.mkdir(exist_ok=False)
refsha = hashlib.sha256(REFERENCE.read_bytes()).hexdigest()
base = ['--size','standard','--parallel-layer','serial','--threads','1','--ensemble-count','1','--max-steps','300000','--emit-state','--error-limit','1e-4']
CASES = {
 'chirp_grid': [('chart', ['adapter_phase_grid','--mode','chart','--grid-points','129','--physical-atol','1e-6']), ('physical', ['adapter_phase_grid','--mode','physical','--grid-points','129','--physical-atol','5e-9'])],
 'rotation_switch': [('selected_evidence', ['evidence_switch_bench','rotation','evidence','1e-4']), ('direct_dopri', ['evidence_switch_bench','rotation','dopri','1e-4'])],
 'stiff_switch': [('selected_evidence', ['evidence_switch_bench','stiff-forced','evidence','1e-4']), ('direct_rodas', ['evidence_switch_bench','stiff-forced','rodas','1e-4'])],
 'gyro_endpoint': [('selected_switch', ['local_bottleneck_suite','--case','10','--method','switching',*base,'--rtol','1e-9','--atol','1e-12']), ('direct_dopri', ['local_bottleneck_suite','--case','10','--method','dopri5',*base,'--rtol','1e-9','--atol','1e-12'])],
 'case5_endpoint': [('selected_switch', ['local_bottleneck_suite','--case','5','--method','switching',*base,'--rtol','1e-6','--atol','1e-9','--reference-json',str(REFERENCE),'--reference-sha256',refsha]), ('direct_rodas', ['local_bottleneck_suite','--case','5','--method','rodas5p-dense',*base,'--rtol','1e-6','--atol','1e-9','--reference-json',str(REFERENCE),'--reference-sha256',refsha])],
}
env=os.environ.copy()
for k in ('RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','JULIA_NUM_THREADS'):
 env[k]='1'
env['VIGILODE_SOURCE_COMMIT']=SOURCE['source_commit']
index=[]
for cohort, candidates in CASES.items():
 for block in range(6):
  ordered=candidates if block%2==0 else list(reversed(candidates))
  for label, argv in ordered:
   exe=BIN/argv[0]
   cmd=['/usr/bin/time','-f','elapsed_wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kb=%M','-o',str(RUNS/f'{cohort}_{block}_{label}.resource'),'timeout','90','taskset','-c','0',str(exe),*argv[1:]]
   stem=f'{cohort}_{block}_{label}'
   started=datetime.datetime.now(datetime.timezone.utc).isoformat()
   t0=time.monotonic()
   with (RUNS/f'{stem}.stdout').open('wb') as out, (RUNS/f'{stem}.stderr').open('wb') as err:
    proc=subprocess.run(cmd,stdout=out,stderr=err,env=env)
   wall=time.monotonic()-t0
   row={'cohort':cohort,'candidate':label,'block':block,'warmup':block==0,'order':ordered.index((label,argv)),'command':cmd,'exit_code':proc.returncode,'started_utc':started,'controller_wall_seconds':wall,'source_commit':SOURCE['source_commit'],'source_tree':SOURCE['source_tree'],'binary_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'stdout_sha256':hashlib.sha256((RUNS/f'{stem}.stdout').read_bytes()).hexdigest(),'stderr_sha256':hashlib.sha256((RUNS/f'{stem}.stderr').read_bytes()).hexdigest(),'resource_sha256':hashlib.sha256((RUNS/f'{stem}.resource').read_bytes()).hexdigest(),'thread_env':{k:env[k] for k in ('RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','JULIA_NUM_THREADS')},'affinity':[0]}
   (RUNS/f'{stem}.receipt.json').write_text(json.dumps(row,indent=2)+'\n')
   index.append(row)
   print(stem,proc.returncode,round(wall,4),flush=True)
(RUNS/'INDEX.json').write_text(json.dumps({'schema':'vigilode.adapter.narrow-run.v1','source':SOURCE,'rows':index},indent=2)+'\n')
