# Source-bound dynamics and symbolic execution validation

Research identifier: 20261028. Actual UTC and Asia/Seoul execution timestamps
are recorded in receipts. User delivery e935878fefe795a1b7e88aba64e5e30ef7be1578
was fast-forwarded in a fresh validation worktree to PR97
87465835d9d21298d235917f036580b0b53ab645. Observed main is
8d0c79184e09efb5bdadc24a6315c60a71a44264, an ancestor. PR95 remains a separate
conditional chemical-model input, not merged. All PR90–97 metadata is saved.

## Theory admission before coding

Independent agents inspected the attached RVJ/NAG equations and latest source
and used SciSpace/Exa primary literature and Wolfram calculations. The parent
admitted the source-connected theory and code contracts before dispatching
implementation. The dynamics R1–R10 document was complete at that checkpoint;
the supplementary attachment/literature report was consolidated during coding.
See THEORY_ADMISSION.md, theory/DYNAMICS.md, theory/ATTACHMENT.md and the initial
read-only PR97 audit. Existing signed logarithmic-norm and two-envelope
contractive bounds are reused: a proposed weaker bound is not implemented.

Two bounded changes are admitted. A support-only quartic trace computes only
nonzero requested moments, avoiding irrelevant overflow while preserving
all-complex algebraic multiplicity, full input validation and outward bounds.
A coupled switching controller transfers ownership of its prepared snapshot
instead of copying it again. The guarded-word refinement proof preserves
future routing and the physical numerical trajectory; copied-coordinate
telemetry records the operation actually removed. This is a symbolic model
of the controller, not a physical Markov partition or conjugacy theorem.

## Numerical and performance contracts

Trace target: Tr(sum c_k q1^k), degree<=6, original monic quartic
z^4-2az^2+z+a^2-b. Same represented parameters/coefficient boxes and target
on both arms; selected-real and real-only substitution remains refused.
Compare a faithful frozen eager evaluator with the changed public function
in the same native binary. Preserve huge-parameter cases where only the
old arm refuses. A refused arm is not a timing speedup datum. Exact companion
and explicit multiple/complex-root checks provide independent references.

Controller target: identical legal prepare/observe words, full state bits,
epochs, step sizes and physical outcome classes. The old observe body is a
test-only frozen reference. Check state and decision equality after every
letter; counters intentionally differ by one dimension per promoted snapshot.
Benchmark complete prepare+observe calls, including both buffer lifecycles,
at dimensions2,32,4096,65536, with candidate-only and unguarded controls.

Seven paired interleaved/rotated timing blocks, one warmup, raw samples and
operation counts are retained. This is a scoped component experiment;
no generic ODE or whole certified-trajectory speedup follows. Pair ratios,
spread and regressions are reported without deleting unfavorable samples.

## Current execution contract

Use Rust1.94.1, unchanged Cargo.lock, fresh target, cached locked/offline
registry, measurement profile, CARGO_BUILD_JOBS=2 and single Rayon/BLAS/OMP
thread. All native builds/tests/benchmarks share the inherited file lock.
Environment/format/compile preparation is not counted as scientific tests.
The actual implementation and protocol are remotely published and read back
before scientific execution. Use code/run_scoped.py with the actual source
and remote branch. Receipts retain source/input hashes, stdout/stderr, process
exit/timeout, compiler/host/profile and timing. Output directories are new.

Execute coupled and facade library contracts; trace campaign; guarded-word
contracts and its ignored opt-in timing campaign; targeted current-profile
Givens exact exported-bit oracle; coupled release contracts; scoped all-targets
and docs. Each command is bounded (900s tests/build,300s Givens,180s campaign,
600s release,300s docs/check). Independent failures do not erase later evidence.
No zero-selected test count is accepted as test PASS. A missing checkout
fixture is an input-dependency error; preserve it and restore the exact blob.

Givens history: keep existing Ok/Err pins. Export exact A,b,x and threshold
bits; the existing Fraction oracle checks every returned solution. If a
historical pin differs but exact residual passes, report these separately;
do not silently relax the pin or infer convergence from status alone.

## Existing obligations and correction counters

Preserve the original23-node/34-edge DAG,8 suites,73 paths,10 interactions,
negative evidence and original-source replay semantics. Existing counts:
R-STRUCT3/3,R-ADAPT3/3,R-HOMO1/3,I-PROD1/3,Q-LEDGER1/3,N03=2/3,N05=1/3.
The identified lazy-trace correction consumes N05's next coherent episode,
bringing it to2/3. Coupled ownership reuse extends existing N04's explicit
owned-workspace proof obligation (initial implementation0/3, current1/3 after
one coherent prepublication review correction), not exhausted
facade R-ADAPT. No new allowance is created. Review-driven repairs are charged
to their controlling task and retained. Broad production remains HOLD until
mandatory route/client gates pass; full N04/N05/N06 and all interactions are
not closed by these bounded components.

## Results

Pending at source publication. Results are appended in REPORT_KO.md and
RESULTS.json after execution; this original protocol remains preserved.
