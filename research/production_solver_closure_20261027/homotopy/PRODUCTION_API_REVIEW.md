# Independent production API review

Reviewer: /root/production_homotopy; source-reading and control-flow review.
No native execution performed by this reviewer; parent owns the actual contracts.
Scope: provider preflight, JSON completeness, failure/partial-work preservation,
fixture executable.

## Finding P-H1 — production Options decoding is permissive

Status: open at this reviewed snapshot. `ProductionConfig` denies unknown top-level
fields and `SwitchingConfig` rejects unknown nested controls, but its `options`
member uses the legacy `Options` deserializer. That type lacks deny_unknown_fields
and gives `interior_error` a serde default. Consequently a misspelled options key
can be silently ignored, and omitting interior_error supplies a native default.
The adjacent-tagged Atol object also lacks a strict unknown-field contract. This
conflicts with the new deployment type's complete-without-implicit-defaults claim
and can hide a configuration mistake.

Minimal correction: use a production-only strict Options/Atol decoding DTO,
requiring all fields and refusing unknown nested keys. Preserve the legacy
Options decoding behavior for existing clients. Add contract assertions for an
unknown options key, missing interior_error, and an extra Atol key.

## Reviewed paths without an additional finding

- Schema and derivative permission checks precede callback dispatch. ProvidedOnly
  is restricted to a supplied JVP; the underlying adapter additionally refuses
  missing analytic f_t on nonautonomous problems and unsupported outputs/mass.
- Finite-difference policy is explicitly refused. Forbidden retains an RHS-only
  path even when unusable derivative callbacks exist.
- Selective probes require the explicit replay-safe declaration and strict
  entry/return comparisons with positive pilot budget.
- The wrapper directly returns the native SwitchingSolution, preserving accepted
  prefix, stop message, counters and accounting_complete. It does not relabel
  an unsuccessful native result as success.
- The executable validates a finite positive rate, emits the full returned
  solution on a fixture failure, and checks actual atomic RHS/JVP counts and
  a fixed exact-solution cap before returning process success.

## Reviewed source identities

```json
[
  {
    "path": "crates/vigilode/src/production.rs",
    "sha256": "1de7bce4b5190c9b9457c95b2c30d5fbc6c854ce0628d7c120a3ead282308b9a"
  },
  {
    "path": "crates/vigilode/tests/production_setup_contracts.rs",
    "sha256": "3fba1f680d9038c2aa9d0a4614551ffa229f576fdbb270c9848c7f0e0a17b199"
  },
  {
    "path": "crates/vigilode/examples/production_from_json.rs",
    "sha256": "d708f829ce324df67dc8478051476622bbc172fb6a3bc293a1b95ed3ea3f1cad"
  },
  {
    "path": "crates/vigilode/src/options.rs",
    "sha256": "b0355bd34c7038ff18fd37e15315835b8bc6fb4b01e7e0a59308e1f779269b1e"
  },
  {
    "path": "crates/vigilode/src/switching.rs",
    "sha256": "ebea1dd7b76da45def13f6d9e1137f693786f23308f2e0cc589debc82b658347"
  }
]
```

## P-H1 source-level closure

Parent replaced production-only Options decoding by StrictOptions/StrictAtol,
with every options member required and unknown keys refused recursively.
The legacy Options/Atol implementations are unchanged. The existing JSON
contract now includes the options typo, omitted interior_error and extra Atol
key. Independent reread finds the minimal correction addresses the stated
defect. Source review is closed; the parent native test run is still pending
at this closure record.

```json
[
  {
    "path": "crates/vigilode/src/production.rs",
    "sha256": "f46d917cb8205799840b38a5ae8b310b0e785969b5c482b478adfc557a89888c"
  },
  {
    "path": "crates/vigilode/tests/production_setup_contracts.rs",
    "sha256": "ec0c0607f04bd66183e3eded0145bfe6401fab42af8595f12d68f44668818a97"
  },
  {
    "path": "crates/vigilode/src/options.rs",
    "sha256": "b0355bd34c7038ff18fd37e15315835b8bc6fb4b01e7e0a59308e1f779269b1e"
  }
]
```

## P-H1 native closure

Parent executed production_setup_contracts with the reviewed strict DTOs. All
5 distinct contracts passed, including the expanded nested JSON refusal cases.
Actual execution: evidence/production_contracts_pass0/RECEIPT.json, with exit0
and source_unchanged_during_command=true. The receipt source manifest includes
both reviewed current hashes below. P-H1 is closed at source and native-contract
levels; no additional finding remains in this reviewed scope.

```json
{
  "crates/vigilode/src/production.rs": "f46d917cb8205799840b38a5ae8b310b0e785969b5c482b478adfc557a89888c",
  "crates/vigilode/tests/production_setup_contracts.rs": "ec0c0607f04bd66183e3eded0145bfe6401fab42af8595f12d68f44668818a97"
}
```
