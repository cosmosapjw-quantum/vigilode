# Execution-transition and anti-bureaucracy policy

## Objective

Move from validated planning into scientific coding and runtime evidence without allowing audit machinery, packaging, or checksum bookkeeping to become the dominant work product.

## Rules

1. A newly observed defect gets one root-cause analysis, one controlling invariant/test, one bounded repair, and one existing review path. Do not create a review-of-review or a parallel assurance framework.
2. Representation-only repairs reuse the existing WU-05 fresh repair review and final differential audit. They do not trigger a new campaign or restart WU-00 through WU-04.
3. A campaign rerun is authorized only by a change in equations, numerical parameters/tolerances, kernel routing, convergence decision, stage work, numerical output, or an explicitly missing scientific observation that cannot be recovered from preserved raw evidence.
4. A package/manifest/archive SHA change alone is a control-plane event, not a scientific rerun trigger.
5. Missing historical telemetry is recorded as missing. It is never fabricated merely to satisfy a newer schema.
6. After a bounded representation repair passes its direct regression tests, execution returns immediately to the existing source findings and runtime DAG.
7. Fail closed for genuine content ambiguity; do not fail closed for equivalent serialization or renamed labels when numerical semantics can be derived exactly.
8. Preserve failed attempts and negative evidence, but do not multiply gates that detect the same failure class.

## Current application

`WU05-NEW-P0-001` is a representation/validator false-fail. The repair stays within evidence-v3 semantics, keeps the existing `EVIDENCE_V3_PASS` marker, adds no review layer, does not modify raw WU-04 bytes, and does not authorize a campaign rerun.
