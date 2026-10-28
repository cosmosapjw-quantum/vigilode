# Coupled snapshot transfer implementation

Status: implementation prepared after parent C9 prepublication review; native execution awaits parent publication GO.
Source baseline: `87465835d9d21298d235917f036580b0b53ab645`.
Owned production path: `crates/vigilode-coupled/src/switching.rs`.

## Change

Successful guarded protected-reference updates now swap `baseline_state` and `pending_state`. `prepare` already copied the exact start-state snapshot; promotion therefore copies zero coordinates. Receipt accounting retains that first copy and removes the second copy. The previous vector capacity becomes next-attempt scratch. All mode guards, adaptive acceptance, cost scores and cooldown rules remain the baseline implementation.

The R1–R10 proof is in `theory_dynamics.md`. `adaptive.rs` was inspected to verify one prepare before each observed attempt and one observe before continue/break/next iteration. PR-N04 owns this bounded buffer-reuse leaf; its initial implementation does not reset or complete the inherited task. R-ADAPT remains 3/3 and PR-N06 remains pending.

## Test and comparator design

The test-only `LegacySwitchState` wraps unchanged prepare/skip logic and contains the original baseline observe body. A static text comparison, after removing the wrapper's `.0` field projection and restoring method visibility, matched the original observe body exactly. SHA-256 of that original method is `e863f7380cb5d4acfc364c5968abb6ae24cd2d967e3c850d6ae1b2f538af0288`.

`guarded_symbolic_words_preserve_legacy_routing_and_snapshot_bits` enumerates 6^5 outcome words in each of two guard regimes (15,552 encoded words; terminal symbols terminate the legal prefix). It compares all serialized routing receipt fields except deliberately changed copy telemetry, exact reference state bits, prepared state bits and skip decisions. The perturbed regime includes epoch changes, dimension changes, step-ratio changes, state displacement and signed zero. Outcomes include protected acceptance/retry, candidate-only acceptance/local rejection, failed candidates with fallback, and terminal failure.

`promoted_snapshot_survives_scratch_reuse_and_rejected_attempts` checks the promoted signed-zero snapshot survives subsequent scratch overwrite and rejected/candidate-only attempts, then exercises invalidation and dimension growth.

The ignored test `benchmark_snapshot_transfer_against_legacy` times complete prepare+observe pairs, including allocations of both reference buffers and routing receipt work. It uses dimensions 2,32,4096,65536; guarded protected updates, candidate-only continuation and unguarded controls; seven paired blocks with deterministic randomized order. One warmup per arm/case (24 total runs) is omitted from measurement rows; measured runs still begin with empty reference buffers so initial allocation cost is included. Every pair checks complete routing-word and final-state bit parity. The raw receipt retains each timing, inspected/copied count and exact saved-coordinate count. Output uses create-new semantics and refuses to replace existing evidence. Work receipts are synthetic and no RHS or ODE solve is performed, so timing is a transducer microbenchmark.

Parent C9 prepublication correction episode: inherited PR-N04 moves from historical 0/3 to 1/3 for adding preregistered warmups and create-only evidence writing. No native failure occurred. The existing dimension-two copied-counter assertion covers a candidate-only acceptance and remains 2 because only prepare copies that state.

## Commands after publication GO

```sh
cargo test --offline -p vigilode-coupled --lib switching::tests -- --nocapture
VIGILODE_SWITCH_BENCH_OUT=/absolute/evidence/switch_benchmark.json cargo test --offline --release -p vigilode-coupled --lib switching::tests::benchmark_snapshot_transfer_against_legacy -- --ignored --exact --nocapture
```

Use the parent's locked Rust environment and registered evidence paths. Formatting/build/targeted native tests and benchmark are not yet executed by this subagent. The parent should publish the complete source tree before issuing GO, retain initial failures, and charge actual corrective episodes to inherited PR-N04.
