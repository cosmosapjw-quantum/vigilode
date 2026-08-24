# PM-3 Timing Authority Validator Verification Receipt

## STATUS

`FINAL_AUTHORITY_EVIDENCE_CLOSURE_READY_PENDING_REMOTE_REVIEW`

## Authority anchors

- canonical remote parent: `main@6a10345e5a8b2fd77a74642aedea0c68ee0041fb`
- canonical remote parent tree: `0a98b761c917670318e2213b81fe77f3fbe08d0c`
- scientific implementation baseline: `4384eab8397b20903377705a99b4db6e8370e2bd`
- scientific implementation tree: `4a82e7b9196c383fdd9a9cae5ba566035ea420e0`
- first published PR #10 head: `02ae95350777939bce4208488581bcb073cc45a1`
- first published PR #10 tree: `17d5fbd760c6b16a4ca39f2f88a291fc72b5e5e8`
- R4 corrective PR #10 head: `98688a494477a78ea82ec6ae4f0c0468ef8d56d4`
- R4 corrective PR #10 tree: `acb29a7efdf5d9f883b48e63234f1e6a185eea68`

PR #7 and PR #8 changed design/plan documents only. PM-3 modifies only
the v3.7 timing-authority validator research surface. The final closure is a
single bounded successor commit to the R4 PR head and does not alter solver
physics or generate timing measurements.

## Verified commands

```text
python -m unittest test_timing_authority_validator -v
  55/55 PASS

python research/generic_frozen_full_e_shadow_v36/scripts/test_analyze_shadow_economics.py
  2/2 PASS

python research/generic_frozen_full_e_shadow_v36/scripts/test_analyze_full_e_shadow_ledger.py
  3/3 PASS

bash research/generic_timing_replication_continuation_transaction_v37/scripts/run_timing_authority_validator_selftest.sh
  PASS; deterministic retrospective output; no real timing-attempt directory

python -m py_compile <validator> <tests>
  PASS

cargo fmt --all -- --check
  PASS under Rust 1.94.1

git diff --check
  PASS
```

Toolchain:

```text
rustc 1.94.1 (e408947bf 2026-03-25)
cargo 1.94.1 (29ea6fb6a 2026-03-24)
rustfmt 1.8.0-stable
```

## Behavioral verification

- exact sealed contract thresholds and frozen zeta-tau identity fail closed;
- exact campaign-root and five-profile regular-file layouts are enforced;
- release/measurement profile metadata, R-JF calibration-arm identity, and
  frozen per-profile repetition counts are enforced for every paired arm;
- valid synthetic campaigns retain 5 profiles, 5 warm-ups, and 35 measured
  pairs;
- missing tail or middle rows retain every structurally readable row and reject
  the whole campaign;
- zero-row, one-row, duplicate-index, and unexpected-index attempts emit
  serializable NON_AUTHORITY decisions with named failures;
- proposed-interval and Gamma/ratio formulas are validated;
- repetition/frozen-count, Cargo-profile, and calibration-arm drift fail closed;
- boolean repetition counts, non-integer declared pair counts, and invalid
  host/counter numeric types fail closed;
- favorable ratio direction cannot change the host-quality verdict;
- dirty tree, contract hash, idle/steal, swap, thermal, arm-span, and order-gap
  gates each fail closed;
- Git, Rust, binary, host, affinity, and thread identity mismatches are named
  independently across attempts;
- campaign-decision and attempt-summary schemas are v2;
- each summary input is freshly regenerated from its campaign path and must
  match the submitted nested decision under type-sensitive canonical JSON
  equality; boolean/integer substitutions cannot cross this boundary;
- canonical authority-evidence SHA-256 is derived from validated attestation
  inputs and primary pair evidence, not path labels or incidental files;
- row ordering, JSON formatting/numeric spelling, unvalidated metadata notes,
  signed zero, and tolerated redundant Gamma/ratio/CPU-fraction spelling cannot
  manufacture a distinct campaign;
- duplicate normalized paths, campaign-tree hashes, or canonical evidence hashes
  are rejected before counting;
- only three genuinely distinct, fully revalidated passing campaigns within at
  most five attempts promote descriptive timing;
- CLI summary succeeds for three genuine campaigns and fails atomically for a
  tampered complete-shaped decision;
- 2/5 passing campaigns yield host-unsuitable/no-promotion;
- six attempts are rejected;
- speedup and active switching authorization remain false.

## Retrospective result

- all 35 historical v3.6 measured pairs retained;
- N=384 R-JF span: `13.82492944330828`;
- N=384 shadow span: `59.214273363533934`;
- N=384 order-median absolute gap: `0.6857626375408328`;
- historical `PASS_DESCRIPTIVE_ECONOMICS` is not rewritten;
- retrospective contract diagnostic:
  `WHOLE_V36_CAMPAIGN_NON_AUTHORITY_DUE_TO_N384_HOST_QUALITY_FAILURE`;
- historical host counters remain `NOT_RECORDED`;
- identical input files under different checkout roots produce identical
  retrospective objects with stable profile identifiers.

## Review history and closure

The first PR #10 review found duplicate/fabricated promotion,
interrupted-layout exceptions/NaN, and checkout-dependent retrospective paths.
R4 closed the latter two and introduced shallow decision validation. The second
review demonstrated that complete-looking parsed-row stubs and unrelated files
could still create false distinctness. The final closure:

1. enforces the exact custody layout;
2. binds summaries to a full fresh campaign replay;
3. introduces a primary-evidence canonical digest;
4. rejects duplicate paths, tree hashes, and evidence hashes;
5. normalizes or recomputes redundant representations;
6. adds load-bearing regressions for all known bypasses.

Fresh review of the final published head remains required before merge.

## Residual operational limits

- `/proc/stat` negative deltas, including non-monotone `iowait`, fail closed;
- capture currently requires an adjacent `target/measurement` binary path;
- custody is content-based and appropriate to the current single-user research
  workflow; this is not a cryptographic multi-party attestation claim.

## Claim boundary

This receipt verifies validator infrastructure only. It is not a wall campaign,
a timing promotion, a speedup result, active-switch authorization, a release,
or merge authorization.

## Files

| path | bytes | SHA-256 |
|---|---:|---|
| `scripts/timing_authority_validator.py` | 59150 | `0da1ff464823bc3150536eb680d8673a3cbcaee10d662dd9eafa63ac58c3249b` |
| `scripts/test_timing_authority_validator.py` | 50566 | `c340a0bbf643ad5927983d4a551828e2800f759b32d8f219a0c1f34baf67140c` |
| `scripts/run_timing_authority_validator_selftest.sh` | 1615 | `cc665aaf74aea83667b8854b65bf54154dad55690ecd07863d5989e78b4854ab` |
| `results/V36_RETROSPECTIVE_TIMING_QUALITY_DIAGNOSTIC.json` | 4312 | `4d3bb8d1f91416868f95d30846f2ec7eadcaeeabdd8fa1c39ef76936f48f4794` |
| `reports/TIMING_AUTHORITY_VALIDATOR_RESULT.md` | 4121 | `3953f4084abbf51ff298f6c2d78264110c3e253fed82699909d9225f4278427f` |
| `reports/TIMING_AUTHORITY_VALIDATOR_PHYS_MATH_CODE_AUDIT.md` | 7623 | `eeb0adaa8f5cd9fb7044c539c06abf47292879bab50876a229ba6a645bb093c1` |
