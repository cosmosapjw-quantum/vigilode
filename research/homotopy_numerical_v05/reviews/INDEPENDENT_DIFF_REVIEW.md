# Independent base-to-head diff review

Date: 2026-08-07
Base: `ce1cc51034186958da3e27f641f29aae777ceaf1`
Reviewed head: `1704f2ce60c556f3bb577bcfc6d12c90e5650168`
Review workspace: separate clean Git worktree `review/homotopy-v05`

## Scope

The review inspected the protected-base-to-head diff for:

- sign and scaling consistency in `D`, `C`, the nonlinear remainder, and the target Jacobian;
- endpoint equivalence with the original RODAS5P stage residual;
- finite-value and false-acceptance handling;
- transactional sequential fallback;
- work-accounting comparability;
- scope creep beyond the fixed-schedule cycle-1 contract;
- reproducibility of the canonical JSON and independent reconstruction of the headline counts.

## Findings resolved before closeout

### R-001 — misleading acceptance-budget name — Important, resolved

The original name `defect_budget_fraction` implied a fraction of the embedded LTE, but the code
actually compared the normalized original-output correction directly with an absolute threshold:

```text
certificate.output_wrms <= 0.1
```

A failing CLI contract was added first. The field and public accessor are now named
`output_wrms_budget`, the emitted JSON no longer contains `defect_budget_fraction`, and the
canonical/smoke results were regenerated. This is not cosmetic: the global-order screen shows why
an absolute normalized budget must not be mistaken for an LTE-relative policy.

### R-002 — incomparable `direct_solve_calls` interpretation — Important, resolved/documented

Sequential RODAS records eight per-stage common-`W` solves, while one `solve_rows` call records one
batched API call even when it carries eight stage vectors. Therefore `direct_solve_calls` measures
call depth, not scalar linear work. The analysis now:

- reports `w_solve_batches` and `w_solve_vectors` separately;
- treats `direct_solve_calls` only as a call-depth proxy;
- bases the equal-work rejection on the independent scalar-work counters RHS evaluations and JVP
  vectors as well;
- records that no fast-accepted candidate dominates sequential or SABR in both of those counters.

## Scientific concerns accepted as explicit HOLD conditions

### R-003 — low-depth initialization is an approximate propagation, not exact path tracking

At `lambda=0`, the exact partial-coupling start solution requires the full finite inverse. For
`q<7`, `run_fixed_homotopy_path` intentionally applies the same truncated inverse used later in the
path, so the recorded initial homotopy residual can be nonzero. The implementation is therefore a
fixed-schedule approximate propagation experiment, not a classical exact-start continuation
algorithm. The residual is exposed in every path report; future papers and APIs must preserve this
distinction.

### R-004 — exact nonlinear certificate is a correctness oracle, not a lightweight fast path

The certificate assembles every stage Jacobian, factors the full current target Jacobian, and
solves for the output correction. It is suitable for false-acceptance screening and calibration,
but its cost precludes a speedup claim. A cheaper certified bound remains future work.

### R-005 — one-step safety does not imply fifth-order global convergence

The canonical one-step grid has zero false accepts at the absolute output-WRMS budget 0.1, yet the
all-fast low-depth trajectories do not robustly retain fifth order. The fixed budget is rejected as
a method policy. An LTE- or step-size-scaled budget must be tested under a new contract.

### R-006 — no actual concurrent backend was measured

`eval_rhs_batch` and common-`W` `solve_rows` expose batch interfaces and call depth, but this cycle
does not execute the stages concurrently on threads, SIMD, GPU, or MPI. No wall-time acceleration
claim is authorized.

### R-007 — q=7 is a correctness oracle only

The q=7 lane reproduces the affine endpoint and fifth-order trend, but requires far more common-`W`
vector work. It must not be presented as a fast candidate.

## Independent evidence

A separate stdlib-only parse of the canonical artifact verified:

```text
SHA-256                            a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e
cases                              57
controls                           114
candidate rows                     8208
fast accepts                       3510
fallbacks                          4698
numerical failures                 0
one-step false accepts             0
low-depth fast rows                401
maximum accepted oracle WRMS       0.09999101609468791
median low-depth W batches         5
median low-depth W vectors         40
fast rows dominating sequential
  in RHS evaluations and JVPs      0
fast rows dominating SABR
  in RHS evaluations and JVPs      0
```

The first independent helper run used the wrong literal control name (`sabr-nilpotent` rather than
`sabr-nilpotent-3`) and stopped on its assertion. The corrected run passed; no result artifact or
production source was changed by that helper error.

## Assessment

**PASS WITH SCIENTIFIC AND PERFORMANCE HOLD.**

No blocking sign, endpoint, false-acceptance, fallback, or scope defect remains in the reviewed
cycle-1 reference layer. The implementation is suitable as an experimental oracle and negative-
result dataset. It is not ready for default integration, an order-preserving low-depth policy, or a
performance claim.
