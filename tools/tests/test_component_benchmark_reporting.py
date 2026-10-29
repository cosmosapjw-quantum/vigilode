"""Reporting joins only authenticated receipts and explicit independent decisions."""
from __future__ import annotations

import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT))
from tools.component_benchmark.cases import build_cases
from tools.component_benchmark.protocol import digest, file_digest, strict_load
from tools.component_benchmark.registry import inventory
from tools.component_benchmark.reporting import generate
from tools.component_benchmark.runner import atomic_json, save_receipt


class ReportingTests(unittest.TestCase):
    def build_run(self, directory):
        case = next(c for c in build_cases() if c['case_id']=='ode-scalar-decay')
        component = inventory(ROOT)[0]['component_id']
        source = {'fixture':'frozen'}
        jobs=[]
        for name in ('ok','fail','missing'):
            jobs.append({'job_id':'fixture-'+name,'case_id':case['case_id'],
                         'solver_id':'fixture-solver','config_id':'default',
                         'measurement_mode':'timing','round':0,'kind':'screening',
                         'request':{'case':case,'solver_id':'fixture-solver'}})
        plan={'schema':'fixture','kind':'screening','source_binding':source,
              'solvers':{'fixture-solver':{'components':[component]}},'jobs':jobs}
        atomic_json(directory/'plan.json',plan)
        for job in jobs[:2]:
            folder=directory/'jobs'/job['job_id']
            folder.mkdir(parents=True)
            atomic_json(folder/'request.json',job['request'])
            (folder/'attempt-0.stdout').write_text('{"status":"OK"}' if job['job_id'].endswith('ok') else 'failure')
            (folder/'attempt-0.stderr').write_text('' if job['job_id'].endswith('ok') else 'first raw failure')
            result = ({'status':'OK','times':case['output_times'],'states':[[0.0] for _ in case['output_times']],
                       'timing':{'timed_seconds':1.0,'warmup_seconds':.1},
                       'counters':{'rhs_evaluations':10}}
                      if job['job_id'].endswith('ok') else None)
            status='OK' if result else 'ENVIRONMENT_FAILURE'
            attempt={'stdout_path':str(folder/'attempt-0.stdout'),
                     'stderr_path':str(folder/'attempt-0.stderr'),
                     'stdout_sha256':file_digest(folder/'attempt-0.stdout'),
                     'stderr_sha256':file_digest(folder/'attempt-0.stderr')}
            receipt={'schema':'fixture','job':job,'result':result,'attempts':[attempt],
                     'source_binding_sha256':digest(source),'status':status,
                     'request_sha256':file_digest(folder/'request.json'),
                     'result_sha256':digest(result) if result else None}
            save_receipt(folder,receipt)
        atomic_json(directory/'analysis.json',{'source_binding_valid':True,
                    'scientific_gates':{'fixture-ok':{'status':'PASS','max_error_budget_ratio':.4}}})
        return component,digest(source)

    def test_all_components_failures_missing_and_svg(self):
        with tempfile.TemporaryDirectory() as temp:
            directory=Path(temp)/'run';directory.mkdir()
            component,_=self.build_run(directory)
            output=Path(temp)/'report'
            counts=generate(ROOT,[directory],output)
            self.assertEqual(counts['jobs'],3)
            self.assertEqual(counts['missing_jobs'],1)
            data=strict_load((output/'component_evidence.json').read_text())
            row=next(x for x in data['components'] if x['component_id']==component)
            self.assertEqual(row['recommended_action'],'need to optimize')
            self.assertEqual(row['evidence_status'],'HOLD')
            self.assertEqual(row['failed_or_unsupported_jobs'],1)
            self.assertEqual(row['missing_jobs'],1)
            self.assertEqual(row['passing_gates'],1)
            self.assertIn('<svg',(output/'work_precision.svg').read_text())
            self.assertIn('fixture-fail',(output/'work_precision.csv').read_text())
            self.assertIn('HOLD',(output/'all_components_ko.md').read_text())
            raw=directory/'jobs'/'fixture-fail'/'attempt-0.stderr'
            raw.write_text('tampered')
            with self.assertRaisesRegex(RuntimeError,'RECEIPT_IDENTITY_MISMATCH'):
                generate(ROOT,[directory],output)

    def test_only_independent_source_bound_decision_overrides_default(self):
        with tempfile.TemporaryDirectory() as temp:
            directory=Path(temp)/'run';directory.mkdir()
            component,source_hash=self.build_run(directory)
            decisions=Path(temp)/'decisions.json'
            decisions.write_text(json.dumps({'independent_review':True,
                'source_binding_sha256':source_hash,
                'decisions':{component:{'action':'specialize',
                                        'evidence_status':'REVIEWED_DIAGNOSTIC',
                                        'reason':'independent fixture decision'}}}))
            output=Path(temp)/'report'
            counts=generate(ROOT,[directory],output,decisions)
            self.assertEqual(counts['independent_decisions'],1)
            data=strict_load((output/'component_evidence.json').read_text())
            row=next(x for x in data['components'] if x['component_id']==component)
            self.assertEqual(row['recommended_action'],'specialize')
            bad=json.loads(decisions.read_text());bad['independent_review']=False
            decisions.write_text(json.dumps(bad))
            with self.assertRaisesRegex(ValueError,'independent decisions'):
                generate(ROOT,[directory],output,decisions)


if __name__=='__main__':
    unittest.main()
