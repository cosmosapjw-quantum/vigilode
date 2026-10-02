# Preregistration: residual-seeded radius and stage/component radius box (thread-transfer DAG node P1-RADIUS-PROPOSAL)

## Question

The doubling certificate needs an a priori state radius `D` with `B E(D) <= D` (`E(D) = sum_k H(D)^k a`, `B` the
absolute `alpha` map). The R4 schedule `0.001 * 4^k` is blind to the problem. Since `H(D)` is nonnegative and
monotone in `D`, `D >= max B E(0)` is necessary for a common radius (review, section 4.2), and `max B E(0)` uses only
the current residual. The review also gives a system where no common radius exists but a stage-dependent radius
does (section 4.3).

1. Does a residual-seeded common radius `D = factor * max B E(0)` close the R4 fixtures in one preflight and one
   check?
2. Does a stage/component radius box `D_(i,u)`, re-verified independently, close where a common radius cannot?

Every proposal is only a proposal: validity is decided by recomputing the full closure inequality with the
certificate's own directed arithmetic at the proposed radii. A failed proposal is an explicit failure or fallback,
never an infeasibility verdict.

## Implementation (written after this commit)

In `crates/rodas5p-integrators/src/causal_majorant.rs`:

- a `MajorantEntries` trait (stages, components, seeds `a_(u,i)`, `|alpha_ij|`, and the coupling
  `H_u(D)_ij` at a stage radius, nonnegative and rounded upward) and `evaluate_radius_box`, which for a radius box
  `D_(u,i)` forms `H_u` with row `i` at radius `D_(u,i)`, evaluates `E_u` (matrix order or action-first) and returns
  the state radii `sum_j |alpha_ij| E_(u,j)` and whether every one is `<= D_(u,i)`;
- `residual_seeded_common_radius(entries, factor, mode)`: one evaluation at `D = 0`, proposal
  `D = factor * max B E(0)` rounded upward, one evaluation at the proposal; all attempts are returned;
- `causal_radius_box(entries, inflation, mode)`: the causal construction `D_(u,i) = sum_{j<i} |alpha_ij| E_(u,j)`,
  `E_(u,i) = a_(u,i) + sum_{j<i} H_u(D_(u,i))_ij E_(u,j)`, inflated by the relative factor `1 + 2^-20` and then
  re-verified by `evaluate_radius_box` (the construction itself is not the proof);
- `AffineMajorant`, a test family with `H(D)_ij = H0_ij + D_i H1_ij`.

In `outward_certificate.rs`: `DiagonalMajorant` (the blocked certificate's entries, the same formula and rounding)
and `blocked_box_certificate_with_execution`, which rebuilds the entries from the step's own inputs, evaluates the
given box, and finishes the certificate only if it closes. With a constant box in matrix order it must reproduce the
blocked certificate bit for bit. The existing PastStepData policy, the six-attempt schedule and the default routing
are unchanged.

## Tests and command

`crates/rodas5p-integrators/tests/thread_transfer_radius_policy.rs`:

`THREAD_TRANSFER_RADIUS_POLICY_OUTPUT=research/thread_transfer_radius_proposal_20261002/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test thread_transfer_radius_policy -- --nocapture`

## Gate

**PASS** if all hold:

1. **Residual seed closes R4.** With factor 2 (the implementation plan's `D = 2B`), on all five R4 fixtures
   (n = 1, 2, 4, 8, 16), in both matrix and action order, the proposal closes at its first check (two evaluations in
   total), and the native box certificate at that radius encloses the interval reference root (as in
   `research/thread_transfer_path_action_20261002`).
2. **Constant box = blocked.** A constant box in matrix order reproduces `blocked_doubling_certificate_with_execution`
   (one attempt at the same radius) bit for bit, on all five fixtures.
3. **Scalar no-go versus box.** On the affine system `k_0 = 1, k_1 = 1 + k_0^2, k_2 = 1 + k_1^2` (zero candidate:
   `a = (1, 1, 1)`, `H1` and `|alpha|` the subdiagonal ones, `H0 = 0`), the common radius fails at every tested
   `D` in {0, 2^-10, 1, 2, 2^10, 2^40} and the residual-seeded proposal reports failure; the causal box closes and
   is verified, and the hand box `(0, 1, 2)` closes. (That no common radius exists is proved analytically: stage 2
   needs `1 + D <= D`. The tested values only illustrate it.)
4. **Causal box on R4.** On all five fixtures the inflated causal box is verified by the independent evaluation,
   and its certificate encloses the interval reference root.
5. **Fail closed.** NaN, infinite or negative radii, a box of the wrong shape, a non-finite or negative factor and
   an overflowing proposal are errors; a seed of zero proposes `D = 0`, which then closes.

Otherwise **FAIL**. Reported, not gated: factor 1.1 (the review's heuristic) as a second arm; evaluations and
directed operations of preflight + check against the extended 4x schedule; box and common bound widths against the
serial certificate. The serial certificate remains the cheapest by operation count (L-0024); radius closure is not
an admission or speed result, and the cost margin `1 + p1 - 8 pf` of L-0024 is not re-estimated here.

## Prior information

- The review's Python probe: the 1.1x proposal closed all five fixtures (n = 8: proposal 2.5164, required 2.2994;
  n = 16: 16.2533 vs 15.0046); an anisotropic box was verified with one recheck on each.
- No Rust code of this node exists before this commit.

## Amendment before the recorded run (enclosure reference)

During development the directed-interval reference root named above proved unusable as an oracle: where a
certificate is tight to rounding it cannot decide. At stage 0 the bound `U |r_0|` equals the exact distance up to
rounding, so the few-ulp width of an interval root makes `|K_hat - K*|` over the whole enclosure exceed even the
serial certificate, which is known to be valid (it is checked against exact roots in
`outward_certificate_contracts.rs`). All three paths (serial, matrix, action) "failed" that check at every n, while
all other development checks behaved as expected.

The reference is therefore replaced, before the recorded run, by the repository's existing oracle convention:

- `cargo test ... --test thread_transfer_path_action -- --ignored write_r4_stage_inputs` writes the exact R4
  fixture inputs (`fixtures/thread_transfer_r4_stage_inputs.json`): `J`, `y`, `q`, `h`, candidate bits, and the
  target's `alpha` and native `Gamma` bits;
- `python3 tools/thread_transfer_root_oracle.py` solves the same causal stage recurrence in exact rationals
  (`Fraction`, with `alpha_ij + Gamma_ij` as the exact real sum) and writes one-ulp brackets `[down, up]` of every
  exact distance `|K_hat - K*|` (`fixtures/thread_transfer_r4_root_oracle.json`);
- a bound encloses when `bound >= up`, or `down == up` and `bound >= down`, as in
  `outward_certificate_contracts.rs`; an undecidable case counts as not enclosed. The tests check that the oracle's
  inputs equal the fixture bits in memory, so a stale oracle fails.

The gates are otherwise unchanged. Development runs of the tests before the recorded run showed: exact identity,
rejection of invalid input, the same closing attempts, determinism over workers, and an operation ratio of 0.565
(action/matrix) on all five fixtures. No result file was written.
