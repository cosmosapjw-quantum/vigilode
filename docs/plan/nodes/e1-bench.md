# Node e1-bench: matched-accuracy work–precision harness with external comparators

Branch `production/e1-bench` from a5a48f7. Plan §7, wave-2 task 5. New standalone crate
`crates/vigilode-bench` (own `[workspace]` and `Cargo.lock`; the 179 lock entries for diffsol 0.16.1 and its
dependencies are identical to `crates/vigilode-hybrid/Cargo.lock`, only the local crates differ). Research
node `research/bench_wave2_20261021/`; ledger row `L-0139` (INCONCLUSIVE, as preassigned).
Toolchain: `RUSTUP_TOOLCHAIN=stable` (rustc 1.97.0), gfortran 13.3.0, SUNDIALS 6.4.1 (system package),
`CARGO_TARGET_DIR=/home/claude/targets/e1-bench` (1.4 GB), `-j 1`.

Commits: 7665296 (crate and tests), 1e3ca6d (PREREGISTRATION.md + corpus, pushed before the run),
72cb5d0 (results, generated report, run log, ledger row), and this report.

## What was built

- `src/model.rs`: one Rust RHS and Jacobian per problem (Robertson, HIRES, van der Pol mu = 1000, Brusselator
  1D, and a linear problem with exact solution), with layout writers (dense row/column major, LINPACK/Hairer
  band, row-major band, JVP) over one entry list, and process-wide callback counters.
- `src/arms.rs`, `src/ffi.rs`, `native/bridge/`: 13 arms. VigilODE dense and banded (the facade's
  `Jacobian::Dense` and `Jacobian::Banded` routes); Diffsol BDF with `FaerLU` and with `FaerSparseLU`
  (Diffsol has no banded backend); CVODE BDF dense and band through a C bridge (`CV_ONE_STEP` +
  `CVodeGetDky`); DVODE MF 21/24 (ITASK = 1) and Hairer RADAU5/RODAS dense and band (SOLOUT + CONTR5/CONTRO)
  through an ISO_C_BINDING bridge; internal Radau IIA3 tagged `reference-implementation-only`. Callbacks are
  passed as C function pointers with the model as context, so every arm calls the same Rust code.
- `native/`: Netlib DVODE + LINPACK from the pinned PR #71 location (hashes equal `NETLIB_SHA256SUMS`);
  `radau_decsol.f`, `rodas_decsol.f`, `LICENSE_HAIRER` from the Assimulo 3.0 sdist (archive sha256 verified
  `61cdca45…af9e8a`). `SHA256SUMS` is checked by `build.rs` before compiling; `SOURCES.json` records origin and
  hashes. RODAS's duplicate DECSOL symbols are localized in its object with objcopy (no source edit).
- Allocations: a counting `#[global_allocator]` (Rust) and an interposer on `malloc`/`calloc`/`realloc`/
  `posix_memalign`/`aligned_alloc`/`memalign` forwarding to glibc `__libc_*`, exported dynamically so
  SUNDIALS and libgfortran bind to it. The binary refuses to run unless both counters move.
- `src/harness.rs`: references and cross-checks, warm-ups, rotated-order repetition blocks, rows (failed arms
  kept with work), matched-error comparison at common levels (`DIAGNOSTIC_ONLY` unless both arms bracket);
  `src/report.rs`: Markdown from the JSON only.

## Commands and exit codes

| Command (in `crates/vigilode-bench`, env per constraints) | Exit | Result |
| --- | --- | --- |
| `cargo test -j 1 --locked` | 0 | 11 tests: 5 unit (Jacobian vs finite differences, layout agreement, WRMS, interpolation, median), 3 `adapters` (all 13 arms vs exact linear solution on 101 points; VigilODE arms bit-identical to `Rodas5p::solve` endpoint and step counts on 3 problems; failed-arm prefix/work), 2 `report` (every Markdown number appears in the JSON, no row dropped; tokenizer), 1 `sources` |
| mutation check: inject `98765.4321` into the report | test failed as expected | restored |
| `cargo clippy -j 1 --locked --all-targets -- -D warnings` | 0 | no warnings in the crate (4 pre-existing dead-code warnings in `rodas5p-integrators` under this crate's feature set) |
| `cargo fmt --check` | 0 | |
| `cargo build -j 1 --locked --profile measurement` | 0 | binary sha256 `1006bd5c…caaf7b0e` |
| `OPENBLAS_NUM_THREADS=1 vigilode-bench wp --corpus …/corpus_v1.json --out …/results_v1.json` | 0 | 37.0 s |
| `vigilode-bench report --in …/results_v1.json --out …/REPORT.md` | 0 | |
| `python3 tools/check-research-node.py --base origin/production/vigilode-plan-20261021` | 1 | only the pre-existing L-0124 hash mismatches outside this node (as in t-verify); the new node and L-0139 pass |

## Results (status: diagnostic timing, INCONCLUSIVE)

102 rows (none dropped), one failure (DVODE MF 21, van der Pol, rtol 1e-8, ISTATE −1: MXSTEP = 500 per
output interval, its default). All rows bit-deterministic over 21 repetitions. References undisputed: Diffsol
at rtol 1e-12 agrees with VigilODE at 1e-12 to 1.7e-11 (Robertson), 4.2e-11 (HIRES), 1.0e-9 (van der Pol),
3.4e-10 (Brusselator). 80 of 375 matched-error entries bracket on both curves; the rest are
`DIAGNOSTIC_ONLY`. Time ratio comparator/VigilODE at matched `error_common` (above 1: VigilODE faster):

| Comparator | Robertson | HIRES | van der Pol | Brusselator n = 100 |
| --- | --- | --- | --- | --- |
| RADAU5 (dense / band) | 0.57 | 0.81–1.01 | 0.64–0.70 | 0.91–1.35 / 0.48–0.69 |
| RODAS (dense / band) | 0.72–1.12 | 0.83–2.80 | 0.89–1.38 | 0.77–1.17 / 0.59–0.89 |
| DVODE (MF21 / MF24) | 1.50 | 1.36–1.52 | none matched | 0.41–0.59 / 0.40–0.55 |
| Diffsol BDF (dense / sparse) | 2.12–2.28 | 1.58–2.05 | 3.99–4.12 | 1.43–1.44 / 0.71–0.78 |
| CVODE BDF (dense / band) | 4.79 | 5.62–6.08 | 7.59 | 2.71–3.78 / 1.59–2.24 |

Re-measuring the Hairer codes (L-0029 predates the fast drivers): RADAU5 is at or below VigilODE at every
matched level on the three small dense problems, and the banded Hairer arms are below `vigilode-banded` on the
Brusselator; RODAS (dense) crosses above 1 as the level tightens. Full tables:
`research/bench_wave2_20261021/REPORT.md`.

## Deviations from the brief and the plan

1. VigilODE output: the facade at a5a48f7 refuses `dense_output` and returns t0/t1 only, so the VigilODE arms
   call the protected drivers it routes to with its defaults and a 101-point `OutputSchedule`. That schedule
   lands a step on each grid time (clipped steps), not an interpolant; it costs VigilODE extra steps and gives
   it extra accuracy, both charged in the matched comparison. Equivalence to the facade is tested bit for bit
   at the endpoint.
2. atol: the facade and the drivers take a scalar atol, so the corpus atol vectors are uniform; atol is
   `rtol x atol_scale` (prior-art scales from PR #71), not fixed. Both error metrics are recorded:
   `error_own` (plan rule 2) and the tolerance-independent `error_common` used as the matched axis.
3. Reference rule: the brief's (VigilODE at 1e-12, Diffsol cross-check, dispute above 1e-8) was applied. Plan
   §7 rule 1 (RADAU5 and CVODE agreeing to 1e-10) was recorded informationally only; under it HIRES (RADAU5
   2.1e-9) and van der Pol (CVODE 1.5e-9) would be disputed.
4. Diffsol takes the shared Jacobian as a JVP (builder API) and assembles its matrix from n or coloured
   products. CVODE runs in one-step mode with a total 100,000-step cap. Hairer codes start from H = 0.
5. Internal reference arms: only Radau IIA3 was included; the internal BDF was not run.
6. Timing: in-process repetitions in a rotated arm order, not fresh-process Williams-ordered blocks;
   allocations cover the whole arm run, not "after setup". Both are recorded in the JSON.
7. Extra arms not named in the brief: banded RADAU5 and RODAS.
8. Two one-repetition smoke runs preceded the preregistration; three design changes after the first are
   disclosed in PREREGISTRATION.md (scaled atol, common levels, measurement window).

## Concerns

- Product gaps found while wiring the facade: no vector atol, no dense output, a per-call `DenseMatrix`
  allocation in the dense Jacobian route (8.7 to 18.5 MB of heap per Brusselator n = 100 run, against 0.12 MB for the banded route).
- `libblas.so.3` resolves to OpenBLAS (pthread build); the run set `OPENBLAS_NUM_THREADS=1`.
- The internal Radau IIA3 arm's allocation counts vary across repetitions on the Brusselator (outputs do not).
