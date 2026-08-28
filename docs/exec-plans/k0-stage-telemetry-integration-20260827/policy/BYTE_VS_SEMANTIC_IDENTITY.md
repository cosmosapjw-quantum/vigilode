# Scientific coding identity policy — byte provenance versus semantic validity

## Purpose

A checksum or commit mismatch is not, by itself, a scientific failure. Identity checks must be applied to the artifact class they actually govern.

## Artifact classes

1. **Control/source bytes** — executable code, schema, validator, frozen input fixture. Exact commit/tree or byte hash selects the authority. Drift is a control-plane stop until inspected.
2. **Raw deterministic evidence** — preserve the original file bytes and SHA-256 for provenance. A changed copy is not silently substituted.
3. **Numerical/scientific content** — compare canonical quantities that affect the scientific claim: equations, parameters, tolerances, solver route, accept/reject sequence, work counters, hard gates, event rows, stage values, audit outcomes. Serialization-only differences are not scientific failures.
4. **Representation/wrapper/transport** — JSON key order, archive layout, wrapper schema metadata, source-location metadata, and equivalent labels may change without changing numerical content. Compare structure and canonical semantic projections, not archive SHA alone.

## Mandatory rules

- Preserve every raw WU-04 SHA-256 in the migration receipt.
- Never fabricate a field that the historical raw run did not record.
- `error: null` means no recorded error; it is not an ERROR terminal state.
- Legacy missing signed-residual telemetry is represented as `null` plus `LEGACY_NOT_RECORDED`; current source sign correctness is tested separately with a vector-aware mutation test.
- Tolerance identity is the exact numerical pair `rtol=1e-10`, `atol=1e-12`; an absent or differently formatted label is not a mismatch.
- Source head/tree are provenance and must be derived from the raw envelope where actually recorded. They are excluded from the numerical payload digest.
- A wrapper/package/archive SHA mismatch is a scientific blocker only if the canonical numerical/scientific projection differs or provenance cannot be traced.
- A genuine difference in equations, numerical tolerance, kernel arm, convergence authority, stage work, hard gates, event rows, unsafe recommendations, or numerical payload remains fail-closed.

## Required outputs

Each cell migration records both:

```text
raw_receipt_sha256        # exact-byte provenance
numerical_payload_sha256  # canonical scientific/numerical content
raw_stage_payload_sha256  # canonical stage scientific/work content
```

These hashes answer different questions and must never be conflated.
