"""Shared helpers of the BASS/rei NCP C1 work-precision campaign.

Process accounting follows the ncpf-20261021 campaign: every solver run is a
child process timed by monotonic ns with its rusage from wait4; rows are
appended to JSONL with fsync; status OK alone is never success.
"""
import hashlib
import json
import math
import os
import subprocess
import threading
import time
from pathlib import Path

HARNESS = Path(__file__).resolve().parent
CAMPAIGN_ID = "vigilode_bassrei_ncp_c1_20261022"


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def sha256_bytes(b):
    return hashlib.sha256(b).hexdigest()


def utc_now():
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def sanitize(x):
    """JSON-safe copy: non-finite floats become their repr strings."""
    if isinstance(x, dict):
        return {str(k): sanitize(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [sanitize(v) for v in x]
    try:
        import numpy as np
        if isinstance(x, np.ndarray):
            return sanitize(x.tolist())
        if isinstance(x, np.bool_):
            return bool(x)
        if isinstance(x, np.integer):
            return int(x)
        if isinstance(x, np.floating):
            x = float(x)
    except ImportError:  # pragma: no cover
        pass
    if isinstance(x, float):
        return x if math.isfinite(x) else repr(x)
    return x


def append_jsonl(path, row):
    with open(path, "a") as f:
        f.write(json.dumps(sanitize(row), allow_nan=False, separators=(",", ":")) + "\n")
        f.flush()
        os.fsync(f.fileno())


def read_jsonl(path):
    rows = []
    p = Path(path)
    if not p.exists():
        return rows
    for line in p.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            rows.append({"status": "MALFORMED_ROW", "raw": line[:500]})
    return rows


def write_json(path, obj):
    tmp = Path(str(path) + ".tmp")
    tmp.write_text(json.dumps(sanitize(obj), indent=1, allow_nan=False) + "\n")
    os.replace(tmp, path)


def child_env(extra=None):
    env = dict(os.environ)
    env.update(OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1", MKL_NUM_THREADS="1",
               RAYON_NUM_THREADS="1", JULIA_NUM_THREADS="1", PYTHONDONTWRITEBYTECODE="1")
    if extra:
        env.update(extra)
    return env


# ---------------------------------------------------------------------------
# Child registry: every child session is recorded under NCPC1_CHILD_REG (one
# file per live child, named by its PGID) so that a driver's signal handler and
# the campaign's finalize/watchdog can kill every solver, Julia server and
# diffsol process; nothing outlives the 120-min cap.
# ---------------------------------------------------------------------------

def _reg_dir():
    d = os.environ.get("NCPC1_CHILD_REG")
    if not d:
        return None
    Path(d).mkdir(parents=True, exist_ok=True)
    return Path(d)


def register_child(pid, argv):
    d = _reg_dir()
    if d is not None:
        (d / str(pid)).write_text(" ".join(str(a) for a in argv)[:500])


def unregister_child(pid):
    d = _reg_dir()
    if d is not None:
        try:
            (d / str(pid)).unlink()
        except FileNotFoundError:
            pass


_LOCAL_CHILDREN = set()
_LOCAL_LOCK = threading.Lock()


def kill_registered(sig=9, only_local=False):
    """killpg every recorded child session (own ones, or all in the registry)."""
    with _LOCAL_LOCK:
        pids = set(_LOCAL_CHILDREN)
    d = _reg_dir()
    if d is not None and not only_local:
        pids |= {int(p.name) for p in d.iterdir() if p.name.isdigit()}
    for pid in pids:
        try:
            os.killpg(pid, sig)
        except (ProcessLookupError, PermissionError):
            pass
        unregister_child(pid)
    return sorted(pids)


def install_signal_cleanup():
    """SIGTERM/SIGINT: kill this driver's child sessions, then exit 143."""
    import signal

    def handler(signum, frame):
        kill_registered(only_local=True)
        os._exit(143)
    signal.signal(signal.SIGTERM, handler)
    signal.signal(signal.SIGINT, handler)


def track(p, argv):
    with _LOCAL_LOCK:
        _LOCAL_CHILDREN.add(p.pid)
    register_child(p.pid, argv)


def untrack(p):
    with _LOCAL_LOCK:
        _LOCAL_CHILDREN.discard(p.pid)
    unregister_child(p.pid)


def run_child(argv, stdout_path, stderr_path, env=None, timeout=None, stdin_path=None):
    """Run argv in its own session; return exact wall (monotonic ns) and the
    child's rusage (wait4). The child is registered for campaign-wide cleanup."""
    la = os.getloadavg()
    started_utc = utc_now()
    fin = open(stdin_path, "rb") if stdin_path else subprocess.DEVNULL
    try:
        with open(stdout_path, "wb") as fo, open(stderr_path, "wb") as fe:
            t0 = time.monotonic_ns()
            p = subprocess.Popen(argv, stdout=fo, stderr=fe, stdin=fin, env=env, start_new_session=True)
            track(p, argv)
            killed = {"v": False}

            def _kill():
                killed["v"] = True
                try:
                    os.killpg(p.pid, 9)
                except ProcessLookupError:
                    pass
            timer = threading.Timer(timeout, _kill) if timeout else None
            if timer:
                timer.start()
            try:
                _, status, ru = os.wait4(p.pid, 0)
            finally:
                untrack(p)
            t1 = time.monotonic_ns()
            if timer:
                timer.cancel()
            code = os.waitstatus_to_exitcode(status)
    finally:
        if stdin_path:
            fin.close()
    return {"argv": [str(a) for a in argv], "exit_code": code, "timed_out": killed["v"],
            "effective_timeout_s": timeout,
            "process_wall_ns": t1 - t0, "monotonic_start_ns": t0, "monotonic_end_ns": t1,
            "started_utc": started_utc, "child_user_s": ru.ru_utime, "child_sys_s": ru.ru_stime,
            "child_maxrss_kb": ru.ru_maxrss, "loadavg_at_start": la, "timing_authority": "diagnostic_only",
            "stdout_path": str(stdout_path), "stdout_sha256": sha256_file(stdout_path),
            "stdout_bytes": os.path.getsize(stdout_path), "stderr_path": str(stderr_path)}


def load_stdout_json(info):
    try:
        return json.loads(Path(info["stdout_path"]).read_text()), None
    except Exception as e:  # noqa: BLE001
        return None, repr(e)


class Deadline:
    """Absolute monotonic deadline shared through an environment variable."""

    def __init__(self, seconds_from_now=None, env_key="NCPC1_DEADLINE_MONO"):
        v = os.environ.get(env_key)
        if v is not None:
            self.t = float(v)
        elif seconds_from_now is not None:
            self.t = time.monotonic() + seconds_from_now
        else:
            self.t = float("inf")

    def left(self):
        return self.t - time.monotonic()

    def expired(self, margin=0.0):
        return self.left() <= margin

    def budget(self, cap, reserve=15.0):
        """(effective timeout, deadline_capped): min(cap, left - reserve)."""
        room = self.left() - reserve
        return (cap, False) if room >= cap else (max(room, 0.0), True)
