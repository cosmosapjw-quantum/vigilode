"""Independent adversarial checks. Synthetic receipts only; no solver launch."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE=Path(__file__).resolve().parents[1]/'tools/export_local_measurements.py'
spec=importlib.util.spec_from_file_location('independent_exporter',SOURCE)
exporter=importlib.util.module_from_spec(spec)
spec.loader.exec_module(exporter)

class AdversarialExportTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base=Path(self.temp.name)
        self.root=self.base/'run'
        self.folder=self.root/'runs'/'task-a'/'attempt_001'
        self.folder.mkdir(parents=True)
        self.result={'kind':'result','case':10,'method':'dopri5','parallel_layer':'serial',
                     'threads':1,'instrumented':False,'solver_success':True,'scientific_status':'PASS_SCOPED',
                     'actual_error':1e-7,'reference_absolute_bound':1e-12,'error_limit':1e-4,
                     'elapsed_seconds':.2,'final_time':4.0,'final_state':[1.,2.]}
        self.raw={'execution_status':'COMPLETED','exit_code':0,'finished_utc':'2026-10-25T00:00:00Z',
                  'result':self.result,'task':{'id':'task-a','case':10,'method':'dopri5','parallel_layer':'serial',
                  'threads':1,'instrumented':False},'source':{'commit':'a'*40,'clean':True}}
        self.save()
    def save(self,stdout=None):
        data=stdout if stdout is not None else (json.dumps(self.result)+'\n').encode()
        (self.folder/'stdout.jsonl').write_bytes(data)
        (self.folder/'stderr.log').write_bytes(b'')
        self.raw['stdout_sha256']=hashlib.sha256(data).hexdigest()
        self.raw['stderr_sha256']=hashlib.sha256(b'').hexdigest()
        (self.folder/'RECEIPT.json').write_text(json.dumps(self.raw))
    def row(self):return exporter.export_receipt(self.folder/'RECEIPT.json',self.root)

    def test_reread_disappearance_never_leaves_timing_candidate(self):
        real=exporter.bounded_read
        calls=0
        def read(path):
            nonlocal calls
            if path.name=='RECEIPT.json':
                calls+=1
                if calls==2:raise FileNotFoundError('concurrent receipt replacement')
            return real(path)
        with patch.object(exporter,'bounded_read',side_effect=read):r=self.row()
        self.assertFalse(r['timing_reanalysis_candidate'])
        self.assertNotEqual(r['evidence_status'],'RAW_VERIFIED')

    def test_task_index_hash_cannot_bind_another_task(self):
        p=self.base/'index.json'
        p.write_text(json.dumps({'rows':[{'task_id':'task-b','receipt_sha256':hashlib.sha256((self.folder/'RECEIPT.json').read_bytes()).hexdigest()}]}))
        try:summary=exporter.export(self.root,self.base/'export',p)
        except ValueError:return
        self.assertNotEqual(summary['task_index']['rows'][0]['status'],'EXACT_RECEIPT_BYTES_FOUND')

    def test_null_index_identity_cannot_match_missing_identity(self):
        (self.folder/'RECEIPT.json').write_text('{}')
        p=self.base/'index.json';p.write_text(json.dumps({'rows':[{'task_id':None,'receipt_sha256':None}]}))
        try:summary=exporter.export(self.root,self.base/'export',p)
        except (ValueError,TypeError):return
        self.assertNotIn(summary['task_index']['rows'][0]['status'],('EXACT_RECEIPT_BYTES_FOUND','TASK_FOUND_DIFFERENT_RECEIPT_BYTES'))

    def test_extreme_json_integer_remains_record_level_failure(self):
        self.result['elapsed_seconds']=10**400;self.save()
        r=self.row()
        self.assertFalse(r['timing_reanalysis_candidate'])
        self.assertNotEqual(r['evidence_status'],'RAW_VERIFIED')

    def test_partial_terminal_write_is_preserved_as_issue(self):
        (self.folder/'RECEIPT.json').write_bytes(b'{"execution_status":"COMPLETED",')
        r=self.row();self.assertFalse(r['timing_reanalysis_candidate']);self.assertNotEqual(r['evidence_status'],'RAW_VERIFIED')

    def test_multiple_results_are_not_an_extra_sample(self):
        data=((json.dumps(self.result)+'\n')*2).encode();self.save(data)
        r=self.row();self.assertFalse(r['timing_reanalysis_candidate']);self.assertIn('MULTIPLE_RESULTS',r['issues'])

    def test_modified_stream_is_not_accepted(self):
        (self.folder/'stdout.jsonl').write_bytes(b'{}\n')
        r=self.row();self.assertFalse(r['timing_reanalysis_candidate']);self.assertIn('HASH_MISMATCH_STDOUT',r['issues'])

    def test_external_receipt_symlink_not_read(self):
        external=self.base/'outside.json';external.write_text(json.dumps(self.raw))
        (self.folder/'RECEIPT.json').unlink();(self.folder/'RECEIPT.json').symlink_to(external)
        r=self.row();self.assertFalse(r['timing_reanalysis_candidate']);self.assertNotEqual(r['evidence_status'],'RAW_VERIFIED')

    def test_duplicate_wrapper_keeps_same_evidence_identity(self):
        (self.folder/'COMPARISON_RECEIPT.json').write_text(json.dumps({'raw_receipt':self.raw,'case':10,'method':'dopri5','warmup':True}))
        summary=exporter.export(self.root,self.base/'export')
        rows=[json.loads(x) for x in (self.base/'export/MEASUREMENTS.jsonl').read_text().splitlines()]
        self.assertEqual(len(rows),2);self.assertEqual(summary['raw_evidence_keys'],1)
        self.assertFalse(next(r for r in rows if r['receipt_name']=='COMPARISON_RECEIPT.json')['timing_reanalysis_candidate'])

if __name__=='__main__':unittest.main()
