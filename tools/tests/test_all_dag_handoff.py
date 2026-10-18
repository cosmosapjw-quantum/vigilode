"""Adversarial source/manifest replacement during a successful child execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("handoff", ROOT / "tools/validate_all_dag_completion.py")
handoff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(handoff)


class HandoffIdentityContracts(unittest.TestCase):
    def run_case(self, child):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / "report"
            report.mkdir()
            (root / "source.txt").write_text("original")
            (report / "SOURCE_MANIFEST.json").write_text(json.dumps({
                "files_sha256": {"source.txt": hashlib.sha256(b"original").hexdigest()}}))
            (report / "DAG.json").write_text(json.dumps({"nodes": [
                {"id": node, "owner": "test", "delivered": ["test"],
                 "evidence": ["source.txt"], "depends_on": []}
                for node in handoff.NODE_IDS]}))
            with patch.object(handoff, "ROOT", root), patch.object(handoff, "REPORT", report), \
                 patch.object(handoff, "native_commands", return_value=[("child", [sys.executable, "-c", child])]):
                result = handoff.execute(root / "execution")
                self.assertTrue(result["verification_before"]["passed"])
                self.assertTrue(result["verification_after"]["passed"])
                self.assertEqual(result["commands"][0]["exit_code"], 0)
                return result

    def test_unchanged_success(self):
        self.assertTrue(self.run_case("print('unchanged')")["passed"])

    def test_source_and_manifest_replacement_cannot_pass(self):
        result = self.run_case("""
from pathlib import Path
import json, hashlib
Path('source.txt').write_text('replacement')
Path('report/SOURCE_MANIFEST.json').write_text(json.dumps({'files_sha256': {
    'source.txt': hashlib.sha256(b'replacement').hexdigest()}}))
""")
        self.assertFalse(result["source_and_dag_unchanged"])
        self.assertFalse(result["passed"])

    def test_valid_dag_replacement_cannot_pass(self):
        result = self.run_case("""
from pathlib import Path
import json
path=Path('report/DAG.json'); data=json.loads(path.read_text())
data['nodes'][0]['owner']='replacement'; path.write_text(json.dumps(data))
""")
        self.assertFalse(result["source_and_dag_unchanged"])
        self.assertFalse(result["passed"])


if __name__ == "__main__":
    unittest.main()
