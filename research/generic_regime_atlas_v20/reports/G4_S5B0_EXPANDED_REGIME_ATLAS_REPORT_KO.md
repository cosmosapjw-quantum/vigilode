# G4-S5B0 Expanded Paired Regime Atlas + Residence-Time Economics

Date: 2026-08-15 Asia/Seoul  
Branch: `generic-regime-atlas-v2.0`  
Mode: read-only paired telemetry; active switching forbidden

## 1. Current DAG state

G4-S5A showed that a conservative cost interval cannot rescue a selector that acquires
both method prefixes at every entry: coverage was insufficient and discarded-prefix cost
was economically dominant. The research loop then identified the missing variable as
**regime persistence**, not a more elaborate instantaneous cost-ratio model.

G4-S5B0 therefore executes exactly one bounded node:

> On transition-bearing generic stiff ODE trajectories, do E-K and R-JF retain their
> complementarity for long enough that method-entry cost can be amortized?

No selector, switch, BDF/Radau optimization, physical-client execution, or homotopy
extension is performed.

## 2. Frozen methods and code path

### R-JF committed path

```text
accepted state
  -> protected sequential matrix-free RODAS5P
  -> embedded 5(4) error
  -> accept/reject and PI controller
  -> commit
```

The shifted action is matrix-free:

\[
Wv=Mv-h\gamma Jv.
\]

### E-K shadow path

At every **accepted R-JF state and accepted R-JF step size**, the code evaluates fused
full-MGS `pexprb54s4` read-only:

```text
same (t,y,h)
  -> nonautonomous augmentation when needed
  -> fused phi-action / Arnoldi
  -> embedded + phi error
  -> local admissibility and cost telemetry
  -> no commit
```

An actual two-vector E-K Arnoldi prefix is timed separately. No explicit Jacobian,
direct factorization, or Newton iteration is used by either primary path.

## 3. Corpus and claim boundary

Six transition-bearing families were implemented at \(N=128,512\):

1. Robertson-ramped
2. HIRES-ramped
3. van-der-Pol-ramped
4. rotating-nonnormal
5. nonautonomous-stiff-forcing
6. semilinear-advection-diffusion-ramped

These are **scaled/replicated atlas variants** designed to create controlled stiffness,
nonlinearity, nonnormality, oscillation, and time-dependence transitions. They are not
canonical work-precision reproductions of the historical benchmark definitions.

The official Robertson problem is a three-equation chemical-kinetics IVP with a fast
initial transient followed by a smooth phase; the standard van der Pol stiff benchmark
uses a large nonlinear parameter. Those facts motivate the family selection, but do not
license treating the present replicated ramps as the official problems.

Canonical controls:

```text
rtol  1e-5
atol  1e-7
dimensions 128, 512
campaigns 7, CPU 0 affinity
N=2048 sealed
```

## 4. Execution result

```text
trajectories                 12 / 12 successful
accepted-step rows          839
scientific/work identity      exact across 7 campaigns
scientific SHA-256
65737fbcab39d50115597131d78a068b3a2acfa16365ba68ca862f0b74313628
explicit Jacobian builds      0
direct factorizations         0
fast-path Newton iterations   0
active switching              false
```

### Step-local winner map

A raw E-K winner requires local E-K admissibility and a smaller campaign-median step wall.

```text
E-K winner steps   755
R-JF winner steps   84
```

R-JF winner causes:

```text
E-K locally inadmissible       80
R-JF at least 5% faster         4
near tie                         0
```

This is load-bearing. In this atlas the complementarity is primarily **accuracy/admission
complementarity**, not a finely balanced cost boundary. When E-K is locally admissible it
is almost always faster; R-JF remains essential because it closes the steps E-K cannot
safely accept.

## 5. Residence-time result

Raw winner segments:

```text
E-K segments                  37
R-JF segments                 29
E-K segments length >=6       25 / 37 = 67.57%
E-K winner steps inside >=6 segments
                              95.89%
trajectories with E-K residence >=6
                              12 / 12
```

The longest E-K residence contained 112 accepted steps. R-JF residences were generally
shorter; the longest contained 10 steps.

Thus the original research-loop hypothesis survives:

\[
\boxed{\text{method advantage is often persistent enough to amortize entry information}}
\]

rather than flickering independently at every step.

## 6. Economics

### Static and oracle costs

Across the twelve trajectories:

```text
static R-JF wall                         4.901993592 s
raw feasible oracle wall                 1.573343489 s
sealed conservative E-entry penalty      0.001638553 s
entry-adjusted oracle wall               1.574982042 s
entry-adjusted oracle speedup             3.1124x
```

The sealed protocol charges one measured E-K prefix at the beginning of every E-K
residence segment even though a production reusable-prefix implementation would normally
continue the selected prefix. This is a conservative information penalty.

The prefix penalty is only 0.104% of raw oracle wall in this atlas. Short E-K segments can
individually exceed a 3% prefix fraction, but almost all E-K winner steps lie in longer
segments where amortization is negligible.

### Simple E-first transactional control

A separate, non-oracle control executes E-K first and, only when E-K is locally
inadmissible, pays a complete protected R-JF fallback:

```text
E-first full wall + R-JF fallback wall   1.638030608 s
fallback steps                           80 / 839
speedup over static R-JF                  2.9926x
```

All twelve trajectories remained above parity; the weakest was HIRES-ramped N=128 at
about 1.10x. This is not yet a selector promotion: older quadratic and oscillatory-PR
corpora contain regimes where E-K is admissible or executable but uneconomic. It does,
however, show that **full E-K trial + transactional R-JF fallback** deserves a bounded
combined-corpus shadow test and may be simpler than predicting a precise final cost ratio.

## 7. Repeated-timing and adversarial sensitivity

After removing timing and free-form failure text, all seven campaigns are scientifically
identical. Raw winner labels are unanimous in 817/839 rows. The 22 nonunanimous rows are
near timing boundaries; the authoritative campaign-median map contains no 5%-margin ties.

Campaign-level entry-adjusted speedup range:

\[
3.011\le S\le3.176,
\qquad \mathrm{median}=3.097.
\]

Per-row one-shot timing remains noisy:

```text
R-JF wall CV median / P95   7.19% / 32.63%
E-K wall CV median / P95    5.86% / 29.09%
```

Therefore this node establishes residence geometry and large economic opportunity, not a
final randomized work-precision ranking.

An adversarial timing mutation that inflates E-K wall by 20% while discounting R-JF by
20% still gives a 5%-margin entry-adjusted aggregate speedup of 2.163x. The opposite
mutation gives 4.400x. The selector-research continuation decision is therefore not being
driven by a marginal microbenchmark difference.

## 8. PHYS–MATH interpretation

- Independent time and state are dimensionless in the atlas; wall ratios and regret are
  dimensionless.
- Every E-K shadow uses the exact same \((t,y,h)\) as the committed R-JF step.
- E-K local admissibility is an embedded/phi error statement, not an external global-error
  proof.
- `partial_t` is supplied for nonautonomous augmentation; some constructed families use a
  bounded central difference in time. This is not a persistent stateful Jacobian factor.
- Positivity, chemical invariants, and canonical benchmark reference solutions are not
  universally certified by this atlas and are not claimed.
- The ramped/replicated families are transition probes, not substitutes for later sealed
  canonical benchmarks.

## 9. PHYS–MATH–CODE interpretation

Genuinely closed:

- twelve transition-bearing paired trajectories;
- exact committed/shadow separation;
- matrix-free/Newton-free primary paths;
- seven-campaign deterministic science/work state;
- step-local winner and residence ledger;
- actual E-K prefix amortization ledger;
- BDF/Radau byte freeze.

Still open:

- randomized or order-reversed full-step timing;
- actual method switch and controller/cache transfer;
- global-order recovery after switches;
- old G4-S1 quadratic/oscillatory-PR plus new atlas in one selector contract;
- \(N=2048\) sealed holdout;
- canonical standard benchmark fidelity.

## 10. Decision

```text
G4_S5B0_PAIRED_REGIME_ATLAS       PASS
G4_S5B0_RESIDENCE_ECONOMICS       PASS
G4_S5B0_SELECTOR_RESEARCH_VALUE   PASS
G4_S5B0_ACTIVE_SWITCHING          HOLD
```

The selector program is worth continuing, but the next node should not return to a
point-cost predictor. The most economical next hypothesis is an **asymmetric transactional
shadow controller** that combines the older difficult nonlinear corpus with this expanded
atlas and tests an E-K attempt / R-JF fallback policy with residence and cooldown.
