# PM-3 Timing Authority Validator Verification Receipt

## STATUS

`COMPLETED_NOT_YET_REMOTE_REVIEWED`

## Authority anchors

- canonical remote parent: `main@6a10345e5a8b2fd77a74642aedea0c68ee0041fb`
- canonical remote parent tree: `0a98b761c917670318e2213b81fe77f3fbe08d0c`
- scientific implementation baseline: `4384eab8397b20903377705a99b4db6e8370e2bd`
- scientific implementation tree: `4a82e7b9196c383fdd9a9cae5ba566035ea420e0`
- local PM-3 implementation commit: `68b1c2a39831d81700f732b304816a88679c1454`
- local PM-3 implementation tree: `b127ac8028f0fc5dfab276ae8a9a75d34a16a48b`
- local independent-audit commit: `72c923c89db8b740bcc1d60a86c9a181e262ef52`
- local independent-audit tree: `219c00da82b68a8e93dcd1e6607e8e70e3e243ec`

PR #7 and PR #8 changed design/plan documents only. The local executable source
used for this verification is the exact merged v3.7 scientific implementation
tree; PM-3 itself modifies only the v3.7 research validator surface.

## Verified commands

```text
python -m unittest test_timing_authority_validator -v
  29/29 PASS

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
- missing measured row yields whole-campaign failure while retaining all five
  profile files and the 34 rows actually present;
- proposed-interval mismatch remains a retained quality failure;
- favorable ratio direction cannot change host-quality verdict;
- dirty tree, contract-hash mismatch, low idle, excess steal, swap delta,
  thermal-throttle delta, R-JF span, shadow span, and order gap each fail closed;
- missing thermal counters alone do not fail;
- Git, Rust, binary, host, affinity, and thread-environment mismatches are named
  independently across attempts;
- 3 passing campaigns within at most 5 attempts promote descriptive timing only;
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

## Independent checklist review

No P0/P1 defect remains after two corrections:

1. structurally parseable pair-cardinality and proposed-interval defects were
   changed from exception-only behavior to retained whole-campaign failures;
2. cross-campaign identity failures were split into the sealed named categories
   instead of one opaque aggregate label.

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
| `scripts/timing_authority_validator.py` | 44658 | `2df4d10861ae301cb2f8ce69f357d46b5d2d53bb1476795f4aab6bb0df0e9bf7` |
| `scripts/test_timing_authority_validator.py` | 24855 | `0af8e15a8d845de38a2d1ee0d028c7805821afde5ef54be8b3cf426719e13cea` |
| `scripts/run_timing_authority_validator_selftest.sh` | 1615 | `cc665aaf74aea83667b8854b65bf54154dad55690ecd07863d5989e78b4854ab` |
| `results/V36_RETROSPECTIVE_TIMING_QUALITY_DIAGNOSTIC.json` | 4752 | `5cc28824b56b57432ff647b6de1601dcdae7776dafe27614ecadddaa2f34bab7` |
| `reports/TIMING_AUTHORITY_VALIDATOR_RESULT.md` | 2309 | `8c2c0b8014f1ac482ba40f85438aba087376238a094d6da421be79eef8136b68` |
| `reports/TIMING_AUTHORITY_VALIDATOR_PHYS_MATH_CODE_AUDIT.md` | 4098 | `4dc7d9884605fb9888a26833dcdd6a18f33907dced2529cbf0447ef162ba942d` |
