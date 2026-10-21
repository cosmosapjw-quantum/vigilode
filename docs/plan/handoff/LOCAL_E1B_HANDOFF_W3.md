# Local measurement handoff — E1b-dense (VigilODE dense output against the comparators)

You are a Claude Code session on the repository owner's workstation (Ryzen 9 5900X, 12 cores / 24 threads, 96 GB RAM). A cloud controller session hands you one job: run the E1b-dense corpus once on a pinned commit, with the comparator toolchain below, and push the results back. The cloud sandbox has 2 cores shared with other builds, which is why the measurement runs here.

SHA_UNDER_TEST: to be filled by controller after T3/T7 merge

Do not start until the line above holds a SHA. The controller fills it after `production/e1b-dense` has merged the post-T7 tip of `production/wave3-integration` and the bench crate has been re-tested there (`cargo test --locked` and `cargo clippy --all-targets -- -D warnings` in `crates/vigilode-bench`).

## CODE_TASK

- Outcome: a pushed results branch `results/local-e1b-<sha>` holding `results_v2.json`, `REPORT.md`, `run.log`, `host.json`, `host.txt` and `SUMMARY.json` for the commit under test.
- Purpose: re-run the wave-2 E1 comparator benchmark with VigilODE entering through `Rodas5p::solve` with free stepping and interpolated output (`dense_output = true`, `Solution::at`), so that no VigilODE row is constrained to the output grid. The preregistration is `research/bench_wave3_20261021/PREREGISTRATION.md`; the corpus is `research/bench_wave3_20261021/corpus_v2.json`.
- In scope: build, one corpus run, report generation, host record.
- Out of scope:
  - editing code, the corpus or the preregistration
  - rerunning to improve a number, or changing any flag, repetition count or arm list
  - merging, rebasing or force-pushing
  - pushing to any branch other than the results branch
  - touching `main`
- Approval boundary: stop and report in `SUMMARY.json` rather than work around it. Examples: a toolchain install fails, the comparator sources fail their hash check, the disk runs out, or the run is still going after 2 hours.

## Setup

```bash
git clone https://github.com/cosmosapjw-quantum/vigilode.git vigilode-e1b || (cd vigilode-e1b && git fetch origin)
cd vigilode-e1b
git fetch origin production/e1b-dense
git checkout --detach <SHA_UNDER_TEST>      # verify with git rev-parse HEAD
SHA=$(git rev-parse --short HEAD)
R=$PWD/results/local_e1b_$SHA
mkdir -p $R
```

The repository pins Rust 1.94.1 in `rust-toolchain.toml`; let rustup install it on the first cargo call. If that fails use `RUSTUP_TOOLCHAIN=stable` and record it in `SUMMARY.json`.

### Comparator toolchain (as wave-2 E1 used it; see `research/bench_wave2_20261021/PREREGISTRATION.md` and `docs/plan/nodes/e1-bench.md`)

| Component | Required | Where it comes from |
| --- | --- | --- |
| SUNDIALS CVODE | 6.4.1 | system package `libsundials-dev` (Ubuntu 24.04 ships 6.4.1); `build.rs` links `sundials_cvode`, `nvecserial`, `sunmatrixdense/band`, `sunlinsoldense/band` |
| gfortran | any recent (wave 2: 13.3.0) | `sudo apt install gfortran`; Fortran flags `-O3 -std=legacy` are set by `build.rs` |
| `objcopy` (binutils) | any | localizes RODAS's duplicate DECSOL symbols; no source edit |
| Netlib DVODE + LINPACK (`vode.f`, `dgefa.f`, `dgesl.f`, `dgbfa.f`, `dgbsl.f`) | the frozen copies in `crates/vigilode-bench/native/netlib` | pinned by `native/SHA256SUMS`; `build.rs` runs `sha256sum -c` before compiling |
| Hairer RADAU5 and RODAS (`radau_decsol.f`, `rodas_decsol.f`) | the frozen copies in `native/hairer` (Assimulo 3.0 sdist, archive sha256 `61cdca45…af9e8a`) | same hash check |
| Diffsol | 0.16.1 | `Cargo.lock` of the bench crate (`diffsol = "=0.16.1"`) |
| BLAS | `libblas.so.3` via `libopenblas-dev` (alternatives); DVODE/LINPACK call level-1 BLAS | run with `OPENBLAS_NUM_THREADS=1` |

Record the versions in `$R/host.txt` (commands below). A SUNDIALS version other than 6.4.1 stops the run: report it, do not continue.

```bash
sudo apt-get update
sudo apt-get install -y build-essential gfortran libsundials-dev libopenblas-dev binutils util-linux git curl python3
for p in build-essential gfortran libsundials-dev libopenblas-dev binutils util-linux git curl python3; do dpkg -s $p >/dev/null || { echo "MISSING $p"; exit 1; }; done
command -v rustup >/dev/null || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && . "$HOME/.cargo/env"
dpkg -s libsundials-dev | grep -E '^Version'      # expect 6.4.1
gfortran --version | head -1
```

`crates/vigilode-bench/build.rs` needs, by tool: `gfortran` (Fortran sources, `--version`), `gcc` and `ar` (C bridge and static archive; from build-essential), `nm` and `objcopy` (binutils: symbol scan and RODAS symbol localization), `sha256sum` (coreutils: `native/SHA256SUMS` check), `rustc`. It links `sundials_cvode`, `sundials_nvecserial`, `sundials_sunmatrixdense`, `sundials_sunmatrixband`, `sundials_sunlinsoldense`, `sundials_sunlinsolband` (libsundials-dev), `blas` (`-lblas`, resolved by libopenblas-dev's alternatives to OpenBLAS), `gfortran` and `m`. The run's `git` call needs git, `taskset` is in util-linux. Do not hide apt errors: a missing package must stop the setup here, not surface later as a link error.

## Commands (run in order, from the repo root, each once)

```bash
export CARGO_TARGET_DIR=$PWD/../vigilode-e1b-target CARGO_INCREMENTAL=0
cd crates/vigilode-bench

# 1. build the measurement binary
cargo build -j 12 --locked --profile measurement
BIN=$CARGO_TARGET_DIR/measurement/vigilode-bench
sha256sum $BIN | tee $R/binary.sha256

# 2. the host record (raw text first, then host.json)
{ lscpu; uname -a; rustc -V; gfortran --version; dpkg -s libsundials-dev | grep -E '^Version'; date -Is; } > $R/host.txt
python3 - "$R" "$SHA" <<'PY'
import json, platform, subprocess, sys
R, sha = sys.argv[1], sys.argv[2]
def sh(c):
    return subprocess.run(c, shell=True, capture_output=True, text=True).stdout.strip()
json.dump({
    "schema": "vigilode-bench.host.v1",
    "sha_under_test": sha,
    "timing_authority": "diagnostic_only",
    "timing_note": "no pinned frequency; in-process repetitions in rotated arm order; one pinned core, no other heavy job",
    "pinning": "taskset -c 2; OPENBLAS_NUM_THREADS=1",
    "cpu_model": sh("lscpu | sed -n 's/^Model name: *//p'"),
    "logical_cpus": sh("nproc"),
    "kernel": platform.platform(),
    "rustc": sh("rustc -V"),
    "gfortran": sh("gfortran --version | head -1"),
    "sundials": sh("dpkg -s libsundials-dev | sed -n 's/^Version: //p'"),
    "binary_sha256": sh("cut -d' ' -f1 " + R + "/binary.sha256"),
    "date": sh("date -Is"),
}, open(R + "/host.json", "w"), indent=2)
PY

# 3. the corpus run: pinned to one core, no other heavy job on the host during it
cd ../..
( time OPENBLAS_NUM_THREADS=1 taskset -c 2 $BIN wp --corpus research/bench_wave3_20261021/corpus_v2.json --out $R/results_v2.json 2> $R/run.log ) 2> $R/time.txt; echo "wp $?" >> $R/exit_codes.txt

# 4. the report, generated from the JSON only
$BIN report --in $R/results_v2.json --out $R/REPORT.md; echo "report $?" >> $R/exit_codes.txt
```

`wp` is run from the repo root so that `corpus_file` and `provenance.git_commit` resolve there. If the working tree is dirty, stop: `provenance.git_crate_tree_dirty` must be false.

## SCIENTIFIC_CONTRACT (what counts as correct here)

- The run is one execution of the committed corpus with the committed binary. No flag changes, no reruns to improve numbers, the measurement binary only (`--profile measurement`: `opt-level = 3`, `codegen-units = 1`, no LTO). A failed run is reported as it is.
- Timing is `diagnostic_only`: the host has no pinned frequency and repetitions are in-process. The verdict of the node is INCONCLUSIVE by preregistration. Do not compare ratios with wave-2 v1 ratios (different host); do not write speed claims.
- The dense VigilODE arms must show `output_clipped_steps = 0` in every row. A row with clipped steps, or with a grid time the interpolant does not answer, is a `protocol-violation` row: kept in the JSON, counted in `protocol_violation_rows`, and reported, never repaired by rerunning.
- Failed comparator rows (wave 2 had one: DVODE on van der Pol at rtol 1e-8, ISTATE -1) are rows with their status and work; they are not dropped.
- `error_own` values of the dense arms may exceed one tolerance unit (the smoke run showed about 9 on the Brusselator at rtol 1e-6). That is a finding to report in the summary, not a defect to fix here.

## VALIDATION_MATRIX

| Check | Expected | Source of expectation |
| --- | --- | --- |
| wp exit code (`exit_codes.txt`) | 0 | harness contract |
| report exit code | 0 | harness contract |
| rows in `results_v2.json` | 132 = 3 dense-only problems x 3 rtols x 9 arms + 1 Brusselator x 3 rtols x 17 arms, none dropped | corpus v2: v1's 13 arms plus 4; banded and sparse arms run on the Brusselator only |
| `protocol_violation_rows` | 0 | preregistration: protocol rule |
| `output_clipped_steps` of every `vigilode-dense*` and `vigilode-banded-dense*` row | 0 | task contract |
| references undisputed | all four problems `disputed = false` | wave-2 E1 result |
| `provenance.git_crate_tree_dirty` | false | clean checkout |
| `provenance.sundials_header_version` | 6.4.1 | toolchain table |
| `provenance.profile` | `optimized` | measurement binary |
| `timing_authority` | `diagnostic_only` | preregistration |
| R-C5 gate at SHA_UNDER_TEST: `cargo test --locked` and `cargo clippy --all-targets -- -D warnings` in `crates/vigilode-bench` | exit 0 both (run before the measurement, `-j 12`) | ruling R-C5 |
| `native.ran.control` of dense rows | `WFilteredEnforce` for `vigilode-dense`/`vigilode-banded-dense`, `Off` for the `-off` arms | interior mode reached the facade |
| `binary_sha256` | recorded in `host.json` and `binary.sha256` | provenance |
| every dense arm `driver_id` | recorded (`native.driver_id` per row); list the distinct values in the summary | the shipping dense route at the SHA under test |

## Summary file

Write `$R/SUMMARY.json` with: `sha_under_test`, `toolchain` (output of `rustc -V`), `pinned_toolchain_used` (bool), `sundials_version`, `gfortran_version`, `wall_seconds` (from `time.txt`), `rows`, `protocol_violation_rows`, `failed_rows` (list of problem/arm/rtol/status), `distinct_dense_driver_ids`, `sha256` of `results_v2.json`, `REPORT.md`, `run.log`, `host.json` and `host.txt`, `matrix` (one object per row above, `{check, expected, observed, status}` with status MATCH, DIFF or NOT_RUN) and `notes` (facts separated from interpretation). Never mark a check MATCH if it did not run.

## Return path (results branch, host record, B1a)

```bash
git checkout -b results/local-e1b-$SHA
git add -f $R
git commit -m "results: local E1b-dense run on $SHA"
git push -u origin results/local-e1b-$SHA
```

The pushed branch `results/local-e1b-<sha>` must contain `results_v2.json`, `REPORT.md`, `run.log`, `host.json` and `host.txt` (and `SUMMARY.json`). Then show the user the branch name and the filled matrix. The cloud controller fetches the branch, copies the result files unchanged into `research/bench_wave3_20261021/`, verifies their sha256 against `SUMMARY.json`, and appends ledger row `L-0142` (the controller appends it; you do not touch `research/LEDGER.jsonl`).
