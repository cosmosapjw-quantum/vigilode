# Coupled matrix-free stage adoption

Baseline: 8878e55a54e3a5f99fa81d6d3b08e05c94ed3db2. User-provided prior ideas are source inputs, not newly reproduced performance evidence. NCP use is prohibited. Use a bounded local Rayon pool and two compiler jobs.

## Question and implementation

Connect JVP-only shared-basis projection, simultaneous stage RHS/derivatives, affine predictor, endpoint-first adaptive homotopy, original raw-U residual assessment, explicit work accounting and transactional sequential fallback in a native opt-in solver. Preserve existing default methods. Scope is autonomous identity-mass ODEs with a user JVP; unsupported mass/nonautonomous inputs are typed refusals. Add an actual adaptive integration entry point using existing clock/output/controller semantics.

## Acceptance before executing new tests

1. At equal h, raw-U stage/output parity against an independently assembled sequential raw target on affine, nonlinear, nonnormal and oscillatory fixtures; state WRMS <= 0.05 for normal fixtures, and full-target residual criterion always met for admitted candidates.
2. A low-dimensional invariant subspace fixture must use no full-space sequential W solve in the candidate; all JVP/RHS/basis/validation work remains counted. Verify actual multi-worker overlap separately from timing.
3. Wide-spectrum and singular projected systems must expand or fail cleanly to same-target fallback; never accept projected residual alone. Invalid input, callback errors, partial parallel work, rejection and changed-operator identity must preserve committed state.
4. A smooth nonlinear exact solution must show expected temporal refinement before roundoff; compare full trajectories and error/worker configurations. Existing unchanged evidence is reused.
5. Benchmark candidate versus its raw sequential baseline at equal achieved accuracy on this sandbox only. Record warm-up, repeats, dimensions, basis, work and elapsed time; choose no universal speed threshold and report losses.
6. Independent review of the changed algorithm and tests; correct concrete findings once, with affected retests only.

The original 30-case Python probes and claimed 12,288-variable timings are retained as user-supplied historical descriptions until their raw package is available; this run will create native evidence independently. No new general-production or global-error certificate follows from scoped tests.

## Intake refinement before new executions

Full inspection of the 35-page supplied report and current source found the shared-basis/homotopy kernel already integrated in `vigilode-coupled` (including residual-first evaluation). Reuse that implementation and its existing evidence. The new implementation is the raw-U fallback and conditional causal bound adapter, a reusable adaptive output session, and an observed coupled trajectory driver. The raw-U target stays unchanged; no native-K substitution is permitted in the new fallback.

Commands (run with the restored Rust toolchain, `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_TEST_DEBUG=0`, locked offline dependencies):

```sh
cargo test --locked --offline -p vigilode-coupled
cargo test --locked --offline -p rodas5p-integrators adaptive_output_session
cargo run --locked --offline --release -p vigilode-coupled --example adaptive_bench
```

The benchmark runner records one warm-up and three measured repetitions at one and two local workers; no NCP execution is permitted. Accuracy and counters are authoritative for the tested fixtures; sandbox timing is exploratory. Holdouts are a nonnormal linear problem, a rapidly oscillating problem, and a broad-spectrum forced-fallback case, separate from the smooth scalar/low-subspace construction fixtures. No post-result numerical threshold adjustment is allowed without retaining the failed result and recording the change. Existing kernel contract tests are regression protection, not newly published kernel performance claims. Conditional bounds carry explicit inverse/Lipschitz/domain assumptions; no caller-supplied bound is presented as an independently verified certificate.

## Structured reduced solve extension (before execution)

The mathematical review identifies a second implementation opportunity in the reused kernel: the reduced affine/Newton matrices are block lower triangular with one common diagonal W_m. Replace a full (8m)-order LU by one m-order LU and block forward substitution, retaining the same raw-stage target. Register exact algebraic comparison against independent full dense solves (including nonnormal off-diagonal blocks and singular W_m), existing coupled regressions, and a bounded m sweep timing/operation comparison if useful. The arithmetic complexity reduction is conditional on this verified structure; elapsed speedup is measured separately. Existing work fields retain documented meanings and any new block-factor/solve fields count real operations.

## Build-profile refinement before benchmark execution

Repository inspection shows `release` uses fat LTO and a single codegen unit. Use the repository's existing `measurement` profile (`opt-level=3`, `lto=false`, `codegen-units=16`) for the sandbox campaign to bound linking resources. All compared methods are in the same executable with these identical flags. Replace the benchmark command above with `cargo build --locked --offline --profile measurement -p vigilode-coupled --example adaptive_bench`, then run the produced binary using `run_local_bench.py`. No benchmark output was observed before this choice.

## Observed results (appended after execution)

Implementation source e419f920f0b8d0b123af03df5e802e8fc612a75d: 62 targeted Rust tests passed. Original failed logs are preserved. Benchmark source 53920cc617c317927fcabe90f6b66f1a9fbf79bd: 51 parsed configurations, 51 with all warm-up/repetitions successful, 0 numerical failures. See DECISIONS.json and evidence/local_bench/SUMMARY.json for actual errors, failures, work and refinement slopes. Timing remains exploratory. No NCP run or general production promotion occurred.
