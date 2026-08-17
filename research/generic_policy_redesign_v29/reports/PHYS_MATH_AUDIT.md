# PHYS–MATH AUDIT — v2.9 Independent Stage-Growth Safety Calibration

## Verdict

**NEGATIVE CALIBRATION / ALL-ABSTAIN.** The predeclared scalar witness

\[
A_{34}=\frac{\rho_4/\rho_3}{(c_4/c_3)^2}
      =\frac{\rho_4/\rho_3}{3.24}
\]

does not admit a nontrivial monotone safety threshold under the predeclared calibration gates.
The sealed N=384 holdout was not opened because no nontrivial policy existed to freeze.

## Data

Fresh calibration was independent of the v2.8 N=128/512, rtol=1e-5 discovery rows:

- N=96, atol=3e-8, rtol=3e-6;
- N=256, atol=3e-7, rtol=3e-5;
- six fixed generic families per profile;
- 48 budget-admitted events total;
- 43 audit-E admissible and 5 audit-E inadmissible events;
- zero runtime full-E continuations and zero budget breaches.

## Load-bearing counterexample

The smallest unsafe witness is a HIRES N=256 event with

\[
A_{34}=0.8620858274<1,
\qquad q_E=1.1537358>1.
\]

Thus even the intuitive rule “sub-quadratic normalized stage growth is safe” is false on the fresh calibration corpus.
Three of the five unsafe events have A34<1.

Safe and unsafe ranges overlap:

\[
A_{34}^{\rm safe}\in[0.68275,1.55302],
\qquad
A_{34}^{\rm unsafe}\in[0.86209,1.36830].
\]

The v2.8 strict descriptive gap therefore does not generalize as a scalar ordering.

## Threshold geometry

Among the complete finite threshold scan:

- at most 8 calibration events can be recommended with zero observed unsafe events;
- the first unsafe recommendation enters at tau=0.8620858274 with the 9th recommendation;
- the predeclared minimum recommendation count was 12;
- the multiplicity-aware zero-failure upper-bound gate also cannot rescue a nontrivial threshold.

The frozen selector therefore correctly returns `all-abstain`.

## Interpretation

The failure does **not** falsify the mathematical small-h statement

\[
\rho_i=Cc_i^2|h|^3+O(h^4)
\]

under its regularity/nondegeneracy assumptions. It falsifies using the single finite-h ratio A34 as a generic safety authority over the current regime family.

The missing information is plausibly the *shape* of the defect curve across all retained stages, not merely its final slope. A natural next discovery feature is the log-stage curvature

\[
\kappa_{234}
=
\frac{\log(\rho_4/\rho_3)}{\log(c_4/c_3)}
-
\frac{\log(\rho_3/\rho_2)}{\log(c_3/c_2)}.
\]

If \(\rho(c)=Kc^p(1+O(h))\) locally, both secant slopes equal p+O(h), hence \(\kappa_{234}=O(h)\). This gives a scale-free measure of departure from a single local power-law remainder growth. It is a research feature, not a safety theorem.

## Claim ceiling

Allowed:
- independent calibration falsified a monotone scalar A34 safety rule;
- staged level-2 probing remains numerically meaningful instrumentation;
- pathwise JVP reserve was respected in all 48 calibration events.

Not allowed:
- A34 is a generic safety certificate;
- the N=384 holdout passed (it was not run);
- active switching is ready;
- a new curvature witness is safe before independent calibration/holdout.
