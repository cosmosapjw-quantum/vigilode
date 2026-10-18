"""Probe campaign orchestration tests using a local fake adapter only."""
from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import platform
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from tools.component_benchmark.budget import close, reserve
from tools.component_benchmark.cases import build_cases
from tools.component_benchmark.probe_campaign import run, unique_cases


class ProbeCampaignTests(unittest.TestCase):
    def test_physical_dedup_ignores_tolerance_variant(self):
        first = next(c for c in build_cases() if c['case_id'] == 'ode-scalar-decay')
        second = copy.deepcopy(first)
        second['case_id'] += '@tighter'
        second['rtol'] *= 0.1
        second['atol'] = [a * 0.1 for a in second['atol']]
        self.assertEqual(len(unique_cases({'cases':[first,second]})), 1)

    def test_resume_preserves_raw_receipt_and_rejects_tampering(self):
        first = next(c for c in build_cases() if c['case_id'] == 'ode-scalar-decay')
        second = copy.deepcopy(first)
        second['case_id'] += '@tighter'
        second['rtol'] *= 0.1
        second['atol'] = [a * 0.1 for a in second['atol']]
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            adapter = base/'fake_adapter.py'
            invoked = base/'invoked.txt'
            adapter.write_text('import json,os\nfrom pathlib import Path\n'
                               'Path(os.environ["FAKE_INVOKED"]).open("a").write("run\\n")\n'
                               'print(json.dumps({"status":"UNSUPPORTED_CAPABILITY",'
                               '"case_id":"ode-scalar-decay","probes":[]}))\n')
            spec = {'command':[sys.executable,str(adapter)],
                    'cwd':str(ROOT),'env':{'FAKE_INVOKED':str(invoked)},
                    'equation_forms':['ODE']}
            config = base/'config.json'
            config.write_text(json.dumps({'cases':[first,second],
                                          'solvers':{'fake-solver':spec}}))
            ledger = base/'ledger.json'
            lease = reserve(ledger,'P1',120,platform.node())
            try:
                with patch('tools.component_benchmark.probe_campaign.source_binding',
                           return_value={'frozen':'source'}):
                    mapping = run(config,base/'output',ledger,lease,
                                  adapters=['fake-solver'],concurrency=1)
                    self.assertEqual(len(mapping),1)
                    self.assertEqual(invoked.read_text().count('run'),1)
                    again = run(config,base/'output',ledger,lease,
                                adapters=['fake-solver'],concurrency=1)
                    self.assertEqual(mapping,again)
                    self.assertEqual(invoked.read_text().count('run'),1)
                    raw = next((base/'output'/'jobs').glob('*/adapter.stdout'))
                    raw.write_text('tampered')
                    with self.assertRaisesRegex(RuntimeError,'RECEIPT_IDENTITY_MISMATCH'):
                        run(config,base/'output',ledger,lease,
                            adapters=['fake-solver'],concurrency=1)
            finally:
                close(ledger,lease)

    def test_missing_lease_is_rejected(self):
        with self.assertRaisesRegex(ValueError,'ledger and active lease'):
            run('missing','output',None,None)


if __name__ == '__main__':
    unittest.main()
