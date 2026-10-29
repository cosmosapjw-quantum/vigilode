# PR96 and attached RVJ/NAG evidence: one-pass critical audit

Date: 2026-10-28. Authoring task: `/root/rvj_pr_audit`. Execution mode: theory first, selected physmath 1.3.0 R3/R6/R9, with read-only C2 source mapping. This audit makes no runtime changes, performs no native campaign, and does not review an earlier reviewer as a new acceptance authority. Its companion `PRODUCTION_GATES.json` is a handoff, not a claim that pending gates passed.

## 1. Finding and admission decision

PR96 materially improves the reliability of the cumulative evidence and closes three narrow defects: independent verification of returned Givens/GMRES solutions, attempted versus admitted/refused work accounting, and the inspected lint failures. It does **not** complete the six remaining native extensions or the ten interaction gates. Its own broad-production HOLD is appropriate.

The RVJ attachment supplies a useful next implementation: a bounded cyclic algebraic closure with an exact scalar rational tangent circuit and a whole-path certificate. It also supplies a distinct all-root trace observable that remains polynomial at multiplicity changes. Neither is evidence that the present Rust solver already implements those clients. An additive, model-specific native implementation is justified after the parent combines the two mathematical research results; promoting it to generic production routing is not yet justified.

The strongest immediate performance opportunity is certificate work. In the attachment's matched N=64 runs, the whole-path certificate is 98.4–99.3% of compiled-method total time. Removing more root-finder work cannot by itself remove that dominant cost. Fixed-degree residual/Bernstein evaluation and reuse of identical endpoint data are bounded hypotheses worth implementing and measuring.

## 2. Source and evidence identity actually inspected

Repository baseline: PR96 `13f3298bd0213f121e07385903f9508c435ad6a1`; comparison delivery PR94 `e935878fefe795a1b7e88aba64e5e30ef7be1578`. PR96 records execution source commit `5c57bf146f2f5faf69f711f133a83fcb265cee90`, tree `9633bde7dda1e9dd4748d1aecc8f22af68b5f5ac`. The read-only comparison

```text
git diff 5c57bf146f2f5faf69f711f133a83fcb265cee90 13f3298bd0213f121e07385903f9508c435ad6a1 -- crates tools Cargo.toml Cargo.lock
```

is empty. The inspected runtime/test/tool source therefore has not changed between that measured source and final PR96. This does not transfer an earlier native receipt to any future implementation.

This pass recomputed all **24/24** hashes in `research/cumulative_validation_20261028/EVIDENCE_MANIFEST.json::files`, with no mismatch, and all five selected source hashes in `evidence/SOURCE_IDENTITY.json`, also with no mismatch. It checked that the referenced original integrator-extension log, rapid raw runs and exact Givens oracle receipt are **not present in this workspace** at their recorded `/tmp` paths. The hashes establish identity of available compact artifacts; they do not reconstruct missing raw bytes. All previous scientific outcomes below are inherited recorded evidence, not new numerical executions by this auditor.

The execution commit was not remote-reachable before the old campaign; only its preregistration commit was. PR96 discloses this process miss. It should remain recorded, rather than being retrospectively marked satisfied or used to erase otherwise source-bound numerical observations. New claimed native runs should bind the actual source and preserve accessible raw receipts.

The parent intake preserves selected original attachment bytes and hashes in `intake/INPUTS.json`. Sources read here include the F1/F23/F6 derivations, F23 native implementation and results, attachment review/claim/failure records, and the PR96 manifest, reports, summary, source identity, review decisions and relevant diffs. Related PR95 `36104bfc65d1fdc695c8acea383c54b50a1c4e78` was initially tree-enumerated; optional git blob reads encountered missing promisor objects and unavailable network. The parent subsequently retrieved its pinned theory into `intake/PR95_THEORY_KO.md`, whose complete text was read here. This is theory-document coverage, not replay or a review of all PR95 code. None of that branch is merged.

The earlier project protocols `project_sources/04-*` and `05-*` inform original-physics preservation, source tracing, independent physical checks, explicit DAG dependencies and useful plots. Their older generic stage/deferral boilerplate does not override the current user's explicit theory-then-coding request. Their application-specific CMB assumptions are not imported into this generic solver/client audit.

## 3. Old defects closed in their documented scope

| Finding | Inspected code and evidence | Status and limit |
|---|---|---|
| Q-GIVENS: returned success previously lacked a sufficiently independent original residual check | `crates/rodas5p-krylov/tests/givens_production_differential_contracts.rs`; `tools/q_givens_exact_residual_oracle.py`; PR96 `evidence/REVIEW_DECISIONS.json::q_numeric` | Closed for the recorded fixture scope. Every returned `Ok` is checked before pinned status expectations. The Python oracle uses exact represented binary64 A, b and x as `Fraction`, squares the residual norm, and compares both the exact requested and represented solver thresholds. A deliberately corrupted x must fail. Decimal120 is display only. |
| Q-FAIR-AB: callback refusal counted as admitted work | `crates/rodas5p-fair-ab/src/{budget,adapters,contracts}.rs`, `tests/fair_ab_contracts.rs` | Closed for the tested callback scope. `OperatorBudget::refused()` separates refusals; `operator_total` means requests. Independent callback observations establish A=E+R, E=C+F, E<=B for admitted failures and solver/diagnostic refusals. This does not execute the complete I-WORK interaction gate. |
| Q-LINT | Inspected integrator/example/test changes, `REVIEW_DECISIONS.json::q_lint` | Recorded Clippy exit0 and focused202pass/1ignored. Iterator changes preserve the inspected numerical order and arithmetic expressions; intentional large-value APIs remain inline using explained lint expectations. This is not a speed result or a proof that all numerical paths were exercised. |

Q-GIVENS exact export/oracle execution is **environment-triggered**. A normal cargo-test exit0 alone does not show that the Fraction path ran. A local replay claiming that result must set `Q_GIVENS_ORACLE_DIR` and `Q_GIVENS_ORACLE_SCRIPT` and retain the create-only exports and receipt. No new flaw in the supported fixture acceptance was established in this pass.

The release integrator extension records **1196 pass /54 ignored** after the first1800s attempt timed out with917 pass /24 ignored. These are overlapping attempts: do not report2113 distinct passing tests. The separate first release command records830 pass /21 ignored. Documentation/check success, benchmark exit0 and physical qualification remain separate statuses.

## 4. Current blockers and their actual meaning

| ID | Classification | Evidence / current defect | Required bounded resolution |
|---|---|---|---|
| AUD-01 | Current qualification blocker | Q-LEDGER scanner missing49 in current campaign; PR94 CI missing53. The attempted typed repair was rejected for external-URL collapse, malformed record handling, suppression of selected failure lines, and scan-scope-dependent ownership. | Repair authoritative object classification and add adversarial cases without bulk allowlisting. Preserve both source-specific counts. Cumulative correction use is now1/3. |
| AUD-02 | Current qualification blocker | Q-API remains HOLD; downstream exhaustive public struct/enum compatibility receipt is absent. | Compile a small downstream client against the intended public API and previous supported contract before claiming compatibility. Prefer an additive RVJ module rather than gratuitous changes to existing configurations. |
| AUD-03 | Current implementation/validation gap | `SUMMARY.json`: PR-N01–06 and I-STATE/I-EPOCH/I-WORK/I-BUDGET/I-POLY/I-ROOT/I-STRUCTURE/I-NEGATIVE/I-PROFILE/I-CLIENT are all NOT_RUN. Existing similarly named helper tests are not those registered task receipts. | Execute only the relevant bounded implementation and interaction receipts now; leave other IDs pending in the local DAG. Never mark all six tasks complete because a research prototype runs. |
| AUD-04 | Current native integration gap | Attachment F23 is standalone C++; F6 is a polynomial-observable Python client. `stage_chart_candidate.rs` is a different research Newton/JVP/LU stage adapter, `chart_transport.rs` is another model, and `fixed_point_continuation.rs` only certifies declared uniform provider premises. | Implement an explicitly supported native closure/trace path with original residual/domain/error contracts; bind branch and observable semantics. Do not relabel the old modules as already implementing F23/F6. |
| AUD-05 | Evidence availability / process issue | Compact manifest is intact, but selected raw logs and exact oracle receipt are unavailable here; remote-before-execution old process gate unmet. | Preserve source/binary/argv/environment and raw output for new bounded executions. Inherit old results with explicit provenance and limits, not a false fresh replay claim. |
| AUD-06 | Current negative scientific result | Q-NEGATIVE reproduces W-filtered cost15/7>1.3 and inaccurate Off interior error311.467. The matched cubic driver was rejected and not run. | Keep the negative arm excluded from a production success claim. Driver repair must reject missing initial output and conflicting duplicates and execute error arithmetic in the declared precision context. Do not substitute endpoint-only error for physical interior accuracy. |
| AUD-07 | Scope-specific release hardening |54 integrator ignored tests,21 ignored in the separate first release command, two inherited coupled formatting failures; standalone vendor validator exit2 from missing vendor_dir. | Classify relevant ignored tests and run the required supported-profile subset locally. Correctly invoke the real vendor validator if claiming vendor qualification. Locked/offline build success and synthetic mode checks do not prove the missing vendor gate. |

AUD-01/02/03/06/07 constrain broad production qualification, not the mathematical permission to implement and test the bounded new RVJ client. AUD-04 is the immediate coding gap. This distinction prevents a large governance repair from blocking useful admitted science while also preventing a small science result from erasing open production gates.

## 5. Attachment interpretation and theoretical boundaries

### 5.1 F23 cyclic closure is a justified finite target

For `G_i=q_i+q_(i+1)^2/4-x_i`, X=[-1/8,1/8]^n and Q=[-1/4,1/4]^n, the supplied contraction proof gives1/8, root sensitivity8/7 and reduced slow-field dissipation6/7. These are exact mathematical constants. For original whole-slab residual bounds Dg,Df, the inherited certificate uses delta=Df+Dg/7, Ewhole=E+h delta, Eend=E/(1+6h/7)+h delta and Qwhole=8(Ewhole+Dg)/7, with every native operation outward. Parent theory must approve any sharpening before implementation.

The native attachment uses 36 configurations /252 timed integrations, with2160 distinct and15120 timed slabs. Its independent Fraction oracle checks27 representative slabs and135 resulting bound comparisons, not every recorded slab. The 204 interval cases /816 exact corner checks and15 cyclic identities support the recorded arithmetic scope; they are not a verified compiler or all-platform floating-point proof.

The comparator is not deliberately dense: Newton uses the same O(n) cyclic structure. Krawczyk is **post-Newton inclusion**, not an independent candidate root algorithm. The first passing tested grid is N=64 for all three methods at n=4,8,16; that is not proof of the globally optimal integer step count. N=16/32 failures remain negative evidence.

`native.cpp::Integrator::root` uses residual2e-16 and a12-iteration cap; `krawczyk` uses radius2^-42. These are candidate-generation/resource choices in the old comparator, not derived physical error constants. They do not invalidate the independently applied final certificate, but should not become supposedly theorem-derived production defaults. Accuracy stopping for a new root fallback should derive from the remaining physical budget and the represented arithmetic floor; a resource limit must be labelled as such.

### 5.2 F1 does not cross a selected-root singularity

The u and v charts have the same determinant d=4uv-1. Switching coordinates does not enlarge their regularity domain. Its separate discriminant-crossing control follows a regular selected root while *other* roots collide. On the small box, contraction gives actual branch identity; zero original residual alone is insufficient because another exact root exists outside the selected Q. A future atlas route must check each one-sided branch/domain certificate and account for slow-state jumps. These are representation restrictions, not a discovered failure of the valid local contraction method.

### 5.3 F6 changes the observable contract

The full quotient trace is over all complex roots with algebraic multiplicity. It is polynomial at the triple root and can avoid all root enumeration. It cannot be substituted for a selected real branch or a branch-dependent physical amplitude. A trace API should explicitly reject such requests. This is an admissible complementary polyalgorithm route when the requested observable is actually an aggregate trace.

The attachment reports meaningful whole-path residual bounds for its manufactured coupled trace ODE, plus a separately evaluated exact-path error. The controlled ODE is different from the selected-branch baseline. Its roughly10x integration-only improvement reduces to approximately1.29x recorded full-time ratio when shared rational certification is charged, with sequential-batch timing and no robust general speed claim. It supplies no generic compiler-scaling result.

### 5.4 Open global theory is separate

The F5/F4 correction invalidates116/58 as a complete intrinsic global collision count; conditional120/60 requires unresolved completeness assumptions. Local quotient length3 is only established at its stated D=0 point. Those counts and asymptotic germ expansions must not be used as finite-domain admission constants. Their open completeness/stratification problems do not prevent a separately proved contractive local closure or full quotient trace from being implemented.

### 5.5 PR95 supplies a conditional remainder leaf

The separately inspected PR95 chemical theory gives F(z)=b+Pz+(e^T z)Qz, J(z)=P+(e^T z)Q+(Qz)e^T and the exact frozen-coefficient remainder R(d)=(e^T d)Qd. This is directly useful to a near-affine carrier: if J0 differs from J(z), a verified same-norm mismatch deltaJ adds deltaJ*rho to Gamma*rho^2. Omitting the rank-one electron-feedback term changes the mathematical model; it is not an admissible optimization.

Its36 scalar entry ranges and72 attaining witnesses are exact for the declared independent12-rate box times the physical chemical simplex because each scalar entry is separately affine. They do not imply that all entry endpoints are attained together, that eigenvalues/determinants/RHS extrema occur at vertices, or that the actual correlated rate family is already enclosed. The fixed-Q remainder bound does not supply a uniform varying-Q family bound. Actual source-clock rates involve H; geometry and omitted full-model blocks remain OPEN_NOT_ZERO. In particular the represented hydrogen total1-2^-53 cannot be silently replaced by1.

The inspected theory explicitly records zero actual model/native calls for its authoring step and production HOLD. Reuse its proved coefficient identity when the source binding, frozen coefficients, physical norm and clock premises match; do not turn it into a complete VerifiedModelProvider or use it to change the bounded cyclic F23 model without a new derivation. This is an additional constructive local theory input, not a reason to merge PR95 or replay its historical campaign now.

## 6. Cost audit: adapter/switcher versus mode kernel

F23 matched physical budget1e-8, N=64, inherited median milliseconds:

| n | Compiled integration | Compiled certificate | Compiled total | Structured Newton total | Compiled certificate/total |
|---|---:|---:|---:|---:|---:|
|4|0.030555|1.870575|1.900971|2.130236|98.401%|
|8|0.033671|3.692006|3.725426|3.828602|99.103%|
|16|0.058017|7.398545|7.448059|7.624115|99.335%|

Medians of separately timed phases need not add exactly to the median total. Full time already includes certification; this audit does not accuse the attachment of omitting it. Ranking reverses on passing N=128 cases: n4 compiled4.338149ms versus Newton3.997807ms, and n16 compiled16.689528ms versus Newton15.770549ms. The source preserves seven in-process repeats and spread but fixed ordering; these are descriptive observations, not a general causal speed theorem.

The concrete native target `native.cpp::certificate` allocates and combines polynomial vectors, recomputes binomial ratios in `range`, and evaluates both interval endpoint slopes for every slab. For this fixed low-degree problem, precompute exact outward Bernstein weights, use fixed-size workspaces and reuse a shared endpoint slope when its represented state/model/arithmetic identity is unchanged. Validate the transformed bound against independently constructed exact original-residual polynomials, including adversarial coefficients; an optimization must not merely agree with its old floating-point implementation.

`rodas5p_core::directed::Interval::new/point` already rejects nonfinite inputs, unlike the attachment's singleton `I(double)`. Reuse that finite API or an equally explicit bounded implementation. The attachment's downstream malformed-certificate checks were negative, so the singleton weakness is **nonblocking hardening within its declared finite-input experiment**, not evidence that the accepted attachment returned a false certificate. Avoid public-field construction that bypasses input validation in a new boundary.

These changes concern the **mode/certificate kernel**. For the **adapter/switcher**, PR96's rapid matched run reports switch5.701655ms versus DOPRI5.591458ms, with the same accepted work in that cohort: it provides no material speed win. Its16-to32 observation Callgrind slope changes accepted steps and RHS work, so14849.375 versus14827.4375 instructions/added observation is not pure observation or selection overhead. Measure a complete persistent session with identical physical output schedule and independently counted RHS/refusals/certificate operations before attributing a change to session retention, probing or selection.

Parallelizing independent slab residual construction is plausible after the candidate trajectory is known; endpoint-error propagation still needs its exact recurrence, and thread startup/join plus every worker's attempted work must be charged. Generic nonlinear RK stage dependencies are not removed by this observation. A scalar affine error recurrence can admit associative composition only after its parameters and outward rounding contract are proved; parallel and serial enclosures need not be bitwise identical.

## 7. Bounded coding handoff and useful verification

1. **Native selected-branch closure and certificate.** Add a model-specific module with exact domain/model constants, validated inputs, branch identity and original residuals. Retain the initial error, both slow/root whole-path errors and actual represented time. Reuse finite outward primitives and fixed-degree workspaces. Independent checks: exact rational cyclic identities, exact represented-node polynomial residual/Bernstein oracle, wrong-branch/out-of-domain/nonfinite and perturbed-coefficient controls. No runtime Jacobian, matrix construction, Newton or JVP is needed for the admitted compiled circuit.
2. **Trace route with explicit observable semantics.** Add only the admitted finite polynomial aggregate class and fixed multiplicity convention. Verify Newton sums independently by exact companion-algebra traces, including a triple root and a complex-pair case. Verify the manufactured trajectory in the physical norm and its dense-path domain/error tube. Reject selected-branch/real-only requests at the API boundary. Do not silently attach this route to every algebraic client.
3. **Cost and bounded integration bridge.** A single short before/after cohort at the same physical cap should record candidate, endpoint preparation, residual/Bernstein, propagation, admission and total time. Record degree, scalar interval work, allocations/workspace, RHS/root/JVP counts and all rejected work. Interleave or rotate arms, use a monotonic clock, report all samples and actual error, and make no speed claim below measurement resolution. Broaden profiling only if the dominant phase remains unexplained. Leave unrelated N tasks and global solver comparisons in the local handoff.

The applicable existing N contracts should be attached rather than renamed: N01 target-specific actions, N02 correct stability/clock planner, N03 physical error consumer, N04 complete persistent session, N05 scalar preparation versus vector-stage work, N06 continuation-aware selection. A bounded RVJ implementation is partial evidence for the relevant contracts, not automatic closure of them. The parent should identify the controlling existing task before each correction; no additional independent allowance is created here.

## 8. Cumulative budgets and completion ceiling

Use PR96 `SUMMARY.json`, not only the older frozen plan: R-STRUCT3/3, R-HOMO1/3, R-ADAPT3/3, I-PROD1/3, Q-LEDGER **1/3 cumulative**. PR-N01–06 and the remaining Q counters are0/3 as currently recorded. This audit does not reinterpret successful prior scoped fixes as new loops or create a fourth attempt under a renamed task. A new measured correction must update its inherited controlling counter.

The appropriate present status is **theory admission recommended for bounded native RVJ work; broad production qualification remains HOLD**. The audit has established source applicability and a concrete cost hypothesis, not a new achieved speedup. The parent coding phase may update new-task gates from actual evidence, while the inherited pending, failed and negative outcomes remain visible.
