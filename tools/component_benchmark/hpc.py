"""Opt-in, resource-admitted P1/P2 screening/reference/probe execution.

CPU slots are disjoint allowed logical CPUs. The eight-CPU and 16-GiB host
reserves are admission controls, not evidence of physical-core isolation.
"""
from __future__ import annotations

import concurrent.futures
import json
import math
import os
import platform
import re
import shutil
import signal
import subprocess
import tempfile
import threading
import time
from dataclasses import dataclass
from pathlib import Path

GIB = 1 << 30
RESERVED_CPUS = 8
RESERVED_MEMORY = 16 * GIB
MAX_WORKERS = 56
MAX_JULIA_JOBS = 48
MEMORY_BY_CLASS = {"julia": 2*GIB, "reference": 2*GIB, "native": GIB//2,
                   "diffsol": GIB, "python": GIB, "other": 2*GIB, "contract": GIB//2}
THREAD_KEYS = ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS",
               "RAYON_NUM_THREADS", "JULIA_NUM_THREADS", "RUST_TEST_THREADS")


def parse_cpu_list(value):
    cpus = set()
    for part in value.strip().split(","):
        if not part:
            continue
        endpoints = part.split("-")
        if len(endpoints) == 1:
            cpus.add(int(endpoints[0]))
        elif len(endpoints) == 2:
            low, high = map(int, endpoints)
            if high < low:
                raise ValueError("invalid cpuset range")
            cpus.update(range(low, high+1))
        else:
            raise ValueError("invalid cpuset")
    return tuple(sorted(cpus))


def _read(path):
    try:
        return Path(path).read_text().strip()
    except OSError:
        return None


def _read_raw(path):
    try:
        return Path(path).read_text()
    except OSError:
        return None


def _mem_available():
    raw = _read("/proc/meminfo") or ""
    for line in raw.splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("MEMAVAILABLE_UNAVAILABLE")


def _cgroup_v2_paths():
    """Current cgroup then ancestors, stopping at its cgroup2 mount root."""
    membership = _read("/proc/self/cgroup")
    mountinfo = _read("/proc/self/mountinfo")
    if membership is None or mountinfo is None:
        return None
    groups = [line.split("::", 1)[1] for line in membership.splitlines()
              if line.startswith("0::")]
    if len(groups) != 1:
        return None
    group = groups[0]
    def unescape(value):
        return re.sub(r"\\([0-7]{3})",lambda match:chr(int(match.group(1),8)),value)
    for line in mountinfo.splitlines():
        before, separator, after = line.partition(" - ")
        fields = before.split()
        if not separator or len(fields) < 5 or not after.startswith("cgroup2 "):
            continue
        root = unescape(fields[3]).rstrip("/") or "/"
        mount = Path(unescape(fields[4]))
        if group != root and not group.startswith(root.rstrip("/") + "/"):
            continue
        relative = group[len(root):].lstrip("/") if root != "/" else group.lstrip("/")
        current = mount / relative
        paths = []
        while True:
            paths.append(current)
            if current == mount:
                return tuple(paths)
            if mount not in current.parents:
                break
            current = current.parent
    return None


def _cgroup_memory():
    paths = _cgroup_v2_paths()
    if paths is None:
        return None, None, None
    limits = []
    for path in paths:
        maximum = _read(path / "memory.max")
        if maximum is None or maximum == "max":
            continue
        current = _read(path / "memory.current")
        if current is None:
            raise RuntimeError("HPC_CGROUP_MEMORY_CURRENT_UNAVAILABLE")
        try:
            ceiling, used = int(maximum), int(current)
        except ValueError as exc:
            raise RuntimeError("HPC_CGROUP_MEMORY_INVALID") from exc
        limits.append((max(0,ceiling-used),ceiling,used))
    return min(limits) if limits else (None,None,None)


def _quota_cores():
    paths = _cgroup_v2_paths()
    if paths is None:
        return None
    quotas = []
    for path in paths:
        raw = _read(path / "cpu.max")
        if raw is None:
            continue
        parts = raw.split()
        if len(parts) != 2:
            raise RuntimeError("HPC_CGROUP_CPU_QUOTA_INVALID")
        amount, period = parts
        if amount != "max":
            try:
                quotas.append(max(0,int(amount)//int(period)))
            except (ValueError,ZeroDivisionError) as exc:
                raise RuntimeError("HPC_CGROUP_CPU_QUOTA_INVALID") from exc
    return min(quotas) if quotas else None


@dataclass(frozen=True)
class Resources:
    host: str
    allowed_cpus: tuple[int, ...]
    worker_cpus: tuple[int, ...]
    reserved_cpus: tuple[int, ...]
    cpu_quota_cores: int | None
    mem_available_bytes: int
    cgroup_mem_remaining_bytes: int | None
    cgroup_mem_max_bytes: int | None
    cgroup_mem_current_bytes: int | None
    memory_budget_bytes: int
    max_workers: int


def detect_resources():
    affinity = set(os.sched_getaffinity(0))
    for path in _cgroup_v2_paths() or ():
        cpuset_raw = _read(path / "cpuset.cpus.effective")
        if cpuset_raw:
            affinity.intersection_update(parse_cpu_list(cpuset_raw))
    allowed = tuple(sorted(affinity))
    quota = _quota_cores()
    count = len(allowed) if quota is None else min(len(allowed), quota)
    if count <= RESERVED_CPUS:
        raise RuntimeError("HPC_CPU_RESERVE_UNAVAILABLE")
    eligible = allowed[:count]
    reserved = eligible[:RESERVED_CPUS]
    workers = eligible[RESERVED_CPUS:RESERVED_CPUS+MAX_WORKERS]
    mem_available = _mem_available()
    cg_remaining, cg_max, cg_current = _cgroup_memory()
    available = min(mem_available, cg_remaining) if cg_remaining is not None else mem_available
    budget = available - RESERVED_MEMORY
    if budget < min(MEMORY_BY_CLASS.values()):
        raise RuntimeError("HPC_MEMORY_RESERVE_UNAVAILABLE")
    return Resources(platform.node(), allowed, workers, reserved, quota,
                     mem_available, cg_remaining, cg_max, cg_current, budget,
                     min(MAX_WORKERS, len(workers), budget//min(MEMORY_BY_CLASS.values())))


def job_class(solver_id, kind=None):
    if kind == "reference":
        return "reference"
    sid = (solver_id or "").casefold()
    if sid.startswith("julia-"):
        return "julia"
    if sid.startswith("native-") or sid.startswith("rodas5p-"):
        return "native"
    if sid.startswith("diffsol-"):
        return "diffsol"
    if sid.startswith(("scipy-", "vode-", "hairer-")):
        return "python"
    return "other"


def job_threads(job, spec=None):
    spec = spec or job.get("solver_spec", {})
    configured = spec.get("threads")
    request = job.get("request", {}).get("solver_config", {}).get("threads")
    if configured is not None and request is not None and configured != request:
        raise ValueError("SOLVER_THREAD_CONFIG_MISMATCH")
    threads = configured if configured is not None else (request if request is not None else 1)
    if type(threads) is not int or threads not in (1, 2, 4, 8):
        raise ValueError("SOLVER_THREAD_CONFIG_INVALID")
    return threads


@dataclass(frozen=True)
class Demand:
    cores: int
    memory_bytes: int
    job_class: str


def demand_for(job, spec=None, kind=None):
    category = job_class(job.get("solver_id", job.get("id", "")), kind)
    if kind == "contract":
        category = "contract"
    return Demand(job_threads(job, spec), MEMORY_BY_CLASS[category], category)


def estimate_waves(jobs, resources, *, demand=None, max_workers=None):
    """Conservative synchronous bin packing for lease admission."""
    pending = [demand(job) if demand else demand_for(job) for job in jobs]
    limit = min(resources.max_workers, max_workers or resources.max_workers)
    waves = 0
    while pending:
        cores = len(resources.worker_cpus)
        memory = resources.memory_budget_bytes
        count = julia = 0
        rest = []
        for req in pending:
            if req.cores > len(resources.worker_cpus) or req.memory_bytes > resources.memory_budget_bytes:
                raise RuntimeError("HPC_JOB_EXCEEDS_ADMITTED_RESOURCES")
            if count < limit and req.cores <= cores and req.memory_bytes <= memory and (req.job_class != "julia" or julia < MAX_JULIA_JOBS):
                cores -= req.cores
                memory -= req.memory_bytes
                count += 1
                julia += req.job_class == "julia"
            else:
                rest.append(req)
        if not count:
            raise RuntimeError("HPC_WAVE_PACKING_STALLED")
        pending = rest
        waves += 1
    return waves


@dataclass(frozen=True)
class Allocation:
    cpus: tuple[int, ...]
    memory_bytes: int
    job_class: str

    @property
    def threads(self):
        return len(self.cpus)


def thread_environment(base, allocation, *, contract=False):
    env = dict(base)
    for key in THREAD_KEYS:
        env[key] = "1" if contract and key == "RUST_TEST_THREADS" else str(allocation.threads)
    return env


def affinity_command(argv, allocation):
    taskset = shutil.which("taskset")
    if not taskset:
        raise RuntimeError("TASKSET_UNAVAILABLE")
    return [taskset, "-c", ",".join(map(str, allocation.cpus)), *argv]


def assigned_cpu_sample(cpus):
    """Raw /proc/stat counters for exactly the assigned logical CPUs."""
    wanted = {f"cpu{int(cpu)}" for cpu in cpus}
    counters = {}
    for line in (_read("/proc/stat") or "").splitlines():
        fields = line.split()
        if not fields or fields[0] not in wanted:
            continue
        values = [int(x) for x in fields[1:]]
        counters[fields[0]] = {"total":sum(values),
                               "idle":values[3]+(values[4] if len(values)>4 else 0),
                               "steal":values[7] if len(values)>7 else 0}
    if set(counters) != wanted:
        raise RuntimeError("ASSIGNED_CPU_STAT_UNAVAILABLE")
    return counters


def assigned_cpu_delta(start, end):
    if set(start) != set(end):
        raise ValueError("assigned CPU sample identity mismatch")
    total = sum(end[cpu]["total"]-start[cpu]["total"] for cpu in start)
    idle = sum(end[cpu]["idle"]-start[cpu]["idle"] for cpu in start)
    steal = sum(end[cpu]["steal"]-start[cpu]["steal"] for cpu in start)
    if total <= 0:
        return {"busy_fraction":None,"steal_fraction":None,"sample_source":"/proc/stat assigned CPU delta"}
    return {"busy_fraction":1-idle/total,"steal_fraction":steal/total,
            "sample_source":"/proc/stat assigned CPU delta"}


def _memory_events_sample():
    paths = _cgroup_v2_paths()
    path = paths[0] / "memory.events" if paths else None
    raw = _read_raw(path) if path is not None else None
    if raw is None:
        return {"path":str(path) if path is not None else None,"raw":None,"counters":None}
    try:
        events = {key:int(value) for key,value in (row.split() for row in raw.splitlines())}
    except (ValueError, TypeError):
        events = None
    if events is not None and not all(key in events for key in ("oom", "oom_kill")):
        events = None
    return {"path":str(path),"raw":raw,"counters":events}


def _memory_events():
    return _memory_events_sample()["counters"]


def _oom_delta(before, after):
    if before is None or after is None:
        return None
    delta = {key:after[key]-before[key] for key in ("oom", "oom_kill")}
    return delta if all(value >= 0 for value in delta.values()) else None


def _process_tree_sample(root_pid, assigned_cpus):
    queue = [root_pid]
    seen = set()
    rss = hwm = 0
    escaped = False
    while queue:
        pid = queue.pop()
        if pid in seen:
            continue
        seen.add(pid)
        try:
            lines = Path(f"/proc/{pid}/status").read_text().splitlines()
        except OSError:
            continue
        fields = {line.split(":",1)[0]:line.split(":",1)[1].strip() for line in lines if ":" in line}
        current = int(fields.get("VmRSS","0").split()[0])*1024
        high = int(fields.get("VmHWM","0").split()[0])*1024
        rss += current
        hwm += max(current,high)
        allowed = fields.get("Cpus_allowed_list")
        if allowed and not set(parse_cpu_list(allowed)).issubset(assigned_cpus):
            escaped = True
        children = _read(f"/proc/{pid}/task/{pid}/children")
        if children:
            queue.extend(int(child) for child in children.split())
    return {"rss":rss,"hwm":hwm,"escaped":escaped,"pids":seen}


class ProcessTracker:
    def __init__(self):
        self._lock = threading.Lock()
        self._pids = set()

    def add(self, pid):
        with self._lock:
            self._pids.add(pid)

    def remove(self, pid):
        with self._lock:
            self._pids.discard(pid)

    def rss_bytes(self):
        with self._lock:
            pids = tuple(self._pids)
        total = 0
        for pid in pids:
            total += _process_tree_sample(pid,set()).get("rss",0)
        return total


def run_process(argv, *, allocation, tracker, env, cwd=None, timeout=60, metrics_path=None):
    time_binary = Path("/usr/bin/time")
    temporary_metrics = None
    if metrics_path is None:
        handle = tempfile.NamedTemporaryFile(prefix="vigilode-hpc-time-", delete=False)
        metrics_path = Path(handle.name)
        handle.close()
        temporary_metrics = metrics_path
    else:
        metrics_path = Path(metrics_path).resolve()
    timed_argv = [str(time_binary), "-v", "-o", str(metrics_path), *argv] if time_binary.is_file() else argv
    command = affinity_command(timed_argv, allocation)
    tick = time.monotonic()
    events_before = _memory_events_sample()
    cpu_before = assigned_cpu_sample(allocation.cpus)
    process = subprocess.Popen(command, cwd=cwd, env=env, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, start_new_session=True)
    tracker.add(process.pid)
    stop = threading.Event()
    observed = {"peak_rss":0,"peak_hwm":0,"samples":0,"affinity_escape":False,
                "affinity_verified":False}
    def monitor():
        while True:
            sample = _process_tree_sample(process.pid,set(allocation.cpus))
            observed["peak_rss"] = max(observed["peak_rss"],sample["rss"])
            observed["peak_hwm"] = max(observed["peak_hwm"],sample["hwm"])
            observed["samples"] += 1
            # The taskset launcher exists briefly before setting its own affinity.
            # Only count an escape after a correctly pinned process tree was observed.
            if observed["affinity_verified"]:
                observed["affinity_escape"] |= sample["escaped"]
            elif sample["rss"] > 0 and not sample["escaped"]:
                observed["affinity_verified"] = True
            if stop.wait(0.01):
                break
    monitor_thread = threading.Thread(target=monitor,daemon=True)
    monitor_thread.start()
    try:
        try:
            stdout, stderr = process.communicate(timeout=timeout)
            timed_out = False
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            stdout, stderr = process.communicate()
            timed_out = True
    finally:
        stop.set()
        monitor_thread.join(timeout=0.5)
        tracker.remove(process.pid)
    events_after = _memory_events_sample()
    cpu_observed = assigned_cpu_delta(cpu_before, assigned_cpu_sample(allocation.cpus))
    time_report = _read(metrics_path) or ""
    time_kib = None
    for line in time_report.splitlines():
        if "Maximum resident set size (kbytes):" in line:
            time_kib = int(line.rsplit(":",1)[1].strip())
            break
    if temporary_metrics is not None:
        temporary_metrics.unlink(missing_ok=True)
    observed_peak = max(observed["peak_rss"], observed["peak_hwm"],
                        (time_kib or 0)*1024)
    affinity_verified = observed["affinity_verified"] or process.returncode == 0
    return {"command":command,"stdout":stdout,"stderr":stderr,"exit_code":process.returncode,
            "timed_out":timed_out,"elapsed_seconds":time.monotonic()-tick,
            "affinity_cpus":list(allocation.cpus),
            "peak_rss_bytes":observed_peak,
            "peak_live_rss_bytes":observed["peak_rss"],
            "rss_samples":observed["samples"],
            "rss_source":"max(GNU time wait4 peak, sampled /proc process-tree VmRSS/VmHWM)" if time_kib is not None else "sampled /proc process-tree VmRSS/VmHWM; observed bound",
            "gnu_time_max_rss_bytes":time_kib*1024 if time_kib is not None else None,
            "gnu_time_report":str(metrics_path) if temporary_metrics is None and time_binary.is_file() else None,
            "affinity_escape":observed["affinity_escape"],
            "affinity_verified":affinity_verified,
            "affinity_evidence":"sampled pinned process tree" if observed["affinity_verified"] else (
                "successful taskset launch; no pinned process-tree sample" if process.returncode == 0 else "UNKNOWN"),
            "cgroup_oom_events_delta":_oom_delta(events_before["counters"],events_after["counters"])
                if events_before["path"] is not None and events_before["path"] == events_after["path"] else None,
            "cgroup_oom_events_path":events_before["path"] if events_before["path"] == events_after["path"] else None,
            "cgroup_oom_events_raw_before":events_before["raw"],
            "cgroup_oom_events_raw_after":events_after["raw"],
            "cgroup_oom_scope":"current cgroup v2 memory.events; shared cgroup attribution unavailable",
            "assigned_cpu_busy_fraction":cpu_observed["busy_fraction"],
            "assigned_cpu_steal_fraction":cpu_observed["steal_fraction"],
            "assigned_cpu_sample_source":cpu_observed["sample_source"]}


def _cpu_sample():
    fields = (_read("/proc/stat") or "").splitlines()[0].split()
    values = [int(x) for x in fields[1:]]
    total = sum(values)
    idle = values[3] + (values[4] if len(values)>4 else 0)
    steal = values[7] if len(values)>7 else 0
    return total, idle, steal


class ResourcePool:
    def __init__(self, resources=None, max_workers=None, telemetry_path=None, tracker=None):
        self.resources = resources or detect_resources()
        self.max_workers = min(self.resources.max_workers, max_workers or self.resources.max_workers)
        if self.max_workers < 1:
            raise RuntimeError("HPC_WORKER_CAP_ZERO")
        self.telemetry_path = Path(telemetry_path) if telemetry_path else None
        self.tracker = tracker or ProcessTracker()
        self._last_cpu = None

    def _telemetry(self, queued, running, completed, allocations):
        current = _cpu_sample()
        busy = steal = None
        if self._last_cpu:
            total = current[0]-self._last_cpu[0]
            if total>0:
                busy = 1-(current[1]-self._last_cpu[1])/total
                steal = (current[2]-self._last_cpu[2])/total
        self._last_cpu = current
        row = {"unix":time.time(),"host":self.resources.host,
               "cpuset":list(self.resources.allowed_cpus),"worker_cpuset":list(self.resources.worker_cpus),
               "reserved_cpuset":list(self.resources.reserved_cpus),
               "cpu_quota_cores":self.resources.cpu_quota_cores,
               "memory_budget_bytes":self.resources.memory_budget_bytes,
               "mem_available_bytes":self.resources.mem_available_bytes,
               "cgroup_mem_remaining_bytes":self.resources.cgroup_mem_remaining_bytes,
               "cpu_busy_fraction":busy,"cpu_steal_fraction":steal,
               "own_child_rss_bytes":self.tracker.rss_bytes(),
               "external_background_status":"UNKNOWN_NOT_ATTRIBUTABLE",
               "queued":queued,"running":running,"completed":completed,
               "allocated_cores":sum(len(x.cpus) for x in allocations),
               "allocated_memory_bytes":sum(x.memory_bytes for x in allocations)}
        if self.telemetry_path:
            self.telemetry_path.parent.mkdir(parents=True, exist_ok=True)
            with self.telemetry_path.open("a") as stream:
                stream.write(json.dumps(row,sort_keys=True)+"\n")
                stream.flush()
                os.fsync(stream.fileno())
        return row

    def run(self, jobs, worker, *, demand=None, poll_seconds=5):
        pending = list(jobs)
        demands = [demand(job) if demand else demand_for(job) for job in pending]
        for requirement in demands:
            if requirement.cores > len(self.resources.worker_cpus) or requirement.memory_bytes > self.resources.memory_budget_bytes:
                raise RuntimeError("HPC_JOB_EXCEEDS_ADMITTED_RESOURCES")
        free = set(self.resources.worker_cpus)
        memory_free = self.resources.memory_budget_bytes
        active = {}
        completed = []
        self._telemetry(len(pending),0,0,[])
        with concurrent.futures.ThreadPoolExecutor(max_workers=self.max_workers) as executor:
            while pending or active:
                while len(active)<self.max_workers:
                    chosen = next((i for i,req in enumerate(demands)
                                   if req.cores<=len(free) and req.memory_bytes<=memory_free
                                   and (req.job_class!="julia" or sum(a.job_class=="julia" for a in active.values())<MAX_JULIA_JOBS)),None)
                    if chosen is None:
                        break
                    job,req = pending.pop(chosen),demands.pop(chosen)
                    cpus = tuple(sorted(free)[:req.cores])
                    free.difference_update(cpus)
                    memory_free -= req.memory_bytes
                    allocation = Allocation(cpus,req.memory_bytes,req.job_class)
                    future = executor.submit(worker,job,allocation,self.tracker)
                    active[future] = allocation
                    self._telemetry(len(pending),len(active),len(completed),active.values())
                if not active:
                    raise RuntimeError("HPC_SCHEDULER_STALLED")
                done,_ = concurrent.futures.wait(active,timeout=poll_seconds,
                                                  return_when=concurrent.futures.FIRST_COMPLETED)
                if not done:
                    self._telemetry(len(pending),len(active),len(completed),active.values())
                    continue
                for future in done:
                    allocation = active.pop(future)
                    free.update(allocation.cpus)
                    memory_free += allocation.memory_bytes
                    completed.append(future.result())
                self._telemetry(len(pending),len(active),len(completed),active.values())
        return completed
