# Task 5 review: E1-bench (a5a48f7..76032cd)

Reviewed in passes from the review package: crate code, bridges, tests, corpus, preregistration and generated REPORT.md. Vendored sources, Cargo.lock and the raw results JSON were checked by command, not read line by line. Read-only; nothing in the checkout was changed.

### Spec Compliance

- ✅ Spec compliant. The deviations are all disclosed in the node report, PREREGISTRATION.md and the per-arm `deviations`:
  - the VigilODE arms call the protected drivers, because the facade has no dense output or vector atol;
  - atol is a uniform scalar;
  - only the internal Radau IIA3 reference arm was included, so the brief's internal BDF arm is missing (disclosed as deviation 5);
  - repetitions run in-process.
- ⚠️ Cannot verify from the diff: that 1e3ca6d was *pushed* before the run. Commit order and the results provenance (`git_commit` = 1e3ca6d, `git_crate_tree_dirty` = false) are consistent with it. Push time is only visible on the remote.

### Strengths

- One `Model` (model.rs:18089-18280) feeds every arm.
  - The Fortran and C bridges take it as C function pointers (ffi.rs:16863-16894, bench_bridge.f90:2143-2169, cvode_bridge.c:2359-2386).
  - The band layouts are correct, including the CVODE `smu - mu` shift (cvode_bridge.c:2377-2385).
  - `adapters.rs` asserts that every arm moved the shared RHS and Jacobian counters.
- Error is computed one way for every arm (harness.rs:17378-17400): `max_wrms` against the same reference states on the same `c.grid(p)` grid.
- Bracketing is strict. `interpolate_log` returns None outside [min, max] (metrics.rs:17906), and ratios are set only for `MATCHED` (harness.rs:17516).
- Failed arms stay as rows with their work (102 rows, 1 failure).
- The allocation interposer works:
  - the C/Fortran heap is counted (e.g. Brusselator cvode-bdf-dense: 200 heap calls vs 105 Rust; radau5: 109 vs 105);
  - the comparators' Rust-side buffers fall inside the measured window (arms.rs:16124-16126).
- The binary refuses to run if the counters do not move (main.rs:17685).

### Checks run (one per named risk)

1. **Same equation, atol and grid in every arm.**
   - The shared callbacks and band layouts were checked through the FFI as listed under Strengths.
   - All arms get the same `rtol` and `atol = rtol*atol_scale[0]`, and Diffsol gets `vec![atol; n]` (arms.rs:16464).
   - Step landing is disclosed only qualitatively: see the Important issue.
2. **Error measured the same way.** Confirmed at harness.rs:17378-17400.
3. **Bracketing.** A script re-checked all 80 MATCHED entries against both curves' successful `error_common` range: 0 out of bracket.
4. **Allocations.** The interposer and global allocator results are confirmed (see Strengths). What is *not* counted is not stated (Minor).
5. **Report numbers.** I regenerated REPORT.md from the committed results_v1.json with the measurement binary: byte-identical. An independent Python token check of the committed REPORT.md against the JSON found 0 of 2278 numeric tokens missing.
6. **Preregistration order.**
   - Commits: 7665296 (code) < 1e3ca6d (preregistration + corpus) < 72cb5d0 (results).
   - Between 1e3ca6d and 72cb5d0, `git diff` shows no change to corpus_v1.json or crates/, and PREREGISTRATION.md has only appended lines.
   - The corpus sha256 is b6ed01af… and Cargo.lock is d52dbdd0…, both as preregistered.
   - The smoke-run changes predate the preregistration and are disclosed.
- **Vendored sources.**
  - `sha256sum -c SHA256SUMS`: all OK.
  - The Netlib files equal commit 64b9852's `refined_runs/39261895983/sources`.
  - The Assimulo-3.0 sdist downloaded from the origin URL has sha256 61cdca45…af9e8a, and its three Hairer files match SOURCES.json.
  - The 174 registry lock entries are identical to vigilode-hybrid's.

### Issues

#### Critical
None.

#### Important
- **Step-landing penalty is disclosed in kind, not in size.**
  - Where: arms.rs:16326-16335 (OutputSchedule) and REPORT.md:583/590.
  - What is wrong: VigilODE is forced to land a step on every grid time, so its step size can never exceed the grid spacing. The results show `output_clipped_steps` = 99 of 111 accepted steps (Robertson, rtol 1e-4) and 99 of 105 (Brusselator, 1e-4).
  - Why it matters: every time ratio carries a large, solver-irrelevant cost on the VigilODE side, plus a smaller accuracy gain from having no interpolation error. REPORT.md shows only the qualitative note, not the counts. The node report's sentence "RADAU5 is at or below VigilODE at every matched level" reads as a solver comparison.
  - Fix: add an `output_clipped_steps` column to the VigilODE rows of REPORT.md (it is already in the JSON `native`). Add one sentence in the matched-error section saying the VigilODE side is grid-step-constrained. Soften the node-report conclusion accordingly.

#### Minor
- The allocation statement in harness.rs:17446-17451 says "every … call of the process". It does not say what is uncounted: stack/automatic Fortran arrays, direct mmap (e.g. OpenBLAS buffers), and `valloc`/`pvalloc`/`reallocarray`. The window also includes setup, against plan rule 3; that is disclosed.
- tests/report.rs:18921 only checks set membership on a freshly run tiny corpus. A misplaced number, or a literal that also happens to occur in the JSON, would pass. No test runs over the committed REPORT.md/results_v1.json pair (my manual check passed).
- The headline HIRES rodas ratio 2.80 is MATCHED at level 1e-8. That is about 5× the informational RADAU5 disagreement with the reference (2.1e-9). It is admissible under the brief's decisive Diffsol rule, but worth a flag.
- arms.rs:16345-16356 and 16362-16372 contain verbatim duplicate `SolverFailure` RunOutput blocks; a small helper would remove them.
- Preregistration change (a), atol scaled by rtol, flipped the van der Pol dispute outcome between the two smoke runs. It is honestly disclosed, but it is a design change made after an outcome was observed.

### Assessment

**Task quality:** Needs fixes.

**Reasoning:** The harness mechanics are sound and verified end to end: shared model, uniform error metric, strict bracketing, live allocation counters, JSON-only report, preregistration order and source pins. The one gap is that REPORT.md and the node report do not quantify the grid-landing cost that dominates VigilODE's step count, so the ratios can be misread.
