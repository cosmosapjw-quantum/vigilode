# K0 Stage Telemetry Codex Execution Package

This is a machine-readable, weak-agent-safe implementation package for canonical VigilODE K0 stage telemetry integration.

## Authority

- Repository: `https://github.com/cosmosapjw-quantum/vigilode`
- Canonical main: `8d0c79184e09efb5bdadc24a6315c60a71a44264`, tree `acd94364cf69f19d782619fc6c75554cb0754208`
- Required stacked implementation base: draft PR #20 at `e1124586a4029f86669e7489278c61ef676d61aa`, tree `adbb933cf3bf3d401d652c8a6d9df661d8500a2b`
- Package branch: `docs/k0-codex-execution-package-20260827`
- Control PR: `BOOTSTRAP_PENDING`
- Jira: `PM-7`
- Confluence: `BOOTSTRAP_PENDING`
- Publication state: `BOOTSTRAP`

## Objective

Integrate opt-in, behavior-neutral K0 per-stage telemetry; preserve every failure; then execute exactly two frozen kernel arms across six families under `LegacyFixed`.

## Read order

1. `AGENTS.md`
2. `plan.json`
3. `CODEX_HANDOFF_PROMPT.md`
4. active work-unit JSON
5. `docs/invariants/K0_STAGE_TELEMETRY.md`
6. `docs/quality/P0_P1_POLICY.md`

## Validate

```bash
python tools/verify-k0-stage-telemetry-plan.py --repo-root . --check-package
```

The package contains contracts and evidence pointers, not a canonical implementation. The external K0 prototype bundle remains exploratory and is identified only by SHA-256.
