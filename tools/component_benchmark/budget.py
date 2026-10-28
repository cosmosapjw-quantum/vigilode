"""Persistent, locked host-lease accounting for the 72 instance-hour cap."""
from __future__ import annotations

import fcntl
import json
import os
import time
from contextlib import contextmanager
from pathlib import Path

CAP_SECONDS = 72 * 3600
PHASE_SECONDS = {"P0": 6*3600, "P1": 12*3600, "P2": 20*3600, "P3": 24*3600, "P4": 6*3600, "reserve": 4*3600}


def boot_id():
    path = Path("/proc/sys/kernel/random/boot_id")
    return path.read_text().strip() if path.exists() else "UNKNOWN_BOOT"


def pid_alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


@contextmanager
def locked_ledger(path):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        handle.seek(0)
        raw = handle.read()
        data = json.loads(raw) if raw else {"schema": "host-lease-v1", "leases": []}
        try:
            yield data
        finally:
            handle.seek(0)
            handle.truncate()
            json.dump(data, handle, indent=2, sort_keys=True)
            handle.flush()
            os.fsync(handle.fileno())
            fcntl.flock(handle, fcntl.LOCK_UN)


def usage(data, now=None):
    now = time.time() if now is None else now
    by_phase = {phase: 0.0 for phase in PHASE_SECONDS}
    for lease in data["leases"]:
        end = lease.get("end", min(now, lease["deadline"]))
        by_phase[lease["phase"]] += max(0.0, end - lease["start"])
    return by_phase


def reserve(path, phase, seconds, host, cleanup_seconds=30, persistent=False):
    if phase not in PHASE_SECONDS or seconds <= 0 or cleanup_seconds < 0:
        raise ValueError("invalid lease request")
    with locked_ledger(path) as data:
        now = time.time()
        for lease in data["leases"]:
            if "end" in lease: continue
            if not lease.get("persistent") and lease["host"] == host and lease.get("boot_id") == boot_id() and not pid_alive(lease["pid"]):
                lease["end"] = lease["deadline"]
                lease["recovery"] = "OWNER_PROCESS_GONE_FULL_RESERVATION_CHARGED"
            elif now >= lease["deadline"]:
                raise RuntimeError("EXPIRED_ACTIVE_LEASE_REQUIRES_RECOVERY")
            elif lease["host"] == host:
                raise RuntimeError("HOST_LEASE_ALREADY_ACTIVE")
        used = usage(data, now)
        # Open leases may run to their full reserved deadline. Count each host
        # lease once, even when multiple jobs share that invocation.
        held = sum(max(0.0, x["deadline"] - now) for x in data["leases"] if "end" not in x)
        proposed = seconds + cleanup_seconds
        if sum(used.values()) + held + proposed > CAP_SECONDS or used[phase] + sum(max(0.0, x["deadline"] - now) for x in data["leases"] if "end" not in x and x["phase"] == phase) + proposed > PHASE_SECONDS[phase]:
            raise RuntimeError("LEASE_CAP_EXCEEDED")
        lease_id = str(time.time_ns()) + "-" + str(os.getpid())
        data["leases"].append({"id": lease_id, "host": host, "phase": phase, "start": now, "start_monotonic": time.monotonic(),
                               "boot_id": boot_id(), "pid": os.getpid(), "deadline": now + proposed, "reserved_seconds": proposed,
                               "persistent": persistent})
        return lease_id


def close(path, lease_id):
    with locked_ledger(path) as data:
        for lease in data["leases"]:
            if lease["id"] == lease_id:
                if "end" not in lease:
                    elapsed = time.monotonic() - lease["start_monotonic"] if lease.get("boot_id") == boot_id() else time.time() - lease["start"]
                    lease["end"] = lease["start"] + max(0.0, elapsed)
                    lease["elapsed_seconds"] = max(0.0, elapsed)
                    lease["reservation_overrun"] = lease["end"] > lease["deadline"]
                return lease
    raise KeyError(lease_id)
