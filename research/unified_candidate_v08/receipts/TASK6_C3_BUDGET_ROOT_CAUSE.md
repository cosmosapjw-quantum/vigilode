# C3 budget gate root cause

The first implementation stored a policy rejection message in `failure`.  The existing
result contract intentionally reserves `failure` for execution/certification failures and
requires a scientifically valid C3 reference to have no such failure.  A candidate may
have a valid reference and still be rejected because its output defect exceeds the common
budget.  These are different states.

Fix: add a separate `decision_reason` field.  Keep `failure=None` for policy rejection,
record the output-budget reason in `decision_reason`, and preserve `failure` for numerical
or certification failures only.
