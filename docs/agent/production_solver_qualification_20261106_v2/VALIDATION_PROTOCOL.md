# Validation protocol

Current pinned checks:
- cargo test --locked -p vigilode --release --test switching_v2_remediation_contracts: PASS 17/17
- cargo test --locked -p vigilode-coupled --release --test rhs_homotopy_contracts: PASS 17/17
- cargo fmt --all -- --check: PASS
- git diff --check: PASS

These do not imply full workspace tests/clippy/build, global enclosure validation or release qualification.

Future order: source authority -> exact task worktree -> RED test -> math/code map -> allowlist patch -> positive/negative tests -> required workspace checks -> one read-only review -> evidence record and owner closure.

Failure taxonomy: theory, numerical, implementation, runtime_environment, input_missing, authorization. Build failure is not a theorem refutation; blocked command is not PASS.
