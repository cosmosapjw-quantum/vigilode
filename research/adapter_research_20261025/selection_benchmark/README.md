# Selection-centered benchmark tools

Start with [PROTOCOL_KO.md](PROTOCOL_KO.md). The analyzer evaluates the actual policy
choice against only the relevant alternatives at the same physical checkpoint and
error ceiling. It preserves noncompletion, missing references, differing identities
and unqualified faster alternatives separately.

- `analyze_selection.py`: read-only Python standard-library evaluator.
- `selection_window.schema.json`: input structure and required identities.
- `test_analyze_selection.py`, `TEST.log`: 22 executed contract tests.
- `SYNTHETIC_CONTRACT_INPUT.json`, `SYNTHETIC_CONTRACT_OUTPUT.json`: **synthetic
  validator examples, not scientific performance data**.
- `capture_run_metadata.py`: capture current scientific source files, binary,
  host/affinity, compiler, recorded build command and build-receipt hash before a
  new measurement. It does not execute that build command or prove the binary was
  built from those sources.
- `NATIVE_IMPORT_CONTRACT.json`: missing bridge from whole-trajectory switcher
  pilot measurements to future same-checkpoint measurements.

Example source capture, on the host that actually runs the measurement:

```bash
python3 -B research/adapter_research_20261025/selection_benchmark/capture_run_metadata.py \
  --repo "$PWD" \
  --binary target/measurement/examples/evidence_switch_bench \
  --build-command 'cargo build --locked --offline --profile measurement -p vigilode --example evidence_switch_bench' \
  --build-receipt /path/to/ACTUAL_BUILD.log \
  --output /path/to/NEW_RUN_METADATA.json
```

Use the actual binary/profile/build log paths. The executable digest and source
manifest must correspond to the build; a final documentation commit alone cannot
establish this. `source_tree_sha256` here hashes a canonical scientific-file
manifest, rather than a Git SHA-1 tree object.

The native `evidence_switch_bench` reports complete trajectory solve time including
workspace and pilots. Keep those original exploratory results intact. Its
mode-by-mode repetitions are not decision-window paired replay, and it does not
separately time nested cost components. They are therefore not silently converted
into the strict window input. In a future runner, record the entire inclusive solve
under `solve_seconds` and zero separately charged components when those components
are already inside it; use the timing-scope document to record this inclusion.
Never invent an overhead decomposition.
