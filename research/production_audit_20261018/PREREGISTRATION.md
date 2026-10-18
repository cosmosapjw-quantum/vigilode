# Production audit continuation preregistration

## Frozen identity and scope

- Continuation: `production-audit-20261018-continuation`.
- Base branch: `audit/production-readiness-20261018`.
- Frozen starting commit: `de990927d0f9197f0e37fcaeccee33bea6c7b8af`.
- Frozen contract: `NEXT_STEPS.json` and the 2026-10-18 production-audit findings, with this continuation limited to their stated acceptance conditions.
- Preserve `Cargo.toml`, `Cargo.lock`, prior audit outputs, failed runs, tolerance values, and production claim ceilings.

## Work units and acceptance rules

- **A01:** Repair diagnostic tail scaling for actual stored weight sum and spectral support. Keep `roundoff_enclosed=false` unless every coefficient, residual, tail, output, and returned-center arithmetic term has an outward enclosure bound to an immutable source identity and work record. Include an independent exact one-mode and high-precision multi-mode check, including intermediate and endpoint outputs, and typed nonfinite/overflow refusal.
- **A02:** Execute only if A01 supplies an independently reviewed outward center/radius interface. Otherwise record dependency HOLD and do not consume the diagnostic A01 result as a verified trajectory.
- **A03:** Execute verified damped-packet work only if A01 is complete; retain scalar damped-moment authority separately from packet authority.
- **A04:** Preserve `EstimateOnly` unless additive-underflow and operator-action, recurrence, coefficient, norm, summation, and output-scale arithmetic are all closed; preserve the warmed zero-allocation contract.
- **A05:** Charge only attempted callbacks after input-shape validation. Do not claim global output accuracy without continuous residual and full-tube one-sided-gain enclosures.
- **A06:** Implement and validate only the frozen rotating-frame carrier-zero structured stage solve, with Schur exclusion and outward residual/error witness. Keep trajectory and performance claims separate; charge setup, solve, residual, identity, and refusal work.
- **A07:** Do not repeat the existing mismatched benchmark campaign. Do not claim fairness, speed, allocation, or matched accuracy without a new frozen equation, component tolerance vector, common output grid, independent reference, setup boundary, failure-arm retention, allocator measurement, and source/build/config identity.
- **A08/A09:** Run only if both A04 and A07 dependencies meet their stated acceptance conditions.
- **A10:** Run only if A01's candidate coefficient and residual intervals exist; preserve T2's outward transform and refinement conditions.
- **A11:** Bound the analytic Gray-Scott Jacobian logarithmic norm on supplied componentwise boxes by the reaction-block symmetric-part eigenvalue formula plus symmetric negative-semidefinite periodic diffusion. This does not imply trajectory-box inclusion.
- **A12:** Issue scope-specific decisions only after implementation evidence and independent review; publication is not scientific promotion.

## Frozen verification plan

After the implementation checkpoint is reachable remotely, rerun the affected focused Rust contract targets, the affected package all-target check, formatting and diff checks. Then run the repository pre-push gates without widening tolerances or altering frozen inputs. Preserve every initial failure and its repair outcome. Report command, environment, exit code, source identity, and claim ceiling for each run.

## Stopping rules

Classify mathematical, numerical, implementation, input, and environment failures separately. If a required enclosure primitive, physical convention, or immutable source identity is absent, record the exact dependency and HOLD that node. Do not set certified authority from a diagnostic estimate, component PASS, benchmark timing, or publication event.


## Continuation results (EXEC_SHA bbb9bb4aaa9e8748109cee7b439ea2828015de8b)

- Source execution commit `bbb9bb4aaa9e8748109cee7b439ea2828015de8b` was pushed to `origin/audit/production-readiness-20261018` before final A04 scoped validation; fetched remote SHA matched.
- Focused integrator targets: 44 passed, 0 failed; core damped/Leja targets: 18 passed, 0 failed (the pre-final A04 EXEC `e9748019152b7a4063bc59d1ece3af1673bdbbaa`).
- Final A04 EXEC: Leja contract 14/14, direct zero-center coefficient radius 1/1, doc tests 13/13 (including caller-forged-cache compile-fail 1/1), workspace all-target check and rustfmt check exit 0. No full workspace `cargo test` was run.
- Independent read-only reviews: A01 overall HOLD with scoped diagnostic repair accepted; A04 scoped PASS with total-error HOLD; A05 callback-accounting scoped PASS with global-output HOLD; A06 Woodbury residual/scoped math PASS with cost/performance HOLD; A11 supplied-box logarithmic-norm scoped PASS with trajectory HOLD.
- Scope decision: packet kappa=0 HOLD; damped packet HOLD; Leja total error HOLD; generic all-output accuracy HOLD; matched accuracy/speed HOLD; existing generic RODAS5P use remains restricted to previously validated problem-specific scope.
- Machine results, source/input identity, output hashes, and detailed report: `CONTINUATION_RESULTS.json`, `CONTINUATION_DAG.json`, `CONTINUATION_DECISION.json`, `CONTINUATION_FINDINGS.json`, `CONTINUATION_INDEPENDENT_REVIEW.json`, and `CONTINUATION_OUTPUT_HASHES.json`.
- Claim ceiling: PASS refers only to bounded implementation/contract validation and reviewed component scopes. It does not establish full scientific or production promotion.
