# v3.7 Timing Authority Validator Result

## Verdict

`VALIDATOR_IMPLEMENTED_LOCALLY_PENDING_REMOTE_REVIEW`

The validator implements the sealed v3.7 timing-authority contract without
creating any new paired-wall campaign. It validates complete five-profile
campaign directories, preserves all warm-up and measured rows, rejects only at
the whole-campaign level, and keeps timing quality independent of whether the
shadow/R-JF ratio is favorable.

## Implemented authority surface

- exact contract loader and threshold lock;
- `/proc/stat` idle/steal arithmetic over the sealed eight fields;
- Git/toolchain/binary/contract/host/affinity/thread attestation;
- swap and exposed thermal-throttle preflight deltas;
- exact five-profile file set, one warm-up and seven measured pairs per profile;
- exact paired proposed interval and `Gamma = wall / interval` validation;
- R-JF/shadow arm-span and order-sensitivity checks;
- cross-campaign Git, Rust, binary, host, affinity, and thread identity checks;
- three passing campaigns within at most five retained attempts;
- atomic JSON output;
- deterministic v3.6 retrospective diagnostic.

## Retrospective v3.6 diagnostic

The historical `PASS_DESCRIPTIVE_ECONOMICS` verdict is preserved. Under the
new timing-authority contract, the complete historical campaign is
non-authority because the N=384 profile violates both arm-span thresholds and
the order-median-gap threshold. Historical CPU idle/steal, swap, and thermal
counters were not recorded and are represented as `NOT_RECORDED`, never as a
silent pass.

| N | measured pairs | R-JF span | shadow span | order median gap | failures |
|---:|---:|---:|---:|---:|---|
| 96 | 7 | 1.150545 | 1.062651 | 0.017236 | none |
| 192 | 7 | 1.077856 | 1.032763 | 0.007673 | none |
| 256 | 7 | 1.273388 | 1.052853 | 0.026534 | none |
| 320 | 7 | 1.072802 | 1.072245 | 0.009008 | none |
| 384 | 7 | 13.824929 | 59.214273 | 0.685763 | order-median-gap, rjf-arm-span, shadow-arm-span |

All 35 measured pairs remain present. No pair or profile is excluded.

## Claim boundary

- `speedup_claim_authorized = false`
- `active_switching_authorized = false`
- no real timing attempt was generated;
- no solver dynamics, R-JF state, frozen policy, BDF/Radau path, physical-client
  path, N=2048, tag, or release was modified.
