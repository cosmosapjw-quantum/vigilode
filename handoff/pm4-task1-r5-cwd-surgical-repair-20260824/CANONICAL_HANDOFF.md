# VigilODE PM-4 Task-1 R5 CWD-independent canonical handoff

This clean handoff supersedes the accumulated PM-4 recovery handoff in draft PR #12. It addresses one control-plane defect: a checksum sidecar containing a bare archive filename was invoked from an unspecified current working directory.

## Exact authority

```text
repository  cosmosapjw-quantum/vigilode
main        140f6b5c078c3d8fcd5b6c52310c063ee233dc12
main tree   77b8b9648b7acb4acddc2fd315b19e4257cf0fa5
feature     b2d5ec41cb147e01aadbc9c42928da8abfa75c58
PR #11      OPEN / DRAFT / UNMERGED / ZERO DIFF before Task-1 publication
R4 archive  6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333
Task-1 patch 705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3
R4 script    63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7
vendor       /home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
```

The prior `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095` declaration is withdrawn and must be rejected, not accepted as an alternative.

## Canonical control-plane gate

Set absolute paths and call the script by absolute path from three unrelated directories:

```bash
export PM4_R4_ARCHIVE="$HOME/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz"
export PM4_R4_SIDECAR="$HOME/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256"
bash /absolute/path/to/handoff/acceptance/run_control_plane_preflight.sh
```

The sole load-bearing outer archive validator is:

```bash
python3 /absolute/path/to/handoff/acceptance/test_archive_authority_contract.py \
  --archive "$PM4_R4_ARCHIVE" \
  --sidecar "$PM4_R4_SIDECAR"
```

It reads archive and sidecar by absolute path, requires exactly one active sidecar record, requires the exact archive basename, rejects the withdrawn hash, verifies the canonical outer SHA-256, extracts the archive, verifies internal `SHA256SUMS` with an explicit extraction-root CWD, and verifies the sealed Task-1 patch and R4 publication script.

A raw outer checksum-sidecar command is not a load-bearing gate because the sidecar stores a relative filename.

## Mandatory regressions

The canonical validator and aggregate preflight must pass from:

1. the handoff package root;
2. the repository root;
3. an unrelated temporary directory.

Negative tests must reject wrong hash, withdrawn hash, wrong basename, multiple active records, and missing archive. Load-bearing handoff files must contain no raw outer sidecar check.

## R5 implementation contract

After all control-plane gates pass:

1. reproduce and preserve the R4 `308 (expected 262)` failure;
2. build R5 from immutable R4 inputs;
3. add structural vendor validation that inspects one immediate directory level, ignores dot-prefixed and manifestless directories, treats only directories containing `Cargo.toml` as package candidates, requires parseable `.cargo-checksum.json` for every candidate, requires `faer-0.24.4`, and never enforces an exact global count;
4. test valid 262 and 308 sources, incidental directories, missing checksum, missing `faer`, and deterministic JSON;
5. create and verify the sibling vendor bridge required by checked-in Cargo config;
6. run `cargo metadata --frozen --format-version 1` before tests or push;
7. preserve the Task-1 patch and exact final Rust file hashes;
8. require exact final M/A/A/D surface on:
   - `M crates/rodas5p-integrators/src/lib.rs`
   - `A crates/rodas5p-integrators/src/v38d_performance_tournament.rs`
   - `A crates/rodas5p-integrators/tests/v38d_performance_probe_contracts.rs`
   - `D research/generic_v38d_high_entropy_performance_tournament/RECOVERY_START.md`
9. run focused five tests, all-target compile, Clippy with `-D warnings`, rustfmt, and diff/clean-tree checks;
10. rehearse against an isolated clone and local bare remote;
11. deterministically package and fresh-extract R5;
12. immediately recheck exact main and feature refs;
13. perform one ordinary non-force fast-forward to the feature branch;
14. independently verify main unchanged, normal ancestry, exact four-file bytes/surface, PR #11 open/draft/unmerged, and zero wall campaigns;
15. produce completion evidence and stop before merge or Task 2.

## Hard bans

No main mutation, force push, merge, wall timing, candidate ranking, Task 2, dependency/Cargo-config/lockfile change, Task-1 patch change, test weakening, failure suppression, or partial-success report.

On any failed gate emit `BLOCKED_BY_UNRESOLVED_SPEC`, preserve the exact command, exit code, stdout/stderr, workdir, and unchanged remote refs, then stop.

## Fresh-context review

A separate reviewer must not fix on the first pass. It independently verifies three-CWD preflight, absence of raw sidecar gates, exact archive chain, structural Cargo vendor closure, `cargo metadata --frozen` ordering, exact Task-1 hashes/surface, all Rust checks, normal feature fast-forward, unchanged main, draft/unmerged PR state, valid completion evidence, and no wall timing/merge/Task 2. Pass requires P0=0 and P1=0.
