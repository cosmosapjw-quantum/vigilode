# Krylov math and allocation result cross-review

Reviewer: `/root/krylov_theory`, 2026-10-29. This review reads existing source, receipts and raw result bytes. It executes no solver, benchmark, symbolic campaign or new scientific test. JSON aggregation below is a readback calculation of already recorded data. Prior independent proof/source review was performed by root, state_audit and proof_audit; this result cross-review is by the theory/implementation author and is not represented as a fourth independent source audit.

## Identity and tests

Current tested source is `ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc`, tree `2989f07fb04134aa7bab38e38d7272e45d6a0917`. The baseline allocation executable was built at `c41caffa68d9f4eb9d1976422f7f74ec7e764cca`; `git diff bc5ddbe..c41caffa -- crates Cargo.lock` is empty. Native source, compiler, lockfile, binary and build receipts are separately bound in `lane-bench02/RUN.json`.

- `lane-checks02/proof-oracles`: 25 research-oracle methods pass. Nine are the exact directional-quotient tests; the full25 includes a Decimal diagnostic, so “25 exact general proofs” is not an appropriate description.
- `lane-checks02/libraries`: Krylov17, facade37, coupled45 pass; one coupled benchmark stays ignored. The K1 poisoned augmented-reuse method is contained in Krylov17.
- `lane-checks02/krylov-contracts`: 29 binaries,115 passed,1 failed. All three new K1 integration methods pass. Existing failure/output-commit/progress regression methods also pass.
- The one failure is the inherited `sp02_workspace_charge` LGMRES retained-charge assertion,2304>640. The source uses one process-global allocator counter without test-level exclusion. `lane-charge03` reruns that same source and threshold with `--test-threads=1`:3/3 pass. This supports a parallel harness interference diagnosis; it does not turn the original exit101 into exit0 or prove all allocator parallelism safe.
- Scoped Clippy for facade/Krylov all targets passes.

The first source `7fce5a4cbf5e1de3dd25de3b247f638bc551be24` failed to compile the authored recording wrapper because it omitted `LinearOperator::token`. Episode2 delegates token and exact identity to immutable `DenseOperator`. Its failure receipt remains in `lane-checks01`; K1 numerical source was not changed by this fix. The separate build-cache provenance correction and Decimal diagnostic correction belong to the parent campaign and remain in its correction ledger.

## Mathematics and what the tests support

K1 changes only preparation of unread direction vectors. Its proof depends on the actual `gmres_into` basis-only reads; the tests now confirm cold/reserved output bits, callback words, mixed legacy/into dimension histories, monitored prefixes and actual augmented reuse after poisoned stale storage. No MGS, QR, original residual, stopping or physical tolerance rule was weakened.

K3 retains the exact identity

\[
v-(I-\sigma J)Q_\sigma(J)v/D(\sigma)
=\sigma^d p(J)v/D(\sigma),
\]

including σ=0 and explicit D=0 refusal. The nine exact tests independently solve original systems by rational Gaussian elimination and check the residual/error bound, near-structure, a representation pole in a nonsingular system, nonnormal coercivity refusal, cap failure and a sign mutation. A missing cap returns `admitted=None`, so available error bounds are not silently converted into requested-accuracy admission. The Python reference is opt-in; a native directed-rounding/provider implementation remains an explicit local obligation.

## Raw returned parity and allocation observations

I independently loaded each baseline/candidate `stdout.txt` for four cases × two modes, removed only `solve_allocations` from each sample, and compared every other sample field exactly. All24 sample pairs match; all eight case/mode callback-total pairs match. This includes serialized Stats and returned observation/state/time bits. The recorded endpoint analytic errors stay at the same values, with maximum4.4617096683907675e-9. These are returned-checkpoint diagnostics; they do not establish an all-interior trajectory enclosure or a rigorous uncertainty bound for the binary64 analytic reference.

| Case | Baseline allocations, repeats1/2/3 | Candidate | Difference each repeat |
|---|---|---|---|
| dense-fd |324,68,68|315,59,59|9|
| banded-fd |318,62,62|314,58,58|4|
| time-jvp |386,129,129|358,101,101|28|
| autonomous-jvp |389,133,133|361,105,105|28|

The first repeat differs substantially from later repeats, so a single allocation number must be labelled by repeat/setup state. These whole-solve observations combine K1 and facade output ownership changes. The table alone does not isolate all28 JVP allocations as K1. The K1 theorem independently predicts removal of m unused n-vector allocations on a cold direction pool, plus any allocator-specific outer-vector growth; resource reuse and output ownership explain different parts of the combined result. No full hot-loop zero-allocation claim follows.

## All timing blocks retained

Each recorded ratio is the sum of the two baseline child medians divided by the sum of the two candidate child medians inside one ABBA/BAAB block. I recomputed all28 ratios exactly from the JSON numeric values and they equal the recorded ratios. For each workload all seven blocks are retained:

| Case | Median baseline/candidate | Min–max | Blocks below1 |
|---|---:|---:|---:|
| dense-fd |0.965492|0.938935–1.036816|5/7|
| banded-fd |0.964152|0.890093–0.980125|7/7|
| time-jvp |1.014862|0.982244–1.118594|2/7|
| autonomous-jvp |1.010487|0.977919–1.035212|2/7|

The allocation reduction is observed. Broad wall-time improvement is not established: dense/banded medians regress, all banded blocks regress, and both JVP cases retain adverse blocks. The next local experiment should separate K1-only, A1-only and combined changes and include A/A control, warm/cold setup, every repeated result, and complete solve-plus-drop accounting. This recommendation does not authorize another cloud correction episode beyond the user’s three-episode cap.

## Receipt identities and verdict

| Key | SHA256 |
|---|---|
|lane-checks01/RUN.json|df05ccc45316e0a4d3763ed8318eb0b533e00e6b34c2fb3aba53a3ff87886a67|
|lane-checks02/RUN.json|70004fdf605436b740ab5f7796799b4e1977aa48d805bde487ef0da5b93d75ff|
|lane-charge03/RUN.json|cc25bb93a933e5577442e826bedd2975b5133625a1b2958f7ccce19d55375854|
|lane-bench02/PAIRS.json|248e9a2eba82ecb75fc03acf579f1a2e9abb5188c04cf687709746b431a2f957|
|lane-bench02/TIMINGS.json|6413cfabf68906c376a16d3dd1d754e051868df03b581ee296ff9f99cecef3a5|

**ACCEPT_SCOPED_PUBLICATION** for K1’s storage/callback equivalence evidence, K3’s exact reference implementation and the stated allocation observations. Timing remains diagnostic with regressions. Parallel allocator-test invocation, native K3 certification, full physical clients and parent production gates remain open. Broad production stays **HOLD**.
