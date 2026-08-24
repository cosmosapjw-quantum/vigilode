# E4 Fresh-Clone / Build Reproducibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the repository-external default Cargo dependency and provide independently verifiable normal and air-gapped build modes.

**Architecture:** The default checkout uses Cargo's locked crates.io graph. Air-gapped builds opt into a validated caller-supplied Cargo vendor directory through a wrapper that creates an isolated temporary Cargo home with an absolute source replacement. CI exercises both modes from a fresh checkout and compares their resolved dependency graphs.

**Tech Stack:** Rust/Cargo 1.94.1, Bash, Python 3 standard library, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-25-e4-fresh-clone-build-reproducibility-design.md`

## Global Constraints

- Canonical base is `main@3186d53365fb90e9e40340f7bba52a22bcd012ee`.
- Do not change `Cargo.toml`, `Cargo.lock`, dependency versions, solver source, tolerances, policies, scientific fixtures, or expected outputs.
- Do not commit a vendor directory.
- No wall timing, ranking, speedup claim, active switching, PM-4 Task 2, A1, A2/A3, tag, release, or merge.
- Exact vendor package count is diagnostic only; structural validity plus Cargo frozen closure is authority.

---

### Task 1: Encode the failing default/offline build-mode contract

**Files:**
- Create: `tools/test_e4_build_modes.py`

**Interfaces:**
- Consumes: repository root, `.cargo` configuration, README, `.gitignore`, workflow and helper paths.
- Produces: one standard-library unittest suite that fails on the pre-E4 tree and guards every later implementation step.

- [ ] **Step 1: Write the failing tests**

Require:

```text
default config has no forced source replacement or offline policy
offline template exists but is not auto-discovered
wrapper and vendor validator exist
vendor/ is ignored
README documents both modes
CI checks default and explicit offline builds
```

Include structural vendor fixtures at package counts 3 and 5, hidden/manifestless directory cases, missing checksum, missing `faer-0.24.4`, and metadata graph mismatch.

- [ ] **Step 2: Verify RED**

Run:

```bash
python3 tools/test_e4_build_modes.py -v
```

Expected: failures caused by the forced default configuration and absent E4 files.

- [ ] **Step 3: Commit the contract**

```bash
git add tools/test_e4_build_modes.py
git commit -m "test: define E4 fresh-clone build-mode contract"
```

### Task 2: Separate default and explicit offline Cargo modes

**Files:**
- Modify or delete: `.cargo/config.toml`
- Create: `.cargo/config.offline.toml`
- Modify: `.gitignore`
- Create: `tools/validate-cargo-vendor.py`
- Create: `tools/cargo-offline.sh`
- Create: `tools/compare-cargo-metadata.py`
- Modify: `README.md`

**Interfaces:**
- `validate_vendor_source(path: pathlib.Path) -> dict[str, object]`
- `tools/cargo-offline.sh [--vendor-dir PATH] CARGO_SUBCOMMAND...`
- `tools/compare-cargo-metadata.py DEFAULT.json OFFLINE.json`

- [ ] **Step 1: Remove the default source replacement/offline policy**

Remove the existing source replacement/offline policy from `.cargo/config.toml`; deletion or a neutral comment-only file is acceptable. Do not add another auto-discovered Cargo config.

- [ ] **Step 2: Add the opt-in template**

Create `.cargo/config.offline.toml` with:

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
offline = true
```

This file documents manual repository-local offline use; the wrapper may create an equivalent temporary Cargo home with an absolute path.

- [ ] **Step 3: Implement structural vendor validation**

At one immediate directory level:

```text
ignore dot-prefixed directories
ignore directories without Cargo.toml
require parseable Cargo.toml and .cargo-checksum.json for each candidate
require faer version 0.24.4
never enforce an exact global package count
```

The JSON report must be deterministic and include observed counts and required-package evidence.

- [ ] **Step 4: Implement the offline wrapper**

Parse `--vendor-dir` or `VIGILODE_CARGO_VENDOR_DIR`, canonicalize it, run the validator, create a temporary Cargo home containing the absolute source replacement and offline mode, invoke Cargo from the repository root, forward the exit status, and clean up on exit. Reject missing commands or vendors. Do not mutate tracked files.

- [ ] **Step 5: Implement metadata graph comparison**

Compare package identity/version/source/dependency edges, workspace membership, and resolve root. Ignore environment-specific target directory, workspace root, and manifest absolute paths.

- [ ] **Step 6: Document the public interface and ignore vendor**

Add `/vendor/` to `.gitignore`. Document default locked commands, `cargo vendor`, wrapper examples, accepted inputs, and tracked-file non-mutation.

- [ ] **Step 7: Verify GREEN**

Run:

```bash
python3 tools/test_e4_build_modes.py -v
python3 -m compileall -q tools
bash -n tools/cargo-offline.sh
git diff --check
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add .cargo .gitignore README.md tools
git commit -m "fix: make Cargo offline mode explicit and portable"
```

### Task 3: Add fresh-clone CI closure

**Files:**
- Create: `.github/workflows/e4-fresh-clone-build.yml`

**Interfaces:**
- Produces a fresh-runner proof of the default locked build and explicit air-gapped build from the same lockfile.

- [ ] **Step 1: Add the workflow**

The workflow must:

```text
checkout fresh
install Rust 1.94.1 plus rustfmt/clippy
run Python E4 tests
run default cargo metadata --locked
compile workspace --all-targets --no-run --locked
run the focused v3.8-D test
run focused Clippy with -D warnings
cargo vendor --locked into RUNNER_TEMP
run offline metadata and compile through tools/cargo-offline.sh
compare default/offline metadata
run the focused offline test
run rustfmt and diff/clean-tree checks
```

- [ ] **Step 2: Verify workflow contract locally**

Run:

```bash
python3 tools/test_e4_build_modes.py -v
```

Expected: workflow static contract passes.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/e4-fresh-clone-build.yml
git commit -m "ci: verify E4 fresh-clone build modes"
```

### Task 4: Run exact local closure and open the draft PR

**Files:**
- Create local-only evidence logs outside the Git diff.

**Interfaces:**
- Produces a verified E4 feature head and a draft PR; does not merge.

- [ ] **Step 1: Validate the actual persistent vendor**

Run the structural validator against the discovered local Cargo vendor and record dynamic counts. Do not compare against a historical count.

- [ ] **Step 2: Run offline frozen closure**

```bash
./tools/cargo-offline.sh --vendor-dir "$VENDOR_DIR" metadata --frozen --format-version 1
./tools/cargo-offline.sh --vendor-dir "$VENDOR_DIR" test --workspace --all-targets --no-run --frozen
./tools/cargo-offline.sh --vendor-dir "$VENDOR_DIR" test -p rodas5p-integrators --test v38d_performance_probe_contracts --frozen
./tools/cargo-offline.sh --vendor-dir "$VENDOR_DIR" clippy -p rodas5p-integrators --all-targets --frozen -- -D warnings
```

Expected: metadata/build/test/Clippy pass; focused test reports exactly 5 tests.

- [ ] **Step 3: Verify direct template mode**

Temporarily create an ignored repository-root `vendor` symlink and invoke Cargo with `.cargo/config.offline.toml`; remove the symlink afterwards. Confirm metadata succeeds and no tracked file changes.

- [ ] **Step 4: Run final repository checks**

```bash
cargo fmt --all -- --check
git diff --check
git status --porcelain
```

Expected: only the declared E4 files differ before commit; clean tree after commit.

- [ ] **Step 5: Push and open one draft PR**

Use ordinary non-force semantics. PR claims only build-reproducibility closure and explicitly states no scientific, timing, ranking, Task 2, A1, or A2/A3 work.

### Task 5: Fresh-context review and stop

**Files:**
- No implementation files unless a reviewer finds a bounded E4 defect.

- [ ] **Step 1: Review the exact remote diff and CI**

A fresh reviewer verifies default fresh-clone independence, explicit offline structure, lockfile preservation, metadata parity, tests, and claim boundary.

- [ ] **Step 2: Classify findings**

Report P0/P1/P2/P3 with exact reproducer. Do not fix on the first pass.

- [ ] **Step 3: Stop at the integration gate**

Pass requires P0=0 and P1=0 and successful CI. Do not merge. The next planned node is A1 inner-tolerance parity.
