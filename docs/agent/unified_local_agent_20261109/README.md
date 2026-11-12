# VigilODE Unified Local-Agent Package

이 패키지는 2026-11-09 KST 20:38:53의 VigilODE 연구 branch를 기준으로 여러 branch/PR에서 분산된 solver 계획과 근거를 한 DAG로 통합한다.

## Files
- `VIGILODE_UNIFIED_DAG_20261109.json`: machine-readable task graph, lineage, authority, source lock, gates and claim boundaries.
- `LOCAL_AGENT_DEVELOPMENT_SPEC_KO.md`: method contracts, mutation policy, verification protocol, negative evidence and anti-drift rules.
- `MUTATION_GATES_AND_EVIDENCE_CONTRACT.json`: mutation/evidence status schema and stop rules.
- `LOCAL_AGENT_HANDOFF_PROMPT_KO.txt`: copy-ready low-capability local-agent prompt.
- `validate_unified_dag.py`: JSON, dependency, DAG-cycle, G0 immutability and required-node checks.
- `PACKAGE_MANIFEST.json`: SHA-256 of package files (excluding the manifest itself).

The DAG explicitly carries the R5–R10/C5 lineage, WP6–WP9 source-conditioned line, C8/PR98 symbolic router, RVJ/Volterra and Fourier negatives, application-only REI/BASS21 line, Peer-W PTSW2B/C, EPPEER/DQC and PW07. The parent source package remains a required reading source; it is not copied wholesale into this compact handoff.

## Current scientific verdict
`HOLD_NOT_RELEASE_QUALIFIED`. The original G0 graph and thresholds remain immutable. This package is an additive documentation/plan overlay, not a code qualification or release proposal.

## Validate locally
```bash
python3 validate_unified_dag.py
python3 -m json.tool VIGILODE_UNIFIED_DAG_20261109.json >/dev/null
python3 -m json.tool MUTATION_GATES_AND_EVIDENCE_CONTRACT.json >/dev/null
```

## Provenance
Latest research HEAD: `bd82eff9d1d967ef489ac4eb0c302ef6a4102f31`.
Pinned code source: `569ce02629498fec133db9ce368eb303ba943b69`, tree `a4b2f09521b197b37612ff7dc66a13fbb74a0bae`.
The latest research branch descends from the docs plan branch `becbc75438354d7bc8ae3fea362ce943d4d2272a`; it does not replace the code source lock.
