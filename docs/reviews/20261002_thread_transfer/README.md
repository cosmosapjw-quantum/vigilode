# Scoped vigilode thread-transfer review

Read `REVIEW_KO.md`, `REVIEW.json`, and `IMPLEMENTATION_PLAN.md` first.

Reproduce the exact-real review probes, without network access or Rust:

```sh
python -m pip install -r requirements.txt
python run_review.py
```

Python 3.11 or later is required. Output is written under `results/` relative to this directory. The 54 tests include scientific counterexamples and invalid-input rejection. Their success does not promote a solver or timing claim.

`inputs/target_bits.json` is an explicit selection from the pinned native fixture, not a byte-for-byte copy of the whole fixture. Its intervals are preserved. `SOURCE_MAP.json` distinguishes the repository evidence, current execution, and prior attached thread evidence.

No production source, protected R-JF, coefficient file, holdout, existing ledger row or timing authority was changed. Native Rust validation and performance campaigns are future work with commands in the implementation plan.
