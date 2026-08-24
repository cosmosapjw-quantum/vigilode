from __future__ import annotations

import copy
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parents[2]
CONTRACT = SCRIPT_DIR.parent / "contracts" / "V37_TIMING_REPLICATION_CONTINUATION_TRANSACTION_CONTRACT.json"
V36_ECONOMICS = REPO_ROOT / "research" / "generic_frozen_full_e_shadow_v36" / "results" / "economics"

sys.path.insert(0, str(SCRIPT_DIR))

from timing_authority_validator import (  # noqa: E402
    ValidationError,
    atomic_write_json,
    capture_attestation,
    cpu_idle_steal_fractions,
    generate_v36_retrospective_diagnostic,
    load_contract,
    parse_proc_stat_cpu,
    sha256_path,
    summarize_attempts,
    validate_campaign,
)

PROFILE_NAMES = {
    96: "calibration96.json",
    192: "calibration192.json",
    256: "calibration256.json",
    320: "holdout320.json",
    384: "holdout384.json",
}
PROFILE_LABELS = {
    96: "stage-growth-calibration-96",
    192: "stage-growth-calibration-192",
    256: "stage-growth-calibration-256",
    320: "enforced-budget-holdout-320",
    384: "stage-growth-holdout-384",
}
THREAD_FIELDS = [
    "RAYON_NUM_THREADS",
    "OMP_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "MKL_NUM_THREADS",
    "BLIS_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
    "NUMEXPR_NUM_THREADS",
]


def make_attestation(contract_sha: str = "a" * 64, ratio_tag: str = "same") -> dict:
    return {
        "schema": "vigilode-v37-timing-host-attestation-v1",
        "git": {"head": "1" * 40, "tree": "2" * 40, "clean": True},
        "rust": {"rustc_vv": "rustc 1.94.1\n", "cargo_version": "cargo 1.94.1"},
        "contract_sha256": contract_sha,
        "binary_sha256": "b" * 64,
        "measurement_profile": "measurement",
        "host": {
            "kernel": "Linux test 6.8 x86_64",
            "cpu_model": "Synthetic CPU",
            "logical_cpu_count": 24,
            "physical_core_count": 12,
            "microcode": "0x123",
            "numa_node_count": 1,
            "frequency_governor": "performance",
            "boost_or_turbo_state": "enabled",
        },
        "cpu_affinity": list(range(24)),
        "thread_environment": {field: None for field in THREAD_FIELDS},
        "preflight": {
            "probe_seconds": 10,
            "cpu_before": [100, 0, 10, 800, 0, 0, 0, 0],
            "cpu_after": [102, 0, 11, 890, 0, 0, 0, 0],
            "cpu_idle_fraction": 90 / 93,
            "cpu_steal_fraction": 0.0,
            "swap_before": {"pswpin": 0, "pswpout": 0},
            "swap_after": {"pswpin": 0, "pswpout": 0},
            "swap_delta": {"pswpin": 0, "pswpout": 0},
            "thermal_before": {},
            "thermal_after": {},
            "thermal_delta": {},
            "ratio_tag": ratio_tag,
        },
    }


def make_pair(index: int, *, ratio: float = 1.0, rjf_wall: float = 1.0) -> dict:
    interval = 5.0
    shadow_wall = rjf_wall * ratio
    order = "rjf-first" if index % 2 == 0 else "shadow-first"
    return {
        "pair_index": index,
        "order": order,
        "rjf_only": {
            "mode": "rjf-only",
            "repetitions": 1,
            "wall_seconds": rjf_wall,
            "proposed_interval": interval,
            "gamma_seconds_per_interval": rjf_wall / interval,
            "family_count": 6,
            "all_suite_identities_passed": True,
        },
        "frozen_full_e_shadow": {
            "mode": "frozen-full-e-shadow",
            "repetitions": 1,
            "wall_seconds": shadow_wall,
            "proposed_interval": interval,
            "gamma_seconds_per_interval": shadow_wall / interval,
            "family_count": 6,
            "all_suite_identities_passed": True,
        },
        "wall_ratio_shadow_over_rjf": ratio,
        "gamma_ratio_shadow_over_rjf": ratio,
    }


def make_profile(dim: int, *, ratios: list[float] | None = None) -> dict:
    ratios = ratios or [1.0] * 7
    rows = [make_pair(i, ratio=ratios[i], rjf_wall=1.0 + 0.01 * i) for i in range(7)]
    return {
        "schema": "g4-s5b0-frozen-full-e-shadow-economics-v1",
        "status": "complete",
        "profile": PROFILE_LABELS[dim],
        "switching_active": False,
        "committed_method": "protected-sequential-matrix-free-rodas5p",
        "shadow_method": "pexprb54s4-fused-resume-retained-level2",
        "frozen_zeta34_tau": 13.39706618860016,
        "all_six_families_present": True,
        "paired_wall": {
            "required_build_profile": "measurement",
            "measurement_build_verified": True,
            "compiled_cargo_profile": "release",
            "compiled_profile_directory": "measurement",
            "suite_scope": "all-six-families",
            "calibration_arm": "rjf-only",
            "gamma_denominator": "sum-absolute-proposed-attempt-h",
            "warmup_pairs": 1,
            "measured_pairs": 7,
            "frozen_repetitions": 1,
            "warmup_rows": [make_pair(0)],
            "measured_rows": rows,
            "all_suite_identities_passed": True,
        },
    }


def write_campaign(root: Path, *, ratios: dict[int, list[float]] | None = None) -> None:
    contract_sha = sha256_path(CONTRACT)
    (root / "profiles").mkdir(parents=True)
    (root / "ATTESTATION.json").write_text(
        json.dumps(make_attestation(contract_sha), indent=2) + "\n", encoding="utf-8"
    )
    for dim, name in PROFILE_NAMES.items():
        profile = make_profile(dim, ratios=(ratios or {}).get(dim))
        (root / "profiles" / name).write_text(json.dumps(profile, indent=2) + "\n", encoding="utf-8")


class ContractAndCpuStatTests(unittest.TestCase):
    def test_contract_thresholds_are_exact(self):
        timing = load_contract(CONTRACT)["timing_replication"]
        self.assertEqual(timing["profiles"], [96, 192, 256, 320, 384])
        self.assertEqual(timing["required_passing_campaigns"], 3)
        self.assertEqual(timing["maximum_campaign_attempts"], 5)
        self.assertFalse(timing["quality_rules_reference_ratio_direction"])

    def test_proc_stat_uses_sealed_eight_fields(self):
        before = parse_proc_stat_cpu("cpu 10 1 4 80 2 1 1 0 0 0\n")
        after = parse_proc_stat_cpu("cpu 12 1 5 170 2 1 1 0 0 0\n")
        idle, steal = cpu_idle_steal_fractions(before, after)
        self.assertAlmostEqual(idle, 90 / 93)
        self.assertEqual(steal, 0.0)

    def test_negative_proc_stat_delta_fails_closed(self):
        with self.assertRaises(ValidationError):
            cpu_idle_steal_fractions((10, 0, 0, 10, 2, 0, 0, 0), (11, 0, 0, 20, 1, 0, 0, 0))


class AttestationTests(unittest.TestCase):
    @mock.patch("timing_authority_validator.time.sleep", return_value=None)
    @mock.patch("timing_authority_validator._capture_thermal_counters", side_effect=[{}, {}])
    @mock.patch("timing_authority_validator._capture_swap_counters", side_effect=[{"pswpin": 1, "pswpout": 2}, {"pswpin": 1, "pswpout": 2}])
    @mock.patch("timing_authority_validator._read_text")
    @mock.patch("timing_authority_validator._run")
    def test_capture_attestation_has_stable_schema(self, run, read_text, swap, thermal, sleep):
        run.side_effect = [
            "1" * 40,
            "2" * 40,
            "",
            "rustc 1.94.1\ncommit-hash: abc",
            "cargo 1.94.1",
            "Linux test 6.8 x86_64",
        ]
        read_text.side_effect = [
            "model name\t: Synthetic CPU\nphysical id\t: 0\ncore id\t: 0\nmicrocode\t: 0x123\n",
            "cpu 100 0 10 800 0 0 0 0 0 0\n",
            "cpu 102 0 11 890 0 0 0 0 0 0\n",
        ]
        with tempfile.TemporaryDirectory() as td:
            td = Path(td)
            binary = td / "target" / "measurement" / "measurement-bin"
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"binary")
            with mock.patch("timing_authority_validator.os.cpu_count", return_value=2), mock.patch(
                "timing_authority_validator.os.sched_getaffinity", return_value={0, 1}
            ), mock.patch("timing_authority_validator._numa_node_count", return_value=1):
                result = capture_attestation(REPO_ROOT, binary, CONTRACT, 10)
        self.assertEqual(result["schema"], "vigilode-v37-timing-host-attestation-v1")
        self.assertTrue(result["git"]["clean"])
        self.assertEqual(result["measurement_profile"], "measurement")
        self.assertEqual(set(result["thread_environment"]), set(THREAD_FIELDS))
        self.assertAlmostEqual(result["preflight"]["cpu_idle_fraction"], 90 / 93)

    def test_capture_rejects_binary_without_measurement_path(self):
        with tempfile.TemporaryDirectory() as td:
            binary = Path(td) / "release-bin"
            binary.write_bytes(b"binary")
            with self.assertRaises(ValidationError):
                capture_attestation(REPO_ROOT, binary, CONTRACT, 10)


class CampaignLayoutTests(unittest.TestCase):
    def test_valid_campaign_retains_exact_rows(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertEqual(result["verdict"], "PASS_HOST_QUALIFIED_DESCRIPTIVE_TIMING")
        self.assertEqual(result["retained_measured_pair_count"], 35)
        self.assertEqual(result["retained_warmup_pair_count"], 5)
        self.assertEqual(len(result["profiles"]), 5)

    def test_missing_pair_rejects_entire_campaign_and_retains_file_inventory(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "holdout320.json"
            data = json.loads(path.read_text())
            data["paired_wall"]["measured_rows"].pop()
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
            self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
            self.assertEqual(result["retained_profile_count"], 5)
            self.assertEqual(result["retained_measured_pair_count"], 34)
            self.assertIn("profile-pair-cardinality", result["quality_failure_names"])

    def test_missing_middle_pair_is_retained_as_named_whole_campaign_failure(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "holdout320.json"
            data = json.loads(path.read_text())
            del data["paired_wall"]["measured_rows"][3]
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
        self.assertEqual(result["retained_measured_pair_count"], 34)
        self.assertIn("profile-pair-cardinality", result["quality_failure_names"])
        self.assertIn("profile-pair-index-set", result["quality_failure_names"])
        json.dumps(result, allow_nan=False)

    def test_one_measured_row_uses_serializable_unavailable_order_metric(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "holdout320.json"
            data = json.loads(path.read_text())
            data["paired_wall"]["measured_rows"] = data["paired_wall"]["measured_rows"][:1]
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        profile = next(item for item in result["profiles"] if item["dimension"] == 320)
        self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
        self.assertEqual(result["retained_measured_pair_count"], 29)
        self.assertIsNone(profile["metrics"]["order_median_absolute_gap"])
        self.assertIn("profile-metric-unavailable", profile["quality_failure_names"])
        json.dumps(result, allow_nan=False)

    def test_zero_measured_rows_use_null_metrics_and_serialize(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "holdout320.json"
            data = json.loads(path.read_text())
            data["paired_wall"]["measured_rows"] = []
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        profile = next(item for item in result["profiles"] if item["dimension"] == 320)
        self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
        self.assertEqual(result["retained_measured_pair_count"], 28)
        self.assertTrue(all(value is None for value in profile["metrics"].values()))
        self.assertIn("profile-metric-unavailable", profile["quality_failure_names"])
        json.dumps(result, allow_nan=False)

    def test_duplicate_pair_index_is_retained_as_named_failure(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "holdout320.json"
            data = json.loads(path.read_text())
            data["paired_wall"]["measured_rows"][-1] = copy.deepcopy(
                data["paired_wall"]["measured_rows"][-2]
            )
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
        self.assertEqual(result["retained_measured_pair_count"], 35)
        self.assertIn("profile-pair-index-set", result["quality_failure_names"])
        json.dumps(result, allow_nan=False)

    def test_unexpected_pair_index_is_retained_as_named_failure(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "holdout320.json"
            data = json.loads(path.read_text())
            row = data["paired_wall"]["measured_rows"][-1]
            row["pair_index"] = 7
            row["order"] = "shadow-first"
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
        self.assertEqual(result["retained_measured_pair_count"], 35)
        self.assertIn("profile-pair-index-set", result["quality_failure_names"])
        json.dumps(result, allow_nan=False)

    def test_proposed_interval_mismatch_is_retained_quality_failure(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "calibration96.json"
            data = json.loads(path.read_text())
            row = data["paired_wall"]["measured_rows"][0]
            row["frozen_full_e_shadow"]["proposed_interval"] = 6.0
            row["frozen_full_e_shadow"]["gamma_seconds_per_interval"] = row["frozen_full_e_shadow"]["wall_seconds"] / 6.0
            row["gamma_ratio_shadow_over_rjf"] = row["frozen_full_e_shadow"]["gamma_seconds_per_interval"] / row["rjf_only"]["gamma_seconds_per_interval"]
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertEqual(result["retained_measured_pair_count"], 35)
        self.assertIn("proposed-interval", result["quality_failure_names"])

    def test_extra_profile_rejects(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            (root / "profiles" / "extra.json").write_text("{}")
            with self.assertRaises(ValidationError):
                validate_campaign(load_contract(CONTRACT), root)

    def test_arm_repetition_must_match_frozen_profile_repetitions(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "calibration96.json"
            data = json.loads(path.read_text())
            data["paired_wall"]["frozen_repetitions"] = 2
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaises(ValidationError):
                validate_campaign(load_contract(CONTRACT), root)

    def test_measurement_protocol_metadata_is_exact(self):
        mutations = {
            "compiled Cargo profile": lambda paired: paired.__setitem__(
                "compiled_cargo_profile", "debug"
            ),
            "calibration arm": lambda paired: paired.__setitem__(
                "calibration_arm", "frozen-full-e-shadow"
            ),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as td:
                root = Path(td)
                write_campaign(root)
                path = root / "profiles" / "calibration96.json"
                data = json.loads(path.read_text())
                mutate(data["paired_wall"])
                path.write_text(json.dumps(data), encoding="utf-8")
                with self.assertRaises(ValidationError):
                    validate_campaign(load_contract(CONTRACT), root)

    def test_boolean_arm_repetition_count_rejects(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "calibration96.json"
            data = json.loads(path.read_text())
            data["paired_wall"]["measured_rows"][0]["rjf_only"]["repetitions"] = True
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaises(ValidationError):
                validate_campaign(load_contract(CONTRACT), root)

    def test_declared_pair_counts_require_exact_integer_types(self):
        mutations = {
            "warmup bool": lambda paired: paired.__setitem__("warmup_pairs", True),
            "measured float": lambda paired: paired.__setitem__("measured_pairs", 7.0),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as td:
                root = Path(td)
                write_campaign(root)
                path = root / "profiles" / "calibration96.json"
                data = json.loads(path.read_text())
                mutate(data["paired_wall"])
                path.write_text(json.dumps(data), encoding="utf-8")
                with self.assertRaises(ValidationError):
                    validate_campaign(load_contract(CONTRACT), root)

    def test_extra_root_file_rejects_campaign_layout(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            (root / "unrelated-note.txt").write_text("not authority evidence\n", encoding="utf-8")
            with self.assertRaises(ValidationError):
                validate_campaign(load_contract(CONTRACT), root)


    def test_frozen_zeta34_tau_mismatch_rejects(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "calibration96.json"
            data = json.loads(path.read_text())
            data["frozen_zeta34_tau"] = 13.0
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaises(ValidationError):
                validate_campaign(load_contract(CONTRACT), root)


class HostQualityTests(unittest.TestCase):
    def test_ratio_direction_does_not_change_quality_verdict(self):
        with tempfile.TemporaryDirectory() as td1, tempfile.TemporaryDirectory() as td2:
            a, b = Path(td1), Path(td2)
            write_campaign(a, ratios={dim: [0.6] * 7 for dim in PROFILE_NAMES})
            write_campaign(b, ratios={dim: [1.4] * 7 for dim in PROFILE_NAMES})
            ra = validate_campaign(load_contract(CONTRACT), a)
            rb = validate_campaign(load_contract(CONTRACT), b)
        self.assertEqual(ra["verdict"], rb["verdict"])
        self.assertEqual(ra["quality_failures"], rb["quality_failures"])

    def test_one_bad_n384_pair_rejects_whole_campaign_but_retains_all_pairs(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "profiles" / "holdout384.json"
            data = json.loads(path.read_text())
            row = data["paired_wall"]["measured_rows"][4]
            row["rjf_only"]["wall_seconds"] *= 2.0
            row["rjf_only"]["gamma_seconds_per_interval"] *= 2.0
            row["wall_ratio_shadow_over_rjf"] /= 2.0
            row["gamma_ratio_shadow_over_rjf"] /= 2.0
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
        self.assertEqual(result["retained_measured_pair_count"], 35)
        self.assertIn("rjf-arm-span", result["quality_failure_names"])

    def test_low_idle_is_quality_failure_not_parse_error(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "ATTESTATION.json"
            data = json.loads(path.read_text())
            data["preflight"]["cpu_before"] = [100, 0, 10, 800, 0, 0, 0, 0]
            data["preflight"]["cpu_after"] = [140, 0, 20, 850, 0, 0, 0, 0]
            data["preflight"]["cpu_idle_fraction"] = 0.5
            data["preflight"]["cpu_steal_fraction"] = 0.0
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
        self.assertIn("cpu-idle", result["quality_failure_names"])

    def _mutate_attestation(self, root: Path, mutate) -> dict:
        path = root / "ATTESTATION.json"
        data = json.loads(path.read_text())
        mutate(data)
        path.write_text(json.dumps(data), encoding="utf-8")
        return validate_campaign(load_contract(CONTRACT), root)

    def test_dirty_tree_is_quality_failure(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root)
            result = self._mutate_attestation(root, lambda data: data["git"].__setitem__("clean", False))
        self.assertIn("clean-tree", result["quality_failure_names"])

    def test_contract_hash_mismatch_is_quality_failure(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root)
            result = self._mutate_attestation(root, lambda data: data.__setitem__("contract_sha256", "c" * 64))
        self.assertIn("contract-hash", result["quality_failure_names"])

    def test_cpu_steal_threshold_is_enforced(self):
        def mutate(data):
            data["preflight"]["cpu_before"] = [100, 0, 10, 800, 0, 0, 0, 0]
            data["preflight"]["cpu_after"] = [101, 0, 10, 898, 0, 0, 0, 1]
            data["preflight"]["cpu_idle_fraction"] = 0.98
            data["preflight"]["cpu_steal_fraction"] = 0.01
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root)
            result = self._mutate_attestation(root, mutate)
        self.assertIn("cpu-steal", result["quality_failure_names"])

    def test_swap_in_and_out_thresholds_are_enforced(self):
        def mutate(data):
            data["preflight"]["swap_after"] = {"pswpin": 1, "pswpout": 2}
            data["preflight"]["swap_delta"] = {"pswpin": 1, "pswpout": 2}
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root)
            result = self._mutate_attestation(root, mutate)
        self.assertIn("swap-in", result["quality_failure_names"])
        self.assertIn("swap-out", result["quality_failure_names"])

    def test_exposed_thermal_delta_is_enforced(self):
        def mutate(data):
            data["preflight"]["thermal_before"] = {"cpu0": 4}
            data["preflight"]["thermal_after"] = {"cpu0": 5}
            data["preflight"]["thermal_delta"] = {"cpu0": 1}
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root)
            result = self._mutate_attestation(root, mutate)
        self.assertIn("thermal-throttle", result["quality_failure_names"])

    def test_absent_thermal_counters_are_not_a_failure(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root)
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertNotIn("thermal-throttle", result["quality_failure_names"])

    def test_shadow_arm_span_is_enforced(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root)
            path = root / "profiles" / "holdout384.json"
            data = json.loads(path.read_text())
            row = data["paired_wall"]["measured_rows"][4]
            row["frozen_full_e_shadow"]["wall_seconds"] *= 2.0
            row["frozen_full_e_shadow"]["gamma_seconds_per_interval"] *= 2.0
            row["wall_ratio_shadow_over_rjf"] *= 2.0
            row["gamma_ratio_shadow_over_rjf"] *= 2.0
            path.write_text(json.dumps(data), encoding="utf-8")
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertIn("shadow-arm-span", result["quality_failure_names"])

    def test_order_median_gap_is_enforced_without_using_favorable_direction(self):
        ratios = [1.0, 1.2, 1.0, 1.2, 1.0, 1.2, 1.0]
        with tempfile.TemporaryDirectory() as td:
            root = Path(td); write_campaign(root, ratios={384: ratios})
            result = validate_campaign(load_contract(CONTRACT), root)
        self.assertIn("order-median-gap", result["quality_failure_names"])
        self.assertFalse(result["ratio_direction_used_for_quality"])


class AttemptSummaryTests(unittest.TestCase):
    def _decision(
        self,
        base: Path,
        index: int,
        *,
        passed: bool = True,
        identity_mutator=None,
    ) -> dict:
        root = base / f"attempt-{index:02d}"
        write_campaign(root)

        profile_path = root / "profiles" / "calibration96.json"
        profile = json.loads(profile_path.read_text())
        row = profile["paired_wall"]["measured_rows"][0]
        scale = 1.0 + 0.001 * index
        for arm_name in ("rjf_only", "frozen_full_e_shadow"):
            row[arm_name]["wall_seconds"] *= scale
            row[arm_name]["gamma_seconds_per_interval"] *= scale
        profile_path.write_text(json.dumps(profile), encoding="utf-8")

        attestation_path = root / "ATTESTATION.json"
        attestation = json.loads(attestation_path.read_text())
        if not passed:
            attestation["preflight"]["cpu_before"] = [100, 0, 10, 800, 0, 0, 0, 0]
            attestation["preflight"]["cpu_after"] = [140, 0, 20, 850, 0, 0, 0, 0]
            attestation["preflight"]["cpu_idle_fraction"] = 0.5
            attestation["preflight"]["cpu_steal_fraction"] = 0.0
        if identity_mutator is not None:
            identity_mutator(attestation)
        attestation_path.write_text(json.dumps(attestation), encoding="utf-8")
        return validate_campaign(load_contract(CONTRACT), root)

    def test_three_passes_within_four_attempts_promotes_descriptive_timing(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            attempts = [
                self._decision(base, 1, passed=True),
                self._decision(base, 2, passed=False),
                self._decision(base, 3, passed=True),
                self._decision(base, 4, passed=True),
            ]
            result = summarize_attempts(load_contract(CONTRACT), attempts)
        self.assertEqual(result["verdict"], "PASS_HOST_QUALIFIED_DESCRIPTIVE_TIMING")
        self.assertEqual(result["passing_campaign_count"], 3)
        self.assertFalse(result["speedup_claim_authorized"])

    def test_three_distinct_validated_campaign_decisions_promote(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            decisions = [self._decision(base, index + 1) for index in range(3)]
            self.assertEqual(
                len({decision["authority_evidence_sha256"] for decision in decisions}),
                3,
            )
            result = summarize_attempts(load_contract(CONTRACT), decisions)
        self.assertEqual(result["verdict"], "PASS_HOST_QUALIFIED_DESCRIPTIVE_TIMING")
        self.assertEqual(result["passing_campaign_count"], 3)

    def test_two_passes_in_five_is_host_unsuitable(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            attempts = [
                self._decision(base, index, passed=index in {1, 3})
                for index in range(1, 6)
            ]
            result = summarize_attempts(load_contract(CONTRACT), attempts)
        self.assertEqual(result["verdict"], "HOST_UNSUITABLE_NO_TIMING_PROMOTION")

    def test_six_attempts_is_invalid(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            attempts = [self._decision(base, index) for index in range(1, 7)]
            with self.assertRaises(ValidationError):
                summarize_attempts(load_contract(CONTRACT), attempts)

    def test_duplicate_campaign_cannot_count_as_three_passes(self):
        with tempfile.TemporaryDirectory() as td:
            attempt = self._decision(Path(td), 1)
            with self.assertRaises(ValidationError):
                summarize_attempts(
                    load_contract(CONTRACT),
                    [attempt, copy.deepcopy(attempt), copy.deepcopy(attempt)],
                )

    def test_minimal_fabricated_pass_objects_are_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            valid = self._decision(Path(td), 1)
            fabricated = [
                {
                    "schema": valid["schema"],
                    "campaign_path": valid["campaign_path"],
                    "campaign_sha256": f"{index + 2000:064x}",
                    "verdict": valid["verdict"],
                    "quality_failure_names": [],
                    "comparison_identity": copy.deepcopy(valid["comparison_identity"]),
                }
                for index in range(3)
            ]
            with self.assertRaises(ValidationError):
                summarize_attempts(load_contract(CONTRACT), fabricated)

    def test_complete_shaped_fabricated_rows_are_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            decisions = [self._decision(base, index + 1) for index in range(3)]
            for decision in decisions:
                for profile in decision["profiles"]:
                    profile["warmup_rows"] = [{"pair_index": 0}]
                    profile["measured_rows"] = [
                        {"pair_index": pair_index} for pair_index in range(7)
                    ]
            with self.assertRaises(ValidationError):
                summarize_attempts(load_contract(CONTRACT), decisions)

    def test_decision_scalar_types_must_exactly_match_revalidation(self):
        mutations = {
            "authority integer": lambda decision: decision.__setitem__("timing_authority", 1),
            "forbidden flag integer": lambda decision: decision.__setitem__(
                "speedup_claim_authorized", 0
            ),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as td:
                decision = self._decision(Path(td), 1)
                mutate(decision)
                with self.assertRaises(ValidationError):
                    summarize_attempts(load_contract(CONTRACT), [decision])

    def test_non_authority_profile_notes_cannot_create_distinct_campaigns(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td) / "base"
            write_campaign(base)
            decisions = []
            for index in range(3):
                root = Path(td) / f"copy-{index + 1:02d}"
                shutil.copytree(base, root)
                path = root / "profiles" / "calibration96.json"
                data = json.loads(path.read_text())
                data["non_authority_note"] = f"copy-{index + 1}"
                path.write_text(json.dumps(data), encoding="utf-8")
                decisions.append(validate_campaign(load_contract(CONTRACT), root))
            self.assertEqual(len({item["campaign_sha256"] for item in decisions}), 3)
            self.assertEqual(
                len({item["authority_evidence_sha256"] for item in decisions}), 1
            )
            with self.assertRaises(ValidationError):
                summarize_attempts(load_contract(CONTRACT), decisions)


    def test_reordered_pair_rows_cannot_create_distinct_campaigns(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td) / "base"
            write_campaign(base)
            decisions = []
            permutations = [
                [0, 1, 2, 3, 4, 5, 6],
                [6, 5, 4, 3, 2, 1, 0],
                [3, 4, 5, 6, 0, 1, 2],
            ]
            for index, permutation in enumerate(permutations, start=1):
                root = Path(td) / f"copy-{index:02d}"
                shutil.copytree(base, root)
                path = root / "profiles" / "calibration96.json"
                data = json.loads(path.read_text())
                rows = data["paired_wall"]["measured_rows"]
                data["paired_wall"]["measured_rows"] = [
                    rows[position] for position in permutation
                ]
                path.write_text(json.dumps(data), encoding="utf-8")
                decisions.append(validate_campaign(load_contract(CONTRACT), root))
            self.assertEqual(len({item["campaign_sha256"] for item in decisions}), 3)
            self.assertEqual(
                len({item["authority_evidence_sha256"] for item in decisions}), 1
            )
            with self.assertRaises(ValidationError):
                summarize_attempts(load_contract(CONTRACT), decisions)

    def test_numeric_json_spelling_cannot_create_distinct_campaigns(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td) / "base"
            write_campaign(base)
            decisions = []
            for index in range(3):
                root = Path(td) / f"copy-{index + 1:02d}"
                shutil.copytree(base, root)
                if index > 0:
                    path = root / "profiles" / "calibration96.json"
                    data = json.loads(path.read_text())
                    kind = "measured_rows" if index == 1 else "warmup_rows"
                    row = data["paired_wall"][kind][0]
                    for arm_name in ("rjf_only", "frozen_full_e_shadow"):
                        row[arm_name]["wall_seconds"] = int(
                            row[arm_name]["wall_seconds"]
                        )
                        row[arm_name]["proposed_interval"] = int(
                            row[arm_name]["proposed_interval"]
                        )
                    row["wall_ratio_shadow_over_rjf"] = int(
                        row["wall_ratio_shadow_over_rjf"]
                    )
                    row["gamma_ratio_shadow_over_rjf"] = int(
                        row["gamma_ratio_shadow_over_rjf"]
                    )
                    path.write_text(json.dumps(data), encoding="utf-8")
                decisions.append(validate_campaign(load_contract(CONTRACT), root))
            self.assertEqual(len({item["campaign_sha256"] for item in decisions}), 3)
            self.assertEqual(
                len({item["authority_evidence_sha256"] for item in decisions}), 1
            )
            with self.assertRaises(ValidationError):
                summarize_attempts(load_contract(CONTRACT), decisions)


    def test_redundant_derived_fields_cannot_create_distinct_campaigns(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td) / "base"
            write_campaign(base)
            decisions = []
            for index in range(3):
                root = Path(td) / f"copy-{index + 1:02d}"
                shutil.copytree(base, root)
                if index == 1:
                    path = root / "profiles" / "calibration96.json"
                    data = json.loads(path.read_text())
                    row = data["paired_wall"]["measured_rows"][0]
                    row["rjf_only"]["gamma_seconds_per_interval"] = math.nextafter(
                        row["rjf_only"]["gamma_seconds_per_interval"], math.inf
                    )
                    row["frozen_full_e_shadow"]["gamma_seconds_per_interval"] = math.nextafter(
                        row["frozen_full_e_shadow"]["gamma_seconds_per_interval"], math.inf
                    )
                    row["wall_ratio_shadow_over_rjf"] = math.nextafter(
                        row["wall_ratio_shadow_over_rjf"], math.inf
                    )
                    row["gamma_ratio_shadow_over_rjf"] = math.nextafter(
                        row["gamma_ratio_shadow_over_rjf"], math.inf
                    )
                    path.write_text(json.dumps(data), encoding="utf-8")
                elif index == 2:
                    path = root / "ATTESTATION.json"
                    data = json.loads(path.read_text())
                    data["preflight"]["cpu_idle_fraction"] = math.nextafter(
                        data["preflight"]["cpu_idle_fraction"], math.inf
                    )
                    data["preflight"]["cpu_steal_fraction"] = math.nextafter(
                        data["preflight"]["cpu_steal_fraction"], math.inf
                    )
                    path.write_text(json.dumps(data), encoding="utf-8")
                decisions.append(validate_campaign(load_contract(CONTRACT), root))
            self.assertEqual(len({item["campaign_sha256"] for item in decisions}), 3)
            self.assertEqual(
                len({item["authority_evidence_sha256"] for item in decisions}), 1
            )
            with self.assertRaises(ValidationError):
                summarize_attempts(load_contract(CONTRACT), decisions)

    def test_preflight_numeric_spelling_fails_closed(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "ATTESTATION.json"
            data = json.loads(path.read_text())
            data["preflight"]["probe_seconds"] = 10.0
            data["preflight"]["swap_before"]["pswpin"] = 0.0
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaises(ValidationError):
                validate_campaign(load_contract(CONTRACT), root)

    def test_host_numeric_spelling_fails_closed(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            write_campaign(root)
            path = root / "ATTESTATION.json"
            data = json.loads(path.read_text())
            data["host"]["physical_core_count"] = 12.0
            data["host"]["numa_node_count"] = 1.0
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaises(ValidationError):
                validate_campaign(load_contract(CONTRACT), root)

    def test_identity_mismatch_cannot_count_as_passing(self):
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            attempts = [
                self._decision(base, 1),
                self._decision(base, 2),
                self._decision(
                    base,
                    3,
                    identity_mutator=lambda att: att["git"].__setitem__(
                        "head", "3" * 40
                    ),
                ),
            ]
            result = summarize_attempts(load_contract(CONTRACT), attempts)
        self.assertEqual(result["passing_campaign_count"], 2)
        self.assertIn("git-identity", result["attempts"][2]["effective_quality_failures"])

    def test_each_cross_campaign_identity_dimension_is_named(self):
        mutations = {
            "git-identity": lambda att: att["git"].__setitem__("tree", "4" * 40),
            "rust-toolchain": lambda att: att["rust"].__setitem__(
                "cargo_version", "cargo other"
            ),
            "contract-hash": lambda att: att.__setitem__(
                "contract_sha256", "c" * 64
            ),
            "measurement-binary": lambda att: att.__setitem__(
                "binary_sha256", "d" * 64
            ),
            "host-fingerprint": lambda att: att["host"].__setitem__(
                "kernel", "other kernel"
            ),
            "cpu-affinity": lambda att: att.__setitem__("cpu_affinity", [0]),
            "thread-environment": lambda att: att["thread_environment"].__setitem__(
                "OMP_NUM_THREADS", "2"
            ),
        }
        for expected, mutate in mutations.items():
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as td:
                base = Path(td)
                baseline = self._decision(base, 1)
                changed = self._decision(base, 2, identity_mutator=mutate)
                result = summarize_attempts(
                    load_contract(CONTRACT), [baseline, changed]
                )
                self.assertIn(
                    expected, result["attempts"][1]["effective_quality_failures"]
                )


class AtomicOutputTests(unittest.TestCase):
    def test_atomic_write_replaces_complete_json(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "out.json"
            atomic_write_json(path, {"b": 2, "a": 1})
            self.assertEqual(json.loads(path.read_text()), {"a": 1, "b": 2})
            self.assertFalse(path.with_suffix(path.suffix + ".tmp").exists())

    def test_cli_malformed_campaign_returns_one_without_output(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            output = root / "decision.json"
            proc = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT_DIR / "timing_authority_validator.py"),
                    "validate-campaign",
                    "--contract",
                    str(CONTRACT),
                    "--campaign-root",
                    str(root),
                    "--output",
                    str(output),
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(proc.returncode, 1, proc.stderr)
            self.assertFalse(output.exists())

    def test_cli_valid_campaign_returns_zero(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            campaign = root / "attempt-01"
            write_campaign(campaign)
            output = root / "decision.json"
            proc = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT_DIR / "timing_authority_validator.py"),
                    "validate-campaign",
                    "--contract",
                    str(CONTRACT),
                    "--campaign-root",
                    str(campaign),
                    "--output",
                    str(output),
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(proc.returncode, 0, proc.stderr)
            self.assertTrue(output.exists())

    def test_cli_interrupted_campaign_emits_non_authority_json(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            campaign = root / "attempt-01"
            write_campaign(campaign)
            path = campaign / "profiles" / "holdout320.json"
            data = json.loads(path.read_text())
            del data["paired_wall"]["measured_rows"][3]
            path.write_text(json.dumps(data), encoding="utf-8")
            output = root / "decision.json"
            proc = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT_DIR / "timing_authority_validator.py"),
                    "validate-campaign",
                    "--contract",
                    str(CONTRACT),
                    "--campaign-root",
                    str(campaign),
                    "--output",
                    str(output),
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(proc.returncode, 0, proc.stderr)
            result = json.loads(output.read_text())
            self.assertEqual(result["verdict"], "NON_AUTHORITY_HOST_QUALITY_FAIL")
            self.assertEqual(result["retained_measured_pair_count"], 34)
            self.assertIn("profile-pair-index-set", result["quality_failure_names"])


    def test_cli_summarize_revalidates_complete_campaign_evidence(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            decision_paths = []
            for index in range(1, 4):
                campaign = root / f"attempt-{index:02d}"
                write_campaign(campaign)
                profile_path = campaign / "profiles" / "calibration96.json"
                profile = json.loads(profile_path.read_text())
                row = profile["paired_wall"]["measured_rows"][0]
                scale = 1.0 + 0.001 * index
                for arm_name in ("rjf_only", "frozen_full_e_shadow"):
                    row[arm_name]["wall_seconds"] *= scale
                    row[arm_name]["gamma_seconds_per_interval"] *= scale
                profile_path.write_text(json.dumps(profile), encoding="utf-8")
                decision = validate_campaign(load_contract(CONTRACT), campaign)
                decision_path = root / f"decision-{index:02d}.json"
                decision_path.write_text(json.dumps(decision), encoding="utf-8")
                decision_paths.append(decision_path)
            output = root / "summary.json"
            command = [
                sys.executable,
                str(SCRIPT_DIR / "timing_authority_validator.py"),
                "summarize",
                "--contract",
                str(CONTRACT),
            ]
            for decision_path in decision_paths:
                command.extend(["--attempt-result", str(decision_path)])
            command.extend(["--output", str(output)])
            proc = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertEqual(proc.returncode, 0, proc.stderr)
            summary = json.loads(output.read_text())
            self.assertEqual(summary["verdict"], "PASS_HOST_QUALIFIED_DESCRIPTIVE_TIMING")
            self.assertEqual(summary["passing_campaign_count"], 3)
            self.assertTrue(
                all("authority_evidence_sha256" in item for item in summary["attempts"])
            )

    def test_cli_summarize_rejects_tampered_complete_decision_without_output(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            campaign = root / "attempt-01"
            write_campaign(campaign)
            decision = validate_campaign(load_contract(CONTRACT), campaign)
            decision["profiles"][0]["measured_rows"][0] = {"pair_index": 0}
            decision_path = root / "tampered-decision.json"
            decision_path.write_text(json.dumps(decision), encoding="utf-8")
            output = root / "summary.json"
            proc = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT_DIR / "timing_authority_validator.py"),
                    "summarize",
                    "--contract",
                    str(CONTRACT),
                    "--attempt-result",
                    str(decision_path),
                    "--output",
                    str(output),
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(proc.returncode, 1, proc.stderr)
            self.assertFalse(output.exists())


class RetrospectiveDiagnosticTests(unittest.TestCase):
    def test_v36_diagnostic_retains_all_pairs_and_does_not_rewrite_history(self):
        result = generate_v36_retrospective_diagnostic(load_contract(CONTRACT), V36_ECONOMICS)
        self.assertEqual(result["retained_measured_pair_count"], 35)
        self.assertFalse(result["historical_verdict_rewritten"])
        self.assertEqual(result["historical_verdict"], "PASS_DESCRIPTIVE_ECONOMICS")
        self.assertEqual(result["retrospective_verdict"], "WHOLE_V36_CAMPAIGN_NON_AUTHORITY_DUE_TO_N384_HOST_QUALITY_FAILURE")
        n384 = next(profile for profile in result["profiles"] if profile["dimension"] == 384)
        self.assertIn("rjf-arm-span", n384["quality_failure_names"])
        self.assertIn("shadow-arm-span", n384["quality_failure_names"])
        self.assertEqual(result["historical_host_counters"]["status"], "NOT_RECORDED")

    def test_v36_diagnostic_is_checkout_path_independent(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            first = root / "checkout-a" / "economics"
            second = root / "checkout-b" / "economics"
            shutil.copytree(V36_ECONOMICS, first)
            shutil.copytree(V36_ECONOMICS, second)
            a = generate_v36_retrospective_diagnostic(load_contract(CONTRACT), first)
            b = generate_v36_retrospective_diagnostic(load_contract(CONTRACT), second)
        self.assertEqual(a, b)
        self.assertTrue(all(Path(profile["path"]).name == profile["path"] for profile in a["profiles"]))
        json.dumps(a, allow_nan=False, sort_keys=True)


if __name__ == "__main__":
    unittest.main()
