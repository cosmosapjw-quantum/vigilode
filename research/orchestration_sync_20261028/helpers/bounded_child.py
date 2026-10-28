"""Standard-library POSIX child runner; budgets must come from an approved contract.

CLI: python bounded_child.py /absolute/path/to/runner_input.json
Required input: argv (string list), cwd, timeout_seconds, cpu_seconds,
address_space_bytes, output_prefix. Does not select scientific criteria.
"""
from __future__ import annotations
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import time


def run(input_path: Path) -> int:
    raw = input_path.read_bytes()
    spec = json.loads(raw)
    argv = spec['argv']
    if not isinstance(argv, list) or not argv or any(not isinstance(x, str) for x in argv):
        raise ValueError('argv must be a nonempty list of strings')
    cwd = Path(spec['cwd'])
    prefix = Path(spec['output_prefix'])
    if not cwd.is_absolute() or not cwd.is_dir() or not prefix.is_absolute():
        raise ValueError('cwd must exist and cwd/output_prefix must be absolute')
    timeout = float(spec['timeout_seconds'])
    cpu = int(spec['cpu_seconds'])
    memory = int(spec['address_space_bytes'])
    if not math.isfinite(timeout) or timeout <= 0 or cpu <= 0 or memory <= 0:
        raise ValueError('explicit positive finite budgets required')
    prefix.parent.mkdir(parents=True, exist_ok=True)
    receipt_path = Path(str(prefix) + '.receipt.json')
    stdout_path = Path(str(prefix) + '.stdout.log')
    stderr_path = Path(str(prefix) + '.stderr.log')
    if any(p.exists() for p in (receipt_path, stdout_path, stderr_path)):
        raise FileExistsError('refusing to overwrite an existing run')

    def limits():
        os.setsid()
        resource.setrlimit(resource.RLIMIT_CPU, (cpu, cpu))
        resource.setrlimit(resource.RLIMIT_AS, (memory, memory))

    start = time.monotonic()
    timed_out = False
    with stdout_path.open('xb') as out, stderr_path.open('xb') as err:
        child = subprocess.Popen(argv, cwd=cwd, stdout=out, stderr=err, preexec_fn=limits)
        while True:
            pid, status, usage = os.wait4(child.pid, os.WNOHANG)
            if pid:
                break
            if time.monotonic() - start >= timeout:
                timed_out = True
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                _, status, usage = os.wait4(child.pid, 0)
                break
            time.sleep(min(0.02, timeout / 10))
        child.returncode = os.waitstatus_to_exitcode(status)
    receipt = {
        'schema': 'bounded-child-v1', 'input_path': str(input_path.resolve()),
        'input_sha256': hashlib.sha256(raw).hexdigest(), 'spec': spec,
        'pid': child.pid, 'returncode': child.returncode, 'timed_out': timed_out,
        'wall_seconds': time.monotonic() - start,
        'user_seconds': usage.ru_utime, 'system_seconds': usage.ru_stime,
        'maxrss_native': usage.ru_maxrss, 'maxrss_unit': 'KiB' if sys.platform.startswith('linux') else 'bytes',
        'minor_faults': usage.ru_minflt, 'major_faults': usage.ru_majflt,
        'stdout': str(stdout_path), 'stderr': str(stderr_path),
        'python': sys.version, 'platform': sys.platform,
    }
    receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
    return child.returncode if child.returncode >= 0 else 128 - child.returncode


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('usage: bounded_child.py /absolute/path/to/runner_input.json')
    raise SystemExit(run(Path(sys.argv[1])))
