# A07 Robertson comparison harness

This isolated harness compares the native VigilODE `integrate_rodas5p_fast_observed`
path with Diffsol 0.16.1 BDF. It does not modify the workspace Cargo manifests,
lockfile, or the earlier Diffsol campaign.

Frozen problem contract: Robertson kinetics, `y(0)=[1,0,0]`, `[0,40]`, 101
uniform retained output times `40*i/100`, `rtol=1e-8`, and scalar `atol=1e-11`
represented as `[1e-11; 3]` on both solvers. The reference uses SciPy Radau at
three tolerances and an independent LSODA check. The accepted reference spread
threshold is 0.05 in maximum-grid normalized WRMS units. This is an empirical
stability check, not a rigorous enclosure.

All binaries, JSONL rows, logs, and generated reports belong under
`research/production_audit_20261018/evidence/continuation_a07/`. Do not include
reference generation in either arm's timed call. Timing authority remains
`diagnostic_only` regardless of the result.

## Build and run

From this directory, after the controller confirms the linked A01 source is
frozen:

```sh
export RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
cargo build --locked --offline --profile measurement
cp target/measurement/vigilode-a07 target/measurement/vigilode-a07-timing
cargo build --locked --offline --profile measurement --features allocation-audit
cp target/measurement/vigilode-a07 target/measurement/vigilode-a07-allocator
```

Create the source/build manifest immediately before samples, after the final
build. `GRID` and `REFERENCE` below are the frozen evidence files:

```sh
python3 src/validate_report.py capture \
  --grid "$GRID" --reference "$REFERENCE" \
  --timing-binary target/measurement/vigilode-a07-timing \
  --allocator-binary target/measurement/vigilode-a07-allocator \
  --out "$EVIDENCE/manifest.json"
```

Cold observations use ten matched fresh-process pairs, alternating order.
Warm observations use twenty in-process ABBA/BAAB blocks. Allocation is measured
in a separate feature build and is outside timing:

```sh
python3 src/cold_pairs.py --binary target/measurement/vigilode-a07-timing \
  --grid "$GRID" --manifest "$EVIDENCE/manifest.json" --pairs 10 \
  --logs "$EVIDENCE/logs/cold" --out "$EVIDENCE/cold.jsonl"
target/measurement/vigilode-a07-timing --warm-abba 20 \
  --grid "$GRID" --manifest "$EVIDENCE/manifest.json" > "$EVIDENCE/warm.jsonl"
target/measurement/vigilode-a07-allocator --alloc native \
  --grid "$GRID" --manifest "$EVIDENCE/manifest.json" > "$EVIDENCE/alloc_native.jsonl"
target/measurement/vigilode-a07-allocator --alloc diffsol \
  --grid "$GRID" --manifest "$EVIDENCE/manifest.json" > "$EVIDENCE/alloc_diffsol.jsonl"
```

Validate all rows and generate the report only from the validated JSON:

```sh
python3 src/validate_report.py validate \
  --grid "$GRID" --reference "$REFERENCE" --manifest "$EVIDENCE/manifest.json" \
  --samples "$EVIDENCE/cold.jsonl" "$EVIDENCE/warm.jsonl" \
  "$EVIDENCE/alloc_native.jsonl" "$EVIDENCE/alloc_diffsol.jsonl" \
  --out "$EVIDENCE/validated.json" --markdown "$EVIDENCE/REPORT.md"
```

The validator requires all 20 cold rows, all 160 warm rows in the prescribed
order, and one allocation receipt for each arm. It retains failed rows and
blocks the accuracy comparison if schema, identity, output-grid, count, or
accuracy conditions fail. It reports timing ratios only as diagnostics and
does not infer a speed advantage from a single wall-clock campaign.

The timing build compiles out callback counters entirely: it has no callback
counter atomics or counter branches, and timing rows record
`callback_counters: null` with reason `callback counters disabled to preserve
timing path`. A separate `allocation-audit` build/run records callback counts
and allocator totals using the same equations, tolerances, and grid. That audit
has a different instrumentation boundary and is excluded from timing results.
