"""Independent Fraction-from-bits audit of fixed native rotation stage records.

Input is the STRUCTURAL_NATIVE_BITS JSON line emitted by the actual Rust
directional-quadratic contract test (run with --nocapture), or that JSON object
alone. The declared operator is a direct sum of [[0,-3],[3,0]] blocks. This
verifies sampled represented binary64 stage solves, not an IVP certificate or
an arbitrary client/operator identity. No native kernel is imported or run.
"""

import argparse
from fractions import Fraction as Q
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import sys
import time


PREFIX = "STRUCTURAL_NATIVE_BITS "


def packet_from_bytes(raw):
    text = raw.decode("utf-8")
    matches = [line.split(PREFIX, 1)[1] for line in text.splitlines() if PREFIX in line]
    if matches:
        if len(matches) != 1:
            raise ValueError("exactly one native-bit packet is required")
        return json.loads(matches[0])
    return json.loads(text)


def bit_pattern(value):
    if isinstance(value, bool):
        raise ValueError("boolean is not a binary64 bit pattern")
    if isinstance(value, str):
        value = int(value.removeprefix("0x"), 16)
    if not isinstance(value, int) or not 0 <= value < 2**64:
        raise ValueError("binary64 bits must be unsigned64 integers or hex strings")
    return value


def dyadic(value):
    number = struct.unpack(">d", struct.pack(">Q", bit_pattern(value)))[0]
    if not math.isfinite(number):
        raise ValueError("scientific scalar must be finite")
    return Q.from_float(number)


def vector(values):
    if not isinstance(values, list):
        raise ValueError("vector bit patterns must be arrays")
    return tuple(dyadic(v) for v in values)


def shifted_rotation(x, sigma):
    # Original A=I-sigma*J, independently assembled from exact block entries.
    out = []
    for index in range(0, len(x), 2):
        left, right = x[index:index + 2]
        out.extend((left + 3*sigma*right, right - 3*sigma*left))
    return tuple(out)


def original_residual(rhs, x, sigma):
    return tuple(b - ax for b, ax in zip(rhs, shifted_rotation(x, sigma)))


def squared_norm(x):
    return sum((v*v for v in x), Q(0))


def exact_solution(rhs, sigma):
    denominator = 1 + 9*sigma*sigma
    out = []
    for index in range(0, len(rhs), 2):
        left, right = rhs[index:index + 2]
        out.extend(((left - 3*sigma*right)/denominator,
                    (right + 3*sigma*left)/denominator))
    return tuple(out)


def ratio(value):
    return f"{value.numerator}/{value.denominator}"


def audit(packet, expected_source):
    if not re.fullmatch(r"[0-9a-f]{40}", expected_source):
        raise ValueError("expected source must be a full lowercase commit hash")
    if packet.get("schema") != "structural-native-bits-v1":
        raise ValueError("unsupported native-bit schema")
    if packet.get("source_commit") != expected_source:
        raise ValueError("native source label does not match the execution pin")
    rows = packet.get("records")
    if not isinstance(rows, list) or len(rows) != 8:
        raise ValueError("registered packet requires six accepted and two refused records")
    output, exact_checks = [], 0
    accepted, refused, poles, wrong = 0, 0, 0, 0

    def check(condition, message):
        nonlocal exact_checks
        exact_checks += 1
        if not condition:
            raise AssertionError(message)

    for index, row in enumerate(rows):
        sigma, a, b = (dyadic(row[key]) for key in ("sigma_bits", "a_bits", "b_bits"))
        rhs = vector(row["rhs_bits"])
        check(len(rhs) in (2, 4), f"record{index}: unregistered rotation dimension")
        x0 = tuple(Q(0) for _ in rhs) if row["x0_bits"] is None else vector(row["x0_bits"])
        check(len(x0) == len(rhs), f"record{index}: initial-guess shape")
        threshold = dyadic(row["threshold_bits"])
        check(threshold >= 0, f"record{index}: negative threshold")
        before = tuple(bit_pattern(v) for v in row["output_before_bits"])
        after = tuple(bit_pattern(v) for v in row["output_after_bits"])
        check(len(before) == len(after) == len(rhs), f"record{index}: output shape")
        actions = row["operator_actions"]
        check(type(actions) is int and actions >= 0, f"record{index}: action count")
        result = {"index": index, "status": row["status"], "dimension": len(rhs),
                  "sigma_exact": ratio(sigma), "threshold_exact": ratio(threshold),
                  "operator_actions": actions}
        denominator = 1 - a*sigma - b*sigma*sigma
        if row["status"] == "accepted":
            accepted += 1
            candidate = vector(row["candidate_bits"])
            check(len(candidate) == len(rhs), f"record{index}: candidate shape")
            check(tuple(bit_pattern(v) for v in row["candidate_bits"]) == after,
                  f"record{index}: committed candidate bits")
            check(a == 0 and b == -9, f"record{index}: accepted rotation hypothesis")
            residual = original_residual(rhs, candidate, sigma)
            residual_squared = squared_norm(residual)
            check(residual_squared <= threshold*threshold,
                  f"record{index}: exact original residual exceeds represented threshold")
            solution = exact_solution(rhs, sigma)
            error_squared = squared_norm(tuple(x-y for x, y in zip(candidate, solution)))
            resolvent_squared = 1 + 9*sigma*sigma
            check(error_squared == residual_squared/resolvent_squared,
                  f"record{index}: exact rotation residual/error identity")
            check(error_squared <= threshold*threshold/resolvent_squared,
                  f"record{index}: exact stage-error bound")
            initial_squared = squared_norm(original_residual(rhs, x0, sigma))
            warm_action = int(any(x0))
            expected_actions = (warm_action if initial_squared <= threshold*threshold else
                                warm_action + (1 if sigma == 0 else 2))
            check(actions == expected_actions, f"record{index}: original-action accounting")
            result.update(exact_residual_squared=ratio(residual_squared),
                          exact_stage_error_squared=ratio(error_squared),
                          stage_error_squared_upper=ratio(threshold*threshold/resolvent_squared),
                          residual_components=[ratio(x) for x in residual])
        elif row["status"] == "refused":
            refused += 1
            check(before == after, f"record{index}: refusal modified output sentinel")
            if denominator == 0:
                poles += 1
                check(actions == 0, f"record{index}: representation pole incurred an action")
                result["refusal_control"] = "exact_representation_pole"
            else:
                wrong += 1
                v = original_residual(rhs, x0, sigma)
                jv = []
                for j in range(0, len(v), 2):
                    jv.extend((-3*v[j+1], 3*v[j]))
                exact_proposal = tuple(start+((1-a*sigma)*vi+sigma*ji)/denominator
                                       for start, vi, ji in zip(x0, v, jv))
                r2 = squared_norm(original_residual(rhs, exact_proposal, sigma))
                check(r2 > threshold*threshold,
                      f"record{index}: wrong-coefficient fixture lacks exact separation")
                check(actions == 2 + int(any(x0)), f"record{index}: refusal action accounting")
                result.update(refusal_control="wrong_coefficients_separated_from_threshold",
                              exact_hypothetical_proposal_residual_squared=ratio(r2))
        else:
            raise ValueError(f"record{index}: unknown status")
        output.append(result)
    check((accepted, refused, poles, wrong) == (6, 2, 1, 1), "registered case inventory")
    return {"result": "PASS_SCOPED", "source_commit": expected_source,
            "accepted_records": accepted, "refused_records": refused,
            "exact_checks": exact_checks, "records": output,
            "scope": "sampled native binary64 rotation stage residual/error certification",
            "model": "direct sum J=[[0,-3],[3,0]], original A=I-sigma*J",
            "physical_ivp_certificate": False,
            "general_client_operator_identity_verified": False,
            "native_source_authentication": "external published execution receipt; packet label cross-checked"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--expected-source", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit("refusing to overwrite an existing audit output")
    raw = args.input.read_bytes()
    started, cpu = time.perf_counter(), time.process_time()
    try:
        result = audit(packet_from_bytes(raw), args.expected_source)
        exit_code = 0
    except (AssertionError, KeyError, TypeError, ValueError, OverflowError) as error:
        result = {"result": "FAIL", "expected_source": args.expected_source,
                  "error_type": type(error).__name__, "error": str(error),
                  "physical_ivp_certificate": False}
        exit_code = 1
    result.update(input_path=str(args.input.resolve()),
                  input_sha256=hashlib.sha256(raw).hexdigest(),
                  verifier_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  wall_seconds=time.perf_counter()-started,
                  cpu_seconds=time.process_time()-cpu,
                  python=sys.version, exit_code=exit_code)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2)
        stream.write("\n")
    print(json.dumps({key: result.get(key) for key in
                     ("result", "source_commit", "accepted_records", "refused_records",
                      "exact_checks", "error_type", "error", "input_sha256", "exit_code")}))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
