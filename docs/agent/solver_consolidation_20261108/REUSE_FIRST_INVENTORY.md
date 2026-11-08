# Reuse-first implementation inventory

**Audit source:** `research/vigilode-c5-native-graph-schur-20261102`  
**Commit:** `569ce02629498fec133db9ce368eb303ba943b69`  
**Tree:** `a4b2f09521b197b37612ff7dc66a13fbb74a0bae`

This is the implementation authority map for the parallel Rosenbrock work. The first action for every new task is to extend, adapt or replay an existing component. Do not introduce a duplicate pool, ODE frontend, work ledger, benchmark harness, homotopy engine, Fourier algebra, interval transport or Krylov implementation.

## 1. Reuse map

| Existing component | Source identity at pinned commit | Reuse in this project | Boundary / do not claim |
|---|---|---|---|
| `ParallelExecution` | `crates/rodas5p-integrators/src/parallel.rs`, blob `ffdfb1cef258ed43586db0323eb0e7f9de27dd53` | One reusable bounded local Rayon pool; use `map_ordered` / `try_for_each_ordered_mut` for independent peer stages, RHS rows and ensemble jobs. | Do not create another thread-pool abstraction. `apply_operator_rows` performs independent operator actions, not a solve or a proof of RODAS stage independence. |
| Stage-batch feasibility harness | `src/stage_batch.rs`, blob `c2f44c327c53d7c6dfe7ba80b26cd43486d56b43`; tests `tests/stage_batch_contracts.rs`, blob `35ee8df413bab5dfb6143ceb6d475a8ff05ae8e6` | Reuse the case/report/timing schema, worker-count comparisons, numerical parity checks and concurrency probe. Add a source-backed peer-W arm to this harness. | Existing report explicitly records `strict_jacobian_free_common_w_demonstrated=false` and `explicit_simd_demonstrated=false`. Existing synthetic/independent-row feasibility does not certify actual method-stage speed. |
| DOPRI5 independent ensemble | `crates/vigilode/src/explicit_ensemble.rs`, blob `a17c533d089b92c7d3df27154711efec6d6408ed`; tests `crates/vigilode/tests/explicit_ensemble_contracts.rs`, blob `1cd7b7334f5803cca3d66b4f4b645eac1773a2f5` | Preserve per-member controller/norm, order-preserving results, partial results and failure isolation. Reuse its `ParallelExecution` scheduling and evolve the job contract toward portfolio methods. | It is currently DOPRI5-specific. Do not claim a method-agnostic stiff ensemble already exists; do not pool adaptive norms or histories. |
| Existing public problem/API | `crates/vigilode/src/lib.rs`, blob `9370f4e50e454dd43472160c8d65e4fdcd030e8a`; `src/problem.rs`, blob `f1b5ef43ec66d11585193b6be9d0ee0ce271f2ec` | Extend the existing `Problem`, Jacobian provider, options, solution and solver facade. Preserve dense/banded/JVP/finite-difference choices and callbacks. | Do not create a competing `Problem`, frontend, options type or second public facade. The current public solver is still protected-RODAS5P-centered; a new method needs a distinct method identity and capability contract. |
| Coupled raw-U research crate | `crates/vigilode-coupled/README.md`, blob `bb70ae81e71cc8f9f1434a7cb6e7f2b3edc1256f`; `src/lib.rs`, blob `a67486b946e8d2682b94349f1242c1702b555f54` | Reuse residual-first candidate/transactional commit semantics, bounded continuation, callback budgets, epoch invalidation, explicit protected fallback and separate work accounting where the new method has the same contract. | It targets original raw-U RODAS5P, uses supplied JVP for coupled correction and does not remove the original outer stage dependency. Operational stage-root success is not an ODE/global-error certificate. |
| RHS-only homotopy | `crates/vigilode-coupled/src/rhs_homotopy.rs`, blob `990103d9ab5d59b7c40f3ccd323f6ad52e504324` | Reuse `RhsOnlyMap`, lambda continuation, screened residual-dominant Anderson option, finite map-evaluation budget and model-epoch contract for source maps that satisfy this exact interface. | It requests no derivative, but the map itself may hide work. A reported f64 residual is not a rigorous certificate without callback/roundoff bounds. This is a root/corrector primitive, not a generic ODE stepper. |
| Structural mean-field reduction | `crates/vigilode-coupled/src/structural_mean_field.rs`, blob `e255e883d26fca6862033f3dcc7d93df6965a4ce` | Reuse as a template for source-owned exact reductions, full-space residual verification, outward enclosures and charged failures. | Exact only for its declared affine mean-field model. Do not generalize its two-channel closure to arbitrary nonlinear RHS. |
| Actual-K homotopy certificate | `crates/rodas5p-integrators/src/actual_k_homotopy.rs`, blob `32f0c397645b10ac7e707c4c74526166b196d485`; tests `tests/actual_k_homotopy_contracts.rs` | Reuse interval tube, all-entry alpha/beta retention, inverse/leakage witnesses and stage-output error accounting as a certificate design. | It is bound to `CollectiveModel` and certifies represented stage/output functionals, not the time-discretization truncation error. |
| Collective RODAS5P backend | `crates/rodas5p-integrators/src/collective_rodas5p.rs`, blob `45300fc235e56faa45938fa94143c1cafc96ded3`; model blob `adab07834539cdf3b93bb19de9e67e6795ec2637` | Reuse the generative `CollectiveModel`, exact source admission, rank-two/block factorization, reusable buffers, K-coordinate semantics and work ledger as a stiff MOL/structured-source comparator. | It uses the original eight-stage tableau. One common factorization is reused, but current-step stage construction remains sequential. Do not relabel it stage-parallel. |
| Nonlinear window homotopy | `crates/rodas5p-integrators/src/nonlinear_homotopy.rs`, blob `529e8b26eaebb486654e6e8e19e2cca8c740c00a`; tests `tests/nonlinear_homotopy_contracts.rs` | Reuse its source-specific exact-split candidate, independent outward packet verifier and transactional window-output contracts when the same packet/curve model applies. | This is not an arbitrary-RHS Rosenbrock solver. Candidate rounds and verified original ODE are intentionally separate. |
| Affine error transport | `crates/rodas5p-integrators/src/affine_transport.rs`, blob `1c7024c2e1c567b337f82b0ba4530a1747022f51`; tests `tests/affine_transport_contract.rs`, blob `393917d576b14488fd5f042c8ded19f6e25961d6` | Reuse outward affine-map composition and prefix bounds for causal/global error transport and stage/peer defect accounting. | The O(n)-work/O(log n)-span scan verifies arithmetic under a supplied recurrence; it does not establish that the recurrence is a valid ODE error bound or make nonlinear stage solves parallel. |
| Damped-rotation quotient/Lawson flow | `crates/rodas5p-integrators/src/quotient_flow.rs`, blob `7b76f3bd2d352504fa4c02e0fcaa2733ab1c2dce`; tests `tests/quotient_flow_contracts.rs`, blob `7d307a00423f75da177cd855d48734850fdcf47f` | Reuse the exact declared 2x2 damped-rotation semigroup action and Lawson split when the source has that block structure. | Fixed declared blocks only; no generic spectral inference and no stiffness-uniform nonlinear order claim. |
| Polynomial resolvent/chart machinery | `polynomial_resolvent.rs`, blob `f8e0ef1dc8780fda12dc18483a816be5163996e2`; `conditioned_polynomial.rs`, blob `df466c69b4b1cfadc1c62fb0c6ae10f3cb935767` | Reuse declared operator identity, chart scaling, polynomial action/refinement and outward residual infrastructure for parent-operator actions where the declarations fit. | It does not generate a Poincare-Dulac normal form or prove that the user-supplied operator declaration is true. Preserve external proof/provider identity. |
| Fixed-point continuation | `fixed_point_continuation.rs`, blob `88d90148d1a04ac9363daf6d295b453f7f3cd2af` | Reuse a source-bound F-only continuation patch when a uniform contraction, inverse action and closed-domain bounds are available. | It explicitly does not certify an ODE trajectory and is not a replacement for homotopy/RODAS stage equations. |
| Fourier-Volterra path | `fourier_path_certificate.rs`, blob `ceb9ee6df733032c6f778075bb508bf8ce91ce0b`; tests `pp05_fourier_client.rs`, blob `a60a3adeb0fb0e6dfa22b1116f6124dbe6ccded7` | Continue from the existing sparse harmonic-polynomial representation, conjugation involution, noncyclic convolution, original-target interval certificate and commit gate. | Preserve PP05 G2/G3 registered FAILs. This is one exact invariant-leaf source, not a generic Fourier solver. |
| Fourier FFT candidate | `fourier_path_candidate.rs`, blob `1ea8a4f0e15f904b230765a4e56e3a1e1c37941d`; tests `pp06_fourier_fft.rs`, blob `311b06132ba97baccfe6e9a6b41769863ed4c78e` | Reuse padded 2D FFT predictor and direct noncyclic comparison; improve only the registered harmful-alias control and timings. | FFT is untrusted predictor only. Keep original interval certificate as authority. PP06 alias gate remains failed. |
| Existing matched-error benchmark process | PR74 / PP15 / PR187 run records and `stage_batch` report schema | Extend the existing work/timing ledgers and reproducibility practices; replay archived commands before designing new workloads. | Do not compare different counter units or unmatched achieved errors. PR187 is a negative result and PP15 had no timing. |

## 2. Existing research queue to extend, not duplicate

The current `PLAN_BUNDLE.json` already defines:
- `V11-02-PROBLEM`, `V11-03-STEPPER`, `V11-04-CAPABILITIES`, `V11-05-SESSION`, `V11-08-OPERATORS`: reuse these as the public interface/lifecycle foundation. Do not design another problem/session/operator API for peer-W.
- `V11-10-ROK`: keep genuine Rosenbrock-K/BOROK separate from peer-W. Share operator/Krylov infrastructure only after preserving distinct method equations.
- `V11-11-SPECIALIZED`: extend its classification of coupled/causal/homotopy and support layers with the algebraic lift and Fourier client.
- `V11-13-SPATIAL`: extend for spatial CPU kernels. Gray-Scott 2D already exists as a reaction-diffusion/logarithmic-norm regression source.
- `V11-14-PARALLEL`: extend its block/multivector kernels and stage-batch feasibility report; no duplicate executor or synthetic-only parallel framework.
- `V11-15-BENCH`: extend the existing benchmark work ledger and matched-error corpus; no second benchmark standard.
- `V11-16-CI`: reuse current clean-clone/provenance gate, preserving existing blockers.

These are planned work items in the portfolio plan, not assertions that every plugin/API task is already implemented.

## 3. Minimal genuinely new work

Only these components currently appear absent from the pinned source tree and therefore justify greenfield scientific code after the reuse gate:

1. **Peer-W method mathematics and stage workspace:** exact coefficient table, variable-step transformations, startup/restart, estimator, method-specific plugin, peer history and independent peer-stage assembly. Reuse `ParallelExecution`, `LinearOperator`, `LinearSolverConfig`, existing LU/Krylov/preconditioner and the stage-batch report.
2. **Resonance/parent-lift generator:** exact resonance enumeration, homological equation/near-resonance refusal, finite resonant basis, parent-matrix construction and invariant-ideal verification. Reuse polynomial-resolvent/chart, fixed-point continuation, interval arithmetic and error transport where their contracts apply.
3. **Harmful Fourier alias adversarial case and PP05 parity repair:** modify existing PP05/PP06 source/tests. Do not write another Fourier implementation.
4. **Method-agnostic ensemble adapter:** extend existing `Dopri5Ensemble` scheduling and `ParallelExecution`; preserve the DOPRI wrapper.
5. **Missing spatial operator implementations:** extend existing `Problem`/operator contracts and Gray-Scott 2D pattern for 3D and sphere backends. Do not create another ODE frontend.

## 4. Mandatory reuse-first execution order

1. Re-pin branch/commit/tree and read the blob identities above. If any differ, regenerate this inventory before editing.
2. Reproduce the smallest existing contract tests and archived benchmark rows relevant to the node. Classify any non-reproduction as source drift, environment failure, runtime failure or scientific discrepancy.
3. Write a reuse decision record: existing component, exact functions/types reused, required extension, and why an extension cannot be done by composition.
4. Implement the smallest delta in the existing crate/module. A new crate is allowed only when the existing crate boundary demonstrably prevents reuse and the reason is recorded.
5. Reuse `ParallelExecution` and the existing WorkLedger/report structures; never add a second pool or independent counter taxonomy.
6. Benchmark the actual source-backed method using the existing report/corpus before introducing synthetic workloads.
7. Preserve all previous negative findings and append new results under a new identity.

**Hard stop:** If a task cannot explain which existing module/test/receipt it reuses, it is not ready for implementation. GPU is forbidden for all paths and fallbacks.
