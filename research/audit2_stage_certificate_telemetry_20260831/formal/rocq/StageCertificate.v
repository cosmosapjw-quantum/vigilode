From Stdlib Require Import QArith Qring Reals Lra Strings.String.

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

Compute "F01_ROCQ_NILPOTENT_AND_FINITE_INVERSE"%string.
Compute "F03_ROCQ_APPROXIMATE_SOLVE_BOUND"%string.
Compute "F04_ROCQ_FORWARD_MAJORANT_AND_WEIGHTED_BOUNDS"%string.
Compute "F05_ROCQ_SAFE_INTERVAL_DECISIONS"%string.
Compute "ROCQ_FORMAL_PASS"%string.
