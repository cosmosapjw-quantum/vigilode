# Homotopy Expensive-Client Contract (HT01 / TP02 / TP08)

This contract defines the preconditions, target formulation, parallel scope, work/span accounting,
and screening criteria under which windowed homotopy parallelism may be considered for native ODE clients
in VigilODE.

References:
- `research/ht01_homotopy_client_contract_20261011/PREREGISTRATION.md`
- `docs/reviews/20261010_accuracy_speed/MATHEMATICS_PORTING_KO.md`, section 7
- `docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`, section 5

---

## 1. Frozen Target and Endpoint Certificate

1. **Original $\lambda = 1$ Stage Target:**
   - The stage equation at $\lambda = 1$ is:
     $$(\mathcal{D} - \mathcal{C}) K - b_0 - h N(K) = 0$$
   - Here $\mathcal{D}$ denotes the block-diagonal linear stage operator ($I - h\gamma J$),
     $\mathcal{C}$ denotes the strictly lower-block stage coupling, and $N(K)$ is the nonlinear stage defect.
   - Any accepted step output MUST be certified against this original $\lambda = 1$ target. Intermediate
     continuation points ($\lambda < 1$), predictor step sizes, or truncated partial-inverse residuals
     are speculative candidate-generation devices and NEVER an acceptance proof.

2. **Interval Tube and Enclosure:**
   - Define the parameterized family of stage equations:
     $$H_{\theta,\lambda}(K) = (\mathcal{D} - \eta \mathcal{C}) K - b_0 - \lambda h N(K) = 0$$
     where $\eta \in [0, 1]$ parameterizes stage coupling and $\lambda \in [0, 1]$ parameterizes nonlinear defect.
     Accepted outputs MUST be certified against the full endpoint $(\eta, \lambda) = (1, 1)$.
   - An invariant tube $\mathcal{T}$ and contracting ratio:
     $$q_\lambda \ge \|\mathcal{D}^{-1}\| \left( \eta \|\mathcal{C}\| + |\lambda h| L_N \right) < 1$$
     must be certified using directed outward interval arithmetic (`rodas5p_core::directed`), where
     $L_N$ is an outward upper bound on the Lipschitz constant of $N(K)$ rigorously enclosed over $\mathcal{T}$.
   - For candidate $\widehat{K}$ with preconditioned defect $\delta_\lambda = \|\mathcal{D}^{-1} H_{\theta,\lambda}(\widehat{K})\|$,
     the candidate-centered ball of radius $r$ satisfying $\delta_\lambda + q_\lambda r \le r$ must lie entirely within $\mathcal{T}$.

3. **Admission Mode Restrictions:**
   - Only `Q2Admission::NativeTargetCertificate` or `Q2Admission::PreparedStructuredCertificate` with
     `ModelBinding::GeneratedFromModel` can admit an accepted output.
   - `Q2Admission::OperationalDiagnostic` is an operational heuristic and NEVER admits a certified output.
   - `ModelBinding::SampledAgreement` (checking callbacks at finite sample points) is a consistency sanity check,
     never a mathematical proof (per R4-HOM-DEV-03).

---

## 2. Parallel Scope and Dependency Structure

1. **Independent Tasks Within One Round:**
   - Within a fixed round with frozen stage states $K$, the evaluations of $N(K)_i$ and linear solves with the
     same factored operator $\mathcal{D}$ across independent stage vectors are independent tasks that can run concurrently.
   - Jacobi-style sweeps with fixed previous-round state:
     $$\Phi_{\theta,\lambda}(K) = \mathcal{D}^{-1} \left( b_0 + \eta \mathcal{C} K + \lambda h N(K) \right)$$
     allow parallel stage-wise RHS and solve execution for the current iteration.

2. **Strict Sequential Dependencies Across Rounds:**
   - Continuation steps in $\lambda$, Newton/Picard iteration rounds, and recurrence levels $\ell \to \ell+1$
     in truncated partial inverses:
     $$(\mathcal{D} - \eta \mathcal{C})^{-1} = \sum_{\ell=0}^{s-1} (\eta \mathcal{D}^{-1} \mathcal{C})^\ell \mathcal{D}^{-1}$$
     are strictly sequential. Level $\ell+1$ must wait for level $\ell$.
   - Parallel workers NEVER remove sequential depth. Claiming that parallel lanes eliminate the fundamental
     staged depth of Runge-Kutta / Rosenbrock methods is strictly prohibited.

---

## 3. Work and Span Accounting (`HomotopyWorkLedger`)

Parallel budget models must report both total work $\mathcal{W}_{\text{total}}$ and critical-path span $T_{\text{par,model}}$
against the strongest sequential comparator at matched output accuracy.

1. **Total Work Equation:**
   $$\mathcal{W}_{\text{total}} = \mathcal{W}_{\text{setup}} + \mathcal{W}_{\text{candidate}} + \mathcal{W}_{\text{certificate}} + \mathcal{W}_{\text{rejected}} + \mathcal{W}_{\text{fallback}} + \mathcal{W}_{\text{dispatch}}$$
   Every unit of speculative, rejected, certificate, and fallback work must be fully charged.

2. **Model Span Equation for $P$ Workers:**
   $$T_{\text{par,model}} = T_{\text{setup}} + \sum_{\text{round}} \max_{\text{worker}} T_{\text{assigned tasks}} + T_{\text{sync}} + T_{\text{certificate}} + T_{\text{fallback}}$$
   Due to scheduling granularity, memory bus saturation, and synchronization barriers, $T_{\text{par,model}}$ cannot
   be approximated by $\mathcal{W}_{\text{total}} / P$.

3. **Peak Memory Accounting:**
   - Peak retained heap bytes across all worker threads and staging buffers must be measured and reported.

4. **HT02 Ledger Obligations:**
   - Fields currently omitted from `HomotopyWorkLedger` (`max_task_per_round`, `barriers`, `rejected_candidate_work`,
     `fallback_work`, `peak_bytes`) are mandatory implementation obligations for HT02 before any parallel speedup claim.

---

## 4. Cost Declaration: `ExpensiveClientDeclaration`

To prevent manufactured toy workloads from claiming parallel speedups, any candidate client evaluated for
homotopy parallelism MUST provide an explicit cost declaration:

```rust
pub struct ExpensiveClientDeclaration {
    /// Unique identifier of the problem client.
    pub client_id: &'static str,
    /// Exact function pointer / identity of RHS callback.
    pub rhs_fn_name: &'static str,
    /// Exact function pointer / identity of JVP callback.
    pub jvp_fn_name: &'static str,
    /// Declared instruction count Ir per RHS evaluation.
    pub declared_rhs_ir: u64,
    /// Declared instruction count Ir per JVP evaluation.
    pub declared_jvp_ir: u64,
    /// Provenance / measurement evidence for the declared Ir values.
    pub provenance: &'static str,
    /// Whether an optimized batched RHS evaluation callback (BatchRhsFn) is provided.
    pub provides_batch_rhs: bool,
}
```

A declaration is a binding empirical claim that the follow-on node (HT02) must independently verify under valgrind.

---

## 5. Screening Criteria (S1 – S4)

A candidate client must satisfy all four criteria to be eligible for parallel homotopy piloting:

1. **S1: Declared Expensive RHS / JVP:**
   - **(a) Cost Declaration:** The client must statically carry an `ExpensiveClientDeclaration`.
   - **(b) Measured Callback Share $f$:** From committed callgrind profiles (`research/*/PROFILE*.json`),
     the instruction share of `problem-user-code` relative to total trajectory instructions:
     $$f = \frac{\text{Ir}(\text{problem-user-code})}{\text{Ir}(\text{total})}$$
     must satisfy:
     $$f \ge f_{\min} = 0.348$$
   - *Derivation of $f_{\min}$:* Under Amdahl-type bounds with $r = 5$ (RHS evaluation multiplier of certified
     transactional arms, L-0008) and $P = 8$ (maximum HOM-06 thread count), achieving the required $1.15\times$ speedup
     (`PAIRED_TIMING_REQUIRED_SPEEDUP`) without overhead requires:
     $$\text{Span Ratio} = 1 - f \left(1 - \frac{r}{P}\right) \le \frac{1}{1.15} \implies f \ge \frac{1 - 1/1.15}{1 - 5/8} \approx 0.348$$
   - If no profile is committed, $f$ is recorded as `unmeasured` and S1 rests on (a) alone.

2. **S2: Shifted Solves Present:**
   - The sequential solver's counters must show $W = I - h\gamma J$ solves per attempt $> 0$, with $\ge 2$ independent
     stage-solve tasks within a round.
   - Clients with zero shifted solves (e.g. Fourier client K1, which uses pure Picard recurrence with $q1\_solves = 0$,
     preserving PP07 ABSTAIN, L-0074) fail S2.

3. **S3: Strong Sequential Comparator:**
   - A valid sequential baseline must exist: the cheapest native arm by committed same-binary $I_r$ per trajectory
     among applicable native arms (`small`, `dense_v2`, `colext64`, `banded`, `matrix_free_u_form`), with committed accuracy
     evidence and ledger row.
   - A comparison against an unoptimized JF-GMRES baseline where a direct sparse/banded/dense LU arm applies fails S3.
   - Closed list of applicable arms per candidate:
     - K1: `[]` (`no_applicable_arm`)
     - K2: `["small", "dense_v2", "matrix_free_u_form"]`
     - K3: `["small", "dense_v2"]`
     - K4: `["small", "dense_v2"]`
     - K5: `["small", "dense_v2", "matrix_free_u_form"]`
     - K6: `["dense_v2", "colext64", "banded", "matrix_free_u_form"]`
     - K7: `["dense_v2", "colext64", "banded", "matrix_free_u_form"]`
     - K8: `["small", "dense_v2"]`

4. **S4: Work/Span Accounting and Endpoint Certificate:**
   - **(a) Certificate Source:** `Q2CertificateSource::capability` must report `WitnessCapability::Available` for the
     client's dimension and structure, bound with `ModelBinding::GeneratedFromModel`.
   - **(b) Work Ledger Completeness:** `HomotopyWorkLedger` must support all $W_{\text{total}}$ and $T_{\text{par,model}}$
     terms (missing fields flagged as HT02 obligations).

---

## 6. Decision Rule

1. **`SELECTED(k)`**: At least one candidate meets all criteria S1, S2, S3, and S4 with measured $f \ge 0.348$.
   If multiple candidates qualify, the candidate with the largest minimum measured $f$ is selected.
   Candidates without measured $f$ cannot be SELECTED; they remain leads only.
2. **`ABSTAIN`**: No candidate meets all criteria S1 – S4 with measured $f \ge 0.348$.
3. **Leads:** Any candidate with $f \ge 0.348$ but lacking an `ExpensiveClientDeclaration` is recorded as a lead for
   future registration, but cannot be selected in this node.

---

## 7. Stated Limits of Existing Modules

1. **`crates/rodas5p-integrators/src/chart_transport.rs`:**
   - The Darboux chart transport module is a standalone scalar chart transformation ($y = x^2 (w + 1/\kappa)$).
   - It is called by no integrator in the solver pipeline and performs zero linear solves ($q1\_solves = 0$).
2. **`crates/rodas5p-core/src/shared_shift_jet.rs`:**
   - The shared shift jet represents a single dissipative Jacobian jet. It executes no shifted solves in parallel.
   - Any parallel speedup claim based on shared shift jet without multi-stage parallel tasks is mathematically invalid.
