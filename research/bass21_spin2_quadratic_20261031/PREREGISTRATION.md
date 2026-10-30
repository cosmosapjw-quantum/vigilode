# Prospective validation addendum, registered BEFORE the new alignment experiment

Registration boundary: this file is published on the GitHub research branch before computing the NEW alignment diagnostic and before writing its output. The earlier 231600-value quadratic-work data published in commit `85fb646883400c428258db5003e890a77773d405` **were NOT prospectively registered**. Their actual post-hoc exploratory status remains unchanged. This document does not retroactively preregister those prior measurements. The project's research-node gate was found to fail and requires a documented, genuinely subsequent experiment and an append-only numeric ledger receipt.

## Source locks and permissions
- Parent original PR148 HEAD `2e29f98379d3a7d56cf3cbcda0c71355fbc0117c`, full archived source SHA256 `626ea52c9fae38db23e0dd9e5f628ed0cba35ee05d54391b02b91ca34a0b3b1c`.
- Nested PR145 corrected source SHA256 `c7626d6f7e01e578e4e9825e60ea8e1631e6ec79cc535867d95fc690b0e0c564`.
- Original owner `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`; exactly 50 original cases, 193 saved epochs; frozen LRS screen E_l0(+) coefficients l=2,4,...,24. Cutoffs **L=16,18,20,22**. No new IVP, no changed owner/source/time/opacity/Einstein/thermal/electron fields.

## New predeclared scientific questions (not previously evaluated)
For the source-owned truncated sky `Q_L(mu,x)=sum_even_l<=L [E_l(x)/k_l] P_l^2(mu)` on `0<=mu<=1` with `k_l²=4pi*(l+2)!/((2l+1)*(l-2)!)`, evaluate its normalized quadrupolar **polarization angular-shape alignment**:
- `I0_L = int_0^1 Q_L(mu,x)^2 dmu`, `I2_L=int_0^1 mu^2 Q_L(mu,x)^2 dmu`.
- Where `I0_L>0`, `eta_L=I2_L/I0_L` and `alignment_L=1-3eta_L`. Where `I0_L==0`, mark value undefined (not zero) and preserve an explicit validity mask. `0<=eta_L<=1`, `-2<=alignment_L<=1` follow directly from nonnegative `Q_L^2`.
- Calculate `alignment_L` at all 50×193×4 source positions and the s04/s00 source-matched 25 pairs, plus 193-time averages, but do not claim weak-shear s00 source signals resolved beyond the original GL64/96 quadrature limit.
- Pointwise check the independent identity `W_int=5 b'(x) P_L alignment_L` on valid rows, where previous `P_L=I0_L/2`. For invalid `I0_L=0`, identity is checked as zero work and no quotient is formed. This is a non-evolving source-functional consistency check, not a new transport equation.

## Independent implementation checks, stopping rules, evidence
- Primary implementation: 64-point Gauss-Legendre on positive half-sky with SciPy associated Legendre basis.
- Independent construction: 96-point Gauss-Legendre and `(1-mu²)d²/dmu² P_l` polynomial basis. Compare `I0,I2`, their normalized quotient and the source matrix shear-work identity. Required absolute agreement `<1e-10` for resolved source magnitudes. This is solely algebra/angular accuracy, not a temporal/IVP criterion.
- Check masks and exact bounds above (up to 1e-12 floating algebra slack), nonnegative integrals, x=0 initial behavior; reject wrong input SHA; one negative source/constant mutation test.
- Produce full numerical NPZ, at least one genuine numeric CSV, source ownership and scope JSON, and source-based science figure; preserve exact output SHA256 and deterministic regeneration. Register the execution command, status and output SHA256 in **one append-only** `research/LEDGER.jsonl` row, with a true retrospective note for the earlier dataset. No backdating of registration.
- A nonzero work/shape alignment is not a universal no-go, an observational CMB E/B, a generic observer E/B or coupled native E_l trajectory. Official VigilODE DAG remains 7/23, successor admitted 0/12, production HOLD, no merge/force push/new owner IVPs/automatic successor.
