# C5 authored implementation — not executed

Unit: `V8_SOURCE_CHEMICAL_JACOBIAN_REMAINDER_20261028`.

## Source-to-code mapping

`chemical_polynomial.py` contains only real-arithmetic algebra on supplied coefficients. It never imports the five preserved owner files in `deps/source_bytes` and never initializes an atomic model, evaluates rates, calls a native RHS/Jacobian/norm/lift, runs an ODE/NCP, or builds native code.

`polynomial` builds b/P/Q from the pinned adjacent transition topology on HII, HeII, HeIII, LiII, LiIII, LiIV. The first photo row of each element retains its neutral offset and subtraction from every charged coordinate. The hydrogen total is exactly `(2**53-1)/2**53`, not one. The twelve effective rates and three positive frozen inventory weights are exact supplied inputs with source/clock binding and provenance. `evaluate`, `jacobian`, `hessian_action` and `quadratic_remainder` implement the frozen contract, retaining the electron-feedback outer product.

`jacobian_extrema` constructs the 24 exact product-simplex vertices. At each vertex it derives the twelve linear rate coefficients of each entry using unit-rate polynomial models, chooses rate endpoints separately according to the coefficient signs, and retains the winning exact values and full witnesses. It returns all 36 intervals and 72 attaining state/rate witnesses. Its center6/radius6 objects remain chemical partial blocks. A full-model-provider scope request is refused. Geometry columns/rows, tau/Thomson rows, clock/rate variation and callback rounding remain OPEN_NOT_ZERO.

`remainder_bound` embeds only the named frozen polynomial coefficient operator Qbar and the electron covector. It reuses unchanged next7 Gram/inverse, sparse matrix multiplication, generalized PSD eigenvalue upper and scale-aware scalar square root. It returns Gamma and the Hessian factor2Gamma solely for this polynomial remainder. A zero Q returns zero. It does not declare missing full Jacobian blocks zero, generate a full model provider or implement interval-rate remainder generalization.

`reconstruct_inventory_RN` statically parses pinned atomic source literals and JSON numeric literals. It reconstructs RN binary64 ratios, mass products, exact-fsum-rounded normalization and divisions, preserving hex traces and exact element-weight sums. It labels the record SOURCE_RECONSTRUCTED_UNDER_RN, with original owner scale-array observation and source_manifest recovery both false. No owner code or initialization executes.

## Independent controls: ten groups

The suite in `tests/test_chemical_polynomial.py` independently assembles conservative edge fluxes and their nine-state derivatives, then applies neutral elimination. It never obtains its expected F/J from production P/Q/J/e or the endpoint-sign optimizer.

1. Independent nine-state F/J comparison at physical vertices, diagnostic interior points and signed polynomial trials.
2. Hydrogen scalar oracle and an intentionally omitted feedback term with exact nonzero discrepancy; saved diagnostic feedback curve.
3. Conservation, inward boundary fluxes, exact neutral offsets and the non-unit hydrogen total.
4. All36 entry intervals and72 witnesses checked for simplex/rate-box membership and independent derivative attainment.
5. Three-variable independent exhaustive rate-corner subset:8corners×24state vertices; all36 intervals compared. Fixed zero-width rates checked separately.
6. Four interior exact family controls, with continuum conclusion explicitly assigned to the reviewed affine vertex proof.
7. Signed exact remainder, Hessian polarization/symmetry, degree-two scaling, and saved H-only scaling diagnostics.
8. Unchanged metric producer plus independent nine-term chemical quadratic oracle. Unit weights give sigma²495/2; H-only Q with kappa3/2 gives sQ9/4 and full-positive-electron Gamma²4455/8. The restricted one-dimensional H sharp coefficient is separately identified. Uniform scale2 changes sigma² by4, leaves sQ fixed and scales the inherited dyadic root enclosure by2. General signed remainder/Hessian inequalities and zero Q are checked.
9. Source-reconstructed RN hex/provenance and exact rational serialization controls.
10. Shape/rate/total/weight/binding/provenance/resource/serialization/scope refusals.

`code/EXACT_CHECKS.wl` is authored input for one later time-limited evaluator call. Its24 controls use an independently assembled nine-state flux, exact differentiation/polarization, a real omitted-feedback discrepancy and six-dimensional rational Gram/dual controls. It is not an evaluated result or actual model enclosure.

`plot_saved_results.py` reads only first-suite saved JSON and creates a named H feedback omission panel and exact quadratic-scaling panel. Diagnostic effective rates, synthetic weights, source restriction, non-unit H total and absence of actual trajectory data are labeled explicitly.

## Reuse and execution

The unchanged previous matrix module and its exact dependency tree are under `deps/prior_matrix`; identities are in `deps/PRIOR_MATRIX_IDENTITY.json`. Only source data bytes are copied under `deps/source_bytes`, with source metadata in `deps/SOURCE_BYTE_IDENTITY.json`. No old tests are copied or run. Prior source and budgets remain closed.

The bounded runner preserves first source snapshots, raw stdout/stderr, first result JSON and measurements, with exclusive one-invocation markers. A matching root GO file and the actual preexecution review/backed-up checkpoint are required. No automatic retry is implemented.

Proposed commands from the unit root:

```bash
python -B code/run_bounded.py suite --go SCIENCE_EXECUTION_GO.json
python -B code/run_bounded.py figure --go SCIENCE_EXECUTION_GO.json
```

New total local scientific cap: CPU60s/wall90s. Suite CPU45s/wall60s; figure CPU10s/wall15s; each process768MiB. One Wolfram evaluator call has outer20s/internal17s with full raw response and tool wall saved; remote CPU remains unknown/null. All actuals are null before execution. Packaging is separate. Requested coding model is gpt-6.1-sol; observed runtime metadata is UNOBSERVED_RUNTIME_METADATA.

At this C5 checkpoint no scientific imports, tests, CAS, plots, model/native calls or builds have executed. All tests and symbolic expressions are authored expectations, not observed passes.
