# G3 PHYS–MATH–CODE AUDIT

## Equation-to-code map

| Mathematical object | Code path | Status |
|---|---|---|
| Matrix-free `Jv` | `OdeProblem::linearize_matrix_free` / `LinearOperator` | implemented and used |
| Fused phi combination | `fused_phi_linear_combination` | implemented and used |
| Augmented operator | `augmented_fused_operator` | implemented and used |
| Arnoldi residual estimate | `projected_exponential_action_with_residual_estimate` | implemented, research estimate |
| Full-MGS / IOP2 | `FusedOrthogonalization` | both implemented; Full-MGS authority |
| Adaptive pexprb54s4 | `integrate_pexprb54s4_fused_adaptive_observed` | implemented and used |
| Transactional output failure | `OutputCollector::finish_partial` | implemented and tested |
| Same-error authority | G3 analysis over frozen fair-measurement controls | implemented |

## Load-bearing implementation findings

### Genuinely fixed

1. Fifteen separate pexprb54s4 phi terms are fused into five augmented actions.
2. Failed adaptive integrations preserve attempted work and partial committed output.
3. T1/T4 scientific fields are exact-identical.
4. BDF/Radau source hashes and Cargo.lock remain unchanged.
5. Physical-client bindings remain absent from production Rust source.

### Remaining risks

- IOP2 lacks production-grade adaptive restart/orthogonality-loss policy.
- Each Krylov checkpoint evaluates projected `phi_0` and `phi_1`; projected-space cost is not yet optimized.
- Full-MGS allocates intermediate vectors in research code; large-state memory profiling is pending.
- The first-term residual estimate can be nonconservative under nonnormality.
- Current T4 scheduling overhead dominates small-state work.
- Current generic corpus is calibration/early comparator data, not a sealed expanded holdout.

## Regression sufficiency

New/modified contracts cover:

- fused action versus dense oracle;
- residual estimate versus first Arnoldi term;
- convergence does not use nested-dimension difference;
- adaptive tolerance response;
- failure-work preservation;
- actual parallel dependency overlap;
- G3 deterministic scientific replay;
- frozen comparator hashes and physical-client exclusion.

Exact-HEAD verification is split by crate to avoid monolithic wrapper timeouts. All unit, integration, CLI, and doc-test groups pass.

## Risk ledger

| Priority | Risk | Disposition |
|---|---|---|
| P0 | claim generic acceleration despite missing target | blocked by promotion gate |
| P0 | hide failed adaptive work | fixed by partial-output failure path |
| P1 | tune against oscillatory PR and call it holdout | forbidden; G4 split required |
| P1 | default IOP2 after calibration success | HOLD |
| P1 | enable T4 by default on small states | rejected by measured crossover |
| P2 | projected-space duplicate exponential work | optimization candidate after holdout |
| P2 | memory/allocation scaling | expanded holdout/profile required |
| P3 | cosmetic refactor | defer |

## Verdict

```text
Code-path reality        STRONG RESEARCH IMPLEMENTATION
Failure accounting       PASS
Scope/freeze discipline  PASS
Production maturity      HOLD
```
