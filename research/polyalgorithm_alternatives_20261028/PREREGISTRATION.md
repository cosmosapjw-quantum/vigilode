# Polynomial polyalgorithm investigation: supporting check protocol

Date: 2026-10-28 UTC. Baseline commit:
`7eb2dd0ffda39afb2871d76700a720986ff09fe0` (PR 92).

## Selected work and chronology

The user asks for parallel theoretical investigation, broad constructive alternatives,
reuse of previous PRs, and publication with an updated handoff. The selected
physmath-research-loop 1.3.0 stages are R2 (primary-source investigation) and R4
(constructive alternatives), with supporting derivations and finite checks. This is
not a new execution of all ten theory or coding stages, nor a runtime promotion.

Literature reads, hand derivations, and some exploratory CAS calculations preceded
this registration. They will be identified as exploratory. The three local CHECK.py
programs below are to be committed with this protocol before their first execution.
No experimental holdout or wall-time inference is claimed for exact algebraic
identities; a future native performance campaign requires its own prospective
registration. The user's explicit request for theoretical proofs takes precedence
over the older ledger text that defers formal algebra until a numerical holdout.
The ledger's append-only and published-source requirements remain in force.

## Question

Can existing quotient, polynomial-action, causal, structural, and homotopy results
be extended with explicit finite-step constants and computation-saving contracts
without changing the original target, derivative permissions, or error budget?

## Commands and inputs

From the repository root, on the published source commit containing these programs:

```sh
python3 research/polyalgorithm_alternatives_20261028/polynomial/CHECK.py
python3 research/polyalgorithm_alternatives_20261028/homotopy/CHECK.py
python3 research/polyalgorithm_alternatives_20261028/geometry/CHECK.py --preregistration-sha "$(git rev-parse HEAD)"
```

Each program uses deterministic rational/integer fixtures declared in its source;
any ordinary floating-point diagnostic is marked separately. Source hashes and
output hashes are recorded with the resulting ledger row. All output is retained.
The checks are bounded examples and implementation checks of their written proofs,
not a substitute for those proofs or a general validation of production Rust.

## Gate and negative controls

PASS means all declared exact identities/inequalities and expected counterexamples
in the corresponding supporting check pass. A discovered algebraic failure is
retained and the relevant theorem is corrected or left open; no tolerance is
relaxed to rescue a claim. There is no speedup gate because these commands do not
benchmark the solver. Nonnormal matrices, failed admission and invalid reuse must
remain visible. A successful finite check cannot establish an opaque callback's
operator, domain, symmetry, or global nonlinear properties.

## Bounded scope and next work

No production backend, default route, public configuration, dependency lockfile,
or original accuracy cap is changed in this investigation. The updated plan keeps
the six PR-N01--06 contracts and the seven outstanding qualification tasks.
Existing per-task correction budgets are not reset by this theory supplement.
After at most one production/verification pass and one justified review pass,
publish the useful results and explicit remaining native obligations.

## Results appended after the registered run

All three commands exited 0 with exact-rational checks passing. Ledger rows L-0174, L-0175, L-0176 bind the source commit, checker hashes and actual output hashes. See RESULTS.json and evidence/*_RECEIPT.json. This PASS concerns only the declared supporting checks; native solver correctness, timing, profiling and production qualification were not tested. The L=0 residence prose clarification changed no equation or checker.
