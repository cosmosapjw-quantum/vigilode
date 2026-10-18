"""HPC admission and affinity checks with fake jobs only."""
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from component_benchmark import hpc


class HpcTest(unittest.TestCase):
    def resources(self, cpus=64, gib=112):
        allowed=tuple(range(cpus))
        return hpc.Resources("fixture",allowed,allowed[8:][:56],allowed[:8],cpus,
                             128*hpc.GIB,128*hpc.GIB,None,None,gib*hpc.GIB,
                             min(56,cpus-8))

    def test_cpuset_cgroup_and_memory_reserves(self):
        self.assertEqual(hpc.parse_cpu_list("0-3,8,10-11"),(0,1,2,3,8,10,11))
        real_read=hpc._read
        def fake_read(path):
            path=str(path)
            if path.endswith("cpuset.cpus.effective"):return "0-63"
            if path.endswith("cpu.max"):return "640000 10000"
            if path.endswith("memory.max"):return str(128*hpc.GIB)
            if path.endswith("memory.current"):return str(4*hpc.GIB)
            return real_read(path)
        with patch.object(hpc.os,"sched_getaffinity",return_value=set(range(64))), patch.object(hpc,"_read",side_effect=fake_read), patch.object(hpc,"_mem_available",return_value=120*hpc.GIB), patch.object(hpc,"_cgroup_v2_paths",return_value=(Path('/sys/fs/cgroup'),)):
            r=hpc.detect_resources()
        self.assertEqual(len(r.worker_cpus),56)
        self.assertEqual(r.memory_budget_bytes,104*hpc.GIB)
        self.assertEqual(r.reserved_cpus,tuple(range(8)))
        self.assertEqual(hpc.demand_for({"solver_id":"julia-tsit5"}).memory_bytes,2*hpc.GIB)
        self.assertEqual(hpc.demand_for({"solver_id":"native-rodas5p-fast"}).memory_bytes,hpc.GIB//2)
        with patch.object(hpc.os,"sched_getaffinity",return_value=set(range(8))), patch.object(hpc,"_read",return_value=None), patch.object(hpc,"_cgroup_v2_paths",return_value=None):
            with self.assertRaisesRegex(RuntimeError,"HPC_CPU_RESERVE_UNAVAILABLE"):
                hpc.detect_resources()

    def test_nested_cgroup_ancestor_limits_and_current_events(self):
        mount=Path('/sys/fs/cgroup')
        parent=mount/'user.slice'
        current=parent/'session.scope'
        values={
            '/proc/self/cgroup':'0::/user.slice/session.scope',
            '/proc/self/mountinfo':'1 0 0:1 / /sys/fs/cgroup rw - cgroup2 cgroup rw',
            str(mount/'cpuset.cpus.effective'):'0-63',
            str(parent/'cpuset.cpus.effective'):'0-31',
            str(current/'cpuset.cpus.effective'):'0-31',
            str(mount/'cpu.max'):'max 100000',
            str(parent/'cpu.max'):'2000000 100000',
            str(current/'cpu.max'):'max 100000',
            str(mount/'memory.max'):'max',
            str(parent/'memory.max'):str(128*hpc.GIB),
            str(parent/'memory.current'):str(8*hpc.GIB),
            str(current/'memory.max'):str(100*hpc.GIB),
            str(current/'memory.current'):str(10*hpc.GIB),
            str(current/'memory.events'):'low 0\noom 0\noom_kill 0\n'}
        with patch.object(hpc,'_read',side_effect=lambda path:values.get(str(path))), \
             patch.object(hpc.os,'sched_getaffinity',return_value=set(range(64))), \
             patch.object(hpc,'_mem_available',return_value=128*hpc.GIB), \
             patch.object(hpc,'_read_raw',side_effect=lambda path:values.get(str(path))):
            self.assertEqual(hpc._cgroup_v2_paths(),(current,parent,mount))
            resource=hpc.detect_resources()
            sample=hpc._memory_events_sample()
        self.assertEqual(resource.allowed_cpus,tuple(range(32)))
        self.assertEqual(resource.cpu_quota_cores,20)
        self.assertEqual(resource.memory_budget_bytes,74*hpc.GIB)
        self.assertEqual(sample['path'],str(current/'memory.events'))
        self.assertEqual(sample['raw'],'low 0\noom 0\noom_kill 0\n')
        self.assertEqual(sample['counters']['oom_kill'],0)
        values[str(current/'memory.events')]='low 0\noom 0\n'
        with patch.object(hpc,'_read',side_effect=lambda path:values.get(str(path))), \
             patch.object(hpc,'_read_raw',side_effect=lambda path:values.get(str(path))):
            self.assertIsNone(hpc._memory_events_sample()['counters'])
        del values[str(parent/'memory.current')]
        with patch.object(hpc,'_read',side_effect=lambda path:values.get(str(path))):
            with self.assertRaisesRegex(RuntimeError,'HPC_CGROUP_MEMORY_CURRENT_UNAVAILABLE'):
                hpc._cgroup_memory()

    def test_missing_oom_events_remain_unknown(self):
        with patch.object(hpc,"_cgroup_v2_paths",return_value=(Path('/fixture'),)), \
             patch.object(hpc,"_read_raw",return_value=None):
            self.assertIsNone(hpc._memory_events())
        with patch.object(hpc,"_cgroup_v2_paths",return_value=(Path('/fixture'),)), \
             patch.object(hpc,"_read_raw",return_value="low 0\n"):
            self.assertIsNone(hpc._memory_events())
        self.assertIsNone(hpc._oom_delta(None,{"oom":0,"oom_kill":0}))
        self.assertIsNone(hpc._oom_delta({"oom":0,"oom_kill":0},None))
        self.assertIsNone(hpc._oom_delta({"oom":1,"oom_kill":0},{"oom":0,"oom_kill":0}))
        self.assertEqual(hpc._oom_delta({"oom":1,"oom_kill":2},{"oom":1,"oom_kill":2}),
                         {"oom":0,"oom_kill":0})

    def test_heterogeneous_disjoint_slots_and_memory(self):
        resources=self.resources(cpus=16,gib=3)
        pool=hpc.ResourcePool(resources)
        jobs=[{"id":f"native{i}","solver_id":"native-fast"} for i in range(5)]
        jobs += [{"id":"julia","solver_id":"julia-tsit5"}]
        observations=[]
        import threading,time
        lock=threading.Lock(); active=[]
        def worker(job,allocation,tracker):
            with lock:
                assert set(allocation.cpus).isdisjoint(set().union(*(set(x.cpus) for x in active)))
                assert sum(x.memory_bytes for x in active)+allocation.memory_bytes<=resources.memory_budget_bytes
                active.append(allocation)
                observations.append((job["id"],allocation.cpus))
            time.sleep(0.02)
            with lock:active.remove(allocation)
            return job["id"]
        result=pool.run(jobs,worker,poll_seconds=0.01)
        self.assertEqual(set(result),{x["id"] for x in jobs})
        self.assertEqual(len(observations),6)
        with self.assertRaisesRegex(RuntimeError,"HPC_JOB_EXCEEDS_ADMITTED_RESOURCES"):
            hpc.ResourcePool(self.resources(cpus=16,gib=1)).run([{"solver_id":"julia-tsit5"}],worker)

    def test_actual_parallel_thread_binding_and_timeout_fake_only(self):
        job={"solver_id":"native-pexprb54s4","request":{"solver_config":{"threads":4}}}
        self.assertEqual(hpc.job_threads(job,{"threads":4}),4)
        with self.assertRaisesRegex(ValueError,"SOLVER_THREAD_CONFIG_MISMATCH"):
            hpc.job_threads(job,{"threads":2})
        alloc=hpc.Allocation(tuple(sorted(os.sched_getaffinity(0))[:1]),hpc.GIB//2,"native")
        env=hpc.thread_environment({"PATH":os.environ.get("PATH","")},alloc)
        self.assertEqual(env["OMP_NUM_THREADS"],"1")
        with tempfile.TemporaryDirectory() as d:
            script=Path(d)/"fake.py"
            script.write_text("import time; time.sleep(2)\n")
            result=hpc.run_process([sys.executable,str(script)],allocation=alloc,
                                   tracker=hpc.ProcessTracker(),env=env,timeout=0.1)
            self.assertTrue(result["timed_out"])
            self.assertLess(result["elapsed_seconds"],1)
            self.assertEqual(result["affinity_cpus"],list(alloc.cpus))

    def test_telemetry_start_periodic_final(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/"telemetry.jsonl"
            pool=hpc.ResourcePool(self.resources(cpus=16,gib=2),telemetry_path=path)
            rows=pool.run([{"solver_id":"native-fast"}],lambda job,alloc,tracker:alloc.cpus,poll_seconds=0.01)
            self.assertEqual(len(rows),1)
            import json
            lines=[json.loads(x) for x in path.read_text().splitlines()]
            self.assertGreaterEqual(len(lines),3)
            self.assertEqual(lines[0]["queued"],1)
            self.assertEqual(lines[-1]["completed"],1)
            self.assertIn("cpu_steal_fraction",lines[-1])

    def test_screening_hpc_fake_adapter_and_confirmation_hold(self):
        import json,platform
        from component_benchmark.budget import reserve
        from component_benchmark.runner import make_plan,run_plan,source_binding
        from test_component_benchmark_protocol import CASE
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)/"source"
            (root/"tools").mkdir(parents=True)
            (root/"crates").mkdir()
            (root/"Cargo.toml").write_text("[workspace]\n")
            (root/"Cargo.lock").write_text("version = 3\n")
            (root/"tools"/"component_benchmark.py").write_text("# fixture\n")
            script=Path(d)/"fake_adapter.py"
            script.write_text("import json,sys\nc=json.load(open(sys.argv[-1]))['case']\nprint(json.dumps({'status':'OK','times':c['output_times'],'states':[[1],[2]],'counters':None,'timing':{'timed_seconds':.1,'warmup_seconds':.01},'allocations':None,'measurement_mode':'timing','timing_scope':'warmed_solver_setup_integration_output_certificate'}))\n")
            config={"solvers":{"fake":{"command":[sys.executable,str(script)],"equation_forms":["ODE"],"measurement_modes":["timing"]}}}
            plan=make_plan([CASE],config,source_binding(root))
            ledger=Path(d)/"ledger.json"
            lease=reserve(ledger,"P1",180,platform.node(),persistent=True)
            allowed=tuple(sorted(os.sched_getaffinity(0))[:12])
            resource=hpc.Resources(platform.node(),allowed,allowed[8:],allowed[:8],None,
                32*hpc.GIB,None,None,None,4*hpc.GIB,4)
            with patch.object(hpc,"detect_resources",return_value=resource):
                receipts=run_plan(plan,Path(d)/"out",root,ledger,lease_id=lease,hpc=True)
            self.assertEqual(receipts[0]["status"],"OK")
            attempt=receipts[0]["attempts"][0]
            self.assertGreater(attempt["peak_rss_bytes"],0)
            self.assertEqual(len(attempt["affinity_cpus"]),1)
            self.assertIsNotNone(attempt["gnu_time_report_sha256"])
            self.assertTrue((Path(d)/"out"/"hpc_telemetry.jsonl").is_file())
            plan["kind"]="confirmation"
            with self.assertRaisesRegex(RuntimeError,"HPC_CONFIRMATION_HOLD"):
                run_plan(plan,Path(d)/"other",root,ledger,lease_id=lease,hpc=True)
            p2_ledger=Path(d)/"p2-ledger.json"
            p2_lease=reserve(p2_ledger,"P2",180,platform.node(),persistent=True)
            p2_plan=make_plan([CASE],config,source_binding(root),phase="P2",kind="screening")
            with patch.object(hpc,"detect_resources",return_value=resource):
                p2_receipts=run_plan(p2_plan,Path(d)/"p2-out",root,p2_ledger,
                                     lease_id=p2_lease,hpc=True)
            self.assertEqual(p2_receipts[0]["status"],"OK")

    def test_probe_hpc_removes_eight_worker_cap_without_running_adapter(self):
        import json,time
        from component_benchmark import probe_campaign
        from test_component_benchmark_protocol import CASE
        with tempfile.TemporaryDirectory() as d:
            config=Path(d)/"config.json"
            config.write_text(json.dumps({"cases":[CASE],"solvers":{"fake":{"command":[sys.executable],"equation_forms":["ODE"]}}}))
            allowed=tuple(sorted(os.sched_getaffinity(0))[:20])
            resource=hpc.Resources("fixture",allowed,allowed[8:],allowed[:8],None,
                32*hpc.GIB,None,None,None,8*hpc.GIB,12)
            def fake_one(job,output,hard_stop,oracle_stop,allocation=None,tracker=None):
                return {"job":job,"adapter_status":"OK","parity":{"status":"HOLD","reason":"FAKE_TEST_ONLY"}}
            with patch.object(probe_campaign,"_one",side_effect=fake_one), \
                 patch.object(probe_campaign,"_lease_deadline",return_value=time.monotonic()+2000), \
                 patch.object(probe_campaign,"_bound_files",return_value={"status":"BOUND","files":{}}), \
                 patch.object(probe_campaign,"source_binding",return_value={"fixture":"hash"}), \
                 patch.object(probe_campaign.hpc_resources,"detect_resources",return_value=resource):
                mapping=probe_campaign.run(config,Path(d)/"out",Path(d)/"unused-ledger","fixture-lease",
                                           adapters=["fake"],concurrency=12,hpc=True)
            self.assertEqual(len(mapping),1)
            self.assertTrue((Path(d)/"out"/"hpc_telemetry.jsonl").exists())

    def test_probe_hpc_parses_adapter_and_oracle_bytes(self):
        import time
        from component_benchmark import probe_campaign
        from test_component_benchmark_protocol import CASE
        with tempfile.TemporaryDirectory() as d:
            output=Path(d)
            job={"job_id":"fixture-job", "request":{"case":CASE,"solver_id":"fake","probes":[]},
                 "solver_spec":{"command":[sys.executable],"env":{}},
                 "adapter_binding":{"status":"BOUND","files":{}},
                 "source_binding_sha256":"fixture-source", "case_sha256":"fixture-case"}
            allocation=hpc.Allocation(tuple(sorted(os.sched_getaffinity(0))[:1]),hpc.GIB//2,"native")
            def fake_process(command, **kwargs):
                payload=(b'{"status":"OK","probes":[]}' if "--probe" in command else
                         b'{"status":"HOLD","reason":"FAKE_ORACLE_ONLY"}')
                return {"stdout":payload,"stderr":b"","exit_code":0,"timed_out":False,
                        "command":command,"affinity_cpus":list(allocation.cpus),
                        "peak_rss_bytes":1024,"rss_source":"fixture","rss_samples":1,
                        "affinity_escape":False,"affinity_verified":True,
                        "cgroup_oom_events_delta":0,"assigned_cpu_steal_fraction":0.0,
                        "assigned_cpu_busy_fraction":0.0}
            with patch.object(probe_campaign.hpc_resources,"run_process",side_effect=fake_process) as run:
                receipt=probe_campaign._one(job,output,time.monotonic()+100,time.monotonic()+100,
                                            allocation,hpc.ProcessTracker())
            self.assertEqual(run.call_count,2)
            self.assertEqual(receipt["adapter_status"],"OK")
            self.assertEqual(receipt["parity"],{"status":"HOLD","reason":"FAKE_ORACLE_ONLY"})
            self.assertEqual((output/"jobs"/"fixture-job"/"oracle.stdout").read_bytes(),
                             b'{"status":"HOLD","reason":"FAKE_ORACLE_ONLY"}')


if __name__=="__main__":unittest.main()
