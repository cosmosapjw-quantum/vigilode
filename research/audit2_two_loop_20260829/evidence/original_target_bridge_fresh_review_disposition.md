# ORIGINAL_TARGET_BRIDGE fresh-review disposition

## Boundary

Exactly one fresh-context, read-only review inspected the new scientific delta
from delivery `e77ec86376ca89850e18e99963992aeeb01055c2`. It focused on bridge
sign, frozen-W and strict-lower assumptions, condition-aware criteria,
missing/uncertain budgets, and failure/work preservation. It did not reopen the
earlier c894 review, inspect old K0, or claim a second fresh review.

The reviewer independently ran:

```text
cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_structured_correction_contracts -- --nocapture
exit 0; 13 passed

bash tools/check-audit2-readiness.sh
exit 0

git diff --check e77ec86376ca89850e18e99963992aeeb01055c2 -- \
  crates/rodas5p-integrators/src/audit2_research.rs \
  crates/rodas5p-integrators/tests/audit2_structured_correction_contracts.rs
exit 0
```

## Initial review decision

`FAIL`: P0=0, P1=1, P2=2, P3=0.

| Finding | Disposition |
|---|---|
| P1: a late failure could report completed output/embedded counters while dropping the corresponding computed values. | Repaired once. `Audit2OriginalTargetPartial` is updated after each completed diagnostic/projection and attached to every failure. A late embedded overflow regression asserts that completed output and diagnostics remain available. |
| P2: tracked evidence omitted the detailed attempt/completion work profiles even though the claim ledger said they were serialized. | Repaired in the evidence. Four compact profiles retain every attempt/completion field and every nonzero `WorkCounters` field for n=4, n=8, n=16, and the mass/nonnormal case. |
| P2: no original-Jacobian-only negative ruled out substitution of the projected Jacobian. | Repaired with an injection that lets both projected arms complete, then fails the original Jacobian action and asserts an `original-target-setup` failure with retained work. |

The review confirmed the bridge sign and `K <- K-z`, use of the same supplied
trial-stage slice, and `BudgetNotSpecified` treatment. It also recorded the
inherited boundary: the original Jacobian is the existing unprojected API
linearization, whose exact-derivative interpretation still assumes the
pre-existing strict-lower-alpha structure. The diagnostics observed 28 projected
forbidden-alpha entries; no general nonlinear exact-Jacobian claim is admitted.

## Post-repair disposition

The single authorized repair round was followed by the exact targeted test:

```text
cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_structured_correction_contracts -- --nocapture --test-threads=1
exit 0; 15 passed
```

The two added regressions pass, the original 13 contracts remain green, and the
result/raw/work evidence exactly matches this post-repair test log. The full
readiness command then exited 0. On those objective checks, unresolved counts
are P0=0 and P1=0, so the scoped review gate is dispositioned
`PASS_AFTER_ONE_BOUNDED_REPAIR`. This is a disposition of the original findings,
not a claimed second fresh review.

No tolerance, fixture, case selection, accuracy budget, production route, or
claim ceiling changed during repair.
