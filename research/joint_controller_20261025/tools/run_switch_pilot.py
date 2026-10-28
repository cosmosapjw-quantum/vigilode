#!/usr/bin/env python3
"""A bounded selection study: new policy, previous policy, one direct alternative."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
from run_recorded import source_manifest


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--binary', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    p.add_argument('--repeats', type=int, default=5)
    p.add_argument('--cpu', type=int, default=0)
    args = p.parse_args()
    if not 1 <= args.repeats <= 10:
        p.error('bounded pilot supports 1..10 repetitions')
    root = Path(__file__).resolve().parents[3]
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    before = source_manifest(root)
    cases = [('rotation', 'dopri'), ('decay', 'dopri'), ('stiff-forced', 'rodas'),
             ('case5', 'rodas'), ('transition-47', 'rodas'), ('transition-91', 'rodas'),
             ('relaxation', 'rodas'), ('transition-43', 'rodas'), ('transition-87', 'rodas')]
    metadata = {'schema': 'vigilode.bounded-selection-run.v1', 'cases': cases,
                'repeats': args.repeats, 'warmups_per_arm': 1,
                'binary': str(binary), 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                'cpu_affinity': [args.cpu], 'source_inputs': before, 'runs': [],
                'holdout_note': 'initial study used onsets 0.47/0.91. After the case5 transient-validity correction, add untouched onsets 0.43/0.87; thresholds unchanged. Earlier onsets are regression controls, not fresh holdouts.'}
    env = os.environ.copy()
    for key in ('OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS'):
        env[key] = '1'
    metadata['thread_environment'] = {k: env[k] for k in
                                    ('OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS')}
    started = time.monotonic()
    with (output / 'pilot.jsonl').open('w') as collected:
        for case, direct in cases:
            policies = ['adaptive', 'evidence', direct]
            for rep in range(args.repeats + 1):
                order = policies[rep % 3:] + policies[:rep % 3]
                for policy in order:
                    stem = f'{case}_{policy}_{rep}'
                    command = ['taskset', '-c', str(args.cpu), str(binary), case, policy, str(rep)]
                    t0 = time.monotonic()
                    with (output / (stem + '.stdout')).open('wb') as out, (output / (stem + '.stderr')).open('wb') as err:
                        try:
                            run = subprocess.run(command, cwd=root, env=env, stdout=out, stderr=err,
                                                 timeout=30, check=False)
                            code = run.returncode
                        except subprocess.TimeoutExpired:
                            code = 124
                    receipt = {'command': command, 'exit_code': code,
                               'process_seconds': time.monotonic() - t0,
                               'stdout_sha256': hashlib.sha256((output / (stem + '.stdout')).read_bytes()).hexdigest()}
                    metadata['runs'].append(receipt)
                    if code == 0:
                        text = (output / (stem + '.stdout')).read_text()
                        parsed = json.loads(text)
                        assert parsed['case'] == case and parsed['policy'] == policy and parsed['repetition'] == rep
                        collected.write(json.dumps(parsed, allow_nan=False) + '\n')
                    (output / 'RUN.json').write_text(json.dumps(metadata, indent=2) + '\n')
    metadata['wall_seconds'] = time.monotonic() - started
    metadata['source_unchanged'] = before == source_manifest(root)
    metadata['all_processes_completed'] = all(r['exit_code'] == 0 for r in metadata['runs'])
    (output / 'RUN.json').write_text(json.dumps(metadata, indent=2) + '\n')
    print(json.dumps({k: metadata[k] for k in ('wall_seconds', 'source_unchanged', 'all_processes_completed')}))
    raise SystemExit(0 if metadata['all_processes_completed'] and metadata['source_unchanged'] else 1)


if __name__ == '__main__':
    main()
