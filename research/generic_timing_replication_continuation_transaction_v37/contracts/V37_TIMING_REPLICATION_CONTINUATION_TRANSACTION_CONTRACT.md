# v3.7 Timing Replication and Continuation Transaction Contract

## Parent and immutable authority

Parent Git state:

- HEAD: `2e83cd1052d6276671c6199b06a1301dbb2ab9dd`
- tree: `31a9baf921fec9d91157cf5044540cd31638bcbc`
- verdict: `PASS_DESCRIPTIVE_ECONOMICS`

Immutable policy:

- committed trajectory: protected sequential matrix-free R-JF;
- E path: read-only shadow;
- persistence `k=3`;
- prefix absolute cap `B_abs=80` JVP vectors;
- prefix cumulative fraction `delta=0.25`;
- recommendation threshold `tau_zeta=13.39706618860016`;
- active switching disabled;
- N=2048 sealed.

This contract is authority only because it is committed before any v3.7 wall
output and before the runtime shadow path is changed.

## A. Timing replication authority

### A.1 Complete campaign unit

One campaign contains all five consumed profiles N=96, 192, 256, 320, and 384.
Each profile uses the existing six-family whole-suite measurement seam, one
warm-up pair, and seven alternating-order measured pairs. All pairs are retained.

Authority requires three passing campaigns within at most five complete campaign
attempts. Every failed attempt remains in the durable raw artifact set.

### A.2 Frozen host-quality fields

Before each attempt, record and compare:

- Git HEAD/tree and dirty state;
- Rust and Cargo versions;
- measurement binary SHA-256 and compile-time measurement-profile attestation;
- host/kernel/CPU fingerprint: kernel, CPU model, logical/physical core counts,
  microcode, NUMA count, frequency governor, and boost/turbo state;
- CPU affinity and the exact values of `RAYON_NUM_THREADS`, `OMP_NUM_THREADS`,
  `OPENBLAS_NUM_THREADS`, `MKL_NUM_THREADS`, `BLIS_NUM_THREADS`,
  `VECLIB_MAXIMUM_THREADS`, and `NUMEXPR_NUM_THREADS`;
- timing protocol constants;
- ten-second aggregate `/proc/stat` fractions with
  `total=delta(user+nice+system+idle+iowait+irq+softirq+steal)`,
  `idle_fraction=delta(idle)/total`, and `steal_fraction=delta(steal)/total`;
- swap-in/swap-out deltas;
- available thermal-throttle counters.

Hard preflight gates:

- CPU idle fraction `>=0.90`;
- CPU steal fraction `<=0.001`;
- swap-in delta `=0` and swap-out delta `=0`;
- no increase in any exposed thermal-throttle counter;
- exact identity/fingerprint/protocol agreement across passing campaigns.

Unavailable thermal counters are recorded and do not alone invalidate a
campaign; any exposed counter increase does.

### A.3 Frozen post-run quality gates

For every profile in a campaign:

- all wall and Gamma values are finite and positive;
- proposed-interval denominators are exact between arms;
- R-JF arm `max(wall)/min(wall) <= 1.50`;
- shadow arm `max(wall)/min(wall) <= 1.50`;
- absolute R-first versus shadow-first median-ratio gap `<=0.10`.

If any gate fails, the entire campaign is `NON_AUTHORITY_HOST_QUALITY_FAIL`.
No pair or profile is removed. Failed campaigns remain visible in raw and
summary artifacts. Quality decisions cannot depend on the direction of the
shadow/R ratio.

### A.4 Claim ceiling

Passing replication supports descriptive host-qualified read-only-shadow timing
only. It does not authorize speedup, active switching, fresh safety, or
same-error active-polyalgorithm claims.

## B. Continuation transaction authority

### B.1 Prefix policy remains frozen

`B^P_k = min(80, (R_k/4).saturating_sub(S^P_k))`.

Only `S_prefix` drives prefix admission. v3.6 continuation cost is not fed back
into the historical prefix chain.

### B.2 Separate event-local continuation cap

A frozen recommendation may consume its retained level-2 prefix exactly once
under an event-local continuation cap

`B^C = 80 JVP vectors`.

The transaction refuses any scalar or batched JVP request whose prospective
vector count would exceed 80 before invoking the underlying operator; denied
batches are not partially executed or charged as completed work. No new cumulative continuation fraction is introduced.
`S_total` records prefix plus continuation economics but is not an admission
authority.

### B.3 Outcomes

- `complete`: endpoint and exact work split are available;
- `budget-exhausted`: charged abstention, no endpoint/admissibility label;
- `failed`: hard shadow failure with all completed work retained.

Budget exhaustion is not an unsafe recommendation and not a numerical failure.
Recommendation is still determined only from the completed prefix and frozen
zeta threshold. Outcome fields must be separate so recommendation counts remain
exact while endpoint completion counts may change.

Every outcome must preserve componentwise
`prefix_work + continuation_work = cumulative_work`; negative or hidden deltas
are hard failures. R-JF state/controller/trace/output mutation and prefix
recomputation are forbidden.

### B.4 Consumed replay oracle

Before implementation, the durable v3.6 ledger predicts 64 unchanged
recommendations, 62 completions, and two cap-80 exhaustions. The predicted
exhaustions are semilinear ramped N=192 target attempt 12 and N=384 target
attempt 23, whose v3.6 continuation work was 140 JVP vectors.

This replay is implementation evidence only and cannot promote safety.

## Kill conditions

1. any v3.7 timing output predates this contract commit;
2. any individual pair/profile is deleted or omitted;
3. campaign authority depends on a favorable ratio direction;
4. a continuation invokes JVP 81;
5. budget exhaustion emits a full-E endpoint or admissibility label;
6. recommendation decisions drift from the frozen prefix witness;
7. completed work disappears or a negative delta is hidden;
8. R-JF state/controller/trace/output changes;
9. prefix recomputation, explicit Jacobian build, direct factorization, or Newton
   iteration appears in the shadow path;
10. any frozen constant is retuned, active switching is enabled, or N=2048 is run.

## Retrospective v3.6 diagnostic only

Applying the newly frozen quality thresholds retrospectively does not rescue or
filter v3.6: N=384 violates the arm-span and order-gap gates, so the **entire
v3.6 campaign** would be non-authority under this contract. All 35 pairs remain
retained. This diagnostic is used only to show that the rule catches the known
gross host pathology; it does not rewrite the v3.6 verdict or create new timing
evidence.
