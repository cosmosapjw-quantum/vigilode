# Handoff acceptance

Run from any directory:

```bash
bash /absolute/path/to/handoff/acceptance/run_preflight.sh
```

The preflight validates the machine-readable identity policy and direct patch
payload. It deliberately does not inspect any tar/zip/wheel outer checksum.

The implementation agent later validates the actual repository state, Cargo
closure, Rust tests, final diff, and remote topology.
