# F1 support-sensitive polynomial trace implementation

Controlling correction task: **PR-N05, coherent episode 2/3** (inherited 1/3). Scope was admitted by the parent after its theory phase. This implementer wrote the two assigned source files only, with no native execution, commit, or push. The parent owns publication and serialized execution.

## Contract and implementation

For the unchanged monic quartic `z^4-2*a*z^2+z+a*a-b`, an all-complex algebraic-multiplicity query `Tr(sum c_k z^k)` equals `sum c_k S_k`. Exact coefficient intervals `[0,0]` have no contribution; `S1=0` identically. Every supplied coefficient is validated before either fact is used. An interval merely containing zero is retained.

`quartic_polynomial_trace` now forms a seven-entry liveness mask. Its private `supported_moments` circuit evaluates only required moments:

- S0=4 and S3=-3 are constants.
- S2=4a and S5=-10a are independent linear circuits; they never evaluate a² or a³.
- S4=4(a²+b) shares a² with S6 only when both are requested.
- S6=4a³+12ab+3 retains the original outward expression.

The result accumulates live terms and avoids an initial addition to zero. All interval operations remain checked and outward. Request/counter overflow still fails closed. `moment_batches` is charged once when a parameter-dependent moment is constructed; fixed S0/S3 queries charge only actual product/addition work. The public full-moment provider, field routes, enum/config surfaces, and model semantics are unchanged.

The fixed seven-entry scan is bounded overhead. Dense-polynomial performance must therefore be measured along with sparse cases; no unconditional speedup is claimed from operation counts.

## Staged regression checks

Four additional unit tests are in `crates/vigilode-coupled/src/trace_quotient.rs`:

1. `live_support_matches_exact_companion_traces_for_every_support_mask`: 128 masks × four exact quotient matrices; includes the triple root and the real-double-plus-complex-conjugate configuration, and an explicit cubic distinct-root negative control.
2. `finite_sparse_low_moments_do_not_compute_unused_powers`: powers 2,3,4,5 at large exact dyadic parameters; padded zero support; nonzero linear coefficient multiplying the identically zero first moment. These previously fail when unused powers overflow.
3. `support_liveness_preserves_interval_uncertainty_and_validates_dead_coefficients`: a nontrivial interval containing zero remains live; malformed coefficients in zero-moment/dead positions are rejected before any arithmetic.
4. `required_sixth_moment_overflow_is_rejected_and_charged`: a genuinely required overflowing S6 still fails closed and retains request/partial-work counts.

Existing moment, branch/refusal, coefficient enclosure, and field tests remain in place. No native test has yet been run by this implementer.

## Complete-call campaign

New example: `crates/vigilode-coupled/examples/trace_support_campaign.rs`.

The reference function copies the operation order of PR97 `quartic_polynomial_trace`, `all_moments`, and `even_moments` at `87465835d9d21298d235917f036580b0b53ab645`. It preserves one request boundary and original zero/linear fast path. Calling the public full-moment API would add a second boundary and inflate the baseline, so that shortcut is not used. Both arms compile in one binary.

Registered fixed workload: five query families (`square`, `cubic_padded`, `fifth_padded`, `dense_six`, `mixed_six`) × sixteen small dyadic parameter pairs. Every timed input is independently checked with exact integer companion-matrix powers and small exactly representable dyadic weighted sums. The matched-accuracy gate requires **both returned intervals to equal the exact singleton target**; this per-arm result is persisted. Four additional huge finite queries are reported separately, retaining baseline refusals; they do not enter timing ratios.

There are seven seeded randomized ABBA/BAAB repeats per family, each consisting of four blocks of 2,048 calls. One warmup per arm/family is retained separately. All timings and work counters remain in the output; no favorable samples are discarded.

Each timed call includes parameter/coefficient construction, validation, evaluation, counters and result black-boxing. Shared workload-vector construction is measured separately. The per-block counter reset, JSON serialization and stored-result lifetime are outside block timing and are not presented as solver work. No support plan, compiler, or external certificate setup is hidden in the comparison. The benchmark concerns interval polynomial query calls, not an ODE trajectory or a complete certified integration.

Both the legacy evaluator and candidate adapter have `inline(never)` boundaries. The candidate still calls the public cross-crate API, whereas the frozen legacy body is local to this example; that public-call overhead remains part of the measured comparison and is disclosed in JSON. No compiler-level identical-call-cost assertion is made.

Prepublication compile preparation initially found 13 unavailable `Interval::constant` calls in the new example: that constructor belonged to the implementation's private trait. The example now uses a private exact-point helper only on fixed finite dyadic literals. The parent preserves the original E0599 log. This build repair and the requested singleton/overflow/boundary checks are part of the same prepublication F1 correction episode; no new allowance or scientific rerun is inferred.

## Parent execution interface

After remote publication/readback of the exact source, the parent may run its registered recorder/lock around:

```bash
cargo test --profile measurement --locked --offline -p vigilode-coupled trace_quotient
cargo run --profile measurement --locked --offline -p vigilode-coupled --example trace_support_campaign
```

The example prints exactly one JSON object to stdout. It exits 1 if the exact reference validation fails or any timed call is rejected; expected baseline refusals in the separate huge-input scope cases do not fail the comparison. Cargo build/run diagnostics go to stderr. Capture both streams and source/binary/environment hashes. The parent is responsible for checking the expected nonzero test selection and parsing all timing samples, including dense-query regressions if any.

Status at handoff: **SOURCE_STAGED; NATIVE_TESTS_NOT_RUN_BY_IMPLEMENTER; PUBLICATION_NOT_PERFORMED_BY_IMPLEMENTER**.
