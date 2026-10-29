# Preregistration: Common Damping Entire-Moment Kernel (LP03)

Node LP03 of `research/promotion_loop12_20261011/PORTING_DAG.json` (pr_id `PR-LP03`). Registered on branch `pr/lp03-damped-moments` (base `6ed07d5`). It depends on `LP02` (merged and verified in `L-0119`).

**Claim boundary.**
- Outward entire moments for the damped Volterra convolution kernel $K(t) = \exp(-\kappa t) \sum_j \frac{(i \cdot \text{width} \cdot t)^j}{j!} \mu_j$ on admitted collective models with $\kappa \ge 0$.
- Normalized damped moment $J_{j, k}(z) = \int_0^1 e^{-z(1-u)} (1-u)^j u^k du$ ($z = \kappa h \ge 0$) and unscaled physical moment $I_{j, k}(h, \kappa) = h^{j+k+1} J_{j, k}(\kappa h)$ in binary64 interval-enclosed (`Interval`) arithmetic.
- When $\kappa \to 0$ ($z = 0$), continuously reduces to un-damped beta weights $B(j+1, k+1) = \frac{j! k!}{(j+k+1)!}$.
- For small $z \le 1.0$, computes via Taylor series with certified outward truncation remainder $|R_N| \le \frac{z^N}{N!} B(j+N+1, k+1)$.
- For large $z > 1.0$, computes via backward recurrence on incomplete exponential moments $E_m(z) = \int_0^1 e^{-zv} v^m dv$ ($E_{m-1} = \frac{z E_m + e^{-z}}{m}$) from a provably bounded high-order ceiling $E_{M_{\text{top}}} \in [0, \frac{1}{M_{\text{top}}+1}]$, followed by binomial recombination, completely avoiding subtractive cancellation.
- Refuses negative $\kappa < 0$, non-positive horizon $h \le 0$, non-finite parameters, orders $j, k > 64$, or out-of-bounds queries.

## Question

Can the common damping entire-moment kernel be implemented as a zero-allocation, typed Rust module in `crates/rodas5p-core` that:
1. Precomputes and caches normalized moment enclosures $J_{j, k}(\kappa h)$ and unscaled physical moments $I_{j, k}(h, \kappa) = h^{j+k+1} J_{j, k}(\kappa h)$ in $O(p^2)$ operations;
2. Provides certified outward `Interval` bounds with rigorous containment $0 \le \text{lo} \le \text{hi} \le B(j+1, k+1)$ across all $\kappa \ge 0, h > 0$;
3. Continuously reduces to exact beta weights when $\kappa = 0$;
4. Demonstrates strict monotonic damping reduction as $\kappa$ increases;
5. Validates continuity across the algorithmic switch boundary at $z = \kappa h = 1.0$ to $< 10^{-5}$ without cancellation jumps;
6. Strictly refuses invalid parameters with typed errors?

## Specification

### 1. Module `crates/rodas5p-core/src/damped_moments.rs`
- Struct `DampedMomentsConfig`:
  - `kappa: f64`
  - `horizon: f64`
  - `max_j: usize`
  - `max_k: usize`
- Struct `DampedMomentTable`:
  - `compile(config: DampedMomentsConfig) -> CoreResult<Self>`
  - `get_normalized(&self, j: usize, k: usize) -> CoreResult<Interval>`
  - `get_unscaled(&self, j: usize, k: usize) -> CoreResult<Interval>`
- Standalone functions:
  - `compute_damped_moment(kappa: f64, horizon: f64, j: usize, k: usize) -> CoreResult<Interval>`
  - `compute_normalized_damped_moment(kappa: f64, horizon: f64, j: usize, k: usize) -> CoreResult<Interval>`

### 2. Contract Test `crates/rodas5p-core/tests/damped_moment_contracts.rs`
- `VG-1`: Zero allocations during compiled table lookups.
- `VG-2`: Exact beta limit when $\kappa = 0$, monotonic damping reduction with increasing $\kappa$.
- `VG-3`: Outward interval bounds enclosure $0 \le \text{lo} \le \text{hi} \le B(j+1, k+1)$, continuity across $z=1.0$, unscaled scaling identity $I_{j, k} = h^{j+k+1} J_{j, k}$.
- Negative admission gates: negative $\kappa$, non-positive $h$, non-finite values, order $> 64$, out-of-bounds indexing.

## Commands

```bash
cargo test --offline --locked -p rodas5p-core --test damped_moment_contracts -- --test-threads=1
python3 tools/lp03_stage0_check.py --output research/lp03_damped_moments_20261011/STAGE0.json
```

## Results

Executed 2026-10-18 under commit `deb1eff80f43f71b08f3aed9d04dccb34c8db317` (ledger `L-0120`).

### 1. Stage 0 Feasibility Outcome
- **VG-1 (Zero-allocation):** Compiled table lookups for normalized moments and unscaled physical moments allocate 0 heap bytes. `met = true`.
- **VG-2 (Mathematical limits):** When $\kappa = 0$, normalized moments reduce to exact beta weights $B(j+1, k+1)$ within machine precision; monotonic damping reduction strictly confirmed across varying $\kappa$. `met = true`.
- **VG-3 (Interval enclosure & branch continuity):** Outward intervals rigorously satisfy $0 \le \text{lo} \le \text{hi} \le B(j+1, k+1)$ across all damping levels; continuity jump across $z = 1.0$ is $< 10^{-5}$; unscaled scaling identity $I_{j, k} = h^{j+k+1} J_{j, k}$ matches to relative error $< 10^{-12}$. `met = true`.
- **Negative admission gate:** Strictly refuses negative $\kappa$, non-positive horizon, non-finite parameters, orders $> 64$, and out-of-bounds indexing. `met = true`.
- **Decision:** `FEASIBLE`.
- **Verdict:** `PASS`.

