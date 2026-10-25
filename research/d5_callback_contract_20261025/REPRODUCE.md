# D5 exact execution record and continuation rule

The completed D2/D3/D4 campaigns are reused. Do not run their tests, trajectories,
benchmarks, NCP or V3 harmonic campaigns as a prerequisite for this unit.
Use EXECUTION_INPUTS.json, REUSED_SOURCE_PINS.json and the actual containing S1
commit in delivery/S1_PUBLICATION.json to identify the new code.

## Toolchain and dependency boundary

Rust1.94.1, commit e408947bfd200af42db322daf0fadfe7e26d3bd1,
x86_64-unknown-linux-gnu, LLVM21.1.8. Supplied archive SHA256:
294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40.
The supplied archive is read from project_sources/ locally, not through Library.

After observed workspace pruning, the first restore stopped at duplicated
component manifest.in. That exclusive-write failure and a subsequent loader
probe failure are retained. resume_toolchain.py completed only bin/lib payloads,
verified73 existing runtime files and added11. It skipped21 component metadata
entries. All failed and successful timings are in the new D5 budget.

The inherited fixture is unchanged, including its read-only MXCSR instruction.
New gateway/tests/probes are compiled with forbid unsafe. The old D4 expected
failure when applying that lint directly to the inherited fixture is reused as
historical boundary evidence; it is not executed again. This is not an
unsafe-free dependency-tree claim and not a production FP-provider approval.

## Actual command plan (cwd = this directory)

The runner appends measured children, refuses duplicate execution names and a
closed budget, and bounds each child by the remaining D5 budget. Options must
precede the execution name because the child argv is argparse.REMAINDER.
The stored BUDGET.json records which commands actually ran and their outcomes.

```sh
python3 run.py --kind build fixture_compile -- toolchain/bin/rustc --edition=2021 --crate-type=rlib source/fixture.rs -o bin/libd3_fixture.rlib
python3 run.py --kind build safe_gateway_compile -- toolchain/bin/rustc --edition=2021 --crate-name=d5_gateway --crate-type=lib -F unsafe-code source/gateway.rs -o bin/libd5_gateway.rlib
python3 run.py --kind build gateway_tests_compile -- toolchain/bin/rustc --edition=2021 --test -F unsafe-code source/tests.rs --extern d3_fixture=bin/libd3_fixture.rlib -o bin/gateway_tests
python3 run.py --kind build session_tests_compile -- toolchain/bin/rustc --edition=2021 --test -F unsafe-code source/session_tests.rs --extern d3_fixture=bin/libd3_fixture.rlib -o bin/session_tests
python3 run.py --kind build new_probes_compile -- toolchain/bin/rustc --edition=2021 -O -F unsafe-code source/probes.rs --extern d3_fixture=bin/libd3_fixture.rlib -o bin/probes
python3 run.py --kind new_contract_tests new_gateway_contracts -- bin/gateway_tests --test-threads=1
python3 run.py --kind new_contract_tests new_session_composition -- bin/session_tests --test-threads=1
python3 run.py --kind new_manufactured_points frozen_new_points -- bin/probes evidence/points.jsonl
python3 run.py --kind saved_output_review exact_saved_point_oracle -- python3 review_points.py
python3 run.py --kind saved_output_figure saved_point_figures -- python3 plot_saved.py
```

These commands are an exact record/plan, not permission to reopen a closed
budget or repeat a completed run. A future fresh execution requires a concrete
changed obligation and its own scoped ledger, preserving prior evidence.

## Inputs and acceptance

All new model inputs and expected row counts are frozen in PREREGISTRATION.md.
The Rust point corpus is synthetic, two-dimensional and nondimensional. It
contains729 points,49 time stencils,14 refusals and one actual finite-wrong-J
control, plus summary. No new ODE trajectory is integrated. Five Session tests
exercise the existing admission object; their synthetic accepted-prefix setup
does not validate an ODE solution.

The independent Fraction oracle reads saved JSONL only. The plotter reads the
same saved result and oracle without invoking callbacks. The three mutant
interpretations are analyses of the saved true-J rows, not compiled Rust mutants.
The separate actual J+I callback control demonstrates why pointwise interface
admission is insufficient to certify derivative correctness.

Symbolic input is evidence/DERIVATION.wl. WOLFRAM_RESULT.json is the actual
connector output; WOLFRAM_CHECK.json is its parsed12-group result. This check
was executed once. Source hand algebra and the independent review are separate
from that tool execution.

## Publication, preservation and limits

Each immutable phase archive includes a complete manifest and passes local
payload SHA256/size and ZIP CRC verification. Provider ACK, Git identity,
whole remote archive restore, scientific checks and production admission are
distinct. Remote byte restoration of the new D5 archives is NOT_RUN unless a
later actual receipt explicitly changes that field.

Existing D2/D3/D4 budgets, original archives/failure logs and branch heads are
preserved. Production HOLD; global completion=null. The next thread must begin
with current native source/reservation/live dirty/completion receipts, reuse
existing hooks, and preserve the separate actual BASS, general model derivative,
dense-domain, nonreturning-callback, durable-crash and production FP gates.

## Mandatory complete coverage companion

The original numerical oracle uses zip. Its PASS must be combined with
SCHEMA_REVIEW.json and REVIEW_BUNDLE.json, never consumed as a standalone
complete-field claim for arbitrary malformed data. Actual additional command:

```sh
python3 run.py --kind saved_output_schema additive_saved_schema -- python3 validate_saved_schema.py
```

This was executed once after C9 found the specific short-array risk, over the
identical saved794-row corpus. It made zero model/callback evaluations and did
not replay the original oracle. Future authorized fresh pipelines must validate
complete schema/coverage before accepting the numerical oracle result.
