# Phase 8 — Numerical reproducibility

## Determinism

The design check contains no random input, adaptive branch, wall-clock timing, or unordered map serialization. Two fresh invocations are byte-identical and share SHA-256 `cc6cb1c963e75415f5b35ba879d83b081a08fd7103e0f2cc27c234e63ec7743a`.

## Reference diversity

- Rust direct reference: `faer` partial-pivot LU on explicit block matrices.
- Independent reference: NumPy 2.3.5 reconstruction from the raw coefficient fixture.
- Structural toy: scalar nearest-neighbor stage chain.
- Adversarial toy: determinant-one nonnormal lower shift.
- Physical-space algebra screen: noncommuting `M,J` pair.

## Sensitivity represented in this loop

The deterministic report varies:

- `theta` over `0, 0.25, 0.5, 0.75, 1` at the target endpoint;
- `lambda` over the same set in the affine FLRH limit;
- nilpotent truncation depth `q=0,1,2,7`;
- nonnormal coupling `mu=0.5,1,2,5,10`;
- all powers `L^1` through `L^8`.

No timestep, nonlinear strength, tolerance, or hardware scaling sweep is performed because those belong to the next numerical-experiment branch.

## Reproduction command

```bash
export PATH=/mnt/data/rust-1.94.1-local/bin:$PATH
cargo build --workspace --release --offline --locked
./target/release/rodas5p homotopy-design-check --output design-check.json
sha256sum design-check.json
```

Expected status: `pass`; expected schema: `rodas5p-homotopy-design-check-v1`.

## Interpretation boundary

The observed rapid error decrease from `q=0` to `q=2` is a single affine screen and must not be reported as a convergence rate or speedup. The condition screen is deliberately pessimistic evidence that exact path regularity does not imply numerical benignity.
