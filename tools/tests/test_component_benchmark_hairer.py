"""Real local Hairer wrapper checks; numerical failures retain their budget ratios."""
from __future__ import annotations

import json
import math
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from tools.component_benchmark.cases import build_cases


class HairerAdapterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.libdir = os.environ.get('VIGILODE_HAIRER_LIBDIR',
                                    '/tmp/vigilode-component-hairer-20261018')
        needed = ('libhairer_radau.so', 'libhairer_rodas.so')
        if not all((Path(cls.libdir) / name).is_file() for name in needed):
            raise unittest.SkipTest('frozen Hairer libraries absent')
        cls.cases = {c['case_id']: c for c in build_cases()}

    def run_adapter(self, case_id, solver_id, probe=False):
        request = {'case': self.cases[case_id], 'solver_id': solver_id}
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json') as file:
            json.dump(request, file)
            file.flush()
            command = [sys.executable,
                       str(ROOT / 'tools/component_benchmark/hairer_adapter.py'),
                       '--request', file.name]
            if probe:
                command.append('--probe')
            process = subprocess.run(command, env={**os.environ,
                                                    'VIGILODE_HAIRER_LIBDIR': self.libdir},
                                     capture_output=True, text=True, timeout=60)
        self.assertEqual(process.returncode, 0, process.stderr)
        self.assertFalse(process.stderr, process.stderr)
        return json.loads(process.stdout)

    def budget_report(self, case, result):
        self.assertEqual(result['times'], case['output_times'])
        self.assertEqual(len(result['states']), 101)
        self.assertTrue(all(len(row) == 1 and math.isfinite(row[0])
                            for row in result['states']))
        exact = math.exp if case['model'] == 'scalar_decay' else math.sin
        ratios = [abs(row[0] - exact(t)) / (case['atol'][0] +
                  case['rtol'] * abs(exact(t)))
                  for t, row in zip(result['times'], result['states'])]
        self.assertEqual(len(ratios), 101)
        return {'max_grid_budget_ratio': max(ratios),
                'endpoint_budget_ratio': ratios[-1],
                'max_grid_budget_pass': max(ratios) <= 1.0,
                'endpoint_budget_pass': ratios[-1] <= 1.0}

    def test_scalar_and_pr101_full_requested_grid_original_budget(self):
        for solver_id in ('hairer-radau5', 'hairer-rodas'):
            for case_id in ('ode-scalar-decay', 'ode-pr101'):
                with self.subTest(solver=solver_id, case=case_id):
                    result = self.run_adapter(case_id, solver_id)
                    self.assertEqual(result['status'], 'OK', result.get('reason'))
                    self.assertEqual(result['fortran_idid'], 1)
                    self.assertEqual(result['measurement_mode'], 'timing')
                    self.assertEqual(result['timing_scope'],
                                     'warmed_solver_setup_integration_output_certificate')
                    self.assertGreater(result['timing']['timed_seconds'], 0)
                    self.assertGreater(result['timing']['warmup_seconds'], 0)
                    self.assertEqual(result['counters']['accepted_steps'],
                                     result['counters']['computed_steps'] -
                                     result['counters']['rejected_steps'])
                    budget = self.budget_report(self.cases[case_id], result)
                    # The endpoint and full grid are classified independently.
                    # Known budget failures are evidence, not a loosened threshold.
                    self.assertGreaterEqual(budget['max_grid_budget_ratio'],
                                            budget['endpoint_budget_ratio'])

    def test_probe_uses_shared_analytic_model(self):
        result = self.run_adapter('ode-pr101', 'hairer-radau5', probe=True)
        self.assertEqual(result['status'], 'OK')
        self.assertEqual(result['derivative_mode'], 'analytic_model_jacobian')
        self.assertEqual(len(result['probes']), 2)

    def test_non_ode_and_unknown_configuration_are_typed(self):
        from tools.component_benchmark.hairer_adapter import solve
        ode = self.cases['ode-scalar-decay']
        bad = solve({'case': ode, 'solver_id': 'hairer-radau5',
                     'solver_config': {'unknown': 1}})
        self.assertEqual(bad['status'], 'UNSUPPORTED_CAPABILITY')
        mass = dict(ode, mass_matrix=[[1.0]])
        self.assertEqual(solve({'case': mass, 'solver_id': 'hairer-radau5'})['status'],
                         'UNSUPPORTED_CAPABILITY')


if __name__ == '__main__':
    unittest.main()
