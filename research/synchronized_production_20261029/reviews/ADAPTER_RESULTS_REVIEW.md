# Adapter ownership result review

Review date:2026-10-29. Reviewer:`/root/adapter_theory`, the author of the ownership patch. This is a fresh raw-data reaggregation independent of the runner's aggregate calculation; it is **author-side verification**, not independent code authorship review. The separate `/root/state_audit` review supplies that role.

## Source and read-only method

Candidate native source:`ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc`, tree:`2989f07fb04134aa7bab38e38d7272e45d6a0917`. The candidate binary binding records ownership probe SHA256 `7d86ce4e68af6c861703eaacce35d24d139d486d1ec0124f4a08697686d797c1`. Baseline ownership-harness build source:`d37e3b2d2c418ca425a0ceed5e930caffd5bb771`; its original numerical source was kept separate from the added common harness.

I read the published source diff, the probe source, both actual ownership stdout files, their receipts and the four allocation-case stdout pairs. I did not invoke a native binary or change production source. Both ownership processes exited0. The raw stdout hashes independently recomputed from bytes match the receipts:

* Baseline:`95788acbad5b10e07f6eb76437f332004800f45d3d07765c7910cdd8a4a36068`.
* Candidate:`6912615f48d81adf4017193c5e0eb4a59708567eb53f722f4b9e6f94e48772d2`.

Raw files are under `lane-bench02/ownership-{baseline,candidate}/stdout.txt` in the coordinator's evidence directory. I reparsed all33 rows and compared every row field except `solve_allocations`, `solve_ns` and `drop_ns`. Both row counts are33 and every remaining field is equal. This includes full serialized Solution/Stats/status, every output time/state bit, five dense sample results, event outputs and effectful callback counters, separately for every repeat.

## Allocation attribution

For nonempty dimension n and m observed rows, a cloned observation allocates the time vector, outer state vector, m inner state vectors and nonempty message: **m+3 allocation calls**. The later t/y clone contributes **m+2**. A routed partial clone adds one row allocation, and cloning an appended partial row adds another. The moved implementation removes these sites. Additional capacity-growth changes can in general affect reallocations; exact total savings are therefore workload observations, not a portable API guarantee.

| Probe case | Baseline allocations by repeat | Candidate allocations by repeat | Saved each repeat | Source attribution |
|---|---|---|---:|---|
| dense |318,62,62|309,53,53|9|2m+5 with m=2|
| jvp |94,94,94|82,82,82|12|ownership9 + concurrent Krylov storage reduction3|
| declared-band |78,78,78|69,69,69|9|routed ownership, m=2|
| vector-atol |60,60,60|56,56,56|4|direct route; final t/y clone only|
| max-steps |62,62,62|53,53,53|9|m=1 preappend, plus two partial-row clones|
| fatal |67,67,67|58,58,58|9|m=1 preappend, plus two partial-row clones|
| zero-span |57,57,57|50,50,50|7|routed ownership, m=1|
| dense-event |77,77,77|77,77,77|0|FastDense extension path unchanged|
| event-nonfinite |72,72,72|72,72,72|0|same control with typed event failure|
| fd |68,68,68|59,59,59|9|routed ownership, m=2|
| rhs-reentrant |126,126,126|110,110,110|16|outer9 + nested zero-span7|

The dense case's first repeat has256 additional allocations in both arms. This process-first versus later-repeat distinction is preserved; allocation measurements include whole solve setup/export, not only numerical hot loops. The consistent differences at the exact expected clone sites are stronger attribution evidence than treating every allocation in the solve as avoidable overhead.

The four pre-existing probe workloads separately give:

| Case | Baseline | Candidate | Saved each repeat |
|---|---|---|---:|
| dense-fd |324,68,68|315,59,59|9|
| banded-fd |318,62,62|314,58,58|4|
| time-jvp |386,129,129|358,101,101|28|
| autonomous-jvp |389,133,133|361,105,105|28|

The banded-FD route is direct, explaining its four allocations rather than the routed declared-band case's nine. The JVP totals combine ownership and Krylov changes; attributing all28 to output ownership would be wrong. The three extra savings in the small n=2 JVP ownership probe and the larger19 extra savings in the n=16 existing JVP workloads match elimination of the unused `directions` vector pool by `prepare_basis_only`. For the current Rust Vec growth behavior, constructing2 direction vectors causes2 inner allocations plus1 outer allocation; constructing16 causes16 inner allocations plus3 outer allocation/reallocation calls. The basis itself remains allocated. Kernel-specific numerical equivalence evidence is owned by the Krylov unit.

## Failure, callback and scientific checks

The max-step case retains a failed status and its last accepted time0.060000000000000005. The injected RHS-fatal case retains the accepted state at0.01, failure time0.060000000000000005 and15 RHS calls. Zero-span returns only its initial row and invokes no RHS/derivative callback. The ordinary event case keeps10 event calls; the nonfinite event fails on call7 and truncates output exactly as baseline. The actual nested-solve callback executes once and keeps the same outer32 RHS calls. These observations exercise effectful callbacks rather than assuming purity.

The source comparison confirms that stop metadata is read before partial-row ownership moves, equal-time partial rows remain unappended, dense extensions remain owned and untouched, and routing verification metadata is preserved. The equivalence theorem excludes numerical behavior deliberately tied to allocation addresses, allocator failure/panic observations and future APIs that expose ownership identity. It preserves the current value/callback contract, with all native arithmetic before projection unchanged.

The candidate's facade integration command completed **37 binaries /280 passed /0 failed** (`lane-checks02/facade-contracts`). Focused all-target Clippy passed. Independent reviewer `/root/state_audit` separately accepted the solve.rs source diff with no remaining findings, and re-read all33 ownership pairs plus24 ordinary pairs with exact parity. Its `FINAL_INDEPENDENT_REVIEW.md` is handed to the coordinator separately. Broader lane results remain separately scoped: the Krylov contracts command exited101, so this review does not call the entire workspace green. Existing production gates remain open according to the coordinator's cumulative status.

No ownership-probe wall-time claim is made. Its solve and drop are separately timed with full JSON/dense inspection between; these samples support value/allocation auditing. The paired timing campaign belongs to the coordinator's broader diagnostic and must preserve adverse blocks and its actual environment. Reduced allocation counts do not establish fewer RHS/JVP calls, a general speed ratio, whole-path physical qualification or allocation-free solving.

The clock–fiber theorem in `theory/ADAPTER.md` remains a proved conditional mathematical contract. This native patch does not instantiate outward rate/carrier/defect enclosures or alter numerical acceptance. Local implementation must retain clock defects and use fiber jumps, or transform original-coordinate jumps by `J_fiber≤J_state+G J_clock`.

## Decision

**PASS_SCOPED for consumed-output refinement and observed allocation reduction.** All33 raw ownership pairs pass exact value/callback parity, failure controls are retained and the measured allocation differences match source-specific predictions. PR-N04 remains2/3 after the one coherent ownership episode; no switch-policy change or reset of exhausted R-ADAPT occurred. Full production qualification and the physical certificate implementation remain open.
