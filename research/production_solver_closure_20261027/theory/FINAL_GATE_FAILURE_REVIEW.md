# Final native gate: inherited lagged-GMRES work-accounting defect

## Conclusion and scope

The failure is a deterministic mismatch between solver-level attempted work
and provider-level completed-work reporting. It is not a shared-atomic test
race and is not caused by the adapter, structural, homotopy or CI-style changes
made in this closure. Before the repair, the failing test, lagged driver,
generic operator accounting and relevant GMRES implementation were byte-for-byte
identical to PR91 `206298b4068d923bb47c7160b111b7439098660c`.

The parent authorized a separate integration-accounting repair. Only
`crates/rodas5p-integrators/src/rodas5p_lagged.rs` changed: physical JVP/Jacobian
work is now reconciled against its own charged-counter delta. The callback
execution path, state arithmetic, iteration choices, tolerances, fixture and
test expectations are unchanged. This is not a fourth adapter numerical
correction. No native command was executed by this reviewer; the parent owns
the before/after reproductions and full gate.

Repair source SHA-256:
`5d454206bf3377b0e1105ea4f92a86d26f9c8ad202455be595cd98b1743b8db5`.

At this review's creation, the source repair is frozen and its native
after-validation is pending the parent's coordinated command. No after-pass
or general production qualification is inferred here.

## Actual failure provenance

`../evidence/native_final_r1/RECEIPT.json` records the five-package native
command's actual exit101, no timeout, and unchanged source throughout. It ran
with `--test-threads=1` and `RAYON_NUM_THREADS=1`. The stdout records one test
in this integration-test executable and the assertion failure at
`reaudit_lagged_failure_work.rs:77`:

```text
c.jvp_calls                       = 603
r.work.jvp_applications            = 604
```

The parent also reproduced the isolated test before editing, with actual
exit101, source unchanged and wall0.41272110400313977s in
`../evidence/lagged_failure_before/RECEIPT.json`. Both receipts preserve their
stdout/stderr and source-input hashes. Their receipt SHA-256 values are:

- Five-package failure:
  `37fa156f1430a3a98097e475c890976e7b5d727e0ca9f9e79de5660dd726f0d7`.
- Isolated before-repair failure:
  `16674e178294bc5a4609cd5c23e8fee616e1c6609b941a5950ce6a21cdf1310f`.

The test's subsequent equality against the callback's own atomic counter
was not reached in the failing execution. Its value must not be described
as separately printed or asserted by that before-run. The exact callback
relationship follows from the source argument below and will be checked by
the unchanged test after the repair.

## Why a parallel/shared-statistics explanation does not apply

The test allocates `Arc::new(AtomicU64::new(0))` inside its only test function.
Its sole captured clone belongs to that problem's JVP callback. It injects one
recoverable `NonFinite` error when `fetch_add(1)+1 == 4`. No static work counter
or other test's reset is involved.

Each lagged stage separately creates its own `StageOperator.applications`
and `StoredLu.applications` atomics at zero. Integration-level `WorkCounters`
and `LaggedWork` are also local. The stage and GMRES control paths are
synchronous. Relaxed ordering of a locally owned monotonically incremented
counter does not explain a deterministic loss of precisely the failed
callback. The independent single-test reproduction and unchanged-source
receipts also rule out attributing the observed failure to overlapping edits.

## Exact failure mechanism

The relevant contracts are in `rodas5p-core/src/operator.rs`:

1. `apply_counted` checks shapes, then increments `linear_matvecs` before
   entering the provider. This field counts attempted applications.
2. `LinearOperator::apply_with_work` has a conservative default: call
   `self.apply(...)` and only after success accumulate `application_work()`.
   Custom providers must override it if they expose partial work themselves.
3. The private lagged `StageOperator` uses that default. Its own `apply`
   validates shapes, increments the per-stage atomic, and invokes exactly one
   JVP callback when a JVP is present. Its declared physical work is one JVP
   call and one vector per successful application.

Before this repair, `Lagged::iterate_gmres` used

```text
failed_products = entered_products - delta(linear_matvecs)
```

and added that same number to both solver-level and physical JVP counters.
Its accompanying comment incorrectly said generic `apply_counted` counted
only successful calls. Because the generic solver-level counter already
includes failures, `failed_products` is zero even when physical JVP work is
missing. The per-stage atomic and `LaggedWork` correctly retain the failed
entry, producing604 versus603.

`apply_preconditioner` still increments its counters only after a successful
application. The existing independent stored-LU reconciliation therefore
remains correct and was preserved.

## Local accounting proof and exact correction

Consider one invocation of `iterate_gmres` after valid shape checks. Let A be
the stage operator's atomic entry count, L the increase in solver-level
`linear_matvecs`, and P the increase in its physical `jvp_calls` counter
(or `jacobian_matvecs` when the stage uses the explicit matrix). Only this
private stage operator contributes the relevant physical work during that
invocation; its stored-LU preconditioner does not evaluate JVPs/Jacobians.
The identity assumes that the cumulative counters remain representable in
`u64`. This local repair does not redesign the repository's existing counter
overflow policy.

For the JVP route, each entered operator invokes the supplied callback exactly
once. Thus the external callback count, A, and the increment of
`LaggedWork::jvp_applications` must agree, including the failing callback.
If F entered applications fail before the default work hook returns, then

```text
L = A
P = A - F
old correction = A - L = 0
required physical correction = A - P = F
```

The repaired code snapshots the physical counter before the stage and computes
the physical correction from its own after-minus-before delta. It continues
to reconcile the solver-level attempt counter independently. For successful
stages, P=A and the new addition is exactly zero. For this injected error,
F=1, so exactly one JVP call and one JVP vector are restored. The additions
occur before the existing error propagation, preserving the attempted-work
contract for the recovered failure. No empirical allowance, changed
assertion, extra callback or altered numerical acceptance is introduced.

The explicit-matrix branch uses the same separate physical-counter ledger.
Within this private operator its dimensions come from the same valid native
workspace, and one matrix-product entry corresponds to one stage application.
This is a scoped statement about these native stage providers. It is not a
universal claim that arbitrary custom or composite operators reveal all
partial failure work.

## Baseline and changed-file identity

The following hashes were compared directly with `git show` of PR91 before
editing. All listed files matched PR91; only the lagged driver now differs.

| File | PR91 and pre-repair SHA-256 |
|---|---|
| `rodas5p-integrators/tests/reaudit_lagged_failure_work.rs` | `24f1c52e58829b0aecd1f93cd326a8baab5d4efaa493f0fab4a7e4c67895faee` |
| `rodas5p-integrators/src/rodas5p_lagged.rs` | `3f726d226fbee4ee093fff751adc9c61cd85a819a53d1ac8c8fad53163fb1e66` |
| `rodas5p-core/src/operator.rs` | `989a2d3953c1f6b83138cb93a51072aa829e311c00bc76ee488ea85d125bff0a` |
| `rodas5p-krylov/src/gmres_into.rs` | `87d0ffccfb3ae0281494a51be1bef3b9c0627cbcc322c2df8afba3a06e9dc561` |
| `rodas5p-krylov/src/common.rs` | `b8a03ade17035487c9e92734d3e4c31dd6b94b43f0d16809fb5178a5e26238a8` |
| `rodas5p-integrators/src/problem.rs` | `890aa2218931a12ec7fcea7c72a0fb25eb588e7984a04ad2c5d89b4d5e95e066` |
| `rodas5p-integrators/src/rodas5p_fast.rs` | `33d0d7831ab5fffe558df69d37adc71ef7f4ec984420a757af2185f935cdfa66` |

Paths in the table are under `crates/`. The patch has one23-addition,
10-deletion hunk, including corrected variable names and comments. Rustfmt
was applied only to the changed file. The failure test's source hash remains
unchanged.

## Meaningful verification targets for the parent

The unchanged `reaudit_lagged_failure_work` target checks the one injected
recoverable JVP failure, retained completed Arnoldi columns, operator-work
partitions, equality with the external callback atomic, stored-LU accounting
and final solution accuracy. Its pass is the necessary narrow regression
check; weakening its equality or serializing an imagined shared-global race
would conceal the actual defect.

`b1b_gmres_contracts` provides existing successful dense-branch and supplied-JVP
coverage. In particular:

- `gmres_run_charges_every_refresh_and_counts_every_product` exercises
  Robertson and HIRES without JVP providers.
- `gmres_jvp_path_runs_inside_iterating_stages` also runs a plain HIRES
  explicit-matrix comparison.

These tests exercise ordinary dense/JVP trajectories and existing ledger
invariants. They do not independently inject a dense matrix-product failure,
and their pass must not be described as such. No existing injected dense
failure test was identified during this review. The parent's final
five-package `--no-fail-fast` native gate can then expose any further failures
instead of stopping at this formerly first failing integration target.
