# ORIGINAL_TARGET_BRIDGE claim ledger

## Authority and fixed rules

This work unit starts from delivery `e77ec86376ca89850e18e99963992aeeb01055c2`
(tree `44e5d68dcb91c8a167c987fa9c92a8134f866f87`) and changes only the
feature-gated Audit-2 research entry, its contract test, and this research
directory. The publication PR supplies the actual delivered head/tree.

The following choices are result-independent. They were inherited from the
handoff or selected by input coordinates before examining outcomes:

- the existing 12 cases are the Cartesian grid `n in {4,8,16}` and
  `h in {0.001,0.01,0.05,0.1}`;
- the thirteenth case is the pre-existing nonidentity-mass/strong-nonnormal
  fixture;
- structural projection uses the existing absolute `64*eps` rule;
- the bridge sign is
  `rho_o = rho_p + (A_o-A_p)z - (r_o-r_p)` and the update is `K <- K-z`;
- the inherited small-system checks use `4096*eps` for backward error and
  `8192*eps*condition_f` for same-original-target correction agreement;
- the raw vector sample is `n=4,h=0.01`, selected by its input coordinates,
  not by an extremum or a favorable outcome;
- output and embedded-error projections have no acceptance threshold and are
  recorded without comparing tiny secondary errors by relative equality.

No rule or tolerance was fitted or widened after seeing these results.

## Result ledger

| Claim | Evidence | Disposition |
|---|---|---|
| The direct and decomposed original-target residuals agree in the declared cases. | 12 grid rows plus the mass/nonnormal row; maximum bridge identity L2 error `0`. The wrong-sign mutant has L2 gap `26.907248094147423`. | `SUPPORTED_IN_DECLARED_CASES` |
| Common-W corrections satisfy the inherited backward-error ceiling against the actual original target in the declared cases. | Maximum `2.0497995971367258e-13`; fixed ceiling `4096*eps = 9.094947017729282e-13`. | `SUPPORTED_IN_EXISTING_SMALL_EXPLICIT_DOMAIN` |
| Common-W corrections agree condition-aware with a direct solve of the same original target in the declared cases. | Maximum relative difference `2.7388464792698878e-11`; each row is checked against its own fixed `8192*eps*condition_f` bound. | `SUPPORTED_IN_EXISTING_SMALL_EXPLICIT_DOMAIN` |
| The original target was evaluated rather than silently projected. | The original residual uses the preserved `StructuredBlockSystem::target_residual`. Separate original-only residual and Jacobian failure injections leave both projected arms completed, so substituting either projected action is rejected. | `SUPPORTED_BY_NEGATIVE_CONTRACT` |
| Diagnostic work and partial results are retained. | Original residual/snapshot/Jacobian setup, LU, every direct solve used for the oracle and condition estimate, projected/original diagnostic applies, JVP/RHS/mass work, output/embedded projections, and attempts/completions are serialized separately. A late embedded-projection overflow retains the already completed original output and diagnostic results. | `SUPPORTED_BY_ACCOUNTING_CONTRACT` |
| The projected correction is an accurate nonlinear step or output solution. | No external observable budget or authoritative reference uncertainty treatment was supplied. | `FORBIDDEN`; `BudgetNotSpecified` |
| The common-W research entry is faster, scalable, production-ready, or preferable to another solver. | No timing, scalable W backend, whole-step driver, client campaign, fair comparator, holdout, or production dispatch was run. | `FORBIDDEN` |

The compact 13-row table and failure/work ledger are in
`original_target_bridge_results.json`; the result-independent raw sample is in
`original_target_bridge_raw_sample.json`.

## Updated claim ceiling

> `EXPLORATORY_NONAUTHORITATIVE`: On exactly the 12 inherited manufactured
> vector trial systems and the inherited nonidentity-mass/strong-nonnormal
> small explicit system, the opt-in common-W correction was reconciled against
> the unchanged original residual and original Jacobian at the identical trial
> K. The direct/decomposed bridge identity, inherited small-system backward
> error, and condition-aware same-target correction criteria passed, with
> complete diagnostic work accounting and a preserved original-action failure.
> This is compatibility evidence only. `BudgetNotSpecified` forbids an
> accuracy PASS. It authorizes no nonlinear/output certification, production
> activation, generalization, scalable-backend, timing, ranking, speedup,
> holdout, freeze, PM-7, K0, tag, or release claim.

The one fresh-context review initially returned `FAIL` for one P1 retained-data
defect and two P2 evidence/coverage gaps. One bounded repair added a partial
result payload, the late-failure and original-Jacobian negative contracts, and
the detailed work profiles. The post-repair targeted suite and readiness check
pass. No second fresh review or review-of-review is claimed; see
`evidence/original_target_bridge_fresh_review_disposition.md`.

## Validity separation

- `RESULT_VALIDITY`: supported only for the 13 declared small explicit trial
  systems and the specified linearized diagnostics.
- `PROVENANCE_VALIDITY`: bound to the inspected delivery and the published
  stacked-PR diff; the old 54 rows are diagnosis-only historical material and
  were neither rerun nor changed.
- `PACKAGING_VALIDITY`: established only when the draft stacked PR records its
  actual head/tree, changed paths, checks, and synchronization status. A ZIP or
  its byte identity is not a numerical acceptance condition.
