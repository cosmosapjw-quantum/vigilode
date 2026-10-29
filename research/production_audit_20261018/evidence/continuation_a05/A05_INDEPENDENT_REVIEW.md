# A05 independent review (counter repair)

Reviewed read-only at base HEAD `c3036c63da02ca9b04cd39ccf580cd67d00ccb0c`.

## Frozen source identity

- `crates/rodas5p-integrators/src/dense_output_v2.rs`: `b17eebcdabfdc1741abe8d3be1f7ad1a72b55a1c28ef4f7f6dab43a2de2198e7`
- `crates/rodas5p-integrators/src/lib.rs`: `bb8f86fc41585a66fb898a75261ec8b974bc6549b82ab4bccb2c2c27f4e502d4`
- `crates/rodas5p-integrators/tests/a05_dense_global_output.rs`: `15203683d1675260c9fe5a9c4112bb6213bc4f7e0cec9f606000791114607e2f`

## Findings and decision

- The stored quartic segment is private and identity-bound. Admission requires an exact positive normal represented clock span; value and physical derivative use directed interval arithmetic.
- Interior value evaluation precharges one shared operation and nine interval primitives per component. Derivative evaluation precharges two shared operations and fourteen per component before the fallible primitives. The source expressions match those counts; counter regressions check nonzero accounting, while exact counts were independently checked against source.
- `from_step` allocates endpoint, coefficient and identity vectors. Successful evaluation uses caller-provided buffers and has no apparent heap allocation by source inspection; this is not allocator-instrumented zero-allocation evidence.
- The retained focused target has 10 passing tests. Clippy, rustfmt, `git diff --check`, and receipt JSON validation exit 0. The receipt binds source/test and raw-log hashes.
- The typed `ContinuousResidualProviderUnavailable` remains. No interval RHS, verified full-tube logarithmic norm, or continuous ODE residual exists in the generic API.

Decision: `PASS_WITH_SCOPE` for enclosure of the represented stored quartic curve and its derivative, with the recorded operation-cost convention. A05 continuous residual, global trajectory accuracy, and production authority remain `HOLD`.
