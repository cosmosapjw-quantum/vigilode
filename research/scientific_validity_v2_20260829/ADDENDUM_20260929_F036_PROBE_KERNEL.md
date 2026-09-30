# Addendum: the spectral corpus GMRES probe now uses the production kernel (2026-09-29)

Appended note (audit F-036). No committed v2 record, ledger or frozen rule
is edited.

`deterministic_matrix_free_gmres_probe_is_not_dimension_constant` in
`crates/rodas5p-integrators/tests/scientific_corpus_v2_spectral_contracts.rs`
called `solve_gmres_givens`, a test-only kernel that production dispatch
never calls. It now calls `solve_gmres` with the identical `GmresConfig`
(restart 64, 128 Arnoldi vectors, rtol 2e-13, atol 0). The Givens count is
kept as a second, labelled column.

Kernel-provenance correction, nonautonomous v2 probe, identity preconditioner:

| Dimension | production `solve_gmres` | test-only `solve_gmres_givens` |
|---:|---:|---:|
| 96 | 3 | 3 |
| 384 | 7 | 7 |
| 1536 | 11 | 9 |

No iteration count was pinned; the test asserts that the counts are not all
equal, which holds for both kernels. The corpus authority for this probe is
the production column.
