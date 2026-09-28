# E-06 phi-action accuracy — results
Status: **FAIL** (18/54 cases exceed the pre-registered 1e3*eps = 2.220e-13 relative-error rule).

All failures are the 18 cases with ||v||_2 = 1e8: `dense_phi_action` k=1..4 relative error 2.7e-9 .. 1.6e-8 vs the mpmath 30-digit reference, for every matrix type (dense/upper-triangular/stiff-diagonal), every ||A||_1 (1e-3, 1, 1e3) and both sizes (8, 64). k=0 and `matrix_exp_pade13` are accurate (worst 3.5e-14). At ||v||=1 and 1e-8 all k pass (worst 2.4e-14).

Independent confirmation without mpmath: phi_k(A, 1e8 v) differs from 1e8 * phi_k(A, v) (both computed in Rust) by ~1e-8 for k>=1, and by <1e-15 for the 1e-8 scaling.

Mechanism: `crates/rodas5p-core/src/matrix_functions.rs` `dense_phi_action` builds the augmented matrix with the unscaled v; `matrix_exp_pade13` picks the number of squarings from the 1-norm of that augmented matrix (lines 80-86), so ||v||=1e8 adds ~24 squarings and the rounding error grows to ~1e-8. Scaling the augmentation column (v/||v||) and rescaling the output would remove the dependence.

Files: cases.json (54 cases), rust_out.json, reference_results.json (per-case relerr_k, proportionality), reference_stdout.txt, results.json. Reference wall time 242 s.
