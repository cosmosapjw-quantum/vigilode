# Independent review of native results

Reviewer: /root/attachment_theory. Run: native-run01. Source commit 58ec9b053163da6b6f0906010e0422c104db72fd; source tree b859c2d34128145df5fd931e456922554a83b4d5. This is a read-only interpretation of existing receipts and raw outputs. The reviewer did not launch a new native test or benchmark.

## Decision

Accept the measured trace improvement for this fixed degree-six quotient-query workload and its exact-singleton accuracy contract. Accept the controller's exact reduction in copied coordinates and semantic parity. Controller wall-clock measurements remain descriptive and include regressions; they do not establish a general speedup. Current-profile Givens validation passes without closing historical CI or broad production obligations.

## Evidence identity and completed checks

Read RUN.json; completed process receipts and stdout/stderr; trace-support/stdout.log; switch-timing.json; and all three Givens oracle JSON files with exported bit files.

For coupled-lib, facade-lib, givens-exact, trace-support, switch-timing, coupled-release and all-targets-scoped, each receipt records exit 0, no timeout and unchanged source during the command. The reviewer recalculated every recorded stdout.log, stderr.log and SOURCE_INPUTS.json hash for these seven completed commands: all 21 match. At this review read, docs-scoped was still running; it is excluded from the completed count.

Observed executable test summaries:
- coupled-lib measurement: 39 passed, 1 intentionally ignored benchmark.
- facade-lib measurement: 31 passed.
- Givens exact contracts: 2 passed.
- explicit opt-in switching benchmark selection: 1 passed, 39 filtered out.
- coupled-lib release: the same 39 test methods passed, 1 ignored.
- scoped all-targets compilation: exit 0, a compile check rather than a test-count result.

Release rerunning the 39 coupled methods is distinct execution evidence, not 39 newly independent scientific cases.

## Trace correctness and matched accuracy

The recorded target remains the sum of polynomial observables over all four complex roots counted with algebraic multiplicity. The five workload families use identical represented state and coefficient bits for both arms.

All 80 validation inputs have two accepted arms, with singleton_equals_exact=true for both. This verifies the prospective matched-accuracy gate: each returned interval is exactly the independent companion-matrix trace singleton. There is no comparison between a loose fast interval and a tighter slow one.

All measured blocks and warmups satisfy calls=requests_attempted=requests_completed, with requests_rejected=failures=0. Counts are:
- 286,720 measured query calls across 5 families, 7 repeats, 4 blocks per repeat, 2,048 calls per block.
- 20,480 warmup calls across 5 families and 2 arms.
- 160 correctness calls for the 80 paired validation inputs, separate from timed calls.
- 8 calls for 4 large-finite recovery inputs, also separate.

The four huge-parameter cases each record an old eager refusal and a new exact singleton result. They are finite-domain recovery demonstrations, not speedup data. Their refusals never enter the timing denominator.

### Timing interpretation

For each repeat, take the mean of the two eager block times divided by the mean of the two support block times. Report the median and full observed range of those seven paired ratios. Ratios above one favor requested-support evaluation.

| Family | Paired ratio median | Observed range | Interval multiply/add operations per query, old -> new |
|---|---:|---:|---|
| square | 2.900454 | 2.693507–3.250170 | 11/6 -> 2/0 |
| cubic_padded | 3.977076 | 3.789920–4.118178 | 15/10 -> 1/0 |
| fifth_padded | 3.273568 | 3.160236–3.983031 | 15/10 -> 2/0 |
| dense_six | 1.075453 | 0.989594–1.164708 | 15/10 -> 14/8 |
| mixed_six | 1.294637 | 1.182893–1.361144 | 15/10 -> 10/5 |

Sparse queries show the intended substantial savings on this host. Dense-six has a small median improvement and at least one unfavorable pair. Its regression sample must remain in the report. These are complete query-call times, including common input construction, validation, counters and output use. They are not end-to-end ODE integration or certified-trajectory speedups. The frozen eager and exported candidate compilation boundaries are disclosed by the campaign metadata.

No statistical confidence interval or hardware-general claim is inferred from seven repeats. The table uses paired ratios, not a ratio of independently summarized medians.

## Controller data

There are 84 paired rows: four dimensions, three scenarios and seven repetitions. Every row has routing_and_snapshot_bit_parity=true. The reviewer checked equal inspected-coordinate counts and exact equality between recorded saved copies and old-minus-new copied coordinates in every row.

For guarded protected updates, copy counts fall from twice the dimension per call to once the dimension per call:

| Dimension | Iterations per arm | Old -> new copied coordinates | Paired time-ratio median | Observed range |
|---:|---:|---:|---:|---:|
| 2 | 10,000 | 40,000 -> 20,000 | 0.980707 | 0.662865–1.143371 |
| 32 | 5,000 | 320,000 -> 160,000 | 1.040656 | 1.006736–1.160714 |
| 4,096 | 256 | 2,097,152 -> 1,048,576 | 1.050124 | 0.907395–1.198389 |
| 65,536 | 32 | 4,194,304 -> 2,097,152 | 1.126628 | 0.915714–1.250749 |

The n=2 median regresses. Ranges cross one at n=2, 4096 and 65536. The exact work saving is established independently of this variability; the samples do not establish a uniform wall-clock speedup.

The controls remain important:
- Candidate-only updates save only the initial reference-promotion copy; median ratios at n=2,32,4096,65536 are 0.978045, 0.998587, 1.017622 and 1.046322.
- Unguarded updates save zero coordinates. Corresponding median ratios are 1.016223, 0.993142, 0.988838 and 0.912383.
- In particular the unguarded n=65536 median is an unfavorable 0.912383 despite zero algorithmic work change. It demonstrates measurement/code-path variability that must accompany any favorable controller timing.

Each measured arm includes prepare+observe, initial buffer allocation and routing-word collection. These are synthetic controller receipt sequences, with no RHS evaluation or ODE solution. There are 24 separately excluded warmup runs. Byte-for-byte reference parity is checked inside the benchmark; full words are not redundantly persisted, and the separate exhaustive finite-word unit test supplies repeatable semantic coverage.

Avoid calling all 2*6^5 enumerations unique legal words: terminal symbols truncate words and repeated suffix encodings revisit the same prefix. The source accurately describes enumeration of encoded words, with terminal early exit.

## Givens exact-residual results

The reviewer verified every exported bit-file hash and the fixture hash against the three oracle receipts. Each scale has one returned solution that passes exact rational squared-residual comparison against both the exact requested threshold and the represented binary64 solver threshold. Every deliberately corrupted solution is rejected under both thresholds.

| Scale | Production GMRES | Givens candidate | Returned residual / represented threshold |
|---|---|---|---|
| 1e-14 | returned, 3,200 iterations | exhausted 4,000 vectors | about 7.61189e-25 / 1e-24 |
| 1 | exhausted 4,000 vectors | returned, 3,739 iterations | about 9.72380e-11 / 1e-10 |
| 1e14 | returned, 3,440 iterations | exhausted 4,000 vectors | about 9,426.43 / 10,000 |

Thus three returned solutions are independently checked; three failures remain explicit NO_RETURNED_SOLUTION rows. A failure row is not a convergence success. The exact oracle certifies the original represented linear-system residual, not global ODE accuracy or an unexecuted historical environment.

The native outcome-pinning test passed with unchanged expected outcomes in the current measurement profile. Historical CI/profile discrepancies, Q-LEDGER and production qualification remain separate open records. This successful run does not retroactively close them.

## Final scope

No false speedup from a refused eager baseline was found. Every timed trace call completed under the matched exact-singleton target. No hidden controller-state change was found in the recorded parity data. The defensible summary is: sparse trace queries became materially cheaper on this workload; guarded reference promotion halves its copying work; controller timing is mixed; current-profile Givens successes satisfy exact original residual thresholds. Full production readiness remains outside these component results.

