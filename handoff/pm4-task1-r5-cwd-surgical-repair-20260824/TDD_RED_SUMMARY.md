# TDD RED evidence — PM-4 Task-1 CWD-independent repair

The tests were first executed against the superseded R3 handoff before any production repair.

Observed failures:

```text
missing acceptance/run_control_plane_preflight.sh
wrong sidecar basename accepted
multiple active sidecar records accepted
raw outer sidecar command remained in AUDIT_COMPILED_EXEC_PLAN.yaml
```

The direct absolute-path archive validator already passed from three working directories, which isolated the defect to the execution contract and sidecar parser rather than the archive bytes.

Remote mutation during RED: none.

Canonical production refs remained:

```text
main    140f6b5c078c3d8fcd5b6c52310c063ee233dc12
feature b2d5ec41cb147e01aadbc9c42928da8abfa75c58
```
