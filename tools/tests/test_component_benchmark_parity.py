import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from component_benchmark.cases import build_cases
from component_benchmark.parity import frozen_points,admit_probe,check_probe
from component_benchmark.protocol import digest
from component_benchmark.scipy_adapter import probe


class ParityContracts(unittest.TestCase):
    def setUp(self):
        self.case=build_cases()[0]
        self.request={'case':self.case,'solver_id':'scipy-radau','probes':frozen_points(self.case)}
        self.result=probe(self.request)
    def admit(self,request=None,result=None,source='source'):
        return admit_probe(digest(self.case),'source',source,request or self.request,result or self.result)
    def test_analytic_scalar(self):
        self.assertEqual(self.admit()['status'],'PASS_DIAGNOSTIC')
    def test_changed_equation_rejected(self):
        request=copy.deepcopy(self.request);request['case']['parameters']['rate']=-2.
        self.assertEqual(self.admit(request=request)['status'],'HOLD')
    def test_changed_grid_rejected(self):
        request=copy.deepcopy(self.request);request['case']['output_times'][1]+=.001
        self.assertEqual(self.admit(request=request)['status'],'HOLD')
    def test_changed_source_rejected(self):
        self.assertEqual(self.admit(source='changed')['status'],'HOLD')
    def test_false_numerical_output_rejected(self):
        result=copy.deepcopy(self.result);result['probes'][0]['rhs'][0]+=.01
        self.assertEqual(self.admit(result=result)['status'],'HOLD')
    def test_cancellation_and_independent_equations(self):
        for case in build_cases():
            if len(case['initial_state'])>512:continue
            request={'case':case,'solver_id':'scipy-radau','probes':frozen_points(case)}
            with self.subTest(case=case['case_id']):
                self.assertEqual(check_probe(case,request['probes'],probe(request))['status'],'PASS_DIAGNOSTIC')


if __name__=='__main__':unittest.main()
