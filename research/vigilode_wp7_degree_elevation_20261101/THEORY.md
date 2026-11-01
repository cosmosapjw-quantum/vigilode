# WP7: exact Bernstein degree elevation as a nested residual envelope

Original source: cooperative dissipative cubic with k_i>0, g_i>=0, c_ij>=0 and frozen WP2 cubic Hermite reference. WP6 defect r_i is degree <=9 per panel, with original dyadic source, clock and trace bytes unchanged.

Let b^(m) be the degree-m Bernstein coefficients of exactly the SAME polynomial r(u), and R_m(u)=sum_{k=0}^m |b_k^(m)|*B_(k,m)(u). Elevate from m to m+1 by
 b_0'=b_0, b_(m+1)'=b_m,
 b_k'=[k/(m+1)]*b_(k-1)+[1-k/(m+1)]*b_k.
By Bernstein degree elevation and the convexity of absolute value,
 |r(u)| <= R_(m+1)(u) <= R_m(u) for ALL u in [0,1].
Also max |b_k'| <= max |b_k|, so the WP6 Poisson-tail bound does not increase.
Both properties hold for real coefficient vectors, without original solver changes.
The exact nonnegative beta moment is C(m,l)*l!*(m-l+j+k)!/(m+j+k+1)!; integrating the elevated envelope against the same positive Metzler semigroup preserves the componentwise ordering. No sampled residual is used as proof.

Source-frozen numerical evidence: 4 WP2 original cohorts x micro32/micro64 x 96 panels = 768 exact-rational panels. Both degree 9 and 12 use identical depth1, M8, N5 and cubic-positive mode; 768/768 nonincrease, 764/768 strict, tests 8/8. The eight actual numeric source-bound end states live in RESULTS.json. Higher degree has additional evaluation cost and usually <1% gain; cost-adjusted OpenEvolve/UCT 8+8 waves (20 candidates) retain degree9 as best default. Source is immutable WP6 PR#167, original input blob d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482. No new native solver speed or new owner IVP.

Adversarial finding: strict improvement for ALL polynomials is false. Counterexample b_k=1 for every k has R_m(u)=1 for all m and u. Positivity argument depends on exact convex Bernstein basis and original Metzler assumption, not general matrices. This is a derived theorem and exact Python Fraction checks, NOT a new HOL Light kernel theorem.

Primary literature comparison: Corless & Corliss (1991), Rationale for Guaranteed ODE Defect Control; Akrivis, Makridakis & Nochetto (2009), doi:10.1007/s00211-009-0254-2; Steinebach (2023) RODAS5P, doi:10.1007/s10543-023-00967-x. Broad novelty is unresolved.

HOLD invariants: official VigilODE DAG=7/23, successor admitted=0/12, production HOLD, no merge, no owner source/default/tolerance mutation, distinct-author C9 NOT_RUN. This file is a source-visible scoped report, NOT the complete raw binary reference archive.
