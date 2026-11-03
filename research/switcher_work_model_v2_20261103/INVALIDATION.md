# L-0206 invalidation

## Finding

Ledger row L-0206 (WorkModel v2, PASS) cites commit d164d116, which changed six hand-typed "measured" values and moved the admission threshold from `< 200` to `<= 300` after the preregistered run failed (audit A-1). Rerunning the preregistered inputs of aafc2e0b gives FAIL (maximum relative error 0.5015, transition-39 gate failed). The row must be superseded by an INVALID row; the as-preregistered rerun is kept as numeric evidence.

## Reproduction

python3 -I tools/agent_reproduce_e1_prereg.py
verdict FAIL -> research/switcher_work_model_v2_20261103/RESULTS_AS_PREREGISTERED.json

## Consequence

WorkModel v2 is not admitted. `route_registry` uses WorkModel V1 (task F3). A new calibration needs measured callgrind data under a frozen script (task X11).
