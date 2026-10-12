# Expensive-client contract for windowed homotopy parallelism (HT01)

Contract id: `vigilode-homotopy-expensive-client-v1`. Registered by research node HT01
(`research/ht01_homotopy_client_contract_20261011/PREREGISTRATION.md`); derived from the October 10 re-audit
(`docs/reviews/20261010_accuracy_speed/MATHEMATICS_PORTING_KO.md`, section 7). This document is a design contract.
It changes no solver, it is not a claim that parallel homotopy helps any client, and it promotes nothing to a default.
HT02 (`research/ht02_homotopy_parallel_pilot_20261011/PREREGISTRATION.md`) may build against it only after HT01
selects a client.

## 1. Frozen target and output authority

The K-form stage target of one RODAS5P attempt is fixed before any candidate is built:

    H_{theta,lambda}(K) = (D - eta C) K - b0 - lambda h N(K),   eta = theta + lambda (1 - theta),

with `D = I_s (x) W`, `W = I - h gamma J`, `C` the strictly lower stage coupling, `b0` the base right-hand side and
`N` the nonlinear remainder (`homotopy.rs::evaluate_partial_path`). The only output authority is the original
lambda = 1 target

    (D - C) K - b0 - h N(K) = 0.

- The interval tube `T`, the contraction bound `q_lambda >= ||D^-1|| (eta ||C|| + |lambda h| L_N)` with
  `q_lambda < 1`, and the self-map condition `delta_lambda + q_lambda r <= r` (ball inside `T`) are fixed per attempt
  before speculative work. Inverse defect and the rounding of inverse application are part of the enclosure.
- The endpoint certificate is `Q2Admission::NativeTargetCertificate` or
  `Q2Admission::PreparedStructuredCertificate` (`transactional_q1_q2.rs`), with `ModelBinding::GeneratedFromModel`
  and `Q2CertificateSource::capability` = `Available`, decided before speculative work.
  `Q2Admission::OperationalDiagnostic` never admits an output. `ModelBinding::SampledAgreement` is never a proof
  (R4-HOM-DEV-03).
- Convergence at intermediate lambda, predictor agreement and a truncated q-level residual are candidate generators,
  never error bounds.

## 2. Parallel scope

- Parallel: the independent tasks inside one round. In a Jacobi round of
  `Phi(K) = D^-1 (b0 + eta C K + lambda h N(K))` every stage reads only the previous round's frozen `K`, so the
  stage RHS/JVP evaluations and the common-W solves of that round are independent. Coefficient tasks of one level of
  the truncated partial inverse are likewise independent.
- Sequential: rounds, lambda continuation and the levels `l -> l + 1` of the truncated inverse. Workers do not
  remove this depth; nothing in this contract claims they do.
- Output authority is independent of the task partition and the thread count (HT02 item F3).

## 3. Work and span ledger

Both numbers are required; neither replaces the other (section 7.4):

    W_total     = W_setup + W_candidate + W_certificate + W_rejected + W_fallback + W_dispatch
    T_par,model = T_setup + sum_rounds max_worker T_assigned_tasks + T_sync + T_certificate + T_fallback

plus the peak bytes of the attempt. Charged in full: common-W factorization, witness preparation and batch packing;
RHS/JVP evaluations, orthogonalization and coefficient setup; preflight and failed certificate radii; discarded
speculative work; worker pools, barriers and synchronization; the 8-stage sequential fallback and every step-size
retry. The comparator is the strongest sequential arm at the same output accuracy.

`homotopy.rs::HomotopyWorkLedger` today carries rounds (`path_rounds`, `correction_rounds`), setup
(`w_factorizations`, `pool_creations`), candidate work (`tangent_evaluations`, `nonlinear_snapshots`,
`coupling_actions`, `w_solve_batches`, `w_solve_vectors`), certificate work (`certificate_attempts`,
`certificate_operations`) and dispatch (`w_solve_batches`, `pool_creations`). It lacks: maximum task work per
round, barrier count, rejected-candidate work, fallback work and peak bytes. These are HT02 obligations (its F4),
not HT01 failures.

## 4. Cost declaration

A client is eligible only with an `ExpensiveClientDeclaration`, which is a claim HT02 must verify, not evidence:

| field | meaning |
|---|---|
| `client_id` | stable client identity |
| `rhs_callback`, `jvp_callback` | callback identities (source path and symbol) |
| `ir_per_rhs`, `ir_per_jvp` | declared instructions per RHS / JVP evaluation |
| `cost_provenance` | the committed profile (path, SHA-256, entry) the declared costs come from |
| `batch_rhs` | whether a `problem.rs::BatchRhsFn` is provided |

No such type exists in the crates at HT01's base; HT01 screens for it statically.

## 5. Preconditions under which windowed parallelism could pay (HT01 criteria)

- **S1** declared expensive RHS/JVP: a declaration (S1a), and a measured `problem-user-code` Ir share `f` of the
  strongest sequential arm's committed callgrind profile with `f >= f_min = 0.348` at every profiled rtol (S1b). From
  the overhead-free span ratio `1 - f (1 - r/P)` with `r = 5` (RHS multiplier of the certified transactional arms,
  L-0008) and `P = 8`, reaching `1/1.15` needs `f >= (1 - 1/1.15) / (1 - 5/8) = 0.348`. A client without a committed
  profile has `f` unmeasured and rests on S1a alone.
- **S2** shifted solves present: the strongest sequential arm performs `W = I - h gamma J` solves in its step, with at
  least two stage solves per attempt (the tasks a homotopy round makes independent).
- **S3** strong sequential comparator: the cheapest applicable native arm by same-binary Ir per trajectory, with
  committed accuracy evidence and its ledger row; JF-GMRES alone does not qualify where a direct arm applies.
- **S4** endpoint certificate and complete accounting: S4a as in section 1; S4b the ledger of section 3 (missing
  fields are HT02 obligations).

## 6. Stated limits of existing modules (unchanged by HT01)

- `crates/rodas5p-integrators/src/chart_transport.rs`: model-specific error transport for the chart
  `y = x^2 (w + 1/kappa)`; "No integrator calls this module"; `chart_integrate` performs no linear solve. It is not a
  homotopy client and carries no `Q2CertificateSource`.
- `crates/rodas5p-core/src/shared_shift_jet.rs`: one fixed dissipative real `J`; it "does not select a solver, reuse
  operators across calls, certify nonlinear/time integration errors, execute shifts in parallel or claim a speedup".
- PP07 (L-0074): the native Fourier client performs no shifted solve; its ABSTAIN stands, and no artificial
  many-shift workload is substituted. The cheap diagonal (L-0008) and depth-7 (L-0050) studies are not rerun without
  a new cost mechanism.
