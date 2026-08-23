# PM-3 Timing Authority Validator Verification Receipt

## STATUS

`CORRECTIVE_IMPLEMENTATION_READY_PENDING_REMOTE_REVIEW`

## Authority anchors

- canonical remote parent: `main@6a10345e5a8b2fd77a74642aedea0c68ee0041fb`
- canonical remote parent tree: `0a98b761c917670318e2213b81fe77f3fbe08d0c`
- scientific implementation baseline: `4384eab8397b20903377705a99b4db6e8370e2bd`
- scientific implementation tree: `4a82e7b9196c383fdd9a9cae5ba566035ea420e0`
- local PM-3 implementation commit: `68b1c2a39831d81700f732b304816a88679c1454`
- local PM-3 implementation tree: `b127ac8028f0fc5dfab276ae8a9a75d34a16a48b`
- local independent-audit commit: `72c923c89db8b740bcc1d60a86c9a181e262ef52`
- local independent-audit tree: `219c00da82b68a8e93dcd1e6607e8e70e3e243ec`
- first published PR #10 head: `02ae95350777939bce4208488581bcb073cc45a1`
- first published PR #10 tree: `17d5fbd760c6b16a4ca39f2f88a291fc72b5e5e8`

PR #7 and PR #8 changed design/plan documents only. The local executable source
used for this verification is the exact merged v3.7 scientific implementation
tree; PM-3 itself modifies only the v3.7 research validator surface.

## Verified commands

```text
python -m unittest test_timing_authority_validator -v
  39/39 PASS

python research/generic_frozen_full_e_shadow_v36/scripts/test_analyze_shadow_economics.py
  2/2 PASS

python research/generic_frozen_full_e_shadow_v36/scripts/test_analyze_full_e_shadow_ledger.py
  3/3 PASS

bash research/generic_timing_replication_continuation_transaction_v37/scripts/run_timing_authority_validator_selftest.sh
  PASS; deterministic retrospective output; no real timing attempt directory

cargo fmt --all -- --check
  PASS under Rust 1.94.1
```

Toolchain:

```text
rustc 1.94.1 (e408947bf 2026-03-25)
cargo 1.94.1 (29ea6fb6a 2026-03-24)
rustfmt 1.8.0-stable
```

## Behavioral verification

- exact sealed contract thresholds loaded fail-closed;
- valid synthetic campaign retains 5 profiles, 5 warm-ups, 35 measured pairs;
- missing tail or middle rows yield whole-campaign failure while retaining all
  five profile files and every structurally readable row actually present;
- zero-row, one-row, duplicate-index, and unexpected-index attempts produce
  serializable NON_AUTHORITY decisions with named index/metric failures;
- proposed-interval mismatch remains a retained quality failure;
- favorable ratio direction cannot change host-quality verdict;
- dirty tree, contract-hash mismatch, low idle, excess steal, swap delta,
  thermal-throttle delta, R-JF span, shadow span, and order gap each fail closed;
- missing thermal counters alone do not fail;
- Git, Rust, binary, host, affinity, and thread-environment mismatches are named
  independently across attempts;
- only 3 distinct structurally complete passing campaigns within at most 5
  attempts promote descriptive timing;
- duplicate campaign paths/hashes and incomplete fabricated PASS-shaped
  decisions are rejected before counting;
- 2/5 passing campaigns yield host-unsuitable/no-promotion;
- six attempts are rejected;
- speedup and active switching authorization remain false.

## Retrospective result

- all 35 historical v3.6 measured pairs retained;
- N=384 R-JF span: `13.82492944330828`;
- N=384 shadow span: `59.214273363533934`;
- N=384 order-median absolute gap: `0.6857626375408328`;
- historical `PASS_DESCRIPTIVE_ECONOMICS` is not rewritten;
- retrospective contract diagnostic: `WHOLE_V36_CAMPAIGN_NON_AUTHORITY_DUE_TO_N384_HOST_QUALITY_FAILURE`;
- historical host counters are explicitly `NOT_RECORDED`.
- identical input files under different checkout roots produce identical
  retrospective objects with stable relative profile identifiers.

## Independent checklist review and bounded correction

The first remote PR #10 review exposed two P1 authority defects and one P2
reproducibility defect:

1. one campaign or a minimal PASS-shaped object could be counted repeatedly;
2. several interrupted pair layouts raised or emitted non-finite metrics instead
   of a durable whole-campaign decision;
3. the retrospective JSON embedded a transient checkout path.

The bounded correction adds full decision-shape/invariant checks, distinct path
and campaign-tree hash enforcement, retained index-set diagnostics, nullable
unavailable metrics, and stable relative retrospective identifiers. The new
adversarial tests were observed failing before these changes and pass afterward.
Fresh remote review of the corrective commit is still required before merge.

P3 operational assumption: capture accepts only a binary whose path contains an
adjacent `target/measurement` profile directory. This is intentionally
fail-closed and may be generalized only with a new explicit attestation source.

## Claim boundary

This receipt verifies the validator implementation only. It is not a real wall
campaign, a timing promotion, a speedup result, active-switch authorization, or
release-wide completeness.

## Files

| path | bytes | SHA-256 |
|---|---:|---|
| `scripts/timing_authority_validator.py` | 58952 | `3a9fd61ee3cee8ef2a7be729d4ded341185e3cb459bbd9f22e3dbbbbf86d6810` |
| `scripts/test_timing_authority_validator.py` | 34869 | `f6404ce730ef73dbd9d69e83c68d5dd3af6ee556354c03b9cfaa0bbce134d5ef` |
| `scripts/run_timing_authority_validator_selftest.sh` | 1615 | `cc665aaf74aea83667b8854b65bf54154dad55690ecd07863d5989e78b4854ab` |
| `results/V36_RETROSPECTIVE_TIMING_QUALITY_DIAGNOSTIC.json` | 4312 | `4d3bb8d1f91416868f95d30846f2ec7eadcaeeabdd8fa1c39ef76936f48f4794` |
| `reports/TIMING_AUTHORITY_VALIDATOR_RESULT.md` | 2719 | `3398dcd2240fd00a3715d3a6aa44e6f1921220cee33c0c05e3df0cbcb22463ca` |
| `reports/TIMING_AUTHORITY_VALIDATOR_PHYS_MATH_CODE_AUDIT.md` | 5329 | `4278e73aeb016e8a38e49c9efa2cf1eff451d99760fb552a1edf334188d9accf` |
