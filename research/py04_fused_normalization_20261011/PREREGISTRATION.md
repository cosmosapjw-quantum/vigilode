# Preregistration: power-of-two normalization of the fused Taylor phi certificate (PY04)

Node PY04 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit
(`REVIEW_KO.md` §5, `MATHEMATICS_PORTING_KO.md` §6.5). Registered on branch `audit/rvj-reaudit-remaining-20261011`
(base `0ce7c32`). It depends on AS03, the fail-closed evidence validator (merged).

**Claim boundary.**
- **No relative-only gate.** A pure relative gate is infeasible below the binary64 range: an output of a few subnormal
  quanta has no relative accuracy to certify (PP12b, L-0076: exact solutions 6.2e-373 and 5.0e-414). This node
  therefore declares a **mixed absolute-relative budget** (below) and makes no relative-only claim.
- **No rerun.** The PP11 campaign (L-0078, 72 cases) is not rerun. Its PASS, and its "subnormal: useless" and "large:
  Unbounded" findings, stand unedited.
- **Opt-in only.** The new entry point is opt-in. `certify_fused_phi_total` is unchanged. There is no wall-time claim
  (normalization work is counted only) and no default promotion.

## Question

The fused target `F(A, h, w) = sum_k phi_k(hA) w_k` is linear in w, so F(A, h, sigma w) = sigma F(A, h, w) for any
power of two sigma. PP11's certificate is not scale-homogeneous:
- The augmented `M = [[hA, W], [0, J]]` and `v = [w_0; e_4]` keep O(1) entries.
- Large w puts W into omega, so e^omega overflows (PP11: 1e100 cases all Unbounded).
- Subnormal w leaves an O(1e-15) absolute floor (PP11: 1e-310 cases valid but useless).

Does normalizing w by a common power of two, then rescaling candidate and bound outward, give valid and useful
certificates at fresh scales around the normal/subnormal and overflow boundaries? The rescaling must enclose, or show
exact, every rounding of the normalization.

## Change

1. **`crates/rodas5p-core/src/directed.rs`:**
   - `PowerOfTwo { exponent: i64 }`.
   - `scale_pow2_nearest/up/down(x, e)`, with `scale_pow2_is_exact(x, e)`. These are staged scalbn by factors of at
     most 2^1000 in magnitude. Every stage but the last is exact; the last rounds once, and the directed variants step
     outward when it is inexact.
   - sigma^-1 is the exponent's negation. 2^e is never formed as a binary64 when |e| > 1022, so an overflowing
     reciprocal never occurs.
2. **`crates/rodas5p-core/src/taylor_phi_total.rs`:**
   `certify_fused_phi_total_normalized(a, h, w, caller_candidate: Option<&[f64]>) ->
   CoreResult<NormalizedFusedPhiCertificate>`, a separate type with private fields and no conversion from the
   unnormalized certificate.
   1. **Normalize.** e is the binary exponent of the largest |w_k,i| over all five w_k: one sigma = 2^e for all five,
      which preserves the relationships among the vectors. Then v_k = scale_pow2_nearest(w_k, -e).
   2. **Certify on v.** `taylor_phi_action` on (A, h, v) gives z; if Taylor rejects, z is the stepped top block. Then
      `certify_fused_phi_total(A, h, v, z)` gives E.
   3. **Rescale.** The candidate is F_hat = fl(sigma z), staged.
   4. **Bound.** bound(F_hat) = up(sigma E) + R + L, where:
      - R = ||fl(sigma z) - sigma z|| is 0 when every entry is exact, and otherwise at most sqrt(count) 2^-1075,
        rounded up;
      - L = sigma sum_k e^{max(mu, 0)} / k! ||w_k / sigma - v_k||_2, rounded up, and exactly 0 when every v_k is exact.
        mu is an upper bound on the logarithmic 2-norm of hA, from the interval Gershgorin bound of the symmetric part.
   5. **Caller candidate.** For a caller candidate u, the bound is ||u - F_hat||_up + bound(F_hat).
   6. **Range.** A result or bound above `f64::MAX` is a typed refusal, `FUSED_PHI_RANGE_UNSUPPORTED`. It is never a
      Bounded certificate with a non-finite value.
   7. **Normalization work.** The number of entries scaled and of stages is counted and recorded.

## Cases (fresh scales; not a PP11 rerun)

**Matrices.** The PP11 generator (diagonal `-(r_i + c_i)/2 - 1`) with fresh SplitMix64 seeds starting at 41000:
- n in {5, 12} (PP11 used {4, 8, 16});
- symmetric and nonsymmetric;
- ||hA||_1 in {1, 8} (PP11 used {0.5, 4, 20});
- h = 0.3, so hA rounds; h = 0.25 as the power-of-two control.

**Inputs.** Base w_k are seeded uniform in [-1, 1). Every family multiplies the base w_k by an exact power of two s,
and the binary64 result defines the exact input:
- **S1 boundary straddle:** s = 2^-1016, so entries straddle 2^-1022.
- **S2 mid-subnormal:** s = 2^-1050, about 24 bits.
- **S3 deep subnormal:** s = 2^-1068, about 6 bits.
- **S4 near overflow:** s = 2^1018.
- **S5 overflow boundary:** s = 2^1023. F may exceed `f64::MAX`.
- **S6 mixed:** w_0 x 2^1000, w_1..w_3 x 1, w_4 x 2^-1070. Normalization makes w_4 / sigma underflow, so L > 0.
- **S7 invariance triple:** s in {1, 2^600, 2^-600}. Every entry stays normal, so all scalings are exact.

**Count.** 2 x 2 x 2 x 2 = 16 matrices, times 9 inputs (the 7 families, S7 counted three times), gives 144 cases.

**Candidates per case:** the normalized F_hat (gated), and Taylor on the original w as the caller candidate where
Taylor accepts it.

**Baseline.** The unnormalized `certify_fused_phi_total` runs on the same cases. It is reported only.

**Scaling table.** A directed-scaling table exercises e in {-1074, -1073, -1023, -1022, -1021, 0, 1021, 1022, 1023}
on seeded mantissas.

**Oracle.** mpmath 1.3.0 (`python3 -c 'import mpmath'` succeeds here):
- `mpmath.expm` of the exact augmented matrix at 50 digits, from the exact binary inputs, with w divided by a power
  of two before expm and multiplied back in mpf (exact by linearity; mpf has no exponent range limit);
- rechecked at 70 digits;
- the directed-scaling table checked exactly with Python `fractions.Fraction`.

The oracle is high-precision, not interval arithmetic. Its uncertainty is bounded by the 50/70-digit difference
(validity item).

## Commands

    cargo test --offline --locked -p rodas5p-core --test fused_normalization_contract
    PY04_CASES=research/py04_fused_normalization_20261011/cases.json cargo test --offline --locked --release -p rodas5p-core --test fused_normalization_contract -- --ignored --nocapture --test-threads=1 export_cases
    python3 tools/py04_fused_normalization_check.py --cases research/py04_fused_normalization_20261011/cases.json --output research/py04_fused_normalization_20261011/RESULTS.json

**Contract tests.** The first command is the DAG command. Its contract tests:
- staged scaling at the extreme exponents, including e = -1074 with no 2^1074 ever formed;
- type separation, as a compile_fail doctest;
- exact homogeneity on one small case;
- the typed range refusal.

**Checker.** The checker calls `tools/evidence_schema_v2.py` first (INVALID exit 2, FAIL exit 1, PASS exit 0). The
checker is committed before the recorded run, and the export runs once.

## Gate

**Mixed budget (declared).** A bound is *useful* if bound <= 1e-10 ||F||_2 + 16 sqrt(n) 2^-1074.
- The relative part 1e-10 is PP11's usefulness level.
- The absolute part is a design choice. It allows a few subnormal quanta per entry: a binary64 vector can in general
  be no closer than sqrt(n) 2^-1075 to F.

**Validity.** INVALID if any of the following holds:
- The AS03 rules fail, or the case set is not exactly the 144 registered cases.
- An input is not bound to its family rule; the checker regenerates s x base and compares bits.
- The 50- and 70-digit references differ by more than 1e-45 ||F||_2 + 2^-1100.
- The checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold.
1. **Enclosure.** For every Bounded certificate and both candidates, bound >= ||candidate - F||_2 + |ref50 - ref70|.
   There must be zero violations.
2. **Homogeneity.** In each S7 triple:
   - the normalized internals (v, z, E, perturbation, stepped error and normalized-level distance) are bitwise
     identical;
   - bound_s = s x bound_1 exactly, wherever the checker finds every rescaling in the triple exact (R = L = 0 and
     sigma E normal). A triple with an inexact rescaling is reported and gated on its internals only.
3. **Normalization accounting.** The checker recomputes exactness from the bits.
   - L = 0 if and only if every w_k / sigma is exact, and R = 0 if and only if every entry of sigma z is exact. Every
     nonzero L and R is > 0 and enclosed.
   - No Bounded certificate has a non-finite candidate or bound.
4. **Directed scaling.** At every table entry, down <= exact <= up, nearest equals the correctly rounded value, and the
   exact flag is correct.
5. **Usefulness at the new extremes.** In S1, S2, S3, S4 and S6, every case is Bounded and useful under the mixed
   budget for F_hat.
6. **Overflow boundary.** Each S5 case is either Bounded and valid, or refused with `FUSED_PHI_RANGE_UNSUPPORTED`.

Everything else is **FAIL**, with every number preserved.

**Reported, not gated.**
- The relative-only usefulness (bound <= 1e-10 ||F||) per family, showing where it is infeasible.
- The Taylor caller-candidate usefulness.
- The unnormalized baseline's status and usefulness on the same cases.
- The dominant term, and the normalization work.

## Kill and hold

- **No relative-only gate,** in this node or in any rescoring: a pure relative gate below the binary64 range is
  infeasible.
- **No relaxation.** A FAIL is not re-scored by relaxing the absolute part after the run.
- **Records unchanged.** The PP11 PASS and the PP12b FAIL are not edited.

## Coordination with sibling nodes

AS05 and PY04 both add helpers to `crates/rodas5p-core/src/directed.rs`. The helpers are additive only (no existing
function changes behaviour) and are merged serially: AS05 first, then PY04. Before PY04's merge a name-collision check
confirms that no PY04 helper name duplicates or shadows an existing or AS05-added item in `directed.rs`.

## Prior information (disclosed)

- **PP11** (L-0078):
  - seeded and control cases are useful (relative 1.4e-15 to 1.0e-11);
  - subnormal w (1e-310) is valid, but the absolute floor is about 1e-15;
  - large w (1e100) is Unbounded: Taylor rejected every case, and e^omega overflowed;
  - Taylor outputs lost subnormal precision (relative 1.2e-13 to 1.7e-12).
- **PP12b** (L-0076): the binary64 floor of relative criteria.
- **Nothing run.** No code of this node exists, and no case of these families has been run.

## Predictions

- Items 1-4 and 6 hold.
- **Item 5:**
  - S1-S4 and S6 are useful: sigma E is about 1e-15 relative, R <= sqrt(n) 2^-1075, and L is below the absolute part
    in S6.
  - The least certain is S3. There, up(sigma E) and R are each about one quantum.
- **S5:** mostly Bounded. A refusal is expected only where ||F|| or the bound exceeds `f64::MAX`.
- **Baseline:** the unnormalized baseline is Unbounded in S4/S5 and not useful in S1-S3.
- **Overall:** probably PASS, with S3 the main risk.
