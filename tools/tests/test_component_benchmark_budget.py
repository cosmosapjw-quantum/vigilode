import tempfile
import unittest
from pathlib import Path
from component_benchmark.budget import close, locked_ledger, reserve, usage

class BudgetTest(unittest.TestCase):
    def test_persistent_single_host_lease_and_cap(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"lease.json"
            ident=reserve(p,"P0",10,"host",1)
            with self.assertRaisesRegex(RuntimeError,"HOST_LEASE_ALREADY_ACTIVE"):
                reserve(p,"P0",10,"host",1)
            with locked_ledger(p) as data:
                self.assertEqual(len(data["leases"]),1)
                self.assertGreaterEqual(usage(data)["P0"],0)
            close(p,ident)
            with self.assertRaises(RuntimeError): reserve(p,"P0",7*3600,"host")

    def test_persistent_lease_spans_cli_processes(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"lease.json"
            ident=reserve(p,"P0",10,"host",persistent=True)
            with locked_ledger(p) as data:
                data["leases"][0]["pid"]=-999999
            with self.assertRaisesRegex(RuntimeError,"HOST_LEASE_ALREADY_ACTIVE"):
                reserve(p,"P0",10,"host")
            close(p,ident)
