# Loop12 integration and positive theory upgrade

Start with [REPORT_KO.md](../../docs/reviews/20261011_loop12_upgrade/REPORT_KO.md).
The full original attachment is retained in input/rvj_independent_loop12_20261011.zip.
Its selected readable files are byte-identical extracts; input/IMPORT_INDEX.json
identifies that partial view. The original author's HOLD and failures are preserved.

Authority order:

1. DECISION.json plus the actual independent reviewer records define approved scope.
2. THEOREMS.json links claims to proofs, assumptions and those decisions.
3. PORTING_DAG.json defines proposed native work, with current source in SOURCE_BINDINGS.json.
4. Imported author records are historical evidence, not the final decision of this integration.

The preserved evidence/CLAIMS_DAMPING.json and evidence/CLAIMS_RANK.json are the
authors' pre-decision submissions and retain PENDING fields. Read DECISION.json
and THEOREMS.json for the later independent adjudication; do not rewrite those
original submissions to manufacture an earlier approval.

Previous promoted results remain at ../promotion_theory_20261011 and are not rerun.
The new proofs separate algebraic operation counts, scalar precision, parallel span,
native runtime and actual global trajectory certificates.

New executable research artifacts:

- causal_weights.py: exact beta-weight table compiler and causal convolution.
- check_causal_weights.py: small exact checks of that new prototype.
- evidence/check_rank_exact.py and evidence/RANK_EXACT_CHECKS.json: new rank-2 identities.
- validate_package.py: identity/schema/path/DAG checks; not a theorem prover.

Run the new coefficient check with:

~~~bash
python research/promotion_loop12_20261011/check_causal_weights.py
~~~

Run package validation with:

~~~bash
python research/promotion_loop12_20261011/validate_package.py
~~~

No native Rust source, dependencies, production defaults, prior preregistration or
published campaign results are modified by this package. Native claims require
the selected route's actual implementation and gates, described in
[NATIVE_HANDOFF_KO.md](../../docs/reviews/20261011_loop12_upgrade/NATIVE_HANDOFF_KO.md).
