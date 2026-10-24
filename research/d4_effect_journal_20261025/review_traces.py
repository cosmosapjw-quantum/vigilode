#!/usr/bin/env python3
"""Independent saved-trace oracle for the frozen D4 protocol.

The reference is the CONTRACT transition relation, not a reimplementation of
the Rust journal/Session. It does not compute ODE stages, norm arithmetic or
controller proposals. It verifies finite protocol words plus exact ownership
bindings and scientific-prefix preservation in the observed trace.

Usage: python3 review_traces.py TRACES.jsonl NEW_REVIEW.json
All output creation is exclusive. Run only under the root resource ledger.
"""
from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass, field
import hashlib
import json
import math
from pathlib import Path
import struct
import sys
from typing import Any

ALPHABET = "ARFBLSKW"
WORDS = 8 ** 4
CASES = 4 * WORDS
ROWS = 4 * CASES
SCHEMA = "d4.protocol.trace.v1"
WRONG_RUN_OFFSET = 65536
MUTANTS = (
    "clear_pending_on_delivery_error",
    "advance_while_pending",
    "non_idempotent_receiver",
)


def word_text(word: int) -> str:
    return "".join(ALPHABET[(word // (8 ** power)) % 8] for power in (3, 2, 1, 0))


@dataclass
class Protocol:
    """State (attempts, accepted, rejected, pending, halt, ACK, sink bindings).

    A/R/F are producer requests; B/L/S are single delivery requests; K/W are
    caller assertions. A pending batch is an admission barrier, and delivery
    neither creates solver work nor grants ACK. K may acknowledge before S.
    The tiny receiver map is an explicit conditional test fixture, not a claim
    that a remote receiver is durable.
    """
    cap: int
    case_id: int
    mutation: str | None = None
    a: int = 0
    n: int = 0
    r: int = 0
    p: tuple[int, int] | None = None
    halt: str = "none"
    last_ack: tuple[int, int] | None = None
    acknowledged: int = 0
    sink: dict[tuple[int, int], int] = field(default_factory=dict)
    success_ids: set[tuple[int, int]] = field(default_factory=set)
    delivery_calls: int = 0
    delivery_errors: int = 0
    apply_requests: int = 0
    delivery_successes: int = 0
    applications: int = 0
    dedup_hits: int = 0
    fatal_callbacks: int = 0
    ack_requests: int = 0
    ack_duplicates: int = 0
    ack_errors: int = 0
    wrong_ack_errors: int = 0
    no_pending_ack_errors: int = 0
    acks_without_sink_effect: int = 0
    acks_without_success: int = 0

    @property
    def run_id(self) -> int:
        return self.case_id + 1

    def transition(self, letter: str) -> dict[str, Any]:
        received: tuple[int, int, int] | None = None
        if letter in "ARF":
            if self.p is not None and self.mutation != "advance_while_pending":
                result = "waiting"
            elif self.halt != "none":
                result = self.halt
            elif self.a == self.cap:
                self.halt = "budget"
                result = "budget"
            else:
                self.a += 1
                if letter == "A":
                    self.n += 1
                    self.p = (self.a, self.n)
                    result = "accepted"
                elif letter == "R":
                    self.r += 1
                    result = "recoverable"
                else:
                    self.fatal_callbacks += 1
                    self.halt = "fatal"
                    result = "fatal"
        elif letter in "BLS":
            if self.p is None:
                result = "no_pending"
            else:
                self.delivery_calls += 1
                if letter != "B":
                    self.apply_requests += 1
                    if self.p in self.sink:
                        if self.mutation == "non_idempotent_receiver":
                            self.applications += 1
                        else:
                            self.dedup_hits += 1
                    else:
                        self.sink[self.p] = self.case_id + self.p[0]
                        self.applications += 1
                if letter in "BL":
                    self.delivery_errors += 1
                    result = "delivery_error"
                    if self.mutation == "clear_pending_on_delivery_error":
                        self.p = None
                else:
                    self.delivery_successes += 1
                    self.success_ids.add(self.p)
                    result = "delivered"
        else:
            self.ack_requests += 1
            exact = self.p if self.p is not None else (self.last_ack or (0, 0))
            wrong = letter == "W"
            received = (self.run_id + (WRONG_RUN_OFFSET if wrong else 0), *exact)
            if not wrong and self.last_ack == exact:
                self.ack_duplicates += 1
                result = "already_acknowledged"
            elif self.p is None:
                self.ack_errors += 1
                self.no_pending_ack_errors += 1
                result = "no_pending_ack"
            elif wrong:
                self.ack_errors += 1
                self.wrong_ack_errors += 1
                result = "wrong_ack"
            else:
                self.acknowledged += 1
                self.last_ack = self.p
                if self.p not in self.sink:
                    self.acks_without_sink_effect += 1
                if self.p not in self.success_ids:
                    self.acks_without_success += 1
                self.p = None
                result = "acknowledged"

        return {
            "result": result,
            "attempts": self.a,
            "accepted": self.n,
            "rejected": self.r,
            "producer": self.a,
            "producer_observed": self.a,
            "attempted_events": self.a,
            "attempted_payload_bytes": 8 * self.a,
            "recoverable_callbacks": self.r,
            "fatal_callbacks": self.fatal_callbacks,
            "domain_rejected": 0,
            "pending_run_id": self.run_id if self.p is not None else None,
            "pending_attempt": self.p[0] if self.p is not None else None,
            "pending_epoch": self.p[1] if self.p is not None else None,
            "pending_payload": self.case_id + self.p[0] if self.p is not None else None,
            "pending_events": 1 if self.p is not None else 0,
            "pending_bytes": 8 if self.p is not None else 0,
            "terminalkind": self.halt,
            "epoch": self.n,
            "state_zero": True,
            "deliverycalls": self.delivery_calls,
            "deliveryerrors": self.delivery_errors,
            "sink_calls": self.delivery_calls,
            "apply_requests": self.apply_requests,
            "deliverysuccesses": self.delivery_successes,
            "published": self.applications,
            "sink_unique_batches": len(self.sink),
            "dedup_hits": self.dedup_hits,
            "binding_conflicts": 0,
            "published_records": [
                {
                    "run_id": self.run_id,
                    "attempt": attempt,
                    "epoch": epoch,
                    "payload": payload,
                    "kind": 17,
                    "bytes": 8,
                }
                for (attempt, epoch), payload in sorted(self.sink.items())
            ],
            "ack_requests": self.ack_requests,
            "acknowledged": self.acknowledged,
            "acknowledged_observed": self.acknowledged,
            "ack_duplicates": self.ack_duplicates,
            "ack_errors": self.ack_errors,
            "wrong_ack_errors": self.wrong_ack_errors,
            "no_pending_ack_errors": self.no_pending_ack_errors,
            "acks_without_sink_effect": self.acks_without_sink_effect,
            "acks_without_success": self.acks_without_success,
            "last_ack_attempt": self.last_ack[0] if self.last_ack is not None else None,
            "last_ack_epoch": self.last_ack[1] if self.last_ack is not None else None,
            "ack_received_run_id": received[0] if received else None,
            "ack_received_attempt": received[1] if received else None,
            "ack_received_epoch": received[2] if received else None,
        }


def same(actual: Any, expected: Any) -> bool:
    """JSON semantic comparison with exact types (True must not equal 1)."""
    if type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(
            same(actual[k], value) for k, value in expected.items()
        )
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(
            same(a, b) for a, b in zip(actual, expected)
        )
    return actual == expected


def source_sha(path: Path) -> str | None:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def inspect_traces(path: Path) -> dict[str, Any]:
    trace_hash = hashlib.sha256()
    failures: list[dict[str, Any]] = []
    failure_count = 0
    comparisons = 0
    prefix_checks = 0
    progress_checks = 0
    ledger_checks = 0
    barrier_checks = 0
    delivery_retention_checks = 0
    action_counts: Counter[str] = Counter()
    result_counts: Counter[str] = Counter()
    cases_per_cap: Counter[int] = Counter()
    final_terminal_counts: Counter[str] = Counter()
    totals: Counter[str] = Counter()
    mutant_stats = {
        name: {"differing_rows": 0, "case_ids": set(), "first_witness": None}
        for name in MUTANTS
    }
    current: Protocol | None = None
    mutants: dict[str, Protocol] = {}
    previous_t_bits = 0
    previous_t = 0.0
    accepted_ids: set[tuple[int, int]] = set()
    observed_rows = 0

    def fail(code: str, line: int, **details: Any) -> None:
        nonlocal failure_count
        failure_count += 1
        if len(failures) < 32:
            failures.append({"code": code, "line": line, **details})

    with path.open("rb") as stream:
        for ordinal, raw in enumerate(stream):
            trace_hash.update(raw)
            observed_rows += 1
            if ordinal >= ROWS:
                fail("extra_row", ordinal + 1)
                continue
            case_id, step = divmod(ordinal, 4)
            cap, word = divmod(case_id, WORDS)
            sequence = word_text(word)
            letter = sequence[step]
            if step == 0:
                current = Protocol(cap, case_id)
                mutants = {name: Protocol(cap, case_id, name) for name in MUTANTS}
                previous_t_bits = 0
                previous_t = 0.0
                accepted_ids = set()
                cases_per_cap[cap] += 1
            assert current is not None
            before_p = current.p
            before_a = current.a
            expected = current.transition(letter)
            expected.update({
                "schema": SCHEMA,
                "case_id": case_id,
                "word": word,
                "run_id": case_id + 1,
                "sequence": sequence,
                "step": step,
                "action": letter,
                "cap": cap,
            })
            mutated = {name: model.transition(letter) for name, model in mutants.items()}
            action_counts[letter] += 1
            result_counts[expected["result"]] += 1
            if expected["result"] == "accepted":
                accepted_ids.add((current.a, current.n))
            try:
                actual = json.loads(raw)
                if type(actual) is not dict:
                    raise ValueError("record must be a JSON object")
            except (ValueError, UnicodeDecodeError) as error:
                fail("invalid_json_record", ordinal + 1, detail=str(error))
                actual = {}

            required_keys = set(expected) | {"t_bits"}
            if set(actual) != required_keys:
                fail(
                    "record_fields", ordinal + 1,
                    missing=sorted(required_keys - set(actual)),
                    unexpected=sorted(set(actual) - required_keys),
                )
            for key, value in expected.items():
                comparisons += 1
                if key not in actual or not same(actual.get(key), value):
                    fail(
                        "field_mismatch", ordinal + 1, case_id=case_id,
                        step=step, action=letter, field=key,
                        expected=value, actual=actual.get(key),
                    )

            t_bits = actual.get("t_bits")
            if type(t_bits) is not int or not 0 <= t_bits < (1 << 64):
                fail("invalid_t_bits", ordinal + 1)
            else:
                t = struct.unpack(">d", t_bits.to_bytes(8, "big"))[0]
                if expected["result"] == "accepted":
                    progress_checks += 1
                    if not math.isfinite(t) or not previous_t < t <= 1.0e6:
                        fail("accepted_clock_progress", ordinal + 1, before=previous_t, after=t)
                else:
                    prefix_checks += 1
                    if t_bits != previous_t_bits:
                        fail("scientific_prefix_changed_without_acceptance", ordinal + 1)
                previous_t_bits, previous_t = t_bits, t

            # Independent algebraic relations, in addition to the state oracle.
            ledger_checks += 1
            if actual.get("attempts") != (
                actual.get("accepted", -100) + actual.get("rejected", -100)
                + actual.get("fatal_callbacks", -100)
            ):
                fail("attempt_ledger_identity", ordinal + 1)
            if type(actual.get("producer")) is not int or not 0 <= actual["producer"] <= cap:
                fail("producer_cap", ordinal + 1)
            for record in actual.get("published_records", []):
                if not isinstance(record, dict) or (record.get("attempt"), record.get("epoch")) not in accepted_ids:
                    fail("published_without_accepted_batch", ordinal + 1, record=record)
            if before_p is not None and letter in "ARF":
                barrier_checks += 1
                if actual.get("attempts") != before_a or actual.get("result") != "waiting":
                    fail("pending_admission_barrier", ordinal + 1)
            if before_p is not None and letter in "BLS":
                delivery_retention_checks += 1
                if (actual.get("pending_attempt"), actual.get("pending_epoch")) != before_p:
                    fail("delivery_changed_pending_identity", ordinal + 1)

            # These are explicit semantic mutants, not compiled Rust mutants.
            for name, model_row in mutated.items():
                differing = [
                    key for key, value in model_row.items()
                    if key not in actual or not same(actual.get(key), value)
                ]
                if differing:
                    stat = mutant_stats[name]
                    stat["differing_rows"] += 1
                    stat["case_ids"].add(case_id)
                    if stat["first_witness"] is None:
                        fields = differing[:8]
                        stat["first_witness"] = {
                            "case_id": case_id, "cap": cap, "sequence": sequence,
                            "step": step, "action": letter,
                            "fields": fields,
                            "observed": {key: actual.get(key) for key in fields},
                            "mutant": {key: model_row[key] for key in fields},
                        }
            if step == 3:
                final_terminal_counts[current.halt] += 1
                for key in (
                    "attempts", "accepted", "rejected", "producer", "deliverycalls",
                    "deliveryerrors", "deliverysuccesses", "published", "dedup_hits",
                    "acknowledged", "ack_duplicates", "ack_errors",
                    "acks_without_sink_effect", "acks_without_success",
                ):
                    totals[key] += expected[key]

    if observed_rows != ROWS:
        fail("row_count", observed_rows, expected=ROWS, actual=observed_rows)
    if cases_per_cap != Counter({cap: WORDS for cap in range(4)}):
        fail("case_coverage", observed_rows, actual=dict(cases_per_cap))
    if action_counts != Counter({letter: ROWS // 8 for letter in ALPHABET}):
        fail("action_coverage", observed_rows, actual=dict(action_counts))
    for name, stat in mutant_stats.items():
        stat["cases_detected"] = len(stat.pop("case_ids"))
        stat["detected"] = stat["differing_rows"] > 0
        if not stat["detected"]:
            fail("undetected_semantic_mutant", observed_rows, mutation=name)
    if totals["acks_without_sink_effect"] <= 0 or totals["dedup_hits"] <= 0:
        fail("missing_discriminating_fixture", observed_rows, totals=dict(totals))

    unit = Path(__file__).resolve().parent
    return {
        "schema": "d4.independent.protocol.review.v1",
        "status": "PASS_ALL_FROZEN_PROTOCOL_TRACES" if failure_count == 0 else "FAIL",
        "reference": "Frozen CONTRACT state relation, independent of Rust implementation arithmetic",
        "input": {"file": path.name, "sha256": trace_hash.hexdigest(), "rows": observed_rows},
        "source_sha256": {
            "review_traces.py": source_sha(Path(__file__).resolve()),
            "source/trace_corpus.rs": source_sha(unit / "source" / "trace_corpus.rs"),
            "CONTRACT.md": source_sha(unit / "CONTRACT.md"),
        },
        "coverage": {
            "cases": sum(cases_per_cap.values()), "rows": observed_rows,
            "alphabet": ALPHABET, "sequence_length": 4, "caps": [0, 1, 2, 3],
            "cases_per_cap": dict(cases_per_cap), "action_counts": dict(action_counts),
            "result_counts": dict(result_counts), "final_terminal_counts": dict(final_terminal_counts),
        },
        "checks": {
            "field_comparisons": comparisons,
            "scientific_prefix_preservation_rows": prefix_checks,
            "accepted_clock_progress_rows": progress_checks,
            "attempt_ledger_identity_rows": ledger_checks,
            "pending_admission_barrier_rows": barrier_checks,
            "delivery_retained_identity_rows": delivery_retention_checks,
            "published_batches_bound_to_accepted_history": True if failure_count == 0 else None,
        },
        "aggregate_final_case_counters": dict(totals),
        "semantic_mutants": mutant_stats,
        "failure_count": failure_count, "first_failures": failures,
        "scope": {
            "finite_trace_corpus": "All words of length4 only; longer-word claims use the theorem",
            "remote_exactly_once": "Conditional on an atomic durable idempotent sink; this receiver is in-memory",
            "bare_exact_ack": "Caller assertion can clear pending before delivery and is counted explicitly",
            "nonreturning_callbacks": "Outside finite invocation-to-return termination contract",
            "ODE_stage_calls": 0,
            "D2_D3_prior_experiments_replayed": 0,
            "native_owner_reservation_dirty": "UNRESOLVED_INPUT_DEPENDENCY",
            "production": "HOLD", "global_completion": None,
        },
        "python": sys.version,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    # Reserve the new report before spending execution on its input.
    with args.output.open("x", encoding="utf-8") as destination:
        report = inspect_traces(args.trace)
        json.dump(report, destination, indent=2, ensure_ascii=False)
        destination.write("\n")
    print(json.dumps({
        "status": report["status"],
        "cases": report["coverage"]["cases"],
        "rows": report["coverage"]["rows"],
        "failure_count": report["failure_count"],
        "mutants_detected": sum(v["detected"] for v in report["semantic_mutants"].values()),
    }))
    return 0 if report["failure_count"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

