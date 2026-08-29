from __future__ import annotations

import copy
import json
import math
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import numpy as np
from scipy.sparse import csc_matrix

import external_evidence as external


REVISION = "1" * 40
SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64


def small_context(*, breakpoint: bool = False) -> dict:
    problem = {
        "problem_id": "small-contract-v2",
        "family": "robertson-ramped",
        "partition": "calibration",
        "dimension": 3,
        "grid_shape": None,
        "t_span": [0.0, 0.001],
        "uniform_output_points": 3,
        "mandatory_breakpoints": [0.0005] if breakpoint else [],
        "source": {},
    }
    return {
        "binding": {
            "case_id": "small-contract-rtol-1e-4-v2.1",
            "problem_id": problem["problem_id"],
            "reference_checksum_sha256": SHA_A,
        },
        "entry": {"problem": problem},
        "problem": problem,
        "requested_times": [0.0, 0.0005, 0.001],
        "rtol": 1.0e-4,
        "atol": 1.0e-6,
    }


def small_manifest() -> dict:
    return {
        "producer": {"problem_definition_sha256": SHA_B},
        "artifact_set_sha256": SHA_A,
        "binding_set_sha256": SHA_C,
    }


def canonical_freeze() -> dict:
    campaign = external.expected_canonical_campaign_binding(REVISION)
    rows = []
    for index, case in enumerate(external.calibration_case_contracts()):
        rows.append(
            {
                "case_id": case["case_id"],
                "family": case["family"],
                "partition": "calibration",
                "dimension": case["dimension"],
                "atol": case["atol"],
                "rtol": case["rtol"],
                "status": "pass",
                "conservative_max_wrms": float(index + 1),
                "binding": {
                    "campaign": campaign,
                    "reference_checksum_sha256": "4" * 64,
                    "clipped_output_checksum_sha256": "5" * 64,
                    "dense_output_checksum_sha256": "6" * 64,
                },
                "evidence": "finite canonical test evidence",
                "wall_seconds": float(index) / 10.0,
            }
        )
    threshold = max(row["conservative_max_wrms"] for row in rows)
    payload = {
        "schema": "scientific-validity-v2-calibration-freeze-v1",
        "corpus_version": external.reference.CORPUS_VERSION,
        "profile": "canonical",
        "campaign_label": "canonical-scientific-campaign",
        "threshold_derivation_id": "scientific-validity-v2-conservative-max-wrms-v1",
        "campaign_binding": campaign,
        "predeclared_holdout_family": "oregonator",
        "sealed_remaining_holdout_families": [
            "pollution",
            "medical-akzo",
            "brusselator-2d",
        ],
        "conservative_threshold_wrms": threshold,
        "conservative_threshold_bits": external.f64_bits(threshold),
        "rows": rows,
    }
    return {
        "payload": payload,
        "checksum_sha256": external.calibration_freeze_checksum(payload),
    }


def canonical_campaign(freeze: dict | None = None) -> dict:
    freeze = freeze or canonical_freeze()
    rows = copy.deepcopy(freeze["payload"]["rows"])
    records = []
    ledger = []
    for row in rows:
        artifact_checksum = external.sha256_bytes(
            ("artifact:" + row["case_id"]).encode("utf-8")
        )
        records.append(
            {
                "status": "complete",
                "artifact": {
                    "spec": {"id": row["case_id"]},
                    "row": copy.deepcopy(row),
                    "code_revision": REVISION,
                    "artifact_checksum_sha256": artifact_checksum,
                },
            }
        )
        ledger.append(
            {
                "case_id": row["case_id"],
                "status": "complete",
                "artifact_checksum_sha256": artifact_checksum,
            }
        )
    ledger_bytes = json.dumps(
        ledger, ensure_ascii=False, allow_nan=False, separators=(",", ":")
    ).encode("utf-8")
    return {
        "schema": "scientific-validity-v2-calibration-campaign-v1",
        "status": "complete-pass",
        "corpus_version": external.reference.CORPUS_VERSION,
        "code_revision": REVISION,
        "expected_case_count": 54,
        "attempted_case_count": 54,
        "failure_count": 0,
        "freeze_eligible": True,
        "freeze_checksum_sha256": freeze["checksum_sha256"],
        "freeze_admission_error": None,
        "record_set_sha256": external.sha256_bytes(
            b"vigilode-scientific-v2-campaign-record-set-v1\0" + ledger_bytes
        ),
        "records": records,
        "rows": rows,
    }


def deterministic_probe() -> dict:
    probe = {
        "cvode_available": False,
        "executable_names_checked": ["cvode"],
        "pkg_config_modules_checked": ["sundials-cvode"],
        "header_paths_checked": ["/usr/include/cvode/cvode.h"],
        "library_names_checked": ["libsundials_cvode.so"],
        "python_modules_checked": ["scikits.odes"],
        "ida_only_version": "6.4.1",
        "probe_findings": [
            {
                "category": "executable",
                "target": "cvode",
                "observed": False,
                "detail": "not found",
            },
            {
                "category": "pkg-config",
                "target": "sundials-cvode",
                "observed": False,
                "detail": "not found",
            },
            {
                "category": "header",
                "target": "/usr/include/cvode/cvode.h",
                "observed": False,
                "detail": "not found",
            },
            {
                "category": "library",
                "target": "libsundials_cvode.so",
                "observed": False,
                "detail": "not found",
            },
            {
                "category": "python-module",
                "target": "scikits.odes",
                "observed": False,
                "detail": "not found",
            },
        ],
    }
    probe["probe_evidence_sha256"] = external.sundials_probe_evidence_checksum(probe)
    return probe


class ChecksumAndFreezeTests(unittest.TestCase):
    def test_runtime_and_probe_checksums_have_fixed_serde_compatible_goldens(
        self,
    ) -> None:
        runtime = {"kind": "scipy-python", "identity": external.reference.RUNTIME}
        self.assertEqual(
            external.external_runtime_identity_checksum(runtime),
            "409aa09faf49a92445a7ffaeccf785898531b6e27d16064664f4e6be193c56bf",
        )
        self.assertEqual(
            external.external_runtime_identity_checksum(
                json.loads(external.canonical_json(runtime))
            ),
            "409aa09faf49a92445a7ffaeccf785898531b6e27d16064664f4e6be193c56bf",
        )
        self.assertEqual(
            deterministic_probe()["probe_evidence_sha256"],
            "99b33e803d752d34a82e4bd8eceeee4d75d325c683fb1050a229e34c9e73ee05",
        )
        canonical_probe = json.loads(external.canonical_json(deterministic_probe()))
        self.assertEqual(
            external.sundials_probe_evidence_checksum(canonical_probe),
            canonical_probe["probe_evidence_sha256"],
        )

    def test_checked_in_fixtures_survive_canonical_json_reload(self) -> None:
        fixture_dir = Path(__file__).parent / "fixtures"
        for path in sorted(fixture_dir.glob("*.json")):
            with self.subTest(path=path.name):
                raw = path.read_bytes()
                evidence = json.loads(raw)
                self.assertEqual(raw, external.canonical_json(evidence))
                external.validate_external_evidence(evidence)

    def test_runner_dependency_closure_covers_every_local_executable_input(
        self,
    ) -> None:
        entries = external.runner_dependency_closure_entries()
        self.assertEqual(
            [entry["path"] for entry in entries],
            sorted(external.RUNNER_DEPENDENCY_PATHS),
        )
        original = external.runner_dependency_closure_checksum(entries)
        mutated = copy.deepcopy(entries)
        mutated[0]["sha256"] = "f" * 64
        self.assertNotEqual(
            original,
            external.runner_dependency_closure_checksum(mutated),
        )
        self.assertEqual(
            external.scipy_runner_binding()["dependency_closure_sha256"],
            original,
        )

    def test_canonical_freeze_is_verified_and_wall_time_is_not_authenticated(
        self,
    ) -> None:
        freeze = canonical_freeze()
        self.assertEqual(
            freeze["checksum_sha256"],
            "d50e3c29c8206ec902281db0395fe395a480559945c5778cec7e5f7bf511ce1a",
        )
        self.assertEqual(
            external.verify_calibration_freeze(freeze, REVISION),
            freeze["checksum_sha256"],
        )
        modified = copy.deepcopy(freeze)
        modified["payload"]["rows"][0]["wall_seconds"] += 1000.0
        self.assertEqual(
            external.verify_calibration_freeze(modified, REVISION),
            freeze["checksum_sha256"],
        )

    def test_every_scientific_freeze_mutation_is_rejected(self) -> None:
        mutators = (
            lambda value: value["payload"].__setitem__("corpus_version", "wrong"),
            lambda value: value["payload"]["rows"][0].__setitem__("status", "fail"),
            lambda value: value["payload"]["rows"][0].__setitem__("rtol", 2.0e-4),
            lambda value: value["payload"]["rows"][0]["binding"].__setitem__(
                "reference_checksum_sha256", "7" * 64
            ),
            lambda value: value.__setitem__("checksum_sha256", "8" * 64),
        )
        for mutate in mutators:
            with self.subTest(mutate=mutate):
                freeze = canonical_freeze()
                mutate(freeze)
                with self.assertRaises(external.ExternalEvidenceError):
                    external.verify_calibration_freeze(freeze, REVISION)

    def test_complete_campaign_is_required_and_bound_to_freeze_artifacts(self) -> None:
        freeze = canonical_freeze()
        campaign = canonical_campaign(freeze)
        self.assertIsNone(
            external.verify_calibration_campaign(campaign, freeze, REVISION)
        )
        corrupted = copy.deepcopy(campaign)
        corrupted["records"][0]["artifact"]["row"]["evidence"] = "self asserted"
        with self.assertRaises(external.ExternalEvidenceError):
            external.verify_calibration_campaign(corrupted, freeze, REVISION)

    def test_oregonator_binds_the_exact_consumed_campaign_file_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            freeze = canonical_freeze()
            campaign = canonical_campaign(freeze)
            freeze_path = root / "freeze.json"
            campaign_path = root / "campaign.json"
            freeze_path.write_bytes(external.canonical_json(freeze))
            compact = external.canonical_json(campaign)
            pretty = json.dumps(
                campaign, ensure_ascii=False, allow_nan=False, indent=2
            ).encode("utf-8")
            self.assertEqual(json.loads(compact), json.loads(pretty))
            self.assertNotEqual(
                external.sha256_bytes(compact), external.sha256_bytes(pretty)
            )
            arguments = Namespace(
                freeze=freeze_path,
                calibration_campaign=campaign_path,
                code_revision=REVISION,
                reference_manifest=root / "must-not-open.json",
                output_dir=root / "must-not-create",
                aggregate=root / "must-not-create.json",
                partition=(0, 1),
                comparator="all",
                resume=False,
            )
            for pinned in (compact, pretty):
                with self.subTest(layout="compact" if pinned is compact else "pretty"):
                    campaign_path.write_bytes(pinned)
                    with mock.patch.object(
                        external, "run_evidence_set", return_value={}
                    ) as run:
                        external.run_oregonator_command(arguments)
                    self.assertEqual(
                        run.call_args.kwargs["calibration_authority"][
                            "campaign_file_sha256"
                        ],
                        external.sha256_bytes(pinned),
                    )

    def test_oregonator_rejects_freeze_before_reference_or_holdout_access(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            freeze_path = Path(directory) / "freeze.json"
            freeze = canonical_freeze()
            freeze["checksum_sha256"] = "9" * 64
            freeze_path.write_text(json.dumps(freeze), encoding="utf-8")
            arguments = Namespace(
                freeze=freeze_path,
                calibration_campaign=Path(directory) / "must-not-open-campaign.json",
                code_revision=REVISION,
                reference_manifest=Path(directory) / "must-not-open.json",
                output_dir=Path(directory) / "must-not-create",
                aggregate=Path(directory) / "must-not-create.json",
                partition=(0, 1),
                comparator="all",
                resume=False,
            )
            with mock.patch.object(external, "run_evidence_set") as run:
                with self.assertRaises(external.ExternalEvidenceError):
                    external.run_oregonator_command(arguments)
                run.assert_not_called()

    def test_oregonator_rejects_missing_campaign_before_reference_access(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            freeze_path = Path(directory) / "freeze.json"
            freeze_path.write_bytes(external.canonical_json(canonical_freeze()))
            arguments = Namespace(
                freeze=freeze_path,
                calibration_campaign=Path(directory) / "missing-campaign.json",
                code_revision=REVISION,
                reference_manifest=Path(directory) / "must-not-open-reference.json",
                output_dir=Path(directory) / "must-not-create",
                aggregate=Path(directory) / "must-not-create.json",
                partition=(0, 1),
                comparator="all",
                resume=False,
            )
            with mock.patch.object(external, "run_evidence_set") as run:
                with self.assertRaises(external.ExternalEvidenceError):
                    external.run_oregonator_command(arguments)
                run.assert_not_called()

    def test_valid_checksum_wrong_canonical_binding_cannot_unlock_oregonator(
        self,
    ) -> None:
        freeze = canonical_freeze()
        freeze["payload"]["campaign_binding"]["candidate_id"] = "different-candidate"
        for row in freeze["payload"]["rows"]:
            row["binding"]["campaign"] = freeze["payload"]["campaign_binding"]
        freeze["checksum_sha256"] = external.calibration_freeze_checksum(
            freeze["payload"]
        )
        with self.assertRaisesRegex(
            external.ExternalEvidenceError, "exact current canonical RODAS5P"
        ):
            external.verify_calibration_freeze(freeze, REVISION)


class SolverEvidenceTests(unittest.TestCase):
    def test_real_scipy_radau_smoke_uses_dense_output_and_native_work(self) -> None:
        outcome = external.solve_scipy_radau(small_context())
        self.assertTrue(outcome.success, outcome.reason)
        self.assertEqual(outcome.times, [0.0, 0.0005, 0.001])
        self.assertEqual(len(outcome.states), 3)
        self.assertGreater(outcome.nfev, 0)
        self.assertGreater(outcome.njev, 0)
        self.assertGreater(outcome.nlu, 0)

    def test_branch_fixed_segments_deduplicate_exact_endpoint(self) -> None:
        context = small_context(breakpoint=True)

        def runtime(_problem):
            rhs = lambda _time, state: -state
            jac = lambda _time, _state: csc_matrix(
                [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]]
            )
            return rhs, jac, np.asarray([1.0, 2.0, 3.0]), None

        with mock.patch.object(
            external.reference, "problem_runtime", side_effect=runtime
        ):
            outcome = external.solve_scipy_radau(context)
        self.assertTrue(outcome.success, outcome.reason)
        self.assertEqual(outcome.times, [0.0, 0.0005, 0.001])
        self.assertEqual(
            sum(external.same_f64(value, 0.0005) for value in outcome.times), 1
        )

    def test_solver_failure_preserves_finite_initial_prefix_and_actual_zero_work(
        self,
    ) -> None:
        context = small_context()
        failed = SimpleNamespace(
            success=False,
            message="deliberate failure",
            nfev=0,
            njev=0,
            nlu=0,
            t=np.asarray([0.0]),
            y=np.asarray([[1.0], [0.0], [0.0]]),
            sol=lambda values: np.asarray(
                [[1.0] * len(values), [0.0] * len(values), [0.0] * len(values)]
            ),
        )
        with mock.patch.object(external.reference, "solve_ivp", return_value=failed):
            evidence, _ = external.scipy_evidence(context, small_manifest(), REVISION)
        self.assertEqual(evidence["status"]["kind"], "solver-failure")
        self.assertEqual(evidence["committed_times"], [0.0])
        self.assertEqual(
            evidence["native_work"],
            {"kind": "scipy-radau", "nfev": 0, "njev": 0, "nlu": 0},
        )
        external.validate_external_evidence(evidence)

    def test_pre_result_exception_preserves_observed_callback_work(self) -> None:
        context = small_context()

        def fail_after_callbacks(fun, t_span, y0, *, jac, **_options):
            for _ in range(7):
                fun(t_span[0], y0)
            for _ in range(2):
                jac(t_span[0], y0)
            raise RuntimeError("deliberate pre-result failure")

        with mock.patch.object(
            external.reference, "solve_ivp", side_effect=fail_after_callbacks
        ):
            evidence, _ = external.scipy_evidence(context, small_manifest(), REVISION)
        self.assertEqual(evidence["status"]["kind"], "solver-failure")
        self.assertEqual(evidence["committed_times"], [0.0])
        self.assertEqual(
            evidence["native_work"],
            {"kind": "scipy-radau", "nfev": 7, "njev": 2, "nlu": 0},
        )
        self.assertIn(
            "native nlu unavailable before OdeResult",
            evidence["status"]["detail"]["reason"],
        )
        external.validate_external_evidence(evidence)

    def test_dense_evaluation_exception_is_a_typed_failure_not_a_partition_abort(
        self,
    ) -> None:
        context = small_context()

        def explode(_values):
            raise FloatingPointError("deliberate dense failure")

        result = SimpleNamespace(
            success=True,
            message="success",
            nfev=17,
            njev=2,
            nlu=4,
            t=np.asarray([0.0, 0.001]),
            y=np.asarray([[1.0, 0.9], [0.0, 0.01], [0.0, 0.09]]),
            sol=explode,
        )
        with mock.patch.object(external.reference, "solve_ivp", return_value=result):
            evidence, _ = external.scipy_evidence(context, small_manifest(), REVISION)
        self.assertEqual(evidence["status"]["kind"], "solver-failure")
        self.assertEqual(evidence["committed_times"], [0.0])
        self.assertEqual(
            evidence["native_work"],
            {"kind": "scipy-radau", "nfev": 17, "njev": 2, "nlu": 4},
        )
        self.assertIn(
            "dense-output evaluation failed", evidence["status"]["detail"]["reason"]
        )

    def test_sundials_unavailable_is_typed_without_states_work_or_source_revision(
        self,
    ) -> None:
        evidence, wall = external.sundials_unavailable_evidence(
            small_context(), small_manifest(), REVISION, deterministic_probe()
        )
        self.assertEqual(wall, 0.0)
        self.assertEqual(evidence["status"]["kind"], "unavailable")
        self.assertIsNone(evidence["states"])
        self.assertIsNone(evidence["native_work"])
        self.assertEqual(evidence["runner"]["source_revision"], "not-observed")
        self.assertEqual(evidence["runner"]["source_sha256"], external.ZERO_SHA256)
        self.assertFalse(evidence["runner"]["observed_upstream_identity"])
        self.assertFalse(
            evidence["reference_dependency"]["shares_implementation_lineage"]
        )
        mutated = copy.deepcopy(evidence)
        mutated["runner"]["dependency_closure_sha256"] = "f" * 64
        with self.assertRaises(external.ExternalEvidenceError):
            external.validate_external_evidence(mutated)

    def test_scipy_evidence_is_explicitly_correlated_and_not_independent_ranking(
        self,
    ) -> None:
        context = small_context()
        with mock.patch.object(
            external,
            "solve_scipy_radau",
            return_value=external.ScipySolveOutcome(
                True,
                "success",
                context["requested_times"],
                [[1.0, 0.0, 0.0]] * 3,
                7,
                1,
                2,
            ),
        ):
            evidence, _ = external.scipy_evidence(context, small_manifest(), REVISION)
        dependency = evidence["reference_dependency"]
        self.assertTrue(dependency["shares_implementation_lineage"])
        self.assertEqual(
            dependency["reference_lineage_id"], dependency["runner_lineage_id"]
        )


class PersistenceAndCampaignTests(unittest.TestCase):
    def test_atomic_create_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "evidence.json"
            external.atomic_create(path, b"first")
            with self.assertRaises(external.ExternalEvidenceError):
                external.atomic_create(path, b"second")
            self.assertEqual(path.read_bytes(), b"first")

    def test_evidence_set_checksum_excludes_wall_time_but_not_artifact_identity(
        self,
    ) -> None:
        records = [
            {
                "case_id": "case",
                "comparator": "scipy-radau",
                "artifact_path": "scipy-radau/case.json",
                "artifact_sha256": SHA_A,
                "status": "success",
                "wall_seconds": 1.0,
            }
        ]
        manifest = small_manifest()
        first = external.evidence_set_checksum(
            "calibration", REVISION, manifest, SHA_C, SHA_A, 0, 1, 54, 54, records
        )
        records[0]["wall_seconds"] = 999.0
        self.assertEqual(
            first,
            external.evidence_set_checksum(
                "calibration",
                REVISION,
                manifest,
                SHA_C,
                SHA_A,
                0,
                1,
                54,
                54,
                records,
            ),
        )
        records[0]["artifact_sha256"] = SHA_B
        self.assertNotEqual(
            first,
            external.evidence_set_checksum(
                "calibration",
                REVISION,
                manifest,
                SHA_C,
                SHA_A,
                0,
                1,
                54,
                54,
                records,
            ),
        )
        holdout_authority = {
            "freeze_checksum_sha256": SHA_A,
            "campaign_file_sha256": SHA_B,
        }
        holdout = external.evidence_set_checksum(
            "oregonator",
            REVISION,
            manifest,
            SHA_C,
            SHA_A,
            0,
            1,
            3,
            3,
            records,
            holdout_authority,
        )
        holdout_authority["campaign_file_sha256"] = SHA_C
        self.assertNotEqual(
            holdout,
            external.evidence_set_checksum(
                "oregonator",
                REVISION,
                manifest,
                SHA_C,
                SHA_A,
                0,
                1,
                3,
                3,
                records,
                holdout_authority,
            ),
        )
        records[0]["artifact_sha256"] = SHA_A
        self.assertNotEqual(
            first,
            external.evidence_set_checksum(
                "calibration",
                REVISION,
                manifest,
                SHA_B,
                SHA_A,
                0,
                1,
                54,
                54,
                records,
            ),
        )
        self.assertNotEqual(
            first,
            external.evidence_set_checksum(
                "calibration",
                REVISION,
                manifest,
                SHA_C,
                SHA_B,
                0,
                1,
                54,
                54,
                records,
            ),
        )

    def test_partial_empty_and_unavailable_sets_never_claim_full_completion(
        self,
    ) -> None:
        partial_records = [{"status": "success"} for _ in range(27)]
        partial = external.evidence_set_completion(
            expected_case_count=54,
            selected_case_count=27,
            comparator_count=1,
            partition_count=2,
            records=partial_records,
        )
        self.assertEqual(partial["status"], "partition-complete")
        self.assertTrue(partial["partition_covered"])
        self.assertFalse(partial["full_surface_complete"])

        empty = external.evidence_set_completion(
            expected_case_count=54,
            selected_case_count=0,
            comparator_count=1,
            partition_count=100,
            records=[],
        )
        self.assertEqual(empty["status"], "empty-partition")
        self.assertFalse(empty["full_surface_complete"])

        all_with_unavailable = external.evidence_set_completion(
            expected_case_count=54,
            selected_case_count=54,
            comparator_count=2,
            partition_count=1,
            records=[{"status": "success"} for _ in range(54)]
            + [{"status": "unavailable"} for _ in range(54)],
        )
        self.assertEqual(
            all_with_unavailable["status"], "full-surface-with-unavailable"
        )
        self.assertEqual(all_with_unavailable["status_counts"]["unavailable"], 54)
        self.assertFalse(all_with_unavailable["full_surface_complete"])

    def test_resume_rejects_record_count_reference_and_checksum_mutations(self) -> None:
        manifest = small_manifest()
        records = [
            {
                "case_id": "case",
                "problem_id": "problem",
                "comparator": "scipy-radau",
                "artifact_path": "scipy-radau/case.json",
                "artifact_sha256": SHA_A,
                "status": "success",
                "resumed": False,
                "wall_seconds": 1.0,
            }
        ]
        aggregate = {
            "schema_version": external.EVIDENCE_SET_SCHEMA,
            "status": "partition-complete",
            "mode": "calibration",
            "corpus_version": external.reference.CORPUS_VERSION,
            "implementation_revision": REVISION,
            "reference_manifest_sha256": SHA_C,
            "reference_artifact_set_sha256": manifest["artifact_set_sha256"],
            "reference_binding_set_sha256": manifest["binding_set_sha256"],
            "runner_dependency_closure_sha256": external.runner_dependency_closure_checksum(),
            "partition": {"index": 0, "count": 2},
            "expected_case_count": 54,
            "selected_case_count": 1,
            "comparator_selection": ["scipy-radau"],
            "expected_record_count": 54,
            "selected_record_count": 1,
            "record_count": 1,
            "status_counts": {
                "success": 1,
                "solver-failure": 0,
                "unavailable": 0,
                "not-run": 0,
                "non-applicable": 0,
            },
            "partition_covered": True,
            "full_surface_covered": False,
            "full_surface_complete": False,
            "records": records,
            "wall_seconds": 1.0,
        }
        aggregate["scientific_set_sha256"] = external.evidence_set_checksum(
            "calibration",
            REVISION,
            manifest,
            SHA_C,
            aggregate["runner_dependency_closure_sha256"],
            0,
            2,
            54,
            54,
            records,
        )
        external.validate_resumed_aggregate(
            copy.deepcopy(aggregate), aggregate, manifest
        )
        for label, mutate in (
            (
                "record",
                lambda value: value["records"][0].__setitem__("artifact_sha256", SHA_B),
            ),
            ("record-count", lambda value: value.__setitem__("record_count", 2)),
            (
                "reference",
                lambda value: value.__setitem__("reference_manifest_sha256", SHA_B),
            ),
            (
                "expected-count",
                lambda value: value.__setitem__("expected_record_count", 108),
            ),
        ):
            with self.subTest(label=label):
                mutated = copy.deepcopy(aggregate)
                mutate(mutated)
                with self.assertRaises(external.ExternalEvidenceError):
                    external.validate_resumed_aggregate(mutated, aggregate, manifest)

    def test_campaign_continues_after_solver_failure_and_preserves_all_records(
        self,
    ) -> None:
        contexts = []
        for index in range(3):
            context = small_context()
            context["binding"] = dict(context["binding"])
            context["binding"]["case_id"] = f"case-{index}-rtol-1e-4-v2.1"
            contexts.append(context)

        calls: list[str] = []

        def build(context, manifest, revision, _runner=None):
            calls.append(context["binding"]["case_id"])
            evidence = external.base_evidence(
                "scipy-radau",
                external.scipy_runner_binding(),
                context,
                manifest,
                revision,
            )
            times = context["requested_times"]
            states = [[1.0, 0.0, 0.0]] * len(times)
            evidence["committed_times"] = times
            evidence["states"] = states
            evidence["checksums"]["committed_grid_sha256"] = (
                external.reference.grid_checksum(times)
            )
            evidence["checksums"]["state_sha256"] = external.reference.state_checksum(
                states
            )
            evidence["native_work"] = {
                "kind": "scipy-radau",
                "nfev": 1,
                "njev": 0,
                "nlu": 0,
            }
            evidence["status"] = (
                {"kind": "solver-failure", "detail": {"reason": "first failed"}}
                if len(calls) == 1
                else {"kind": "success"}
            )
            return evidence, 0.01

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "manifest.json").write_text("{}", encoding="utf-8")
            with (
                mock.patch.object(
                    external,
                    "load_complete_reference_manifest",
                    return_value=small_manifest(),
                ),
                mock.patch.object(
                    external, "selected_case_contexts", return_value=contexts
                ),
                mock.patch.object(external, "scipy_evidence", side_effect=build),
            ):
                aggregate = external.run_evidence_set(
                    mode="calibration",
                    manifest_path=root / "manifest.json",
                    output_dir=root / "artifacts",
                    aggregate_path=root / "aggregate.json",
                    revision=REVISION,
                    partition=(0, 1),
                    comparators="scipy-radau",
                    resume=False,
                )
        self.assertEqual(calls, [context["binding"]["case_id"] for context in contexts])
        self.assertEqual(aggregate["record_count"], 3)
        self.assertEqual(aggregate["status_counts"]["solver-failure"], 1)
        self.assertEqual(aggregate["status"], "partition-with-solver-failures")
        self.assertFalse(aggregate["full_surface_complete"])

    def test_manifest_and_dependency_closure_are_pinned_for_whole_run(self) -> None:
        context = small_context()

        def success(context, manifest, revision, runner):
            evidence = external.base_evidence(
                "scipy-radau", runner, context, manifest, revision
            )
            times = context["requested_times"]
            states = [[1.0, 0.0, 0.0]] * len(times)
            evidence["status"] = {"kind": "success"}
            evidence["committed_times"] = times
            evidence["states"] = states
            evidence["native_work"] = {
                "kind": "scipy-radau",
                "nfev": 1,
                "njev": 0,
                "nlu": 0,
            }
            evidence["checksums"]["committed_grid_sha256"] = (
                external.reference.grid_checksum(times)
            )
            evidence["checksums"]["state_sha256"] = external.reference.state_checksum(
                states
            )
            return evidence, 0.01

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest_path = root / "manifest.json"
            manifest_path.write_text("{}", encoding="utf-8")

            def mutate_manifest(*arguments):
                manifest_path.write_text('{"changed":true}', encoding="utf-8")
                return success(*arguments)

            with (
                mock.patch.object(
                    external,
                    "load_complete_reference_manifest",
                    return_value=small_manifest(),
                ),
                mock.patch.object(
                    external, "selected_case_contexts", return_value=[context]
                ),
                mock.patch.object(
                    external, "scipy_evidence", side_effect=mutate_manifest
                ),
            ):
                with self.assertRaisesRegex(
                    external.ExternalEvidenceError,
                    "reference manifest changed during external evidence execution",
                ):
                    external.run_evidence_set(
                        mode="calibration",
                        manifest_path=manifest_path,
                        output_dir=root / "manifest-mutation-artifacts",
                        aggregate_path=root / "manifest-mutation-aggregate.json",
                        revision=REVISION,
                        partition=(0, 1),
                        comparators="scipy-radau",
                        resume=False,
                    )
            self.assertFalse((root / "manifest-mutation-aggregate.json").exists())

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest_path = root / "manifest.json"
            manifest_path.write_text("{}", encoding="utf-8")
            pinned = external.runner_dependency_closure_entries()
            changed = copy.deepcopy(pinned)
            changed[0]["sha256"] = "f" * 64
            with (
                mock.patch.object(
                    external,
                    "load_complete_reference_manifest",
                    return_value=small_manifest(),
                ),
                mock.patch.object(
                    external, "selected_case_contexts", return_value=[context]
                ),
                mock.patch.object(external, "scipy_evidence", side_effect=success),
                mock.patch.object(
                    external,
                    "runner_dependency_closure_entries",
                    side_effect=[pinned, changed],
                ) as closure_entries,
            ):
                with self.assertRaisesRegex(
                    external.ExternalEvidenceError,
                    "runner dependency closure changed during external evidence execution",
                ):
                    external.run_evidence_set(
                        mode="calibration",
                        manifest_path=manifest_path,
                        output_dir=root / "closure-mutation-artifacts",
                        aggregate_path=root / "closure-mutation-aggregate.json",
                        revision=REVISION,
                        partition=(0, 1),
                        comparators="scipy-radau",
                        resume=False,
                    )
            self.assertEqual(closure_entries.call_count, 2)
            self.assertFalse((root / "closure-mutation-aggregate.json").exists())


if __name__ == "__main__":
    unittest.main()
