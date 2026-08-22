# VigilODE v3.7 Continuation Transaction — Final Verification

## Source state

- base: merged contract authority `main@e493d21f9a9c89cae995b44e0847db4b4292f421`;
- implementation branch: `research/v37-continuation-transaction-implementation`;
- Rust/Cargo: 1.94.1;
- Cargo: locked offline vendor;
- paired-wall timing campaign: not run;
- N=2048: not run.

## Focused Rust verification

- `cargo fmt --all -- --check`: pass;
- focused Clippy with `-D warnings`: pass;
- atomic/bounded continuation contracts: 5/5 pass;
- accounted continuation regression: 5/5 pass;
- retained-prefix resumability: 1/1 pass;
- v3.7 completing-family contract: pass;
- optimized N=192 charged-exhaustion contract: pass;
- v3.6 full-E shadow regression: 3/3 pass;
- v3.7 CLI schema regression: 1/1 pass;
- v3.6 CLI schema regression: 2/2 pass.

## Replay verification

- verifier unit tests: 6/6 pass;
- runtime reports: 30/30;
- prefix-policy rows exact v3.6→v3.7: 127/127;
- recommendation decisions exact v3.6→v3.7: 64/64;
- completed endpoints exact excluding wall fields: 62/62;
- charged continuation-budget exhaustions: 2;
- numerical continuation failures: 0;
- unsafe recommendations: 0;
- prefix/continuation budget breaches: 0;
- replay-verification JSON reproduction: byte-exact.

## v3.6 non-regression

The following durable products reproduced byte-exactly:

1. `FULL_E_LEDGER_PREFLIGHT.json`;
2. `FULL_E_LEDGER_EVENTS.csv`;
3. `RUNTIME_SHADOW_VERIFICATION.json`;
4. `ECONOMICS_SUMMARY.json`;
5. `ECONOMICS_PROFILE_SUMMARY.csv`;
6. `PAIRED_WALL_RATIOS_ALL_PAIRS.png`;
7. `REALIZED_SPECULATIVE_JVP_FRACTION.png`.

## Verdict

`PASS_CONSUMED_CONTINUATION_TRANSACTION`

This receipt does not claim fresh safety, timing replication, speedup, active
switching, controller/cache transfer, release-wide completeness, or N=2048
coverage.

## Evidence hashes

- sealed contract JSON: `66f082aeec8c70e0ef23926d2c6f7057fb40fe280c45fd02c200be8778a6e659`;
- measurement binary used for the final replay: `a1cdb1ac8e02e49b9869fe769438ac26e67042d7daf96fc9496a0947c177d618`;
- replay-verification JSON: `2406c16bdb379992672371432a1fb7681683dd40b8402aa7983a2c67240eb657`;
- retained exact second failed-replay archive in the external closeout evidence:
  `878f07d53332a7031c8fa708a875159c412c8827d576101b0312fc03dafbd95c`.
