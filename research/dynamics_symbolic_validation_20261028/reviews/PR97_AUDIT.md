# Latest PR97 critical source audit

Audited source: `87465835d9d21298d235917f036580b0b53ab645` in the fresh validation worktree. This is a read-only source and metadata review; no native test, solver, benchmark, or scientific replay was run by this reviewer. No `AGENTS.md` was found in the checkout or its scoped workspace. The requested PR94 `INDEPENDENT_PR_HANDOFF_KO.md` and `INDEPENDENT_VALIDATION.json` were read.

## Findings that justify bounded follow-up

### F1 — requested-degree trace queries still evaluate irrelevant high moments

Location: `crates/vigilode-coupled/src/trace_quotient.rs:353`, through `all_moments` and `even_moments` at lines 211–243.

For every polynomial with at least three coefficients, the implementation constructs all moments through S6. A valid input with `a=1e150`, `b=0`, and coefficients `[0,0,1]` requests only `Tr(q1^2)=4a`, which is finite. The unused `a^3` multiplication in S6 overflows, so this query returns an error. Degree-three queries have the same issue even though S3=-3 is parameter independent. This is an unnecessary safe refusal, not a false enclosure. The prior C9-03 correction handles only degrees zero and one (`CODE_REVIEW.json` explicitly says larger requests unchanged).

Minimal correction: evaluate only the moments actually needed by nonzero coefficient support, or at least by requested maximum degree; retain input validation even for unused coefficients. All-complex algebraic multiplicity semantics and outward arithmetic remain unchanged. This also removes arithmetic on ordinary low-degree inputs. Stage native large-parameter tests and an exact companion/power-sum check; record real work reduction. Charge any correction to the existing governing N task, not a newly named allowance.

### F2 — current CI mixes a historical outcome pin with a portable scientific contract

Location: `crates/rodas5p-krylov/tests/givens_production_differential_contracts.rs:223–252`; exact export/oracle hooks at 124–192. `research/production_rvj_nag_20261028/CURRENT_STATE.json` reports CI candidate `Ok(3640)` at scale 1e14 where the old profile expected `Err`.

This test reconstructs its operator using sqrt/powf/matrix products and then requires a historical Ok/Err pattern. The two bit anchors cover only three RHS entries, not the matrix. A profile-dependent status change is therefore not yet evidence of false convergence. The independent binary64 residual is checked before the status assertion, while the exact rational oracle is optional and is absent unless `Q_GIVENS_ORACLE_SCRIPT` is supplied.

Next action: retain the existing failure and pin, export matrix/RHS/preconditioner/solution bits under the actual failing profile, and explicitly invoke `tools/q_givens_exact_residual_oracle.py`. Compare requested and represented threshold results. Only then decide whether the fault is the solver, a profile-dependent fixture, or a scientifically inappropriate status assertion. A portable residual contract and a separately source/profile-bound historical replay may be appropriate; changing `false` to `true` without this evidence is not.

### F3 — trace whole-path certification is still an external exact-rational consumer

Location: `research/production_rvj_nag_20261028/code/verify_native.py:483–524`, especially 501–506; `trace_quotient.rs:376–387` provides only point/box field enclosures.

The external trace certificate uses `E_next <= E/(1+lambda*h)+h*delta`, whole-slab `E+h*delta`, lambda=39/64, and explicitly checks the complete contraction tube. The already-derived sharper rational envelope in TRACE.md is not this executed propagation. A native bounded whole-path certificate (with its own outward residual/domain proof and independent exact oracle) is a meaningful next production component. Merely replacing the recurrence cannot be credited with the earlier 8.7–9.3x cyclic speedup or treated as a generic solver speedup. Both recurrence and residual representation costs must be included in a new matched-cap cohort.

## Source and contract preservation verified now

Using metadata-only Python/git checks on this checkout:

- PR90, PR91, PR92, PR93, PR94 delivery, and PR96 are all ancestors of HEAD.
- SHA-256 of the original development-plan file, coverage inventory, and command catalog equals the PR94 frozen references.
- The frozen overlay still contains 23 node IDs, 34 dependency edges, eight execution suites, 73 changed paths, and ten interaction gates.
- These are identity/bookkeeping checks; no semantic behavior or native release gate is certified by ancestry.

Current cumulative counters must be read from PR97 `CURRENT_STATE.json`/handoff, not the older theory audit snapshot: R-STRUCT 3/3, R-ADAPT 3/3, R-HOMO 1/3, I-PROD 1/3, Q-LEDGER 1/3, PR-N03 2/3, PR-N05 1/3, other N counters 0/3. The original frozen zero counters remain historical provenance.

PR94 original-source replay must occur in its own source snapshot because its package verifier explicitly assumes no subsequent runtime change. The current cumulative tree needs different native receipts. PR96's compact results and hashes are present; some original `/tmp` raw data are not preserved in the repository. They must not be relabeled newly recovered or replayed.

## Existing mathematical and performance claims that remain valid in their scope

- Cyclic endpoint preparation binds exact state bits and model epoch; clocks are private exact dyadic labels; full Hermite control hull is checked; the certificate uses original equations and propagates incoming error. The cyclic small-real-root target is not replaced by a trace target.
- The quartic eliminant is monic and quotient length is four through the triple root. The native API rejects selected-real and real-only requests before field arithmetic. The finite exact-integer companion tests are a distinct oracle for the fixed identities.
- PR97's cyclic ratios refer to a fixed bounded model and measured complete native calls with all seven repeats retained. The tighter radius alone did not change the first passing tested grid, and some timings regress; these negatives are already disclosed.
- Trace shared-field operation reduction does not establish a complete trajectory improvement: documented medians are 607.695 microseconds direct versus 667.394 microseconds shared, with overlapping ranges and shared faster in 4/7 paired repeats. The record properly keeps this result.
- The trace direct exact-reference bound passes at N=64 while the residual gate first passes at N=256. This is certification conservatism under an unchanged gate, not demonstrated physical inaccuracy at N=64.

## Remaining production gates

Broad production remains HOLD. Q-LEDGER is still a typed repository/object authority issue (PR94 historical missing53, PR96 missing49, PR97 CI missing56, and partial-clone missing178 are different checkout/source observations). Q-API, ten interactions, full N01–N06, negative-cost evidence, supported ignored features, release/vendor graph, comparator MPI/SUNDIALS setup, and actual-client qualification remain independent obligations. The supplied F5 global count and selected singular branch do not become solved because the fixed quotient trace is smooth. PR95's chemical remainder is a separate conditional model input, not a full-model provider.

Recommended smallest integrated coding unit: requested-support trace evaluation plus a source-bound native trace whole-path certificate and exact oracle; investigate Q-GIVENS under actual profile without changing its evidence first. A symbolic-dynamics scheduling layer may be useful only if it binds a concrete mode/session/epoch guard and is measured; naming arbitrary mode labels a subshift does not establish physical conjugacy or safe dispatch.
