# One substantive theory and implementation review

Reviewer: `/root/theory_derivative_free`, a separately dispatched native
subagent with inherited project context. This is a complementary code/proof
review, not a claim of blind or heterogeneous-model validation. Read-only
review of production modules; independent rational checker and mathematical
derivations written only in this research directory.

## Reviewed objects

- `crates/vigilode-coupled/src/causal.rs`: `picard_reuse`,
  `affine_solve_reuse`, and the unchanged baseline recurrence.
- `crates/vigilode-coupled/tests/causal_reuse_contracts.rs`: exact state,
  stage, residual, callback and solve accounting; epoch/failure/budget controls.
- `crates/vigilode-coupled/src/rhs_homotopy.rs`: map evaluation boundary,
  station reset, depth-one coefficient, actual-residual comparison, work and
  partial-station failure state.
- `crates/vigilode-coupled/src/structural_mean_field.rs`: immutable model,
  reusable rate/step plan, two scalar recurrences, lifting, sequential
  comparator, and retained full-space screen.
- `THEORY.md`, `exact_checks.py`, and their result receipts.

This review was performed while the implementation lanes were active. It
does not report unobserved native test results; those remain in each lane's
actual execution receipt.

## Findings and resolutions

### R-01: exact cache identity

Object: causal remainder caching. Risk: a physical-state-only key need not
fix the separately evaluated frozen action. The implementation keys every
bit of the projected displacement, keeps all other operands immutable, and
checks the epoch. This meets Theorem A's premise. No code change requested.
Resolved by direct source inspection and the deterministic-program proof.

### R-02: prefix dependence and callback failures

Object: reused affine solve blocks. A disconnected unchanged RHS row cannot
be skipped merely because that row is unchanged. The code reuses the maximal
unchanged prefix and recomputes the suffix in the baseline ordered dot/solve
sequence. Workers are joined and their attempted work merged before callback
failure propagation. These are the necessary contracts; the implemented
tests exercise their meaningful externally observable consequences.
No code correctness defect found in this pass.

### R-03: actual-residual acceptance and map changes

Object: RHS-only Anderson depth-one trial. A smaller least-squares combination
of historical residuals would not establish the new nonlinear residual.
The code explicitly evaluates the extrapolated map and compares its actual
residual against the already evaluated Picard candidate. It retains map
values in `Value`, resets history per continuation station, bounds the
coefficient, and labels the returned residual operational. Theorem E uses
these exact semantics. No code change requested.

### R-04: physical root accuracy outside map contraction

Object: cubic implicit-midpoint consumer. `h*k/2>1` can invalidate a Picard
contraction argument even though the target has a well-conditioned root.
Derived the global monotonicity bound
`|K-K*| <= |F_lambda(K)|/(1+lambda*h*k/2)`, independent bisection brackets,
and continuation bound (28). Sent this result to the homotopy implementation
lane and added Section E.4. Resolved mathematically; floating enclosures
remain distinct from the operational residual measured by native code.

### R-05: full-state quotient information

Object: mean-field specialization. Arbitrary transverse initial state and
forcing must survive reduction. `f0` retains all n components, and the lift
uses both its mean and full transverse component. The rate-only plan is
independent of forcing, so allowing reuse across forcing arrays of different
dimensions is mathematically valid when the immutable rates and h match.
The two scalar raw-U recurrences and endpoint `b_code` combination agree
with Theorem D. Exact-rational checks cover arbitrary nonsymmetric forcing.

### R-06: complete cost statement

Object: Theorem D's complexity wording. The initial draft described
`O(s^2+s*n)` construction cost without immediately charging the retained
`O(s^2*n)` original-space screen. Corrected the theorem text to separate
construction from total screened execution. Native successful counters
already charge the common screen. Resolved; no benchmark data changed.

### R-07: structural failures and future fallback accounting

Object: structural API returning `CoreResult<StructuralCandidate>`. On an
error, partial `StructuralWork` is not returned. The current standalone
successful-arm benchmark can report its complete successful counts. A
future adaptive fallback that invokes this path must retain failed work in
its own receipt before making total-cost claims. Parent informed. This is
an integration obligation outside the current successful standalone cost
comparison, not a reason to repeat those measurements.

### R-08: symbolic tool expression

Object: first Wolfram nilpotent inverse check. The zero-power `MatrixPower`
inside `Sum` produced a singular/unevaluated expression. It supplied no
positive evidence for that identity. Replaced the expression with
`IdentityMatrix + lambda L + lambda^2 L.L + lambda^3 L.L.L` and obtained
True for the inverse/nilpotence identities, zero for scalar affine Anderson,
and True for the mean-field resolvent. Both tool responses are saved.
Independent Fraction checks separately verified the same finite inverse.

## Review conclusion

The inspected implementations match the stated cache, quotient, and
operational residual contracts. The proofs identify the exact conditions
under which these operations preserve the target or certify root error.
The principal unclosed general scientific obligation is to furnish
full-domain constants/closure defects for other nonlinear physical models;
the finite experiments do not manufacture those constants. No new strong
no-go result is admitted.

## Narrow changed-dependency follow-up: R-09 checkpoint seed

The parent requested this follow-up after the actual adapter run exposed the
old B3 cold-seed mechanism. It reviews only the new proposal selector and its
call site; the completed causal/quotient/homotopy reviews are not repeated.

Read `switching.rs::entry_pilot_checkpoint_seed`, its entry replay call,
`run_persistent_explicit`, `entry_checkpoint_seed_contracts.rs`, the private
invalid-proposal helper contract, and `adapter/CHECKPOINT_SEED_PROTOCOL.md`.

The field defaults to false, including both factory configurations. When
enabled, it uses only a finite positive `record.initial_step`, captured from
the accepted explicit checkpoint **before** the current batch. The maximum
with `Options::h0` is clipped to the actually reached explicit replay span,
then raised to the existing valid native minimum-step configuration floor.
The native driver endpoint-clips the actual trial. `proposed_next_step` is
not used, so no proposal measured at the future endpoint enters this replay.

The physical initial state and endpoint, error tolerances, global/pilot
attempt caps, callback ceiling, derivative permission, `charge` call, fatal
handling, and final qualified-work predicate are unchanged. The proposal
may alter adaptive meshes and route decisions; physical accuracy is tested
against the original analytic cap instead of requiring bitwise parity.
No correctness defect was found. The API comment was clarified to distinguish
configured minimum-step floors from the endpoint-clipped actual attempt.

Actually inspected native log `adapter/seed_contracts.log` reports **4 new
contracts PASS** and **7 unchanged evidence-switch contracts PASS**. This
reviewer did not rerun them. The new tests compare each pilot with its own
starting checkpoint, reconcile all callback and attempt totals, enforce the
original analytic error cap, and use a panic derivative callback to verify
`Forbidden` remains effective. A private helper contract covers nonfinite,
nonpositive, and smaller proposals, independently of the future proposal.

The corresponding mathematical statement and startup-cost mechanism are in
`THEORY.md` Section 8.1 and the final subsection of `theorems.tex`. Native
repair measurements are retained separately from the initial failed/losing
pilot experiment under `adapter/`; their success does not retroactively
change the old raw result.

## Focused follow-up: derived constants and exact work admission

The new user requirement asks for analytically or algebraically derived
constants in place of discretionary hyperparameters. This review covers
only the changed homotopy, sealed cubic consumer, structural budget, and
strict entry comparison. It does not repeat earlier cache or full-project
reviews. The exact source hashes are preserved in `REVIEW.json`.

### R-10: cubic truncation, root conditioning, and global transport

Independently derived the polynomial secant identity, sharp lower divisor
`1+h*k/2`, exact bracket-invariance criterion, and global nonexpansivity of
the midpoint step. The truncation proof uses the **difference** of two
nonnegative defects; the identity
`2*phi'(Y)^2-phi''(Y)*phi(Y)>=0` justifies bounding their maximum by
`C*h^3`, `C=L^2*phi(Y)/8`. The constant is safe and analytically derived,
without a claim of optimality.

Read `CubicDecayAccuracyPlan::new`, `CubicStepMap`,
`cubic_verified_step`, and `cubic_integrate`. The planner rounds constants
in the safe direction, admits only exact dyadic subdivisions, derives a
positive remaining algebraic budget, and uses `Y+epsilon` only for the
iteration domain. The true orbit remains in `[0,Y]`, so its local
truncation bound does not need that enlarged domain. Global midpoint
nonexpansivity makes the transport proof valid even if a rounded
approximate state becomes slightly negative.

The final accepted step uses an outward enclosure of the original cubic
residual, a lower enclosure of the positive divisor, and the actual
represented endpoint-addition displacement. It then checks the outward
accumulated bound and adds the time-label displacement multiplied by the
true derivative bound. The final certificate does not rely on the ordinary
floating Picard map retaining exact contraction. Resource, interval-sign,
and representability failures retain attempted work and do not authorize
an incomplete endpoint. No correctness defect found in this scoped pass.

### R-11: residual dominance without fitted margins

Read the new `ResidualDominance` enum, configuration validation, normalized
dot products, trial checks, and acceptance branch. This mode bypasses the
legacy empirical ratio, coefficient magnitude, and rejection-count fields;
its actual residual comparison is `trial <= already_evaluated_Picard`.
Theorem E needs exactly this factor-one inequality. A represented zero
denominator leads to a nonfinite quotient and skips the extrapolation;
finite but inaccurate proposals still face domain, finite-output, and
actual-residual checks. No arbitrary near-zero epsilon is used. The
consumer's final interval residual is the physical accuracy authority.

The mode provides no theorem that an extra trial evaluation lowers total
work. The actual new constant-budget experiment reports the same total
polynomial counts for Picard and residual dominance at each warm/cold
setting; its observed reduction comes from the previous-stage seed.
This is consistent with the proof and prevents relabeling warm-start
savings as an acceleration improvement.

### R-12: exact accumulated structural amplification

Read `MeanFieldPlan::algebraic_residual_budget` and its independent full
operator contract. The reverse recurrence solves the transpose of the
actual lower-triangular raw-stage operator with the stored `b_code`.
The one-step row-sum formula includes the diagonal entry separately and
handles `n=1`. The accumulated computation uses separate transverse and
collective scalar powers at every step, matching the sharp norm `C_N`,
not merely the geometric bound `A*sum(G^j)`. The latter remains explicitly
reported as an upper bound. No contraction assumption or singular
division by `G-1` is introduced.

The proof of sharpness uses independent sign-aligned residuals across
step, stage, and component. The fixed-h autonomous target is essential.
The code and docs appropriately distinguish the ordinary binary64 budget
evaluation from an outward floating or physical certificate. Temporal
truncation, endpoint rounding, and residual evaluation errors are separate.
The parent exact-rational receipt independently verifies 12 cells, including
the 64-step accumulated identity. All current fixtures have equality with
the simpler geometric bound, so no numerical improvement from the tighter
formula is claimed on those cells. No correctness defect found.

### R-13: strict integer work and the callback stopping threshold

Read `integer_work`, `strict_entry_callback_ceiling`,
`strict_entry_work_preferred`, the replay call and post-call charge path,
the native callback guard semantics, and the new arithmetic/integration
contracts. The checked integer objective retains all existing categories:
RHS, JVP, time derivative, dimension times Jacobian builds, and linear
solves. Same checkpoint and reached accepted prefix are required. An
accepted explicit prefix remains a valid incumbent even if its larger
requested batch stopped at a resource cap.

The exact `E-1` ceiling is correct because the callback guard charges the
crossing callback before stopping: at that point `L>=E` and the final
candidate total cannot be strictly smaller. Large-count or checked-overflow
cases decline evidence or the early ceiling instead of truncating counts.
Ties retain the explicit route. The existing native error acceptance,
derivative permission, attempt limits, fatal handling, and charging remain
in force.

Requested one documentation clarification: the strict flag applies only
when `implicit_pilot_attempt_budget>0`; zero keeps existing pressure-only
direct entry. The owning agent applied exactly that comment change and
preserved a pre-comment execution snapshot and hashes. This closes the
scope ambiguity without changing behavior. No further change requested.
Remaining pressure, retry, and return thresholds are still heuristics;
the new theorem concerns the specified local integer objective.

### R-14: actual verification read by this reviewer

Executed the independent `exact_constant_checks.py` with exit zero:
900 secant/root-error cells, 900 nonexpansive derivative identities,
180 analytic bracket cells, 36 truncation polynomial cells, 25 normalized
Anderson quotient cells, and six minimal-dyadic budget identities passed.
The checker contains no floating tolerance or fitted margin.

Read the owning lanes' native logs rather than rerunning their builds:

- `homotopy/exact_constants_contracts.stdout.log`: 16 passed.
- `homotopy/rounding_floor_test.stdout.log`: one additional precision-floor
  contract passed; the earlier 16 were filtered for this run.
- `evidence/exact_structural_contract.stdout.log`: one new full-operator
  and extremizer contract passed; four earlier tests filtered.
- `adapter/exact_arithmetic.log`: six arithmetic contracts passed.
- `adapter/exact_integration.log`: five entry contracts and seven existing
  evidence-switch contracts passed.

Also read the new cubic summary: 12 successful accuracy-qualified rows,
with independent Decimal80 observation checks and native outward bounds.
That high-precision check is consistency evidence, while the directed
arithmetic and the proof supply the certificate. The report preserves
initial failures, old measurements, and all attempted work; new exact
constants do not retroactively alter earlier results.

The source review and independent derivations pass within the stated model,
arithmetic, and work-objective scopes. The new full proofs are in
`THEORY.md` Section 11 and the appended REVTeX input `theorems.tex`.
