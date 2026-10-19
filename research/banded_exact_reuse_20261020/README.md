# Banded solver hardening and exact-LU specialization

Read [CRITICAL_REVIEW_KO.md](CRITICAL_REVIEW_KO.md), then [RESULTS.md](RESULTS.md) and [VALIDATION.json](VALIDATION.json). Results/validation are generated from the actual frozen run by the closeout workflow, not handwritten PASS receipts.

## Implemented

- Standalone band solve rejects nonfinite input/output and validates storage arithmetic; tiny representable solutions remain accepted.
- Opt-in `BandedKernel::FixedWidth2` retains the original Slices arithmetic/pivot order for bandwidth2 and falls back for other widths.
- Opt-in `FixedWidth2ExactCache` reuses only successful finite factors with bit-identical rounded shift and all Jacobian entry bits. It does not skip Jacobian/RHS calls or freeze approximate Jacobians.
- Benchmark adapter now registers the analytical JVP required by OdeProblem and performs a preflight smoke before the campaign.

## Evidence

- Parent numerical library: `b56bb19ce85bc28a021138ee29689701a81bc4f0`.
- Production patch: `b51c051b045e977c038e5eec468de0004840872b`.
- Successful benchmark source: `f8e89b70528da8490fbe1febf0b0ede9ff84a01e`.
- [Successful measurement](runs/39304607635):432accuracy rows,324bit-identical output pairs,576timing processes,3744timed integrations,36paired comparisons.
- [Numerical readback](runs/39304607635/AUDIT.json):1008metrics recomputed,87full-state objects,36paired medians.
- [Original RED](runs/39298620202/BASELINE_RED.log):4real standalone failures;4retry tests passed, refuting the earlier retry hypothesis.
- [Adapter failure and workspace timeout](runs/39301232025):retained; not relabelled as numerical non-equivalence.
- [Run source binding](SOURCE_BINDING.json):the baseline library worktree used the common NEW benchmark driver; it was not a clean build solely from the historical baseline commit.

Default Indexed/Slices selection is unchanged. Full workspace test timed out after660seconds and remains NOT_PASS. No merge, production-default promotion, universal speedup, continuous-time error certificate or independent-personnel review claim. Existing triage/Diffsol changes on the upstream branch were not overwritten.

See [THEORY.md](THEORY.md), [AMENDMENT_01.md](AMENDMENT_01.md), [ADAPTER_REPAIR.md](ADAPTER_REPAIR.md), [REPRODUCE.md](REPRODUCE.md), and [DAG.json](DAG.json).
