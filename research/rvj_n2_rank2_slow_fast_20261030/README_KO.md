# RVJ N2 rank-2 two-rate slow/fast causal Volterra source (2026-10-30)

## Exact binary publication record

This is an **isolated, stacked research-only draft** on [RVJ-N1 draft #135](https://github.com/cosmosapjw-quantum/vigilode/pull/135), no production/default code changes and **no full executable Python source embedded in this GitHub branch**. The source-complete executable N2 ZIP is independently stored and verified:

- [N2 full source ZIP on Google Drive](https://drive.google.com/file/d/1yBl30J0lN_RVaNhfVX6DQ_LEMGAGiIXm/view), 18,828 bytes, SHA-256 `5f7a77bfdd3b2e4ba031c2085d0f70be76b6a90872d975a85effb0ee5afb15db`, 15 entries and 14/14 payload hash checks.
- Google Drive provider ACK, separate raw-file download and local `cmp -s` **byte-identical PASS**.
- ChatGPT Library copy: `/VIGILODE_RESEARCH/VIGILODE_RVJ_N2_RANK2_SLOW_FAST_20261030_FULL.zip`. Independently uploaded N2 handoff Markdown.
- N1 parent complete archive SHA `605123cfebfcab73593edc9f100be622b5741c3c7fca435308131a5ea95875ae`.

## True slow/fast mathematical construction

The source-specific model is
```text
a_j' = (-kappa_j+i omega xi_j) a_j + i sum_alpha chi_alpha phi[alpha,j]|m_alpha|^2 m_alpha
m_alpha = sum_j w_j phi[alpha,j] a_j
w_j>0, sum w_j=1, chi_alpha and phi real, sum w_j phi[alpha,j]^2<=1.
```
The exact derivative `E'= -2 sum_j w_j kappa_j |a_j|^2<=0` is established **analytically** (independent n4 rank2 SymPy identity zero). With `z_j=-kappa_j+i omega xi_j`, kernel matrix `K[beta,alpha](t)=sum_j w_j phi[beta,j]phi[alpha,j]exp(z_j*t)`; no fast-mode initial data are dropped. Two actual damping clusters are `kappa_s=1/4` and `kappa_f=1000` (stiffness-rate ratio 4000), `omega=1/2`, `T=1/2`; rank2 `phi1=1`, `phi2=(-1)^j`, `chi=(1/20,1/50)`. Explicit cluster cubic convolution uses all source exponents; frequency Taylor p=2 only, never expand fast damping.

For `Q_j=min(T,1/kappa_j)` or T if zero, define rational `Q_ba=sum_j w_j |phi_bj phi_aj|Q_j`, `e=max_j(|Re a0_j|+|Im a0_j|)`, `A_a=sum_jw_j|phi_aj|(|Re a0_j|+|Im a0_j|)`, `tau_p=(|omega|T)^(p+1)/(p+1)!`, `R=max(e,max_a A_a(1+tau_p))<=1`, and `b_a=e^3 sum_c |chi_c|Q_ac`. The exact-real all-mode **first-Picard** bound is

```text
|a_j(t)-a_hat_j(t)| <= 3R^2 Q_j sum_alpha |phi[alpha,j]chi_alpha| (b_alpha + A_alpha*tau_p)
for ALL 0<=t<=T, all j, under exact model, source identity and premises.
```

This exploits the true energy law and does not require a generic resolvent contraction. **It does not certify binary64/mpmath rounding or arbitrary OdeProblem.**

## Actual bounded tests and numerical output

- New N2 tests **10/10 PASS**, with negative controls on wrong source/coupling, original fast initial data, mismatched common damping, all-source energy, odd moment preservation, small-n protected skip.
- Independent SymPy exact energy cancellation **PASS**, 10 positive/negative-complex exponential-moment recurrence values versus 85-digit quadrature **PASS**, maximum relative difference approximately `1.197e-86`.
- For n=12 and n=24, source-dependent exact-rational bound `4.677590723845725e-9` at p=2, model requested tolerance `1e-8`; BDF numeric reference t=.001 maximum discrepancy `5.23348e-12` (n12), `5.23678e-12` (n24), max sampled all times n12 `5.23348e-12`, n24 `5.23678e-12`.
- The known initial test oracle demanded stronger than Taylor's actual bound and was corrected by using exact `A_a*tau_p`; a separate mpmath `gammainc` recursion failure for large negative complex argument was corrected by integer-order exponential integration-by-parts recurrence and quadrature-regressed. These are recorded as test/numerical failure types, not theoretical no-go.
- Actual slow amplitude n24 t=.5 about 0.01794, fast about 1.95e-11; actual two decay timescales, not just a common fast damping factor.
- **NO** measured native Rosenbrock wall advantage, full adaptive IVP, arbitrary-r guarantee, native outward complex interval, or genuine independent-author C9 result. No source-blind adapter or default route changes.

## DAG and next action

```text
RVJ_LOOP12_FOLLOWON=2/8_SCOPED (N1 N2 source-specific Python)
OFFICIAL_VIGILODE=7/23; CENTRAL_REI=28/88; SUCCESSOR=0/12
C9_REAL_DISTINCT_AUTHOR=0/1_NOT_RUN; C10=NOT_STARTED
PRODUCTION=HOLD; MERGE_READY=false; AUTO_NEXT_NODE=false
```

Next N3 adaptive source-epoch window chaining and inherited mode interval uncertainty; N4 opt-in actual VigilODE original RODAS5P source-lock and original residual after native proof; matched n3/8/12/24/65/257 physical error and work. The prior `research/rvj-full-history-20261004` branch contains **metadata index only**, original 4,550 full source-path payloads not remotely complete, and is NOT silently substituted for original archives.

Full mathematical derivation, reproducible code, 10 tests, 2 BDF cases, exact symbolic evidence, exact hashes and copyable next-thread handoff reside in the SHA-locked source ZIP. No main merge.
