#!/usr/bin/env python3
"""Reconstruct the v3.6 continuation ledger from durable v3.5 rows.

This is deliberately a no-solver preflight.  A frozen recommendation is derived
only from an already-completed transactional prefix and the sealed zeta34
threshold.  Full-E work is then split component-wise into retained-prefix and
incremental-continuation ledgers and joined to the exact target R-JF attempt.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import statistics
from pathlib import Path
from typing import Iterable, Mapping


TAU_ZETA34 = 13.39706618860016
SCHEMA = "vigilode-v36-full-e-ledger-preflight-v1"


class LedgerError(RuntimeError):
    """Raised when durable inputs cannot support an exact work ledger."""


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _quantile(values: Iterable[float], probability: float) -> float | None:
    ordered = sorted(values)
    if not ordered:
        return None
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def _subtract_work(
    full: Mapping[str, int], prefix: Mapping[str, int], context: str
) -> dict[str, int]:
    if set(full) != set(prefix):
        missing_from_full = sorted(set(prefix) - set(full))
        missing_from_prefix = sorted(set(full) - set(prefix))
        raise LedgerError(
            f"{context}: work-counter keys differ; "
            f"missing_from_full={missing_from_full}, "
            f"missing_from_prefix={missing_from_prefix}"
        )
    continuation: dict[str, int] = {}
    for key in sorted(prefix):
        prefix_value = prefix[key]
        full_value = full[key]
        if (
            isinstance(prefix_value, bool)
            or isinstance(full_value, bool)
            or not isinstance(prefix_value, int)
            or not isinstance(full_value, int)
            or prefix_value < 0
            or full_value < 0
        ):
            raise LedgerError(f"{context}: nonnegative integer work required for {key}")
        if full_value < prefix_value:
            raise LedgerError(
                f"{context}: negative work delta for {key}: "
                f"full={full_value}, prefix={prefix_value}"
            )
        continuation[key] = full_value - prefix_value
    if any(prefix[key] + continuation[key] != full[key] for key in prefix):
        raise LedgerError(f"{context}: prefix + continuation != full work")
    return continuation


def _attempt_index(report: Mapping, source: Path) -> dict[tuple[str, int], Mapping]:
    attempts: dict[tuple[str, int], Mapping] = {}
    for attempt in report.get("attempt_rows", []):
        key = (attempt.get("trajectory_id"), attempt.get("attempt_index"))
        if key in attempts:
            raise LedgerError(f"{source}: duplicate target R-JF attempt key {key}")
        attempts[key] = attempt
    return attempts


def _same_target(event: Mapping, attempt: Mapping, context: str) -> None:
    comparisons = {
        "accepted_steps_before": "target_accepted_steps_before",
        "t_start": "t_start",
        "h": "h",
        "accepted": "target_r_attempt_accepted",
    }
    for attempt_key, event_key in comparisons.items():
        if attempt.get(attempt_key) != event.get(event_key):
            raise LedgerError(
                f"{context}: target R-JF attempt mismatch for {attempt_key}: "
                f"attempt={attempt.get(attempt_key)!r}, event={event.get(event_key)!r}"
            )


def _summary(events: list[dict]) -> dict:
    def sum_work(field: str) -> dict[str, int]:
        if not events:
            return {}
        keys = set(events[0][field])
        if any(set(event[field]) != keys for event in events):
            raise LedgerError(f"cannot aggregate inconsistent {field} counter keys")
        return {
            key: sum(event[field][key] for event in events) for key in sorted(keys)
        }

    target_rjf = sum(row["target_rjf_jvp_vectors"] for row in events)
    prefix = sum(row["prefix_jvp_vectors"] for row in events)
    continuation = sum(row["continuation_jvp_vectors"] for row in events)
    full_e = sum(row["full_e_jvp_vectors"] for row in events)
    if events and target_rjf == 0:
        raise LedgerError("recommended events have zero cumulative target R-JF JVP work")
    continuation_ratios = [
        row["continuation_over_target_rjf_jvp"] for row in events
    ]
    full_ratios = [row["full_e_over_target_rjf_jvp"] for row in events]
    continuation_jvp_values = [row["continuation_jvp_vectors"] for row in events]
    return {
        "recommendations": len(events),
        "unsafe_recommendations": sum(
            not row["full_e_locally_admissible"] for row in events
        ),
        "target_rjf_jvp_vectors": target_rjf,
        "prefix_jvp_vectors": prefix,
        "continuation_jvp_vectors": continuation,
        "full_e_jvp_vectors": full_e,
        "prefix_work": sum_work("prefix_work"),
        "continuation_work": sum_work("continuation_work"),
        "full_e_work": sum_work("full_e_work"),
        "cumulative_prefix_over_target_rjf_jvp": prefix / target_rjf
        if target_rjf
        else None,
        "cumulative_continuation_over_target_rjf_jvp": continuation / target_rjf
        if target_rjf
        else None,
        "cumulative_full_e_over_target_rjf_jvp": full_e / target_rjf
        if target_rjf
        else None,
        "continuation_over_target_rjf_jvp_quantiles": {
            "p50": _quantile(continuation_ratios, 0.50),
            "p90": _quantile(continuation_ratios, 0.90),
            "p95": _quantile(continuation_ratios, 0.95),
            "max": max(continuation_ratios, default=None),
        },
        "full_e_over_target_rjf_jvp_quantiles": {
            "p50": _quantile(full_ratios, 0.50),
            "p90": _quantile(full_ratios, 0.90),
            "p95": _quantile(full_ratios, 0.95),
            "max": max(full_ratios, default=None),
        },
        "maximum_continuation_jvp_vectors": max(
            continuation_jvp_values, default=None
        ),
        "continuation_jvp_vectors_quantiles": {
            "min": min(continuation_jvp_values, default=None),
            "p50": statistics.median(continuation_jvp_values)
            if continuation_jvp_values
            else None,
            "p95": _quantile(continuation_jvp_values, 0.95),
            "max": max(continuation_jvp_values, default=None),
        },
    }


def analyze_profiles(
    profile_directories: Mapping[str, Path],
    tau: float = TAU_ZETA34,
    source_root: Path | None = None,
) -> dict:
    if not math.isfinite(tau):
        raise LedgerError("zeta34 threshold must be finite")
    events: list[dict] = []
    source_profiles: list[dict] = []

    def display_path(path: Path) -> str:
        if source_root is None:
            return str(path)
        try:
            return str(path.resolve().relative_to(Path(source_root).resolve()))
        except ValueError:
            return str(path)

    for profile, directory in profile_directories.items():
        directory = Path(directory)
        sources = sorted(directory.glob("*.json"))
        if not sources:
            raise LedgerError(f"{profile}: no durable family JSON files in {directory}")
        profile_event_start = len(events)
        source_rows = []
        for source in sources:
            payload = json.loads(source.read_text(encoding="utf-8"))
            if payload.get("schema") != "g4-s5b0-enforced-prefix-budget-v1":
                raise LedgerError(f"{source}: unexpected source schema")
            if payload.get("status") != "complete":
                raise LedgerError(f"{source}: source status is not complete")
            if payload.get("switching_active") is not False:
                raise LedgerError(f"{source}: active switching must be false")
            if payload.get("runtime_full_e_continuations") != 0:
                raise LedgerError(f"{source}: source already contains runtime full-E work")

            attempts = _attempt_index(payload, source)
            source_recommendations = 0
            for event in payload.get("rows", []):
                zeta = event.get("quadratic_drift_zeta34")
                recommended = (
                    event.get("prefix_succeeded") is True
                    and isinstance(zeta, (int, float))
                    and not isinstance(zeta, bool)
                    and math.isfinite(zeta)
                    and zeta <= tau
                )
                if not recommended:
                    continue
                source_recommendations += 1
                context = (
                    f"{profile}/{event.get('family')}/"
                    f"attempt-{event.get('target_attempt_index')}"
                )
                if event.get("budget_exhausted") is True:
                    raise LedgerError(f"{context}: budget-exhausted event was recommended")
                if event.get("audit_full_e_completed") is not True:
                    raise LedgerError(f"{context}: recommended event lacks full-E audit work")
                prefix_work = event.get("prefix_work")
                full_e_work = event.get("audit_full_e_work")
                if not isinstance(prefix_work, dict) or not isinstance(full_e_work, dict):
                    raise LedgerError(f"{context}: missing prefix/full-E work ledger")
                continuation_work = _subtract_work(full_e_work, prefix_work, context)

                attempt_key = (
                    event.get("trajectory_id"),
                    event.get("target_attempt_index"),
                )
                attempt = attempts.get(attempt_key)
                if attempt is None:
                    raise LedgerError(f"{context}: target R-JF attempt was not found")
                _same_target(event, attempt, context)
                target_jvp = attempt.get("jvp_vectors")
                if (
                    isinstance(target_jvp, bool)
                    or not isinstance(target_jvp, int)
                    or target_jvp <= 0
                ):
                    raise LedgerError(f"{context}: target R-JF JVP work must be positive")
                prefix_jvp = prefix_work.get("jvp_vectors")
                continuation_jvp = continuation_work.get("jvp_vectors")
                full_e_jvp = full_e_work.get("jvp_vectors")
                if not all(
                    isinstance(value, int) and not isinstance(value, bool)
                    for value in (prefix_jvp, continuation_jvp, full_e_jvp)
                ):
                    raise LedgerError(f"{context}: JVP ledger fields are missing")

                events.append(
                    {
                        "profile": profile,
                        "source_file": display_path(source),
                        "source_sha256": _sha256(source),
                        "trajectory_id": event.get("trajectory_id"),
                        "family": event.get("family"),
                        "dimension": event.get("dimension"),
                        "rtol": event.get("rtol"),
                        "decision_accepted_step": event.get("decision_accepted_step"),
                        "target_attempt_index": event.get("target_attempt_index"),
                        "target_r_attempt_accepted": event.get(
                            "target_r_attempt_accepted"
                        ),
                        "t_start": event.get("t_start"),
                        "h": event.get("h"),
                        "zeta34": zeta,
                        "full_e_total_error": event.get("audit_full_e_total_error"),
                        "full_e_locally_admissible": event.get(
                            "audit_full_e_locally_admissible"
                        )
                        is True,
                        "target_rjf_wall_seconds": attempt.get("wall_seconds"),
                        "target_rjf_jvp_vectors": target_jvp,
                        "prefix_jvp_vectors": prefix_jvp,
                        "continuation_jvp_vectors": continuation_jvp,
                        "full_e_jvp_vectors": full_e_jvp,
                        "prefix_over_target_rjf_jvp": prefix_jvp / target_jvp,
                        "continuation_over_target_rjf_jvp": continuation_jvp
                        / target_jvp,
                        "full_e_over_target_rjf_jvp": full_e_jvp / target_jvp,
                        "continuation_over_prefix_jvp": continuation_jvp / prefix_jvp
                        if prefix_jvp
                        else None,
                        "prefix_work": dict(prefix_work),
                        "continuation_work": continuation_work,
                        "full_e_work": dict(full_e_work),
                    }
                )
            source_rows.append(
                {
                    "path": display_path(source),
                    "sha256": _sha256(source),
                    "recommendations": source_recommendations,
                }
            )
        profile_events = events[profile_event_start:]
        source_profiles.append(
            {
                "profile": profile,
                "directory": display_path(directory),
                "source_files": source_rows,
                "summary": _summary(profile_events),
            }
        )

    events.sort(
        key=lambda row: (
            row["profile"],
            row["family"],
            row["trajectory_id"],
            row["target_attempt_index"],
        )
    )
    overall = _summary(events)
    verdict = (
        "PASS_TO_RUNTIME_SHADOW_MEASUREMENT"
        if overall["recommendations"] > 0
        and overall["unsafe_recommendations"] == 0
        else "HOLD_LEDGER_PREFLIGHT"
    )
    return {
        "schema": SCHEMA,
        "frozen_zeta34_tau": tau,
        "analysis_kind": "durable-ledger-only-no-solver-run",
        "verdict": verdict,
        "source_profiles": source_profiles,
        "overall": overall,
        "events": events,
        "limitations": [
            "Durable v3.5 work counters contain no continuation wall time; optimized paired wall economics require a separate runtime shadow measurement.",
            "The PASS verdict authorizes implementation/measurement of the read-only shadow only; it does not authorize active switching or a speedup claim.",
        ],
    }


def _default_profiles(repository_root: Path) -> dict[str, Path]:
    root = repository_root / "research/generic_enforced_prefix_budget_v35/results"
    return {
        "N96": root / "consumed_replay/calibration96",
        "N192": root / "consumed_replay/calibration192",
        "N256": root / "consumed_replay/calibration256",
        "N320": root / "fresh_holdout320",
        "N384": root / "consumed_replay/holdout384",
    }


def _write_events_csv(path: Path, events: list[dict]) -> None:
    columns = [
        "profile",
        "family",
        "dimension",
        "rtol",
        "decision_accepted_step",
        "target_attempt_index",
        "t_start",
        "h",
        "zeta34",
        "full_e_total_error",
        "full_e_locally_admissible",
        "target_rjf_jvp_vectors",
        "prefix_jvp_vectors",
        "continuation_jvp_vectors",
        "full_e_jvp_vectors",
        "prefix_over_target_rjf_jvp",
        "continuation_over_target_rjf_jvp",
        "full_e_over_target_rjf_jvp",
        "continuation_over_prefix_jvp",
        "source_file",
        "source_sha256",
    ]
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=columns, lineterminator="\n")
        writer.writeheader()
        for event in events:
            writer.writerow({column: event.get(column) for column in columns})


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-events-csv", type=Path, required=True)
    arguments = parser.parse_args()

    repository_root = Path(__file__).resolve().parents[3]
    result = analyze_profiles(
        _default_profiles(repository_root), source_root=repository_root
    )
    arguments.output_json.parent.mkdir(parents=True, exist_ok=True)
    arguments.output_events_csv.parent.mkdir(parents=True, exist_ok=True)
    arguments.output_json.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    _write_events_csv(arguments.output_events_csv, result["events"])


if __name__ == "__main__":
    main()
