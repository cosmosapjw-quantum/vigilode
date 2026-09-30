#!/usr/bin/env python3
"""Relocatable, source-gated reproduction runner (Python standard library).

Native probes use the caller's Rust/offline Cargo environment. Scientific Python
scripts additionally require NumPy/SciPy. No original probe, evidence or repo file
is modified; all builds and generated results live in a fresh --out directory.
"""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import tomllib

HERE = Path(__file__).resolve().parent
NATIVE = {
    'phi': ('phi/probe', ['vigilode-r2-phi-independent-probe']),
    'output': ('output_cert/probe', ['vigilode-output-cert-reaudit', 'safe_budget']),
    'statistics': ('statistics/probe', ['vigilode-statistics-reaudit']),
}
MATH = [
    ('polynomial_joint', 'research/polynomial_joint_probe.py', []),
    ('homotopy_majorant', 'research/homotopy_majorant_probe.py', ['--tableau']),
    ('telemetry_covariance', 'owner/telemetry_covariance.py', []),
    ('output_exact_oracle', 'output_cert/derive_exact_oracles.py', []),
    ('statistics_exact_bootstrap', 'statistics/exact_bootstrap_oracle.py', []),
]
TABLEAU = 'fixtures/rodas5p_coefficients_snapshot.json'


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + '\n')


def checked_source(repo: Path) -> dict:
    manifest_path = HERE / 'evidence/source/SOURCE_FILE_HASHES.json'
    manifest = json.loads(manifest_path.read_text())
    files = manifest.get('files')
    if not isinstance(files, dict) or not files:
        raise ValueError('SOURCE_FILE_HASHES.json must contain a nonempty files map')
    mismatches = []
    for relative, expected in files.items():
        relative_path = Path(relative)
        if relative_path.is_absolute() or '..' in relative_path.parts:
            raise ValueError(f'Unsafe source manifest path: {relative}')
        path = (repo / relative_path).resolve()
        if not path.is_relative_to(repo):
            raise ValueError(f'Source path escapes repository: {relative}')
        if not isinstance(expected, str) or not re.fullmatch(r'[0-9a-fA-F]{64}', expected):
            raise ValueError(f'Invalid SHA256 in manifest: {relative}')
        actual = digest(path) if path.is_file() else None
        if actual != expected.lower():
            mismatches.append({'file': relative, 'expected': expected, 'actual': actual})
    if mismatches:
        raise ValueError('SOURCE_IDENTITY_MISMATCH: ' + json.dumps(mismatches, ensure_ascii=False))
    try:
        head = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True, stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        head = None
    return {'status': 'MANIFESTED_SOURCE_FILES_MATCHED', 'files_checked': len(files),
            'manifest_sha256': digest(manifest_path), 'source_commit': manifest.get('source_commit'),
            'observed_head': head, 'scope': 'Listed source bytes only; a later report-only commit may have a different HEAD.'}


def prepare_native(case: str, repo: Path, out: Path) -> list[dict]:
    relative, binaries = NATIVE[case]
    original = HERE / relative
    destination = out / 'work' / case / 'probe'
    shutil.copytree(original, destination, ignore=shutil.ignore_patterns('target', '__pycache__', '*.pyc'))
    manifest_path = destination / 'Cargo.toml'
    text = manifest_path.read_text()
    pattern = r'"\.\./\.\./\.\./vigilode(?:/([^"\n]*))?"'
    text, count = re.subn(pattern, lambda m: json.dumps(str(repo / (m.group(1) or ''))), text)
    if count == 0:
        raise ValueError(f'No known repository dependency paths in {relative}/Cargo.toml')
    parsed = tomllib.loads(text)
    if 'workspace' not in parsed:
        text += '\n[workspace]\n'
    tomllib.loads(text)
    manifest_path.write_text(text)
    if not (destination / 'Cargo.lock').is_file():
        raise ValueError(f'Missing locked dependency list in {relative}')
    return [{'name': f'{case}_{binary}',
             'argv': ['cargo', 'run', '--locked', '--offline', '--manifest-path', str(manifest_path), '--bin', binary],
             'cwd': str(destination), 'kind': 'native'} for binary in binaries]


def prepare_math(repo: Path, out: Path) -> list[dict]:
    result = []
    for name, relative, flags in MATH:
        original = HERE / relative
        destination = out / 'work' / 'math' / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(original, destination)
        argv = [sys.executable, '-B', str(destination)]
        if flags:
            argv += [*flags, str(repo / TABLEAU)]
        result.append({'name': name, 'argv': argv, 'cwd': str(destination.parent), 'kind': 'python'})
    return result


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--repo', type=Path, required=True, help='VigilODE source with bytes matching the source manifest')
    p.add_argument('--out', type=Path, required=True, help='New output directory outside source repository and audit package')
    p.add_argument('--case', choices=[*NATIVE, 'math', 'all'], default='all')
    p.add_argument('--prepare-only', action='store_true', help='Validate source and copy/rebase probes without executing them')
    p.add_argument('--timeout', type=float, default=600.0, help='Per-command seconds, default 600')
    a = p.parse_args()
    repo, out = a.repo.expanduser().resolve(), a.out.expanduser().resolve()
    if not repo.is_dir():
        p.error('--repo must be an existing directory')
    if a.timeout <= 0:
        p.error('--timeout must be positive')
    if out.exists():
        p.error('--out already exists; choose a fresh directory to preserve evidence')
    if out.is_relative_to(repo) or out.is_relative_to(HERE):
        p.error('--out must be outside the source repository and audit package')
    # Check every listed source hash BEFORE creating output or running any probe.
    identity = checked_source(repo)
    if a.case in ('math', 'all'):
        manifest = json.loads((HERE / 'evidence/source/SOURCE_FILE_HASHES.json').read_text())
        if TABLEAU not in manifest['files']:
            raise ValueError('The homotopy tableau must be included in SOURCE_FILE_HASHES.json')
    out.mkdir(parents=True)
    (out / 'logs').mkdir()
    receipt = {'schema': 'vigilode.reaudit.reproduction.v1', 'started_at_utc': utc(), 'case': a.case,
               'prepare_only': a.prepare_only, 'source_identity': identity,
               'original_evidence_mutation': False, 'native_build_reuse': False,
               'scientific_claim': 'Reexecution only; scenario-specific acceptance and limitations remain in lane reports.',
               'commands': [], 'status': 'PREPARING'}
    receipt_path = out / 'REPRODUCTION_RECEIPT.json'
    try:
        commands = []
        for case in NATIVE:
            if a.case in (case, 'all'):
                commands += prepare_native(case, repo, out)
        if a.case in ('math', 'all'):
            commands += prepare_math(repo, out)
        for command in commands:
            command.update(status='NOT_RUN', exit_code=None)
        receipt['commands'] = commands
        receipt['status'] = 'PREPARED_NOT_EXECUTED'
        write_json(receipt_path, receipt)
        if not a.prepare_only:
            env = os.environ.copy()
            env.update(CARGO_TARGET_DIR=str(out / 'cargo-target'), CARGO_BUILD_JOBS='1', PYTHONDONTWRITEBYTECODE='1')
            receipt['status'] = 'RUNNING'
            for command in commands:
                stdout_path = out / 'logs' / (command['name'] + '.stdout')
                stderr_path = out / 'logs' / (command['name'] + '.stderr')
                command.update(stdout=str(stdout_path.relative_to(out)), stderr=str(stderr_path.relative_to(out)))
                started = time.monotonic()
                with stdout_path.open('w') as stdout, stderr_path.open('w') as stderr:
                    try:
                        process = subprocess.run(command['argv'], cwd=command['cwd'], env=env,
                                                 stdout=stdout, stderr=stderr, timeout=a.timeout, check=False)
                        command.update(exit_code=process.returncode, status='PASS' if process.returncode == 0 else 'EXECUTION_FAILED')
                    except subprocess.TimeoutExpired:
                        command.update(status='TIMEOUT', timeout_seconds=a.timeout)
                    except OSError as error:
                        command.update(status='EXECUTION_FAILED', error=str(error))
                command['elapsed_seconds'] = time.monotonic() - started
                write_json(receipt_path, receipt)
                if command['status'] != 'PASS':
                    receipt['status'] = command['status']
                    break
            else:
                receipt['status'] = 'PASS'
        receipt['finished_at_utc'] = utc()
        receipt['output_sha256'] = {str(path.relative_to(out)): digest(path) for path in sorted(out.rglob('*'))
                                   if path.is_file() and path != receipt_path and 'cargo-target' not in path.relative_to(out).parts}
        write_json(receipt_path, receipt)
        print(json.dumps({'status': receipt['status'], 'receipt': str(receipt_path), 'commands': len(commands)}, ensure_ascii=False))
        return 0 if receipt['status'] in ('PASS', 'PREPARED_NOT_EXECUTED') else 1
    except Exception as error:
        receipt.update(status='PREPARATION_FAILED', error=str(error), finished_at_utc=utc())
        write_json(receipt_path, receipt)
        raise


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (ValueError, FileNotFoundError, json.JSONDecodeError) as error:
        print(f'REPRODUCTION_REFUSED: {error}', file=sys.stderr)
        raise SystemExit(2)
