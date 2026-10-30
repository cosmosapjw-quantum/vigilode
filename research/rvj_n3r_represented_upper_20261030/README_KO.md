# RVJ N3R: represented-source outward algebraic envelope (2026-10-30)

**Scope:** narrow numerical defect repair from [N3 Draft #137](https://github.com/cosmosapjw-quantum/vigilode/pull/137), not a formal N4 node, independent-author C9, or production RODAS5P implementation. Parent exact HEAD `0380ff94c86031d898df478b67c6fcc66e49e3b9`. Existing PR137, default solver, tableau, runtime adapters and main are unchanged.

## 1. Physical source and derived correction

Dimensionless, real nonnegative damping, real `chi_alpha`, positive weights and sign basis `phi_1j=1, phi_2j=(-1)^j`:

```
a'_j = (-kappa_j+i omega xi_j) a_j + i sum_alpha chi_alpha phi_alpha,j |m_alpha|^2 m_alpha
m_alpha = sum_j w_j phi_alpha,j a_j
E = sum_j w_j |a_j|^2       E'=-2 sum_j w_j kappa_j |a_j|^2 <=0.
```

The prior use of nominal `w=1/n` hides a represented-source identity risk: for n=65 and n=257 the *exact real sum of binary64 weights* exceeds one. For this toy `|phi|=1`, put `S=max(1,sum_j w_j)`. Then energy gives `|m_alpha| <= e S` with `e=max_j(|Re(y_j)|+|Im(y_j)|)`, and the complex cubic has `3S` Lipschitz constant over the unit weighted-energy ball. Use

```
L_jk = 3*S*sum_alpha |chi_alpha*phi_alpha,j| * w_k*|phi_alpha,k|
Q_j = min(h, 1/kappa_j)       (Q_j=h for kappa_j=0)
q = max_j Q_j sum_k L_jk < 1
tau_p = (|omega| h)^(p+1)/(p+1)!
A_alpha = sum_j w_j |phi_alpha,j| (|Re(y_j)|+|Im(y_j)|)
R = max(e*S, A_alpha(1+tau_p)) <= 1
Q_ab = sum_j w_j |phi_a,j phi_b,j| Q_j
b_alpha = e^3 S^3 sum_beta |chi_beta| Q_ab
B_j = 3 R^2 Q_j sum_alpha |chi_alpha phi_alpha,j|(b_alpha+A_alpha*tau_p)
d_j_next <= B_j + d_j_old/(1+kappa_j*h) +
             Q_j * sum_k L_jk * max_k(d_k_old)/(1-q).
```

Source input, initial carrier, all inherited bounds and h are interpreted as **exact real values of the actual binary64 representations**, not nominal decimal/rational values. `q=21/400` is a **nominal mathematical simplification**; the represented model's exact rational q need not equal 21/400. This theorem assumes both exact trajectories remain in the source's weighted physical energy domain and a fixed source over the window. No generic arbitrary rank-r model claim.

## 2. True source-specific Rust implementation and independent exact oracle

This draft **contains actual source code** under `rust/` and `exact_represented_oracle.py`, unlike the report-only parent PR #137. The diff from frozen N3 is `N3R_REPRESENTED_UPPER.patch`, exact blob SHA `a85355a06db79046d9574bf4dc0bc98bed934e38`. Rust source `rust/src/lib.rs` exact Git blob `b82db3223bca2b8955b2b4c5f5f2ff751a9bb435` (SHA256 `b11f6c5b439e30dfb355ecfbce26e1bbb3c4ed5c792870479fd041a0df0015f2`). All published Rust/CSV/oracle blobs were fetched back and checked against their local Git blob hashes.

Negative and positive execution evidence:
- **Rust1.99.1** supplied installer (isolated prefix), fresh native **25/25 PASS**, locked/offline `cargo test`, strict `cargo clippy -- -D warnings`, rustfmt PASS.
- Independent Python `Fraction` reconstructs exact represented-source algebraic recurrence for n=12/24/65/257, two h=1/4 windows, 716 full mode-window rows. **Original 674/716 BELOW** exact represented-source analytic bound; **N3R 0/716 BELOW**.
- Native candidate `re,im` output parity N3 vs N3R is **716/716 bitstring-identical**. Clean frozen ZIP restore + `patch -p1` reproduces N3R lib.rs bytes, 25/25 native tests. Clean N3R full ZIP restore + `REPLAY.sh` reproduces the output CSV byte-for-byte and 716/716 exact oracle.
- Directed helpers include subnormal underflow and positive denominator rounding controls; native test checked RN and gradual-subnormal operations in the test host. `Source::energy_upper` fail-closes a represented weight-energy boundary case.
- Candidate-only 3-run median-of-medians cost (new/old): n=12 **1.01344x**, n=24 **1.04136x**, n=65 **1.03914x**, n=257 **1.19427x**. An unfavorable n257 overhead is retained; small n<=8 protected skip remains a choice policy, not a universal speed theorem.

**Crucial limitation:** this is a conservative enclosure of the *ideal exact-real algebraic error recurrence*, **not an enclosure of the actual floating-point Volterra output**. The Rust complex exponentials, nonlinear polynomial/moment recurrence, accepted-carrier representation, and original RODAS5P all-eight raw-U residual are NOT certified. Returned status stays `RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION`. No physical/user-facing tolerance has been relaxed.

## 3. Published binary evidence and review boundary

- [Google Drive exact executable ZIP](https://drive.google.com/file/d/1E4x-6bO5Re9tUGYT2lhT92uJRvUt0l3-/view), **120,211 bytes**, SHA256 `a74b46d726929ba990dc6a1075870a78d8a8b4f5da19c6bf89ca5158848f6af0`; 28/28 manifest payload hashes and ZIP CRC PASS.
- Drive file was actually downloaded back and compared with source ZIP by `cmp` and SHA256 **byte-identical**. ChatGPT Library: `/VIGILODE_RESEARCH/VIGILODE_RVJ_N3R_REPRESENTED_UPPER_20261030_FULL.zip` (upload ACK). Dropbox binary NOT_RUN.
- Rust 1.99.1 detached .asc exists; **signer authentication NOT_VERIFIED**, although the installed compiler version and actual native execution were verified. The supplied older offline snapshot's 27/27 core PASS is inherited evidence, **not** new current PR131 source validation.
- This is same-assistant adversarial work; formal actual distinct-author C9 remains **0/1 NOT_RUN**. No real independent review receipt.

**DAG:** official VigilODE `7/23`; central REI `28/88`; successor admitted `0/12`; RVJ follow-on N1+N2+N3 `3/8_SCOPED` (N3R is a subnode repair, **not 4/8**). C10 NOT_STARTED; PRODUCTION HOLD; MERGE_READY false; AUTO_NEXT_NODE false.

Next candidate only: N4 outward source-bound complex exponential/moment *and actual carrier roundoff* + opt-in original PR131 RODAS5P eight-stage residual/transaction, derivative-free RHS-only primary and optional JVP fallback, matched achieved physical error/speed. Do not auto-open N4, merge or change defaults.
