"""Execution of one run spec as a pinned child process, and the Julia server.

A run spec is {"run_id", "case", "arm", "rtol", "mode" (timing|allocation),
"core", "phase"}. Rows carry the process receipt (wait4), the solver's own
status, its in-process solve time (timing_authority diagnostic_only), its
counters and the CAMP-relative path + sha256 of its raw stdout; trajectories
stay in the raw files and are evaluated by analyze.py. Status OK alone is
never success.

Every run's timeout is min(its own cap, time left before the shared deadline
minus a reserve); a run that cannot start in time is NOT_RUN, and a run killed
at a deadline-capped timeout is typed DEADLINE (its own cap reached: TIMEOUT).
Every child session (solvers, Julia servers, diffsol) is registered for the
campaign-wide cleanup (common.kill_registered).
"""
import json
import os
import subprocess
import threading
import time
from pathlib import Path

import scenarios as S
from common import Deadline, child_env, load_stdout_json, run_child, sha256_file, track, untrack, utc_now

RUNNER_BIN = {"fv": "lrs_photon_fv_run", "block": "lrs_photon_block_run", "chem": "lrs_photon_chem_run"}
RESERVE_S = 15.0     # every run must end this long before the shared deadline
MIN_RUN_S = 5.0      # below this budget a run is not started (NOT_RUN)
TIMING = "diagnostic_only"


class Camp:
    """Campaign directory layout."""

    def __init__(self, root):
        self.root = Path(root).resolve()
        self.bin = self.root / "bin"
        self.raw = self.root / "raw"
        self.results = self.root / "results"
        self.logs = self.root / "logs"
        self.tmp = self.root / "tmp"
        self.harness = Path(__file__).resolve().parent
        for p in (self.raw, self.results, self.logs, self.tmp):
            p.mkdir(parents=True, exist_ok=True)
        self.julia = os.environ.get("NCPC1_JULIA")  # path to the julia binary (None: Julia HOLD)
        self.julia_project = os.environ.get("NCPC1_JULIA_PROJECT")
        self.julia_depot = os.environ.get("NCPC1_JULIA_DEPOT")
        self.diffsol = self.bin / "vigilode_bassrei_diffsol"

    def raw_dir(self, phase):
        p = self.raw / phase
        p.mkdir(parents=True, exist_ok=True)
        return p

    def rel(self, p):
        try:
            return str(Path(p).resolve().relative_to(self.root))
        except ValueError:
            return str(p)

    def julia_env(self):
        """The private depot copy as the single depot entry (never stacked), offline."""
        if not self.julia_depot:
            raise RuntimeError("no private Julia depot (NCPC1_JULIA_DEPOT unset): Julia is HOLD")
        env = child_env({"JULIA_PKG_OFFLINE": "true", "TMPDIR": str(self.tmp)})
        env["JULIA_DEPOT_PATH"] = self.julia_depot   # one entry, the private copy
        return env

    def julia_argv(self, *args):
        argv = [self.julia, "--startup-file=no", "--history-file=no"]
        if self.julia_project:
            argv.append(f"--project={self.julia_project}")
        return argv + [str(a) for a in args]


def pinned(argv, core):
    return ["taskset", "-c", str(core)] + [str(a) for a in argv] if core is not None else [str(a) for a in argv]


def _base_row(spec):
    row = {k: spec.get(k) for k in ("run_id", "phase", "case", "arm", "rtol", "mode", "core", "block_id",
                                     "block_index", "warmup", "position", "order", "pair")}
    row["family"] = S.FAMILY_OF_ARM.get(spec["arm"], spec["arm"])
    row["started_utc"] = utc_now()
    row["timing_authority"] = TIMING
    return row


def _killed_status(info, capped):
    return "DEADLINE" if capped else "TIMEOUT"


def _finish(camp, row, info, capped):
    row.update(process=info, effective_timeout_s=info.get("effective_timeout_s"), deadline_capped=bool(capped))
    out, err = load_stdout_json(info)
    if out is None:
        row.update(status=_killed_status(info, capped) if info["timed_out"] else "OUTPUT_PARSE_FAILURE",
                   parse_error=err, censored=bool(info["timed_out"]))
        return row, None
    row.update(stdout_rel=camp.rel(info["stdout_path"]), stdout_path=info["stdout_path"],
               stdout_sha256=info["stdout_sha256"])
    return row, out


def run_vigilode(camp, spec, timeout, capped):
    kind, req = S.vigilode_request(spec["case"], spec["arm"], spec["rtol"], **spec.get("extra", {}))
    raw = camp.raw_dir(spec["phase"])
    rp = raw / f"{spec['run_id']}.request.json"
    rp.write_text(json.dumps(req, allow_nan=False))
    argv = [camp.bin / RUNNER_BIN[kind], "--request", rp, "--measurement-mode", spec.get("mode", "timing")]
    info = run_child(pinned(argv, spec.get("core")), raw / f"{spec['run_id']}.json", raw / f"{spec['run_id']}.stderr",
                     env=child_env(), timeout=timeout)
    row = _base_row(spec)
    row.update(request_rel=camp.rel(rp), request_sha256=sha256_file(rp), runner=RUNNER_BIN[kind],
               extra=spec.get("extra"))
    row, out = _finish(camp, row, info, capped)
    if out is None:
        return row
    timing = out.get("timing") or {}
    row.update(status=out.get("status"), reason=(out.get("reason") or "")[:300], counters=out.get("counters"),
               details=out.get("details"), allocations=out.get("allocations"),
               solve_seconds=(timing.get("elapsed_ns") or 0) * 1e-9 if timing.get("elapsed_ns") is not None else None,
               timing_scope=timing.get("timing_scope"))
    return row


def run_python(camp, spec, timeout, capped):
    raw = camp.raw_dir(spec["phase"])
    rp = raw / f"{spec['run_id']}.request.json"
    rp.write_text(json.dumps({"case": spec["case"], "arm": spec["arm"], "rtol": spec["rtol"]}))
    argv = ["python3", "-s", camp.harness / "py_worker.py", rp]  # -s: system scipy 1.11.4 (NCP and local)
    info = run_child(pinned(argv, spec.get("core")), raw / f"{spec['run_id']}.json", raw / f"{spec['run_id']}.stderr",
                     env=child_env(), timeout=timeout)
    row = _base_row(spec)
    row.update(request_rel=camp.rel(rp))
    row, out = _finish(camp, row, info, capped)
    if out is None:
        return row
    row.update(status=out.get("status"), reason=(out.get("reason") or "")[:300] if out.get("reason") else None,
               counters=out.get("counters"), solve_seconds=out.get("solve_seconds"),
               setup_seconds=out.get("setup_seconds"), timing_scope="integration call only (scipy)")
    return row


def run_diffsol(camp, spec, timeout, capped):
    raw = camp.raw_dir(spec["phase"])
    rp = raw / f"{spec['run_id']}.request.json"
    d = S.case_data(spec["case"])
    req = {"model": S.model_description(spec["case"]), "algorithm": "bdf" if spec["arm"] == "ds_bdf_dense" else "tsit45",
           "rtol": spec["rtol"], "atol": S.atol_of(d, spec["rtol"]), "repetitions": spec.get("repetitions", 1)}
    rp.write_text(json.dumps(req, allow_nan=False))
    row = _base_row(spec)
    if not camp.diffsol.exists():
        row.update(status="HOLD", reason="diffsol adapter not built (setup HOLD)")
        return row
    info = run_child(pinned([camp.diffsol, "--request", rp], spec.get("core")), raw / f"{spec['run_id']}.json",
                     raw / f"{spec['run_id']}.stderr", env=child_env(), timeout=timeout)
    row.update(request_rel=camp.rel(rp))
    row, out = _finish(camp, row, info, capped)
    if out is None:
        return row
    row.update(status=out.get("status"), reason=(out.get("reason") or "")[:300] if out.get("reason") else None,
               counters=out.get("counters"), solve_seconds=out.get("solve_seconds"),
               timing_scope="solver construction + solve_dense (diffsol)")
    return row


class JuliaServer:
    """One pinned Julia process serving requests line by line (solver.jl --server).

    Each request is a JSON file path; the server writes the result JSON to the
    given output path and answers one line "DONE <id> <status>". The READY wait
    is clipped to the shared deadline; the server session is registered for the
    campaign-wide cleanup.
    """

    def __init__(self, camp, core, tag, phase, ready_cap=1500.0):
        self.camp, self.core, self.tag = camp, core, tag
        raw = camp.raw_dir(phase)
        self.log_path = raw / f"julia_server_{tag}.stderr"
        self.log = open(self.log_path, "wb")
        argv = camp.julia_argv(camp.harness / "julia" / "solver.jl", "--server")
        self.t_start = time.monotonic()
        wait, self.ready_capped = Deadline().budget(ready_cap, RESERVE_S)
        self.p = subprocess.Popen(pinned(argv, core), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=self.log, text=True, bufsize=1, start_new_session=True, env=camp.julia_env())
        track(self.p, argv)
        self.ready_line = self._readline(timeout=max(wait, 1.0))
        self.ready_seconds = time.monotonic() - self.t_start
        if not self.alive:
            self.close()

    def _readline(self, timeout):
        box = {}

        def rd():
            box["line"] = self.p.stdout.readline()
        th = threading.Thread(target=rd, daemon=True)
        th.start()
        th.join(timeout)
        if th.is_alive():
            return None
        return (box.get("line") or "").strip()

    def stderr_tail(self, n=600):
        try:
            self.log.flush()
            return self.log_path.read_text(errors="replace")[-n:]
        except Exception:  # noqa: BLE001
            return ""

    @property
    def alive(self):
        return self.p.poll() is None and bool(self.ready_line) and self.ready_line.startswith("READY")

    def request(self, req, req_path, out_path, timeout):
        Path(req_path).write_text(json.dumps(dict(req, output_path=str(out_path)), allow_nan=False))
        t0 = time.monotonic_ns()
        try:
            self.p.stdin.write(str(req_path) + "\n")
            self.p.stdin.flush()
        except (BrokenPipeError, OSError) as e:
            return {"answer": None, "error": repr(e), "wall_ns": 0}
        line = self._readline(timeout)
        return {"answer": line, "wall_ns": time.monotonic_ns() - t0, "timed_out": line is None}

    def close(self):
        try:
            if self.p.poll() is None:
                self.p.stdin.write("QUIT\n")
                self.p.stdin.flush()
                self.p.wait(timeout=10)
        except Exception:  # noqa: BLE001
            pass
        if self.p.poll() is None:
            try:
                os.killpg(self.p.pid, 9)
            except ProcessLookupError:
                pass
            try:
                self.p.wait(timeout=10)
            except Exception:  # noqa: BLE001
                pass
        untrack(self.p)
        try:
            self.log.close()
        except Exception:  # noqa: BLE001
            pass


def julia_request(spec, timeout):
    d = S.case_data(spec["case"])
    return {"id": spec["run_id"], "kind": "solve", "solver": spec["arm"], "model": S.model_description(spec["case"]),
            "rtol": spec["rtol"], "atol": S.atol_of(d, spec["rtol"]), "maxiters": 10 ** 7,
            "timeout_s": timeout}


def run_julia(server, camp, spec, timeout, capped):
    raw = camp.raw_dir(spec["phase"])
    row = _base_row(spec)
    if server is None or not server.alive:
        row.update(status="HOLD", reason="Julia server unavailable (setup HOLD or crashed)")
        return row
    out_path = raw / f"{spec['run_id']}.json"
    # the in-server deadline callback stops the solve at `timeout`; the client waits 30 s longer
    ans = server.request(julia_request(spec, timeout), raw / f"{spec['run_id']}.request.json", out_path,
                         timeout + 30.0)
    row.update(julia_answer=ans.get("answer"), julia_wall_ns=ans.get("wall_ns"), julia_server=server.tag,
               effective_timeout_s=timeout, deadline_capped=bool(capped))
    if ans.get("timed_out"):
        row.update(status=_killed_status({}, capped), censored=True, reason="no answer from the Julia server")
        server.close()
        return row
    if not out_path.exists():
        row.update(status="ADAPTER_FAILURE", reason=(ans.get("answer") or "")[:300])
        return row
    out, err = load_stdout_json({"stdout_path": str(out_path)})
    if out is None:
        row.update(status="OUTPUT_PARSE_FAILURE", parse_error=err)
        return row
    status = out.get("status")
    if status == "TIMEOUT" and capped:
        status = "DEADLINE"
    row.update(status=status, reason=(out.get("reason") or "")[:300] if out.get("reason") else None,
               counters=out.get("counters"), solve_seconds=out.get("solve_seconds"), retcode=out.get("retcode"),
               warmup_seconds=out.get("warmup_seconds"), timing_scope="solve() call only (Julia, after warm-up)",
               stdout_rel=camp.rel(out_path), stdout_path=str(out_path), stdout_sha256=sha256_file(out_path))
    return row


def execute(camp, spec, julia_server=None, cap=None):
    """Run one spec with timeout = min(cap, deadline - reserve); NOT_RUN if no time is left."""
    fam = S.FAMILY_OF_ARM.get(spec["arm"])
    default_cap = 600.0 if spec["arm"] in S.VIGILODE_ARMS else 900.0
    timeout, capped = Deadline().budget(float(cap or default_cap), RESERVE_S)
    if timeout < MIN_RUN_S:
        row = _base_row(spec)
        row.update(status="NOT_RUN", reason="campaign deadline", effective_timeout_s=timeout, deadline_capped=True)
        return row
    if spec["arm"] in S.VIGILODE_ARMS:
        return run_vigilode(camp, spec, timeout, capped)
    if spec["arm"].startswith("py_"):
        return run_python(camp, spec, timeout, capped)
    if fam == "diffsol":
        return run_diffsol(camp, spec, timeout, capped)
    if fam == "julia":
        return run_julia(julia_server, camp, spec, timeout, capped)
    row = _base_row(spec)
    row.update(status="REFUSED", reason="unknown arm")
    return row
