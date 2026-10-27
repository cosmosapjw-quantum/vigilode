## Post-push qualification update — 2026-10-28 KST

**This remains a draft, scoped implementation PR. Full repository CI and deployment release qualification are not complete.**

The scientific implementation, original measurements and 21-page REVTeX report
are frozen at `78bb8775bf553ad2b514575a0ef4a025446f5d28`. This follow-up adds
metadata, independent read-only CI diagnosis, public logs, and a local
qualification plan. It changes no numerical source and performs no additional
native experiment or performance-tuning iteration.

Completed remote runs on that scientific commit show green production fmt,
facade/core Clippy, and docs/examples/serialized smoke. Other jobs fail:

- Givens pins a historical failure but now receives `Ok(3640)` at RHS `1e14`.
  The failure precedes its external residual assertion. Ten reviewed inputs
  and solver-source files are unchanged from PR91; false convergence is not
  established by this observation. Exact input bits and independent residuals
  must precede any fixture repair.
- The A1 workspace closure exposes an inherited fair-AB accounting mismatch:
  attempted entries `A` include budget refusals, whereas admitted entries `E`
  do not. The scoped identity is `A=E+R`, `E<=B`; the observed case is
  `A=2,E=1,R=1,B=1`. It does not traverse the repaired Lagged driver.
- The bench job installs SUNDIALS 7.1.1 on Ubuntu 26.04, while the existing
  bridge declares 6.4.1 and hard-codes GCC without MPI include/link discovery.
  MPI development headers are installed; the first failure is discovery.
  A version-qualified bridge/environment is required.
- Broader integrator Clippy exposes 23 inherited diagnostic sites, outside
  the successful facade/core scope. Compiler lib/lib-test counts overlap.
- The two new public `SwitchingConfig` fields are a real exhaustive-literal
  source break. A release API/version/migration decision is still needed;
  the nonblocking semver job is not proof of compatibility.
- The ignored suite reproduces the explicitly retained W-filtered negative
  result `15/7>1.3`. Preserve that failed research criterion and review its
  expected-negative regression contract; do not loosen the threshold or
  infer matched-accuracy cost from the inaccurate Off arm.

The missing node/ledger entry is registered as **L-0173 / INCONCLUSIVE**.
It is explicitly retrospective: the aggregate protocol was not committed
before execution and no unopened holdout was used. Wall-time ratios remain
**diagnostic_only** under the existing timing policy. Appending the registration
does not retroactively satisfy that policy or invalidate the separately stated
algebraic derivations. The node checker passes with 170 rows and the original
169-row prefix intact. The new policy-only authority check passes; the full
authority scan fails in the shallow local history and remains unqualified.

See `research/production_solver_closure_20261027/post_push/STATUS_KO.md`,
`STATUS.json`, and `LOCAL_DEVELOPMENT_PLAN.json` for the **23-node / 34-edge**
expanded DAG, seven concrete CI qualification tasks, and six optional research
PR contracts. The original 16-node science plan and evidence manifests remain
unchanged. The previous 464 distinct post-repair native passes retain their
scope; they do not relabel any failed remote job as passing.

---
