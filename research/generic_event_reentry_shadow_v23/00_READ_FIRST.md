# G4-S5B3 threshold-free early-defect attempt geometry

## Verdict

The sealed P0-05 campaign is complete on the frozen 12-trajectory corpus. The
read-only telemetry mechanism remains numerically neutral and has complete
attempt/work coverage, but the raw scalar

\[
\eta_{c_2}=\frac{|h|\,\|D_2^{(m)}\|_2}
                   {\|U_2^{(m)}-y_n^{(m)}\|_2},
\qquad c_2=\frac14,
\]

is rejected as a blanket global admission score across tolerances. No threshold
was selected, no early abort was activated, and no method switching was
implemented.

## Source authority

- parent commit: `18947c878b53398e8400f7748167b17248b93ae5`
- parent tree: `46973a7dc3f962c6d3ab86d058106ac35b2fb720`
- branch: `generic-early-defect-attempt-geometry-v2.3`
- sealed contract: `contracts/G4_S5B3_ATTEMPT_GEOMETRY_CONTRACT.json`
- canonical corpus source: frozen G3 problem/config builders
- production tableau/controller/Krylov/BDF/Radau changes: none

## Hard-gate result

All 12 disabled/read-only trajectory pairs pass:

- bitwise-identical requested outputs and controller histories;
- identical accepted/rejected attempt sequence and proposed step sizes;
- identical existing `WorkCounters`;
- complete per-trial numerical work matching aggregate work;
- zero unscorable and zero legacy-unclassified rows;
- zero added RHS, JVP, JVP-vector, phi-action, partial-time, Jacobian-build,
  or Newton work;
- 248/248 attempts with finite early-defect telemetry.

The corpus contains 241 accepted and 7 error-control-rejected attempts.

## Load-bearing falsification

The first quadratic trial has the exact same

```text
eta_c2 = 0.0009919814843468982
h      = 0.0625
```

at all three tolerances, while the actual error norm is

```text
rtol=1e-4 : 0.0031020374  accepted
rtol=1e-6 : 0.2341414529  accepted
rtol=1e-8 : 23.41414529  rejected
```

Acceptance is tolerance-scaled, whereas raw `eta_c2` is tolerance-blind. The
complete threshold sweep therefore gives:

- all-admit control: 7 false accepts, 0 false rejects;
- best zero-false-accept point: 0 false accepts, 201 false rejects;
- accepted attempts retained at that point: 40/241 (16.6%);
- minimum total false-count: 7, achieved only by the all-admit decision class.

The descriptive pooled rank AUC is 0.723, but only seven rejects exist and the
attempt rows are serially dependent. It is not promotion evidence.

## Sampling limitations

- 208/248 attempts (83.9%) are from oscillatory-PR.
- 154/248 attempts (62.1%) are at `rtol=1e-8`.
- Six of seven rejects are oscillatory-PR.
- All rejects occur at `t=0` in attempt index 0 or 1.
- The corpus is small/medium dense; no sparse or `N=2048` holdout was used.

The last point leaves event-conditioned geometry open, but that view was not
sealed for selection in this node. It cannot rescue the tolerance-blind scalar.

## Paired overhead

The disabled/read-only protocol used one warmup pair and seven measured
alternating-order pairs after calibration froze 16 complete-suite repetitions
per arm. The read-only/disabled ratio is:

- median: `0.9973326022`
- MAD: `0.0117362248`
- observed P95: `1.0189231057`

Ratios below one are measurement noise, not a telemetry speedup. The supported
claim is only that this run resolved no material positive solver-path overhead.

## Claim status

- read-only telemetry mechanism: **PASS**
- numerical/work-ledger neutrality: **PASS**
- complete threshold-free attempt geometry: **PASS**
- raw `eta_c2` as blanket global admission score: **REJECTED**
- selected threshold: **NONE**
- early abort / active switching / controller transfer: **NOT AUTHORIZED**

## Next node

Seal and test a tolerance-aware, component-scaled early defect using the already
available `y_n`, `U2`, and `D2`, with no new expensive numerical work. Retain raw
`eta_c2` as a control, predeclare full-attempt and entry-window views, and again
forbid threshold selection and active switching. Details are in
`02_NEXT_NODE.md`.
