# Final structural synchronization: stage construction, residual authority, and trajectory transport

This final theory loop follows the native structural experiment at source `0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b`, tree `5795e928f78fe3e99bfa91a4e7d10eaa7540c4db`. It joins N1's directional quotient, K2's residual-gap identity, S1–S3's chronological trajectory certificate, PR102's scalar-affine specialization, and PR103's chemical signed-lognorm theorem. Its new conclusion is a precise sufficient contract under which cheaper stage construction can retain the same physical error budget. The current native experiment implements the proposal and floating original-residual gate; it does not instantiate every physical premise below.

The experiment is a structural numerical change: it can remove Arnoldi/MGS/QR work. Allocation-only phase1 observations remain separate. All earlier negative evidence, correction accounting, uncompleted interactions and production HOLD remain in force.

## 1. A common certificate interface for different stage constructions

Fix one attempted step, frozen represented operator/source/clock identities, a norm epoch, and the eight ordered stage equations

\[
A_i k_i=b_i(k_1,\ldots,k_{i-1}).
\]

In the current native identity-mass client, \(A_i=I-\sigma_iJ\). Fix nonnegative original-equation residual budgets \(d_i\), inverse bounds \(\|A_i^{-1}\|\le K_i\), and nonnegative coupling constants \(L_{ij}\) for the actual stage right-hand sides on a domain containing the required connecting paths. These are certified inputs, not inferred from convergence statistics. A stage constructor may use either of the following routes.

**Directional route.** For the configured initial iterate \(x_0\), set \(v=f-A_ix_0\), \(e_v=J^2v-aJv-bv\), and \(D=1-a\sigma_i-b\sigma_i^2\). If \(D\ne0\), N1 proves

\[
q=\frac{(1-a\sigma_i)v+\sigma_iJv}{D},\qquad
f-A_i(x_0+q)=\frac{\sigma_i^2}{D}e_v.
\]

Only the current direction is covered. A rigorous bound on the relation and construction error can certify the stored candidate; alternatively a separately enclosed original residual can do so. Finite caller coefficients, a small computed residual, or success on another stage do not prove a full-space polynomial identity. The implemented candidate is accepted by the existing floating residual criterion, with paid fallback after refusal.

**Krylov-prefix route, for future implementation.** For the current unpreconditioned/identity-preconditioner route, interpret the stored Arnoldi data as exact real arrays and define

\[
AV_m=V_{m+1}\bar H_m+F_m,\qquad
r_0=\beta v_0+g_0,\qquad
\widehat x=x_0+V_my+\Delta x.
\]

Then substitution gives

\[
f-A\widehat x
=V_{m+1}(\beta e_1-\bar H_my)+g_0-F_my-A\Delta x. \tag{Z1}
\]

Consequently any verified bounds \(\|V_{m+1}\|\le M_V\), \(\|g_0\|\le\eta_0\), \(\|F_my\|\le\eta_F\), and \(\|A\Delta x\|\le\eta_x\) imply

\[
\|f-A\widehat x\|
\le M_V\|\beta e_1-\bar H_my\|+\eta_0+\eta_F+\eta_x. \tag{Z2}
\]

Thus a prefix can be admitted when this certified upper is at most \(d_i\). A projected residual alone is insufficient: loss of orthogonality, action errors, initial-residual error and stored solution assembly are explicit terms. For other preconditioning sides, the reconstruction basis and original-equation map must be derived for that side; the identity-preconditioner formula cannot simply be reused under a different operator name. An independently enclosed original residual of the stored candidate can replace Z2, avoiding its decomposition. This is the same choice of authority as on the directional route.

In either route, suppose a computed original residual \(\widehat r\) has a verified evaluation error \(\|r-\widehat r\|\le\eta_{\rm eval}\). The usable bound is \(\|\widehat r\|+\eta_{\rm eval}\), not \(\|\widehat r\|\) alone. If a physical source discrepancy is not included in this residual enclosure, its separate owner must add it once at the appropriate physical comparison stage. A direct residual bound replaces the candidate-construction decomposition; summing both full decompositions would double count the same error.

## 2. Sufficient theorem for unchanged physical budget under cheaper construction

**Theorem Z.** Assume the following for each actually accepted slab, in chronological order.

1. Every stored stage, regardless of its constructor, has a certified original-equation residual upper at most its already assigned \(d_i\). All inverse, coupling, source, clock, norm and domain premises are valid for this actual attempted step. A refused proposal leaves the supplied initial guess/output contract intact and its work remains charged.
2. With \(D_i=K_id_i\) and \(T_{ij}=K_iL_{ij}\) for \(j<i\), the actual reconstruction has a valid whole-slab defect upper supplied either by stage transfer or by direct original-field enclosure.
3. The chosen physical comparison map is valid on the whole slab and its admitted tube. At boundaries, state reset and norm/source transfer have verified factors and additive errors. The accepted slabs cover the requested interval without gaps.
4. The resulting monotonically transported physical bounds satisfy the same external endpoint/whole-slab/event budget as before. A changed adaptive route is checked on its own actual slabs; it is not assumed identical to another solver's route.

Then replacing some stage constructors by the admitted directional or prefix route preserves that external physical budget.

**Proof.** Let \(\eta_i=\|\widehat k_i-k_i\|\). Subtracting the two ordered stage equations gives

\[
\eta_i\le K_i\left(d_i+\sum_{j<i}L_{ij}\eta_j\right).
\]

Since T is strictly lower triangular and nonnegative, induction gives

\[
\eta\le B=(I+T+\cdots+T^7)D. \tag{Z3}
\]

This estimate depends on admitted residuals and stage equations, not on which constructor produced a stored stage. If the reconstruction has perturbation constants C₀,C₁ and the original field has the required connecting-path Lipschitz constant L_f, its defect is bounded by

\[
\delta_{\rm base}+(C_1+L_fC_0)\|B\|_{\rm block}.
\]

A directly certified defect of the actual reconstruction may replace this expression; it already contains stage-solve effects. Apply the valid physical comparison map on that slab. At a boundary transfer the incoming error as \(E_{j+1,0}\le\kappa_j E_{j,\rm end}+\eta_{j,\rm reset}\). Positivity/monotonicity of each comparison and transfer gives the claim by induction over the actual accepted route word. Hypothesis4 then establishes the external budget. No statement about truncation error follows solely from Z3. ∎

The theorem is constructive: it identifies exactly which quantities a native certificate provider must return. It does not say the current ordinary floating acceptance gate supplies them. The six independently verified native rotation records establish represented-bit residual/error bounds for those records only. The full-IVP seventeen-grid-point comparison remains an analytic binary64 diagnostic; its maximum observed error \(1.865174681370263\times10^{-13}\) is not a continuous physical enclosure.

## 3. Joining physical comparison providers without changing their meaning

The contractive nonlinear branch already proved in DEFECT uses a valid same-norm inequality

\[
D^+E\le-\alpha E+\Gamma E^2+\delta,
\]

with a verified invariant radius and all source/rounding terms owned exactly once. PR102's scalar-affine accepted-quartic/Bernstein/Dini provider is a reusable specialization when its source, initial error and accepted reconstruction premises are satisfied. Its pointwise query-rounding displacement is added to the returned observation, not repeatedly to physical endpoint carry. PR102 was source-reviewed here; it was not merged or re-executed in this measured native tree.

PR103 supplies a different conditional ingredient: the exact maximum of a six-dimensional weighted-infinity signed logarithmic norm over24 product-simplex vertices and an independent12-rate box. The saved sharp value displays as0.1640625, versus the entrywise upper0.19140625. It is **positive**, so the sufficient same-clock chemical comparison is a growth envelope

\[
D^+E\le\mu_*E+\delta,\qquad
E(s)\le e^{\mu_*s}E_0+\delta s\phi_1(\mu_*s).
\]

For \(m=\max(\mu_*,0)\) and \(mh<1\), the coarse whole-slab bound \((E_0+h\delta)/(1-mh)\) is sufficient. Positive \(\mu_*\) does not provide the positive contraction \(\alpha\) required by the preceding invariant-tube branch. A whole-domain Jacobian lognorm already includes the restricted field's nonlinear variation; charging that same variation again as a separate \(\Gamma E^2\) term is unnecessary double counting. Missing physical cross-blocks are separate obligations, not included variation.

PR103's theorem is reusable, but general native reuse still requires closing three identified gaps: outward treatment of the exact charge factor \(3w_{\rm Li}\), arithmetic-context validation at the event-dual public entry, and validity checks for public interval inputs. Owner-accepted rate/coefficient arrays, full physical33 norm transport, omitted coupled blocks, clock identity and whole-slab defect/domain evidence remain open. Its original R6/C9 statuses remain unrun in that source; this external static audit does not rewrite them.

A symbolic route word can carry the source/norm/clock/tube certificate associated with each slab and the legal transition at each boundary. The letters do not themselves imply coverage, transversality or a numerical bound. Chemistry being quadratic in state does not imply \(J^2=aJ+bI\); PR103 therefore supplies no automatic coefficients for the new linear-stage proposal.

## 4. Actual work mixture explains the measured structural gain and loss

The following are current-source observed work and seven-block paired timing diagnostics, using the same executable and unchanged controls. A ratio is GMRES elapsed time divided by proposal elapsed time; ranges are observed block extrema, not confidence intervals. Timing includes workspace construction, integration and owned-output/workspace drop. It excludes model setup, independent error audit and JSON serialization.

| Full-IVP fixture | JVP calls, GMRES→proposal | MGS dot calls, GMRES→proposal | Median time ratio | Seven-block range |
|---|---:|---:|---:|---:|
| Rotation, n=16 |699→412|858→0|3.4253|2.6751–3.9031|
| Rotation, n=4096 |702→414|864→0|1.9729|1.9305–2.1611|
| Damped rotation, n=16 |699→412|858→0|3.1744|2.9302–3.5585|
| Sixteen-rate time-JVP |2718→2790|39168→34272|1.1863|1.0109–1.3950|
| Incorrect rotation coefficients |699→1104|858→852|0.8040|0.7466–0.9203|

The exact quadratic models remove all Arnoldi/MGS/QR stages, establishing an actual algorithmic reduction. The n=16 rotation baseline performs143 two-column cycles (286 columns); the candidate accepts144 proposals with no GMRES cycles. The difference from144 baseline cycles is an already-converged stage. Extra O(n) proposal loops and norms still execute; zero MGS counters do not mean zero floating-point work.

The time-JVP fixture is particularly informative. There are18 accepted outer steps and144 stage solves. The ordinary path performs144 full16-column cycles:2304 Arnoldi columns and \(144\cdot16\cdot17=39168\) two-pass MGS dots. The proposal accepts18 stages, exactly the eighth stage of each step, and refuses126. The remaining126 full cycles give2016 columns and34272 dots. No exact degree-two relation is claimed for this16-eigenvalue model.

The414 proposal actions saturate the current path's cap

\[
18\cdot2+126\cdot3=414.
\]

Here18 and126 count **first-stage zero starts and later-stage warm starts**, not accepted and refused proposals. Saturation, together with the source control flow, excludes an initial-convergence shortcut in this fixture. All eighteen accepted eighth stages have warm starts and each uses three actions. The refusals consist of eighteen zero-start stages using two actions and108 warm-start stages using three: their extra paid proposal work is360 actions. Each accepted stage replaces a19-action baseline cycle with3 actions, saving \(18(19-3)=288\). The net JVP change is therefore

\[
+360-288=+72,
\]

exactly matching2718→2790. The same run removes288 Arnoldi columns,4896 MGS dots, the corresponding orthogonalization vector work, and18 small least-squares solves. This explains how elapsed time can improve while the callback-count objective gets worse: strict work increases3060→3132. The measurements demonstrate that these objectives differ; they do not establish a universal conversion from dot/JVP counts to time.

The incorrect-coefficient control has only two accepted and142 refused proposals. JVP calls increase to1104 and every paired block is slower. This is preserved negative evidence for default indiscriminate routing. A valid live-residual fallback protects the acceptance criterion; it does not make an unproductive proposal free.

For a given run the applicable cost condition is the total paid mixture

\[
\sum_{i\in\mathcal A}(C_{{\rm base},i}-C_{{\rm accepted},i})
>\sum_{i\in\mathcal R}C_{{\rm refused},i},
\]

with all model, vector, certificate and fallback costs accounted at the intended comparison scope. Actual adaptive trajectories can differ, so a same-path decomposition must be justified before treating it as a counterfactual equality. The exact stage/count decomposition above is fixture-specific. No universal speedup or strict-switcher win follows from it.

## 5. Next structural bottleneck: prefix-controlled completion

The current `gmres_into.rs` ordinary cycle builds columns until the requested restart length or happy breakdown and solves the small least-squares problem once afterward. Its outer loop checks the original residual between cycles. There is no tolerance-based stop inside `run_cycle`; the inspected source explicitly calls `least_squares_correction` only after the column loop. The16-rate fixture demonstrates the resulting work pattern: every remaining nontrivial solve completes16 columns. This is a substantial remaining algorithmic cost, not an adapter allocation issue.

The next research target is **prefix stopping with original-residual authority**. Z1–Z2 identify a sufficient certificate. An implementation may instead use an incremental small-QR estimate as a trigger and then evaluate the stored prefix candidate's original residual; acceptance must still meet the unchanged criterion. A failed trigger must preserve valid Arnoldi state and charge extra candidate assembly/check work before continuing. Left/right preconditioning, WRMS/L2 metric epochs, rank loss, happy breakdown, callback failures and monitored-path obligations must be treated explicitly. A one-pass MGS substitution is not justified by this result.

For two-pass MGS, stopping at m rather than16 columns changes the per-cycle dot count from272 to m(m+1). This is a conditional operation-count saving, not a prediction that the fixture will safely stop at a particular m. Incremental QR, repeated original checks and certificate maintenance add work. The local coding loop must implement one source-bound candidate, measure the new operation mix, and compare full-IVP matched-accuracy results including failed probes and adverse spectra. No prefix solver was implemented or benchmarked in the present delivery, and no future speedup is claimed.

## 6. Completion and retained limits

This synthesis reuses completed algebra and adds a common sufficient acceptance/composition contract plus an evidence-grounded work explanation. Native validation at the measured source reports19 passing Rust methods,2 ignored methods,88 exact checks on six accepted/two refused kernel records, and2460 full-IVP solves including2240 timed solves. These are scoped current-source observations; prior-source tests and PR102/103 records are not added as new native results.

The current production candidate is opt-in, uses live original-residual validation and default-off structural coefficients, and records paid refusals. A fully certified physical production lane still requires the missing native enclosures and all independently open API, physical-client, interaction, platform and authority gates. Production remains **HOLD**. No new native execution, numerical correction or budget reset was performed to write this synthesis.
