# VigilODE component benchmark: approved execution contract

User approved the full benchmark plan on 2026-10-18. Solver baseline:
`26f0b3a187ab41f7d68922491cc489b268c0caa1`, branch
`research/all-dag-completion-20261018`. Existing A01–A12 evidence remains canonical
in `research/all_dag_completion_20261018`; this campaign does not replace that DAG.

## Frozen scientific and execution scope

Inventory every current numerical component and algorithm variant, including
integrators, direct/Krylov solvers, polynomial actions, controllers, output,
certificates, allocation/workspaces, routers and specialized clients. Every
component receives one recommended action: `promote`, `prune`, `need to optimize`,
or `specialized for specific situation`, with separate evidence status. Missing
evidence implies HOLD, not demonstrated slowness. Prune never deletes source.

The corpus contains 24 ODE anchors across eight balanced families and six
mass-matrix/DAE cases. Up to twelve stress cases are permitted. DAE scope is
capability evaluation only; no new DAE solver and no hidden constraint removal.
Equation, represented parameters, IC/BC, output mesh, tolerance and reference
identity are frozen in each CaseSpec. The common tolerance pairs are
1e-4/1e-7, 1e-6/1e-9, 1e-8/1e-11; selected tight stress uses 1e-10/1e-13.
PR101 and Robertson anchors retain rtol=1e-8, atol=1e-11. PR101 retains [0,1]
and 101 outputs. Fixed-step methods use a declared h-refinement ladder.

Error denominators use atol_i + rtol*abs(reference_i), never candidate state.
Reference uncertainty must be at most 5% of that budget. Numerical reference
convergence is not an outward certificate. Same-target cost and same-achieved-
accuracy cost are distinct; the existing A07 error-ratio <=1.05 gate is retained.
All failed arms, commands, exits, first logs, setup/JIT/output/certificate and
rejected work are retained. Missing counters are null, never zero. Zero allocation
requires an observed positive control. No tolerance widening or protected holdouts.

Published assumptions remain binding: A03 carrier=0; A06 exactly represented
weight sum=1; A05/A11 immutable generated owners; A09 stage root, ODE endpoint and
continuous output are different claims; A10 retains original residual/tail budget.

Compare VODE Adams/BDF (isolated processes), SciPy RK45/DOP853/Radau/BDF/LSODA,
Diffsol0.16.1 BDF/TR-BDF2/ESDIRK34/Tsit45, Julia Tsit5/Vern9/Rodas5P/RadauIIA5/FBDF
and applicable DFBDF/IDA, with distinct existing CVODE/Hairer adapters. Freeze
external environments separately; do not change the root Cargo dependency graph.

## Confirmation and decisions

Independent screening precedes at most sixteen selected primary contrasts.
Freeze configurations and all inferential hypotheses before 17 independent paired
process rounds; randomize arm order. Exact order-statistic median log-ratio CI
uses largest k with 2*BinomCDF(k-1;17,.5)<=.05/M, including all action-supporting
contrasts in M. No optional stopping or outcome-dependent extra rounds. Gain
threshold: CI speed ratio >=1.10. Prune by performance requires >=20% confirmed
cost disadvantage with no compensating accuracy/memory/work/capability benefit.
A confirmed >=25% critical-stratum regression prevents promotion. Incomplete
coverage or unresolved intervals implies HOLD. Timing remains diagnostic_only;
recommendations do not release broad production/statistical authority HOLD.

## Resources and ownership

Existing NCP instances only; cumulative cap 72 instance-hours across hosts.
P0=6h, P1=12h, P2=20h, P3=24h, P4=6h, reserve=4h. Count admitted host lease
wall time once per host, including builds, JIT, idle time, failures and recovery.
Reserve timeout and cleanup before admission. Screening timeout60s,
confirmation solve600s, reference1200s. One diagnosed infrastructure retry.
No killing or changing other users' jobs. Performance requires controlled
contention; guest CPU affinity is not evidence of physical-core exclusivity.
Checkpoint each job and resume without duplicate completed work.

Approved Astra-ultra planning is reused. Lower-cost workers implement and run;
one fresh Astra-ultra agent performs final independent analysis. Requested and
observed runtime identities are separate; absent observation is
NOT_INDEPENDENTLY_OBSERVED. No recursive review tree.

## Shared implementation interface

Python package: tools/component_benchmark. CLI: tools/component_benchmark.py.
Case dictionaries contain case_id, family, model, parameters, initial_state,
t_span, output_times, rtol, atol (vector), equation_form, mass_matrix and
reference_policy. A process request is {"case": CaseSpec, "solver_id": string}.
Every adapter reads --request PATH and emits one JSON object with status,
times, states, counters, timing, allocations and optional certificate. Raw status
is OK / UNSUPPORTED_CAPABILITY / NUMERICAL_FAILURE / ENVIRONMENT_FAILURE.
The parent runner binds source/input/output and computes the scientific gates.

Files owned by runner worker: tools/component_benchmark.py; package __init__,
protocol.py, runner.py, analysis.py, budget.py, registry.py; corresponding tests.
Files owned by case worker: package cases.py, models.py, references.py,
scipy_adapter.py, julia_adapter.jl; corresponding tests.
Files owned by native worker: integrator example component_benchmark_native.rs
and its support module; isolated Diffsol adapter source; corresponding tests.
Controller owns campaign evidence/docs, resource preflight and integration.

Implementation and tests precede the remote-reachable EXEC_SHA checkpoint;
numerical campaign receipts bind that execution revision. Historical evidence
and first failures are never rewritten to conceal results.

## Approved implementation addendum: independent probes and reference gates

Read-only Astra-ultra contract review authorized the following bounded details
before numerical execution. The review is planning authority, not observed
runtime-model identity or a numerical result.

Probe points are serialized once: (t0,y0) and (t0+0.37 span,
y_i+0.01(i+1)), with v_i=0.5+0.01i. Use full Jacobians for n<=512;
otherwise columns 0,n//2,n-1 and full JVP. Compare original f, including DAE
algebraic rows, against independently transcribed 80/120 decimal-digit forward
AD applied to exact represented binary64 inputs. The oracle disagreement must
be <=1e-40 S. With eps=2^-52, the analytic allowance is
B=256 eps S +4 delta80/120+8*2^-1074, where
S_rhs_i=max(1,abs(f_i),sum_j abs(J_ij)*max(1,abs(y_j))),
S_jac_i=max(1,sum_j abs(J_ij)), and
S_jvp_i=max(1,sum_j abs(J_ij*v_j)). Every entry must pass.
Finite-difference probes use actual represented endpoints and denominator:
Q=(Fplus-Fminus)/(2h), R=(Bplus+Bminus)/(2 abs(h))+
16 eps max(1,abs(Q)), T=abs(Q-analytic), BFD=R+T.
Require both error<=BFD and BFD<=1e-5 S. Assembled FD JVP uses
sum_j abs(v_j)*BFD_j +16 eps sum_j abs(Q_j*v_j).
These finite probes establish diagnostic parity, not global model equivalence.
Negative controls cover changed equation/grid/source and corrupted output.

Reference reuse requires identical model, equation_form, parameters, IC/BC,
mass matrix, initial derivative, time interval and full requested grid, bound
by a physical-input digest and raw request/receipt/source hashes. Tolerance
reuse preserves the original request identity and reapplies its error gate.
Numerical references retain Radau at rtol=1e-10,1e-11,1e-12 and BDF at1e-12.
With the frozen output budget b, require
Dfine<=0.5 Dcoarse+N, N=norm(8 eps max(1,abs(Rfine))/b).
Use u=2 max(abs(Rfine-Rmedium),abs(Rfine-BDF))+
8 eps max(1,abs(Rfine)); require max(u/b)<=0.05.
This empirical uncertainty is not an outward enclosure.

Accuracy matching includes uncertainty: L=max(max(0,abs(y-ref)-u)/b),
U=max((abs(y-ref)+u)/b). Require positive L in both arms and
max(Ucandidate/Lbaseline,Ubaseline/Lcandidate)<=1.05.
The sole zero exception is zero errors with zero reference uncertainty.
Unmatched arms retain same-target cost and work-precision results only.

## Approved HPC scheduling addendum

The user's 64-vCPU/128-GB host request changes scheduling, not the corpus,
equations, tolerances, outputs, sample size, cumulative ledger or claim ceiling.
P1/P2 correctness/reference/screening use resource-aware disjoint guest CPU
slots: reserve eight CPUs and16GiB, cap56 single-thread processes, honor actual
affinity/cgroup/available-memory limits. Reserve512MiB for native jobs and2GiB
for Julia/reference jobs. Multithread arms reserve their declared CPU count.
Record per-process-tree peak RSS, affinity, CPU activity, steal and OOM events.
Concurrent screening elapsed time is contention-dependent diagnostic evidence.

P3 is a controlled shared-host experiment with at most16 fixed single-thread
lanes, one per frozen contrast. It is not isolated latency. Each lane reserves
ceil_MiB(2*max(screened RSS of both arms)+512MiB). Freeze fixed memory cohorts
after screening. For each of17 rounds and each cohort, independently randomize
AB order per contrast, launch all first arms, wait for all, then launch all
second arms and wait for all. Faster lanes idle at barriers. No other campaign
screen/reference/build jobs run during P3; existing unrelated jobs are preserved.
No extra replicas fill unused CPUs. Changes in available resources cause HOLD,
not silent concurrency changes. Capture background activity/topology and all
attempts. Affinity escape, unregistered campaign work, OOM/resource kill or
assigned-CPU steal>1% contaminates the affected round. Missing telemetry is
UNKNOWN. All17 valid pairs are required; no selective deletion or favorable
reruns. One diagnosed infrastructure retry retains both attempts.

Bonferroni correction includes all frozen action-supporting contrasts.
Cross-contrast dependence does not invalidate that correction; independence
and stationarity across rounds remain explicit assumptions for exact median
intervals. Timing authority and broad production remain HOLD/diagnostic_only.
P3 admission reserves600s per pending barrier plus cleanup within its24h cap.
The host lease charges wall time once, including preparation and waiting.
