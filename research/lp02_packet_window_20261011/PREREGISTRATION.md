# Preregistration: kappa=0 Finite-Window Packet and Native Total Certificate (LP02)

Node LP02 of `research/promotion_loop12_20261011/PORTING_DAG.json` (pr_id `PR-LP02`). Registered on branch `pr/lp02-packet-window` (base `36d6097`). It depends on `LP01` (merged and verified in `L-0118`).

**Claim boundary.**
- Finite-window packet for $\kappa = 0$ on admitted `CollectiveModel` instances.
- Moment prefix compilation, polynomial waveform Picard iteration, and exact causal factorial convolution in binary64 interval-enclosed / outward arithmetic.
- Independent candidate generation and original-model defect certification enclosing truncation tail, rounding, and mode reconstruction.
- Checked resource/cost ledger (`PacketWorkCounters`): online Jacobian, JVP, VJP, Krylov, and linear solve counters are strictly zero; moment terms, polynomial multiply terms, Volterra convolution terms, nonlinear evaluations, and Horner output terms are fully tracked.
- Precomputed/recursive causal beta weights $B(j+1, k+1) = \frac{j! k!}{(j+k+1)!}$ adopting LC1 complete output cost and LC2 cache-space contracts.
- Refuses when $h L \ge 1$ (simple rational Gronwall bound unsupported) or when candidate leaves unit complex-mean domain or physical bound exceeds budget.

## Question

Can the $\kappa = 0$ finite-window packet and native total certificate be implemented as a zero-allocation, typed Rust module that:
1. Compiles the moment prefix $\mu_k = \sum_j w_j \xi_j^k$, $\nu_k = \sum_j w_j \xi_j^k z_j$, and basis polynomials $b_k = \frac{(i \cdot \text{width} \cdot h)^k}{k!}$ in $O(p N)$ operations;
2. Computes the causal Volterra convolution $\int_0^t K(t-s) F(s) ds$ using precomputed recursive causal beta weights $B(j+1, k+1)$;
3. Generates the Picard polynomial candidate $m(t)$ in sweeps with exact initial mean enforcement $m(0) = \nu_0$;
4. Computes the native total certificate:
   - Volterra defect $D = \|R\|_{[0, 1]}$,
   - Kernel tail $\tau = \frac{(\text{width} \cdot h)^{p+1}}{(p+1)!}$,
   - Mean bound $e_{\text{mean}} = \frac{D + \tau (A_0 + h G / (p+2))}{1 - h L}$,
   - Microscopic mode bounds $e_{\text{mode}, j} = \tau (|z_j| + h G / (p+2)) + h L e_{\text{mean}}$;
5. Reconstructs all $N$ independent microscopic mode waveforms $a_j(t) = \beta(\xi_j) z_j + \gamma(\xi_j)$ in $O(p N)$ operations with Horner evaluation;
6. Strictly maintains zero online Jacobian/JVP/Krylov counters while recording all non-zero packet work terms?

## Specification

### 1. Module `crates/rodas5p-integrators/src/packet_work.rs`
- Struct `PacketWorkCounters`:
  - `moment_terms: u64`
  - `poly_multiply_terms: u64`
  - `volterra_terms: u64`
  - `nonlinear_evaluations: u64`
  - `output_horner_terms: u64`
  - Standard work getters, reset, and accumulation methods.

### 2. Module `crates/rodas5p-integrators/src/packet_window.rs`
- Struct `CausalWeights`:
  - Precomputes and caches $B(j+1, k+1) = \frac{j! k!}{(j+k+1)!}$ via $row[0] = \frac{1}{j+1}$, $row[k] = row[k-1] \cdot \frac{k}{j+k+1}$.
- Struct `CompiledPacket`:
  - `model: CollectiveModel`
  - `horizon: f64`
  - `order: usize`
  - `mu: Vec<f64>`
  - `nu: Vec<(f64, f64)>`
  - `free: Vec<(f64, f64)>`
  - `kernel: Vec<(f64, f64)>`
- Struct `PacketCandidate`:
  - `mean: Vec<(f64, f64)>` (polynomial coefficients in normalized time $\tau = t/h \in [0, 1]$)
  - `degree: usize`
  - `sweeps: usize`
- Struct `PacketCertificate`:
  - `mean_bound: f64`
  - `mode_bounds: Vec<f64>`
  - `volterra_defect: f64`
  - `kernel_model: f64`
  - `tail: f64`
  - `lipschitz_l: f64`
- Functions:
  - `compile_packet(model: &CollectiveModel, horizon: f64, order: usize, work: &mut PacketWorkCounters) -> Result<CompiledPacket, PacketError>`
  - `generate_candidate(packet: &CompiledPacket, degree: usize, sweeps: usize, weights: &CausalWeights, work: &mut PacketWorkCounters) -> Result<PacketCandidate, PacketError>`
  - `compute_certificate(candidate: &PacketCandidate, packet: &CompiledPacket, weights: &CausalWeights, work: &mut PacketWorkCounters) -> Result<PacketCertificate, PacketError>`
  - `reconstruct_modes(candidate: &PacketCandidate, packet: &CompiledPacket, work: &mut PacketWorkCounters) -> Vec<(f64, f64)>`
  - `evaluate_mode_at(candidate: &PacketCandidate, packet: &CompiledPacket, mode_idx: usize, tau: f64, work: &mut PacketWorkCounters) -> (f64, f64)`

### 3. Contract Test `crates/rodas5p-integrators/tests/packet_window_contracts.rs`
- `VG-1`: Zero online solver counters; hot loop allocations audit.
- `VG-2`: Convergence of Picard sweeps and agreement with exact Dahlquist linear solution when $\chi = 0$.
- `VG-3`: Certificate enclosure: verified that numerical error $|m_{\text{cand}}(t) - m_{\text{true}}(t)| \le e_{\text{mean}}$ and $|a_{j, \text{cand}}(t) - a_{j, \text{true}}(t)| \le e_{\text{mode}, j}$.
- Refusal contracts: $h L \ge 1$, invalid degree, negative horizon, budget exceeded.

## Commands

```bash
cargo test --offline --locked -p rodas5p-integrators --test packet_window_contracts -- --test-threads=1
python3 tools/lp02_stage0_check.py --output research/lp02_packet_window_20261011/STAGE0.json
```

## Results

Appended after the recorded run. Nothing above this heading changes.
