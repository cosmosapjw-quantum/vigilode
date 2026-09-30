/-
UNCHECKED DRAFT, added 2026-09-28 for audit F-058.

This file states F01, F03 and F04 for an arbitrary dimension `n`.
It has NOT been compiled. Lean/mathlib was unavailable in the session that
wrote it: the network policy denied releases.lean-lang.org,
lakecache.blob.core.windows.net, mathlib4.lean-cache.cloud and
reservoir.lean-lang.org. The lean-mathlib backend is therefore
FORMAL_BACKEND_UNAVAILABLE for these statements, and nothing here is a proof
until `lake env lean` accepts this file against a pinned mathlib.

`tools/run_audit2_stage_certificate_formal.py` does not compile this file;
it compiles only `StageCertificate.lean`.
-/
import Mathlib

open scoped BigOperators Matrix

namespace Audit2StageCertificate

variable {n : ℕ}

/-- `T` is strictly lower triangular on `Fin n`. -/
def StrictLower {R : Type*} [Zero R] (T : Matrix (Fin n) (Fin n) R) : Prop :=
  ∀ i j : Fin n, i.val ≤ j.val → T i j = 0

/-- Entries of `T^k` vanish unless the row index exceeds the column index by at least `k`. -/
theorem strict_lower_pow_apply_eq_zero {R : Type*} [CommRing R]
    {T : Matrix (Fin n) (Fin n) R} (hT : StrictLower T) :
    ∀ (k : ℕ) (i j : Fin n), i.val < j.val + k → (T ^ k) i j = 0 := by
  intro k
  induction k with
  | zero =>
    intro i j hij
    have hne : i ≠ j := by
      intro h; subst h; omega
    simp [Matrix.one_apply_ne hne]
  | succ k ih =>
    intro i j hij
    rw [pow_succ, Matrix.mul_apply]
    apply Finset.sum_eq_zero
    intro l _
    by_cases hl : i.val < l.val + k
    · rw [ih i l hl, zero_mul]
    · have hlj : l.val ≤ j.val := by omega
      rw [hT l j hlj, mul_zero]

/-- F01: an arbitrary strict-lower `T` on `Fin n` satisfies `T^n = 0`. -/
theorem f01_strict_lower_nilpotent {R : Type*} [CommRing R]
    {T : Matrix (Fin n) (Fin n) R} (hT : StrictLower T) : T ^ n = 0 := by
  ext i j
  have hi := i.isLt
  rw [strict_lower_pow_apply_eq_zero hT n i j (by omega)]
  simp

/-- The finite Neumann sum `S = sum_{k<n} T^k`. -/
def neumann {R : Type*} [CommRing R] (T : Matrix (Fin n) (Fin n) R) :
    Matrix (Fin n) (Fin n) R :=
  ∑ k ∈ Finset.range n, T ^ k

/-- F01: `(I - T) S = I`. -/
theorem f01_finite_neumann_left_inverse {R : Type*} [CommRing R]
    {T : Matrix (Fin n) (Fin n) R} (hT : StrictLower T) : (1 - T) * neumann T = 1 := by
  unfold neumann
  rw [mul_neg_geom_sum, f01_strict_lower_nilpotent hT, sub_zero]

/-- F01: `S (I - T) = I`. -/
theorem f01_finite_neumann_right_inverse {R : Type*} [CommRing R]
    {T : Matrix (Fin n) (Fin n) R} (hT : StrictLower T) : neumann T * (1 - T) = 1 := by
  unfold neumann
  rw [geom_sum_mul_neg, f01_strict_lower_nilpotent hT, sub_zero]

/-- F03: for a two-sided inverse `V` of `W` whose norm is bounded by `kappa`,
`V b = x + V rho`, `||V b - x|| <= kappa ||rho||` and
`||V b|| <= ||x|| + kappa ||rho||` with `rho = b - W x`.  The right-hand side
`b` and the residual `rho` stay distinct throughout. -/
theorem f03_residual_solution_norm_bound {E : Type*} [NormedAddCommGroup E]
    [NormedSpace ℝ E] (W V : E →ₗ[ℝ] E) (hVW : ∀ v, V (W v) = v)
    (_hWV : ∀ v, W (V v) = v) (kappa : ℝ) (_hkappa : 0 ≤ kappa)
    (hbound : ∀ v, ‖V v‖ ≤ kappa * ‖v‖) (b x : E) :
    V b = x + V (b - W x) ∧ ‖V b - x‖ ≤ kappa * ‖b - W x‖ ∧
      ‖V b‖ ≤ ‖x‖ + kappa * ‖b - W x‖ := by
  have hsplit : V b = x + V (b - W x) := by
    rw [map_sub, hVW]; abel
  refine ⟨hsplit, ?_, ?_⟩
  · rw [hsplit, add_sub_cancel_left]
    exact hbound _
  · rw [hsplit]
    exact (norm_add_le _ _).trans (by linarith [hbound (b - W x)])

/-- F03 on `R^n` with the Euclidean norm, for every `n`. -/
theorem f03_residual_solution_norm_bound_euclidean
    (W V : EuclideanSpace ℝ (Fin n) →ₗ[ℝ] EuclideanSpace ℝ (Fin n))
    (hVW : ∀ v, V (W v) = v) (hWV : ∀ v, W (V v) = v) (kappa : ℝ) (hkappa : 0 ≤ kappa)
    (hbound : ∀ v, ‖V v‖ ≤ kappa * ‖v‖) (b x : EuclideanSpace ℝ (Fin n)) :
    V b = x + V (b - W x) ∧ ‖V b - x‖ ≤ kappa * ‖b - W x‖ ∧
      ‖V b‖ ≤ ‖x‖ + kappa * ‖b - W x‖ :=
  f03_residual_solution_norm_bound W V hVW hWV kappa hkappa hbound b x

/-- Entrywise nonnegativity of a real matrix. -/
def Nonneg (A : Matrix (Fin n) (Fin n) ℝ) : Prop := ∀ i j, 0 ≤ A i j

theorem nonneg_mul {A B : Matrix (Fin n) (Fin n) ℝ} (hA : Nonneg A) (hB : Nonneg B) :
    Nonneg (A * B) := by
  intro i j
  rw [Matrix.mul_apply]
  exact Finset.sum_nonneg fun l _ => mul_nonneg (hA i l) (hB l j)

theorem nonneg_pow {A : Matrix (Fin n) (Fin n) ℝ} (hA : Nonneg A) : ∀ k, Nonneg (A ^ k) := by
  intro k
  induction k with
  | zero =>
    intro i j
    by_cases h : i = j
    · subst h; simp
    · simp [Matrix.one_apply_ne h]
  | succ k ih => rw [pow_succ]; exact nonneg_mul ih hA

theorem nonneg_neumann {A : Matrix (Fin n) (Fin n) ℝ} (hA : Nonneg A) : Nonneg (neumann A) := by
  intro i j
  unfold neumann
  rw [Matrix.sum_apply]
  exact Finset.sum_nonneg fun k _ => nonneg_pow hA k i j

theorem mulVec_mono {A : Matrix (Fin n) (Fin n) ℝ} (hA : Nonneg A) {u v : Fin n → ℝ}
    (huv : ∀ i, u i ≤ v i) : ∀ i, (A *ᵥ u) i ≤ (A *ᵥ v) i := by
  intro i
  simp only [Matrix.mulVec, dotProduct]
  exact Finset.sum_le_sum fun j _ => mul_le_mul_of_nonneg_left (huv j) (hA i j)

/-- The stage majorant `z = S q` with the finite Neumann sum `S`. -/
def stageMajorant (A : Matrix (Fin n) (Fin n) ℝ) (q : Fin n → ℝ) : Fin n → ℝ :=
  neumann A *ᵥ q

/-- F04: the majorant is nonnegative. -/
theorem f04_stage_majorant_nonnegative {A : Matrix (Fin n) (Fin n) ℝ} (hA : Nonneg A)
    {q : Fin n → ℝ} (hq : ∀ i, 0 ≤ q i) : ∀ i, 0 ≤ stageMajorant A q i := by
  intro i
  simp only [stageMajorant, Matrix.mulVec, dotProduct]
  exact Finset.sum_nonneg fun j _ => mul_nonneg (nonneg_neumann hA i j) (hq j)

/-- F04: the majorant solves the forward recurrence `z_i = q_i + sum_{j<i} A_ij z_j`. -/
theorem f04_stage_majorant_recurrence {A : Matrix (Fin n) (Fin n) ℝ} (hlow : StrictLower A)
    (q : Fin n → ℝ) :
    ∀ i, stageMajorant A q i =
      q i + ∑ j ∈ Finset.univ.filter (fun j : Fin n => j.val < i.val),
        A i j * stageMajorant A q j := by
  intro i
  have hz : stageMajorant A q = q + A *ᵥ stageMajorant A q := by
    have h := congrArg (fun M => M *ᵥ q) (f01_finite_neumann_left_inverse hlow)
    simp only [Matrix.sub_mulVec, Matrix.one_mulVec, ← Matrix.mulVec_mulVec] at h
    unfold stageMajorant
    rw [eq_add_of_sub_eq h]
    abel
  conv_lhs => rw [hz]
  simp only [Pi.add_apply, Matrix.mulVec, dotProduct]
  congr 1
  rw [← Finset.sum_filter_add_sum_filter_not Finset.univ (fun j : Fin n => j.val < i.val)]
  have hzero : ∑ j ∈ Finset.univ.filter (fun j : Fin n => ¬ j.val < i.val),
      A i j * stageMajorant A q j = 0 :=
    Finset.sum_eq_zero fun j hj => by
      rw [hlow i j (by simpa using hj), zero_mul]
  rw [hzero, add_zero]

/-- F04: every admissible error `d`, i.e. one with `d <= q + A d` componentwise,
is dominated by the majorant. -/
theorem f04_stage_majorant_dominates {A : Matrix (Fin n) (Fin n) ℝ} (hA : Nonneg A)
    (hlow : StrictLower A) {q d : Fin n → ℝ} (hd : ∀ i, d i ≤ q i + (A *ᵥ d) i) :
    ∀ i, d i ≤ stageMajorant A q i := by
  -- d <= sum_{k<m} A^k q + A^m d for every m, then A^n = 0.
  have step : ∀ m, ∀ i,
      d i ≤ ((∑ k ∈ Finset.range m, A ^ k) *ᵥ q) i + ((A ^ m) *ᵥ d) i := by
    intro m
    induction m with
    | zero => intro i; simp
    | succ m ih =>
      intro i
      have hmono := mulVec_mono (nonneg_pow hA m) hd i
      have hsplit : ((A ^ m) *ᵥ (q + A *ᵥ d)) i =
          ((A ^ m) *ᵥ q) i + ((A ^ (m + 1)) *ᵥ d) i := by
        rw [Matrix.mulVec_add, Matrix.mulVec_mulVec, ← pow_succ, Pi.add_apply]
      have hsum : ((∑ k ∈ Finset.range (m + 1), A ^ k) *ᵥ q) i =
          ((∑ k ∈ Finset.range m, A ^ k) *ᵥ q) i + ((A ^ m) *ᵥ q) i := by
        rw [Finset.sum_range_succ, Matrix.add_mulVec, Pi.add_apply]
      have h1 := ih i
      have h2 : ((A ^ m) *ᵥ d) i ≤ ((A ^ m) *ᵥ (q + A *ᵥ d)) i := by
        simpa [Pi.add_apply] using hmono
      linarith [hsplit, hsum]
  intro i
  have h := step n i
  rw [f01_strict_lower_nilpotent hlow, Matrix.zero_mulVec, Pi.zero_apply, add_zero] at h
  exact h

/-- F04: nonnegative endpoint and estimator weights bound the weighted
contamination of any admissible error by that of the majorant. -/
theorem f04_weighted_contamination_bound {A : Matrix (Fin n) (Fin n) ℝ} (hA : Nonneg A)
    (hlow : StrictLower A) {q d alpha beta : Fin n → ℝ}
    (hd : ∀ i, d i ≤ q i + (A *ᵥ d) i) (halpha : ∀ i, 0 ≤ alpha i) (hbeta : ∀ i, 0 ≤ beta i) :
    alpha ⬝ᵥ d ≤ alpha ⬝ᵥ stageMajorant A q ∧ beta ⬝ᵥ d ≤ beta ⬝ᵥ stageMajorant A q := by
  have hdom := f04_stage_majorant_dominates hA hlow hd
  constructor
  · exact Finset.sum_le_sum fun i _ => mul_le_mul_of_nonneg_left (hdom i) (halpha i)
  · exact Finset.sum_le_sum fun i _ => mul_le_mul_of_nonneg_left (hdom i) (hbeta i)

end Audit2StageCertificate
