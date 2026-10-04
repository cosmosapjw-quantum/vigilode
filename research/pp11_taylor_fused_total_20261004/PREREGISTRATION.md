# PP11 — total-target certificate for the scaled-Taylor fused phi action (prospective registration)

RVJ DAG node `PP11`. Base commit: the commit that adds this file.

## Target and gap

Exact target `F = sum_{k=0}^{4} phi_k(h A) w_k` for the binary64 `A`,
`h`, `w_k` taken as exact reals; it is the top block of `exp(M) v` with
`M = [[hA, W], [0, J]]` (Al-Mohy-Higham 2011). `taylor_phi_action`
reports only an exact-arithmetic tail for the stored binary64 `M~`
(`hA` rounded once), so it is EstimateOnly. This node closes the gap
without reusing the exp-only certificate for the fused target.

## Certificate (new, opt-in)

`certify_fused_phi_total(A, h, w[0..5], candidate)`:
1. `M~` with `fl(h a_ij)` and the exact rest; `Delta = hA - fl(hA)`
   enclosed entrywise (`|Delta_ij| <= |mul_up - mul_down|`), `||Delta||_2`
   bounded by `sqrt(||Delta||_1 ||Delta||_inf)` upward.
2. A directed stepped certificate (REV-02 `certify_exp_action_stepped`
   with the automatic stepping and metric) of `exp(M~) v`, giving its own
   candidate `y` and bound `E1`.
3. Perturbation `||exp(M) v - exp(M~) v||_2 <= ||Delta||_2 e^{omega}
   ||v||_2`, `omega >= max(mu(M), mu(M~), 0)` with `mu` the symmetric-part
   Gershgorin upper bound (logarithmic norm), rounded up.
4. Bound for the caller's candidate `u` (e.g. `taylor_phi_action`'s fused
   output): `||u - F|| <= ||u - top(y)||_up + E1 + perturbation`.
The type is distinct from the exp certificate; a report built for
`exp(A) v` cannot be passed as a fused certificate.

## Fixtures (fixed now)

The R4 POLY-DEV-06 style inputs: symmetric and nonsymmetric `A`, n in
{4, 8, 16}, `||hA||_1` in {0.5, 4, 20}, `h` not a power of two (so `hA`
rounds), five `w_k` seeded; subnormal-scaled `w` (1e-310) and large
(1e100) variants; plus `h` a power of two (no rounding) as a control.

## Gate

G1 Every reported bound is at least the 50-digit error of `u` against `F`
   computed from the exact binary inputs (mpmath `expm` of the exact
   augmented matrix).
G2 The perturbation term is positive whenever `hA` rounds and zero for
   the power-of-two control; type separation is enforced by a compile-time
   distinct type and a contract test.
G3 Reported: bound / error ratios, which term dominates, and the count of
   cases where the certificate is within 1e-10 of `||F||` (usefulness),
   without a gate on usefulness.

## Results (append only after the recorded run)

### Recorded run (source commit `764d3c2bbf97b16b4ad67977e3aa17e093de59b7`)

Before the run, at that commit: `cargo fmt --all -- --check`, `cargo clippy -p rodas5p-core --all-targets --locked -- -D warnings`
and `cargo test --release -p rodas5p-core --locked` all exited 0 (rustc 1.94.1). Contract tests
`pp11_taylor_fused_total_contracts` 7/7, plus the module's `compile_fail` doctest (an `&NonnormalExpCertificate` passed
where `&FusedPhiCertificate` is required does not compile) and its positive twin. No exporter or checker run on the
fixtures preceded the commit.

Commands (with `CARGO_TARGET_DIR` set to a scratch directory), both exit 0:

1. `PP11_CASES=research/pp11_taylor_fused_total_20261004/cases.json cargo test --release -p rodas5p-core --locked --test pp11_taylor_fused_total_study -- --ignored --nocapture --test-threads=1`
2. `python3 tools/pp11_taylor_fused_check.py --cases research/pp11_taylor_fused_total_20261004/cases.json --output research/pp11_taylor_fused_total_20261004/RESULTS.json`
   (mpmath 1.3.0, 50 digits)

Outputs: `cases.json` (sha256 `095cc115289edbec5149711dffc30158be0f6de0dba28cb6d16e80a7da3f581f`), `RESULTS.json`
(sha256 `915dd4065a34f4eae5d0a36255209c23213772be697e33c56e0cffcec6aa07cb`).

**Verdict: PASS** (G1 and G2 hold).

| Gate | Outcome |
|---|---|
| G1 enclosure | **holds**: in all 54 `Bounded` certificates the 50-digit error of the candidate is below the bound. The 18 `Unbounded` certificates report `+inf` and pass trivially |
| G2 perturbation | **holds**: `hA` rounds in all 54 `h = 0.1` cases and the perturbation is > 0 in each (finite in 36, `+inf` in 18); `hA` is exact in all 18 `h = 0.125` controls and the perturbation is exactly 0 there. Type separation: contract test `an_exp_certificate_is_a_different_type_from_a_fused_certificate` and the `compile_fail` doctest |
| G3 (reported) | see below |

Fixtures: 72 cases = {symmetric, nonsymmetric} x n in {4, 8, 16} x `||hA||_1` in {0.5, 4, 20} x four variants: seeded
`w_k` with `h = 0.1`; the same `w_k` times 1e-310 (subnormal) and times 1e100, `h = 0.1`; seeded `w_k` with
`h = 0.125` (control).

G3 by variant (ratio = bound / 50-digit error; useful = bound <= 1e-10 `||F||`):

| Variant | Taylor accepted | Bounded | Useful | Bound / error | Relative bound | Dominant term |
|---|---|---|---|---|---|---|
| seeded, `h = 0.1` | 18/18 | 18 | 18 | 8.7 to 4.9e4 | 3.4e-15 to 1.0e-11 | perturbation 16, stepped `E1` 2 |
| power-of-two control | 18/18 | 18 | 18 | 3.5 to 2.2e4 | 1.4e-15 to 4.5e-12 | stepped `E1` 18 (perturbation 0) |
| subnormal `w` (1e-310) | 18/18 | 18 | 0 | 4.7e306 to 6.2e308 | 9.8e293 to 5.2e296 | perturbation 18 |
| large `w` (1e100) | 0/18 | 0 | 0 | - | - | perturbation (`+inf`) 18 |

Totals: 54 bounded, 18 unbounded, 36 useful. The 50-digit reference agrees with a 70-digit recomputation to 1.7e-51
relative (worst case). Not gated, also checked on the full augmented vector: `E1` encloses `||y - exp(M~) v||` and the
perturbation term encloses `||exp(M) v - exp(M~) v||` in every case where they are finite.

What the run shows:

- **Seeded and control cases are certified usefully.** `taylor_phi_action`'s fused output is within 8e-17 to 5e-16
  (relative) of `F`, and the total bound is 4.6e-15 to 4.9e-12 absolute. Where `hA` rounds, the perturbation term
  dominates in 16 of 18 cases: it is 74 to 1.1e5 times the actual `||exp(M) v - exp(M~) v||` (`omega` 2.2 to 5.9 from
  the Gershgorin bound of the augmented symmetric part, which includes the `W` block). The power-of-two control
  isolates `E1`: its `E1` is within 0.94 to 1.07 of the seeded case's, and its total bound is 1.5 to 7.0 times smaller
  (the control's `A` is rescaled for `h = 0.125`, so the pairs are not identical operators).
- **Subnormal `w`: valid but useless.** `||F||` is about 1e-311 to 3e-310, while the bound stays at 2.6e-16 to 9.6e-15
  absolute: the `e_4` entry of `v` and the shift block `J` are O(1) whatever the scale of `w`, so `E1`, `||v||_2 >= 1`
  and `omega >= 1` do not scale with `F`. The Taylor output itself loses subnormal precision (relative error 1.2e-13 to
  1.7e-12).
- **Large `w`: no bound.** `taylor_phi_action` rejects every case (`TAYLOR_DOMAIN_UNSUPPORTED`, `||M||_1` about 2e100 to
  1e101 above 64); the alternative candidate is the stepped certificate's own top block, whose actual relative error is
  5e-17 to 3e-16. The stepped certificate itself is finite only under the Osborne metric (the identity needs more than
  100,000 steps): `E1` is 1.2e-15 to 8.2e-3 relative to `||F||`. The total is `Unbounded` because the perturbation
  term overflows: `omega >= mu_2(M)` includes the 1e100 entries of `W`, so `e^omega` is not representable. The
  logarithmic 2-norm is not invariant under the diagonal similarity that rescales `W`, so the preregistered
  perturbation bound cannot follow `w` to large scales. Nothing was changed after seeing this.

Implementation choices the preregistration left open (fixed in the committed code before the run):

- The automatic stepped certificate is the rule of `certify_exp_action_auto` (degree 20, `stepping_rule`, identity and
  Osborne metrics, the smaller `error_upper`, identity on ties) re-run per metric inside the new module, so that an
  error under one metric (an unusable balancing or an overflow) removes only that metric's bound instead of aborting
  the certificate. `certify_exp_action_auto` itself was not called and `nonnormal_certificate.rs` was not modified.
- One interval Gershgorin computation, with the top-left block enclosed by `[mul_down(h, a_ij), mul_up(h, a_ij)]`,
  gives `omega` for `M` and `M~` at once; when the enclosure of `Delta` is 0 the perturbation is set to exactly 0
  without evaluating `e^omega` (then `M = M~`).
- Fixtures: off-diagonal entries of a SplitMix64-seeded base matrix uniform in [-1, 1) (mirrored when symmetric), the
  diagonal `-(r_i + c_i)/2 - 1`, then scaled by `||hA||_1 / (h ||A_0||_1)` separately for `h = 0.1` and `h = 0.125`;
  `w_k` seeded uniform in [-1, 1); the `taylor_phi_action` budget is 1e-13 (absolute). Where Taylor rejected, the
  candidate is the stepped top block from a first certificate call (the call is deterministic).
- Reference: `mpmath.expm` of the exact augmented matrix with all `w_k` divided by a power of two near their largest
  magnitude and the top block multiplied back (exact by linearity in the `w_k`), so the 1e100 and 1e-310 variants stay
  well scaled for `expm`; recomputed at 70 digits as a check.

No deviation from the registered gates. Claim ceiling: dense `A` with n <= 16, `||hA||_1 <= 20`, the fixed stepping
rule; no timing.
