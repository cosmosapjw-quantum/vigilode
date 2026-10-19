# Recovery, source lineage and external checks

## Runtime interruption

The current conversation sandbox initially executed a 12-cell SciPy 1.17 Jacobian differential diagnosis and installed the uploaded Rust 1.94.1 toolchain. Later both terminal and Python execution returned caas ServerError. The connected desktop route separately reported an exhausted monthly tool quota and was not retried. No native three-way benchmark from the interrupted sandbox is claimed. A source dump from Dropbox dated 2026-10-16 was located but was not substituted for the current pinned GitHub code.

Execution moved to an explicitly bounded GitHub Actions job on the already-authorized publication branch. Run 39252371864 verified runner access and receipt publication only. Run 39253322042 recorded the RED contract test: 10 tests, 3 expected failures because metric admission was not yet implemented. These are not solver correctness passes. Subsequent run directories retain actual build, tests, accuracy and timing separately.

## Primary sources

- SciPy 1.17 release notes: https://docs.scipy.org/doc/scipy/release/1.17.0-notes.html ; VODE/ZVODE/LSODA/dopri routines ported from Fortran77 to C.
- SciPy issue https://github.com/scipy/scipy/issues/24933 and fix https://github.com/scipy/scipy/pull/24934 ; dense user Jacobian was converted to C layout and copied to a Fortran-layout destination, effectively transposing it. Fix commit 9d88ff1739705d8c9f483eca014d6293d499c260, merged 2026-03-29. Found with Exa and inspected against source. Merely returning a Fortran-contiguous array does not evade the forced C conversion.
- Original VODE source: https://www.netlib.org/ode/vode.f ; Brown, Byrne and Hindmarsh, VODE: A Variable-Coefficient ODE Solver (1989). MF21 analytical dense, MF24 analytical banded; local tolerance is not a global trajectory error guarantee.
- Diffsol =0.16.1 API follows this repository's tools/benchmarks/diffsol/a07 source. Its recorded older measurements are not pooled into this campaign. Dependency resolution is captured in each run's Cargo.lock.
- Wolfram connected evaluator independently checked Robertson conservation/Jacobian and the exact nonsymmetric pivot fixture. See SYMBOLIC_CHECK.wl. This is a symbolic consistency check, not a certified global integration error bound.

The SciSpace literature query was executed; broad returned papers were not treated as numerical performance evidence. No claim is founded on the irrelevant fixed-step comparison articles returned by that search.

## Historical conclusion corrected

The previous statement that the SciPy 1.17.0 VODE results measure the original Fortran implementation is withdrawn. The very large steps/errors are wrapper/implementation regression diagnostics, not evidence that the VODE algorithm is intrinsically inferior. A transpose workaround fixed much of the Robertson step explosion but did not resolve the HIRES/van-der-Pol anomalies in the interrupted sandbox. The additional cause was not established there; the new full-band/finite-difference/exact-fixture audit preserves that uncertainty.
