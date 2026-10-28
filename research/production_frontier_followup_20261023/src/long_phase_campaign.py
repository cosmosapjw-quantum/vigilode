#!/usr/bin/env python3
"""Frozen LP01 sampled linear cohort. No inference of uniform/continuous accuracy.

--self-test exercises scoring/refusal without launching a solver or a timer.
A campaign requires committed sources and explicit source/tree/binary identities.
Every warmup and measured process has immutable raw receipts. Timing is diagnostic.
"""
import argparse
import hashlib
import json
import math
import os
import signal
import selectors
from pathlib import Path
import subprocess
import sys
import time

BASE = Path(__file__).resolve().parents[3]
SCOPE = Path(__file__).resolve().parents[1] / "LP01_SCOPE.json"
EXAMPLE = "crates/rodas5p-integrators/examples/frontier_long_phase.rs"
SCRIPT = str(Path(__file__).resolve().relative_to(BASE))
CASES = {"normal_100_f100": (100, 100.0, 0.0),
         "damped_100_f10000": (100, 10000.0, 1.0),
         "normal_1000_f1": (1000, 1.0, 0.0)}
CAPS = {"state": 1e-5, "phase": 1e-3, "amplitude": 1e-5}
MASK = 1e-12
REFERENCE_CACHE = {}
PROCESS_ACCOUNTING = None


class DeadlineExpired(TimeoutError):
    pass


class ProcessBudgetExhausted(RuntimeError):
    pass


class Deadline:
    def __init__(self, seconds):
        self.end = time.monotonic() + seconds
        self.phase = "PREFLIGHT"
        self.previous_handler = None

    def remaining(self):
        value = self.end - time.monotonic()
        if value <= 0:
            raise DeadlineExpired(self.phase)
        return value

    def enter(self, phase):
        self.phase = phase
        return self.remaining()

    def arm(self):
        require(hasattr(signal, "setitimer"), "absolute deadline requires POSIX interval timer")
        if self.previous_handler is None:
            self.previous_handler = signal.getsignal(signal.SIGALRM)
        signal.signal(signal.SIGALRM, self.expire)
        signal.setitimer(signal.ITIMER_REAL, self.remaining())

    def expire(self, *_):
        raise DeadlineExpired(self.phase)

    def cancel(self):
        signal.setitimer(signal.ITIMER_REAL, 0)
        if self.previous_handler is not None:
            signal.signal(signal.SIGALRM, self.previous_handler)
            self.previous_handler = None


def charge_process(command):
    if PROCESS_ACCOUNTING is not None:
        require(PROCESS_ACCOUNTING["deadline"].remaining() > 0, "expired deadline")
        if PROCESS_ACCOUNTING["attempts"] >= PROCESS_ACCOUNTING["cap"]:
            raise ProcessBudgetExhausted("shared process cap exhausted")
        PROCESS_ACCOUNTING["attempts"] += 1
        PROCESS_ACCOUNTING["commands"].append(command)




class RSSLimitExceeded(RuntimeError):
    pass


def resident_snapshot(process, known):
    """Conservative sum of VmRSS over group members and observed descendants.

    Shared resident pages can be counted more than once. Observed peak is a
    sampled diagnostic, not a bound on between-sample physical memory peaks.
    """
    def identity(pid):
        tail = (Path("/proc")/str(pid)/"stat").read_text().rsplit(")", 1)[1].split()
        return int(tail[19]), int(tail[2])  # starttime, process group

    members = {}
    for pid, start in known.items():
        try:
            current, _ = identity(pid)
            if current == start:
                members[pid] = start
        except (FileNotFoundError, ProcessLookupError):
            continue
    entries = list(Path("/proc").iterdir())
    require(len(entries) <= 131072, "/proc inventory exceeds bounded RSS monitor capacity")
    for entry in entries:
        if not entry.name.isdigit():
            continue
        try:
            tail = (entry/"stat").read_text().rsplit(")", 1)[1].split()
            if int(tail[2]) == process.pid:  # state, ppid, pgrp
                members[int(entry.name)] = int(tail[19])
        except (FileNotFoundError, ProcessLookupError):
            continue
    # Discover children forked from any thread, including a child which moves
    # itself to another process group after it has been observed.
    pending = list(members)
    visited = set()
    while pending:
        pid = pending.pop()
        if pid in visited:
            continue
        visited.add(pid)
        try:
            tasks = list((Path("/proc")/str(pid)/"task").iterdir())
            require(len(tasks) <= 65536, "task inventory exceeds RSS monitor capacity")
            for task in tasks:
                try:
                    children = (task/"children").read_text().split()
                except (FileNotFoundError, ProcessLookupError):
                    continue
                for child in children:
                    child = int(child)
                    if child not in members:
                        try:
                            members[child] = identity(child)[0]
                            pending.append(child)
                        except (FileNotFoundError, ProcessLookupError):
                            continue
        except (FileNotFoundError, ProcessLookupError):
            continue
    total = 0
    live = {}
    for pid, start in members.items():
        try:
            if identity(pid)[0] != start:
                continue
            status = (Path("/proc")/str(pid)/"status").read_text()
            for line in status.splitlines():
                if line.startswith("VmRSS:"):
                    total += int(line.split()[1])*1024
                    live[pid] = start
                    break
        except (FileNotFoundError, ProcessLookupError):
            continue
    return total, live


def teardown_owned_process(process, known=None):
    """Bounded failure cleanup; SIGALRM cannot replace the original failure."""
    previous_mask = signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGALRM})
    signal.setitimer(signal.ITIMER_REAL, 0)
    try:
        try:
            _, observed = resident_snapshot(process, known or {})
        except (OSError, ValueError):
            observed = known or {}
        group_owned = process.poll() is None
        verified = []
        for pid, start in observed.items():
            try:
                tail = (Path("/proc")/str(pid)/"stat").read_text().rsplit(")", 1)[1].split()
                if int(tail[19]) == start:
                    verified.append(pid)
                    group_owned |= int(tail[2]) == process.pid
            except (FileNotFoundError, ProcessLookupError):
                continue
        if group_owned:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        for pid in verified:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        if process.stdout.closed or process.stderr.closed:
            process.wait(timeout=1.0)
            return b"", b""
        try:
            return process.communicate(timeout=1.0)
        except subprocess.TimeoutExpired as error:
            process.stdout.close()
            process.stderr.close()
            process.wait(timeout=1.0)
            return error.stdout or b"", error.stderr or b""
    finally:
        # Consume a pending timer alarm while blocked, before restoring the mask.
        # The absolute monotonic deadline is rechecked before any next work row.
        if signal.SIGALRM in signal.sigpending():
            signal.sigwait({signal.SIGALRM})
        signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)


def monitored_communicate(process, timeout, deadline, rss_limit=1073741824, interval=0.02):
    """One controller process drains both pipes and samples child-tree VmRSS."""
    selector = None
    chunks = [[], []]
    peak = peak_members = 0
    known = {}
    try:
        # Setup is inside the same cleanup owner as the entire wait loop.
        selector = selectors.DefaultSelector()
        for index, stream in enumerate((process.stdout, process.stderr)):
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ, index)
        end = time.monotonic()+timeout
        while True:
            deadline.remaining()
            remaining = end-time.monotonic()
            if remaining <= 0:
                raise subprocess.TimeoutExpired(process.args, timeout)
            resident, known = resident_snapshot(process, known)
            peak = max(peak, resident)
            peak_members = max(peak_members, len(known))
            if resident > rss_limit:
                raise RSSLimitExceeded("solver/descendant VmRSS exceeded limit")
            if process.poll() is not None and not selector.get_map() and not known:
                break
            for key, _ in selector.select(min(interval, remaining, deadline.remaining())):
                try:
                    block = os.read(key.fileobj.fileno(), 65536)
                except BlockingIOError:
                    continue
                if block:
                    chunks[key.data].append(block)
                else:
                    selector.unregister(key.fileobj)
        process.wait()
        return b"".join(chunks[0]), b"".join(chunks[1]), peak, peak_members
    except BaseException as error:
        try:
            tail_out, tail_err = teardown_owned_process(process, known)
        except BaseException as cleanup_error:
            tail_out = tail_err = b""
            error.cleanup_error = getattr(error, "cleanup_error", repr(cleanup_error))
        error.stdout = b"".join(chunks[0])+tail_out
        error.stderr = b"".join(chunks[1])+tail_err
        error.peak_rss_bytes = peak
        error.peak_resident_processes = peak_members
        raise
    finally:
        if selector is not None:
            selector.close()
        process.stdout.close()
        process.stderr.close()


def owned_monitored_process(command, environment, timeout, deadline, rss_limit=1073741824):
    """Ownership begins at Popen, before any handoff argument or selector setup."""
    process = None
    try:
        timeout = min(timeout, deadline.remaining())
        process = subprocess.Popen(command, cwd=BASE, env=environment, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, start_new_session=True)
        stdout, stderr, peak, members = monitored_communicate(process, timeout, deadline, rss_limit)
        return stdout, stderr, peak, members, process.returncode
    except BaseException as error:
        tail_out = tail_err = b""
        if process is not None:
            try:
                tail_out, tail_err = teardown_owned_process(process)
            except BaseException as cleanup_error:
                error.cleanup_error = getattr(error, "cleanup_error", repr(cleanup_error))
        error.stdout = getattr(error, "stdout", b"")+tail_out
        error.stderr = getattr(error, "stderr", b"")+tail_err
        error.launched = process is not None
        error.child_pid = process.pid if process is not None else None
        error.child_returncode = process.returncode if process is not None else None
        raise


def preserve_owner_failure(record, error):
    """Retain first failure and stop the cohort when cleanup is unconfirmed."""
    record.setdefault("first_failure_status", record["status"])
    record.setdefault("original_failure_type", type(error).__name__)
    record.setdefault("original_failure_reason", str(error))
    if hasattr(error, "child_returncode"):
        record["child_returncode"] = error.child_returncode
    if hasattr(error, "cleanup_error"):
        record.setdefault("cleanup_error", error.cleanup_error)
    unconfirmed = getattr(error, "launched", False) and getattr(error, "child_returncode", None) is None
    if hasattr(error, "cleanup_error") or unconfirmed:
        record.setdefault("original_status", record["status"])
        record["status"] = "CLEANUP_FAILURE"
        return True
    return False


def resume_post_child_freeze(record, deadline):
    if record["status"] == "CLEANUP_FAILURE":
        return False
    deadline.enter("FREEZE")
    deadline.arm()  # Teardown cancels SIGALRM; absolute end was never changed.
    return True


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def write_new(path, value):
    with open(path, "x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write("\n")


def git(*args):
    command = ["git", *args]
    charge_process(command)
    return subprocess.check_output(command, cwd=BASE, text=True).strip()


def expected_times(case):
    cycles, frequency, _ = CASES[case]
    end = 2.0 * math.pi * cycles / frequency
    return [end * i / 4096.0 for i in range(4097)]


def check_scope():
    scope = json.loads(SCOPE.read_text())
    require(scope["status"] == "READY_CONTRACT_FROZEN_NO_RUN", "LP01 scope is not frozen")
    require([c["id"] for c in scope["cases"]] == list(CASES), "registered cases changed")
    for case in scope["cases"]:
        cycles, frequency, damping = CASES[case["id"]]
        d = case.get("damping", case.get("damping_fraction_of_frequency", 0) * frequency)
        require((case["cycles"], case["angular_frequency"], d, case["observations"]) ==
                (cycles, frequency, damping, 4097), "case parameters changed")
    require(scope["same_achieved_error_cap"]["cap"] == CAPS["state"], "state cap changed")
    require(scope["proposed_thresholds"]["phase_radians"] == CAPS["phase"], "phase cap changed")
    require(scope["proposed_thresholds"]["relative_amplitude"] == CAPS["amplitude"], "amplitude cap changed")
    require(scope["solver_tolerances"] == [{"rtol": 1e-6, "atol": 1e-8}, {"rtol": 1e-8, "atol": 1e-10}], "tolerances changed")
    return scope


def references(case, times):
    """mp.mpf(float) is the EXACT binary64 value, not a decimal approximation."""
    key = (case, tuple(times))
    if key in REFERENCE_CACHE:
        return REFERENCE_CACHE[key]
    import mpmath as mp
    _, frequency, damping = CASES[case]
    evaluations = []
    for precision in (80, 100):
        with mp.workdps(precision):
            rows = []
            for t in times:
                phase = mp.mpf(frequency) * mp.mpf(t)
                amp = mp.exp(-mp.mpf(damping) * mp.mpf(t))
                rows.append((amp * mp.cos(phase), amp * mp.sin(phase), phase, amp))
            evaluations.append(rows)
    uncertainty = {name: 0.0 for name in CAPS}
    with mp.workdps(110):
        for low, high in zip(*evaluations):
            uncertainty["state"] = max(uncertainty["state"], float(mp.hypot(low[0]-high[0], low[1]-high[1])))
            uncertainty["phase"] = max(uncertainty["phase"], float(abs(low[2]-high[2])))
            uncertainty["amplitude"] = max(uncertainty["amplitude"], float(abs(low[3]-high[3])/high[3]))
    require(all(uncertainty[k] <= 0.1 * CAPS[k] for k in CAPS), "reference uncertainty exceeds 10% of cap")
    # Scoring uses binary64 math. Include conservative engineering allowances for
    # converting the high-precision reference and 4096 phase summations. These
    # are declared diagnostic allowances, NOT directed interval certificates.
    allowance = {"state": 32*sys.float_info.epsilon,
                 "amplitude": 32*sys.float_info.epsilon,
                 "phase": 64*sys.float_info.epsilon*(1+abs(float(evaluations[1][-1][2])))*len(times)}
    require(all(uncertainty[k] + allowance[k] <= 0.1 * CAPS[k] for k in CAPS), "scoring/reference allowance exceeds cap fraction")
    ref = {"rows": [[float(x) for x in row] for row in evaluations[1]],
           "uncertainty_80_vs_100": uncertainty, "scoring_allowance": allowance,
           "oracle": "mpmath 80/100 dps at exact binary64 times/parameters",
           "reference_status": "independent high precision numeric check; not outward certificate"}
    REFERENCE_CACHE[key] = ref
    return ref


def evaluate_samples(times, states, reference):
    require(len(times) == 4097 and len(states) == 4097, "missing observation(s)")
    require(all(math.isfinite(t) for t in times) and all(b > a for a, b in zip(times, times[1:])), "invalid observation times")
    require(all(len(row) == 2 and all(math.isfinite(x) for x in row) for row in states), "nonfinite/invalid state")
    rows = reference["rows"]
    require(len(rows) == len(times), "reference observation mismatch")
    maxima = {name: 0.0 for name in CAPS}
    unwrapped = math.atan2(states[0][1], states[0][0])
    compensation = 0.0
    masked = []
    phase_values = []
    for i, (state, ref) in enumerate(zip(states, rows)):
        x, y = state
        rx, ry, phase, amplitude = ref
        maxima["state"] = max(maxima["state"], math.hypot(x-rx, y-ry)/max(1.0, amplitude))
        mask = amplitude <= MASK
        if mask:
            masked.append(i)
            phase_values.append(None)
            continue
        norm = math.hypot(x, y)
        require(norm > 0.0 and math.isfinite(norm), "numerical collapse at unmasked observation")
        if i:
            previous = states[i-1]
            require(rows[i-1][3] > MASK, "cannot resume phase after a masked gap")
            expected_increment = phase - rows[i-1][2]
            require(0.0 < expected_increment < math.pi, "analytic phase observation spacing is alias ambiguous")
            # Normalize before cross/dot to avoid spurious underflow/overflow.
            previous_norm = math.hypot(*previous)
            require(previous_norm > 0.0, "previous numerical state collapsed")
            px, py = previous[0]/previous_norm, previous[1]/previous_norm
            nx, ny = x/norm, y/norm
            increment = math.atan2(px*ny-py*nx, px*nx+py*ny)
            require(math.isfinite(increment) and abs(increment-expected_increment) < math.pi/2,
                    "numerical phase increment is alias ambiguous")
            adjusted = increment - compensation
            updated = unwrapped + adjusted
            compensation = (updated-unwrapped)-adjusted
            unwrapped = updated
        maxima["phase"] = max(maxima["phase"], abs(unwrapped-phase))
        maxima["amplitude"] = max(maxima["amplitude"], abs(norm/amplitude-1.0))
        phase_values.append(unwrapped)
    require(len(masked) < len(times), "all phase observations masked")
    checks = {k: maxima[k] + reference["uncertainty_80_vs_100"][k] + reference["scoring_allowance"][k] <= CAPS[k] for k in CAPS}
    return {"status": "PASS_SAMPLED" if all(checks.values()) else "FAILED_ACCURACY",
            "maxima": maxima, "caps": CAPS, "checks": checks,
            "mask_threshold": MASK, "masked_count": len(masked), "masked_indices": masked,
            "unwrapped_phase": phase_values,
            "scope": "4097 sampled observations; cannot detect unobserved extra turns between samples"}


def score(document, case, method, tolerance, deadline=None):
    require(document.get("schema") == "vigilode.frontier-long-phase.v1", "wrong solver schema")
    require((document.get("case"), document.get("method"), document.get("tolerance_index")) ==
            (case, method, tolerance), "solver row identity mismatch")
    cycles, frequency, damping = CASES[case]
    require((document.get("cycles"), document.get("frequency"), document.get("damping"), document.get("threads")) ==
            (cycles, frequency, damping, 1), "solver parameters mismatch")
    expected_tolerance = (0.0, 0.0) if method == "direct-carrier" else ((1e-6, 1e-8) if tolerance == "0" else (1e-8, 1e-10))
    require((document.get("rtol"), document.get("atol")) == expected_tolerance, "solver tolerance mismatch")
    result = document["result"]
    require(result.get("success") is True, "solver did not report complete success")
    times = result["times"]
    require(times == expected_times(case), "solver did not return exact registered physical observations")
    require(result.get("last_accepted_time") == times[-1], "solver did not reach final time")
    require(document.get("observation_count") == 4097 and document.get("initial_state") == [1.0, 0.0], "initial/output contract mismatch")
    if deadline is not None:
        deadline.enter("REFERENCE")
    ref = references(case, times)
    if deadline is not None:
        deadline.enter("SCORING")
    output = evaluate_samples(times, result["states"], ref)
    output["reference"] = {k: v for k, v in ref.items() if k != "rows"}
    require(math.isfinite(result.get("solve_seconds", math.nan)) and result["solve_seconds"] >= 0.0, "missing solve timing")
    return output


def verify_freeze(args, snapshot=None):
    if snapshot is not None:
        for path, expected in snapshot["frozen_file_sha256"].items():
            require(Path(path).is_file() and sha(path) == expected, "frozen source/identity changed: " + path)
        for directory in (BASE/"crates", BASE/"tools", Path(__file__).parent):
            for path in directory.rglob("*"):
                if path.is_file() and path.suffix in (".rs", ".py", ".toml", ".lock"):
                    require(str(path.resolve()) in snapshot["frozen_file_sha256"], "new uncommitted source: " + str(path))
        require(sha(args.binary) == args.binary_sha256, "binary SHA differs from freeze")
        return snapshot
    require(git("rev-parse", "HEAD") == args.source_commit, "source commit differs from freeze")
    require(git("rev-parse", "HEAD^{tree}") == args.source_tree, "source tree differs from freeze")
    relevant = ["crates", "tools", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo", SCRIPT, str(SCOPE.relative_to(BASE))]
    require(not git("diff", "HEAD", "--", *relevant), "uncommitted source/scope changes; timing refused")
    tracked = git("ls-files", "--", *relevant).splitlines()
    for path in [EXAMPLE, SCRIPT, str(SCOPE.relative_to(BASE))]:
        require(path in tracked, "uncommitted required source: " + path)
    for path in git("ls-files", "--others", "--exclude-standard").splitlines():
        if Path(path).suffix in (".rs", ".py", ".toml", ".lock") and (path.startswith(("crates/", "tools/")) or path == SCRIPT):
            raise ValueError("uncommitted source file: " + path)
    require(Path(args.binary).is_file(), "binary missing")
    require(sha(args.binary) == args.binary_sha256, "binary SHA differs from freeze")
    gitdir = Path(git("rev-parse", "--absolute-git-dir"))
    common = gitdir
    if (gitdir/"commondir").exists():
        common = (gitdir/(gitdir/"commondir").read_text().strip()).resolve()
    metadata = [gitdir/"HEAD", gitdir/"index"]
    head = (gitdir/"HEAD").read_text().strip()
    if head.startswith("ref: "):
        ref = common/head[5:]
        metadata.append(ref if ref.exists() else common/"packed-refs")
    sources = [BASE/p for p in tracked if Path(p).suffix in (".rs", ".py", ".toml", ".lock")]
    sources += [SCOPE]
    frozen = {str(p.resolve()): sha(p) for p in sources+metadata if p.is_file()}
    snapshot = {"source_commit": args.source_commit, "source_tree": args.source_tree,
                "binary": str(Path(args.binary).resolve()), "binary_sha256": args.binary_sha256,
                "frozen_file_sha256": frozen, "verification": "initial git validation; subsequent content/HEAD/index/ref checks without extra child processes"}
    return verify_freeze(args, snapshot)


def matrix():
    for case in CASES:
        for tolerance in ("0", "1"):
            for method in ("full-rhs-dopri5", "lawson"):
                yield case, method, tolerance
        yield case, "direct-carrier", "none"


def campaign(args):
    global PROCESS_ACCOUNTING
    require(0 < args.shared_remaining_seconds <= 5400 and 0 < args.shared_remaining_processes <= 500, "invalid shared remaining budget")
    require(0 < args.lp_seconds <= 1200, "LP01 time cap exceeds 1200s")
    cap = min(args.lp_seconds, args.shared_remaining_seconds)
    deadline = Deadline(cap)  # Includes preflight, freeze hashing, oracle and scoring.
    start = time.monotonic()
    PROCESS_ACCOUNTING = {"attempts": 0, "cap": args.shared_remaining_processes,
                          "commands": [], "deadline": deadline}
    accounting = PROCESS_ACCOUNTING
    root = Path(args.output).resolve()
    root.mkdir(parents=True, exist_ok=False)
    records = []
    identity = None
    stop_reason = None
    allowed = None
    scope = None
    thread_env = {"RAYON_NUM_THREADS": "1", "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1", "MKL_NUM_THREADS": "1"}
    deadline.arm()
    try:
        scope = check_scope()
        import mpmath  # noqa: F401
        require(hasattr(os, "sched_getaffinity") and hasattr(os, "sched_setaffinity"), "fixed CPU affinity unsupported")
        allowed = os.sched_getaffinity(0)
        cpu = min(allowed) if args.cpu is None else args.cpu
        require(cpu in allowed, "requested CPU is outside allowed affinity")
        os.sched_setaffinity(0, {cpu})  # Children inherit this exact one-CPU mask.
        deadline.enter("FREEZE")
        identity = verify_freeze(args)
        identity["cpu_affinity"] = [cpu]
        identity["thread_environment"] = thread_env
        write_new(root/"FREEZE.json", identity)
        write_new(root/"SCOPE.json", scope)
    except DeadlineExpired as error:
        stop_reason = "TIMEOUT_" + str(error)
    except ProcessBudgetExhausted:
        stop_reason = "NOT_RUN_PROCESS_BUDGET"
    except (ValueError, ImportError, OSError, subprocess.CalledProcessError) as error:
        stop_reason = "PREFLIGHT_REFUSAL"
        write_new(root/"PREFLIGHT_ERROR.json", {"status": stop_reason, "reason": str(error)})
    # Receipt finalization is administrative; the absolute monotonic end is
    # retained and rechecked/rearmed before any subsequent qualification work.
    deadline.cancel()
    try:
        for case, method, tolerance in matrix():
            warmup_failed = False
            for repeat in range(6):
                key = f"{case}__{method}__t{tolerance}__r{repeat}"
                record = {"row": key, "case": case, "method": method, "tolerance_index": tolerance,
                          "warmup": repeat == 0, "repeat": repeat, "timing_authority": "diagnostic_only",
                          "cpu_affinity": identity.get("cpu_affinity") if identity else None, "thread_environment": thread_env}
                stdout = stderr = b""
                if stop_reason:
                    record["status"] = "NOT_RUN_AFTER_" + stop_reason
                elif warmup_failed:
                    record["status"] = "NOT_RUN_FAILED_WARMUP"
                else:
                    try:
                        deadline.arm()
                        deadline.enter("FREEZE")
                        verify_freeze(args, identity)
                        require(list(os.sched_getaffinity(0)) == identity["cpu_affinity"], "CPU affinity changed")
                        command = [str(Path(args.binary).resolve()), "--case", case, "--method", method, "--tolerance-index", tolerance]
                        record["command"] = command
                        deadline.enter("SOLVER")
                        charge_process(command)
                        # No intervening freeze/reference work between this remaining
                        # calculation and launch. Global timer also covers launch.
                        record["process_cap_seconds"] = min(90.0, deadline.remaining())
                        begin = time.monotonic()
                        process = None
                        record["rss_limit_bytes"] = 1073741824
                        record["rss_sampling_interval_seconds"] = 0.02
                        record["rss_monitor"] = "sum sampled /proc VmRSS over process group and discovered descendants; shared pages may count twice"
                        try:
                            stdout, stderr, peak, peak_members, returncode = owned_monitored_process(
                                command, {**os.environ, **thread_env}, record["process_cap_seconds"], deadline)
                            record["peak_sampled_rss_bytes"] = peak
                            record["peak_resident_processes"] = peak_members
                            record["exit_code"] = returncode
                            record["status"] = "SOLVER_EXIT_FAILURE" if returncode else "OUTPUT_PENDING"
                        except RSSLimitExceeded as error:
                            stdout, stderr = error.stdout, error.stderr
                            record["exit_code"] = error.child_returncode
                            record["status"] = "RSS_LIMIT"
                            record["peak_sampled_rss_bytes"] = error.peak_rss_bytes
                            record["peak_resident_processes"] = error.peak_resident_processes
                            preserve_owner_failure(record, error)
                        except subprocess.TimeoutExpired as error:
                            stdout, stderr = error.stdout or b"", error.stderr or b""
                            record["exit_code"] = error.child_returncode
                            record["status"] = "TIMEOUT_SOLVER_PROCESS"
                            record["peak_sampled_rss_bytes"] = getattr(error, "peak_rss_bytes", None)
                            preserve_owner_failure(record, error)
                        except DeadlineExpired:
                            raise
                        except OSError as error:
                            stdout = getattr(error, "stdout", stdout)
                            stderr = getattr(error, "stderr", stderr)
                            record["exit_code"] = None
                            record["status"] = "RSS_MONITOR_ERROR" if getattr(error, "launched", False) else "LAUNCH_ERROR"
                            record["reason"] = str(error)
                            preserve_owner_failure(record, error)
                        record["process_elapsed_seconds"] = time.monotonic()-begin
                        # Raw solver bytes are persisted before reference/scoring.
                        (root/(key+".stdout")).write_bytes(stdout)
                        (root/(key+".stderr")).write_bytes(stderr)
                        if resume_post_child_freeze(record, deadline):
                            verify_freeze(args, identity)
                        else:
                            stop_reason = "CLEANUP_FAILURE"
                            deadline.cancel()
                        if record["status"] == "OUTPUT_PENDING":
                            deadline.enter("SCORING")
                            document = json.loads(stdout)
                            scored = score(document, case, method, tolerance, deadline)
                            record["status"] = scored["status"]
                            record["solve_seconds"] = document["result"]["solve_seconds"]
                            write_new(root/(key+".score.json"), scored)
                    except DeadlineExpired as error:
                        stdout = getattr(error, "stdout", stdout)
                        stderr = getattr(error, "stderr", stderr)
                        record["status"] = "TIMEOUT_" + str(error)
                        record["exit_code"] = record.get("exit_code")
                        preserve_owner_failure(record, error)
                        stop_reason = record["status"]
                        deadline.cancel()  # Only receipt/final NOT_RUN assembly follows.
                    except ProcessBudgetExhausted:
                        record["status"] = "NOT_RUN_PROCESS_BUDGET"
                        stop_reason = record["status"]
                        deadline.cancel()
                    except (ValueError, KeyError, TypeError, OverflowError, OSError) as error:
                        stdout = getattr(error, "stdout", stdout)
                        stderr = getattr(error, "stderr", stderr)
                        record["status"] = "RSS_MONITOR_ERROR" if deadline.phase == "SOLVER" else "SCORING_OR_FREEZE_REFUSAL"
                        record["reason"] = str(error)
                        cleanup_failed = preserve_owner_failure(record, error)
                        if cleanup_failed or deadline.phase == "FREEZE":
                            stop_reason = record["status"]
                            deadline.cancel()
                    if repeat == 0 and record["status"] != "PASS_SAMPLED":
                        warmup_failed = True
                deadline.cancel()
                # Always emit raw files, including launch/preflight/timeout rows.
                for suffix, data in (("stdout", stdout), ("stderr", stderr)):
                    path = root/(key+"."+suffix)
                    if not path.exists():
                        path.write_bytes(data)
                    record[suffix+"_sha256"] = sha(path)
                write_new(root/(key+".receipt.json"), record)
                records.append(record)
                print(json.dumps({"row": key, "status": record["status"], "all_child_process_attempts": PROCESS_ACCOUNTING["attempts"]}), flush=True)
    finally:
        deadline.cancel()
        if allowed is not None:
            os.sched_setaffinity(0, allowed)
        PROCESS_ACCOUNTING = None
    summary = {"schema": "vigilode.lp01-linear-campaign.v1", "freeze": identity,
               "timing_authority": "diagnostic_only", "all_child_process_attempts": 0,
               "elapsed_seconds_including_finalize": time.monotonic()-start, "lp_cap_seconds": cap,
               "stop_reason": stop_reason, "rows": records,
               "production_scope": "HOLD; sampled three-case qualification only",
               "negative_controls": {"nonlinear_amplitude": "NOT_RUN_REQUIRES_CONTRACT", "noncommuting_residual": "NOT_RUN_REQUIRES_CONTRACT", "exact_carrier_nonhomogeneous_controls": "UNSUPPORTED"}}
    summary["all_child_process_attempts"] = accounting["attempts"]
    summary["child_commands"] = accounting["commands"]
    write_new(root/"SUMMARY.json", summary)
    return 0 if all(r["status"] == "PASS_SAMPLED" for r in records) else 1


def self_test():
    check_scope()
    case = "normal_1000_f1"
    times = expected_times(case)
    reference = references(case, times)
    exact = [row[:2] for row in reference["rows"]]
    require(evaluate_samples(times, exact, reference)["status"] == "PASS_SAMPLED", "exact oracle self-test failed")
    drift = [[x*1.00002, y*1.00002] for x, y in exact]
    require(not evaluate_samples(times, drift, reference)["checks"]["amplitude"], "amplitude negative control escaped")
    phase_drift = []
    for i, (x, y) in enumerate(exact):
        phase = .002*math.sin(2*math.pi*i/4096)
        c, s = math.cos(phase), math.sin(phase)
        phase_drift.append([c*x-s*y, s*x+c*y])
    require(not evaluate_samples(times, phase_drift, reference)["checks"]["phase"], "interior phase drift escaped endpoint test")
    collapse = [row[:] for row in exact]
    collapse[100] = [0.0, 0.0]
    for name, bad_times, bad_states, bad_reference in [
        ("collapse", times, collapse, reference),
        ("missing_observation", times[:-1], exact[:-1], reference),
        ("nonfinite", times, exact[:1]+[[math.nan, 0.0]]+exact[2:], reference),
        ("undersampled", times, exact, {**reference, "rows": [[x,y,phase*3,amp] for x,y,phase,amp in reference["rows"]]}),
    ]:
        try:
            evaluate_samples(bad_times, bad_states, bad_reference)
        except ValueError:
            continue
        raise AssertionError(name+" should refuse")
    # A phase mask is determined only by reference amplitude; masked collapse is
    # allowed and never contributes an invented phase value.
    masked = {**reference, "rows": [row[:] for row in reference["rows"]]}
    masked["rows"][-1] = [0.0, 0.0, masked["rows"][-1][2], 0.0]
    tail = [row[:] for row in exact]
    tail[-1] = [0.0, 0.0]
    mask_result = evaluate_samples(times, tail, masked)
    require(mask_result["masked_count"] == 1 and mask_result["unwrapped_phase"][-1] is None, "mask contract failed")
    require(len(list(matrix())) == 15, "run matrix changed")
    from types import SimpleNamespace
    try:
        verify_freeze(SimpleNamespace(source_commit="0"*40, source_tree="0"*40,
                      binary="/nonexistent-lp01-binary", binary_sha256="0"*64))
    except ValueError as error:
        require("source commit differs" in str(error), "unexpected freeze refusal")
    else:
        raise AssertionError("unfrozen source was accepted")
    expired = Deadline(1)
    expired.end = time.monotonic() - 1
    try:
        expired.enter("REFERENCE")
    except DeadlineExpired as error:
        require(str(error) == "REFERENCE", "reference deadline lost its type")
    else:
        raise AssertionError("expired reference was allowed")
    timer = Deadline(1)
    timer.arm()
    try:
        signal.raise_signal(signal.SIGALRM)
    except DeadlineExpired as error:
        require(str(error) == "PREFLIGHT", "signal deadline lost its phase")
    finally:
        timer.cancel()
    child_source = "import subprocess,sys; print('rss-parent-ready',flush=True); p=subprocess.Popen([sys.executable,'-c',\"import time; x=bytearray(64*1024*1024); print('rss-child-ready',flush=True); time.sleep(10)\"]); p.wait()"
    child = subprocess.Popen([sys.executable, "-c", child_source], stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, start_new_session=True)
    try:
        monitored_communicate(child, 4.0, Deadline(5.0), rss_limit=32*1024*1024)
    except RSSLimitExceeded as error:
        require(error.peak_rss_bytes > 32*1024*1024, "RSS refusal lacks measured evidence")
        require(error.peak_resident_processes >= 2, "descendant RSS was not included")
        require(b"rss-parent-ready" in error.stdout, "RSS refusal lost raw stdout")
        require(child.poll() is not None, "RSS refusal failed to reap child")
    else:
        raise AssertionError("RSS-limited controlled child escaped")
    # Expiration immediately after successful Popen must still kill/reap it.
    class ExpiredHandoff(Deadline):
        calls = 0
        def remaining(self):
            self.calls += 1
            if self.calls > 1:
                raise DeadlineExpired("SOLVER_HANDOFF")
            return 2.0
    command = [sys.executable, "-c", "import time; print('controlled-ready',flush=True); time.sleep(10)"]
    try:
        owned_monitored_process(command, os.environ.copy(), 2.0, ExpiredHandoff(3))
    except DeadlineExpired as error:
        require(error.launched and error.child_returncode is not None, "expired handoff leaked its child")
        require(str(error) == "SOLVER_HANDOFF", "cleanup replaced original handoff failure")
    else:
        raise AssertionError("expired child handoff escaped")
    original_selector = selectors.DefaultSelector
    def failed_selector():
        raise OSError("controlled selector setup failure")
    selectors.DefaultSelector = failed_selector
    resumed_deadline = Deadline(3)
    resumed_deadline.arm()
    try:
        try:
            owned_monitored_process(command, os.environ.copy(), 2.0, resumed_deadline)
        except OSError as error:
            require(error.launched and error.child_returncode is not None, "selector setup failure leaked child")
            require(str(error) == "controlled selector setup failure", "cleanup replaced original setup error")
            row = {"status": "RSS_MONITOR_ERROR"}
            require(not preserve_owner_failure(row, error), "confirmed teardown falsely lost its child")
            require(signal.getitimer(signal.ITIMER_REAL)[0] == 0, "teardown did not cancel timer")
            require(resume_post_child_freeze(row, resumed_deadline), "confirmed teardown cannot resume freeze")
            require(signal.getitimer(signal.ITIMER_REAL)[0] > 0, "post-child freeze lacks hard deadline")
            try:
                signal.raise_signal(signal.SIGALRM)
            except DeadlineExpired as timeout:
                require(str(timeout) == "FREEZE", "rearmed freeze lost deadline phase")
        else:
            raise AssertionError("selector setup failure escaped")
    finally:
        resumed_deadline.cancel()
        selectors.DefaultSelector = original_selector
    # Inject loss of cleanup confirmation only after actually reaping the local
    # child. This tests fail-closed reporting without leaving a process behind.
    original_teardown = teardown_owned_process
    def failed_cleanup(process, known=None):
        original_teardown(process, known)
        raise RuntimeError("controlled cleanup confirmation failure")
    globals()["teardown_owned_process"] = failed_cleanup
    selectors.DefaultSelector = failed_selector
    try:
        try:
            owned_monitored_process(command, os.environ.copy(), 2.0, Deadline(3))
        except OSError as error:
            require(error.child_returncode is not None, "controlled test failed to reap its real child")
            row = {"status": "RSS_MONITOR_ERROR"}
            require(preserve_owner_failure(row, error), "cleanup uncertainty did not stop campaign")
            require(row["status"] == "CLEANUP_FAILURE" and "cleanup_error" in row, "cleanup failure lost raw evidence")
            require(row["original_status"] == "RSS_MONITOR_ERROR", "original failure classification lost")
            require(json.loads(json.dumps(row))["cleanup_error"] == row["cleanup_error"], "cleanup error is not persistable")
            require(not resume_post_child_freeze(row, Deadline(3)), "cleanup failure permitted further qualification")
        else:
            raise AssertionError("controlled cleanup confirmation failure escaped")
    finally:
        globals()["teardown_owned_process"] = original_teardown
        selectors.DefaultSelector = original_selector
    print(json.dumps({"status": "PASS", "self_tests": 17, "controlled_test_child_processes": 5, "solver_processes_started": 0,
                      "scope": "scoring/refusal tests only; no Cargo, solver run, or timing"}))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--binary")
    parser.add_argument("--binary-sha256")
    parser.add_argument("--source-commit")
    parser.add_argument("--source-tree")
    parser.add_argument("--output")
    parser.add_argument("--cpu", type=int, help="one allowed CPU; default smallest allowed CPU")
    parser.add_argument("--lp-seconds", type=float, default=1200)
    parser.add_argument("--shared-remaining-seconds", type=float)
    parser.add_argument("--shared-remaining-processes", type=int)
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    for key in ("binary", "binary_sha256", "source_commit", "source_tree", "output", "shared_remaining_seconds", "shared_remaining_processes"):
        require(getattr(args, key) is not None, "missing explicit campaign input: "+key)
    return campaign(args)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, ImportError, subprocess.CalledProcessError) as error:
        print(json.dumps({"status": "PREFLIGHT_OR_FREEZE_REFUSAL", "reason": str(error)}), file=sys.stderr)
        sys.exit(2)
