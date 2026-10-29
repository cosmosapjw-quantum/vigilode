# Independent static review of the native directional quadratic proposal

Outcome: **ACCEPT_SCOPED_STATIC** for the first source-bound build and preregistered experiment. This is not a claim of compilation, native test success, speedup, rigorous IVP enclosure, or production readiness. No native solver, numerical test, benchmark, or symbolic calculation was executed in this review.

Reviewer: independent `/root/structural_review`; reviewed working tree based on `dd600057d16a7717c47409f7806c33dd515e9da5`, before the new source publication. The file hashes below bind the scope.

## Mathematical and native contract

- Expanding `(I−σJ)((1−aσ)v+σJv)` yields `Dv−σ²(J²v−aJv−bv)`, with `D=1−aσ−bσ²`. The implementation numerator `(2−aσ)v−Av` is algebraically the same. No diagonalizability premise is needed for this identity.
- The operator and shift are consumed anew for each stage. Supplied coefficients are explicitly a numerical hypothesis; neither successful directional acceptance nor the public constructor claims a full-space relation or epoch certificate.
- The strict MF client admits only identity mass, no preconditioner, ordinary unstaged GMRES-into. Configuration is revalidated before attempt callbacks, so disabling GMRES-into after enabling the proposal cannot bypass the contract.
- The configured initial iterate is preserved. Proposal scratch is separate from the stage output. Every normal refusal leaves the stage output untouched; the existing fallback receives the same original `x0`. The zero/initial-convergence shortcut is explicitly documented as a changed callback-count contract.
- A newly formed candidate is accepted only after a live original shifted-operator action and the existing computed L2 threshold. Optional standalone WRMS validation requires positive finite scales. This is a floating residual gate, not an outward-rounded physical-error certificate.
- Denominator/range/finite-arithmetic refusal is conservative; it does not assert mathematical singularity. Finite-input norm or initial-subtraction overflow is refused before a nonfinite construction input reaches a callback.
- Every entered action uses `apply_counted`; action storage is poisoned first. Callback errors and incomplete/nonfinite writes propagate, and work already performed is preserved. Accepted proposal reports zero Arnoldi iterations/cycles/least-squares solves and one linear solve, with separate cumulative proposal statistics.
- Rotation resolvent and logarithmic-norm bounds follow from skew symmetry. The mixed 16-rate endpoint polynomial bound uses the actual diagonal/normal assumption; it is not generalized to nonnormal Jacobians. The stated break-even condition retains paid refusals and does not imply generic speedup.

## Experiment and evidence controls

- Both arms share one executable, model, represented initial state, controller, tolerances and 17 requested output times. The large rotation repeats the original 16-state amplitudes. Timing includes workspace construction, full IVP integration and owned drop; model creation, error evaluation and JSON serialization are excluded in both arms.
- The runner verifies published HEAD/fetched ref, committed runner/protocol and inherited-results bytes, successful build receipt, binary digest, and before/after source hashes. It preserves prior 491.53919015300016 seconds inside the 900-second cumulative scientific cap; compilation is separately identified.
- Tests/oracle/clippy must pass before timing. Timings retain seven ABBA/BAAB blocks per case. Correctness includes zero and previous starts; timing uses previous start. Nonfinite/unsuccessful/grid-cap failures refuse timing qualification.
- Interrupted child waits kill the child process group, wait for termination, save command receipt and campaign checkpoint, then propagate the interruption; timeout paths likewise retain receipts. No hidden child is deliberately allowed to continue beyond an interrupted scientific run.
- The Fraction oracle reconstructs exact dyadics from actual Rust-emitted bits, independently assembles original rotation residuals and proves the exact residual/error identity for six accepted records. Two refusals check unchanged output, pole/no-action and wrong-coefficient separation. Its scope is these fixed represented kernel records, not whole-IVP certification.
- WorkCounters dot/axpy fields do not count the newly added plain vector loops; the probe explicitly says so. Operator calls, strict work units and wall time must be reported separately. Relative-residual log values may exceed rtol when atol governs; do not mislabel that as a failure.

## Nonblocking limits and required readback

- Static inspection cannot establish that compilation, all 11 expected new test methods, or the emitted eight-record oracle will pass. Compare actual stdout with the preregistered inventory; do not infer counts from a merely successful command.
- No guarantee of speedup exists for mixed spectra, bad coefficients or tiny solves. Preserve all adverse results and charged fallback work, including any initial-iterate shortcut acceptance in the bad-coefficient arm.
- The 1e-5 analytic grid criterion is a binary64 diagnostic. No current evidence closes rigorous residual-evaluation error for arbitrary callbacks, nonlinear physical transport, dense-output interior error, or all inherited production gates.
- Source-dependent conclusions must be reevaluated if any hashed file changes. Run the registered native checks and independent oracle on the actual published new source before any performance or production claim.

## Reviewed SHA-256 hashes

| File | SHA-256 |
|---|---|
| `crates/rodas5p-integrators/src/directional_quadratic.rs` | `25749aa521873a072a7c48be99b85d52f664446287d89d1070d7aa88d4fe3986` |
| `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs` | `8425652310c29d8b3f8e649ea477040fa48c35bb391dcaf6f3073b83e1aae185` |
| `crates/rodas5p-integrators/src/lib.rs` | `80d3ff9544c28cd2661f73d018663faed826ed35419f558c6d83b0bb0c6588ae` |
| `crates/rodas5p-integrators/tests/directional_quadratic_contracts.rs` | `2f6e2c1aefdfe502476141f82cc188d4137c86cc5c1626aff2dd621345fdb0b8` |
| `crates/vigilode/examples/structural_quadratic_probe.rs` | `1457d066cf53a463d8f1e414cc669633aa5f436f28e8ffffba1d264d26bfe297` |
| `research/synchronized_production_20261029/theory/STRUCTURAL_NATIVE.md` | `d52eae1d694061c16c9d0e55ac764e4944079a7d6f941785f575782a4101e2e3` |
| `research/synchronized_production_20261029/STRUCTURAL_PROTOCOL.json` | `0736be7c1ca355c84c3e0d5fac2ccfcf00bd0b47f0ebf456891b7dacb171f9b9` |
| `research/synchronized_production_20261029/code/run_structural.py` | `bd49aba2edfeaeed298b5ab31adefac437fc19c8c54a50c9f949412c6e842a1c` |
| `research/synchronized_production_20261029/code/verify_structural_bits.py` | `25db1cb7e513b7316fc03d95b1a9f4560d45915bff406825754aa49b93d7b4fc` |
