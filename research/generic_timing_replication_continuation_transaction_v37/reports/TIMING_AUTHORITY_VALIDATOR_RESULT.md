# v3.7 Timing Authority Validator Result

## Verdict

`FINAL_AUTHORITY_EVIDENCE_CLOSURE_READY_PENDING_REMOTE_REVIEW`

The validator implements the sealed v3.7 timing-authority contract without
creating a paired-wall campaign. It validates complete five-profile campaign
directories, preserves every structurally readable retained row when an attempt
is interrupted, rejects only at the whole-campaign level, and keeps host-quality
decisions independent of whether the shadow/R-JF ratio is favorable.

The final authority-evidence closure binds every summary input back to the
campaign directory from which it was generated. A submitted decision is counted
only after a fresh `validate_campaign` replay produces the exact same nested
object. Path labels, JSON formatting, row order, unvalidated metadata notes, and
redundant derived numeric spellings cannot manufacture a distinct passing
campaign.

## Implemented authority surface

- exact contract loader and threshold lock;
- `/proc/stat` idle/steal arithmetic over the sealed eight fields;
- Git/toolchain/binary/contract/host/affinity/thread attestation;
- swap and exposed thermal-throttle preflight deltas;
- exact campaign-root layout: one regular `ATTESTATION.json`, one regular
  `profiles/` directory, and exactly the five regular profile JSON files;
- exact five-profile labels and frozen-policy identity, including the sealed
  `zeta34_tau` value;
- exact measurement-protocol metadata (`measurement` profile directory,
  `release` Cargo profile, R-JF calibration arm) and per-profile frozen
  repetition counts shared by every paired arm;
- one warm-up and seven measured pairs per profile, with exact non-boolean
  integer declared counts, positive finite wall/proposed-interval/Gamma fields,
  and exact paired-denominator checks;
- retained missing/duplicate/unexpected pair-index accounting for interrupted
  attempts, with unavailable metrics represented by JSON `null`;
- R-JF/shadow arm-span and order-sensitivity checks;
- cross-campaign Git, Rust, binary, host, affinity, and thread identity checks;
- campaign-decision schema v2 and attempt-summary schema v2;
- a canonical authority-evidence SHA-256 derived only from validated
  attestation inputs and primary pair evidence;
- canonical evidence normalization that sorts retained rows, normalizes numeric
  representation, and recomputes redundant CPU/Gamma/ratio consequences rather
  than hashing their claimed spelling;
- full summary replay from the retained campaign path, with type-sensitive
  canonical JSON equality required between the submitted and freshly
  regenerated decision;
- three distinct, freshly revalidated passing campaigns within at most five
  retained attempts;
- duplicate normalized paths, campaign-tree hashes, or canonical
  authority-evidence hashes rejected before promotion;
- atomic JSON output;
- checkout-path-independent deterministic v3.6 retrospective diagnostic.

## Retrospective v3.6 diagnostic

The historical `PASS_DESCRIPTIVE_ECONOMICS` verdict is preserved. Under the
new timing-authority contract, the complete historical campaign is
non-authority because the N=384 profile violates both arm-span thresholds and
the order-median-gap threshold. Historical CPU idle/steal, swap, and thermal
counters were not recorded and remain `NOT_RECORDED`, never a silent pass.

| N | measured pairs | R-JF span | shadow span | order median gap | failures |
|---:|---:|---:|---:|---:|---|
| 96 | 7 | 1.150545 | 1.062651 | 0.017236 | none |
| 192 | 7 | 1.077856 | 1.032763 | 0.007673 | none |
| 256 | 7 | 1.273388 | 1.052853 | 0.026534 | none |
| 320 | 7 | 1.072802 | 1.072245 | 0.009008 | none |
| 384 | 7 | 13.824929 | 59.214273 | 0.685763 | order-median-gap, rjf-arm-span, shadow-arm-span |

All 35 measured pairs remain present. No pair or profile is excluded.

## Claim boundary

- `speedup_claim_authorized = false`;
- `active_switching_authorized = false`;
- no real timing attempt was generated;
- no solver dynamics, R-JF state, frozen selector policy, BDF/Radau path,
  physical-client path, N=2048, tag, release, or merge was modified or
  authorized.
