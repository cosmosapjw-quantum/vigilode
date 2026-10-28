# Executed fixture configurations

These two files were emitted by `production_from_json --template` and then
consumed by the same native executable in four successful smoke runs.
Their source is `../evidence/production_smoke_pass0/RECEIPT.json`.

The numerical values describe the n16 linear-decay fixture on [0,1]. Replace
physical tolerances, step proposal, total attempt cap and observation windows
with the actual client's contract. The full operational schedule is explicit.
No JSON field is silently defaulted by the production decoder.

`rhs-only.json` requests no derivative callback. `selective-jvp.json` enables
only a supplied JVP and exact same-window work comparisons. Observed-step
reuse remains false and productive sentinels are not universally enabled,
because adverse nonlinear results are preserved in the report.
