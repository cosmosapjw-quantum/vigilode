# VigilODE R9 constructive mathematical + computational contract | 2026-11-01 KST

Selected physmath-research-loop R9 ONLY (reusable theorem/implementation contract); support via one bounded R4-like OpenEvolve↔UCT exploratory test and source-locked numerical verification. Parent PR184 immutable HEAD e7e33fb77599be99f214009ab5a330ebaa3b62d2. Save/publish this prospective lock before new data generation. Scope: original standalone VigilODE generic stiff/oscillatory ODE, NOT REI/BASS applied branches.

## Original source and inherited numeric science

- Original RODAS5P frozen raw-U implementation Git blob d68239d53977e36f98b186b627aa0c4c5c7de7cb, original coefficient fixture SHA256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6.
- Original cooperative cubic RHS Git blob d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482; original coupled case fixture Git blob 1bfc17ec2ed4b902cc06073807388f6e332e31d6.
- R8 science result SHA256 61adc2c39a191b14f7e705dc25fc56914ccaf147fe26331e210ec98c6babe833. R7 actual matched global-error native protected/transactional wall-time median 0.21069269165444293 on three original small source families, not universal no-go.
- Prior R8 actual HOL 3 named scalar/2x2 kernel sublemmas, real offline OpenEvolve 0.4.0 ProgramDatabase and MCTS/UCT with GP-expected-improvement switch, 8-wave source search, original 236-file pre-implementation tree ZIP SHA256 bf78183ea353979fe6bf009e725fa28a7228ed55ca4593f3659eb2add26f03f3. No new external LLM calls.

## Selected new mathematical claim to attempt and falsify

For original source k_i>0,g_i>=0,c_ij>=0,tau=h gamma>0, W=D(I-T) with T nonnegative, zero diagonal, q=max_i sum_j T_ij<1. Choose source-authenticated graph-feedback vertex set B with |B|<=3, complement A acyclic in directed dependency T graph. Then N=T_AA nilpotent (verified by exact topological sort), R=(I_A-N)^-1=sum_{l=0}^{L}N^l. The Schur core S_B=I_B-T_BB-T_BA R T_AB is a nonsingular M-matrix, and for b=D^-1r:

  u_B=S_B^-1 (b_B+T_BA R b_A),
  u_A=R(b_A+T_AB u_B).

Use R8 exact Cayley-Hamilton for |B|<=3, or 1x1 scalar; if B empty, finite topological path solve. Acyclic path inverse should use topological dynamic programming, not exponential enumeration. Original W residual bound ||u-u*||_inf<=||Wu-r||_inf / min_i (1+tau(k_i+3g_i y_n,i^2)). Show exact represented-rational algebra with source-owner identity/epoch/clock/frozen J and no runtime JVP/Newton callbacks; still mathematical Df source owner REQUIRED. Verify Schur positivity, core inverse and all original raw-U stages from four original source cohorts; for |B| small and n>3 use synthetic n12 single-cycle, n14 two-cycle, n24 ring, optional n8 DAG. Dense n8 requires FVS>=7 -> REFUSE, negative signed coupling REFUSE. Synthetic examples are *algorithmic controls*, not independent original physical data.

Explore a nontrivial alternative **signed H-matrix graph elimination**: residual norm bound may survive strict row dominance, but Schur core positivity/homotopy need not. Keep claims separate. Experimental hybrid branches at each wave: (i) RODAS5P+cycle-core Schur+protected fallback, (ii) BDF2/BE homotopy+RODAS as two independent solvers, (iii) exponential-Krylov+graph-source certificate. No invalid substitution of BE nonlinear root into original linear ROW stage.

## Evaluation and discovery protocol

- R9 mathematical theorem and executable source-scoped contract, not C5 native production integration.
- 4 original first timestep cases with exactly 8 raw-U stages each; original h/clock/source/tableau from R6; 90-digit reference inherited from R6; exact dyadic-rational original W residual, strictly outward bound; n12/n14/n24 algorithmic holdouts, all samples and failures preserved.
- B FVS selected deterministically from physically declared cycle core; validate A acyclicity and ALL cycles hitting B. Do not pretend optimal/minimal FVS in large graphs. Use algebraic operation counts and actual sandbox CPU timing only as micro-economics, no Rust-native wall speed claim; R7 work-precision remains ground truth for method routing.
- OpenEvolve/MCTS 8 waves (4/4), mandatory one genuinely new graph/signed-H node + one hybrid branch with >=2 distinct theories/integrators EACH wave, GP expected-improvement operator allocation (with bounded 4/4 quotas), search candidates reweighted by actually measured source residual, S size, original R7 negative work-precision; reward is heuristic not a posterior theorem probability; preserve false premises with counterexample, actual vendor ProgramDatabase, deterministic replay PYTHONHASHSEED=0, render charts in final response.
- Proof assistant: reuse preinstalled sandbox OCaml5.3 HOL Light3.1, run new named kernel theorems on block Schur scalar identity, not pretend full n arbitrary theorem proved. Wolfram separate 2x2 block elimination symbolics.
- One primary validation pass + one same-author hostile scope-specific review, no native distinct-author C9 unless actual agent available.

## Stopping/admission

SOURCE/GRAPH/CORE_EPOCH_MISMATCH, unsupported |B|>3, non-Metzler source, nonzero original residual above requested error, proof failure, or inflated model sample count -> reject scoped candidate/fallback to protected original RODAS5P. No merge, no default/tolerance change, no production promotion. OFFICIAL_VIGILODE_DAG=7/23_UNCHANGED; SUCCESSOR_ADMITTED=0/12_UNCHANGED; PRODUCTION=HOLD. Pending global authority CI on PR184 is separate release blocker, never weaken.
