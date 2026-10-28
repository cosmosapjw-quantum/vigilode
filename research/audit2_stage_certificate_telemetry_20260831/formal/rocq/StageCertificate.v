From Stdlib Require Import QArith Qring Reals Lra Strings.String.
From Stdlib Require Import Lia Arith.

Open Scope Q_scope.
Open Scope R_scope.

Record M3 := mkM3 {
  m00 : Q; m01 : Q; m02 : Q;
  m10 : Q; m11 : Q; m12 : Q;
  m20 : Q; m21 : Q; m22 : Q
}.

Definition madd (A B : M3) : M3 :=
  match A, B with
  | mkM3 a00 a01 a02 a10 a11 a12 a20 a21 a22,
    mkM3 b00 b01 b02 b10 b11 b12 b20 b21 b22 =>
      mkM3 (a00+b00) (a01+b01) (a02+b02)
           (a10+b10) (a11+b11) (a12+b12)
           (a20+b20) (a21+b21) (a22+b22)
  end.

Definition msub (A B : M3) : M3 :=
  match A, B with
  | mkM3 a00 a01 a02 a10 a11 a12 a20 a21 a22,
    mkM3 b00 b01 b02 b10 b11 b12 b20 b21 b22 =>
      mkM3 (a00-b00) (a01-b01) (a02-b02)
           (a10-b10) (a11-b11) (a12-b12)
           (a20-b20) (a21-b21) (a22-b22)
  end.

Definition mmul (A B : M3) : M3 :=
  match A, B with
  | mkM3 a00 a01 a02 a10 a11 a12 a20 a21 a22,
    mkM3 b00 b01 b02 b10 b11 b12 b20 b21 b22 =>
      mkM3 (a00*b00+a01*b10+a02*b20) (a00*b01+a01*b11+a02*b21) (a00*b02+a01*b12+a02*b22)
           (a10*b00+a11*b10+a12*b20) (a10*b01+a11*b11+a12*b21) (a10*b02+a11*b12+a12*b22)
           (a20*b00+a21*b10+a22*b20) (a20*b01+a21*b11+a22*b21) (a20*b02+a21*b12+a22*b22)
  end.

Definition mzero := mkM3 0 0 0 0 0 0 0 0 0.
Definition mid := mkM3 1 0 0 0 1 0 0 0 1.
Definition stageT := mkM3 0 0 0 (1#4) 0 0 (1#5) (1#3) 0.

Definition meq (A B : M3) : Prop :=
  match A, B with
  | mkM3 a00 a01 a02 a10 a11 a12 a20 a21 a22,
    mkM3 b00 b01 b02 b10 b11 b12 b20 b21 b22 =>
      (a00 == b00)%Q /\ (a01 == b01)%Q /\ (a02 == b02)%Q /\
      (a10 == b10)%Q /\ (a11 == b11)%Q /\ (a12 == b12)%Q /\
      (a20 == b20)%Q /\ (a21 == b21)%Q /\ (a22 == b22)%Q
  end.

(* F01: exact finite-dimensional algebra for the declared strictly-lower T. *)
Theorem f01_nilpotent : meq (mmul (mmul stageT stageT) stageT) mzero.
Proof. vm_compute. repeat split; reflexivity. Qed.

Theorem f01_finite_inverse :
  meq (mmul (msub mid stageT) (madd (madd mid stageT) (mmul stageT stageT))) mid.
Proof. vm_compute. repeat split; reflexivity. Qed.

(* F03: W=((2,1),(0,3)), r=(0,1), kappa=1 gives squared bound 5/36 <= 1. *)
Theorem f03_declared_inverse_residual_bound : (5#36 <= 1)%Q.
Proof. vm_compute. easy. Qed.

(* F04: exact forward propagation q=(1/10,1/5,3/10), u=(1/10,9/40,79/200). *)
Theorem f04_forward_second : Qeq ((1#5) + (1#4)*(1#10))%Q (9#40).
Proof. vm_compute. reflexivity. Qed.
Theorem f04_forward_third :
  Qeq ((3#10) + (1#5)*(1#10) + (1#3)*(9#40))%Q (79#200).
Proof. vm_compute. reflexivity. Qed.
Theorem f04_endpoint_contamination :
  Qeq ((1#2)*(1#10) + (1#3)*(9#40) + (1#6)*(79#200))%Q (229#1200).
Proof. vm_compute. reflexivity. Qed.
Theorem f04_estimator_contamination :
  Qeq ((1#3)*(1#10) + (1#3)*(9#40) + (1#3)*(79#200))%Q (6#25).
Proof. vm_compute. reflexivity. Qed.

(* F05: interval arithmetic implications over exact real numbers. *)
Theorem f05_safe_accept : forall E Ehat Theta : R,
  Rabs (E - Ehat) <= Theta -> Ehat + Theta <= 1 -> E <= 1.
Proof.
  intros E Ehat Theta Herror Hupper.
  assert (Hdelta : E - Ehat <= Theta).
  { eapply Rle_trans. apply Rle_abs. exact Herror. }
  lra.
Qed.

Theorem f05_safe_reject : forall E Ehat Theta : R,
  Rabs (E - Ehat) <= Theta -> 1 < Ehat - Theta -> 1 < E.
Proof.
  intros E Ehat Theta Herror Hlower.
  assert (Hdelta : Ehat - E <= Theta).
  { replace (Ehat - E) with (-(E - Ehat)) by ring.
    eapply Rle_trans. apply Rle_abs.
    rewrite Rabs_Ropp. exact Herror. }
  lra.
Qed.

(* ================================================================== *)
(* General statements for an arbitrary finite dimension n.              *)
(* The 3x3 theorems above are fixtures; the theorems in this module are  *)
(* quantified over every n and every matrix satisfying the premises.     *)
(* Added 2026-09-28 (audit F-058).                                       *)
(* ================================================================== *)

Module General.
Local Open Scope R_scope.
(* ------------------------------------------------------------------ *)
(* Finite sums over 0..k-1, matrices and vectors indexed by nat.        *)
(* Every statement below is quantified over the dimension n.            *)
(* ------------------------------------------------------------------ *)

Fixpoint fsum (k : nat) (f : nat -> R) : R :=
  match k with
  | O => 0
  | S k' => fsum k' f + f k'
  end.

Lemma fsum_ext : forall k f g,
  (forall l, (l < k)%nat -> f l = g l) -> fsum k f = fsum k g.
Proof.
  induction k as [|k IH]; intros f g H; simpl; [reflexivity|].
  rewrite (IH f g) by (intros; apply H; lia).
  rewrite (H k) by lia. reflexivity.
Qed.

Lemma fsum_zero : forall k f, (forall l, (l < k)%nat -> f l = 0) -> fsum k f = 0.
Proof.
  intros k f H. rewrite (fsum_ext k f (fun _ => 0)) by exact H.
  clear H. induction k as [|k IH]; simpl; [reflexivity|]. rewrite IH. ring.
Qed.

Lemma fsum_plus : forall k f g, fsum k (fun l => f l + g l) = fsum k f + fsum k g.
Proof. induction k as [|k IH]; intros; simpl; [ring|]. rewrite IH. ring. Qed.

Lemma fsum_minus : forall k f g, fsum k (fun l => f l - g l) = fsum k f - fsum k g.
Proof. induction k as [|k IH]; intros; simpl; [ring|]. rewrite IH. ring. Qed.

Lemma fsum_scal_l : forall k c f, c * fsum k f = fsum k (fun l => c * f l).
Proof. induction k as [|k IH]; intros; simpl; [ring|]. rewrite <- IH. ring. Qed.

Lemma fsum_scal_r : forall k c f, fsum k f * c = fsum k (fun l => f l * c).
Proof. induction k as [|k IH]; intros; simpl; [ring|]. rewrite <- IH. ring. Qed.

Lemma fsum_swap : forall a b (f : nat -> nat -> R),
  fsum a (fun i => fsum b (fun j => f i j)) = fsum b (fun j => fsum a (fun i => f i j)).
Proof.
  induction a as [|a IH]; intros b f; simpl.
  - symmetry. apply fsum_zero. intros. reflexivity.
  - rewrite IH. rewrite <- fsum_plus. reflexivity.
Qed.

Lemma fsum_le : forall k f g,
  (forall l, (l < k)%nat -> f l <= g l) -> fsum k f <= fsum k g.
Proof.
  induction k as [|k IH]; intros f g H; simpl; [lra|].
  pose proof (IH f g ltac:(intros; apply H; lia)). pose proof (H k ltac:(lia)). lra.
Qed.

Lemma fsum_nonneg : forall k f, (forall l, (l < k)%nat -> 0 <= f l) -> 0 <= fsum k f.
Proof.
  intros k f H. replace 0 with (fsum k (fun _ => 0)) by (apply fsum_zero; auto).
  apply fsum_le. exact H.
Qed.

Lemma fsum_telescope : forall k (f : nat -> R),
  fsum k (fun l => f l - f (S l)) = f O - f k.
Proof. induction k as [|k IH]; intros; simpl; [ring|]. rewrite IH. ring. Qed.

(* The indicator picks one term out of a finite sum. *)
Lemma fsum_delta : forall k i (g : nat -> R),
  fsum k (fun l => (if Nat.eqb i l then 1 else 0) * g l) =
  if Nat.ltb i k then g i else 0.
Proof.
  induction k as [|k IH]; intros i g; simpl.
  - destruct (Nat.ltb_spec i 0); [lia|reflexivity].
  - rewrite IH. destruct (Nat.eqb_spec i k) as [->|Hne].
    + destruct (Nat.ltb_spec k k); [lia|].
      destruct (Nat.ltb_spec k (S k)); [ring|lia].
    + destruct (Nat.ltb_spec i k); destruct (Nat.ltb_spec i (S k)); try lia; ring.
Qed.

(* A split of a sum whose terms vanish from index i on. *)
Lemma fsum_truncate : forall k i f,
  (i <= k)%nat -> (forall l, (i <= l < k)%nat -> f l = 0) -> fsum k f = fsum i f.
Proof.
  induction k as [|k IH]; intros i f Hik H.
  - assert (i = O) by lia. subst. reflexivity.
  - destruct (Nat.eq_dec i (S k)) as [->|Hne]; [reflexivity|].
    simpl. rewrite (H k) by lia. rewrite (IH i f) by (try lia; intros; apply H; lia). ring.
Qed.

Definition mat := nat -> nat -> R.
Definition vec := nat -> R.

Definition mid : mat := fun i j => if Nat.eqb i j then 1 else 0.
Definition mzero : mat := fun _ _ => 0.
Definition msub (A B : mat) : mat := fun i j => A i j - B i j.
Definition mmul (n : nat) (A B : mat) : mat := fun i j => fsum n (fun l => A i l * B l j).
Definition mvmul (n : nat) (A : mat) (v : vec) : vec := fun i => fsum n (fun l => A i l * v l).

(* T^0 = I, T^(k+1) = T^k T. *)
Fixpoint mpow (n : nat) (T : mat) (k : nat) : mat :=
  match k with
  | O => mid
  | S k' => mmul n (mpow n T k') T
  end.

Definition msum (K : nat) (F : nat -> mat) : mat := fun i j => fsum K (fun k => F k i j).

(* Equality of the entries indexed by Fin n. *)
Definition meq (n : nat) (A B : mat) : Prop :=
  forall i j, (i < n)%nat -> (j < n)%nat -> A i j = B i j.

Definition strict_lower (n : nat) (T : mat) : Prop :=
  forall i j, (i < n)%nat -> (j < n)%nat -> (i <= j)%nat -> T i j = 0.

(* The finite Neumann sum S = sum_{k<n} T^k. *)
Definition neumann (n : nat) (T : mat) : mat := msum n (mpow n T).

Lemma mmul_assoc : forall n A B C i j,
  mmul n (mmul n A B) C i j = mmul n A (mmul n B C) i j.
Proof.
  intros. unfold mmul.
  transitivity (fsum n (fun l => fsum n (fun m => A i m * B m l * C l j))).
  - apply fsum_ext. intros. rewrite fsum_scal_r. reflexivity.
  - rewrite fsum_swap. apply fsum_ext. intros. rewrite fsum_scal_l.
    apply fsum_ext. intros. ring.
Qed.

Lemma mmul_id_l : forall n B i j, (i < n)%nat -> mmul n mid B i j = B i j.
Proof.
  intros. unfold mmul, mid. rewrite fsum_delta.
  destruct (Nat.ltb i n) eqn:E; [reflexivity|apply Nat.ltb_ge in E; lia].
Qed.

Lemma mmul_id_r : forall n A i j, (j < n)%nat -> mmul n A mid i j = A i j.
Proof.
  intros. unfold mmul, mid.
  rewrite (fsum_ext n _ (fun l => (if Nat.eqb j l then 1 else 0) * A i l)).
  - rewrite fsum_delta. destruct (Nat.ltb j n) eqn:E; [reflexivity|apply Nat.ltb_ge in E; lia].
  - intros l _. destruct (Nat.eqb_spec l j); destruct (Nat.eqb_spec j l);
      subst; try lia; ring.
Qed.

(* Products only read entries indexed by Fin n. *)
Lemma mmul_meq : forall n A A' B B',
  meq n A A' -> meq n B B' -> meq n (mmul n A B) (mmul n A' B').
Proof.
  intros n A A' B B' HA HB i j Hi Hj. unfold mmul. apply fsum_ext. intros l Hl.
  rewrite HA, HB by lia. reflexivity.
Qed.

(* Entries of T^k vanish unless the row exceeds the column by at least k. *)
Lemma strict_lower_pow_zero : forall n T, strict_lower n T ->
  forall k i j, (i < n)%nat -> (j < n)%nat -> (i < j + k)%nat -> mpow n T k i j = 0.
Proof.
  intros n T HT k. induction k as [|k IH]; intros i j Hi Hj Hij; simpl.
  - unfold mid. destruct (Nat.eqb i j) eqn:E; [apply Nat.eqb_eq in E; lia|reflexivity].
  - unfold mmul. apply fsum_zero. intros l Hl.
    destruct (Nat.lt_ge_cases i (l + k)) as [H|H].
    + rewrite (IH i l) by lia. ring.
    + rewrite (HT l j) by lia. ring.
Qed.

(* F01: an arbitrary strict-lower T on Fin n satisfies T^n = 0. *)
Theorem f01_strict_lower_nilpotent : forall n T,
  strict_lower n T -> meq n (mpow n T n) mzero.
Proof.
  intros n T HT i j Hi Hj. unfold mzero.
  apply (strict_lower_pow_zero n T HT n i j Hi Hj). lia.
Qed.

Lemma mpow_comm : forall n T k i j, (i < n)%nat -> (j < n)%nat ->
  mmul n T (mpow n T k) i j = mpow n T (S k) i j.
Proof.
  intros n T k. induction k as [|k IH]; intros i j Hi Hj.
  - simpl. rewrite mmul_id_r by exact Hj. rewrite mmul_id_l by exact Hi. reflexivity.
  - change (mpow n T (S k)) with (mmul n (mpow n T k) T).
    rewrite <- mmul_assoc.
    change (mpow n T (S (S k))) with (mmul n (mpow n T (S k)) T).
    apply mmul_meq; [|intros ? ? ? ?; reflexivity|exact Hi|exact Hj].
    intros a b Ha Hb. apply IH; assumption.
Qed.

(* Entry (i, j) of a product with a matrix sum, exchanged into a sum of products. *)
Lemma mmul_msum_l : forall n K F B i j,
  mmul n (msum K F) B i j = fsum K (fun k => mmul n (F k) B i j).
Proof.
  intros. unfold mmul, msum.
  rewrite (fsum_ext n _ (fun l => fsum K (fun k => F k i l * B l j)))
    by (intros; apply fsum_scal_r).
  apply fsum_swap.
Qed.

Lemma mmul_msum_r : forall n K A F i j,
  mmul n A (msum K F) i j = fsum K (fun k => mmul n A (F k) i j).
Proof.
  intros. unfold mmul, msum.
  rewrite (fsum_ext n _ (fun l => fsum K (fun k => A i l * F k l j)))
    by (intros; apply fsum_scal_l).
  apply fsum_swap.
Qed.

Lemma mmul_msub_r : forall n A B C i j,
  mmul n A (msub B C) i j = mmul n A B i j - mmul n A C i j.
Proof.
  intros. unfold mmul, msub. rewrite <- fsum_minus. apply fsum_ext. intros. ring.
Qed.

Lemma mmul_msub_l : forall n A B C i j,
  mmul n (msub A B) C i j = mmul n A C i j - mmul n B C i j.
Proof.
  intros. unfold mmul, msub. rewrite <- fsum_minus. apply fsum_ext. intros. ring.
Qed.

(* F01: S (I - T) = I. *)
Theorem f01_finite_neumann_right_inverse : forall n T,
  strict_lower n T -> meq n (mmul n (neumann n T) (msub mid T)) mid.
Proof.
  intros n T HT i j Hi Hj. unfold neumann.
  rewrite mmul_msum_l.
  rewrite (fsum_ext n _ (fun k => mpow n T k i j - mpow n T (S k) i j)).
  - rewrite (fsum_telescope n (fun k => mpow n T k i j)).
    rewrite (f01_strict_lower_nilpotent n T HT i j Hi Hj). unfold mzero. simpl. ring.
  - intros k _. rewrite mmul_msub_r, mmul_id_r by exact Hj. reflexivity.
Qed.

(* F01: (I - T) S = I. *)
Theorem f01_finite_neumann_left_inverse : forall n T,
  strict_lower n T -> meq n (mmul n (msub mid T) (neumann n T)) mid.
Proof.
  intros n T HT i j Hi Hj. unfold neumann.
  rewrite mmul_msum_r.
  rewrite (fsum_ext n _ (fun k => mpow n T k i j - mpow n T (S k) i j)).
  - rewrite (fsum_telescope n (fun k => mpow n T k i j)).
    rewrite (f01_strict_lower_nilpotent n T HT i j Hi Hj). unfold mzero. simpl. ring.
  - intros k _. rewrite mmul_msub_l, mmul_id_l by exact Hi.
    rewrite mpow_comm by assumption. reflexivity.
Qed.

(* ------------------------------------------------------------------ *)
(* F03 over an arbitrary normed abelian group with a two-sided inverse. *)
(* ------------------------------------------------------------------ *)

Section F03.
  Variable E : Type.
  Variable add : E -> E -> E.
  Variable opp : E -> E.
  Variable zero : E.
  Variable norm : E -> R.
  Hypothesis add_assoc : forall a b c, add a (add b c) = add (add a b) c.
  Hypothesis add_comm : forall a b, add a b = add b a.
  Hypothesis add_zero : forall a, add a zero = a.
  Hypothesis add_opp : forall a, add a (opp a) = zero.
  Hypothesis norm_triangle : forall a b, norm (add a b) <= norm a + norm b.

  Let sub a b := add a (opp b).

  Variables W V : E -> E.
  Hypothesis V_add : forall a b, V (add a b) = add (V a) (V b).
  Hypothesis VW : forall v, V (W v) = v.
  Hypothesis WV : forall v, W (V v) = v.

  (* F03: rho = b - W x is distinct from b; V b = x + V rho and the two
     norm bounds follow from the pointwise inverse bound. *)
  Theorem f03_residual_solution_norm_bound :
    forall (kappa : R) (b x : E),
      0 <= kappa ->
      (forall v, norm (V v) <= kappa * norm v) ->
      V b = add x (V (sub b (W x))) /\
      norm (sub (V b) x) <= kappa * norm (sub b (W x)) /\
      norm (V b) <= norm x + kappa * norm (sub b (W x)).
  Proof.
    intros kappa b x _ Hbound.
    assert (Hb : b = add (sub b (W x)) (W x)).
    { unfold sub. rewrite <- add_assoc, (add_comm (opp (W x))), add_opp, add_zero.
      reflexivity. }
    assert (Hsplit : V b = add x (V (sub b (W x)))).
    { rewrite Hb at 1. rewrite V_add, VW, add_comm. reflexivity. }
    split; [exact Hsplit|]. split.
    - assert (Hdiff : sub (V b) x = V (sub b (W x))).
      { rewrite Hsplit. unfold sub.
        rewrite (add_comm x), <- add_assoc, add_opp, add_zero. reflexivity. }
      rewrite Hdiff. apply Hbound.
    - rewrite Hsplit. eapply Rle_trans; [apply norm_triangle|].
      pose proof (Hbound (sub b (W x))). lra.
  Qed.
End F03.

(* ------------------------------------------------------------------ *)
(* F04: the stage majorant z = S q for nonnegative strict-lower A.       *)
(* ------------------------------------------------------------------ *)

Definition mnonneg (n : nat) (A : mat) : Prop :=
  forall i j, (i < n)%nat -> (j < n)%nat -> 0 <= A i j.

Definition stage_majorant (n : nat) (A : mat) (q : vec) : vec := mvmul n (neumann n A) q.

Lemma mnonneg_mid : forall n, mnonneg n mid.
Proof. intros n i j _ _. unfold mid. destruct (Nat.eqb i j); lra. Qed.

Lemma mnonneg_mmul : forall n A B, mnonneg n A -> mnonneg n B -> mnonneg n (mmul n A B).
Proof.
  intros n A B HA HB i j Hi Hj. unfold mmul. apply fsum_nonneg. intros l Hl.
  apply Rmult_le_pos; [apply HA|apply HB]; assumption.
Qed.

Lemma mnonneg_pow : forall n A k, mnonneg n A -> mnonneg n (mpow n A k).
Proof.
  intros n A k HA. induction k as [|k IH]; simpl; [apply mnonneg_mid|].
  apply mnonneg_mmul; assumption.
Qed.

Lemma mnonneg_neumann : forall n A, mnonneg n A -> mnonneg n (neumann n A).
Proof.
  intros n A HA i j Hi Hj. unfold neumann, msum. apply fsum_nonneg. intros k _.
  apply mnonneg_pow; assumption.
Qed.

(* F04: the majorant is nonnegative for nonnegative A and q. *)
Theorem f04_stage_majorant_nonnegative : forall n A q,
  mnonneg n A -> (forall i, (i < n)%nat -> 0 <= q i) ->
  forall i, (i < n)%nat -> 0 <= stage_majorant n A q i.
Proof.
  intros n A q HA Hq i Hi. unfold stage_majorant, mvmul. apply fsum_nonneg.
  intros l Hl. apply Rmult_le_pos; [apply mnonneg_neumann|apply Hq]; assumption.
Qed.

Lemma mvmul_mmul : forall n A B v i,
  mvmul n (mmul n A B) v i = mvmul n A (mvmul n B v) i.
Proof.
  intros. unfold mvmul, mmul.
  transitivity (fsum n (fun l => fsum n (fun m => A i m * B m l * v l))).
  - apply fsum_ext. intros. rewrite fsum_scal_r. reflexivity.
  - rewrite fsum_swap. apply fsum_ext. intros. rewrite fsum_scal_l.
    apply fsum_ext. intros. ring.
Qed.

Lemma mvmul_mono : forall n A u v, mnonneg n A ->
  (forall l, (l < n)%nat -> u l <= v l) ->
  forall i, (i < n)%nat -> mvmul n A u i <= mvmul n A v i.
Proof.
  intros n A u v HA Huv i Hi. unfold mvmul. apply fsum_le. intros l Hl.
  apply Rmult_le_compat_l; [apply HA; assumption|apply Huv; assumption].
Qed.

(* F04: the finite Neumann representation solves z = q + A z, i.e.
   z_i = q_i + sum_{j<i} A_ij z_j for strict-lower A. *)
Theorem f04_stage_majorant_recurrence : forall n A q,
  strict_lower n A ->
  forall i, (i < n)%nat ->
    stage_majorant n A q i = q i + fsum i (fun j => A i j * stage_majorant n A q j).
Proof.
  intros n A q HA i Hi.
  assert (H1 : mvmul n (mmul n (msub mid A) (neumann n A)) q i = q i).
  { unfold mvmul.
    rewrite (fsum_ext n _ (fun l => mid i l * q l)).
    - unfold mid. rewrite fsum_delta. destruct (Nat.ltb_spec i n); [reflexivity|lia].
    - intros l Hl. rewrite (f01_finite_neumann_left_inverse n A HA i l Hi Hl). reflexivity. }
  assert (H2 : mvmul n (mmul n (msub mid A) (neumann n A)) q i =
      mvmul n (neumann n A) q i - mvmul n A (mvmul n (neumann n A) q) i).
  { rewrite <- mvmul_mmul. unfold mvmul. rewrite <- fsum_minus. apply fsum_ext.
    intros l Hl. rewrite mmul_msub_l, mmul_id_l by exact Hi. ring. }
  assert (Hfix : stage_majorant n A q i = q i + mvmul n A (stage_majorant n A q) i)
    by (unfold stage_majorant; lra).
  rewrite Hfix. f_equal. unfold mvmul.
  apply fsum_truncate; [lia|]. intros l Hl. rewrite (HA i l) by lia. ring.
Qed.

(* F04: every admissible error d (componentwise d <= q + A d) is dominated. *)
Theorem f04_stage_majorant_dominates : forall n A q d,
  mnonneg n A -> strict_lower n A ->
  (forall i, (i < n)%nat -> d i <= q i + mvmul n A d i) ->
  forall i, (i < n)%nat -> d i <= stage_majorant n A q i.
Proof.
  intros n A q d HA Hlow Hd.
  assert (Hstep : forall m i, (i < n)%nat ->
      d i <= mvmul n (msum m (mpow n A)) q i + mvmul n (mpow n A m) d i).
  { induction m as [|m IH]; intros i Hi.
    - assert (Hs : mvmul n (msum 0 (mpow n A)) q i = 0)
        by (unfold mvmul, msum; simpl; apply fsum_zero; intros; ring).
      assert (Hm : mvmul n (mpow n A 0) d i = d i).
      { simpl. unfold mvmul, mid. rewrite fsum_delta.
        destruct (Nat.ltb_spec i n); [reflexivity|lia]. }
      lra.
    - pose proof (IH i Hi) as H1.
      assert (H2 : mvmul n (mpow n A m) d i <=
          mvmul n (mpow n A m) (fun l => q l + mvmul n A d l) i).
      { apply mvmul_mono; [apply mnonneg_pow; exact HA| |exact Hi].
        intros l Hl. apply Hd. exact Hl. }
      assert (H3 : mvmul n (mpow n A m) (fun l => q l + mvmul n A d l) i =
          mvmul n (mpow n A m) q i + mvmul n (mpow n A (S m)) d i).
      { unfold mvmul at 1. rewrite (fsum_ext n _ (fun l => mpow n A m i l * q l
            + mpow n A m i l * mvmul n A d l)) by (intros; ring).
        rewrite fsum_plus. f_equal. simpl. rewrite mvmul_mmul. reflexivity. }
      assert (H4 : mvmul n (msum (S m) (mpow n A)) q i =
          mvmul n (msum m (mpow n A)) q i + mvmul n (mpow n A m) q i).
      { unfold mvmul, msum. simpl. rewrite <- fsum_plus. apply fsum_ext.
        intros. ring. }
      lra. }
  intros i Hi. pose proof (Hstep n i Hi) as H.
  assert (Hz : mvmul n (mpow n A n) d i = 0).
  { unfold mvmul. apply fsum_zero. intros l Hl.
    rewrite (f01_strict_lower_nilpotent n A Hlow i l Hi Hl). unfold mzero. ring. }
  unfold stage_majorant, neumann. lra.
Qed.

(* F04: nonnegative endpoint and estimator weights bound the weighted
   contamination of any admissible error by that of the majorant. *)
Theorem f04_weighted_contamination_bound : forall n A q d alpha beta,
  mnonneg n A -> strict_lower n A ->
  (forall i, (i < n)%nat -> d i <= q i + mvmul n A d i) ->
  (forall i, (i < n)%nat -> 0 <= alpha i) ->
  (forall i, (i < n)%nat -> 0 <= beta i) ->
  fsum n (fun i => alpha i * d i) <= fsum n (fun i => alpha i * stage_majorant n A q i) /\
  fsum n (fun i => beta i * d i) <= fsum n (fun i => beta i * stage_majorant n A q i).
Proof.
  intros n A q d alpha beta HA Hlow Hd Ha Hb.
  pose proof (f04_stage_majorant_dominates n A q d HA Hlow Hd) as Hdom.
  split; apply fsum_le; intros i Hi; apply Rmult_le_compat_l; auto.
Qed.

(* A concrete n = 4 instance, distinct from the 3x3 fixture. *)
Definition T4 : mat := fun i j =>
  if andb (Nat.ltb j i) (Nat.ltb i 4) then INR (i + j + 1) else 0.

Lemma T4_strict_lower : strict_lower 4 T4.
Proof.
  intros i j Hi Hj Hij. unfold T4.
  destruct (Nat.ltb_spec j i); [lia|reflexivity].
Qed.

Theorem n4_instance_nilpotent : meq 4 (mpow 4 T4 4) mzero.
Proof. apply f01_strict_lower_nilpotent. exact T4_strict_lower. Qed.
End General.

Compute "F01_ROCQ_NILPOTENT_AND_FINITE_INVERSE"%string.
Compute "F03_ROCQ_APPROXIMATE_SOLVE_BOUND"%string.
Compute "F04_ROCQ_FORWARD_MAJORANT_AND_WEIGHTED_BOUNDS"%string.
Compute "F05_ROCQ_SAFE_INTERVAL_DECISIONS"%string.
Compute "F01_ROCQ_GENERAL_N_NILPOTENT_AND_NEUMANN_INVERSE"%string.
Compute "F03_ROCQ_GENERAL_RESIDUAL_SOLUTION_BOUND"%string.
Compute "F04_ROCQ_GENERAL_N_MAJORANT_DOMINATION"%string.
Compute "ROCQ_FORMAL_PASS"%string.
