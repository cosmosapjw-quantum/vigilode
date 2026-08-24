# VigilODE PM-4 Task-1 — content-equivalent final handoff

This handoff removes archive-byte identity from the PM-4 Task-1 publication
path. Repeated blockers came from conflating Git object identity, materialized
working-tree bytes, regenerated artifacts, numerical equivalence, and archive
packaging metadata.

## Current production authority

```text
repository
cosmosapjw-quantum/vigilode

canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

canonical main tree
77b8b9648b7acb4acddc2fd315b19e4257cf0fa5

PM-4 feature before Task-1 publication
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

implementation PR
#11 — OPEN / DRAFT / UNMERGED / ZERO FILE DIFF
```

## Direct Git-tracked payload

```text
payload/PM4_TASK1_SCHEMA_BOUNDARY.patch
```

No outer R4 archive, sidecar, base64 transport, or deterministic repacking is
required. The patch file in this handoff commit is the transport object. Its Git
blob identity and content are checked through Git itself.

The publication also deletes:

```text
research/generic_v38d_high_entropy_performance_tournament/RECOVERY_START.md
```

The final PR surface must be exactly:

```text
M  crates/rodas5p-integrators/src/lib.rs
A  crates/rodas5p-integrators/src/v38d_performance_tournament.rs
A  crates/rodas5p-integrators/tests/v38d_performance_probe_contracts.rs
D  research/generic_v38d_high_entropy_performance_tournament/RECOVERY_START.md
```

## Hard vs soft identity

Hard gates:

- Git commit/tree/blob authority for tracked source;
- immutable scientific input/data identity;
- exact allowed Git diff surface;
- Cargo dependency closure without lock/config mutation;
- focused tests, relevant compilation, Clippy, rustfmt, and clean diff;
- ordinary non-force fast-forward and unchanged `main`.

Not hard gates by themselves:

- tar/zip/wheel outer SHA mismatch;
- gzip timestamp, tar ordering, uid/gid, permission normalization, compressor
  version, or other packaging metadata;
- float-output byte drift when numerical invariants and declared tolerances pass;
- signed-zero or harmless serialization spelling differences.

A packaging SHA mismatch must be recorded, classified, and then ignored unless
byte-identical packaging is itself an explicit deliverable. It must never be
reported as a scientific-integrity failure on its own.

## Boundary

No merge, wall timing, candidate ranking, speedup claim, Task 2, dependency
update, Cargo config/lockfile change, or scientific policy retuning is allowed.
