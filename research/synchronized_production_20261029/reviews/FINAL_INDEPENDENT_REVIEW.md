# Independent final evidence review

Reviewer: `/root/state_audit`. Date: 2026-10-29 KST. The reviewer independently read source, raw stdout/stderr and build/command receipts, recomputed hashes, reconstructed sample parity and recalculated every paired timing block. The reviewer did not rerun scientific solver experiments. The machine-readable independent reaggregation is `INDEPENDENT_DATA_REAGGREGATION.json` beside this file.

**Verdict: scoped ownership/storage preservation and allocation reduction are supported on the measured fixtures. A general speedup and production qualification are not supported. Production remains HOLD.** The existing parallel allocation-test failure remains recorded, with a successful same-source serial diagnostic. Initial source/test and build-provenance failures also remain part of the evidence.

## Source and binary provenance

The final native/exact-test source is `ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc`, tree `2989f07fb04134aa7bab38e38d7272e45d6a0917`. The first attempted source was `7fce5a4cbf5e1de3dd25de3b247f638bc551be24`. The final source corrects the independent test operator's required identity delegation and the Decimal stationary test oracle; the native A1/K1 numerical source is unchanged between these two pins.

The ordinary baseline executable is source `c41caffa68d9f4eb9d1976422f7f74ec7e764cca`. An independent `git diff` against PR101 `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087` finds no difference in `crates`, Cargo.lock or Cargo.toml. The ownership baseline is `d37e3b2d2c418ca425a0ceed5e930caffd5bb771`; its only native-tree change from PR101 is the probe example. The baseline and candidate ownership probe Git blob is identically `4067db063bc502dbcb936c9e08039c066e1d42b5`.

Every final executable's SHA256 and linked build-receipt SHA256 was recomputed and matches its BINDINGS.json. Candidate02 receipt `runtime/build_candidate02/RECEIPT.json` records the isolated target, Rust1.94.1, unchanged before/after source hash inventories, locked/offline build and exit0. Cargo.lock SHA256 is `efb379071fa42c2b6bdbe969163ac2f41cf318c262c5c67d0910450298333351`. The isolated target had already compiled the unchanged native library before the candidate02 no-op rebuild. Source identity is therefore supported by the build chain, not merely the short rebuild time.

| Executable | Arm | SHA256 |
|---|---|---|
| rust_allocation_probe | baseline | `8161613153b1442fcfef94656cc7b4f24544171e4a10fdfe99d62d1bcec8856f` |
| rust_allocation_probe | candidate | `bc9dabfd20171da12c63324182990b7db4eedacbec8532877d58459f355c4e18` |
| ownership_projection_probe | baseline | `51f4afe78bbe82d1846b739db42b2a6687e1a16d2fc2e28c409026ff46b1d00b` |
| ownership_projection_probe | candidate | `7d86ce4e68af6c861703eaacce35d24d139d486d1ec0124f4a08697686d797c1` |
| production_from_json | candidate | `0883aa2b3a6a98a067b73c36d96d1253ff841e903406fe4daca0dd5c844fbcda` |

The previously detected shared-target stale executables were not used as benchmark authority. Their failed provenance must remain archived; changing a binding alone would not have sufficed. The final measured candidate differs from baseline and has isolated-target build evidence.

## Checks and preserved failures

All raw stream hashes in both checks runs, the serial diagnostic, benchmark and deployment runs match their receipts. In final checks02:

| Target group | Result |
|---|---|
| Exact Python research tests |25 passed |
| Three native library suites |99 passed,1 ignored |
|37 facade integration binaries |280 passed |
|29 Krylov integration binaries |115 passed,1 failed |
| Time-FD contract |2 passed |
| Focused facade/Krylov Clippy |exit0 |
| Time-FD Clippy |exit0 |

The three new K1 integration tests passed, as did the private NaN-poisoned augmented-reuse library test. The Givens production differential contract passed in this measurement-profile run. This does not erase its separately observed PR101 CI-profile failure or certify all build profiles.

The single final checks02 failure was `sp02_workspace_charge::lgmres_into_workspace_charge_contract`: charged20288 minus released17984 equalled2304, above640. Static inspection shows that three tests concurrently read process-global LIVE_BYTES around workspace drop, without mutual exclusion. Allocations from another test between those two reads can reduce the apparent release. The parent reran the same source/target with `--test-threads=1`; all three charge tests passed in0.164976829seconds. No tolerance or charge formula changed. This supports the specific test-isolation diagnosis; the original parallel result remains a failure and the test harness itself remains unsafe for overlapping measurements until isolated by its invocation or implementation.

Initial checks01 are retained: the exact diagnostic failed at a stationary comparison because Decimal80 rounded a sum one ulp above the exact Fraction bound; the new test operator initially failed to compile because its required token method was absent. Episode2 uses exact Fraction equality at equilibrium, exact subtraction before Decimal conversion elsewhere, and negative controls with2^-300 underbounds invisible to Decimal80. No physical acceptance epsilon or helper formula was relaxed.

## Independent paired observation audit

The reviewer compared raw fixture metadata, callback counts, complete returned sample fields except allocation counts, and finite success/error conditions. All four cases times two modes times three repeats give **24 matching ordinary sample pairs**. All error diagnostics are finite and below the unchanged1e-5 cap. These returned-output diagnostics do not establish every accepted/interior trajectory point or a directed reference enclosure.

The ownership probe yields **33 matching pairs**, eleven cases times three repeats. Full serialized solutions, explicit time/state bits, dense samples and callback counts match. The step-ceiling, fatal-RHS and nonfinite-event cases remain the expected failures, rather than becoming successful outcomes. The nested-RHS case records exactly one nested facade call per repeat. Dense-event and nonfinite-event allocation controls are unchanged, consistent with their different output path.

Allocation observations are process-cold first calls followed by two warm calls in one process. They must not be collapsed into an assumption of equal repetitions:

| Fixture | Baseline cold / warm / warm | Candidate cold / warm / warm | Savings each repeat |
|---|---|---|---:|
| Dense FD |324 /68 /68 |315 /59 /59 |9 |
| Banded FD |318 /62 /62 |314 /58 /58 |4 |
| Time JVP |386 /129 /129 |358 /101 /101 |28 |
| Autonomous JVP |389 /133 /133 |361 /105 /105 |28 |

Ownership-only fixtures also agree with the intended path distinctions: dense318→309 cold,62→53 warm; JVP94→82; declared band78→69; vector-atol60→56; ceiling62→53; fatal67→58; zero span57→50; FD68→59; nested RHS126→110. Dense-event77→77 and nonfinite-event72→72 are useful unchanged controls. These are whole-facade allocation observations, not all allocations uniquely attributed to one Rust instruction.

Maximum endpoint errors are DenseFD6.944889108240204e-12, BandedFD7.354006292814574e-12, TimeJVP4.4617096683907675e-9 and AutonomousJVP2.0572032966015286e-10.

## Paired timing audit

For each of28 blocks the reviewer reread all four raw child payloads, required16 successful samples per child, took each child's median solve-plus-drop nanoseconds, and divided the sum of two baseline medians by the sum of two candidate medians. All1792 timed solve samples are retained; no block was selected or discarded.

| Fixture | Median baseline/candidate | Min | Max |
|---|---:|---:|---:|
| Dense FD |0.9654923705 |0.9389349166 |1.0368164467 |
| Banded FD |0.9641515744 |0.8900926845 |0.9801250231 |
| Time JVP |1.0148618184 |0.9822436256 |1.1185942615 |
| Autonomous JVP |1.0104865172 |0.9779192805 |1.0352120099 |

A ratio below1 means the candidate took longer in this diagnostic. In particular, all seven banded blocks are below1. Memory reduction therefore does not warrant a generic speedup claim. These are very short fixtures in one environment; ranges are not confidence intervals. A local longer matched-accuracy campaign should preserve the slower dense/banded cases as investigation targets.

## Single configuration invocation

Both `production_from_json` runs used the same generated selective-JVP configuration and the candidate executable binding. At rate1 and1000, both report success, accounting_complete and actual callback accounting match. The reviewer independently summed all segment RHS/JVP/time/Jacobian/linear-solve/attempt/accepted/rejected counts and obtained the reported totals. Independent binary64 exponential evaluation at every returned row reproduces each reported maximum error:3.3718316472253207e-9 and1.0654127077097675e-9, below the existing1e-6 diagnostic cap.

Rate1 has8 segments and0 switches; rate1000 has11 segments and1 switch. These manufactured invocation checks establish concrete configuration/accounting behavior. They do not close the physical-client, native whole-slab-provider, platform, API, full interaction or authority gates.

## Budget and receipt identities

Total recorded scientific phase wall, including the failed first checks and serial diagnostic, is **491.539190153seconds**, below the900second campaign ceiling. This deliberately charges the embedded build time appearing inside the check phases; separately recorded compile preparation is not silently added as new solver evidence. The reviewer found and reported an analysis-only risk of omitting earlier phases and a false assertion that cold/warm allocation counts should be identical. Those postprocessing corrections belong to the parent episode3; raw data must remain unchanged. The maximum three correction episodes is exhausted when the parent closes episode3, without resetting inherited counters.

| Original receipt path under `/workspace/scratch/f9615df1ce94` | SHA256 |
|---|---|
| lane-checks01/RUN.json | `df05ccc45316e0a4d3763ed8318eb0b533e00e6b34c2fb3aba53a3ff87886a67` |
| lane-checks02/RUN.json | `70004fdf605436b740ab5f7796799b4e1977aa48d805bde487ef0da5b93d75ff` |
| lane-charge03/RUN.json | `cc25bb93a933e5577442e826bedd2975b5133625a1b2958f7ccce19d55375854` |
| lane-bench02/RUN.json | `22606ebfb658388b1023fa59f58cce2e9aea131d73807f3dd9254a8c97c247e1` |
| lane-deployment02/RUN.json | `2337f9d53c451428b0c5962b5ba1773c5e2d6be91b82b07aa038477efdaa427e` |

The final runner includes the requested production executable source/digest guard; that formerly pending static-review note is closed. The synthesis monotone block norm and exact-field epsilon ownership clarifications, fixed-RHS recycle identity, later-slab type guard and absent quotient accuracy-cap refusal state were all read back. The native implementation is a narrowly scoped identity transformation; the constructive numerical theorems remain conditional research contracts until their source/domain/norm/action/rounding providers are implemented and validated.
