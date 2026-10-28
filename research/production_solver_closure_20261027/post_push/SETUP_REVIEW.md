# Post-push CI setup review

**Anchor:** `78bb8775bf553ad2b514575a0ef4a025446f5d28` (PR #92); baseline PR #91 `206298b4068d923bb47c7160b111b7439098660c`. Review date: 2026-10-28 KST. This was a read-only source/log audit; no native build or rerun and no runtime/configuration edits. The checked-in numerical measurements remain frozen.

## Bench: inherited comparator setup now meets a different ABI

The actual job log records Ubuntu 26.04, `libsundials-dev 7.1.1+dfsg1-10build3`, and installed `libopenmpi-dev 5.0.10-1`. This is not an absent MPI package: compilation stops because the hard-coded `gcc` command in `build.rs` has no MPI include discovery. The first observed error is `sundials_types.h:57: fatal error: mpi.h: No such file or directory`. The bridge source explicitly declares SUNDIALS CVODE 6.4.1 and still uses `realtype` and `SUNContext_Create(NULL, ...)`. `build.rs`, the C bridge, `native/SOURCES.json`, the bench Cargo manifest, and the complete Production CI bench job are unchanged from PR #91.

The SUNDIALS project's 7.0 changelog identifies further API changes: `realtype` leaves the ordinary types header, and the context communicator becomes `SUNComm`, with `SUN_COMM_NULL` for non-MPI use. MPI-enabled builds also require MPI linking. These are source-based migration risks, not later errors observed in this failed run. Sources: [versioned changelog](https://sundials.readthedocs.io/en/v7.2.0/Changelog_link.html), [project changelog](https://sundials.readthedocs.io/en/latest/Changelog_link.html), section on 7.0.0.

Two useful local alternatives remain: reproduce the declared 6.4.1 comparator in a pinned environment; or qualify a deliberate 6/7 bridge with compiler/include/link discovery, version checks and context/type handling. Record each comparator's actual version/precision/compiler identity. Installing MPI again, adding one include path, or setting `CC=mpicc` is insufficient: current code ignores `CC`, and those changes do not resolve the full ABI transition. Neither alternative was implemented or tested in this review.

## Broader Clippy: inherited sites, different gate scope

E4 and Audit2 run `cargo clippy -p rodas5p-integrators --all-targets --locked -- -D warnings` with Rust 1.94.1. Both logs report 17 lib errors and 22 lib-test errors. These compilation counts overlap; the emitted diagnostic list contains 23 distinct sites and should not be summed to 39 defects. Of the 23 sites, 21 are in byte-identical PR #91 files. The other two are in `conditioned_polynomial.rs` and `fixed_point_continuation.rs`: the existing exact-rustfmt receipt's before/after hashes match both commits, and the warned loop/conditional already existed. The JSON records all sites and line references.

This establishes inherited source sites; it does not claim a newly executed baseline Clippy run. The green Production CI Clippy job explicitly checks only `vigilode` and `rodas5p-core` with `--no-deps`, so it never established a warning-free integrator crate.

Use a separate lint cleanup preserving arithmetic order and API semantics. Moving test modules and narrowing unused support-helper compilation are low-impact changes. Argument-count, large-error and large-enum warnings require an explicit local API/layout decision; blanket allows or mechanical boxing can conceal cost or source-compatibility changes. Run the exact broader command and affected contracts after that cleanup. No solver theorem or tolerance change follows from these diagnostics.

## Semver: two newly introduced public fields

`cargo-semver-checks 0.51.0` against PR #91 fails `constructible_struct_adds_field` for `SwitchingConfig.strict_return_work_improvement` and `SwitchingConfig.reuse_observed_entry_step`. This is an actual new source break for exhaustive public struct literals. Both legacy defaults are `false`; adding these two explicit fields preserves that choice, or callers can intentionally use struct update from `SwitchingConfig::default()`. A caller migration does not settle version policy.

The semver job is inherited and currently `continue-on-error: true`; its comment refers to a first published 0.x baseline. The workspace currently declares 0.1.0. Registry publication history was not checked here, so the comment is not evidence of registry status. Before release, choose either a compatibility-preserving API arrangement or an explicitly versioned/documented break, then rerun the semver command against the exact base and compile a representative downstream literal. Do not describe this change as unconditionally additive or silently bump a version to quiet CI.

## Local closure

The Givens regression needs its own fixture/profile audit (outside this report). Bench compatibility, inherited lint cleanup and API migration can proceed independently. Once those concrete problems are resolved, rerun the intended full production/release validation in the recorded environment. Release qualification remains pending. The numerical correction budgets remain structural 3/3, homotopy 1/3, adapter 3/3; this review did not consume an additional numerical correction.
