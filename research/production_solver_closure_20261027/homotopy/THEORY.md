# Scalar homotopy specialization and a verified cubic inverse

## Scope and inherited theorem

The sealed model is y'=-ky-gy³, k>0, g>=0, Y=y(0)>=0. PR91's
`CubicDecayAccuracyPlan` and every scientific constant are reused unchanged:
L=k+3gY², C=L²(kY+gY³)/8, D=1+hk/2, the smallest exact dyadic grid satisfying
NCh³/D<epsilon and q_B=(h/2)[k+3g(Y+epsilon)²]<1, and residual budget
D[epsilon-NCh³/D]/N. These are model bounds, not fitted margins.

The original increment target is F(K)=K+h phi(y+K/2)=0. For every real K,
F'(K)>=D, so |K-K*|<=|F(K)|/D. The exact midpoint step is globally
nonexpansive. The existing outward interval calculation also includes actual
state-addition and time-label rounding. We reuse that theorem and implementation;
only candidate generation changes. The published baseline is PR91 commit
206298b4068d923bb47c7160b111b7439098660c, with source hash in BASELINE_IDENTITY.json.

## Theorem H1: stable positive-sum inverse representation

For a>0, b>=0, y real, the equation a m+b m³=y has exactly one real root.
Set beta=b y²/a³, z=3 sqrt(3 beta)/2, u=(z+sqrt(1+z²))^(1/3), and

    s=3/(u²+1+u⁻²),    m=(y/a) s.

This formula includes b=0 and y=0 by direct substitution, with beta=z=0,
u=s=1. It uses no subtraction of nearly equal cube roots.

Proof. The derivative a+3b m² is strictly positive and the polynomial has
opposite limits at infinity, establishing existence and uniqueness. For beta>0,
u³-u⁻³=2z, because (sqrt(1+z²)+z)(sqrt(1+z²)-z)=1. Factor the left side:
(u-u⁻¹)(u²+1+u⁻²)=2z. Consequently
s=(u-u⁻¹)/sqrt(3 beta). Since (u-u⁻¹)³+3(u-u⁻¹)=u³-u⁻³=3 sqrt(3 beta),
we get beta s³+s=1. Substitution m=(y/a)s gives a m+b m³=y.
The zero limits follow immediately from the positive-sum formula.

The constants 3, 3/2 and 27/4 are exact algebraic coefficients from the
triple-angle/cube identity, not solver hyperparameters. Wolfram independently
reduces s+beta s³-1 to zero with beta=(u³-u⁻³)²/27; see WOLFRAM_RECEIPT.json.

For midpoint a=1+hk/2 and b=hg/2. At any accepted represented state,
|y|<=B=Y+epsilon. Therefore 0<=beta<=bB²/a³<1/3 and 0<=z<3/2 under q_B<1.
The formula's radical/cube-root arguments are thus bounded on the admitted
mathematical domain. In code beta uses (h/2)(g y²)/a³, avoiding formation of
b=hg/2 when separate large parameters would create an unnecessary intermediate.
The actual floating-point predictor remains untrusted. Nonfinite arithmetic or
a candidate outside the increment bracket is refused with partial work retained.

## Theorem H2: certificate inheritance independent of predictor accuracy

Let Khat be any finite candidate that reaches `cubic_verified_step`, regardless
of how it was generated. Let R enclose its original F(Khat), let delta enclose
the represented update displacement |fl(y+Khat)-(y+Khat)|, and let Dlo<=D.
Then the returned state differs from the exact midpoint image of y by at most
alpha=mag(R)/Dlo+delta. If alpha satisfies the unchanged algebraic step budget,
the existing nonexpansive error induction and physical cap apply verbatim.

Proof. Strong monotonicity gives |Khat-K*|<=|F(Khat)|/D. The interval enclosure
and directed operations bound this by mag(R)/Dlo. Add the actual represented
update displacement. The previous physical-error induction then gives
E_n<=nCh³/D+sum(alpha_j), plus the already accounted time-label displacement.

The runtime cube root therefore requires no correct-rounding assumption for
the final certificate. The library implementation supplies a candidate only.
If its original outward residual misses the budget, Scalar Picard supplies a
second candidate and monotone bisection retains the established rounding-floor
refusal. Every ordinary/interval polynomial dispatch is charged to the original
caller resource cap. Algebraic inverse evaluations are separately reported,
because transcendental work is not comparable to one polynomial evaluation.

The increment uses -h phi(mhat), rather than 2(mhat-y), to avoid subtracting
endpoint-scale quantities to produce a small increment. Verification still
uses y+Khat/2 in the original polynomial, so this numerical representation is
fully covered by H2.

## Theorem H3: scalar implementation preserves the generic calculation

At lambda=1, the sealed dimension-one map is immutable and has a known bracket.
Replace each length-one state/mapped/residual Vec by a scalar, preserve the
same evaluation order, arithmetic expression tree, seed, actual-residual screen
and resource cap, and retain the exact normalized Anderson scalar operations.
Then every successful current/previous/selected state and charged callback
count agrees with the generic implementation, by induction over updates.

The scalar specialization keeps 0*anchor+1*mapped and hypot(0,residual), so
signed-zero and norm arithmetic follow the original implementation. It also
preserves the old Screened/SlowOnly defaults solely as compatibility controls.
The production comparisons use Picard or eta=1 ResidualDominance, introducing
no new heuristic coefficients. The finite resource cap bounds every scalar
loop, and failures retain the accepted physical trajectory prefix.

## Divergent alternatives and interpretation

1. Generic homotopy: strongest compatibility control; incurs several tiny
   allocations per polynomial evaluation and retains full station output.
2. Scalar specialization: removes those container allocations and vector
   dispatch while retaining identical numeric work. It is the attribution
   control for deciding how much improvement comes from representation alone.
3. Algebraic predictor: replaces iteration by a polynomial-specific inverse,
   one ordinary RHS evaluation and the same outward interval verification.
   Cube-root cost can outweigh saved polynomial evaluations, so wall time and
   Callgrind instructions must decide its deployment alongside scalar Picard.
4. Rational predictor: correction pass1 replaces the expensive radical formula
   with H4 below. This is a separate value-only candidate, still certified by H2.

The analytical planner's conservative truncation bound remains an open
improvement target. This work strengthens stage production behavior without
claiming that a closed polynomial inverse solves general vector homotopy.

## Theorem H4: rational inverse and its exact defect

For beta>=0, define r(beta)=(1+2 beta)/(1+3 beta), and let s* be the unique
positive solution of s+beta s³=1. Then

    r+beta r³-1 = beta³(3+8 beta)/(1+3 beta)³ >=0,
    0 <= r-s* <= beta³(3+8 beta)/(1+3 beta)³.

Proof. Multiplication by (1+3 beta)³ leaves the polynomial beta³(3+8 beta),
an exact identity checked independently by Wolfram. Since the scalar residual
has derivative 1+3 beta s²>=1, monotonicity gives the root ordering and upper
error bound. No asymptotic approximation is used in this defect bound.

The inverse series is s*=1-beta+3 beta²+O(beta³). A [1/1] candidate
(1+A beta)/(1+B beta)=1+(A-B)beta+(B²-AB)beta²+O(beta³) matches it exactly
when A-B=-1 and B(B-A)=3. Hence B=3 and A=2 are uniquely determined.
They are exact approximation coefficients, not data-fitted tuning constants.

For midpoint mrat=(y/a)r, the theorem bounds the midpoint discrepancy by
|y|/a times this defect. The implemented candidate increment -h phi(mrat)
still passes the original increment residual, so no floating-point residual
bound is inferred from evaluating the rational defect in ordinary binary64.
The fallback is charged and preserves the original algebraic budget. For
beta=0, r=1, so linear and zero limits require no empirical switching threshold.

This candidate is selected after pass0 timing showed that an exact algebraic
inverse can cost more than the stack iterative baseline. The prior negative
results and pass0 source are retained; no holdout or preregistration claim is
made for this diagnosed correction.

## Primary literature context

W. Kahan, *To Solve a Real Cubic Equation*, 1986 lecture notes:
https://people.eecs.berkeley.edu/~wkahan/Math128/Cubic.pdf . The notes identify
rounding and intermediate scaling issues in general cubic formulas, and use
rigorously bounded polynomial residual evaluation to assess candidate roots.
Our restricted monotone cubic has a unique real root and admits H1; its
certificate is the existing directed residual, not a borrowed general-cubic
stability assertion. Exa search receipts preserve all five returned candidates;
the original-author notes were read, and other search hits were not adopted as
proof. H1 is an algebraic reformulation, without a novelty claim for Cardano.
