# Constructive promotion theory, 2026-10-11

Start with [the Korean integrated report](../../docs/reviews/20261011_promotion_theory/REPORT_KO.md).

This package supplies conditional proofs, current-source applicability boundaries,
exact illustrative diagnostics, independent decision reviews and concrete porting
contracts. It makes no new native implementation, historical rerun, measured
wall-time or automatic production promotion claim.

Machine-readable entry points:

- `THEOREMS.json`: independent theorem decisions, assumptions and proof pointers.
- `DECISION.json`: scope-separated final decision, especially theory versus executable.
- `PORTING_DAG.json`: proposed native extension steps, tests, dependencies and gates.
- `evidence/REUSED_RESULTS.json`: all103published ledger rows and supersessions.
- `SOURCE_BINDING.json`: immutable source and harness identities.
- `CORRECTIONS.json`: actual adversarial-review findings and corrections.
- `REVIEW_INPUT_BINDINGS.json` and `PATH_ALIASES.json`: reviewed bytes and delivery mapping.
- `LITERATURE.json`: primary-source provenance and actual read scope.
- `ARTIFACT_SCHEMA.json`: portable JSON Schema for the theorem index.
- `VALIDATION.json`: delivery consistency checks; not a theorem proof.

Candidate-author JSON files under `evidence/` preserve their original bytes,
including pre-decision `PENDING` fields. The independent reviews and `DECISION.json`
are the final adjudication; author self-descriptions never supersede them.
Original draft path names resolve through `PATH_ALIASES.json` and the explicit
`REVIEW_INPUT_BINDINGS.json` mapping. Reports were relocated without changing the
reviewed proof bytes.

Optional reproduction of **new illustrative diagnostics**, from repository root:

```sh
python3 research/promotion_theory_20261011/check_exact.py
python3 research/promotion_theory_20261011/evidence/POLYNOMIAL_EXACT_CHECK.py
python3 research/promotion_theory_20261011/evidence/check_global_exact.py
python3 research/promotion_theory_20261011/evidence/homotopy_exact_check.py
python3 research/promotion_theory_20261011/validate_package.py
```

The Fraction scripts illustrate exact identities and work examples; finite
fixtures do not prove universal theorems. General proofs were checked separately
by independent reviewers. The scripts are not production authority constructors
or a substitute for outward Rust implementation. Wolfram symbolic results are
recorded in `evidence/WOLFRAM.json` and the global diagnostic evidence.

The actual represented K-form has previously recorded noncausal coefficient
leakage. Exact nilpotence/no-fold results apply to explicitly causal targets.
H5 supplies a conditional actual-target alternative using `B+E` and verified
`||(I-B)^-1 E||<1`; no current B/E/q values were measured in this theory run.
Do not silently zero entries or interpret a pending source binding as a PASS.

Publication is a research-only addition on a separate branch. Existing native
preregistrations and the append-only experimental ledger are unchanged.
