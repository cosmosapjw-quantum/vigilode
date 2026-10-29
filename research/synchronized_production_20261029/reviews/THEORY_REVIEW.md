# Independent theory review — R6 admission

Reviewer: `/root/state_audit`, a separate agent from all three theory authors. Date: 2026-10-29 KST. Baseline native source: PR101 `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087`.

**Verdict: ACCEPT_SCOPED for A1 consumed-output implementation, K1 private basis-only preparation, K3 exact-rational research oracle and D1–D10 exact-rational certificate consumer.** Native physical-provider/quotient/recycle admission remains governed by their additional stated contracts. The four definition clarifications below are accounted for separately from the proofs; two adapter and two defect clarifications have been read back in the author documents. K4's fixed-RHS clarification is a condition already implicit in its fixed-system residual definition and is explicitly requested for the final text.

The review read actual source rather than accepting author summaries: `crates/vigilode/src/solve.rs` routed and direct output paths; `crates/vigilode/src/switching.rs` integer work, strict comparator and ceiling helpers; `crates/vigilode/examples/support/production_return_cases.rs` model/config/reference; `crates/rodas5p-krylov/src/workspace.rs`; `gmres_into.rs` reserve/cycle/column/correction/monitored paths; and `crates/vigilode-coupled/src/trace_certificate.rs` propagation. No solver source was changed by this review and no native execution result is asserted by it.

## A1–A4: independent challenge and derivation

### A1 value-preserving move

The routed source first clones `partial`, later clones `routed.run.observed()`, and non-dense output then clones observed `t/y` and an appended partial state. Metadata is read before output projection; `StopRecord` can be constructed before moving partial ownership. The return value retains dense extension ownership separately.

The independent simulation relation is equality of all live numerical/scalar/string values, with unconstrained allocation identities. For every clone-to-move replacement, elements keep identical bits. Predicates therefore select identical branches, and the partial append condition remains `last < partial.t`. Callback equality follows from the stated same-value/history effect contract; the theorem explicitly excludes callbacks which make allocator addresses/counts part of their scientific input. Thus the implementation may remove copying without adding an arithmetic error term.

I checked the possibility that an event callback sees an altered allocation address. This is handled by the explicit observation contract, not hidden by calling callbacks pure. OOM/panic/resource behavior is outside the successful-execution theorem. Native tests still must cover fatal and ceiling stops, equal-time partial rows, zero span, dense event output and complete Stats/callback parity.

Finding A-R6-1: the initial savings paragraph attributed removal of a routed clone to dense output. Actual dense-output plans are FastDense/Sequential; that routed path is unreachable for this scope. The author corrected the savings statement. **Closed.**

### A2 owned session vs numeric cache

A pause boundary can be an identity operation on the live solver state only when schedule/controller/FSAL/epoch/budget provenance is retained. A fresh implicit step invalidates the prior explicit numerical provenance. The proof does not use dimension equality to authorize reuse of cached numbers. A1 output consumption has no such cache dependency, so it is appropriately the first implementation.

### A3 strict local cost vs global policy cost

The actual objective uses checked integer accumulation:
`W = rhs + jvp_vectors + ft + n*jacobian_builds + linear_solves`.
For a deterministic same-window continuation, charged prefix `L` satisfies `L<=I`; therefore `C<L` proves `C<I`. The entry guard's `E-1` and return guard's `C` agree with the source's stop-on-charged-cost-greater-than-cap semantics. Overflow yields missing evidence. Every local conclusion also needs the explicit same-start/same-window/permission conditions.

The independent global decomposition is `W_policy=P+D`. A local winning productive segment constrains no unrelated RHS-only trajectory. The observed1.428x work loss therefore does not contradict the local theorem. The conditional guard `D<=beta*P+C0` yields `W_policy<=(1+beta)P+C0` by addition and gives no ratio against another trajectory without another lemma. No policy change is admitted by A1.

### A4 clock–fiber signs and domains

Directly from the actual benchmark source,
`F0=1`, `Fj=-lambda(x0)*phi(xj-g(x0))+g'(x0)`,
`phi(e)=e+e^3/2`.
With `rho=hat x'-F(hat x)`,
`hat e'=hat xj'-g'(hat x0)*hat x0'`
equals
`-lambda(hat x0)*phi(hat e)+rhoj-g'(hat x0)*rho0`.
This independently confirms the **minus sign** on carrier-clock defect. Subtracting the exact fiber equation using `lambda(x0)` gives contraction plus
`[lambda(x0)-lambda(hat x0)]*phi(hat e)+rhoj-g'(hat x0)*rho0`.

For `d=hat e-e`, the divided difference is
`1+(hat e^2+hat e*e+e^2)/2>=1`,
including the diagonal limit. Consequently the forcing bound is
`A+B*s`, with `A=deltaj+G*delta0+L*Phi(Btube)*C`,
`B=L*Phi(Btube)*delta0`.
Integrating the positive kernel gives
`integral_0^s exp(-k*(s-u))*u du=(k*s-1+exp(-k*s))/k^2`.
Differentiating the stated envelope and adding `k` times itself reproduces `A+B*s`; its initial value is the declared incoming fiber error. All terms have state units. Conversion back to original coordinates adds `G*(C+s*delta0)`. The whole-slab simpler bound safely drops damping and integrates the affine forcing; it does not rely on sampled endpoints.

Finding A-R6-2: jumps were not explicitly labelled by coordinate basis. Original-coordinate jumps `D0,Dj` imply transformed fiber jump `Jj<=Dj+G*D0`; adding only `Dj` would omit clock-carrier forcing. The author now defines fiber jumps and gives this conversion. A1 has zero jumps. **Closed.**

Additional directly derived current-model constants sent to the author: for the reentrant two-tanh rate, `q1>=q2`, so `1<=lambda<=10000`; both derivatives lie in[0,200], so `|lambda'|<=999900`; `g=cos(t)+0.01sin(150t)` gives `|g'|<=5/2`. These analytic constants require the stated native outward-transcendental contract if used by compiled certification. A reconstruction bound on `|hat e|` is still required.

## K1–K4: independent challenge and derivation

### K1 basis-only preparation

Actual `gmres_into::begin_cycle` prepares Arnoldi then fully overwrites/normalizes `basis[0]`. Columns use `basis[j]` and write `basis[j+1]`; the right-preconditioned temporary is `raw_operator_output`. Final and monitored correction use a linear combination of `basis[..actual]`. None reads `directions`.

Thus suppressing only directions allocation/resize preserves the live arithmetic relation by induction on columns, then on cycles. Callback errors occur on the same logical call and output is committed only at the success points. Existing full `reserve` and legacy/augmented preparation must remain full; otherwise the public capacity and mixed-route contracts would change. Existing directions must not be discarded merely because into does not read them.

Capacity/retained-byte/growth telemetry intentionally changes and must describe actual resources. It is not included in numerical-counter parity. Cold m-vector savings are a prediction to be tested; the m=16 profile association alone is not proof of allocation attribution. **Admit the stated private K1 implementation.**

### K2 residual authority

Using `AV=VH+F`, `r0=beta*v0+g0` and `x=x0+Vy` gives
`r=V(beta*e1-H*y)+g0-F*y`.
The negative sign is correct. No orthogonality is needed for the identity. A Gram certificate bounds the rectangular induced norm of V; absent it, the sum-of-column-norm-squares bound is valid. Assembly error contributes at most `||A*delta_x||`. This establishes why dropping a second MGS pass would require a new residual/Gram argument. K1 changes no such numerical operation.

### K3 RHS-specific quotient

I independently compared coefficients of `(1-sigma*z)Q`: the constant is `D-sigma^d*p0`, each interior coefficient is `-sigma^d*pk`, and the top coefficient is `-sigma^d`. Therefore `r=sigma^d*p(J)v/D` has the stated positive sign. The d=2 substitution yields
`D=1-a*sigma-b*sigma^2`,
`Q=(1-a*sigma)+sigma*J`,
which agrees with the construction. At sigma0, the polynomial definitions give D=1,Q=I without reciprocal-sigma evaluation. D0 is a route refusal, not proof that the original linear system is singular.

The units of `p_j` are time^(j-d), of `N_k` time^k, and D,Q(J) are dimensionless. Weighted dissipativity gives
`<z,(I-sigma*J)z>_W >= alpha*||z||_W^2`
for nonnegative sigma. Cauchy–Schwarz implies a positive smallest gain and then the finite-dimensional inverse bound1/alpha. The RHS-only example has p(J)=diag(0,0,56), so its zero third-coordinate v is genuinely weaker than full-space annihilation. A near third coordinate1/1000 yields7/125 exactly.

The two floating-error contracts are alternatives: either candidate-distance error or direct residual error; adding both without ownership would double count. The exact-rational research helper is admitted. Native model/metric/provider and outward residual admission remains separate.

### K4 shifted images

Expansion of `rho*C0+(1-rho)*U` gives `(I-sigma1*J)U` exactly. With `F0=A0U-Chat0`, the new defect is `rho*F0-deltaC`; the triangle bound uses abs(rho). A fixed nonidentity left preconditioner replaces U by P^-1U in the affine formula. C1 is not automatically orthonormal; paired U/Pi/R^-1 update and rank control are required as stated.

Clarification K-R6-1: the general `r1=r0-deltaA*x` statement assumes fixed RHS. With a changing RHS the additional term is `delta_b`. This is requested explicitly; it does not affect K1/K3 coding admission.

## D1–D10: independent challenge and derivation

The original error equation has a carrier term A*e, linear mismatch L*e, nonlinear N and forcing equal to minus the candidate defect. The sign of the forcing disappears under its norm. Hence `D+E<=-(lambda_c-deltaJ)E+Gamma*E^2+(delta_r+epsilon)` is correct. Carrier mismatch subtracts from damping rather than acting as a constant source.

For the closed boundary q(R)=0, I checked the supporting-functional radial-projection proof rather than relying on the insufficient bare phrase “nonpositive boundary derivative.” Uniform local Lipschitz gives a bound on positive-part distance outside the tube and Gronwall prevents first exit. Compact containment permits continuation. R0 forces zero initial error/forcing, so exact uniqueness handles it independently of beta.

Inside the invariant tube, `Gamma*E^2<=Gamma*R*E` yields beta. D4 implies `beta*R>=delta`, giving beta>=0 for positive R; beta0 forces delta0. This validates every degenerate branch. For positive beta, both rational exponential bounds are correctly oriented. Their affine maximum handles either sign of E0-w. Since w<=R and E0<=R, the true comparison envelope stays within R; taking min(R,U) preserves a valid endpoint upper. Its monotonic dependence on incoming E0 is established by nonnegative slopes. The whole-slab maximum follows from monotonicity of the exact comparison envelope, not of a sampled numerical path.

Finding D-R6-1: the old/new weight ratio needs a norm convention. The author now defines `||x||_w=max_i|x_i|/w_i`, for which max(old/new) is correct. **Closed.**

Finding D-R6-2: the finite uniform Lipschitz constant used in the equality proof was implicit. The author now states locally Lipschitz in x uniformly on compact time-state neighborhoods. **Closed.**

The transfer identity `F(T(E))-T(F(E))=(q-1)*eta+(1-kappa)*d` is correct. It proves why reset/norm transport must remain in chronological order. The event-sign argument uses a verified tube Lipschitz bound; uniqueness uses a strict original Lie-derivative sign, so grazing and boundary equality remain unresolved. No approximate-bisimulation claim is inferred.

The exact rational consumer can validate declared bounds and return scoped radius/transport decisions. It must not manufacture provider constants or issue a native VerifiedModelProvider. The inherited PR-N05 endpoint-slope reuse opportunity remains deferred at its exhausted counter; this review grants no extra correction budget.

## Budget ownership and review closure

A1 is the consumed owned-output part of existing PR-N04, whose intake counter is1/3. K1 is the active PR-N01 workspace/algorithm cost separation, intake0/3. The new exact-rational K3 and joint-defect composition helpers are mathematical research consumers; they do not reopen exhausted native generic-closure PR-N05. Global correction episodes and applicable inherited implementation counters are owned by the parent campaign ledger. The clarifications here occurred before code admission and do not retrospectively reclassify old failures.

No new strong no-go is proposed in these documents. NG1–NG7 therefore do not grant or obstruct these constructive conditional results. The review is a proof/source review. Native bitwise parity, allocation counts, benchmark/profiler outputs and integration interactions must be assessed from the ensuing exact execution source and receipts.

