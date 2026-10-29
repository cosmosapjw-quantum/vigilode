# N01 nonlinear source-owned reaction kinetics: preregistration (2026-10-29)

Source parent PR114 HEAD `a62c4215fa2c329c6a48be2ccf544ac056b747a5`; PR113 and N00F as inherited conditional evidence. This is the **next bounded physics/source-ownership research node**, not N02 and not full BASS physical33 admission.

## Exact scientific question
An immutable, opt-in 3-species Robertson-type nonlinear mass-action law
`f(a,b,c)=(-k1*a+k2*b*c, k1*a-k2*b*c-k3*b*b, k3*b*b)` with finite represented binary64 nonnegative k1,k2,k3, physical nonnegative state, owns the *same* RHS, Jacobian and JVP algebra. Time is autonomous but bound to a source-owned clock/epoch snapshot; stage certificate targets `A*=I-sigma*J(z_live)`, `b*=f(z_live)`, reference `J0=fl(J(z_ref)), b0=fl(f(z_ref))`, `sigma>=0`, positive w,p. The represented floating outputs of the model are not assumed exact. Every model/application rounding discrepancy must be enclosed by directed binary64 interval arithmetic.

## Hypotheses and falsifiers
H1: compute source-owned `deltaJ >= ||J_exact(z_live)-J0||_w` and `delta_b >= ||f_exact(z_live)-b0||_w` via **independent outward interval evaluations** of the exact polynomial model and represented point matrix/RHS. A reference JVP must be evaluated by the same immutable source and its floating error `eta_apply` enclosed as an outward interval.
H2: `rho_bound=||b0-y||_w+delta_b+eta_apply+sigma deltaJ||xhat||_w`; `mu_w(J*)<=mu_w(J0)+deltaJ`, margin=1-sigma(mu+deltaJ)>0, `err_p<=max(w_i/p_i)*rho_bound/margin`. Admit only if both outward residual and stage-error thresholds pass and all source/clock/epoch/state/parameter identities match. Existing GMRES residuals are triggers only.
H3: no output mutation on a stale clock, epoch or state, false source, NaN, negative concentration, margin failure, callback error or budget rejection. Preserve method/GMRES/default routing untouched.

Kill tests: exact rational 3x3 nonlinear source and stage solve; independent Fraction binary64 oracle (reference vs actual RHS/J/JVP, exact true original residual, exact forward error); deliberately missing callback/source ownership; stale-state/clock/epoch, incorrect norm and hard physical gate; negative/out-of-box states; nonfinite and overflow; transactional no mutation. Existing native primitive rounding floor must be reused unmodified. No relaxing tolerance to pass.

## Execution budget and claims
Rust 1.94.1, target compiled native source in `rodas5p-core` and opt-in real `OdeProblem` factory if API permits, offline mirror test separately classified; no timed benchmark loops without established native stage acceptance. Native repository clone may fail network/DNS: do not equate mirror success with full native cargo workspace. Independent R6/C9 requires distinct actual agent; otherwise NOT_RUN. Max one risk-justified audit/repair.

Official DAG: original VigilODE 7/23; central REI 28/88; proposed successor 0/12 admitted; prior N00F audit 1/1 scoped; new N01 mass-action **0/1**, production HOLD. No merge, force push, default-route mutation or N02.
