#!/usr/bin/env python3
"""Independent original REI 15-state source/solver evidence audit.

Uses Python stdlib and frozen source/ref bytes, not the new Rust derivative
routine. Checks all real-source rows, invariants, error budgets, original
client-code byte identity and charged work. Does not certify 26-event owner.
"""
from __future__ import annotations
import hashlib
import json
import math
import subprocess
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
SOURCE = ROOT / "exploratory/rei_igm_timeforced_photoionization_20261101/five_clock_pr342_original_source.json"
REFERENCE = ROOT / "exploratory/rei_igm_timeforced_photoionization_20261101/reference_trajectory_15state.json"
BASELINE = ROOT / "docs/vigilode_core_applied_bridge_20261101/ORIGINAL_REI_15STATE_NATIVE_TWOARM.json"
REPLAY = HERE / "BASELINE_DEFAULT_REPLAY.json"
PHYSICS = HERE / "REAL_SOURCE_NONAUTON_METHODS.json"
TESTS = HERE / "LOCAL_TESTS.log"
OLD_HEAD = "8cad5fdbcbde883be61f50ffdb51b3212aa2eab9"
RUST = "crates/rodas5p-integrators/examples/rei_igm_five_clock_source.rs"


def sha_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def check() -> dict:
    src, ref, record = (
        json.loads(p.read_text()) for p in (SOURCE, REFERENCE, PHYSICS)
    )
    assert len(src["sites"]) == 5 and len(ref["samples_u"]) == 11
    assert [v["u"] for v in src["sites"]] == [
        t for t in ref["samples_u"] if t in {z["u"] for z in src["sites"]}
    ]
    assert src["N"] == 46080
    assert record["original_groups"] == 4
    assert record["original_neutral_moments"] == 3
    assert record["preregistered_candidate_count"] == 8
    assert record["posthoc_adversarial_control_count"] == 1
    assert len(record["runs"]) == 9 and record["source_scoped_successful_routes"] == 8
    prev = json.loads(BASELINE.read_text())
    replay = json.loads(REPLAY.read_text())
    assert replay['source_pr332_artifact_sha256'] == prev['source_pr332_artifact_sha256']
    assert replay['source_pr342_artifact_sha256'] == prev['source_pr342_artifact_sha256']
    assert len(replay['cases']) == len(prev['cases']) == 2
    old_new_max = 0.0
    for earlier, current_case in zip(prev['cases'], replay['cases']):
        assert earlier['case'] == current_case['case']
        assert earlier['t'] == current_case['t'] == ref['samples_u']
        assert len(earlier['y']) == len(current_case['y']) == 11
        for a, b in zip(earlier['y'], current_case['y']):
            assert len(a) == len(b) == 15
            old_new_max = max(old_new_max, max(abs(x-y) for x, y in zip(a, b)))
    assert old_new_max <= 5e-10
    original_bytes_same_host = prev == replay

    original = subprocess.check_output(["git", "-C", str(ROOT), "show", f"{OLD_HEAD}:{RUST}"])
    new = (ROOT / RUST).read_bytes()
    directive = b'#[path = "rei_igm_five_clock_source/nonauton_study.rs"]\nmod nonauton_study;\n'
    hook = (b'    if std::env::args().any(|a|a=="--nonauton-study") {\n'
            b'        return nonauton_study::cli();\n    }\n')
    assert new.count(directive) == 1 and new.count(hook) == 1
    assert new.replace(directive, b"").replace(hook, b"") == original
    assert "test result: ok. 6 passed; 0 failed" in TESTS.read_text()

    original_s0 = [float(v) for v in src["S0_particles"]]
    Eion = [float(v) for v in src["ionization_potentials_eV"]]
    seen = set()
    rows = []
    for run in record["runs"]:
        method, forced, rtol = run["method"], run["forced"], run["rtol"]
        assert method in {"rodas5p-exact-ft", "dopri5-rhs-only", "rodas5p-legacy-zero-ft"}
        assert (method, forced, rtol) not in seen
        seen.add((method, forced, rtol))
        assert run["samples_u"] == ref["samples_u"]
        assert run["complete_requested_observations"] == 11
        assert len(run["trajectory_15state"]) == 11
        assert run["stage_count"] == (4 if forced else 1)
        assert run["scientific_declaration_valid"] == (method != "rodas5p-legacy-zero-ft")
        target = ref["forced_states"] if forced else ref["frozen_states"]
        err = 0.0
        inv = 0.0
        for state, reference in zip(run["trajectory_15state"], target):
            assert len(state) == len(reference) == 15
            assert all(math.isfinite(x) for x in state)
            assert all(x > 0.0 for x in state[:3])
            err = max(err, max(abs(x-y) for x, y in zip(state, reference)))
            mass_h = state[0] + state[3] - 1.0
            mass_hei = state[1] + state[4] - 1.0
            mass_heii = state[2] - 1.0 - (original_s0[1]/original_s0[2])*state[4] + state[5]
            photons = sum(state[10:14]) - sum(state[6:10]) - sum(
                state[3+i]*original_s0[i]/original_s0[0] for i in range(3)
            )
            binding = state[14] - sum(
                Eion[i]*state[3+i]*original_s0[i]/original_s0[0] for i in range(3)
            )
            inv = max(inv, *(abs(x) for x in (mass_h, mass_hei, mass_heii, photons, binding)))
        assert abs(err - run["max_native_minus_independent_Radau_abs"]) < 2e-18
        assert abs(inv - run["max_photon_nuclei_binding_invariant"]) < 2e-18
        assert err <= 5e-10 and inv <= 2e-11
        assert run["source_original_scoped_pass"] is True
        work = Counter()
        for seg in run["segments"]:
            for k, v in seg["work"].items():
                if type(v) is int:
                    work[k] += v
        assert work["rhs_calls"] > 0 and work["accepted_steps"] > 0
        rows.append(dict(
            method=method, forced=forced, rtol=rtol, max_absolute_state_error=err,
            max_invariant_error=inv, declaration_valid=run["scientific_declaration_valid"],
            accepted_steps=work["accepted_steps"], rejected_steps=work["rejected_steps"],
            rhs_calls=work["rhs_calls"], jvp_calls=work["jvp_calls"],
            ft_calls=work["ft_calls"],
            wall_seconds_diagnostic_only=run["segment_diagnostic_wall_seconds"],
            final_binding_eV_per_initial_hi=run["final_ionization_binding_eV_per_HI"]
        ))
    current = {x["method"]: x for x in rows if x["forced"] and x["rtol"] == 2e-11}
    # Require a real source-conditioned work separation, not exact platform-
    # dependent iteration/BLAS counts. Exact 215/17 values are local evidence.
    assert current["rodas5p-legacy-zero-ft"]["accepted_steps"] > current["rodas5p-exact-ft"]["accepted_steps"]
    assert current["rodas5p-legacy-zero-ft"]["rejected_steps"] >= current["rodas5p-exact-ft"]["rejected_steps"]
    assert current["dopri5-rhs-only"]["accepted_steps"] > 0
    assert not current["rodas5p-legacy-zero-ft"]["declaration_valid"]
    return {
        "schema": "REI_FIVE_CLOCK_NONAUTON_INDEPENDENT_AUDIT_V2",
        "source_scoped_original_lanes": 1,
        "original_source_cells": 46080,
        "original_photon_groups": 4,
        "original_neutral_moments": 3,
        "output_time_samples_each": 11,
        "original_source_fixture_sha256": sha_bytes(SOURCE.read_bytes()),
        "original_reference_sha256": sha_bytes(REFERENCE.read_bytes()),
        "real_source_output_sha256": sha_bytes(PHYSICS.read_bytes()),
        "original_default_client_reference_replay": "BYTE_EXACT_SAME_HOST" if original_bytes_same_host else "NUMERIC_SOURCE_PARITY_PASS",
        "original_default_replay_max_abs_gap": old_new_max,
        "original_parent_rhs_jvp_after_removing_optin": "BYTE_EXACT_PASS",
        "native_tests": "6/6_PASS",
        "preregistered_source_routes": "8/8_SOURCE_SCOPED_PASS",
        "posthoc_counterfactual": "1/1_VERIFIED_BUT_SCIENTIFICALLY_INVALID_AUTONOMOUS_DECLARATION",
        "same_knot_legacy_to_corrected_accepted_step_ratio": current["rodas5p-legacy-zero-ft"]["accepted_steps"] / current["rodas5p-exact-ft"]["accepted_steps"],
        "strict_same_source_matched_achieved_state_error": True,
        "scientific_status": "SOURCE_SCOPED_METHOD_REPRODUCTION_NOT_26EVENT",
        "original_26event_full_map": "NOT_RUN",
        "vigilode_original_progress": "7/23_REPORTED",
        "rei_central_progress": "28/88_REPORTED",
        "rei_working_progress": "33/88_REPORTED",
        "rei_fast_progress": "4/7_REPORTED",
        "canonical_delta": 0,
        "production": "HOLD",
        "source_rows": rows,
        "strict_main_method_comparison": current,
        "not_continuum_error_theorem": True,
        "not_general_g0_speedup": True,
    }


if __name__ == "__main__":
    data = check()
    dest = HERE / "INDEPENDENT_AUDIT.json"
    dest.write_text(json.dumps(data, indent=2, sort_keys=True, allow_nan=False) + "\n")
    print("INDEPENDENT_ORIGINAL_SOURCE_AUDIT_PASS", len(data["source_rows"]),
          data["preregistered_source_routes"],
          data["posthoc_counterfactual"],
          "LEGACY_CORRECTED_STEP_RATIO", data["same_knot_legacy_to_corrected_accepted_step_ratio"])
    print("AUDIT_SHA256", sha_bytes(dest.read_bytes()))
