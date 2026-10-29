# Adapter lane: comparator dominance stopping

Source anchor: `3064d453091af1dec7f99f0c8afabd98b447ea34`.
Skill context: PhysMath Research Loop 1.3.0, SKILL.md, POLICY_INTEGRATION.md,
CODING_LOOP.md read through c38. Scope R3–R7/C2–C9, no recursive delegation,
no historical campaign rerun. Read current switching source, latest integration
report and checkpoint-followup raw relaxation pair. Parent upgraded source
from dda8 to 3064 before edits; initial patch is preserved separately.

## Corrected audit finding

At dda8 the ordinary adaptive return required a completed implicit replay.
3064 already accepts a nonfatal incomplete replay as a lower bound and adds
implicit-first return sentinels. These two issues are therefore closed in the
current source. They must not be claimed as new fixes.

The remaining ordinary explicit-first path supplies `None` for the implicit
comparison callback ceiling. Thus an already completed admissible explicit
candidate waits for all 64 allowed implicit attempts or a full completion,
even after a monotone work lower bound settles the comparison. This is an
implementation economy defect, not a failure of the underlying ODE formula.

The current published relaxation trace provides a concrete cost witness:
explicit return 6 attempts, 37 RHS/work; its implicit comparator 6 attempts,
48 RHS + 6 Jacobian + 6 df/dt + 48 solves = 108 proxy units. These are reused
historical observations, not a newly measured speedup. At rho=0.8 the already
completed explicit candidate dominates once implicit callback cost exceeds
ceil(37/0.8), so the full 60 callback-unit comparator is unnecessary.

## Patch

Only `crates/vigilode/src/switching.rs` is changed in production source.
`SwitchingConfig::stop_dominated_return_comparator` defaults to false and
applies to the ordinary explicit-first adaptive return comparator. It does not
change the native formulas, error tests, initial proposals, return scheduling,
or the independent implicit-first sentinel. A true switch computes an outward
upper bound on the completed explicit candidate's exact integer work proxy,
sets the existing callback ceiling, and admits the candidate only after an
outward lower bound certifies dominance. `ExplicitCheaperThanComparatorLowerBound`
distinguishes this evidence from a fully completed matched-window comparison.
Every callback remains charged; an observed fatal callback takes precedence;
the partial comparator state is never committed.

## Mathematical contract

For a fixed checkpoint, target, authorized callback environment and cost unit,
let E satisfy the native candidate error/output contract with total work C_E.
Let L_I(s) be the nondecreasing accounted full-work prefix accumulated by its
implicit challenger before time s; the callback-only prefix is also a valid
conservative lower bound. Every continuation that finishes the same challenger
has work C_I >= L_I(s), since both subsequent callbacks and solve costs are
nonnegative. If C_E^upper <= downward(rho*L_I^lower), then C_E <= rho*C_I.
Stopping that challenger and committing E preserves E's admitted state and
local accuracy checks. It proves comparative work on that window only. It does
not establish a global error bound, stiffness classification, future optimal
selection, or a bound relative to an omniscient no-probe solver.

The native ceiling is checked after callbacks. Under sequential callbacks
with maximal callback weight U, an armed cap B and preflight cost V imply at
most B+U+V callback units before the ceiling stop. The full proxy also includes
linear solves; it is not a wall-clock bound. Any dynamic model mutation or
non-replay-safe callbacks violates the comparison premise.

## New artifacts and requested validation

- `crates/vigilode/examples/audit_adapter_20261027.rs`: paired complete-solve
  benchmark with the new switch off/on in one binary; analytic smooth
  relaxing-forcing ODE, optional repeated scalar blocks/dense provider.
  It retains every returned state, exact accepted-trajectory equality,
  all-component trajectory sample error, charged-work sums and lower-bound
  evidence. ABBA/BAAB batches include setup, allocation, all probes and counters;
  JSON extraction is outside timing. Blocks=0 performs only deterministic
  diagnostic solves. Arguments: tau frequency stiffness h0 sentinel blocks
  windows dimension (defaults .4 1 9999 1e-4 0 5 12 1).
- `crates/vigilode/tests/comparator_dominance_contracts.rs`: three real-IVP
  tests for full accepted-trajectory parity with work reduction, fatal
  comparator precedence, and implicit-first sentinel independence.
- Two targeted internal arithmetic tests cover binary64 integer-range
  boundaries, strict conservative admission, incomplete candidate refusal and
  callback-only lower bound.

This was initially a pre-execution checkpoint while the supplied Rust archive
was being repaired. The bounded verification completion below supersedes that
status; no speedup was inferred from the earlier historical trace.

## Parent review correction: telemetry semantics

The typed `ExplicitCheaperThanComparatorLowerBound` is now emitted for every
winning incomplete return comparator, including the pre-existing attempt-budget
path when early stopping is disabled. The corresponding existing sentinel
regression expectation was updated; no success criterion was relaxed.
Incomplete entry pilots now leave `comparator_work_ratio` absent, because their
prefix cost is not the completed-method ratio. These are diagnostic corrections
separate from the opt-in performance change.

Full accepted-trajectory equality with a full-comparator baseline is a testable
conditional property: the unchanged route must finish within an ample common
global attempt budget, and the skipped suffix must not contain a future fatal
callback that the full comparator would have observed. An actual fatal callback
encountered before the stop still prevents candidate commit. The method cannot
predict unobserved failures on a discarded challenger; it guarantees only the
already completed candidate's own contract.

## Separate incomplete-output semantics

After parent review, all incomplete return-comparator decisions use a separate
`prefix_cost_dominates` predicate over the exact accumulated full work proxy.
Only completed comparators enter `qualified_work_preferred` and its output
admissibility contract. This removes the earlier misuse of `admissible=true`
for incomplete incumbent output. Partial full work (including already-spent
linear solves) is a valid completion-cost lower bound; the proactive guard
still meters callback cost, a conservative subset.

Both sides of every incomplete comparison now use outward conversion from a
checked u128 sum. Positive integers are expanded even when exactly representable,
so equality or sub-ulp margins may be retained at ordinary counts as well as
above 2^53. For example, C_E=80, L_I=160 and rho=1/2 is conservatively retained,
while L_I=161 provides a resolved strict margin and is admitted. This is an
intentional conservative boundary behavior, not tolerance widening. Decisions
with a margin larger than the constructed arithmetic enclosure are unchanged.

## Complementary mode-code review (read-only)

Reviewed only changed LU workspace and partial-t call sites, as requested by
parent, without a new recursive review. The pinned faer 0.24.4 source
`linalg/lu/partial_pivoting/solve.rs` ignores `par` when calculating scratch
requirements; the buffer holds permutation scratch, so a later global
parallelism change does not underallocate that pinned implementation. The
new call uses the same L/U/P, Conj::No, current parallelism and solve kernel as
faer's existing PartialPivLu SolveCore implementation. Workspaces hold no
factor identity. Dimension and finite-input checks precede buffer writes.
The numerical failure path may modify caller RHS, as documented; its inspected
integration users propagate failure and do not publish that buffer.
`eval_partial_t_into` has the same count, zero initialization and supplied
callback/finite-difference arithmetic as the replaced allocating path on valid
states. No concrete correctness defect found on these reviewed changed surfaces.
This is a complementary source-level check by a sibling agent in the same
research context. It is not a blinded independent review or an independent
execution result. No recursive reviewer or broad campaign was launched.

The mode reviewer identified that an incomplete-comparator displayed ratio
was only ordinarily rounded. That finding is fixed: `prefix_ratio_upper` uses
candidate upper / prefix lower with upward rounding and returns None when no
finite positive-denominator bound exists.

## Bounded verification completion

On the repaired Rust 1.94.1 toolchain, all 19 selected tests passed (2 arithmetic lib contracts, 3 dominance contracts, 7 evidence-switch contracts, 7 return-sentinel contracts). The `measurement` example build passed. Command elapsed times, including compilation, were 7.3587 s + 9.5962 s + 8.0841 s = 25.0390 s. Source hashes were stable throughout every command. Raw stdout/stderr and machine-readable receipts are in `adapter_lane/validation/`. No timing trial was run by this lane. Parent owns the diagnostic grid and paired timing. Candidate example copied to `tmp/bin_candidate/audit_adapter_20261027`. `git diff --check` passed.


## Frozen candidate and fixed-arm timing harness

The first tested benchmark binary had SHA-256
`d6a0e3c06c432a1320d78a7bc62b8ada57c1569984f2549e8182252fdfb7e608`.
After the parent completed its initial serial timing, it authorized a harness-only
addition for one focused variance check. Production source and tests remained
unchanged. Optional argument 9 is now `baseline` or `optimized`; argument 10 is
the fixed repetition count in `1..=100000`. Invalid or missing fixed-mode
arguments terminate with status 2 before constructing the problem. With these
arguments, the example performs the existing two diagnostic solves and one
batch of the selected arm, without calibration or ABBA repetition. Without
them, the previous CLI and calibrated paired workflow are unchanged.

The selected batch's `seconds` includes complete solves and excludes the two
initial diagnostic solves, result summaries and JSON output. External process
CPU time includes process initialization, **both** initial diagnostic solves,
the selected batch, summary construction and JSON. It must therefore be
reported as process CPU, not batch-only CPU. The common diagnostic overhead can
dilute a speed ratio, especially at small repetition counts. Determinism compares
the final batch result's full accepted trajectory and summary with its matching
initial diagnostic solve. It does not compare every intermediate repetition.

This harness revision was compiled once, with no further tests or scientific
runs by the adapter lane: `build_r2.receipt.json`, exit 0, 2.0322 seconds. The
19 PASS results apply to unchanged production and test sources; the revised
harness has a separate successful build receipt. Total adapter compile/test
command elapsed time is 27.0712 seconds. The replacement candidate example is
`tmp/bin_candidate/audit_adapter_20261027`, SHA-256
`3796f6cb2c927dd10ba26ee24b0736154a91bb2f46edf646db178c134a2f30c2`.
Its example source SHA-256 is
`7eadc2b0028c4c7343506e2a0b8474ebcd44e519f0f86fe14c16566beee6795e`.
The binary receipt records both hashes. Timing claims belong to the parent's
serial benchmark evidence and must not be inferred from successful compilation,
trajectory equality or a reduced arithmetic work proxy alone.

## Evidence packaging and review boundary

This finalized note is copied to
`research/rigorous_performance_audit_20261027/ADAPTER_AUDIT.md`.
All adapter validation raw stdout, stderr and JSON receipts are copied to
`research/rigorous_performance_audit_20261027/evidence/adapter_validation/`.
Receipts contain exact commands, working directory, status, elapsed time and
source hashes. This lane's contribution is one bounded adapter audit, one
minimal opt-in optimization, diagnostic semantic corrections, focused regression
checks, and a same-context complementary review of the sibling mode patch.
It is not external certification, a whole-repository correctness proof, or a
rerun of historical benchmark campaigns.


## First-pass harness preservation

The first-pass example source was reconstructed by reversing only the two
fixed-arm additions in the final formatted source. Its SHA-256,
`8f61ed7ea65f4c5410104a22a94a3e7000d0b6a8f527f37605731b9c07246fbf`,
**matches the original successful build receipt**. It is preserved as
`adapter_lane/audit_adapter_first_pass.rs` and in the validation evidence
folder, with a reconstruction receipt stating the method and match. The old
`d6a0...e608` executable was overwritten after the parent recorded its first
environment and was not archived. No compilation or scientific run was made to
recover that executable. The final `3796...30c2` executable and final source
support both the original CLI and the new fixed-arm CLI; distinct timing passes
must retain their respective recorded source and binary identities.
