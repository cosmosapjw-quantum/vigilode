"""Proposed single-invocation runner. Requires an explicit root GO JSON.

Only standard library is imported here. Science executes exclusively in the
bounded child after the GO check and exclusive first-invocation marker.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
UNIT = 'V8_SOURCE_CHEMICAL_JACOBIAN_REMAINDER_20261028'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('kind',choices=('suite','figure'))
    parser.add_argument('--go',required=True,type=Path)
    args = parser.parse_args()
    go_bytes = args.go.read_bytes()
    go = json.loads(go_bytes)
    if go.get('status') != 'GO' or go.get('unit_id') != UNIT:
        raise ValueError('EXPLICIT_MATCHING_ROOT_GO_REQUIRED')
    budget = json.loads((ROOT/'EXECUTION_BUDGET_FROZEN.json').read_text())
    key = 'primary_suite' if args.kind == 'suite' else 'saved_results_figure'
    cap = budget[key]
    if cap['max_invocations'] != 1:
        raise ValueError('FROZEN_SINGLE_INVOCATION_REQUIRED')
    total = budget['local_scientific_total_caps']
    if sum(budget[k]['cpu_cap_s'] for k in ('primary_suite','saved_results_figure')) > total['CPU_s'] or sum(budget[k]['wall_cap_s'] for k in ('primary_suite','saved_results_figure')) > total['wall_s']:
        raise ValueError('PROCESS_CAPS_EXCEED_TOTAL_NEW_UNIT_ALLOCATION')
    if args.kind == 'figure':
        previous = json.loads((ROOT/'results/suite_first.measurement.json').read_text())
        if previous['status'] != 'PASS':
            raise ValueError('FIRST_PRIMARY_SUITE_PASS_REQUIRED')
    target = ROOT/'results'
    target.mkdir(exist_ok=True)
    started = {'unit_id':UNIT,'kind':args.kind,'status':'STARTED',
               'utc':datetime.now(timezone.utc).isoformat(),
               'GO_path':str(args.go.resolve()),'GO_sha256':hashlib.sha256(go_bytes).hexdigest(),
               'caps':cap,'requested_execution_model':'gpt-6.1-sol',
               'observed_execution_model':'UNOBSERVED_RUNTIME_METADATA'}
    with (target/f'{args.kind}_invocation_started.json').open('x') as stream:
        json.dump(started,stream,indent=2)
        stream.write('\n')
    first = target/f'{args.kind}_first_source'
    first.mkdir()
    source_files = [ROOT/'UNIT_CONTRACT.md',ROOT/'EXECUTION_BUDGET_FROZEN.json',
                    *sorted(p for dirname in ('code','tests','deps','theory')
                            for p in (ROOT/dirname).rglob('*') if p.is_file())]
    sources = []
    for path in source_files:
        dest = first/path.relative_to(ROOT)
        dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(path,dest)
        sources.append({'path':str(path.relative_to(ROOT)),
                        'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
    with (first/'source_manifest.json').open('x') as stream:
        json.dump(sources,stream,indent=2)
        stream.write('\n')
    script = ROOT/'tests/test_chemical_polynomial.py' if args.kind == 'suite' else ROOT/'code/plot_saved_results.py'
    command = [sys.executable,'-B',str(script)]

    def child_limits():
        resource.setrlimit(resource.RLIMIT_CPU,(cap['cpu_cap_s'],cap['cpu_cap_s']))
        resource.setrlimit(resource.RLIMIT_AS,(cap['memory_bytes'],cap['memory_bytes']))
        resource.setrlimit(resource.RLIMIT_CORE,(0,0))

    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.monotonic()
    timed_out = False
    env = dict(os.environ,PYTHONDONTWRITEBYTECODE='1',OPENBLAS_NUM_THREADS='1',
               OMP_NUM_THREADS='1',MPLCONFIGDIR='/tmp/vigilode_next8_mpl')
    with (target/f'{args.kind}_first.stdout.txt').open('xb') as stdout, \
         (target/f'{args.kind}_first.stderr.txt').open('xb') as stderr:
        try:
            completed = subprocess.run(command,cwd=ROOT,env=env,stdout=stdout,stderr=stderr,
                          timeout=cap['wall_cap_s'],preexec_fn=child_limits,check=False)
            code = completed.returncode
        except subprocess.TimeoutExpired:
            timed_out = True
            code = -9
    wall = time.monotonic()-start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    measurement = {'unit_id':UNIT,'kind':args.kind,'invocation':1,'command':command,
                   'returncode':code,'timed_out':timed_out,'wall_s':wall,
                   'child_user_cpu_s':after.ru_utime-before.ru_utime,
                   'child_system_cpu_s':after.ru_stime-before.ru_stime,
                   'child_maxrss_kib':after.ru_maxrss,
                   'status':'PASS' if code == 0 else 'FAIL_PRESERVED_NO_RETRY',
                   'physical_calls_allowed':budget['physical_calls_allowed'],
                   'first_source_and_raw_logs_preserved':True}
    with (target/f'{args.kind}_first.measurement.json').open('x') as stream:
        json.dump(measurement,stream,indent=2)
        stream.write('\n')
    print(json.dumps(measurement,indent=2))
    return 0 if code == 0 else 1


if __name__ == '__main__':
    sys.exit(main())
