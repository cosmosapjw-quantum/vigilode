# Independent exact native oracle contract

Parent theory admission: `../THEORY_ADMISSION.md`. Native source owners implement the cyclic closure and trace routes separately. This lane owns `verify_native.py` and this contract only. It does not import either native implementation, generated circuits, or attachment certificate routines.

## Execution and identity

The parent publishes the actual native sources, this oracle and the experiment protocol before the claimed scientific runs. Authoring and Python syntax compilation are preparation; neither is a numerical verification result. Execute only after the parent's GO:

```sh
python3 research/production_rvj_nag_20261028/code/verify_native.py \
  --closure /absolute/new-run/closure.json \
  --trace /absolute/new-run/trace.json \
  --output /absolute/new-run/exact_oracle_receipt.json
```

Either `--closure` or `--trace` may be omitted for a separately recorded phase. The output is create-only; an existing receipt is never overwritten. The receipt records SHA256 for the script and complete input bytes, input byte sizes, execution time, explicit failed check and completed-check count. The caller preserves command/environment/stdout/stderr and source/binary identities. Nonfinite JSON constants, duplicate object keys and nonfinite IEEE words are rejected.

Every scientific inequality uses `fractions.Fraction`, including exact recovery of represented binary64 input. Decimal/float displays are not the acceptance arithmetic. The exact common physical cap is `1/100000000`; the native `1e-8` metadata is checked separately as represented binary64. Coarse-grid failures are retained as physical outcomes. An honest `physical_budget_met:false` does not fail the verifier; a false claimed acceptance does.

## 1. Cyclic closure: original target and independent construction

Original equations, with cyclic indices:

\[
G_i(x,q)=q_i+q_{i+1}^2/4-x_i,\qquad
f_i(x,q)=-x_i+(q_i+q_{i+1})/16+1/32+(i\bmod3)/256.
\]

The domain is X=[-1/8,1/8]^n, Q=[-1/4,1/4]^n, t∈[0,1], with zero initial x and q. The circuit trajectory is certified as the **exact-original-field cubic Hermite path through its represented endpoint values**. This is a precise reconstruction definition; it is not a claim that unspecified native dense output has the same error.

The tangent equations are

\[
v_i+b_i v_{i+1}=f_i,\qquad b_i=q_{i+1}/2.
\]

The oracle uses an independent explicit cycle expansion

\[
v_i=\frac{\sum_{k=0}^{n-1}(-1)^k
 (\prod_{j=0}^{k-1}b_{i+j})f_{i+k}}
 {1-(-1)^n\prod_{j=0}^{n-1}b_j}.
\]

It then verifies the original tangent equations exactly. This O(n²) reference does not reuse the native O(n) backward recurrence. Reusing a checked exact slope at an identical endpoint within the oracle is an evaluation cache, not native evidence reuse.

For endpoint values y0,y1 and exact slopes s0,s1, each component is first constructed in the power basis:

\[
H(\theta)=y_0+h s_0\theta+
 [3(y_1-y_0)-h(2s_0+s_1)]\theta^2+
 [2(y_0-y_1)+h(s_0+s_1)]\theta^3.
\]

The oracle substitutes these power polynomials directly into **G** and **H'x−f(H)**. It independently converts the resulting power coefficients using

\[
\beta_i=\sum_{k=0}^i \frac{\binom{i}{k}}{\binom{d}{k}}a_k.
\]

Native direct Bernstein multiplication/elevation formulas are never called. Every exported path, slow-residual and algebraic-residual interval must contain the corresponding exact coefficient.

### Full finite coverage

Configurations are n∈{4,8,16}, N∈{16,32,64,128}. The three arms are:

- `historical_power_legacy`: original power/convolution/Bernstein bound and conservative radius.
- `direct_bernstein_legacy`: direct coefficients with the proved endpoint-identity weights and conservative radius.
- `direct_bernstein_tight`: the same direct residual bound and proved maximum-of-two-rationals propagation.

There are **720 distinct slabs and 2160 arm/slab records**. All are checked. Each record contains 8n path, 4n slow-residual and 7n algebraic-residual intervals: **383040 coefficient intervals** across the stated grid. Exact path polynomials are constructed once per configuration/slab and checked against every arm's exported intervals. No representative-slab sampling is used.

For each exact slow residual, its two endpoint coefficients must be exactly zero. For each exact algebraic residual, beta1=beta0 and beta5=beta6 must hold. These follow from the exact field reconstruction, not from an interval cancellation operation. The direct residual bounds are checked against

\[
D_f\ge\tfrac34\max_i\max(|r_{i1}|,|r_{i2}|),
\]

\[
D_g\ge\max_i\max\{g_i,(7g_i+25m_i)/32\},
\]

where gi bounds the two endpoint coefficients and mi the three middle coefficients. The native implementation may use the more conservative global g/global m formula. Both are valid; the oracle does not demand an artificial agreement of rounding or slack. The historical arm must bound the unweighted maximum of all exact coefficients.

### Error propagation and acceptance

Every arm has its own incoming-radius chain. The next record's `e0_bits` must equal the previous record's returned `ee`. The represented h must equal exactly 1/N. Define delta=Df+Dg/7 and z=6h/7 using exact rational arithmetic on **exported native bounds**.

For the conservative arms, the oracle requires

\[
E_w\ge E+h\delta,\qquad E_+\ge E/(1+z)+h\delta.
\]

For the tight arm it requires

\[
E_+\ge\max\left\{\frac{E+h\delta}{1+z},
\frac{(1-z/2)E+h\delta}{1+z/2}\right\},\qquad
E_w\ge\max(E,E_+^{\rm exact\ formula}).
\]

Finally Qw≥8(Ew+Dg)/7 is checked using the actually exported Ew, preventing a hidden uncharged rounding gap in the composition. Both exact path coefficients and native coefficient intervals must remain in the declared X/Q boxes. The arm's cap claim must match max(Ew,Qw)≤1/100000000 over all slabs.

A negative control deep-copies an actual exported record and changes its first slow endpoint coefficient interval to [1,1], while the independent exact coefficient is zero. The same coefficient checker must reject it. This verifies the external checker's behavior; it is not described as a native injected-fault experiment.

### Cyclic JSON contract

Top-level fields are `schema="vigilode.rvj_closure_campaign.v1"`, `path_definition="exact-original-slope-hermite-v1"`, `cap`, `warmups`, `repeats`, `configurations`.

Each configuration contains `n`, `steps`, `endpoint_bits` and `arms`. Endpoint rows list exactly 2n u64 decimal strings, x components followed by q components. Each arm has `name`, seven `raw_timings`, `counts`, `max_df`, `max_dg`, `max_ew`, `max_qw`, `physical_budget_met` and `slabs`. Each timing row includes `candidate_seconds`, `preparation_seconds`, `certificate_seconds`, `total_seconds`.

Each slab contains `index`, `h_bits`, `e0_bits`, `df`, `dg`, `ew`, `ee`, `qw` and corresponding scalar `*_bits`, plus arrays `path_bernstein[2n][4][2]`, `slow_bernstein[n][4][2]`, `algebraic_bernstein[n][7][2]`. Numeric interval endpoints must round-trip the native binary64 value. Exact bit words are authoritative, and where both encodings are present they must agree.

Native interval certification is part of the timed cyclic execution. The independent Fraction replay is offline validation of that certificate and is reported separately. It is not silently added to or removed from a different timing claim.

The declared counter identities are checked against the exported counters: candidate RHS/solves4N; baseline interval slopes/solves2N versus direct N+1; baseline original-G preparation0 versus direct N+1; direct endpoint reusesN−1; attempted/certified slabsN. The baseline has4nN power-to-Bernstein transforms and16nN convolution products; the direct arm has nN slow and nN algebraic direct components. Reported Jacobian/JVP/AD/Newton/matrix-factorization counts must be zero. These checks verify receipt consistency with the finite algorithm; they do not pretend that an external JSON reader independently instruments native callbacks. Source review and native counter tests remain separate evidence.

## 2. Trace: full quotient semantics and original field

The supported observable is the sum over **all complex roots counted with algebraic multiplicity** of

\[
P(z)=z^4-2az^2+z+a^2-b.
\]

The independent point oracle builds the companion multiplication matrix

\[
M=\begin{pmatrix}0&0&0&b-a^2\\1&0&0&-1\\0&1&0&2a\\0&0&1&0\end{pmatrix}
\]

and multiplies exact Fraction matrices to compute Tr(M^k), k=0,…,6. It does not use the native closed moment formulas. Points include an ordinary point, a=b=3/4 (triple root), and a=b=−1/4 (complex pair). Native shared/direct field enclosures are compared with the **original** combination

\[
G_1=-a+S_2/32+S_4/128+S_6/512,\qquad
G_2=-b+S_2/64+S_4/64+S_6/256.
\]

The invariants q1²+q2 and q1q2 are checked through q2=a−q1² and the independent matrix traces. `selected_branch` and `real_only` requests must explicitly report rejection. Replacing the triple root's length-four trace S0=4 by the number of distinct roots2 must be rejected by the oracle's negative control.

### External whole-path certificate

The manufactured trajectory is

\[
p(t)=3/4+t/8+t^5/64,\quad t\in[-1,1],\qquad
F(t,y)=G(y)+p'(t)(1,1)-G(p(t),p(t)),
\]

with exact initial state (39/64,39/64). Both native routes (`direct_moments`, `shared_polynomial`) export every represented state at N∈{16,32,64,128,256}. All **992 slabs** are checked. Time bits must match the declared exact dyadic schedule, not just an approximately equivalent clock.

For each path, the oracle constructs exact-original-slope cubic Hermite polynomials, substitutes them into the original S2/S4/S6 field and the quintic control in the **power basis**, and obtains the entire residual polynomial. Its Bernstein hull supplies a closed-slab defect bound. The sharp admitted lognorm constant lambda=39/64 is used with the conservative valid propagation

\[
E_w=E+h\delta,\qquad
E_+=\operatorname{ceil}_{112}\{E/(1+\lambda h)+h\delta\}.
\]

The upward dyadic quantization prevents denominator growth; it is exact enclosure arithmetic, not an empirically fitted margin. Every complete error tube must lie strictly inside [1/2,1]². The oracle separately bounds the exact polynomial difference between the candidate path and (p,p), retaining it as an independent reference result rather than substituting it for residual-only certification. Original node errors are computed exactly from stored bits; native rounded node diagnostics alone never establish whole-path validity.

The native trace implementation initially supplies point interval field enclosures and a candidate path. Consequently this Fraction result is explicitly **an external exact whole-path certificate, not an implemented native binary64 interval path certificate**. The receipt records each slab's exact residual/error/domain quantities and per-run external certificate time. If a result is presented as trace integration-plus-certification total time, that external work must be charged. Native integration-only timing remains correctly labelled.

### Trace JSON contract

Top-level fields include `schema_version=1`, `task_id="RVJ-TRACE-20261028"`, `semantics="all_complex_roots_algebraic_multiplicity"`, `interval_cases`, `semantics_checks` and `runs`.

Each interval case contains `name`, `a_bits`, `b_bits`, `moments[7][2]`, `field[2][2]`, `field_direct[2][2]`, `invariant_q1_squared_plus_q2[2]`, and `invariant_q1_q2[2]`.

Each run contains `route`, `slabs`, and `states`; every state contains `t`, `t_bits`, `state[2]`, `state_bits[2]`. JSON floats and their bit words must agree. Semantic records contain `query`, `expected`, and `actual`, with `accepted`/`rejected` values. Native work/timing metadata remain in the full raw input for the parent's performance analysis.

## 3. Failure handling and limits

Malformed input, mismatched source format, omitted/duplicated required configuration, missing slab, inconsistent represented time, interval non-enclosure, invalid recurrence, domain failure or false native cap claim produces a nonzero exit and a preserved FAIL receipt. The program does not retry a failed scientific case or modify inputs. A real failure belongs to the parent-controlled correction episode and existing N03/N05 budgets.

A passing receipt verifies the finite exported cohort and the admitted model equations. It does not claim arbitrary sparse compilation, selected singular-branch continuation, generic stiffness-uniform order, downstream API compatibility, all historical N/I gates, or broad production qualification. Those remain in `../theory/PRODUCTION_GATES.json` and the parent's local handoff.
