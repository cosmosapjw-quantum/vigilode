# N01-JF R3 source-path audit: preregistered findings
Parent PR115: 8555b68a8053fc081be1195b0a40024d994d5798. Source blobs 0fab50efc17d4ce64f43fd0a639245c65f3628c0 (core) and c42a9cba71fdc7461165e3dc728f24e95d3eb53a (adapter).

1. Opt-in N01 factory currently registers a Jacobian even though a JVP exists. Existing OdeProblem::linearize() gives preference to explicit Jacobian, therefore this is not the intended JVP-only forward route. OdeProblem::linearize_matrix_free() works only when separately requested.
2. N01MassActionOwner::jvp_at() explicitly calls jacobian_at(state) and forms a 3x3 Jacobian at every application, contrary to the large-system matrix-free design. This is not merely an allocation issue.
3. Each RHS/JVP/Jac callback invokes advance(t,state), updating owner epoch on temporary stage probe. R-NEXT-07 changes epoch for model data mutation, with time/state separately frozen in operator identity; callback probes must not invalidate the model generation.
4. Existing source-owned point original residual and physical stage bound remain mathematically conditional and useful; no need to delete, weaken or rederive them. A temporary stage evaluation must not implicitly become accepted physical-state binding.

Original design links:
- docs/superpowers/specs/2026-08-07-rust-parity-port-design.md (matrix-free operators)
- docs/superpowers/specs/2026-08-10-vectorized-jf-path-controller-design.md (same original stage target, fail accounting)
- research/rnext07_model_epoch_20261003/PREREGISTRATION.md (client model-epoch promise)
- research/audit2_matrix_free_common_w_20260830/CLAIM_LEDGER.md (matrix-free shifted-W reference)
- research/production_solver_closure_20261027/PRODUCTION_SETUP.md (RHS-only default, supplied JVP opt-in)
- research/synchronized_production_20261029/REPORT_KO.md (operator-only quadratic candidate, original residual before admission)

Derived identity for Robertson source:
J(z)v = (-u+s, u-s-q, q), u=k1 v_a, s=k2(c v_b+b v_c), q=2 k3 b v_b; exact equality with Df and sum_i Jv_i=0 checked by actual Wolfram symbolic evaluation (True).
Scope claim: no general JFNK global guarantee, no full BASS33 physical source or whole-slab certificate.
DAG at R3: VigilODE 7/23, REI 28/88, successor 0/12 admitted, N00F 1/1 scoped, N01-JF 0/1 (implementation pending), production HOLD.
