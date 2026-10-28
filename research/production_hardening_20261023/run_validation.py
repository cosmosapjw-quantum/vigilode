#!/usr/bin/env python3
"""Run source-pinned targeted validation; preserve every command and failure."""
import argparse
import datetime
import hashlib
import json
import pathlib
import platform
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--source-commit', required=True)
parser.add_argument('--out', required=True)
args = parser.parse_args()
def git(*a):
    return subprocess.check_output(['git', *a], text=True).strip()
if git('rev-parse', 'HEAD') != args.source_commit:
    raise SystemExit('HEAD differs from source')
if subprocess.check_output(['git', 'diff', 'HEAD', '--', 'crates', 'Cargo.lock', 'Cargo.toml', 'research/production_hardening_20261023'], text=True):
    raise SystemExit('tracked source differs from published commit')
out = pathlib.Path(args.out)
out.mkdir(parents=True, exist_ok=False)
commands = [
 ['cargo','test','--locked','--offline','-p','rodas5p-integrators','--test','dopri5_contracts','--test','dopri5_allocation','--test','dopri5_dense_contracts','--test','affine_error_contracts','--test','dopri5_extreme_contracts'],
 ['cargo','test','--locked','--offline','-p','vigilode','--test','explicit_contracts','--test','explicit_ensemble_contracts','--test','explicit_hardening_contracts'],
 ['cargo','test','--locked','--offline','-p','rodas5p-integrators','--lib','adaptive_output::tests'],
 ['cargo','test','--locked','--offline','-p','rodas5p-integrators','--lib','dopri5_dense::tests'],
 ['cargo','test','--locked','--offline','-p','vigilode-coupled','--test','adaptive_trajectory'],
 ['python3','research/production_hardening_20261023/test_bench_protocol.py'],
]
summary = {'source_commit': args.source_commit, 'source_tree':git('rev-parse','HEAD^{tree}'),
 'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'platform':platform.platform(), 'commands':[], 'ncp_used':False}
for i, command in enumerate(commands):
    start = time.monotonic()
    with (out / f'{i:02d}.stdout.log').open('wb') as stdout, (out / f'{i:02d}.stderr.log').open('wb') as stderr:
        result = subprocess.run(command, stdout=stdout, stderr=stderr, timeout=900)
    row = {'command':command, 'exit_code':result.returncode, 'elapsed_seconds':time.monotonic()-start,
           'stdout':f'{i:02d}.stdout.log', 'stderr':f'{i:02d}.stderr.log'}
    for key in ['stdout','stderr']:
        row[key+'_sha256']=hashlib.sha256((out/row[key]).read_bytes()).hexdigest()
    summary['commands'].append(row)
    (out/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(row),flush=True)
summary['all_commands_passed']=all(r['exit_code']==0 for r in summary['commands'])
summary['finished_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
(out/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
raise SystemExit(0 if summary['all_commands_passed'] else 1)
