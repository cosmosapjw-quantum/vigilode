import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from component_benchmark.runner import _run_one, make_plan, run_plan, source_binding
from test_component_benchmark_protocol import CASE

ROOT=Path(__file__).resolve().parents[2]


def frozen_root(parent):
    root=Path(parent)/"source"
    (root/"tools").mkdir(parents=True)
    (root/"crates").mkdir()
    (root/"Cargo.toml").write_text("[workspace]\n")
    (root/"Cargo.lock").write_text("version = 3\n")
    (root/"tools"/"component_benchmark.py").write_text("# frozen fixture\n")
    return root

class RunnerTest(unittest.TestCase):
    def test_isolated_receipt_and_resume(self):
        with tempfile.TemporaryDirectory() as d:
            root=frozen_root(d)
            script=Path(d)/"adapter.py"
            script.write_text("import json,sys\nr=json.load(open(sys.argv[-1]))\nc=r['case']\nprint(json.dumps({'status':'OK','times':c['output_times'],'states':[[1.0],[2.0]],'counters':None,'timing':{'timed_seconds':.1,'warmup_seconds':.01},'allocations':None,'measurement_mode':'timing','timing_scope':'warmed_solver_setup_integration_output_certificate'}))\n")
            config={"solvers":{"fake":{"command":[sys.executable,str(script)],"equation_forms":["ODE"]}}}
            plan=make_plan([CASE],config,source_binding(root))
            results=run_plan(plan,Path(d)/"out",root,Path(d)/"shared_ledger.json")
            self.assertEqual(results[0]["status"],"OK")
            job=plan["jobs"][0]["job_id"]
            self.assertTrue((Path(d)/"out"/"jobs"/job/"receipt.jsonl").exists())
            self.assertIn("tools/component_benchmark.py",plan["source_binding"])
            self.assertIn("fixtures/rodas5p_coefficients_snapshot.json",source_binding(ROOT))
            self.assertEqual(run_plan(plan,Path(d)/"out",root,Path(d)/"shared_ledger.json",resume=True),[])
            script.write_text("print('changed executable')\n")
            with self.assertRaisesRegex(RuntimeError,"EXECUTABLE_BINDING_MISMATCH"):
                run_plan(plan,Path(d)/"out",root,Path(d)/"shared_ledger.json",resume=True)

    def test_tampered_raw_log_blocks_resume(self):
        with tempfile.TemporaryDirectory() as d:
            root=frozen_root(d)
            script=Path(d)/"adapter.py"
            script.write_text("import json,sys\nc=json.load(open(sys.argv[-1]))['case']\nprint(json.dumps({'status':'OK','times':c['output_times'],'states':[[1],[2]],'counters':None,'timing':{'timed_seconds':.1,'warmup_seconds':.01},'allocations':None,'measurement_mode':'timing','timing_scope':'warmed_solver_setup_integration_output_certificate'}))\n")
            config={"solvers":{"fake":{"command":[sys.executable,str(script)],"equation_forms":["ODE"]}}}
            plan=make_plan([CASE],config,source_binding(root))
            out=Path(d)/"out"; ledger=Path(d)/"ledger.json"
            run_plan(plan,out,root,ledger)
            folder=out/"jobs"/plan["jobs"][0]["job_id"]
            (folder/"attempt-0.stdout").write_text("altered")
            with self.assertRaisesRegex(RuntimeError,"RECEIPT_IDENTITY_MISMATCH"):
                run_plan(plan,out,root,ledger,resume=True)

    def test_capability_is_explicit(self):
        config={"solvers":{"fake":{"command":["false"],"equation_forms":["DAE"]}}}
        plan=make_plan([CASE],config,{})
        self.assertEqual(plan["jobs"][0]["capability"],"UNSUPPORTED_CAPABILITY")

    def test_held_prerequisite_retains_arm_without_spawning(self):
        with tempfile.TemporaryDirectory() as d:
            root=frozen_root(d)
            marker=Path(d)/"launched"
            script=Path(d)/"adapter.py"
            script.write_text("from pathlib import Path\nPath("+repr(str(marker))+").write_text('launched')\n")
            solver={"command":[sys.executable,str(script)],"equation_forms":["ODE"]}
            gate={"status":"HOLD","reason":"SOURCE_PARITY_HOLD","evidence_sha256":"a"*64}
            config={"solvers":{"fake":solver},"case_gates":{CASE["case_id"]:{"fake":gate}}}
            source=source_binding(root)
            plan=make_plan([CASE],config,source)
            old_plan=make_plan([CASE],{"solvers":{"fake":solver}},source)
            self.assertNotEqual(plan["jobs"][0]["job_id"],old_plan["jobs"][0]["job_id"])
            self.assertEqual(plan["jobs"][0]["capability"],"HOLD_PREREQUISITE:SOURCE_PARITY_HOLD")
            out=Path(d)/"out"; ledger=Path(d)/"ledger.json"
            with patch("component_benchmark.runner.subprocess.run",side_effect=AssertionError("spawned")):
                receipts=run_plan(plan,out,root,ledger)
            self.assertEqual(receipts[0]["status"],"HOLD_PREREQUISITE")
            self.assertEqual(receipts[0]["prerequisite"],gate)
            self.assertEqual(receipts[0]["attempts"],[])
            self.assertFalse(marker.exists())
            self.assertEqual(run_plan(plan,out,root,ledger,resume=True),[])
            changed=dict(config,case_gates={CASE["case_id"]:{"fake":dict(gate,status="PASS")}})
            self.assertNotEqual(make_plan([CASE],changed,source)["jobs"][0]["job_id"],plan["jobs"][0]["job_id"])

    def test_equation_text_maps_to_ode_and_retry_requires_diagnosis(self):
        config={"solvers":{"fake":{"command":["false"],"equation_forms":["ODE"]}}}
        case={**CASE,"equation_form":"y′=-y"}
        plan=make_plan([case],config,{})
        self.assertEqual(plan["jobs"][0]["capability"],"SUPPORTED")
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaisesRegex(ValueError,"explicit infrastructure diagnosis"):
                _run_one(plan["jobs"][0],config["solvers"]["fake"],d,1,"hash",float("inf"),retry=1)

    def test_confirmation_serial_and_contrast_limit(self):
        config={"solvers":{"fake":{"command":["false"],"equation_forms":["ODE"]}},"contrasts":[{}]*17}
        with self.assertRaisesRegex(ValueError,"sixteen"):
            make_plan([CASE],config,{})

    def test_fixed_ladder_unique_and_applicability_explicit(self):
        config={"solvers":{"native-bdf1":{"command":["false"],"equation_forms":["ODE"],"models":["other"]}}}
        plan=make_plan([CASE],config,{})
        self.assertEqual(len(plan["jobs"]),8)
        timing=[j for j in plan["jobs"] if j["measurement_mode"]=="timing"]
        self.assertEqual({j["request"]["solver_config"]["h_divisor"] for j in timing},{1000,2000,4000,8000})
        self.assertEqual(len({j["job_id"] for j in plan["jobs"]}),8)
        self.assertTrue(all(j["capability"]=="NOT_APPLICABLE:models" for j in plan["jobs"]))

    def test_confirmation_only_selected_pair_and_global_multiplicity(self):
        solvers={sid:{"command":["false"],"equation_forms":["ODE"],"measurement_modes":["timing"]} for sid in ("base","candidate")}
        local={"case_id":CASE["case_id"],"baseline":"base","candidate":"candidate","critical":True}
        other={"case_id":"other","baseline":"base","candidate":"candidate","critical":True}
        config={"solvers":solvers,"contrasts":[local],"global_contrasts":[local,other]}
        plan=make_plan([CASE],config,{},kind="confirmation")
        self.assertEqual(len(plan["jobs"]),34)
        self.assertEqual(plan["global_multiplicity"],2)
        with self.assertRaisesRegex(ValueError,"one case/two solvers"):
            make_plan([CASE,dict(CASE,case_id="other")],config,{},kind="confirmation")

    def test_diagnosed_retry_retains_first_failure(self):
        with tempfile.TemporaryDirectory() as d:
            root=frozen_root(d)
            marker=Path(d)/"ready"
            script=Path(d)/"adapter.py"
            script.write_text("import json,sys,pathlib\nc=json.load(open(sys.argv[-1]))['case']\nif not pathlib.Path(" + repr(str(marker)) + ").exists():\n print('first infrastructure failure',file=sys.stderr);sys.exit(3)\nprint(json.dumps({'status':'OK','times':c['output_times'],'states':[[1.0],[2.0]],'counters':None,'timing':{'timed_seconds':.1,'warmup_seconds':.01},'allocations':None,'measurement_mode':'timing','timing_scope':'warmed_solver_setup_integration_output_certificate'}))\n")
            config={"solvers":{"fake":{"command":[sys.executable,str(script)],"equation_forms":["ODE"]}}}
            plan=make_plan([CASE],config,source_binding(root))
            output=Path(d)/"out"; ledger=Path(d)/"shared.json"
            first=run_plan(plan,output,root,ledger)
            self.assertEqual(first[0]["status"],"ENVIRONMENT_FAILURE")
            marker.write_text("ready")
            second=run_plan(plan,output,root,ledger,resume=True,retry_diagnosis="marker was absent during environment setup")
            self.assertEqual(second[0]["status"],"OK")
            folder=output/"jobs"/plan["jobs"][0]["job_id"]
            self.assertIn("first infrastructure failure",(folder/"attempt-0.stderr").read_text())
            self.assertEqual(len(json.loads((folder/"receipt.json").read_text())["attempts"]),2)
