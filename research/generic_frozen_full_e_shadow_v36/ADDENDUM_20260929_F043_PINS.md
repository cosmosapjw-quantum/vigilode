# Addendum: two v3.6 test pins move under the F-043 fix (2026-09-29)

Appended note. No sealed v3.6 result, receipt or ledger is edited.

## What changed in the code

Audit F-043: the fused phi action measured its Krylov convergence against
the augmented start vector `[b0; 0; ...; 1]`. The trailing 1 is bookkeeping,
so the tolerance depended on the physical scale of the inputs: small inputs
over-converged and large ones under-converged. Branch
`claude/jolly-wozniak-7wl15h-wu10-exponential-scaling` now

- balances the augmentation: the B block is divided by sigma, the power of two
  at or above max_k |tau|^k ||b_k||, and the chain's start entry is sigma, so
  the augmented system is homogeneous in the input scale;
- measures the convergence threshold on the physical part of the vectors only,
  in `krylov_exponential_once` and in `FusedPhiPrefixSession`;
- converts the adaptive fused path's Euclidean Krylov estimate to a WRMS
  bound, ||e||_2 / (sqrt(n) min w), instead of dividing by ||w||_2.

`tests/fused_phi_scaling_contracts.rs` pins the result: scaling every input
by 2^10 or 2^-10 now leaves the Krylov dimension, the substep count and the
scaled value unchanged. Before the fix, n = 64 diffusion needed dimension 6
at c = 2^-10 and 8 at c = 1.

## Pins that moved

`StageGrowthCalibration96`, Robertson, frozen full-E shadow
(`crates/rodas5p-integrators/tests/frozen_full_e_shadow_contracts.rs`,
`crates/rodas5p-cli/tests/frozen_full_e_shadow_cli_contracts.rs`):

| Quantity | Before | After |
|---|---:|---:|
| continuation JVP vectors | 24 | 26 |
| total speculative JVP vectors | 66 | 68 |
| prefix JVP vectors | 42 | 42 |
| recommendations, completions, unsafe, breaches | 2, 2, 0, 0 | 2, 2, 0, 0 |
| target attempt indices | 9, 24 | 9, 24 |

The continuation runs the production fused kernel, which now converges to the
physical tolerance; it takes one more Krylov vector on each of the two rows.
The RJF (committed) trajectory is unchanged, and so are all safety counts.

The ignored v3.7 replay still fails at its first literal (17 against 18), as
before this change (audit F-003); its later literals were not re-measured.
