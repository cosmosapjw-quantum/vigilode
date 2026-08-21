# v3.6 Frozen Full-E Ledger Preflight

## Method

This preflight executed no solver.  It read all 30 durable v3.5 family shards
for N=96/192/256/320/384, derived the frozen recommendation

`prefix_succeeded && finite(zeta34) && zeta34 <= 13.39706618860016`,

joined each recommendation to its exact target R-JF attempt, and reconstructed
every work-counter component as

`continuation_work = audit_full_e_work - prefix_work`.

Before subtraction, every component was required to satisfy
`full_e_work >= prefix_work`; the reconstruction also required the exact
round-trip `prefix_work + continuation_work == full_e_work`.

## Results

| profile | recommendations | target R-JF JVP | prefix JVP | continuation JVP | continuation/R | full shadow/R |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| N96 | 12 | 1,886 | 256 | 156 | 0.0827147 | 0.2184517 |
| N192 | 13 | 3,368 | 323 | 308 | 0.0914489 | 0.1873515 |
| N256 | 11 | 2,229 | 245 | 172 | 0.0771646 | 0.1870794 |
| N320 | 13 | 2,423 | 285 | 186 | 0.0767643 | 0.1943871 |
| N384 | 15 | 3,137 | 347 | 308 | 0.0981830 | 0.2087982 |
| **all** | **64** | **13,043** | **1,456** | **1,130** | **0.0866365** | **0.1982673** |

Unsafe frozen recommendations: **0**.

Continuation JVP min/median/p95/max is `8 / 12 / 42.8 / 140`.
The two 140-JVP semilinear tails at N=192 and N=384 exceed the frozen
prefix-only cap of 80.  The v3.6 contract deliberately supplies no post-hoc
continuation cap or numeric economics threshold, so this is a required runtime
measurement target rather than an automatic rejection.

Nonzero aggregate continuation counters are:

- JVP calls/vectors: 1,130 / 1,130;
- phi actions: 128;
- phi Krylov vectors/projected exponentials/dense oracle calls: 1,130 each;
- phi restarts: 4;
- orthogonalization inner products/vector updates: 15,478 each.

## Decision

Verdict: **PASS_TO_RUNTIME_SHADOW_MEASUREMENT**.

The durable ledgers are internally complete, join exactly, and contain no
unsafe frozen recommendation.  Aggregate JVP evidence does not reproduce an
obvious cumulative tail-overhead failure, but it cannot establish wall
economics because v3.5 did not time retained-prefix continuation separately.
Proceed to the contract-frozen read-only runtime shadow and optimized paired
wall measurement.

This verdict does not authorize active switching, forced-switch recovery, a
fresh safety claim, or a speedup claim.

## Durable artifacts

- `results/FULL_E_LEDGER_PREFLIGHT.json` — SHA-256
  `eb8153f1e4ebe520ba0c6dc7846b76f9cefb1b46621f5c1901fb850093298705`
- `results/FULL_E_LEDGER_EVENTS.csv` — SHA-256
  `d06b9d2cdc3f6d0b7b727b7b4147ab644f8ef9d97ac8ad7eba1b2cce45573071`
