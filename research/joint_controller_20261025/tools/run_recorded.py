#!/usr/bin/env python3
"""Run one explicitly supplied command, preserving its identity and failure output."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import time


def source_manifest(root):
    paths = [root / 'Cargo.toml', root / 'Cargo.lock']
    paths += sorted((root / 'crates').rglob('*.rs'))
    paths += sorted((root / 'crates').rglob('Cargo.toml'))
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in paths if p.is_file()}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--output', required=True)
    p.add_argument('--timeout', type=float, default=600)
    p.add_argument('command', nargs=argparse.REMAINDER)
    args = p.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command:
        p.error('a command after -- is required')
    root = Path(__file__).resolve().parents[3]
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    before = source_manifest(root)
    (output / 'SOURCE_INPUTS.json').write_text(json.dumps(before, indent=2) + '\n')
    metadata = {
        'schema': 'vigilode.recorded-command.v1', 'command': command,
        'cwd': str(root), 'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'host': platform.node(), 'platform': platform.platform(), 'logical_cpus': os.cpu_count(),
        'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        'environment': {k: os.environ.get(k) for k in
                        ('CARGO_HOME', 'CARGO_TARGET_DIR', 'CARGO_BUILD_JOBS', 'RUSTFLAGS',
                         'CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG', 'CARGO_INCREMENTAL',
                         'OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS')},
    }
    start = time.monotonic()
    with (output / 'stdout.log').open('wb') as out, (output / 'stderr.log').open('wb') as err:
        try:
            result = subprocess.run(command, cwd=root, stdout=out, stderr=err,
                                    timeout=args.timeout, check=False)
            metadata.update(exit_code=result.returncode, timeout=False)
        except subprocess.TimeoutExpired:
            metadata.update(exit_code=124, timeout=True)
    metadata['wall_seconds'] = time.monotonic() - start
    after = source_manifest(root)
    metadata['source_unchanged_during_command'] = before == after
    metadata['source_changes'] = sorted(k for k in set(before) | set(after)
                                        if before.get(k) != after.get(k))
    metadata['status'] = 'completed' if metadata['exit_code'] == 0 else 'failed'
    for name in ('stdout.log', 'stderr.log', 'SOURCE_INPUTS.json'):
        metadata[name + '_sha256'] = hashlib.sha256((output / name).read_bytes()).hexdigest()
    (output / 'RECEIPT.json').write_text(json.dumps(metadata, indent=2) + '\n')
    print(json.dumps({k: metadata[k] for k in
                     ('status', 'exit_code', 'wall_seconds', 'source_unchanged_during_command')}))
    raise SystemExit(metadata['exit_code'])


if __name__ == '__main__':
    main()
