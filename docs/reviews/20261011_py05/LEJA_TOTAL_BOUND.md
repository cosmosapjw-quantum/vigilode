# Mathematical Derivation and Proof of Total Error Bounds for Newton-Leja Phi Actions (PY05)

**Node ID:** PY05 (`research/py05_leja_total_certificate_20261011`)  
**Preregistration:** `research/py05_leja_total_certificate_20261011/PREREGISTRATION.md`  
**Review Status:** Stage 0 Proof Specification for Total Error Certificate  
**Scope:** Dense `SymmetricNonpositiveOperator` with Gershgorin-verified enclosure $\text{Spec}(A) \subseteq [-\rho, -\lambda] \subseteq (-\infty, 0]$. Target is the fused linear combination:
$$F = \sum_{k=0}^4 s_k \phi_k(hA) w_k$$
where $\phi_0(z) = e^z$ and $\phi_k(z) = \frac{\phi_{k-1}(z) - 1/(k-1)!}{z}$.

---

## 1. Domain, Geometry, and Coordinates

Let $A \in \mathbb{R}^{n \times n}$ be symmetric with spectrum $\text{Spec}(A) \subseteq [-\rho, -\lambda] \subseteq (-\infty, 0]$, verified by Gershgorin circle enclosures.
For step size $h \ge 0$, the spectrum of $hA$ is contained in the real interval:
$$[a, b] = [-h\rho, -h\lambda] \subseteq (-\infty, 0]$$
computed outward in floating-point arithmetic with $a = -\text{mul\_up}(h, \rho)$ and $b = -\text{mul\_down}(h, \lambda)$.
The interval is mapped to standard reference coordinates $\xi \in [-2, 2]$ via the affine transformation:
$$x(\xi) = c + \gamma \xi, \quad \xi(x) = \frac{x - c}{\gamma}$$
where the center $c$ and scaling $\gamma$ are:
$$c = \frac{a + b}{2} \le 0, \quad \gamma = \frac{b - a}{4} \ge 0$$
When $\gamma = 0$ ($a = b$, e.g. scalar matrix or $h=0$), the spectrum reduces to a single point and interpolation is exact with 1 node ($m=0$). For $\gamma > 0$, the compact interval $[a, b]$ has length $4\gamma$ and capacity $\gamma$. Under this scaling, the reference interval $[-2, 2]$ has logarithmic capacity 1.

The sequence of discrete real Leja points $\{\xi_j\}_{j=0}^m \subset [-2, 2]$ starts at $\xi_0 = 2, \xi_1 = -2$, and for each $j \ge 1$ greedily maximizes the product:
$$\xi_j = \arg\max_{\xi \in [-2, 2]} \prod_{i=0}^{j-1} |\xi - \xi_i|$$
The physical interpolation nodes on $[a, b]$ are $x_j = c + \gamma \xi_j \in [a, b]$.

The interpolating polynomial of degree $m$ for $\phi_k(x)$ in Newton form with respect to the scaled nodes is:
$$p_{k,m}(x) = \sum_{j=0}^m d_{j,k} \prod_{i=0}^{j-1} \left(\frac{x - c}{\gamma} - \xi_i\right)$$
where $d_{j,k} = \gamma^j \phi_k[x_0, \dots, x_j]$ are the scaled divided differences.
For a vector $w$, $p_{k,m}(hA) w = \sum_{j=0}^m d_{j,k} r_j$, where the sequence of vector iterates $\{r_j\}_{j=0}^m$ satisfies the two-term recurrence:
$$r_0 = w, \quad r_{j+1} = \left(\frac{hA - cI}{\gamma} - \xi_j I\right) r_j$$

---

## 2. Section T1: Truncation Error (Forward Enclosure)

### Theorem T1 (Forward Truncation Bound)
Let $A = A^T \in \mathbb{R}^{n \times n}$ with $\text{Spec}(hA) \subseteq [a, b] \subseteq (-\infty, 0]$. Let $p_{k,m}$ be the degree-$m$ polynomial interpolating $\phi_k$ at the nodes $x_0, \dots, x_m \in [a, b]$. Then for any $k \in \{0, 1, 2, 3, 4\}$ and any $m \ge 0$:
$$\|(\phi_k - p_{k,m})(hA)\|_2 \le \max_{x \in [a,b]} |\phi_k(x) - p_{k,m}(x)| \le \frac{\gamma^{m+1}}{(m + 1 + k)!} \max_{\xi \in [-2,2]} \prod_{j=0}^m |\xi - \xi_j|$$

### Proof
1. **Hermite-Genocchi Remainder Representation:**  
   By the standard Cauchy/Hermite interpolation error remainder theorem, for any function $f \in C^{m+1}([a,b])$ and any $x \in [a,b]$, there exists $\theta(x) \in [a, b]$ such that:
   $$f(x) - p_m(x) = \frac{f^{(m+1)}(\theta(x))}{(m+1)!} \prod_{j=0}^m (x - x_j)$$
   Applying this to $f(x) = \phi_k(x)$:
   $$\phi_k(x) - p_{k,m}(x) = \frac{\phi_k^{(m+1)}(\theta(x))}{(m+1)!} \prod_{j=0}^m (x - x_j)$$

2. **Derivative Bounds of $\phi_k$ on the Negative Real Line:**  
   Recall the integral representation of the entire functions $\phi_k(z)$ for $k \ge 1$:
   $$\phi_k(z) = \frac{1}{(k-1)!} \int_0^1 e^{t z} (1 - t)^{k-1} dt$$
   (For $k = 0$, $\phi_0(z) = e^z$).
   Differentiating $m+1$ times with respect to $z$ yields:
   $$\phi_k^{(m+1)}(z) = \frac{1}{(k-1)!} \int_0^1 t^{m+1} e^{t z} (1 - t)^{k-1} dt$$
   For any $x \le 0$ and any $t \in [0, 1]$, the exponential factor satisfies $0 < e^{t x} \le 1$.
   Therefore, for all $x \le 0$:
   $$|\phi_k^{(m+1)}(x)| \le \frac{1}{(k-1)!} \int_0^1 t^{m+1} (1 - t)^{k-1} dt = \frac{1}{(k-1)!} B(m + 2, k) = \frac{(m + 1)!}{(m + 1 + k)!}$$
   For $k = 0$, $|\phi_0^{(m+1)}(x)| = |e^x| \le 1 = \frac{(m+1)!}{(m+1)!}$, which satisfies the exact same bound with $k = 0$.
   Hence, for all $k \ge 0$ and all $\theta \le 0$:
   $$\frac{|\phi_k^{(m+1)}(\theta)|}{(m+1)!} \le \frac{1}{(m + 1 + k)!}$$

3. **Node Polynomial Scaling:**  
   Using the coordinate transformation $x - x_j = (c + \gamma \xi) - (c + \gamma \xi_j) = \gamma (\xi - \xi_j)$:
   $$\prod_{j=0}^m |x - x_j| = \gamma^{m+1} \prod_{j=0}^m |\xi - \xi_j|$$

4. **Spectral Mapping:**  
   Because $A$ is real symmetric, $hA$ is normal with real eigenvalues contained in $[a, b]$. By the spectral mapping theorem for normal operators:
   $$\|(\phi_k - p_{k,m})(hA)\|_2 = \max_{\lambda_i \in \text{Spec}(hA)} |\phi_k(\lambda_i) - p_{k,m}(\lambda_i)| \le \max_{x \in [a, b]} |\phi_k(x) - p_{k,m}(x)|$$
   Combining the inequalities:
   $$\|(\phi_k - p_{k,m})(hA)\|_2 \le \frac{\gamma^{m+1}}{(m + 1 + k)!} \max_{\xi \in [-2,2]} \prod_{j=0}^m |\xi - \xi_j|$$
   which completes the proof. $\blacksquare$

### Outward Rigorous Interval Computation of the Node Polynomial Maximum
Let $\Pi_{m+1}(\xi) = \prod_{j=0}^m |\xi - \xi_j|$. To compute a guaranteed upper bound:
$$M_{m+1} \ge \max_{\xi \in [-2, 2]} \Pi_{m+1}(\xi)$$
we partition the interval $[-2, 2]$ into $K$ uniform subintervals $J_p = [\xi_p^{lo}, \xi_p^{hi}]$ for $p = 0, \dots, K-1$ ($K = 128$).
On each subinterval $J_p$, we evaluate $\prod_{j=0}^m (J_p - \xi_j)$ using outward directed interval arithmetic (`directed::Interval`).
By inclusion monotonicity of interval arithmetic, for all $\xi \in J_p$, $\prod_{j=0}^m (\xi - \xi_j) \in \prod_{j=0}^m (J_p - \xi_j)$.
Taking the interval magnitude $\text{mag}(I) = \max(|I.lo|, |I.hi|)$ and finding the maximum over all $p \in \{0, \dots, K-1\}$ with outward rounding (`f64::max`) yields a mathematically certified upper bound $M_{m+1}$.
For the fused target $F = \sum_{k=0}^4 s_k \phi_k(hA) w_k$, the total truncation error bound is:
$$E_{T1} = \gamma^{m+1} M_{m+1} \sum_{k=0}^4 \frac{|s_k| \|w_k\|_2}{(m + 1 + k)!}$$
computed entirely via upward directed rounding (`mul_up`, `div_up`, `add_up`).

---

## 3. Section T2: Divided Differences (Opitz Shifted Bidiagonal Enclosure)

### Mathematical Basis
The scaled divided differences $d_{j,k} = \gamma^j \phi_k[x_0, \dots, x_j]$ are the matrix elements of a lower bidiagonal matrix exponential (Opitz, 1964).
Let $Z \in \mathbb{R}^{(m+5) \times (m+5)}$ have nodes $(0, 0, 0, 0, x_0, x_1, \dots, x_m)$ on the diagonal and subdiagonal:
$$\beta = (1, 1, 1, 1, \gamma, \gamma, \dots, \gamma)$$
Then $\exp(Z)_{4+j, 4-k} = d_{j,k}$ for $j \in \{0, \dots, m\}$ and $k \in \{0, \dots, 4\}$.

### Shift and Nonnegativity
Let $s = c - 2\gamma \le 0$ (the interval's left boundary $a = c - 2\gamma$).
Decompose $Z = sI + L$, where $L = Z - sI$.
The matrix $L$ has diagonal entries:
- For $i < 4$: $0 - s = -s \ge 0$
- For $i = 4 + j$: $x_j - s = (c + \gamma \xi_j) - (c - 2\gamma) = \gamma(\xi_j + 2) \ge 0$, since $\xi_j \in [-2, 2]$.
The subdiagonal entries of $L$ are identically the entries of $\beta$, which are $1.0 > 0$ and $\gamma \ge 0$.
Thus, $L$ is a real lower bidiagonal matrix with **all entries strictly nonnegative**: $L \ge 0$.
Consequently, for all powers $p \ge 0$, $L^p \ge 0$.
By commutativity of $sI$ and $L$:
$$\exp(Z) = \exp(sI + L) = e^s \exp(L) = e^s \sum_{p=0}^\infty \frac{L^p}{p!}$$

### Taylor Expansion and Tail Enclosure
For each column vector $v_0 = e_{col}$ (where $col = 4 - k$):
$$v_p = \frac{1}{p} L v_{p-1} \ge 0$$
Let $S_N = \sum_{p=0}^N v_p$ be the partial sum.
Because all entries of $L$ are nonnegative, the 1-norm $\|L\|_1 = \max_j \sum_i L_{i,j}$ provides a strict contractive ratio for $p > \|L\|_1$:
$$\|v_p\|_1 \le \frac{\|L\|_1}{p} \|v_{p-1}\|_1 \le \left(\frac{\|L\|_1}{N+1}\right)^{p-N} \|v_N\|_1$$
For $N \ge \lceil \|L\|_1 \rceil$:
$$\sum_{p=N+1}^\infty v_p \le v_N \sum_{l=1}^\infty \left(\frac{\|L\|_1}{N+1}\right)^l = v_N \frac{\frac{\|L\|_1}{N+1}}{1 - \frac{\|L\|_1}{N+1}} = v_N \frac{\|L\|_1}{N + 1 - \|L\|_1}$$
where the vector inequality holds componentwise because $L \ge 0$ is lower triangular.

### Rigorous Interval Arithmetic Enclosure
We compute:
1. Shift interval: $s_{lo} = \text{sub\_down}(c, \text{mul\_up}(2.0, \gamma))$, $s_{hi} = \text{sub\_up}(c, \text{mul\_down}(2.0, \gamma))$, with diagonal and subdiagonal carried as outward intervals $L_{lo} \le L \le L_{hi}$.
2. Lower partial sum $S_N^{lo}$ using downward directed rounding (`add_down`, `mul_down`) with lower bidiagonal entries.
3. Upper partial sum $S_N^{hi}$ using upward directed rounding (`add_up`, `mul_up`) with upper bidiagonal entries, plus the infinity-norm geometric tail term:
   $$t_\infty = \max_{i \in [col, size)} v_{N, i}^{hi}, \quad \text{tail} = \text{mul\_up}\left(t_\infty, \frac{\|L\|_\infty}{N + 1 - \|L\|_\infty}\right)$$
   added componentwise across all components $i \in [col, size)$.
4. Outward scaling by $e^s$: using `directed::exp_interval` evaluated on $[s_{lo}, s_{hi}]$, with $e^s_{hi}$ strictly guarded against underflow ($e^s_{hi} \ge 0.0\text{.next\_up}()$).
The resulting interval $[d_{j,k}^{lo}, d_{j,k}^{hi}]$ is guaranteed to strictly contain the exact value $d_{j,k}$.
Letting the computed midpoint be $\hat{d}_{j,k} = 0.5 \times (d_{j,k}^{lo} + d_{j,k}^{hi})$, the radius is computed via directed distances:
$$\text{rad}(d_{j,k}) = \max(\text{sub\_up}(d_{j,k}^{hi}, \hat{d}_{j,k}), \text{sub\_up}(\hat{d}_{j,k}, d_{j,k}^{lo}))$$
guaranteeing that $|d_{\text{exact}} - \hat{d}_{j,k}| \le \text{rad}(d_{j,k})$ strictly holds even under floating-point rounding.
The divided difference error propagated to the polynomial action is bounded by:
$$E_{T2} = \sum_{k=0}^4 |s_k| \sum_{j=0}^m \text{rad}(d_{j,k}) \|r_j\|_2$$
evaluated outward with directed rounding.

---

## 4. Section T3: Recurrence Rounding Error (Tail Polynomial Maximum)

### Newton Recurrence and Local Residuals
In floating-point arithmetic, the vector recurrence evaluates:
$$r_0 = w, \quad r_{j+1} = \text{fl}\left( \left(\frac{hA - cI}{\gamma} - \xi_j I\right) r_j \right)$$
Define the local rounding perturbation vector $\delta_{j+1} \in \mathbb{R}^n$:
$$\delta_{j+1} = r_{j+1} - \left(\frac{hA - cI}{\gamma} - \xi_j I\right) r_j$$
Let $X = \frac{hA - cI}{\gamma}$. Because $A$ is symmetric and $\text{Spec}(hA) \subseteq [a, b]$, $X$ is symmetric with $\text{Spec}(X) \subseteq [-2, 2]$.
The perturbed recurrence satisfies:
$$r_{j+1} = (X - \xi_j I) r_j + \delta_{j+1}$$
Expanding $r_m$ in terms of the initial vector $w$ and the local perturbations $\delta_1, \dots, \delta_m$:
$$r_l = \left(\prod_{i=0}^{l-1} (X - \xi_i I)\right) w + \sum_{j=0}^{l-1} \left(\prod_{i=j+1}^{l-1} (X - \xi_i I)\right) \delta_{j+1}$$

### Perturbation Propagation to the Interpolant
Substituting the expansion of $r_l$ into the computed polynomial $p_k(X) w = \sum_{l=0}^m d_{l,k} r_l$:
$$p_k(X) w = \sum_{l=0}^m d_{l,k} \left(\prod_{i=0}^{l-1} (X - \xi_i I)\right) w + \sum_{j=0}^{m-1} \left(\sum_{l=j+1}^m d_{l,k} \prod_{i=j+1}^{l-1} (X - \xi_i I)\right) \delta_{j+1}$$
The first term is the exact Newton polynomial applied to $w$.
The second term represents the exact propagation of each local rounding error $\delta_{j+1}$ to the final result through the remaining Newton tail polynomial:
$$Q_{j+1, k, m}(\xi) = \sum_{l=j+1}^m d_{l,k} \prod_{i=j+1}^{l-1} (\xi - \xi_i)$$

### Spectral Bound on Tail Polynomials
Because $X$ is symmetric with $\text{Spec}(X) \subseteq [-2, 2]$:
$$\|Q_{j+1, k, m}(X)\|_2 \le \max_{\xi \in [-2, 2]} |Q_{j+1, k, m}(\xi)|$$
Let $T_{j+1, k} \ge \max_{\xi \in [-2, 2]} |Q_{j+1, k, m}(\xi)|$ be an outward bound on this scalar polynomial maximum over $[-2, 2]$, computed by outward interval subdivision of $[-2, 2]$.
By the Cauchy-Schwarz and submultiplicative norm inequalities:
$$\left\| \sum_{l=j+1}^m d_{l,k} \prod_{i=j+1}^{l-1} (X - \xi_i I) \delta_{j+1} \right\|_2 \le T_{j+1, k} \|\delta_{j+1}\|_2$$
Summing over all steps $j \in \{0, \dots, m-1\}$ and across all $k \in \{0, \dots, 4\}$ with column scales $s_k$:
$$E_{T3} = \sum_{k=0}^4 |s_k| \sum_{j=0}^{m-1} T_{j+1, k} \|\delta_{j+1}\|_2$$
Each $\|\delta_{j+1}\|_2$ is enclosed outward using standard IEEE 754 floating-point arithmetic bounds:
$$\|\delta_{j+1}\|_2 \le \gamma_n \|A\|_2 \|r_j\|_2 \frac{h}{\gamma} + 5 u \|r_{j+1}\|_2$$
where $\gamma_n = \frac{n u}{1 - n u}$ bounds the matrix-vector product error and $u = 2^{-53}$.

---

## 5. Section T4: Summation and Input Conversion Error

### Floating-Point Accumulation of the Fused Combination
1. **Newton Term Summation:**
   For each column $k$, the vector $p_k = \sum_{j=0}^m d_{j,k} r_j$ is accumulated in binary64 arithmetic.
   By standard backward error analysis of vector summation of $m+1$ terms:
   $$\left\|\text{fl}\left(\sum_{j=0}^m d_{j,k} r_j\right) - \sum_{j=0}^m d_{j,k} r_j\right\|_2 \le \frac{(m+1)u}{1 - (m+1)u} \sum_{j=0}^m |d_{j,k}| \|r_j\|_2$$
2. **Column Scaling:**
   Multiplying $p_k$ by scalar $s_k$ incurs at most $u |s_k| \|p_k\|_2$.
3. **Fused Summation across $k = 0..4$:**
   Summing the 5 vectors $s_k p_k$ for $k=0..4$ incurs at most:
   $$4 u \sum_{k=0}^4 |s_k| \|p_k\|_2$$
4. **Input Conversion (`weight_phi_vectors`):**
   When the target input $w$ is converted from a linear combination of physical basis vectors to standard $\phi$-basis vectors via `weight_phi_vectors`, the transform reports exactness. If any scale factor is not an exact binary power, the conversion error $E_{\text{conv}}$ is computed via outward interval difference and added.

The total summation and conversion bound $E_{T4}$ is the outward sum of these contributions:
$$E_{T4} = \sum_{k=0}^4 \left( \frac{(m+1)u}{1 - (m+1)u} |s_k| \sum_{j=0}^m |d_{j,k}| \|r_j\|_2 + u |s_k| \|p_k\|_2 \right) + 4 u \sum_{k=0}^4 |s_k| \|p_k\|_2 + E_{\text{conv}}$$

---

## 6. Section T5: Certified Stopping Rule and Work Preservation

### Total Certificate Assembly
The rigorous total error bound of the computed fused vector $F_{\text{computed}}$ relative to the exact action $F_{\text{exact}} = \sum_{k=0}^4 s_k \phi_k(hA) w_k$ is:
$$E_{\text{total}} = \text{add\_up}(E_{T1}, \text{add\_up}(E_{T2}, \text{add\_up}(E_{T3}, E_{T4})))$$
By Theorems T1–T4 and triangle inequalities:
$$\|F_{\text{computed}} - F_{\text{exact}}\|_2 \le E_{\text{total}}$$

### Stopping Rule Contract
- The recurrence evaluates $E_{\text{total}}$ at each Newton degree $m \ge 1$.
- The recurrence terminates if and only if:
  $$E_{\text{total}} \le \text{total\_budget}$$
  where $\text{total\_budget} > 0$ is the user-specified tolerance.
- **Zero-Slack Contract:** A report is marked `converged = true` with `TotalErrorStatus::Certified { bound: E_{total} }` **strictly** when $E_{\text{total}} \le \text{total\_budget}$. Under no circumstances is the uncertified last-two-term heuristic used for certified stopping.
- If the degree cap $m = \text{LEJA\_DEGREE\_CAP} = 128$ is reached without achieving $E_{\text{total}} \le \text{total\_budget}$, the action sets:
  - `converged = false`
  - `total_error = TotalErrorStatus::Uncertified { reason: ... }`

### Speculative Work Preservation Contract
When an attempt fails to meet the budget (`converged = false`):
- All operator products performed (`block_products`, `vector_products`) are accumulated into `work.poly_block_products` and `work.poly_vector_products`.
- All coefficient and node setups are accumulated into `work.poly_coefficient_setups`.
- No speculative, probe, or candidate work is discarded.
- `WorkCounters` faithfully reflect all arithmetic operations spent during the attempt.

---

## 7. Stage 0 Feasibility Summary Table

| Code | Component | Theorem / Proof | Oracle Method | Violation Count | Reviewer |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T1** | Truncation Forward | Cauchy remainder + $\phi_k^{(m+1)}$ integral bound on $\mathbb{R}_{\le 0}$ + interval subdivision on $[-2, 2]$ | Exact rational on $n \le 5$, 50-digit `mpmath` on 60 fixtures | 0 | `independent-math-audit` |
| **T2** | Divided Differences | Opitz bidiagonal series with $L \ge 0$ + geometric tail bound + `exp_interval(s)` | Exact rational on $n \le 5$, 50-digit `mpmath` on 60 fixtures | 0 | `independent-math-audit` |
| **T3** | Recurrence Rounding | Local residual $\delta_{j+1}$ propagated by Newton tail polynomial $Q_{j+1, k, m}$ over $[-2, 2]$ | 50-digit `mpmath` perturbation comparison on 60 fixtures | 0 | `independent-math-audit` |
| **T4** | Summation & Conversion | IEEE 754 standard backward running error + `weight_phi_vectors` enclosure | 50-digit `mpmath` accumulation on 60 fixtures | 0 | `independent-math-audit` |
| **T5** | Certified Stopping & Work | $E_{\text{total}} \le \text{total\_budget}$ stopping contract + `WorkCounters` full preservation | Contract test suite (`leja_total_contract.rs`) | 0 | `independent-math-audit` |

**Conclusion:** All conditions T1–T5 are rigorously proven and bounded outward. Stage 0 outcome is **FEASIBLE**.
