# Final verification receipt

## Rust

- `cargo fmt --all -- --check`: pass
- `cargo clippy -p rodas5p-core -p rodas5p-integrators -p rodas5p-cli --all-targets -- -D warnings`: pass
- focused core work-counter contracts: 2/2 pass
- accounted prefix/continuation contracts: 5/5 pass
- frozen full-E shadow contracts: 3/3 pass
- enforced v3.5 prefix-budget contracts: 3/3 pass
- retained level-2 compatibility contract: 1/1 pass
- CLI contracts: 2/2 pass
- optimized ignored all-six paired-wall contract under `--profile measurement`: 1/1 pass

## Python and campaigns

- preflight analyzer/verifier unit tests: pass
- runtime/economics verifier unit tests: pass
- 30-shard optimized runtime campaign: complete
- v3.5-to-v3.6 127-row runtime verification: pass
- preflight-to-runtime 64-event verification: pass
- five-profile optimized paired-wall campaign: complete
- structural economics analysis: pass
- all 35 measured pairs retained; no timing exclusions

The exact commands are encoded in the scripts under `research/generic_frozen_full_e_shadow_v36/scripts`.
