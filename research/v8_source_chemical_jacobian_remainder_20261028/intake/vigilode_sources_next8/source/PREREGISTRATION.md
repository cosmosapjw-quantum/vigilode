# Supporting exact checks for the integrated polyalgorithm

Date: 2026-10-28 KST. The research source is the published PR93 tree at
`384dba091a3d9e1aac0ecddec264958298239253`. This protocol and both CHECK.py
programs must be published before their recorded execution. The execution
receipt will bind the actual published source commit and SHA-256 of each input.

## Question and selected scope

Can the existing PR90--93 mathematical leaves be composed into a valid physical
accuracy contract, and can declared finite operator structure remove entire
affine stage-vector recurrences while retaining an original-target certificate?
R3/R5/R9 are the selected theory stages; supporting exact calculations distinguish
the proposed identities from specific invalid combinations. The independent
native verification PR is specified in the handoff and is not executed here.

## Inputs and commands

Python standard library only, exact `fractions.Fraction`, no solver binary.
The authoritative inputs are the corresponding THEORY.md and CHECK.py files.

```text
python3 research/integrated_polyalgorithm_20261028/error_contract/CHECK.py
python3 research/integrated_polyalgorithm_20261028/algebra_contract/CHECK.py
```

Save complete stdout/stderr and exit status separately for each command. Checks
print JSON and do not overwrite historical evidence. These finite checks do not
replace the general proofs or native directed-arithmetic verification.

## Pass/kill gates

Error lane: all six named check groups must pass exact assertions, including
selector-independent recurrence, both valid anchors and the invalid mixed-anchor
counterexample, norm/clock accounting, finite-menu optimum versus exhaustive
enumeration, KKT allocation and upper-certificate/lower-representability distinction.
The documented menu example has total cost7 versus equal-split cost10 in its
declared toy work units; this is not a VigilODE timing observation.

Algebra lane: all nine compiler fixtures must match independently solved full
stage targets or enclose the nonzero original residual as specified; denominator
refusal must occur; inexact noncommuting correction identities and the exact
least sufficient count6 must hold. A nonzero near-annihilator defect must not
be mislabeled zero. Numeric output is required, not only a schema-valid receipt.

Any failed assertion or nonzero exit gives FAIL for that lane. Preserve stdout
and the failing source. A concrete diagnosed repair may use the user's bounded
correction policy; do not change the original target or weaken an inequality.
No native research correction budget is reset by these support checks.

## Holdout, prior calculations and policy scope

No statistical holdout is claimed: these are explicitly constructed exact
identities and counterexamples. Their examples and expected outcomes are public
before the run. Prior source-reading, AST/JSON syntax checks and the parent's
Wolfram KKT sanity calculation are exploratory/preparatory, not these registered
runs. The Wolfram response's symbolic-variable warning is retained.

The user's explicit request for theoretical synthesis and extension takes
precedence over the older general instruction in docs/RESEARCH_LEDGER.md to
defer formal/CAS obligations until a numerical holdout. We perform the requested
mathematics and exact checks now, while preserving append-only provenance and
all performance holdout/authority rules. No solver speed or release claim is
admitted by this protocol.

## Result append point

Execution results will be appended after the registered run.
