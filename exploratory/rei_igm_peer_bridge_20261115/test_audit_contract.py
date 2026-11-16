#!/usr/bin/env python3
"""Source-bound regression checks for actual saved Peer experiment receipts.
Run with PEER_EVIDENCE_ROOT set to the preserved native run directory.
No mocked solver and no changes to original source data.
"""
import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from audit_full_ivp import audit

ROOT = Path(os.environ['PEER_EVIDENCE_ROOT'])

class AuditContract(unittest.TestCase):
    def mutated_runs(self, change):
        directory = tempfile.TemporaryDirectory(prefix='peer_audit_contract_')
        self.addCleanup(directory.cleanup)
        target = Path(directory.name)
        for path in (ROOT/'run2_centered').glob('*.raw'):
            (target/path.name).symlink_to(path)
        result = json.loads((ROOT/'run2_centered/RESULT.json').read_text())
        change(result)
        (target/'RESULT.json').write_text(json.dumps(result))
        return target

    def check_rejected(self, change, expected):
        runs = self.mutated_runs(change)
        with contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(ValueError, expected):
                audit(ROOT/'source', runs, ROOT/'independent', runs/'audit.json')

    def test_missing_planned_case_is_rejected(self):
        self.check_rejected(lambda r:r['runs'].pop(), 'CASE_CENSUS_MISMATCH')

    def test_nonfinite_reported_W_residual_is_rejected(self):
        self.check_rejected(lambda r:r['runs'][0].update(max_original_W_relative_residual=float('nan')), 'W_RESIDUAL_FAILED')

    def test_forged_zero_executed_solve_count_is_rejected(self):
        self.check_rejected(lambda r:r['runs'][0].update(W_solves=0), 'EXECUTED_WORK_COUNT_MISMATCH')

    def test_forged_official_acceptance_is_rejected(self):
        self.check_rejected(lambda r:r.update(official_tick161='ACCEPTED'), 'FORGED_OFFICIAL_ACCEPTANCE')

if __name__ == '__main__':
    unittest.main()
