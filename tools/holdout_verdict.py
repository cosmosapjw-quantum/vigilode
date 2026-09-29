#!/usr/bin/env python3
"""Holdout verdict rule with a minimum evidence standard (audit F-023).

A threshold policy recommends an event when its score is at most tau. The
holdout verdict at tau is PASS when no recommended event is a positive
(unsafe), FAIL otherwise. The verdict is called discriminating only when

  * the holdout has at least MIN_POSITIVES positives, and
  * the verdict is unchanged when tau moves one distinct-score rank down and
    one rank up among the holdout scores.

Otherwise the verdict is INCONCLUSIVE_INSUFFICIENT_POSITIVES (too few
positives) or INCONCLUSIVE_RANK_UNSTABLE (enough positives, but a one-rank
move flips it). With zero misses among n positives the 95% upper bound on the
miss rate is about 3/n, which is why n = 1 carries no information.

    python3 tools/holdout_verdict.py --v35-holdout320
"""

import argparse
import glob
import json
import math
import sys
from pathlib import Path

MIN_POSITIVES = 5
REPO = Path(__file__).resolve().parents[1]


def verdict_at(events, tau):
    unsafe_recommended = [e for e in events if e["score"] <= tau and e["positive"]]
    return "PASS" if not unsafe_recommended else "FAIL"


def neighbours(scores, tau):
    """Thresholds that admit one distinct score fewer and one more than tau."""
    distinct = sorted(set(scores))
    admitted = [s for s in distinct if s <= tau]
    rejected = [s for s in distinct if s > tau]
    down = admitted[-2] if len(admitted) >= 2 else -math.inf
    up = rejected[0] if rejected else math.inf
    return down, up


def holdout_verdict(events, tau):
    """events: [{"score": float, "positive": bool}] with finite scores."""
    if any(not math.isfinite(e["score"]) for e in events):
        raise ValueError("scores must be finite")
    n_pos = sum(1 for e in events if e["positive"])
    at_tau = verdict_at(events, tau)
    down, up = neighbours([e["score"] for e in events], tau)
    at_down, at_up = verdict_at(events, down), verdict_at(events, up)
    rank_stable = at_tau == at_down == at_up
    if n_pos < MIN_POSITIVES:
        verdict = "INCONCLUSIVE_INSUFFICIENT_POSITIVES"
    elif not rank_stable:
        verdict = "INCONCLUSIVE_RANK_UNSTABLE"
    else:
        verdict = at_tau
    return {
        "verdict": verdict,
        "raw_verdict_at_tau": at_tau,
        "n_events": len(events),
        "n_positives": n_pos,
        "minimum_positives": MIN_POSITIVES,
        "tau": tau,
        "tau_one_rank_down": down,
        "tau_one_rank_up": up,
        "verdict_one_rank_down": at_down,
        "verdict_one_rank_up": at_up,
        "rank_stable": rank_stable,
        "miss_rate_upper_95": min(1.0, 3.0 / n_pos) if n_pos else None,
    }


def v35_holdout320_events():
    events = []
    for path in sorted(
        glob.glob(str(REPO / "research/generic_enforced_prefix_budget_v35/results/fresh_holdout320/*.json"))
    ):
        for row in json.load(open(path))["rows"]:
            zeta = row.get("quadratic_drift_zeta34")
            if zeta is None or not row.get("audit_full_e_completed"):
                continue
            events.append(
                {
                    "family": row["family"],
                    "step": row["decision_accepted_step"],
                    "score": zeta,
                    # The v3.5 label is the shadow's own estimate (audit F-045).
                    "positive": not row["audit_full_e_locally_admissible"],
                }
            )
    return events


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--v35-holdout320", action="store_true")
    parser.add_argument("--events", type=Path, help="JSON list of {score, positive}")
    parser.add_argument("--tau", type=float)
    args = parser.parse_args(argv)
    if args.v35_holdout320:
        policy = json.load(
            open(REPO / "research/generic_policy_redesign_v33/results/calibration_analysis/FROZEN_ZETA34_POLICY.json")
        )
        tau_final = policy["final"]["tau"]
        tau_selected = policy["selected"]["tau"] if "selected" in policy else None
        events = v35_holdout320_events()
        report = holdout_verdict(events, tau_final)
        ranked = sorted(events, key=lambda e: e["score"])
        positives = [
            {"family": e["family"], "step": e["step"], "score": e["score"], "rank": ranked.index(e) + 1}
            for e in ranked
            if e["positive"]
        ]
        report["positives"] = positives
        report["tau_final"] = tau_final
        if tau_selected is not None:
            report["tau_selected"] = tau_selected
            report["raw_verdict_at_tau_selected"] = verdict_at(events, tau_selected)
        print(json.dumps(report, indent=2))
        return 0
    if args.events is None or args.tau is None:
        parser.error("--events and --tau, or --v35-holdout320")
    print(json.dumps(holdout_verdict(json.load(open(args.events)), args.tau), indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
