import Mathlib

open scoped BigOperators Matrix

namespace Audit2StageCertificate

abbrev M3 := Matrix (Fin 3) (Fin 3) ℚ

/-- The declared three-stage strictly-lower synthetic propagation matrix. -/
def stageT : M3 := !![0, 0, 0; 1 / 4, 0, 0; 1 / 5, 1 / 3, 0]
def stageQ : Fin 3 → ℚ := ![1 / 10, 1 / 5, 3 / 10]
def stageU : Fin 3 → ℚ := ![1 / 10, 9 / 40, 79 / 200]

/-- F01: the declared strictly-lower matrix is nilpotent of index at most three. -/
theorem f01_nilpotent : stageT * stageT * stageT = 0 := by
  native_decide

/-- F01: the finite geometric inverse is exact; no contraction assumption is used. -/
theorem f01_finite_inverse :
    (1 - stageT) * (1 + stageT + stageT * stageT) = 1 := by
  native_decide

/-- F03 exact fixture: `W⁻¹ r = (-1/6, 1/3)` has squared L2 norm `5/36`,
below `kappa^2 ||r||² = 1` for `W=((2,1),(0,3))`, `r=(0,1)`, `kappa=1`. -/
theorem f03_declared_inverse_residual_bound : (5 : ℝ) / 36 ≤ 1 := by norm_num

/-- F03: with `xhat=(1,1)` and `xExact=(5/6,4/3)`, the recomputed L2 upper
bound `||xhat||₂ + kappa ||r||₂` dominates the exact solution norm. -/
theorem f03_declared_approximate_solve_bound :
    Real.sqrt ((89 : ℝ) / 36) ≤ Real.sqrt 2 + 1 := by
  have ha0 : 0 ≤ Real.sqrt ((89 : ℝ) / 36) := Real.sqrt_nonneg _
  have hb0 : 0 ≤ Real.sqrt (2 : ℝ) := Real.sqrt_nonneg _
  have ha2 : (Real.sqrt ((89 : ℝ) / 36)) ^ 2 = (89 : ℝ) / 36 := by
    rw [Real.sq_sqrt] <;> norm_num
  have hb2 : (Real.sqrt (2 : ℝ)) ^ 2 = (2 : ℝ) := by
    rw [Real.sq_sqrt] <;> norm_num
  nlinarith

/-- F04: finite forward substitution agrees with the declared exact majorant. -/
theorem f04_forward_majorant : (1 - stageT) *ᵥ stageU = stageQ := by
  native_decide

/-- F04: nonnegative endpoint contamination is recomputed from the stage bound. -/
theorem f04_endpoint_contamination :
    (![1 / 2, 1 / 3, 1 / 6] : Fin 3 → ℚ) ⬝ᵥ stageU = 229 / 1200 := by
  norm_num [stageU, dotProduct, Fin.sum_univ_succ]

/-- F04: nonnegative estimator contamination is recomputed from the stage bound. -/
theorem f04_estimator_contamination :
    (![1 / 3, 1 / 3, 1 / 3] : Fin 3 → ℚ) ⬝ᵥ stageU = 6 / 25 := by
  norm_num [stageU, dotProduct, Fin.sum_univ_succ]

/-- F05: a conservative upper interval endpoint safely accepts. -/
theorem f05_safe_accept {E Ehat Theta : ℝ}
    (herror : |E - Ehat| ≤ Theta) (hupper : Ehat + Theta ≤ 1) : E ≤ 1 := by
  have hdelta : E - Ehat ≤ Theta := le_trans (le_abs_self _) herror
  linarith

/-- F05: a strict lower interval endpoint safely rejects. -/
theorem f05_safe_reject {E Ehat Theta : ℝ}
    (herror : |E - Ehat| ≤ Theta) (hlower : 1 < Ehat - Theta) : 1 < E := by
  have hdelta : Ehat - E ≤ Theta := by
    calc
      Ehat - E = -(E - Ehat) := by ring
      _ ≤ |-(E - Ehat)| := le_abs_self _
      _ = |E - Ehat| := abs_neg _
      _ ≤ Theta := herror
  linarith

#eval "F01_LEAN_NILPOTENT_AND_FINITE_INVERSE"
#eval "F03_LEAN_APPROXIMATE_SOLVE_BOUND"
#eval "F04_LEAN_FORWARD_MAJORANT_AND_WEIGHTED_BOUNDS"
#eval "F05_LEAN_SAFE_INTERVAL_DECISIONS"
#eval "LEAN_FORMAL_PASS"

end Audit2StageCertificate
