# Scientific Validity v2 — External Re-audit Bundle

This is a bounded, failure-preserving evidence package for independent re-audit of
VigilODE scientific-validity-v2 at implementation revision
`ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04`. It does not change the scientific result or
admit a performance, ranking, equal-error, production-baseline, or publication claim.

## Start here

1. `claims/CANONICAL_EXECUTION_EVIDENCE.md` contains the detailed execution result.
2. `claims/CLAIM_SCOPE_AND_INVALIDATION.md` is the updated claim ceiling.
3. `claims/PROTOCOL.md` states the pass-only freeze and holdout protocol.
4. `rust/calibration_all_cases_compact.json` retains detailed metadata, rows, metrics,
   counters, diagnostics, bindings, and checksums for all 54 cases. Only the four large
   per-arm state/output-time arrays are omitted.
5. `rust/selected_raw_n96_rtol1e-8.json` retains complete Rust records, including both
   clipped and dense state trajectories, for a six-family raw slice.
6. `reference/selected_raw/` and `external/selected_raw/` contain the exact matching
   reference, SciPy Radau, and typed-unavailable CVODE artifact bytes.

## Result and claim ceiling

- Rust calibration: 54/54 cases completed, zero execution failures, 54/54
  `output-policy-dominated`.
- Freeze: not created by the pass-only protocol.
- Oregonator: `NOT_RUN_BY_PROTOCOL`; the sealed holdout was not opened.
- External calibration: SciPy Radau 54/54 success; CVODE 54/54 typed unavailable; zero
  comparator solver failures.
- Claim disposition: `NOT_ADMITTED` for output-policy equivalence, equal-error ranking,
  scaling, production-baseline comparison, endpoint-contamination certification, and
  publication readiness.

The standalone claim copies are byte-identical to the authoritative tracked documents in
the parent directory. `SHA256SUMS` binds both the claim copies and the included evidence.

## Fixed outcome-blind raw selection rule

The raw subset is selected by the fixed rule in `SELECTION_RULE.json`:

```text
partition == calibration
dimension == 96
rtol == 1e-8
family in all six calibration families
```

This predicate reads only corpus metadata: `spec.partition`, `spec.dimension`,
`spec.rtol`, and `spec.family`. It does **not** inspect or filter on row status, error,
clipped/dense discrepancy, work counters, wall time, checksums, comparator result, or any
other numerical outcome. The rule was fixed for this re-audit package after campaign
execution; it is outcome-blind packaging selection, not a claim of pre-campaign
preregistration. Selected records are serialized in ascending `case_id` order; ordering
also does not inspect outcomes.

The rule keeps the package below its size ceiling while retaining every calibration
family and full state trajectories at the smallest declared dimension and tightest
declared tolerance. This raw slice is family-complete but is **not** raw-data coverage
across dimensions or tolerances. Cross-dimension and cross-tolerance evidence is retained
only in the all-54-case compact file through metrics, diagnostics, work, bindings, and
output checksums.

## Byte identity and semantic extraction

- The claim copies, reference manifest, selected reference artifacts, external aggregate,
  selected external artifacts, runtime probe, and operational logs are exact byte copies.
- The Rust compact and selected-raw JSON files are deterministic semantic extractions from
  the 260,879,060-byte source campaign. They are not claimed byte-identical to the source
  file. Their records retain the original per-artifact scientific checksums.
- The omitted source campaign is bound by SHA-256
  `afbdbfb032a27b9d4ce8189a489a4ffb5745e096a53bed4349d90f6a780db80c`.

## Verify

From this directory:

```bash
sha256sum -c SHA256SUMS

jq -e '
  .campaign.status == "complete-nonpassing" and
  (.rows | length) == 54 and
  ([.rows[].status] | all(. == "output-policy-dominated")) and
  (.records | length) == 54
' rust/calibration_all_cases_compact.json

jq -e '
  .selection.rule_id ==
    "corpus-metadata-only-n96-rtol1e-8-all-families-v1" and
  (.records | length) == 6 and
  ([.records[].artifact.spec.family] | unique | length) == 6 and
  ([.records[].artifact.spec.dimension] | all(. == 96)) and
  ([.records[].artifact.spec.rtol] | all(. == 1e-8))
' rust/selected_raw_n96_rtol1e-8.json
```

`MANIFEST.json` records the inclusions, omissions, identity classes, and size ceiling.

## Deliberate omissions

- The 260,879,060-byte full Rust campaign is not committed.
- State trajectories for the other 48 Rust cases are not committed.
- The other 16 physical reference artifacts and 96 external comparator artifacts are not
  committed; their identities and aggregate statuses remain in the complete manifests.
- No freeze or Oregonator artifact exists, because the calibration did not pass.
- Wall time is diagnostic only and supports no performance claim on this host.

These omissions are packaging limits, not missing pass evidence. The canonical campaign
remains nonpassing.
