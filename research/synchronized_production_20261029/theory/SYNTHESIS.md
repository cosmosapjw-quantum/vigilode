# Synchronized residual-to-trajectory theorem

This final theory loop joins the three parallel units after their independent
R6 review. The target is one numerical lane with one ownership/accounting
contract: proposed linear-stage work, original-equation trajectory defect,
chronological certificate transport, and returned public values. The current
native patch changes private storage and return ownership only. New quotient
and nonlinear-tube consumers remain executable exact research leaves until
native bound providers exist.

## S1. Triangular stage-error transfer

Fix one attempted step, its represented initial state/time, frozen operator
and all source/norm epochs. A reference stage system has s ordered stages

A_i k_i = b_i(k_1,...,k_(i-1)).

In one fixed norm assume ||A_i^-1|| <= 1/alpha_i with alpha_i>0 and

||b_i(u)-b_i(v)|| <= sum_(j<i) L_ij ||u_j-v_j||,

for nonnegative L_ij on a domain containing both stage tuples and all
intermediate tuples used by this Lipschitz estimate. A computed tuple hat k
has original-system residual r_i=b_i(hat k_<i)-A_i hat k_i with
||r_i||<=d_i, including operator/action/source/rounding uncertainty exactly once.
Let eta_i=||hat k_i-k_i||. Subtract the two equations and apply the inverse
bound:

eta_i <= (d_i + sum_(j<i) L_ij eta_j)/alpha_i.                 (S1)

Let D_i=d_i/alpha_i and T_ij=L_ij/alpha_i for j<i, zero otherwise. T is
entrywise nonnegative and strictly lower triangular, hence T^s=0. The
unique recurrence B_i=D_i+sum_(j<i)T_ij B_j gives

eta <= B = (I+T+...+T^(s-1))D.                              (S2)

Proof: stage1 follows directly; the nonnegative induction substitutes earlier
eta_j<=B_j into S1. Nilpotence proves the finite inverse formula, without an
assumed spectral radius or iterative convergence. This is an a posteriori
stage perturbation bound, not the time integrator's truncation error.

For VigilODE Rosenbrock stages, the same represented frozen J and h define
A_i=I-h gamma_i J. K3's signed-norm coercivity supplies alpha_i when verified.
The RHS dependence includes the actual stage coefficients, stage-state RHS
Lipschitz bounds, mass/clock terms and all Jacobian products; L_ij must be
derived from the selected native method. No universal constants are inserted
by this package. Existing full-space structural providers remain unchanged.

A direction-specific quotient proposal from K3 gives one way to reduce the
work needed to construct hat k_i. Its relation defect yields d_i only under
K3's coefficient/action uncertainty contract, or a separately enclosed original
residual provides d_i directly. Both routes charge failed candidate work and
fall back under the original threshold. In particular, a polynomial relation
for stage1's RHS does not automatically apply to later stages.

## S2. Linking stages to continuous trajectory without double counting

Let a reconstruction map produce p=R(k) and hat p=R(hat k). Suppose on this
fixed slab it satisfies uniform bounds

||hat p-p|| <= C0 ||hat k-k||_block,
||hat p'-p'|| <= C1 ||hat k-k||_block,

The block norm is monotone under componentwise nonnegative bounds (the helper
uses max_i eta_i; positive weighted l-p norms also qualify). Assume also that
the original field is Lipschitz with constant Lf on the segment joining
both curves. If ||p'-f(t,p)||<=delta_base, the triangle inequality gives

||hat p'-f(t,hat p)|| <= delta_base+(C1+Lf C0)||B||_block.    (S3)

The proof adds and subtracts p' and f(t,p), then uses S2. The derivative term
has units state/time, so C1 and Lf*C0 have the same units. This alternative is
useful when a validated reconstruction perturbation bound is available.

If the actual represented reconstruction hat p already has an independently
certified original residual ||hat p'-f(t,hat p)||<=delta_direct, use that bound
instead. Its value includes the stage-solve effects. Adding S3's stage term
again would double count. Conversely, an embedded estimator does not supply
either delta_base or delta_direct. For delta_direct against the exact original field set epsilon=0. An unevaluated physical source discrepancy
remains a distinct epsilon, as in D1–D2.

## S3. Admissible route-word composition

For each actual accepted slab, feed a valid residual bound from S2 into the
DEFECT theorem with alpha=lambda_carrier-deltaJ, Gamma, source offset epsilon,
and a supplied R. Verify incoming E0<=R and
Gamma R^2-alpha R+(delta_residual+epsilon)<=0 on the declared physical tube.
D1–D9 then give its endpoint and whole-slab error bounds. A boundary map
transfers E_next<=kappa E_end+eta_reset before the next slab's admission.

Induction on the chronological route word proves that every represented
endpoint and every covered slab has its stated physical bound. Hypotheses
include: complete interval coverage without gaps, the same target IVP,
verified source/clock/norm identities and all map domains. Neither matching
controller letters nor matching endpoint bits supplies these hypotheses.
Event signs/crossings may then use D5's continuous margin/transversality tests.

The new native ownership transformations instantiate kappa=1,eta_reset=0:
they transfer the same floating sequences, do not approximate the physical
state, and preserve callback value traces under A1/K1. Thus they introduce
zero additional *numerical* error in this composition. Their measured memory
cost can fall without spending a physical error budget. They do not by
themselves create an original-field provider for an uncertified client.

## S4. Unified cost accounting and the implemented lane

Write total work as W= W_setup + W_committed + W_discarded + W_certificate
+ W_export. All terms are charged once to the actual execution. K1 reduces
unused pool preparation; A1 reduces W_export. K3 may lower Arnoldi/MGS/QR
work for a valid low-degree directional relation, but adds proposal/residual
cost and may lose after a fallback. The strict switcher theorem compares
same-window completed work and retains speculative W_discarded. Therefore
it supplies no generic speedup relative to a different RHS-only trajectory.

The single executable lane is `code/run_lane.py`: one published source and
unchanged Cargo.lock; exact theory consumers; the existing protected facade
and Krylov contracts; one paired baseline/candidate benchmark protocol; then
one existing selective-JVP ProductionConfig dispatcher on two manufactured
invocation fixtures. These invocation fixtures establish configuration and
accounting behavior only. The actual physical-client, API, historical-profile,
full interaction, release/platform and authority gates remain separately open.

The production implementation does not dynamically switch to K3 or D's Python
oracles. Their contracts and exact tests are the local Codex transplantation
input. One lane means one concrete source/configuration/execution record,
not a waiver of the outstanding production gates.

## Final loop R1–R10 and C1–C10

R1 fixes the shared lane objective; R2 reuses K/A/D and prior PR contracts;
R3 states S1's complete domain/epoch assumptions; R4 selects residual-first
composition; R5 proves S1–S4; R6 requests independent source/math review;
R7 executes exact triangular and mutation tests after source publication;
R8 admits only scoped claims; R9 supplies the code/native handoff; R10 freezes
results and remaining obligations. R6/R7 are pending until their receipts.

The coding loop follows that contract: C1 source pins; C2 functions and bounds;
C3 exact Fraction consumer design; C4 bounded runner; C5 implementation;
C6/C7 exact and native tests; C8 matched diagnostic measurements; C9 independent
diff/data review; C10 one published delivery and local handoff. Step completion
is recorded only with its actual evidence, not by this enumeration.
