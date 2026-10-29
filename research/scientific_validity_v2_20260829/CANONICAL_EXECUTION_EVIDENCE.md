# Scientific Validity v2 — Canonical Execution Evidence

## Authority

- Date: 2026-08-29 (Asia/Seoul)
- Base: `8d0c79184e09efb5bdadc24a6315c60a71a44264`
- Implementation revision: `ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04`
- Clean-bound release binary SHA-256:
  `b680f915eaed8fb07e153ecf0f6aeef4a224acad8254e8e1f89cf4fbee82a548`
- Local raw-evidence root: `/tmp/vigilode-v2-ab8fbcdb.SfqTVy`

Wall times are operational diagnostics only. They are excluded from scientific checksums
and support no performance or ranking claim on this non-quiescent host.

## Reference authority

- Manifest status: `complete`
- Manifest file SHA-256:
  `e272f32f3e235f506cfa59060846a54e20195ea4d991800c0706645c3fb78da2`
- Artifacts / bindings: 22 / 66
- Resumed artifacts / generation failures: 0 / 0
- Artifact-set SHA-256:
  `402e389b19f8b1570eda68850b54d88c9fffd14d1e526c538b48e27b9988846b`
- Binding-set SHA-256:
  `8141617a9af77cec68095aba21e487b903fafbe082d90335d768fbebc7d1893e`
- Reference generator SHA-256:
  `0ac0dbe76fb0ec9b598725db3cf90c8f527c291782875616c3920cf3842535fa`

## RODAS5P canonical calibration

- Exit: 1 after a complete nonpassing campaign
- Campaign status: `complete-nonpassing`
- Attempted / expected / execution failures: 54 / 54 / 0
- Artifact records: 54 `complete`
- Gate rows: 54 `output-policy-dominated`; no pass, fail, or reference-dominated rows
- Campaign file size: 260,879,060 bytes
- Campaign file SHA-256:
  `afbdbfb032a27b9d4ce8189a489a4ffb5745e096a53bed4349d90f6a780db80c`
- Record-set SHA-256:
  `73aeb3e9cdab1f4c59acb584cc0df2991106e144d7c201cd228ff7c52c27d8af`
- Clipped/dense discrepancy range: 0.22414998248334056 to 397777.80860599974 WRMS
- Discrepancy / dense-error ratio range: 0.12454170289388176 to
  1.0941303386549561; every row strictly exceeded the frozen 0.1 dominance limit
- Freeze eligibility: false
- Freeze admission error: all 54 validated calibration artifacts must pass before freeze
- Freeze: absent
- Oregonator: `NOT_RUN_BY_PROTOCOL`

This result is not a solver-execution crash: every case and both output arms completed.
It is a scientific output-policy invalidation. No threshold was tuned and no numerical
retry was made.

## External comparator calibration

- Exit: 3 (`unavailable` retained)
- Aggregate status: `full-surface-with-unavailable`
- Cases / records: 54 / 108
- Surface covered / complete: true / false
- SciPy Radau: 54 success
- SUNDIALS CVODE: 54 typed unavailable
- Solver failure / not-run / non-applicable: 0 / 0 / 0
- CVODE capability finding: absent from executables, pkg-config, headers, dynamic
  libraries, and Python bindings; the host exposes an IDA-only SUNDIALS 6.4.1 runtime
- Aggregate file SHA-256:
  `e5b6ac1604c2e504839925c64b137422e5cc6eb6ec6be084e3d894d67daba4f7`
- Scientific-set SHA-256:
  `ca13d5911f45e8421ce1deefc6604862bfe7fd3488fdf3eafa87bdedeca61a84`
- Runner dependency-closure SHA-256:
  `bde3b015c2ec18b4058fa15a88b138baf3edb6d913a6df38b6b1d1cb2ae5e40c`
- External runner script SHA-256:
  `f3124f15505dafde3995ed9b80a8ee5fc056e24621d96d0ec2fef3565766e10a`
- External Oregonator: `NOT_RUN_BY_PROTOCOL` because no Rust calibration freeze exists

The typed CVODE absence is a host-capability limitation, not a fabricated comparator row
or a numerical solver failure. The incomplete external surface supports no production
baseline claim.

## Preserved build-state divergence

The first release binary was built after a Python preflight had generated an untracked
`__pycache__`. Its build script correctly embedded `source_dirty=true`; the canonical
producer rejected it before opening the reference manifest.

- Rejected binary SHA-256:
  `22dd9a4a99d915f71ee4da791a218bb6c0e71859386e607470ba2ab73d81ffb9`
- Failure artifact SHA-256:
  `4a68fdef6cb0a328907e94d40e925ff09c6c85b8c27bbacf99fbeb94aa1bd82d`
- Failure status: `failed-preflight`
- Reference manifest accessed: false
- Error: canonical v2 runner refuses a dirty source overlay

The failure was preserved. The source was cleaned without changing tracked bytes, then a
new target directory produced the clean-bound binary above. The 54-case campaign was the
first numerical execution admitted by the canonical source preflight.

## Claim disposition

Validated here: reference generation completed; source and artifact bindings held; all
54 Rust cases executed; the pass-only freeze correctly withheld authority; external
unavailability was typed and retained.

Not admitted: an Oregonator result, a frozen scientific threshold, output-policy
equivalence, equal-error ranking, scaling, production-baseline comparison, endpoint
contamination bound, or publication readiness.
