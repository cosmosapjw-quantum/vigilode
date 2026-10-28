"""Fake libtest processes only; no compiled scientific binary is run here."""
import hashlib
import json
import os
import platform
import stat
import subprocess
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

from component_benchmark import component_contracts as cc
from component_benchmark.budget import boot_id

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "tools/component_benchmark/component_contracts.py"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def fake_script(path, mode, marker):
    path.write_text(f'''#!/usr/bin/env python3
import pathlib,sys,time
mode={mode!r}; marker=pathlib.Path({str(marker)!r})
if '--list' in sys.argv:
    print(('protected_holdout_case' if mode=='protected' else 'fake_contract')+': test')
else:
    marker.write_text(marker.read_text()+'x' if marker.exists() else 'x')
    if mode=='timeout': time.sleep(2)
    elif mode=='failure':
        print('test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s')
        sys.exit(101)
    else: print('test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s')
''')
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


class ComponentContractUtilityTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.work = Path(self.tmp.name)
        self.ledger = self.work / "ledger.json"
        self.output = self.work / "output"
        self.manifest = self.work / "manifest.json"
        self.lease = "fixture-p1-persistent"
        self.marker = self.work / "marker"
        now = time.time()
        self.ledger.write_text(json.dumps({"schema":"host-lease-v1","leases":[{
            "id":self.lease,"host":platform.node(),"phase":"P1","start":now,
            "start_monotonic":time.monotonic(),"boot_id":boot_id(),"pid":os.getpid(),
            "deadline":now+500,"reserved_seconds":500,"persistent":True
        }]}))
        self.head = subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip()

    def entry(self, name, mode="good", missing=False):
        exe = self.work / name
        fake_script(exe, mode, self.marker)
        digest = sha(exe)
        if missing: exe.unlink()
        return {"id":name,"executable":str(exe),"sha256":digest,
                "source_paths":[SOURCE],"source_hashes":{SOURCE:sha(ROOT/SOURCE)},
                "argv":["--test-threads=1"]}

    def write_manifest(self, entries):
        self.manifest.write_text(json.dumps({"schema":cc.SCHEMA,"exec_sha":self.head,
            "source_root":str(ROOT),"entries":entries}))

    def run_utility(self, concurrency=4):
        return cc.run_manifest(self.manifest,self.output,self.ledger,self.lease,concurrency)

    def test_success_resumes_without_duplicate_and_hashes_raw_receipts(self):
        self.write_manifest([self.entry("good")])
        first=self.run_utility()
        self.assertEqual(first[0]["status"],"OK")
        self.assertEqual(first[0]["test_counts"]["passed"],1)
        self.assertEqual(self.marker.read_text(),"x")
        self.assertEqual(self.run_utility()[0]["status"],"OK")
        self.assertEqual(self.marker.read_text(),"x")
        raw=self.output/"jobs"/"good"/"run.stdout"
        raw.write_text("tampered")
        with self.assertRaisesRegex(RuntimeError,"RECEIPT_IDENTITY_MISMATCH"):
            self.run_utility()

    def test_failure_protected_and_missing_are_independent(self):
        self.write_manifest([self.entry("failure","failure"),self.entry("protected","protected"),self.entry("missing",missing=True)])
        rows={x["id"]:x for x in self.run_utility()}
        self.assertEqual(rows["failure"]["status"],"TEST_FAILURE")
        self.assertEqual(rows["protected"]["status"],"PROTECTED_TEST_REFUSED")
        self.assertEqual(rows["missing"]["status"],"ENVIRONMENT_FAILURE")
        self.assertFalse((self.output/"jobs"/"protected"/"run.stdout").exists())
        self.assertEqual(rows["missing"]["reason"],"MISSING_EXECUTABLE")

    def test_duplicate_and_source_identity_refused(self):
        entry=self.entry("good")
        self.write_manifest([entry,dict(entry,id="same_binary")])
        with self.assertRaisesRegex(ValueError,"duplicate component executable"):
            self.run_utility()
        entry["source_hashes"][SOURCE]="0"*64
        self.write_manifest([entry])
        with self.assertRaisesRegex(RuntimeError,"SOURCE_BINDING_MISMATCH"):
            self.run_utility()

    def test_lease_identity_and_worst_case_deadline_before_launch(self):
        self.write_manifest([self.entry("good")])
        with patch.object(cc.platform,"node",return_value="wrong-host"):
            with self.assertRaisesRegex(RuntimeError,"LEASE_IDENTITY_MISMATCH"):
                self.run_utility()
        ledger=json.loads(self.ledger.read_text())
        ledger["leases"][0]["deadline"]=time.time()+35
        self.ledger.write_text(json.dumps(ledger))
        with self.assertRaisesRegex(RuntimeError,"LEASE_REMAINING_INSUFFICIENT"):
            self.run_utility()
        self.assertFalse((self.output/"jobs"/"good"/"run.stdout").exists())

    def test_run_timeout_is_distinct(self):
        self.write_manifest([self.entry("timeout","timeout")])
        with patch.object(cc,"TIMEOUT_SECONDS",0.3):
            receipt=self.run_utility()[0]
        self.assertEqual(receipt["status"],"TEST_TIMEOUT")
        self.assertEqual(receipt["reason"],"RUN_TIMEOUT")
        self.assertTrue((self.output/"jobs"/"timeout"/"run.stdout").exists())

    def test_hpc_affinity_receipt_fake_binary(self):
        from component_benchmark import hpc
        self.write_manifest([self.entry("good")])
        allowed=tuple(sorted(os.sched_getaffinity(0))[:12])
        resource=hpc.Resources(platform.node(),allowed,allowed[8:],allowed[:8],None,
            32*hpc.GIB,None,None,None,4*hpc.GIB,4)
        with patch.object(hpc,"detect_resources",return_value=resource):
            receipt=cc.run_manifest(self.manifest,self.output,self.ledger,self.lease,4,hpc=True)[0]
        self.assertEqual(receipt["status"],"OK")
        self.assertEqual(len(receipt["phases"]),2)
        self.assertEqual(len(receipt["phases"][1]["affinity_cpus"]),1)
        phase=receipt["phases"][1]
        if phase["peak_rss_bytes"] is None:
            self.assertEqual(phase["rss_status"],"UNAVAILABLE")
            self.assertIn("unavailable",phase["rss_source"])
        else:
            self.assertGreater(phase["peak_rss_bytes"],0)
            self.assertEqual(phase["rss_status"],"OBSERVED")
        self.assertTrue((self.output/"hpc_telemetry.jsonl").is_file())


if __name__ == "__main__":
    unittest.main()
