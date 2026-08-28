"""Git lifecycle tests; synthetic history and dependency validators, never solver evidence.

Only pinned authority IDs are rebound in the fixture copy. Real git fetch,
worktree creation, merge, abort, and repeated invocation are executed.
"""
from __future__ import annotations
import argparse
import base64
import hashlib
import zlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = "docs/exec-plans/k0-stage-telemetry-integration-20260827"
BRANCH = "research/k0-stage-telemetry-integration-20260827"
PBRANCH = "docs/k0-codex-execution-package-20260827"
BMARKS = ["PACKAGE_CONTRACT_PASS"]
SMARKS = ["WU05_SUPPLEMENT_MANIFEST_PASS", "LEGACY_REPAIR_BLOBS_PASS",
          "EXTERNAL_PACKAGE_PIN_PASS", "WU05_SUPPLEMENT_AUTHORITY_PASS", "HOSTILE_FIXTURES_PASS"]
HERE = Path(__file__).resolve()
DEFAULT_SOURCE = HERE.parent
RUNNER = Path(os.environ.get("K0_TEST_RUNNER", str(DEFAULT_SOURCE / "k0-wu05-bootstrap-v2.sh")))
VALIDATOR = Path(os.environ.get("K0_TEST_VALIDATOR", str(DEFAULT_SOURCE / "verify-k0-wu05-bootstrap-v2.py")))

def git(root, *args, check=True):
    p = subprocess.run(["git", *args], cwd=root, text=True, capture_output=True,
                       env={**os.environ, "GIT_TERMINAL_PROMPT": "0"})
    if check and p.returncode:
        raise AssertionError(f"git {args}: {p.stderr}")
    return p

def dependency(markers, fail=False):
    return ("import json, sys\n" +
            "markers=" + repr(markers) + "\n" +
            "if '--check-repair-merge' in sys.argv: markers=markers+['WU05_REPAIR_MERGE_PASS']\n" +
            "for m in markers: print(json.dumps({'status':'PASS','marker':m}))\n" +
            ("sys.exit(7)\n" if fail else ""))

class Fixture:
    def __init__(self, folder, *, blank_marker=False, fail_package=False,
                 source_drift=False, conflict=False, missing_helper=False):
        self.folder=Path(folder); self.repo=self.folder/'repo'; self.repo.mkdir()
        git(self.repo,'init','-b','main')
        git(self.repo,'config','user.name','K0 fixture')
        git(self.repo,'config','user.email','fixture@example.invalid')
        self.write('crates/untouched.rs','// unchanged source fixture\n')
        self.write('Cargo.toml','# immutable Cargo fixture\n')
        self.write(f'{ROOT}/conflict.md','common\n')
        git(self.repo,'add','.'); git(self.repo,'commit','-m','synthetic common base')
        self.base=git(self.repo,'rev-parse','HEAD').stdout.strip()
        git(self.repo,'switch','-c',BRANCH)
        self.write('research/preserved.txt','WU-00 through WU-04 fixture evidence\n')
        if conflict: self.write(f'{ROOT}/conflict.md','local independent edit\n')
        git(self.repo,'add','.'); git(self.repo,'commit','-m','synthetic preserved review')
        self.review=git(self.repo,'rev-parse','HEAD').stdout.strip()
        self.review_tree=git(self.repo,'rev-parse','HEAD^{tree}').stdout.strip()
        git(self.repo,'switch','-c',PBRANCH,self.base)
        template=VALIDATOR.read_text()
        constants=runpy.run_path(str(VALIDATOR),run_name='fixture_import')
        for old,new in [('e95ce1e58a603306cb665a6ab91cfe02d279972f',self.review),
                        ('e3621a370297a76907e97730ebd18c5c1e0fb83e',self.review_tree),
                        ('cbdb597fdd58fd1f08a104b4ea8b662dac1f8ba1',self.base),
                        ('13aed8dabfbb5da4381d9d73d3cb0c0403ad5354',self.base)]:
            template=template.replace(old,new)
        paths=constants['BOOTSTRAP_PATHS']+constants['REQUIRED_PACKAGE_PATHS']
        for rel in paths:
            self.write(rel,'{}\n' if rel.endswith('.json') else '# synthetic package fixture\n')
        self.write(f'{ROOT}/WU05_BOOTSTRAP_V2_AUTHORITY.json', json.dumps({
            'schema':'vigilode-k0-wu05-bootstrap-authority/v2','status':'BOUND',
            'revision':2, 'bootstrap_owner':'HOST_CODEX_ORCHESTRATOR'}))
        self.write('tools/k0-wu05-bootstrap-v2.sh',RUNNER.read_text())
        self.write('tools/verify-k0-wu05-bootstrap-v2.py',template)
        self.write('tools/verify-k0-stage-telemetry-plan.py',dependency([] if blank_marker else BMARKS,fail_package))
        self.write('tools/verify-k0-wu05-supplement.py',dependency(SMARKS))
        if source_drift: self.write('crates/untouched.rs','// forbidden package source mutation\n')
        if conflict: self.write(f'{ROOT}/conflict.md','package independent edit\n')
        if missing_helper: (self.repo/'tools/verify-k0-wu05-supplement.py').unlink()
        git(self.repo,'add','.'); git(self.repo,'commit','-m','synthetic pinned package')
        self.package=git(self.repo,'rev-parse','HEAD').stdout.strip()
        self.remote=self.folder/'origin.git'
        git(self.folder,'clone','--bare',str(self.repo),str(self.remote))
        git(self.repo,'remote','add','origin',str(self.remote))
        git(self.repo,'switch',BRANCH)
        self.extracted=self.folder/'extracted-bootstrap.sh'
        self.extracted.write_text(git(self.repo,'show',self.package+':tools/k0-wu05-bootstrap-v2.sh').stdout)
    def write(self,rel,text):
        p=self.repo/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
    def run(self):
        return subprocess.run(['bash',str(self.extracted),'--repo-root',str(self.repo),
                               '--package-sha',self.package],text=True,capture_output=True,
                              timeout=40,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
    def head(self): return git(self.repo,'rev-parse','HEAD').stdout.strip()
    def clean(self): return not git(self.repo,'status','--porcelain=v1').stdout

class BootstrapTests(unittest.TestCase):
    def fixture(self, **kw):
        d=tempfile.TemporaryDirectory(prefix='k0-bootstrap-test-');self.addCleanup(d.cleanup)
        return Fixture(d.name,**kw)
    def test_missing_validator_before_merge_is_expected_and_bootstrap_installs_it(self):
        f=self.fixture()
        old=subprocess.run([sys.executable,'tools/verify-k0-fresh-review-repair.py'],cwd=f.repo,capture_output=True)
        self.assertEqual(old.returncode,2)
        before=(f.repo/'crates/untouched.rs').read_bytes()
        p=f.run();self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertIn('LOCAL_WU05_AUTHORITY_READY',p.stdout)
        self.assertEqual(git(f.repo,'show','-s','--format=%P','HEAD').stdout.strip(),f.review+' '+f.package)
        self.assertEqual((f.repo/'crates/untouched.rs').read_bytes(),before);self.assertTrue(f.clean())
    def test_successful_bootstrap_is_idempotent(self):
        f=self.fixture();p=f.run();self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        first=f.head();p=f.run()
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertEqual(f.head(),first);self.assertTrue(f.clean())
    def test_exit_zero_without_required_markers_cannot_authorize_merge(self):
        f=self.fixture(blank_marker=True);p=f.run()
        self.assertNotEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertEqual(f.head(),f.review);self.assertTrue(f.clean())
    def test_source_in_package_delta_is_rejected_before_merge(self):
        f=self.fixture(source_drift=True);p=f.run()
        self.assertNotEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertEqual(f.head(),f.review);self.assertTrue(f.clean())
    def test_package_validation_failure_preserves_review(self):
        f=self.fixture(fail_package=True);p=f.run()
        self.assertNotEqual(p.returncode,0);self.assertEqual(f.head(),f.review);self.assertTrue(f.clean())
    def test_missing_package_helper_preserves_review(self):
        f=self.fixture(missing_helper=True);p=f.run()
        self.assertNotEqual(p.returncode,0);self.assertEqual(f.head(),f.review);self.assertTrue(f.clean())
    def test_merge_conflict_is_aborted_without_touching_evidence(self):
        f=self.fixture(conflict=True);p=f.run()
        self.assertNotEqual(p.returncode,0);self.assertEqual(f.head(),f.review);self.assertTrue(f.clean())
        self.assertEqual((f.repo/f'{ROOT}/conflict.md').read_text(),'local independent edit\n')
        self.assertNotEqual(git(f.repo,'rev-parse','--verify','MERGE_HEAD',check=False).returncode,0)
    def test_dirty_state_is_preserved_and_not_stashed(self):
        f=self.fixture();f.write('research/unrelated-dirty.txt','keep me\n');p=f.run()
        self.assertNotEqual(p.returncode,0);self.assertEqual(f.head(),f.review)
        self.assertEqual((f.repo/'research/unrelated-dirty.txt').read_text(),'keep me\n')
        self.assertFalse(git(f.repo,'stash','list').stdout)



class RealDependencyFixture:
    """Real package and supplement validators; synthetic Git IDs/metadata only."""
    def __init__(self, folder, *, inherited_repair=False):
        self.folder=Path(folder); self.repo=self.folder/'repo'; self.repo.mkdir()
        base_root=Path(os.environ.get('K0_TEST_PACKAGE_ROOT', str(DEFAULT_SOURCE.parent)))
        git(self.repo,'init','-b','main')
        git(self.repo,'config','user.name','K0 real-validator fixture')
        git(self.repo,'config','user.email','fixture@example.invalid')
        mf=(base_root/'PACKAGE_MANIFEST.sha256').read_text()
        for line in mf.splitlines():
            if not line.strip(): continue
            _,rel=line.split('  ',1)
            self.write(rel,(base_root/rel).read_bytes())
        self.write('PACKAGE_MANIFEST.sha256',mf)
        self.write('crates/untouched.rs','// synthetic source retained byte-for-byte\n')
        git(self.repo,'add','.');git(self.repo,'commit','-m','synthetic base with real package checks')
        self.base=git(self.repo,'rev-parse','HEAD').stdout.strip()
        git(self.repo,'switch','-c',BRANCH)
        self.write('research/preserved.json','{"fixture":true,"keep":true}\n')
        if inherited_repair:
            v=self.repo/'tools/verify-k0-stage-telemetry-plan.py'
            old=hashlib.sha256(v.read_bytes()).hexdigest()
            v.write_text(v.read_text()+'\n# authorized review-side repair fixture\n')
            mf=mf.replace(old,hashlib.sha256(v.read_bytes()).hexdigest())
            self.write('PACKAGE_MANIFEST.sha256',mf)
        git(self.repo,'add','.');git(self.repo,'commit','-m','synthetic preserved review')
        self.review=git(self.repo,'rev-parse','HEAD').stdout.strip()
        self.review_tree=git(self.repo,'rev-parse','HEAD^{tree}').stdout.strip()
        git(self.repo,'switch','-c',PBRANCH,self.base)
        payload_file=Path(os.environ.get('K0_TEST_SUPPLEMENT_PAYLOAD',
                         str(DEFAULT_SOURCE/'verify-k0-wu05-supplement.payload.b64')))
        source=zlib.decompress(base64.b64decode(payload_file.read_bytes())).decode()
        source=source.replace('e95ce1e58a603306cb665a6ab91cfe02d279972f',self.review)
        source=source.replace('e3621a370297a76907e97730ebd18c5c1e0fb83e',self.review_tree)
        source=source.replace('e1124586a4029f86669e7489278c61ef676d61aa',self.base)
        ns={'__name__':'dependency_fixture','__file__':str(payload_file)}
        exec(compile(source,str(payload_file),'exec'),ns)
        fixture_payload=base64.b64encode(zlib.compress(source.encode(),9))+b'\n'
        self.write('tools/verify-k0-wu05-supplement.payload.b64',fixture_payload)
        loader_path=Path(os.environ.get('K0_TEST_SUPPLEMENT_LOADER',
                         str(DEFAULT_SOURCE/'verify-k0-wu05-supplement.py')))
        loader=loader_path.read_text()
        # Rebind the payload checksum only because the two synthetic Git IDs
        # above deliberately changed the payload; runtime code has no override.
        loader=loader.replace(hashlib.sha256(payload_file.read_bytes()).hexdigest(),
                              hashlib.sha256(fixture_payload).hexdigest())
        self.write('tools/verify-k0-wu05-supplement.py',loader)
        supplement_files=ns['SUPPLEMENT_FILES']
        for path in supplement_files:
            if not (self.repo/path).exists(): self.write(path,'{}\n' if path.suffix=='.json' else 'Synthetic metadata only.\n')
        markers=['PACKAGE_CONTRACT_PASS','WU05_SUPPLEMENT_MANIFEST_PASS','LEGACY_REPAIR_BLOBS_PASS',
                 'EXTERNAL_PACKAGE_PIN_PASS','WU05_SUPPLEMENT_AUTHORITY_PASS','WU05_REPAIR_MERGE_PASS','HOSTILE_FIXTURES_PASS']
        ids=['SR-K0-P0-001','SR-K0-P0-002','SR-K0-P1-001','SR-K0-P1-002','SR-K0-P1-003','SR-K0-P1-004']
        self.write(f'{ROOT}/WU05_LOCAL_REPAIR_SUPPLEMENT.json',json.dumps({
            'schema':'vigilode-k0-wu05-local-repair-supplement/v2','status':'BOUND',
            'observed_failure_modes':[{'id':x} for x in ids],
            'required_pre_repair_markers':markers,'legacy_unmanifested_git_blobs':{}}))
        stage_fields=['execution_state','case_id','family','kernel_arm','attempt_id','stage_index',
                      'solver_method','initial_guess_source','initial_true_residual','final_true_residual',
                      'signed_residual_digest','work','geometry','failure']
        raw_fields=['profile','family','attempts','accepted_steps','rejected_steps','rhs_evaluations',
                    'jvp_vectors','linear_matvecs','trace_digest','switching_active','frozen_zeta34_tau',
                    'event_rows','recommendation_rows','hard_gates']
        canonical={
          'raw_cell_contract':{
            'receipt_object_discovery':{'required_receipt_keys':raw_fields},
            'stage_array_discovery':{'required_stage_keys':['attempt_id','stage_index','execution_state','work']}},
          'numerical_payload_projection':{'fields':['raw_schema','raw_status','kernel_arm','tolerance_arm',
            'profile','family','linear_rtol','linear_atol','attempts','accepted_steps','rejected_steps',
            'rhs_evaluations','jvp_vectors','linear_matvecs','trace_digest','switching_active',
            'frozen_zeta34_tau','event_rows','recommendation_rows','hard_gates','stages']},
          'stage_payload_projection':{'fields':stage_fields},
          'campaign_projection':{'hard_gates':sorted(ns['HARD_GATES'])}}
        self.write(f'{ROOT}/evidence/EVIDENCE_V3_CANONICALIZATION.json',json.dumps(canonical))
        self.write(f'{ROOT}/WU05_REPAIR_SUPPLEMENT_MANIFEST.sha256',''.join(
          hashlib.sha256((self.repo/rel).read_bytes()).hexdigest()+'  '+str(rel)+'\n'
          for rel in sorted(supplement_files)))
        checker=VALIDATOR.read_text()
        for old,new in [('e95ce1e58a603306cb665a6ab91cfe02d279972f',self.review),
                        ('e3621a370297a76907e97730ebd18c5c1e0fb83e',self.review_tree),
                        ('cbdb597fdd58fd1f08a104b4ea8b662dac1f8ba1',self.base),
                        ('13aed8dabfbb5da4381d9d73d3cb0c0403ad5354',self.base)]:
            checker=checker.replace(old,new)
        self.write('tools/verify-k0-wu05-bootstrap-v2.py',checker)
        self.write('tools/k0-wu05-bootstrap-v2.sh',RUNNER.read_text())
        self.write(f'{ROOT}/WU05_BOOTSTRAP_V2_AUTHORITY.json',json.dumps({
          'schema':'vigilode-k0-wu05-bootstrap-authority/v2','status':'BOUND','revision':2}))
        self.write(f'{ROOT}/WU05_BOOTSTRAP_V2_HANDOFF.md','Synthetic bootstrap metadata\n')
        self.write(f'{ROOT}/WU05_BOOTSTRAP_V2_CODEX_PROMPT.md','Synthetic prompt metadata\n')
        git(self.repo,'add','.');git(self.repo,'commit','-m','synthetic pin with real validators')
        self.package=git(self.repo,'rev-parse','HEAD').stdout.strip()
        self.remote=self.folder/'origin.git'
        git(self.folder,'clone','--bare',str(self.repo),str(self.remote))
        git(self.repo,'remote','add','origin',str(self.remote))
        git(self.repo,'switch',BRANCH)
        self.extracted=self.folder/'entry.sh';self.extracted.write_text(RUNNER.read_text())
    def write(self,rel,value):
        p=self.repo/rel;p.parent.mkdir(parents=True,exist_ok=True)
        p.write_bytes(value if isinstance(value,bytes) else value.encode())
    def run(self):
        return subprocess.run(['bash',str(self.extracted),'--repo-root',str(self.repo),
          '--package-sha',self.package],text=True,capture_output=True,timeout=40,
          env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})

class ActualDependencyTests(unittest.TestCase):
    def fixture(self,**kw):
        d=tempfile.TemporaryDirectory(prefix='k0-real-dependency-');self.addCleanup(d.cleanup)
        return RealDependencyFixture(d.name,**kw)
    def test_actual_dependency_validators_and_manifest_complete_before_ready(self):
        f=self.fixture();p=f.run()
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertIn('LOCAL_WU05_AUTHORITY_READY',p.stdout)
        self.assertIn('LEGACY_REPAIR_BLOBS_PASS',p.stdout)
        self.assertIn('EXTERNAL_PACKAGE_PIN_PASS',p.stdout)
        self.assertFalse(git(f.repo,'status','--porcelain=v1').stdout)
    def test_inherited_authorized_validator_repair_is_preserved(self):
        f=self.fixture(inherited_repair=True);p=f.run()
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertFalse(git(f.repo,'diff','--name-only',f.review,'HEAD','--',
                              'tools/verify-k0-stage-telemetry-plan.py','PACKAGE_MANIFEST.sha256').stdout)
    def test_existing_structured_auxiliary_marker_fields_are_recognized(self):
        module=runpy.run_path(str(VALIDATOR),run_name='marker_fixture')
        output=json.dumps({'status':'PASS','results':{
            'manifest':{'status':'PASS','marker':'WU05_SUPPLEMENT_MANIFEST_PASS','legacy_marker':'LEGACY_REPAIR_BLOBS_PASS'},
            'authority':{'status':'PASS','marker':'WU05_SUPPLEMENT_AUTHORITY_PASS','pin_marker':'EXTERNAL_PACKAGE_PIN_PASS'}}})
        self.assertTrue({'LEGACY_REPAIR_BLOBS_PASS','EXTERNAL_PACKAGE_PIN_PASS'} <= module['markers_in'](output))
        self.assertFalse(module['markers_in']('{"status":"FAIL","legacy_marker":"LEGACY_REPAIR_BLOBS_PASS"}'))


if __name__=='__main__': unittest.main(verbosity=2)
