# Switcher WorkModel v2 — Preregistration (Node E1)

*Evaluation and freezing of deterministic, integer, versioned WorkModel v2 (§7) predicting instruction count (Ir) per solve across routes and dimension $n$.*

- **Date**: 2026-11-03
- **Branch**: `research/switcher-v2-20261103`
- **Protocol**: Section 7.2 & 7.3 calibration corpus protocol
- **Classification**: Research node E1 of Switcher v2 Plan

---

## 1. Question

Does WorkModel v2—defined with frozen fixed-point polynomial weights $\omega_k(n)$ over denominator $2^{16} = 65536$ for the nine solver-internal events $\mathcal K$—accurately predict instruction counts (Ir) per solve across routes and dimension $n$, removing the matrix-free bias of v1 while preserving determinism, bit-reproducible integer decisions, and continuous admission of the transition-39 entry?

---

## 2. Mathematical Form & Integer Arithmetic

For a batch with deterministic counter vector $\mathbf c$ on route $r$ at dimension $n$:

$$
W_{v2}(\mathbf c; r, n) = c_\text{rhs} + c_\text{jvp} + c_\text{ft} + n\,c_\text{jac} + \frac{1}{\kappa}\sum_{k\in\mathcal K} \omega_k(n)\,c_k ,
$$

where:
- $\mathcal K$ = {`linear_solves`, `linear_iterations`, `orthogonalization_inner_products`, `orthogonalization_vector_updates`, `direct_factorizations`, `direct_solve_calls`, `phi_krylov_vectors`, `phi_projected_exponentials`, `preconditioner_apps`}.
- Each $\omega_k(n) = \frac{1}{65536}(a_0 + a_1 n + a_2 n^2 + a_3 n^3)$ is stored as fixed-point $u64$ coefficients in `crates/vigilode/data/work_model_v2.toml`.
- $\kappa$ is the caller-declared RHS instruction scale (`RhsCost::Cheap = 50`, `Moderate = 500`, `Expensive = 5000`, or `DeclaredIr(u64)`).
- Arithmetic is strictly checked $u128$, evaluating user callbacks and overhead without any floating-point conversion.

---

## 3. Calibration Corpus & Arms

### Arms
1. `v1`: Existing callback proxy $c_\text{rhs} + c_\text{jvp} + c_\text{ft} + n\,c_\text{jac} + c_\text{solves}$.
2. `v2_cheap`: WorkModel v2 with default conservative $\kappa = 50$ (RHS-first principle).
3. `v2_declared`: WorkModel v2 with problem-matched declared $\kappa$ (Moderate $\kappa = 500$, Expensive $\kappa = 5000$).

### Corpus (Section 7.3)
1. **Existing registered cells** ($n \le 6$, cheap): `rotation`, `decay`, `stiff-forced`, `relaxation`, `case5`, `transition-39`, `transition-79`, `gyro`.
2. **1-D Heat / Advection-Diffusion MOL** ($n \in \{32, 64, 128, 256\}$, cheap, $O(n)$): dimension scaling of Krylov and LU events.
3. **1-D Brusselator MOL** ($n \in \{64, 128\}$, moderate): stiff reaction-diffusion with banded structure.
4. **Expensive-RHS wrapper** ($n \in \{32, 128\}$, expensive): synthetic arithmetic loop sensitivity.

---

## 4. Pass / Kill Gates

### PASS Gate
1. **Ir Prediction Accuracy**: Predicted Ir per solve ($\kappa \cdot W_{v2}$) matches measured callgrind Ir within $\pm 25\%$ across calibration routes.
2. **Rank Correlation**: Spearman rank correlation $\rho \ge 0.90$ between predicted and measured Ir across solver arms within each cell family.
3. **Transition-39 Admission**: The cheap-$\kappa$ default does not delay the transition-39 entry beyond the B5 warning threshold.
4. **Zero-Allocation**: Zero heap allocations in hot loops during $W_{v2}$ evaluation (`hot_loop_allocations == 0`).
5. **Exact Integer Safety**: All calculations use checked $u128$ arithmetic returning `None` on overflow without truncating.

### KILL Gate
1. WorkModel v2 delays the transition-39 entry beyond the B5 warning threshold.
2. Any regression in registered numerical accuracy caps.
3. Any heap allocation detected in hot loop evaluations.

---

## 5. Sealed Holdout Isolation

In strict compliance with `docs/HOLDOUT_HYGIENE.md` and `AGENTS.md` §2.6:
- Sealed holdout profiles (`holdout-512`, `enforced-budget-holdout-320`, `stage-growth-holdout-384`) are NOT executed.
- Sealed artifacts (`tools/reference_v2/artifacts/*-holdout-v2.json`) are NOT opened or inspected.
- Only the pre-registered open calibration cells defined above are evaluated.

---

## 6. Execution Command

```bash
cargo test --locked --offline -p vigilode --test work_model_contracts
python3 research/switcher_work_model_v2_20261103/eval_work_model.py
```
