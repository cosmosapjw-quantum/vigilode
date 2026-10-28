# Addendum: v3.6 test pins move under the time-normalized phi augmentation (2026-09-30)

Appended note. No sealed v3.6 result, receipt or ledger is edited.

## What changed in the code

Audit 2026-09-30, PHI-P1 / PHI-P2: the fused phi action exponentiated
`tau M` with `M = [[A, [b_p .. b_1]], [0, J_p]]`. In the scaled convention
`b_k ~ v_k / tau^k`, so `||tau M||` grows like `tau^-p` as tau shrinks. On the
scalar A = -1 the Arnoldi residual estimate then accepted a dimension-1
projection with a relative error of 3e-3 (h = 1e-4, 1e-8, 1e-12), and the
dense Pade oracle returned 0 for h <= 1e-14.

Branch `claude/jolly-wozniak-7wl15h-wu21-audit0930` exponentiates
`M_hat = D^-1 (tau M) D = [[tau A, [tau^p b_p .. tau b_1]], [0, J_p]]`, with
`D = diag(I, tau^(p-1), .., tau, 1)`. `D` fixes the start vector and the
physical projection, so the physical output is the same exactly. The upper
block holds `w_k = tau^k b_k` directly. The fused action, its substeps, the
prefix session and the dense oracle (`dense_phi_combination`) all use this
form. `tests/phi_normalized_augmentation_contracts.rs` pins the scalar
counterexample to 1e-12 for h = 1e-1 .. 1e-70, including zero and negative
scale.

## Pins that moved

`StageGrowthCalibration96`, Robertson, frozen full-E shadow
(`crates/rodas5p-integrators/tests/frozen_full_e_shadow_contracts.rs`,
`crates/rodas5p-cli/tests/frozen_full_e_shadow_cli_contracts.rs`):

| Quantity | F-043 (2026-09-29) | Now |
|---|---:|---:|
| continuation JVP vectors | 26 | 28 |
| total speculative JVP vectors | 68 | 70 |
| prefix JVP vectors | 42 | 42 |
| recommendations, completions, unsafe, breaches | 2, 2, 0, 0 | 2, 2, 0, 0 |

The continuation takes one more Krylov vector on each of the two rows: the
normalized chain no longer shrinks with h, so the residual estimate stops
accepting early. The prefix and the committed trajectory are unchanged, and
so are all safety counts.

`tests/phi_action_certificate_contracts.rs`, adaptive fused failed-trial
accounting: the Krylov cap is raised from 3 to 5 (augmented dimension 6).
Under the normalized form a cap of 3 never converges; the old cap passed
through the same early acceptance as PHI-P1.

The v3.7 consumed N=192 replay moves more, see
`research/generic_timing_replication_continuation_transaction_v37/reports/ADDENDUM_20260930_PHI_NORMALIZATION.md`.
