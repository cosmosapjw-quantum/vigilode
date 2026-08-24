# VigilODE PM-4 Task-1 publication recovery handoff — referential closure revision

## Purpose

This handoff compiles the PM-4 Task-1 publication failure chain into executable contracts. The immediate goal is not PM-4 Task 2. It is to replace the brittle R4 Cargo-vendor count gate, create a verified R5 kit, publish the already-sealed Task-1 patch to draft PR #11 by ordinary non-force fast-forward, and stop before merge.

## Current authority

- repository: `cosmosapjw-quantum/vigilode`
- canonical `main`: `140f6b5c078c3d8fcd5b6c52310c063ee233dc12`
- target feature before publication: `b2d5ec41cb147e01aadbc9c42928da8abfa75c58`
- target PR: `#11`, open / draft / unmerged / zero file diff
- R4 archive SHA-256: `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`
- sealed Task-1 patch SHA-256: `705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3`
- R4 publication script SHA-256: `63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7`
- persistent vendor: `/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor`

The prior R4 outer hash `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095` is withdrawn and must be rejected, not accepted as an alternative.

## Failure chain absorbed into tests

1. R2 used a diff view that omitted staged/untracked paths.
2. R3 omitted the temporary sibling bridge required by checked-in Cargo source replacement.
3. R4 promoted a rehearsal-specific count `262` into a universal vendor gate; the actual host contained 308 immediate directories.
4. The first Codex run exposed contradictory R4 outer hashes and stopped safely.
5. The second Codex run exposed a dangling required path, `templates/COMPLETION_EVIDENCE_SCHEMA.json`, and stopped safely.

No production ref moved in any of these failures.

## Referential-closure gate

The missing contract is now supplied as a real JSON Schema Draft 2020-12 document, together with a positive example, executable semantic validator, positive/negative tests, and a scan that rejects every dangling `templates/` or `acceptance/` reference in load-bearing handoff files.

Before implementation run:

```bash
python3 -m unittest acceptance.test_handoff_completeness_contract -v
python3 -m unittest acceptance.test_completion_evidence_contract -v
python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json \
  --instance templates/COMPLETION_EVIDENCE_EXAMPLE.json
```

After successful publication, write `COMPLETION_EVIDENCE.json` and run:

```bash
python3 acceptance/validate_completion_evidence.py \
  --evidence COMPLETION_EVIDENCE.json
```

A blocked or partial run must not emit a success evidence object.

## Cargo closure rule

Do not replace `262` with `308`. At one immediate directory level, ignore dot-prefixed and manifestless directories, require `Cargo.toml` plus parseable `.cargo-checksum.json` for each package candidate, require `faer-0.24.4`, then let:

```bash
cargo metadata --frozen --format-version 1
```

prove the checked-in lockfile/config/vendor dependency closure. Observed counts are evidence, not identity.

## Read order

Follow `AGENTS.md`, then read state/correction records, the audit-compiled plan, threat and invariant matrices, evidence chain, formal templates, acceptance contracts, and `IMPLEMENTER_PROMPT.md`.

## Hard boundary

No `main` mutation, force push, merge, wall timing, candidate ranking, speedup claim, dependency change, or PM-4 Task 2 is authorized. PR #12 is documentation/transport only and must not be merged as part of Task-1 publication.
