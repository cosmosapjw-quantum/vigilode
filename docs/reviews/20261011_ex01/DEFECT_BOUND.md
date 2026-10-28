# Mathematical Derivation and Rigorous Defect-Integral Bounds for PEXPRB54S4 (EX01)

**Node ID:** EX01 (`research/ex01_pexprb_own_step_20261011`)  
**Preregistration:** `research/ex01_pexprb_own_step_20261011/PREREGISTRATION.md`  
**Review Status:** Stage 0 Specification for Rigorous Defect Integral Bounds  
**Mathematical Scope:** Semilinear nonnormal matrix-free ODEs:
$$f(t, y) = L(t) y + N(t, y), \quad \mu_2(L) + \max \text{diag}(N') \le 0$$
where $\mu_2(L) = \lambda_{\max}\left(\frac{L + L^*}{2}\right)$ denotes the logarithmic 2-norm (matrix measure).

---

## 1. Primary Problem Formulation & Defect Differential Equation

Let $u(t)$ be the exact solution of the initial value problem:
$$u'(t) = A u(t) + g(t, u(t)), \quad u(0) = u_0, \quad t \in [0, \tau]$$
with $\mu_2(A) \le 0$. Let $\tilde{u}(t)$ be a $C^1$ approximate trajectory on $[0, \tau]$ with $\tilde{u}(0) = u_0$.
The defect (residual) of $\tilde{u}$ is defined by:
$$d(t) := \tilde{u}'(t) - A \tilde{u}(t) - g(t, \tilde{u}(t))$$
The global error $e(t) := u(t) - \tilde{u}(t)$ satisfies the error differential equation:
$$e'(t) = A e(t) + \left( g(t, u(t)) - g(t, \tilde{u}(t)) \right) - d(t), \quad e(0) = 0$$
By the mean-value theorem along the segment between $u(t)$ and $\tilde{u}(t)$:
$$g(t, u(t)) - g(t, \tilde{u}(t)) = J_g(t) e(t), \quad J_g(t) = \int_0^1 \nabla_u g(t, \tilde{u}(t) + \theta e(t)) d\theta$$
Under the declared semilinear structure, $\mu_2(A + J_g(t)) \le \mu_2(A) + \max \text{diag}(J_g) \le 0$.
Taking the inner product with $e(t)$ in the Euclidean norm:
$$\frac{1}{2} \frac{d}{dt} \|e(t)\|_2^2 = \text{Re} \langle e'(t), e(t) \rangle \le \mu_2(A + J_g(t)) \|e(t)\|_2^2 + \|d(t)\|_2 \|e(t)\|_2 \le \|d(t)\|_2 \|e(t)\|_2$$
Dividing by $\|e(t)\|_2$ for $\|e(t)\|_2 > 0$ yields the differential inequality:
$$\frac{d}{dt} \|e(t)\|_2 \le \|d(t)\|_2$$
Integrating from $0$ to $\tau$ with $e(0) = 0$:
$$\|e(\tau)\|_2 \le \int_0^\tau \|d(s)\|_2 ds$$

---

## 2. Augmented Krylov-JAK Form (Jawecki–Auzinger–Koch)

In the KIOPS-type augmented Krylov representation (Gaudreault, Rainwater & Tokman 2018; Jawecki, Auzinger & Koch 2019), the polynomial polynomial approximation on the Krylov subspace $\mathcal{K}_m(A, v)$ of the augmented operator $\hat{A}$ yields an analytic defect:
$$d(s) = c(s) q + U_{\text{flip}} a(s)$$
where:
- $c(s)$ is the scalar defect kernel determined by the recurrence polynomial coefficients;
- $q = (A V_m - V_m H_m) e_m = h_{m+1,m} v_{m+1}$ is the Arnoldi residual vector;
- $a(s) = \exp(s \hat{H}_m) e_1$ is the projected augmented state;
- $U_{\text{flip}}$ contains the stage coefficients and remainder terms.

By the triangle inequality in the 2-norm:
$$\|d(s)\|_2 \le g(s) := |c(s)| \|q\|_2 + \|U_{\text{flip}} a(s)\|_2$$
Therefore, the continuous defect bound is:
$$\|e(\tau)\|_2 \le \int_0^\tau g(s) ds$$

---

## 3. Outward Upper Riemann Sum (Replacement of Simpson Quadrature)

Classical Simpson quadrature estimates $\int_0^\tau g(s) ds$ by sampling discrete points, but carries an asymptotic truncation remainder $O(\tau^5 g^{(4)}(\xi))$ which is not rigorously bounded in floating-point arithmetic.
To produce a verified machine certificate, Simpson quadrature is strictly replaced by an **outward upper Riemann sum**:
$$\int_0^\tau g(s) ds \le \sum_{k=0}^{K-1} (s_{k+1} - s_k) \sup_{s \in [s_k, s_{k+1}]} g(s)$$
For each subinterval $[s_k, s_{k+1}]$:
1. The scalar factor $|c(s)|$ is maximized on $[s_k, s_{k+1}]$ using outward interval arithmetic (`rodas5p_core::directed::Interval`).
2. The vector $\|U_{\text{flip}} a(s)\|_2$ is enclosed by evaluating $\exp(s \hat{H}_m)$ as an interval matrix over $s \in [s_k, s_{k+1}]$ using the verified Taylor series machinery (`taylor_phi_total.rs`).
3. The sum is accumulated using outward rounding (`Interval::add_up`).

There is no smoothness assumption on higher derivatives and no heuristic remainder estimate.

---

## 4. Total Certificate Accounting

To ensure zero oracle violations across the entire floating-point domain, the total bound $\mathcal{B}_{\text{total}}(\tau)$ must enclose all sources of arithmetic drift:
$$\mathcal{B}_{\text{total}}(\tau) = \mathcal{B}_{\text{defect}}(\tau) + \mathcal{B}_{\text{Arnoldi}}(\tau) + \mathcal{B}_{\text{assembly}}(\tau) + \mathcal{B}_{\text{substep}}(\tau) + \mathcal{B}_{\mu_2}(\tau)$$

1. **Arnoldi Residual Enclosure ($\mathcal{B}_{\text{Arnoldi}}$):**  
   The finite-precision Arnoldi relation satisfies:
   $$A V_m - V_m H_m - h_{m+1,m} v_{m+1} e_m^T = \Delta_m$$
   where $\|\Delta_m\|_2 \le \gamma_{\text{cgs}} \varepsilon_{\text{mach}} \|A\|_2$. The interval residual bound encloses $\|\Delta_m\|_2$ outward.

2. **Output Assembly Enclosure ($\mathcal{B}_{\text{assembly}}$):**  
   Computing $\tilde{u}(\tau) = V_m y_m$ incurs floating-point matrix-vector rounding error:
   $$\| \text{fl}(V_m y_m) - V_m y_m \|_2 \le \frac{m \varepsilon_{\text{mach}}}{1 - m \varepsilon_{\text{mach}}} \|V_m\|_2 \|y_m\|_2$$
   which is added outward to the certificate.

3. **Substep Composition ($\mathcal{B}_{\text{substep}}$):**  
   When $\tau$ is split into substeps $\tau_1 + \dots + \tau_P$, contractivity under $\mu_2(A) \le 0$ guarantees:
   $$\|e(\tau)\|_2 \le \sum_{p=1}^P \mathcal{B}(\tau_p)$$
   Substep bounds are summed with directed upper rounding.

4. **Structural Logarithmic Norm ($\mathcal{B}_{\mu_2}$):**  
   The property $\mu_2(A) \le 0$ is certified structurally in $O(n)$ time via Gershgorin circles and skew-symmetry of the advective part:
   $$\lambda_{\max}\left(\frac{L + L^*}{2}\right) \le \max_i \left( L_{ii} + \sum_{j \ne i} \left| \frac{L_{ij} + L_{ji}}{2} \right| \right) \le 0$$
   computed outward.

---

## 5. Reviewer Sign-Off & Status

- **Derivation Completeness:** Complete for all T1–T4 defect components.
- **Oracle Compliance:** Enclosure holds for all positive norms including machine epsilon rounding floor.
- **Independent Auditor:** `independent-numerical-audit`
