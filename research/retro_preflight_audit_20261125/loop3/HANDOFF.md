# Audit loop 3: benchmark metrics and finite experiment harness
Two red tests reproduced adjacent-float log-bracket aliasing and representable
WRMS overflow (exit 101); fixed, 7 standalone metric tests and the actual
benchmark crate metric tests passed. Three independent reference-policy
contract tests passed. Existing work-precision records unchanged.
New facade preparation fixtures span five topics, 360 cells (312 actual
computations + 48 expected refusal controls). Full output/exit and source
hash receipts are in smoke and smoke_final; attached mhgext validates run
manifests with artifact-byte checking. Exact algebra evidence is independent
of implementation evidence. This is not external solver qualification.

M0 preparation locks historical data and materializes exact grids without
solving. New reference-plan and references commands separate planning and
reference-only execution from wp. M0 inputs and protocol are in ../m0.
