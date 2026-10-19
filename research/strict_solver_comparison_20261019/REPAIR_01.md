# Evidence-directed recording/bounds repair

Run 39255702592 built the native driver successfully and passed both native tests and all ten contract tests, but aborted while saving the first nonlinear reference because a SciPy solver counter was a NumPy int32. This is RECORD_SERIALIZATION_FAILURE, not numerical integration failure. The failed run is retained.

The bounded repair recipe checks exact original Git blob identities. It converts only NumPy scalar JSON values to corresponding Python scalars; nonfinite values and unknown objects remain rejected. The three serialization tests are executed before and after repair, retaining both results.

Two additional source-review findings are addressed before any ranking: the native completion predicate now includes observed.success; the Fortran band Jacobian callback zeros only logical band rows and does not overwrite trailing workspace padding. A sentinel regression checks the boundary and the packed analytic entries. No production solver crate, tolerance, case, timing selection or reference equation changes.

CI publishes the repaired source as a distinct commit BEFORE compiling it, and writes EXEC_SOURCE_COMMIT.txt. This avoids claiming that the patch-free parent commit is the measured binary. The previous run's exact Cargo.lock is reused to prevent dependency-resolution drift.
