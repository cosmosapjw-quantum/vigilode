# Loop L1 (2026-11-16): audit, release gate, lineage reconciliation, stage-loop research

- written: 2026-11-16 (committed and pushed before any V-node run)
- base: `research/production-unification-20261115` @ `074a5699a348abd15fe8a235ec5a695354e8cd8c`; toolchain 1.94.1; branch `research/production-loop-20261116`
- approved plan: `PLAN_KO.md`; owner decisions: `docs/agent/production_solver_program_20261112/OWNER_DECISIONS_20261116.json`
- machine contracts: `docs/agent/production_solver_program_20261112/TASK_CONTRACTS.json` (ids `V00`..`V15`)
- questions:
  1. Are the recorded claims of U01-U11 and of the unmerged switcher lineage (`93392249`) supported by executed evidence?
  2. Does the package meet a release-readiness gate (R-gate) that is separate from the unchanged research gate G0?
  3. Which method-preserving or spatial levers reduce the wall time of the eight sequential RODAS5P stages at matched achieved error, and in which regime?
- rules:
  - no change to `main`, G0, tolerances, protected RODAS5P sources, tableau, controller constants or default routing;
  - an evaluation script executes solver binaries and reads numbers only from raw outputs (no literal results);
  - negative results and failed runs are kept;
  - primary metrics are deterministic counters and `perf stat instructions:u`; wall time is authoritative only with pinned cores, interleaved arms, 2 warm-ups + 7 repetitions and a recorded quiet host, otherwise it is labelled `diagnostic_only`;
  - comparisons are at matched achieved global error against an independent reference with stated reference uncertainty;
  - a consumed holdout (HIRES, Oregonator, Brusselator MOL32 of U11) is not reused as holdout.
- ledger ids: this loop uses `L-0300` and above (rows `L-0205..L-0214` exist on the unmerged switcher branch, and other `L-0205..` rows on the rei branch).

## Frozen discriminators

| node | pass | kill / fallback |
|---|---|---|
| V01 audit, R-gate | every listed finding reproduced by an independent verifier with command and evidence path; R-gate axes judged with receipts or NOT_RUN | unverified items stay leads |
| V03G default-path golden | two consecutive runs reproduce every golden file bit for bit | a non-reproducible route is excluded with its reason, never loosened to a tolerance |
| V02 trial merge | T1 `cargo check --workspace --all-targets --locked` on 1.94.1 with fix-ups limited to toolchain lines, post-1.94 std APIs, the `Jacobian::SparseJvp` match arm and lint expectations; T2 suites of both lines pass; T3 bitwise equality of states and counters on the V03G oracle between `074a5699` and the merged tree; T5 no new clippy warnings; T6 ledger ids do not collide and simulation-backed rows are superseded by INVALID rows | any T3 difference that cannot be removed by excluding its hunk rejects the merge; then additive leaf modules are ported blob-identically |
| V15 stage client kit | states and step counts bit-identical to the protected dense driver (HIRES, Robertson, Brusselator; in-place and faer LU); zero allocations per step; wall <= 1.05x at n >= 64 | sibling-driver pattern of `rodas5p_lagged.rs` |
| V05 theory | scripts exit 0; two independent implementations agree; each statement tagged exact / zero-by-design-within-representation / conjecture | conjectures are not used as gates |
| V04 cost anatomy | instrumented/plain wall <= 1.02 and phases sum to 0.90-1.00 of the total at n >= 128; trajectory bit-identical to the protected driver | otherwise `diagnostic_only` |
| V06 shared Krylov basis | stage a: JVP ratio >= 1.5 at step ratio <= 1.05 on >= 2 of 3 families against a per-stage GMRES client with identical stopping test; final: at matched error JVP <= 0.5x and instructions <= 0.8x of per-stage GMRES with the same preconditioner on 3 of 4 families | stage a median JVP ratio < 1.2 cancels v2; no family improves instructions, or a residual check fails |
| V07 speculative Jacobian | stage a (sequential emulation): step ratio <= 1.02 median and <= 1.05 max, error ratio in [0.67, 1.5], observed order >= 4.8; stage b (threaded) starts only if V04 shows a regime with (J+LU)/chain in [0.15, 0.92] and per-step wall >= 500 microseconds; stage b pass: wall >= 1.15x at matched error with step ratio <= 1.05 | any step ratio > 1.10 with g7; wall < 1.05x |
| V08 Jacobian assembly | Jacobian and trajectory bitwise equal to sequential assembly; RHS calls = colours + 1; wall >= 1.5x on finite-difference problems with n >= 64 and RHS cost >= 50 microseconds | < 1.1x |
| V09 lagged LU at large n | wall >= 1.15x at matched error in >= 1 size class with LU per step <= 0.5 and step ratio <= 1.05 | LU per step > 0.8 or steps > 1.2x in every class |
| V10 PPSW1 decision | stable for sigma in {1.2, 1.3, 1.5} with the independent oracle, matched-error step ratio <= 1.5 on new problems, and measured wall >= 1.15x in >= 1 pre-registered regime | unstable for sigma > 1.1, or the ratios fail: freeze as opt-in comparator |
| V11 geometry | second-order spatial convergence; conservation <= 1e-12; battery v4 tables at matched achieved error with reference uncertainty | no promotion |
| V14 Schwarz block | screen: median sweeps <= 2 at overlap <= 4*sqrt(gamma*h*D)/dx with a halo predictor, endpoint error indistinguishable from the exact solve at tolerance, median halo <= subdomain width/8; native stage: wall >= 1.15x against a per-stage parallel solve at matched error | sweeps > 4 |
| V03 unified frontend | 100% decision equality of `route(General)` and `GeneralOde::select` on an exhaustive grid with zero callbacks; each adapter bitwise equal to the direct integrator call; refusal before any callback; default-route golden unchanged | default routing is not changed; an adapter may stay unavailable |

The V07 band follows from the overlap window of two of eight stages: the saving is min(T_J+LU, T_chain/4), so the speed-up is at most 1.25 and reaches 1.15 only for T_J+LU / T_chain in [0.15, 0.92].

## Results (appended after runs; nothing above this line changes)
