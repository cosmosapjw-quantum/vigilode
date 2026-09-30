#!/usr/bin/env python3
"""Reproduce the R3 audit into a new directory; never overwrite its evidence."""
from contextlib import nullcontext
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
SOURCE = 'cc2cd041737e7ff543624d1b59893a3b4397369f'


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def infer_repo(explicit):
    if explicit:
        repo = explicit.resolve()
        if (repo / 'Cargo.toml').is_file() and (repo / 'crates').is_dir():
            return repo
        raise ValueError('--repo must identify the VigilODE checkout')
    for parent in ROOT.parents:
        if (parent / 'Cargo.toml').is_file() and (parent / 'crates').is_dir():
            return parent
    raise ValueError('Cannot infer checkout from this report location; supply --repo')


def check_sources(repo):
    contract = json.loads((ROOT / 'time/SOURCE_CONTRACT.json').read_text())
    hashes = {row['path']: row['sha256'] for row in contract['files']}
    statistics = json.loads((ROOT / 'statistics/SOURCE_HASHES.json').read_text())
    assert contract['source_commit'] == statistics['source_commit'] == SOURCE
    for name, digest in statistics['files'].items():
        if name in hashes and hashes[name] != digest:
            raise ValueError('Conflicting recorded source hashes: ' + name)
        hashes[name] = digest
    for name, expected in hashes.items():
        path = repo / name
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError('Audited source differs: ' + name)
    return {'source_commit': SOURCE, 'verified_source_files': len(hashes)}


def run(command, log_dir, label, records, env=None):
    start = time.monotonic()
    record = {'label': label, 'argv': list(map(str, command)), 'execution': 'STARTED'}
    records.append(record)
    with (log_dir / (label + '.stdout.log')).open('w') as out, \
         (log_dir / (label + '.stderr.log')).open('w') as err:
        completed = subprocess.run(list(map(str, command)), stdout=out, stderr=err, env=env)
    record.update(exit_code=completed.returncode,
                  elapsed_seconds=time.monotonic() - start,
                  execution='COMPLETED' if completed.returncode == 0 else 'FAILED')
    if completed.returncode:
        raise RuntimeError(f'{label} exited {completed.returncode}; see output logs')


def copy_file(relative, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / relative, destination)


def relocate_manifest(path, repo):
    # TOML string encoding is needed if the checkout path includes spaces,
    # quotes or backslashes; no shell interpretation is used.
    text = path.read_text()
    text, count = re.subn(r'"../../../vigilode/(crates/[^"\n]+)"',
                         lambda m: json.dumps(str(repo / m.group(1))), text)
    if count == 0:
        raise ValueError('No expected dependency paths found in ' + str(path))
    path.write_text(text)


def prepare_research(out, independent):
    scripts = ['polynomial/joint_polynomial_probe.py',
               'homotopy/certified_majorant.py', 'homotopy/coefficient_structure.py',
               'statistics/bootstrap_mc_candidate.py']
    if independent:
        scripts += ['decision/independent_math_checks.py',
                    'decision/polynomial_independent_check.py']
    for relative in scripts:
        copy_file(relative, out / relative)
    coefficients = 'homotopy/native_tableau_bits.json'
    expected = json.loads((ROOT / 'homotopy/EXECUTION_RECEIPT.json').read_text())['native_coefficients_sha256']
    if hashlib.sha256((ROOT / coefficients).read_bytes()).hexdigest() != expected:
        raise ValueError('Captured coefficient identity mismatch')
    copy_file(coefficients, out / coefficients)
    py = sys.executable
    commands = [
        ('polynomial', [py, out / scripts[0], '--out', out / 'polynomial']),
        ('homotopy_structure', [py, out / 'homotopy/coefficient_structure.py',
                               '--coefficients', out / coefficients,
                               '--out', out / 'homotopy/coefficient_structure.json']),
        ('homotopy', [py, out / 'homotopy/certified_majorant.py',
                     '--coefficients', out / coefficients,
                     '--target', 'strict-lower-projection', '--out', out / 'homotopy/results.json']),
        ('statistics_mc', [py, out / 'statistics/bootstrap_mc_candidate.py']),
    ]
    if independent:
        commands += [('independent_math', [py, out / 'decision/independent_math_checks.py']),
                     ('independent_polynomial', [py, out / 'decision/polynomial_independent_check.py'])]
    return commands, expected


def prepare_native(out, repo, selected, records):
    lanes = ['time', 'statistics', 'arithmetic'] if selected == 'all' else [selected]
    commands = []
    py = sys.executable
    for lane in lanes:
        if lane in ('time', 'statistics'):
            # Existing wrappers perform source binding and prepare their own
            # fresh child directory. Do not execute their scientific probes here.
            wrapper = ROOT / lane / 'run_probe.py'
            run([py, wrapper, '--repo', repo, '--out', out / lane, '--prepare-only'],
                out, lane + '_prepare', records)
            manifest = out / lane / 'probe/Cargo.toml'
            # Normalize paths safely even on unusual checkout names after the
            # original wrapper's rebasing step.
            source_manifest = (ROOT / lane / 'probe/Cargo.toml').read_text()
            manifest.write_text(source_manifest)
            relocate_manifest(manifest, repo)
            commands.append((lane + '_native', ['cargo', 'run', '--locked', '--offline',
                                               '--jobs', '1', '--manifest-path', manifest]))
            if lane == 'time':
                commands.append(('time_oracle', [py, ROOT / 'time/exact_oracle.py',
                    '--raw', out / 'time_native.stdout.log', '--out', out / 'time/exact_oracle.json']))
        else:
            shutil.copytree(ROOT / 'arithmetic', out / 'arithmetic',
                            ignore=shutil.ignore_patterns('__pycache__', '*.pyc', 'target'))
            for name in ['native.jsonl', 'native.stderr', 'legacy_closure.jsonl',
                         'legacy_closure.stderr', 'tableau_bits.jsonl', 'tableau_bits.stderr',
                         'NATIVE_EXECUTION.json', 'EXACT_ORACLES.json', 'CLOSURE.json']:
                (out / 'arithmetic' / name).unlink(missing_ok=True)
            manifest = out / 'arithmetic/probe/Cargo.toml'
            relocate_manifest(manifest, repo)
            commands += [
                ('arithmetic_build', ['cargo', 'build', '--locked', '--offline', '--jobs', '1',
                                      '--bins', '--manifest-path', manifest]),
                ('arithmetic_native', [py, out / 'arithmetic/run_native.py', '--target', out / 'cargo-target']),
                ('arithmetic_oracle', [py, out / 'arithmetic/exact_oracle.py']),
                ('arithmetic_closure', [py, out / 'arithmetic/analyze_closures.py']),
            ]
    return commands


class BuildLock:
    def __init__(self, path): self.path = path
    def __enter__(self):
        import fcntl
        self.stream = self.path.open('a')
        fcntl.flock(self.stream.fileno(), fcntl.LOCK_EX)
        return self
    def __exit__(self, *args): self.stream.close()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    mode = p.add_mutually_exclusive_group(required=True)
    mode.add_argument('--validate', action='store_true', help='Run root package validator; no science/build')
    mode.add_argument('--research', action='store_true', help='Run polynomial, projected homotopy and MC research')
    mode.add_argument('--native', nargs='?', const='all', choices=['all', 'time', 'statistics', 'arithmetic'],
                      help='Run focused native reproductions; default all, never the workspace suite')
    p.add_argument('--repo', type=Path, help='VigilODE checkout; inferred from report parents when possible')
    p.add_argument('--out', type=Path, help='Required new output directory for research/native')
    p.add_argument('--prepare-only', action='store_true', help='Prepare/source-check only; do not compile or execute science')
    p.add_argument('--independent-checks', action='store_true', help='Also replay the two recorded reviewer-selected Python checks')
    p.add_argument('--lock-file', type=Path, help='Optional shared Linux flock for the sequential native run')
    args = p.parse_args()
    if args.validate:
        if args.prepare_only or args.independent_checks or args.native or args.out or args.lock_file:
            p.error('--validate is read-only; use without execution/output options')
        validator = ROOT / 'validate_review_package.py'
        if not validator.is_file():
            raise SystemExit('BLOCKED: root validate_review_package.py is not present yet')
        raise SystemExit(subprocess.run([sys.executable, str(validator)], cwd=ROOT).returncode)
    if args.out is None:
        p.error('--out is required and must not already exist')
    if args.independent_checks and not args.research:
        p.error('--independent-checks requires --research')
    out = args.out.resolve()
    if out == ROOT or ROOT in out.parents:
        p.error('--out must be outside the published evidence folder')
    source = check_sources(infer_repo(args.repo)) if args.native or args.repo else {
        'source_commit': SOURCE, 'source_binding': 'recorded captured coefficients; no checkout requested'}
    out.mkdir(parents=True, exist_ok=False)
    receipt = {'schema': 'vigilode.r3.reproduction.v1', **source,
               'mode': 'native' if args.native else 'research', 'execution': 'PREPARING',
               'production_modified': False, 'original_evidence_overwritten': False,
               'commands': [], 'planned_commands': []}
    try:
        if args.native:
            commands = prepare_native(out, infer_repo(args.repo), args.native, receipt['commands'])
        else:
            commands, digest = prepare_research(out, args.independent_checks)
            receipt['captured_coefficients_sha256'] = digest
        receipt['planned_commands'] = [{'label': label, 'argv': list(map(str, argv))} for label, argv in commands]
        if args.prepare_only:
            receipt['execution'] = 'PREPARED_ONLY'
        else:
            env = os.environ.copy()
            if args.native:
                env.update(CARGO_TARGET_DIR=str(out / 'cargo-target'), CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='1')
            lock = BuildLock(args.lock_file.resolve()) if args.native and args.lock_file else nullcontext()
            with lock:
                for label, command in commands:
                    run(command, out, label, receipt['commands'], env=env)
            receipt['execution'] = 'COMPLETED'
    except Exception as error:
        receipt.update(execution='FAILED', error=str(error), error_type=type(error).__name__)
        raise
    finally:
        write_json(out / 'REPRODUCTION_RECEIPT.json', receipt)
    print(json.dumps({'execution': receipt['execution'], 'out': str(out),
                      'source_commit': SOURCE}, indent=2))


if __name__ == '__main__':
    main()
