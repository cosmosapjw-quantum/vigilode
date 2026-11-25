# M0-REF-ADJUDICATION-01: preparation only
Status: INPUTS_LOCKED_NOT_RUN. No reference trajectory generated in this
preflight, and no historical dispute flag changed.

## Authority
- Historical original 1032 rows and expanded 2076 fresh rows (504 original
  key replays, 2030 successes / 46 failures) remain immutable. They retain
  their diagnostic timing status and their original source revisions.
- M0_INPUT_LOCK.json contains 30 historical run file byte hashes, both exact
  archived corpus byte identities, all 14 problem specs, and the 12 original
  disputes, including the full old independent ladder metadata.
- canonical.corpus.json and spatial.corpus.json are byte copies, not new
  settings. reference-plan materialized their actual 101-point grids and
  three levels without solving. All subsequent references are new artifacts.

## Sequential execution for the next authorized research stage
1. Verify input lock with prepare_m0.py into a new output directory. Verify
   repository clean, source commit, toolchain 1.99.0 benchmark and native
   comparator source identities; record runtime shared-library identity.
   Build the standalone benchmark with locked dependencies and source guard.
2. Reproduce the original independent RADAU5/CVODE ladders using the new
   reference-only command, not the full work-precision campaign. Save
   references, incomplete levels, status and raw provenance under new names.
   The old threshold 1e-10 and original three tolerance/atol levels are
   unchanged. A new run cannot erase an old failure.
3. For the disputed cases, choose and register one independent reference
   route per physical model *before* looking at its new output. Use exact
   analytic/semi-discrete solution where available and demonstrably the
   same spatial operator; otherwise independent high-precision or refined
   stiff numerical integration. For Robertson/HIRES/VDP/Oregonator use an
   independent stiff reference (arbitrary precision if implementable);
   for Brusselator and seven disputed spatial cases verify exact same
   ordering, IC/BC, coefficient values and spatial discretization, then
   independent sparse/implicit high-precision or refined comparator.
   Do not treat a continuum PDE solution as the reference for a fixed
   semidiscrete system. Do not use a VigilODE candidate as sole reference.
   For any new ladder freeze solver implementation/version, node grid,
   tolerance sequence, resource cap and acceptance criterion first.
4. Compare complete finite trajectories on the identical declared grid
   using max-over-grid WRMS, with the fixed common scale of the original
   corpus (error_rtol=1, per-component atol_scale + |y_ref|).
   Preserve reference direction or record both directional comparisons.
   Retain cross-family and successive-refinement disagreement separately.
   Require complete success and all preregistered bounds for each new
   protocol. Cross-family agreement and convergence constitute empirical
   evidence only, not rigorous interval enclosure. If a proven enclosure
   exists, distinguish it from empirical uncertainty.
5. Write a separate per-case adjudication receipt: original status, new
   protocol, reference identity, convergence, independence, failures and
   remaining uncertainty. Keep DISPUTED if evidence is insufficient;
   no forced global PASS. M6 matched-error benchmarking remains deferred,
   as do M1/M2/M3/M4/M5 full algorithm research.

## Commands (DO NOT execute references until M0 authorized)
From the clean published preflight checkout:
    export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-preflight-bench-20261125
    cargo +stable build --locked --offline -j8 --release --manifest-path crates/vigilode-bench/Cargo.toml
    out=research/retro_preflight_audit_20261125/m0
    bin=$CARGO_TARGET_DIR/release/vigilode-bench
    "$bin" references --corpus "$out/canonical.corpus.json" --out <fresh-canonical-reference.json> --require-independent-reference
    "$bin" references --corpus "$out/spatial.corpus.json" --out <fresh-spatial-reference.json> --require-independent-reference

Use actual fresh paths in place of placeholders. The new command refuses
existing output paths and uses the same RunGuard as wp. Only reference-plan
was executed this turn; reference-only numerical execution is NOT_RUN.
