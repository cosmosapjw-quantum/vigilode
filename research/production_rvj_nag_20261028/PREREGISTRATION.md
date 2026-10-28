# Native RVJ closure and trace: registered bounded validation

Research identifier 20261028; actual UTC and Asia/Seoul timestamps belong in
the execution receipt. Base is PR96 13f3298bd0213f121e07385903f9508c435ad6a1.
The theoretical phase and parent admission precede native implementation.
The actual code, oracle and this protocol must be remotely published before
the recorded scientific runs. Compilation/format preparation is not a new
scientific result. Existing attachment measurements are historical inputs.

## Questions and unchanged targets

**Cyclic:** original G_i=q_i+q_(i+1)^2/4-x_i and
f_i=-x_i+(q_i+q_(i+1))/16+1/32+(i mod 3)/256, n=4,8,16,
x(0)=q(0)=0, t in [0,1], X=[-1/8,1/8]^n, Q=[-1/4,1/4]^n.
Both slow and algebraic all-component errors on every whole closed slab must
meet the requested 1e-8 cap. The reconstructed path is exact-field-slope
Hermite through represented endpoint nodes. The native circuit contains no
online Jacobian, matrix construction/factorization, Newton, JVP or AD call.
This is a declared structural model, not a generic black-box theorem.

Compare three native certifiers on identical compiled RK4 endpoints:
historical power-polynomial/Bernstein with the old valid radius;
direct Bernstein with that same radius; direct Bernstein with the proved
C5 rational radius. The baseline uses the same native outward primitives
and no newly added unnecessary endpoint work. Test N=16,32,64,128.
Do not compare native timings across the attachment's old host/language.
No new native Newton/Krawczyk speed claim is part of this cohort.

**Trace:** full complex quartic roots with algebraic multiplicity only.
Compare original S2/S4/S6 field with its shared polynomial circuit in the
same native binary. Controlled target p=3/4+t/8+t^5/64 on [-1,1], exact
solution (p,p), includes the triple point at t=0. Test N=16,32,64,128,256;
N256 is included before execution because the inherited path certificate
first passed there. Fixed moments/invariants are interval-enclosed and
checked independently by exact companion traces. Branch-selected and
real-only requests must be rejected explicitly. Native integration timing
alone will not be called complete certified-trajectory speed. The independent
exact Python path certificate is timed and labeled separately.

## Finite experiment and profiling budget

One in-process warmup and seven interleaved/rotated repeats per native case.
These are resource/design choices, not mathematical safety constants or a
statistical holdout. Preserve every sample, failed physical-cap case and
refused input. Record integration, preparation/certificate, total costs and
actual operation/work counters. Medians of phase timings need not sum to
the median total. Report spread and the first passing tested grid, not a
global grid optimum or a hardware-independent speed theorem.

Use Rust1.94.1, Cargo.lock unchanged, measurement profile (opt3/no LTO/16
codegen units), CPU runs serialized under /tmp/vigilode-cargo-20261027.lock,
CARGO_BUILD_JOBS=2. No concurrent benchmark/build jobs. New package tests
and campaign examples only; inherited broad PR96 runs are not replayed just
for reassurance. Full release/feature/external-client qualification is local.

Per command caps: compile/test900s, each native example180s, exact oracle900s.
Source, compiler, argv, environment subset, raw outputs, elapsed times and
exit/timeout are retained. No result file is overwritten. Native phase timing
and work counters constitute the initial profile; no unsupported hardware
counter or Callgrind claim is made.

## Independent scientific oracle and rejection controls

`code/verify_native.py` constructs exact rational endpoint slopes independently,
Hermite POWER polynomials and original-equation residuals, then converts to
Bernstein form. Check every exported cyclic slab and coefficient enclosure,
the sharp weight bounds, the declared radius propagation, full domains,
initial/clock identities and physical cap. Include deliberate corrupted
coefficient or identity rejection. No sampled supremum is accepted as proof.

Trace controls include ordinary parameters, exact triple root, double root
with a complex pair, full multiplicity, a regular selected-branch mismatch,
finite/overflow refusal, original moment field and exact quintic path. Exact
whole-slab original residual and tube certificates remain distinct from node
error. Both successful and failed grids are scientifically informative.

`code/SYMBOLIC_CHECKS.wl` supplies ten exact symbolic supporting checks for
companion moments, factorization, Bernstein weights and rational identities.
Run once via Wolfram after publication and preserve raw response. These
checks do not prove floating implementation or replace the analytical proofs.

## Commands and execution binding

Run `code/run_registered.py --source-sha FULL_SHA --output NEW_DIRECTORY` from
the checkout, with cargo/rustc available or the explicit --cargo path. The
runner checks the remote-tracking ref and source identity, obtains the shared
lock, runs targeted native tests and both examples, then the exact oracle.
Its full argv list and unabridged logs are the command authority. Wolfram is
recorded separately against the same file digest/source commit.

Any test/oracle assertion, source mismatch or nonzero command exit yields
FAIL, with the first failing variant retained. A coarse-grid physical refusal
is an expected result, not permission to relax the cap. A diagnosed repair
may use one of at most three inherited N03/N05 correction episodes; do not
reset prior budgets or silently alter target/observable/domain. Successful
bounded results do not close Q-LEDGER, Q-API, all N tasks, ignored tests,
hardware/external-solver comparisons or broad production qualification.

## Results append point

No new scientific execution recorded at source registration time.

## Completed results (appended after registered execution)

Actual execution source: 64004e23042f2cf076da5378be724f50c3856f4c. Remote readback preceded native execution.
Run status PASS; cyclic6 and trace9 tests passed; independent all-slab oracle
completed732947 checks; registered Wolfram identities10/10 passed.
Whole-state cap and all nonadmitted grids remain unchanged. Cyclic direct/tight
first-passing whole-call ratios8.7206/9.3338/9.1984 are scoped to this cohort.
Trace whole-trajectory speed improvement is not established. See RESULTS.json,
analysis/BENCHMARK_ANALYSIS.json and evidence/registered01/RUN_RECEIPT.json.
Production remains HOLD; inherited gates and correction budgets persist.
