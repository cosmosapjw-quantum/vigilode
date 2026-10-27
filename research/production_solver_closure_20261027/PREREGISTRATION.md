# Retrospective research index — prospective registration was missing

**Written on 2026-10-27 after the experiments and the first publication. This
file is not evidence of a pre-run committed preregistration.** The filename is
the repository's required node entry point; the chronology is deliberately
explicit. The original runtime and evidence were published in commit
`78bb8775bf553ad2b514575a0ef4a025446f5d28` on PR92. The scientific baseline was
PR91, commit `206298b4068d923bb47c7160b111b7439098660c`.

The lane protocols existed during the work and their execution/correction
records describe the local order of decisions and runs. They were not committed
to Git before those runs. Adding this file cannot retroactively satisfy the
chronology required by [the research-node rule](../../docs/RESEARCH_LEDGER.md)
or the prospective timing campaign required by
[the timing contract](../../docs/TIMING_DESIGN_CONTRACT.md).

Ledger entry **L-0173** is therefore **INCONCLUSIVE** for prospective research
admission. It indexes existing numerical observations and preserved failures;
it does not turn them into a preregistered PASS. Its commit identifies the
published evidence snapshot. The actual source and executable identity of each
historical run remains in that run's receipt. There was no single prospective
registered aggregate command, which the ledger marks `UNRECORDED`; the
individual executed commands are retained and indexed below.

No old ledger row, checker, exemption, experiment, accuracy cap or runtime code
is changed by this indexing correction. It does not consume or reopen a
numerical correction loop.

## Questions and recorded contracts

The three lanes studied separate costs under the PhysMath theory/coding loop:

| Lane | Question | Recorded scientific contract | Original protocol |
|---|---|---|---|
| Structural mean-field | Can full original-space stage verification and failed-attempt accounting use fewer operations? | `f(y)=b-d*y+kappa*mean(y)`, finite `d>=kappa>=0`, all represented state/forcing components, eight raw-U stages, original residual cap `1e-9`, common-grid physical cap `1e-6`, same-target cap `1e-11` | [structural/PROTOCOL.md](structural/PROTOCOL.md) |
| Scalar cubic/homotopy | Can scalar representation and an algebraically derived root candidate reduce work without weakening the original certificate? | `k=1,g=1/2,Y=T=1`, physical caps `1e-3,1e-4,1e-5`; unchanged dyadic plan and outward original-polynomial/physical checks; linear, zero, nonlinear-dominated and rounding-floor cases | [homotopy/PROTOCOL.md](homotopy/PROTOCOL.md) |
| Adapter/switcher | Can exact same-window cost comparisons and past accepted-step proposals reduce discarded work? | Dimension16, `[0,1]`, `rtol=1e-7`, `atol=1e-10`, `h0=1e-4`, 100000-attempt resource cap, eight windows, cubic physical cap `1e-6` and existing return-family cap `1e-5`; all discarded work charged | [adapter/PROTOCOL.md](adapter/PROTOCOL.md) |

The original alternative controls remain: PR91 and fused direct structural
screens, generic/scalar/algebraic/rational cubic routes, and persistent RHS-only
versus the selective adapter policies. No unscreened structural shortcut or
automatic state Jacobian/Newton fallback was admitted. Model-specific
derivative-free results do not establish a general nonlinear convergence or
order theorem. [Theory and implementation scope](PRODUCTION_SETUP.md),
[structural derivation](structural/THEORY.md), [homotopy derivation](homotopy/THEORY.md),
[adapter derivation](adapter/THEORY.md)

Recorded correction counts are **structural 3/3, homotopy 1/3, adapter 3/3**.
The scalar `n=1` structural diagnostic, rational cubic alternative and additional
adapter arms arose after identified failures or inefficiencies; they are
explicitly post-diagnosis work. The adapter's final lint-equivalence correction
is conservatively counted as its third pass. Registration repair supplies no
fourth pass. [Structural corrections](structural/CORRECTION_LOOPS.json),
[homotopy run record](homotopy/RUN_RECORD.json),
[adapter correction3](adapter/PASS3_RECEIPT.json)

## Historical executed commands and source identities

These are records of commands already run, not commands executed by this
registration correction. `MEASUREMENT_BINARY` below is the built example whose
absolute path and SHA-256 are in the linked receipt. All build/native timing
jobs used Rust1.94.1, locked offline dependencies, the `measurement` profile and
the shared native lock. Timing, instruction counts, callback work and numerical
error are separate quantities. [Environment](evidence/HOST.json)

| Operation | Recorded command or family | Exact argv / source / outputs |
|---|---|---|
| Structural contracts | `cargo test -p vigilode-coupled --profile measurement --locked --offline --test structural_mean_field_contracts --test structural_production_contracts -- --nocapture` | [CORRECTION3_EXECUTION.json](structural/CORRECTION3_EXECUTION.json) |
| Structural build | `cargo build -p vigilode-coupled --profile measurement --locked --offline --example structural_production_bench_20261027` | [CORRECTION3_EXECUTION.json](structural/CORRECTION3_EXECUTION.json) |
| Structural exact-input capture and paired timing | `MEASUREMENT_BINARY oracle`; `MEASUREMENT_BINARY bench N 64 5 WORKERS`, recorded cells `(1,1),(64,1),(2048,1),(2048,2)` | [Execution and binary identity](structural/CORRECTION3_EXECUTION.json), [exact-rational analysis receipt](structural/FINAL_ANALYSIS_RECEIPT.json) |
| Cubic contracts/build | `cargo test -p vigilode-coupled --profile measurement --locked --offline --test rhs_homotopy_contracts --test cubic_solver_production_contracts`; `cargo build -p vigilode-coupled --profile measurement --locked --offline --example cubic_production_bench` | [PASS1_EXECUTION.json](homotopy/PASS1_EXECUTION.json) |
| Cubic audit/timing/analysis | `MEASUREMENT_BINARY`; `MEASUREMENT_BINARY --bench 128`; `python3 research/production_solver_closure_20261027/homotopy/analyze.py` | [Native executions](homotopy/PASS1_EXECUTION.json), [analysis receipt](homotopy/ANALYSIS_RECEIPT.json) |
| Adapter final native contracts | `cargo test --locked --offline --profile measurement -j 2 -p vigilode --lib switching::`; the integration command adds `--test comparator_dominance_contracts --test entry_checkpoint_seed_contracts --test return_sentinel_contracts --test strict_return_work_contracts` | [PASS3_RECEIPT.json](adapter/PASS3_RECEIPT.json), referenced archived contract receipts |
| Adapter final 36-arm correctness audit | `MEASUREMENT_BINARY` for `production_return_bench_20261027` | [Final audit identity](adapter/PASS3_RECEIPT.json), [non-timing equality checks](adapter/FINAL_AUDIT.json) |
| Adapter historical paired timing | `MEASUREMENT_BINARY stiff-cubic pr91 64`, `MEASUREMENT_BINARY stiff-cubic observed-pr91 64`, plus the recorded adverse reentrant arm pairs | [Every executed argv and artifact identity](adapter/PERFORMANCE.json) |
| Instruction profiles | Structural `profile 2048 64 ARM REPETITIONS 1`, repetitions 2/6; cubic `--arm ARM REPETITIONS`, repetitions 20/40; adapter repetitions 4/12 | [Structural run list](structural/instruction_profile/RUNS.json), [cubic run list](homotopy/instruction_profile_r1/RUNS.json), [adapter run list](adapter/PERFORMANCE.json) |
| Final facade/coupled regression | `cargo test --profile measurement --locked --offline -p vigilode -p vigilode-coupled --lib --tests --no-fail-fast -- --test-threads=1` | [Final validation summary](evidence/VALIDATION_SUMMARY.json) |

The adapter's wall-time/profile evidence precedes its final lint-equivalence
change and remains bound to the historical measured binary. Its final 36-arm
correctness audit establishes equality of non-timing outputs, not a fresh
performance measurement. [Identity separation](adapter/EXECUTION.json)

## Observations retained without prospective promotion

- The structural exact-rational oracle checked 21 cases, 168 stage bounds and
  728 original-space components. The final `n=2048`, one-worker enclosure has
  a descriptive paired median ratio 2.063 and Callgrind instruction ratio about
  0.374823 relative to its same-binary PR91 control. Initial scalar slowdown,
  final fused-direct slowdown/noise and two-worker variability remain visible.
  [Exact checks](structural/FINAL_EXACT_ORACLE_RESULTS.json),
  [all final cells](structural/FINAL_BENCHMARK_SUMMARY.json),
  [instruction counts](structural/instruction_profile/SUMMARY.json)
- All 24 final cubic audit arms met their own recorded physical-error checks.
  At cap `1e-5`, the unchanged N512 plan used 1659 polynomial/interval evaluations
  for warm Picard versus 1024 for the rational candidate, with 512 separate
  predictor evaluations. The initially slower radical candidate and ambiguous
  scalar-versus-rational native timing are preserved.
  [Analysis](homotopy/ANALYSIS.json), [report](homotopy/REPORT_KO.md),
  [instruction counts](homotopy/instruction_profile_r1/SUMMARY.json)
- All 36 adapter arms retained their fixed accuracy/accounting checks. Reduced
  callback work in stiff cubic did not establish a stable CPU-time benefit;
  the reentrant selective route used about 42.81% more work and 145.02% more
  guest instructions than RHS-only, and was slower in every recorded paired
  block. The initial inactive policies and branch-coverage/fixture failures are
  retained. [Numerical results](adapter/RESULTS.json),
  [performance](adapter/PERFORMANCE.json), [failure interpretation](adapter/REPORT_KO.md)
- The final facade/coupled gate completed 424 passing tests. The earlier
  five-package invocation timed out after partial progress; its retry recorded
  1312 passes, one failure and 45 ignored tests before the narrow counter repair.
  The repaired route received its own tests; there is no subsequent complete
  five-package PASS claim. The precise scopes and ignored tests remain in the
  [validation summary](evidence/VALIDATION_SUMMARY.json).

These statements distinguish proof obligations, recorded numerical checks and
the project's prospective authority. This index changes none of the preserved
numerical output files. It supplies no timing-admissibility certificate.
**All recorded wall-time ratios remain `diagnostic_only`; no authoritative
wall-time speedup is admitted here.**

## Holdout and future work

There was no separately committed, unopened holdout for this aggregate
campaign. Exact-rational and high-precision reference calculations provide
independent arithmetic/model checks, but they are not a blinded holdout.
Reentrant, strong nonlinear and scalar adverse cases were inspected during
development and must not be relabeled as untouched validation data now.

The next local campaign must commit its question, exact source/build identity,
commands, original accuracy/accounting gate, case corpus and genuinely unopened
holdout before execution. It must preserve all outcomes, use the existing
three-pass maximum per research task, and meet the timing authority contract
before promoting a wall-time claim. The six future PRs and local release gates
remain plans in [DEVELOPMENT_PLAN.json](DEVELOPMENT_PLAN.json); this index does
not execute or preregister their eventual experiments.

## Policy closure boundary

L-0173 hashes the already published numeric lane outputs. Its inputs are the
published protocols and execution records at the cited evidence snapshot, not
this later-created file. The original 169 ledger rows remain an unchanged byte
prefix. A successful repository Python check establishes schema, coverage,
append-only behavior and published-reference validity; it cannot establish the
missing historical chronology. No exception entry or policy weakening is used.
