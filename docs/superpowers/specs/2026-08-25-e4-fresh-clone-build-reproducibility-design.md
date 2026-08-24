# E4 Fresh-Clone / Build Reproducibility Design

## Status

Approved implementation design for the E4 reproducibility closure following the merged PM-4 Task-1 schema checkpoint.

## Problem

The repository currently auto-discovers `.cargo/config.toml`, replaces crates.io with a vendored directory outside the repository, and forces Cargo offline. A fresh clone therefore cannot even resolve metadata unless a specific sibling path already exists. This couples the source tree to one developer's filesystem and prevents independent build verification.

## Goal

Make an ordinary fresh clone buildable through Cargo's normal locked registry path while preserving an explicit, validated air-gapped workflow that uses a caller-supplied standard Cargo vendor directory.

## Non-goals

- No solver, Krylov, tolerance, performance-policy, benchmark, or scientific-output changes.
- No dependency or lockfile updates.
- No vendored dependency tree committed to Git.
- No timing, candidate ranking, speedup claim, Task 2, A1, or A2/A3 work.

## Architecture

### Default build mode

The auto-discovered `.cargo/config.toml` is removed or reduced to neutral comments with no source replacement or offline policy. A normal clone uses Cargo's default crates.io source and the pinned `Cargo.lock`:

```bash
cargo metadata --locked --format-version 1
cargo test --workspace --all-targets --no-run --locked
```

The default path may use the network or an existing Cargo cache. It must not depend on any repository-external relative path.

### Explicit offline mode

A tracked `.cargo/config.offline.toml` documents the intended source replacement using a repository-local `vendor/` placeholder. The executable interface is `tools/cargo-offline.sh`, which accepts either `--vendor-dir PATH` or `VIGILODE_CARGO_VENDOR_DIR`.

The wrapper:

1. resolves the vendor directory to an absolute canonical path;
2. validates Cargo directory-source structure through `tools/validate-cargo-vendor.py`;
3. creates an isolated temporary Cargo home containing an absolute source replacement and offline policy;
4. invokes the pinned Cargo command from the repository root;
5. removes the temporary Cargo home on exit;
6. does not modify tracked Cargo configuration, manifests, or lockfile.

Vendor validity is structural, not an exact directory count. Immediate dot-prefixed and manifestless directories are ignored; every manifest-bearing package must contain parseable `.cargo-checksum.json`; `faer` version `0.24.4` must be present.

### Equivalence evidence

The default and offline `cargo metadata` dependency graphs are compared by `tools/compare-cargo-metadata.py`. Environment-specific top-level paths are excluded; package identity, version, source, dependency edges, workspace membership, and resolve root must agree.

### CI

A dedicated workflow checks out the repository into a fresh GitHub runner, installs Rust 1.94.1, verifies the Python contract suite, builds the default locked graph, vendors the same lockfile into a temporary directory, rebuilds under explicit offline mode, compares metadata graphs, runs the focused v3.8-D contract, checks Clippy and rustfmt, and verifies no tracked file was mutated.

## Failure policy

Hard blockers:

- default metadata/build requires the old sibling vendor;
- offline vendor structure is invalid;
- locked default and offline dependency graphs differ;
- `Cargo.lock`, `Cargo.toml`, tracked Cargo configuration, or scientific source changes unexpectedly;
- build, focused tests, Clippy, rustfmt, or diff checks fail.

Non-blocking diagnostics:

- vendor package count differs from a historical inventory;
- the caller's vendor path differs from the developer's path;
- Cargo cache location differs;
- packaging archive hashes differ.

## Test strategy

TDD contract tests verify:

- the default Cargo configuration does not force source replacement or offline mode;
- the offline template is not auto-discovered;
- the wrapper rejects a missing vendor and accepts both environment-variable and command-line selection;
- the wrapper provides an isolated temporary Cargo home to Cargo;
- valid structural vendor fixtures pass at different package counts;
- hidden and manifestless directories are ignored;
- missing checksums or missing `faer 0.24.4` fail;
- metadata comparison detects graph drift;
- CI contains both default and explicit offline fresh-clone paths;
- README and `.gitignore` expose the public workflow without tracking `vendor/`.

## Completion boundary

E4 closes when the branch has P0=0/P1=0, the fresh-clone CI workflow passes, default and offline dependency graphs match, no scientific source or dependency lock changed, and the PR remains unmerged pending explicit user approval. The next node is A1 inner-tolerance parity, not PM-4 Task 2.
