# Production hardening: observations, physical error and honest cost

Registered 2026-10-23 before new numerical or symbolic experiments. Baseline commit: `fbc3c272fa06b443de148631ea78bfa6a34f1b2b`, branch `research/nonstiff-explicit-20261023`. New work is isolated in `research/production-hardening-20261023`; NCP is excluded.

## Questions and scope

1. Can a separately selected DOPRI5 quartic continuous extension remove forced observation landings while preserving finite, ordered, accepted-step-only observations and the published fifth-order endpoint method?
2. Can reduced-coordinate integrations control the embedded error in a caller-declared affine physical state `v=d+L u`, with full-space scales and reusable memory?
3. Can redundant facade memory passes be removed without weakening callback failure detection, and can representable extreme-step arithmetic be made more robust?
4. What do the new diffsol/Julia external comparisons actually establish about production suitability? Audit published raw identity and analysis; do not rerun unchanged experiments.

## Methods, inputs and commands

Primary input paths: `crates/rodas5p-integrators/src/dopri5.rs`, `crates/vigilode/src/explicit.rs`, their tests, and the latest nonstiff external audit directories. Retain the published tableau and FSAL proofs unchanged. New files will contain dense interpolation, affine error mapping, regression contracts, and a local paired benchmark runner. Freeze implemented source in a published commit before authoritative measurements. Preserve failed development runs separately.

Target validation commands (test targets will be made concrete in the frozen source):

```sh
cargo test --locked --offline -p rodas5p-integrators --test dopri5_contracts --test dopri5_allocation --test dopri5_dense_contracts --test affine_error_contracts
cargo test --locked --offline -p vigilode --test explicit_contracts --test explicit_ensemble_contracts --test explicit_hardening_contracts
cargo build --locked --offline --profile measurement -p vigilode --example production_hardening_bench
python3 research/production_hardening_20261023/run_bench.py --help
```

Use Rust 1.94.1, locked dependencies, at most two build jobs; record actual environment, source tree, commands, binary hashes, exit codes and raw outputs. Measurement profile is the repository's existing opt3/no-LTO profile. No external package-version substitutions.

## Acceptance, contrary evidence and holdouts

Dense extension: exact rational continuous-order conditions through order four, endpoint equality, nonlinear/nonautonomous convergence, correct interior observations, preserved partial results and invalid-input rejection. At fixed endpoint/tolerance settings, changing only interior observation density must leave dense-mode RHS/accept/reject counts unchanged. Quartic interpolation has local O(h^5) error; no automatic fifth-order global continuous-output assertion or uniform tolerance guarantee.

Physical norm: validate shape, finiteness, tolerances, full old/new state scaling and lifted embedded error; demonstrate a reduced WRMS counterexample, equivalence to a directly computed full norm, zero-scale behavior, and no hot-loop allocation in the reusable evaluator. Dense physical scaling must be explicitly documented; it does not certify global physical error.

Memory/robustness: preserve all existing callback contracts and partial results; adversarial subnormal-step test, reject nonrepresentable arithmetic honestly. A memory-pass reduction is a structural result. Claim elapsed-time improvement only from paired same-host, same-case measurements at comparable achieved error; retain all losses/noisy cases.

Benchmark: analytic decay, nonlinear logistic, driven nonautonomous, rapidly oscillating/damped pairs, nonnormal and large diagonal vectors; sparse versus dense observations; at least three tolerances and dimensions 1/2, 4096 and 16384 where practical. One warmup and at least three timed repetitions. Independently recompute all saved analytic errors and expose RHS/attempt counts. Holdout includes a nonlinear trajectory and oscillatory case not used to tune implementation. Bound campaign to 15 minutes excluding compilation, preserve NOT_RUN/timeout cases. Existing VODE/diffsol/Julia evidence remains separately attributed and is not pooled across hosts as a new speed claim.

## Policy interpretation

The user's explicit theory-plus-code request authorizes symbolic checks needed to construct the new interpolant/norm before the numerical holdout; this narrowly overrides the ledger document's default order of formal obligations. Publication and targeted local execution are authorized. No strong no-go claim is proposed. Existing research tasks remain open unless their own obligations are actually met.

## Results

To be appended after execution; see source-specific evidence and report.
