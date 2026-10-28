#!/usr/bin/env python3
"""Gate of research node research/spd08_small_ensemble_lanes_20261007 (speed node SPD08).

Inputs:
  --base BASE_ENSEMBLE.json   `tools/spd08_ensemble_profile.py --base` on the unmodified source:
                              arms[scalar] with ir_per_trajectory_members_difference, attempts,
                              checksum.
  --profile PROFILE.json      `tools/spd08_ensemble_profile.py` (repetitions protocol) on the new
                              binary, run with `--dump-members`: arms[arm] with ir_per_trajectory,
                              ir_per_trajectory_members_difference, deterministic, attempts,
                              checksum, checksum_bits, members_dump (a file name next to
                              PROFILE.json) and members_dump_sha256.
  the contract-test outcome, one of
  --contract OUTCOME.json     written by `SPD08_CONTRACT_OUTCOME=OUTCOME.json cargo test --release
                              -p rodas5p-integrators --locked --test spd08_small_ensemble_lanes`
                              (the test writes it only after every assertion held);
  --contract-exit-code N      the exit status of the registered `cargo test` command (0 = passed),
                              when no outcome file was recorded.

Gate (PREREGISTRATION.md):
  1. members identical: the per-member dumps of both batch arms equal the scalar arm's (final
     state bits, attempts, accepted, rejected, reuses, counters, output times and states,
     internal and clipped steps), all members present;
  2. checksum: both batch arms' checksum bits (and attempts) equal the scalar arm's;
  3. failure masking: the contract test passed (and, with an outcome file, it exercised typed
     linear-solve failures and a non-finite right-hand side confined to one lane);
  4. instructions: Ir per trajectory (2-minus-1 repetitions) of batch8 <= 0.80 x the scalar
     arm's in the same binary; batch4 reported; kill if both batch arms > 0.90 x. The ratios
     are only taken from deterministic arms (the 1-repetition run repeated);
  5. legacy reproduction: the scalar arm reproduces BASE_ENSEMBLE.json: attempts 20,083 and the
     checksum exactly, Ir per trajectory (64-minus-32 members protocol) within +-2 %.
Counted instructions only; no wall-time claim. The output is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

SCHEMA = "vigilode-spd08-check-v1"
LEGACY_ATTEMPTS = 20083
BATCH8_MAX = 0.80
KILL_ABOVE = 0.90
LEGACY_IR_TOLERANCE = 0.02


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_dump(profile_path: Path, entry: dict):
    """The member list of an arm's dump, or (None, reason)."""
    name = entry.get("members_dump")
    if not name:
        return None, "no members_dump (profile recorded without --dump-members)"
    path = profile_path.parent / name
    if not path.exists():
        return None, f"members dump missing: {path}"
    digest = entry.get("members_dump_sha256")
    if digest is not None and sha256(path) != digest:
        return None, f"members dump hash mismatch: {path}"
    return json.loads(path.read_text())["results"], None


def first_difference(scalar, batch):
    if len(scalar) != len(batch):
        return f"member count {len(batch)} vs {len(scalar)}"
    for k, (s, b) in enumerate(zip(scalar, batch)):
        if s != b:
            keys = sorted(key for key in set(s) | set(b) if s.get(key) != b.get(key))
            return f"member {k}: {keys}"
    return None


def contract_item(args):
    if args.contract is not None:
        outcome = json.loads(args.contract.read_text())
        checks = {
            "identical": outcome.get("identical") is True,
            "typed_linear_solve_failures": outcome.get("oversized_step_linear_solve_failures", 0) > 0
            and outcome.get("singular_member_factorization_failures", 0) > 0,
            "nonfinite_rhs_in_one_lane": outcome.get("nonfinite_failures_poisoned_lane", 0) > 0
            and outcome.get("nonfinite_failures_other_lanes") == 0,
            "both_lane_counts": sorted(outcome.get("lanes", [])) == [4, 8],
        }
        return all(checks.values()), {"source": str(args.contract), "sha256": sha256(args.contract),
                                      "checks": checks, "outcome": outcome}
    return args.contract_exit_code == 0, {"source": "exit code", "exit_code": args.contract_exit_code}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--profile", required=True, type=Path)
    contract = parser.add_mutually_exclusive_group(required=True)
    contract.add_argument("--contract", type=Path)
    contract.add_argument("--contract-exit-code", type=int)
    parser.add_argument("--scalar-arm", default="rodas5p-fast-small")
    parser.add_argument("--batch4-arm", default="rodas5p-fast-small-batch4")
    parser.add_argument("--batch8-arm", default="rodas5p-fast-small-batch8")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    base = json.loads(args.base.read_text())
    profile = json.loads(args.profile.read_text())
    if profile.get("protocol") != "repetitions":
        raise SystemExit("PROFILE.json must use the repetitions protocol")
    if base.get("protocol") != "base":
        raise SystemExit("BASE_ENSEMBLE.json must use the base protocol")
    arms = profile["arms"]
    scalar_arm, batch_arms = args.scalar_arm, [args.batch4_arm, args.batch8_arm]
    for arm in [scalar_arm, *batch_arms]:
        if arm not in arms:
            raise SystemExit(f"arm {arm} missing from {args.profile}")
    scalar = arms[scalar_arm]

    # 1. Members identical.
    scalar_members, scalar_problem = load_dump(args.profile, scalar)
    item1_rows = {}
    for arm in batch_arms:
        members, problem = load_dump(args.profile, arms[arm])
        problem = problem or scalar_problem
        if problem is None:
            problem = first_difference(scalar_members, members)
            if problem is None and len(members) != profile["members"]:
                problem = f"{len(members)} members dumped, {profile['members']} run"
        item1_rows[arm] = {"identical": problem is None, "difference": problem,
                           "members": None if members is None else len(members)}
    item1 = all(r["identical"] for r in item1_rows.values())

    # 2. Checksum.
    item2_rows = {arm: {"checksum": arms[arm]["checksum"], "checksum_bits": arms[arm].get("checksum_bits"),
                        "attempts": arms[arm]["attempts"],
                        "equal": arms[arm].get("checksum_bits") is not None
                        and arms[arm]["checksum_bits"] == scalar.get("checksum_bits")
                        and arms[arm]["checksum"] == scalar["checksum"]
                        and arms[arm]["attempts"] == scalar["attempts"]}
                  for arm in batch_arms}
    item2 = all(r["equal"] for r in item2_rows.values())

    # 3. Failure masking.
    item3, item3_detail = contract_item(args)

    # 4. Instructions.
    deterministic = {arm: bool(arms[arm]["deterministic"]) for arm in [scalar_arm, *batch_arms]}
    ratios = {arm: arms[arm]["ir_per_trajectory"] / scalar["ir_per_trajectory"] for arm in batch_arms}
    cross_check = {arm: arms[arm]["ir_per_trajectory_members_difference"]
                   / scalar["ir_per_trajectory_members_difference"] for arm in batch_arms}
    valid = all(deterministic.values())
    item4 = valid and ratios[args.batch8_arm] <= BATCH8_MAX
    kill = valid and all(r > KILL_ABOVE for r in ratios.values())

    # 5. Legacy reproduction.
    base_arm = base["arms"].get(scalar_arm)
    if base_arm is None:
        raise SystemExit(f"arm {scalar_arm} missing from {args.base}")
    legacy_ratio = scalar["ir_per_trajectory_members_difference"] / base_arm["ir_per_trajectory_members_difference"]
    item5_checks = {
        "attempts_20083": scalar["attempts"] == LEGACY_ATTEMPTS and base_arm["attempts"] == LEGACY_ATTEMPTS,
        "checksum_exact": scalar["checksum"] == base_arm["checksum"]
        and (base_arm.get("checksum_bits") is None or base_arm["checksum_bits"] == scalar.get("checksum_bits")),
        "ir_within_2_percent": abs(legacy_ratio - 1.0) <= LEGACY_IR_TOLERANCE,
    }
    item5 = all(item5_checks.values())

    gate = {"item1_members_identical": item1, "item2_checksum": item2, "item3_failure_masking": item3,
            "item4_instructions": item4, "item5_legacy_reproduction": item5, "kill": kill}
    result = {
        "schema": SCHEMA,
        "inputs": {"base": str(args.base), "base_sha256": sha256(args.base), "profile": str(args.profile),
                   "profile_sha256": sha256(args.profile)},
        "binary_sha256": profile.get("binary_sha256"), "valgrind": profile.get("valgrind"),
        "members": profile["members"], "rtol": profile["rtol"],
        "gate": gate,
        "item1_rows": item1_rows,
        "item2_rows": item2_rows, "scalar_checksum": scalar["checksum"],
        "scalar_checksum_bits": scalar.get("checksum_bits"), "scalar_attempts": scalar["attempts"],
        "item3": item3_detail,
        "item4": {"deterministic": deterministic,
                  "ir_per_trajectory": {arm: arms[arm]["ir_per_trajectory"] for arm in [scalar_arm, *batch_arms]},
                  "ratio_to_scalar": ratios, "batch8_max": BATCH8_MAX, "kill_above": KILL_ABOVE,
                  "reported_members_difference_ratio_to_scalar": cross_check,
                  "predicted_batch8": [0.65, 0.75]},
        "item5": {"checks": item5_checks, "base_attempts": base_arm["attempts"], "base_checksum": base_arm["checksum"],
                  "base_ir_per_trajectory_members_difference": base_arm["ir_per_trajectory_members_difference"],
                  "new_ir_per_trajectory_members_difference": scalar["ir_per_trajectory_members_difference"],
                  "ratio": legacy_ratio, "tolerance": LEGACY_IR_TOLERANCE},
    }
    result["verdict"] = "PASS" if (item1 and item2 and item3 and item4 and item5) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(json.dumps(gate), result["verdict"])
    for arm in batch_arms:
        print(f"{arm}: {ratios[arm]:.4f} x scalar (members-difference cross-check {cross_check[arm]:.4f})")
    print(f"legacy Ir ratio {legacy_ratio:.4f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
