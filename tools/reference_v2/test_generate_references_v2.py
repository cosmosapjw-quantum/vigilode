from __future__ import annotations

import copy
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import numpy as np

import generate_references_v2 as generator


def fake_artifact(problem):
    times = generator.requested_times(problem)
    states = [[0.0] * problem["dimension"] for _ in times]
    grid_sha = generator.grid_checksum(times)
    state_sha = generator.state_checksum(states)
    evidence = []
    methods = [*generator.GENERATOR["radau_ladder"], generator.GENERATOR["tight_lsoda"]]
    for method in methods:
        evidence.append(
            {
                "label": method["label"],
                "status": "complete",
                "wall_seconds": 0.01,
                "nfev": 1,
                "njev": 1,
                "nlu": 1,
                "process_peak_rss_bytes_at_run_end": 1024,
                "message": None,
            }
        )
    return {
        "schema_version": generator.ARTIFACT_SCHEMA,
        "problem_id": problem["problem_id"],
        "requested_times": times,
        "states": states,
        "canonical_method": generator.GENERATOR["radau_ladder"][2],
        "independent_method": generator.GENERATOR["tight_lsoda"],
        "convergence": {
            "d0_max_grid_wrms": 4.0,
            "d1_max_grid_wrms": 1.0,
            "q": 0.25,
            "richardson_uncertainty_wrms": 1.0 / 3.0,
            "method_disagreement_wrms": 0.2,
            "reference_uncertainty_wrms": 1.0 / 3.0 + 0.2,
            "wrms_basis": {
                "formula_id": generator.WRMS_FORMULA_ID,
                "absolute": 1.0e-10,
                "relative": 1.0e-8,
                "anchor_state_sha256": state_sha,
            },
        },
        "checksums": {"grid_sha256": grid_sha, "state_sha256": state_sha},
        "run_evidence": evidence,
    }


class ReferenceV2Tests(unittest.TestCase):
    def test_manifest_shape_digest_domains_and_order_are_cross_language_golden(self):
        manifest = generator.build_not_run_manifest()
        generator.validate_manifest_layout(manifest)
        self.assertEqual(len(manifest["artifacts"]), 22)
        self.assertEqual(len(manifest["bindings"]), 66)
        self.assertEqual(
            manifest["artifact_set_sha256"],
            "745b6822f15ed70b17541edfec8d76aae240c09cba17961cb6f36130b75a0998",
        )
        self.assertEqual(
            manifest["binding_set_sha256"],
            "f2db52f133f95751395a13f131083bed20952be9be2cb313b97c8c8c81fa2c6c",
        )
        self.assertEqual(
            manifest["producer"]["problem_definition_sha256"],
            "65d720834347250b2f4604da33c7b3634951985c478056d92205a93d382b72ad",
        )
        self.assertEqual(
            manifest["bindings"][0]["reference_checksum_sha256"],
            "e5475c392ea36b5827c1f3e57a28ecbdb32d835199e8b4e5f647dce6733286d1",
        )
        self.assertEqual(
            generator.artifact_set_checksum(list(reversed(manifest["artifacts"]))),
            manifest["artifact_set_sha256"],
        )
        self.assertEqual(
            generator.binding_set_checksum(list(reversed(manifest["bindings"]))),
            manifest["binding_set_sha256"],
        )
        self.assertNotEqual(
            generator.artifact_set_checksum(manifest["artifacts"]),
            generator.legacy.artifact_set_checksum(
                [
                    {
                        "problem": {"problem_id": entry["problem"]["problem_id"]},
                        "artifact_sha256": entry["artifact_sha256"],
                        "grid_sha256": entry["grid_sha256"],
                        "state_sha256": entry["state_sha256"],
                    }
                    for entry in manifest["artifacts"]
                ]
            ),
        )
        # Raw grid/state checksum payload domains intentionally remain v1
        # compatible; only binding and aggregate domains are v2-separated.
        self.assertEqual(
            generator.grid_checksum([0.0, 1.0]),
            "d5faaa8a445674f57b8cd51d266ebb915dbc9369d38343249b26f1f38e74a044",
        )
        self.assertEqual(
            generator.state_checksum([[100.0], [101.0]]),
            "f852937fb3ab93c72b8ca1b149aadf9c4e84aa0fb57572314a7b0654b72e44ef",
        )

    def test_anchor_wrms_is_tight_reference_based_and_swap_sensitive(self):
        anchor = np.asarray([[100.0]])
        left = np.asarray([[99.0]])
        right = np.asarray([[101.0]])
        value = generator.anchor_wrms(left, right, anchor)
        swapped = generator.anchor_wrms(anchor, right, left)
        self.assertAlmostEqual(value, 2.0 / (1.0e-10 + 1.0e-8 * 100.0))
        self.assertAlmostEqual(swapped, 1.0 / (1.0e-10 + 1.0e-8 * 99.0))
        self.assertNotEqual(value, swapped)

    def test_python_callbacks_consume_both_shared_mpmath_oracles(self):
        generator.validate_source_equation_oracles()

    def test_atomic_create_is_create_new(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "value.json"
            generator.atomic_create(path, b"first")
            with self.assertRaises(generator.V2ReferenceError):
                generator.atomic_create(path, b"second")
            self.assertEqual(path.read_bytes(), b"first")

    def test_partition_resume_checkpoint_and_failure_preservation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest_path = root / "reference_manifest_v2.json"
            generator.atomic_create(manifest_path, generator.canonical_json(generator.build_not_run_manifest()))
            with mock.patch.object(generator, "require_exact_runtime"):
                generator.generate_partition(
                    manifest_path, 0, 22, root / "checkpoint-1.json", False, fake_artifact
                )
                generator.generate_partition(
                    manifest_path, 0, 22, root / "checkpoint-2.json", True, fake_artifact
                )
            first = json.loads((root / "checkpoint-1.json").read_text())
            second = json.loads((root / "checkpoint-2.json").read_text())
            self.assertEqual(len(first["completed_problem_ids"]), 1)
            self.assertEqual(second["resumed_problem_ids"], first["completed_problem_ids"])

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest_path = root / "reference_manifest_v2.json"
            generator.atomic_create(manifest_path, generator.canonical_json(generator.build_not_run_manifest()))

            def fail(problem):
                raise generator.ArtifactGenerationFailed(
                    "intentional",
                    [
                        {
                            "label": "L0",
                            "status": "failed",
                            "wall_seconds": 0.01,
                            "nfev": 7,
                            "njev": 2,
                            "nlu": 2,
                            "process_peak_rss_bytes_at_run_end": 2048,
                            "message": "intentional",
                        }
                    ],
                )

            with mock.patch.object(generator, "require_exact_runtime"):
                with self.assertRaises(generator.V2ReferenceError):
                    generator.generate_partition(
                        manifest_path, 0, 22, root / "failure-checkpoint.json", False, fail
                    )
            checkpoint = json.loads((root / "failure-checkpoint.json").read_text())
            self.assertEqual(len(checkpoint["failures"]), 1)
            failure = json.loads((root / checkpoint["failures"][0]["failure_path"]).read_text())
            self.assertEqual(failure["run_evidence"][0]["nfev"], 7)

    def test_assemble_and_full_self_check_reject_missing_or_corrupt_any_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "not-run.json"
            manifest = generator.build_not_run_manifest()
            generator.atomic_create(source, generator.canonical_json(manifest))
            for entry in manifest["artifacts"]:
                path = root / entry["artifact_path"]
                generator.atomic_create(path, generator.canonical_json(fake_artifact(entry["problem"])))
            complete = root / "complete.json"
            with mock.patch.object(generator, "require_exact_runtime"), mock.patch.dict(
                os.environ, {"VIGILODE_CODE_REVISION": "1" * 40}
            ):
                generator.assemble_complete_manifest(source, complete)
                generator.self_check(complete)
                last = root / manifest["artifacts"][-1]["artifact_path"]
                last.write_bytes(last.read_bytes() + b" ")
                with self.assertRaises(generator.V2ReferenceError):
                    generator.self_check(complete)
                last.unlink()
                with self.assertRaises(generator.V2ReferenceError):
                    generator.self_check(complete)

    def test_layout_and_artifact_fail_closed_on_runtime_formula_anchor_and_method_mutations(self):
        manifest = generator.build_not_run_manifest()
        changed = copy.deepcopy(manifest)
        changed["runtime"]["numpy_record_verified_file_count"] -= 1
        with self.assertRaises(generator.V2ReferenceError):
            generator.validate_manifest_layout(changed)

        dishonest_complete = copy.deepcopy(manifest)
        dishonest_complete["generation_status"] = "complete"
        dishonest_complete["producer"]["implementation_revision"] = "1" * 40
        with self.assertRaises(generator.V2ReferenceError):
            generator.validate_manifest_layout(dishonest_complete)

        for mutation in ("artifact-set", "binding-set", "binding", "order", "path", "case-id"):
            changed = copy.deepcopy(manifest)
            if mutation == "artifact-set":
                changed["artifact_set_sha256"] = "f" * 64
            elif mutation == "binding-set":
                changed["binding_set_sha256"] = "f" * 64
            elif mutation == "binding":
                changed["bindings"][0]["reference_checksum_sha256"] = "f" * 64
            elif mutation == "order":
                changed["artifacts"][0], changed["artifacts"][1] = (
                    changed["artifacts"][1], changed["artifacts"][0]
                )
            elif mutation == "path":
                changed["artifacts"][1]["artifact_path"] = changed["artifacts"][0]["artifact_path"]
            else:
                changed["bindings"][0]["case_id"] = changed["bindings"][0]["case_id"].replace(
                    "1e-4", "1e-04"
                )
            with self.assertRaises(generator.V2ReferenceError, msg=mutation):
                generator.validate_manifest_layout(changed)

        problem = manifest["artifacts"][0]["problem"]
        artifact = fake_artifact(problem)
        for mutation in ("formula", "anchor", "method", "state", "scale"):
            changed_artifact = copy.deepcopy(artifact)
            if mutation == "formula":
                changed_artifact["convergence"]["wrms_basis"]["formula_id"] = "pairwise-max"
            elif mutation == "anchor":
                changed_artifact["convergence"]["wrms_basis"]["anchor_state_sha256"] = "0" * 64
            elif mutation == "method":
                changed_artifact["canonical_method"] = generator.GENERATOR["radau_ladder"][1]
            elif mutation == "state":
                changed_artifact["states"][0][0] = 1.0
            else:
                changed_artifact["convergence"]["wrms_basis"]["absolute"] = 2.0e-10
            with self.assertRaises(generator.V2ReferenceError):
                generator.validate_artifact(changed_artifact, problem)


if __name__ == "__main__":
    unittest.main()
