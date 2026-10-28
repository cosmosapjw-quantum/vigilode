# Scientific Validity v2 Freeze and Holdout Protocol

## Authority boundary

This protocol is `IMPLEMENTED` as a deterministic gate and source-bound CLI. The
checked-in smoke JSON files are synthetic wiring fixtures labelled
`ci-smoke-nonauthoritative`; they are not calibration, holdout, benchmark, or publication
evidence. The canonical producer was executed at revision
`ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04`: all 54 cases completed, all 54 rows were
`output-policy-dominated`, no freeze was created, and the Oregonator holdout remained
unopened by protocol.

No v3.5, v3.6, v3.7, V25, V36, or A1 threshold is an input to this protocol.

## Predeclared sets

| Profile | Calibration input | Holdout replay |
| --- | --- | --- |
| `smoke` | exactly six rows: all six calibration families at `n=96`, `rtol=1e-4`, `atol=1e-6` | exactly one Oregonator row at `rtol=1e-4` |
| `canonical` | exactly 54 rows: six families times `n={96,384,1536}` times `rtol={1e-4,1e-6,1e-8}`, with `atol=0.01*rtol` | exactly three Oregonator rows, one at each declared tolerance |

Oregonator is the only predeclared replay family. Pollution, Medical Akzo, and
Brusselator 2-D remain sealed. Neither public gate function nor CLI accepts a family
selector.

## Typed measurement rows

Every row records one of:

- `pass`
- `fail`
- `reference-dominated`
- `output-policy-dominated`

`conservative_max_wrms` is optional so a failed or dominated measurement can be retained
without a JSON NaN/Infinity sentinel. A `pass` row must have a finite nonnegative value.
Every row must retain nonempty evidence text. `wall_seconds` is diagnostic only.

Every row also carries a mandatory source binding. All rows in a freeze/replay share one
campaign identity containing the authority, runner schema, candidate ID, code revision,
solver-configuration checksum, WRMS-scale checksum, and output-policy-protocol checksum.
Each row separately binds the numerical reference and distinct clipped/dense output
artifacts. All artifact identities are lowercase SHA-256 values. A canonical profile
accepts only authority `canonical-v2-runner`, schema
`scientific-validity-v2-campaign-runner-v1`, and a 40-digit lowercase hexadecimal code
revision. Bare or mixed-campaign rows are rejected before a freeze can acquire authority.
The smoke profile accepts only its explicitly synthetic authority and schema. A binding and
SHA-shaped strings are not authentication by themselves: canonical authority additionally
requires the source-bound producer to validate all 54 complete case artifacts and emit the
campaign aggregate and freeze in the same invocation.

## Calibration freeze

The freeze rejects missing, duplicate, unexpected, metadata-mismatched, holdout, failed,
or dominated calibration rows. Only after all expected rows are `pass` is the frozen
threshold derived as

```text
max over eligible calibration rows of conservative_max_wrms
```

The payload stores the exact threshold bits and derivation ID
`scientific-validity-v2-conservative-max-wrms-v1`, the predeclared Oregonator family, and
the three sealed families. It also copies the common campaign binding. Rows are
canonically sorted. The SHA-256 binds all scientific fields, source bindings, and evidence
but excludes wall time, so input ordering and operational timing do not change the
scientific checksum.

## Oregonator replay

Replay verifies the complete freeze and its checksum before inspecting holdout input. It
then admits only the exact profile-specific Oregonator set. Each result preserves the
input status and evidence, adds `within_frozen_threshold`, copies the frozen threshold
bits and calibration checksum exactly, and derives `overall_pass` without modifying the
freeze. A `fail` or dominated row produces a replay artifact with `overall_pass=false`;
the CLI preserves that artifact and exits nonzero.

## CLI

All commands use atomic create-new file admission at the filesystem boundary: an existing
output is never overwritten. Raw-row commands are deliberately limited to synthetic smoke
wiring. They reject `--profile canonical` before opening any input.

```text
rodas5p scientific-validity-v2-freeze \
  --profile smoke --input CALIBRATION_ROWS.json --output FREEZE.json

rodas5p scientific-validity-v2-holdout-replay \
  --profile smoke --freeze FREEZE.json \
  --input OREGONATOR_ROWS.json --output REPLAY.json

rodas5p scientific-validity-v2-run-calibration \
  --reference-manifest REFERENCES.json --output CAMPAIGN.json \
  --freeze-output FREEZE.json

rodas5p scientific-validity-v2-run-oregonator \
  --profile canonical --freeze FREEZE.json \
  --calibration-campaign CAMPAIGN.json \
  --reference-manifest REFERENCES.json --output OREGONATOR.json
```

Smoke protocol-shape or freeze failures produce a create-new failure JSON containing the
input rows and then exit nonzero. A CLI parse failure, any attempted `--family` selector,
or a canonical raw-row request creates no authority output. The canonical producer publishes
the complete campaign first and the freeze second; a nonpassing 54-case campaign remains
preserved but produces no freeze. Oregonator validates both the freeze and its complete
campaign/artifact lineage before opening the holdout reference.
The external SciPy/SUNDIALS Oregonator lane requires the same campaign file in addition to
the freeze and includes the freeze semantic checksum plus the SHA-256 of the exact campaign
file bytes it parsed and validated in its scientific-set checksum.

## Checked-in smoke fixtures

- `CI_SMOKE_FREEZE_FIXTURE.json`: six synthetic passing calibration rows; envelope
  checksum `c6485e5b964c3d733a5c3b9abe7ebd59237b81b1b8f981a33fc52429c0b1aa03`;
  file SHA-256 `45c4629a5d76d07b1c53209df7e5271f0bfa4778dc71e6107388bf692bf62f90`.
- `CI_SMOKE_OREGONATOR_REPLAY_FIXTURE.json`: one synthetic passing Oregonator row;
  envelope checksum `0ad1cac03b459f9034d6c394eb15ffeff414ba4c9b5809fa9268f2ac26d0ac49`;
  file SHA-256 `80b7fe6fed0146c8c7921f93b4ba4c21b0640d2a2f23898ce4ced5ba988f3332`.

The synthetic threshold `0.06` exists solely to test serialization, hashing, and replay
wiring. It is forbidden as a scientific or production threshold.

## Canonical execution state — 2026-08-29

- Implementation revision: `ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04`.
- Complete reference manifest: 22 artifacts and 66 bindings; file SHA-256
  `e272f32f3e235f506cfa59060846a54e20195ea4d991800c0706645c3fb78da2`.
- Rust campaign: `complete-nonpassing`; 54/54 attempted, zero execution failures,
  54 `output-policy-dominated`; campaign file SHA-256
  `afbdbfb032a27b9d4ce8189a489a4ffb5745e096a53bed4349d90f6a780db80c`.
- Freeze: absent by pass-only admission, with no frozen threshold.
- Oregonator: `NOT_RUN_BY_PROTOCOL`; the holdout reference was not opened.
- External calibration: 108/108 records; SciPy Radau 54/54 success, SUNDIALS CVODE
  54/54 typed unavailable, zero solver failures; aggregate status
  `full-surface-with-unavailable` and scientific-set SHA-256
  `ca13d5911f45e8421ce1deefc6604862bfe7fd3488fdf3eafa87bdedeca61a84`.

The raw campaign remains failure-preserving evidence. No threshold or tolerance was
changed, and the canonical numerical campaign was not retried. A future campaign must use
new declared output paths and authority after correcting the output-policy disagreement;
it may not overwrite or reinterpret this result. Only a future passing 54-row campaign
may freeze exactly once and then open the three Oregonator measurements. Holdout results
may not feed back into the threshold or checksum. Claim admission still requires a
separate review of any passing artifacts; execution alone is insufficient.

The WRMS inner policy is a per-stage true-residual heuristic. Its `0.1` allocation does
not bound endpoint contamination unless an independent bound on `W^-1` is available.
Matrix-free recycle images are refreshed for every new linearization identity; approximate
cross-step reuse for merely nearby Jacobians is intentionally not admitted without an exact
stable identity or quantitative operator-change certificate. Refresh applications remain
included in total operator/JVP work.
