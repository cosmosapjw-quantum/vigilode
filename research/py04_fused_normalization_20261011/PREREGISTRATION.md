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

## Results

Everything above this heading is the registered text, unchanged (last changed at `8ce9bda`). Executed on branch
`node/py04` from `28c067c` (AS05 and PY01 merged); the registered text names `audit/rvj-reaudit-remaining-20261011`
as the registration branch. Only the branch differs.

**Verdict: PASS** (evidence VALID). All 144 normalized certificates are Bounded and enclose the 50-digit error of
F_hat, and of the Taylor caller candidate in the 80 cases where Taylor accepted the original w. The six gate items
hold with zero violations. Claim boundary as registered: mixed absolute-relative budget only, no relative-only claim,
no PP11 rerun (the PP11 PASS and the PP12b FAIL stand unedited), opt-in entry point, `certify_fused_phi_total`
unchanged, no default promotion, no wall-time claim.

### Record

| Item | Commit | SHA-256 |
|---|---|---|
| Registration (text above) | `8ce9bda` | — |
| A: `directed.rs` helpers, `certify_fused_phi_total_normalized`, contract tests and exporter | `2dd3ba2` | `directed.rs` `f49d7ca9b90f8b976acf4c23bd41bffdfb35640ee34d1b3dbe5335ba938b3f34`; `taylor_phi_total.rs` `982a06a11857dcdf6833a4f64a3cf5dc655af612b8b2446b5614add2976b5f96`; `tests/fused_normalization_contract.rs` `cddbb0ce951c21f567e2fdec071ee1c3da1a0ee979731040bccc97e1eaca52e8` |
| B: checker and synthetic checker tests (before the run) | `b3e1d93` | `py04_fused_normalization_check.py` `1581bc5ebb619b24649a6704c297ab5286f7ae59f85a88c9ae86a6cf624583a1`; `test_py04_fused_normalization_check.py` `34c20686ab296c372cb7bc1a6d5f7b0304048f0e03fe39c777f7baba86f0f945` |
| C: recorded run at `b3e1d93` (clean tree) | `9ed58d9` | `cases.json` `f2e83b223d24379595243222c0cbf6f34dbbd6b0df372ba75ca655e4443f4db8` (1,103,711 bytes, compact JSON); `RESULTS.json` `c701467b3f19923da0405e0ad1687462a6686db57b81bd58a33fbc72d2b6e75c` (292,730 bytes) |
| Unchanged input | — | `tools/evidence_schema_v2.py` `1d2bfe9276252b2079914e4fa958612ed06ffb04d1fecf9016bd4f370206ff10` |

**How the run was made.** The three registered commands were run once each, in order, from a clean tree at
`b3e1d93`, with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`, rustc/cargo 1.94.1, Python 3.11, mpmath 1.3.0.
- Command 1 (debug): 7 passed, 1 ignored (`export_cases`), exit 0.
- Command 2 (release): `export_cases` passed and wrote `cases.json` (144 cases, 72 table rows). Its recorded
  `source_commit` is `b3e1d93c5b84a191a83a1b986e3afef6819a6248` and its `tree_status` (`git status --porcelain`,
  untracked files included) is empty.
- Command 3: exit 0. The checker recorded HEAD `b3e1d93…` and an empty `git status --porcelain --untracked-files=no`;
  its blob at the source commit equals the checker that ran.

**Tests.** At A: `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` clean;
`cargo test -p rodas5p-core --all-targets` passed (41 result groups, 0 failures, PP11's contract tests included) and
`cargo test -p rodas5p-core --doc` passed (7, including the two new `compile_fail` type-separation doctests and the
PP11 one). At B: `python3 -m unittest discover -s tools -p test_py04_fused_normalization_check.py` passed 32 tests,
normally and with `-O` (a PASS fixture; 14 INVALID tests: duplicate key, NaN, missing and duplicate cases, extra key,
non-canonical hex, an unbound input bit, a wrong family rule, a caller candidate not equal to the Taylor output, an
unbound table input, dirty export and checker trees, a checker-blob mismatch, 50/70-digit disagreement; 13 FAIL tests
covering each of the six items; arithmetic and closed-form reference tests). Before B, the checker's regeneration of
the 144 inputs and 72 table inputs was compared bit for bit with the Rust generator compiled standalone (inputs only;
no certificate was computed), and its matrix generator with PP11's committed `cases.json` (72/72).

**Coordination (directed.rs).** The PY04 additions are additive: `PowerOfTwo`, `POW2_STAGE_LIMIT`, `Pow2Scaling`,
`scale_pow2_staged`, `scale_pow2_nearest`, `scale_pow2_up`, `scale_pow2_down`, `scale_pow2_is_exact` (public) and
`POW2_EXPONENT_CLAMP`, `floor_log2`, `pow2_stage` (private). None of these names occurs anywhere in `crates/` at
`28c067c` (after the AS05 merge); `git diff 28c067c -- directed.rs` removes no line. The only existing item with a
similar name, `polynomial_action`'s private `fn scale_pow2`, is not glob-imported anywhere, so nothing is shadowed.
In `taylor_phi_total.rs` the only removed line is the `use crate::directed::{…}` list, replaced by a longer one.

### Validity

All conditions hold. AS03 rules pass (strict JSON, exact key sets, canonical hex, exactly the 144 registered labels
and 72 table rows). Every A, base w_k and input w (s x base, rounded once) and every table input equals the
checker's regeneration bit for bit; both Taylor budgets are 1e-13; every caller candidate is the exported Taylor
output on the original w. The worst 50/70-digit reference difference is 2.5e-51 relative (tolerance 1e-45 relative
plus 2^-1100); the S6 L references agree likewise. Provenance and both tree statuses are clean.

### Gate

| Item | Result | Numbers |
|---|---|---|
| 1 Enclosure | holds | 144/144 F_hat bounds and 80/80 caller bounds enclose error + reference difference; bound/error min 2.56 (S3), caller bound/error min 1.29 |
| 2 Homogeneity | holds | 16/16 triples: v, z, E, perturbation, stepped error, normalized distance bitwise identical; all 16 triples exact (R = L = 0, sigma E normal) and bound_s = s bound_1 exactly in all 16 |
| 3 Accounting | holds | L > 0 in exactly the 16 S6 cases (w_4 / sigma underflows), = 0 in all others; R > 0 in 37 cases (5 S1, 16 S2, 16 S3), every R^2 >= the exact squared rescale error (min ratio R/actual 1.71); no Bounded certificate has a null value |
| 4 Directed scaling | holds | 72/72 rows (68 finite: down <= exact <= up, nearest correctly rounded, exact flag right; 4 overflow rows at e = 1022/1023 refused); largest stage 2^1000 |
| 5 Usefulness (mixed budget) | holds | S1, S2, S3, S4, S6: 80/80 Bounded and useful; bound / budget max 0.048 (S1, S4), 0.085 (S2, S3), 0.003 (S6) |
| 6 Overflow boundary | holds | S5: 16/16 Bounded and valid, 0 refused (largest ||F||_2 = 2.40e308 > f64::MAX, every entry finite) |

Per family (bound for F_hat; e is sigma's exponent):

| Family | e | Bounded | relative bound min..max | bound/error min..max | dominant term |
|---|---|---|---|---|---|
| S1 (2^-1016) | -1017 | 16 | 2.3e-15 .. 4.8e-12 | 3.14 .. 1.8e4 | up(sigma E) 16 |
| S2 (2^-1050) | -1051 | 16 | 6.7e-8 .. 6.9e-7 | 2.79 .. 5.92 | R 16 |
| S3 (2^-1068) | -1069, -1068 | 16 | 0.018 .. 0.18 (at most 3.04 x 2^-1074) | 2.56 .. 8.26 | R 16 |
| S4 (2^1018) | 1017 | 16 | 2.3e-15 .. 4.8e-12 | 3.14 .. 1.8e4 | up(sigma E) 16 |
| S5 (2^1023) | 1022 | 16 | 2.3e-15 .. 4.8e-12 | 3.14 .. 1.8e4 | up(sigma E) 16 |
| S6 (mixed) | 999 | 16 | 2.7e-15 .. 3.0e-13 | 6.52 .. 577 | up(sigma E) 16 |
| S7 (1, 2^600, 2^-600) | -1, 599, -601 | 48 | 2.3e-15 .. 4.8e-12 | 3.14 .. 1.8e4 | up(sigma E) 48 |

### Reported, not gated

- **Relative-only usefulness** (bound <= 1e-10 ||F||): 16/16 in S1, S4, S5, S6 and each S7 member; 0/16 in S2 and
  S3, where the bound is a few subnormal quanta (S2 6.7e-8 to 6.9e-7 relative, S3 up to 0.18). This is where the
  registered relative-only gate is infeasible.
- **Taylor caller candidate** (Taylor on the original w): accepted in S1, S2, S3, S7 (s = 1 and 2^-600), 80 cases;
  rejected in S4, S5, S6 and S7 (2^600) (`||M||_1` above the Taylor limit). All 80 caller bounds enclose; mixed-useful
  80/80, relative-useful 48/80 (0/16 in S2 and S3).
- **Unnormalized baseline** (`certify_fused_phi_total`, PP11 candidate rule): Bounded and enclosing in S1, S2, S3
  (16 each) but never useful there (absolute floor ~1e-15 as in PP11); S4 8/16 Bounded and useful (the h = 0.25 cases,
  zero perturbation) and 8 Unbounded (7 perturbation overflow, 1 no stepped bound); S5 and S6 16/16 Unbounded (no
  stepped bound); S7: s = 1 16/16 Bounded and useful, s = 2^600 8/16 Bounded and useful, s = 2^-600 16/16 Bounded
  and not useful.
- **Dominant term**: up(sigma E) in S1, S4-S7 (128 cases), R in S2 and S3 (32). Inside E: the perturbation for
  h = 0.3 and the stepped error for h = 0.25 (64 each), and the stepped error in all 16 S6 cases.
- **L in S6** is 1.2e-24 x (growth/factorial sum) scale against ||F|| ~ 1e300: loose (L / actual ~ 4e299, since the
  bound uses half a quantum for an entry that underflowed far below it) and negligible (S6 bound / budget <= 0.003).
- **Normalization work** (counted, not timed): per case 31-38 scaled entries and 31-83 stages at n = 5, 73-87 entries
  and 73-188 stages at n = 12 (5n inputs, n candidate entries, 1-15 bound scalings); every stage factor is at most
  2^1000 in magnitude.
- **Candidate source**: Taylor accepted the normalized data in all 144 cases (no stepped-top fallback).

### Prediction check

- Items 1-4 and 6 hold: as predicted.
- Item 5 (S1-S4, S6 useful): as predicted. R dominates in S2 and S3 rather than sigma E (predicted ~1e-15 relative);
  S3, the predicted main risk, has bounds of at most 3.04 quanta against a budget of 35.8-55.4 quanta.
- S5 mostly Bounded: all 16 Bounded, no refusal (no rescaled entry reached 2^1024; the refusal path is exercised by
  the contract test only).
- Baseline Unbounded in S4/S5: **partly wrong**. S5 16/16 Unbounded, but S4 has 8 Bounded and useful baseline
  certificates (power-of-two h, where the perturbation term is exactly 0 and no e^omega product is formed).
  Baseline not useful in S1-S3: as predicted.
- Overall PASS: as predicted.

### Interpretations and deviations

Fixed before the recorded run in the checker docstring (items 1-10): one seed per matrix 41000 + index, base w_k drawn
after the matrix, table seed 41016; Taylor budget 1e-13 for both Taylor calls (PP11's; the registration names none);
R and L count only the inexact entries; item 1 gates the normalized certificates (the baseline is reported); items 1
and 3 apply to Bounded certificates; norms in mpmath at 50 digits plus the 50/70 difference; L's enclosure checked
against the mpmath reference of sigma F(A, h, w/sigma - v); the `compile_fail` doctests run under
`cargo test -p rodas5p-core --doc`, not under the registered `--test` command; non-finite values exported as null; an
Unbounded normalized-level certificate gives an Unbounded result, not a refusal.

After the run (reported field only, nothing re-run): the reported consistency flag `v_is_nearest` is false in 5 cases
(3 S3, 2 S6). In each, one input entry is -0.0 (a negative base entry that underflowed); the exporter's v is -0.0, and
the checker's fractions comparison yields +0.0, which differs only in the sign of zero. The values are equal. The gated
exactness test (item 3) compares values, not bits, so no gate is affected. There is no `*_FIRST` file.
