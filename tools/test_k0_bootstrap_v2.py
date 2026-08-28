"""Git lifecycle tests; synthetic history and dependency validators, never solver evidence.

Only pinned authority IDs are rebound in the fixture copy. Real git fetch,
worktree creation, merge, abort, and repeated invocation are executed.
"""
from __future__ import annotations
import argparse
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

if __name__=='__main__': unittest.main(verbosity=2)
